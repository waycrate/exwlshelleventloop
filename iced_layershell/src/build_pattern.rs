//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod application;
mod daemon;

/// The renderer of some Program.
use iced_exdevtools::gen_attach;

gen_attach! {Action = LayershellCustomActionWithId}

#[doc = include_str!("./build_pattern/application.md")]
pub use application::application;

/// It is the same with the timed in iced
pub use application::timed;

pub use application::SingleApplication;

#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;

pub use daemon::Daemon;

use crate::actions::LayershellCustomActionWithId;
