use sctk::output::OutputInfo;
use wayland_client::{
    QueueHandle, WEnum,
    globals::GlobalList,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::WlOutput,
        wl_pointer::{self, ButtonState, WlPointer},
    },
};

use crate::CursorShape;

use crate::xkb_keyboard::KeyEvent;

use crate::keyboard::ModifiersState;

use super::WindowState;

use crate::id::Id;

use crate::utils::*;

use std::fmt::Debug;

/// tell program what event happened during init
///
/// InitRequest will tell the program is inited, you can request to Bind other wayland-protocols
/// there, with return [InitRequest::RequestBind]
///
/// RequestBuffer request to get the wl-buffer, so you init a buffer_pool here. It return a
/// GlobalList and a QueueHandle. This will enough for bind a extra wayland-protocol, and also,
/// seat can be gotten directly from [WindowState]
///
/// RequestMessages store the DispatchMessage, you can know what happened during dispatch with this
/// event.
pub enum ExWlShellInitEvent<'a, T> {
    /// the first event when start a new gui, program. you can return [InitRequest::None] or
    /// [InitRequest::RequestBind], then it will continue to the next request.
    /// Here only the above two [InitRequest] are acceptable.
    Start,
    /// After you return [InitRequest::RequestBind] in the [LayerShellEvent::InitRequest] stage, next
    /// event is [LayerShellEvent::BindProvide], you can use the GlobalList and QueueHandle to create
    /// new wayland objects.
    BindProvide(&'a GlobalList, &'a QueueHandle<WindowState<T>>),
    /// After you return [InitRequest::RequestCompositor] in the init stage, next
    /// event is [LayerShellEvent::CompositorProvide], you can use the WlCompositor and QueueHandle to
    /// create new wayland objects.
    CompositorProvide(&'a WlCompositor, &'a QueueHandle<WindowState<T>>),
}

/// the return data
/// Note: when event is RequestBuffer, you must return WlBuffer
/// Note: when receive InitRequest, you can request to bind extra wayland-protocols. this time you
/// can bind virtual-keyboard. you can take startcolorkeyboard as reference, or the simple.rs. Also,
/// it should can bind with text-input, but I am not fully understand about this, maybe someone
/// familiar with it can do
///
/// When send RequestExit, it will tell the event to finish.
///
/// Use `RequestSetCursor` with [`Cursor::Shape`] for standard shapes or [`Cursor::ThemeName`] for
/// an exact cursor name from the theme.
///
/// None means nothing will happened, no request, and no return data
#[derive(Debug, PartialEq, Eq)]
pub enum InitRequest {
    RequestBind,
    RequestCompositor,
    None,
}

/// A standard cursor shape or a named cursor from the current theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    /// Use the [cursor-shape](https://wayland.app/protocols/cursor-shape-v1#wp_cursor_shape_device_v1:enum:shape) protocol, with the matching theme cursor as a fallback.
    Shape(CursorShape),
    /// Load a cursor by its exact Xcursor name, even when the cursor-shape protocol is available.
    ThemeName(String),
}

/// the return data
/// Note: when event is RequestBuffer, you must return WlBuffer
/// Note: when receive InitRequest, you can request to bind extra wayland-protocols. this time you
/// can bind virtual-keyboard. you can take startcolorkeyboard as reference, or the simple.rs. Also,
/// it should can bind with text-input, but I am not fully understand about this, maybe someone
/// familiar with it can do
///
/// When send RequestExit, it will tell the event to finish.
///
/// Use `RequestSetCursor` with [`Cursor::Shape`] for standard shapes or [`Cursor::ThemeName`] for
/// an exact cursor name from the theme.
///
/// None means nothing will happened, no request, and no return data
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    RequestExit,
    RequestLock,
    RequestUnLock,
    RequestSetCursor { cursor: Cursor, pointer: WlPointer },
}

