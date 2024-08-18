# layershell binding for iced

[![Crates.io](https://img.shields.io/crates/v/iced-layershell.svg)](https://crates.io/crates/iced-layershell)

iced-layershell provides binding for iced and layershell.

## Feature:

- support to open new layershell and support popup window.
- support ext-virtual-keyboard

With this crate, you can use iced to build your kde-shell, notification application, and etc.

## Example

The smallest example is like

```rust
use iced::widget::{button, column, row, text, text_input};
use iced::{event, Alignment, Command, Element, Event, Length, Theme};
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;

pub fn main() -> Result<(), iced_layershell::Error> {
    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            ..Default::default()
        },
        ..Default::default()
    })
}

struct Counter {
    value: i32,
    text: String,
}

#[derive(Debug, Clone, Copy)]
enum WindowDirection {
    Top,
    Left,
    Right,
    Bottom,
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    Direction(WindowDirection),
    IcedEvent(Event),
}

impl Application for Counter {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: "eee".to_string(),
            },
            Command::none(),
        )
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IcedEvent(event) => {
                println!("hello {event:?}");
                Command::none()
            }
            Message::IncrementPressed => {
                self.value += 1;
                Command::none()
            }
            Message::DecrementPressed => {
                self.value -= 1;
                Command::none()
            }
            Message::TextInput(text) => {
                self.text = text;
                Command::none()
            }
            Message::Direction(direction) => match direction {
                WindowDirection::Left => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Left | Anchor::Top | Anchor::Bottom,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((400, 0)).into()),
                ]),
                WindowDirection::Right => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Right | Anchor::Top | Anchor::Bottom,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((400, 0)).into()),
                ]),
                WindowDirection::Bottom => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Bottom | Anchor::Left | Anchor::Right,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((0, 400)).into()),
                ]),
                WindowDirection::Top => Command::batch(vec![
                    Command::single(
                        LayershellCustomActions::AnchorChange(
                            Anchor::Top | Anchor::Left | Anchor::Right,
                        )
                        .into(),
                    ),
                    Command::single(LayershellCustomActions::SizeChange((0, 400)).into()),
                ]),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .padding(20)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);
        row![
            button("left")
                .on_press(Message::Direction(WindowDirection::Left))
                .height(Length::Fill),
            column![
                button("top")
                    .on_press(Message::Direction(WindowDirection::Top))
                    .width(Length::Fill),
                center,
                text_input("hello", &self.text)
                    .on_input(Message::TextInput)
                    .padding(10),
                button("bottom")
                    .on_press(Message::Direction(WindowDirection::Bottom))
                    .width(Length::Fill),
            ]
            .width(Length::Fill),
            button("right")
                .on_press(Message::Direction(WindowDirection::Right))
                .height(Length::Fill),
        ]
        .padding(20)
        .spacing(10)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

```

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
