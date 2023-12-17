//! # Handle the ext_session_lock with a winit way
//!
//! Min example is under
//!
//! ```rust, no_run
//! use std::fs::File;
//! use std::os::fd::AsFd;
//!
//! use sessionlockev::reexport::*;
//! use sessionlockev::*;
//!
//! const ESC_KEY: u32 = 1;
//!
//! fn main() {
//!     let mut ev: WindowState<()> = WindowState::new();
//!
//!     let mut virtual_keyboard_manager = None;
//!     ev.running(|event, _ev, _index| {
//!         println!("{:?}", event);
//!         match event {
//!             // NOTE: this will send when init, you can request bind extra object from here
//!             LayerEvent::InitRequest => ReturnData::RequestBind,
//!             LayerEvent::BindProvide(globals, qh) => {
//!                 // NOTE: you can get implied wayland object from here
//!                 virtual_keyboard_manager = Some(
//!                     globals
//!                         .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
//!                             qh,
//!                             1..=1,
//!                             (),
//!                         )
//!                         .unwrap(),
//!                 );
//!                 println!("{:?}", virtual_keyboard_manager);
//!                 ReturnData::RequestLock
//!             }
//!             LayerEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
//!                 draw(file, (init_w, init_h));
//!                 let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
//!                 ReturnData::WlBuffer(pool.create_buffer(
//!                     0,
//!                     init_w as i32,
//!                     init_h as i32,
//!                     (init_w * 4) as i32,
//!                     wl_shm::Format::Argb8888,
//!                     qh,
//!                     (),
//!                 ))
//!             }
//!             LayerEvent::RequestMessages(DispatchMessage::RequestRefresh { width, height }) => {
//!                 println!("{width}, {height}");
//!                 ReturnData::None
//!             }
//!             LayerEvent::RequestMessages(DispatchMessage::MouseButton { .. }) => ReturnData::None,
//!             LayerEvent::RequestMessages(DispatchMessage::MouseEnter {
//!                 serial, pointer, ..
//!             }) => ReturnData::RequestSetCursorShape((
//!                 "crosshair".to_owned(),
//!                 pointer.clone(),
//!                 *serial,
//!             )),
//!             LayerEvent::RequestMessages(DispatchMessage::KeyBoard { key, .. }) => {
//!                 if *key == ESC_KEY {
//!                     return ReturnData::RequestUnlockAndExist;
//!                 }
//!                 ReturnData::None
//!             }
//!             LayerEvent::RequestMessages(DispatchMessage::MouseMotion {
//!                 time,
//!                 surface_x,
//!                 surface_y,
//!             }) => {
//!                 println!("{time}, {surface_x}, {surface_y}");
//!                 ReturnData::None
//!             }
//!             _ => ReturnData::None,
//!         }
//!     })
//!     .unwrap();
//! }
//!
//! fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
//!     use std::{cmp::min, io::Write};
//!     let mut buf = std::io::BufWriter::new(tmp);
//!     for y in 0..buf_y {
//!         for x in 0..buf_x {
//!             let a = 0xFF;
//!             let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
//!             let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
//!             let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
//!
//!             let color = (a << 24) + (r << 16) + (g << 8) + b;
//!             buf.write_all(&color.to_ne_bytes()).unwrap();
//!         }
//!     }
//!     buf.flush().unwrap();
//! }
//! ```

mod events;

mod strtoshape;

use strtoshape::str_to_shape;

use std::fmt::Debug;

use events::DispatchMessageInner;

pub use events::{DispatchMessage, LayerEvent, ReturnData};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, BindError, GlobalError, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_keyboard::{self, WlKeyboard},
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry,
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
        wl_touch::{self, WlTouch},
    },
    ConnectError, Connection, Dispatch, DispatchError, Proxy, QueueHandle, WEnum,
};

use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
    ext_session_lock_v1::ExtSessionLockV1,
};

use wayland_cursor::{CursorImageBuffer, CursorTheme};
use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
    wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// return the error during running the eventloop
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

/// reexport the wayland objects which are needed
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
        pub use wayland_client::{
            globals::GlobalList,
            protocol::{
                wl_keyboard::{self, KeyState},
                wl_pointer::{self, ButtonState},
                wl_seat::WlSeat,
            },
            QueueHandle, WEnum,
        };
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
pub struct WindowStateUnit<T: Debug> {
    wl_surface: WlSurface,
    size: (u32, u32),
    buffer: Option<WlBuffer>,
    session_shell: ExtSessionLockSurfaceV1,
    binding: Option<T>,
}

/// This is the unit, binding to per screen.
/// Because ext-session-shell is so unique, on surface bind to only one
/// wl_output, only one buffer, only one output, so it will store
/// includes the information of ZxdgOutput, size, and layer_shell
///
/// and it can set a binding, you to store the related data. like
/// a cario_context, which is binding to the buffer on the wl_surface.
impl<T: Debug> WindowStateUnit<T> {
    /// set the data binding to the unit
    pub fn set_binding(&mut self, binding: T) {
        self.binding = Some(binding);
    }

