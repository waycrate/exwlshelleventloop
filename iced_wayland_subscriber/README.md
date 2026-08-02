# iced_wayland_subscriber

Wayland events as `iced` subscriptions: the surfaces a shell runtime creates,
the monitors they are shown on, and the compositor's workspaces.

## Shell broadcast

The main entry point. `shell::channel()` makes a sender/receiver pair: hand the
sender to the runtime through `Settings::shell_broadcast`, keep the receiver in
your application, and subscribe to it.

```rust
use iced_layershell::daemon;
use iced_layershell::settings::Settings;
use iced_wayland_subscriber::shell::{ShellEvent, ShellReceiver};

pub fn main() -> Result<(), iced_layershell::Error> {
    let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

    daemon(
        move || Counter::new(shell_events.clone()),
        Counter::namespace,
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

impl Counter {
    fn subscription(&self) -> iced::Subscription<Message> {
        self.shell_events.listen().map(Message::Shell)
    }
}
```

Six events:

| event | meaning |
| --- | --- |
| `NewShell(ShellInfo)` | the runtime created a surface: its `window: Id` and `shell: ShellType` |
| `Closed(Id)` | that window is gone |
| `WindowOutputChanged { window, output }` | which monitor a window is on, `None` if not yet known |
| `OutputAdded(OutputInfo)` | a monitor was connected |
| `OutputUpdated(OutputInfo)` | its mode, scale, name or position changed |
| `OutputRemoved(OutputInfo)` | it was disconnected |

`ShellType` distinguishes `LayerShell`, `PopUp`, `XdgTopLevel`, `InputPanel` and
`SessionLock`, so an application can tell a bar from a popup without tracking
ids itself.

Subscribing late is safe. The channel keeps the current monitors, the live
shells and their outputs, and replays them to a new subscriber before any new
event, so the application does not have to be listening from the first frame.

Opening one surface per monitor:

```rust
match event {
    ShellEvent::OutputAdded(output) => {
        let id = iced::window::Id::unique();
        self.bars.insert(OutputId::from(&output), id);
        Command::done(Message::NewLayerShell {
            settings: NewLayerShellSettings {
                output_option: OutputOption::GlobalName(output.id),
                ..Default::default()
            },
            id,
        })
    }
    ShellEvent::OutputRemoved(output) => {
        self.bars.remove(&OutputId::from(&output));
        Command::none()
    }
    _ => Command::none(),
}
```

See `iced_examples/counter_universe` for the whole thing, and
`iced_examples/counter_multi` for the smaller version.

### `on_new_shell`

The broadcast is asynchronous: a subscription delivers on the runtime's
schedule, so `view(id)` can run for a window before the `NewShell` for it
arrives. When the first frame would be wrong without that knowledge, use the
`on_new_shell` hook on the daemon builder instead, it runs synchronously at
surface creation and feeds `update` before anything is drawn.

```rust
.on_new_shell(|info: ShellInfo| {
    matches!(info.shell, ShellType::SessionLock).then(|| Message::LockAppeared(info.window))
})
```

Both report the same surfaces. Use the hook when the timing matters, the
broadcast otherwise.

## Standalone output subscription

`output::listen(connection)` watches monitors over a connection you supply,
without a shell runtime. It exists for applications without event loop to
broadcast from like a plain `iced` program on winit.

```rust
iced_wayland_subscriber::output::listen(connection).map(Message::Output)
```

## Workspaces

`workspace::listen(connection)` tracks `ext_workspace_manager_v1` when the
compositor implements it. The whole tree arrives as
`WorkspaceEvent::Updated(Arc<WorkspaceSnapshot>)`, once per compositor-side
change. The protocol's `done` event is an atomic barrier, so a snapshot is never
torn mid-update.

```rust
match event {
    WorkspaceEvent::Updated(snapshot) => {
        for group in &snapshot.groups {
            // Which monitors this group covers.
            let outputs = &group.outputs;
            // The workspace to highlight for those monitors.
            let active = snapshot.active_in(&group.id);
            let _ = (outputs, active);
        }
        self.workspaces = Some(snapshot);
    }
    WorkspaceEvent::Unsupported => {
        // Hide workspace UI.
    }
    WorkspaceEvent::Finished => {
        // The compositor ended the protocol. Nothing follows.
    }
    WorkspaceEvent::Stop(error) => eprintln!("{error}"),
}
```

`Unsupported` means the global was not there at start-up. It is not the end of
subscription, a compositor that registers the global later is picked up, and
an `Updated` follows. `Finished` is the end - the compositor destroyed the
manager and nothing more arrives.

To switch workspace, call the request methods on a stored snapshot. Requests go
straight onto the connection from wherever you call them, so `update()` is fine:

```rust
snapshot.activate(&workspace_id)?;
```

Each method checks the capability the compositor advertised and returns
`RequestError::Unsupported` rather than doing nothing, because the compositor
itself silently ignores requests it does not support.
Also available: `deactivate`, `remove`, `assign` (which moves a
workspace to another group, not a window) and `create_workspace`.

See `iced_examples/workspace_bar` for a working bar.

## Feature flags

Because `ext_workspace_manager_v1` is not that widely supported,
workspace subscription is behind the non-default `workspace` feature:

```toml
iced_wayland_subscriber = { version = "…", features = ["workspace"] }
```
