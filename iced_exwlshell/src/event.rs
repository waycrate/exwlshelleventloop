use exwlshellev::keyboard::ModifiersState;
use exwlshellev::reexport::wayland_client::{ButtonState, KeyState, WEnum, WlRegion};
use exwlshellev::xkb_keyboard::KeyEvent as LayerShellKeyEvent;
use exwlshellev::{ExWlShellBroadcast, ExWlShellEvent, WindowState};
use iced_core::mouse;

use crate::gesture;

use iced_core::keyboard::Modifiers as IcedModifiers;

use iced_wayland_subscriber::OutputInfo;

fn from_u32_to_icedmouse(code: u32) -> mouse::Button {
    match code {
        273 => mouse::Button::Right,
        274 => mouse::Button::Middle,
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
    Scroll {
        deltas: [Option<mouse::ScrollDelta>; 2],
        frame: gesture::Frame,
        stop: Option<gesture::Stop>,
    },
    PointerGesture(exwlshellev::PointerGesture),
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
    Ime(exwlshellev::Ime),
    Closed,
    ThemeChanged(iced_core::theme::Mode),
    OutputChanged(Option<OutputInfo>),
    OutputAdded(OutputInfo),
    OutputUpdated(OutputInfo),
    OutputRemoved(OutputInfo),
    Locked,
    LockDenied,
    LockFinished,
    ToplevelStateChanged(exwlshellev::ToplevelState),
}

#[derive(Debug)]
pub enum IcedWlShellEvent {
    UpdateInputRegion(WlRegion),
    Window(WindowEvent),
}

impl WindowEvent {
    pub(crate) fn from_dispatch<T>(value: ExWlShellEvent, ev: &WindowState<T>) -> Self {
        match value {
            ExWlShellEvent::Closed => WindowEvent::Closed,
            ExWlShellEvent::MouseEnter {
                surface_x: x,
                surface_y: y,
                ..
            } => WindowEvent::CursorEnter { x, y },
            ExWlShellEvent::MouseMotion {
                surface_x: x,
                surface_y: y,
                ..
            } => WindowEvent::CursorMoved { x, y },
            ExWlShellEvent::MouseLeave => WindowEvent::CursorLeft,
            ExWlShellEvent::MouseButton { state, button, .. } => {
                let btn = from_u32_to_icedmouse(button);
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
            ExWlShellEvent::TouchUp { id, x, y, .. } => WindowEvent::TouchUp { id, x, y },
            ExWlShellEvent::TouchDown { id, x, y, .. } => WindowEvent::TouchDown { id, x, y },
            ExWlShellEvent::TouchMotion { id, x, y, .. } => WindowEvent::TouchMotion { id, x, y },
            ExWlShellEvent::TouchCancel { id, x, y, .. } => WindowEvent::TouchCancel { id, x, y },
            ExWlShellEvent::PreferredScale {
                scale_u32,
                scale_float,
            } => WindowEvent::ScaleFactorChanged {
                scale_u32,
                scale_float,
            },

            ExWlShellEvent::KeyboardInput {
                event,
                is_synthetic,
            } => WindowEvent::KeyBoardInput {
                event,
                is_synthetic,
            },
            ExWlShellEvent::Unfocus => WindowEvent::Unfocus,
            ExWlShellEvent::Focused(_) => WindowEvent::Focused,
            ExWlShellEvent::ModifiersChanged(modifiers) => WindowEvent::ModifiersChanged(modifiers),
            ExWlShellEvent::Axis {
                horizontal,
                vertical,
                time,
                source,
            } => WindowEvent::Scroll {
                deltas: gesture::deltas(&horizontal, &vertical),
                frame: gesture::Frame::new(time, source, &horizontal, &vertical),
                stop: gesture::Stop::new(time, &horizontal, &vertical),
            },
            ExWlShellEvent::PointerGesture(gesture) => WindowEvent::PointerGesture(gesture),
            ExWlShellEvent::Ime(ime) => WindowEvent::Ime(ime.clone()),
            ExWlShellEvent::OutputChanged(wl_output) => WindowEvent::OutputChanged(
                wl_output
                    .as_ref()
                    .and_then(|output| ev.get_output_info_of(output)),
            ),
            ExWlShellEvent::ToplevelStateChanged(state) => WindowEvent::ToplevelStateChanged(state),
        }
    }
    pub(crate) fn from_dispatch_broadcast(value: ExWlShellBroadcast) -> Self {
        match value {
            ExWlShellBroadcast::OutputAdded(info) => WindowEvent::OutputAdded(info.clone()),
            ExWlShellBroadcast::OutputUpdated(info) => WindowEvent::OutputUpdated(info.clone()),
            ExWlShellBroadcast::OutputRemoved(info) => WindowEvent::OutputRemoved(info.clone()),
            ExWlShellBroadcast::Locked => WindowEvent::Locked,
            ExWlShellBroadcast::LockDenied => WindowEvent::LockDenied,
            ExWlShellBroadcast::LockFinished => WindowEvent::LockFinished,
        }
    }
}
