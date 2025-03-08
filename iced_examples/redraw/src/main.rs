use iced::Border;
use iced::Event;
use iced::Length;
use iced::Rectangle;
use iced::Shadow;
use iced::Size;
use iced::advanced::Clipboard;
use iced::advanced::Layout;
use iced::advanced::Shell;
use iced::advanced::Widget;
use iced::advanced::layout::Limits;
use iced::advanced::layout::Node;
use iced::advanced::renderer;
use iced::advanced::widget::Tree;
use iced::advanced::widget::tree::State;
use iced::advanced::widget::tree::Tag;
use iced::mouse::Cursor;
use iced::widget::container;
use iced::{Color, window};
use iced::{Element, Task};
use std::time::{Duration, Instant};

use iced_layershell::build_pattern::application;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;

fn main() -> iced_layershell::Result {
    application("Example", Panel::update, Panel::view)
        .layer_settings(LayerShellSettings {
            size: Some((600, 50)),
            anchor: Anchor::empty(),
            ..Default::default()
        })
        .run_with(Panel::new)
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {}

struct Panel;

impl Panel {
    fn new() -> (Self, Task<Message>) {
        (Self, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        container(LoadingBar::default()).into()
    }
}

pub struct LoadingBar {
    width: Length,
    segment_width: f32,
    height: Length,
    rate: Duration,
}

impl Default for LoadingBar {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            segment_width: 200.0,
            height: Length::Fixed(50.0),
            rate: Duration::from_secs_f32(1.0),
        }
    }
}

struct LoadingBarState {
    last_update: Instant,
    t: f32,
}

fn is_visible(bounds: &Rectangle) -> bool {
    bounds.width > 0.0 && bounds.height > 0.0
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for LoadingBar
where
    Renderer: renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            Size::new(f32::INFINITY, f32::INFINITY),
        ))
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !is_visible(&bounds) {
            return;
        }

        let position = bounds.position();
        let size = bounds.size();
        let state = state.state.downcast_ref::<LoadingBarState>();

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: position.x + (size.width * state.t * 1.3) - self.segment_width,
                    y: position.y,
                    width: self.segment_width,
                    height: size.height,
                },
                border: Border::default(),
                shadow: Shadow::default(),
            },
            Color::from_rgba(1.0, 0.0, 0.0, 1.0),
        );
    }

    fn tag(&self) -> Tag {
        Tag::of::<LoadingBarState>()
    }

    fn state(&self) -> State {
        State::new(LoadingBarState {
            last_update: Instant::now(),
            t: 0.0,
        })
    }

    fn update(
        &mut self,
        state: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            if is_visible(&bounds) {
                let state = state.state.downcast_mut::<LoadingBarState>();
                let duration = (*now - state.last_update).as_secs_f32();
                let increment = if self.rate == Duration::ZERO {
                    0.0
                } else {
                    duration * 1.0 / self.rate.as_secs_f32()
                };

                state.t += increment;

                if state.t > 1.0 {
                    state.t -= 1.0;
                }

                shell.request_redraw();
                state.last_update = *now;
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<LoadingBar> for Element<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer + 'a,
{
    fn from(spinner: LoadingBar) -> Self {
        Self::new(spinner)
    }
}
