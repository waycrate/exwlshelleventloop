mod applications;
use applications::{App, all_apps};
use iced::widget::{container, row};
use iced::{Color, Element, Task};

use iced_layershell::build_pattern::application;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::LayerShellSettings;
use iced_layershell::to_layer_message;

fn main() -> iced_layershell::Result {
    application(
        || Panel::new(),
        Panel::namespace,
        Panel::update,
        Panel::view,
    )
    .layer_settings(LayerShellSettings {
        size: Some((600, 50)),
        anchor: Anchor::Bottom,
        margin: (0, 0, 10, 0),
        ..Default::default()
    })
    .style(Panel::style)
    .run()
}

struct Panel {
    apps: Vec<App>,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Launch(usize),
}

impl Panel {
    fn new() -> (Self, Task<Message>) {
        (Self { apps: all_apps() }, Task::none())
    }
    fn namespace() -> String {
        String::from("bottom panel")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Launch(index) => {
                self.apps[index].launch();
                Task::none()
            }
            _ => unreachable!(),
        }
    }

    fn view(&self) -> Element<Message> {
        let bottom_vec: Vec<Element<Message>> = self
            .apps
            .iter()
            .enumerate()
            .map(|(index, app)| app.view(index, false))
            .collect();

        let row = row(bottom_vec);
        container(row).into()
    }

    fn style(&self, theme: &iced::Theme) -> iced::theme::Style {
        use iced::theme::Style;
        Style {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }
}
