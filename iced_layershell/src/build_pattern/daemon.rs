use std::borrow::Cow;

use iced_core::Element;
use iced_core::Font;
use iced_runtime::Task;

use crate::actions::LayershellCustomActionWithId;

use crate::DefaultStyle;
use crate::settings::LayerShellSettings;

use crate::Result;

use crate::settings::Settings;

pub trait NameSpace {
    /// Produces the namespace of the [`Daemon`].
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

/// The update logic of some [`Daemon`].
///
/// This trait allows the [`daemon`] builder to take any closure that
/// returns any `Into<Task<Message>>`.
pub trait UpdateFn<State, Message> {
    /// Processes the message and updates the state of the [`Daemon`].
    fn update(&self, state: &mut State, message: Message) -> impl Into<Task<Message>>;
}

impl<State, Message> UpdateFn<State, Message> for () {
    fn update(&self, _state: &mut State, _message: Message) -> impl Into<Task<Message>> {}
}

impl<T, State, Message, C> UpdateFn<State, Message> for T
where
    T: Fn(&mut State, Message) -> C,
    C: Into<Task<Message>>,
{
    fn update(&self, state: &mut State, message: Message) -> impl Into<Task<Message>> {
        self(state, message)
    }
}

/// The view logic of some [`Daemon`].
///
/// This trait allows the [`application`] builder to take any closure that
/// returns any `Into<Element<'_, Message>>`.
pub trait ViewFn<'a, State, Message, Theme, Renderer> {
    /// Produces the widget of the [`Daemon`].
    fn view(
        &self,
        state: &'a State,
        window: iced_core::window::Id,
    ) -> Element<'a, Message, Theme, Renderer>;
}

impl<'a, T, State, Message, Theme, Renderer, Widget> ViewFn<'a, State, Message, Theme, Renderer>
    for T
where
    T: Fn(&'a State, iced_core::window::Id) -> Widget,
    State: 'static,
    Widget: Into<Element<'a, Message, Theme, Renderer>>,
{
    fn view(
        &self,
        state: &'a State,
        window: iced_core::window::Id,
    ) -> Element<'a, Message, Theme, Renderer> {
        self(state, window).into()
    }
}

pub trait BootFn<State, Message> {
    /// Initializes the [`Daemon`] state.
    fn boot(&self) -> (State, Task<Message>);
}

impl<T, C, State, Message> BootFn<State, Message> for T
where
    T: Fn() -> C,
    C: IntoBoot<State, Message>,
{
    fn boot(&self) -> (State, Task<Message>) {
        self().into_boot()
    }
}

