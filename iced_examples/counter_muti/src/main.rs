use std::collections::HashMap;

use iced::widget::{button, column, row, text_input};
use iced::window::Id;
use iced::{event, Alignment, Command, Element, Event, Length, Theme};
use iced_layershell::actions::{
    LayershellCustomActions, LayershellCustomActionsWithId, LayershellCustomActionsWithIdAndInfo,
    LayershellCustomActionsWithInfo, MenuDirection, NewMenuSettings,
};
use iced_runtime::command::Action;
use iced_runtime::window::Action as WindowAction;

use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
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
    ids: HashMap<iced::window::Id, WindowInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowInfo {
    Left,
    Right,
    PopUp,
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
    NewWindowLeft,
    NewWindowRight,
    Close(Id),
    TextInput(String),
    Direction(WindowDirection),
    IcedEvent(Event),
}

impl Counter {
    fn window_id(&self, info: &WindowInfo) -> Option<&iced::window::Id> {
        for (k, v) in self.ids.iter() {
            if info == v {
                return Some(k);
            }
        }
        None
    }
}

impl MultiApplication for Counter {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type WindowInfo = WindowInfo;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: "eee".to_string(),
                ids: HashMap::new(),
            },
            Command::none(),
        )
    }

    fn id_info(&self, id: iced::window::Id) -> Option<&Self::WindowInfo> {
        self.ids.get(&id)
    }

    fn set_id_info(&mut self, id: iced::window::Id, info: Self::WindowInfo) {
        self.ids.insert(id, info);
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        self.ids.remove(&id);
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use iced::keyboard;
        use iced::keyboard::key::Named;
        use iced::Event;
        match message {
            Message::IcedEvent(event) => {
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(Named::Escape),
                        ..
                    }) => {
                        if let Some(id) = self.window_id(&WindowInfo::Left) {
                            return Command::single(Action::Window(WindowAction::Close(*id)));
                        }
                    }
                    Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                        return Command::single(
                            LayershellCustomActionsWithIdAndInfo::new(
                                iced::window::Id::MAIN,
                                LayershellCustomActionsWithInfo::NewMenu((
                                    NewMenuSettings {
                                        size: (100, 100),
                                        direction: MenuDirection::Up,
                                    },
                                    WindowInfo::PopUp,
                                )),
                            )
                            .into(),
                        );
                    }
                    _ => {}
                }
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
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Left | Anchor::Top | Anchor::Bottom,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::SizeChange((400, 0)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Right(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Right | Anchor::Top | Anchor::Bottom,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::SizeChange((400, 0)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Bottom(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::AnchorChange(
                                Anchor::Bottom | Anchor::Left | Anchor::Right,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActionsWithInfo::SizeChange((0, 400)),
                        )
                        .into(),
                    ),
                ]),
                WindowDirection::Top(id) => Command::batch(vec![
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActionsWithInfo::AnchorChange(
                                Anchor::Top | Anchor::Left | Anchor::Right,
                            ),
                        )
                        .into(),
                    ),
                    Command::single(
                        LayershellCustomActionsWithId::new(
                            id,
                            LayershellCustomActions::SizeChange((0, 400)),
                        )
                        .into(),
                    ),
                ]),
            },
            Message::NewWindowLeft => Command::single(
                LayershellCustomActionsWithIdAndInfo::new(
                    iced::window::Id::MAIN,
                    LayershellCustomActionsWithInfo::NewLayerShell((
                        NewLayerShellSettings {
                            size: Some((100, 100)),
                            exclusize_zone: None,
                            anchor: Anchor::Left | Anchor::Bottom,
                            layer: Layer::Top,
                            margins: None,
                            keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        },
                        WindowInfo::Left,
                    )),
                )
                .into(),
            ),
            Message::NewWindowRight => Command::single(
                LayershellCustomActionsWithIdAndInfo::new(
                    iced::window::Id::MAIN,
                    LayershellCustomActionsWithInfo::NewLayerShell((
                        NewLayerShellSettings {
                            size: Some((100, 100)),
                            exclusize_zone: None,
                            anchor: Anchor::Right | Anchor::Bottom,
                            layer: Layer::Top,
                            margins: None,
                            keyboard_interactivity: KeyboardInteractivity::None,
                        },
                        WindowInfo::Right,
                    )),
                )
                .into(),
            ),
            Message::Close(id) => Command::single(Action::Window(WindowAction::Close(id))),
        }
    }

    fn view(&self, id: iced::window::Id) -> Element<Message> {
        if let Some(WindowInfo::Left) = self.id_info(id) {
            return button("close left").on_press(Message::Close(id)).into();
        }
        if let Some(WindowInfo::Right) = self.id_info(id) {
            return button("close right").on_press(Message::Close(id)).into();
        }
        if let Some(WindowInfo::PopUp) = self.id_info(id) {
            return button("close PopUp").on_press(Message::Close(id)).into();
        }
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            button("Decrement").on_press(Message::DecrementPressed),
            button("newwindowLeft").on_press(Message::NewWindowLeft),
            button("newwindowRight").on_press(Message::NewWindowRight),
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
