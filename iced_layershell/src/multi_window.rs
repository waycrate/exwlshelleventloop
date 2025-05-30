mod state;
use crate::{
    DefaultStyle,
    actions::{IcedNewPopupSettings, LayershellCustomActionWithId, MenuDirection},
    ime_preedit::ImeState,
    multi_window::window_manager::WindowManager,
    settings::VirtualKeyboardSettings,
    user_interface::UserInterfaces,
};
use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    f64, mem,
    os::fd::AsFd,
    sync::Arc,
    task::Poll,
    time::Duration,
};

use crate::{
    actions::LayershellCustomAction, clipboard::LayerShellClipboard, conversion, error::Error,
};

use iced::{
    Event as IcedEvent, theme,
    window::{Event as IcedWindowEvent, Id as IcedId, RedrawRequest},
};
use iced_graphics::{Compositor, compositor};
use iced_runtime::Action;

use iced_runtime::debug;

use iced_core::{Size, mouse::Cursor, time::Instant};
use iced_runtime::user_interface;

use iced_futures::{Executor, Runtime};

use layershellev::{
    LayerEvent, NewPopUpSettings, RefreshRequest, ReturnData, WindowState, WindowWrapper,
    calloop::timer::{TimeoutAction, Timer},
    id::Id as LayerShellId,
    reexport::{
        wayland_client::{WlCompositor, WlRegion},
        zwp_virtual_keyboard_v1,
    },
};

use futures::{FutureExt, future::LocalBoxFuture};
use window_manager::Window;

use crate::{
    event::{LayerShellEvent, WindowEvent as LayerShellWindowEvent},
    proxy::IcedProxy,
    settings::Settings,
};

mod window_manager;

type MultiRuntime<E, Message> = Runtime<E, IcedProxy<Action<Message>>, Action<Message>>;

