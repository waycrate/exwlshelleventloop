//! # Handle the layer_shell in a winit way
//!
//! Min example is under
//!
//! ```rust, no_run
//! use std::fs::File;
//! use std::os::fd::AsFd;
//!
//! use layershellev::keyboard::{KeyCode, PhysicalKey};
//! use layershellev::reexport::*;
//! use layershellev::*;
//!
//! fn main() {
//!     let mut ev: WindowState<()> = WindowState::new("Hello")
//!         .with_allscreens()
//!         .with_size((0, 400))
//!         .with_layer(Layer::Top)
//!         .with_margin((20, 20, 100, 20))
//!         .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
//!         .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
//!         .with_exclusive_zone(-1)
//!         .build()
//!         .unwrap();
//!
//!     ev.running(|event, ev, index| {
//!         match event {
//!             // NOTE: this will send when init, you can request bind extra object from here
//!             LayerShellEvent::InitRequest => ReturnData::RequestBind,
//!             LayerShellEvent::BindProvide(globals, qh) => {
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
//!             LayerShellEvent::XdgInfoChanged(_) => {
//!                 let index = index.unwrap();
//!                 let unit = ev.get_unit_with_id(index).unwrap();
//!                 println!("{:?}", unit.get_xdgoutput_info());
//!                 ReturnData::None
//!             }
//!             LayerShellEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
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
//!             LayerShellEvent::RequestMessages(DispatchMessage::RequestRefresh { width, height, .. }) => {
//!                 println!("{width}, {height}");
//!                 ReturnData::None
//!             }
//!             LayerShellEvent::RequestMessages(DispatchMessage::MouseButton { .. }) => ReturnData::None,
//!             LayerShellEvent::RequestMessages(DispatchMessage::MouseEnter {
//!                 pointer, ..
//!             }) => ReturnData::RequestSetCursorShape((
//!                 "crosshair".to_owned(),
//!                 pointer.clone(),
//!             )),
//!             LayerShellEvent::RequestMessages(DispatchMessage::MouseMotion {
//!                 time,
//!                 surface_x,
//!                 surface_y,
//!             }) => {
//!                 println!("{time}, {surface_x}, {surface_y}");
//!                 ReturnData::None
//!             }
//!             LayerShellEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
//!                if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
//!                    ReturnData::RequestExit
//!                } else {
//!                    ReturnData::None
//!                }
//!            }
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
pub use events::NewInputPanelSettings;
pub use events::NewLayerShellSettings;
pub use events::NewPopUpSettings;
pub use events::NewXdgWindowSettings;
pub use events::OutputOption;
pub use waycrate_xkbkeycode::keyboard;
pub use waycrate_xkbkeycode::xkb_keyboard;

pub mod dpi;
mod events;
mod strtoshape;

use events::DispatchMessageInner;

pub mod id;

pub use events::{
    AxisScroll, DispatchMessage, Ime, LayerShellEvent, ReturnData, XdgInfoChangedType,
};

use strtoshape::str_to_shape;

use waycrate_xkbkeycode::xkb_keyboard::RepeatInfo;

use wayland_client::{
    ConnectError, Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle, WEnum,
    delegate_noop,
    globals::{BindError, GlobalError, GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::{Event as WlCallbackEvent, WlCallback},
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_keyboard::{self, KeyState, KeymapFormat, WlKeyboard},
        wl_output::{self, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_region::WlRegion,
        wl_registry,
        wl_seat::{self, WlSeat},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
        wl_touch::{self, WlTouch},
    },
};

use wayland_cursor::{CursorImageBuffer, CursorTheme};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

use wayland_protocols::xdg::shell::client::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::XdgPositioner,
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::XdgWmBase,
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

use wayland_protocols::wp::input_method::zv1::client::{
    zwp_input_panel_surface_v1::{Position as ZwpInputPanelPosition, ZwpInputPanelSurfaceV1},
    zwp_input_panel_v1::ZwpInputPanelV1,
};

use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

use wayland_protocols::wp::cursor_shape::v1::client::{
    wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
    wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{self, ContentHint, ContentPurpose, ZwpTextInputV3},
};
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    zxdg_toplevel_decoration_v1::{self, ZxdgToplevelDecorationV1},
};

pub use calloop;
use calloop::{
    Error as CallLoopError, EventLoop, LoopHandle,
    timer::{TimeoutAction, Timer},
};
use calloop_wayland_source::WaylandSource;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::Instant;

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
    #[error("Event Loop Error")]
    EventLoopInitError(#[from] CallLoopError),
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
            Connection, QueueHandle, WEnum,
            globals::GlobalList,
            protocol::{
                wl_compositor::WlCompositor,
                wl_keyboard::{self, KeyState},
                wl_pointer::{self, ButtonState},
                wl_region::WlRegion,
                wl_seat::WlSeat,
            },
        };
    }
    pub mod wp_cursor_shape_device_v1 {
        pub use crate::strtoshape::ShapeName;
        pub use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
    }
    pub mod xdg_toplevel {
        pub use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
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

/// this struct store the xdg_output information
#[derive(Debug, Clone)]
pub struct ZxdgOutputInfo {
    name: String,
    description: String,
    zxdgoutput: ZxdgOutputV1,
    logical_size: (i32, i32),
    position: (i32, i32),
}

impl ZxdgOutputInfo {
    fn new(zxdgoutput: ZxdgOutputV1) -> Self {
        Self {
            zxdgoutput,
            name: "".to_owned(),
            description: "".to_owned(),
            logical_size: (0, 0),
            position: (0, 0),
        }
    }

    /// you can get the Logic position of the screen current surface in
    pub fn get_position(&self) -> (i32, i32) {
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
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum Shell {
    LayerShell(ZwlrLayerSurfaceV1),
    PopUp((XdgPopup, XdgSurface)),
    XdgTopLevel((XdgToplevel, XdgSurface, Option<ZxdgToplevelDecorationV1>)),
    InputPanel(#[allow(unused)] ZwpInputPanelSurfaceV1),
}

impl PartialEq<ZwlrLayerSurfaceV1> for Shell {
    fn eq(&self, other: &ZwlrLayerSurfaceV1) -> bool {
        match self {
            Self::LayerShell(shell) => shell == other,
            _ => false,
        }
    }
}

impl PartialEq<XdgPopup> for Shell {
    fn eq(&self, other: &XdgPopup) -> bool {
        match self {
            Self::PopUp((popup, _)) => popup == other,
            _ => false,
        }
    }
}

impl PartialEq<XdgSurface> for Shell {
    fn eq(&self, other: &XdgSurface) -> bool {
        match self {
            Self::PopUp((_, surface)) => surface == other,
            _ => false,
        }
    }
}
impl PartialEq<XdgToplevel> for Shell {
    fn eq(&self, other: &XdgToplevel) -> bool {
        match self {
            Self::XdgTopLevel((level, _, _)) => level == other,
            _ => false,
        }
    }
}
impl Shell {
    fn destroy(&self) {
        match self {
            Self::PopUp((popup, xdg_surface)) => {
                popup.destroy();
                xdg_surface.destroy();
            }
            Self::XdgTopLevel((top_level, xdg_surface, decoration)) => {
                if let Some(decoration) = decoration {
                    decoration.destroy();
                }
                top_level.destroy();
                xdg_surface.destroy();
            }
            Self::LayerShell(shell) => shell.destroy(),
            Self::InputPanel(_) => {}
        }
    }

    fn is_popup(&self) -> bool {
        matches!(self, Self::PopUp(_))
    }

    fn top_level(&self) -> Option<XdgToplevel> {
        match self {
            Self::XdgTopLevel((level, _, _)) => Some(level.clone()),
            _ => None,
        }
    }
}

/// The state of if we can call a `present` for the window.
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

struct WindowStateUnitBuilder<T> {
    inner: WindowStateUnit<T>,
}

impl<T> WindowStateUnitBuilder<T> {
    fn new(
        id: id::Id,
        qh: QueueHandle<WindowState<T>>,
        display: WlDisplay,
        wl_surface: WlSurface,
        shell: Shell,
    ) -> Self {
        Self {
            inner: WindowStateUnit {
                id,
                qh,
                display,
                wl_surface,
                shell,
                size: (0, 0),
                buffer: Default::default(),
                zxdgoutput: Default::default(),
                fractional_scale: Default::default(),
                viewport: Default::default(),
                wl_output: Default::default(),
                binding: Default::default(),
                becreated: Default::default(),
                // Unknown why it is 120
                scale: 120,
                request_flag: Default::default(),
                present_available_state: Default::default(),
            },
        }
    }

    fn build(self) -> WindowStateUnit<T> {
        self.inner
    }

    fn size(mut self, size: (u32, u32)) -> Self {
        self.inner.size = size;
        self
    }

    fn zxdgoutput(mut self, zxdgoutput: Option<ZxdgOutputInfo>) -> Self {
        self.inner.zxdgoutput = zxdgoutput;
        self
    }

    fn fractional_scale(mut self, fractional_scale: Option<WpFractionalScaleV1>) -> Self {
        self.inner.fractional_scale = fractional_scale;
        self
    }

    fn viewport(mut self, viewport: Option<WpViewport>) -> Self {
        self.inner.viewport = viewport;
        self
    }

    fn wl_output(mut self, wl_output: Option<WlOutput>) -> Self {
        self.inner.wl_output = wl_output;
        self
    }

    fn binding(mut self, binding: Option<T>) -> Self {
        self.inner.binding = binding;
        self
    }

    fn becreated(mut self, becreated: bool) -> Self {
        self.inner.becreated = becreated;
        self
    }
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

#[derive(Debug, Default)]
struct WindowStateUnitRequestFlag {
    /// The flag of if this window has been requested to be closed.
    close: bool,
    /// The flag of if this window has been requested to be refreshed.
    refresh: RefreshRequest,
}

#[derive(Debug)]
pub struct WindowStateUnit<T> {
    id: id::Id,
    qh: QueueHandle<WindowState<T>>,
    display: WlDisplay,
    wl_surface: WlSurface,
    size: (u32, u32),
    buffer: Option<WlBuffer>,
    shell: Shell,
    zxdgoutput: Option<ZxdgOutputInfo>,
    fractional_scale: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
    wl_output: Option<WlOutput>,
    binding: Option<T>,
    becreated: bool,

    scale: u32,
    request_flag: WindowStateUnitRequestFlag,
    present_available_state: PresentAvailableState,
}

impl<T> WindowStateUnit<T> {
    fn is_popup(&self) -> bool {
        self.shell.is_popup()
    }
}

impl<T> WindowStateUnit<T> {
    /// get the WindowState id
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

    /// gen the WindowState [WindowWrapper]
    pub fn gen_wrapper(&self) -> WindowWrapper {
        WindowWrapper {
            id: self.id,
            display: self.display.clone(),
            wl_surface: self.wl_surface.clone(),
            viewport: self.viewport.clone(),
            toplevel: self.shell.top_level(),
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

impl<T> rwh_06::HasWindowHandle for WindowStateUnit<T> {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_window_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::WindowHandle::borrow_raw(raw) })
    }
}

impl<T> rwh_06::HasDisplayHandle for WindowStateUnit<T> {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = self.raw_display_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::DisplayHandle::borrow_raw(raw) })
    }
}

// if is only one window, use it will be easy
impl<T> rwh_06::HasWindowHandle for WindowState<T> {
    fn window_handle(&self) -> Result<rwh_06::WindowHandle<'_>, rwh_06::HandleError> {
        let raw = self.main_window().raw_window_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::WindowHandle::borrow_raw(raw) })
    }
}

