use std::collections::HashMap;

use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_layershell::actions::{IcedNewMenuSettings, MenuDirection};
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{Action, task};

use iced_layershell::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

pub fn main() -> Result<(), iced_layershell::Error> {
    tracing_subscriber::fmt().init();
    daemon(
        || Counter::new("hello"),
        Counter::namespace,
        Counter::update,
        Counter::view,
    )
    .subscription(Counter::subscription)
    .remove_id(Counter::remove_id)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            start_mode: StartMode::AllScreens,
            ..Default::default()
        },
        ..Default::default()
    })
    .run()
}

#[derive(Debug, Default)]
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

#[to_layer_message(multi)]
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

impl Counter {
    fn new(text: &str) -> Self {
        Self {
            value: 0,
            text: text.to_string(),
            ids: HashMap::new(),
        }
    }

    fn id_info(&self, id: iced::window::Id) -> Option<WindowInfo> {
        self.ids.get(&id).cloned()
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        self.ids.remove(&id);
    }

    fn namespace() -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use iced::Event;
        use iced::keyboard;
        use iced::keyboard::key::Named;
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
                        let id = iced::window::Id::unique();
                        self.ids.insert(id, WindowInfo::PopUp);
                        return Command::done(Message::NewMenu {
                            settings: IcedNewMenuSettings {
                                size: (100, 100),
                                direction: MenuDirection::Up,
                            },
                            id,
                        });
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
                WindowDirection::Left(id) => Command::done(Message::AnchorSizeChange {
                    id,
                    anchor: Anchor::Top | Anchor::Left | Anchor::Bottom,
                    size: (400, 0),
                }),
                WindowDirection::Right(id) => Command::done(Message::AnchorSizeChange {
                    id,
                    anchor: Anchor::Top | Anchor::Right | Anchor::Bottom,
                    size: (400, 0),
                }),
                WindowDirection::Bottom(id) => Command::done(Message::AnchorSizeChange {
                    id,
                    anchor: Anchor::Left | Anchor::Right | Anchor::Bottom,
                    size: (0, 400),
                }),
                WindowDirection::Top(id) => Command::done(Message::AnchorSizeChange {
                    id,
                    anchor: Anchor::Left | Anchor::Right | Anchor::Top,
                    size: (0, 400),
                }),
            },
            Message::NewWindowLeft => {
                let id = iced::window::Id::unique();
                self.ids.insert(id, WindowInfo::Left);
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((100, 100)),
                        exclusive_zone: None,
                        anchor: Anchor::Left | Anchor::Bottom,
                        layer: Layer::Top,
                        margin: None,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        use_last_output: false,
                        ..Default::default()
                    },
                    id,
                })
            }
            Message::NewWindowRight => {
                let id = iced::window::Id::unique();
                self.ids.insert(id, WindowInfo::Right);
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((100, 100)),
                        exclusive_zone: None,
                        anchor: Anchor::Right | Anchor::Bottom,
                        layer: Layer::Top,
                        margin: None,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        use_last_output: false,
                        ..Default::default()
                    },
                    id,
                })
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
                    background: Some(iced::Color::from_rgba(0., 0.5, 0.7, 0.6).into()),
                    ..Default::default()
                })
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
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
