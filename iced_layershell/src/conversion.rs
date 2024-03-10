use crate::event::IcedButtonState;
use crate::event::WindowEvent as LayerShellEvent;
use iced_core::{mouse, Event as IcedEvent};

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
        _ => None,
    }
}

pub(crate) fn mouse_interaction(
    interaction: mouse::Interaction
) -> String {
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
        Interaction::ResizingHorizontally => "ns_resize".to_owned()
    }
}
