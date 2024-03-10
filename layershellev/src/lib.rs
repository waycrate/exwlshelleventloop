//! # Handle the layer_shell in a winit way
//!
//! Min example is under
//!
//! ```rust, no_run
//! use std::fs::File;
//! use std::os::fd::AsFd;
//!
//! use layershellev::reexport::*;
//! use layershellev::*;
//!
//! const Q_KEY: u32 = 16;
//! const W_KEY: u32 = 17;
//! const E_KEY: u32 = 18;
//! const A_KEY: u32 = 30;
//! const S_KEY: u32 = 31;
//! const D_KEY: u32 = 32;
//! const Z_KEY: u32 = 44;
//! const X_KEY: u32 = 45;
//! const C_KEY: u32 = 46;
//! const ESC_KEY: u32 = 1;
//!
//! fn main() {
//!     let mut ev: WindowState<()> = WindowState::new("Hello")
//!         .with_single(false)
//!         .with_size((0, 400))
//!         .with_layer(Layer::Top)
//!         .with_margin((20, 20, 100, 20))
//!         .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
//!         .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
//!         .with_exclusize_zone(-1)
//!         .build()
//!         .unwrap();
//!
//!     let mut virtual_keyboard_manager = None;
//!     ev.running(|event, ev, index| {
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
//!                 ReturnData::None
//!             }
//!             LayerEvent::XdgInfoChanged(_) => {
//!                 let index = index.unwrap();
//!                 let unit = ev.get_unit(index);
//!                 println!("{:?}", unit.get_xdgoutput_info());
//!                 ReturnData::None
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
//!             LayerEvent::RequestMessages(DispatchMessage::MouseMotion {
//!                 time,
//!                 surface_x,
//!                 surface_y,
//!             }) => {
//!                 println!("{time}, {surface_x}, {surface_y}");
//!                 ReturnData::None
//!             }
//!             LayerEvent::RequestMessages(DispatchMessage::KeyBoard { key, .. }) => {
//!                 match index {
//!                     Some(index) => {
//!                         let ev_unit = ev.get_unit(index);
//!                         match *key {
//!                             Q_KEY => ev_unit.set_anchor(Anchor::Top | Anchor::Left),
//!                             W_KEY => ev_unit.set_anchor(Anchor::Top),
//!                             E_KEY => ev_unit.set_anchor(Anchor::Top | Anchor::Right),
//!                             A_KEY => ev_unit.set_anchor(Anchor::Left),
//!                             S_KEY => ev_unit.set_anchor(
//!                                 Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom,
//!                             ),
//!                             D_KEY => ev_unit.set_anchor(Anchor::Right),
//!                             Z_KEY => ev_unit.set_anchor(Anchor::Left | Anchor::Bottom),
//!                             X_KEY => ev_unit.set_anchor(Anchor::Bottom),
//!                             C_KEY => ev_unit.set_anchor(Anchor::Bottom | Anchor::Right),
//!                             ESC_KEY => return ReturnData::RequestExist,
//!                             _ => {}
//!                         }
//!                     }
//!                     None => {
//!                         for ev_unit in ev.get_unit_iter() {
//!                             match *key {
//!                                 Q_KEY => ev_unit.set_anchor(Anchor::Top | Anchor::Left),
//!                                 W_KEY => ev_unit.set_anchor(Anchor::Top),
//!                                 E_KEY => ev_unit.set_anchor(Anchor::Top | Anchor::Right),
//!                                 A_KEY => ev_unit.set_anchor(Anchor::Left),
//!                                 S_KEY => ev_unit.set_anchor(
//!                                     Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom,
//!                                 ),
//!                                 D_KEY => ev_unit.set_anchor(Anchor::Right),
//!                                 Z_KEY => ev_unit.set_anchor(Anchor::Left | Anchor::Bottom),
//!                                 X_KEY => ev_unit.set_anchor(Anchor::Bottom),
//!                                 C_KEY => ev_unit.set_anchor(Anchor::Bottom | Anchor::Right),
//!                                 ESC_KEY => return ReturnData::RequestExist,
//!                                 _ => {}
//!                             }
//!                         }
//!                     }
//!                 };
//!
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
//!

