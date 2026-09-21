//! # Handle the layer_shell in a winit way
//!
//! Min example is under
//! ```rust, no_run
//! use std::fs::File;
//! use std::os::fd::AsFd;
//!
//! use exwlshellev::keyboard::{KeyCode, PhysicalKey};
//! use exwlshellev::reexport::*;
//! use exwlshellev::*;
//!
//! struct Window;
//! impl WindowTrait<()> for Window {
//!    fn request_buffer(
//!        &mut self,
//!        _state: &mut WindowState<()>,
//!        file: &mut std::fs::File,
//!        shm: &wl_shm::WlShm,
//!        qh: &wayland_client::QueueHandle<WindowState<()>>,
//!        width: u32,
//!        height: u32,
//!    ) -> wayland_client::WlBuffer {
//!        draw(file, (width, height));
//!        let pool = shm.create_pool(file.as_fd(), (width * height * 4) as i32, qh, ());
//!        pool.create_buffer(
//!            0,
//!            width as i32,
//!            height as i32,
//!            (width * 4) as i32,
//!            wl_shm::Format::Argb8888,
//!            qh,
//!            (),
//!        )
//!    }
//!     fn on_event(
//!         &mut self,
//!         event: ExWlShellEvent<()>,
//!         state: &mut WindowState<()>,
//!         _id: Option<id::Id>,
//!     ) -> ReturnData<()> {
//!         match event {
//!             // NOTE: this will send when init, you can request bind extra object from here
//!             ExWlShellEvent::InitRequest => ReturnData::RequestBind,
//!             ExWlShellEvent::BindProvide(globals, qh) => {
//!                 // NOTE: you can get implied wayland object from here
//!                 let virtual_keyboard_manager = globals
//!                     .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
//!                         qh,
//!                         1..=1,
//!                         (),
//!                     )
//!                     .unwrap();
//!                 println!("{:?}", virtual_keyboard_manager);
//!                 ReturnData::RequestCompositor
//!             }
//!            ExWlShellEvent::CompositorProvide(_compositor, _qh) => {
//!                // NOTE: this is an example to use the CompositorProvide,
//!                // but this is quite useless, because you can get the window_unit to set it directly
//!                // NOTE: you can set input region to limit area which gets input events
//!                // surface outside region becomes transparent for input events
//!                // To ignore all input events use region with (0,0) size
//!                // for x in state.get_unit_iter() {
//!                //     let region = _compositor.create_region(_qh, ());
//!                //     region.add(0, 0, 0, 0);
//!                //     x.get_wlsurface().set_input_region(Some(&region));
//!                // }
//!                ReturnData::None
//!            }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::RequestRefresh {
//!                 width,
//!                 height,
//!                 ..
//!             }) => {
//!                 println!("{width}, {height}");
//!                 ReturnData::None
//!             }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::MouseButton { .. }) => {
//!                 ReturnData::None
//!             }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::MouseEnter { pointer, .. }) => {
//!                 ReturnData::RequestSetCursor((
//!                     Cursor::Shape(CursorShape::Crosshair),
//!                     pointer.clone(),
//!                 ))
//!             }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::MouseMotion {
//!                 time,
//!                 surface_x,
//!                 surface_y,
//!             }) => {
//!                 println!("{time}, {surface_x}, {surface_y}");
//!                 ReturnData::None
//!             }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::OutputChanged(output)) => {
//!                 // NOTE: sent when surface enters another output, or its output info changes
//!                 let info = output.as_ref().and_then(|o| state.get_output_info_of(o));
//!                 println!("{info:?}");
//!                 ReturnData::None
//!             }
//!             ExWlShellEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
//!                 if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
//!                     ReturnData::RequestExit
//!                 } else {
//!                     ReturnData::None
//!                 }
//!             }
//!             _ => ReturnData::None,
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let window = Window;
//!     let ev: EventContext<(), _> = WindowState::new("Hello")
//!         .with_allscreens()
//!         .with_size(LayerSize::fill_width(400))
//!         .with_layer(Layer::Top)
//!         .with_margin((20, 20, 100, 20))
//!         .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
//!         .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
//!         .with_exclusive_zone(-1)
//!         .build(window)
//!         .unwrap();
//!
//!     ev.run().unwrap()
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
use calloop::LoopSignal;
pub use events::NewInputPanelSettings;
pub use events::NewLayerShellSettings;
pub use events::NewXdgWindowSettings;
pub use events::OutputOption;
pub use events::ToplevelState;
pub use events::{NewPopUpSettings, PopUpRepositionSettings, PopupPlacement};
pub use sctk::output::OutputInfo;
pub use waycrate_xkbkeycode::keyboard;
pub use waycrate_xkbkeycode::xkb_keyboard;
pub mod blur;
pub mod dpi;
mod events;
mod seat;
mod size;

use events::DispatchMessageInner;
use size::warn_if_exclusive_zone_ignored;
pub use size::{Extent, LayerSize, PixelSize};

pub mod id;

pub use events::{AxisScroll, Cursor, DispatchMessage, ExWlShellEvent, Ime, ReturnData};
pub use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape as CursorShape;

use waycrate_xkbkeycode::xkb_keyboard::ElementState;
use waycrate_xkbkeycode::xkb_keyboard::RepeatInfo;

use sctk::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::SeatState,
};
use wayland_backend::client::ObjectId;
use wayland_client::protocol::wl_surface;
use wayland_client::{
    ConnectError, Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle,
    delegate_noop,
    globals::{BindError, GlobalError, GlobalList, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::{Event as WlCallbackEvent, WlCallback},
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_keyboard::KeyState,
        wl_output::{self, WlOutput},
        wl_pointer::WlPointer,
        wl_region::WlRegion,
        wl_seat::WlSeat,
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_cursor::{CursorImageBuffer, CursorTheme};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
    ext_session_lock_v1::ExtSessionLockV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::KeyboardInteractivity;
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, Anchor, ZwlrLayerSurfaceV1},
};

use wayland_protocols::xdg::shell::client::{
    xdg_popup::{self, XdgPopup},
    xdg_positioner::{self, XdgPositioner},
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
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

use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
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
    Error as CallLoopError, EventLoop, LoopHandle, RegistrationToken, channel,
    timer::{TimeoutAction, Timer},
};
use calloop_wayland_source::WaylandSource;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use std::fmt::Formatter;
use std::time::Duration;
use std::time::Instant;

use crate::blur::{BlurOption, BlurRegion};
use crate::seat::SeatStorage;

