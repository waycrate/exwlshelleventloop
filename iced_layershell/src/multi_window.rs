mod state;
use crate::multi_window::window_manager::WindowManager;
use std::{collections::HashMap, f64, mem::ManuallyDrop, sync::Arc};

use crate::{
    actions::{LayerShellActions, LayershellCustomActions},
    clipboard::LayerShellClipboard,
    conversion,
    error::Error,
};

use iced_graphics::Compositor;

use iced_core::{time::Instant, Size};

use iced_runtime::{multi_window::Program, user_interface, Command, Debug, UserInterface};

use iced_style::application::StyleSheet;

use iced_futures::{Executor, Runtime, Subscription};

use layershellev::{
    reexport::wayland_client::{KeyState, WEnum},
    LayerEvent, ReturnData, WindowState,
};

use futures::{channel::mpsc, SinkExt, StreamExt};

use crate::{
    event::{IcedLayerEvent, MutiWindowIcedLayerEvent},
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

    /// Returns the current [`Theme`] of the [`Application`].
    fn theme(&self) -> Self::Theme;

    /// Returns the [`Style`] variation of the [`Theme`].
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
    use iced::window;

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

    let ev: WindowState<()> = layershellev::WindowState::new(&application.namespace())
        .with_single(false)
        .with_use_display_handle(true)
        .with_option_size(settings.layer_settings.size)
        .with_layer(settings.layer_settings.layer)
        .with_anchor(settings.layer_settings.anchor)
        .with_exclusize_zone(settings.layer_settings.exclusize_zone)
        .with_margin(settings.layer_settings.margins)
        .with_keyboard_interacivity(settings.layer_settings.keyboard_interactivity)
        .build()
        .unwrap();

    let window = Arc::new(ev.gen_main_wrapper());
    let mut compositor = C::new(compositor_settings, window.clone())?;

    let mut window_manager = WindowManager::new();
    let _ = window_manager.insert(
        window::Id::MAIN,
        ev.main_window().get_size(),
        window,
        &application,
        &mut compositor,
    );

    let (mut event_sender, event_receiver) =
        mpsc::unbounded::<MutiWindowIcedLayerEvent<A::Message>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded::<Vec<LayerShellActions>>();

    let mut instance = Box::pin(run_instance::<A, E, C>(
        application,
        compositor,
        runtime,
        proxy,
        debug,
        event_receiver,
        control_sender,
        //state,
        window_manager,
        init_command,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    let mut pointer_serial: u32 = 0;
    let mut key_event: Option<IcedLayerEvent<A::Message>> = None;
    let mut key_ping_count: u32 = 400;

    let _ = ev.running_with_proxy(message_receiver, move |event, ev, index| {
        use layershellev::DispatchMessage;
        let id = index.map(|index| ev.get_unit(index).id());
        match event {
            LayerEvent::InitRequest => {}
            // TODO: maybe use it later
            LayerEvent::BindProvide(_, _) => {}
            LayerEvent::RequestMessages(message) => 'outside: {
                match message {
                    DispatchMessage::RequestRefresh { width, height } => {
                        event_sender
                            .start_send(MutiWindowIcedLayerEvent(
                                id,
                                IcedLayerEvent::RequestRefreshWithWrapper {
                                    width: *width,
                                    height: *height,
                                    wrapper: ev.get_unit(index.unwrap()).gen_wrapper(),
                                },
                            ))
                            .expect("Cannot send");
                        break 'outside;
                    }
                    DispatchMessage::MouseEnter { serial, .. } => {
                        pointer_serial = *serial;
                    }
                    DispatchMessage::KeyBoard { state, .. } => {
                        if let WEnum::Value(KeyState::Pressed) = state {
                            key_event = Some(message.into());
                        } else {
                            key_event = None;
                            key_ping_count = 400;
                        }
                    }
                    _ => {}
                }

                event_sender
                    .start_send(MutiWindowIcedLayerEvent(id, message.into()))
                    .expect("Cannot send");
            }
            LayerEvent::NormalDispatch => match &key_event {
                Some(keyevent) => {
                    if let IcedLayerEvent::Window(windowevent) = keyevent {
                        let event = IcedLayerEvent::Window(*windowevent);
                        if key_ping_count > 70 && key_ping_count < 74 {
                            event_sender
                                .start_send(MutiWindowIcedLayerEvent(id, event))
                                .expect("Cannot send");
                            key_ping_count = 0;
                        } else {
                            event_sender
                                .start_send(MutiWindowIcedLayerEvent(
                                    id,
                                    IcedLayerEvent::NormalUpdate,
                                ))
                                .expect("Cannot send");
                        }
                        if key_ping_count >= 74 {
                            key_ping_count -= 1;
                        } else {
                            key_ping_count += 1;
                        }
                    }
                }
                None => {
                    event_sender
                        .start_send(MutiWindowIcedLayerEvent(id, IcedLayerEvent::NormalUpdate))
                        .expect("Cannot send");
                }
            },
            LayerEvent::UserEvent(event) => {
                event_sender
                    .start_send(MutiWindowIcedLayerEvent(
                        id,
                        IcedLayerEvent::UserEvent(event),
                    ))
                    .ok();
            }
            _ => {}
        }
        let poll = instance.as_mut().poll(&mut context);
        match poll {
            task::Poll::Pending => 'peddingBlock: {
                if let Ok(Some(flow)) = control_receiver.try_next() {
                    for flow in flow {
                        match flow {
                            LayerShellActions::CustomActions(actions) => {
                                for action in actions {
                                    match action {
                                        LayershellCustomActions::AnchorChange(anchor) => {
                                            ev.main_window().set_anchor(anchor);
                                        }
                                        LayershellCustomActions::LayerChange(layer) => {
                                            ev.main_window().set_layer(layer);
                                        }
                                        LayershellCustomActions::SizeChange((width, height)) => {
                                            ev.main_window().set_size((width, height));
                                        }
                                    }
                                }
                            }
                            LayerShellActions::Mouse(mouse) => {
                                let Some(pointer) = ev.get_pointer() else {
                                    break 'peddingBlock ReturnData::None;
                                };

                                break 'peddingBlock ReturnData::RequestSetCursorShape((
                                    conversion::mouse_interaction(mouse),
                                    pointer.clone(),
                                    pointer_serial,
                                ));
                            }
                            LayerShellActions::RedrawAll => {
                                break 'peddingBlock ReturnData::RedrawAllRequest;
                            }
                            LayerShellActions::RedrawWindow(index) => {
                                break 'peddingBlock ReturnData::RedrawIndexRequest(index);
                            }
                        }
                    }
                }
                ReturnData::None
            }
            task::Poll::Ready(_) => ReturnData::RequestExist,
        }
    });
    Ok(())
}

