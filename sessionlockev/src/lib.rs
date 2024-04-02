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

pub mod key;

pub mod id;

use key::KeyModifierType;
use strtoshape::str_to_shape;

use std::fmt::Debug;

use events::DispatchMessageInner;

pub use events::{DispatchMessage, ReturnData, SessionLockEvent};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, BindError, GlobalError, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
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
    ConnectError, Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle, WEnum,
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

use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};

/// return the error during running the eventloop
#[derive(Debug, thiserror::Error)]
pub enum SessonLockEventError {
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
    pub mod wp_fractional_scale_v1 {
        pub use wayland_protocols::wp::fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1::{self, WpFractionalScaleManagerV1},
            wp_fractional_scale_v1::{self, WpFractionalScaleV1},
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
pub struct WindowWrapper {
    pub id: id::Id,
    display: WlDisplay,
    wl_surface: WlSurface,
}

impl WindowWrapper {
    pub fn id(&self) -> id::Id {
        self.id
    }
}

impl WindowWrapper {
    #[inline]
    pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandWindowHandle::new({
            let ptr = self.wl_surface.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_surface will never be null")
        })
        .into())
    }

    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandDisplayHandle::new({
            let ptr = self.display.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_proxy should never be null")
        })
        .into())
    }
}
impl rwh_06::HasWindowHandle for WindowWrapper {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_window_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::WindowHandle::borrow_raw(raw) })
    }
}

impl rwh_06::HasDisplayHandle for WindowWrapper {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_display_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::DisplayHandle::borrow_raw(raw) })
    }
}

#[derive(Debug)]
pub struct WindowStateUnit<T: Debug> {
    id: id::Id,
    display: WlDisplay,
    wl_surface: WlSurface,
    size: (u32, u32),
    buffer: Option<WlBuffer>,
    session_shell: ExtSessionLockSurfaceV1,
    fractional_scale: Option<WpFractionalScaleV1>,
    binding: Option<T>,
}

impl<T: Debug> WindowStateUnit<T> {
    pub fn id(&self) -> id::Id {
        self.id
    }
    pub fn gen_wrapper(&self) -> WindowWrapper {
        WindowWrapper {
            id: self.id,
            display: self.display.clone(),
            wl_surface: self.wl_surface.clone(),
        }
    }
}

impl<T: Debug> WindowStateUnit<T> {
    #[inline]
    pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandWindowHandle::new({
            let ptr = self.wl_surface.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_surface will never be null")
        })
        .into())
    }

    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandDisplayHandle::new({
            let ptr = self.display.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_proxy should never be null")
        })
        .into())
    }
}

/// This is the unit, binding to per screen.
/// Because ext-session-shell is so unique, on surface bind to only one
/// wl_output, only one buffer, only one output, so it will store
/// includes the information of ZxdgOutput, size, and layer_shell
///
/// and it can set a binding, you to store the related data. like
/// a cario_context, which is binding to the buffer on the wl_surface.
impl<T: Debug> WindowStateUnit<T> {
    /// get the wl surface from WindowState
    pub fn get_wlsurface(&self) -> &WlSurface {
        &self.wl_surface
    }
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

    connection: Option<Connection>,
    event_queue: Option<EventQueue<WindowState<T>>>,
    wl_compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    cursor_manager: Option<WpCursorShapeManagerV1>,
    lock: Option<ExtSessionLockV1>,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    globals: Option<GlobalList>,

    // base managers
    seat: Option<WlSeat>,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    touch: Option<WlTouch>,

    // keyboard
    modifier: KeyModifierType,
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
    pub fn gen_main_wrapper(&self) -> WindowWrapper {
        self.main_window().gen_wrapper()
    }
    // return the first window
    // I will use it in iced
    pub fn main_window(&self) -> &WindowStateUnit<T> {
        &self.units[0]
    }

