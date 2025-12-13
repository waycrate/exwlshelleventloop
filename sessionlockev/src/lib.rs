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
//! use sessionlockev::keyboard::{KeyCode, PhysicalKey};
//!
//! const ESC_KEY: u32 = 1;
//!
//! fn main() {
//!     let ev: WindowState<()> = WindowState::new();
//!
//!     ev.running(|event, _ev, _index| {
//!         println!("{:?}", event);
//!         match event {
//!             // NOTE: this will send when init, you can request bind extra object from here
//!             SessionLockEvent::InitRequest => ReturnData::RequestBind,
//!             SessionLockEvent::BindProvide(globals, qh) => {
//!                 // NOTE: you can get implied wayland object from here
//!                 let virtual_keyboard_manager = globals
//!                         .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
//!                             qh,
//!                             1..=1,
//!                             (),
//!                         )
//!                         .unwrap();
//!                 println!("{:?}", virtual_keyboard_manager);
//!                 ReturnData::None
//!             }
//!             SessionLockEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
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
//!             SessionLockEvent::RequestMessages(DispatchMessage::RequestRefresh { width, height,scale_float }) => {
//!                 println!("{width}, {height}, {scale_float}");
//!                 ReturnData::None
//!             }
//!             SessionLockEvent::RequestMessages(DispatchMessage::MouseButton { .. }) => ReturnData::None,
//!             SessionLockEvent::RequestMessages(DispatchMessage::MouseEnter {
//!                 pointer, ..
//!             }) => ReturnData::RequestSetCursorShape((
//!                 "crosshair".to_owned(),
//!                 pointer.clone(),
//!             )),
//!             SessionLockEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
//!                if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
//!                    ReturnData::RequestUnlockAndExist
//!                } else {
//!                    ReturnData::None
//!                }
//!            }
//!             SessionLockEvent::RequestMessages(DispatchMessage::MouseMotion {
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

use calloop::RegistrationToken;
pub use waycrate_xkbkeycode::keyboard;
pub use waycrate_xkbkeycode::xkb_keyboard;
use waycrate_xkbkeycode::xkb_keyboard::ElementState;
use waycrate_xkbkeycode::xkb_keyboard::RepeatInfo;

mod strtoshape;

pub mod id;

use strtoshape::str_to_shape;

use events::{AxisScroll, DispatchMessageInner};

pub use events::{DispatchMessage, ReturnData, SessionLockEvent};

use wayland_client::protocol::wl_callback::WlCallback;
use wayland_client::{
    ConnectError, Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle, WEnum,
    delegate_noop,
    globals::{BindError, GlobalError, GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_keyboard::{self, KeyState, KeymapFormat, WlKeyboard},
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_registry,
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
        wl_touch::{self, WlTouch},
    },
};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
    ext_session_lock_v1::ExtSessionLockV1,
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::Instant;

pub use calloop;

use calloop::{
    Error as CallLoopError, EventLoop, LoopHandle,
    timer::{TimeoutAction, Timer},
};
use calloop_wayland_source::WaylandSource;

use wayland_client::backend::WaylandError;

/// return the error during running the eventloop
#[derive(Debug, thiserror::Error)]
pub enum SessionLockEventError {
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
    #[error("Event Loop Error")]
    EventLoopInitError(#[from] CallLoopError),
    #[error("roundtrip Error")]
    RoundTripError(#[from] WaylandError),
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
            Connection, QueueHandle, WEnum,
            globals::GlobalList,
            protocol::{
                wl_keyboard::{self, KeyState},
                wl_pointer::{self, ButtonState},
                wl_seat::WlSeat,
            },
        };
    }
    pub mod wp_cursor_shape_device_v1 {
        pub use crate::strtoshape::ShapeName;
        pub use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
    }
    pub mod wp_viewport {
        pub use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
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
    pub viewport: Option<WpViewport>,
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
pub struct WindowStateUnit<T> {
    id: id::Id,
    display: WlDisplay,
    wl_surface: WlSurface,
    size: (u32, u32),
    buffer: Option<WlBuffer>,
    session_shell: ExtSessionLockSurfaceV1,
    fractional_scale: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
    binding: Option<T>,
    qh: QueueHandle<WindowState<T>>,

    scale: u32,
    present_available_state: PresentAvailableState,
    refresh: RefreshRequest,
}

impl<T> WindowStateUnit<T> {
    pub fn id(&self) -> id::Id {
        self.id
    }
    pub fn try_set_viewport_destination(&self, width: i32, height: i32) -> Option<()> {
        let viewport = self.viewport.as_ref()?;
        viewport.set_destination(width, height);
        Some(())
    }

    pub fn try_set_viewport_source(&self, x: f64, y: f64, width: f64, height: f64) -> Option<()> {
        let viewport = self.viewport.as_ref()?;
        viewport.set_source(x, y, width, height);
        Some(())
    }

    pub fn gen_wrapper(&self) -> WindowWrapper {
        WindowWrapper {
            id: self.id,
            display: self.display.clone(),
            wl_surface: self.wl_surface.clone(),
            viewport: self.viewport.clone(),
        }
    }
}

impl<T> WindowStateUnit<T> {
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
impl<T> WindowStateUnit<T> {
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

