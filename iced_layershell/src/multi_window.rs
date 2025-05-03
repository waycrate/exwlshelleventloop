mod state;
use crate::{
    DefaultStyle,
    actions::{
        IcedNewMenuSettings, IcedNewPopupSettings, LayerShellActionVec,
        LayershellCustomActionsWithId, LayershellCustomActionsWithIdInner, MenuDirection,
    },
    multi_window::window_manager::WindowManager,
    settings::VirtualKeyboardSettings,
};
use std::{
    borrow::Cow, collections::HashMap, f64, mem::ManuallyDrop, os::fd::AsFd, sync::Arc,
    time::Duration,
};

use crate::{
    actions::{LayerShellAction, LayershellCustomActions},
    clipboard::LayerShellClipboard,
    conversion,
    error::Error,
};

use super::Appearance;
use iced::theme;
use iced_graphics::{Compositor, compositor};
use iced_runtime::{Action, Task};

use iced_runtime::debug;

use crate::program::multi_window::Program;
use iced_core::{Size, mouse::Cursor, time::Instant};
use iced_runtime::{UserInterface, user_interface};

use iced_futures::{Executor, Runtime, Subscription};

use layershellev::{
    LayerEvent, NewPopUpSettings, ReturnData, WindowState,
    calloop::timer::{TimeoutAction, Timer},
    reexport::wayland_client::{WlCompositor, WlRegion},
    reexport::zwp_virtual_keyboard_v1,
};

use futures::{StreamExt, channel::mpsc};

use crate::{
    event::{IcedLayerEvent, MultiWindowIcedLayerEvent},
    proxy::IcedProxy,
    settings::Settings,
};

mod window_manager;

