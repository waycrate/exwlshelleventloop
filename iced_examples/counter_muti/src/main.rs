use iced::widget::{button, column, row, text, text_input};
use iced::window::Id;
use iced::{event, Alignment, Command, Element, Event, Length, Theme};
use iced_layershell::actions::{LayershellCustomActions, LayershellCustomActionsWithId};
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::MultiApplication;
pub fn main() -> Result<(), iced_layershell::Error> {
    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusize_zone: 400,
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
    Top(Id),
    Left(Id),
    Right(Id),
    Bottom(Id),
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    Direction(WindowDirection),
    IcedEvent(Event),
}

impl MultiApplication for Counter {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type WindowInfo = ();

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
                WindowDirection::Left(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Left | Anchor::Top | Anchor::Bottom,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::SizeChange((400, 0)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Right(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Right | Anchor::Top | Anchor::Bottom,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::SizeChange((400, 0)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Bottom(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Bottom | Anchor::Left | Anchor::Right,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::SizeChange((0, 400)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Top(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Top | Anchor::Left | Anchor::Right,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId(
                            id,
                            LayershellCustomActions::SizeChange((0, 400)),
                        )
                        .into(),
                    ),
                ]),
            },
        }
    }

    fn view(&self, id: iced::window::Id) -> Element<Message> {
        //println!("{:?}, {}", _id, self.value);
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
                .on_press(Message::Direction(WindowDirection::Left(id)))
                .height(Length::Fill),
            column![
                button("top")
                    .on_press(Message::Direction(WindowDirection::Top(id)))
                    .width(Length::Fill),
                center,
                text_input("hello", &self.text)
                    .on_input(Message::TextInput)
                    .padding(10),
                button("bottom")
                    .on_press(Message::Direction(WindowDirection::Bottom(id)))
                    .width(Length::Fill),
            ]
            .width(Length::Fill),
            button("right")
                .on_press(Message::Direction(WindowDirection::Right(id)))
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