    pub fn get_binding(&self) -> Option<&T> {
        self.binding.as_ref()
    }

    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    pub fn scale_u32(&self) -> u32 {
        self.scale
    }

    pub fn scale_float(&self) -> f64 {
        self.scale as f64 / 120.
    }

    /// this function will refresh whole surface. it will reattach the buffer, and damage whole,
    /// and final commit
    pub fn request_refresh(&mut self, request: RefreshRequest) {
        // refresh request in nearest future has the highest priority.
        match self.refresh {
            RefreshRequest::NextFrame => {}
            RefreshRequest::At(instant) => match request {
                RefreshRequest::NextFrame => self.refresh = request,
                RefreshRequest::At(other_instant) => {
                    if other_instant < instant {
                        self.refresh = request;
                    }
                }
                RefreshRequest::Wait => {}
            },
            RefreshRequest::Wait => self.refresh = request,
        }
    }

    fn should_refresh(&self) -> bool {
        match self.refresh {
            RefreshRequest::NextFrame => true,
            RefreshRequest::At(instant) => instant <= Instant::now(),
            RefreshRequest::Wait => false,
        }
    }

    pub fn take_present_slot(&mut self) -> bool {
        if !self.should_refresh() {
            return false;
        }
        if self.present_available_state != PresentAvailableState::Available {
            return false;
        }
        self.refresh = RefreshRequest::Wait;
        self.present_available_state = PresentAvailableState::Taken;
        true
    }

    pub fn reset_present_slot(&mut self) -> bool {
        if self.present_available_state == PresentAvailableState::Taken {
            self.present_available_state = PresentAvailableState::Available;
            true
        } else {
            false
        }
    }
}
impl<T: 'static> WindowStateUnit<T> {
    pub fn request_next_present(&mut self) {
        match self.present_available_state {
            PresentAvailableState::Taken => {
                self.present_available_state = PresentAvailableState::Requested;
                self.wl_surface
                    .frame(&self.qh, (self.id, PresentAvailableState::Available));
            }
            PresentAvailableState::Requested | PresentAvailableState::Available => {}
        }
    }
}

#[derive(Debug)]
struct KeyboardTokenState {
    delay: Duration,
    key: u32,
    surface_id: Option<id::Id>,
    pressed_state: ElementState,
}

#[derive(Debug)]
pub struct WindowState<T> {
    outputs: Vec<(u32, wl_output::WlOutput)>,
    current_surface: Option<WlSurface>,
    active_surfaces: HashMap<Option<i32>, (WlSurface, Option<id::Id>)>,
    units: Vec<WindowStateUnit<T>>,
    message: Vec<(Option<id::Id>, DispatchMessageInner)>,

    connection: Option<Connection>,
    event_queue: Option<EventQueue<WindowState<T>>>,
    wl_compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    cursor_manager: Option<WpCursorShapeManagerV1>,
    viewporter: Option<WpViewporter>,
    lock: Option<ExtSessionLockV1>,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    globals: Option<GlobalList>,

    // base managers
    seat: Option<WlSeat>,
    keyboard_state: Option<xkb_keyboard::KeyboardState>,
    pointer: Option<WlPointer>,
    touch: Option<WlTouch>,

    // settings:
    to_remove_tokens: Vec<RegistrationToken>,
    repeat_delay: Option<KeyboardTokenState>,
    closed_ids: Vec<id::Id>,

    // keyboard
    use_display_handle: bool,

    finger_locations: HashMap<i32, (f64, f64)>,
    enter_serial: Option<u32>,

    return_data: Vec<ReturnData>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefreshRequest {
    /// Redraw the next frame.
    NextFrame,

    /// Redraw at the given time.
    At(Instant),

    /// No redraw is needed.
    #[default]
    Wait,
}

impl<T> WindowState<T> {
    /// get a seat from state
    pub fn get_seat(&self) -> &WlSeat {
        self.seat.as_ref().unwrap()
    }

    /// get the keyboard
    pub fn get_keyboard(&self) -> Option<&WlKeyboard> {
        Some(&self.keyboard_state.as_ref()?.keyboard)
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

impl<T> WindowState<T> {
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

    fn push_window(&mut self, window_state_unit: WindowStateUnit<T>) {
        let surface = window_state_unit.wl_surface.clone();
        self.units.push(window_state_unit);
        // new created surface will be current_surface.
        self.update_current_surface(Some(surface));
    }
}

impl<T> WindowState<T> {
    /// create a new WindowState
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_return_data(&mut self, data: ReturnData) {
        self.return_data.push(data);
    }

    pub fn handle_event<F, Message>(
        &mut self,
        mut event_handler: F,
        event: SessionLockEvent<T, Message>,
        unit_id: Option<id::Id>,
    ) where
        Message: std::marker::Send + 'static,
        F: FnMut(SessionLockEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData,
    {
        let return_data = event_handler(event, self, unit_id);
        if !matches!(return_data, ReturnData::None) {
            self.append_return_data(return_data);
        }
    }
}

impl<T> Default for WindowState<T> {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            current_surface: None,
            active_surfaces: HashMap::new(),
            units: Vec::new(),
            message: Vec::new(),

            connection: None,
            event_queue: None,
            wl_compositor: None,
            shm: None,
            cursor_manager: None,
            viewporter: None,
            fractional_scale_manager: None,
            lock: None,
            globals: None,

            seat: None,
            keyboard_state: None,
            pointer: None,
            touch: None,

            to_remove_tokens: Vec::new(),
            repeat_delay: None,
            closed_ids: Vec::new(),

            use_display_handle: false,

            finger_locations: HashMap::new(),
            enter_serial: None,

            return_data: Vec::new(),
        }
    }
}

impl<T> WindowState<T> {
    pub fn get_id_list(&self) -> Vec<id::Id> {
        self.units.iter().map(|unit| unit.id).collect()
    }
    /// it return the iter of units. you can do loop with it
    pub fn get_unit_iter(&self) -> impl Iterator<Item = &WindowStateUnit<T>> {
        self.units.iter()
    }

