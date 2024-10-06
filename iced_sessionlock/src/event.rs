use sessionlockev::id::Id;
use sessionlockev::keyboard::ModifiersState;
use sessionlockev::reexport::wayland_client::{ButtonState, KeyState, WEnum};
use sessionlockev::xkb_keyboard::KeyEvent as SessionLockEvent;
use sessionlockev::{DispatchMessage, WindowWrapper};

use iced_core::keyboard::Modifiers as IcedModifiers;
#[derive(Debug, Clone, Copy)]
pub enum IcedButtonState {
    Pressed,
    Released,
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
        event: SessionLockEvent,
        is_synthetic: bool,
    },
    ModifiersChanged(ModifiersState),
    Axis {
        x: f32,
        y: f32,
    },
    PixelDelta {
        x: f32,
        y: f32,
    },
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
}

#[derive(Debug)]
pub enum IcedSessionLockEvent<Message: 'static> {
    RequestRefreshWithWrapper {
        width: u32,
        height: u32,
        wrapper: WindowWrapper,
    },
    #[allow(unused)]
    RequestRefresh {
        width: u32,
        height: u32,
    },
    Window(WindowEvent),
    NormalUpdate,
    UserEvent(Message),
}

#[derive(Debug)]
pub struct MultiWindowIcedSessionLockEvent<Message: 'static>(
    pub Option<Id>,
    pub IcedSessionLockEvent<Message>,
);

impl<Message: 'static> From<(Option<Id>, IcedSessionLockEvent<Message>)>
    for MultiWindowIcedSessionLockEvent<Message>
{
    fn from((id, message): (Option<Id>, IcedSessionLockEvent<Message>)) -> Self {
        MultiWindowIcedSessionLockEvent(id, message)
    }
}

impl<Message: 'static> From<&DispatchMessage> for IcedSessionLockEvent<Message> {
    fn from(value: &DispatchMessage) -> Self {
        match value {
            DispatchMessage::RequestRefresh { width, height, .. } => {
                IcedSessionLockEvent::RequestRefresh {
                    width: *width,
                    height: *height,
                }
            }
            DispatchMessage::MouseEnter {
                surface_x: x,
                surface_y: y,
                ..
            } => IcedSessionLockEvent::Window(WindowEvent::CursorEnter { x: *x, y: *y }),
            DispatchMessage::MouseMotion {
                surface_x: x,
                surface_y: y,
                ..
            } => IcedSessionLockEvent::Window(WindowEvent::CursorMoved { x: *x, y: *y }),
            DispatchMessage::MouseLeave => IcedSessionLockEvent::Window(WindowEvent::CursorLeft),
            DispatchMessage::MouseButton { state, .. } => match state {
                WEnum::Value(ButtonState::Pressed) => {
                    IcedSessionLockEvent::Window(WindowEvent::MouseInput(IcedButtonState::Pressed))
                }
                WEnum::Value(ButtonState::Released) => {
                    IcedSessionLockEvent::Window(WindowEvent::MouseInput(IcedButtonState::Released))
                }
                _ => unreachable!(),
            },
            DispatchMessage::TouchUp { id, x, y, .. } => {
                IcedSessionLockEvent::Window(WindowEvent::TouchUp {
                    id: *id,
                    x: *x,
                    y: *y,
                })
            }
            DispatchMessage::TouchDown { id, x, y, .. } => {
                IcedSessionLockEvent::Window(WindowEvent::TouchDown {
                    id: *id,
                    x: *x,
                    y: *y,
                })
            }
            DispatchMessage::TouchMotion { id, x, y, .. } => {
                IcedSessionLockEvent::Window(WindowEvent::TouchMotion {
                    id: *id,
                    x: *x,
                    y: *y,
                })
            }
            DispatchMessage::TouchCancel { id, x, y, .. } => {
                IcedSessionLockEvent::Window(WindowEvent::TouchCancel {
                    id: *id,
                    x: *x,
                    y: *y,
                })
            }
            DispatchMessage::PreferredScale {
                scale_float,
                scale_u32,
            } => IcedSessionLockEvent::Window(WindowEvent::ScaleFactorChanged {
                scale_u32: *scale_u32,
                scale_float: *scale_float,
            }),
            DispatchMessage::KeyboardInput {
                event,
                is_synthetic,
            } => IcedSessionLockEvent::Window(WindowEvent::KeyBoardInput {
                event: event.clone(),
                is_synthetic: *is_synthetic,
            }),
            DispatchMessage::ModifiersChanged(modifiers) => {
                IcedSessionLockEvent::Window(WindowEvent::ModifiersChanged(*modifiers))
            }
            DispatchMessage::Axis {
                horizontal,
                vertical,
                ..
            } => {
                if horizontal.stop && vertical.stop {
                    return Self::NormalUpdate;
                }
                let has_scroll = vertical.discrete != 0 || horizontal.discrete != 0;
                if has_scroll {
                    return IcedSessionLockEvent::Window(WindowEvent::Axis {
                        x: -horizontal.discrete as f32,
                        y: -vertical.discrete as f32,
                    });
                }
                IcedSessionLockEvent::Window(WindowEvent::Axis {
                    x: -horizontal.absolute as f32,
                    y: -vertical.absolute as f32,
                })
            }
        }
    }
}
