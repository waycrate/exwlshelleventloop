# The application function allow you to create the application with just some functions

```rust, no_run

use iced::widget::{button, column, row, text, text_input};
use iced::{event, Alignment, Color, Element, Event, Length, Task as Command};
use iced_layershell::build_pattern::{application, MainSettings};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode};
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

    application(namespace, update, view)
        .style(style)
        .subscription(subscription)
        .settings(MainSettings {
            layer_settings: LayerShellSettings {
                size: Some((0, 400)),
                exclusive_zone: 400,
                anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
                start_mode,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}

#[derive(Default)]
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

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    Direction(WindowDirection),
    IcedEvent(Event),
}

fn namespace(_: &Counter) -> String {
    String::from("Counter - Iced")
}

fn subscription(_: &Counter) -> iced::Subscription<Message> {
    event::listen().map(Message::IcedEvent)
}

fn update(counter: &mut Counter, message: Message) -> Command<Message> {
    match message {
        Message::IcedEvent(event) => {
            println!("hello {event:?}");
            Command::none()
        }
        Message::IncrementPressed => {
            counter.value += 1;
            Command::none()
        }
        Message::DecrementPressed => {
            counter.value -= 1;
            Command::none()
        }
        Message::TextInput(text) => {
            counter.text = text;
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

fn view(counter: &Counter) -> Element<Message> {
    let center = column![
        button("Increment").on_press(Message::IncrementPressed),
        text(counter.value).size(50),
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
            text_input("hello", &counter.text)
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

fn style(_counter: &Counter, theme: &iced::Theme) -> iced_layershell::Appearance {
    use iced_layershell::Appearance;
    Appearance {
        background_color: Color::TRANSPARENT,
        text_color: theme.palette().text,
    }
}

```
