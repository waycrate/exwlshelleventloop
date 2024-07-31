# Extra wayland shell event loop

Take winit as reference a lot, to make easilier program on layershell and ext-session-lock.

This project bind `ext-session-lock` and `layershell` with the similar way of winit, which storing message and handle it in callback

Here are five subprojects

## waycrate_xkbkeycode
[![Crates.io](https://img.shields.io/crates/v/waycrate_xkbkeycode.svg)](https://crates.io/crates/waycrate_xkbkeycode)

Take a lot of reference from winit (mainly from winit). Mainly handle the xkbcommon events.

## layershellev
[![Crates.io](https://img.shields.io/crates/v/layershellev.svg)](https://crates.io/crates/layershellev)

Winit like layershell event crate

## sessionlockev
[![Crates.io](https://img.shields.io/crates/v/sessionlockev.svg)](https://crates.io/crates/sessionlockev)

Winit like sessionlock event crate

## iced-layershell
[![Crates.io](https://img.shields.io/crates/v/iced-layershell.svg)](https://crates.io/crates/iced-layershell)

iced binding for layershell

## iced-sessionlock
[![Crates.io](https://img.shields.io/crates/v/iced-sessionlock.svg)](https://crates.io/crates/iced-sessionlock)

iced binding for sessionlock