#[derive(Debug, thiserror::Error)]
pub enum ExShellEventError {
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
    EventLoopError(#[from] CallLoopError),
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
    pub mod xdg_positioner {
        pub use wayland_protocols::xdg::shell::client::xdg_positioner::{
            Anchor, ConstraintAdjustment, Gravity,
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
                wl_buffer::WlBuffer,
            },
        };
    }
    pub mod wp_cursor_shape_device_v1 {
        pub use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
    }
    pub mod xdg_toplevel {
        pub use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
    }
    pub mod wp_viewport {
        pub use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WlShellType {
    LayerShell,
    PopUp,
    XdgTopLevel,
    InputPanel,
    SessionLock,
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
    SessionLock(ExtSessionLockSurfaceV1),
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
            Self::PopUp((_, surface)) | Self::XdgTopLevel((_, surface, _)) => surface == other,
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
impl PartialEq<ExtSessionLockSurfaceV1> for Shell {
    fn eq(&self, other: &ExtSessionLockSurfaceV1) -> bool {
        match self {
            Self::SessionLock(lock) => lock == other,
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
            Self::SessionLock(lock) => lock.destroy(),
        }
    }

    fn is_lock(&self) -> bool {
        matches!(self, Self::SessionLock(_))
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
        wmcompositor: WlCompositor,
        shell: Shell,
    ) -> Self {
        let configured = matches!(shell, Shell::InputPanel(_));
        Self {
            inner: WindowStateUnit {
                id,
                qh,
                window: Arc::new(WindowWrapper {
                    id,
                    display,
                    wl_surface,
                    viewport: None,
                    toplevel: shell.top_level(),
                }),
                wmcompositor,
                shell,
                parent: None,
                size: (0, 0),
                anchor: Anchor::empty(),
                layer_size: LayerSize::FILL,
                buffer: Default::default(),
                fractional_scale: Default::default(),
                wl_outputs: Default::default(),
                pending_leave: None,
                binding: Default::default(),
                configured,
                blur_option: BlurOption::None,
                pending_reposition: None,
                toplevel_state: ToplevelState::default(),
                effect: None,
                // Unknown why it is 120
                scale: 120,
                request_flag: Default::default(),
                present_available_state: Default::default(),
                frame_callback: None,
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

    fn layout(mut self, anchor: Anchor, layer_size: LayerSize) -> Self {
        self.inner.anchor = anchor;
        self.inner.layer_size = layer_size;
        self
    }

    fn fractional_scale(mut self, fractional_scale: Option<WpFractionalScaleV1>) -> Self {
        self.inner.fractional_scale = fractional_scale;
        self
    }

    fn viewport(mut self, viewport: Option<WpViewport>) -> Self {
        Arc::get_mut(&mut self.inner.window)
            .expect("the window is only shared once the unit is built")
            .viewport = viewport;
        self
    }

    fn blur_option(mut self, blur_option: BlurOption) -> Self {
        self.inner.blur_option = blur_option;
        self
    }

    fn effect_surface(mut self, effect: Option<ExtBackgroundEffectSurfaceV1>) -> Self {
        self.inner.effect = effect;
        self
    }

    fn wl_output(mut self, wl_output: Option<WlOutput>) -> Self {
        self.inner.wl_outputs = wl_output.into_iter().collect();
        self
    }

    fn binding(mut self, binding: Option<T>) -> Self {
        self.inner.binding = binding;
        self
    }

    fn parent(mut self, parent: Option<id::Id>) -> Self {
        self.inner.parent = parent;
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
    /// Shared because the renderer holds this surface too. Wayland proxies are
    /// not refcounted, so destroying it here would leave the renderer a dead one.
    window: Arc<WindowWrapper>,
    wmcompositor: WlCompositor,
    size: (u32, u32),
    /// Only meaningful for LayerShell
    anchor: Anchor,
    /// Only meaningful for LayerShell
    layer_size: LayerSize,
    buffer: Option<WlBuffer>,
    shell: Shell,
    parent: Option<id::Id>,
    fractional_scale: Option<WpFractionalScaleV1>,
    wl_outputs: Vec<WlOutput>,
    /// A leave held back to keep a layer surface on an output. Applied as
    /// soon as an enter offers a real one, so a move cannot pin the old one.
    pending_leave: Option<WlOutput>,
    effect: Option<ExtBackgroundEffectSurfaceV1>,
    binding: Option<T>,
    /// True after the compositor sends the initial configure for this shell role.
    configured: bool,

    blur_option: BlurOption,

    /// Only meaningful for PopUp
    pending_reposition: Option<u32>,

    /// Last configure state. Only used for `XdgTopLevel` windows.
    toplevel_state: ToplevelState,

    scale: u32,
    request_flag: WindowStateUnitRequestFlag,
    present_available_state: PresentAvailableState,
    frame_callback: Option<WlCallback>,
}

/// wayland-rs sends nothing on drop.
/// A destructor request has to be issued, so every object the unit owns is released.
impl<T> Drop for WindowStateUnit<T> {
    fn drop(&mut self) {
        // wl_callback has no destructor request, so something like
        // this needs to be done
        if let Some(callback) = self.frame_callback.take()
            && let Some(backend) = callback.backend().upgrade()
        {
            let _ = backend.destroy_object(&callback.id());
        }
        self.shell.destroy();
        if let Some(buffer) = &self.buffer {
            buffer.destroy();
        }
        if let Some(scale) = &self.fractional_scale {
            scale.destroy();
        }
        if let Some(effect) = &self.effect {
            effect.destroy();
        }
    }
}

impl<T> WindowStateUnit<T> {
    fn is_lock(&self) -> bool {
        self.shell.is_lock()
    }
}

impl<T> WindowStateUnit<T> {
    /// get the WindowState id
    pub fn id(&self) -> id::Id {
        self.id
    }

    pub fn wl_shell_type(&self) -> WlShellType {
        match self.shell {
            Shell::PopUp(_) => WlShellType::PopUp,
            Shell::LayerShell(_) => WlShellType::LayerShell,
            Shell::XdgTopLevel(_) => WlShellType::XdgTopLevel,
            Shell::SessionLock(_) => WlShellType::SessionLock,
            Shell::InputPanel(_) => WlShellType::InputPanel,
        }
    }

    pub fn try_set_viewport_destination(&self, width: i32, height: i32) -> Option<()> {
        let viewport = self.window.viewport.as_ref()?;
        // (-1, -1) unsets the destination
        if (width, height) != (-1, -1) && (width <= 0 || height <= 0) {
            log::warn!(
                target: "exwlshellev",
                "ignoring viewport destination {width}x{height} for {:?}: wp_viewport requires \
                 positive dimensions, or (-1, -1) to unset",
                self.id
            );
            return None;
        }
        viewport.set_destination(width, height);
        Some(())
    }

    pub fn try_set_viewport_source(&self, x: f64, y: f64, width: f64, height: f64) -> Option<()> {
        let viewport = self.window.viewport.as_ref()?;
        viewport.set_source(x, y, width, height);
        Some(())
    }

    /// Share this window [`WindowWrapper`], keeping its surface alive for as
    /// long as the caller holds it.
    pub fn gen_wrapper(&self) -> Arc<WindowWrapper> {
        self.window.clone()
    }
}
impl<T> WindowStateUnit<T> {
    #[inline]
    pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandWindowHandle::new({
            let ptr = self.window.wl_surface.id().as_ptr();
            std::ptr::NonNull::new(ptr as *mut _).expect("wl_surface will never be null")
        })
        .into())
    }

    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::WaylandDisplayHandle::new({
            let ptr = self.window.display.id().as_ptr();
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

impl<T: 'static> WindowStateUnit<T> {
    pub fn set_blur_option(&mut self, blur_option: BlurOption) {
        self.blur_option = blur_option;
        if let Some(effect) = &self.effect {
            match &self.blur_option {
                BlurOption::None => {
                    effect.set_blur_region(None);
                }
                BlurOption::FullRegion => {
                    let (width, height) = self.size;
                    let region = self.wmcompositor.create_region(&self.qh, ());
                    region.add(0, 0, width as i32, height as i32);
                    effect.set_blur_region(Some(&region));
                    region.destroy();
                }
                BlurOption::Region(regions) => {
                    let region = self.wmcompositor.create_region(&self.qh, ());
                    for BlurRegion {
                        x,
                        y,
                        width,
                        height,
                    } in regions
                    {
                        region.add(*x, *y, *width, *height);
                    }
                    effect.set_blur_region(Some(&region));
                    region.destroy();
                }
            }
            self.window.wl_surface.commit();
        }
    }
}

impl<T> WindowStateUnit<T> {
    /// get the wl surface from WindowState
    pub fn get_wlsurface(&self) -> &WlSurface {
        &self.window.wl_surface
    }

    /// output this surface is displayed on from compositor through `wl_surface::enter`
    ///
    /// A surface may overlap several outputs, this is the first one it entered.
    pub fn get_wloutput(&self) -> Option<&WlOutput> {
        self.wl_outputs.first()
    }

    /// every output this surface overlaps
    pub fn get_wloutputs(&self) -> &[WlOutput] {
        &self.wl_outputs
    }

    /// last requested anchor for surface
    pub fn get_anchor(&self) -> Anchor {
        self.anchor
    }

    /// actual layer surface anchor
    pub fn get_effective_anchor(&self) -> Anchor {
        if !matches!(self.shell, Shell::LayerShell(_)) {
            return Anchor::empty();
        }
        self.anchor | self.layer_size.missing_edges(self.anchor)
    }

    /// last requested size for surface
    pub fn get_layer_size(&self) -> LayerSize {
        self.layer_size
    }

    /// commit anchor and size, while adding required edges for size
    fn commit_layout(&mut self, anchor: Anchor, size: LayerSize) {
        let Shell::LayerShell(layer_shell) = &self.shell else {
            return;
        };
        let (width, height) = size.to_set();
        layer_shell.set_anchor(size.resolve_anchor(anchor));
        layer_shell.set_size(width, height);
        self.anchor = anchor;
        self.layer_size = size;
        self.window.wl_surface.commit();
    }

    /// set the anchor of the current unit. please take the simple.rs as reference
    pub fn set_anchor(&mut self, anchor: Anchor) {
        self.commit_layout(anchor, self.layer_size);
    }

    /// you can reset the margin which bind to the surface
    pub fn set_margin(&self, (top, right, bottom, left): (i32, i32, i32, i32)) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_margin(top, right, bottom, left);
            self.window.wl_surface.commit();
        }
    }

    /// set the layer
    pub fn set_layer(&self, layer: Layer) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_layer(layer);
            self.window.wl_surface.commit();
        }
    }

    /// set the anchor and set the size together
    /// When you want to change the layout from LEFT|RIGHT|BOTTOM to TOP|LEFT|BOTTOM, use it.
    pub fn set_layout(&mut self, anchor: Anchor, size: LayerSize) {
        self.commit_layout(anchor, size);
    }

    /// set the layer size of current unit
    pub fn set_size(&mut self, size: LayerSize) {
        self.commit_layout(self.anchor, size);
    }

    /// set current exclusive_zone
    pub fn set_exclusive_zone(&self, zone: i32) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            warn_if_exclusive_zone_ignored(zone, self.get_effective_anchor());
            layer_shell.set_exclusive_zone(zone);
            self.window.wl_surface.commit();
        }
    }

    /// set keyboard interactivity
    pub fn set_keyboard_interactivity(&self, keyboard_interactivity: KeyboardInteractivity) {
        if let Shell::LayerShell(layer_shell) = &self.shell {
            layer_shell.set_keyboard_interactivity(keyboard_interactivity);
            self.window.wl_surface.commit();
        }
    }

    /// State reported by the last `xdg_toplevel::configure` event.
    pub fn toplevel_state(&self) -> ToplevelState {
        self.toplevel_state
    }

    pub fn is_maximized(&self) -> bool {
        self.toplevel_state.maximized
    }

    pub fn is_fullscreen(&self) -> bool {
        self.toplevel_state.fullscreen
    }

    /// Request compositor to maximize or unmaximize this toplevel.
    pub fn set_maximized(&self, maximized: bool) {
        let Some(toplevel) = self.shell.top_level() else {
            return;
        };
        if maximized {
            toplevel.set_maximized();
        } else {
            toplevel.unset_maximized();
        }
    }

    /// Request compositor to minimize this xdg toplevel.
    pub fn set_minimized(&self) {
        let Some(toplevel) = self.shell.top_level() else {
            return;
        };
        toplevel.set_minimized();
    }

    /// Request compositor to start an interactive move of this xdg toplevel.
    pub fn start_move(&self, seat: &WlSeat, serial: u32) {
        let Some(toplevel) = self.shell.top_level() else {
            return;
        };
        toplevel._move(seat, serial);
    }

