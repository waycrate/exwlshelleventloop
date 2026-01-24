mod keymap;

use crate::event::IcedButtonState;
use crate::event::WindowEvent as LayerShellEvent;
use iced_core::SmolStr;
use iced_core::input_method;
use iced_core::touch;
use iced_core::{Event as IcedEvent, keyboard, mouse};
use keymap::{key, physical_key};
use layershellev::keyboard::KeyLocation;
use layershellev::keyboard::ModifiersState;
use layershellev::xkb_keyboard::ElementState;
use layershellev::xkb_keyboard::KeyEvent as LayerShellKeyEvent;
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
    layerevent: &LayerShellEvent,
    application_scale_factor: f64,
    modifiers: ModifiersState,
) -> Option<IcedEvent> {
    match layerevent {
        LayerShellEvent::CursorLeft => Some(IcedEvent::Mouse(mouse::Event::CursorLeft)),
        LayerShellEvent::CursorMoved { x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        LayerShellEvent::CursorEnter { .. } => Some(IcedEvent::Mouse(mouse::Event::CursorEntered)),
        LayerShellEvent::MouseInput(state) => Some(IcedEvent::Mouse(match state {
            IcedButtonState::Pressed(btn) => mouse::Event::ButtonPressed(*btn),
            IcedButtonState::Released(btn) => mouse::Event::ButtonReleased(*btn),
        })),
        LayerShellEvent::Axis { x, y } => Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: *x, y: *y },
        })),

        LayerShellEvent::PixelDelta { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: *x, y: *y },
            }))
        }
        LayerShellEvent::KeyBoardInput { event, .. } => Some(IcedEvent::Keyboard({
            let key = event.key_without_modifiers();
            let text = event
                .text_with_all_modifiers()
                .map(SmolStr::new)
                .filter(|text| !text.as_str().chars().any(is_private_use));
            let LayerShellKeyEvent {
                state,
                location,
                logical_key,
                physical_key,
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
                    repeat: false,
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
        LayerShellEvent::TouchDown { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerPressed {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        LayerShellEvent::TouchUp { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLifted {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        LayerShellEvent::TouchMotion { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerMoved {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        LayerShellEvent::TouchCancel { id, x, y } => {
            let (x, y) = scale_down((*x, *y), application_scale_factor);
            Some(IcedEvent::Touch(touch::Event::FingerLost {
                id: touch::Finger(*id as u64),
                position: iced_core::Point {
                    x: x as f32,
                    y: y as f32,
                },
            }))
        }
        LayerShellEvent::ModifiersChanged(new_modifiers) => Some(IcedEvent::Keyboard(
            keyboard::Event::ModifiersChanged(keymap::modifiers(*new_modifiers)),
        )),
        LayerShellEvent::Unfocus => Some(IcedEvent::Window(iced_core::window::Event::Unfocused)),
        LayerShellEvent::Focused => Some(IcedEvent::Window(iced_core::window::Event::Focused)),
        LayerShellEvent::Ime(event) => Some(IcedEvent::InputMethod(match event {
            layershellev::Ime::Enabled => input_method::Event::Opened,
            layershellev::Ime::Preedit(content, size) => {
                input_method::Event::Preedit(content.clone(), size.map(|(start, end)| start..end))
            }
            layershellev::Ime::Commit(content) => input_method::Event::Commit(content.clone()),
            layershellev::Ime::Disabled => input_method::Event::Closed,
        })),
        _ => None,
    }
}

pub fn ime_purpose(purpose: input_method::Purpose) -> layershellev::ImePurpose {
    match purpose {
        input_method::Purpose::Normal => layershellev::ImePurpose::Normal,
        input_method::Purpose::Secure => layershellev::ImePurpose::Password,
        input_method::Purpose::Terminal => layershellev::ImePurpose::Terminal,
    }
}

pub(crate) fn mouse_interaction(interaction: mouse::Interaction) -> String {
    use layershellev::reexport::wp_cursor_shape_device_v1::{Shape, ShapeName};
    use mouse::Interaction;
    match interaction {
        Interaction::None => Shape::Default.name().to_owned(),
        Interaction::Idle => Shape::Wait.name().to_owned(),
        Interaction::Wait => Shape::Wait.name().to_owned(),
        Interaction::Pointer => Shape::Pointer.name().to_owned(),
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
        _ => Shape::Default.name().to_owned(),
    }
}

fn is_private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}
