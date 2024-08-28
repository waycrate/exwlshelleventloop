mod state;

use std::{mem::ManuallyDrop, os::fd::AsFd, sync::Arc, time::Duration};

use crate::{
    actions::{LayerShellActions, LayershellCustomActionsWithInfo},
    clipboard::LayerShellClipboard,
    conversion,
    error::Error,
    settings::VirtualKeyboardSettings,
};

use iced_graphics::Compositor;
use state::State;

use iced_core::{time::Instant, window as IcedCoreWindow, Event as IcedCoreEvent, Size};

use iced_runtime::{user_interface, Command, Debug, Program, UserInterface};

use iced_style::application::StyleSheet;

use iced_futures::{Executor, Runtime, Subscription};

use layershellev::{
    calloop::timer::{TimeoutAction, Timer},
    reexport::zwp_virtual_keyboard_v1,
    LayerEvent, ReturnData, WindowWrapper,
};

use futures::{channel::mpsc, SinkExt, StreamExt};

use crate::{event::IcedLayerEvent, proxy::IcedProxy, settings::Settings};

/// An interactive, native cross-platform application.
///
/// This trait is the main entrypoint of Iced. Once implemented, you can run
/// your GUI application by simply calling [`run`]. It will run in
/// its own window.
///
/// An [`Application`] can execute asynchronous actions by returning a
/// [`Command`] in some of its methods.
///
/// When using an [`Application`] with the `debug` feature enabled, a debug view
/// can be toggled by pressing `F12`.
pub trait Application: Program
where
    Self::Theme: StyleSheet,
{
    /// The data needed to initialize your [`Application`].
    type Flags;

    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Command`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>);

    fn namespace(&self) -> String;
    /// Returns the current title of the [`Application`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn title(&self) -> String {
        self.namespace()
    }

    /// Returns the current `Theme` of the [`Application`].
    fn theme(&self) -> Self::Theme;

    /// Returns the `Style` variation of the `Theme`.
    fn style(&self) -> <Self::Theme as StyleSheet>::Style {
        Default::default()
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
    fn scale_factor(&self) -> f64 {
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

// a dispatch loop, another is listen loop
pub fn run<A, E, C>(
    settings: Settings<A::Flags>,
    compositor_settings: C::Settings,
) -> Result<(), Error>
where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use futures::task;
    use futures::Future;

    let mut debug = Debug::new();
    debug.startup_started();

    let (message_sender, message_receiver) = std::sync::mpsc::channel::<A::Message>();

    let proxy = IcedProxy::new(message_sender);
    let runtime: Runtime<E, IcedProxy<A::Message>, <A as Program>::Message> = {
        let executor = E::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy.clone())
    };

    let (application, init_command) = {
        let flags = settings.flags;

        runtime.enter(|| A::new(flags))
    };

    let ev = layershellev::WindowStateSimple::new(&application.namespace())
        .with_single(true)
        .with_use_display_handle(true)
        .with_option_size(settings.layer_settings.size)
        .with_layer(settings.layer_settings.layer)
        .with_anchor(settings.layer_settings.anchor)
        .with_exclusize_zone(settings.layer_settings.exclusive_zone)
        .with_margin(settings.layer_settings.margin)
        .with_keyboard_interacivity(settings.layer_settings.keyboard_interactivity)
        .with_xdg_output_name(settings.layer_settings.binded_output_name)
        .build()
        .expect("Cannot create layershell");

    let window = Arc::new(ev.gen_main_wrapper());
    let compositor = C::new(compositor_settings, window.clone())?;
    let mut renderer = compositor.create_renderer();

    for font in settings.fonts {
        use iced_core::text::Renderer;

        renderer.load_font(font);
    }

    let state = State::new(&application, &ev);

    let (mut event_sender, event_receiver) = mpsc::unbounded::<IcedLayerEvent<A::Message, ()>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded::<Vec<LayerShellActions<()>>>();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        renderer,
        runtime,
        proxy,
        debug,
        event_receiver,
        control_sender,
        state,
        init_command,
        window.clone(),
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    let mut pointer_serial: u32 = 0;

    let _ = ev.running_with_proxy(message_receiver, move |event, ev, _| {
        use layershellev::DispatchMessage;
        let mut def_returndata = ReturnData::None;
        match event {
            LayerEvent::InitRequest => {
                if settings.virtual_keyboard_support.is_some() {
                    def_returndata = ReturnData::RequestBind;
                }
            }
            LayerEvent::BindProvide(globals, qh) => {
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
            LayerEvent::RequestMessages(message) => {
                if let DispatchMessage::MouseEnter { serial, .. } = message {
                    pointer_serial = *serial;
                }

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
            _ => {}
        }
        let poll = instance.as_mut().poll(&mut context);
        match poll {
            task::Poll::Pending => {
                let Ok(Some(flow)) = control_receiver.try_next() else {
                    return def_returndata;
                };
                for flow in flow {
                    match flow {
                        LayerShellActions::CustomActions(actions) => {
                            for action in actions {
                                match action {
                                    LayershellCustomActionsWithInfo::AnchorChange(anchor) => {
                                        ev.main_window().set_anchor(anchor);
                                    }
                                    LayershellCustomActionsWithInfo::LayerChange(layer) => {
                                        ev.main_window().set_layer(layer);
                                    }
                                    LayershellCustomActionsWithInfo::MarginChange(margin) => {
                                        ev.main_window().set_margin(margin);
                                    }
                                    LayershellCustomActionsWithInfo::SizeChange((
                                        width,
                                        height,
                                    )) => {
                                        ev.main_window().set_size((width, height));
                                    }
                                    LayershellCustomActionsWithInfo::VirtualKeyboardPressed {
                                        time,
                                        key,
                                    } => {
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
                                }
                            }
                        }
                        LayerShellActions::Mouse(mouse) => {
                            let Some(pointer) = ev.get_pointer() else {
                                return ReturnData::None;
                            };

                            ev.append_return_data(ReturnData::RequestSetCursorShape((
                                conversion::mouse_interaction(mouse),
                                pointer.clone(),
                                pointer_serial,
                            )));
                        }
                        LayerShellActions::RedrawAll => {
                            ev.append_return_data(ReturnData::RedrawAllRequest);
                        }
                        LayerShellActions::RedrawWindow(index) => {
                            ev.append_return_data(ReturnData::RedrawIndexRequest(index));
                        }
                        _ => {}
                    }
                }
                def_returndata
            }
            task::Poll::Ready(_) => ReturnData::RequestExit,
        }
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut renderer: A::Renderer,
    mut runtime: Runtime<E, IcedProxy<A::Message>, A::Message>,
    mut proxy: IcedProxy<A::Message>,
    mut debug: Debug,
    mut event_receiver: mpsc::UnboundedReceiver<IcedLayerEvent<A::Message, ()>>,
    mut control_sender: mpsc::UnboundedSender<Vec<LayerShellActions<()>>>,
    mut state: State<A>,
    init_command: Command<A::Message>,
    window: Arc<WindowWrapper>,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use iced_core::mouse;
    use iced_core::Event;
    let physical_size = state.physical_size();
    let mut cache = user_interface::Cache::default();
    let mut surface =
        compositor.create_surface(window.clone(), physical_size.width, physical_size.height);

    let mut should_exit = false;

    let mut clipboard = LayerShellClipboard::connect(&window);

    let mut mouse_interaction = mouse::Interaction::default();
    let mut messages = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut custom_actions = Vec::new();

    run_command(
        &application,
        &mut compositor,
        &mut surface,
        &mut cache,
        &state,
        &mut renderer,
        init_command,
        &mut runtime,
        &mut clipboard,
        &mut custom_actions,
        &mut should_exit,
        &mut proxy,
        &mut debug,
    );

    runtime.track(application.subscription().into_recipes());

    let mut user_interface = ManuallyDrop::new(build_user_interface(
        &application,
        cache,
        &mut renderer,
        state.logical_size(),
        &mut debug,
    ));

    debug.startup_finished();

    while let Some(event) = event_receiver.next().await {
        match event {
            IcedLayerEvent::RequestRefresh { width, height } => {
                state.update_view_port(width, height);
                let ps = state.physical_size();
                let width = ps.width;
                let height = ps.height;
                //state.update_view_port(width, height);
                debug.layout_started();
                user_interface =
                    ManuallyDrop::new(ManuallyDrop::into_inner(user_interface).relayout(
                        Size {
                            width: width as f32,
                            height: height as f32,
                        },
                        &mut renderer,
                    ));
                debug.layout_finished();

                compositor.configure_surface(&mut surface, width, height);
                let redraw_event = IcedCoreEvent::Window(
                    IcedCoreWindow::Id::MAIN,
                    IcedCoreWindow::Event::RedrawRequested(Instant::now()),
                );

                user_interface.update(
                    &[redraw_event.clone()],
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );
                runtime.broadcast(redraw_event, iced_core::event::Status::Ignored);

                debug.draw_started();
                let new_mouse_interaction = user_interface.draw(
                    &mut renderer,
                    state.theme(),
                    &iced_core::renderer::Style {
                        text_color: state.text_color(),
                    },
                    state.cursor(),
                );
                debug.draw_finished();

                if new_mouse_interaction != mouse_interaction {
                    custom_actions.push(LayerShellActions::Mouse(new_mouse_interaction));
                    mouse_interaction = new_mouse_interaction;
                }
                // TODO: check mouse_interaction

                debug.render_started();

                debug.draw_started();
                user_interface.draw(
                    &mut renderer,
                    &application.theme(),
                    &iced_core::renderer::Style {
                        text_color: state.text_color(),
                    },
                    state.cursor(),
                );
                debug.draw_finished();
                compositor
                    .present(
                        &mut renderer,
                        &mut surface,
                        state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    )
                    .ok();
                debug.render_finished();
            }
            IcedLayerEvent::Window(event) => {
                state.update(&event);

                if let Some(event) =
                    conversion::window_event(IcedCoreWindow::Id::MAIN, &event, state.modifiers())
                {
                    events.push(event);
                }
            }
            IcedLayerEvent::UserEvent(event) => {
                messages.push(event);
            }
            IcedLayerEvent::NormalUpdate => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }
                debug.event_processing_started();
                let (interface_state, statuses) = user_interface.update(
                    &events,
                    state.cursor(),
                    &mut renderer,
                    &mut clipboard,
                    &mut messages,
                );
                debug.event_processing_finished();

                for (event, status) in events.drain(..).zip(statuses.into_iter()) {
                    runtime.broadcast(event, status);
                }

                if !messages.is_empty()
                    || matches!(interface_state, user_interface::State::Outdated)
                {
                    let mut cache = ManuallyDrop::into_inner(user_interface).into_cache();

                    // Update application
                    update(
                        &mut application,
                        &mut compositor,
                        &mut surface,
                        &mut cache,
                        &mut state,
                        &mut renderer,
                        &mut runtime,
                        &mut clipboard,
                        &mut should_exit,
                        &mut proxy,
                        &mut debug,
                        &mut messages,
                        &mut custom_actions,
                    );

                    user_interface = ManuallyDrop::new(build_user_interface(
                        &application,
                        cache,
                        &mut renderer,
                        state.logical_size(),
                        &mut debug,
                    ));

                    if should_exit {
                        break;
                    }
                }
                custom_actions.push(LayerShellActions::RedrawAll);
            }
            _ => unreachable!(),
        }
        control_sender.start_send(custom_actions.clone()).ok();
        custom_actions.clear();
    }

    drop(ManuallyDrop::into_inner(user_interface));
}

/// Builds a [`UserInterface`] for the provided [`Application`], logging
/// [`struct@Debug`] information accordingly.
pub fn build_user_interface<'a, A: Application>(
    application: &'a A,
    cache: user_interface::Cache,
    renderer: &mut A::Renderer,
    size: Size,
    debug: &mut Debug,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    A::Theme: StyleSheet,
{
    debug.view_started();
    let view = application.view();
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();
    user_interface
}

/// Updates an [`Application`] by feeding it the provided messages, spawning any
/// resulting [`Command`], and tracking its [`Subscription`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn update<A: Application, C, E: Executor>(
    application: &mut A,
    compositor: &mut C,
    surface: &mut C::Surface,
    cache: &mut user_interface::Cache,
    state: &mut State<A>,
    renderer: &mut A::Renderer,
    runtime: &mut Runtime<E, IcedProxy<A::Message>, A::Message>,
    clipboard: &mut LayerShellClipboard,
    should_exit: &mut bool,
    proxy: &mut IcedProxy<A::Message>,
    debug: &mut Debug,
    messages: &mut Vec<A::Message>,
    custom_actions: &mut Vec<LayerShellActions<()>>,
) where
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
    A::Message: 'static,
{
    for message in messages.drain(..) {
        debug.log_message(&message);

        debug.update_started();
        let command: Command<A::Message> = runtime.enter(|| application.update(message));
        debug.update_finished();

        run_command(
            application,
            compositor,
            surface,
            cache,
            state,
            renderer,
            command,
            runtime,
            clipboard,
            custom_actions,
            should_exit,
            proxy,
            debug,
        );
    }
    state.synchronize(application);

    let subscription = application.subscription();
    runtime.track(subscription.into_recipes());
}

#[allow(unused)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_command<A, C, E>(
    application: &A,
    compositor: &mut C,
    surface: &mut C::Surface,
    cache: &mut user_interface::Cache,
    state: &State<A>,
    renderer: &mut A::Renderer,
    command: Command<A::Message>,
    runtime: &mut Runtime<E, IcedProxy<A::Message>, A::Message>,
    clipboard: &mut LayerShellClipboard,
    custom_actions: &mut Vec<LayerShellActions<()>>,
    should_exit: &mut bool,
    proxy: &mut IcedProxy<A::Message>,
    debug: &mut Debug,
) where
    A: Application,
    E: Executor,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
    A::Message: 'static,
{
    use iced_core::widget::operation;
    use iced_runtime::clipboard;
    use iced_runtime::command;
    use iced_runtime::window;
    use iced_runtime::window::Action as WinowAction;
    let mut customactions = Vec::new();
    for action in command.actions() {
        match action {
            command::Action::Future(future) => {
                runtime.spawn(future);
            }
            command::Action::Stream(stream) => {
                runtime.run(stream);
            }
            command::Action::Clipboard(action) => match action {
                clipboard::Action::Read(tag, kind) => {
                    let message = tag(clipboard.read(kind));

                    proxy.send(message);
                }
                clipboard::Action::Write(contents, kind) => {
                    clipboard.write(kind, contents);
                }
            },
            command::Action::Widget(action) => {
                let mut current_cache = std::mem::take(cache);
                let mut current_operation = Some(action);

                let mut user_interface = build_user_interface(
                    application,
                    current_cache,
                    renderer,
                    state.logical_size(),
                    debug,
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
            command::Action::Window(action) => match action {
                WinowAction::Close(_) => {
                    *should_exit = true;
                }
                WinowAction::Screenshot(_id, tag) => {
                    let bytes = compositor.screenshot(
                        renderer,
                        surface,
                        state.viewport(),
                        state.background_color(),
                        &debug.overlay(),
                    );

                    proxy.send(tag(window::Screenshot::new(bytes, state.physical_size())));
                }
                _ => {}
            },
            command::Action::LoadFont { bytes, tagger } => {
                use iced_core::text::Renderer;

                // TODO: Error handling (?)
                renderer.load_font(bytes);

                proxy.send(tagger(Ok(())));
            }
            command::Action::Custom(custom) => {
                if let Some(action) = custom.downcast_ref::<LayershellCustomActionsWithInfo<()>>() {
                    customactions.push(*action);
                }
            }
            _ => {}
        }
    }
    custom_actions.push(LayerShellActions::CustomActions(customactions));
}