    /// it return the mut iter of units. you can do loop with it
    pub fn get_unit_iter_mut(&mut self) -> impl Iterator<Item = &mut WindowStateUnit<T>> {
        self.units.iter_mut()
    }

    /// use [id::Id] to get the mut [WindowStateUnit]
    pub fn get_mut_unit_with_id(&mut self, id: id::Id) -> Option<&mut WindowStateUnit<T>> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    /// use [id::Id] to get the immutable [WindowStateUnit]
    pub fn get_unit_with_id(&self, id: id::Id) -> Option<&WindowStateUnit<T>> {
        self.units.iter().find(|unit| unit.id == id)
    }

    fn current_surface_id(&self) -> Option<id::Id> {
        self.units
            .iter()
            .find(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref())
            .map(|unit| unit.id())
    }

    /// use display_handle to render surface, not to create buffer yourself
    pub fn with_use_display_handle(mut self, use_display_handle: bool) -> Self {
        self.use_display_handle = use_display_handle;
        self
    }
    /// set a callback to create a wayland connection
    pub fn with_connection(mut self, connection_or: Option<Connection>) -> Self {
        self.connection = connection_or;
        self
    }

    fn get_id_from_surface(&self, surface: &WlSurface) -> Option<id::Id> {
        self.units
            .iter()
            .find(|unit| &unit.wl_surface == surface)
            .map(|unit| unit.id())
    }

    pub fn is_mouse_surface(&self, surface_id: id::Id) -> bool {
        self.active_surfaces
            .get(&None)
            .filter(|(_, id)| *id == Some(surface_id))
            .is_some()
    }

    /// update `current_surface` only if a finger is down or a mouse button is clicked or a surface
    /// is created.
    fn update_current_surface(&mut self, surface: Option<WlSurface>) {
        if surface == self.current_surface {
            return;
        }
        if let Some(surface) = surface {
            self.current_surface = Some(surface);

            // reset repeat when surface is changed
            if let Some(keyboard_state) = self.keyboard_state.as_mut() {
                keyboard_state.current_repeat = None;
            }
        }
    }
}
impl<T: 'static> WindowState<T> {
    pub fn request_next_present(&mut self, id: id::Id) {
        self.get_mut_unit_with_id(id)
            .map(WindowStateUnit::request_next_present);
    }

    pub fn reset_present_slot(&mut self, id: id::Id) {
        self.get_mut_unit_with_id(id)
            .map(WindowStateUnit::reset_present_slot);
    }
    pub fn request_refresh_all(&mut self, request: RefreshRequest) {
        self.units
            .iter_mut()
            .for_each(|unit| unit.request_refresh(request));
    }

    pub fn request_refresh(&mut self, id: id::Id, request: RefreshRequest) {
        if let Some(unit) = self.get_mut_unit_with_id(id) {
            unit.request_refresh(request);
        }
    }
}
impl<T: 'static> Dispatch<wl_registry::WlRegistry, ()> for WindowState<T> {
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
                let removed_states = state
                    .units
                    .extract_if(.., |unit| !unit.wl_surface.is_alive());
                for deleled in removed_states.into_iter() {
                    state.closed_ids.push(deleled.id);
                }
            }

            _ => {}
        }
    }
}

impl<T: 'static> Dispatch<wl_seat::WlSeat, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: <wl_seat::WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        use xkb_keyboard::KeyboardState;
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.is_empty() && state.keyboard_state.is_some() {
                let keyboard = state.keyboard_state.take().unwrap();
                drop(keyboard);
            }
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                if state.keyboard_state.is_none() {
                    state.keyboard_state = Some(KeyboardState::new(seat.get_keyboard(qh, ())));
                } else {
                    let keyboard = state.keyboard_state.take().unwrap();
                    state.keyboard_state = Some(keyboard.update(seat, qh, ()));
                }
                if let Some(surface_id) = state.current_surface_id() {
                    state
                        .message
                        .push((Some(surface_id), DispatchMessageInner::UnFocused));
                }
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                if state.pointer.is_none() {
                    state.pointer = Some(seat.get_pointer(qh, ()));
                } else {
                    let pointer = state.pointer.take().unwrap();
                    if pointer.version() >= 3 {
                        pointer.release();
                    }
                }
            }
            if capabilities.contains(wl_seat::Capability::Touch) {
                if state.touch.is_none() {
                    state.touch = Some(seat.get_touch(qh, ()));
                } else {
                    let touch = state.touch.take().unwrap();
                    if touch.version() >= 3 {
                        touch.release();
                    }
                }
            }
        }
    }
}

