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

    fn style(&self, theme: &Self::Theme) -> iced::theme::Style {
        use iced::theme::Style;
        Style {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }
}
```

### Multi window

```rust, no_run
use std::collections::HashMap;

use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{event, Alignment, Element, Event, Length, Task as Command};
use iced_layershell::actions::{IcedNewMenuSettings, MenuDirection};
use iced_runtime::window::Action as WindowAction;
use iced_runtime::{task, Action};

use iced_layershell::build_pattern::{daemon, MainSettings};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;

pub fn main() -> Result<(), iced_layershell::Error> {
    daemon(
        Counter::namespace,
        Counter::update,
        Counter::view,
        Counter::remove_id,
    )
    .subscription(Counter::subscription)
    .settings(MainSettings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            exclusive_zone: 400,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            start_mode: StartMode::AllScreens,
            ..Default::default()
        },
        ..Default::default()
    })
    .run_with(|| Counter::new("Hello"))
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
    fn new(text: &str) -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: text.to_string(),
                ids: HashMap::new(),
            },
            Command::none(),
        )
    }

    fn id_info(&self, id: iced::window::Id) -> Option<WindowInfo> {
        self.ids.get(&id).cloned()
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        self.ids.remove(&id);
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
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