// if is only one window, use it will be easy
impl<T> rwh_06::HasDisplayHandle for WindowState<T> {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = self.main_window().raw_display_handle_rwh_06()?;

        // SAFETY: The window handle will never be deallocated while the window is alive,
        // and the main thread safety requirements are upheld internally by each platform.
        Ok(unsafe { rwh_06::DisplayHandle::borrow_raw(raw) })
    }
}
impl<T> WindowStateUnit<T> {
    /// get the wl surface from WindowState
    pub fn get_wlsurface(&self) -> &WlSurface {
        &self.wl_surface
    }

    /// get the xdg_output info related to this unit
    pub fn get_xdgoutput_info(&self) -> Option<&ZxdgOutputInfo> {
        self.zxdgoutput.as_ref()
    }

    /// set the anchor of the current unit. please take the simple.rs as reference
    pub fn set_anchor(&self, anchor: Anchor) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_anchor(anchor);
            self.wl_surface.commit();
        }
    }

    /// you can reset the margin which bind to the surface
    pub fn set_margin(&self, (top, right, bottom, left): (i32, i32, i32, i32)) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_margin(top, right, bottom, left);
            self.wl_surface.commit();
        }
    }

    /// set the layer
    pub fn set_layer(&self, layer: Layer) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_layer(layer);
            self.wl_surface.commit();
        }
    }

    /// set the anchor and set the size together
    /// When you want to change layer from LEFT|RIGHT|BOTTOM to TOP|LEFT|BOTTOM, use it
    pub fn set_anchor_with_size(&self, anchor: Anchor, (width, height): (u32, u32)) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_anchor(anchor);
            layer_shell.set_size(width, height);
            self.wl_surface.commit();
        }
    }

    /// set the layer size of current unit
    pub fn set_size(&self, (width, height): (u32, u32)) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_size(width, height);
            self.wl_surface.commit();
        }
    }

    /// set current exclusive_zone
    pub fn set_exclusive_zone(&self, zone: i32) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_exclusive_zone(zone);
            self.wl_surface.commit();
        }
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

    /// get the binding data
    pub fn get_binding(&self) -> Option<&T> {
        self.binding.as_ref()
    }

    /// get the size of the surface
    pub fn get_size(&self) -> (u32, u32) {
        self.size
    }

    /// this function will refresh whole surface. it will reattach the buffer, and damage whole,
    /// and final commit
    pub fn refresh(&self) {
        self.wl_surface.attach(self.buffer.as_ref(), 0, 0);
        self.wl_surface
            .damage(0, 0, self.size.0 as i32, self.size.1 as i32);
        self.wl_surface.commit();
    }

    pub fn scale_u32(&self) -> u32 {
        self.scale
    }

    pub fn scale_float(&self) -> f64 {
        self.scale as f64 / 120.
    }

    pub fn request_close(&mut self) {
        self.request_flag.close = true;
    }

    pub fn request_refresh(&mut self, request: RefreshRequest) {
        // refresh request in nearest future has the highest priority.
        match self.request_flag.refresh {
            RefreshRequest::NextFrame => {}
            RefreshRequest::At(instant) => match request {
                RefreshRequest::NextFrame => self.request_flag.refresh = request,
                RefreshRequest::At(other_instant) => {
                    if other_instant < instant {
                        self.request_flag.refresh = request;
                    }
                }
                RefreshRequest::Wait => {}
            },
            RefreshRequest::Wait => self.request_flag.refresh = request,
        }
    }

    fn should_refresh(&self) -> bool {
        match self.request_flag.refresh {
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
        self.request_flag.refresh = RefreshRequest::Wait;
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ImePurpose {
    /// No special hints for the IME (default).
    Normal,
    /// The IME is used for password input.
    Password,
    /// The IME is used to input into a terminal.
    ///
    /// For example, that could alter OSK on Wayland to show extra buttons.
    Terminal,
}

/// main state, store the main information
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
    xdg_output_manager: Option<ZxdgOutputManagerV1>,
    wmbase: Option<XdgWmBase>,
    shm: Option<WlShm>,
    cursor_manager: Option<WpCursorShapeManagerV1>,
    viewporter: Option<WpViewporter>,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    globals: Option<GlobalList>,

    // background
    background_surface: Option<WlSurface>,
    display: Option<WlDisplay>,

    // base managers
    seat: Option<WlSeat>,
    keyboard_state: Option<xkb_keyboard::KeyboardState>,

    pointer: Option<WlPointer>,
    touch: Option<WlTouch>,
    virtual_keyboard: Option<ZwpVirtualKeyboardV1>,

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
    loop_handler: Option<LoopHandle<'static, Self>>,

    last_unit_index: usize,
    last_wloutput: Option<WlOutput>,

    return_data: Vec<ReturnData<T>>,
    finger_locations: HashMap<i32, (f64, f64)>,
    enter_serial: Option<u32>,

    xdg_info_cache: Vec<(wl_output::WlOutput, ZxdgOutputInfo)>,

    start_mode: StartMode,
    init_finished: bool,
    events_transparent: bool,

    text_input_manager: Option<ZwpTextInputManagerV3>,
    text_input: Option<ZwpTextInputV3>,
    text_inputs: Vec<ZwpTextInputV3>,

    xdg_decoration_manager: Option<ZxdgDecorationManagerV1>,

    ime_purpose: ImePurpose,
    ime_allowed: bool,
}

impl<T> WindowState<T> {
    pub fn append_return_data(&mut self, data: ReturnData<T>) {
        self.return_data.push(data);
    }
    /// remove a shell, destroy the surface
    fn remove_shell(&mut self, id: id::Id) -> Option<()> {
        let index = self
            .units
            .iter()
            .position(|unit| unit.id == id && unit.becreated)?;

        self.units[index].shell.destroy();
        self.units[index].wl_surface.destroy();

        if let Some(buffer) = self.units[index].buffer.as_ref() {
            buffer.destroy()
        }
        self.units.remove(index);
        Some(())
    }

    /// forget the remembered last output, next time it will get the new activated output to set the
    /// layershell
    pub fn forget_last_output(&mut self) {
        self.last_wloutput.take();
    }
}

/// Simple WindowState, without any data binding or info
pub type WindowStateSimple = WindowState<()>;

impl<T> WindowState<T> {
    // return the first window
    // I will use it in iced
    pub fn main_window(&self) -> &WindowStateUnit<T> {
        &self.units[0]
    }

    /// use iced id to find WindowStateUnit
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

#[derive(Debug)]
pub struct WindowWrapper {
    pub id: id::Id,
    display: WlDisplay,
    wl_surface: WlSurface,
    pub viewport: Option<WpViewport>,
    pub toplevel: Option<XdgToplevel>,
}

/// Define the way layershell program is start
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum StartMode {
    /// default is use the activated display, in layershell, the param is `None`
    #[default]
    Active,
    /// be started as background program, be used with some programs like xdg-desktop-portal
    Background,
    /// listen on the create event of display, always shown on all screens
    AllScreens,
    /// only shown on target screen
    TargetScreen(String),

    /// Target the output
    /// NOTE: use the same wayland connection
    TargetOutput(WlOutput),
}

impl StartMode {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
    pub fn is_background(&self) -> bool {
        matches!(self, Self::Background)
    }
    pub fn is_allscreens(&self) -> bool {
        matches!(self, Self::AllScreens)
    }
    pub fn is_with_target(&self) -> bool {
        matches!(self, Self::TargetScreen(_))
    }
}

impl WindowWrapper {
    pub fn id(&self) -> id::Id {
        self.id
    }
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
    /// gen the wrapper to the main window
    /// used to get display and etc
    pub fn gen_mainwindow_wrapper(&self) -> WindowWrapper {
        self.main_window().gen_wrapper()
    }

    pub fn is_active(&self) -> bool {
        self.start_mode.is_active()
    }

    pub fn is_background(&self) -> bool {
        self.start_mode.is_background()
    }

    pub fn is_allscreens(&self) -> bool {
        self.start_mode.is_allscreens()
    }

    pub fn is_with_target(&self) -> bool {
        self.start_mode.is_with_target()
    }

    pub fn ime_allowed(&self) -> bool {
        self.ime_allowed
    }

    pub fn set_ime_allowed(&mut self, ime_allowed: bool) {
        self.ime_allowed = ime_allowed;
        for text_input in &self.text_inputs {
            if ime_allowed {
                text_input.enable();
                text_input.set_content_type_by_purpose(self.ime_purpose);
            } else {
                text_input.disable();
            }
            text_input.commit();
        }
    }

    pub fn set_ime_cursor_area<P: Into<dpi::Position>, S: Into<dpi::Size>>(
        &self,
        position: P,
        size: S,
        id: id::Id,
    ) {
        if !self.ime_allowed() {
            return;
        }
        let position: dpi::Position = position.into();
        let size: dpi::Size = size.into();
        let Some(unit) = self.get_window_with_id(id) else {
            return;
        };
        let scale_factor = unit.scale_float();
        let position: dpi::LogicalPosition<u32> = position.to_logical(scale_factor);
        let size: dpi::LogicalSize<u32> = size.to_logical(scale_factor);
        let (x, y) = (position.x as i32, position.y as i32);
        let (width, height) = (size.width as i32, size.height as i32);
        for text_input in self.text_inputs.iter() {
            text_input.set_cursor_rectangle(x, y, width, height);
            text_input.commit();
        }
    }

    pub fn set_ime_purpose(&mut self, purpose: ImePurpose) {
        self.ime_purpose = purpose;
        self.text_input.iter().for_each(|text_input| {
            text_input.set_content_type_by_purpose(purpose);
            text_input.commit();
        });
    }

    #[inline]
    pub fn text_input_entered(&mut self, text_input: &ZwpTextInputV3) {
        if !self.text_inputs.iter().any(|t| t == text_input) {
            self.text_inputs.push(text_input.clone());
        }
    }

    #[inline]
    pub fn text_input_left(&mut self, text_input: &ZwpTextInputV3) {
        if let Some(position) = self.text_inputs.iter().position(|t| t == text_input) {
            self.text_inputs.remove(position);
        }
    }

    fn ime_purpose(&self) -> ImePurpose {
        self.ime_purpose
    }
}

pub trait ZwpTextInputV3Ext {
    fn set_content_type_by_purpose(&self, purpose: ImePurpose);
}

impl ZwpTextInputV3Ext for ZwpTextInputV3 {
    fn set_content_type_by_purpose(&self, purpose: ImePurpose) {
        let (hint, purpose) = match purpose {
            ImePurpose::Normal => (ContentHint::None, ContentPurpose::Normal),
            ImePurpose::Password => (ContentHint::SensitiveData, ContentPurpose::Password),
            ImePurpose::Terminal => (ContentHint::None, ContentPurpose::Terminal),
        };
        self.set_content_type(hint, purpose);
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

impl<T> WindowState<T> {
    /// create a WindowState, you need to pass a namespace in
    pub fn new(namespace: &str) -> Self {
        assert_ne!(namespace, "");
        Self {
            namespace: namespace.to_owned(),
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

    /// set the window size, optional
    pub fn with_option_size(mut self, size: Option<(u32, u32)>) -> Self {
        self.size = size;
        self
    }

    /// exclusive_zone, please take look at [layer_shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1)
    pub fn with_exclusive_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = Some(exclusive_zone);
        self
    }

    /// set layershellev to use display_handle
    pub fn with_use_display_handle(mut self, use_display_handle: bool) -> Self {
        self.use_display_handle = use_display_handle;
        self
    }

    /// set a callback to create a wayland connection
    pub fn with_connection(mut self, connection_or: Option<Connection>) -> Self {
        self.connection = connection_or;
        self
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

            background_surface: None,
            display: None,

            connection: None,
            event_queue: None,
            wl_compositor: None,
            shm: None,
            wmbase: None,
            cursor_manager: None,
            viewporter: None,
            xdg_output_manager: None,
            globals: None,
            fractional_scale_manager: None,
            virtual_keyboard: None,

            seat: None,
            keyboard_state: None,
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
            loop_handler: None,

            last_wloutput: None,
            last_unit_index: 0,

            return_data: Vec::new(),
            finger_locations: HashMap::new(),
            enter_serial: None,
            // NOTE: if is some, means it is to be binded, but not now it
            // is not binded
            xdg_info_cache: Vec::new(),

            start_mode: StartMode::Active,
            init_finished: false,
            events_transparent: false,

            text_input_manager: None,
            text_input: None,
            text_inputs: Vec::new(),
            ime_purpose: ImePurpose::Normal,
            ime_allowed: false,

            xdg_decoration_manager: None,
        }
    }
}

impl<T> WindowState<T> {
    /// You can save the virtual_keyboard here
    pub fn set_virtual_keyboard(&mut self, keyboard: ZwpVirtualKeyboardV1) {
        self.virtual_keyboard = Some(keyboard);
    }

    /// get the saved virtual_keyboard
    pub fn get_virtual_keyboard(&self) -> Option<&ZwpVirtualKeyboardV1> {
        self.virtual_keyboard.as_ref()
    }

    /// with loop_handler you can do more thing
    pub fn get_loop_handler(&self) -> Option<&LoopHandle<'static, Self>> {
        self.loop_handler.as_ref()
    }

    /// use [id::Id] to get the mut [WindowStateUnit]
    fn get_mut_unit_with_id(&mut self, id: id::Id) -> Option<&mut WindowStateUnit<T>> {
        self.units.iter_mut().find(|unit| unit.id == id)
    }

    /// use [id::Id] to get the immutable [WindowStateUnit]
    pub fn get_unit_with_id(&self, id: id::Id) -> Option<&WindowStateUnit<T>> {
        self.units.iter().find(|unit| unit.id == id)
    }

    /// it return the iter of units. you can do loop with it
    pub fn get_unit_iter(&self) -> impl Iterator<Item = &WindowStateUnit<T>> {
        self.units.iter()
    }

    fn surface_pos(&self) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref())
    }

    /// get the current focused surface id
    pub fn current_surface_id(&self) -> Option<id::Id> {
        self.units
            .iter()
            .find(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref())
            .map(|unit| unit.id())
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

            let unit = self
                .units
                .iter()
                .find(|unit| Some(&unit.wl_surface) == self.current_surface.as_ref());
            if let Some(unit) = unit {
                self.message
                    .push((Some(unit.id), DispatchMessageInner::Focused(unit.id)));
                self.last_unit_index = self
                    .outputs
                    .iter()
                    .position(|(_, output)| Some(output) == unit.wl_output.as_ref())
                    .unwrap_or(0);
            }
        }
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

    pub fn request_close(&mut self, id: id::Id) {
        self.get_mut_unit_with_id(id)
            .map(WindowStateUnit::request_close);
    }

    pub fn get_binding_mut(&mut self, id: id::Id) -> Option<&mut T> {
        self.get_mut_unit_with_id(id)
            .and_then(WindowStateUnit::get_binding_mut)
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
                if state
                    .last_wloutput
                    .as_ref()
                    .is_some_and(|output| !output.is_alive())
                {
                    state.last_wloutput.take();
                }
                state.outputs.retain(|x| x.0 != name);
                state.units.retain(|unit| unit.wl_surface.is_alive());
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
            let mut keyboard_installing = true;
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                if state.keyboard_state.is_none() {
                    state.keyboard_state = Some(KeyboardState::new(seat.get_keyboard(qh, ())));
                } else {
                    keyboard_installing = false;
                    let keyboard = state.keyboard_state.take().unwrap();
                    drop(keyboard);
                    if let Some(surface_id) = state.current_surface_id() {
                        state
                            .message
                            .push((Some(surface_id), DispatchMessageInner::Unfocus));
                    }
                }
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                if state.pointer.is_none() {
                    state.pointer = Some(seat.get_pointer(qh, ()));
                } else {
                    let pointer = state.pointer.take().unwrap();
                    pointer.release();
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
            if keyboard_installing {
                let text_input = state
                    .text_input_manager
                    .as_ref()
                    .map(|manager| manager.get_text_input(seat, qh, TextInputData::default()));
                state.text_input = text_input;
            } else if let Some(text_input) = state.text_input.take() {
                text_input.destroy();
            }
        }
    }
}

impl<T> Dispatch<wl_keyboard::WlKeyboard, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _wl_keyboard: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
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
                if let (Some(token), Some(loop_handle)) = (
                    keyboard_state.repeat_token.take(),
                    state.loop_handler.as_ref(),
                ) {
                    loop_handle.remove(token);
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
                    .push((surface_id, DispatchMessageInner::Unfocus));
                if let (Some(token), Some(loop_handle)) = (
                    keyboard_state.repeat_token.take(),
                    state.loop_handler.as_ref(),
                ) {
                    loop_handle.remove(token);
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

                        if let (Some(token), Some(loop_handle)) = (
                            keyboard_state.repeat_token.take(),
                            state.loop_handler.as_ref(),
                        ) {
                            loop_handle.remove(token);
                        }
                        let timer = Timer::from_duration(delay);

                        if let Some(looph) = state.loop_handler.as_ref() {
                            keyboard_state.repeat_token = looph
                                .insert_source(timer, move |_, _, state| {
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
                                    match keyboard_state.repeat_info {
                                        RepeatInfo::Repeat { gap, .. } => {
                                            TimeoutAction::ToDuration(gap)
                                        }
                                        RepeatInfo::Disable => TimeoutAction::Drop,
                                    }
                                })
                                .ok();
                        }
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
                            if let (Some(token), Some(loop_handle)) = (
                                keyboard_state.repeat_token.take(),
                                state.loop_handler.as_ref(),
                            ) {
                                loop_handle.remove(token);
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
                    if let (Some(token), Some(loop_handle)) = (
                        keyboard_state.repeat_token.take(),
                        state.loop_handler.as_ref(),
                    ) {
                        loop_handle.remove(token);
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
                    log::warn!(target: "layershellev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
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
                    log::warn!(target: "layershellev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(source) => state.message.push((
                    surface_id,
                    DispatchMessageInner::Axis {
                        horizontal: AxisScroll::default(),
                        vertical: AxisScroll::default(),
                        scale,
                        source: Some(source),
                        time: 0,
                    },
                )),
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "layershellev", "unknown pointer axis source: {unknown:x}");
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
                    log::warn!(target: "layershellev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
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

impl<T> Dispatch<xdg_surface::XdgSurface, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &xdg_surface::XdgSurface,
        event: <xdg_surface::XdgSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            state
                .units
                .iter_mut()
                .filter(|unit| unit.shell == *surface)
                .for_each(|unit| unit.request_refresh(RefreshRequest::NextFrame));
        }
    }
}

impl<T> Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let unit_index = state.units.iter().position(|unit| unit.shell == *surface);
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);

                let Some(unit_index) = unit_index else {
                    return;
                };
                state.units[unit_index].size = (width, height);

                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                if let Some(i) = unit_index {
                    state.units[i].request_close();
                }
            }
            _ => log::info!("ignore zwlr_layer_surface_v1 event: {event:?}"),
        }
    }
}

