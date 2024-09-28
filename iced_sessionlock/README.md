# sessionlock binding for iced

[![Crates.io](https://img.shields.io/crates/v/iced-sessionlock.svg)](https://crates.io/crates/iced-sessionlock)

iced-layershell provides binding for iced and sessionlock.

Session lock is the wayland protocol for lock. This protocol is supported in river, sway and etc. We use it make a beautiful lock program in [twenty](https://github.com/waycrate/twenty). You can also use it to build your sessionlock. This will become very easy to use our crate with pam crate.

The smallest example is like

```rust
use iced::widget::{button, column, text, text_input, Space};
use iced::{event, Alignment, Element, Event, Length, Task as Command, Theme};

use iced_sessionlock::actions::UnLockAction;
use iced_sessionlock::settings::Settings;
use iced_sessionlock::MultiApplication;
use iced_sessionlock::to_session_message;

pub fn main() -> Result<(), iced_sessionlock::Error> {
    Counter::run(Settings::default())
}

struct Counter {
    value: i32,
    text: String,
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    IcedEvent(Event),
}

impl MultiApplication for Counter {
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
            Message::UnLock => Command::done(message),
        }
    }

    fn view(&self, _id: iced::window::Id) -> Element<Message> {
        column![
            Space::with_height(Length::Fill),
            button("Increment").on_press(Message::IncrementPressed),
            button("Lock").on_press(Message::UnLock),
            text(self.value).size(50),
            text_input("hello", &self.text)
                .on_input(Message::TextInput)
                .padding(10),
            button("Decrement").on_press(Message::DecrementPressed),
            Space::with_height(Length::Fill),
        ]
        .padding(20)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

```

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)