    /// Request compositor to show the window menu for this xdg toplevel.
    pub fn show_window_menu(&self, seat: &WlSeat, serial: u32, x: i32, y: i32) {
        let Some(toplevel) = self.shell.top_level() else {
            return;
        };
        toplevel.show_window_menu(seat, serial, x, y);
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
        self.window.wl_surface.attach(self.buffer.as_ref(), 0, 0);
        self.window
            .wl_surface
            .damage(0, 0, self.size.0 as i32, self.size.1 as i32);
        self.window.wl_surface.commit();
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
    /// Returns the duration until this unit needs its next refresh,
    /// or `None` if no refresh is pending, the surface has not received
    /// its initial configure, or the present slot is unavailable (waiting
    /// for a compositor frame callback).
    fn refresh_timeout(&self) -> Option<Duration> {
        if !self.configured {
            return None;
        }

        match self.request_flag.refresh {
            RefreshRequest::NextFrame => {
                if self.present_available_state == PresentAvailableState::Available {
                    Some(Duration::ZERO)
                } else {
                    None
                }
            }
            RefreshRequest::At(instant) => {
                let timeout = instant.saturating_duration_since(Instant::now());
                if timeout.is_zero()
                    && self.present_available_state != PresentAvailableState::Available
                {
                    None
                } else {
                    Some(timeout)
                }
            }
            RefreshRequest::Wait => None,
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
        if !self.configured || !self.should_refresh() {
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
                self.frame_callback = Some(
                    self.window
                        .wl_surface
                        .frame(&self.qh, (self.id, PresentAvailableState::Available)),
                );
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

#[derive(Debug)]
struct KeyboardTokenState {
    delay: Duration,
    key: u32,
    surface_id: Option<id::Id>,
    pressed_state: ElementState,
    object_id: ObjectId,
}

#[derive(Debug)]
pub struct VirtualKeyRelease {
    pub delay: Duration,
    pub time: u32,
    pub key: u32,
}

/// a wrapper for implement Debug, so we don't need to implement Debug for WindowState
pub enum WithConnection {
    Fun(Box<dyn FnOnce() -> Result<Connection, ConnectError>>),
    Value(Connection),
}

impl WithConnection {
    fn get_connection(self) -> Result<Connection, ConnectError> {
        match self {
            Self::Fun(f) => f(),
            Self::Value(value) => Ok(value),
        }
    }
}

impl Debug for WithConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("WithConnection Callback")
    }
}

impl<F> From<F> for WithConnection
where
    F: 'static + FnOnce() -> Result<Connection, ConnectError>,
{
    fn from(value: F) -> Self {
        Self::Fun(Box::new(value))
    }
}

impl From<Connection> for WithConnection {
    fn from(value: Connection) -> Self {
        Self::Value(value)
    }
}

/// main state, store the main information
#[derive(Debug)]
pub struct WindowState<T> {
    outputs: Vec<wl_output::WlOutput>,
    keyboard_focus: Option<WlSurface>,
    active_surfaces: HashMap<Option<i32>, (WlSurface, Option<id::Id>)>,
    units: Vec<WindowStateUnit<T>>,
    message: Vec<(Option<id::Id>, DispatchMessageInner)>,

    with_connection: Option<WithConnection>,
    connection: Option<Connection>,
    event_queue: Option<EventQueue<WindowState<T>>>,
    wl_compositor: Option<WlCompositor>,
    background_effect_manager: Option<ExtBackgroundEffectManagerV1>,
    shm: Option<WlShm>,
    cursor_manager: Option<WpCursorShapeManagerV1>,
    viewporter: Option<WpViewporter>,
    lock_manager: Option<ExtSessionLockManagerV1>,

    // The shells used to create surfaces
    wmbase: Option<XdgWmBase>,
    layer_shell: Option<ZwlrLayerShellV1>,
    input_panel: Option<ZwpInputPanelV1>,

    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    globals: Option<GlobalList>,

    // background
    background_surface: Option<WlSurface>,
    display: Option<WlDisplay>,

    registry_state: Option<RegistryState>,
    output_state: Option<OutputState>,
    // base managers
    seat_state: Option<SeatState>,
    seats: HashMap<ObjectId, SeatStorage>,
    seat_back: Option<WlSeat>,

    virtual_keyboard: Option<ZwpVirtualKeyboardV1>,

    // states
    default_namespace: String,
    keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity,
    anchor: Anchor,
    layer: Layer,
    size: LayerSize,
    exclusive_zone: Option<i32>,
    margin: Option<(i32, i32, i32, i32)>,
    blur_option: BlurOption,

    // settings
    use_display_handle: bool,
    repeat_delay: Option<KeyboardTokenState>,
    to_remove_tokens: Vec<RegistrationToken>,
    closed_ids: Vec<id::Id>,

    to_be_released_key: Option<VirtualKeyRelease>,

    last_unit_index: usize,
    last_wloutput: Option<WlOutput>,

    return_data: Vec<ReturnData<T>>,
    finger_locations: HashMap<i32, (f64, f64)>,
    enter_serial: Option<u32>,
    popup_grab_serial: Option<u32>,

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

impl<T: 'static> WindowState<T> {
    pub fn append_return_data(&mut self, data: ReturnData<T>) {
        self.return_data.push(data);
    }

    /// Read the latest button press or touch down serial without consuming it.
    pub fn popup_grab_serial(&self) -> Option<u32> {
        self.popup_grab_serial
    }

    /// Take the serial to use for the next popup grab, consuming it.
    pub fn take_popup_grab_serial(&mut self) -> Option<u32> {
        self.popup_grab_serial.take()
    }

    /// Compute the minimum dispatch timeout across all window units.
    /// Returns `None` when every unit is idle (`RefreshRequest::Wait`),
    /// meaning the event loop can sleep indefinitely until an external
    /// event (Wayland, channel, etc.) arrives.
    fn min_dispatch_timeout(&self) -> Option<Duration> {
        let mut min: Option<Duration> = None;
        for unit in &self.units {
            match unit.refresh_timeout() {
                Some(Duration::ZERO) => return Some(Duration::ZERO),
                Some(d) => min = Some(min.map_or(d, |m: Duration| m.min(d))),
                None => {}
            }
        }
        min
    }

    /// Whether [`Self::remove_shell`] would actually remove unit
    fn can_remove_shell(&self, id: id::Id) -> bool {
        self.units.iter().any(|unit| unit.id == id)
    }

    /// Drop a pending close request, so it is not retried on every iteration
    fn clear_close_request(&mut self, id: id::Id) {
        if let Some(unit) = self.units.iter_mut().find(|unit| unit.id == id) {
            unit.request_flag.close = false;
        }
    }

    /// remove a shell, destroy the surface
    fn remove_shell(&mut self, id: id::Id) -> Option<()> {
        let index = self.units.iter().position(|unit| unit.id == id)?;

        if self.keyboard_focus.as_ref() == Some(&self.units[index].window.wl_surface) {
            self.keyboard_focus = None;
        }
        self.units.remove(index);
        Some(())
    }

    fn collect_descendants_then_self(&self, id: id::Id, order: &mut Vec<id::Id>) {
        if order.contains(&id) {
            return;
        }
        let children: Vec<id::Id> = self
            .units
            .iter()
            .filter(|unit| unit.parent == Some(id))
            .map(|unit| unit.id)
            .collect();
        for child in children {
            self.collect_descendants_then_self(child, order);
        }
        order.push(id);
    }

    /// forget the remembered last output, next time it will get the new activated output to set the
    /// layershell
    pub fn forget_last_output(&mut self) {
        self.last_wloutput.take();
    }

    fn last_output(&mut self) -> Option<WlOutput> {
        if self.last_wloutput.is_none() {
            self.last_wloutput = self.outputs.get(self.last_unit_index).cloned();
        }

        self.last_wloutput
            .as_ref()
            .or_else(|| self.outputs.first())
            .cloned()
    }
}

/// Simple WindowState, without any data binding or info
pub type WindowStateSimple = WindowState<()>;

impl<T> WindowState<T> {
    pub fn display_wrapper(&self) -> DisplayWrapper {
        DisplayWrapper {
            display: self.display.clone().expect("You should it after build"),
        }
    }
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
        let surface = window_state_unit.window.wl_surface.clone();
        self.units.push(window_state_unit);
        // update newest window output for `OutputOption::LastOutput`
        self.update_active_output(&surface);
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

impl Drop for WindowWrapper {
    fn drop(&mut self) {
        // Built on the surface, so it goes first.
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
        self.wl_surface.destroy();
    }
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
    /// gen the wrapper to the main window
    /// used to get display and etc
    pub fn gen_mainwindow_wrapper(&self) -> Arc<WindowWrapper> {
        self.main_window().gen_wrapper()
    }
    pub fn with_blur_option(mut self, blur_option: BlurOption) -> Self {
        self.blur_option = blur_option;
        self
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

#[derive(Debug, Clone)]
pub struct DisplayWrapper {
    display: WlDisplay,
}

impl DisplayWrapper {
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

impl rwh_06::HasDisplayHandle for DisplayWrapper {
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
}

impl<T> Default for WindowState<T> {
    fn default() -> Self {
        Self {
            outputs: Vec::new(),
            keyboard_focus: None,
            active_surfaces: HashMap::new(),
            units: Vec::new(),
            message: Vec::new(),

            background_surface: None,
            display: None,

            with_connection: None,
            connection: None,
            event_queue: None,
            wl_compositor: None,
            shm: None,
            wmbase: None,
            background_effect_manager: None,
            cursor_manager: None,
            lock_manager: None,
            layer_shell: None,
            input_panel: None,
            viewporter: None,
            globals: None,
            fractional_scale_manager: None,
            virtual_keyboard: None,

            output_state: None,
            registry_state: None,

            seat_state: None,
            seats: HashMap::new(),
            seat_back: None,

            default_namespace: "osd".to_owned(),
            keyboard_interactivity: zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand,
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right | Anchor::Bottom,
            size: LayerSize::FILL,
            exclusive_zone: None,
            margin: None,
            blur_option: BlurOption::None,

            use_display_handle: false,
            repeat_delay: None,
            to_remove_tokens: Vec::new(),
            to_be_released_key: None,
            closed_ids: Vec::new(),

            last_wloutput: None,
            last_unit_index: 0,

            return_data: Vec::new(),
            finger_locations: HashMap::new(),
            enter_serial: None,
            popup_grab_serial: None,

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

    pub fn set_virtual_key_release(&mut self, key_info: VirtualKeyRelease) {
        self.to_be_released_key = Some(key_info);
    }

    /// use [id::Id] to get the mut [WindowStateUnit]
    pub fn get_mut_unit_with_id(&mut self, id: id::Id) -> Option<&mut WindowStateUnit<T>> {
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

    /// every output the compositor advertises with its info
    pub fn outputs(&self) -> Vec<(WlOutput, OutputInfo)> {
        let Some(state) = self.output_state.as_ref() else {
            return Vec::new();
        };
        state
            .outputs()
            .filter_map(|output| state.info(&output).map(|info| (output, info)))
            .collect()
    }

    /// the info of a given output, if it is still known.
    pub fn get_output_info_of(&self, output: &WlOutput) -> Option<OutputInfo> {
        self.output_state.as_ref()?.info(output)
    }

    /// where a new surface should go
    fn resolve_output(&mut self, option: OutputOption) -> Option<WlOutput>
    where
        T: 'static,
    {
        match option {
            OutputOption::Output(output) => Some(output),
            OutputOption::Active => None,
            OutputOption::LastOutput => self.last_output(),
            OutputOption::GlobalName(name) => self.output_by_global_name(name).or_else(|| {
                log::warn!(target: "exwlshellev", "no connected output with global name {name}, letting the compositor choose");
                None
            }),
            OutputOption::OutputName(name) => self.output_by_name(&name).or_else(|| {
                log::warn!(target: "exwlshellev", "no connected output named {name}, letting the compositor choose");
                None
            }),
        }
    }

    /// Find an output by its `wl_registry` global name.
    pub fn output_by_global_name(&self, name: u32) -> Option<WlOutput> {
        let state = self.output_state.as_ref()?;
        state
            .outputs()
            .find(|output| state.info(output).is_some_and(|info| info.id == name))
    }

    /// the output matching `name` (`HDMI-A-1` and such), if it is connected.
    pub fn output_by_name(&self, name: &str) -> Option<WlOutput> {
        let state = self.output_state.as_ref()?;
        state
            .outputs()
            .find(|output| state.info(output).and_then(|info| info.name).as_deref() == Some(name))
    }

    /// the output info the surface `id` is currently displayed on
    pub fn get_output_info(&self, id: id::Id) -> Option<OutputInfo> {
        let output = self.get_unit_with_id(id)?.get_wloutput().cloned()?;
        self.get_output_info_of(&output)
    }

    /// get the current keyboard focus window id
    pub fn keyboard_focus_id(&self) -> Option<id::Id> {
        self.units
            .iter()
            .find(|unit| Some(&unit.window.wl_surface) == self.keyboard_focus.as_ref())
            .map(|unit| unit.id())
    }

    /// parent popup/menu when no parent from caller
    pub fn popup_parent_id(&self) -> Option<id::Id> {
        self.keyboard_focus_id()
            .or_else(|| self.pointer_surface_id())
            .or_else(|| {
                self.active_surfaces
                    .values()
                    .filter_map(|(_, id)| *id)
                    .find(|id| self.get_unit_with_id(*id).is_some())
            })
            .or_else(|| self.units.last().map(|unit| unit.id()))
    }

    /// window id under the pointer
    pub fn pointer_surface_id(&self) -> Option<id::Id> {
        self.active_surfaces
            .get(&None)
            .and_then(|(_, id)| *id)
            .filter(|id| self.get_unit_with_id(*id).is_some())
    }

    fn get_id_from_surface(&self, surface: &WlSurface) -> Option<id::Id> {
        self.units
            .iter()
            .find(|unit| &unit.window.wl_surface == surface)
            .map(|unit| unit.id())
    }

    pub fn is_mouse_surface(&self, surface_id: id::Id) -> bool {
        self.active_surfaces
            .get(&None)
            .filter(|(_, id)| *id == Some(surface_id))
            .is_some()
    }

    /// update output window is on, for `OutputOption::LastOutput`
    fn update_active_output(&mut self, surface: &WlSurface) {
        let Some(unit) = self
            .units
            .iter()
            .find(|unit| &unit.window.wl_surface == surface)
        else {
            return;
        };
        if let Some(index) = self
            .outputs
            .iter()
            .position(|output| Some(output) == unit.get_wloutput())
        {
            self.last_unit_index = index;
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

    /// Request compositor to move window `id` with the pointer.
    pub fn request_move(&self, id: id::Id, serial: u32) {
        let Some(seat) = self.seat_back.as_ref() else {
            log::warn!(target: "exwlshellev", "no seat, cannot move {id:?}");
            return;
        };
        if let Some(unit) = self.get_unit_with_id(id) {
            unit.start_move(seat, serial);
        }
    }

    /// Request compositor to maximize or unmaximize window `id`.
    pub fn request_maximized(&self, id: id::Id, maximized: bool) {
        if let Some(unit) = self.get_unit_with_id(id) {
            unit.set_maximized(maximized);
        }
    }

    /// Request compositor to minimize window `id`.
    pub fn request_minimized(&self, id: id::Id) {
        if let Some(unit) = self.get_unit_with_id(id) {
            unit.set_minimized();
        }
    }

    /// Request compositor to show the menu for window `id` at the given surface coordinates.
    pub fn request_show_window_menu(&self, id: id::Id, serial: u32, (x, y): (i32, i32)) {
        let Some(seat) = self.seat_back.as_ref() else {
            log::warn!(target: "exwlshellev", "no seat, cannot show the window menu for {id:?}");
            return;
        };
        if let Some(unit) = self.get_unit_with_id(id) {
            unit.show_window_menu(seat, serial, x, y);
        }
    }

    /// State from the last `xdg_toplevel::configure` event for window `id`.
    pub fn toplevel_state(&self, id: id::Id) -> Option<ToplevelState> {
        self.get_unit_with_id(id)
            .map(WindowStateUnit::toplevel_state)
    }

    pub fn get_binding_mut(&mut self, id: id::Id) -> Option<&mut T> {
        self.get_mut_unit_with_id(id)
            .and_then(WindowStateUnit::get_binding_mut)
    }
}

impl<T: 'static> ProvidesRegistryState for WindowState<T> {
    fn registry(&mut self) -> &mut RegistryState {
        self.registry_state.as_mut().unwrap()
    }
    sctk::registry_handlers![SeatState, OutputState];
}

impl<T: 'static> OutputHandler for WindowState<T> {
    fn output_state(&mut self) -> &mut OutputState {
        self.output_state.as_mut().unwrap()
    }
    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.outputs.push(output.clone());
        if let Some(info) = self.get_output_info_of(&output) {
            self.message
                .push((None, DispatchMessageInner::OutputAdded(info)));
        }
        self.message
            .push((None, DispatchMessageInner::NewDisplay(output)));
    }
    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.get_output_info_of(&output) {
            self.message
                .push((None, DispatchMessageInner::OutputUpdated(info)));
        }
        let affected: Vec<id::Id> = self
            .units
            .iter()
            .filter(|unit| unit.wl_outputs.contains(&output))
            .map(|unit| unit.id)
            .collect();
        for id in affected {
            self.message.push((
                Some(id),
                DispatchMessageInner::OutputChanged(Some(output.clone())),
            ));
        }
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.get_output_info_of(&output) {
            self.message
                .push((None, DispatchMessageInner::OutputRemoved(info)));
        }
        if self
            .last_wloutput
            .as_ref()
            .is_some_and(|output| !output.is_alive())
        {
            self.last_wloutput.take();
        }
        self.outputs.retain(|o| o != &output);

        let removed_states: Vec<_> = self
            .units
            .extract_if(.., |unit| {
                !unit.window.wl_surface.is_alive() || unit.wl_outputs.as_slice() == [output.clone()]
            })
            .collect();
        if removed_states
            .iter()
            .any(|unit| Some(&unit.window.wl_surface) == self.keyboard_focus.as_ref())
        {
            self.keyboard_focus = None;
        }
        for unit in &mut self.units {
            let previous = unit.wl_outputs.first().cloned();
            unit.wl_outputs.retain(|o| o != &output);
            // same rule as the enter/leave path
            if unit.wl_outputs.first() != previous.as_ref() {
                let id = unit.id;
                let output = unit.wl_outputs.first().cloned();
                self.message
                    .push((Some(id), DispatchMessageInner::OutputChanged(output)));
            }
        }
        for deleled in removed_states {
            self.closed_ids.push(deleled.id);
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
                .for_each(|unit| {
                    unit.configured = true;
                    unit.request_refresh(RefreshRequest::NextFrame);
                });
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
                state.units[unit_index].configured = true;
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

fn toplevel_state_from_configure(states: &[u8]) -> ToplevelState {
    let mut toplevel_state = ToplevelState::default();
    for raw in states.as_chunks::<4>().0 {
        let raw = u32::from_ne_bytes([raw[0], raw[1], raw[2], raw[3]]);
        let Ok(state) = xdg_toplevel::State::try_from(raw) else {
            continue;
        };
        match state {
            xdg_toplevel::State::Maximized => toplevel_state.maximized = true,
            xdg_toplevel::State::Fullscreen => toplevel_state.fullscreen = true,
            xdg_toplevel::State::Activated => toplevel_state.activated = true,
            xdg_toplevel::State::TiledLeft
            | xdg_toplevel::State::TiledRight
            | xdg_toplevel::State::TiledTop
            | xdg_toplevel::State::TiledBottom => toplevel_state.tiled = true,
            _ => {}
        }
    }
    toplevel_state
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
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                let Some(unit_index) = unit_index else {
                    return;
                };
                if width != 0 && height != 0 {
                    state.units[unit_index].size = (width as u32, height as u32);
                }

                let toplevel_state = toplevel_state_from_configure(&states);
                if state.units[unit_index].toplevel_state != toplevel_state {
                    state.units[unit_index].toplevel_state = toplevel_state;
                    let id = state.units[unit_index].id;
                    state.message.push((
                        Some(id),
                        DispatchMessageInner::ToplevelStateChanged(toplevel_state),
                    ));
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

            let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
            else {
                return;
            };
            state.units[unit_index].size = (width, height);
            state.units[unit_index].configured = true;
            state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
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
        match event {
            xdg_popup::Event::Configure { width, height, .. } => {
                let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
                else {
                    return;
                };
                state.units[unit_index].size = (width as u32, height as u32);
                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            xdg_popup::Event::PopupDone => {
                if let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
                {
                    state.units[unit_index].request_close();
                }
            }
            xdg_popup::Event::Repositioned { token } => {
                if let Some(unit) = state.units.iter_mut().find(|unit| unit.shell == *surface)
                    && unit.pending_reposition == Some(token)
                {
                    unit.pending_reposition = None;
                }
            }
            _ => {}
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
                    scale_u32: scale,
                    scale_float: scale as f64 / 120.,
                },
            ));
        }
    }
}
impl<T> Dispatch<WlSurface, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        proxy: &WlSurface,
        event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let (wl_surface::Event::Enter { output } | wl_surface::Event::Leave { output }) = &event
        else {
            return;
        };
        if !state.outputs.contains(output) {
            log::trace!(target: "exwlshellev", "ignoring {event:?} for an output this loop did not bind");
            return;
        }

        let Some(unit) = state
            .units
            .iter_mut()
            .find(|unit| unit.window.wl_surface == *proxy)
        else {
            return;
        };
        let id = unit.id;
        let previous = unit.get_wloutput().cloned();
        match &event {
            wl_surface::Event::Enter { output } => {
                enter_output(&mut unit.wl_outputs, &mut unit.pending_leave, output);
            }
            wl_surface::Event::Leave { output } => {
                leave_output(
                    &mut unit.wl_outputs,
                    &mut unit.pending_leave,
                    matches!(unit.shell, Shell::LayerShell(..)),
                    output,
                );
            }
            _ => {}
        }
        if unit.get_wloutput() != previous.as_ref() {
            let output = unit.get_wloutput().cloned();
            state
                .message
                .push((Some(id), DispatchMessageInner::OutputChanged(output)));
        }
    }
}

fn enter_output<O: Clone + PartialEq>(
    outputs: &mut Vec<O>,
    held_leave: &mut Option<O>,
    output: &O,
) {
    if !outputs.contains(output) {
        outputs.push(output.clone());
    }
    if let Some(stale) = held_leave.take()
        && outputs.len() > 1
    {
        outputs.retain(|o| o != &stale);
    }
}

fn leave_output<O: Clone + PartialEq>(
    outputs: &mut Vec<O>,
    held_leave: &mut Option<O>,
    pinned: bool,
    output: &O,
) {
    if outputs.len() > 1 || !pinned {
        outputs.retain(|o| o != output);
    } else {
        *held_leave = Some(output.clone());
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
        if let WlCallbackEvent::Done { callback_data: _ } = event
            && let Some(unit) = state.get_mut_unit_with_id(data.0)
        {
            unit.frame_callback = None;
            unit.present_available_state = data.1;
        }
    }
}

impl<T> Dispatch<ExtSessionLockV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: <ExtSessionLockV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::Event;
        match event {
            Event::Locked => {
                state.message.push((None, DispatchMessageInner::Locked));
            }
            Event::Finished => {
                state
                    .message
                    .push((None, DispatchMessageInner::LockFinished));
            }
            _ => unreachable!(),
        }
    }
}

delegate_noop!(@<T> WindowState<T>: ignore WlCompositor); // WlCompositor is need to create a surface
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

delegate_noop!(@<T> WindowState<T>: ignore WpFractionalScaleManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore XdgPositioner);
delegate_noop!(@<T> WindowState<T>: ignore ExtSessionLockManagerV1); // buffer show the picture

sctk::delegate_registry!(@<T: 'static> WindowState<T>);
sctk::delegate_dispatch2!(@<T: 'static> WindowState<T>);
// we need to reply to the ping event otherwise
// top-level windows will be marked as unresponsive
// by the compositor.
impl<T: 'static> Dispatch<XdgWmBase, ()> for WindowState<T> {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

pub trait WindowTrait<T: 'static> {
    fn on_event(
        &mut self,
        event: ExWlShellEvent<T>,
        state: &mut WindowState<T>,
        id: Option<id::Id>,
    ) -> ReturnData<T>;

    fn request_buffer(
        &mut self,
        _state: &mut WindowState<T>,
        _file: &mut std::fs::File,
        _shm: &WlShm,
        _qh: &QueueHandle<WindowState<T>>,
        _width: u32,
        _height: u32,
    ) -> WlBuffer {
        unimplemented!("you need to implement one")
    }
}

#[derive(Debug, Clone, Copy)]
enum LockTeardown {
    Unlock,
    Exit,
}

#[derive(Debug, Clone)]
enum LockLifecycle {
    Unlocked,
    Pending {
        lock: ExtSessionLockV1,
        teardown: Option<LockTeardown>,
    },
    Locked {
        lock: ExtSessionLockV1,
    },
}
impl LockLifecycle {
    fn take(&mut self) -> Self {
        std::mem::replace(self, LockLifecycle::Unlocked)
    }
}

/// storage the context for the events
pub struct EventContext<T: 'static, W: WindowTrait<T>> {
    state: WindowState<T>,
    window_context: W,
    event_loop: Option<EventLoop<'static, Self>>,
    looph: LoopHandle<'static, Self>,
    lock: LockLifecycle,
    signal: LoopSignal,
    tokens: Vec<RegistrationToken>,
    cursor_update_context: CursorUpdateContext<T>,
}

impl<T: 'static, W: WindowTrait<T>> Drop for EventContext<T, W> {
    fn drop(&mut self) {
        if let Some(lock) = self.state.lock_manager.take() {
            lock.destroy();
        }
        if let Some(layer_shell) = self.state.layer_shell.take() {
            layer_shell.destroy();
        }
    }
}

impl<T: 'static, W: WindowTrait<T>> EventContext<T, W> {
    /// return the context, you can use it to change the state before enter [Self::run]
    pub fn window_context(&mut self) -> &mut W {
        &mut self.window_context
    }

    /// Registry other events, for example, the UserEvent or a11y
    pub fn register<Event, F>(&mut self, callback: F) -> Option<channel::Sender<Event>>
    where
        F: Fn(&mut W, Event) + 'static,
        Event: 'static,
    {
        let (sender, receiver) = channel::channel::<Event>();
        let token = self
            .looph
            .insert_source(receiver, move |event, _, state| {
                let channel::Event::Msg(event) = event else {
                    return;
                };
                callback(&mut state.window_context, event);
            })
            .ok()?;
        let _ = self.looph.disable(&token);
        self.tokens.push(token);
        Some(sender)
    }

    fn handle_event(&mut self, event: ExWlShellEvent<T>, unit_id: Option<id::Id>) {
        let return_data = self
            .window_context
            .on_event(event, &mut self.state, unit_id);
        if !matches!(return_data, ReturnData::None) {
            self.state.append_return_data(return_data);
        }
    }

    /// Run the program
    pub fn run(mut self) -> Result<(), ExShellEventError> {
        let connection = self.state.connection.take().unwrap();
        let mut event_queue_origin = self.state.event_queue.take().unwrap();
        let qh = event_queue_origin.handle();

        let wmcompositer = self.state.wl_compositor.take().unwrap();

        let shm = self.state.shm.take().unwrap();
        let fractional_scale_manager = self.state.fractional_scale_manager.take();
        let wmbase = self.state.wmbase.take().unwrap();
        let viewporter = self.state.viewporter.take();
        let zxdg_decoration_manager = self.state.xdg_decoration_manager.take();
        fn remove_lock_units<T>(window_state: &mut WindowState<T>) {
            for removed in window_state.units.extract_if(.., |unit| unit.is_lock()) {
                if window_state.keyboard_focus.as_ref() == Some(&removed.window.wl_surface) {
                    window_state.keyboard_focus = None;
                }
                window_state.closed_ids.push(removed.id);
            }
        }

        let process_window_state = |context: &mut Self| {
            let mut messages = Vec::new();
            std::mem::swap(&mut messages, &mut context.state.message);
            for msg in messages.iter() {
                match msg {
                    (_, DispatchMessageInner::NewDisplay(output_display)) => {
                        if let LockLifecycle::Pending { lock, .. }
                        | LockLifecycle::Locked { lock } = &context.lock
                        {
                            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more
                            // NOTE: it maybe a bug here, if we do not commit first, it won't enter the configure place, when a new display is in
                            // if it is the same with layershell and wmbase, we can send commit
                            // later, but we cannot
                            wl_surface.commit();
                            let session_lock_surface =
                                lock.get_lock_surface(&wl_surface, output_display, &qh, ());

                            // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                            // and if you need to reconfigure it, you need to commit the wl_surface again
                            // so because this is just an example, so we just commit it once
                            // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
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
                            context.state.push_window(
                                WindowStateUnitBuilder::new(
                                    id::Id::unique(),
                                    qh.clone(),
                                    connection.display(),
                                    wl_surface,
                                    wmcompositer.clone(),
                                    Shell::SessionLock(session_lock_surface),
                                )
                                .layout(context.state.anchor, context.state.size)
                                .viewport(viewport)
                                .fractional_scale(fractional_scale)
                                .wl_output(Some(output_display.clone()))
                                .build(),
                            );
                        }

                        if !context.state.is_allscreens() {
                            continue;
                        }
                        let wl_surface = wmcompositer.create_surface(&qh, ());
                        let layer_shell = context
                            .state
                            .layer_shell
                            .as_ref()
                            .expect("We need layershell here");
                        let layer = layer_shell.get_layer_surface(
                            &wl_surface,
                            Some(output_display),
                            context.state.layer,
                            context.state.default_namespace.clone(),
                            &qh,
                            (),
                        );
                        let wire_anchor = context.state.size.resolve_anchor(context.state.anchor);
                        layer.set_anchor(wire_anchor);
                        layer.set_keyboard_interactivity(context.state.keyboard_interactivity);
                        let (init_w, init_h) = context.state.size.to_set();
                        layer.set_size(init_w, init_h);

                        if let Some(zone) = context.state.exclusive_zone {
                            warn_if_exclusive_zone_ignored(zone, wire_anchor);
                            layer.set_exclusive_zone(zone);
                        }

                        if let Some(zone) = context.state.exclusive_zone {
                            layer.set_exclusive_zone(zone);
                        }

                        if let Some((top, right, bottom, left)) = context.state.margin {
                            layer.set_margin(top, right, bottom, left);
                        }

                        if context.state.events_transparent {
                            let region = wmcompositer.create_region(&qh, ());
                            wl_surface.set_input_region(Some(&region));
                            region.destroy();
                        }
                        wl_surface.commit();

                        let mut fractional_scale = None;
                        if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                            fractional_scale = Some(fractional_scale_manager.get_fractional_scale(
                                &wl_surface,
                                &qh,
                                (),
                            ));
                        }
                        let viewport = viewporter
                            .as_ref()
                            .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));

                        context.state.push_window(
                            WindowStateUnitBuilder::new(
                                id::Id::unique(),
                                qh.clone(),
                                connection.display(),
                                wl_surface,
                                wmcompositer.clone(),
                                Shell::LayerShell(layer),
                            )
                            .layout(context.state.anchor, context.state.size)
                            .viewport(viewport)
                            .fractional_scale(fractional_scale)
                            .wl_output(Some(output_display.clone()))
                            .build(),
                        );
                    }
                    (_, DispatchMessageInner::Locked) => match context.lock.take() {
                        LockLifecycle::Pending {
                            lock: l_lock,
                            teardown: Some(goal),
                        } => {
                            l_lock.unlock_and_destroy();
                            remove_lock_units(&mut context.state);
                            match goal {
                                LockTeardown::Exit => {
                                    let _ = connection.roundtrip();
                                    context.signal.stop();
                                    return true;
                                }
                                LockTeardown::Unlock => {
                                    let _ = connection.flush();
                                }
                            }
                        }
                        LockLifecycle::Pending {
                            lock: l_lock,
                            teardown: None,
                        } => {
                            context.lock = LockLifecycle::Locked { lock: l_lock };
                            context.handle_event(
                                ExWlShellEvent::RequestMessages(&DispatchMessage::Locked),
                                None,
                            );
                        }
                        other => {
                            log::warn!(
                                "Received `locked` without a pending lock request; ignoring"
                            );
                            context.lock = other;
                        }
                    },
                    (_, DispatchMessageInner::LockFinished) => match context.lock.take() {
                        LockLifecycle::Pending {
                            lock: l_lock,
                            teardown,
                        } => {
                            l_lock.destroy();
                            let _ = connection.flush();
                            remove_lock_units(&mut context.state);
                            context.handle_event(
                                ExWlShellEvent::RequestMessages(&DispatchMessage::LockDenied),
                                None,
                            );
                            if matches!(teardown, Some(LockTeardown::Exit)) {
                                context.signal.stop();
                                return true;
                            }
                        }
                        LockLifecycle::Locked { lock: l_lock } => {
                            l_lock.unlock_and_destroy();
                            let _ = connection.flush();
                            remove_lock_units(&mut context.state);
                            context.handle_event(
                                ExWlShellEvent::RequestMessages(&DispatchMessage::LockFinished),
                                None,
                            );
                        }
                        LockLifecycle::Unlocked => {
                            log::warn!("Received `finished` without an active lock; ignoring");
                        }
                    },
                    _ => {
                        let (index_message, msg) = msg;

                        let msg: DispatchMessage = msg.clone().into();
                        context.handle_event(ExWlShellEvent::RequestMessages(&msg), *index_message);
                    }
                }
            }

            context.handle_event(ExWlShellEvent::NormalDispatch, None);
            loop {
                let mut return_data = vec![];
                std::mem::swap(&mut context.state.return_data, &mut return_data);

                for data in return_data {
                    match data {
                        ReturnData::RequestExit => {
                            match context.lock.take() {
                                LockLifecycle::Locked { lock: l_lock } => {
                                    l_lock.unlock_and_destroy();
                                    let _ = connection.roundtrip();
                                    remove_lock_units(&mut context.state);
                                }
                                LockLifecycle::Pending { lock: l_lock, .. } => {
                                    context.lock = LockLifecycle::Pending {
                                        lock: l_lock,
                                        teardown: Some(LockTeardown::Exit),
                                    };
                                    continue;
                                }
                                LockLifecycle::Unlocked => {}
                            }
                            context.signal.stop();
                            return true;
                        }
                        ReturnData::RequestLock => {
                            if !matches!(context.lock, LockLifecycle::Unlocked) {
                                log::warn!(
                                    "Session lock already requested or active; ignoring duplicate lock request"
                                );
                                continue;
                            }
                            let Some(lock_manager) = context.state.lock_manager.as_ref() else {
                                log::error!("SessionLock is not supported");
                                context.handle_event(
                                    ExWlShellEvent::RequestMessages(&DispatchMessage::LockDenied),
                                    None,
                                );
                                continue;
                            };
                            let l_lock = lock_manager.lock(&qh, ());
                            let wl_outputs = context.state.outputs.clone();
                            for wl_output in wl_outputs.iter() {
                                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
                                // NOTE: it maybe a bug here, if we do not commit first, it won't enter the configure place, when a new display was in
                                // if it is the same with layershell and wmbase, we can send commit
                                // later, but we cannot
                                wl_surface.commit();
                                let session_lock_surface =
                                    l_lock.get_lock_surface(&wl_surface, wl_output, &qh, ());

                                // so during the init Configure of the shell, a buffer, atleast a buffer is needed.
                                // and if you need to reconfigure it, you need to commit the wl_surface again
                                // so because this is just an example, so we just commit it once
                                // like if you want to reset anchor or KeyboardInteractivity or resize, commit is needed
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
                                context.state.push_window(
                                    WindowStateUnitBuilder::new(
                                        id::Id::unique(),
                                        qh.clone(),
                                        connection.display(),
                                        wl_surface,
                                        wmcompositer.clone(),
                                        Shell::SessionLock(session_lock_surface),
                                    )
                                    .viewport(viewport)
                                    .fractional_scale(fractional_scale)
                                    .wl_output(Some(wl_output.clone()))
                                    .build(),
                                );
                            }
                            context.lock = LockLifecycle::Pending {
                                lock: l_lock,
                                teardown: None,
                            };
                        }

                        ReturnData::RequestUnLock => match context.lock.take() {
                            LockLifecycle::Locked { lock: l_lock } => {
                                l_lock.unlock_and_destroy();
                                let _ = connection.flush();
                                remove_lock_units(&mut context.state);
                            }
                            LockLifecycle::Pending {
                                lock: l_lock,
                                teardown,
                            } => {
                                context.lock = LockLifecycle::Pending {
                                    lock: l_lock,
                                    teardown: teardown.or(Some(LockTeardown::Unlock)),
                                };
                            }
                            LockLifecycle::Unlocked => {}
                        },
                        ReturnData::RequestSetCursor((cursor, pointer)) => {
                            let Some(serial) = context.state.enter_serial else {
                                continue;
                            };
                            set_cursor(&context.cursor_update_context, cursor, pointer, serial);
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
                                blur_option,
                            },
                            id,
                            info,
                        )) => {
                            let wire_anchor = size.resolve_anchor(anchor);
                            let output = context.state.resolve_output(output_type);

                            let wl_surface = wmcompositer.create_surface(&qh, ());

                            let layer_shell = context
                                .state
                                .layer_shell
                                .as_ref()
                                .expect("We need layershell here");
                            let layer = layer_shell.get_layer_surface(
                                &wl_surface,
                                output.as_ref(),
                                layer,
                                namespace
                                    .unwrap_or_else(|| context.state.default_namespace.clone()),
                                &qh,
                                (),
                            );
                            layer.set_anchor(wire_anchor);
                            layer.set_keyboard_interactivity(keyboard_interactivity);
                            let (init_w, init_h) = size.to_set();
                            layer.set_size(init_w, init_h);

                            if let Some(zone) = exclusive_zone {
                                warn_if_exclusive_zone_ignored(zone, wire_anchor);
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

                            let mut effect = None;
                            if let Some(effect_manger) = &context.state.background_effect_manager {
                                effect =
                                    Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
                            }
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

                            context.state.push_window(
                                WindowStateUnitBuilder::new(
                                    id,
                                    qh.clone(),
                                    connection.display(),
                                    wl_surface,
                                    wmcompositer.clone(),
                                    Shell::LayerShell(layer),
                                )
                                .layout(context.state.anchor, context.state.size)
                                .viewport(viewport)
                                .blur_option(blur_option)
                                .effect_surface(effect)
                                .fractional_scale(fractional_scale)
                                .wl_output(output)
                                .binding(info)
                                .build(),
                            );
                        }
                        ReturnData::NewPopUp((
                            NewPopUpSettings {
                                size,
                                id,
                                placement,
                                anchor,
                                gravity,
                                constraint_adjustment,
                                grab_serial,
                            },
                            targetid,
                            info,
                        )) => {
                            let Some(index) =
                                context.state.units.iter().position(|unit| unit.id == id)
                            else {
                                continue;
                            };
                            let wl_surface = wmcompositer.create_surface(&qh, ());
                            let positioner = build_positioner(
                                &wmbase,
                                &qh,
                                size,
                                placement,
                                anchor,
                                gravity,
                                constraint_adjustment,
                            );
                            let wl_xdg_surface = wmbase.get_xdg_surface(&wl_surface, &qh, ());

                            let popup = match &context.state.units[index].shell {
                                Shell::LayerShell(shell) => {
                                    let popup =
                                        wl_xdg_surface.get_popup(None, &positioner, &qh, ());
                                    shell.get_popup(&popup);
                                    popup
                                }
                                Shell::PopUp((_, parent_xdg_surface)) => wl_xdg_surface.get_popup(
                                    Some(parent_xdg_surface),
                                    &positioner,
                                    &qh,
                                    (),
                                ),
                                Shell::XdgTopLevel((_, parent_xdg_surface, _)) => wl_xdg_surface
                                    .get_popup(Some(parent_xdg_surface), &positioner, &qh, ()),
                                _ => {
                                    log::warn!(
                                        target: "exwlshellev",
                                        "cannot create popup: parent {:?} must be a layer surface, an xdg_toplevel or a popup",
                                        id
                                    );
                                    positioner.destroy();
                                    wl_xdg_surface.destroy();
                                    wl_surface.destroy();
                                    continue;
                                }
                            };
                            positioner.destroy();

                            match (context.state.seat_back.as_ref(), grab_serial) {
                                (Some(seat), Some(serial)) => popup.grab(seat, serial),
                                (None, Some(_)) => log::warn!(
                                    target: "exwlshellev",
                                    "popup {targetid:?} wants a grab but no seat is available; it will not dismiss on click-outside"
                                ),
                                (_, None) => {}
                            }

                            let mut fractional_scale = None;
                            if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                                fractional_scale =
                                    Some(fractional_scale_manager.get_fractional_scale(
                                        &wl_surface,
                                        &qh,
                                        (),
                                    ));
                            }
                            wl_surface.commit();

                            let viewport = viewporter
                                .as_ref()
                                .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                            context.state.push_window(
                                WindowStateUnitBuilder::new(
                                    targetid,
                                    qh.clone(),
                                    connection.display(),
                                    wl_surface,
                                    wmcompositer.clone(),
                                    Shell::PopUp((popup, wl_xdg_surface)),
                                )
                                .parent(Some(id))
                                .size(size.to_set())
                                .viewport(viewport)
                                .fractional_scale(fractional_scale)
                                .binding(info)
                                .build(),
                            );
                        }
                        ReturnData::PopUpReposition((
                            PopUpRepositionSettings {
                                size,
                                placement,
                                anchor,
                                gravity,
                                constraint_adjustment,
                            },
                            id,
                        )) => {
                            let Some(unit) =
                                context.state.units.iter_mut().find(|unit| unit.id == id)
                            else {
                                continue;
                            };
                            let Shell::PopUp((popup, _)) = &unit.shell else {
                                log::warn!(
                                    target: "exwlshellev",
                                    "reposition target {id:?} is not a popup; only popups can be repositioned"
                                );
                                continue;
                            };
                            if popup.version() < 3 {
                                log::warn!(
                                    target: "exwlshellev",
                                    "compositor offers xdg_popup v{}, reposition needs v3; leaving popup {id:?} as it is",
                                    popup.version()
                                );
                                continue;
                            }
                            let positioner = build_positioner(
                                &wmbase,
                                &qh,
                                size,
                                placement,
                                anchor,
                                gravity,
                                constraint_adjustment,
                            );
                            let token = unit.pending_reposition.unwrap_or(0).wrapping_add(1);
                            popup.reposition(&positioner, token);
                            positioner.destroy();
                            unit.pending_reposition = Some(token);
                        }
                        ReturnData::NewXdgBase((
                            NewXdgWindowSettings {
                                title,
                                size,
                                client_side_decorations,
                            },
                            id,
                            info,
                        )) => {
                            let wl_surface = wmcompositer.create_surface(&qh, ());
                            let wl_xdg_surface = wmbase.get_xdg_surface(&wl_surface, &qh, ());
                            let toplevel = wl_xdg_surface.get_toplevel(&qh, ());

                            toplevel.set_title(title.unwrap_or("".to_owned()));

                            let decoration = if let Some(decoration_manager) =
                                &zxdg_decoration_manager
                            {
                                let decoration =
                                    decoration_manager.get_toplevel_decoration(&toplevel, &qh, ());
                                use zxdg_toplevel_decoration_v1::Mode;
                                decoration.set_mode(if client_side_decorations {
                                    Mode::ClientSide
                                } else {
                                    Mode::ServerSide
                                });
                                Some(decoration)
                            } else {
                                None
                            };
                            let mut fractional_scale = None;
                            if let Some(ref fractional_scale_manager) = fractional_scale_manager {
                                fractional_scale =
                                    Some(fractional_scale_manager.get_fractional_scale(
                                        &wl_surface,
                                        &qh,
                                        (),
                                    ));
                            }
                            wl_surface.commit();

                            let viewport = viewporter
                                .as_ref()
                                .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));
                            context.state.push_window(
                                WindowStateUnitBuilder::new(
                                    id,
                                    qh.clone(),
                                    connection.display(),
                                    wl_surface,
                                    wmcompositer.clone(),
                                    Shell::XdgTopLevel((toplevel, wl_xdg_surface, decoration)),
                                )
                                .size(size.unwrap_or(PixelSize::px(300, 300)).to_set())
                                .viewport(viewport)
                                .fractional_scale(fractional_scale)
                                .binding(info)
                                .build(),
                            );
                        }

                        ReturnData::NewInputPanel((
                            NewInputPanelSettings {
                                size,
                                keyboard,
                                output_option: output_type,
                            },
                            id,
                            info,
                        )) => {
                            let output = context.state.resolve_output(output_type);

                            let Some(output) = output else {
                                log::warn!("no WlOutput, skip creating input panel");
                                continue;
                            };

                            let wl_surface = wmcompositer.create_surface(&qh, ());
                            let input_panel = context
                                .state
                                .input_panel
                                .as_ref()
                                .expect("This request needs input_panel support");
                            let input_panel_surface =
                                input_panel.get_input_panel_surface(&wl_surface, &qh, ());
                            if keyboard {
                                input_panel_surface.set_toplevel(
                                    &output,
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
                            context.state.push_window(
                                WindowStateUnitBuilder::new(
                                    id,
                                    qh.clone(),
                                    connection.display(),
                                    wl_surface,
                                    wmcompositer.clone(),
                                    Shell::InputPanel(input_panel_surface),
                                )
                                .size(size.to_set())
                                .viewport(viewport)
                                .fractional_scale(fractional_scale)
                                .binding(info)
                                .build(),
                            );
                        }
                        _ => {}
                    }
                }
                // added guard to match `sessionlockev`.
                context
                    .state
                    .return_data
                    .retain(|data| !matches!(data, ReturnData::None));
                if context.state.return_data.is_empty() {
                    break;
                }
            }

