use iced::widget::{Space, button, column, text, text_input};
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_sessionlock::actions::UnLockAction;
use iced_sessionlock::application;

pub fn main() -> Result<(), iced_sessionlock::Error> {
    application(Counter::new, Counter::update, Counter::view)
        .subscription(Counter::subscription)
        .run()
}

struct Counter {
    value: i32,
    text: String,
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    IcedEvent(Event),
    UnLock,
}

impl TryInto<UnLockAction> for Message {
    type Error = Self;
    fn try_into(self) -> Result<UnLockAction, Self::Error> {
        if let Self::UnLock = self {
            return Ok(UnLockAction);
        }
        Err(self)
    }
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
            _ => unreachable!(),
        }
    }

    fn view(&self, _id: iced::window::Id) -> Element<'_, Message> {
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
