use wayland_client::{
    globals::GlobalList,
    protocol::{
        wl_buffer::WlBuffer,
        wl_keyboard::KeyState,
        wl_output::WlOutput,
        wl_pointer::{ButtonState, WlPointer},
        wl_shm::WlShm,
    },
    QueueHandle, WEnum,
};

use super::WindowState;
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
#[derive(Debug)]
pub enum LayerEvent<'a, T: Debug> {
    InitRequest,
    XdgInfoChanged(XdgInfoChangedType),
    BindProvide(&'a GlobalList, &'a QueueHandle<WindowState<T>>),
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState<T>>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage),
    NormalDispatch,
}

/// the return data
/// Note: when event is RequestBuffer, you must return WlBuffer
/// Note: when receive InitRequest, you can request to bind extra wayland-protocols. this time you
/// can bind virtual-keybaord. you can take startcolorkeyboard as refrence, or the simple.rs. Also,
/// it should can bind with text-input, but I am not fully understand about this, maybe someone
/// famillar with it can do
///
/// When send RequestExist, it will tell the event to finish.
///
/// When send RequestSetCursorShape, you can set current pointer shape. pleace take
/// [cursor-shape](https://wayland.app/protocols/cursor-shape-v1#wp_cursor_shape_device_v1:enum:shape) as refrence.
///
/// None means nothing will happened, no request, and no return data
#[derive(Debug, PartialEq, Eq)]
pub enum ReturnData {
    WlBuffer(WlBuffer),
    RequestBind,
    RequestExist,
    RequestSetCursorShape((String, WlPointer, u32)),
    None,
}

/// this tell the what kind of information passed by [LayerEvent::XdgInfoChanged]
#[derive(Debug, Clone, Copy)]
pub enum XdgInfoChangedType {
    Position,
    Size,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum DispatchMessageInner {
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
    PrefredScale(u32),
    XdgInfoChanged(XdgInfoChangedType),
}

/// This tell the DispatchMessage by dispatch
#[derive(Debug)]
pub enum DispatchMessage {
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
    /// forward the event of wayland-touch
    TouchDown {
        serial: u32,
        time: u32,
        id: i32,
        x: f64,
        y: f64,
    },
    /// forward the event of wayland-touch
    TouchUp { serial: u32, time: u32, id: i32 },
    /// forward the event of wayland-touch
    TouchMotion { time: u32, id: i32, x: f64, y: f64 },
    /// forward the event of wayland-keyboard
    KeyBoard {
        state: WEnum<KeyState>,
        serial: u32,
        key: u32,
        time: u32,
    },
    /// this will request to do refresh the whole screen, because the layershell tell that a new
    /// configure happened
    RequestRefresh { width: u32, height: u32 },
    /// fractal scale handle
    PrefredScale(u32),
}

impl From<DispatchMessageInner> for DispatchMessage {
    fn from(val: DispatchMessageInner) -> Self {
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
            DispatchMessageInner::KeyBoard {
                state,
                serial,
                key,
                time,
            } => DispatchMessage::KeyBoard {
                state,
                serial,
                key,
                time,
            },
            DispatchMessageInner::RequestRefresh { width, height } => {
                DispatchMessage::RequestRefresh { width, height }
            }
            DispatchMessageInner::PrefredScale(scale) => DispatchMessage::PrefredScale(scale),
            DispatchMessageInner::RefreshSurface { .. } => unimplemented!(),
            DispatchMessageInner::XdgInfoChanged(_) => unimplemented!(),
        }
    }
}