            let requested: Vec<id::Id> = context
                .state
                .units
                .iter()
                .filter(|unit| unit.request_flag.close)
                .map(WindowStateUnit::id)
                .collect();
            let mut close_roots: Vec<id::Id> = Vec::new();
            for id in requested {
                if context.state.can_remove_shell(id) {
                    close_roots.push(id);
                } else {
                    context.state.clear_close_request(id);
                }
            }
            let mut to_be_closed_ids: Vec<id::Id> = Vec::new();
            for root in close_roots {
                context
                    .state
                    .collect_descendants_then_self(root, &mut to_be_closed_ids);
            }
            for id in to_be_closed_ids {
                context.handle_event(
                    ExWlShellEvent::RequestMessages(&DispatchMessage::Closed),
                    Some(id),
                );
                context.state.remove_shell(id);
            }

            let closed_ids = context.state.closed_ids.clone();
            for id in closed_ids {
                context.handle_event(
                    ExWlShellEvent::RequestMessages(&DispatchMessage::Closed),
                    Some(id),
                );
            }
            context.state.closed_ids.clear();
            if context.state.units.is_empty()
                && !context.state.is_allscreens()
                && !context.state.is_background()
            {
                context.signal.stop();
                return true;
            }

            for idx in 0..context.state.units.len() {
                let unit = &mut context.state.units[idx];
                let (width, height) = unit.size;
                if width == 0 || height == 0 {
                    continue;
                }
                if unit.take_present_slot() {
                    let unit_id = unit.id;
                    let scale_float = unit.scale_float();
                    let wl_surface = unit.window.wl_surface.clone();
                    if unit.buffer.is_none() && !context.state.use_display_handle {
                        let Ok(mut file) = tempfile::tempfile() else {
                            log::error!("Cannot create new file from tempfile");
                            // note: could lead to infinite loop or spam log
                            // if the error is persistent.
                            return false;
                        };
                        let buffer = context.window_context.request_buffer(
                            &mut context.state,
                            &mut file,
                            &shm,
                            &qh,
                            width,
                            height,
                        );
                        wl_surface.attach(Some(&buffer), 0, 0);
                        wl_surface.commit();
                        context.state.units[idx].buffer = Some(buffer);
                    }
                    if let Some(effect) = &context.state.units[idx].effect {
                        match &context.state.units[idx].blur_option {
                            BlurOption::None => {}
                            BlurOption::FullRegion => {
                                let region = wmcompositer.create_region(&qh, ());
                                region.add(0, 0, width as i32, height as i32);
                                effect.set_blur_region(Some(&region));
                                region.destroy();
                            }
                            BlurOption::Region(regions) => {
                                let region = wmcompositer.create_region(&qh, ());
                                for BlurRegion {
                                    x,
                                    y,
                                    width,
                                    height,
                                } in regions
                                {
                                    region.add(*x, *y, *width, *height);
                                }
                                effect.set_blur_region(Some(&region));
                                region.destroy();
                            }
                        }
                        context.state.units[idx].window.wl_surface.commit();
                    }
                    context.handle_event(
                        ExWlShellEvent::RequestMessages(&DispatchMessage::RequestRefresh {
                            width,
                            height,
                            scale_float,
                        }),
                        Some(unit_id),
                    );
                    context.state.units[idx].reset_present_slot();
                }
            }

