use iced::alignment::Vertical;
use iced::widget::{
    button, center, column, container, horizontal_space, row, scrollable, text, text_input,
};
use iced::{Center, Element, Fill, Subscription, Task, Theme, event};
use iced::{Color, window};
use iced_layershell::daemon;
use iced_layershell::reexport::{Anchor, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

use std::collections::BTreeMap;

fn main() -> iced_layershell::Result {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    daemon(
        Example::new,
        "multi_window",
        Example::update,
        Example::view,
        Example::remove_id,
    )
    .theme(Example::theme)
    .style(Example::style)
    .subscription(Example::subscription)
    .scale_factor(Example::scale_factor)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

struct Example {
    windows: BTreeMap<window::Id, Window>,
}

#[derive(Debug)]
struct Window {
    title: String,
    scale_input: String,
    current_scale: f64,
    theme: Theme,
    transparent: bool,
}

#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    OpenWindow,
    CloseWindow(window::Id),
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    ScaleInputChanged(window::Id, String),
    ScaleChanged(window::Id, String),
    TitleChanged(window::Id, String),
}

impl Example {
    fn open(count: usize) -> (window::Id, Task<Message>) {
        let anchor = match count % 8 {
            0 => Anchor::Bottom,
            1 => Anchor::Bottom | Anchor::Right,
            2 => Anchor::Right,
            3 => Anchor::Right | Anchor::Top,
            4 => Anchor::Top,
            5 => Anchor::Top | Anchor::Left,
            6 => Anchor::Left,
            7 => Anchor::Left | Anchor::Bottom,
            _ => Anchor::Bottom,
        };
        let size = (480, 320);
        let id = window::Id::unique();
        (
            id,
            Task::done(Message::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(size),
                    exclusive_zone: None,
                    anchor,
                    layer: Layer::Top,
                    margin: None,
                    //keyboard_interactivity: KeyboardInteractivity::None,
                    use_last_output: false,
                    ..Default::default()
                },
                id,
            }),
        )
    }

    fn new() -> (Self, Task<Message>) {
        let (id, open) = Self::open(0);

        (
            Self {
                windows: BTreeMap::new(),
            },
            open.chain(Task::done(Message::WindowOpened(id))),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWindow => {
                let len = self.windows.len();
                let (id, open) = Self::open(len);
                open.chain(Task::done(Message::WindowOpened(id)))
            }
            Message::CloseWindow(id) => iced::window::close(id),
            Message::WindowOpened(id) => {
                let window = Window::new(self.windows.len() + 1);
                let focus_input = text_input::focus(format!("input-{id}"));

                self.windows.insert(id, window);

                focus_input
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);

                if self.windows.is_empty() {
                    iced::exit()
                } else {
                    Task::none()
                }
            }
            Message::ScaleInputChanged(id, scale) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.scale_input = scale;
                }

                Task::none()
            }
            Message::ScaleChanged(id, scale) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.current_scale = scale
                        .parse::<f64>()
                        .unwrap_or(window.current_scale)
                        .clamp(0.5, 5.0);
                }

                Task::none()
            }
            Message::TitleChanged(id, title) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.title = title;
                }

                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if let Some(window) = self.windows.get(&window_id) {
            center(window.view(window_id)).into()
        } else {
            horizontal_space().into()
        }
    }

    fn theme(&self, window: window::Id) -> Theme {
        if let Some(window) = self.windows.get(&window) {
            window.theme.clone()
        } else {
            Theme::default()
        }
    }

    fn style(&self, theme: &Theme, window: window::Id) -> iced::theme::Style {
        use iced::theme::Base;
        let mut style = theme.base();
        if let Some(window) = self.windows.get(&window) {
            if window.transparent {
                style.background_color = Color::TRANSPARENT;
            }
        }
        style
    }

    fn scale_factor(&self, window: window::Id) -> f64 {
        self.windows
            .get(&window)
            .map(|window| window.current_scale)
            .unwrap_or(1.0)
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, id| {
            tracing::debug!("event: {}, {:?}, {:?}", id, status, event);
            if let iced::Event::Window(iced::window::Event::Closed) = event {
                Some(Message::WindowClosed(id))
            } else {
                None
            }
        })
    }

    fn remove_id(&mut self, _window: window::Id) {
        // self.windows.remove(&window);
    }
}

impl Window {
    fn new(count: usize) -> Self {
        Self {
            title: format!("Window_{}", count),
            scale_input: "1.0".to_string(),
            current_scale: 1.0,
            theme: Theme::ALL[count % Theme::ALL.len()].clone(),
            transparent: count % 2 == 0,
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        let scale_input = column![
            text("Window scale factor:"),
            text_input("Window Scale", &self.scale_input)
                .on_input(move |msg| { Message::ScaleInputChanged(id, msg) })
                .on_submit(Message::ScaleChanged(id, self.scale_input.to_string()))
        ];

        let title_input = column![
            text("Window title:"),
            text_input("Window Title", &self.title)
                .on_input(move |msg| { Message::TitleChanged(id, msg) })
                .id(format!("input-{id}"))
        ];

        let new_window_button = button(text("New Window")).on_press(Message::OpenWindow);

        let close_window_button = button(text("Close")).on_press(Message::CloseWindow(id));

        let content = scrollable(
            column![
                scale_input,
                title_input,
                row![new_window_button, close_window_button]
                    .spacing(10)
                    .align_y(Vertical::Center)
            ]
            .spacing(50)
            .width(Fill)
            .align_x(Center),
        );

        container(content).center_x(200).into()
    }
}
