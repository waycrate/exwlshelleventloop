//! xdg-foreign example.
//!
//! Opens a window that exports itself and shows its handle, with a button that opens a child
//! window parented to it through that handle.
use std::collections::BTreeMap;

use iced::widget::{button, column, container, text};
use iced::window::Id;
use iced::{Element, Length, Task};
use iced_exwlshell::actions::IcedXdgWindowSettings;
use iced_exwlshell::daemon;
use iced_exwlshell::reexport::{ForeignHandleSink, ForeignToplevelHandle};
use iced_exwlshell::settings::{ExWlSettings, LayerShellSettings, StartMode};
use iced_exwlshell::to_exwlshell_message;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};

fn main() -> Result<(), iced_exwlshell::Error> {
    let toplevel_id = std::env::args().nth(1);
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();
    daemon(
        move || App::new(shell_events.clone(), toplevel_id.clone()),
        || "xdg_foreign".to_owned(),
        App::update,
        App::view,
    )
    .subscription(App::subscription)
    .wl_settings(ExWlSettings {
        layer_settings: LayerShellSettings {
            start_mode: StartMode::Background,
            ..Default::default()
        },
        shell_broadcast,
        ..Default::default()
    })
    .run()
}

struct App {
    windows: BTreeMap<Id, String>,
    exported: Option<(Id, ForeignToplevelHandle)>,
    shell_events: ShellReceiver,
}

#[to_exwlshell_message]
#[derive(Debug, Clone)]
enum Message {
    OpenChild,
    Shell(ShellEvent),
    Closed(Id),
}

impl App {
    fn new_foreign_child(
        shell_events: ShellReceiver,
        toplevel_id: String,
    ) -> (Self, Task<Message>) {
        let (id, open) = Message::base_window_open(IcedXdgWindowSettings {
            foreign_parent: Some(ForeignToplevelHandle::new(&toplevel_id)),
            ..Default::default()
        });
        let app = Self {
            windows: BTreeMap::from([(id, format!("parent requested: {toplevel_id}"))]),
            exported: None,
            shell_events,
        };
        (app, open)
    }
    fn new(shell_events: ShellReceiver, toplevel_id: Option<String>) -> (Self, Task<Message>) {
        if let Some(toplevel_id) = toplevel_id {
            return Self::new_foreign_child(shell_events, toplevel_id);
        }
        let (id, open) = Message::base_window_open(IcedXdgWindowSettings::default());
        let app = Self {
            windows: BTreeMap::from([(id, "exporting...".to_owned())]),
            exported: None,
            shell_events,
        };
        let export = Task::done(Message::ExportToplevel {
            id,
            sink: ForeignHandleSink::none(),
        });
        (app, open.chain(export))
    }

    fn open_child(&mut self, parent: ForeignToplevelHandle) -> Task<Message> {
        let (id, open) = Message::base_window_open(IcedXdgWindowSettings {
            foreign_parent: Some(parent.clone()),
            ..Default::default()
        });
        self.windows
            .insert(id, format!("parent requested: {parent}"));
        open
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced::window::close_events().map(Message::Closed),
            self.shell_events.listen().map(Message::Shell),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenChild => {
                if let Some((_, handle)) = self.exported.clone() {
                    return self.open_child(handle);
                }
            }
            Message::Shell(ShellEvent::ToplevelExported { window, handle }) => {
                self.windows.insert(window, format!("handle: {handle}"));
                println!("id = {}", handle.as_str());
                self.exported = Some((window, handle));
            }
            Message::Shell(ShellEvent::ForeignParentDestroyed { window }) => {
                self.windows.insert(
                    window,
                    "parent destroyed by compositor: invalid handle, or the exporter revoked it"
                        .to_owned(),
                );
            }
            Message::Closed(id) => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    return iced::exit();
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self, id: Id) -> Element<'_, Message> {
        let mut content =
            column![text(self.windows.get(&id).map_or("", String::as_str))].spacing(10);
        if self
            .exported
            .as_ref()
            .is_some_and(|(exporter, _)| *exporter == id)
        {
            content = content.push(button("open child window").on_press(Message::OpenChild));
        }
        container(content)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
