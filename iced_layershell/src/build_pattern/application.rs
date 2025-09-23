use std::borrow::Cow;

use iced::Font;
use iced::{Element, Task};

use crate::actions::LayershellCustomActionWithId;

use crate::DefaultStyle;
use crate::settings::LayerShellSettings;

use crate::Result;
use crate::settings::Settings;

use iced_debug as debug;
use iced_program::Program;

pub trait NameSpace {
    /// Produces the namespace of the [`SingleApplication`].
    fn namespace(&self) -> String;
}

impl NameSpace for &'static str {
    fn namespace(&self) -> String {
        self.to_string()
    }
}

impl<T> NameSpace for T
where
    T: Fn() -> String,
{
    fn namespace(&self) -> String {
        self()
    }
}

/// The update logic of some [`SingleApplication`].
///
/// This trait allows the [`application`] builder to take any closure that
/// returns any `Into<Task<Message>>`.
pub trait Update<State, Message> {
    /// Processes the message and updates the state of the [`SingleApplication`].
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

/// The view logic of some [`SingleApplication`].
///
/// This trait allows the [`application`] builder to take any closure that
/// returns any `Into<Element<'_, Message>>`.
pub trait View<'a, State, Message, Theme, Renderer> {
    /// Produces the widget of the [`Application`].
    fn view(&self, state: &'a State) -> impl Into<Element<'a, Message, Theme, Renderer>>;
}

impl<'a, T, State, Message, Theme, Renderer, Widget> View<'a, State, Message, Theme, Renderer> for T
where
    T: Fn(&'a State) -> Widget,
    State: 'static,
    Widget: Into<Element<'a, Message, Theme, Renderer>>,
{
    fn view(&self, state: &'a State) -> impl Into<Element<'a, Message, Theme, Renderer>> {
        self(state)
    }
}

pub trait Boot<State, Message> {
    /// Initializes the [`SingleApplication`] state.
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

/// The initial state of some [`SingleApplication`].
pub trait IntoBoot<State, Message> {
    /// Turns some type into the initial state of some [`SingleApplication`].
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

/// The theme logic of some [`SingleApplication`].
///
/// Any implementors of this trait can be provided as an argument to
/// [`SingleApplication::theme`].
///
/// `iced` provides two implementors:
/// - the built-in [`iced::Theme`] itself
/// - and any `Fn(&State, iced::window::Id) -> impl Into<Option<iced::Theme>>`.
pub trait ThemeFn<State, Theme> {
    /// Returns the theme of the [`Daemon`] for the current state and window.
    ///
    /// If `None` is returned, `iced` will try to use a theme that
    /// matches the system color scheme.
    fn theme(&self, state: &State) -> Option<Theme>;
}

impl<State> ThemeFn<State, iced::Theme> for iced::Theme {
    fn theme(&self, _state: &State) -> Option<iced::Theme> {
        Some(self.clone())
    }
}

impl<F, T, State, Theme> ThemeFn<State, Theme> for F
where
    F: Fn(&State) -> T,
    T: Into<Option<Theme>>,
{
    fn theme(&self, state: &State) -> Option<Theme> {
        (self)(state).into()
    }
}

#[derive(Debug)]
pub struct SingleApplication<A: Program> {
    raw: A,
    settings: Settings,
    namespace: String,
}

pub fn application<State, Message, Theme, Renderer>(
    boot: impl Boot<State, Message>,
    namespace: impl NameSpace,
    update: impl Update<State, Message>,
    view: impl for<'a> self::View<'a, State, Message, Theme, Renderer>,
) -> SingleApplication<impl Program<Message = Message, Theme = Theme, State = State>>
where
    State: 'static,
    Message:
        'static + TryInto<LayershellCustomActionWithId, Error = Message> + Send + std::fmt::Debug,
    Theme: DefaultStyle,
    Renderer: iced_program::Renderer,
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
        Message: 'static + TryInto<LayershellCustomActionWithId, Error = Message> + Send,
        Theme: DefaultStyle,
        Renderer: iced_program::Renderer,
        Update: self::Update<State, Message>,
        Boot: self::Boot<State, Message>,
        View: for<'a> self::View<'a, State, Message, Theme, Renderer>,
    {
        type State = State;
        type Renderer = Renderer;
        type Message = Message;
        type Theme = Theme;
        type Executor = iced_futures::backend::default::Executor;

        fn name() -> &'static str {
            let name = std::any::type_name::<State>();

            name.split("::").next().unwrap_or("a_cool_application")
        }

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            debug::hot(|| self.update.update(state, message)).into()
        }
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.boot.boot()
        }
        fn view<'a>(
            &self,
            state: &'a Self::State,
            _window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            debug::hot(|| self.view.view(state)).into()
        }

        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }
    SingleApplication {
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
        namespace: namespace.namespace(),
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

        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn name() -> &'static str {
            P::name()
        }

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, window: iced::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, window)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, id: iced::window::Id) -> f32 {
            self.program.scale_factor(state, id)
        }
        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
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
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn name() -> &'static str {
            P::name()
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn theme(&self, state: &Self::State, window: iced::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, window)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, id: iced::window::Id) -> f32 {
            self.program.scale_factor(state, id)
        }
        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }

    WithSubscription {
        program,
        subscription: f,
    }
}