            false
        };

        let mut event_loop = self.event_loop.take().unwrap();
        // Dynamic dispatch timeout: compute the sleep duration from each
        // unit's RefreshRequest rather than using a fixed interval.
        // Based on the approach used by winit's Wayland event loop:
        // https://github.com/rust-windowing/winit/blob/master/winit-wayland/src/event_loop/mod.rs#L242-L312
        // Use zero-timeout on first dispatch if we don't have any windows
        // added in order to avoid getting stuck. For windowed startup, use
        // normal timeout to preserve standard lifecycle.
        let mut force_first_tick = self.state.units.is_empty();
        loop {
            let timeout = if force_first_tick {
                Some(Duration::ZERO)
            } else {
                self.state.min_dispatch_timeout()
            };
            event_loop.dispatch(timeout, &mut self)?;
            force_first_tick = false;

            event_queue_origin.dispatch_pending(&mut self.state)?;
            if process_window_state(&mut self) {
                break;
            }
            for token in self.state.to_remove_tokens.iter() {
                self.looph.remove(*token);
            }
            self.state.to_remove_tokens.clear();
            if let Some(VirtualKeyRelease { delay, time, key }) =
                self.state.to_be_released_key.take()
            {
                self.looph
                    .insert_source(Timer::from_duration(delay), move |_, _, r_window_state| {
                        let state = &mut r_window_state.state;
                        let ky = state.get_virtual_keyboard().unwrap();

                        ky.key(time, key, KeyState::Released.into());
                        TimeoutAction::Drop
                    })
                    .ok();
            }

            if let Some(KeyboardTokenState {
                key,
                delay,
                surface_id,
                pressed_state,
                object_id,
            }) = self.state.repeat_delay.take()
            {
                let timer = Timer::from_duration(delay);
                if let Some(keyboard_state) = self.state.get_keyboard_state_by_id(object_id.clone())
                {
                    keyboard_state.repeat_token = self
                        .looph
                        .insert_source(timer, move |_, _, r_window_state| {
                            let state = &mut r_window_state.state;
                            let keyboard_state = match state
                                .seats
                                .values_mut()
                                .find(|seat| {
                                    seat.keyboard_state
                                        .as_ref()
                                        .is_some_and(|state| state.keyboard.id() == object_id)
                                })
                                .map(|storage| storage.keyboard_state.as_mut().unwrap())
                            {
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
                            if let Some(mut key_context) = keyboard_state.xkb_context.key_context()
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
                            r_window_state.handle_event(ExWlShellEvent::NormalDispatch, None);
                            match repeat_info {
                                RepeatInfo::Repeat { gap, .. } => TimeoutAction::ToDuration(gap),
                                RepeatInfo::Disable => TimeoutAction::Drop,
                            }
                        })
                        .ok();
                }
            }

            // Flush after all event handlers have run so outgoing requests
            // (e.g. wl_surface.commit from process_window_state) reach the
            // compositor before the next dispatch() potentially sleeps.
            let _ = connection.flush();

            // NOTE: we need to start the receiver only after the dispatch is run at least once a
            // time
            for token in self.tokens.drain(..) {
                let _ = self.looph.enable(&token);
            }
        }
        Ok(())
    }
}

