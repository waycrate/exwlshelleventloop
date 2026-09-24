use crate::{
    BlurOption, CursorUpdateContext, DispatchMessage, EventContext, EventLoop, ExShellEventError,
    ExWlShellHandler, ExWlShellInitEvent, InitRequest, LayerSize, LockLifecycle, Margin, Shell,
    StartMode, WaylandSource, WindowState, WindowStateUnitBuilder, WithConnection,
    WpCursorShapeManagerV1, id,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
};

use wayland_client::{
    Connection, Proxy,
    globals::registry_queue_init,
    protocol::{wl_compositor::WlCompositor, wl_seat::WlSeat, wl_shm::WlShm},
};
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

use crate::seat::SeatStorage;
use crate::size::warn_if_exclusive_zone_ignored;
use sctk::{output::OutputState, registry::RegistryState, seat::SeatState};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::input_method::zv1::client::zwp_input_panel_v1::ZwpInputPanelV1;
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
#[derive(Debug)]
pub struct ContextBuilder {
    default_namespace: String,
    start_mode: StartMode,
    events_transparent: bool,
    keyboard_interactivity: KeyboardInteractivity,
    anchor: Anchor,
    margin: Option<Margin>,
    size: LayerSize,
    exclusive_zone: Option<i32>,
    with_connection: Option<WithConnection>,
    use_display_handle: bool,
    layer: Layer,
    blur_option: BlurOption,
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            default_namespace: "osd".to_owned(),
            start_mode: StartMode::Active,
            events_transparent: false,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::all(),
            margin: None,
            size: LayerSize::FILL,
            exclusive_zone: None,
            with_connection: None,
            use_display_handle: false,
            layer: Layer::Top,
            blur_option: BlurOption::None,
        }
    }
}

impl ContextBuilder {
    /// create a WindowState, you need to pass a namespace in
    pub fn start(namespace: &str) -> Self {
        assert_ne!(namespace, "");
        Self {
            default_namespace: namespace.to_owned(),
            ..Default::default()
        }
    }

    /// suggest to bind to specific output
    /// if there is no such output , it will bind the output which now is focused,
    /// same with when binded_output_name is None
    pub fn with_xdg_output_name(mut self, binded_output_name: String) -> Self {
        self.start_mode = StartMode::TargetScreen(binded_output_name);
        self
    }

    pub fn with_start_mode(mut self, mode: StartMode) -> Self {
        self.start_mode = mode;
        self
    }

    pub fn with_events_transparent(mut self, transparent: bool) -> Self {
        self.events_transparent = transparent;
        self
    }

    /// if the shell is a single one, only display on one screen,
    /// fi true, the layer will binding to current screen
    pub fn with_active(mut self) -> Self {
        self.start_mode = StartMode::Active;
        self
    }

    pub fn with_active_or_xdg_output_name(self, binded_output_name: Option<String>) -> Self {
        match binded_output_name {
            Some(binded_output_name) => self.with_xdg_output_name(binded_output_name),
            None => self.with_active(),
        }
    }

    pub fn with_allscreens_or_xdg_output_name(self, binded_output_name: Option<String>) -> Self {
        match binded_output_name {
            Some(binded_output_name) => self.with_xdg_output_name(binded_output_name),
            None => self.with_allscreens(),
        }
    }
    pub fn with_xdg_output_name_or_not(self, binded_output_name: Option<String>) -> Self {
        let Some(binded_output_name) = binded_output_name else {
            return self;
        };
        self.with_xdg_output_name(binded_output_name)
    }

    pub fn with_allscreens_or_active(mut self, allscreen: bool) -> Self {
        if allscreen {
            self.start_mode = StartMode::AllScreens;
        } else {
            self.start_mode = StartMode::Active;
        }
        self
    }

    pub fn with_allscreens(mut self) -> Self {
        self.start_mode = StartMode::AllScreens;
        self
    }

    pub fn with_background_or_not(self, background_mode: bool) -> Self {
        if !background_mode {
            return self;
        }
        self.with_background()
    }

    pub fn with_background(mut self) -> Self {
        self.start_mode = StartMode::Background;
        self
    }

    /// keyboard_interacivity, please take look at [layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
    pub fn with_keyboard_interacivity(
        mut self,
        keyboard_interacivity: KeyboardInteractivity,
    ) -> Self {
        self.keyboard_interactivity = keyboard_interacivity;
        self
    }

    /// set the layer_shell anchor
    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// set the layer_shell layer
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    /// set the layer margin
    pub fn with_margin(mut self, margin: Margin) -> Self {
        self.margin = Some(margin);
        self
    }

    /// if not set, the default is [`LayerSize::FILL`], which with the default four-edge
    /// anchor and margins to 0,0,0,0 gives a surface the size of the screen.
    ///
    /// if set, layer_shell will use the size you set
    pub fn with_size(mut self, size: LayerSize) -> Self {
        self.size = size;
        self
    }