mod events;
mod strtoshape;

use std::fmt::Debug;

use events::DispatchMessageInner;

pub use events::{DispatchMessage, LayerEvent, ReturnData, XdgInfoChangedType};

use strtoshape::str_to_shape;
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

use wayland_cursor::{CursorImageBuffer, CursorTheme};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

use wayland_protocols::{
    wp::fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    xdg::xdg_output::zv1::client::{
        zxdg_output_manager_v1::ZxdgOutputManagerV1,
        zxdg_output_v1::{self, ZxdgOutputV1},
    },
};

use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
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

/// this struct store the xdg_output information
#[derive(Debug)]
pub struct ZxdgOutputInfo {
    zxdgoutput: ZxdgOutputV1,
    logical_size: (i32, i32),
    position: (i32, i32),
}

impl ZxdgOutputInfo {
    fn new(zxdgoutput: ZxdgOutputV1) -> Self {
        Self {
            zxdgoutput,
            logical_size: (0, 0),
            position: (0, 0),
        }
    }

    /// you can get the Logic positon of the screen current surface in
    pub fn get_positon(&self) -> (i32, i32) {
        self.position
    }

    /// you can get the LogicalPosition of the screen current surface in
    pub fn get_logical_size(&self) -> (i32, i32) {
        self.logical_size
    }
}

/// This is the unit, binding to per screen.
/// Because layer_shell is so unique, on surface bind to only one
/// wl_output, only one buffer, only one output, so it will store
/// includes the information of ZxdgOutput, size, and layer_shell
///
/// and it can set a binding, you to store the related data. like
/// a cario_context, which is binding to the buffer on the wl_surface.
#[derive(Debug)]
pub struct WindowStateUnit<T: Debug> {
    display: WlDisplay,
    wl_surface: WlSurface,
    size: (u32, u32),
    buffer: Option<WlBuffer>,
    layer_shell: ZwlrLayerSurfaceV1,
    zxdgoutput: Option<ZxdgOutputInfo>,
    fractional_scale: Option<WpFractionalScaleV1>,
    binding: Option<T>,
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

impl<T: Debug> rwh_06::HasWindowHandle for WindowStateUnit<T> {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_window_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::WindowHandle::borrow_raw(raw) })
    }
}

impl<T: Debug> rwh_06::HasDisplayHandle for WindowStateUnit<T> {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_display_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::DisplayHandle::borrow_raw(raw) })
    }
}

// if is only one window, use it will be easy
impl<T: Debug> rwh_06::HasWindowHandle for WindowState<T> {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        let raw = self.main_window().raw_window_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::WindowHandle::borrow_raw(raw) })
    }
}

// if is only one window, use it will be easy
impl<T: Debug> rwh_06::HasDisplayHandle for WindowState<T> {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = self.main_window().raw_display_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::DisplayHandle::borrow_raw(raw) })
    }
}
impl<T: Debug> WindowStateUnit<T> {
    /// get the wl surface from WindowState
    pub fn get_wlsurface(&self) -> &WlSurface {
        &self.wl_surface
    }

    /// get the xdg_output info related to this unit
    pub fn get_xdgoutput_info(&self) -> Option<&ZxdgOutputInfo> {
        self.zxdgoutput.as_ref()
    }

    /// set the anchor of the current unit. please take the simple.rs as refrence
    pub fn set_anchor(&self, anchor: Anchor) {
        self.layer_shell.set_anchor(anchor);
        self.wl_surface.commit();
    }

    /// you can reset the margin which bind to the surface
    pub fn set_margin(&self, (top, right, bottom, left): (i32, i32, i32, i32)) {
        self.layer_shell.set_margin(top, right, bottom, left);
        self.wl_surface.commit();
    }

    /// set the layer size of current unit
    pub fn set_size(&self, (width, height): (u32, u32)) {
        self.layer_shell.set_size(width, height);
        self.wl_surface.commit();
    }

    /// set current exclusive_zone
    pub fn set_exclusive_zone(&self, zone: i32) {
        self.layer_shell.set_exclusive_zone(zone);
        self.wl_surface.commit();
    }

