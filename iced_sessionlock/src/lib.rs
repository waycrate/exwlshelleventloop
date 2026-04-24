#![doc = include_str!("../README.md")]
pub mod actions;
pub mod build_pattern;
pub mod multi_window;
pub mod settings;

mod clipboard;
mod conversion;
mod error;
mod event;
mod proxy;
mod user_interface;

pub use iced_sessionlock_macros::to_session_message;

pub use error::Error;

use iced_core::theme::Base as DefaultStyle;
use iced_core::theme::Style as Appearance;

/// Opt-out for clipboard initialization. Call this before starting the
/// runtime when your app has no text input and doesn't need paste/copy —
/// this skips spawning the always-on smithay-clipboard worker thread.
pub fn disable_clipboard() {
    clipboard::set_disabled();
}

pub type Result = std::result::Result<(), error::Error>;

pub use build_pattern::application;
pub use settings::Settings;
