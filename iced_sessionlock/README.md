# sessionlock binding for iced

[![Crates.io](https://img.shields.io/crates/v/iced-sessionlock.svg)](https://crates.io/crates/iced-sessionlock)

iced-layershell provides binding for iced and sessionlock.

Session lock is the wayland protocol for lock. This protocol is supported in river, sway and etc. We use it make a beautiful lock program in [twenty](https://github.com/waycrate/twenty). You can also use it to build your sessionlock. This will become very easy to use our crate with pam crate.

The smallest example is like

```rust, no_run
use iced::widget::{Space, button, column, text, text_input};
use iced::{Alignment, Element, Event, Length, Task as Command, event};
use iced_sessionlock::actions::UnLockAction;
use iced_sessionlock::application;
use iced_sessionlock::settings::Settings;
use iced_sessionlock::to_session_message;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};

pub fn main() -> Result<(), iced_sessionlock::Error> {
    // The runtime gets the sending end, the application keeps the receiving one.
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

    application(
        move || Counter::new(shell_events.clone()),
        Counter::update,
        Counter::view,
    )
    .subscription(Counter::subscription)
    .settings(Settings {
        shell_broadcast,
        ..Default::default()
    })
    .run()
}

struct Counter {
    value: i32,
    text: String,
    shell_events: ShellReceiver,
}

#[to_session_message]
#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    TextInput(String),
    IcedEvent(Event),
    LockSurface(iced::window::Id, Option<iced_wayland_subscriber::OutputInfo>),
}

impl Counter {
    fn new(shell_events: ShellReceiver) -> (Self, Command<Message>) {
        (
            Self {
                value: 0,
                text: "eee".to_string(),
                shell_events,
            },
            Command::none(),
        )
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch(vec![
            event::listen().map(Message::IcedEvent),
            // A lock surface exists per monitor and the runtime picks their
            // ids, so this is how the application learns what to draw where.
            self.shell_events.listen().filter_map(|event| match event {
                ShellEvent::WindowOutputChanged { window, output } => {
                    Some(Message::LockSurface(window, output))
                }
                _ => None,
            }),
        ])
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
            Message::LockSurface(id, output) => {
                println!("lock surface {id:?} covers {:?}", output.map(|o| o.name));
                Command::none()
            }
            Message::UnLock => Command::done(message),
        }
    }

    fn view(&self, _id: iced::window::Id) -> Element<Message> {
        column![
            Space::new().height(Length::Fill),
            button("Increment").on_press(Message::IncrementPressed),
            button("Lock").on_press(Message::UnLock),
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

## Knowing the lock surfaces

A lock surface is created for every connected monitor, and the runtime picks
their `iced::window::Id`s itself, the application never supplies them. The
example above hands `Settings::shell_broadcast` the sending end of
`iced_wayland_subscriber::shell::channel()` and keeps the receiver, which is how
it finds out what exists.

`WindowOutputChanged` is what makes per-monitor lock UI possible: it names the
monitor each lock surface covers, so `view(id)` can draw something different on
each screen. A lock surface never moves between monitors, but the monitor's own
info is kept current, so the entry stays accurate across a mode or scale change.
`ShellEvent::NewShell` reports the same surfaces without the monitor, and its
`shell` is always `ShellType::SessionLock` here.

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
