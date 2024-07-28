use applications::{all_apps, App};
use iced::widget::{column, scrollable, text_input};
use iced::{event, Command, Element, Event, Length, Theme};
mod applications;
use iced::window::Id;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;
use iced_runtime::command::Action;
use iced_runtime::window::Action as WindowAction;

use once_cell::sync::Lazy;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

fn main() -> Result<(), iced_layershell::Error> {
    Launcher::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((1000, 1000)),
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right | Anchor::Top,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
        ..Default::default()
    })?;
    std::thread::sleep(std::time::Duration::from_millis(1));
    Ok(())
}

struct Launcher {
    text: String,
    apps: Vec<App>,
    scrollpos: usize,
}

#[derive(Debug, Clone)]
enum Message {
    SearchEditChanged(String),
    SearchSubmit,
    Launch(usize),
    IcedEvent(Event),
}

impl Application for Launcher {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                text: "".to_string(),
                apps: all_apps(),
                scrollpos: 0,
            },
            text_input::focus(INPUT_ID.clone()),
        )
    }

    fn namespace(&self) -> String {
        String::from("iced_launcer")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        event::listen().map(Message::IcedEvent)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use iced_runtime::keyboard;
        use keyboard::key::Named;
        match message {
            Message::SearchSubmit => {
                let re = regex::Regex::new(&self.text).ok();
                let index = self
                    .apps
                    .iter()
                    .enumerate()
                    .filter(|(_, app)| {
                        if re.is_none() {
                            return true;
                        }
                        let re = re.as_ref().unwrap();

                        re.is_match(app.title().to_lowercase().as_str())
                            || re.is_match(app.description().to_lowercase().as_str())
                    })
                    .enumerate()
                    .find(|(index, _)| *index == self.scrollpos);
                if let Some((_, (_, app))) = index {
                    app.launch();
                    Command::single(Action::Window(WindowAction::Close(Id::MAIN)))
                } else {
                    Command::none()
                }
            }
            Message::SearchEditChanged(edit) => {
                self.scrollpos = 0;
                self.text = edit;
                Command::none()
            }
            Message::Launch(index) => {
                self.apps[index].launch();
                Command::single(Action::Window(WindowAction::Close(Id::MAIN)))
            }
            Message::IcedEvent(event) => {
                let mut len = self.apps.len();

                let re = regex::Regex::new(&self.text).ok();
                if let Some(re) = re {
                    len = self
                        .apps
                        .iter()
                        .filter(|app| {
                            re.is_match(app.title().to_lowercase().as_str())
                                || re.is_match(app.description().to_lowercase().as_str())
                        })
                        .count();
                }
                if let Event::Keyboard(keyboard::Event::KeyReleased { key, .. })
                | Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
                {
                    match key {
                        keyboard::Key::Named(Named::ArrowUp) => {
                            if self.scrollpos == 0 {
                                return Command::none();
                            }
                            self.scrollpos -= 1;
                        }
                        keyboard::Key::Named(Named::ArrowDown) => {
                            if self.scrollpos >= len - 1 {
                                return Command::none();
                            }
                            self.scrollpos += 1;
                        }
                        keyboard::Key::Named(Named::Escape) => {
                            return Command::single(Action::Window(WindowAction::Close(Id::MAIN)));
                        }
                        _ => {}
                    }
                }
                text_input::focus(INPUT_ID.clone())
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let re = regex::Regex::new(&self.text).ok();
        let text_ip: Element<Message> = text_input("put the launcher name", &self.text)
            .padding(10)
            .on_input(Message::SearchEditChanged)
            .on_submit(Message::SearchSubmit)
            .id(INPUT_ID.clone())
            .into();
        let buttom_vec: Vec<Element<Message>> = self
            .apps
            .iter()
            .enumerate()
            .filter(|(_, app)| {
                if re.is_none() {
                    return true;
                }
                let re = re.as_ref().unwrap();

                re.is_match(app.title().to_lowercase().as_str())
                    || re.is_match(app.description().to_lowercase().as_str())
            })
            .enumerate()
            .filter(|(index, _)| *index >= self.scrollpos)
            .map(|(filter_index, (index, app))| app.view(index, filter_index == self.scrollpos))
            .collect();
        let buttom: Element<Message> = scrollable(column(buttom_vec).width(Length::Fill))
            .id(SCROLLABLE_ID.clone())
            .into();
        column![text_ip, buttom].into()
    }
}
