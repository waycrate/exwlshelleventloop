mod state;
use crate::{
    actions::UnLockAction, event::WindowEvent, multi_window::window_manager::WindowManager,
    user_interface::UserInterfaces,
};
use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    sync::Arc,
    task::Poll,
};

use crate::{clipboard::SessionLockClipboard, conversion, error::Error};

use super::DefaultStyle;
#[cfg(not(all(feature = "linux-theme-detection", target_os = "linux")))]
use iced::theme::Mode;
use iced_graphics::{Compositor, Shell, compositor};

use iced_core::{Size, time::Instant};
use iced_runtime::{Action, UserInterface, user_interface};

use iced_futures::{Executor, Runtime};

use iced::{
    Event as IcedEvent,
    mouse::Cursor,
    theme,
    window::{Event as IcedWindowEvent, Id as IcedId, RedrawRequest},
};

use iced_runtime::debug;
use sessionlockev::RefreshRequest;
use sessionlockev::id::Id as SessionLockId;
use sessionlockev::{ReturnData, SessionLockEvent, WindowState, WindowWrapper};
use window_manager::Window;

use futures::{FutureExt, StreamExt, future::LocalBoxFuture};

use crate::{event::IcedSessionLockEvent, proxy::IcedProxy, settings::Settings};

mod window_manager;

type SessionRuntime<E, Message> = Runtime<E, IcedProxy<Action<Message>>, Action<Message>>;
use iced_program::Instance;
use iced_program::Program;
// a dispatch loop, another is listen loop
pub fn run<P>(
    program: P,
    settings: Settings,
    compositor_settings: iced_graphics::Settings,
) -> Result<(), Error>
where
    P: Program + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<UnLockAction, Error = P::Message>,
{
    use futures::task;

    let (message_sender, message_receiver) = std::sync::mpsc::channel::<Action<P::Message>>();
    let boot_span = debug::boot();
    let proxy = IcedProxy::new(message_sender);

    #[cfg(feature = "debug")]
    {
        let proxy = proxy.clone();

        debug::on_hotpatch(move || {
            proxy.send_action(Action::Reload);
        });
    }

    let proxy_back = proxy.clone();
    let mut runtime: SessionRuntime<P::Executor, P::Message> = {
        let executor = P::Executor::new().map_err(Error::ExecutorCreationFailed)?;

        Runtime::new(executor, proxy)
    };

    let (application, task) = runtime.enter(|| Instance::new(program));

    if let Some(stream) = iced_runtime::task::into_stream(task) {
        runtime.run(stream);
    }

    runtime.track(iced_futures::subscription::into_recipes(
        runtime.enter(|| application.subscription().map(Action::Output)),
    ));
    #[cfg(all(feature = "linux-theme-detection", target_os = "linux"))]
    let system_theme = {
        let to_mode = |color_scheme| match color_scheme {
            mundy::ColorScheme::NoPreference => theme::Mode::None,
            mundy::ColorScheme::Light => theme::Mode::Light,
            mundy::ColorScheme::Dark => theme::Mode::Dark,
        };

        runtime.run(
            mundy::Preferences::stream(mundy::Interest::ColorScheme)
                .map(move |preferences| {
                    Action::System(iced_runtime::system::Action::NotifyTheme(to_mode(
                        preferences.color_scheme,
                    )))
                })
                .boxed(),
        );

        runtime
            .enter(|| {
                mundy::Preferences::once_blocking(
                    mundy::Interest::ColorScheme,
                    core::time::Duration::from_millis(200),
                )
            })
            .map(|preferences| to_mode(preferences.color_scheme))
            .unwrap_or_default()
    };

    #[cfg(not(all(feature = "linux-theme-detection", target_os = "linux")))]
    let system_theme = Mode::default();

    let ev: WindowState<()> = sessionlockev::WindowState::new()
        .with_use_display_handle(true)
        .with_connection(settings.with_connection)
        .build()
        .expect("Seems sessionlock is not supported");

    let mut task_context = task::Context::from_waker(task::noop_waker_ref());
    let context = Context::<
        P,
        <P as iced::Program>::Executor,
        <P::Renderer as iced_graphics::compositor::Default>::Compositor,
    >::new(
        application,
        compositor_settings,
        runtime,
        settings.fonts,
        system_theme,
        proxy_back,
    );
    let mut context_state = ContextState::Context(context);
    boot_span.finish();
    let mut waiting_session_lock_events = VecDeque::new();
    let _ = ev.running_with_proxy(message_receiver, move |event, ev, id| {
        match event {
            SessionLockEvent::InitRequest => {}
            // TODO: maybe use it later
            SessionLockEvent::BindProvide(_, _) => {}
            SessionLockEvent::RequestMessages(message) => {
                waiting_session_lock_events
                    .push_back((id, IcedSessionLockEvent::Window(WindowEvent::from(message))));
            }
            SessionLockEvent::NormalDispatch => {
                waiting_session_lock_events.push_back((id, IcedSessionLockEvent::NormalDispatch));
            }
            SessionLockEvent::UserEvent(event) => {
                waiting_session_lock_events
                    .push_back((id, IcedSessionLockEvent::UserAction(event)));
            }
            _ => {}
        }
        loop {
            let mut need_continue = false;
            context_state = match std::mem::replace(&mut context_state, ContextState::None) {
                ContextState::None => unreachable!("context state is taken but not returned"),
                ContextState::Future(mut future) => {
                    tracing::debug!("poll context future");
                    match future.as_mut().poll(&mut task_context) {
                        Poll::Ready(context) => {
                            tracing::debug!("context future is ready");
                            // context is ready, continue to run.
                            need_continue = true;
                            ContextState::Context(context)
                        }
                        Poll::Pending => ContextState::Future(future),
                    }
                }
                ContextState::Context(context) => {
                    if let Some((session_lock_id, session_lock_event)) =
                        waiting_session_lock_events.pop_front()
                    {
                        need_continue = true;
                        let (context_state, waiting_layer_shell_event) =
                            context.handle_event(ev, session_lock_id, session_lock_event);
                        if let Some(waiting_layer_shell_event) = waiting_layer_shell_event {
                            waiting_session_lock_events
                                .push_front((session_lock_id, waiting_layer_shell_event));
                        }
                        context_state
                    } else {
                        ContextState::Context(context)
                    }
                }
            };
            if !need_continue {
                break;
            }
        }
        ReturnData::None
    });
    Ok(())
}

