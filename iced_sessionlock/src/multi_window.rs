mod state;
use crate::{
    actions::{SessionShellActionVec, UnLockAction},
    multi_window::window_manager::WindowManager,
};
use std::{borrow::Cow, collections::HashMap, f64, mem::ManuallyDrop, sync::Arc};

use crate::{
    actions::SessionShellAction, clipboard::SessionLockClipboard, conversion, error::Error,
};

use super::DefaultStyle;
use iced_graphics::{Compositor, compositor};

use iced_core::{Size, time::Instant};
use iced_runtime::{Action, UserInterface, user_interface};

use iced_futures::{Executor, Runtime};

use iced::theme;
use iced_runtime::debug;
use sessionlockev::{ReturnData, SessionLockEvent, WindowState, WindowWrapper};

use futures::{StreamExt, channel::mpsc};

use crate::{
    event::{IcedSessionLockEvent, MultiWindowIcedSessionLockEvent},
    proxy::IcedProxy,
    settings::Settings,
};

mod window_manager;

type SessionRuntime<E, Message> = Runtime<E, IcedProxy<Action<Message>>, Action<Message>>;
use crate::build_pattern::Instance;
use crate::build_pattern::Program;
// a dispatch loop, another is listen loop
pub fn run<A>(
    program: A,
    settings: Settings,
    compositor_settings: iced_graphics::Settings,
) -> Result<(), Error>
where
    A: Program + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<UnLockAction, Error = A::Message>,
{
    use futures::Future;
    use futures::task;

    let (message_sender, message_receiver) = std::sync::mpsc::channel::<Action<A::Message>>();
    let boot_span = debug::boot();
    let proxy = IcedProxy::new(message_sender);
    let mut runtime: SessionRuntime<A::Executor, A::Message> = {
        let executor = A::Executor::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, task) = runtime.enter(|| Instance::new(program));

    if let Some(stream) = iced_runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(iced_futures::subscription::into_recipes(
        runtime.enter(|| application.subscription().map(Action::Output)),
    ));

    let ev: WindowState<()> = sessionlockev::WindowState::new()
        .with_use_display_handle(true)
        .build()
        .expect("Seems sessionlock is not supported");

    let window = Arc::new(ev.gen_main_wrapper());

    let (mut event_sender, event_receiver) =
        mpsc::unbounded::<MultiWindowIcedSessionLockEvent<Action<A::Message>>>();
    let (control_sender, mut control_receiver) = mpsc::unbounded::<SessionShellActionVec>();

    let mut instance = Box::pin(run_instance::<
        A,
        A::Executor,
        <A::Renderer as iced_graphics::compositor::Default>::Compositor,
    >(
        application,
        compositor_settings,
        runtime,
        event_receiver,
        control_sender,
        //state,
        window,
        settings.fonts,
    ));

    let mut context = task::Context::from_waker(task::noop_waker_ref());

    boot_span.finish();
    let _ = ev.running_with_proxy(message_receiver, move |event, ev, id| {
        use sessionlockev::DispatchMessage;
        match event {
            SessionLockEvent::InitRequest => {}
            // TODO: maybe use it later
            SessionLockEvent::BindProvide(_, _) => {}
            SessionLockEvent::RequestMessages(message) => 'outside: {
                if let DispatchMessage::RequestRefresh {
                    width,
                    height,
                    scale_float,
                } = message
                {
                    event_sender
                        .start_send(MultiWindowIcedSessionLockEvent(
                            id,
                            IcedSessionLockEvent::RequestRefreshWithWrapper {
                                width: *width,
                                height: *height,
                                scale_float: *scale_float,
                                wrapper: ev.get_unit_with_id(id.unwrap()).unwrap().gen_wrapper(),
                            },
                        ))
                        .expect("Cannot send");
                    break 'outside;
                }

                event_sender
                    .start_send(MultiWindowIcedSessionLockEvent(id, message.into()))
                    .expect("Cannot send");
            }
            SessionLockEvent::NormalDispatch => {
                event_sender
                    .start_send(MultiWindowIcedSessionLockEvent(
                        id,
                        IcedSessionLockEvent::NormalUpdate,
                    ))
                    .expect("Cannot send");
            }
            SessionLockEvent::UserEvent(event) => {
                event_sender
                    .start_send(MultiWindowIcedSessionLockEvent(
                        id,
                        IcedSessionLockEvent::UserEvent(event),
                    ))
                    .ok();
            }
            _ => {}
        }
        let poll = instance.as_mut().poll(&mut context);
        match poll {
            task::Poll::Pending => 'peddingBlock: {
                if let Some(flow) = control_receiver
                    .try_next()
                    .ok()
                    .flatten()
                    .and_then(|flows| flows.into_iter().next())
                {
                    match flow {
                        SessionShellAction::Mouse(mouse) => {
                            let Some(pointer) = ev.get_pointer() else {
                                break 'peddingBlock ReturnData::None;
                            };

                            break 'peddingBlock ReturnData::RequestSetCursorShape((
                                conversion::mouse_interaction(mouse),
                                pointer.clone(),
                            ));
                        }
                        SessionShellAction::RedrawAll => {
                            break 'peddingBlock ReturnData::RedrawAllRequest;
                        }
                        SessionShellAction::RedrawWindow(index) => {
                            break 'peddingBlock ReturnData::RedrawIndexRequest(index);
                        }
                    }
                }
                ReturnData::None
            }
            task::Poll::Ready(_) => ReturnData::RequestUnlockAndExist,
        }
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_instance<A, E, C>(
    mut application: Instance<A>,
    compositor_settings: iced_graphics::Settings,
    mut runtime: SessionRuntime<E, A::Message>,
    mut event_receiver: mpsc::UnboundedReceiver<
        MultiWindowIcedSessionLockEvent<Action<A::Message>>,
    >,
    mut control_sender: mpsc::UnboundedSender<SessionShellActionVec>,
    window: Arc<WindowWrapper>,
    fonts: Vec<Cow<'static, [u8]>>,
) where
    A: Program + 'static,
    E: Executor + 'static,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<UnLockAction, Error = A::Message>,
{
    use iced::window;
    use iced_core::Event;
    let mut compositor = C::new(compositor_settings, window.clone())
        .await
        .expect("Cannot create compositer");
    for font in fonts {
        compositor.load_font(font);
    }
    let mut window_manager = WindowManager::new();

    let mut cached_layer_dimensions: HashMap<iced_core::window::Id, (iced_core::Size<u32>, f64)> =
        HashMap::new();

    let mut clipboard = SessionLockClipboard::connect(&window);
    let mut ui_caches: HashMap<window::Id, user_interface::Cache> = HashMap::new();

    let mut user_interfaces = ManuallyDrop::new(build_user_interfaces(
        &application,
        &mut window_manager,
        HashMap::new(),
    ));
    let mut events = Vec::new();
    let mut custom_actions = Vec::new();

    let mut should_exit = false;
    let mut messages = Vec::new();

    while let Some(event) = event_receiver.next().await {
        match event {
            MultiWindowIcedSessionLockEvent(
                _id,
                IcedSessionLockEvent::RequestRefreshWithWrapper {
                    width,
                    height,
                    wrapper,
                    scale_float,
                },
            ) => {
                let (id, window) =
                    if let Some((id, window)) = window_manager.get_mut_alias(wrapper.id()) {
                        let window_size = window.state.window_size();

                        if window_size.width != width
                            || window_size.height != height
                            || window.state.wayland_scale_factor() != scale_float
                        {
                            let layout_span = debug::layout(id);
                            let ui = user_interfaces.remove(&id).expect("Get User interface");
                            window.state.update_view_port(width, height, scale_float);
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
                        let id = window::Id::unique();
                        debug::theme_changed(|| {
                            window_manager
                                .first()
                                .and_then(|window| theme::Base::palette(window.state.theme()))
                        });
                        let window = window_manager.insert(
                            id,
                            (width, height),
                            scale_float,
                            Arc::new(wrapper),
                            &application,
                            &mut compositor,
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

                let ui = user_interfaces.get_mut(&id).expect("Get User interface");

                let redraw_event =
                    iced_core::Event::Window(window::Event::RedrawRequested(Instant::now()));
                let cursor = window.state.cursor();

                events.push((Some(id), redraw_event.clone()));
                let draw_span = debug::draw(id);
                let (ui_state, _) = ui.update(
                    &[redraw_event.clone()],
                    cursor,
                    &mut window.renderer,
                    &mut clipboard,
                    &mut messages,
                );

                ui.draw(
                    &mut window.renderer,
                    window.state.theme(),
                    &iced_core::renderer::Style {
                        text_color: window.state.text_color(),
                    },
                    cursor,
                );
                draw_span.finish();

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
                    input_method: _,   // TODO: someone's help needed
                    mouse_interaction,
                } = ui_state
                {
                    custom_actions.push(SessionShellAction::Mouse(mouse_interaction));
                    window.mouse_interaction = mouse_interaction;
                    events.push((Some(id), redraw_event.clone()));
                }
                let present_span = debug::present(id);
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
            MultiWindowIcedSessionLockEvent(Some(id), IcedSessionLockEvent::Window(event)) => {
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
            MultiWindowIcedSessionLockEvent(_, IcedSessionLockEvent::UserEvent(action)) => {
                let mut cached_interfaces: HashMap<window::Id, user_interface::Cache> =
                    ManuallyDrop::into_inner(user_interfaces)
                        .drain()
                        .map(|(id, ui)| (id, ui.into_cache()))
                        .collect();

                run_action(
                    &application,
                    &mut messages,
                    &mut compositor,
                    action,
                    &mut clipboard,
                    &mut should_exit,
                    &mut window_manager,
                    &mut cached_interfaces,
                );

                user_interfaces = ManuallyDrop::new(build_user_interfaces(
                    &application,
                    &mut window_manager,
                    cached_interfaces,
                ));
                if should_exit {
                    break;
                }
            }
            MultiWindowIcedSessionLockEvent(_, IcedSessionLockEvent::NormalUpdate) => {
                if events.is_empty() && messages.is_empty() {
                    continue;
                }
                #[cfg(not(feature = "unconditional-rendering"))]
                let mut is_updated = false;
                #[cfg(feature = "unconditional-rendering")]
                let is_updated = false;
                let mut uis_stale = false;
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
                    match ui_state {
                        user_interface::State::Updated {
                            redraw_request,
                            mouse_interaction,
                            ..
                        } => {
                            window.mouse_interaction = mouse_interaction;

                            // TODO: just check NextFrame
                            #[cfg(not(feature = "unconditional-rendering"))]
                            if matches!(redraw_request, iced::window::RedrawRequest::NextFrame) {
                                custom_actions.push(SessionShellAction::RedrawWindow(window.id));
                                is_updated = true;
                            }
                        }
                        user_interface::State::Outdated => {
                            uis_stale = true;
                        }
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
                // TODO mw application update returns which window IDs to update
                if !messages.is_empty() || uis_stale {
                    let cached_interfaces: HashMap<window::Id, user_interface::Cache> =
                        ManuallyDrop::into_inner(user_interfaces)
                            .drain()
                            .map(|(id, ui)| (id, ui.into_cache()))
                            .collect();

                    // Update application
                    update(&mut application, &mut runtime, &mut messages);

                    for (_id, window) in window_manager.iter_mut() {
                        if !is_updated {
                            custom_actions.push(SessionShellAction::RedrawWindow(window.id));
                        }

                        window.state.synchronize(&application);
                    }

                    #[cfg(feature = "unconditional-rendering")]
                    custom_actions.push(SessionShellAction::RedrawAll);

                    debug::theme_changed(|| {
                        window_manager
                            .first()
                            .and_then(|window| theme::Base::palette(window.state.theme()))
                    });
                    user_interfaces = ManuallyDrop::new(build_user_interfaces(
                        &application,
                        &mut window_manager,
                        cached_interfaces,
                    ));
                }
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
pub fn build_user_interfaces<'a, A: Program, C>(
    application: &'a Instance<A>,
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
fn build_user_interface<'a, A: Program>(
    application: &'a Instance<A>,
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
pub(crate) fn update<A: Program, E: Executor>(
    application: &mut Instance<A>,
    runtime: &mut SessionRuntime<E, A::Message>,
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
    application: &Instance<A>,
    messages: &mut Vec<A::Message>,
    compositor: &mut C,
    action: Action<A::Message>,
    clipboard: &mut SessionLockClipboard,
    should_exit: &mut bool,
    window_manager: &mut WindowManager<A, C>,
    ui_caches: &mut HashMap<iced::window::Id, user_interface::Cache>,
) where
    A: Program,
    C: Compositor<Renderer = A::Renderer> + 'static,
    A::Theme: DefaultStyle,
    A::Message: 'static + TryInto<UnLockAction, Error = A::Message>,
{
    use iced_core::widget::operation;
    use iced_runtime::clipboard;
    use iced_runtime::window::Action as WindowAction;
    //let mut customactions = Vec::new();
    match action {
        Action::Output(message) => match message.try_into() {
            Ok(action) => {
                let _: UnLockAction = action;
                *should_exit = true;
            }
            Err(message) => messages.push(message),
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

            let mut uis =
                build_user_interfaces(application, window_manager, std::mem::take(ui_caches));

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

            *ui_caches = uis.drain().map(|(id, ui)| (id, ui.into_cache())).collect();
        }
        Action::Window(action) => 'out: {
            match action {
                WindowAction::Close(_) => {
                    *should_exit = true;
                }
                WindowAction::GetSize(id, channel) => {
                    let Some(window) = window_manager.get(id) else {
                        break 'out;
                    };
                    let _ = channel.send(window.state.window_size_f32());
                }
                WindowAction::Screenshot(id, channel) => {
                    let Some(window) = window_manager.get_mut(id) else {
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
                _ => {}
            }
        }
        Action::LoadFont { bytes, channel } => {
            // TODO: Error handling (?)
            compositor.load_font(bytes.clone());

            let _ = channel.send(Ok(()));
        }

        _ => {}
    }
}
