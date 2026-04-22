# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
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