impl<T> Dispatch<wl_keyboard::WlKeyboard, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if state.keyboard_state.is_none() {
            return;
        }

        use keyboard::*;
        use xkb_keyboard::ElementState;
        let surface_id = state.current_surface_id();
        let keyboard_state = state.keyboard_state.as_mut().unwrap();
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => match format {
                WEnum::Value(KeymapFormat::XkbV1) => {
                    let context = &mut keyboard_state.xkb_context;
                    context.set_keymap_from_fd(fd, size as usize)
                }
                WEnum::Value(KeymapFormat::NoKeymap) => {
                    log::warn!("non-xkb compatible keymap")
                }
                _ => unreachable!(),
            },
            wl_keyboard::Event::Enter { .. } => {
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Leave { .. } => {
                keyboard_state.current_repeat = None;
                state.message.push((
                    surface_id,
                    DispatchMessageInner::ModifiersChanged(ModifiersState::empty()),
                ));
                state
                    .message
                    .push((surface_id, DispatchMessageInner::UnFocused));
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Key {
                state: keystate,
                key,
                ..
            } => {
                let pressed_state = match keystate {
                    WEnum::Value(KeyState::Pressed) => ElementState::Pressed,
                    WEnum::Value(KeyState::Released) => ElementState::Released,
                    _ => {
                        return;
                    }
                };
                let key = key + 8;
                if let Some(mut key_context) = keyboard_state.xkb_context.key_context() {
                    let event = key_context.process_key_event(key, pressed_state, false);
                    let event = DispatchMessageInner::KeyboardInput {
                        event,
                        is_synthetic: false,
                    };
                    state.message.push((surface_id, event));
                }

                match pressed_state {
                    ElementState::Pressed => {
                        let delay = match keyboard_state.repeat_info {
                            RepeatInfo::Repeat { delay, .. } => delay,
                            RepeatInfo::Disable => return,
                        };

                        if keyboard_state
                            .xkb_context
                            .keymap_mut()
                            .is_none_or(|keymap| !keymap.key_repeats(key))
                        {
                            return;
                        }

                        keyboard_state.current_repeat = Some(key);

                        if let Some(token) = keyboard_state.repeat_token.take() {
                            state.to_remove_tokens.push(token);
                        }

                        state.repeat_delay = Some(KeyboardTokenState {
                            delay,
                            key,
                            surface_id,
                            pressed_state,
                        });
                    }
                    ElementState::Released => {
                        if keyboard_state.repeat_info != RepeatInfo::Disable
                            && keyboard_state
                                .xkb_context
                                .keymap_mut()
                                .is_some_and(|keymap| keymap.key_repeats(key))
                            && Some(key) == keyboard_state.current_repeat
                        {
                            keyboard_state.current_repeat = None;
                            if let Some(token) = keyboard_state.repeat_token.take() {
                                state.to_remove_tokens.push(token);
                            }
                        }
                    }
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_locked,
                mods_latched,
                group,
                ..
            } => {
                let xkb_context = &mut keyboard_state.xkb_context;
                let xkb_state = match xkb_context.state_mut() {
                    Some(state) => state,
                    None => return,
                };
                xkb_state.update_modifiers(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                let modifiers = xkb_state.modifiers();

                state.message.push((
                    state.current_surface_id(),
                    DispatchMessageInner::ModifiersChanged(modifiers.into()),
                ))
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                keyboard_state.repeat_info = if rate == 0 {
                    // Stop the repeat once we get a disable event.
                    keyboard_state.current_repeat = None;
                    if let Some(token) = keyboard_state.repeat_token.take() {
                        state.to_remove_tokens.push(token);
                    }
                    RepeatInfo::Disable
                } else {
                    let gap = Duration::from_micros(1_000_000 / rate as u64);
                    let delay = Duration::from_millis(delay as u64);
                    RepeatInfo::Repeat { gap, delay }
                };
            }
            _ => {}
        }
    }
}

impl<T> Dispatch<wl_touch::WlTouch, ()> for WindowState<T> {
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
            } => {
                state.finger_locations.insert(id, (x, y));
                let surface_id = state.get_id_from_surface(&surface);
                state
                    .active_surfaces
                    .insert(Some(id), (surface.clone(), surface_id));
                state.update_current_surface(Some(surface));
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchDown {
                        serial,
                        time,
                        id,
                        x,
                        y,
                    },
                ))
            }
            wl_touch::Event::Cancel => {
                let mut mouse_surface = None;
                for (k, v) in state.active_surfaces.drain() {
                    if let Some(id) = k {
                        let (x, y) = state.finger_locations.remove(&id).unwrap_or_default();
                        state
                            .message
                            .push((v.1, DispatchMessageInner::TouchCancel { id, x, y }));
                    } else {
                        // keep the surface of mouse.
                        mouse_surface = Some(v);
                    }
                }
                if let Some(mouse_surface) = mouse_surface {
                    state.active_surfaces.insert(None, mouse_surface);
                }
            }
            wl_touch::Event::Up { serial, time, id } => {
                let surface_id = state
                    .active_surfaces
                    .remove(&Some(id))
                    .or_else(|| {
                        log::warn!("finger[{id}] hasn't been down.");
                        None
                    })
                    .and_then(|(_, id)| id);
                let (x, y) = state.finger_locations.remove(&id).unwrap_or_default();
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchUp {
                        serial,
                        time,
                        id,
                        x,
                        y,
                    },
                ));
            }
            wl_touch::Event::Motion { time, id, x, y } => {
                let surface_id = state
                    .active_surfaces
                    .get(&Some(id))
                    .or_else(|| {
                        log::warn!("finger[{id}] hasn't been down.");
                        None
                    })
                    .and_then(|(_, id)| *id);
                state.finger_locations.insert(id, (x, y));
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchMotion { time, id, x, y },
                ));
            }
            _ => {}
        }
    }
}