    /// the the unit binding data with mut way
    pub fn get_binding_mut(&mut self) -> Option<&mut T> {
        self.binding.as_mut()
    }

    /// this function will refresh whole surface. it will reattach the buffer, and damage whole,
    /// and finall commit
    pub fn request_refresh(&self, (width, height): (i32, i32)) {
        self.wl_surface.attach(self.buffer.as_ref(), 0, 0);
        self.wl_surface.damage(0, 0, width, height);
        self.wl_surface.commit();
    }
}

#[derive(Debug)]
pub struct WindowState<T: Debug> {
    outputs: Vec<(u32, wl_output::WlOutput)>,
    current_surface: Option<WlSurface>,
    units: Vec<WindowStateUnit<T>>,
    message: Vec<(Option<usize>, DispatchMessageInner)>,

    // base managers
    seat: Option<WlSeat>,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    touch: Option<WlTouch>,
}

impl<T: Debug> WindowState<T> {
    /// get a seat from state
    pub fn get_seat(&self) -> &WlSeat {
        self.seat.as_ref().unwrap()
    }

    /// get the keyboard
    pub fn get_keyboard(&self) -> Option<&WlKeyboard> {
        self.keyboard.as_ref()
    }

    /// get the pointer
    pub fn get_pointer(&self) -> Option<&WlPointer> {
        self.pointer.as_ref()
    }

    /// get the touch
    pub fn get_touch(&self) -> Option<&WlTouch> {
        self.touch.as_ref()
    }
}

impl<T: Debug> WindowState<T> {
    /// create a new WindowState
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: Debug> Default for WindowState<T> {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            current_surface: None,
            units: Vec::new(),
            message: Vec::new(),

            seat: None,
            keyboard: None,
            pointer: None,
            touch: None,
        }
    }
}

impl<T: Debug> WindowState<T> {
    /// get the unit with the index returned by eventloop
    pub fn get_unit(&mut self, index: usize) -> &mut WindowStateUnit<T> {
        &mut self.units[index]
    }

    /// it return the iter of units. you can do loop with it
    pub fn get_unit_iter(&self) -> impl Iterator<Item = &WindowStateUnit<T>> {
        self.units.iter()
    }

    /// it return the mut iter of units. you can do loop with it
    pub fn get_unit_iter_mut(&mut self) -> impl Iterator<Item = &mut WindowStateUnit<T>> {
        self.units.iter_mut()
    }

    fn surface_pos(&self) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref())
    }

    fn get_pos_from_surface(&self, surface: &WlSurface) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| &unit.wl_surface == surface)
    }
}

impl<T: Debug + 'static> Dispatch<wl_registry::WlRegistry, ()> for WindowState<T> {
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
                        .push((None, DispatchMessageInner::NewDisplay(output)));
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

impl<T: Debug + 'static> Dispatch<wl_seat::WlSeat, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
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
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
            if capabilities.contains(wl_seat::Capability::Touch) {
                state.touch = Some(seat.get_touch(qh, ()));
            }
        }
    }
}

impl<T: Debug> Dispatch<wl_keyboard::WlKeyboard, ()> for WindowState<T> {
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
                DispatchMessageInner::KeyBoard {
                    state: keystate,
                    serial,
                    key,
                    time,
                },
            ));
        }
    }
}

impl<T: Debug> Dispatch<wl_touch::WlTouch, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &wl_touch::WlTouch,
        event: <wl_touch::WlTouch as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_touch::Event::Down {
                serial,
                time,
                surface,
                id,
                x,
                y,
            } => state.message.push((
                state.get_pos_from_surface(&surface),
                DispatchMessageInner::TouchDown {
                    serial,
                    time,
                    id,
                    x,
                    y,
                },
            )),
            wl_touch::Event::Up { serial, time, id } => state
                .message
                .push((None, DispatchMessageInner::TouchUp { serial, time, id })),
            wl_touch::Event::Motion { time, id, x, y } => state
                .message
                .push((None, DispatchMessageInner::TouchMotion { time, id, x, y })),
            _ => {}
        }
    }
}

