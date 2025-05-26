//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod application;
mod daemon;

/// The renderer of some Program.
pub trait Renderer: iced_core::text::Renderer + iced_graphics::compositor::Default {}

use iced_exdevtools::gen_attach;

gen_attach! {Action = LayershellCustomActionWithId}

impl<T> Renderer for T where T: iced_core::text::Renderer + iced_graphics::compositor::Default {}

#[doc = include_str!("./build_pattern/application.md")]
pub use application::application;

#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;

use crate::actions::LayershellCustomActionWithId;