impl<T> Dispatch<wl_pointer::WlPointer, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        pointer: &wl_pointer::WlPointer,
        event: <wl_pointer::WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // All mouse events should be happened on the surface which is hovered by the mouse.
        let (mouse_surface, surface_id) = state
            .active_surfaces
            .get(&None)
            .map(|(surface, id)| (Some(surface), *id))
            .unwrap_or_else(|| {
                match &event {
                    wl_pointer::Event::Enter { .. } => {}
                    _ => {
                        log::warn!("mouse hasn't entered.");
                    }
                }
                (None, None)
            });
        let scale = surface_id
            .and_then(|id| state.get_unit_with_id(id))
            .map(|unit| unit.scale_float())
            .unwrap_or(1.0);
        match event {
            wl_pointer::Event::Axis { time, axis, value } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.absolute = value;
                        }
                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.absolute = value;
                        }
                        _ => unreachable!(),
                    };

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ))
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::AxisStop { time, axis } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => vertical.stop = true,
                        wl_pointer::Axis::HorizontalScroll => horizontal.stop = true,

                        _ => unreachable!(),
                    }

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ));
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(source) => state.message.push((
                    surface_id,
                    DispatchMessageInner::Axis {
                        horizontal: AxisScroll::default(),
                        vertical: AxisScroll::default(),
                        source: Some(source),
                        time: 0,
                        scale,
                    },
                )),
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "unknown pointer axis source: {unknown:x}");
                }
            },
            wl_pointer::Event::AxisDiscrete { axis, discrete } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.discrete = discrete;
                        }

                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.discrete = discrete;
                        }

                        _ => unreachable!(),
                    };

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time: 0,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ));
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::Button {
                state: btnstate,
                serial,
                button,
                time,
            } => {
                let mouse_surface = mouse_surface.cloned();
                state.update_current_surface(mouse_surface);
                state.message.push((
                    surface_id,
                    DispatchMessageInner::MouseButton {
                        state: btnstate,
                        serial,
                        button,
                        time,
                    },
                ));
            }
            wl_pointer::Event::Leave { .. } => {
                let surface_id = state
                    .active_surfaces
                    .remove(&None)
                    .or_else(|| {
                        log::warn!("mouse hasn't entered.");
                        None
                    })
                    .and_then(|(_, id)| id);
                state
                    .message
                    .push((surface_id, DispatchMessageInner::MouseLeave));
            }
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                let surface_id = state.get_id_from_surface(&surface);
                state
                    .active_surfaces
                    .insert(None, (surface.clone(), surface_id));
                state.enter_serial = Some(serial);
                state.message.push((
                    surface_id,
                    DispatchMessageInner::MouseEnter {
                        pointer: pointer.clone(),
                        serial,
                        surface_x,
                        surface_y,
                    },
                ));
                if let Some(id) = state.current_surface_id() {
                    state
                        .message
                        .push((Some(id), DispatchMessageInner::Focused(id)));
                }
            }
            wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                state.message.push((
                    surface_id,
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

impl<T> Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, ()> for WindowState<T> {
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

            state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
        }
    }
}

impl<T> Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        proxy: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: <wp_fractional_scale_v1::WpFractionalScaleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let Some(unit) = state.units.iter_mut().find(|info| {
                info.fractional_scale
                    .as_ref()
                    .is_some_and(|fractional_scale| fractional_scale == proxy)
            }) else {
                return;
            };
            unit.scale = scale;
            unit.request_refresh(RefreshRequest::NextFrame);
            state.message.push((
                Some(unit.id),
                DispatchMessageInner::PreferredScale {
                    scale_float: scale as f64 / 120.,
                    scale_u32: scale,
                },
            ));
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum PresentAvailableState {
    /// A `wl_surface.frame` request has been sent, and there is no callback yet.
    Requested,
    /// A notification has been received, it is a good time to start drawing a new frame. Because
    /// there is no present at first, so the default state is available.
    #[default]
    Available,
    /// Availability is taken.
    Taken,
}

impl<T> Dispatch<WlCallback, (id::Id, PresentAvailableState)> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &WlCallback,
        event: <WlCallback as Proxy>::Event,
        data: &(id::Id, PresentAvailableState),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_callback::Event as WlCallbackEvent;
        if let WlCallbackEvent::Done { callback_data: _ } = event
            && let Some(unit) = state.get_mut_unit_with_id(data.0)
        {
            unit.present_available_state = data.1;
        }
    }
}

