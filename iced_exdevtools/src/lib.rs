#![allow(missing_docs)]
use iced_program as program;
use iced_program::runtime;
use iced_program::runtime::futures;
pub use iced_widget::core;

pub use crate::core::theme::{self, Base, Theme};
pub use crate::core::window;
pub use crate::core::{Alignment::Center, Color, Element, Length::Fill};
pub use crate::futures::Subscription;
pub use crate::program::Program;
pub use crate::runtime::Task;
pub use crate::program::message::MaybeDebug;
pub use crate::core::Settings as IcedSettings;

pub use iced_devtools::{DevTools, Event};

#[macro_export]
macro_rules! gen_attach {
    (
        Action = $Action:ident
    ) => {
        impl<P: $crate::Program> TryInto<$Action> for $crate::Event<P>
        where
            P::Message: $crate::MaybeDebug + 'static + TryInto<$Action, Error = P::Message>,
        {
            type Error = Self;
            fn try_into(self) -> std::result::Result<$Action, Self::Error> {
                let $crate::Event::Program(message) = self else {
                    return Err(self);
                };

                let message: std::result::Result<$Action, P::Message> = message.try_into();

                match message {
                    Ok(action) => Ok(action),
                    Err(message) => Err(Self::Program(message)),
                }
            }
        }
        fn attach<P>(program: P) -> impl $crate::Program<Message = $crate::Event<P>>
        where
            P: $crate::Program + 'static,
            P::Message: $crate::MaybeDebug + Send + 'static + TryInto<$Action, Error = P::Message>,
            $crate::Event<P>:
                TryInto<$Action, Error = $crate::Event<P>> + std::fmt::Debug + Send + 'static,
        {
            struct Attach<P> {
                program: P,
            }

            impl<P> $crate::Program for Attach<P>
            where
                P: $crate::Program + 'static,
                P::Message: $crate::MaybeDebug,
            {
                type State = $crate::DevTools<P>;
                type Message = $crate::Event<P>;
                type Theme = P::Theme;
                type Renderer = P::Renderer;
                type Executor = P::Executor;

                fn name() -> &'static str {
                    P::name()
                }

                fn boot(&self) -> (Self::State, $crate::Task<Self::Message>) {
                    let (state, boot) = self.program.boot();
                    let (devtools, task) = $crate::DevTools::new(state);

                    (
                        devtools,
                        $crate::Task::batch([
                            boot.map($crate::Event::Program),
                            task.map($crate::Event::Message),
                        ]),
                    )
                }

                fn title(&self, state: &Self::State, window: $crate::window::Id) -> String {
                    state.title(&self.program, window)
                }

                fn update(
                    &self,
                    state: &mut Self::State,
                    message: Self::Message,
                ) -> $crate::Task<Self::Message> {
                    state.update(&self.program, message)
                }

                fn view<'a>(
                    &self,
                    state: &'a Self::State,
                    window: $crate::window::Id,
                ) -> $crate::Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                    state.view(&self.program, window)
                }

                fn subscription(&self, state: &Self::State) -> $crate::Subscription<Self::Message> {
                    state.subscription(&self.program)
                }

                fn settings(&self) -> $crate::IcedSettings {
                    self.program.settings()
                }

                fn window(&self) -> Option<$crate::core::window::Settings> {
                    self.program.window()
                }

                fn theme(
                    &self,
                    state: &Self::State,
                    window: $crate::window::Id,
                ) -> Option<Self::Theme> {
                    self.program.theme(state.state(), window)
                }

                fn style(&self, state: &Self::State, theme: &Self::Theme) -> $crate::theme::Style {
                    self.program.style(state.state(), theme)
                }

                fn scale_factor(&self, state: &Self::State, id: $crate::window::Id) -> f32 {
                    self.program.scale_factor(state.state(), id)
                }
            }

            Attach { program }
        }
    };
}
