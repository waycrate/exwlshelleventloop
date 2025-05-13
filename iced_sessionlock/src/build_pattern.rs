use std::borrow::Cow;

use iced::Font;

/// The renderer of iced program.
pub trait Renderer: iced_core::text::Renderer + iced_graphics::compositor::Default {}

impl<T> Renderer for T where T: iced_core::text::Renderer + iced_graphics::compositor::Default {}

pub use pattern::application;

mod pattern {
    use super::*;
    use crate::settings::Settings;
    use iced::{Element, Task};

    use crate::actions::UnLockAction;

    use crate::DefaultStyle;

    use crate::Result;
    use iced_exdevtools::devtools_generate;

    devtools_generate! {
        Type = DevTools,
        Program = Program,
        MyAction = UnLockAction
    }
    #[allow(unused)]
    fn attach<P>(program: P) -> impl Program<Message = Event<P>>
    where
        P: Program + 'static,
        Event<P>: TryInto<UnLockAction, Error = Event<P>> + std::fmt::Debug + Send + 'static,
    {
        struct Attach<P> {
            program: P,
        }

        impl<P> Program for Attach<P>
        where
            P: Program + 'static,
        {
            type State = DevTools<P>;
            type Message = Event<P>;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = P::Executor;

            fn name() -> &'static str {
                P::name()
            }

            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                let (state, boot) = self.program.boot();
                let (devtools, task) = DevTools::new(state);

                (
                    devtools,
                    Task::batch([boot.map(Event::Program), task.map(Event::Message)]),
                )
            }

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                state.update(&self.program, message)
            }

            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                state.view(&self.program, window)
            }

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                state.subscription(&self.program)
            }

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                self.program.theme(state.state(), window)
            }

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> iced::theme::Style {
                self.program.style(state.state(), theme)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(state.state(), window)
            }
        }

        Attach { program }
    }

    use iced_program::Program;

    /// The update logic of some [`Application`].
    ///
    /// This trait allows the [`application`] builder to take any closure that
    /// returns any `Into<Task<Message>>`.
    pub trait Update<State, Message> {
        /// Processes the message and updates the state of the [`Application`].
        fn update(&self, state: &mut State, message: Message) -> impl Into<Task<Message>>;
    }

    impl<State, Message> Update<State, Message> for () {
        fn update(&self, _state: &mut State, _message: Message) -> impl Into<Task<Message>> {}
    }

    impl<T, State, Message, C> Update<State, Message> for T
    where
        T: Fn(&mut State, Message) -> C,
        C: Into<Task<Message>>,
    {
        fn update(&self, state: &mut State, message: Message) -> impl Into<Task<Message>> {
            self(state, message)
        }
    }

    /// The view logic of some [`Application`].
    ///
    /// This trait allows the [`application`] builder to take any closure that
    /// returns any `Into<Element<'_, Message>>`.
    pub trait View<'a, State, Message, Theme, Renderer> {
        /// Produces the widget of the [`Application`].
        fn view(
            &self,
            state: &'a State,
            window: iced_core::window::Id,
        ) -> impl Into<Element<'a, Message, Theme, Renderer>>;
    }

    impl<'a, T, State, Message, Theme, Renderer, Widget> View<'a, State, Message, Theme, Renderer> for T
    where
        T: Fn(&'a State, iced_core::window::Id) -> Widget,
        State: 'static,
        Widget: Into<Element<'a, Message, Theme, Renderer>>,
    {
        fn view(
            &self,
            state: &'a State,
            window: iced_core::window::Id,
        ) -> impl Into<Element<'a, Message, Theme, Renderer>> {
            self(state, window)
        }
    }

    pub trait Boot<State, Message> {
        /// Initializes the [`Application`] state.
        fn boot(&self) -> (State, Task<Message>);
    }

    impl<T, C, State, Message> Boot<State, Message> for T
    where
        T: Fn() -> C,
        C: IntoBoot<State, Message>,
    {
        fn boot(&self) -> (State, Task<Message>) {
            self().into_boot()
        }
    }

    /// The initial state of some [`Application`].
    pub trait IntoBoot<State, Message> {
        /// Turns some type into the initial state of some [`Application`].
        fn into_boot(self) -> (State, Task<Message>);
    }

    impl<State, Message> IntoBoot<State, Message> for State {
        fn into_boot(self) -> (State, Task<Message>) {
            (self, Task::none())
        }
    }

    impl<State, Message> IntoBoot<State, Message> for (State, Task<Message>) {
        fn into_boot(self) -> (State, Task<Message>) {
            self
        }
    }
    #[derive(Debug)]
    pub struct Application<A: Program> {
        raw: A,
        settings: Settings,
    }

    pub fn application<State, Message, Theme, Renderer>(
        boot: impl Boot<State, Message>,
        update: impl Update<State, Message>,
        view: impl for<'a> self::View<'a, State, Message, Theme, Renderer>,
    ) -> Application<impl Program<Message = Message, Theme = Theme, State = State>>
    where
        State: 'static,
        Message: 'static + TryInto<UnLockAction, Error = Message> + Send + std::fmt::Debug,
        Theme: Default + DefaultStyle,
        Renderer: self::Renderer,
    {
        use std::marker::PhantomData;
        struct Instance<State, Message, Theme, Renderer, Update, View, Boot> {
            update: Update,
            view: View,
            boot: Boot,
            _state: PhantomData<State>,
            _message: PhantomData<Message>,
            _theme: PhantomData<Theme>,
            _renderer: PhantomData<Renderer>,
        }
        impl<State, Message, Theme, Renderer, Update, View, Boot> Program
            for Instance<State, Message, Theme, Renderer, Update, View, Boot>
        where
            Message: 'static + TryInto<UnLockAction, Error = Message> + Send + std::fmt::Debug,
            Theme: Default + DefaultStyle,
            Renderer: self::Renderer,
            Update: self::Update<State, Message>,
            Boot: self::Boot<State, Message>,
            View: for<'a> self::View<'a, State, Message, Theme, Renderer>,
        {
            type State = State;
            type Renderer = Renderer;
            type Message = Message;
            type Theme = Theme;
            type Executor = iced_futures::backend::default::Executor;

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.update.update(state, message).into()
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.boot.boot()
            }
            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.view.view(state, window).into()
            }
            fn name() -> &'static str {
                let name = std::any::type_name::<State>();

                name.split("::").next().unwrap_or("a_cool_application")
            }
        }
        Application {
            raw: Instance {
                update,
                view,
                boot,
                _state: PhantomData,
                _message: PhantomData,
                _theme: PhantomData,
                _renderer: PhantomData,
            },
            settings: Settings::default(),
        }
    }

    pub fn with_executor<P: Program, E: iced_futures::Executor>(
        program: P,
    ) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
        use std::marker::PhantomData;

        struct WithExecutor<P, E> {
            program: P,
            executor: PhantomData<E>,
        }

        impl<P: Program, E> Program for WithExecutor<P, E>
        where
            E: iced_futures::Executor,
        {
            type State = P::State;
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = E;

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.program.update(state, message)
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.program.boot()
            }
            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(state, window)
            }

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                self.program.subscription(state)
            }

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                self.program.theme(state, window)
            }

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
                self.program.style(state, theme)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(state, window)
            }
            fn name() -> &'static str {
                P::name()
            }
        }

        WithExecutor {
            program,
            executor: PhantomData::<E>,
        }
    }

    pub fn with_subscription<P: Program>(
        program: P,
        f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
    ) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
        struct WithSubscription<P, F> {
            program: P,
            subscription: F,
        }

        impl<P: Program, F> Program for WithSubscription<P, F>
        where
            F: Fn(&P::State) -> iced::Subscription<P::Message>,
        {
            type State = P::State;
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = P::Executor;

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                (self.subscription)(state)
            }

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.program.update(state, message)
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.program.boot()
            }
            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(state, window)
            }

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                self.program.theme(state, window)
            }

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
                self.program.style(state, theme)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(state, window)
            }
            fn name() -> &'static str {
                P::name()
            }
        }

        WithSubscription {
            program,
            subscription: f,
        }
    }

    pub fn with_theme<P: Program>(
        program: P,
        f: impl Fn(&P::State, iced_core::window::Id) -> P::Theme,
    ) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
        struct WithTheme<P, F> {
            program: P,
            theme: F,
        }

        impl<P: Program, F> Program for WithTheme<P, F>
        where
            F: Fn(&P::State, iced_core::window::Id) -> P::Theme,
        {
            type State = P::State;
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = P::Executor;

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                (self.theme)(state, window)
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.program.boot()
            }

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.program.update(state, message)
            }

            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(state, window)
            }

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                self.program.subscription(state)
            }

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
                self.program.style(state, theme)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(state, window)
            }
            fn name() -> &'static str {
                P::name()
            }
        }

        WithTheme { program, theme: f }
    }

    pub fn with_style<P: Program>(
        program: P,
        f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
    ) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
        struct WithStyle<P, F> {
            program: P,
            style: F,
        }

        impl<P: Program, F> Program for WithStyle<P, F>
        where
            F: Fn(&P::State, &P::Theme) -> crate::Appearance,
        {
            type State = P::State;
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = P::Executor;

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
                (self.style)(state, theme)
            }

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.program.update(state, message)
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.program.boot()
            }
            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(state, window)
            }

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                self.program.subscription(state)
            }

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                self.program.theme(state, window)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(state, window)
            }
            fn name() -> &'static str {
                P::name()
            }
        }

        WithStyle { program, style: f }
    }

    pub fn with_scale_factor<P: Program>(
        program: P,
        f: impl Fn(&P::State, iced_core::window::Id) -> f64,
    ) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
        struct WithScaleFactor<P, F> {
            program: P,
            scale_factor: F,
        }

        impl<P: Program, F> Program for WithScaleFactor<P, F>
        where
            F: Fn(&P::State, iced_core::window::Id) -> f64,
        {
            type State = P::State;
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            type Executor = P::Executor;

            fn update(
                &self,
                state: &mut Self::State,
                message: Self::Message,
            ) -> Task<Self::Message> {
                self.program.update(state, message)
            }
            fn boot(&self) -> (Self::State, Task<Self::Message>) {
                self.program.boot()
            }
            fn view<'a>(
                &self,
                state: &'a Self::State,
                window: iced_core::window::Id,
            ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(state, window)
            }

            fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
                self.program.subscription(state)
            }

            fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
                self.program.theme(state, window)
            }

            fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
                self.program.style(state, theme)
            }

            fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
                (self.scale_factor)(state, window)
            }
            fn name() -> &'static str {
                P::name()
            }
        }

        WithScaleFactor {
            program,
            scale_factor: f,
        }
    }

    impl<P: Program> Application<P> {
        pub fn run(self) -> Result
        where
            Self: 'static,
            P::Message:
                std::fmt::Debug + Send + 'static + TryInto<UnLockAction, Error = P::Message>,
        {
            #[cfg(all(feature = "debug", not(target_arch = "wasm32")))]
            let program = {
                iced_debug::init(iced_debug::Metadata {
                    name: Program::name(),
                    theme: None,
                    can_time_travel: cfg!(feature = "time-travel"),
                });

                attach(self.raw)
            };

            #[cfg(any(not(feature = "debug"), target_arch = "wasm32"))]
            let program = self.raw;
            let settings = self.settings;
            let renderer_settings = iced_graphics::Settings {
                default_font: settings.default_font,
                default_text_size: settings.default_text_size,
                antialiasing: if settings.antialiasing {
                    Some(iced_graphics::Antialiasing::MSAAx4)
                } else {
                    None
                },
            };
            crate::multi_window::run(program, settings, renderer_settings)
        }

        pub fn settings(self, settings: Settings) -> Self {
            Self { settings, ..self }
        }

        /// Sets the [`Settings::antialiasing`] of the [`Application`].
        pub fn antialiasing(self, antialiasing: bool) -> Self {
            Self {
                settings: Settings {
                    antialiasing,
                    ..self.settings
                },
                ..self
            }
        }

        /// Sets the default [`Font`] of the [`Application`].
        pub fn default_font(self, default_font: Font) -> Self {
            Self {
                settings: Settings {
                    default_font,
                    ..self.settings
                },
                ..self
            }
        }

        /// Adds a font to the list of fonts that will be loaded at the start of the [`Application`].
        pub fn font(mut self, font: impl Into<Cow<'static, [u8]>>) -> Self {
            self.settings.fonts.push(font.into());
            self
        }

        /// set the default_text_size
        pub fn default_text_size(self, default_text_size: iced::Pixels) -> Self {
            Self {
                settings: Settings {
                    default_text_size,
                    ..self.settings
                },
                ..self
            }
        }

        /// Sets the style logic of the [`Application`].
        pub fn style(
            self,
            f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
        ) -> Application<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
        {
            Application {
                raw: with_style(self.raw, f),
                settings: self.settings,
            }
        }
        /// Sets the subscription logic of the [`Application`].
        pub fn subscription(
            self,
            f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
        ) -> Application<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
        {
            Application {
                raw: with_subscription(self.raw, f),
                settings: self.settings,
            }
        }

        /// Sets the theme logic of the [`Application`].
        pub fn theme(
            self,
            f: impl Fn(&P::State, iced_core::window::Id) -> P::Theme,
        ) -> Application<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
        {
            Application {
                raw: with_theme(self.raw, f),
                settings: self.settings,
            }
        }

        /// Sets the scale factor of the [`Application`].
        pub fn scale_factor(
            self,
            f: impl Fn(&P::State, iced_core::window::Id) -> f64,
        ) -> Application<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
        {
            Application {
                raw: with_scale_factor(self.raw, f),
                settings: self.settings,
            }
        }
        /// Sets the executor of the [`Application`].
        pub fn executor<E>(
            self,
        ) -> Application<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
        where
            E: iced_futures::Executor,
        {
            Application {
                raw: with_executor::<P, E>(self.raw),
                settings: self.settings,
            }
        }
    }
}