delegate_noop!(@<T> WindowState<T>: ignore ZwpTextInputManagerV3);
delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelSurfaceV1);
delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelV1);

delegate_noop!(@<T> WindowState<T>: ignore ZxdgDecorationManagerV1);
delegate_noop!(@<T> WindowState<T>: ignore ZxdgToplevelDecorationV1);

impl<T: 'static> WindowState<T> {
    pub fn build<Window>(
        mut self,
        mut window: Window,
    ) -> Result<EventContext<T, Window>, ExShellEventError>
    where
        Window: WindowTrait<T> + 'static,
    {
        self.build_inner()?;
        let globals = self.globals.take().unwrap();
        let event_queue_origin = self.event_queue.as_ref().unwrap();
        let qh = event_queue_origin.handle();

        let connection = self.connection.clone().unwrap();

        let shm = self.shm.clone().unwrap();

        let wmcompositer = self.wl_compositor.clone().unwrap();

        let mut init_event = None;

        let cursor_manager: Option<WpCursorShapeManagerV1> = self.cursor_manager.clone();

        let cursor_update_context = CursorUpdateContext {
            cursor_manager,
            qh: qh.clone(),
            connection: connection.clone(),
            shm: shm.clone(),
            cursor_surface: wmcompositer.create_surface(&qh, ()),
        };

        while !matches!(init_event, Some(ReturnData::None)) {
            match init_event {
                None => {
                    init_event =
                        Some(window.on_event(ExWlShellEvent::InitRequest, &mut self, None));
                }
                Some(ReturnData::RequestBind) => {
                    init_event = Some(window.on_event(
                        ExWlShellEvent::BindProvide(&globals, &qh),
                        &mut self,
                        None,
                    ));
                }
                Some(ReturnData::RequestCompositor) => {
                    init_event = Some(window.on_event(
                        ExWlShellEvent::CompositorProvide(&wmcompositer, &qh),
                        &mut self,
                        None,
                    ));
                }
                _ => panic!("Not provide server here"),
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
            state: self,
            window_context: window,
            looph: event_loop.handle(),
            event_loop: Some(event_loop),
            lock: LockLifecycle::Unlocked,
            signal,
            tokens: vec![],
            cursor_update_context,
        })
    }

    /// build a new WindowState
    fn build_inner(&mut self) -> Result<(), ExShellEventError> {
        let connection = if let Some(with_connection) = self.with_connection.take() {
            with_connection.get_connection()?
        } else {
            Connection::connect_to_env()?
        };
        let (globals, mut event_queue) = registry_queue_init::<Self>(&connection)?;
        self.display = Some(connection.display());

        let qh = event_queue.handle();

        self.registry_state = Some(RegistryState::new(&globals));
        self.output_state = Some(OutputState::new(&globals, &qh));
        let seat_state = SeatState::new(&globals, &qh);
        for seat in seat_state.seats() {
            self.seats.insert(seat.id(), SeatStorage::new());
        }
        self.seat_state = Some(seat_state);
        let wmcompositer = globals.bind::<WlCompositor, _, _>(&qh, 1..=5, ())?;
        self.background_effect_manager = globals
            .bind::<ExtBackgroundEffectManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        self.shm = Some(shm);
        self.seat_back = Some(globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?);

        let wmbase = globals.bind::<XdgWmBase, _, _>(&qh, 2..=6, ())?;
        self.wmbase = Some(wmbase);

        let cursor_manager = globals
            .bind::<WpCursorShapeManagerV1, _, _>(&qh, 1..=2, ())
            .ok();
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();

        // register this

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

        let lock_manager = globals
            .bind::<ExtSessionLockManagerV1, _, _>(&qh, 1..=1, ())
            .ok();
        let layer_shell = globals.bind::<ZwlrLayerShellV1, _, _>(&qh, 3..=4, ()).ok();
        let input_panel = globals.bind::<ZwpInputPanelV1, _, _>(&qh, 1..=1, ()).ok();

        self.text_input_manager = text_input_manager;
        event_queue.blocking_dispatch(self)?; // then make a dispatch

        // OutputState bound its own xdg_outputs before the dispatch above, so output info is
        // populated by now, a second roundtrip is not needed
        // so it is the same way, to get surface detach to protocol, first get the shell, like
        // wmbase or layer_shell or session-shell, then get `surface` from the wl_surface you
        // get before, and set it
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
            let binded_output = match self.start_mode.clone() {
                StartMode::TargetScreen(name) => self.output_by_name(&name),
                StartMode::TargetOutput(output) => Some(output),
                _ => None,
            };

            let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,
            let layer_shell_ref = layer_shell.as_ref().expect("We need layershell here");
            let layer = layer_shell_ref.get_layer_surface(
                &wl_surface,
                binded_output.as_ref(),
                self.layer,
                self.default_namespace.clone(),
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
            let mut effect = None;
            if let Some(effect_manger) = &self.background_effect_manager {
                effect = Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
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
                    wmcompositer.clone(),
                    Shell::LayerShell(layer),
                )
                .blur_option(self.blur_option.clone())
                .layout(self.anchor, self.size)
                .effect_surface(effect)
                .viewport(viewport)
                .fractional_scale(fractional_scale)
                .wl_output(binded_output.clone())
                .build(),
            );
        } else {
            let displays = self.outputs.clone();

            let layer_shell_ref = layer_shell.as_ref().expect("We need layershell here");
            for output_display in displays.iter() {
                let wl_surface = wmcompositer.create_surface(&qh, ()); // and create a surface. if two or more,

                let layer = layer_shell_ref.get_layer_surface(
                    &wl_surface,
                    Some(output_display),
                    self.layer,
                    self.default_namespace.clone(),
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
                let mut effect = None;
                if let Some(effect_manger) = &self.background_effect_manager {
                    effect = Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
                }
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
                        wmcompositer.clone(),
                        Shell::LayerShell(layer),
                    )
                    .layout(self.anchor, self.size)
                    .viewport(viewport)
                    .blur_option(self.blur_option.clone())
                    .effect_surface(effect)
                    .fractional_scale(fractional_scale)
                    .wl_output(Some(output_display.clone()))
                    .build(),
                );
            }
            self.message
                .retain(|(_, message)| !matches!(message, DispatchMessageInner::NewDisplay(_)));
        }
        self.init_finished = true;
        self.viewporter = viewporter;
        self.event_queue = Some(event_queue);
        self.globals = Some(globals);
        self.wl_compositor = Some(wmcompositer);
        self.fractional_scale_manager = fractional_scale_manager;
        self.cursor_manager = cursor_manager;
        self.lock_manager = lock_manager;
        self.layer_shell = layer_shell;
        self.input_panel = input_panel;
        self.connection = Some(connection);

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
}