impl<T: Debug> Dispatch<wl_pointer::WlPointer, ()> for WindowState<T> {
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
                    DispatchMessageInner::MouseButton {
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
                    DispatchMessageInner::MouseEnter {
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
                    DispatchMessageInner::MouseMotion {
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

impl<T: Debug> Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, ()>
    for WindowState<T>
{
    fn event(
        state: &mut Self,
        surface: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
        event: <ext_session_lock_surface_v1::ExtSessionLockSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            surface.ack_configure(serial);

            let Some(unit_index) = state
                .units
                .iter()
                .position(|unit| unit.session_shell == *surface)
            else {
                return;
            };
            state.units[unit_index].size = (width, height);

            state.message.push((
                Some(unit_index),
                DispatchMessageInner::RefreshSurface { width, height },
            ));
        }
    }
}

delegate_noop!(@<T: Debug>WindowState<T>: ignore WlCompositor); // WlCompositor is need to create a surface
delegate_noop!(@<T: Debug>WindowState<T>: ignore WlSurface); // surface is the base needed to show buffer
delegate_noop!(@<T: Debug>WindowState<T>: ignore WlOutput); // output is need to place layer_shell, although here
                                                            // it is not used
delegate_noop!(@<T: Debug>WindowState<T>: ignore WlShm); // shm is used to create buffer pool
delegate_noop!(@<T: Debug>WindowState<T>: ignore WlShmPool); // so it is pool, created by wl_shm
delegate_noop!(@<T: Debug>WindowState<T>: ignore WlBuffer); // buffer show the picture
                                                            //

delegate_noop!(@<T: Debug>WindowState<T>: ignore ExtSessionLockV1); // buffer show the picture
delegate_noop!(@<T: Debug>WindowState<T>: ignore ExtSessionLockManagerV1); // buffer show the picture

delegate_noop!(@<T: Debug>WindowState<T>: ignore WpCursorShapeManagerV1);
delegate_noop!(@<T: Debug>WindowState<T>: ignore WpCursorShapeDeviceV1);

delegate_noop!(@<T: Debug>WindowState<T>: ignore ZwpVirtualKeyboardV1);
delegate_noop!(@<T: Debug>WindowState<T>: ignore ZwpVirtualKeyboardManagerV1);

impl<T: Debug + 'static> WindowState<T> {
    /// main event loop, every time dispatch, it will store the messages, and do callback. it will
    /// pass a LayerEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit] if the even is not spical on one surface,
    /// it will return [None].
    pub fn running<F>(&mut self, mut event_hander: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerEvent<T>, &mut WindowState<T>, Option<usize>) -> ReturnData,
    {
        let connection = Connection::connect_to_env()?;
        let (globals, _) = registry_queue_init::<BaseState>(&connection)?; // We just need the
                                                                           // global, the
                                                                           // event_queue is
                                                                           // not needed, we
                                                                           // do not need
                                                                           // BaseState after
                                                                           // this anymore

        let mut event_queue = connection.new_event_queue::<WindowState<T>>();
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

        let lock_manager = globals.bind::<ExtSessionLockManagerV1, _, _>(&qh, 1..=1, ())?;
        let mut lock: Option<ExtSessionLockV1> = None;
        event_queue.blocking_dispatch(self)?; // then make a dispatch

        let mut init_event = None;

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event = Some(event_hander(LayerEvent::InitRequest, self, None));
                }
                Some(ReturnData::RequestLock) => {
                    lock = Some(lock_manager.lock(&qh, ()));
                    break;
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

        let displays = self.outputs.clone();
        for (_, display) in displays.iter() {
            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            wl_surface.commit();
            let session_lock_surface =
                lock.as_ref()
                    .unwrap()
                    .get_lock_surface(&wl_surface, display, &qh, ());

            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

            self.units.push(WindowStateUnit {
                wl_surface,
                size: (0, 0),
                buffer: None,
                session_shell: session_lock_surface,
                binding: None,
            });
        }
        self.message.clear();
        'out: loop {
            event_queue.blocking_dispatch(self)?;
            if self.message.is_empty() {
                continue;
            }
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut self.message);
            for msg in messages.iter() {
                match msg {
                    (Some(unit_index), DispatchMessageInner::RefreshSurface { width, height }) => {
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
                            event_hander(
                                LayerEvent::RequestMessages(&DispatchMessage::RequestRefresh {
                                    width: *width,
                                    height: *height,
                                }),
                                self,
                                Some(index),
                            );
                        }
                        let surface = &self.units[index].wl_surface;

                        surface.commit();
                    }
                    (_, DispatchMessageInner::NewDisplay(display)) => {
                        let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                                                               //
                        wl_surface.commit();
                        let session_lock_surface =
                            lock.as_ref()
                                .unwrap()
                                .get_lock_surface(&wl_surface, display, &qh, ());

                        // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                        // and if you need to reconfigure it, you need to commit the wl_surface again
                        // so because this is just an example, so we just commit it once
                        // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                        self.units.push(WindowStateUnit {
                            wl_surface,
                            size: (0, 0),
                            buffer: None,
                            session_shell: session_lock_surface,
                            binding: None,
                        });
                    }
                    _ => {
                        let (index_message, msg) = msg;
                        let msg: DispatchMessage = msg.clone().into();
                        match event_hander(LayerEvent::RequestMessages(&msg), self, *index_message)
                        {
                            ReturnData::RequestUnlockAndExist => {
                                lock.as_ref().unwrap().unlock_and_destroy();
                                event_queue.blocking_dispatch(self)?;
                                break 'out;
                            }
                            ReturnData::RequestSetCursorShape((shape_name, pointer, serial)) => {
                                if let Some(ref cursor_manager) = cursor_manager {
                                    let Some(shape) = str_to_shape(&shape_name) else {
                                        eprintln!("Not supported shape");
                                        continue;
                                    };
                                    let device = cursor_manager.get_pointer(&pointer, &qh, ());
                                    device.set_shape(serial, shape);
                                    device.destroy();
                                } else {
                                    let Some(cursor_buffer) =
                                        get_cursor_buffer(&shape_name, &connection, &shm)
                                    else {
                                        eprintln!("Cannot find cursor {shape_name}");
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
