use std::fs::File;

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, BindError, GlobalError, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_keyboard::{self, KeyState},
        wl_output::{self, WlOutput},
        wl_pointer::{self, ButtonState, WlPointer},
        wl_registry,
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    ConnectError, Connection, Dispatch, DispatchError, Proxy, QueueHandle, WEnum,
};

use wayland_cursor::{CursorImageBuffer, CursorTheme};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::{self, WpCursorShapeDeviceV1},
    wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

#[derive(Debug, thiserror::Error)]
pub enum LayerEventError {
    #[error("connect error")]
    ConnectError(#[from] ConnectError),
    #[error("Global Error")]
    GlobalError(#[from] GlobalError),
    #[error("Bind Error")]
    BindError(#[from] BindError),
    #[error("Error during queue")]
    DispatchError(#[from] DispatchError),
    #[error("create file failed")]
    TempFileCreateFailed(#[from] std::io::Error),
}

pub mod reexport {
    pub use wayland_protocols_wlr::layer_shell::v1::client::{
        zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
        zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
    };
    pub mod wl_shm {
        pub use wayland_client::protocol::wl_shm::Format;
        pub use wayland_client::protocol::wl_shm::WlShm;
    }
    pub mod zwp_virtual_keyboard_v1 {
        pub use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
            zwp_virtual_keyboard_manager_v1::{self, ZwpVirtualKeyboardManagerV1},
            zwp_virtual_keyboard_v1::{self, ZwpVirtualKeyboardV1},
        };
    }
    pub mod wayland_client {
        pub use wayland_client::{globals::GlobalList, protocol::wl_seat::WlSeat, QueueHandle};
    }
}

#[derive(Debug)]
struct BaseState;

// so interesting, it is just need to invoke once, it just used to get the globals
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BaseState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

#[derive(Debug)]
pub struct WindowStateUnit {
    wl_surface: WlSurface,
    buffer: Option<WlBuffer>,
    layer_shell: ZwlrLayerSurfaceV1,
}

impl WindowStateUnit {
    pub fn set_anchor(&self, anchor: Anchor) {
        self.layer_shell.set_anchor(anchor);
        self.wl_surface.commit();
    }

    pub fn set_margin(&self, (top, right, bottom, left): (i32, i32, i32, i32)) {
        self.layer_shell.set_margin(top, right, bottom, left);
        self.wl_surface.commit();
    }
}

#[derive(Debug)]
pub struct WindowState {
    outputs: Vec<(u32, wl_output::WlOutput)>,
    current_surface: Option<WlSurface>,
    is_signal: bool,
    units: Vec<WindowStateUnit>,
    message: Vec<(Option<usize>, DispatchMessage)>,

    // base managers
    seat: Option<WlSeat>,

    // states
    namespace: String,
    keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    anchor: Anchor,
    layer: Layer,
    size: Option<(u32, u32)>,
    exclusive_zone: Option<i32>,
    margin: Option<(i32, i32, i32, i32)>,
}

impl WindowState {
    /// get a seat from state
    pub fn get_seat(&self) -> &WlSeat {
        self.seat.as_ref().unwrap()
    }
}

impl WindowState {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_owned(),
            ..Default::default()
        }
    }

    pub fn with_single(mut self, single: bool) -> Self {
        self.is_signal = single;
        self
    }

    pub fn with_keyboard_interacivity(
        mut self,
        keyboard_interacivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    ) -> Self {
        self.keyboard_interactivity = keyboard_interacivity;
        self
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_margin(mut self, (top, right, bottom, left): (i32, i32, i32, i32)) -> Self {
        self.margin = Some((top, right, bottom, left));
        self
    }

    pub fn with_size(mut self, size: (u32, u32)) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_exclusize_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            current_surface: None,
            is_signal: true,
            units: Vec::new(),
            message: Vec::new(),

            seat: None,

            namespace: "".to_owned(),
            keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right | Anchor::Bottom,
            size: None,
            exclusive_zone: None,
            margin: None,
        }
    }
}

impl WindowState {
    pub fn get_unit(&mut self, index: usize) -> &mut WindowStateUnit {
        &mut self.units[index]
    }