fn build_positioner<T: 'static>(
    wmbase: &XdgWmBase,
    qh: &QueueHandle<WindowState<T>>,
    size: PixelSize,
    placement: PopupPlacement,
    anchor: xdg_positioner::Anchor,
    gravity: xdg_positioner::Gravity,
    constraint_adjustment: xdg_positioner::ConstraintAdjustment,
) -> XdgPositioner {
    let positioner = wmbase.create_positioner(qh, ());
    let (width, height) = size.to_set_i32();
    positioner.set_size(width, height);
    match placement {
        PopupPlacement::Position((px, py)) => positioner.set_anchor_rect(px, py, 1, 1),
        PopupPlacement::Anchored {
            position: (arx, ary),
            size: rect,
        } => {
            let (arw, arh) = rect.to_set_i32();
            positioner.set_anchor_rect(arx, ary, arw, arh)
        }
    }
    positioner.set_anchor(anchor);
    positioner.set_gravity(gravity);
    positioner.set_constraint_adjustment(constraint_adjustment);
    if positioner.version() >= 3 {
        positioner.set_reactive();
    }
    positioner
}

fn get_cursor_buffer(
    name: &str,
    connection: &Connection,
    shm: &WlShm,
) -> Option<CursorImageBuffer> {
    let mut cursor_theme = CursorTheme::load(connection, shm.clone(), 23).ok()?;
    let cursor = cursor_theme.get_cursor(name)?;
    Some(cursor[0].clone())
}

