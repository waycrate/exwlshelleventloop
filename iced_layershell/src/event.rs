use layershellev::reexport::wayland_client::{ButtonState, WEnum};
use layershellev::DispatchMessage;

#[derive(Debug, Clone, Copy)]
pub enum IcedButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    ScaleChanged(u32),
    CursorEnter { x: f64, y: f64 },
    CursorMoved { x: f64, y: f64 },
    CursorLeft,
    MouseInput(IcedButtonState),
}

#[derive(Debug, Clone, Copy)]
pub enum IcedLayerEvent {
    RequestRefresh { width: u32, height: u32 },
    Window(WindowEvent),
    NormalUpdate,
}

impl From<&DispatchMessage> for IcedLayerEvent {
    fn from(value: &DispatchMessage) -> Self {
        match value {
            DispatchMessage::RequestRefresh { width, height } => IcedLayerEvent::RequestRefresh {
                width: *width,
                height: *height,
            },
            DispatchMessage::MouseEnter {
                surface_x: x,
                surface_y: y,
                ..
            } => IcedLayerEvent::Window(WindowEvent::CursorEnter { x: *x, y: *y }),
            DispatchMessage::MouseMotion {
                surface_x: x,
                surface_y: y,
                ..
            }
            | DispatchMessage::TouchMotion { x, y, .. } => {
                IcedLayerEvent::Window(WindowEvent::CursorMoved { x: *x, y: *y })
            }
            DispatchMessage::MouseLeave => IcedLayerEvent::Window(WindowEvent::CursorLeft),
            DispatchMessage::MouseButton { state, .. } => match state {
                WEnum::Value(ButtonState::Pressed) => {
                    IcedLayerEvent::Window(WindowEvent::MouseInput(IcedButtonState::Pressed))
                }
                WEnum::Value(ButtonState::Released) => {
                    IcedLayerEvent::Window(WindowEvent::MouseInput(IcedButtonState::Released))
                }
                _ => unreachable!(),
            },
            DispatchMessage::PrefredScale(scale) => {
                IcedLayerEvent::Window(WindowEvent::ScaleChanged(*scale))
            }
            _ => Self::NormalUpdate,
        }
    }
}
