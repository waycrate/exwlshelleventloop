# layershell binding for iced

[![Crates.io](https://img.shields.io/crates/v/iced-layershell.svg)](https://crates.io/crates/iced-layershell)

iced-layershell provides binding for iced and layershell.

## Feature:

- support to open new layershell and support popup window.
- support ext-virtual-keyboard

With this crate, you can use iced to build your kde-shell, notification application, and etc.

## Example

### Single Window iced_layershell
The smallest example is like

```rust, no_run

use iced::widget::{button, column, row, text, text_input};
use iced::{Alignment, Color, Element, Event, Length, Task as Command, event};
use iced_layershell::application;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, StartMode, Settings};
use iced_layershell::to_layer_message;

pub fn main() -> Result<(), iced_layershell::Error> {
    let binded_output_name = std::env::args().nth(1);
    let start_mode = match binded_output_name {
        Some(output) => StartMode::TargetScreen(output),
        None => StartMode::Active,
    };

    application(|| Counter::default(), namespace, update, view)
        .style(style)
        .subscription(subscription)
        .settings(Settings {
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

fn namespace() -> String {
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

fn style(_counter: &Counter, theme: &iced::Theme) -> iced::theme::Style {
    use iced::theme::Style;
    Style {
        background_color: Color::TRANSPARENT,
        text_color: theme.palette().text,
    }
}

```

# Input Regions
You can define which regions of your window receive input events and which parts are transparent to these events by using WlRegion in SetInputRegion message call.
```rust, ignore
Message::SetInputRegion(ActionCallback::new(|region| {
    region.add(0, 0, 400, 400);
    region.subtract(0, 0, 400, 60);
}))
```
view the full example [here](https://github.com/waycrate/exwlshelleventloop/tree/master/iced_layershell/examples/input_regions.rs)

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