delegate_noop!(@<T>WindowState<T>: ignore WlCompositor); // WlCompositor is need to create a surface
delegate_noop!(@<T>WindowState<T>: ignore WlSurface); // surface is the base needed to show buffer
delegate_noop!(@<T>WindowState<T>: ignore WlOutput); // output is need to place layer_shell, although here
// it is not used
delegate_noop!(@<T>WindowState<T>: ignore WlShm); // shm is used to create buffer pool
delegate_noop!(@<T>WindowState<T>: ignore WlShmPool); // so it is pool, created by wl_shm
delegate_noop!(@<T>WindowState<T>: ignore WlBuffer); // buffer show the picture
//

delegate_noop!(@<T>WindowState<T>: ignore ExtSessionLockV1); // buffer show the picture
delegate_noop!(@<T>WindowState<T>: ignore ExtSessionLockManagerV1); // buffer show the picture

delegate_noop!(@<T>WindowState<T>: ignore WpCursorShapeManagerV1);
delegate_noop!(@<T>WindowState<T>: ignore WpCursorShapeDeviceV1);

delegate_noop!(@<T> WindowState<T>: ignore WpViewporter);
delegate_noop!(@<T> WindowState<T>: ignore WpViewport);

delegate_noop!(@<T>WindowState<T>: ignore ZwpVirtualKeyboardV1);
delegate_noop!(@<T>WindowState<T>: ignore ZwpVirtualKeyboardManagerV1);

// fractional_scale_manager
delegate_noop!(@<T>WindowState<T>: ignore WpFractionalScaleManagerV1);

