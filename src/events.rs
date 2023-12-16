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
use std::fs::File;

#[derive(Debug)]
pub enum LayerEvent<'a> {
    InitRequest,
    BindProvide(&'a GlobalList, &'a QueueHandle<WindowState>),
    RequestBuffer(
        &'a mut File,
        &'a WlShm,
        &'a QueueHandle<WindowState>,
        u32,
        u32,
    ),
    RequestMessages(&'a DispatchMessage),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReturnData {
    WlBuffer(WlBuffer),
    RequestBind,
    RequestExist,
    RequestSetCursorShape((String, WlPointer, u32)),
    None,
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
}

#[derive(Debug)]
pub enum DispatchMessage {
    MouseButton {
        state: WEnum<ButtonState>,
        serial: u32,
        button: u32,
        time: u32,
    },
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
            DispatchMessageInner::RefreshSurface { .. } => unimplemented!(),
        }
    }
}
