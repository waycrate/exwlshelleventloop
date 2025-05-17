mod state;

use std::{borrow::Cow, mem::ManuallyDrop, os::fd::AsFd, sync::Arc, time::Duration};

use crate::{
    actions::{LayerShellAction, LayerShellActionVec, LayershellCustomActions},
    clipboard::LayerShellClipboard,
    conversion,
    error::Error,
    ime_preedit::{ImeState, Preedit},
    settings::VirtualKeyboardSettings,
};

use super::DefaultStyle;
use enumflags2::{BitFlag, BitFlags};
use iced_graphics::{Compositor, compositor};
use state::State;

use iced_core::{
    Event as IcedCoreEvent, InputMethod, Size, input_method, time::Instant,
    window as IcedCoreWindow,
};

use iced_runtime::{Action, UserInterface, user_interface};

use iced_futures::{Executor, Runtime};

use layershellev::{
    LayerEvent, ReturnData, StartMode, WindowWrapper,
    calloop::timer::{TimeoutAction, Timer},
    reexport::wayland_client::{WlCompositor, WlRegion},
    reexport::zwp_virtual_keyboard_v1,
};

use futures::{StreamExt, channel::mpsc};
use iced::theme;
use iced_runtime::debug;

use crate::{actions::ActionCallback, event::IcedLayerEvent, proxy::IcedProxy, settings::Settings};

type SingleRuntime<E, Message> = Runtime<E, IcedProxy<Action<Message>>, Action<Message>>;
use iced_program::Program as IcedProgram;

use iced_program::Instance;

