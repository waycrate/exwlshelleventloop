# exwlshelleventloop

Build Wayland desktop components with [iced](https://github.com/iced-rs/iced): panels, docks, launchers, lock screens and on-screen keyboards.

This project binds the `wlr-layer-shell` and `ext-session-lock` protocols, plus `input-panel`, to iced.
These protocols give a surface a role that `xdg-shell` does not cover, and `xdg-shell` is all that winit and iced support.
Hence the "ex" (extra) in `exwlshell`.

The design of `exwlshell` take reference from [winit](https://github.com/rust-windowing/winit), also with something different, and the `iced_exwlshell` help you to write `wlr-layer-shell` and `ext-session-lock` programs easily
This makes layer-shell and session-lock programs easier to write.

## How it works

iced has no way to add custom system actions or system events. We work around this through `Message`.

- The application's `Message` must implement `TryInto<ExWlShellCustomActionWithId, Error = Message>`.
- The event loop calls `try_into` on every message. A message that converts is run as a shell action, such as opening a layer surface or locking the session. Any other message goes to your `update` as usual.
- The `#[to_exwlshell_message]` attribute macro adds the action variants to your `Message` enum and implements that conversion for you.

Custom events are also proposed upstream in [iced-rs/iced#2658](https://github.com/iced-rs/iced/pull/2658).

## Crates

The main crates are listed first. `iced_exwlshell` is the one most users want.

### iced_exwlshell
[![Crates.io](https://img.shields.io/crates/v/iced_exwlshell.svg)](https://crates.io/crates/iced_exwlshell)

The full iced binding for the extra shells. Use it to build shell programs such as lock screens and docks.

- Open new layer-shell surfaces, including popups
- Session lock
- Virtual keyboard (`zwp_virtual_keyboard_v1`)

### exwlshellev
[![Crates.io](https://img.shields.io/crates/v/exwlshellev.svg)](https://crates.io/crates/exwlshellev)

One event loop for layer-shell, session-lock and input-panel. It does not depend on iced.
It provides complete shell support for `iced_exwlshell`, which is built on it.

### iced_wayland_subscriber
[![Crates.io](https://img.shields.io/crates/v/iced_wayland_subscriber.svg)](https://crates.io/crates/iced_wayland_subscriber)

iced subscriptions for Wayland events, such as `ext-workspace` and outputs.
Use it to listen to compositor state from an iced application.

### waycrate_xkbkeycode
[![Crates.io](https://img.shields.io/crates/v/waycrate_xkbkeycode.svg)](https://crates.io/crates/waycrate_xkbkeycode)

Handles `xkbcommon` events. Mostly adapted from winit.

### Supporting crates

| Crate | Purpose |
|-------|---------|
| [`iced_exwlshell_macros`](./iced_exwlshell_macros) | Proc macros, re-exported by `iced_exwlshell`. |
| [`iced_exdevtools`](./iced_exdevtools) | Developer tools for `iced_exwlshell` applications. |

Runnable examples are in [`iced_examples`](./iced_examples).

## Limitations

Popups work in these shells, but neither winit nor iced has popup or tooltip support yet. Treat them as experimental.
A design for proper support exists. It is not implemented, and needs a closer read of the iced internals first.

## Documentation and help

The API documentation is still thin. If something is unclear:

- Ask in our Discord
- Open an issue
- Send a PR

PRs and issues are always welcome.

## Migrating

From `0.20.0`, `iced_layershell` and `iced_sessionlock` are deprecated.
Their functions and macros now live in `iced_exwlshell`. The migration guides are in [`docs`](./docs):

- [0.19.1 → 0.20.0](./docs/MIGRATION-19.1-%3E20.0.md)
- [0.20.0 → 0.21.0](./docs/MIGRATION-20.0-%3E21.0.md)
