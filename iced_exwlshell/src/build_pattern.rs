//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod daemon;
use crate::{FromShellInfo, NewShellInfo};
/// The renderer of some Program.
use iced_exdevtools::gen_attach;

gen_attach! {Action = ExwlShellCustomActionWithId, GetTrait = FromShellInfo, NewShellInfo = NewShellInfo}

#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;

pub use daemon::Daemon;

use crate::actions::ExwlShellCustomActionWithId;