/// The initial state of some [`Daemon`].
pub trait IntoBoot<State, Message> {
    /// Turns some type into the initial state of some [`Daemon`].
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

pub trait ThemeFn<State, Theme> {
    /// Returns the theme of the [`Daemon`] for the current state and window.
    ///
    /// If `None` is returned, `iced` will try to use a theme that
    /// matches the system color scheme.
    fn theme(&self, state: &State, window: iced_core::window::Id) -> Option<Theme>;
}

impl<State> ThemeFn<State, iced_core::Theme> for iced_core::Theme {
    fn theme(&self, _state: &State, _window: iced_core::window::Id) -> Option<iced_core::Theme> {
        Some(self.clone())
    }
}

impl<F, T, State, Theme> ThemeFn<State, Theme> for F
where
    F: Fn(&State, iced_core::window::Id) -> T,
    T: Into<Option<Theme>>,
{
    fn theme(&self, state: &State, window: iced_core::window::Id) -> Option<Theme> {
        (self)(state, window).into()
    }
}

use iced_program::Program;

#[derive(Debug)]
pub struct Daemon<A: Program> {
    raw: A,
    settings: Settings,
    namespace: String,
}

pub fn daemon<State, Message, Theme, Renderer>(
    boot: impl BootFn<State, Message>,
    namespace: impl NameSpace,
    update: impl UpdateFn<State, Message>,
    view: impl for<'a> self::ViewFn<'a, State, Message, Theme, Renderer>,
) -> Daemon<impl Program<Message = Message, Theme = Theme, State = State>>
where
    State: 'static,
    Message:
        'static + TryInto<LayershellCustomActionWithId, Error = Message> + Send + std::fmt::Debug,
    Theme: DefaultStyle,
    Renderer: iced_program::Renderer,
{
    use std::marker::PhantomData;
    struct Instance<State, Message, Theme, Renderer, UpdateFn, ViewFn, BootFn> {
        update: UpdateFn,
        view: ViewFn,
        boot: BootFn,
        _state: PhantomData<State>,
        _message: PhantomData<Message>,
        _theme: PhantomData<Theme>,
        _renderer: PhantomData<Renderer>,
    }
    impl<State, Message, Theme, Renderer, UpdateFn, ViewFn, BootFn> Program
        for Instance<State, Message, Theme, Renderer, UpdateFn, ViewFn, BootFn>
    where
        Message: 'static
            + TryInto<LayershellCustomActionWithId, Error = Message>
            + Send
            + std::fmt::Debug,
        Theme: DefaultStyle,
        Renderer: iced_program::Renderer,
        UpdateFn: self::UpdateFn<State, Message>,
        BootFn: self::BootFn<State, Message>,
        ViewFn: for<'a> self::ViewFn<'a, State, Message, Theme, Renderer>,
    {
        type State = State;
        type Renderer = Renderer;
        type Message = Message;
        type Theme = Theme;
        type Executor = iced_futures::backend::default::Executor;

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
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
            self.view.view(state, window)
        }

        fn name() -> &'static str {
            let name = std::any::type_name::<State>();

            name.split("::").next().unwrap_or("a_cool_application")
        }
        fn settings(&self) -> iced_core::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }
    Daemon {
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

pub fn with_subscription<P: Program>(
    program: P,
    f: impl Fn(&P::State) -> iced_futures::Subscription<P::Message>,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithSubscription<P, F> {
        program: P,
        subscription: F,
    }

    impl<P: Program, F> Program for WithSubscription<P, F>
    where
        F: Fn(&P::State) -> iced_futures::Subscription<P::Message>,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;
        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            self.program.title(state, window)
        }
        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            (self.subscription)(state)
        }
        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn name() -> &'static str {
            P::name()
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

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, id)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            self.program.scale_factor(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
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
    f: impl Fn(&P::State, iced_core::window::Id) -> Option<P::Theme>,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithTheme<P, F> {
        program: P,
        theme: F,
    }

    impl<P: Program, F> Program for WithTheme<P, F>
    where
        F: Fn(&P::State, iced_core::window::Id) -> Option<P::Theme>,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;
        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            self.program.title(state, window)
        }
        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Option<Self::Theme> {
            (self.theme)(state, id)
        }

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
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            self.program.scale_factor(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
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
        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            self.program.title(state, window)
        }
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
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            self.program.scale_factor(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
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
    f: impl Fn(&P::State, iced_core::window::Id) -> f32,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithScaleFactor<P, F> {
        program: P,
        scale_factor: F,
    }

    impl<P: Program, F> Program for WithScaleFactor<P, F>
    where
        F: Fn(&P::State, iced_core::window::Id) -> f32,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;

        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            self.program.title(state, window)
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
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, id)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            (self.scale_factor)(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
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

/// Decorates a [`Program`] with the given title function.
pub fn with_title<P: Program>(
    program: P,
    title: impl Fn(&P::State, iced_core::window::Id) -> Option<String>,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithTitle<P, Title> {
        program: P,
        title: Title,
    }

    impl<P, Title> Program for WithTitle<P, Title>
    where
        P: Program,
        Title: Fn(&P::State, iced_core::window::Id) -> Option<String>,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;

        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            let title = (self.title)(state, window);
            title.unwrap_or_else(|| self.program.title(state, window))
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
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, window)
        }

        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> iced_core::theme::Style {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            self.program.scale_factor(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
            Default::default()
        }

        fn window(&self) -> Option<iced_core::window::Settings> {
            None
        }
    }

    WithTitle { program, title }
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

        fn name() -> &'static str {
            P::name()
        }
        fn title(&self, state: &Self::State, window: iced_core::window::Id) -> String {
            self.program.title(state, window)
        }
        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
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

        fn subscription(&self, state: &Self::State) -> iced_futures::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Option<Self::Theme> {
            self.program.theme(state, id)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f32 {
            self.program.scale_factor(state, window)
        }
        fn settings(&self) -> iced_core::Settings {
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

impl<P: Program> Daemon<P> {
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
        let renderer_settings = iced_graphics::Settings {
            default_font: settings.default_font,
            default_text_size: settings.default_text_size,
            antialiasing: if settings.antialiasing {
                Some(iced_graphics::Antialiasing::MSAAx4)
            } else {
                None
            },
            ..Default::default()
        };
        use layershellev::StartMode;
        assert!(
            settings.layer_settings.size.is_some()
                || matches!(settings.layer_settings.start_mode, StartMode::Background),
            "Size must be specified unless start_mode is Background"
        );
        crate::multi_window::run(program, &self.namespace, settings, renderer_settings)
    }

    pub fn settings(self, settings: Settings) -> Self {
        Self { settings, ..self }
    }

    /// Sets the [`Settings::antialiasing`] of the [`Daemon`].
    pub fn antialiasing(self, antialiasing: bool) -> Self {
        Self {
            settings: Settings {
                antialiasing,
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the default [`Font`] of the [`Daemon`].
    pub fn default_font(self, default_font: Font) -> Self {
        Self {
            settings: Settings {
                default_font,
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the layershell setting of the [`Daemon`]
    pub fn layer_settings(self, layer_settings: LayerShellSettings) -> Self {
        Self {
            settings: Settings {
                layer_settings,
                ..self.settings
            },
            ..self
        }
    }

    /// Adds a font to the list of fonts that will be loaded at the start of the [`Daemon`].
    pub fn font(mut self, font: impl Into<Cow<'static, [u8]>>) -> Self {
        self.settings.fonts.push(font.into());
        self
    }

    /// set the default_text_size
    pub fn default_text_size<Pixels: Into<iced_core::Pixels>>(
        self,
        default_text_size: Pixels,
    ) -> Self {
        Self {
            settings: Settings {
                default_text_size: default_text_size.into(),
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the style logic of the [`Daemon`].
    pub fn style(
        self,
        f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_style(self.raw, move |state, theme| f(state, theme)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
    /// Sets the subscription logic of the [`Daemon`].
    pub fn subscription(
        self,
        f: impl Fn(&P::State) -> iced_futures::Subscription<P::Message>,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_subscription(self.raw, move |state| f(state)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }

    /// Sets the subscription logic of the [`Daemon`].
    pub fn title(
        self,
        f: impl Fn(&P::State, iced_core::window::Id) -> Option<String>,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_title(self.raw, move |state, id| f(state, id)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }

    /// Sets the theme logic of the [`Daemon`].
    pub fn theme(
        self,
        f: impl ThemeFn<P::State, P::Theme>,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_theme(self.raw, move |state, id| f.theme(state, id)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }

    /// Sets the scale factor of the [`Daemon`].
    pub fn scale_factor(
        self,
        f: impl Fn(&P::State, iced_core::window::Id) -> f32,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_scale_factor(self.raw, move |state, id| f(state, id)),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
    /// Sets the executor of the [`Daemon`].
    pub fn executor<E>(
        self,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    where
        E: iced_futures::Executor,
    {
        Daemon {
            raw: with_executor::<P, E>(self.raw),
            settings: self.settings,
            namespace: self.namespace,
        }
    }
}