    pub fn get_window_with_id(&self, id: id::Id) -> Option<&WindowStateUnit<T>> {
        self.units.iter().find(|w| w.id() == id)
    }
    // return all windows
    pub fn windows(&self) -> &Vec<WindowStateUnit<T>> {
        &self.units
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

            connection: None,
            event_queue: None,
            wl_compositor: None,
            shm: None,
            cursor_manager: None,
            fractional_scale_manager: None,
            lock: None,
            globals: None,

            seat: None,
            keyboard: None,
            pointer: None,
            touch: None,

            modifier: KeyModifierType::NoMod,
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
        match event {
            wl_keyboard::Event::Key {
                state: keystate,
                serial,
                key,
                time,
            } => {
                state.message.push((
                    state.surface_pos(),
                    DispatchMessageInner::KeyBoard {
                        state: keystate,
                        modifier: state.modifier,
                        serial,
                        key,
                        time,
                    },
                ));
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_locked,
                ..
            } => {
                state.modifier = KeyModifierType::from_bits(mods_depressed | mods_locked)
                    .unwrap_or(KeyModifierType::empty());
            }
            _ => {}
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

impl<T: Debug> Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        proxy: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: <wp_fractional_scale_v1::WpFractionalScaleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let Some(index) = state.units.iter().position(|info| {
                info.fractional_scale
                    .as_ref()
                    .is_some_and(|fractional_scale| fractional_scale == proxy)
            }) else {
                return;
            };
            state
                .message
                .push((Some(index), DispatchMessageInner::PrefredScale(scale)));
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

// fractional_scale_manager
delegate_noop!(@<T: Debug>WindowState<T>: ignore WpFractionalScaleManagerV1);

impl<T: Debug + 'static> WindowState<T> {
    pub fn build(mut self) -> Result<Self, SessonLockEventError> {
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
        self.shm = Some(shm);
        self.seat = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        let _ = connection.display().get_registry(&qh, ()); // so if you want WlOutput, you need to
                                                            // register this
        let fractional_scale_manager = globals
            .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let lock_manager = globals.bind::<ExtSessionLockManagerV1, _, _>(&qh, 1..=1, ())?;
        event_queue.blocking_dispatch(&mut self)?; // then make a dispatch
        let lock = lock_manager.lock(&qh, ());
        let displays = self.outputs.clone();
        for (_, display) in displays.iter() {
            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            wl_surface.commit();
            let session_lock_surface = lock.get_lock_surface(&wl_surface, display, &qh, ());

            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
            let mut fractional_scale = None;
            if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                fractional_scale =
                    Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
            }

            self.units.push(WindowStateUnit {
                id: id::Id::unique(),
                display: connection.display(),
                wl_surface,
                size: (0, 0),
                buffer: None,
                session_shell: session_lock_surface,
                fractional_scale,
                binding: None,
            });
        }
        self.connection = Some(connection);
        self.event_queue = Some(event_queue);
        self.wl_compositor = Some(wmcompositer);
        self.cursor_manager = cursor_manager;
        self.lock = Some(lock);
        self.fractional_scale_manager = fractional_scale_manager;
        self.globals = Some(globals);
        Ok(self)
    }
    /// main event loop, every time dispatch, it will store the messages, and do callback. it will
    /// pass a LayerEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit] if the even is not spical on one surface,
    /// it will return [None].
    pub fn running<F>(&mut self, mut event_hander: F) -> Result<(), SessonLockEventError>
    where
        F: FnMut(SessionLockEvent<T, ()>, &mut WindowState<T>, Option<usize>) -> ReturnData,
    {
        let globals = self.globals.take().unwrap();
        let mut event_queue = self.event_queue.take().unwrap();
        let qh = event_queue.handle();
        let wmcompositer = self.wl_compositor.take().unwrap();
        let shm = self.shm.take().unwrap();
        let fractional_scale_manager = self.fractional_scale_manager.take();
        let cursor_manager: Option<WpCursorShapeManagerV1> = self.cursor_manager.take();
        let connection = self.connection.take().unwrap();
        let lock = self.lock.take().unwrap();
        let mut init_event = None;

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event = Some(event_hander(SessionLockEvent::InitRequest, self, None));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(event_hander(
                        SessionLockEvent::BindProvide(&globals, &qh),
                        self,
                        None,
                    ));
                }
                _ => panic!("Not privide server here"),
            }
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
                                SessionLockEvent::RequestBuffer(
                                    &mut file, &shm, &qh, *width, *height,
                                ),
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
                                SessionLockEvent::RequestMessages(
                                    &DispatchMessage::RequestRefresh {
                                        width: *width,
                                        height: *height,
                                    },
                                ),
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
                            lock.get_lock_surface(&wl_surface, display, &qh, ());

                        let mut fractional_scale = None;
                        if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                            fractional_scale = Some(fractional_scale_manager.get_fractional_scale(
                                &wl_surface,
                                &qh,
                                (),
                            ));
                        }
                        // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                        // and if you need to reconfigure it, you need to commit the wl_surface again
                        // so because this is just an example, so we just commit it once
                        // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                        self.units.push(WindowStateUnit {
                            id: id::Id::unique(),
                            display: connection.display(),
                            wl_surface,
                            size: (0, 0),
                            buffer: None,
                            session_shell: session_lock_surface,
                            fractional_scale,
                            binding: None,
                        });
                    }
                    _ => {
                        let (index_message, msg) = msg;
                        let msg: DispatchMessage = msg.clone().into();
                        match event_hander(
                            SessionLockEvent::RequestMessages(&msg),
                            self,
                            *index_message,
                        ) {
                            ReturnData::RequestUnlockAndExist => {
                                lock.unlock_and_destroy();
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