// a dispatch loop, another is listen loop
pub fn run<P>(
    program: P,
    namespace: &str,
    settings: Settings,
    compositor_settings: iced_graphics::Settings,
) -> Result<(), Error>
where
    P: IcedProgram + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<LayershellCustomActions, Error = P::Message>,
{
    use futures::Future;
    use futures::task;

    let (message_sender, message_receiver) = std::sync::mpsc::channel::<Action<P::Message>>();

    let boot_span = debug::boot();
    let proxy = IcedProxy::new(message_sender);
    let mut runtime: SingleRuntime<P::Executor, P::Message> = {
        let executor = P::Executor::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, task) = runtime.enter(|| Instance::new(program));

    assert!(!matches!(
        settings.layer_settings.start_mode,
        StartMode::AllScreens | StartMode::Background
    ));

    let ev = layershellev::WindowStateSimple::new(namespace)
        .with_use_display_handle(true)
        .with_option_size(settings.layer_settings.size)
        .with_layer(settings.layer_settings.layer)
        .with_events_transparent(settings.layer_settings.events_transparent)
        .with_anchor(settings.layer_settings.anchor)
        .with_exclusive_zone(settings.layer_settings.exclusive_zone)
        .with_margin(settings.layer_settings.margin)
        .with_keyboard_interacivity(settings.layer_settings.keyboard_interactivity)
        .with_start_mode(settings.layer_settings.start_mode)
        .build()
        .expect("Cannot create layershell");

    let window = Arc::new(ev.gen_mainwindow_wrapper());

    if let Some(stream) = iced_runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(iced_futures::subscription::into_recipes(
        runtime.enter(|| application.subscription().map(Action::Output)),
    ));
    let main_id = IcedCoreWindow::Id::unique();

    let state = State::new(&application, &ev, main_id);

    let (mut event_sender, event_receiver) =
        mpsc::unbounded::<IcedLayerEvent<Action<P::Message>>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded::<LayerShellActionVec>();

    let mut instance = Box::pin(run_instance::<
        P,
        P::Executor,
        <P::Renderer as iced_graphics::compositor::Default>::Compositor,
    >(
        application,
        compositor_settings,
        runtime,
        event_receiver,
        control_sender,
        state,
        main_id,
        window,
        settings.fonts,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());
    boot_span.finish();
    let mut wl_input_region: Option<WlRegion> = None;

    let _ = ev.running_with_proxy(message_receiver, move |event, ev, _| {
        let mut def_returndata = ReturnData::None;
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
            LayerEvent::RequestMessages(message) => {
                event_sender
                    .start_send(message.into())
                    .expect("Cannot send");
            }
            LayerEvent::NormalDispatch => {
                event_sender
                    .start_send(IcedLayerEvent::NormalUpdate)
                    .expect("Cannot send");
            }
            LayerEvent::UserEvent(event) => {
                event_sender
                    .start_send(IcedLayerEvent::UserEvent(event))
                    .ok();
            }
            LayerEvent::WindowClosed => {
                event_sender
                    // there is only one window, id doesn't matter.
                    .start_send(IcedLayerEvent::WindowRemoved(main_id))
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
                LayerShellAction::CustomActions(action) => match action {
                    LayershellCustomActions::AnchorChange(anchor) => {
                        ev.main_window().set_anchor(anchor);
                    }
                    LayershellCustomActions::AnchorSizeChange(anchor, size) => {
                        ev.main_window().set_anchor_with_size(anchor, size);
                    }
                    LayershellCustomActions::LayerChange(layer) => {
                        ev.main_window().set_layer(layer);
                    }
                    LayershellCustomActions::SetInputRegion(ActionCallback(set_region)) => {
                        let window = ev.main_window();

                        let region = wl_input_region.as_ref().expect("region not found");
                        let window_size = window.get_size();
                        let width: i32 = window_size.0.try_into().unwrap_or_default();
                        let height: i32 = window_size.1.try_into().unwrap_or_default();

                        region.subtract(0, 0, width, height);
                        set_region(region);

                        window.get_wlsurface().set_input_region(Some(region));
                    }
                    LayershellCustomActions::MarginChange(margin) => {
                        ev.main_window().set_margin(margin);
                    }
                    LayershellCustomActions::SizeChange((width, height)) => {
                        ev.main_window().set_size((width, height));
                    }
                    LayershellCustomActions::ExclusiveZoneChange(zone_size) => {
                        ev.main_window().set_exclusive_zone(zone_size);
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
                    _ => {}
                },
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
                LayerShellAction::Ime(ime, ime_flags) => match ime {
                    iced_core::InputMethod::Disabled => {
                        if ime_flags.contains(ImeState::Disabled) {
                            ev.set_ime_allowed(false);
                        }
                    }
                    iced_core::InputMethod::Enabled {
                        position, purpose, ..
                    } => {
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
                                ev.main_window().id(),
                            );
                        }
                    }
                },
                _ => {}
            }
        }
        def_returndata
    });
    Ok(())
}

struct IMDrawer<A>
where
    A: IcedProgram,
    A::Theme: iced_core::theme::Base,
{
    preedit: Option<Preedit<A::Renderer>>,
    ime_state: Option<(iced_core::Point, input_method::Purpose)>,
}