#[allow(unused)]
#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: A,
    mut compositor: C,
    mut runtime: Runtime<E, IcedProxy<A::Message>, A::Message>,
    mut proxy: IcedProxy<A::Message>,
    mut debug: Debug,
    mut event_receiver: mpsc::UnboundedReceiver<MutiWindowIcedLayerEvent<A::Message>>,
    mut control_sender: mpsc::UnboundedSender<Vec<LayerShellActions>>,
    mut window_manager: WindowManager<A, C>,
    init_command: Command<A::Message>,
) where
    A: Application + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
{
    use iced::window;
    use iced_core::mouse;
    use iced_core::Event;
    let main_window = window_manager
        .get_mut(window::Id::MAIN)
        .expect("Get main window");
    let main_window_size = main_window.state.logical_size();
    let mut clipboard = LayerShellClipboard;
    let mut ui_caches: HashMap<window::Id, user_interface::Cache> = HashMap::new();

    let mut user_interfaces = ManuallyDrop::new(build_user_interfaces(
        &application,
        &mut debug,
        &mut window_manager,
        HashMap::from_iter([(window::Id::MAIN, user_interface::Cache::default())]),
    ));

    let mut events = {
        vec![(
            Some(window::Id::MAIN),
            Event::Window(
                window::Id::MAIN,
                window::Event::Opened {
                    position: None,
                    size: main_window_size,
                },
            ),
        )]
    };
    let mut custom_actions = Vec::new();

    let mut should_exit = false;
    let mut messages = Vec::new();

    run_command(
        &application,
        &mut compositor,
        init_command,
        &mut runtime,
        &mut custom_actions,
        &mut should_exit,
        &mut proxy,
        &mut debug,
        &mut window_manager,
        &mut ui_caches,
    );

    // TODO: run_command
    runtime.track(application.subscription().into_recipes());
    while let Some(event) = event_receiver.next().await {
        match event {
            MutiWindowIcedLayerEvent(
                _id,
                IcedLayerEvent::RequestRefreshWithWrapper {
                    width,
                    height,
                    wrapper,
                },
            ) => {
                let layerid = wrapper.id();
                let (id, window) = if window_manager.get_mut_alias(wrapper.id()).is_none() {
                    let id = window::Id::unique();

                    let window = window_manager.insert(
                        id,
                        (width, height),
                        Arc::new(wrapper),
                        &application,
                        &mut compositor,
                    );
                    let logical_size = window.state.logical_size();

                    let _ = user_interfaces.insert(
                        id,
                        build_user_interface(
                            &application,
                            user_interface::Cache::default(),
                            &mut window.renderer,
                            logical_size,
                            &mut debug,
                            id,
                        ),
                    );
                    let _ = ui_caches.insert(id, user_interface::Cache::default());

                    events.push((
                        Some(id),
                        Event::Window(
                            id,
                            window::Event::Opened {
                                position: None,
                                size: window.state.logical_size(),
                            },
                        ),
                    ));
                    (id, window)
                } else {
                    let (id, window) = window_manager.get_mut_alias(wrapper.id()).unwrap();
                    let ui = user_interfaces.remove(&id).expect("Get User interface");
                    window.state.update_view_port(width, height);
                    let renderer = &window.renderer;
                    let _ = user_interfaces.insert(
                        id,
                        ui.relayout(
                            Size {
                                width: width as f32,
                                height: height as f32,
                            },
                            &mut window.renderer,
                        ),
                    );
                    (id, window)
                };

                let ui = user_interfaces.get_mut(&id).expect("Get User interface");

                let redraw_event =
                    Event::Window(id, window::Event::RedrawRequested(Instant::now()));

                let cursor = window.state.cursor();

                ui.update(
                    &[redraw_event.clone()],
                    cursor,
                    &mut window.renderer,
                    &mut clipboard,
                    &mut messages,
                );

                debug.draw_started();
                let new_mouse_interaction = ui.draw(
                    &mut window.renderer,
                    window.state.theme(),
                    &iced_core::renderer::Style {
                        text_color: window.state.text_color(),
                    },
                    cursor,
                );
                debug.draw_finished();

                if new_mouse_interaction != window.mouse_interaction {
                    custom_actions.push(LayerShellActions::Mouse(new_mouse_interaction));
                    window.mouse_interaction = new_mouse_interaction;
                }

                compositor.configure_surface(&mut window.surface, width, height);
                runtime.broadcast(redraw_event.clone(), iced_core::event::Status::Ignored);
                debug.render_started();

                debug.draw_started();
                ui.draw(
                    &mut window.renderer,
                    &application.theme(),
                    &iced_core::renderer::Style {
                        text_color: window.state.text_color(),
                    },
                    window.state.cursor(),
                );
                debug.draw_finished();
                compositor
                    .present(
                        &mut window.renderer,
                        &mut window.surface,
                        window.state.viewport(),
                        window.state.background_color(),
                        &debug.overlay(),
                    )
                    .ok();

                debug.render_finished();
            }
            MutiWindowIcedLayerEvent(Some(id), IcedLayerEvent::Window(event)) => {
                let Some((id, window)) = window_manager.get_mut_alias(id) else {
                    continue;
                };
                window.state.update(&event);
                if let Some(event) = conversion::window_event(id, &event) {
                    events.push((Some(id), event));
                }
            }
            MutiWindowIcedLayerEvent(_, IcedLayerEvent::UserEvent(event)) => {
                messages.push(event);
            }
            MutiWindowIcedLayerEvent(_, IcedLayerEvent::NormalUpdate) => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }

                debug.event_processing_started();

                let mut uis_stale = false;
                for (id, window) in window_manager.iter_mut() {
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

                    if !uis_stale {
                        uis_stale = matches!(ui_state, user_interface::State::Outdated);
                    }

                    debug.event_processing_finished();

                    for (event, status) in window_events.drain(..).zip(statuses.into_iter()) {
                        runtime.broadcast(event, status);
                    }
                }
                // TODO mw application update returns which window IDs to update
                if !messages.is_empty() || uis_stale {
                    let mut cached_interfaces: HashMap<window::Id, user_interface::Cache> =
                        ManuallyDrop::into_inner(user_interfaces)
                            .drain()
                            .map(|(id, ui)| (id, ui.into_cache()))
                            .collect();

                    // Update application
                    update(
                        &mut application,
                        &mut compositor,
                        &mut runtime,
                        &mut should_exit,
                        &mut proxy,
                        &mut debug,
                        &mut messages,
                        &mut custom_actions,
                        &mut window_manager,
                        &mut cached_interfaces,
                    );

                    for (id, window) in window_manager.iter_mut() {
                        window.state.synchronize(&application);
                    }

                    custom_actions.push(LayerShellActions::RedrawAll);

                    user_interfaces = ManuallyDrop::new(build_user_interfaces(
                        &application,
                        &mut debug,
                        &mut window_manager,
                        cached_interfaces,
                    ));
                }
            }
            _ => {}
        }
        control_sender.start_send(custom_actions.clone()).ok();
        custom_actions.clear();
    }
    let _ = ManuallyDrop::into_inner(user_interfaces);
}
pub fn build_user_interfaces<'a, A: Application, C: Compositor>(
    application: &'a A,
    debug: &mut Debug,
    window_manager: &mut WindowManager<A, C>,
    mut cached_user_interfaces: HashMap<iced::window::Id, user_interface::Cache>,
) -> HashMap<iced::window::Id, UserInterface<'a, A::Message, A::Theme, A::Renderer>>
where
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: StyleSheet,
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
                    window.state.logical_size(),
                    debug,
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
    debug: &mut Debug,
    id: iced::window::Id,
) -> UserInterface<'a, A::Message, A::Theme, A::Renderer>
where
    A::Theme: StyleSheet,
{
    debug.view_started();
    let view = application.view(id);
    debug.view_finished();

    debug.layout_started();
    let user_interface = UserInterface::build(view, size, cache, renderer);
    debug.layout_finished();
    user_interface
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update<A: Application, C, E: Executor>(
    application: &mut A,
    compositor: &mut C,
    runtime: &mut Runtime<E, IcedProxy<A::Message>, A::Message>,
    should_exit: &mut bool,
    proxy: &mut IcedProxy<A::Message>,
    debug: &mut Debug,
    messages: &mut Vec<A::Message>,
    custom_actions: &mut Vec<LayerShellActions>,
    window_manager: &mut WindowManager<A, C>,
    ui_caches: &mut HashMap<iced::window::Id, user_interface::Cache>,
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
            command,
            runtime,
            custom_actions,
            should_exit,
            proxy,
            debug,
            window_manager,
            ui_caches,
        );
    }

    let subscription = application.subscription();
    runtime.track(subscription.into_recipes());
}

