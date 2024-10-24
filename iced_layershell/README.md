# layershell binding for iced

[![Crates.io](https://img.shields.io/crates/v/iced-layershell.svg)](https://crates.io/crates/iced-layershell)

iced-layershell provides binding for iced and layershell.

## Feature:

- support to open new layershell and support popup window.
- support ext-virtual-keyboard

With this crate, you can use iced to build your kde-shell, notification application, and etc.

## Example

The smallest example is like

```rust, no_run
use iced::widget::{button, column, row, text, text_input};
use iced::{event, Alignment, Color, Element, Event, Length, Task as Command, Theme};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::Application;
use iced_layershell::to_layer_message;

pub fn main() -> Result<(), iced_layershell::Error> {
    let args: Vec<String> = std::env::args().collect();

    let mut binded_output_name = None;
    if args.len() >= 2 {
        binded_output_name = Some(args[1].to_string())
    }

    let start_mode = match binded_output_name {
        Some(output) => StartMode::TargetScreen(output),
        None => StartMode::Active,
    };

    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            start_mode,
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

// Because new iced delete the custom command, so now we make a macro crate to generate
// the Command
#[to_layer_message]
#[derive(Debug, Clone)]
#[doc = "Some docs"]
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
                text: "hello, write something here".to_string(),
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
                WindowDirection::Left => Command::done(Message::AnchorSizeChange(
                    Anchor::Left | Anchor::Top | Anchor::Bottom,
                    (400, 0),
                )),
                WindowDirection::Right => Command::done(Message::AnchorSizeChange(
                    Anchor::Right | Anchor::Top | Anchor::Bottom,
                    (400, 0),
                )),
                WindowDirection::Bottom => Command::done(Message::AnchorSizeChange(
                    Anchor::Bottom | Anchor::Left | Anchor::Right,
                    (0, 400),
                )),
                WindowDirection::Top => Command::done(Message::AnchorSizeChange(
                    Anchor::Top | Anchor::Left | Anchor::Right,
                    (0, 400),
                )),
            },
            _ => unreachable!(),
        }
    }

    fn view(&self) -> Element<Message> {
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .align_x(Alignment::Center)
        .padding(20)
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
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn style(&self, theme: &Self::Theme) -> iced_layershell::Appearance {
        use iced_layershell::Appearance;
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }
}
```

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