impl<T> Dispatch<xdg_toplevel::XdgToplevel, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &xdg_toplevel::XdgToplevel,
        event: <xdg_toplevel::XdgToplevel as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let unit_index = state.units.iter().position(|unit| unit.shell == *surface);
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                let Some(unit_index) = unit_index else {
                    return;
                };
                if width != 0 && height != 0 {
                    state.units[unit_index].size = (width as u32, height as u32);
                }

                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            xdg_toplevel::Event::Close => {
                let Some(unit_index) = unit_index else {
                    return;
                };
                state.units[unit_index].request_flag.close = true;
            }
            _ => {}
        }
    }
}

impl<T> Dispatch<xdg_popup::XdgPopup, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &xdg_popup::XdgPopup,
        event: <xdg_popup::XdgPopup as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_popup::Event::Configure { width, height, .. } = event {
            let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
            else {
                return;
            };
            state.units[unit_index].size = (width as u32, height as u32);

            state.units[unit_index].request_refresh(RefreshRequest::NextFrame)
        }
    }
}

impl<T> Dispatch<zxdg_output_v1::ZxdgOutputV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        proxy: &zxdg_output_v1::ZxdgOutputV1,
        event: <zxdg_output_v1::ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if state.is_with_target() && !state.init_finished {
            let Some((_, xdg_info)) = state
                .xdg_info_cache
                .iter_mut()
                .find(|(_, info)| info.zxdgoutput == *proxy)
            else {
                return;
            };
            match event {
                zxdg_output_v1::Event::LogicalSize { width, height } => {
                    xdg_info.logical_size = (width, height);
                }
                zxdg_output_v1::Event::LogicalPosition { x, y } => {
                    xdg_info.position = (x, y);
                }
                zxdg_output_v1::Event::Name { name } => {
                    xdg_info.name = name;
                }
                zxdg_output_v1::Event::Description { description } => {
                    xdg_info.description = description;
                }
                _ => {}
            };
            return;
        }
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
            zxdg_output_v1::Event::Name { name } => {
                xdg_info.name = name;
                XdgInfoChangedType::Name
            }
            zxdg_output_v1::Event::Description { description } => {
                xdg_info.description = description;
                XdgInfoChangedType::Description
            }
            _ => {
                return;
            }
        };
        state.message.push((
            Some(state.units[index].id),
            DispatchMessageInner::XdgInfoChanged(change_type),
        ));
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
                    scale_u32: scale,
                    scale_float: scale as f64 / 120.,
                },
            ));
        }
    }
}