    /// you can use this function to set a binding data. the message passed back contain
    /// a index, you can use that to get the unit. It will be very useful, because you can
    /// use the binding data to operate the file binding to the buffer. you can take
    /// startcolorkeyboard as reference.
    pub fn set_binding(&mut self, binding: T) {
        self.binding = Some(binding);
    }

    /// return the binding data, with mut reference
    pub fn get_binding_mut(&mut self) -> Option<&mut T> {
        self.binding.as_mut()
    }

    /// get the size of the surface
    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    /// this function will refresh whole surface. it will reattach the buffer, and damage whole,
    /// and finall commit
    pub fn request_refresh(&self, (width, height): (i32, i32)) {
        self.wl_surface.attach(self.buffer.as_ref(), 0, 0);
        self.wl_surface.damage(0, 0, width, height);
        self.wl_surface.commit();
    }
}

/// main state, store the main information
#[derive(Debug)]
pub struct WindowState<T: Debug> {
    outputs: Vec<(u32, wl_output::WlOutput)>,
    current_surface: Option<WlSurface>,
    is_single: bool,
    units: Vec<WindowStateUnit<T>>,
    message: Vec<(Option<usize>, DispatchMessageInner)>,

    connection: Option<Connection>,
    event_queue: Option<EventQueue<WindowState<T>>>,
    wl_compositor: Option<WlCompositor>,
    xdg_output_manager: Option<ZxdgOutputManagerV1>,
    shm: Option<WlShm>,
    cursor_manager: Option<WpCursorShapeManagerV1>,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    globals: Option<GlobalList>,

    // base managers
    seat: Option<WlSeat>,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    touch: Option<WlTouch>,

    // states
    namespace: String,
    keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    anchor: Anchor,
    layer: Layer,
    size: Option<(u32, u32)>,
    exclusive_zone: Option<i32>,
    margin: Option<(i32, i32, i32, i32)>,

    // settings
    use_display_handle: bool,
}

impl<T: Debug> WindowState<T> {
    // return the first window
    // I will use it in iced
    pub fn main_window(&self) -> &WindowStateUnit<T> {
        &self.units[0]
    }

    // return all windows
    pub fn windows(&self) -> &Vec<WindowStateUnit<T>> {
        &self.units
    }
}

