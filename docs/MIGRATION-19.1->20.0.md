# Migration 0.19.1 -> HEAD

## Sizes

`LayerShellSettings::size` and `NewLayerShellSettings::size` are `LayerSize`:

```rust
size: Some((0, 30))    -> size: LayerSize::fill_width(30)
size: Some((400, 0))   -> size: LayerSize::fill_height(400)
size: Some((100, 100)) -> size: LayerSize::px(100, 100)
size: None             -> size: LayerSize::FILL
```

`with_option_size` is gone; use `with_size(LayerSize::...)`.

Fallible constructors: `LayerSize::try_px`, `PixelSize::try_px`.

## Popups

```rust
IcedNewPopupSettings::new(parent, (100, 100), (0, 0, 1, 1))
-> IcedNewPopupSettings::new(parent, PixelSize::px(100, 100), (0, 0), PixelSize::px(1, 1))

IcedNewPopupSettings::at_position(parent, (100, 100), pos)
-> IcedNewPopupSettings::at_position(parent, PixelSize::px(100, 100), pos)
```

Same for `on_current_surface` / `at_position_on_current_surface`.

New: `PopUpRepositionSettings` and the `PopUpReposition` message.

## Messages

Injected variants:

```rust
Message::AnchorChange(..)     -> Message::LayoutChange { id, anchor, size }
Message::AnchorSizeChange(..) -> Message::LayoutChange { id, anchor, size }
Message::SizeChange(..)       -> Message::LayoutChange { id, anchor, size }
```

New: `BlurOptionChange`, `PopUpReposition`.

## Unit mutation

`set_anchor` and `set_size` take `&mut self`; `set_anchor_with_size` is now
`set_layout(anchor, size)`.

```rust
ev.get_unit_with_id(id)      // &WindowStateUnit
ev.get_mut_unit_with_id(id)  // &mut, needed to set anchor/size/layout
```

Removed: `get_logical_size`, `get_position`, `LogicalRegion`, `Position`,
`Size`. Use `LayerSize` / `PixelSize` / `Extent`.

## Outputs

Removed: `XdgInfoChanged`, `XdgInfoChangedType`, `ZxdgOutputInfo`,
`get_xdgoutput_info`, `WaylandEvent`.

`OutputInfo` is sctk's and carries no `wl_output`:

```rust
OutputOption::Output(info.wl_output) -> OutputOption::GlobalName(info.id)
info.physical_size  // millimetres, for pixels use pixel_size(&info)
```

Track monitors by `OutputId`, not by the `WlOutput` proxy.

`OutputOption` is `Active` (default) / `LastOutput` / `OutputName(String)` /
`GlobalName(u32)` / `Output(WlOutput)`. A name matching nothing connected logs a
warning and falls back to letting the compositor choose.

New on the event loops:

```rust
DispatchMessage::{OutputAdded, OutputUpdated, OutputRemoved}   // the monitors
DispatchMessage::OutputChanged(Option<WlOutput>)               // a surface moved
WindowState::{outputs, output_by_name, output_by_global_name,
              get_output_info, get_output_info_of}
WindowStateUnit::{get_wloutput, get_wloutputs}
```

`sessionlockev` has the three monitor messages and `get_output_info_of` /
`get_wloutput`, but no `OutputChanged`: a lock surface is pinned to its monitor
for life.

`get_wloutput()` returns the first output entered, not the last.

## Shell events

`ShellInfo` trait, `FromShellInfo` and the injected `NewShell` variant are gone.
Replaced by two hooks.

Synchronous, before the surface's first frame:

```rust
daemon(...).on_new_shell(|info: ShellInfo| {
    matches!(info.shell, ShellType::SessionLock)
        .then(|| Message::LockAppeared(info.window))
})
```

Asynchronous. `Settings::shell_broadcast` is on all three runtimes,
`iced_layershell`, `iced_exwlshell` and `iced_sessionlock`:

```rust
let (shell_broadcast, shell_events) = iced_wayland_subscriber::shell::channel();

application(move || App::new(shell_events.clone()), ...)
    .settings(Settings { shell_broadcast, ..Default::default() })
```

```rust
self.shell_events.listen().filter_map(|event| match event {
    ShellEvent::NewShell(info) => ...,          // info.window, info.shell
    ShellEvent::Closed(id) => ...,
    ShellEvent::WindowOutputChanged { window, output } => ...,
    ShellEvent::OutputAdded(info) => ...,
    ShellEvent::OutputUpdated(info) => ...,
    ShellEvent::OutputRemoved(info) => ...,
})
```

`ShellType` is `LayerShell` / `PopUp` / `XdgTopLevel` / `InputPanel` /
`SessionLock`.

A late subscriber is replayed the current monitors, live shells and their
outputs before any new event, so it need not listen from the first frame.

Under a shell runtime take monitors from here, not from `output::listen`.
`output::listen(connection)` still exists for an application with no shell
runtime, and is the only route with an error event, `OutputEvent::Stop`.

## Window wrappers

```rust
gen_wrapper()            -> Arc<WindowWrapper>
gen_main_wrapper()       -> Arc<WindowWrapper>
gen_mainwindow_wrapper() -> Arc<WindowWrapper>
```

All three event loops. Drop any `Arc::new(...)` you wrapped these in.

## Connection

```rust
with_connection: Some(connection) -> with_connection: Some(connection.into())
```

`HashConnection` is no longer public.

## Blur (new)

`LayerShellSettings::blur_option` and `NewLayerShellSettings::blur_option` take
a `BlurOption`; `BlurRegion` describes the area. Change it at runtime with
`Message::BlurOptionChange`.

## Workspaces (new, behind feature flag)

```toml
iced_wayland_subscriber = { version = "...", features = ["workspace"] }
```

```rust
iced_wayland_subscriber::workspace::listen(connection.clone())
```

`WorkspaceEvent::Updated` carries an `Arc<WorkspaceSnapshot>`. `Unsupported`
does not end the subscription, a compositor registering the global later is
picked up, but `Finished` does.

Requests (`activate`, `deactivate`, `remove`, `assign`, `create_workspace`) go
on the snapshot and are capability-gated: check `WorkspaceCapabilities` /
`GroupCapabilities` before showing UI. A snapshot outliving its protocol objects
returns `RequestError::Gone`.
