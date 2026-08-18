# the sessionlock application allow you to create a sessionlock

This application will create the lock application, which view all be all the same on all screens. If you want to control every screens, use the daemon instead.

It is the replacement for iced_sessionlock.

```rust, no_run
use iced::widget::{Space, button, column, text, text_input};
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_exwlshell::sessionlock::application;
use iced_exwlshell::to_sessionlock_message;

pub fn main() -> Result<(), iced_exwlshell::Error> {
    application(Counter::new, "lock", Counter::update, Counter::view)
        .subscription(Counter::subscription)
        .run()
}

struct Counter {
    value: i32,
    text: String,
}

#[to_sessionlock_message]
#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    IcedEvent(Event),
    UnLock,
}

impl Counter {
    fn new() -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: "lock".to_string(),
            },
            Command::none(),
        )
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IcedEvent(_event) => Command::none(),
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
            Message::UnLock => iced::exit(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            Space::new().height(Length::Fill),
            button("Increment").on_press(Message::IncrementPressed),
            button("UnLock").on_press(Message::UnLock),
            text(self.value).size(50),
            text_input("hello", &self.text)
                .on_input(Message::TextInput)
                .padding(10),
            button("Decrement").on_press(Message::DecrementPressed),
            Space::new().height(Length::Fill),
        ]
        .padding(20)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
```