#[derive(Default)]
pub struct TextInputData {
    inner: std::sync::Mutex<TextInputDataInner>,
}

#[derive(Default)]
pub struct TextInputDataInner {
    /// The `WlSurface` we're performing input to.
    surface: Option<WlSurface>,

    /// The commit to submit on `done`.
    pending_commit: Option<String>,

    /// The preedit to submit on `done`.
    pending_preedit: Option<Preedit>,
}
/// The state of the preedit.
struct Preedit {
    text: String,
    cursor_begin: Option<usize>,
    cursor_end: Option<usize>,
}

impl<T> Dispatch<zwp_text_input_v3::ZwpTextInputV3, TextInputData> for WindowState<T> {
    fn event(
        state: &mut Self,
        text_input: &zwp_text_input_v3::ZwpTextInputV3,
        event: <zwp_text_input_v3::ZwpTextInputV3 as Proxy>::Event,
        data: &TextInputData,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use zwp_text_input_v3::Event;
        let mut text_input_data = data.inner.lock().unwrap();

        match event {
            Event::Enter { surface } => {
                let Some(id) = state.get_id_from_surface(&surface) else {
                    return;
                };
                text_input_data.surface = Some(surface);

                if state.ime_allowed() {
                    text_input.enable();
                    text_input.set_content_type_by_purpose(state.ime_purpose());
                    text_input.commit();
                    state
                        .message
                        .push((Some(id), DispatchMessageInner::Ime(events::Ime::Enabled)));
                }
                state.text_input_entered(text_input);
            }
            Event::Leave { surface } => {
                text_input_data.surface = None;

                text_input.disable();
                text_input.commit();
                let Some(id) = state.get_id_from_surface(&surface) else {
                    return;
                };
                state.text_input_left(text_input);
                state
                    .message
                    .push((Some(id), DispatchMessageInner::Ime(events::Ime::Disabled)));
            }
            Event::CommitString { text } => {
                text_input_data.pending_preedit = None;
                text_input_data.pending_commit = text;
            }
            Event::DeleteSurroundingText { .. } => {}
            Event::Done { .. } => {
                let Some(id) = text_input_data
                    .surface
                    .as_ref()
                    .and_then(|surface| state.get_id_from_surface(surface))
                else {
                    return;
                };
                // Clear preedit, unless all we'll be doing next is sending a new preedit.
                if text_input_data.pending_commit.is_some()
                    || text_input_data.pending_preedit.is_none()
                {
                    state.message.push((
                        Some(id),
                        DispatchMessageInner::Ime(Ime::Preedit(String::new(), None)),
                    ));
                }

                // Send `Commit`.
                if let Some(text) = text_input_data.pending_commit.take() {
                    state
                        .message
                        .push((Some(id), DispatchMessageInner::Ime(Ime::Commit(text))));
                }

                // Send preedit.
                if let Some(preedit) = text_input_data.pending_preedit.take() {
                    let cursor_range = preedit
                        .cursor_begin
                        .map(|b| (b, preedit.cursor_end.unwrap_or(b)));

                    state.message.push((
                        Some(id),
                        DispatchMessageInner::Ime(Ime::Preedit(preedit.text, cursor_range)),
                    ));
                }
            }
            Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                let text = text.unwrap_or_default();
                let cursor_begin = usize::try_from(cursor_begin)
                    .ok()
                    .and_then(|idx| text.is_char_boundary(idx).then_some(idx));
                let cursor_end = usize::try_from(cursor_end)
                    .ok()
                    .and_then(|idx| text.is_char_boundary(idx).then_some(idx));

                text_input_data.pending_preedit = Some(Preedit {
                    text,
                    cursor_begin,
                    cursor_end,
                })
            }

            _ => {}
        }
    }
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
        if let WlCallbackEvent::Done { callback_data: _ } = event {
            if let Some(unit) = state.get_mut_unit_with_id(data.0) {
                unit.present_available_state = data.1;
            }
        }
    }
}