/// An interactive, native cross-platform application.
///
/// This trait is the main entrypoint of Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// An [`Application`] can execute asynchronous actions by returning a
/// [`Task`] in some of its methods.
///
/// When using an [`Application`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Application: Program
where
    Self::Theme: DefaultStyle,
{
    /// The data needed to initialize your [`Application`].
    type Flags;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Task`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Task<Self::Message>);

    /// The name space of the layershell
    fn namespace(&self) -> String;

    fn remove_id(&mut self, _id: iced_core::window::Id);
    /// Returns the current `Theme` of the [`Application`].
    fn theme(&self, id: iced_core::window::Id) -> Self::Theme;

    /// Returns the `Style` variation of the `Theme`.
    fn style(&self, theme: &Self::Theme, _id: iced_core::window::Id) -> Appearance {
        theme.base()
    }

    /// Returns the event `Subscription` for the current state of the
    /// application.
    ///
    /// The messages produced by the `Subscription` will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// A `Subscription` will be kept alive as long as you keep returning it!
    ///
    /// By default, it returns an empty subscription.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self, _window: iced::window::Id) -> f64 {
        1.0
    }

    /// Defines whether or not to use natural scrolling
    fn natural_scroll(&self) -> bool {
        false
    }

    /// Returns whether the [`Application`] should be terminated.
    ///
    /// By default, it returns `false`.
    fn should_exit(&self) -> bool {
        false
    }
}

type MultiRuntime<E, Message> = Runtime<E, IcedProxy<Action<Message>>, Action<Message>>;

// a dispatch loop, another is listen loop
pub fn run<A, E, C>(
    settings: Settings<A::Flags>,
    compositor_settings: iced_graphics::Settings,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<LayershellCustomActionsWithId, Error = A::Message>,
{
    use futures::Future;
    use futures::task;

    let (message_sender, message_receiver) = std::sync::mpsc::channel::<Action<A::Message>>();

    debug::init(A::name());

    let boot_span = debug::boot();
    let proxy = IcedProxy::new(message_sender);
    let mut runtime: MultiRuntime<E, A::Message> = {
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };
    let (application, task) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    if let Some(stream) = iced_runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(iced_futures::subscription::into_recipes(
        runtime.enter(|| application.subscription().map(Action::Output)),
    ));

    let ev: WindowState<iced::window::Id> =
        layershellev::WindowState::new(&application.namespace())
            .with_start_mode(settings.layer_settings.start_mode)
            .with_use_display_handle(true)
            .with_events_transparent(settings.layer_settings.events_transparent)
            .with_option_size(settings.layer_settings.size)
            .with_layer(settings.layer_settings.layer)
            .with_anchor(settings.layer_settings.anchor)
            .with_exclusive_zone(settings.layer_settings.exclusive_zone)
            .with_margin(settings.layer_settings.margin)
            .with_keyboard_interacivity(settings.layer_settings.keyboard_interactivity)
            .build()
            .expect("Cannot create layershell");

    let (mut event_sender, event_receiver) =
        mpsc::unbounded::<MultiWindowIcedLayerEvent<Action<A::Message>>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded::<LayerShellActionVec>();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor_settings,
        runtime,
        event_receiver,
        control_sender,
        settings.fonts,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    boot_span.finish();

    let mut wl_input_region: Option<WlRegion> = None;

    let _ = ev.running_with_proxy(message_receiver, move |event, ev, index| {
        use layershellev::DispatchMessage;
        let mut def_returndata = ReturnData::None;
        let sended_id = index
            .and_then(|index| ev.get_unit_with_id(index))
            .map(|unit| unit.id());
        match event {
            LayerEvent::InitRequest => {
                def_returndata = ReturnData::RequestBind;
            }
            LayerEvent::BindProvide(globals, qh) => {
                let wl_compositor = globals
                    .bind::<WlCompositor, _, _>(qh, 1..=1, ())
                    .expect("could not bind wl_compositor");
                wl_input_region = Some(wl_compositor.create_region(qh, ()));

                if settings.virtual_keyboard_support.is_some() {
                    let virtual_keyboard_manager = globals
                        .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                            qh,
                            1..=1,
                            (),
                        )
                        .expect("no support virtual_keyboard");
                    let VirtualKeyboardSettings {
                        file,
                        keymap_size,
                        keymap_format,
                    } = settings.virtual_keyboard_support.as_ref().unwrap();
                    let seat = ev.get_seat();
                    let virtual_keyboard_in =
                        virtual_keyboard_manager.create_virtual_keyboard(seat, qh, ());
                    virtual_keyboard_in.keymap((*keymap_format).into(), file.as_fd(), *keymap_size);
                    ev.set_virtual_keyboard(virtual_keyboard_in);
                }
            }
            LayerEvent::RequestMessages(message) => 'outside: {
                if let DispatchMessage::RequestRefresh {
                    width,
                    height,
                    scale_float,
                    ..
                } = message {
                    let Some(unit) = ev.get_mut_unit_with_id(sended_id.unwrap()) else {
                        break 'outside;
                    };
                    event_sender
                        .start_send(MultiWindowIcedLayerEvent(
                            sended_id,
                            IcedLayerEvent::RequestRefreshWithWrapper {
                                width: *width,
                                height: *height,
                                fractal_scale: *scale_float,
                                wrapper: unit.gen_wrapper(),
                                info: unit.get_binding().cloned(),
                                is_mouse_surface: sended_id.map(|id| ev.is_mouse_surface(id)).unwrap_or(false),
                            },
                        ))
                        .expect("Cannot send");
                    break 'outside;
                }

                event_sender
                    .start_send(MultiWindowIcedLayerEvent(sended_id, message.into()))
                    .expect("Cannot send");
            }

            LayerEvent::UserEvent(event) => {
                event_sender
                    .start_send(MultiWindowIcedLayerEvent(
                        sended_id,
                        IcedLayerEvent::UserEvent(event),
                    ))
                    .ok();
            }
            LayerEvent::NormalDispatch => {
                event_sender
                    .start_send(MultiWindowIcedLayerEvent(
                        sended_id,
                        IcedLayerEvent::NormalUpdate,
                    ))
                    .expect("Cannot send");
            }
            _ => {}
        }
        let poll = instance.as_mut().poll(&mut context);
        let task::Poll::Pending = poll else {
            return ReturnData::RequestExit;
        };

        let Ok(Some(flows)) = control_receiver.try_next() else {
            return def_returndata;
        };
        for flow in flows {
            match flow {
                LayerShellAction::CustomActionsWithId(
                    LayershellCustomActionsWithIdInner(id, option_id, action),
                ) => 'out: {
                    match action {
                        LayershellCustomActions::AnchorChange(anchor) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_anchor(anchor);
                        }
                        LayershellCustomActions::AnchorSizeChange(anchor, size) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_anchor_with_size(anchor, size);
                        }
                        LayershellCustomActions::LayerChange(layer) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_layer(layer);
                        }
                        LayershellCustomActions::MarginChange(margin) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_margin(margin);
                        }
                        LayershellCustomActions::SizeChange((width, height)) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_size((width, height));
                        }
                        LayershellCustomActions::ExclusiveZoneChange(zone_size) => {
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            window.set_exclusive_zone(zone_size);
                        }
                        LayershellCustomActions::SetInputRegion(set_region) => {
                            let set_region = set_region.0;
                            let Some(id) = id else {
                                tracing::error!("Here should be an id, it is a bug, please report an issue for us");
                                break 'out;
                            };
                            let Some(window) = ev.get_window_with_id(id) else {
                                break 'out;
                            };
                            let Some(region) = &wl_input_region else {
                                break 'out;
                            };

                            let window_size = window.get_size();
                            let width: i32 = window_size.0.try_into().unwrap_or_default();
                            let height: i32 = window_size.1.try_into().unwrap_or_default();

                            region.subtract(0, 0, width, height);
                            set_region(region);

                            window
                                .get_wlsurface()
                                .set_input_region(wl_input_region.as_ref());
                        }
                        LayershellCustomActions::VirtualKeyboardPressed { time, key } => {
                            use layershellev::reexport::wayland_client::KeyState;
                            let ky = ev.get_virtual_keyboard().unwrap();
                            ky.key(time, key, KeyState::Pressed.into());

                            let eh = ev.get_loop_handler().unwrap();
                            eh.insert_source(
                                Timer::from_duration(Duration::from_micros(100)),
                                move |_, _, state| {
                                    let ky = state.get_virtual_keyboard().unwrap();

                                    ky.key(time, key, KeyState::Released.into());
                                    TimeoutAction::Drop
                                },
                            )
                            .ok();
                        }
                        LayershellCustomActions::NewLayerShell {
                            settings, id: info, ..
                        } => {
                            let id = layershellev::id::Id::unique();
                            ev.append_return_data(ReturnData::NewLayerShell((
                                settings,
                                id,
                                Some(info),
                            )));
                        }
                        LayershellCustomActions::RemoveWindow(id) => {
                            ev.remove_shell(option_id.unwrap());
                            event_sender
                                .start_send(MultiWindowIcedLayerEvent(
                                    None,
                                    IcedLayerEvent::WindowRemoved(id),
                                ))
                                .ok();
                        }
                        LayershellCustomActions::NewPopUp {
                            settings: menusettings,
                            id: info,
                        } => {
                            let IcedNewPopupSettings { size, position } = menusettings;
                            let Some(id) = ev.current_surface_id() else {
                                break 'out;
                            };
                            let popup_settings = NewPopUpSettings { size, position, id };
                            let id = layershellev::id::Id::unique();
                            ev.append_return_data(ReturnData::NewPopUp((
                                popup_settings,
                                id,
                                Some(info),
                            )));
                        }
                        LayershellCustomActions::NewMenu {
                            settings: menusetting,
                            id: info,
                        } => {
                            let Some(id) = ev.current_surface_id() else {
                                break 'out;
                            };
                            event_sender
                                .start_send(MultiWindowIcedLayerEvent(
                                    Some(id),
                                    IcedLayerEvent::NewMenu((menusetting, info)),
                                ))
                                .expect("Cannot send");
                        }
                        LayershellCustomActions::ForgetLastOutput => {
                            ev.forget_last_output();
                        }
                    }
                }
                LayerShellAction::ImeWithId(id, ime, ime_flags) => match ime{
                    iced_core::InputMethod::Disabled => {
                        use crate::ime_preedit::ImeState;
                        if ime_flags.contains(ImeState::Disabled) {
                            ev.set_ime_allowed(false);
                        }
                    }
                    iced_core::InputMethod::Enabled {
                        position, purpose, ..
                    } => {
                        use crate::ime_preedit::ImeState;
                        if ime_flags.contains(ImeState::Allowed) {
                            ev.set_ime_allowed(true);
                        }

                        if ime_flags.contains(ImeState::Update) {
                            ev.set_ime_purpose(conversion::ime_purpose(purpose));
                            ev.set_ime_cursor_area(
                                layershellev::dpi::LogicalPosition::new(position.x, position.y),
                                layershellev::dpi::LogicalSize {
                                    width: 10,
                                    height: 10,
                                },
                                id,
                            );
                        }
                    }
                },
                LayerShellAction::NewMenu(menusettings, info) => 'out: {
                    let IcedNewPopupSettings { size, position } = menusettings;
                    let Some(id) = ev.current_surface_id() else {
                        break 'out;
                    };
                    let popup_settings = NewPopUpSettings { size, position, id };
                    let id = layershellev::id::Id::unique();
                    ev.append_return_data(ReturnData::NewPopUp((popup_settings, id, Some(info))))
                }
                LayerShellAction::Mouse(mouse) => {
                    let Some(pointer) = ev.get_pointer() else {
                        return ReturnData::None;
                    };

                    ev.append_return_data(ReturnData::RequestSetCursorShape((
                        conversion::mouse_interaction(mouse),
                        pointer.clone(),
                    )));
                }
                LayerShellAction::RedrawAll => {
                    ev.append_return_data(ReturnData::RedrawAllRequest);
                }
                LayerShellAction::RedrawWindow(index) => {
                    ev.append_return_data(ReturnData::RedrawIndexRequest(index));
                }
                _ => {}
            }
        }
        def_returndata
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: A,
    compositor_settings: iced_graphics::Settings,
    mut runtime: MultiRuntime<E, A::Message>,
    mut event_receiver: mpsc::UnboundedReceiver<MultiWindowIcedLayerEvent<Action<A::Message>>>,
    mut control_sender: mpsc::UnboundedSender<LayerShellActionVec>,
    fonts: Vec<Cow<'static, [u8]>>,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<LayershellCustomActionsWithId, Error = A::Message>,
{
    use iced::window;
    use iced_core::Event;
    let mut compositor: Option<C> = None;
    let default_fonts = fonts;
    macro_rules! replace_compositor {
        ($window:expr) => {
            let mut new_compositor = C::new(compositor_settings, $window.clone())
                .await
                .expect("Cannot create compositer");
            let fonts = default_fonts.clone();
            for font in fonts {
                new_compositor.load_font(font);
            }
            compositor.replace(new_compositor);
        };
    }
    let mut window_manager: WindowManager<A, C> = WindowManager::new();
    let mut cached_layer_dimensions: HashMap<iced_core::window::Id, (iced_core::Size<u32>, f64)> =
        HashMap::new();

    let mut clipboard = LayerShellClipboard::unconnected();
    let mut ui_caches: HashMap<window::Id, user_interface::Cache> = HashMap::new();

    let mut user_interfaces = ManuallyDrop::new(build_user_interfaces(
        &application,
        &mut window_manager,
        HashMap::new(),
    ));

    let mut events = Vec::new();
    let mut custom_actions = Vec::new();
    let mut waiting_actions = Vec::new();

    let mut should_exit = false;
    let mut messages = Vec::new();

    while let Some(event) = event_receiver.next().await {
        waiting_actions.retain(|(id, custom_action)| {
            let Some(layerid) = window_manager.get_layer_id(*id) else {
                // NOTE: here, the layershell or popup has not been created
                // Still need to wait for sometime
                return true;
            };
            let option_id = if let LayershellCustomActions::RemoveWindow(id) = custom_action {
                let option_id = window_manager.get_layer_id(*id);
                if option_id.is_none() {
                    // NOTE: drop it
                    return false;
                }
                option_id
            } else {
                None
            };
            custom_actions.push(LayerShellAction::CustomActionsWithId(
                LayershellCustomActionsWithIdInner(Some(layerid), option_id, custom_action.clone()),
            ));
            false
        });
        match event {
            MultiWindowIcedLayerEvent(
                oid,
                IcedLayerEvent::RequestRefreshWithWrapper {
                    width,
                    height,
                    fractal_scale,
                    wrapper,
                    info,
                    is_mouse_surface,
                },
            ) => {
                let mut is_new_window = false;
                let (id, window) =
                    if let Some((id, window)) = window_manager.get_mut_alias(wrapper.id()) {
                        let window_size = window.state.window_size();

                        if window_size.width != width
                            || window_size.height != height
                            || window.state.wayland_scale_factor() != fractal_scale
                        {
                            let layout_span = debug::layout(id);
                            let ui = user_interfaces.remove(&id).expect("Get User interface");
                            window.state.update_view_port(width, height, fractal_scale);

                            let _ = user_interfaces.insert(
                                id,
                                ui.relayout(
                                    window.state.viewport().logical_size(),
                                    &mut window.renderer,
                                ),
                            );
                            layout_span.finish();
                        }
                        (id, window)
                    } else {
                        let wrapper = Arc::new(wrapper);
                        is_new_window = true;
                        let id = info.unwrap_or_else(window::Id::unique);
                        if compositor.is_none() {
                            replace_compositor!(wrapper);
                            clipboard = LayerShellClipboard::connect(&wrapper);
                        }

                        debug::theme_changed(|| {
                            if window_manager.is_empty() {
                                theme::Base::palette(&application.theme(id))
                            } else {
                                None
                            }
                        });
                        let window = window_manager.insert(
                            id,
                            (width, height),
                            fractal_scale,
                            wrapper,
                            &application,
                            compositor.as_mut().expect("It should have been created"),
                        );

                        let _ = user_interfaces.insert(
                            id,
                            build_user_interface(
                                &application,
                                user_interface::Cache::default(),
                                &mut window.renderer,
                                window.state.viewport().logical_size(),
                                id,
                            ),
                        );
                        let _ = ui_caches.insert(id, user_interface::Cache::default());

                        events.push((
                            Some(id),
                            Event::Window(window::Event::Opened {
                                position: None,
                                size: window.state.window_size_f32(),
                            }),
                        ));
                        (id, window)
                    };
                let compositor = compositor
                    .as_mut()
                    .expect("The compositor should have been created");

                let ui = user_interfaces.get_mut(&id).expect("Get User interface");

                let redraw_event =
                    iced_core::Event::Window(window::Event::RedrawRequested(Instant::now()));

                let cursor = if is_mouse_surface {
                    window.state.cursor()
                } else {
                    Cursor::Unavailable
                };

                events.push((Some(id), redraw_event.clone()));

                let draw_span = debug::draw(id);
                let (ui_state, _) = ui.update(
                    &[redraw_event.clone()],
                    cursor,
                    &mut window.renderer,
                    &mut clipboard,
                    &mut messages,
                );

                let new_mouse_interaction = ui.draw(
                    &mut window.renderer,
                    window.state.theme(),
                    &iced_core::renderer::Style {
                        text_color: window.state.text_color(),
                    },
                    cursor,
                );

                draw_span.finish();

                if new_mouse_interaction != window.mouse_interaction {
                    custom_actions.push(LayerShellAction::Mouse(new_mouse_interaction));
                    window.mouse_interaction = new_mouse_interaction;
                }

                let physical_size = window.state.viewport().physical_size();

                if cached_layer_dimensions
                    .get(&id)
                    .is_none_or(|(size, scale)| {
                        *size != physical_size || *scale != window.state.viewport().scale_factor()
                    })
                {
                    cached_layer_dimensions
                        .insert(id, (physical_size, window.state.viewport().scale_factor()));

                    compositor.configure_surface(
                        &mut window.surface,
                        physical_size.width,
                        physical_size.height,
                    );
                }

                runtime.broadcast(iced_futures::subscription::Event::Interaction {
                    window: id,
                    event: redraw_event.clone(),
                    status: iced_core::event::Status::Ignored,
                });

                ui.draw(
                    &mut window.renderer,
                    &application.theme(id),
                    &iced_core::renderer::Style {
                        text_color: window.state.text_color(),
                    },
                    window.state.cursor(),
                );
                if let user_interface::State::Updated {
                    redraw_request: _, // NOTE: I do not know how to use it now
                    input_method,
                } = ui_state
                {
                    events.push((Some(id), redraw_event.clone()));
                    let need_update_ime = window.request_input_method(input_method.clone());
                    custom_actions.push(LayerShellAction::ImeWithId(
                        oid.expect("id should exist when refreshing"),
                        input_method,
                        need_update_ime,
                    ));
                }
                window.draw_preedit();

                let present_span = debug::present(id);
                if !is_new_window {
                    match compositor.present(
                        &mut window.renderer,
                        &mut window.surface,
                        window.state.viewport(),
                        window.state.background_color(),
                        || {},
                    ) {
                        Ok(()) => {
                            present_span.finish();
                            // TODO:
                        }
                        Err(error) => match error {
                            compositor::SurfaceError::OutOfMemory => {
                                panic!("{:?}", error);
                            }
                            _ => {
                                tracing::error!(
                                    "Error {error:?} when \
                                        presenting surface."
                                );
                            }
                        },
                    }
                }
            }
            MultiWindowIcedLayerEvent(None, IcedLayerEvent::Window(event)) => {
                let Some((_id, window)) = window_manager.first_window() else {
                    continue;
                };
                // NOTE: just follow the other events
                if let Some(event) = conversion::window_event(
                    &event,
                    window.state.application_scale_factor(),
                    window.state.modifiers(),
                ) {
                    events.push((None, event));
                }
            }
            MultiWindowIcedLayerEvent(Some(id), IcedLayerEvent::Window(event)) => {
                let Some((id, window)) = window_manager.get_mut_alias(id) else {
                    continue;
                };
                window.state.update(&event);
                if let Some(event) = conversion::window_event(
                    &event,
                    window.state.application_scale_factor(),
                    window.state.modifiers(),
                ) {
                    events.push((Some(id), event));
                }
            }
            MultiWindowIcedLayerEvent(_, IcedLayerEvent::UserEvent(event)) => {
                let mut cached_user_interfaces: HashMap<window::Id, user_interface::Cache> =
                    ManuallyDrop::into_inner(user_interfaces)
                        .drain()
                        .map(|(id, ui)| (id, ui.into_cache()))
                        .collect();
                run_action(
                    &application,
                    &mut compositor,
                    event,
                    &mut messages,
                    &mut clipboard,
                    &mut custom_actions,
                    &mut waiting_actions,
                    &mut should_exit,
                    &mut window_manager,
                    &mut cached_user_interfaces,
                );
                user_interfaces = ManuallyDrop::new(build_user_interfaces(
                    &application,
                    &mut window_manager,
                    cached_user_interfaces,
                ));
                if should_exit {
                    break;
                }
            }
            MultiWindowIcedLayerEvent(_, IcedLayerEvent::NormalUpdate) => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }

                let mut window_refresh_events = vec![];
                let mut uis_stale = false;
                let mut has_window_event = false;
                for (id, window) in window_manager.iter_mut() {
                    let interact_span = debug::interact(id);
                    let mut window_events = vec![];

                    events.retain(|(window_id, event)| {
                        if *window_id == Some(id) || window_id.is_none() {
                            window_events.push(event.clone());
                            false
                        } else {
                            true
                        }
                    });

                    if window_events.is_empty() && messages.is_empty() {
                        continue;
                    }

                    if !window_events.is_empty() {
                        has_window_event = true;
                    }

                    let (ui_state, statuses) = user_interfaces
                        .get_mut(&id)
                        .expect("Get user interface")
                        .update(
                            &window_events,
                            window.state.cursor(),
                            &mut window.renderer,
                            &mut clipboard,
                            &mut messages,
                        );

                    window_refresh_events.push(LayerShellAction::RedrawWindow(window.id));
                    if !uis_stale {
                        uis_stale = matches!(ui_state, user_interface::State::Outdated);
                    }

                    for (event, status) in window_events.drain(..).zip(statuses.into_iter()) {
                        runtime.broadcast(iced_futures::subscription::Event::Interaction {
                            window: id,
                            event,
                            status,
                        });
                    }
                    interact_span.finish();
                }

                let mut already_redraw_all = false;
                if has_window_event {
                    custom_actions.push(LayerShellAction::RedrawAll);
                    already_redraw_all = true;
                }

                // TODO mw application update returns which window IDs to update
                if !messages.is_empty() || uis_stale {
                    let cached_user_interfaces: HashMap<window::Id, user_interface::Cache> =
                        ManuallyDrop::into_inner(user_interfaces)
                            .drain()
                            .map(|(id, ui)| (id, ui.into_cache()))
                            .collect();

                    // Update application
                    update(&mut application, &mut runtime, &mut messages);

                    for (_id, window) in window_manager.iter_mut() {
                        window.state.synchronize(&application);
                    }
                    debug::theme_changed(|| {
                        window_manager
                            .first()
                            .and_then(|window| theme::Base::palette(window.state.theme()))
                    });
                    user_interfaces = ManuallyDrop::new(build_user_interfaces(
                        &application,
                        &mut window_manager,
                        cached_user_interfaces,
                    ));
                }

                // NOTE: only append the target window refresh event when not invoke the redrawAll
                // event. This will make the events fewer.
                if !already_redraw_all {
                    custom_actions.append(&mut window_refresh_events);
                }
            }
            MultiWindowIcedLayerEvent(_, IcedLayerEvent::WindowRemoved(id)) => {
                let mut cached_user_interfaces: HashMap<window::Id, user_interface::Cache> =
                    ManuallyDrop::into_inner(user_interfaces)
                        .drain()
                        .map(|(id, ui)| (id, ui.into_cache()))
                        .collect();

                application.remove_id(id);
                cached_layer_dimensions.remove(&id);
                window_manager.remove(id);
                cached_user_interfaces.remove(&id);
                user_interfaces = ManuallyDrop::new(build_user_interfaces(
                    &application,
                    &mut window_manager,
                    cached_user_interfaces,
                ));
                runtime.broadcast(iced_futures::subscription::Event::Interaction {
                    window: id,
                    event: Event::Window(window::Event::Closed),
                    status: iced_core::event::Status::Ignored,
                });
                // if now there is no windows now, then break the compositor, and unlink the clipboard
                if window_manager.is_empty() {
                    compositor = None;
                    clipboard = LayerShellClipboard::unconnected();
                }
            }
            MultiWindowIcedLayerEvent(
                Some(id),
                IcedLayerEvent::NewMenu((
                    IcedNewMenuSettings {
                        size: (width, height),
                        direction,
                    },
                    info,
                )),
            ) => {
                let Some((_, window)) = window_manager.get_alias(id) else {
                    continue;
                };

                let Some(point) = window.state.mouse_position() else {
                    continue;
                };

                let (x, mut y) = (point.x as i32, point.y as i32);
                if let MenuDirection::Up = direction {
                    y -= height as i32;
                }
                custom_actions.push(LayerShellAction::NewMenu(
                    IcedNewPopupSettings {
                        size: (width, height),
                        position: (x, y),
                    },
                    info,
                ));
            }
            _ => {}
        }
        let mut copyactions = vec![];
        std::mem::swap(&mut copyactions, &mut custom_actions);
        control_sender.start_send(copyactions).ok();
    }
    let _ = ManuallyDrop::into_inner(user_interfaces);
}

#[allow(clippy::type_complexity)]
pub fn build_user_interfaces<'a, A: Application, C>(
    application: &'a A,
    window_manager: &mut WindowManager<A, C>,
    mut cached_user_interfaces: HashMap<iced::window::Id, user_interface::Cache>,
) -> HashMap<iced::window::Id, UserInterface<'a, A::Message, A::Theme, A::Renderer>>
where
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    cached_user_interfaces
        .drain()
        .filter_map(|(id, cache)| {
            let window = window_manager.get_mut(id)?;

            Some((
                id,
                build_user_interface(
                    application,
                    cache,
                    &mut window.renderer,
                    window.state.viewport().logical_size(),
                    id,
                ),
            ))
        })
        .collect()
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    id: iced::window::Id,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    A::Theme: DefaultStyle,
{
    let view_span = debug::view(id);
    let view = application.view(id);
    view_span.finish();

    let layout_span = debug::layout(id);
    let user_interface = UserInterface::build(view, size, cache, renderer);
    layout_span.finish();
    user_interface
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update<A: Application, E: Executor>(
    application: &mut A,
    runtime: &mut MultiRuntime<E, A::Message>,
    messages: &mut Vec<A::Message>,
) where
    A::Theme: DefaultStyle,
    A::Message: 'static,
{
    for message in messages.drain(..) {
        let update_span = debug::update(&message);
        let task = runtime.enter(|| application.update(message));
        debug::tasks_spawned(task.units());
        update_span.finish();

        if let Some(stream) = iced_runtime::task::into_stream(task) {
            runtime.run(stream);
        }
    }

    let subscription = runtime.enter(|| application.subscription());
    let recipes = iced_futures::subscription::into_recipes(subscription.map(Action::Output));

    debug::subscriptions_tracked(recipes.len());
    runtime.track(recipes);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_action<A, C>(
    application: &A,
    compositor: &mut Option<C>,
    event: Action<A::Message>,
    messages: &mut Vec<A::Message>,
    clipboard: &mut LayerShellClipboard,
    custom_actions: &mut Vec<LayerShellAction>,
    waiting_actions: &mut Vec<(iced::window::Id, LayershellCustomActions)>,
    should_exit: &mut bool,
    window_manager: &mut WindowManager<A, C>,
    cached_user_interfaces: &mut HashMap<iced::window::Id, user_interface::Cache>,
) where
    A: Application,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<LayershellCustomActionsWithId, Error = A::Message>,
{
    use iced_core::widget::operation;
    use iced_runtime::Action;
    use iced_runtime::clipboard;

    use iced_runtime::window::Action as WindowAction;
    match event {
        Action::Output(stream) => match stream.try_into() {
            Ok(action) => {
                let LayershellCustomActionsWithId(id, custom_action) = action;

                if let Some(id) = id {
                    if window_manager.get_layer_id(id).is_none() {
                        waiting_actions.push((id, custom_action));
                        return;
                    }
                }

                let option_id = if let LayershellCustomActions::RemoveWindow(id) = custom_action {
                    let option_id = window_manager.get_layer_id(id);
                    if option_id.is_none() {
                        return;
                    }
                    option_id
                } else {
                    None
                };
                custom_actions.push(LayerShellAction::CustomActionsWithId(
                    LayershellCustomActionsWithIdInner(
                        id.and_then(|id| window_manager.get_layer_id(id)),
                        option_id,
                        custom_action,
                    ),
                ));
            }
            Err(stream) => {
                messages.push(stream);
            }
        },

        Action::Clipboard(action) => match action {
            clipboard::Action::Read { target, channel } => {
                let _ = channel.send(clipboard.read(target));
            }
            clipboard::Action::Write { target, contents } => {
                clipboard.write(target, contents);
            }
        },
        Action::Widget(action) => {
            let mut current_operation = Some(action);

            let mut uis = build_user_interfaces(
                application,
                window_manager,
                std::mem::take(cached_user_interfaces),
            );

            'operate: while let Some(mut operation) = current_operation.take() {
                for (id, ui) in uis.iter_mut() {
                    if let Some(window) = window_manager.get_mut(*id) {
                        ui.operate(&window.renderer, operation.as_mut());

                        match operation.finish() {
                            operation::Outcome::None => {}
                            operation::Outcome::Some(_message) => {
                                //proxy.send_event(message).ok();

                                // operation completed, don't need to try to operate on rest of UIs
                                break 'operate;
                            }
                            operation::Outcome::Chain(next) => {
                                current_operation = Some(next);
                            }
                        }
                    }
                }
            }

            *cached_user_interfaces = uis.drain().map(|(id, ui)| (id, ui.into_cache())).collect();
        }
        Action::Window(action) => match action {
            WindowAction::Close(id) => {
                if let Some(layerid) = window_manager.get_layer_id(id) {
                    custom_actions.push(LayerShellAction::CustomActionsWithId(
                        LayershellCustomActionsWithIdInner(
                            Some(layerid),
                            Some(layerid),
                            LayershellCustomActions::RemoveWindow(id),
                        ),
                    ))
                }
            }
            WindowAction::GetSize(id, channel) => 'out: {
                let Some(window) = window_manager.get(id) else {
                    break 'out;
                };
                let _ = channel.send(window.state.window_size_f32());
            }
            WindowAction::Screenshot(id, channel) => 'out: {
                let Some(window) = window_manager.get_mut(id) else {
                    break 'out;
                };
                let Some(compositor) = compositor else {
                    break 'out;
                };
                let bytes = compositor.screenshot(
                    &mut window.renderer,
                    window.state.viewport(),
                    window.state.background_color(),
                );

                let _ = channel.send(iced_core::window::Screenshot::new(
                    bytes,
                    window.state.viewport().physical_size(),
                    window.state.viewport().scale_factor(),
                ));
            }
            WindowAction::GetScaleFactor(id, channel) => {
                if let Some(window) = window_manager.get_mut(id) {
                    let _ = channel.send(window.state.wayland_scale_factor() as f32);
                };
            }
            _ => {}
        },
        Action::Exit => {
            *should_exit = true;
        }
        Action::LoadFont { bytes, channel } => {
            if let Some(compositor) = compositor {
                // TODO: Error handling (?)
                compositor.load_font(bytes.clone());

                let _ = channel.send(Ok(()));
            }
        }
        _ => {}
    }
}