enum ContextState<Context> {
    None,
    Context(Context),
    Future(LocalBoxFuture<'static, Context>),
}

struct Context<P, E, C>
where
    P: Program + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    E: Executor + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static,
{
    compositor_settings: iced_graphics::Settings,
    runtime: SessionRuntime<E, P::Message>,
    system_theme: iced::theme::Mode,
    fonts: Vec<Cow<'static, [u8]>>,
    compositor: Option<C>,
    window_manager: WindowManager<P, C>,
    cached_layer_dimensions: HashMap<IcedId, (Size<u32>, f32)>,
    clipboard: SessionLockClipboard,
    user_interfaces: UserInterfaces<P>,
    iced_events: Vec<(IcedId, IcedEvent)>,
    messages: Vec<P::Message>,
    proxy: IcedProxy<Action<P::Message>>,
}

impl<P, E, C> Context<P, E, C>
where
    P: Program + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    E: Executor + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<UnLockAction, Error = P::Message>,
{
    pub fn new(
        application: Instance<P>,
        compositor_settings: iced_graphics::Settings,
        runtime: SessionRuntime<E, P::Message>,
        fonts: Vec<Cow<'static, [u8]>>,
        system_theme: iced::theme::Mode,
        proxy: IcedProxy<Action<P::Message>>,
    ) -> Self {
        Self {
            compositor_settings,
            runtime,
            system_theme,
            fonts,
            compositor: Default::default(),
            window_manager: WindowManager::new(),
            cached_layer_dimensions: HashMap::new(),
            clipboard: SessionLockClipboard::unconnected(),
            user_interfaces: UserInterfaces::new(application),
            iced_events: Default::default(),
            messages: Default::default(),
            proxy,
        }
    }

    async fn create_compositor(mut self, window: Arc<WindowWrapper>) -> Self {
        let shell = Shell::new(self.proxy.clone());
        let mut new_compositor = C::new(self.compositor_settings, window.clone(), shell)
            .await
            .expect("Cannot create compositer");
        for font in self.fonts.clone() {
            new_compositor.load_font(font);
        }
        self.compositor = Some(new_compositor);
        self.clipboard = SessionLockClipboard::connect(&window);
        self
    }

    #[allow(unused)]
    fn remove_compositor(&mut self) {
        self.compositor = None;
        self.clipboard = SessionLockClipboard::unconnected();
    }

    fn handle_event(
        mut self,
        ev: &mut WindowState<()>,
        session_lock_id: Option<SessionLockId>,
        session_lock_event: IcedSessionLockEvent<P::Message>,
    ) -> (ContextState<Self>, Option<IcedSessionLockEvent<P::Message>>) {
        tracing::debug!(
            "Handle sessionlock event, sessionlockev: {:?}, messages: {}",
            session_lock_id,
            self.messages.len(),
        );
        if let IcedSessionLockEvent::Window(WindowEvent::Refresh) = session_lock_event
            && self.compositor.is_none()
        {
            let Some(layer_shell_window) = session_lock_id.and_then(|lid| ev.get_unit_with_id(lid))
            else {
                tracing::error!("layer shell window not found: {:?}", session_lock_id);
                return (ContextState::Context(self), None);
            };
            tracing::debug!("creating compositor");
            let context_state = ContextState::Future(
                self.create_compositor(Arc::new(layer_shell_window.gen_wrapper()))
                    .boxed_local(),
            );
            return (context_state, Some(session_lock_event));
        }
        match session_lock_event {
            IcedSessionLockEvent::Window(WindowEvent::Refresh) => {
                self.handle_refresh_event(ev, session_lock_id)
            }

            IcedSessionLockEvent::Window(window_event) => {
                self.handle_window_event(session_lock_id, window_event)
            }
            IcedSessionLockEvent::UserAction(user_action) => {
                self.handle_user_action(ev, user_action)
            }
            IcedSessionLockEvent::NormalDispatch => self.handle_normal_dispatch(ev),
        }

        (ContextState::Context(self), None)
    }

    fn handle_refresh_event(
        &mut self,
        ev: &mut WindowState<()>,
        session_lock_id: Option<SessionLockId>,
    ) {
        let Some(session_lock_window) = session_lock_id.and_then(|lid| ev.get_unit_with_id(lid))
        else {
            return;
        };
        let (width, height) = session_lock_window.get_size();
        let scale_float = session_lock_window.scale_float();
        let mut events = Vec::new();
        let (iced_id, window) = if let Some((id, window)) =
            self.window_manager.get_mut_alias(session_lock_window.id())
        {
            let window_size = window.state.window_size();

            if window_size.width != width
                || window_size.height != height
                || window.state.wayland_scale_factor() != scale_float
            {
                let layout_span = debug::layout(id);
                window.state.update_view_port(width, height, scale_float);

                if let Some(ui) = self.user_interfaces.ui_mut(&id) {
                    ui.relayout(window.state.viewport().logical_size(), &mut window.renderer);
                }
                layout_span.finish();
            }
            (id, window)
        } else {
            let wrapper = Arc::new(session_lock_window.gen_wrapper());
            let iced_id = IcedId::unique();
            debug::theme_changed(|| {
                self.window_manager
                    .first()
                    .and_then(|window| theme::Base::palette(window.state.theme()))
            });
            let window = self.window_manager.insert(
                iced_id,
                (width, height),
                scale_float,
                wrapper,
                self.user_interfaces.application(),
                self.compositor
                    .as_mut()
                    .expect("It should have been created"),
                self.system_theme,
            );

            self.user_interfaces.build(
                iced_id,
                user_interface::Cache::default(),
                &mut window.renderer,
                window.state.viewport().logical_size(),
            );

            events.push(IcedEvent::Window(IcedWindowEvent::Opened {
                position: None,
                size: window.state.window_size_f32(),
            }));
            (iced_id, window)
        };

        let compositor = self
            .compositor
            .as_mut()
            .expect("The compositor should have been created");

        let mut ui = self
            .user_interfaces
            .ui_mut(&iced_id)
            .expect("Get User interface");

        let cursor = if ev.is_mouse_surface(session_lock_window.id()) {
            window.state.cursor()
        } else {
            Cursor::Unavailable
        };

        events.push(IcedEvent::Window(IcedWindowEvent::RedrawRequested(
            Instant::now(),
        )));
        let draw_span = debug::draw(iced_id);
        let (ui_state, statuses) = ui.update(
            &events,
            cursor,
            &mut window.renderer,
            &mut self.clipboard,
            &mut self.messages,
        );
        let physical_size = window.state.viewport().physical_size();

        if self
            .cached_layer_dimensions
            .get(&iced_id)
            .is_none_or(|(size, scale)| {
                *size != physical_size || *scale != window.state.viewport().scale_factor()
            })
        {
            self.cached_layer_dimensions.insert(
                iced_id,
                (physical_size, window.state.viewport().scale_factor()),
            );

            compositor.configure_surface(
                &mut window.surface,
                physical_size.width,
                physical_size.height,
            );
        }

        for (idx, event) in events.into_iter().enumerate() {
            let status = statuses
                .get(idx)
                .cloned()
                .unwrap_or(iced_core::event::Status::Ignored);
            self.runtime
                .broadcast(iced_futures::subscription::Event::Interaction {
                    window: iced_id,
                    event,
                    status,
                });
        }

        ui.draw(
            &mut window.renderer,
            window.state.theme(),
            &iced_core::renderer::Style {
                text_color: window.state.text_color(),
            },
            cursor,
        );
        draw_span.finish();

        let session_lock_id = session_lock_window.id();
        Self::handle_ui_state(ev, window, ui_state, false);

        let present_span = debug::present(iced_id);
        match compositor.present(
            &mut window.renderer,
            &mut window.surface,
            window.state.viewport(),
            window.state.background_color(),
            || {
                ev.request_next_present(session_lock_id);
            },
        ) {
            Ok(()) => {
                present_span.finish();
            }
            Err(error) => match error {
                compositor::SurfaceError::OutOfMemory => {
                    panic!("{error:?}");
                }
                _ => {
                    tracing::error!("Error {error:?} when presenting surface.");
                }
            },
        }
    }
    fn handle_window_event(&mut self, session_lock_id: Option<SessionLockId>, event: WindowEvent) {
        let id_and_window = if let Some(layer_shell_id) = session_lock_id {
            self.window_manager.get_mut_alias(layer_shell_id)
        } else {
            self.window_manager.iter_mut().next()
        };
        let Some((iced_id, window)) = id_and_window else {
            return;
        };
        // In previous implementation, event without layer_shell_id won't call `update` here, but
        // will broadcast to the application. I'm not sure why, but I think it is
        // reasonable to call `update` here.
        window
            .state
            .update(&event, self.user_interfaces.application());
        if let Some(event) = conversion::window_event(
            &event,
            window.state.application_scale_factor(),
            window.state.modifiers(),
        ) {
            self.iced_events.push((iced_id, event));
        }
    }

    fn handle_ui_state(
        ev: &mut WindowState<()>,
        window: &mut Window<P, C>,
        ui_state: user_interface::State,
        unconditional_rendering: bool,
    ) -> bool {
        match ui_state {
            user_interface::State::Outdated => true,
            user_interface::State::Updated {
                redraw_request,
                mouse_interaction,
                ..
            } => {
                if unconditional_rendering {
                    ev.request_refresh(window.id, RefreshRequest::NextFrame);
                } else {
                    match redraw_request {
                        RedrawRequest::NextFrame => {
                            ev.request_refresh(window.id, RefreshRequest::NextFrame)
                        }
                        RedrawRequest::At(instant) => {
                            ev.request_refresh(window.id, RefreshRequest::At(instant))
                        }
                        RedrawRequest::Wait => {}
                    }
                }

                if mouse_interaction != window.mouse_interaction {
                    if let Some(pointer) = ev.get_pointer() {
                        ev.append_return_data(ReturnData::RequestSetCursorShape((
                            conversion::mouse_interaction(mouse_interaction),
                            pointer.clone(),
                        )));
                    }
                    window.mouse_interaction = mouse_interaction;
                }
                false
            }
        }
    }
    fn handle_user_action(&mut self, ev: &mut WindowState<()>, action: Action<P::Message>) {
        let mut should_exit = false;
        run_action(
            &mut self.user_interfaces,
            &mut self.compositor,
            action,
            &mut self.messages,
            &mut self.clipboard,
            &mut should_exit,
            &mut self.window_manager,
            &mut self.system_theme,
            &mut self.runtime,
            ev,
        );
        if should_exit {
            ev.append_return_data(ReturnData::RequestUnlockAndExist);
        }
    }
    fn handle_normal_dispatch(&mut self, ev: &mut WindowState<()>) {
        if self.iced_events.is_empty() && self.messages.is_empty() {
            return;
        }

        let mut rebuilds = Vec::new();
        for (iced_id, window) in self.window_manager.iter_mut() {
            let interact_span = debug::interact(iced_id);
            let mut window_events = vec![];

            self.iced_events.retain(|(window_id, event)| {
                if *window_id == iced_id {
                    window_events.push(event.clone());
                    false
                } else {
                    true
                }
            });

            if window_events.is_empty() && self.messages.is_empty() {
                continue;
            }

            let (ui_state, statuses) = self
                .user_interfaces
                .ui_mut(&iced_id)
                .expect("Get user interface")
                .update(
                    &window_events,
                    window.state.cursor(),
                    &mut window.renderer,
                    &mut self.clipboard,
                    &mut self.messages,
                );

            #[cfg(feature = "unconditional-rendering")]
            let unconditional_rendering = true;
            #[cfg(not(feature = "unconditional-rendering"))]
            let unconditional_rendering = false;
            if Self::handle_ui_state(ev, window, ui_state, unconditional_rendering) {
                rebuilds.push((iced_id, window));
            }

            for (event, status) in window_events.drain(..).zip(statuses.into_iter()) {
                self.runtime
                    .broadcast(iced_futures::subscription::Event::Interaction {
                        window: iced_id,
                        event,
                        status,
                    });
            }
            interact_span.finish();
        }

        if !self.messages.is_empty() {
            ev.request_refresh_all(RefreshRequest::NextFrame);
            let (caches, application) = self.user_interfaces.extract_all();

            // Update application
            update(application, &mut self.runtime, &mut self.messages);

            for (_, window) in self.window_manager.iter_mut() {
                window.state.synchronize(application);
            }
            debug::theme_changed(|| {
                self.window_manager
                    .first()
                    .and_then(|window| theme::Base::palette(window.state.theme()))
            });

            for (iced_id, cache) in caches {
                let Some(window) = self.window_manager.get_mut(iced_id) else {
                    continue;
                };
                self.user_interfaces.build(
                    iced_id,
                    cache,
                    &mut window.renderer,
                    window.state.viewport().logical_size(),
                );
            }
        } else {
            for (iced_id, window) in rebuilds {
                if let Some(cache) = self.user_interfaces.remove(&iced_id) {
                    self.user_interfaces.build(
                        iced_id,
                        cache,
                        &mut window.renderer,
                        window.state.viewport().logical_size(),
                    );
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn build_user_interfaces<'a, P: Program, C>(
    application: &'a Instance<P>,
    window_manager: &mut WindowManager<P, C>,
    mut cached_user_interfaces: HashMap<iced::window::Id, user_interface::Cache>,
) -> HashMap<iced::window::Id, UserInterface<'a, P::Message, P::Theme, P::Renderer>>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
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
pub(crate) fn update<P: Program, E: Executor>(
    application: &mut Instance<P>,
    runtime: &mut SessionRuntime<E, P::Message>,
    messages: &mut Vec<P::Message>,
) where
    P::Theme: DefaultStyle,
    P::Message: 'static,
{
    for message in messages.drain(..) {
        let task = runtime.enter(|| application.update(message));

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
pub(crate) fn run_action<P, C, E: Executor>(
    user_interfaces: &mut UserInterfaces<P>,
    compositor: &mut Option<C>,
    action: Action<P::Message>,
    messages: &mut Vec<P::Message>,
    clipboard: &mut SessionLockClipboard,
    should_exit: &mut bool,
    window_manager: &mut WindowManager<P, C>,
    system_theme: &mut iced::theme::Mode,
    runtime: &mut SessionRuntime<E, P::Message>,
    ev: &mut WindowState<()>,
) where
    P: Program + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<UnLockAction, Error = P::Message>,
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
        Action::Image(action) => match action {
            iced_runtime::image::Action::Allocate(handle, sender) => {
                use iced_core::Renderer as _;

                // TODO: Shared image cache in compositor
                if let Some((_id, window)) = window_manager.iter_mut().next() {
                    window.renderer.allocate_image(&handle, move |allocation| {
                        let _ = sender.send(allocation);
                    });
                }
            }
        },
        Action::Widget(action) => {
            let mut current_operation = Some(action);

            while let Some(mut operation) = current_operation.take() {
                // kind of suboptimal that we have to iterate over all windows, but since an operation does not have
                // a window id associated with it, this is the best we can do for now
                for (id, window) in window_manager.iter_mut() {
                    if let Some(mut ui) = user_interfaces.ui_mut(&id) {
                        ui.operate(&window.renderer, operation.as_mut());
                    }
                }

                match operation.finish() {
                    operation::Outcome::None => {}
                    operation::Outcome::Some(()) => {}
                    operation::Outcome::Chain(next) => {
                        current_operation = Some(next);
                    }
                }
            }
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
                _ => {}
            }
        }
        Action::System(action) => match action {
            iced_runtime::system::Action::GetTheme(channel) => {
                let _ = channel.send(*system_theme);
            }
            iced_runtime::system::Action::NotifyTheme(mode) => {
                if mode != *system_theme {
                    *system_theme = mode;

                    runtime.broadcast(iced_futures::subscription::Event::SystemThemeChanged(mode));
                }

                for (_id, window) in window_manager.iter_mut() {
                    window.state.update(
                        &WindowEvent::ThemeChanged(mode),
                        user_interfaces.application(),
                    );
                }
                ev.request_refresh_all(RefreshRequest::NextFrame);
            }

            _ => {}
        },
        Action::LoadFont { bytes, channel } => {
            // TODO: Error handling (?)
            if let Some(compositor) = compositor {
                compositor.load_font(bytes.clone());

                let _ = channel.send(Ok(()));
            }
        }
        Action::Reload => {
            for (iced_id, window) in window_manager.iter_mut() {
                if let Some(cache) = user_interfaces.remove(&iced_id) {
                    user_interfaces.build(
                        iced_id,
                        cache,
                        &mut window.renderer,
                        window.state.viewport().logical_size(),
                    );
                }
            }
            ev.request_refresh_all(RefreshRequest::NextFrame);
        }
        _ => {}
    }
}