use iced_program::Instance;
use iced_program::Program as IcedProgram;

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
    P::Message: 'static + TryInto<LayershellCustomActionWithId, Error = P::Message>,
{
    use futures::task;
    let (message_sender, message_receiver) = std::sync::mpsc::channel::<Action<P::Message>>();

    let boot_span = debug::boot();
    let proxy = IcedProxy::new(message_sender);
    let mut runtime: MultiRuntime<P::Executor, P::Message> = {
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

    let ev: WindowState<iced::window::Id> = layershellev::WindowState::new(namespace)
        .with_start_mode(settings.layer_settings.start_mode)
        .with_use_display_handle(true)
        .with_events_transparent(settings.layer_settings.events_transparent)
        .with_option_size(settings.layer_settings.size)
        .with_layer(settings.layer_settings.layer)
        .with_anchor(settings.layer_settings.anchor)
        .with_exclusive_zone(settings.layer_settings.exclusive_zone)
        .with_margin(settings.layer_settings.margin)
        .with_keyboard_interacivity(settings.layer_settings.keyboard_interactivity)
        .with_connection(settings.with_connection)
        .build()
        .expect("Cannot create layershell");

    let context = Context::<
        P,
        <P as iced::Program>::Executor,
        <P::Renderer as iced_graphics::compositor::Default>::Compositor,
    >::new(application, compositor_settings, runtime, settings.fonts);
    let mut context_state = ContextState::Context(context);
    boot_span.finish();

    let mut waiting_layer_shell_events = VecDeque::new();
    let mut task_context = task::Context::from_waker(task::noop_waker_ref());

    let _ = ev.running_with_proxy(message_receiver, move |event, ev, layer_shell_id| {
        let mut def_returndata = ReturnData::None;
        match event {
            LayerEvent::InitRequest => {
                def_returndata = ReturnData::RequestBind;
            }
            LayerEvent::BindProvide(globals, qh) => {
                let wl_compositor = globals
                    .bind::<WlCompositor, _, _>(qh, 1..=1, ())
                    .expect("could not bind wl_compositor");
                waiting_layer_shell_events.push_back((
                    None,
                    LayerShellEvent::UpdateInputRegion(wl_compositor.create_region(qh, ())),
                ));

                if let Some(virtual_keyboard_setting) = settings.virtual_keyboard_support.as_ref() {
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
                    } = virtual_keyboard_setting;
                    let seat = ev.get_seat();
                    let virtual_keyboard_in =
                        virtual_keyboard_manager.create_virtual_keyboard(seat, qh, ());
                    virtual_keyboard_in.keymap((*keymap_format).into(), file.as_fd(), *keymap_size);
                    ev.set_virtual_keyboard(virtual_keyboard_in);
                }
            }
            LayerEvent::RequestMessages(message) => {
                waiting_layer_shell_events.push_back((
                    layer_shell_id,
                    LayerShellEvent::Window(LayerShellWindowEvent::from(message)),
                ));
            }
            LayerEvent::UserEvent(event) => {
                waiting_layer_shell_events
                    .push_back((layer_shell_id, LayerShellEvent::UserAction(event)));
            }
            LayerEvent::NormalDispatch => {
                waiting_layer_shell_events
                    .push_back((layer_shell_id, LayerShellEvent::NormalDispatch));
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
                    if let Some((layer_shell_id, layer_shell_event)) =
                        waiting_layer_shell_events.pop_front()
                    {
                        need_continue = true;
                        let (context_state, waiting_layer_shell_event) =
                            context.handle_event(ev, layer_shell_id, layer_shell_event);
                        if let Some(waiting_layer_shell_event) = waiting_layer_shell_event {
                            waiting_layer_shell_events
                                .push_front((layer_shell_id, waiting_layer_shell_event));
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
        def_returndata
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
    P: IcedProgram + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    E: Executor + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static,
{
    compositor_settings: iced_graphics::Settings,
    runtime: MultiRuntime<E, P::Message>,
    fonts: Vec<Cow<'static, [u8]>>,
    compositor: Option<C>,
    window_manager: WindowManager<P, C>,
    cached_layer_dimensions: HashMap<IcedId, (Size<u32>, f64)>,
    clipboard: LayerShellClipboard,
    wl_input_region: Option<WlRegion>,
    user_interfaces: UserInterfaces<Instance<P>, P::Message, P::Theme, P::Renderer>,
    waiting_layer_shell_actions: Vec<(Option<IcedId>, LayershellCustomAction)>,
    iced_events: Vec<(IcedId, IcedEvent)>,
    messages: Vec<P::Message>,
}

impl<P, E, C> Context<P, E, C>
where
    P: IcedProgram + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    E: Executor + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<LayershellCustomActionWithId, Error = P::Message>,
{
    pub fn new(
        application: Instance<P>,
        compositor_settings: iced_graphics::Settings,
        runtime: MultiRuntime<E, P::Message>,
        fonts: Vec<Cow<'static, [u8]>>,
    ) -> Self {
        Self {
            compositor_settings,
            runtime,
            fonts,
            compositor: Default::default(),
            window_manager: WindowManager::new(),
            cached_layer_dimensions: HashMap::new(),
            clipboard: LayerShellClipboard::unconnected(),
            wl_input_region: Default::default(),
            user_interfaces: UserInterfaces::new(application),
            waiting_layer_shell_actions: Default::default(),
            iced_events: Default::default(),
            messages: Default::default(),
        }
    }

    async fn create_compositor(mut self, window: Arc<WindowWrapper>) -> Self {
        let mut new_compositor = C::new(self.compositor_settings, window.clone())
            .await
            .expect("Cannot create compositer");
        for font in self.fonts.clone() {
            new_compositor.load_font(font);
        }
        self.compositor = Some(new_compositor);
        self.clipboard = LayerShellClipboard::connect(&window);
        self
    }

    fn remove_compositor(&mut self) {
        self.compositor = None;
        self.clipboard = LayerShellClipboard::unconnected();
    }

    fn handle_event(
        mut self,
        ev: &mut WindowState<IcedId>,
        layer_shell_id: Option<LayerShellId>,
        layer_shell_event: LayerShellEvent<P::Message>,
    ) -> (ContextState<Self>, Option<LayerShellEvent<P::Message>>) {
        tracing::debug!(
            "Handle layer shell event, layer_shell_id: {:?}, event: {:?}, waiting actions: {}, messages: {}",
            layer_shell_id,
            layer_shell_event,
            self.waiting_layer_shell_actions.len(),
            self.messages.len(),
        );
        if let LayerShellEvent::Window(LayerShellWindowEvent::Refresh) = layer_shell_event {
            if self.compositor.is_none() {
                let Some(layer_shell_window) =
                    layer_shell_id.and_then(|lid| ev.get_unit_with_id(lid))
                else {
                    tracing::error!("layer shell window not found: {:?}", layer_shell_id);
                    return (ContextState::Context(self), None);
                };
                tracing::debug!("creating compositor");
                let context_state = ContextState::Future(
                    self.create_compositor(Arc::new(layer_shell_window.gen_wrapper()))
                        .boxed_local(),
                );
                return (context_state, Some(layer_shell_event));
            }
        }

        match layer_shell_event {
            LayerShellEvent::UpdateInputRegion(region) => self.wl_input_region = Some(region),
            LayerShellEvent::Window(LayerShellWindowEvent::Refresh) => {
                self.handle_refresh_event(ev, layer_shell_id)
            }
            LayerShellEvent::Window(LayerShellWindowEvent::Closed) => {
                self.handle_closed_event(ev, layer_shell_id)
            }
            LayerShellEvent::Window(window_event) => {
                self.handle_window_event(layer_shell_id, window_event)
            }
            LayerShellEvent::UserAction(user_action) => self.handle_user_action(ev, user_action),
            LayerShellEvent::NormalDispatch => self.handle_normal_dispatch(ev),
        }

        // at each interaction try to resolve those waiting actions.
        let mut waiting_layer_shell_actions = Vec::new();
        mem::swap(
            &mut self.waiting_layer_shell_actions,
            &mut waiting_layer_shell_actions,
        );
        for (iced_id, action) in waiting_layer_shell_actions {
            self.handle_layer_shell_action(ev, iced_id, action);
        }

        (ContextState::Context(self), None)
    }

    fn handle_refresh_event(
        &mut self,
        ev: &mut WindowState<IcedId>,
        layer_shell_id: Option<LayerShellId>,
    ) {
        let Some(layer_shell_window) = layer_shell_id.and_then(|lid| ev.get_unit_with_id(lid))
        else {
            return;
        };
        let (width, height) = layer_shell_window.get_size();
        let scale_float = layer_shell_window.scale_float();
        // events may not be handled after RequestRefreshWithWrapper in the same
        // interaction, we dispatched them immediately.
        let mut events = Vec::new();
        let (iced_id, window) = if let Some((iced_id, window)) =
            self.window_manager.get_mut_alias(layer_shell_window.id())
        {
            let window_size = window.state.window_size();

            if window_size.width != width
                || window_size.height != height
                || window.state.wayland_scale_factor() != scale_float
            {
                let layout_span = debug::layout(iced_id);
                window.state.update_view_port(width, height, scale_float);
                if let Some(ui) = self.user_interfaces.ui_mut(&iced_id) {
                    ui.relayout(window.state.viewport().logical_size(), &mut window.renderer);
                }
                layout_span.finish();
            }
            (iced_id, window)
        } else {
            let wrapper = Arc::new(layer_shell_window.gen_wrapper());
            let iced_id = layer_shell_window
                .get_binding()
                .copied()
                .unwrap_or_else(IcedId::unique);

            debug::theme_changed(|| {
                if self.window_manager.is_empty() {
                    theme::Base::palette(&self.user_interfaces.application().theme(iced_id))
                } else {
                    None
                }
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

        let cursor = if ev.is_mouse_surface(layer_shell_window.id()) {
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

        // get layer_shell_id so that layer_shell_window can be drop, and ev can be borrow mut
        let layer_shell_id = layer_shell_window.id();

        Self::handle_ui_state(ev, window, ui_state, false);

        window.draw_preedit();

        let present_span = debug::present(iced_id);
        match compositor.present(
            &mut window.renderer,
            &mut window.surface,
            window.state.viewport(),
            window.state.background_color(),
            || {
                ev.request_next_present(layer_shell_id);
            },
        ) {
            Ok(()) => {
                present_span.finish();
            }
            Err(error) => match error {
                compositor::SurfaceError::OutOfMemory => {
                    panic!("{:?}", error);
                }
                _ => {
                    // In case of `ev.request_next_present` isn't been called. Reset present_slot.
                    ev.reset_present_slot(layer_shell_id);
                    tracing::error!("Error {error:?} when presenting surface.");
                }
            },
        }
    }

    fn handle_closed_event(
        &mut self,
        ev: &mut WindowState<IcedId>,
        layer_shell_id: Option<LayerShellId>,
    ) {
        let Some(iced_id) = layer_shell_id
            .and_then(|lid| ev.get_unit_with_id(lid))
            .and_then(|layer_shell_window| layer_shell_window.get_binding().copied())
        else {
            return;
        };
        self.cached_layer_dimensions.remove(&iced_id);
        self.window_manager.remove(iced_id);
        self.user_interfaces.remove(&iced_id);
        self.runtime
            .broadcast(iced_futures::subscription::Event::Interaction {
                window: iced_id,
                event: IcedEvent::Window(IcedWindowEvent::Closed),
                status: iced_core::event::Status::Ignored,
            });
        // if now there is no windows now, then break the compositor, and unlink the clipboard
        if self.window_manager.is_empty() {
            self.remove_compositor();
        }
    }

    fn handle_window_event(
        &mut self,
        layer_shell_id: Option<LayerShellId>,
        event: LayerShellWindowEvent,
    ) {
        let id_and_window = if let Some(layer_shell_id) = layer_shell_id {
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
        window.state.update(&event);
        if let Some(event) = conversion::window_event(
            &event,
            window.state.application_scale_factor(),
            window.state.modifiers(),
        ) {
            self.iced_events.push((iced_id, event));
        }
    }

    fn handle_user_action(&mut self, ev: &mut WindowState<IcedId>, action: Action<P::Message>) {
        let mut should_exit = false;
        run_action(
            &mut self.user_interfaces,
            &mut self.compositor,
            action,
            &mut self.messages,
            &mut self.clipboard,
            &mut self.waiting_layer_shell_actions,
            &mut should_exit,
            &mut self.window_manager,
        );
        if should_exit {
            ev.append_return_data(ReturnData::RequestExit);
        }
    }

    fn handle_layer_shell_action(
        &mut self,
        ev: &mut WindowState<IcedId>,
        mut iced_id: Option<IcedId>,
        action: LayershellCustomAction,
    ) {
        let layer_shell_window;
        macro_rules! ref_layer_shell_window {
            ($ev: ident, $iced_id: ident, $layer_shell_id: ident, $layer_shell_window: ident) => {
                if $iced_id.is_none() {
                    // Make application also works
                    if let Some(window) = self.window_manager.first() {
                        $iced_id = Some(window.iced_id);
                        $layer_shell_id = Some(window.id);
                    }
                    if $iced_id.is_none() {
                        tracing::error!(
                            "Here should be an id, it is a bug, please report an issue for us"
                        );
                        return;
                    }
                }
                if let Some(ls_window) =
                    $layer_shell_id.and_then(|layer_shell_id| $ev.get_unit_with_id(layer_shell_id))
                {
                    layer_shell_window = ls_window;
                } else {
                    return;
                }
            };
        }
        // check if window is ready
        let mut layer_shell_id = iced_id
            .and_then(|iced_id| self.window_manager.get(iced_id))
            .map(|window| window.id);
        if iced_id.is_some() && layer_shell_id.is_none() {
            // still waiting
            self.waiting_layer_shell_actions.push((iced_id, action));
            return;
        }
        match action {
            LayershellCustomAction::AnchorChange(anchor) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_anchor(anchor);
            }
            LayershellCustomAction::AnchorSizeChange(anchor, size) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_anchor_with_size(anchor, size);
            }
            LayershellCustomAction::LayerChange(layer) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_layer(layer);
            }
            LayershellCustomAction::MarginChange(margin) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_margin(margin);
            }
            LayershellCustomAction::SizeChange((width, height)) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_size((width, height));
            }
            LayershellCustomAction::ExclusiveZoneChange(zone_size) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                layer_shell_window.set_exclusive_zone(zone_size);
            }
            LayershellCustomAction::SetInputRegion(set_region) => {
                ref_layer_shell_window!(ev, iced_id, layer_shell_id, layer_shell_window);
                let set_region = set_region.0;
                let Some(region) = &self.wl_input_region else {
                    tracing::warn!(
                        "wl_input_region is not set, ignore SetInputRegion, window_id: {:?}",
                        iced_id
                    );
                    return;
                };

                let window_size = layer_shell_window.get_size();
                let width: i32 = window_size.0.try_into().unwrap_or_default();
                let height: i32 = window_size.1.try_into().unwrap_or_default();

                region.subtract(0, 0, width, height);
                set_region(region);

                layer_shell_window
                    .get_wlsurface()
                    .set_input_region(self.wl_input_region.as_ref());
            }
            LayershellCustomAction::VirtualKeyboardPressed { time, key } => {
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
            LayershellCustomAction::NewLayerShell {
                settings,
                id: iced_id,
                ..
            } => {
                let layer_shell_id = layershellev::id::Id::unique();
                ev.append_return_data(ReturnData::NewLayerShell((
                    settings,
                    layer_shell_id,
                    Some(iced_id),
                )));
            }
            LayershellCustomAction::RemoveWindow => {
                if let Some(layer_shell_id) = layer_shell_id {
                    ev.request_close(layer_shell_id)
                }
            }
            LayershellCustomAction::NewPopUp {
                settings: menusettings,
                id: iced_id,
            } => {
                let IcedNewPopupSettings { size, position } = menusettings;
                let Some(parent_layer_shell_id) = ev.current_surface_id() else {
                    return;
                };
                let popup_settings = NewPopUpSettings {
                    size,
                    position,
                    id: parent_layer_shell_id,
                };
                let layer_shell_id = layershellev::id::Id::unique();
                ev.append_return_data(ReturnData::NewPopUp((
                    popup_settings,
                    layer_shell_id,
                    Some(iced_id),
                )));
            }
            LayershellCustomAction::NewMenu {
                settings: menu_setting,
                id: iced_id,
            } => {
                let Some(parent_layer_shell_id) = ev.current_surface_id() else {
                    return;
                };
                let Some((_, window)) = self.window_manager.get_alias(parent_layer_shell_id) else {
                    return;
                };

                let Some(point) = window.state.mouse_position() else {
                    return;
                };

                let (x, mut y) = (point.x as i32, point.y as i32);
                if let MenuDirection::Up = menu_setting.direction {
                    y -= menu_setting.size.1 as i32;
                }
                let popup_settings = NewPopUpSettings {
                    size: menu_setting.size,
                    position: (x, y),
                    id: parent_layer_shell_id,
                };
                let layer_shell_id = layershellev::id::Id::unique();
                ev.append_return_data(ReturnData::NewPopUp((
                    popup_settings,
                    layer_shell_id,
                    Some(iced_id),
                )))
            }
            LayershellCustomAction::NewInputPanel {
                settings,
                id: iced_id,
            } => {
                let layer_shell_id = layershellev::id::Id::unique();
                ev.append_return_data(ReturnData::NewInputPanel((
                    settings,
                    layer_shell_id,
                    Some(iced_id),
                )));
            }
            LayershellCustomAction::ForgetLastOutput => {
                ev.forget_last_output();
            }
        }
    }

    fn handle_normal_dispatch(&mut self, ev: &mut WindowState<IcedId>) {
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

    fn handle_ui_state(
        ev: &mut WindowState<IcedId>,
        window: &mut Window<P, C>,
        ui_state: user_interface::State,
        unconditional_rendering: bool,
    ) -> bool {
        match ui_state {
            user_interface::State::Outdated => true,
            user_interface::State::Updated {
                redraw_request,
                input_method,
                mouse_interaction,
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

                let ime_flags = window.request_input_method(input_method.clone());
                match input_method {
                    iced_core::InputMethod::Disabled => {
                        if ime_flags.contains(ImeState::Disabled) {
                            ev.set_ime_allowed(false);
                        }
                    }
                    iced_core::InputMethod::Enabled {
                        position,
                        purpose,
                        preedit: _,
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
                                window.id,
                            );
                        }
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
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update<P: IcedProgram, E: Executor>(
    application: &mut Instance<P>,
    runtime: &mut MultiRuntime<E, P::Message>,
    messages: &mut Vec<P::Message>,
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

    let subscription = runtime.enter(|| application.subscription());
    let recipes = iced_futures::subscription::into_recipes(subscription.map(Action::Output));

    debug::subscriptions_tracked(recipes.len());
    runtime.track(recipes);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_action<P, C>(
    user_interfaces: &mut UserInterfaces<Instance<P>, P::Message, P::Theme, P::Renderer>,
    compositor: &mut Option<C>,
    event: Action<P::Message>,
    messages: &mut Vec<P::Message>,
    clipboard: &mut LayerShellClipboard,
    waiting_layer_shell_actions: &mut Vec<(Option<iced::window::Id>, LayershellCustomAction)>,
    should_exit: &mut bool,
    window_manager: &mut WindowManager<P, C>,
) where
    P: IcedProgram + 'static,
    C: Compositor<Renderer = P::Renderer> + 'static,
    P::Theme: DefaultStyle,
    P::Message: 'static + TryInto<LayershellCustomActionWithId, Error = P::Message>,
{
    use iced_core::widget::operation;
    use iced_runtime::Action;
    use iced_runtime::clipboard;

    use iced_runtime::window::Action as WindowAction;
    match event {
        Action::Output(stream) => match stream.try_into() {
            Ok(action) => {
                let LayershellCustomActionWithId(id, action) = action;
                waiting_layer_shell_actions.push((id, action));
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

            'operate: while let Some(mut operation) = current_operation.take() {
                for (id, window) in window_manager.iter_mut() {
                    if let Some(mut ui) = user_interfaces.ui_mut(&id) {
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
        }
        Action::Window(action) => match action {
            WindowAction::Close(id) => {
                waiting_layer_shell_actions.push((Some(id), LayershellCustomAction::RemoveWindow));
            }
            WindowAction::GetOldest(channel) => {
                let _ = channel.send(window_manager.first_window().map(|(id, _)| *id));
            }
            WindowAction::GetLatest(channel) => {
                let _ = channel.send(window_manager.last_window().map(|(id, _)| *id));
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
