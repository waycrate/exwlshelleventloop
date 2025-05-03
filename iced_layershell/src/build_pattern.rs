//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod application;
mod daemon;

pub use application::Program as ApplicationProgram;
pub use daemon::Program as DaemonProgram;
/// The renderer of some Program.
pub trait Renderer: iced_core::text::Renderer + iced_graphics::compositor::Default {}

impl<T> Renderer for T where T: iced_core::text::Renderer + iced_graphics::compositor::Default {}

pub use application::Instance as ApplicationInstance;
#[doc = include_str!("./build_pattern/application.md")]
pub use application::application;

pub use daemon::Instance as DaemonInstance;
#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;
