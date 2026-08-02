use iced::widget::{button, row, text};
use iced::{Element, Length, Task as Command};
use iced_layershell::application;
use iced_layershell::reexport::{Anchor, Layer, LayerSize};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;
use iced_wayland_subscriber::OutputId;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};
use iced_wayland_subscriber::workspace::{
    GroupCapabilities, State, Workspace, WorkspaceCapabilities, WorkspaceEvent, WorkspaceGroup,
    WorkspaceId, WorkspaceSnapshot,
};
use wayland_client::Connection;

/// Example to show even dead(unswitchable) slots
const SLOTS: std::ops::RangeInclusive<u32> = 1..=10;

pub fn main() -> Result<(), iced_layershell::Error> {
    tracing_subscriber::fmt().init();
    let connection = Connection::connect_to_env().unwrap();
    let subscriber_connection = connection.clone();
    // The runtime gets the sending end, bar keeps the receiving one.
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

    application(
        move || Bar::new(subscriber_connection.clone(), shell_events.clone()),
        Bar::namespace,
        Bar::update,
        Bar::view,
    )
    .subscription(Bar::subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: LayerSize::fill_width(30),
            exclusive_zone: 30,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            layer: Layer::Top,
            start_mode: StartMode::Active,
            ..Default::default()
        },
        with_connection: Some(connection.into()),
        shell_broadcast,
        ..Default::default()
    })
    .run()
}

struct Bar {
    connection: Connection,
    shell_events: ShellReceiver,
    workspaces: Option<std::sync::Arc<WorkspaceSnapshot>>,
    output: Option<OutputId>,
    status: String,
}

enum Slot {
    Active,
    Hidden,
    Present,
    Missing,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    WorkspacesUpdated(std::sync::Arc<WorkspaceSnapshot>),
    Unsupported,
    Finished,
    Activate(WorkspaceId),
    Create(String),
    OutputChanged(Option<OutputId>),
    Stopped(String),
}

impl Bar {
    fn new(connection: Connection, shell_events: ShellReceiver) -> Self {
        Self {
            connection,
            shell_events,
            workspaces: None,
            output: None,
            status: String::from("waiting for compositor"),
        }
    }

    fn namespace() -> String {
        String::from("workspace_bar")
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced_wayland_subscriber::workspace::listen(self.connection.clone()).map(|event| {
                match event {
                    WorkspaceEvent::Updated(snapshot) => Message::WorkspacesUpdated(snapshot),
                    WorkspaceEvent::Unsupported => Message::Unsupported,
                    WorkspaceEvent::Finished => Message::Finished,
                    WorkspaceEvent::Stop(error) => Message::Stopped(error.to_string()),
                }
            }),
            // Tells us which monitor the bar landed on.
            self.shell_events.listen().filter_map(|event| match event {
                ShellEvent::WindowOutputChanged { output, .. } => {
                    Some(Message::OutputChanged(output.as_ref().map(OutputId::from)))
                }
                _ => None,
            }),
        ])
    }

    /// The group covering this bar's output. Compositors are free to give every
    /// output its own group, so this is what is reachable from this monitor.
    fn group(&self) -> Option<&WorkspaceGroup> {
        self.workspaces.as_ref()?.group_for_output(self.output?)
    }

    /// The workspace occupying `slot`, if the compositor has one.
    fn occupant(&self, slot: u32) -> Option<&Workspace> {
        let snapshot = self.workspaces.as_ref()?;
        let group = self.group()?;
        let slot = slot.to_string();
        snapshot
            .workspaces_in(&group.id)
            .find(|workspace| workspace.name == slot)
    }

    fn describe(&self) -> String {
        if self.output.is_none() {
            return String::from("locating monitor");
        }
        if self.group().is_none() {
            return String::from("no workspace group on this monitor");
        }
        let live = SLOTS.filter(|slot| self.occupant(*slot).is_some()).count();
        format!("{live}/{} live", SLOTS.count())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WorkspacesUpdated(snapshot) => {
                self.workspaces = Some(snapshot);
                self.status = self.describe();
            }
            Message::OutputChanged(output) => {
                self.output = output;
                if self.workspaces.is_some() {
                    self.status = self.describe();
                }
            }
            Message::Unsupported => {
                self.status = String::from("compositor has no ext-workspace-v1");
            }
            Message::Finished => {
                // The manager is destroyed, so every handle is inert on an empty tree.
                self.workspaces = None;
                self.status = String::from("compositor ended ext-workspace-v1");
            }
            Message::Activate(id) => {
                let Some(snapshot) = self.workspaces.as_ref() else {
                    self.status = String::from("no workspaces yet");
                    return Command::none();
                };
                self.status = match snapshot.activate(&id) {
                    // The compositor confirms (or refuses) asynchronously,
                    // through the next `Updated` snapshot.
                    Ok(()) => String::from("activate requested"),
                    Err(error) => format!("activate failed: {error}"),
                };
            }
            Message::Create(name) => {
                let Some(group) = self.group().map(|group| group.id.clone()) else {
                    self.status = String::from("no group to create in");
                    return Command::none();
                };
                let snapshot = self
                    .workspaces
                    .as_ref()
                    .expect("a group implies a snapshot");
                self.status = match snapshot.create_workspace(&group, &name) {
                    Ok(()) => format!("create {name} requested"),
                    Err(error) => format!("create failed: {error}"),
                };
            }
            Message::Stopped(error) => {
                // Same reasoning as for `Finished`
                self.workspaces = None;
                self.status = format!("subscription stopped: {error}");
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        if self.workspaces.is_none() {
            return text(&self.status).into();
        }
        // as of 03/08/26 only cosmic and a few unstble/minor compositors
        // support `CreateWorkspace`
        let can_create = self.group().is_some_and(|group| {
            group
                .capabilities
                .contains(GroupCapabilities::CreateWorkspace)
        });

        let slots = row(SLOTS.map(|slot| {
            let occupant = self.occupant(slot);
            let state = match occupant {
                Some(workspace) if workspace.state.contains(State::Active) => Slot::Active,
                Some(workspace) if workspace.state.contains(State::Hidden) => Slot::Hidden,
                Some(_) => Slot::Present,
                None => Slot::Missing,
            };
            let label = match state {
                Slot::Active => format!("[{slot}]"),
                Slot::Hidden => format!("({slot})"),
                Slot::Present => slot.to_string(),
                Slot::Missing => format!("+{slot}"),
            };
            let style = match state {
                Slot::Active => button::primary,
                Slot::Hidden => button::secondary,
                Slot::Present => button::success,
                Slot::Missing => button::text,
            };
            let press = match occupant {
                // A compositor may expose workspaces it will not let us switch to.
                Some(workspace) => workspace
                    .capabilities
                    .contains(WorkspaceCapabilities::Activate)
                    .then(|| Message::Activate(workspace.id.clone())),
                None => can_create.then(|| Message::Create(slot.to_string())),
            };
            button(text(label))
                .style(style)
                .on_press_maybe(press)
                .into()
        }))
        .spacing(4);

        row![slots, text(&self.status)]
            .spacing(12)
            .width(Length::Fill)
            .into()
    }
}