    /// exclusive_zone, please take look at [layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
    pub fn with_exclusive_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }

    /// set exwlshellev to use display_handle
    pub fn with_use_display_handle(mut self, use_display_handle: bool) -> Self {
        self.use_display_handle = use_display_handle;
        self
    }

    /// set a callback to create a wayland connection
    pub fn with_connection(mut self, connection_or: Option<WithConnection>) -> Self {
        self.with_connection = connection_or;
        self
    }
    pub fn with_blur_option(mut self, blur_option: BlurOption) -> Self {
        self.blur_option = blur_option;
        self
    }
}

impl ContextBuilder {
    pub fn attach<T: 'static, Window>(
        self,
        mut window: Window,
    ) -> Result<EventContext<T, Window>, ExShellEventError>
    where
        Window: ExWlShellHandler<T> + 'static,
    {
        let mut state: WindowState<T> = self.build_state()?;
        let globals = state.globals.take().unwrap();
        let event_queue_origin = state.event_queue.as_ref().unwrap();
        let qh = event_queue_origin.handle();

        let connection = state.connection.clone().unwrap();

        let shm = state.shm.clone().unwrap();

        let wmcompositer = state.wl_compositor.clone().unwrap();

        let mut init_event = None;

        let cursor_manager: Option<WpCursorShapeManagerV1> = state.cursor_manager.clone();

        let cursor_update_context = CursorUpdateContext {
            cursor_manager,
            qh: qh.clone(),
            connection: connection.clone(),
            shm: shm.clone(),
            cursor_surface: wmcompositer.create_surface(&qh, ()),
        };

        while !matches!(init_event, Some(InitRequest::None)) {
            match init_event {
                None => {
                    init_event = Some(window.on_init(&mut state, ExWlShellInitEvent::Start));
                }
                Some(InitRequest::RequestBind) => {
                    init_event = Some(
                        window.on_init(&mut state, ExWlShellInitEvent::BindProvide(&globals, &qh)),
                    );
                }
                Some(InitRequest::RequestCompositor) => {
                    init_event = Some(window.on_init(
                        &mut state,
                        ExWlShellInitEvent::CompositorProvide(&wmcompositer, &qh),
                    ));
                }
                _ => unreachable!(),
            }
        }

        let event_loop: EventLoop<_> =
            EventLoop::try_new().expect("Failed to initialize the event loop");

        let event_queue = connection.new_event_queue::<EventContext<T, Window>>();
        WaylandSource::new(connection.clone(), event_queue)
            .insert(event_loop.handle())
            .expect("Failed to init wayland source");
        let signal = event_loop.get_signal();
        Ok(EventContext {
            state,
            window_context: window,
            looph: event_loop.handle(),
            event_loop: Some(event_loop),
            lock: LockLifecycle::Unlocked,
            signal,
            cached_tokens: vec![],
            cursor_update_context,
        })
    }
    fn build_state<T: 'static>(mut self) -> Result<WindowState<T>, ExShellEventError> {
        let connection = if let Some(with_connection) = self.with_connection.take() {
            with_connection.get_connection()?
        } else {
            Connection::connect_to_env()?
        };
        let mut state: WindowState<T> = Default::default();

        state.default_namespace = self.default_namespace;
        state.margin = self.margin;
        state.events_transparent = self.events_transparent;
        state.start_mode = self.start_mode;
        let (globals, mut event_queue) = registry_queue_init::<WindowState<T>>(&connection)?;
        state.display = Some(connection.display());

        let qh = event_queue.handle();

        state.registry_state = Some(RegistryState::new(&globals));
        state.output_state = Some(OutputState::new(&globals, &qh));
        let seat_state = SeatState::new(&globals, &qh);
        for seat in seat_state.seats() {
            state.seats.insert(seat.id(), SeatStorage::new());
        }
        state.seat_state = Some(seat_state);
        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?;
        state.background_effect_manager = globals
            .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        state.shm = Some(shm);
        state.seat_back = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let wmbase = globals.bind::<XdgWmBase, _, _>(&qh, 2..=6, ())?;
        state.wmbase = Some(wmbase);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=2, ())
            .ok();
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();

        // register this

        let decoration_manager = globals
            .bind::<ZxdgDecorationManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        state.xdg_decoration_manager = decoration_manager;

        let fractional_scale_manager = globals
            .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let text_input_manager = globals
            .bind::<ZwpTextInputManagerV3, _, _>(&qh, 1..=1, ())
            .ok();

        let lock_manager = globals
            .bind::<ExtSessionLockManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let layer_shell = globals.bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ()).ok();
        let input_panel = globals.bind::<ZwpInputPanelV1, _, _>(&qh, 1..=1, ()).ok();

        state.text_input_manager = text_input_manager;
        event_queue.blocking_dispatch(&mut state)?; // then make a dispatch

        // OutputState bound its own xdg_outputs before the dispatch above, so output info is
        // populated by now, a second roundtrip is not needed
        // so it is the same way, to get surface detach to protocol, first get the shell, like
        // wmbase or layer_shell or session-shell, then get `surface` from the wl_surface you
        // get before, and set it
        // finally thing to remember is to commit the surface, make the shell to init.
        //let (init_w, init_h) = self.size;
        // this example is ok for both xdg_surface and layer_shell
        if state.is_background() {
            let background_surface = wmcompositer.create_surface(&qh, ());
            if state.events_transparent {
                let region = wmcompositer.create_region(&qh, ());
                background_surface.set_input_region(Some(&region));
                region.destroy();
            }
            state.background_surface = Some(background_surface);
        } else if !state.is_allscreens() {
            let binded_output = match state.start_mode.clone() {
                StartMode::TargetScreen(name) => state.output_by_name(&name),
                StartMode::TargetOutput(output) => Some(output),
                _ => None,
            };

            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            let layer_shell_ref = layer_shell.as_ref().expect("We need layershell here");
            let layer = layer_shell_ref.get_layer_surface(
                &wl_surface,
                binded_output.as_ref(),
                state.layer,
                state.default_namespace.clone(),
                &qh,
                (),
            );
            let wire_anchor = self.size.resolve_anchor(self.anchor);
            layer.set_anchor(wire_anchor);
            layer.set_keyboard_interactivity(self.keyboard_interactivity);
            let (init_w, init_h) = self.size.to_set();
            layer.set_size(init_w, init_h);

            if let Some(zone) = self.exclusive_zone {
                warn_if_exclusive_zone_ignored(zone, wire_anchor);
                layer.set_exclusive_zone(zone);
            }

            if let Some(Margin {
                top,
                right,
                bottom,
                left,
            }) = self.margin
            {
                layer.set_margin(top, right, bottom, left);
            }

            if self.events_transparent {
                let region = wmcompositer.create_region(&qh, ());
                wl_surface.set_input_region(Some(&region));
                region.destroy();
            }

            wl_surface.commit();

            let mut fractional_scale = None;
            if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                fractional_scale =
                    Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
            }
            let mut effect = None;
            if let Some(effect_manger) = &state.background_effect_manager {
                effect = Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
            }
            let viewport = viewporter
                .as_ref()
                .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
            state.push_window(
                WindowStateUnitBuilder::new(
                    id::Id::unique(),
                    qh.clone(),
                    connection.display(),
                    wl_surface,
                    wmcompositer.clone(),
                    Shell::LayerShell(layer),
                )
                .blur_option(state.blur_option.clone())
                .layout(state.anchor, self.size)
                .effect_surface(effect)
                .viewport(viewport)
                .fractional_scale(fractional_scale)
                .wl_output(binded_output.clone())
                .build(),
            );
        } else {
            let displays = state.outputs.clone();

            let layer_shell_ref = layer_shell.as_ref().expect("We need layershell here");
            for output_display in displays.iter() {
                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,

                let layer = layer_shell_ref.get_layer_surface(
                    &wl_surface,
                    Some(output_display),
                    state.layer,
                    state.default_namespace.clone(),
                    &qh,
                    (),
                );
                let wire_anchor = self.size.resolve_anchor(self.anchor);
                layer.set_anchor(wire_anchor);
                layer.set_keyboard_interactivity(self.keyboard_interactivity);
                let (init_w, init_h) = self.size.to_set();
                layer.set_size(init_w, init_h);

                if let Some(zone) = self.exclusive_zone {
                    warn_if_exclusive_zone_ignored(zone, wire_anchor);
                    layer.set_exclusive_zone(zone);
                }

                if let Some(Margin {
                    top,
                    right,
                    bottom,
                    left,
                }) = state.margin
                {
                    layer.set_margin(top, right, bottom, left);
                }

                if self.events_transparent {
                    let region = wmcompositer.create_region(&qh, ());
                    wl_surface.set_input_region(Some(&region));
                    region.destroy();
                }
                wl_surface.commit();

                let mut fractional_scale = None;
                if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                    fractional_scale =
                        Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
                }
                let viewport = viewporter
                    .as_ref()
                    .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                let mut effect = None;
                if let Some(effect_manger) = &state.background_effect_manager {
                    effect = Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
                }
                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                // and if you need to reconfigure it, you need to commit the wl_surface again
                // so because this is just an example, so we just commit it once
                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                state.push_window(
                    WindowStateUnitBuilder::new(
                        id::Id::unique(),
                        qh.clone(),
                        connection.display(),
                        wl_surface,
                        wmcompositer.clone(),
                        Shell::LayerShell(layer),
                    )
                    .layout(state.anchor, self.size)
                    .viewport(viewport)
                    .blur_option(state.blur_option.clone())
                    .effect_surface(effect)
                    .fractional_scale(fractional_scale)
                    .wl_output(Some(output_display.clone()))
                    .build(),
                );
            }
            state
                .messages
                .retain(|(_, message)| !matches!(message, DispatchMessage::NewDisplay(_)));
        }
        state.init_finished = true;
        state.viewporter = viewporter;
        state.event_queue = Some(event_queue);
        state.globals = Some(globals);
        state.wl_compositor = Some(wmcompositer);
        state.fractional_scale_manager = fractional_scale_manager;
        state.cursor_manager = cursor_manager;
        state.lock_manager = lock_manager;
        state.layer_shell = layer_shell;
        state.input_panel = input_panel;
        state.connection = Some(connection);
        todo!()
    }
}
