# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed (breaking)
- Feat: track outputs with sctk's `OutputState` instead of manual `zxdg_output_v1` handling, removed `XdgInfoChanged` event, `XdgInfoChangedType`, `ZxdgOutputInfo` and `get_xdgoutput_info`
- Feat: new `DispatchMessage::OutputChanged` event sent when surface enters another output or its output info changes

### Changed
- Feat: add `WindowState::outputs()`, `output_by_name()`, `get_output_info()`, `get_output_info_of()` and `WindowStateUnit::get_wloutput()`
- Feat: add `output::listen()` subscription to iced_layershell and iced_exwlshell to track the output a window is displayed on

## [0.19.1] - 2026-07-12
### Changed
- Make the fields of LocalRegion in iced_wayland_subscriber public
- Fixed: iced_wayland_subscriber cannot work since wm did not send Done event to me

[0.19.1]: https://github.com/waycrate/exwlshelleventloop/compare/v0.19.1...v0.19.0

## [0.19.0] - 2026-07-12
### Changed (breaking)
- Feat: `IcedNewPopupSettings`/`NewPopUpSettings` take explicit parent surface plus full xdg_positioner controls (`anchor_rect`, `anchor`, `gravity`, `constraint_adjustment`) instead of a single `position`.
- Popups now positioned by the compositor relative to their parent surface, auto-flip/slide near screen edges, can be nested, and are dismissed via an xdg_popup grab.
- Removed None options in layershellev and replaced it with Active, fix the problem that original None option failed by @id3v1669
- Share keymap conversion implementation and induce winit-core by @httpsworldview

### Changed
- Feat: record transform information in iced_wayland_subscriber
- Removed the useless smol_str dependience
- Fixed: lifecycle handling #386 by @httpsworldview
- Refactor: share keymap conversion implementation by @krteke

[0.19.0]: https://github.com/waycrate/exwlshelleventloop/compare/v0.19.0...v0.18.1

### Removed (breaking)
- Removed `MenuDirection`, `IcedNewMenuSettings`, the `NewMenu` action, and `menu_open`. 
- Removed the now-unused public getters `WindowState::mouse_position()` and `WindowManager::get_alias()`
- Removed the option of None

## [0.18.1] - 2026-05-09
- Fix: handle wl_pointer::Event::AxisValue120 for scrolling (#376) by @ibrahimduran
- Fix: (Message, Instant) cannot try_into to LayerShellCustomActionWithId

[0.18.1]: https://github.com/waycrate/exwlshelleventloop/compare/v0.18.1...v0.18.0

## [0.18.0] - 2026-05-01

### Changed
- Feat: Sctk refactor in #368
- Fix: high cpu usage in #364 by @Magniquck
- Feat: add iced_exwlshell library
- Feat: add client side decoration control for XDG windows (#375) by @httpsworldview

[0.18.0]: https://github.com/waycrate/exwlshelleventloop/compare/v0.18.0...v0.17.1

## [0.18.0-beta4] - 2026-04-22

### Changed
- Feat: Sctk refactor in #368
- Fix: high cpu usage in #364 by @Magniquck
- Feat: add iced_exwlshell library
- Chore: republish

[0.18.0-beta4]: https://github.com/waycrate/exwlshelleventloop/compare/v0.18.0-beta4...v0.17.1

## [0.18.0-beta3] - 2026-04-22

### Changed
- Feat: Sctk refactor in #368
- Fix: high cpu usage in #364 by @Magniquck
- Feat: add iced_exwlshell library
- Chore: republish

[0.18.0-beta3]: https://github.com/waycrate/exwlshelleventloop/compare/v0.18.0-beta3...v0.17.1

## [0.18.0-beta2] - 2026-04-22

### Changed
- Feat: Sctk refactor in #368
- Fix: high cpu usage in #364 by @Magniquck
- Feat: add iced_exwlshell library

[0.18.0-beta2]: https://github.com/waycrate/exwlshelleventloop/compare/v0.18.0-beta2...v0.17.1

## [0.17.1] - 2026-03-25

### Changed
- Fixed: background cannot run

[0.17.1]: https://github.com/waycrate/exwlshelleventloop/compare/v0.17.1...v0.17.0

## [0.17.0] - 2026-03-25

### Changed

- Fixed: single program never exits after switching to tty
- Fixed: mouse and pointer dead after switching to tty
- refactor: use winit keyboard types (#357) by @bitbloxhub

### NOTE:
It should be breaking change because the waycrate_xkbkeycode is rewritten. I forgot that.

[0.17.0]: https://github.com/waycrate/exwlshelleventloop/compare/v0.17.0...v0.16.0

## [0.16.1] - 2026-03-25

### Changed

- Fixed: single program never exits after switching to tty
- Fixed: mouse and pointer dead after switching to tty

### Others
Maybe it is wired that layershell program must die, but seems that it is needed.

[0.16.1]: https://github.com/waycrate/exwlshelleventloop/compare/v0.16.1...v0.16.0

## [0.16.0] - 2026-03-18

### Changed

- Allow changing KeyboardInteractivity at runtime by @danhandrea
- feat: make with_connection accept function
- feat(layershellev): replace use_last_output with output_option in NewInputPanelSettings (#355) by @fortime
- fix: do not panic in eventloop run, and return the error to the top

[0.16.0]: https://github.com/waycrate/exwlshelleventloop/compare/v0.16.0...v0.15.1
