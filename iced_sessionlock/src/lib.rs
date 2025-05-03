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

mod program;
use iced::Element;

pub use iced_sessionlock_macros::to_session_message;

pub use error::Error;

use settings::SettingsMain;

use iced::theme::Base as DefaultStyle;
use iced::theme::Style as Appearance;

pub type Result = std::result::Result<(), error::Error>;

pub use build_pattern::application;