delegate_noop!(@<T> WindowState<T>: ignore WlCompositor); // WlCompositor is need to create a surface
delegate_noop!(@<T> WindowState<T>: ignore WlSurface); // surface is the base needed to show buffer
delegate_noop!(@<T> WindowState<T>: ignore WlOutput); // output is need to place layer_shell, although here
// it is not used
delegate_noop!(@<T> WindowState<T>: ignore WlShm); // shm is used to create buffer pool
delegate_noop!(@<T> WindowState<T>: ignore WlShmPool); // so it is pool, created by wl_shm
delegate_noop!(@<T> WindowState<T>: ignore WlBuffer); // buffer show the picture
delegate_noop!(@<T> WindowState<T>: ignore WlRegion); // region is used to modify input region
delegate_noop!(@<T> WindowState<T>: ignore ZwlrLayerShellV1); // it is similar with xdg_toplevel, also the
// ext-session-shell

delegate_noop!(@<T> WindowState<T>: ignore WpCursorShapeManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore WpCursorShapeDeviceV1);

delegate_noop!(@<T> WindowState<T>: ignore WpViewporter);
delegate_noop!(@<T> WindowState<T>: ignore WpViewport);

delegate_noop!(@<T> WindowState<T>: ignore ZwpVirtualKeyboardV1);
delegate_noop!(@<T> WindowState<T>: ignore ZwpVirtualKeyboardManagerV1);

delegate_noop!(@<T> WindowState<T>: ignore ZxdgOutputManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore WpFractionalScaleManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore XdgPositioner);
delegate_noop!(@<T> WindowState<T>: ignore XdgWmBase);

delegate_noop!(@<T> WindowState<T>: ignore ZwpTextInputManagerV3);
delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelSurfaceV1);
delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelV1);

delegate_noop!(@<T> WindowState<T>: ignore ZxdgDecorationManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore ZxdgToplevelDecorationV1);