pub struct WindowWrapper {
    display: WlDisplay,
    wl_surface: WlSurface,
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
    pub fn gen_wrapper(&self) -> WindowWrapper {
        WindowWrapper {
            display: self.main_window().display.clone(),
            wl_surface: self.main_window().wl_surface.clone(),
        }
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

impl<T: Debug> WindowState<T> {
    /// create a WindowState, you need to pass a namespace in
    pub fn new(namespace: &str) -> Self {
        assert_ne!(namespace, "");
        Self {
            namespace: namespace.to_owned(),
            ..Default::default()
        }
    }

    /// if the shell is a single one, only display on one screen,
    /// fi true, the layer will binding to current screen
    pub fn with_single(mut self, single: bool) -> Self {
        self.is_single = single;
        self
    }

    /// keyboard_interacivity, pleace take look at [layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
    pub fn with_keyboard_interacivity(
        mut self,
        keyboard_interacivity: zwlr_layer_surface_v1::KeyboardInteractivity,
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
    pub fn with_margin(mut self, (top, right, bottom, left): (i32, i32, i32, i32)) -> Self {
        self.margin = Some((top, right, bottom, left));
        self
    }

    /// if not set, it will be the size suggested by layer_shell, like anchor to four ways,
    /// and margins to 0,0,0,0 , the size will be the size of screen.
    ///
    /// if set, layer_shell will use the size you set
    pub fn with_size(mut self, size: (u32, u32)) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_option_size(mut self, size: Option<(u32, u32)>) -> Self {
        self.size = size;
        self
    }

    /// exclusive_zone, please take look at [layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
    pub fn with_exclusize_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }

    pub fn with_use_display_handle(mut self, use_display_handle: bool) -> Self {
        self.use_display_handle = use_display_handle;
        self
    }
}

impl<T: Debug> Default for WindowState<T> {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            current_surface: None,
            is_single: true,
            units: Vec::new(),
            message: Vec::new(),

            connection: None,
            event_queue: None,
            wl_compositor: None,
            shm: None,
            cursor_manager: None,
            xdg_output_manager: None,
            globals: None,
            fractional_scale_manager: None,

            seat: None,
            keyboard: None,
            pointer: None,
            touch: None,

            namespace: "".to_owned(),
            keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right | Anchor::Bottom,
            size: None,
            exclusive_zone: None,
            margin: None,

            use_display_handle: false,
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
            wl_pointer::Event::Leave { .. } => {
                state.current_surface = None;
                state
                    .message
                    .push((state.surface_pos(), DispatchMessageInner::MouseLeave));
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

impl<T: Debug> Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WindowState<T> {
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
            state.units[unit_index].size = (width, height);

            state.message.push((
                Some(unit_index),
                DispatchMessageInner::RefreshSurface { width, height },
            ));
        }
    }
}

impl<T: Debug> Dispatch<zxdg_output_v1::ZxdgOutputV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        proxy: &zxdg_output_v1::ZxdgOutputV1,
        event: <zxdg_output_v1::ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let Some(index) = state.units.iter().position(|info| {
            info.zxdgoutput
                .as_ref()
                .is_some_and(|zxdgoutput| zxdgoutput.zxdgoutput == *proxy)
        }) else {
            return;
        };
        let info = &mut state.units[index];
        let xdg_info = info.zxdgoutput.as_mut().unwrap();
        let change_type = match event {
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                xdg_info.logical_size = (width, height);
                XdgInfoChangedType::Size
            }
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                xdg_info.position = (x, y);
                XdgInfoChangedType::Position
            }
            _ => {
                return;
            }
        };
        state.message.push((
            Some(index),
            DispatchMessageInner::XdgInfoChanged(change_type),
        ));
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
delegate_noop!(@<T: Debug>WindowState<T>: ignore ZwlrLayerShellV1); // it is simillar with xdg_toplevel, also the
                                                                    // ext-session-shell

delegate_noop!(@<T: Debug>WindowState<T>: ignore WpCursorShapeManagerV1);
delegate_noop!(@<T: Debug>WindowState<T>: ignore WpCursorShapeDeviceV1);

delegate_noop!(@<T: Debug>WindowState<T>: ignore ZwpVirtualKeyboardV1);
delegate_noop!(@<T: Debug>WindowState<T>: ignore ZwpVirtualKeyboardManagerV1);

delegate_noop!(@<T: Debug>WindowState<T>: ignore ZxdgOutputManagerV1);
delegate_noop!(@<T: Debug>WindowState<T>: ignore WpFractionalScaleManagerV1);