struct CursorUpdateContext<T: 'static> {
    cursor_manager: Option<WpCursorShapeManagerV1>,
    qh: QueueHandle<WindowState<T>>,
    connection: Connection,
    shm: WlShm,
    cursor_surface: WlSurface,
}

fn set_cursor<T: 'static>(
    context: &CursorUpdateContext<T>,
    cursor: Cursor,
    pointer: WlPointer,
    serial: u32,
) {
    let theme_name = match cursor {
        Cursor::Shape(shape) => {
            let name = match shape {
                CursorShape::Default => "default",
                CursorShape::ContextMenu => "context-menu",
                CursorShape::Help => "help",
                CursorShape::Pointer => "pointer",
                CursorShape::Progress => "progress",
                CursorShape::Wait => "wait",
                CursorShape::Cell => "cell",
                CursorShape::Crosshair => "crosshair",
                CursorShape::Text => "text",
                CursorShape::VerticalText => "vertical-text",
                CursorShape::Alias => "alias",
                CursorShape::Copy => "copy",
                CursorShape::Move => "move",
                CursorShape::NoDrop => "no-drop",
                CursorShape::NotAllowed => "not-allowed",
                CursorShape::Grab => "grab",
                CursorShape::Grabbing => "grabbing",
                CursorShape::EResize => "e-resize",
                CursorShape::NResize => "n-resize",
                CursorShape::NeResize => "ne-resize",
                CursorShape::EwResize => "ew-resize",
                CursorShape::NwResize => "nw-resize",
                CursorShape::SResize => "s-resize",
                CursorShape::SeResize => "se-resize",
                CursorShape::SwResize => "sw-resize",
                CursorShape::WResize => "w-resize",
                CursorShape::NsResize => "ns-resize",
                CursorShape::NeswResize => "nesw-resize",
                CursorShape::NwseResize => "nwse-resize",
                CursorShape::ColResize => "col-resize",
                CursorShape::RowResize => "row-resize",
                CursorShape::AllScroll => "all-scroll",
                CursorShape::ZoomIn => "zoom-in",
                CursorShape::ZoomOut => "zoom-out",
                CursorShape::DndAsk => "dnd-ask",
                CursorShape::AllResize => "all-resize",
                _ => {
                    log::warn!("Unsupported cursor shape: {shape:?}");
                    return;
                }
            };
            let required_version = if matches!(shape, CursorShape::DndAsk | CursorShape::AllResize)
            {
                2
            } else {
                1
            };
            if let Some(manager) = &context.cursor_manager
                && manager.version() >= required_version
            {
                let device = manager.get_pointer(&pointer, &context.qh, ());
                device.set_shape(serial, shape);
                device.destroy();
                return;
            }
            Cow::Borrowed(name)
        }
        Cursor::ThemeName(name) => Cow::Owned(name),
    };
    let Some(cursor_buffer) = get_cursor_buffer(&theme_name, &context.connection, &context.shm)
    else {
        log::error!("Cannot find cursor {theme_name}");
        return;
    };
    let cursor_surface = &context.cursor_surface;
    cursor_surface.attach(Some(&cursor_buffer), 0, 0);
    cursor_surface.damage(0, 0, i32::MAX, i32::MAX);
    let (hotspot_x, hotspot_y) = cursor_buffer.hotspot();
    pointer.set_cursor(
        serial,
        Some(cursor_surface),
        hotspot_x as i32,
        hotspot_y as i32,
    );
    cursor_surface.commit();
}
