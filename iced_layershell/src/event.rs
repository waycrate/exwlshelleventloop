use iced::mouse;
use iced_runtime::Action;
use layershellev::DispatchMessage;
use layershellev::keyboard::ModifiersState;
use layershellev::reexport::wayland_client::{ButtonState, KeyState, WEnum, WlRegion};
use layershellev::xkb_keyboard::KeyEvent as LayerShellKeyEvent;

use iced_core::keyboard::Modifiers as IcedModifiers;

fn from_u32_to_icedmouse(code: u32) -> mouse::Button {
    match code {
        273 => mouse::Button::Right,
        _ => mouse::Button::Left,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IcedButtonState {
    Pressed(mouse::Button),
    Released(mouse::Button),
}

#[derive(Debug, Clone, Copy)]
pub enum IcedKeyState {
    Pressed,
    Released,
}

impl From<WEnum<KeyState>> for IcedKeyState {
    fn from(value: WEnum<KeyState>) -> Self {
        match value {
            WEnum::Value(KeyState::Released) => Self::Released,
            WEnum::Value(KeyState::Pressed) => Self::Pressed,
            _ => unreachable!(),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum WindowEvent {
    ScaleFactorChanged {
        scale_u32: u32,
        scale_float: f64,
    },
    CursorEnter {
        x: f64,
        y: f64,
    },
    CursorMoved {
        x: f64,
        y: f64,
    },
    CursorLeft,
    MouseInput(IcedButtonState),
    Keyboard {
        state: IcedKeyState,
        key: u32,
        modifiers: IcedModifiers,
    },
    KeyBoardInput {
        event: LayerShellKeyEvent,
        is_synthetic: bool,
    },
    Unfocus,
    Focused,
    ModifiersChanged(ModifiersState),
    Axis {
        x: f32,
        y: f32,
    },
    PixelDelta {
        x: f32,
        y: f32,
    },
    ScrollStop,
    TouchDown {
        id: i32,
        x: f64,
        y: f64,
    },
    TouchUp {
        id: i32,
        x: f64,
        y: f64,
    },
    TouchMotion {
        id: i32,
        x: f64,
        y: f64,
    },
    TouchCancel {
        id: i32,
        x: f64,
        y: f64,
    },
    Ime(layershellev::Ime),
    Refresh,
    Closed,
}

#[derive(Debug)]
pub enum IcedLayerShellEvent<Message> {
    UpdateInputRegion(WlRegion),
    Window(WindowEvent),
    UserAction(Action<Message>),
    NormalDispatch,
}

impl From<&DispatchMessage> for WindowEvent {
    fn from(value: &DispatchMessage) -> Self {
        match value {
            DispatchMessage::RequestRefresh { .. } => WindowEvent::Refresh,
            DispatchMessage::Closed => WindowEvent::Closed,
            DispatchMessage::MouseEnter {
                surface_x: x,
                surface_y: y,
                ..
            } => WindowEvent::CursorEnter { x: *x, y: *y },
            DispatchMessage::MouseMotion {
                surface_x: x,
                surface_y: y,
                ..
            } => WindowEvent::CursorMoved { x: *x, y: *y },
            DispatchMessage::MouseLeave => WindowEvent::CursorLeft,
            DispatchMessage::MouseButton { state, button, .. } => {
                let btn = from_u32_to_icedmouse(*button);
                match state {
                    WEnum::Value(ButtonState::Pressed) => {
                        WindowEvent::MouseInput(IcedButtonState::Pressed(btn))
                    }
                    WEnum::Value(ButtonState::Released) => {
                        WindowEvent::MouseInput(IcedButtonState::Released(btn))
                    }
                    _ => unreachable!(),
                }
            }
            DispatchMessage::TouchUp { id, x, y, .. } => WindowEvent::TouchUp {
                id: *id,
                x: *x,
                y: *y,
            },
            DispatchMessage::TouchDown { id, x, y, .. } => WindowEvent::TouchDown {
                id: *id,
                x: *x,
                y: *y,
            },
            DispatchMessage::TouchMotion { id, x, y, .. } => WindowEvent::TouchMotion {
                id: *id,
                x: *x,
                y: *y,
            },
            DispatchMessage::TouchCancel { id, x, y, .. } => WindowEvent::TouchCancel {
                id: *id,
                x: *x,
                y: *y,
            },
            DispatchMessage::PreferredScale {
                scale_u32,
                scale_float,
            } => WindowEvent::ScaleFactorChanged {
                scale_u32: *scale_u32,
                scale_float: *scale_float,
            },

            DispatchMessage::KeyboardInput {
                event,
                is_synthetic,
            } => WindowEvent::KeyBoardInput {
                event: event.clone(),
                is_synthetic: *is_synthetic,
            },
            DispatchMessage::Unfocus => WindowEvent::Unfocus,
            DispatchMessage::Focused(_) => WindowEvent::Focused,
            DispatchMessage::ModifiersChanged(modifiers) => {
                WindowEvent::ModifiersChanged(*modifiers)
            }
            DispatchMessage::Axis {
                horizontal,
                vertical,
                scale,
                ..
            } => {
                if horizontal.stop && vertical.stop {
                    WindowEvent::ScrollStop
                } else if vertical.discrete != 0 || horizontal.discrete != 0 {
                    WindowEvent::Axis {
                        x: (-horizontal.discrete as f64 * scale) as f32,
                        y: (-vertical.discrete as f64 * scale) as f32,
                    }
                } else {
                    WindowEvent::PixelDelta {
                        x: (-horizontal.absolute * scale) as f32,
                        y: (-vertical.absolute * scale) as f32,
                    }
                }
            }
            DispatchMessage::Ime(ime) => WindowEvent::Ime(ime.clone()),
        }
    }
}