/// Describes a scroll along one axis
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct AxisScroll {
    /// The scroll measured in pixels.
    pub absolute: f64,

    /// The scroll measured in steps.
    ///
    /// Note: this might always be zero if the scrolling is due to a touchpad or other continuous
    /// source.
    pub discrete: i32,

    /// The scroll was stopped.
    ///
    /// Generally this is encountered when hardware indicates the end of some continuous scrolling.
    pub stop: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ime {
    /// Notifies when the IME was enabled.
    ///
    /// After getting this event you could receive [`Preedit`][Self::Preedit] and
    /// [`Commit`][Self::Commit] events. You should also start performing IME related requests
    Enabled,

    /// Notifies when a new composing text should be set at the cursor position.
    ///
    /// The value represents a pair of the preedit string and the cursor begin position and end
    /// position. When it's `None`, the cursor should be hidden. When `String` is an empty string
    /// this indicates that preedit was cleared.
    ///
    /// The cursor position is byte-wise indexed.
    Preedit(String, Option<CursorPosition>),

    /// Notifies when text should be inserted into the editor widget.
    ///
    /// Right before this event winit will send empty [`Self::Preedit`] event.
    Commit(String),

    /// Notifies when the IME was disabled.
    ///
    /// After receiving this event you won't get any more [`Preedit`][Self::Preedit] or
    /// [`Commit`][Self::Commit] events until the next [`Enabled`][Self::Enabled] event. You should
    /// also stop issuing IME related requests and clear
    /// pending preedit text.
    Disabled,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum DispatchMessage {
    NewDisplay(WlOutput),
    OutputAdded(OutputInfo),
    OutputUpdated(OutputInfo),
    OutputRemoved(OutputInfo),
    MouseButton {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
    },
    MouseLeave,
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
    Axis {
        time: u32,
        scale: f64,
        horizontal: AxisScroll,
        vertical: AxisScroll,
        source: Option<wl_pointer::AxisSource>,
    },
    TouchDown {
        serial: u32,
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    TouchUp {
        serial: u32,
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    TouchMotion {
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    TouchCancel {
        id: i32,
        x: f64,
        y: f64,
    },

    ModifiersChanged(ModifiersState),
    Focused(Id),
    Unfocus,
    KeyboardInput {
        event: KeyEvent,

        /// If `true`, the event was generated synthetically by winit
        /// in one of the following circumstances:
        ///
        /// * Synthetic key press events are generated for all keys pressed when a window gains
        ///   focus. Likewise, synthetic key release events are generated for all keys pressed when
        ///   a window goes out of focus. ***Currently, this is only functional on X11 and
        ///   Windows***
        ///
        /// Otherwise, this value is always `false`.
        is_synthetic: bool,
    },
    PreferredScale {
        scale_u32: u32,
        scale_float: f64,
    },
    OutputChanged(Option<WlOutput>),
    Locked,
    LockFinished,
    Ime(Ime),
    ToplevelStateChanged(ToplevelState),
}

/// This tell the DispatchMessage by dispatch
#[derive(Debug, Clone)]
pub enum ExWlShellEvent {
    /// forward the event of wayland-mouse
    MouseButton {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
    },
    /// Mouse leave the surface
    MouseLeave,
    /// forward the event of wayland-mouse
    MouseEnter {
        pointer: WlPointer,
        serial: u32,
        surface_x: f64,
        surface_y: f64,
    },
    /// forward the event of wayland-mouse
    MouseMotion {
        time: u32,
        surface_x: f64,
        surface_y: f64,
    },
    /// About the scroll
    Axis {
        time: u32,
        scale: f64,
        horizontal: AxisScroll,
        vertical: AxisScroll,
        source: Option<wl_pointer::AxisSource>,
    },
    /// forward the event of wayland-touch
    TouchDown {
        serial: u32,
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    /// forward the event of wayland-touch
    TouchUp {
        serial: u32,
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    /// forward the event of wayland-touch
    TouchMotion {
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    /// TouchEvent is cancelled
    TouchCancel {
        id: i32,
        x: f64,
        y: f64,
    },
    Focused(Id),
    Unfocus,
    /// Keyboard ModifiersChanged.
    ModifiersChanged(ModifiersState),
    /// Keyboard Event about input.
    KeyboardInput {
        event: KeyEvent,

        /// If `true`, the event was generated synthetically by winit
        /// in one of the following circumstances:
        ///
        /// * Synthetic key press events are generated for all keys pressed when a window gains
        ///   focus. Likewise, synthetic key release events are generated for all keys pressed when
        ///   a window goes out of focus. ***Currently, this is only functional on X11 and
        ///   Windows***
        ///
        /// Otherwise, this value is always `false`.
        is_synthetic: bool,
    },
    /// fractal scale handle
    PreferredScale {
        scale_u32: u32,
        scale_float: f64,
    },
    Ime(Ime),
    /// surface entered output, or left the one it was on
    OutputChanged(Option<WlOutput>),
    /// monitor was connected
    OutputAdded(OutputInfo),
    /// monitor mode, scale, name or position changed
    OutputUpdated(OutputInfo),
    /// monitor was disconnected
    OutputRemoved(OutputInfo),
    Locked,
    LockDenied,
    LockFinished,
    Closed,
    /// The compositor changed the state of an xdg toplevel window.
    ToplevelStateChanged(ToplevelState),
}

impl From<DispatchMessage> for ExWlShellEvent {
    fn from(val: DispatchMessage) -> Self {
        match val {
            DispatchMessage::NewDisplay(_) => {
                unreachable!("NewDisplay is handled before conversion")
            }
            DispatchMessage::OutputAdded(info) => ExWlShellEvent::OutputAdded(info),
            DispatchMessage::OutputUpdated(info) => ExWlShellEvent::OutputUpdated(info),
            DispatchMessage::OutputRemoved(info) => ExWlShellEvent::OutputRemoved(info),
            DispatchMessage::MouseButton {
                state,
                serial,
                button,
                time,
            } => ExWlShellEvent::MouseButton {
                state,
                serial,
                button,
                time,
            },
            DispatchMessage::MouseLeave => ExWlShellEvent::MouseLeave,
            DispatchMessage::MouseEnter {
                pointer,
                serial,
                surface_x,
                surface_y,
            } => ExWlShellEvent::MouseEnter {
                pointer,
                serial,
                surface_x,
                surface_y,
            },
            DispatchMessage::MouseMotion {
                time,
                surface_x,
                surface_y,
            } => ExWlShellEvent::MouseMotion {
                time,
                surface_x,
                surface_y,
            },
            DispatchMessage::TouchDown {
                serial,
                time,
                id,
                x,
                y,
            } => ExWlShellEvent::TouchDown {
                serial,
                time,
                id,
                x,
                y,
            },
            DispatchMessage::TouchUp {
                serial,
                time,
                id,
                x,
                y,
            } => ExWlShellEvent::TouchUp {
                serial,
                time,
                id,
                x,
                y,
            },
            DispatchMessage::TouchMotion { time, id, x, y } => {
                ExWlShellEvent::TouchMotion { time, id, x, y }
            }
            DispatchMessage::TouchCancel { id, x, y } => ExWlShellEvent::TouchCancel { id, x, y },
            DispatchMessage::Axis {
                time,
                scale,
                horizontal,
                vertical,
                source,
            } => ExWlShellEvent::Axis {
                time,
                scale,
                horizontal,
                vertical,
                source,
            },
            DispatchMessage::Focused(id) => ExWlShellEvent::Focused(id),
            DispatchMessage::Unfocus => ExWlShellEvent::Unfocus,
            DispatchMessage::ModifiersChanged(modifier) => {
                ExWlShellEvent::ModifiersChanged(modifier)
            }
            DispatchMessage::KeyboardInput {
                event,
                is_synthetic,
            } => ExWlShellEvent::KeyboardInput {
                event,
                is_synthetic,
            },
            DispatchMessage::PreferredScale {
                scale_u32,
                scale_float,
            } => ExWlShellEvent::PreferredScale {
                scale_u32,
                scale_float,
            },
            DispatchMessage::Ime(ime) => ExWlShellEvent::Ime(ime),
            DispatchMessage::OutputChanged(output) => ExWlShellEvent::OutputChanged(output),
            DispatchMessage::Locked => ExWlShellEvent::Locked,
            DispatchMessage::LockFinished => ExWlShellEvent::LockFinished,
            DispatchMessage::ToplevelStateChanged(state) => {
                ExWlShellEvent::ToplevelStateChanged(state)
            }
        }
    }
}
