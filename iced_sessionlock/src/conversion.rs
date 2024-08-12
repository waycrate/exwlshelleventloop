mod keymap;

use crate::event::IcedButtonState;
use crate::event::WindowEvent as SessionLockEvent;
use iced::touch;
use iced_core::SmolStr;
use keymap::key;
use sessionlockev::keyboard::KeyLocation;
use sessionlockev::xkb_keyboard::ElementState;
use sessionlockev::xkb_keyboard::KeyEvent as SessionLockKeyEvent;

use iced_core::{keyboard, mouse, Event as IcedEvent};
use sessionlockev::keyboard::ModifiersState;

#[allow(unused)]
pub fn window_event(
    id: iced_core::window::Id,
    layerevent: &SessionLockEvent,

    modifiers: ModifiersState,
) -> Option<IcedEvent> {
    match layerevent {
        SessionLockEvent::CursorLeft => Some(IcedEvent::Mouse(mouse::Event::CursorLeft)),
        SessionLockEvent::CursorMoved { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: iced_core::Point {
                    x: *x as f32,
                    y: *y as f32,
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
            Some(IcedEvent::Touch(touch::Event::FingerPressed {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: *x as f32,
                    y: *y as f32,
                },
            }))
        }
        SessionLockEvent::TouchUp { id, x, y } => {
            Some(IcedEvent::Touch(touch::Event::FingerLifted {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: *x as f32,
                    y: *y as f32,
                },
            }))
        }
        SessionLockEvent::TouchMotion { id, x, y } => {
            Some(IcedEvent::Touch(touch::Event::FingerMoved {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: *x as f32,
                    y: *y as f32,
                },
            }))
        }
        SessionLockEvent::TouchCancel { id, x, y } => {
            Some(IcedEvent::Touch(touch::Event::FingerLost {
                id: touch::Finger(*id as u64),
                position: iced::Point {
                    x: *x as f32,
                    y: *y as f32,
                },
            }))
        }
        SessionLockEvent::KeyBoardInput { event, .. } => Some(IcedEvent::Keyboard({
            let logical_key = event.key_without_modifiers();
            let text = event
                .text_with_all_modifiers()
                .map(SmolStr::new)
                .filter(|text| !text.as_str().chars().any(is_private_use));
            let SessionLockKeyEvent {
                state, location, ..
            } = event;
            let key = key(logical_key);
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
                },
                ElementState::Released => keyboard::Event::KeyReleased {
                    key,
                    location,
                    modifiers,
                },
            }
        })),
        SessionLockEvent::ModifiersChanged(new_modifiers) => Some(IcedEvent::Keyboard(
            keyboard::Event::ModifiersChanged(keymap::modifiers(*new_modifiers)),
        )),
        _ => None,
    }
}

pub(crate) fn mouse_interaction(interaction: mouse::Interaction) -> String {
    use mouse::Interaction;
    match interaction {
        Interaction::Idle => "default".to_owned(),
        Interaction::Pointer => "pointer".to_owned(),
        Interaction::Working => "progress".to_owned(),
        Interaction::Grab => "grab".to_owned(),
        Interaction::Text => "text".to_owned(),
        Interaction::ZoomIn => "zoom_in".to_owned(),
        Interaction::Grabbing => "grabbing".to_owned(),
        Interaction::Crosshair => "crosshair".to_owned(),
        Interaction::NotAllowed => "not_allowed".to_owned(),
        Interaction::ResizingVertically => "ns_resize".to_owned(),
        Interaction::ResizingHorizontally => "ew_resize".to_owned(),
    }
}

fn is_private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}