impl<A> IMDrawer<A>
where
    A: IcedProgram,
    A::Theme: iced_core::theme::Base,
{
    fn new() -> Self {
        Self {
            preedit: None,
            ime_state: None,
        }
    }
    pub fn request_input_method(
        &mut self,
        background_color: iced_core::Color,
        input_method: InputMethod,
        renderer: &A::Renderer,
    ) -> BitFlags<ImeState> {
        match input_method {
            InputMethod::Disabled => self.disable_ime(),
            InputMethod::Enabled {
                position,
                purpose,
                preedit,
            } => {
                let mut flags = ImeState::empty();
                if self.ime_state.is_none() {
                    flags.insert(ImeState::Allowed);
                }
                if self.ime_state != Some((position, purpose)) {
                    flags.insert(ImeState::Update);
                }
                self.update_ime(position, purpose);

                if let Some(preedit) = preedit {
                    if preedit.content.is_empty() {
                        self.preedit = None;
                    } else {
                        let mut overlay = self.preedit.take().unwrap_or_else(Preedit::new);

                        overlay.update(position, &preedit, background_color, renderer);

                        self.preedit = Some(overlay);
                    }
                } else {
                    self.preedit = None;
                }
                flags
            }
        }
    }

    pub fn draw_preedit(
        &mut self,
        renderer: &mut A::Renderer,
        text_color: iced_core::Color,
        background_color: iced_core::Color,
        logical_size: iced_core::Size,
    ) {
        use iced_core::Point;
        use iced_core::Rectangle;
        if let Some(preedit) = &self.preedit {
            preedit.draw(
                renderer,
                text_color,
                background_color,
                &Rectangle::new(Point::ORIGIN, logical_size),
            );
        }
    }

    fn update_ime(&mut self, position: iced_core::Point, purpose: input_method::Purpose) {
        if self.ime_state != Some((position, purpose)) {
            self.ime_state = Some((position, purpose));
        }
    }
    fn disable_ime(&mut self) -> BitFlags<ImeState> {
        let flags = if self.ime_state.is_some() {
            ImeState::Disabled.into()
        } else {
            ImeState::empty()
        };
        if self.ime_state.is_some() {
            self.ime_state = None;
        }

        self.preedit = None;
        flags
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: Instance<A>,
    compositor_settings: iced_graphics::Settings,
    mut runtime: SingleRuntime<E, A::Message>,
    mut event_receiver: mpsc::UnboundedReceiver<IcedLayerEvent<Action<A::Message>>>,
    mut control_sender: mpsc::UnboundedSender<LayerShellActionVec>,
    mut state: State<A>,
    main_id: iced::window::Id,
    window: Arc<WindowWrapper>,
    fonts: Vec<Cow<'static, [u8]>>,
) where
    A: IcedProgram + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<LayershellCustomActions, Error = A::Message>,
{
    use iced_core::Event;

    let mut compositor = C::new(compositor_settings, window.clone())
        .await
        .expect("Cannot create compositor");
    for font in fonts {
        compositor.load_font(font);
    }

    let mut renderer = compositor.create_renderer();
    let mut im_drawer: IMDrawer<A> = IMDrawer::new();

    let cache = user_interface::Cache::default();

    // HACK: the surface size should not be set as 0, 0
    // but it will changed later
    // so here set it to 1, 1
    let mut surface = compositor.create_surface(window.clone(), 1, 1);

    let mut should_exit = false;

    let mut clipboard = LayerShellClipboard::connect(&window);

    let mut messages = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut custom_actions = Vec::new();

    let mut user_interface = ManuallyDrop::new(build_user_interface(
        &application,
        cache,
        &mut renderer,
        state.viewport().logical_size(),
        main_id,
    ));

    let mut p_width: u32 = 0;
    let mut p_height: u32 = 0;
    let mut p_fractal_scale: f64 = 0.;

    while let Some(event) = event_receiver.next().await {
        match event {
            IcedLayerEvent::RequestRefresh {
                width,
                height,
                fractal_scale,
            } => {
                if p_width != width || p_height != height || p_fractal_scale != fractal_scale {
                    p_width = width;
                    p_height = height;
                    p_fractal_scale = fractal_scale;

                    let layout_span = debug::layout(main_id);
                    state.update_view_port(width, height, fractal_scale);

                    user_interface = ManuallyDrop::new(
                        ManuallyDrop::into_inner(user_interface)
                            .relayout(state.viewport().logical_size(), &mut renderer),
                    );
                    layout_span.finish();
                    debug::theme_changed(|| theme::Base::palette(&application.theme(main_id)));

                    let physical_size = state.viewport().physical_size();
                    compositor.configure_surface(
                        &mut surface,
                        physical_size.width,
                        physical_size.height,
                    );
                }

                let redraw_event =
                    IcedCoreEvent::Window(IcedCoreWindow::Event::RedrawRequested(Instant::now()));

                let (ui_state, _) = user_interface.update(
                    &[redraw_event.clone()],
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );

                let draw_span = debug::draw(main_id);
                runtime.broadcast(iced_futures::subscription::Event::Interaction {
                    window: main_id,
                    event: redraw_event.clone(),
                    status: iced_core::event::Status::Ignored,
                });

                user_interface.draw(
                    &mut renderer,
                    state.theme(),
                    &iced_core::renderer::Style {
                        text_color: state.text_color(),
                    },
                    state.cursor(),
                );

                draw_span.finish();
                if let user_interface::State::Updated {
                    redraw_request: _, // NOTE: I do not know how to use it now
                    input_method,
                    mouse_interaction,
                } = ui_state
                {
                    custom_actions.push(LayerShellAction::Mouse(mouse_interaction));
                    events.push(redraw_event);

                    let ime_flags = im_drawer.request_input_method(
                        state.background_color(),
                        input_method.clone(),
                        &renderer,
                    );
                    custom_actions.push(LayerShellAction::Ime(input_method, ime_flags));
                }
                im_drawer.draw_preedit(
                    &mut renderer,
                    state.text_color(),
                    state.background_color(),
                    state.viewport().logical_size(),
                );
                match compositor.present(
                    &mut renderer,
                    &mut surface,
                    state.viewport(),
                    state.background_color(),
                    || {},
                ) {
                    Ok(()) => {
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
            IcedLayerEvent::Window(event) => {
                state.update(&event);

                if let Some(event) = conversion::window_event(
                    &event,
                    state.application_scale_factor(),
                    state.modifiers(),
                ) {
                    events.push(event);
                }
            }
            IcedLayerEvent::UserEvent(event) => {
                let mut cache = ManuallyDrop::into_inner(user_interface).into_cache();
                run_action(
                    &application,
                    &mut compositor,
                    &mut cache,
                    &state,
                    &mut renderer,
                    event,
                    &mut messages,
                    &mut clipboard,
                    &mut custom_actions,
                    &mut should_exit,
                    main_id,
                );
                user_interface = ManuallyDrop::new(build_user_interface(
                    &application,
                    cache,
                    &mut renderer,
                    state.viewport().logical_size(),
                    main_id,
                ));
                if should_exit {
                    break;
                }
            }
            IcedLayerEvent::NormalUpdate => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }
                #[cfg(not(feature = "unconditional-rendering"))]
                let mut is_updated = false;
                #[cfg(feature = "unconditional-rendering")]
                let is_updated = false;
                let interact_span = debug::interact(main_id);
                let (ui_state, statuses) = user_interface.update(
                    &events,
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );
                interact_span.finish();

                for (event, status) in events.drain(..).zip(statuses.into_iter()) {
                    runtime.broadcast(iced_futures::subscription::Event::Interaction {
                        window: main_id,
                        event,
                        status,
                    });
                }

                #[cfg(feature = "unconditional-rendering")]
                custom_actions.push(LayerShellAction::RedrawAll);

                match ui_state {
                    user_interface::State::Updated {
                        redraw_request,
                        mouse_interaction,
                        ..
                    } => {
                        custom_actions.push(LayerShellAction::Mouse(mouse_interaction));

                        #[cfg(not(feature = "unconditional-rendering"))]
                        // TODO: now just do when receive NextFrame
                        if matches!(redraw_request, IcedCoreWindow::RedrawRequest::NextFrame) {
                            custom_actions.push(LayerShellAction::RedrawAll);
                            is_updated = true;
                        }
                    }
                    user_interface::State::Outdated => {}
                }

                if !messages.is_empty() || matches!(ui_state, user_interface::State::Outdated) {
                    let cache = ManuallyDrop::into_inner(user_interface).into_cache();
                    // Update application
                    update(
                        &mut application,
                        &mut state,
                        &mut runtime,
                        &mut messages,
                        main_id,
                    );

                    if !is_updated {
                        custom_actions.push(LayerShellAction::RedrawAll);
                    }
                    debug::theme_changed(|| theme::Base::palette(&application.theme(main_id)));
                    user_interface = ManuallyDrop::new(build_user_interface(
                        &application,
                        cache,
                        &mut renderer,
                        state.viewport().logical_size(),
                        main_id,
                    ));
                }
            }
            IcedLayerEvent::WindowRemoved(_) => {
                should_exit = true;
            }
            _ => unreachable!(),
        }
        let mut copyactions = vec![];
        std::mem::swap(&mut copyactions, &mut custom_actions);
        control_sender.start_send(copyactions).ok();
    }

    drop(ManuallyDrop::into_inner(user_interface));
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, P: IcedProgram>(
    application: &'a Instance<P>,
    cache: user_interface::Cache,
    renderer: &mut P::Renderer,
    size: Size,
    id: IcedCoreWindow::Id,
) -> UserInterface<'a, P::Message, P::Theme, P::Renderer>
where
    P::Theme: DefaultStyle,
{
    let view_span = debug::view(id);
    let view = application.view(id);
    view_span.finish();

    let layout_span = debug::layout(id);
    let user_interface = UserInterface::build(view, size, cache, renderer);
    layout_span.finish();
    user_interface
}

/// Updates an [`Application`] by feeding it the provided messages, spawning any
/// tracking its [`Subscription`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn update<P: IcedProgram, E: Executor>(
    application: &mut Instance<P>,
    state: &mut State<P>,
    runtime: &mut SingleRuntime<E, P::Message>,
    messages: &mut Vec<P::Message>,
    mainid: iced::window::Id,
) where
    P::Theme: DefaultStyle,
    P::Message: 'static,
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
    state.synchronize(application, mainid);

    let subscription = runtime.enter(|| application.subscription());
    let recipes = iced_futures::subscription::into_recipes(subscription.map(Action::Output));

    debug::subscriptions_tracked(recipes.len());
    runtime.track(recipes);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_action<P, C>(
    application: &Instance<P>,
    compositor: &mut C,
    cache: &mut user_interface::Cache,
    state: &State<P>,
    renderer: &mut P::Renderer,
    event: Action<P::Message>,
    messages: &mut Vec<P::Message>,
    clipboard: &mut LayerShellClipboard,
    custom_actions: &mut Vec<LayerShellAction>,
    should_exit: &mut bool,
    id: IcedCoreWindow::Id,
) where
    P: IcedProgram,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<LayershellCustomActions, Error = P::Message>,
{
    use iced_core::widget::operation;
    use iced_runtime::Action;
    use iced_runtime::clipboard;

    use iced_runtime::window::Action as WindowAction;
    match event {
        Action::Output(stream) => match stream.try_into() {
            Ok(action) => custom_actions.push(LayerShellAction::CustomActions(action)),
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
            let mut current_cache = std::mem::take(cache);
            let mut current_operation = Some(action);

            let mut user_interface = build_user_interface(
                application,
                current_cache,
                renderer,
                state.viewport().logical_size(),
                id,
            );

            while let Some(mut operation) = current_operation.take() {
                user_interface.operate(renderer, operation.as_mut());

                match operation.finish() {
                    operation::Outcome::None => {}
                    operation::Outcome::Some(_message) => {
                        // TODO:
                    }
                    operation::Outcome::Chain(next) => {
                        current_operation = Some(next);
                    }
                }
            }

            current_cache = user_interface.into_cache();
            *cache = current_cache;
        }
        Action::Window(action) => match action {
            WindowAction::Close(_) => {
                *should_exit = true;
            }
            WindowAction::GetSize(_, channel) => {
                let _ = channel.send(state.window_size_f32());
            }
            WindowAction::Screenshot(_id, channel) => {
                let bytes =
                    compositor.screenshot(renderer, state.viewport(), state.background_color());
                let _ = channel.send(iced_core::window::Screenshot::new(
                    bytes,
                    state.viewport().physical_size(),
                    state.viewport().scale_factor(),
                ));
            }
            WindowAction::GetScaleFactor(_id, channel) => {
                let _ = channel.send(state.wayland_scale_factor() as f32);
            }
            WindowAction::GetLatest(channel) => {
                let _ = channel.send(Some(id));
            }
            _ => {}
        },
        Action::Exit => {
            *should_exit = true;
        }
        Action::LoadFont { bytes, channel } => {
            // TODO: Error handling (?)
            compositor.load_font(bytes.clone());

            let _ = channel.send(Ok(()));
        }
        _ => {}
    }
}
