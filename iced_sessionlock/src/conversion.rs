mod keymap;

use std::ops::Mul;

use crate::event::IcedButtonState;
use crate::event::WindowEvent as SessionLockEvent;
use iced::touch;
use iced_core::SmolStr;
use keymap::{key, physical_key};
use sessionlockev::keyboard::KeyLocation;
use sessionlockev::xkb_keyboard::ElementState;
use sessionlockev::xkb_keyboard::KeyEvent as SessionLockKeyEvent;

use iced_core::{Event as IcedEvent, keyboard, mouse};
use sessionlockev::keyboard::ModifiersState;

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
    layerevent: &SessionLockEvent,
    application_scale_factor: f64,
    modifiers: ModifiersState,
) -> Option<IcedEvent> {
    match layerevent {
        SessionLockEvent::CursorLeft => Some(IcedEvent::Mouse(mouse::Event::CursorLeft)),
        SessionLockEvent::CursorMoved { x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: iced::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        SessionLockEvent::CursorEnter { .. } => Some(IcedEvent::Mouse(mouse::Event::CursorEntered)),
        SessionLockEvent::MouseInput(state) => Some(IcedEvent::Mouse(match state {
            IcedButtonState::Pressed => mouse::Event::ButtonPressed(mouse::Button::Left),
            IcedButtonState::Released => mouse::Event::ButtonReleased(mouse::Button::Left),
        })),
        SessionLockEvent::Axis { x, y } => Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: *x, y: *y },
        })),

        SessionLockEvent::PixelDelta { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: *x, y: *y },
            }))
        }
        SessionLockEvent::TouchDown { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerPressed {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        SessionLockEvent::TouchUp { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLifted {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        SessionLockEvent::TouchMotion { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerMoved {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        SessionLockEvent::TouchCancel { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLost {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        SessionLockEvent::KeyBoardInput { event, .. } => Some(IcedEvent::Keyboard({
            let key = event.key_without_modifiers();
            let text = event
                .text_with_all_modifiers()
                .map(SmolStr::new)
                .filter(|text| !text.as_str().chars().any(is_private_use));
            let SessionLockKeyEvent {
                state,
                location,
                logical_key,
                physical_key,
                ..
            } = event;
            let key = self::key(key);
            let modifiers = keymap::modifiers(modifiers);
            let modified_key = self::key(logical_key.clone());
            let physical_key = self::physical_key(*physical_key);

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
                },
                ElementState::Released => keyboard::Event::KeyReleased {
                    physical_key,
                    key,
                    location,
                    modifiers,
                    modified_key,
                },
            }
        })),
        SessionLockEvent::ModifiersChanged(new_modifiers) => Some(IcedEvent::Keyboard(
            keyboard::Event::ModifiersChanged(keymap::modifiers(*new_modifiers)),
        )),
        SessionLockEvent::Unfocus => Some(IcedEvent::Window(iced::window::Event::Unfocused)),
        SessionLockEvent::Focused => Some(IcedEvent::Window(iced::window::Event::Focused)),
        _ => None,
    }
}

pub(crate) fn mouse_interaction(interaction: mouse::Interaction) -> String {
    use mouse::Interaction;
    use sessionlockev::reexport::wp_cursor_shape_device_v1::{Shape, ShapeName};
    match interaction {
        Interaction::None => Shape::Default.name().to_owned(),
        Interaction::Idle => Shape::Wait.name().to_owned(),
        Interaction::Pointer => Shape::Pointer.name().to_owned(),
        Interaction::Working => Shape::Pointer.name().to_owned(),
        Interaction::Grab => Shape::Grab.name().to_owned(),
        Interaction::Text => Shape::Text.name().to_owned(),
        Interaction::ZoomIn => Shape::ZoomIn.name().to_owned(),
        Interaction::Grabbing => Shape::Grabbing.name().to_owned(),
        Interaction::Crosshair => Shape::Crosshair.name().to_owned(),
        Interaction::NotAllowed => Shape::NotAllowed.name().to_owned(),
        Interaction::ResizingVertically => Shape::NsResize.name().to_owned(),
        Interaction::ResizingHorizontally => Shape::EwResize.name().to_owned(),
        Interaction::Cell => Shape::Cell.name().to_owned(),
        Interaction::Move => Shape::Move.name().to_owned(),
        Interaction::Copy => Shape::Copy.name().to_owned(),
        Interaction::Help => Shape::Help.name().to_owned(),
        Interaction::ZoomOut => Shape::ZoomOut.name().to_owned(),
        Interaction::ResizingDiagonallyUp => Shape::NwseResize.name().to_owned(),
        Interaction::ResizingDiagonallyDown => Shape::NwseResize.name().to_owned(),
    }
}

fn is_private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}
