//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod daemon;
/// The renderer of some Program.
use iced_exdevtools::gen_attach;

gen_attach! {Action = ExwlShellCustomActionWithId}

#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;

pub use daemon::Daemon;

#[doc = include_str!("./build_pattern/sessionlock.md")]
pub mod sessionlock;

#[doc = include_str!("./build_pattern/layershell.md")]
pub mod layershell;

use crate::actions::ExwlShellCustomActionWithId;
use iced_core::Settings;
use iced_program::Program;

use iced_core::Element;
use iced_runtime::Task;

struct ProgramWrapper<P: Program> {
    program: P,
    settings: Settings,
}

impl<P: Program> Program for ProgramWrapper<P> {
    type State = P::State;
    type Message = P::Message;
    type Theme = P::Theme;
    type Renderer = P::Renderer;
    type Executor = P::Executor;

    fn name() -> &'static str {
        P::name()
    }

    fn settings(&self) -> Settings {
        self.settings.clone()
    }

    fn window(&self) -> Option<iced_core::window::Settings> {
        None
    }

    fn boot(&self) -> (Self::State, Task<Self::Message>) {
        self.program.boot()
    }

    #[inline]
    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        self.program.update(state, message)
    }

    #[inline]
    fn view<'a>(
        &self,
        state: &'a Self::State,
        window: iced_core::window::Id,
    ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
        self.program.view(state, window)
    }

    #[inline]
    fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
        self.program.title(state, window)
    }

    #[inline]
    fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
        self.program.subscription(state)
    }

    #[inline]
    fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Option<Self::Theme> {
        self.program.theme(state, window)
    }

    #[inline]
    fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
        self.program.style(state, theme)
    }

    #[inline]
    fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
        self.program.scale_factor(state, window)
    }
}