    pub fn get_unit_iter(&self) -> impl Iterator<Item = &WindowStateUnit> {
        self.units.iter()
    }

    fn surface_pos(&self) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref())
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WindowState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == wl_output::WlOutput::interface().name {
                    let output = proxy.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());
                    state.outputs.push((name, output.clone()));
                    state
                        .message
                        .push((None, DispatchMessage::NewDisplay(output)));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.outputs.retain(|x| x.0 != name);
                state.units.retain(|unit| unit.wl_surface.is_alive());
            }

            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WindowState {
    fn event(
        _state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: <wl_seat::WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WindowState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            state: keystate,
            serial,
            key,
            time,
        } = event
        {
            state.message.push((
                state.surface_pos(),
                DispatchMessage::KeyBoard {
                    state: keystate,
                    serial,
                    key,
                    time,
                },
            ));
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WindowState {
    fn event(
        state: &mut Self,
        pointer: &wl_pointer::WlPointer,
        event: <wl_pointer::WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Button {
                state: btnstate,
                serial,
                button,
                time,
            } => {
                state.message.push((
                    state.surface_pos(),
                    DispatchMessage::MouseButton {
                        state: btnstate,
                        serial,
                        button,
                        time,
                    },
                ));
            }
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                state.current_surface = Some(surface.clone());
                state.message.push((
                    state.surface_pos(),
                    DispatchMessage::MouseEnter {
                        pointer: pointer.clone(),
                        serial,
                        surface_x,
                        surface_y,
                    },
                ));
            }
            wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                state.message.push((
                    state.surface_pos(),
                    DispatchMessage::MouseMotion {
                        time,
                        surface_x,
                        surface_y,
                    },
                ));
            }
            _ => {
                // TODO: not now
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WindowState {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            surface.ack_configure(serial);
            let Some(unit_index) = state
                .units
                .iter()
                .position(|unit| unit.layer_shell == *surface)
            else {
                return;
            };
            state.message.push((
                Some(unit_index),
                DispatchMessage::RefreshSurface { width, height },
            ));
        }
    }
}

delegate_noop!(WindowState: ignore WlCompositor); // WlCompositor is need to create a surface
delegate_noop!(WindowState: ignore WlSurface); // surface is the base needed to show buffer
delegate_noop!(WindowState: ignore WlOutput); // output is need to place layer_shell, although here
                                              // it is not used
delegate_noop!(WindowState: ignore WlShm); // shm is used to create buffer pool
delegate_noop!(WindowState: ignore WlShmPool); // so it is pool, created by wl_shm
delegate_noop!(WindowState: ignore WlBuffer); // buffer show the picture
delegate_noop!(WindowState: ignore ZwlrLayerShellV1); // it is simillar with xdg_toplevel, also the
                                                      // ext-session-shell

delegate_noop!(WindowState: ignore WpCursorShapeManagerV1);
delegate_noop!(WindowState: ignore WpCursorShapeDeviceV1);

delegate_noop!(WindowState: ignore ZwpVirtualKeyboardV1);
delegate_noop!(WindowState: ignore ZwpVirtualKeyboardManagerV1);

#[derive(Debug)]
pub enum LayerEvent<'a> {
    InitRequest,
    BindProvide(&'a GlobalList, &'a QueueHandle<WindowState>),
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReturnData {
    WlBuffer(WlBuffer),
    RequestBind,
    RequestExist,
    RequestSetCursorShape((String, WlPointer, u32)),
    None,
}

#[derive(Debug)]
pub enum DispatchMessage {
    NewDisplay(WlOutput),
    MouseButton {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
    },
    MouseEnter {
        pointer: WlPointer,
        serial: u32,
        surface_x: f64,
        surface_y: f64,
    },
    MouseMotion {
        time: u32,
        surface_x: f64,
        surface_y: f64,
    },
    KeyBoard {
        state: WEnum<KeyState>,
        serial: u32,
        key: u32,
        time: u32,
    },
    RefreshSurface {
        width: u32,
        height: u32,
    },
    RequestRefresh {
        width: u32,
        height: u32,
    },
}

impl WindowState {
    pub fn running<F>(&mut self, mut event_hander: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerEvent, &mut WindowState, Option<usize>) -> ReturnData,
    {
        let connection = Connection::connect_to_env()?;
        let (globals, _) = registry_queue_init::<BaseState>(&connection)?; // We just need the
                                                                           // global, the
                                                                           // event_queue is
                                                                           // not needed, we
                                                                           // do not need
                                                                           // BaseState after
                                                                           // this anymore

        let mut event_queue = connection.new_event_queue::<WindowState>();
        let qh = event_queue.handle();

        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?; // so the first
                                                                                // thing is to
                                                                                // get WlCompositor

        // we need to create more

        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        self.seat = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        let _ = connection.display().get_registry(&qh, ()); // so if you want WlOutput, you need to
                                                            // register this

        event_queue.blocking_dispatch(self)?; // then make a dispatch

        let mut init_event = None;

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event = Some(event_hander(LayerEvent::InitRequest, self, None));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(event_hander(
                        LayerEvent::BindProvide(&globals, &qh),
                        self,
                        None,
                    ));
                }
                _ => panic!("Not privide server here"),
            }
        }

        // do the step before, you get empty list

        // so it is the same way, to get surface detach to protocol, first get the shell, like wmbase
        // or layer_shell or session-shell, then get `surface` from the wl_surface you get before, and
        // set it
        // finally thing to remember is to commit the surface, make the shell to init.
        //let (init_w, init_h) = self.size;
        // this example is ok for both xdg_surface and layer_shell
        if self.is_signal {
            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            let layer_shell = globals
                .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                .unwrap();
            let layer = layer_shell.get_layer_surface(
                &wl_surface,
                None,
                self.layer,
                self.namespace.clone(),
                &qh,
                (),
            );
            layer.set_anchor(self.anchor);
            layer.set_keyboard_interactivity(self.keyboard_interactivity);
            if let Some((init_w, init_h)) = self.size {
                layer.set_size(init_w, init_h);
            }

            if let Some(zone) = self.exclusive_zone {
                layer.set_exclusive_zone(zone);
            }

            if let Some((top, right, bottom, left)) = self.margin {
                layer.set_margin(top, right, bottom, left);
            }

            wl_surface.commit();

            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

            self.units.push(WindowStateUnit {
                wl_surface,
                buffer: None,
                layer_shell: layer,
            });
        } else {
            let displays = self.outputs.clone();
            for (_, display) in displays.iter() {
                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                let layer_shell = globals
                    .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                    .unwrap();
                let layer = layer_shell.get_layer_surface(
                    &wl_surface,
                    Some(display),
                    self.layer,
                    self.namespace.clone(),
                    &qh,
                    (),
                );
                layer.set_anchor(self.anchor);
                layer.set_keyboard_interactivity(self.keyboard_interactivity);
                if let Some((init_w, init_h)) = self.size {
                    layer.set_size(init_w, init_h);
                }

                if let Some(zone) = self.exclusive_zone {
                    layer.set_exclusive_zone(zone);
                }

                if let Some((top, right, bottom, left)) = self.margin {
                    layer.set_margin(top, right, bottom, left);
                }

                wl_surface.commit();

                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                // and if you need to reconfigure it, you need to commit the wl_surface again
                // so because this is just an example, so we just commit it once
                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                self.units.push(WindowStateUnit {
                    wl_surface,
                    buffer: None,
                    layer_shell: layer,
                });
            }
            self.message.clear();
        }
        'out: loop {
            event_queue.blocking_dispatch(self)?;
            if self.message.is_empty() {
                continue;
            }
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut self.message);
            for msg in messages.iter() {
                match msg {
                    (Some(unit_index), DispatchMessage::RefreshSurface { width, height }) => {
                        let index = *unit_index;
                        if self.units[index].buffer.is_none() {
                            let mut file = tempfile::tempfile()?;
                            let ReturnData::WlBuffer(buffer) = event_hander(
                                LayerEvent::RequestBuffer(&mut file, &shm, &qh, *width, *height),
                                self,
                                Some(index),
                            ) else {
                                panic!("You cannot return this one");
                            };
                            let surface = &self.units[index].wl_surface;
                            surface.attach(Some(&buffer), 0, 0);
                            self.units[index].buffer = Some(buffer);
                        } else {
                            // TODO:
                        }
                        let surface = &self.units[index].wl_surface;

                        surface.commit();
                    }
                    (_, DispatchMessage::NewDisplay(display)) => {
                        if self.is_signal {
                            continue;
                        }
                        let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                        let layer_shell = globals
                            .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                            .unwrap();
                        let layer = layer_shell.get_layer_surface(
                            &wl_surface,
                            Some(display),
                            self.layer,
                            self.namespace.clone(),
                            &qh,
                            (),
                        );
                        layer.set_anchor(self.anchor);
                        layer.set_keyboard_interactivity(self.keyboard_interactivity);
                        if let Some((init_w, init_h)) = self.size {
                            layer.set_size(init_w, init_h);
                        }

                        if let Some(zone) = self.exclusive_zone {
                            layer.set_exclusive_zone(zone);
                        }

                        if let Some((top, right, bottom, left)) = self.margin {
                            layer.set_margin(top, right, bottom, left);
                        }

                        wl_surface.commit();

                        // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                        // and if you need to reconfigure it, you need to commit the wl_surface again
                        // so because this is just an example, so we just commit it once
                        // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                        self.units.push(WindowStateUnit {
                            wl_surface,
                            buffer: None,
                            layer_shell: layer,
                        });
                    }
                    _ => {
                        let (index_message, msg) = msg;
                        match event_hander(LayerEvent::RequestMessages(msg), self, *index_message) {
                            ReturnData::RequestExist => {
                                break 'out;
                            }
                            ReturnData::RequestSetCursorShape((shape, pointer, serial)) => {
                                if let Some(ref cursor_manager) = cursor_manager {
                                    let device = cursor_manager.get_pointer(&pointer, &qh, ());
                                    device.set_shape(
                                        serial,
                                        wp_cursor_shape_device_v1::Shape::Crosshair,
                                    );
                                    device.destroy();
                                } else {
                                    let Some(cursor_buffer) =
                                        get_cursor_buffer(&shape, &connection, &shm)
                                    else {
                                        eprintln!("Cannot find cursor {shape}");
                                        continue;
                                    };
                                    let cursor_surface = wmcompositer.create_surface(&qh, ());
                                    cursor_surface.attach(Some(&cursor_buffer), 0, 0);
                                    // and create a surface. if two or more,
                                    let (hotspot_x, hotspot_y) = cursor_buffer.hotspot();
                                    pointer.set_cursor(
                                        serial,
                                        Some(&cursor_surface),
                                        hotspot_x as i32,
                                        hotspot_y as i32,
                                    );
                                    cursor_surface.commit();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn get_cursor_buffer(
    shape: &str,
    connection: &Connection,
    shm: &WlShm,
) -> Option<CursorImageBuffer> {
    let mut cursor_theme = CursorTheme::load(connection, shm.clone(), 23).ok()?;
    let cursor = cursor_theme.get_cursor(shape);
    Some(cursor?[0].clone())
}