impl<T: 'static> WindowState<T> {
    pub fn build(mut self) -> Result<Self, SessionLockEventError> {
        let connection = if let Some(connection) = self.connection.take() {
            connection
        } else {
            Connection::connect_to_env()?
        };
        let (globals, _) = registry_queue_init::<BaseState>(&connection)?;

        let mut event_queue = connection.new_event_queue::<WindowState<T>>();
        let qh = event_queue.handle();

        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?;

        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        self.shm = Some(shm);
        self.seat = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();

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

            let viewport = viewporter
                .as_ref()
                .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
            self.push_window(WindowStateUnit {
                id: id::Id::unique(),
                display: connection.display(),
                wl_surface,
                size: (0, 0),
                buffer: None,
                session_shell: session_lock_surface,
                viewport,
                fractional_scale,
                binding: None,
                scale: 120,
                present_available_state: PresentAvailableState::Available,
                refresh: RefreshRequest::Wait,
                qh: qh.clone(),
            });
        }
        self.viewporter = viewporter;
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
    /// index to get the unit, with [WindowState::get_unit_with_id] if the even is not spical on one surface,
    /// it will return [None].
    ///
    /// Different with running, it receiver a receiver
    pub fn running_with_proxy<F, Message>(
        self,
        message_receiver: std::sync::mpsc::Receiver<Message>,
        event_handler: F,
    ) -> Result<(), SessionLockEventError>
    where
        Message: std::marker::Send + 'static,
        F: FnMut(SessionLockEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData
            + 'static,
    {
        self.running_with_proxy_option(Some(message_receiver), event_handler)
    }
    /// main event loop, every time dispatch, it will store the messages, and do callback. it will
    /// pass a LayerEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit_with_id] if the even is not spical on one surface,
    /// it will return [None].
    pub fn running<F>(self, event_handler: F) -> Result<(), SessionLockEventError>
    where
        F: FnMut(SessionLockEvent<T, ()>, &mut WindowState<T>, Option<id::Id>) -> ReturnData
            + 'static,
    {
        self.running_with_proxy_option(None, event_handler)
    }

    fn running_with_proxy_option<F, Message>(
        mut self,
        message_receiver: Option<std::sync::mpsc::Receiver<Message>>,
        mut event_handler: F,
    ) -> Result<(), SessionLockEventError>
    where
        Message: std::marker::Send + 'static,
        F: FnMut(SessionLockEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData
            + 'static,
    {
        let globals = self.globals.take().unwrap();
        let mut event_queue_origin = self.event_queue.take().unwrap();
        let qh = event_queue_origin.handle();
        let wmcompositer = self.wl_compositor.take().unwrap();
        let shm = self.shm.take().unwrap();
        let fractional_scale_manager = self.fractional_scale_manager.take();
        let cursor_manager: Option<WpCursorShapeManagerV1> = self.cursor_manager.take();
        let viewporter = self.viewporter.take();
        let connection = self.connection.take().unwrap();
        let lock = self.lock.take().unwrap();
        let mut init_event = None;

        let cursor_update_context = CursorUpdateContext {
            cursor_manager,
            qh: qh.clone(),
            connection: connection.clone(),
            shm: shm.clone(),
            wmcompositer: wmcompositer.clone(),
        };

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event = Some(event_handler(
                        SessionLockEvent::InitRequest,
                        &mut self,
                        None,
                    ));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(event_handler(
                        SessionLockEvent::BindProvide(&globals, &qh),
                        &mut self,
                        None,
                    ));
                }
                _ => panic!("Not provide server here"),
            }
        }

        self.message.clear();

        struct EventWrapper<Raw, F> {
            raw: Raw,
            fun: F,
            loop_handle: LoopHandle<'static, Self>,
        }

        let mut event_loop: EventLoop<_> =
            EventLoop::try_new().expect("Failed to initialize the event loop");

        let event_queue = connection.new_event_queue::<EventWrapper<Self, F>>();
        WaylandSource::new(connection.clone(), event_queue)
            .insert(event_loop.handle())
            .expect("Failed to init Wayland Source");
        let mut state = EventWrapper {
            raw: self,
            fun: event_handler,
            loop_handle: event_loop.handle(),
        };
        //self.loop_handler = Some(event_loop.handle());
        let to_exit = Arc::new(AtomicBool::new(false));

        let events: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));

        let thread = std::thread::spawn({
            let events = events.clone();
            let to_exit = to_exit.clone();
            move || {
                let Some(message_receiver) = message_receiver else {
                    return;
                };
                loop {
                    let message = message_receiver.recv_timeout(Duration::from_millis(100));
                    if to_exit.load(Ordering::Relaxed) {
                        break;
                    }
                    match message {
                        Ok(message) => {
                            let mut events_local = events.lock().unwrap();
                            events_local.push(message);
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            }
        });
        let signal = event_loop.get_signal();
        event_loop
            .handle()
            .insert_source(
                Timer::from_duration(Duration::from_millis(50)),
                move |_, _, r_window_state| {
                    let window_state = &mut r_window_state.raw;
                    let event_handler = &mut r_window_state.fun;
                    let mut messages = Vec::new();
                    std::mem::swap(&mut messages, &mut window_state.message);
                    for msg in messages.iter() {
                        match msg {
                            (_, DispatchMessageInner::NewDisplay(display)) => {
                                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                //
                                wl_surface.commit();
                                let session_lock_surface =
                                    lock.get_lock_surface(&wl_surface, display, &qh, ());

                                let mut fractional_scale = None;
                                if let Some(ref fractional_scale_manager) = fractional_scale_manager
                                {
                                    fractional_scale =
                                        Some(fractional_scale_manager.get_fractional_scale(
                                            &wl_surface,
                                            &qh,
                                            (),
                                        ));
                                }
                                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                                // and if you need to reconfigure it, you need to commit the wl_surface again
                                // so because this is just an example, so we just commit it once
                                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
                                let viewport = viewporter
                                    .as_ref()
                                    .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                                window_state.push_window(WindowStateUnit {
                                    id: id::Id::unique(),
                                    display: connection.display(),
                                    wl_surface,
                                    size: (0, 0),
                                    buffer: None,
                                    session_shell: session_lock_surface,
                                    viewport,
                                    fractional_scale,
                                    binding: None,
                                    scale: 120,
                                    present_available_state: PresentAvailableState::Available,
                                    refresh: RefreshRequest::Wait,
                                    qh: qh.clone(),
                                });
                            }
                            _ => {
                                let (index_message, msg) = msg;
                                let msg: DispatchMessage = msg.clone().into();
                                match event_handler(
                                    SessionLockEvent::RequestMessages(&msg),
                                    window_state,
                                    *index_message,
                                ) {
                                    ReturnData::RequestUnlockAndExist => {
                                        lock.unlock_and_destroy();
                                        connection
                                            .roundtrip()
                                            .expect("should roundtrip successfully");
                                        signal.stop();
                                        return TimeoutAction::Drop;
                                    }
                                    ReturnData::RequestSetCursorShape((shape_name, pointer)) => {
                                        let Some(serial) = window_state.enter_serial else {
                                            continue;
                                        };
                                        set_cursor_shape(
                                            &cursor_update_context,
                                            shape_name,
                                            pointer,
                                            serial,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    let mut local_events = events.lock().unwrap();
                    let mut swapped_events: Vec<Message> = vec![];
                    std::mem::swap(&mut *local_events, &mut swapped_events);
                    drop(local_events);
                    for event in swapped_events {
                        window_state.handle_event(
                            &mut *event_handler,
                            SessionLockEvent::UserEvent(event),
                            None,
                        );
                    }

                    window_state.handle_event(
                        &mut *event_handler,
                        SessionLockEvent::NormalDispatch,
                        None,
                    );
                    loop {
                        let mut return_data = vec![];

                        std::mem::swap(&mut window_state.return_data, &mut return_data);
                        for data in return_data {
                            match data {
                                ReturnData::RequestUnlockAndExist => {
                                    lock.unlock_and_destroy();
                                    connection.roundtrip().expect("should go final roundtrip");
                                    signal.stop();
                                    return TimeoutAction::Drop;
                                }
                                ReturnData::RequestSetCursorShape((shape_name, pointer)) => {
                                    let Some(serial) = window_state.enter_serial else {
                                        continue;
                                    };
                                    set_cursor_shape(
                                        &cursor_update_context,
                                        shape_name,
                                        pointer,
                                        serial,
                                    );
                                }
                                _ => {}
                            }
                        }
                        window_state.return_data.retain(|x| *x != ReturnData::None);
                        if window_state.return_data.is_empty() {
                            break;
                        }
                    }
                    let closed_ids = window_state.closed_ids.clone();
                    for id in closed_ids {
                        window_state.handle_event(
                            &mut *event_handler,
                            SessionLockEvent::RequestMessages(&DispatchMessage::Closed),
                            Some(id),
                        );
                    }
                    window_state.closed_ids.clear();

                    for idx in 0..window_state.units.len() {
                        let unit = &mut window_state.units[idx];
                        let (width, height) = unit.size;
                        if width == 0 || height == 0 {
                            // don't refresh, if size is 0.
                            continue;
                        }
                        if unit.take_present_slot() {
                            let unit_id = unit.id;
                            let scale_float = unit.scale_float();
                            let wl_surface = unit.wl_surface.clone();
                            if unit.buffer.is_none() && !window_state.use_display_handle {
                                let Ok(mut file) = tempfile::tempfile() else {
                                    log::error!("Cannot create new file from tempfile");
                                    return TimeoutAction::Drop;
                                };
                                let ReturnData::WlBuffer(buffer) = event_handler(
                                    SessionLockEvent::RequestBuffer(
                                        &mut file, &shm, &qh, width, height,
                                    ),
                                    window_state,
                                    Some(unit_id),
                                ) else {
                                    panic!("You cannot return this one");
                                };
                                wl_surface.attach(Some(&buffer), 0, 0);
                                wl_surface.commit();
                                window_state.units[idx].buffer = Some(buffer);
                            }
                            window_state.handle_event(
                                &mut *event_handler,
                                SessionLockEvent::RequestMessages(
                                    &DispatchMessage::RequestRefresh {
                                        width,
                                        height,
                                        scale_float,
                                    },
                                ),
                                Some(unit_id),
                            );
                            // reset if the slot is not used
                            window_state.units[idx].reset_present_slot();
                        }
                    }
                    TimeoutAction::ToDuration(Duration::from_millis(50))
                },
            )
            .expect("Cannot insert source");
        event_loop
            .run(
                std::time::Duration::from_millis(20),
                &mut state,
                move |r_window_state| {
                    let window_state = &mut r_window_state.raw;
                    let _ = event_queue_origin.roundtrip(window_state);
                    let looph = &r_window_state.loop_handle;
                    for token in window_state.to_remove_tokens.iter() {
                        looph.remove(*token);
                    }
                    window_state.to_remove_tokens.clear();

                    if let Some(KeyboardTokenState {
                        key,
                        delay,
                        surface_id,
                        pressed_state,
                    }) = window_state.repeat_delay.take()
                    {
                        let timer = Timer::from_duration(delay);
                        let keyboard_state = window_state.keyboard_state.as_mut().unwrap();
                        keyboard_state.repeat_token = looph
                            .insert_source(timer, move |_, _, r_window_state| {
                                let state = &mut r_window_state.raw;
                                let event_handler = &mut r_window_state.fun;
                                let keyboard_state = match state.keyboard_state.as_mut() {
                                    Some(keyboard_state) => keyboard_state,
                                    None => return TimeoutAction::Drop,
                                };
                                let repeat_keycode = match keyboard_state.current_repeat {
                                    Some(repeat_keycode) => repeat_keycode,
                                    None => return TimeoutAction::Drop,
                                };
                                // NOTE: not the same key
                                if repeat_keycode != key {
                                    return TimeoutAction::Drop;
                                }
                                if let Some(mut key_context) =
                                    keyboard_state.xkb_context.key_context()
                                {
                                    let event = key_context.process_key_event(
                                        repeat_keycode,
                                        pressed_state,
                                        false,
                                    );
                                    let event = DispatchMessageInner::KeyboardInput {
                                        event,
                                        is_synthetic: false,
                                    };
                                    state.message.push((surface_id, event));
                                }
                                let repeat_info = keyboard_state.repeat_info;

                                let _ = keyboard_state;
                                state.handle_event(
                                    &mut *event_handler,
                                    SessionLockEvent::NormalDispatch,
                                    None,
                                );
                                match repeat_info {
                                    RepeatInfo::Repeat { gap, .. } => {
                                        TimeoutAction::ToDuration(gap)
                                    }
                                    RepeatInfo::Disable => TimeoutAction::Drop,
                                }
                            })
                            .ok();
                    }
                },
            )
            .expect("Error during event loop!");
        to_exit.store(true, Ordering::Relaxed);
        let _ = thread.join();
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

/// avoid too_many_arguments alert in `set_cursor_shape`
struct CursorUpdateContext<T: 'static> {
    cursor_manager: Option<WpCursorShapeManagerV1>,
    qh: QueueHandle<WindowState<T>>,
    connection: Connection,
    shm: WlShm,
    wmcompositer: WlCompositor,
}

fn set_cursor_shape<T: 'static>(
    context: &CursorUpdateContext<T>,
    shape_name: String,
    pointer: WlPointer,
    serial: u32,
) {
    if let Some(cursor_manager) = &context.cursor_manager {
        let Some(shape) = str_to_shape(&shape_name) else {
            log::error!("Not supported shape");
            return;
        };
        let device = cursor_manager.get_pointer(&pointer, &context.qh, ());
        device.set_shape(serial, shape);
        device.destroy();
    } else {
        let Some(cursor_buffer) = get_cursor_buffer(&shape_name, &context.connection, &context.shm)
        else {
            log::error!("Cannot find cursor {shape_name}");
            return;
        };
        let cursor_surface = context.wmcompositer.create_surface(&context.qh, ());
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
