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
    pub use exwlshellev::NewInputPanelSettings;
    pub use exwlshellev::NewLayerShellSettings;
    pub use exwlshellev::OutputOption;
    pub use exwlshellev::WithConnection;
    pub use exwlshellev::WlShellType;
    pub use exwlshellev::reexport::Anchor;
    pub use exwlshellev::reexport::KeyboardInteractivity;
    pub use exwlshellev::reexport::Layer;
    pub use exwlshellev::reexport::wayland_client::{WlRegion, wl_keyboard};
    pub mod core {
        pub use iced_core::*;
    }
    pub use iced_core::window::Id as IcedId;
    pub use iced_runtime::Task;
}

mod ime_preedit;

pub use iced_exwlshell_macros::to_wlshell_message;

pub use error::Error;

pub trait FromShellInfo {
    fn get(shell: NewShellInfo) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct NewShellInfo {
    pub id: iced_core::window::Id,
    pub shell: exwlshellev::WlShellType,
}

pub type Result = std::result::Result<(), error::Error>;
use iced_core::theme::Style as Appearance;

use iced_core::theme::Base as DefaultStyle;

// layershell application
pub use build_pattern::daemon;

pub use settings::Settings;
