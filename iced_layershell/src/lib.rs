#![doc = include_str!("../README.md")]
pub mod actions;
pub mod build_pattern;
mod clipboard;
mod conversion;
mod error;
mod event;
mod multi_window;
mod proxy;
mod user_interface;

pub mod settings;

pub mod reexport {
    pub use layershellev::NewInputPanelSettings;
    pub use layershellev::NewLayerShellSettings;
    pub use layershellev::NewXdgWindowSettings;
    pub use layershellev::WithConnection;
    pub use layershellev::reexport::Anchor;
    pub use layershellev::reexport::KeyboardInteractivity;
    pub use layershellev::reexport::Layer;
    pub use layershellev::reexport::wayland_client::{WlRegion, wl_keyboard};
}

mod ime_preedit;

pub use iced_layershell_macros::to_layer_message;

pub use error::Error;

pub type Result = std::result::Result<(), error::Error>;
use iced::theme::Style as Appearance;

use iced::theme::Base as DefaultStyle;

// layershell application
pub use build_pattern::application;
pub use build_pattern::daemon;

pub use settings::Settings;
