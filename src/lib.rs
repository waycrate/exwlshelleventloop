use std::fs::File;

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, BindError, GlobalError, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_keyboard::{self, KeyState},
        wl_output::{self, WlOutput},
        wl_pointer::{self, ButtonState},
        wl_registry,
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    ConnectError, Connection, Dispatch, DispatchError, Proxy, QueueHandle, WEnum,
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel::XdgToplevel};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
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

#[derive(Debug, Default)]
pub struct WindowState {
    outputs: Vec<wl_output::WlOutput>,
    wl_surface: Option<WlSurface>,
    buffer: Option<WlBuffer>,
    layer_shell: Option<ZwlrLayerSurfaceV1>,
    message: Vec<DispatchMessage>,
}

impl WindowState {
    pub fn set_anchor(&self, anchor: Anchor) {
        let layer_shell = self.layer_shell.as_ref().unwrap();
        let surface = self.wl_surface.as_ref().unwrap();
        layer_shell.set_anchor(anchor);
        surface.commit();
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
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };

        if interface == wl_output::WlOutput::interface().name {
            let output = proxy.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());
            state.outputs.push(output);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WindowState {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            xdg_surface.ack_configure(serial);
            let surface = state.wl_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
            }
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
            state.message.push(DispatchMessage::KeyBoard {
                state: keystate,
                serial,
                key,
                time,
            });
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WindowState {
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: <wl_pointer::WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_pointer::Event::Button {
            state: btnstate,
            serial,
            button,
            time,
        } = event
        {
            state.message.push(DispatchMessage::Button {
                state: btnstate,
                serial,
                button,
                time,
            });
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
            state
                .message
                .push(DispatchMessage::RefreshSurface { width, height });
        }
    }
}

delegate_noop!(WindowState: ignore WlCompositor); // WlCompositor is need to create a surface
delegate_noop!(WindowState: ignore WlSurface); // surface is the base needed to show buffer
delegate_noop!(WindowState: ignore WlOutput); // output is need to place layer_shell, although here
                                              // it is not used
delegate_noop!(WindowState: ignore WlShm); // shm is used to create buffer pool
delegate_noop!(WindowState: ignore XdgToplevel); // so it is the same with layer_shell, private a
                                                 // place for surface
delegate_noop!(WindowState: ignore WlShmPool); // so it is pool, created by wl_shm
delegate_noop!(WindowState: ignore WlBuffer); // buffer show the picture
delegate_noop!(WindowState: ignore ZwlrLayerShellV1); // it is simillar with xdg_toplevel, also the
                                                      // ext-session-shell

#[derive(Debug)]
pub enum LayerEvent<'a> {
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage),
}

#[derive(Debug)]
pub enum ReturnData {
    WlBuffer(WlBuffer),
    RequestExist,
    None,
}

#[derive(Debug)]
pub enum DispatchMessage {
    Button {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
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

#[derive(Debug)]
pub struct LayerEventLoop {
    keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    anchor: Anchor,
    size: Option<(u32, u32)>,
    exclusive_zone: Option<i32>,
    state: WindowState,
}

impl LayerEventLoop {
    pub fn new() -> Self {
        Self::default()
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

    pub fn with_size(mut self, size: (u32, u32)) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_exclusize_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }
}

impl Default for LayerEventLoop {
    fn default() -> Self {
        LayerEventLoop {
            keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right | Anchor::Bottom,
            size: None,
            exclusive_zone: None,
            state: WindowState::default(),
        }
    }
}

impl LayerEventLoop {
    pub fn running<F>(&mut self, mut event_hander: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerEvent, &mut WindowState) -> ReturnData,
    {
        let connection = Connection::connect_to_env()?;
        let (globals, _) = registry_queue_init::<BaseState>(&connection)?; // We just need the
                                                                           // global, the
                                                                           // event_queue is
                                                                           // not needed, we
                                                                           // do not need
                                                                           // BaseState after
                                                                           // this anymore

        let mut state = WindowState::default();

        let mut event_queue = connection.new_event_queue::<WindowState>();
        let qh = event_queue.handle();

        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?; // so the first
                                                                                // thing is to
                                                                                // get WlCompositor
        let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                                               // we need to create more

        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?;

        let _ = connection.display().get_registry(&qh, ()); // so if you want WlOutput, you need to
                                                            // register this

        event_queue.blocking_dispatch(&mut state)?; // then make a dispatch

        // do the step before, you get empty list

        // so it is the same way, to get surface detach to protocol, first get the shell, like wmbase
        // or layer_shell or session-shell, then get `surface` from the wl_surface you get before, and
        // set it
        // finally thing to remember is to commit the surface, make the shell to init.
        //let (init_w, init_h) = self.size;
        // this example is ok for both xdg_surface and layer_shell
        let layer_shell = globals
            .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
            .unwrap();
        let layer = layer_shell.get_layer_surface(
            &wl_surface,
            None,
            Layer::Top,
            "nobody".to_owned(),
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
        state.layer_shell = Some(layer);

        wl_surface.commit();

        // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
        // and if you need to reconfigure it, you need to commit the wl_surface again
        // so because this is just an example, so we just commit it once
        // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

        state.wl_surface = Some(wl_surface);

        self.state = state;
        'out: loop {
            event_queue.blocking_dispatch(&mut self.state)?;
            if self.state.message.is_empty() {
                continue;
            }
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut self.state.message);
            for msg in messages.iter() {
                match msg {
                    DispatchMessage::RefreshSurface { width, height } => {
                        if self.state.buffer.is_none() {
                            let mut file = tempfile::tempfile()?;
                            let ReturnData::WlBuffer(buffer) = event_hander(
                                LayerEvent::RequestBuffer(&mut file, &shm, &qh, *width, *height),
                                &mut self.state,
                            ) else {
                                panic!("You cannot return this one");
                            };
                            let surface = self.state.wl_surface.as_ref().unwrap();
                            surface.attach(Some(&buffer), 0, 0);
                            self.state.buffer = Some(buffer);
                        }
                        let surface = self.state.wl_surface.as_ref().unwrap();

                        surface.commit();
                    }
                    _ => {
                        if let ReturnData::RequestExist =
                            event_hander(LayerEvent::RequestMessages(msg), &mut self.state)
                        {
                            break 'out;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
