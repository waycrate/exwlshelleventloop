mod keymap;

use crate::event::IcedButtonState;
use crate::event::WindowEvent as ExWlShellEvent;
use exwlshellev::keyboard::KeyLocation;
use exwlshellev::keyboard::ModifiersState;
use exwlshellev::xkb_keyboard::ElementState;
use exwlshellev::xkb_keyboard::KeyEvent as ExWlShellKeyEvent;
use iced_core::SmolStr;
use iced_core::input_method;
use iced_core::touch;
use iced_core::{Event as IcedEvent, keyboard, mouse};
use keymap::{key, physical_key};
use std::ops::Mul;

fn scale_down<T>((x, y): (T, T), application_scale_factor: f64) -> (T, T)
where
    T: Mul + TryInto<f64> + TryFrom<f64>,
    <T as TryInto<f64>>::Error: std::fmt::Debug,
    <T as TryFrom<f64>>::Error: std::fmt::Debug,
{
    let (mut x, mut y): (f64, f64) = (x.try_into().unwrap(), y.try_into().unwrap());
    x /= application_scale_factor;
    y /= application_scale_factor;
    (x.try_into().unwrap(), y.try_into().unwrap())
}

pub fn window_event(
    shellevent: &ExWlShellEvent,
    application_scale_factor: f64,
    modifiers: ModifiersState,
) -> Option<IcedEvent> {
    match shellevent {
        ExWlShellEvent::CursorLeft => Some(IcedEvent::Mouse(mouse::Event::CursorLeft)),
        ExWlShellEvent::CursorMoved { x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        ExWlShellEvent::CursorEnter { .. } => Some(IcedEvent::Mouse(mouse::Event::CursorEntered)),
        ExWlShellEvent::MouseInput(state) => Some(IcedEvent::Mouse(match state {
            IcedButtonState::Pressed(btn) => mouse::Event::ButtonPressed(*btn),
            IcedButtonState::Released(btn) => mouse::Event::ButtonReleased(*btn),
        })),
        ExWlShellEvent::Axis { x, y } => Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: *x, y: *y },
        })),

        ExWlShellEvent::PixelDelta { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: *x, y: *y },
            }))
        }
        ExWlShellEvent::KeyBoardInput { event, .. } => Some(IcedEvent::Keyboard({
            let key = event.key_without_modifiers.clone();
            let text = event
                .text_with_all_modifiers
                .clone()
                .map(SmolStr::new)
                .filter(|text| !text.as_str().chars().any(is_private_use));
            let ExWlShellKeyEvent {
                state,
                location,
                logical_key,
                physical_key,
                repeat,
                ..
            } = event;
            let key = self::key(key);
            let modified_key = self::key(logical_key.clone());

            let physical_key = self::physical_key(*physical_key);

            let modifiers = keymap::modifiers(modifiers);

            let location = match location {
                KeyLocation::Standard => keyboard::Location::Standard,
                KeyLocation::Left => keyboard::Location::Left,
                KeyLocation::Right => keyboard::Location::Right,
                KeyLocation::Numpad => keyboard::Location::Numpad,
            };
            match state {
                ElementState::Pressed => keyboard::Event::KeyPressed {
                    key,
                    location,
                    modifiers,
                    text,
                    modified_key,
                    physical_key,
                    repeat: *repeat,
                },
                ElementState::Released => keyboard::Event::KeyReleased {
                    key,
                    location,
                    modifiers,
                    physical_key,
                    modified_key,
                },
            }
        })),
        ExWlShellEvent::TouchDown { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerPressed {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        ExWlShellEvent::TouchUp { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLifted {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        ExWlShellEvent::TouchMotion { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerMoved {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        ExWlShellEvent::TouchCancel { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLost {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        ExWlShellEvent::ModifiersChanged(new_modifiers) => Some(IcedEvent::Keyboard(
            keyboard::Event::ModifiersChanged(keymap::modifiers(*new_modifiers)),
        )),
        ExWlShellEvent::Unfocus => Some(IcedEvent::Window(iced_core::window::Event::Unfocused)),
        ExWlShellEvent::Focused => Some(IcedEvent::Window(iced_core::window::Event::Focused)),
        ExWlShellEvent::Ime(event) => Some(IcedEvent::InputMethod(match event {
            exwlshellev::Ime::Enabled => input_method::Event::Opened,
            exwlshellev::Ime::Preedit(content, size) => {
                input_method::Event::Preedit(content.clone(), size.map(|(start, end)| start..end))
            }
            exwlshellev::Ime::Commit(content) => input_method::Event::Commit(content.clone()),
            exwlshellev::Ime::Disabled => input_method::Event::Closed,
        })),
        _ => None,
    }
}

pub fn ime_purpose(purpose: input_method::Purpose) -> exwlshellev::ImePurpose {
    match purpose {
        input_method::Purpose::Normal => exwlshellev::ImePurpose::Normal,
        input_method::Purpose::Secure => exwlshellev::ImePurpose::Password,
        input_method::Purpose::Terminal => exwlshellev::ImePurpose::Terminal,
    }
}

pub(crate) fn mouse_interaction(interaction: mouse::Interaction) -> exwlshellev::CursorShape {
    use exwlshellev::CursorShape as Shape;
    use mouse::Interaction;
    match interaction {
        Interaction::None | Interaction::Idle | Interaction::Hidden => Shape::Default,
        Interaction::ContextMenu => Shape::ContextMenu,
        Interaction::Help => Shape::Help,
        Interaction::Pointer => Shape::Pointer,
        Interaction::Progress => Shape::Progress,
        Interaction::Wait => Shape::Wait,
        Interaction::Cell => Shape::Cell,
        Interaction::Crosshair => Shape::Crosshair,
        Interaction::Text => Shape::Text,
        Interaction::Alias => Shape::Alias,
        Interaction::Copy => Shape::Copy,
        Interaction::Move => Shape::Move,
        Interaction::NoDrop => Shape::NoDrop,
        Interaction::NotAllowed => Shape::NotAllowed,
        Interaction::Grab => Shape::Grab,
        Interaction::Grabbing => Shape::Grabbing,
        Interaction::ResizingHorizontally => Shape::EwResize,
        Interaction::ResizingVertically => Shape::NsResize,
        Interaction::ResizingDiagonallyUp => Shape::NeswResize,
        Interaction::ResizingDiagonallyDown => Shape::NwseResize,
        Interaction::ResizingColumn => Shape::ColResize,
        Interaction::ResizingRow => Shape::RowResize,
        Interaction::AllScroll => Shape::AllScroll,
        Interaction::ZoomIn => Shape::ZoomIn,
        Interaction::ZoomOut => Shape::ZoomOut,
    }
}

fn is_private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}
