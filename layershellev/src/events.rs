use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer, zwlr_layer_surface_v1::Anchor,
};

use wayland_client::{
    globals::GlobalList,
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::WlOutput,
        wl_pointer::{self, ButtonState, WlPointer},
        wl_shm::WlShm,
    },
    QueueHandle, WEnum,
};

use crate::{id, xkb_keyboard::KeyEvent};

use crate::keyboard::ModifiersState;

use super::WindowState;

use crate::id::Id;

use std::{fmt::Debug, fs::File};

/// tell program what event is happened
///
/// InitRequest will tell the program is inited, you can request to Bind other wayland-protocols
/// there, with return [ReturnData::RequestBind]
///
/// RequestBuffer request to get the wl-buffer, so you init a buffer_pool here. It return a
/// GlobalList and a QueueHandle. This will enough for bind a extra wayland-protocol, and also,
/// seat can be gotten directly from [WindowState]
///
/// RequestMessages store the DispatchMessage, you can know what happened during dispatch with this
/// event.
pub enum LayerEvent<'a, T: Debug, Message, INFO: Clone> {
    InitRequest,
    XdgInfoChanged(XdgInfoChangedType),
    BindProvide(&'a GlobalList, &'a QueueHandle<WindowState<T, INFO>>),
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState<T, INFO>>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage<INFO>),
    NormalDispatch,
    UserEvent(Message),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewLayerShellSettings {
    pub size: Option<(u32, u32)>,
    pub layer: Layer,
    pub anchor: Anchor,
    pub exclude_zone: Option<i32>,
    pub margin: Option<(i32, i32, i32, i32)>,
}

/// the return data
/// Note: when event is RequestBuffer, you must return WlBuffer
/// Note: when receive InitRequest, you can request to bind extra wayland-protocols. this time you
/// can bind virtual-keyboard. you can take startcolorkeyboard as reference, or the simple.rs. Also,
/// it should can bind with text-input, but I am not fully understand about this, maybe someone
/// famillar with it can do
///
/// When send RequestExist, it will tell the event to finish.
///
/// When send RequestSetCursorShape, you can set current pointer shape. please take
/// [cursor-shape](https://wayland.app/protocols/cursor-shape-v1#wp_cursor_shape_device_v1:enum:shape) as reference.
///
/// None means nothing will happened, no request, and no return data
#[derive(Debug, PartialEq, Eq)]
pub enum ReturnData<INFO> {
    WlBuffer(WlBuffer),
    RequestBind,
    RequestExist,
    RedrawAllRequest,
    RedrawIndexRequest(Id),
    RequestSetCursorShape((String, WlPointer, u32)),
    NewLayerShell((NewLayerShellSettings, Option<INFO>)),
    RemoveLayershell(id::Id),
    None,
}

/// this tell the what kind of information passed by [LayerEvent::XdgInfoChanged]
#[derive(Debug, Clone, Copy)]
pub enum XdgInfoChangedType {
    Position,
    Size,
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

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum DispatchMessageInner<INFO: Clone> {
    NewDisplay(WlOutput),
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
    },
    TouchMotion {
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },

    ModifiersChanged(ModifiersState),
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
    RefreshSurface {
        width: u32,
        height: u32,
    },
    RequestRefresh {
        width: u32,
        height: u32,
        is_created: bool,
        info: Option<INFO>,
    },
    PrefredScale(u32),
    XdgInfoChanged(XdgInfoChangedType),
}

/// This tell the DispatchMessage by dispatch
#[derive(Debug)]
pub enum DispatchMessage<INFO: Clone> {
    /// forward the event of wayland-mouse
    MouseButton {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
    },
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
    Axis {
        time: u32,
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
    },
    /// forward the event of wayland-touch
    TouchMotion {
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },

    ModifiersChanged(ModifiersState),
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
    /// this will request to do refresh the whole screen, because the layershell tell that a new
    /// configure happened
    RequestRefresh {
        width: u32,
        height: u32,
        is_created: bool,
        info: Option<INFO>,
    },
    /// fractal scale handle
    PrefredScale(u32),
}

impl<INFO: Clone> From<DispatchMessageInner<INFO>> for DispatchMessage<INFO> {
    fn from(val: DispatchMessageInner<INFO>) -> Self {
        match val {
            DispatchMessageInner::NewDisplay(_) => unimplemented!(),
            DispatchMessageInner::MouseButton {
                state,
                serial,
                button,
                time,
            } => DispatchMessage::MouseButton {
                state,
                serial,
                button,
                time,
            },
            DispatchMessageInner::MouseLeave => DispatchMessage::MouseLeave,
            DispatchMessageInner::MouseEnter {
                pointer,
                serial,
                surface_x,
                surface_y,
            } => DispatchMessage::MouseEnter {
                pointer,
                serial,
                surface_x,
                surface_y,
            },
            DispatchMessageInner::MouseMotion {
                time,
                surface_x,
                surface_y,
            } => DispatchMessage::MouseMotion {
                time,
                surface_x,
                surface_y,
            },
            DispatchMessageInner::TouchDown {
                serial,
                time,
                id,
                x,
                y,
            } => DispatchMessage::TouchDown {
                serial,
                time,
                id,
                x,
                y,
            },
            DispatchMessageInner::TouchUp { serial, time, id } => {
                DispatchMessage::TouchUp { serial, time, id }
            }
            DispatchMessageInner::TouchMotion { time, id, x, y } => {
                DispatchMessage::TouchMotion { time, id, x, y }
            }
            DispatchMessageInner::RequestRefresh {
                width,
                height,
                is_created,
                info,
            } => DispatchMessage::RequestRefresh {
                width,
                height,
                is_created,
                info,
            },
            DispatchMessageInner::Axis {
                time,
                horizontal,
                vertical,
                source,
            } => DispatchMessage::Axis {
                time,
                horizontal,
                vertical,
                source,
            },
            DispatchMessageInner::ModifiersChanged(modifier) => {
                DispatchMessage::ModifiersChanged(modifier)
            }
            DispatchMessageInner::KeyboardInput {
                event,
                is_synthetic,
            } => DispatchMessage::KeyboardInput {
                event,
                is_synthetic,
            },
            DispatchMessageInner::PrefredScale(scale) => DispatchMessage::PrefredScale(scale),
            DispatchMessageInner::RefreshSurface { .. } => unimplemented!(),
            DispatchMessageInner::XdgInfoChanged(_) => unimplemented!(),
        }
    }
}