pub fn with_theme<P: Program>(
    program: P,
    f: impl Fn(&P::State) -> Option<P::Theme>,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithTheme<P, F> {
        program: P,
        theme: F,
    }

    impl<P: Program, F> Program for WithTheme<P, F>
    where
        F: Fn(&P::State) -> Option<P::Theme>,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;

        fn theme(&self, state: &Self::State, _window: iced::window::Id) -> Option<Self::Theme> {
            (self.theme)(state)
        }

        fn name() -> &'static str {
            P::name()
        }
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, id: iced::window::Id) -> f32 {
            self.program.scale_factor(state, id)
        }
        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
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

        fn name() -> &'static str {
            P::name()
        }

        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, window: iced::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, window)
        }

        fn scale_factor(&self, state: &Self::State, id: iced::window::Id) -> f32 {
            self.program.scale_factor(state, id)
        }
        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }

    WithStyle { program, style: f }
}

pub fn with_scale_factor<P: Program>(
    program: P,
    f: impl Fn(&P::State) -> f32,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithScaleFactor<P, F> {
        program: P,
        scale_factor: F,
    }

    impl<P: Program, F> Program for WithScaleFactor<P, F>
    where
        F: Fn(&P::State) -> f32,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;

        fn name() -> &'static str {
            P::name()
        }

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, window: iced::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, window)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, _id: iced::window::Id) -> f32 {
            (self.scale_factor)(state)
        }
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn settings(&self) -> iced::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }

    WithScaleFactor {
        program,
        scale_factor: f,
    }
}

impl<P: Program> SingleApplication<P> {
    pub fn run(self) -> Result
    where
        Self: 'static,
        P::Message: std::fmt::Debug
            + Send
            + 'static
            + TryInto<LayershellCustomActionWithId, Error = P::Message>,
    {
        let settings = self.settings;

        #[cfg(all(feature = "debug", not(target_arch = "wasm32")))]
        let program = {
            iced_debug::init(iced_debug::Metadata {
                name: P::name(),
                theme: None,
                can_time_travel: cfg!(feature = "time-travel"),
            });

            super::attach(self.raw)
        };

        #[cfg(any(not(feature = "debug"), target_arch = "wasm32"))]
        let program = self.raw;

        #[allow(clippy::needless_update)]
        let renderer_settings = iced_graphics::Settings {
            default_font: settings.default_font,
            default_text_size: settings.default_text_size,
            antialiasing: if settings.antialiasing {
                Some(iced_graphics::Antialiasing::MSAAx4)
            } else {
                None
            },
            ..iced_graphics::Settings::default()
        };
        use layershellev::StartMode;
        assert!(!matches!(
            settings.layer_settings.start_mode,
            StartMode::AllScreens | StartMode::Background
        ));
        crate::multi_window::run(program, &self.namespace, settings, renderer_settings)
    }

    pub fn settings(self, settings: Settings) -> Self {
        Self { settings, ..self }
    }

    /// Sets the [`Settings::antialiasing`] of the [`SingleApplication`].
    pub fn antialiasing(self, antialiasing: bool) -> Self {
        Self {
            settings: Settings {
                antialiasing,
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the default [`Font`] of the [`SingleApplication`].
    pub fn default_font(self, default_font: Font) -> Self {
        Self {
            settings: Settings {
                default_font,
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the layershell setting of the [`SingleApplication`]
    pub fn layer_settings(self, layer_settings: LayerShellSettings) -> Self {
        Self {
            settings: Settings {
                layer_settings,
                ..self.settings
            },
            ..self
        }
    }

    /// Adds a font to the list of fonts that will be loaded at the start of the [`SingleApplication`].
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

    pub fn style(
        self,
        f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
    ) -> SingleApplication<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    {
        SingleApplication {
            raw: with_style(self.raw, move |state, theme| f(state, theme)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
    /// Sets the subscription logic of the [`SingleApplication`].
    pub fn subscription(
        self,
        f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
    ) -> SingleApplication<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    {
        SingleApplication {
            raw: with_subscription(self.raw, move |state| debug::hot(|| f(state))),
            settings: self.settings,
            namespace: self.namespace,
        }
    }

    /// Sets the theme logic of the [`SingleApplication`].
    pub fn theme(
        self,
        f: impl ThemeFn<P::State, P::Theme>,
    ) -> SingleApplication<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    {
        SingleApplication {
            raw: with_theme(self.raw, move |state| debug::hot(|| f.theme(state))),
            settings: self.settings,
            namespace: self.namespace,
        }
    }

    /// Sets the scale factor of the [`SingleApplication`].
    pub fn scale_factor(
        self,
        f: impl Fn(&P::State) -> f32,
    ) -> SingleApplication<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    {
        SingleApplication {
            raw: with_scale_factor(self.raw, move |state| debug::hot(|| f(state))),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
    /// Sets the executor of the [`SingleApplication`].
    pub fn executor<E>(
        self,
    ) -> SingleApplication<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    where
        E: iced_futures::Executor,
    {
        SingleApplication {
            raw: with_executor::<P, E>(self.raw),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
}
