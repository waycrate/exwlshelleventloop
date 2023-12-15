use std::fs::File;

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
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
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel::XdgToplevel};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

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
        if let zwlr_layer_surface_v1::Event::Configure { serial, .. } = event {
            surface.ack_configure(serial);
            let surface = state.wl_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
            }
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
pub enum Event<'a> {
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage),
}

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
}

#[derive(Debug)]
pub struct EventLoop {
    keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    anchor: Anchor,
    size: (u32, u32),
    exclusive_zone: Option<i32>,
    state: WindowState,
}

impl EventLoop {
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
        self.size = size;
        self
    }

    pub fn with_exclusize_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        EventLoop {
            keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right | Anchor::Bottom,
            size: (100, 100),
            exclusive_zone: None,
            state: WindowState::default(),
        }
    }
}

impl EventLoop {
    pub fn running<F>(&mut self, mut event_hander: F)
    where
        F: FnMut(Event, &mut WindowState) -> ReturnData,
    {
        let connection = Connection::connect_to_env().unwrap();
        let (globals, _) = registry_queue_init::<BaseState>(&connection).unwrap(); // We just need the
                                                                                   // global, the
                                                                                   // event_queue is
                                                                                   // not needed, we
                                                                                   // do not need
                                                                                   // BaseState after
                                                                                   // this anymore

        let mut state = WindowState::default();

        let mut event_queue = connection.new_event_queue::<WindowState>();
        let qh = event_queue.handle();

        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ()).unwrap(); // so the first
                                                                                        // thing is to
                                                                                        // get WlCompositor
        let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                                               // we need to create more

        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ()).unwrap();
        globals.bind::<WlSeat, _, _>(&qh, 1..=1, ()).unwrap();

        let _ = connection.display().get_registry(&qh, ()); // so if you want WlOutput, you need to
                                                            // register this

        event_queue.blocking_dispatch(&mut state).unwrap(); // then make a dispatch

        // do the step before, you get empty list

        // so it is the same way, to get surface detach to protocol, first get the shell, like wmbase
        // or layer_shell or session-shell, then get `surface` from the wl_surface you get before, and
        // set it
        // finally thing to remember is to commit the surface, make the shell to init.
        let (init_w, init_h) = self.size;
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
        layer.set_size(init_w, init_w);
        if let Some(zone) = self.exclusive_zone {
            layer.set_exclusive_zone(zone);
        }
        state.layer_shell = Some(layer);

        wl_surface.commit();

        // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
        // and if you need to reconfigure it, you need to commit the wl_surface again
        // so because this is just an example, so we just commit it once
        // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

        let mut file = tempfile::tempfile().unwrap();
        let ReturnData::WlBuffer(buffer) = event_hander(
            Event::RequestBuffer(&mut file, &shm, &qh, init_w, init_h),
            &mut self.state,
        ) else {
            panic!("You cannot return this one");
        };

        state.wl_surface = Some(wl_surface);
        state.buffer = Some(buffer);
        self.state = state;
        'out: loop {
            event_queue.blocking_dispatch(&mut self.state).unwrap();
            if self.state.message.is_empty() {
                continue;
            }
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut self.state.message);
            for msg in messages.iter() {
                if let ReturnData::RequestExist =
                    event_hander(Event::RequestMessages(msg), &mut self.state)
                {
                    break 'out;
                }
            }
        }
    }
}
