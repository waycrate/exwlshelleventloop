mod keymap;

use crate::event::IcedButtonState;
use crate::event::IcedKeyState;
use crate::event::WindowEvent as LayerShellEvent;
use iced_core::SmolStr;
use iced_core::{keyboard, mouse, Event as IcedEvent};
use keymap::key;
use keymap::{key_from_u32, text_from_key};
use layershellev::KeyEvent as LayerShellKeyEvent;

#[allow(unused)]
pub fn window_event(id: iced_core::window::Id, layerevent: &LayerShellEvent) -> Option<IcedEvent> {
    match layerevent {
        LayerShellEvent::CursorLeft => Some(IcedEvent::Mouse(mouse::Event::CursorLeft)),
        LayerShellEvent::CursorMoved { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::CursorMoved {
                position: iced_core::Point {
                    x: *x as f32,
                    y: *y as f32,
                },
            }))
        }
        LayerShellEvent::CursorEnter { .. } => Some(IcedEvent::Mouse(mouse::Event::CursorEntered)),
        LayerShellEvent::MouseInput(state) => Some(IcedEvent::Mouse(match state {
            IcedButtonState::Pressed => mouse::Event::ButtonPressed(mouse::Button::Left),
            IcedButtonState::Released => mouse::Event::ButtonReleased(mouse::Button::Left),
        })),
        LayerShellEvent::Axis { x, y } => Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: *x, y: *y },
        })),

        LayerShellEvent::PixelDelta { x, y } => {
            Some(IcedEvent::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: *x, y: *y },
            }))
        }
        LayerShellEvent::Keyboard {
            state,
            key,
            modifiers,
        } => {
            let key = key_from_u32(*key);
            let text = text_from_key(&key);
            match state {
                IcedKeyState::Pressed => Some(IcedEvent::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    location: keyboard::Location::Standard,
                    modifiers: *modifiers,
                    text,
                })),
                IcedKeyState::Released => Some(IcedEvent::Keyboard(keyboard::Event::KeyReleased {
                    key,
                    location: keyboard::Location::Standard,
                    modifiers: *modifiers,
                })),
            }
        }
        LayerShellEvent::KeyBoardInput { event, .. } => {
            let logical_key = event.key_without_modifiers();
            let text = event
                .text_with_all_modifiers()
                .map(SmolStr::new)
                .filter(|text| !text.as_str().chars().any(is_private_use));
            let LayerShellKeyEvent {
                state, location, ..
            } = event;
            let key = key(logical_key);
            todo!()
        }
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
        Interaction::ResizingVertically => "ew_resize".to_owned(),
        Interaction::ResizingHorizontally => "ns_resize".to_owned(),
    }
}

fn is_private_use(c: char) -> bool {
    ('\u{E000}'..='\u{F8FF}').contains(&c)
}