#[allow(unused)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_command<A, C, E>(
    application: &A,
    compositor: &mut C,
    command: Command<A::Message>,
    runtime: &mut Runtime<E, IcedProxy<A::Message>, A::Message>,
    custom_actions: &mut Vec<LayerShellActions>,
    should_exit: &mut bool,
    proxy: &mut IcedProxy<A::Message>,
    debug: &mut Debug,
    window_manager: &mut WindowManager<A, C>,
    ui_caches: &mut HashMap<iced::window::Id, user_interface::Cache>,
) where
    A: Application,
    E: Executor,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: StyleSheet,
    A::Message: 'static,
{
    use iced_core::widget::operation;
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
            command::Action::Clipboard(_action) => {
                // TODO:
            }
            command::Action::Widget(action) => {
                let mut current_operation = Some(action);

                let mut uis = build_user_interfaces(
                    application,
                    debug,
                    window_manager,
                    std::mem::take(ui_caches),
                );

                'operate: while let Some(mut operation) = current_operation.take() {
                    for (id, ui) in uis.iter_mut() {
                        if let Some(window) = window_manager.get_mut(*id) {
                            ui.operate(&window.renderer, operation.as_mut());

                            match operation.finish() {
                                operation::Outcome::None => {}
                                operation::Outcome::Some(message) => {
                                    proxy.send(message);

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

                *ui_caches = uis.drain().map(|(id, ui)| (id, ui.into_cache())).collect();
            }
            command::Action::Window(action) => match action {
                WinowAction::Close(_) => {
                    *should_exit = true;
                }
                WinowAction::Screenshot(id, tag) => {
                    let Some(window) = window_manager.get_mut(id) else {
                        continue;
                    };
                    let bytes = compositor.screenshot(
                        &mut window.renderer,
                        &mut window.surface,
                        window.state.viewport(),
                        window.state.background_color(),
                        &debug.overlay(),
                    );

                    proxy.send(tag(window::Screenshot::new(
                        bytes,
                        window.state.physical_size(),
                    )));
                }
                _ => {}
            },
            command::Action::LoadFont { bytes, tagger } => {
                use iced_core::text::Renderer;

                // TODO change this once we change each renderer to having a single backend reference.. :pain:
                // TODO: Error handling (?)
                for (_, window) in window_manager.iter_mut() {
                    window.renderer.load_font(bytes.clone());
                }

                proxy.send(tagger(Ok(())));
            }
            command::Action::Custom(custom) => {
                if let Some(action) = custom.downcast_ref::<LayershellCustomActions>() {
                    customactions.push(*action);
                }
            }
            _ => {}
        }
    }
    custom_actions.push(LayerShellActions::CustomActions(customactions));
}
