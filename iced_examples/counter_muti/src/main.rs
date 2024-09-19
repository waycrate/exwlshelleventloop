use std::collections::HashMap;

use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{event, Alignment, Element, Event, Length, Task as Command, Theme};
use iced_layershell::actions::{
    IcedNewMenuSettings, LayershellCustomActionsWithIdAndInfo, LayershellCustomActionsWithInfo,
    MenuDirection,
};
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{task, Action};

use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::MultiApplication;
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
    IcedAction(CounterLayerActions),
}

#[derive(Debug, Clone)]
enum CounterLayerActions {
    MenuShown,
    ToLeft(Id),
    LeftSizeChange(Id),
    ToRight(Id),
    RightSizeChange(Id),
    ToBottom(Id),
    BottomSizeChange(Id),
    ToTop(Id),
    TopSizeChange(Id),
    NewWindowLeft,
    NewWindowRight,
}

type CounterLayerAction = LayershellCustomActionsWithIdAndInfo<WindowInfo>;
type CounterLayerCustonAction = LayershellCustomActionsWithInfo<WindowInfo>;

impl TryInto<CounterLayerAction> for Message {
    type Error = Self;
    fn try_into(self) -> Result<CounterLayerAction, Self::Error> {
        let Self::IcedAction(action) = self else {
            return Err(self);
        };
        match action {
            CounterLayerActions::MenuShown => Ok(CounterLayerAction::new(
                None,
                LayershellCustomActionsWithInfo::NewMenu((
                    IcedNewMenuSettings {
                        size: (100, 100),
                        direction: MenuDirection::Up,
                    },
                    WindowInfo::PopUp,
                )),
            )),
            CounterLayerActions::ToLeft(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::AnchorChange(Anchor::Left | Anchor::Top | Anchor::Bottom),
            )),
            CounterLayerActions::LeftSizeChange(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::SizeChange((400, 0)),
            )),
            CounterLayerActions::ToRight(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::AnchorChange(
                    Anchor::Right | Anchor::Top | Anchor::Bottom,
                ),
            )),
            CounterLayerActions::RightSizeChange(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::SizeChange((400, 0)),
            )),
            CounterLayerActions::ToBottom(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::AnchorChange(
                    Anchor::Bottom | Anchor::Left | Anchor::Right,
                ),
            )),
            CounterLayerActions::BottomSizeChange(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::SizeChange((0, 400)),
            )),
            CounterLayerActions::ToTop(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::AnchorChange(Anchor::Top | Anchor::Left | Anchor::Right),
            )),
            CounterLayerActions::TopSizeChange(id) => Ok(CounterLayerAction::new(
                Some(id),
                CounterLayerCustonAction::SizeChange((0, 400)),
            )),
            CounterLayerActions::NewWindowLeft => Ok(CounterLayerAction::new(
                None,
                CounterLayerCustonAction::NewLayerShell((
                    NewLayerShellSettings {
                        size: Some((100, 100)),
                        exclusive_zone: None,
                        anchor: Anchor::Left | Anchor::Bottom,
                        layer: Layer::Top,
                        margin: None,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        use_last_output: false,
                    },
                    WindowInfo::Left,
                )),
            )),
            CounterLayerActions::NewWindowRight => Ok(CounterLayerAction::new(
                None,
                CounterLayerCustonAction::NewLayerShell((
                    NewLayerShellSettings {
                        size: Some((100, 100)),
                        exclusive_zone: None,
                        anchor: Anchor::Right | Anchor::Bottom,
                        layer: Layer::Top,
                        margin: None,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        use_last_output: false,
                    },
                    WindowInfo::Right,
                )),
            )),
        }
    }
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
                text: "type something".to_string(),
                ids: HashMap::new(),
            },
            Command::none(),
        )
    }

    fn id_info(&self, id: iced::window::Id) -> Option<Self::WindowInfo> {
        self.ids.get(&id).cloned()
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
                            return iced_runtime::task::effect(Action::Window(
                                WindowAction::Close(*id),
                            ));
                        }
                    }
                    Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                        return Command::done(Message::IcedAction(CounterLayerActions::MenuShown));
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
                    Command::done(Message::IcedAction(CounterLayerActions::ToLeft(id))),
                    Command::done(Message::IcedAction(CounterLayerActions::LeftSizeChange(id))),
                ]),
                WindowDirection::Right(id) => Command::batch(vec![
                    Command::done(Message::IcedAction(CounterLayerActions::ToRight(id))),
                    Command::done(Message::IcedAction(CounterLayerActions::RightSizeChange(
                        id,
                    ))),
                ]),
                WindowDirection::Bottom(id) => Command::batch(vec![
                    Command::done(Message::IcedAction(CounterLayerActions::ToBottom(id))),
                    Command::done(Message::IcedAction(CounterLayerActions::BottomSizeChange(
                        id,
                    ))),
                ]),
                WindowDirection::Top(id) => Command::batch(vec![
                    Command::done(Message::IcedAction(CounterLayerActions::ToTop(id))),
                    Command::done(Message::IcedAction(CounterLayerActions::TopSizeChange(id))),
                ]),
            },
            Message::NewWindowLeft => {
                Command::done(Message::IcedAction(CounterLayerActions::NewWindowLeft))
            }
            Message::NewWindowRight => {
                Command::done(Message::IcedAction(CounterLayerActions::NewWindowRight))
            }
            Message::Close(id) => task::effect(Action::Window(WindowAction::Close(id))),
            _ => unreachable!(),
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
            return container(button("close PopUp").on_press(Message::Close(id)))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Color::new(0., 0.5, 0.7, 0.6).into()),
                    ..Default::default()
                })
                //.style(Container::Custom(Box::new(BlackMenu)))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            button("Decrement").on_press(Message::DecrementPressed),
            text(self.value).size(50),
            button("newwindowLeft").on_press(Message::NewWindowLeft),
            button("newwindowRight").on_press(Message::NewWindowRight),
        ]
        .align_x(Alignment::Center)
        .padding(20)
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
        //.align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
