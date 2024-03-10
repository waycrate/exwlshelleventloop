use iced::widget::{button, column, text};
use iced::{Alignment, Command, Element, Length, Theme};
use iced_layershell::actions::LayershellActions;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;
use iced_runtime::command::Action;

pub fn main() -> Result<(), iced_layershell::Error> {
    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 300)),
            exclusize_zone: 300,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            ..Default::default()
        },
        ..Default::default()
    })
}

struct Counter {
    value: i32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Application for Counter {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self { value: 0 }, Command::none())
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
            }
            Message::DecrementPressed => {
                self.value -= 1;
            }
        }
        if self.value % 2 == 0 {
            Command::batch(vec![
                Command::single(Action::Custom(Box::new(LayershellActions::AnchorChange(
                    Anchor::Left | Anchor::Top | Anchor::Bottom,
                )))),
                Command::single(Action::Custom(Box::new(LayershellActions::SizeChange((
                    300, 0,
                ))))),
            ])
        } else {
            Command::batch(vec![
                Command::single(Action::Custom(Box::new(LayershellActions::AnchorChange(
                    Anchor::Bottom | Anchor::Left | Anchor::Right,
                )))),
                Command::single(Action::Custom(Box::new(LayershellActions::SizeChange((
                    0, 300,
                ))))),
            ])
        }
    }

    fn view(&self) -> Element<Message> {
        column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .padding(20)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .into()
    }
}