impl<T: Debug + 'static> WindowState<T> {
    pub fn build(mut self) -> Result<Self, LayerEventError> {
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

        let xdg_output_manager = globals.bind::<ZxdgOutputManagerV1, _, _>(&qh, 1..=3, ())?; // bind
                                                                                             // xdg_output_manager

        let fractional_scale_manager = globals
            .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        event_queue.blocking_dispatch(&mut self)?; // then make a dispatch

        // do the step before, you get empty list

        // so it is the same way, to get surface detach to protocol, first get the shell, like wmbase
        // or layer_shell or session-shell, then get `surface` from the wl_surface you get before, and
        // set it
        // finally thing to remember is to commit the surface, make the shell to init.
        //let (init_w, init_h) = self.size;
        // this example is ok for both xdg_surface and layer_shell
        if self.is_single {
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

            let mut fractional_scale = None;
            if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                fractional_scale =
                    Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
            }
            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
            self.units.push(WindowStateUnit {
                display: connection.display(),
                wl_surface,
                size: (0, 0),
                buffer: None,
                layer_shell: layer,
                zxdgoutput: None,
                fractional_scale,
                binding: None,
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

                let zxdgoutput = xdg_output_manager.get_xdg_output(display, &qh, ());
                let mut fractional_scale = None;
                if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                    fractional_scale =
                        Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
                }
                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                // and if you need to reconfigure it, you need to commit the wl_surface again
                // so because this is just an example, so we just commit it once
                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                self.units.push(WindowStateUnit {
                    display: connection.display(),
                    wl_surface,
                    size: (0, 0),
                    buffer: None,
                    layer_shell: layer,
                    zxdgoutput: Some(ZxdgOutputInfo::new(zxdgoutput)),
                    fractional_scale,
                    binding: None,
                });
            }
            self.message.clear();
        }
        self.event_queue = Some(event_queue);
        self.globals = Some(globals);
        self.wl_compositor = Some(wmcompositer);
        self.fractional_scale_manager = fractional_scale_manager;
        self.cursor_manager = cursor_manager;
        self.xdg_output_manager = Some(xdg_output_manager);
        self.connection = Some(connection);

        Ok(self)
    }
    /// main event loop, every time dispatch, it will store the messages, and do callback. it will
    /// pass a LayerEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit] if the even is not spical on one surface,
    /// it will return [None].
    pub fn running<F>(mut self, mut event_hander: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerEvent<T>, &mut WindowState<T>, Option<usize>) -> ReturnData,
    {
        let globals = self.globals.take().unwrap();
        let mut event_queue = self.event_queue.take().unwrap();
        let qh = event_queue.handle();
        let wmcompositer = self.wl_compositor.take().unwrap();
        let shm = self.shm.take().unwrap();
        let fractional_scale_manager = self.fractional_scale_manager.take();
        let cursor_manager: Option<WpCursorShapeManagerV1> = self.cursor_manager.take();
        let xdg_output_manager = self.xdg_output_manager.take().unwrap();
        let connection = self.connection.take().unwrap();
        let mut init_event = None;
        let mut timecounter = 0;

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event = Some(event_hander(LayerEvent::InitRequest, &mut self, None));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(event_hander(
                        LayerEvent::BindProvide(&globals, &qh),
                        &mut self,
                        None,
                    ));
                }
                _ => panic!("Not privide server here"),
            }
        }
        'out: loop {
            // TODO: use blocking_dispatch will block the event,
            // so use roundtrip is ok?
            event_queue.roundtrip(&mut self)?;
            timecounter += 1;
            if self.message.is_empty() {
                if timecounter > 100 {
                    event_hander(LayerEvent::NormalDispatch, &mut self, None);
                    timecounter = 0;
                }
                continue;
            }
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut self.message);
            for msg in messages.iter() {
                match msg {
                    (Some(unit_index), DispatchMessageInner::RefreshSurface { width, height }) => {
                        let index = *unit_index;
                        // NOTE: is is use_display_handle, just send request_refresh
                        // I will use it in iced
                        if self.units[index].buffer.is_none() && !self.use_display_handle {
                            let mut file = tempfile::tempfile()?;
                            let ReturnData::WlBuffer(buffer) = event_hander(
                                LayerEvent::RequestBuffer(&mut file, &shm, &qh, *width, *height),
                                &mut self,
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
                                &mut self,
                                Some(index),
                            );
                        }
                        let surface = &self.units[index].wl_surface;

                        surface.commit();
                    }
                    (index_info, DispatchMessageInner::XdgInfoChanged(change_type)) => {
                        event_hander(
                            LayerEvent::XdgInfoChanged(*change_type),
                            &mut self,
                            *index_info,
                        );
                    }
                    (_, DispatchMessageInner::NewDisplay(display)) => {
                        if self.is_single {
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

                        let zxdgoutput = xdg_output_manager.get_xdg_output(display, &qh, ());
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
                            display: connection.display(),
                            wl_surface,
                            size: (0, 0),
                            buffer: None,
                            layer_shell: layer,
                            zxdgoutput: Some(ZxdgOutputInfo::new(zxdgoutput)),
                            fractional_scale,
                            binding: None,
                        });
                    }
                    _ => {
                        let (index_message, msg) = msg;
                        let msg: DispatchMessage = msg.clone().into();
                        match event_hander(
                            LayerEvent::RequestMessages(&msg),
                            &mut self,
                            *index_message,
                        ) {
                            ReturnData::RequestExist => {
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