impl<T: 'static> WindowState<T> {
    /// build a new WindowState
    pub fn build(mut self) -> Result<Self, LayerEventError> {
        let connection = if let Some(connection) = self.connection.take() {
            connection
        } else {
            Connection::connect_to_env()?
        };
        let (globals, _) = registry_queue_init::<BaseState>(&connection)?;

        self.display = Some(connection.display());
        let mut event_queue = connection.new_event_queue::<WindowState<T>>();
        let qh = event_queue.handle();

        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?;

        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        self.shm = Some(shm);
        self.seat = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let wmbase = globals.bind::<XdgWmBase, _, _>(&qh, 2..=6, ())?;
        self.wmbase = Some(wmbase);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();

        let _ = connection.display().get_registry(&qh, ()); // so if you want WlOutput, you need to
        // register this

        let xdg_output_manager = globals.bind::<ZxdgOutputManagerV1, _, _>(&qh, 1..=3, ())?; // bind
        // xdg_output_manager

        let decoration_manager = globals
            .bind::<ZxdgDecorationManagerV1, _, _>(&qh, 1..=1, ())
            .ok();

        self.xdg_decoration_manager = decoration_manager;

        let fractional_scale_manager = globals
            .bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let text_input_manager = globals
            .bind::<ZwpTextInputManagerV3, _, _>(&qh, 1..=1, ())
            .ok();

        self.text_input_manager = text_input_manager;
        event_queue.blocking_dispatch(&mut self)?; // then make a dispatch

        // do the step before, you get empty list

        // so it is the same way, to get surface detach to protocol, first get the shell, like wmbase
        // or layer_shell or session-shell, then get `surface` from the wl_surface you get before, and
        // set it
        // finally thing to remember is to commit the surface, make the shell to init.
        //let (init_w, init_h) = self.size;
        // this example is ok for both xdg_surface and layer_shell
        if self.is_background() {
            let background_surface = wmcompositer.create_surface(&qh, ());
            if self.events_transparent {
                let region = wmcompositer.create_region(&qh, ());
                background_surface.set_input_region(Some(&region));
                region.destroy();
            }
            self.background_surface = Some(background_surface);
        } else if !self.is_allscreens() {
            let mut output = None;

            let (binded_output, binded_xdginfo) = match self.start_mode.clone() {
                StartMode::TargetScreen(name) => {
                    for (_, output_display) in &self.outputs {
                        let zxdgoutput = xdg_output_manager.get_xdg_output(output_display, &qh, ());
                        self.xdg_info_cache
                            .push((output_display.clone(), ZxdgOutputInfo::new(zxdgoutput)));
                    }
                    event_queue.blocking_dispatch(&mut self)?; // then make a dispatch
                    if let Some(cache) = self
                        .xdg_info_cache
                        .iter()
                        .find(|(_, info)| info.name == *name)
                        .cloned()
                    {
                        output = Some(cache.clone());
                    }
                    self.xdg_info_cache.clear();
                    let binded_output = output.as_ref().map(|(output, _)| output).cloned();
                    let binded_xdginfo = output.as_ref().map(|(_, xdginfo)| xdginfo).cloned();
                    (binded_output, binded_xdginfo)
                }
                StartMode::TargetOutput(output) => (Some(output), None),
                _ => (None, None),
            };

            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            let layer_shell = globals
                .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                .unwrap();
            let layer = layer_shell.get_layer_surface(
                &wl_surface,
                binded_output.as_ref(),
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
            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
            // and if you need to reconfigure it, you need to commit the wl_surface again
            // so because this is just an example, so we just commit it once
            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
            self.push_window(
                WindowStateUnitBuilder::new(
                    id::Id::unique(),
                    qh.clone(),
                    connection.display(),
                    wl_surface,
                    Shell::LayerShell(layer),
                )
                .viewport(viewport)
                .zxdgoutput(binded_xdginfo)
                .fractional_scale(fractional_scale)
                .wl_output(binded_output.clone())
                .build(),
            );
        } else {
            let displays = self.outputs.clone();
            for (_, output_display) in displays.iter() {
                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                let layer_shell = globals
                    .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                    .unwrap();
                let layer = layer_shell.get_layer_surface(
                    &wl_surface,
                    Some(output_display),
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

                if self.events_transparent {
                    let region = wmcompositer.create_region(&qh, ());
                    wl_surface.set_input_region(Some(&region));
                    region.destroy();
                }
                wl_surface.commit();

                let zxdgoutput = xdg_output_manager.get_xdg_output(output_display, &qh, ());
                let mut fractional_scale = None;
                if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                    fractional_scale =
                        Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
                }
                let viewport = viewporter
                    .as_ref()
                    .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                // and if you need to reconfigure it, you need to commit the wl_surface again
                // so because this is just an example, so we just commit it once
                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                self.push_window(
                    WindowStateUnitBuilder::new(
                        id::Id::unique(),
                        qh.clone(),
                        connection.display(),
                        wl_surface,
                        Shell::LayerShell(layer),
                    )
                    .viewport(viewport)
                    .zxdgoutput(Some(ZxdgOutputInfo::new(zxdgoutput)))
                    .fractional_scale(fractional_scale)
                    .wl_output(Some(output_display.clone()))
                    .build(),
                );
            }
            self.message.clear();
        }
        self.init_finished = true;
        self.viewporter = viewporter;
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
    /// pass a LayerShellEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit_with_id] if the even is not spical on one surface,
    /// it will return [None].
    /// Different with running, it receiver a receiver
    pub fn running_with_proxy<F, Message>(
        self,
        message_receiver: std::sync::mpsc::Receiver<Message>,
        event_handler: F,
    ) -> Result<(), LayerEventError>
    where
        Message: std::marker::Send + 'static,
        F: FnMut(LayerShellEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData<T>
            + 'static,
    {
        self.running_with_proxy_option(Some(message_receiver), event_handler)
    }
    /// main event loop, every time dispatch, it will store the messages, and do callback. it will
    /// pass a LayerShellEvent, with self as mut, the last `Option<usize>` describe which unit the event
    /// happened on, like tell you this time you do a click, what surface it is on. you can use the
    /// index to get the unit, with [WindowState::get_unit_with_id] if the even is not spical on one surface,
    /// it will return [None].
    ///
    pub fn running<F>(self, event_handler: F) -> Result<(), LayerEventError>
    where
        F: FnMut(LayerShellEvent<T, ()>, &mut WindowState<T>, Option<id::Id>) -> ReturnData<T>
            + 'static,
    {
        self.running_with_proxy_option(None, event_handler)
    }

    fn running_with_proxy_option<F, Message>(
        mut self,
        message_receiver: Option<std::sync::mpsc::Receiver<Message>>,
        mut event_handler: F,
    ) -> Result<(), LayerEventError>
    where
        Message: std::marker::Send + 'static,
        F: FnMut(LayerShellEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData<T>
            + 'static,
    {
        let globals = self.globals.take().unwrap();
        let event_queue = self.event_queue.take().unwrap();
        let qh = event_queue.handle();
        let wmcompositer = self.wl_compositor.take().unwrap();
        let shm = self.shm.take().unwrap();
        let fractional_scale_manager = self.fractional_scale_manager.take();
        let cursor_manager: Option<WpCursorShapeManagerV1> = self.cursor_manager.take();
        let xdg_output_manager = self.xdg_output_manager.take().unwrap();
        let connection = self.connection.take().unwrap();
        let mut init_event = None;
        let wmbase = self.wmbase.take().unwrap();
        let viewporter = self.viewporter.take();
        let zxdg_decoration_manager = self.xdg_decoration_manager.take();

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
                    init_event = Some(event_handler(LayerShellEvent::InitRequest, &mut self, None));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(event_handler(
                        LayerShellEvent::BindProvide(&globals, &qh),
                        &mut self,
                        None,
                    ));
                }
                Some(ReturnData::RequestCompositor) => {
                    init_event = Some(event_handler(
                        LayerShellEvent::CompositorProvide(&wmcompositer, &qh),
                        &mut self,
                        None,
                    ));
                }
                _ => panic!("Not provide server here"),
            }
        }

        let mut event_loop: EventLoop<Self> =
            EventLoop::try_new().expect("Failed to initialize the event loop");

        WaylandSource::new(connection.clone(), event_queue)
            .insert(event_loop.handle())
            .expect("Failed to init wayland source");

        self.loop_handler = Some(event_loop.handle());

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
                move |_, _, window_state| {
                    let mut messages = Vec::new();
                    std::mem::swap(&mut messages, &mut window_state.message);
                    for msg in messages.iter() {
                        match msg {
                            (index_info, DispatchMessageInner::XdgInfoChanged(change_type)) => {
                                window_state.handle_event(
                                    &mut event_handler,
                                    LayerShellEvent::XdgInfoChanged(*change_type),
                                    *index_info,
                                );
                            }
                            (_, DispatchMessageInner::NewDisplay(output_display)) => {
                                if !window_state.is_allscreens() {
                                    continue;
                                }
                                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                let layer_shell = globals
                                    .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                                    .unwrap();
                                let layer = layer_shell.get_layer_surface(
                                    &wl_surface,
                                    Some(output_display),
                                    window_state.layer,
                                    window_state.namespace.clone(),
                                    &qh,
                                    (),
                                );
                                layer.set_anchor(window_state.anchor);
                                layer
                                    .set_keyboard_interactivity(window_state.keyboard_interactivity);
                                if let Some((init_w, init_h)) = window_state.size {
                                    layer.set_size(init_w, init_h);
                                }

                                if let Some(zone) = window_state.exclusive_zone {
                                    layer.set_exclusive_zone(zone);
                                }

                                if let Some((top, right, bottom, left)) = window_state.margin {
                                    layer.set_margin(top, right, bottom, left);
                                }

                                if window_state.events_transparent {
                                    let region = wmcompositer.create_region(&qh, ());
                                    wl_surface.set_input_region(Some(&region));
                                    region.destroy();
                                }
                                wl_surface.commit();

                                let zxdgoutput =
                                    xdg_output_manager.get_xdg_output(output_display, &qh, ());
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
                                let viewport = viewporter
                                    .as_ref()
                                    .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                                // and if you need to reconfigure it, you need to commit the wl_surface again
                                // so because this is just an example, so we just commit it once
                                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                                window_state.push_window(
                                    WindowStateUnitBuilder::new(
                                        id::Id::unique(),
                                        qh.clone(),
                                        connection.display(),
                                        wl_surface,
                                        Shell::LayerShell(layer),
                                    )
                                    .viewport(viewport)
                                    .zxdgoutput(Some(ZxdgOutputInfo::new(zxdgoutput)))
                                    .fractional_scale(fractional_scale)
                                    .wl_output(Some(output_display.clone()))
                                    .build(),
                                );
                            }
                            _ => {
                                let (index_message, msg) = msg;

                                let msg: DispatchMessage = msg.clone().into();
                                window_state.handle_event(
                                    &mut event_handler,
                                    LayerShellEvent::RequestMessages(&msg),
                                    *index_message,
                                );
                            }
                        }
                    }

                    let mut local_events = events.lock().expect(
                        "This events only used in this callback, so it should always can be unlocked",
                    );
                    let mut swapped_events: Vec<Message> = vec![];
                    std::mem::swap(&mut *local_events, &mut swapped_events);
                    drop(local_events);
                    for event in swapped_events {
                        window_state.handle_event(&mut event_handler, LayerShellEvent::UserEvent(event), None);
                    }
                    window_state.handle_event(&mut event_handler, LayerShellEvent::NormalDispatch, None);
                    loop {
                        let mut return_data = vec![];
                        std::mem::swap(&mut window_state.return_data, &mut return_data);

                        for data in return_data {
                            match data {
                                ReturnData::RequestExit => {
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
                                ReturnData::NewLayerShell((
                                    NewLayerShellSettings {
                                        size,
                                        layer,
                                        anchor,
                                        exclusive_zone,
                                        margin,
                                        keyboard_interactivity,
                                        output_option: output_type,
                                        events_transparent,
                                        namespace,
                                    },
                                    id,
                                    info,
                                )) => {
                                    let output = match output_type {
                                        OutputOption::Output(output) => Some(output),
                                        _ => {
                                            let pos = window_state.surface_pos();

                                            let mut output =
                                                pos.and_then(|p| window_state.units[p].wl_output.as_ref());

                                            if window_state.last_wloutput.is_none()
                                                && window_state.outputs.len() > window_state.last_unit_index
                                            {
                                                window_state.last_wloutput = Some(
                                                    window_state.outputs[window_state.last_unit_index]
                                                        .1
                                                        .clone(),
                                                );
                                            }

                                            if matches!(output_type, events::OutputOption::LastOutput) {
                                                output = window_state.last_wloutput.as_ref();
                                            }

                                            output.cloned()
                                        }
                                    };


                                    let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                    let layer_shell = globals
                                        .bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ())
                                        .unwrap();
                                    let layer = layer_shell.get_layer_surface(
                                        &wl_surface,
                                        output.as_ref(),
                                        layer,
                                        namespace.unwrap_or_else(|| window_state.namespace.clone()),
                                        &qh,
                                        (),
                                    );
                                    layer.set_anchor(anchor);
                                    layer.set_keyboard_interactivity(keyboard_interactivity);
                                    if let Some((init_w, init_h)) = size {
                                        layer.set_size(init_w, init_h);
                                    }

                                    if let Some(zone) = exclusive_zone {
                                        layer.set_exclusive_zone(zone);
                                    }

                                    if let Some((top, right, bottom, left)) = margin {
                                        layer.set_margin(top, right, bottom, left);
                                    }

                                    if events_transparent {
                                        let region = wmcompositer.create_region(&qh, ());
                                        wl_surface.set_input_region(Some(&region));
                                        region.destroy();
                                    }

                                    wl_surface.commit();

                                    let mut fractional_scale = None;
                                    if let Some(ref fractional_scale_manager) =
                                        fractional_scale_manager
                                    {
                                        fractional_scale =
                                            Some(fractional_scale_manager.get_fractional_scale(
                                                &wl_surface,
                                                &qh,
                                                (),
                                            ));
                                    }
                                    let viewport = viewporter.as_ref().map(|viewport| {
                                        viewport.get_viewport(&wl_surface, &qh, ())
                                    });
                                    // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                                    // and if you need to reconfigure it, you need to commit the wl_surface again
                                    // so because this is just an example, so we just commit it once
                                    // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed

                                    window_state.push_window(
                                        WindowStateUnitBuilder::new(
                                            id,
                                            qh.clone(),
                                            connection.display(),
                                            wl_surface,
                                            Shell::LayerShell(layer),
                                        )
                                        .viewport(viewport)
                                        .fractional_scale(fractional_scale)
                                        .wl_output(output)
                                        .binding(info)
                                        .becreated(true)
                                        .build(),
                                    );
                                }
                                ReturnData::NewPopUp((
                                    NewPopUpSettings {
                                        size: (width, height),
                                        position: (x, y),
                                        id,
                                    },
                                    targetid,
                                    info,
                                )) => {
                                    let Some(index) = window_state
                                        .units
                                        .iter()
                                        .position(|unit| !unit.is_popup() && unit.id == id)
                                    else {
                                        continue;
                                    };
                                    let wl_surface = wmcompositer.create_surface(&qh, ());
                                    let positioner = wmbase.create_positioner(&qh, ());
                                    positioner.set_size(width as i32, height as i32);
                                    positioner.set_anchor_rect(x, y, width as i32, height as i32);
                                    let wl_xdg_surface =
                                        wmbase.get_xdg_surface(&wl_surface, &qh, ());
                                    let popup =
                                        wl_xdg_surface.get_popup(None, &positioner, &qh, ());

                                    let Shell::LayerShell(shell) = &window_state.units[index].shell
                                    else {
                                        unreachable!()
                                    };
                                    shell.get_popup(&popup);

                                    let mut fractional_scale = None;
                                    if let Some(ref fractional_scale_manager) =
                                        fractional_scale_manager
                                    {
                                        fractional_scale =
                                            Some(fractional_scale_manager.get_fractional_scale(
                                                &wl_surface,
                                                &qh,
                                                (),
                                            ));
                                    }
                                    wl_surface.commit();

                                    let viewport = viewporter.as_ref().map(|viewport| {
                                        viewport.get_viewport(&wl_surface, &qh, ())
                                    });
                                    window_state.push_window(
                                        WindowStateUnitBuilder::new(
                                            targetid,
                                            qh.clone(),
                                            connection.display(),
                                            wl_surface,
                                            Shell::PopUp((popup, wl_xdg_surface)),
                                        )
                                        .size((width, height))
                                        .viewport(viewport)
                                        .fractional_scale(fractional_scale)
                                        .binding(info)
                                        .becreated(true)
                                        .build(),
                                    );
                                },
                                ReturnData::NewXdgBase((
                                    NewXdgWindowSettings{
                                        title,
                                        size
                                    },
                                    id,
                                    info,
                                )) => {
                                    let wl_surface = wmcompositer.create_surface(&qh, ());
                                    let wl_xdg_surface =
                                        wmbase.get_xdg_surface(&wl_surface, &qh, ());
                                    let toplevel =
                                        wl_xdg_surface.get_toplevel(&qh, ());

                                    toplevel.set_title(title.unwrap_or("".to_owned()));

                                    let decoration = if let Some(decoration_manager) = &zxdg_decoration_manager {
                                        let decoration = decoration_manager.get_toplevel_decoration(&toplevel, &qh, ());
                                        use zxdg_toplevel_decoration_v1::Mode;
                                        decoration.set_mode(Mode::ServerSide);
                                        Some(decoration)

                                    } else {
                                        None
                                    };
                                    let mut fractional_scale = None;
                                    if let Some(ref fractional_scale_manager) =
                                        fractional_scale_manager
                                    {
                                        fractional_scale =
                                            Some(fractional_scale_manager.get_fractional_scale(
                                                &wl_surface,
                                                &qh,
                                                (),
                                            ));
                                    }
                                    wl_surface.commit();

                                    let viewport = viewporter.as_ref().map(|viewport| {
                                        viewport.get_viewport(&wl_surface, &qh, ())
                                    });
                                    window_state.push_window(
                                        WindowStateUnitBuilder::new(
                                            id,
                                            qh.clone(),
                                            connection.display(),
                                            wl_surface,
                                            Shell::XdgTopLevel((toplevel, wl_xdg_surface, decoration)),
                                        )
                                        .size(size.unwrap_or((300, 300)))
                                        .viewport(viewport)
                                        .fractional_scale(fractional_scale)
                                        .binding(info)
                                        .becreated(true)
                                        .build(),
                                    );
                                },

                                ReturnData::NewInputPanel((
                                    NewInputPanelSettings {
                                        size: (width, height),
                                        keyboard,
                                        use_last_output,
                                    },
                                    id,
                                    info,
                                )) => {
                                    let pos = window_state.surface_pos();

                                    let mut output = pos.and_then(|p| window_state.units[p].wl_output.as_ref());

                                    if window_state.last_wloutput.is_none()
                                        && window_state.outputs.len() > window_state.last_unit_index
                                    {
                                        window_state.last_wloutput =
                                            Some(window_state.outputs[window_state.last_unit_index].1.clone());
                                    }

                                    if use_last_output {
                                        output = window_state.last_wloutput.as_ref();
                                    }

                                    if output.is_none() {
                                        output = window_state.outputs.first().map(|(_, o)| o);
                                    }

                                    let Some(output) = output else {
                                        log::warn!("no WlOutput, skip creating input panel");
                                        continue;
                                    };

                                    let wl_surface = wmcompositer.create_surface(&qh, ());
                                    let input_panel = globals
                                        .bind::<ZwpInputPanelV1, _, _>(&qh, 1..=1, ())
                                        .unwrap();
                                    let input_panel_surface =
                                        input_panel.get_input_panel_surface(&wl_surface, &qh, ());
                                    if keyboard {
                                        input_panel_surface.set_toplevel(
                                            output,
                                            ZwpInputPanelPosition::CenterBottom as u32,
                                        );
                                    } else {
                                        input_panel_surface.set_overlay_panel();
                                    }
                                    wl_surface.commit();

                                    let mut fractional_scale = None;
                                    if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                                        fractional_scale =
                                            Some(fractional_scale_manager.get_fractional_scale(
                                                &wl_surface,
                                                &qh,
                                                (),
                                            ));
                                    }

                                    let viewport = viewporter
                                        .as_ref()
                                        .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                                    window_state.push_window(
                                        WindowStateUnitBuilder::new(
                                            id,
                                            qh.clone(),
                                            connection.display(),
                                            wl_surface,
                                            Shell::InputPanel(input_panel_surface),
                                        )
                                        .size((width, height))
                                        .viewport(viewport)
                                        .fractional_scale(fractional_scale)
                                        .binding(info)
                                        .becreated(true)
                                        .build(),
                                    );
                                },
                                _ => {}
                            }
                        }
                        if window_state.return_data.is_empty() {
                            break;
                        }
                    }

                    let to_be_closed_ids: Vec<_> = window_state
                        .units
                        .iter()
                        .filter(|unit| unit.request_flag.close)
                        .map(WindowStateUnit::id)
                        .collect();
                    for id in to_be_closed_ids {
                        window_state.handle_event(
                            &mut event_handler,
                            LayerShellEvent::RequestMessages(&DispatchMessage::Closed),
                            Some(id),
                        );
                        // event_handler may use unit, only remove it after calling event_handler.
                        window_state.remove_shell(id);
                    }

                    for idx in 0..window_state.units.len() {
                        let unit = &mut window_state.units[idx];
                        let (width, height) = unit.size;
                        if width == 0 || height == 0 {
                            // don't refresh, if size is 0.
                            continue;
                        }
                        if unit.take_present_slot() {
                            let unit_id = unit.id;
                            let is_created = unit.becreated;
                            let scale_float = unit.scale_float();
                            let wl_surface = unit.wl_surface.clone();
                            if unit.buffer.is_none() && !window_state.use_display_handle {
                                let Ok(mut file) = tempfile::tempfile() else {
                                    log::error!("Cannot create new file from tempfile");
                                    return TimeoutAction::Drop;
                                };
                                let ReturnData::WlBuffer(buffer) = event_handler(
                                    LayerShellEvent::RequestBuffer(&mut file, &shm, &qh, width, height),
                                    window_state,
                                    Some(unit_id)) else {
                                    panic!("You cannot return this one");
                                };
                                wl_surface.attach(Some(&buffer), 0, 0);
                                wl_surface.commit();
                                window_state.units[idx].buffer = Some(buffer);
                            }
                            window_state.handle_event(
                                &mut event_handler,
                                LayerShellEvent::RequestMessages(&DispatchMessage::RequestRefresh {
                                    width,
                                    height,
                                    is_created,
                                    scale_float,
                                }),
                                Some(unit_id),
                            );
                            // reset if the slot is not used
                            window_state.units[idx].reset_present_slot();
                        }
                    }
                    TimeoutAction::ToDuration(std::time::Duration::from_millis(50))
                },
            )
            .expect("Cannot insert_source");
        event_loop
            .run(
                std::time::Duration::from_millis(20),
                &mut self,
                |_window_state| {
                    // Finally, this is where you can insert the processing you need
                    // to do do between each waiting event eg. drawing logic if
                    // you're doing a GUI app.
                },
            )
            .expect("Error during event loop!");
        to_exit.store(true, Ordering::Relaxed);
        let _ = thread.join();
        Ok(())
    }

    pub fn request_next_present(&mut self, id: id::Id) {
        self.get_mut_unit_with_id(id)
            .map(WindowStateUnit::request_next_present);
    }

    pub fn reset_present_slot(&mut self, id: id::Id) {
        self.get_mut_unit_with_id(id)
            .map(WindowStateUnit::reset_present_slot);
    }

    pub fn handle_event<F, Message>(
        &mut self,
        mut event_handler: F,
        event: LayerShellEvent<T, Message>,
        unit_id: Option<id::Id>,
    ) where
        Message: std::marker::Send + 'static,
        F: FnMut(LayerShellEvent<T, Message>, &mut WindowState<T>, Option<id::Id>) -> ReturnData<T>,
    {
        let return_data = event_handler(event, self, unit_id);
        if !matches!(return_data, ReturnData::None) {
            self.append_return_data(return_data);
        }
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
