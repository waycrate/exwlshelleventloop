mod applications;
use applications::{all_apps, App};
use iced::widget::row;
use iced::{Command, Element, Theme};

use iced_layershell::reexport::{Anchor, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;

fn main() -> Result<(), iced_layershell::Error> {
    Panel::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((600, 50)),
            anchor: Anchor::Bottom,
            layer: Layer::Overlay,
            margin: (0, 0, 10, 0),
            ..Default::default()
        },
        ..Default::default()
    })?;
    Ok(())
}

struct Panel {
    apps: Vec<App>,
}

#[derive(Debug, Clone)]
enum Message {
    Launch(usize),
}

impl Application for Panel {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self { apps: all_apps() }, Command::none())
    }
    fn namespace(&self) -> String {
        String::from("bottom panel")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Launch(index) => {
                self.apps[index].launch();
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let bottom_vec: Vec<Element<Message>> = self
            .apps
            .iter()
            .enumerate()
            .map(|(index, app)| app.view(index, false))
            .collect();

        row(bottom_vec).into()
    }
}
