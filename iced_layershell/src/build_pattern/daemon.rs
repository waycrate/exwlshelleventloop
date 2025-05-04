use std::borrow::Cow;

use iced::Font;
use iced::{Element, Task};

use crate::actions::LayershellCustomActionsWithId;

use crate::DefaultStyle;
use crate::settings::LayerShellSettings;

use super::Renderer;

use crate::Result;

use crate::settings::Settings;

use iced_exdevtools::multilayershell_dev_generate;

multilayershell_dev_generate! {
    Type = DevTools,
    Program = Program
}

impl<P> TryInto<LayershellCustomActionsWithId> for Event<P>
where
    P: Program,
{
    type Error = Self;
    fn try_into(self) -> std::result::Result<LayershellCustomActionsWithId, Self::Error> {
        let Event::Program(message) = self else {
            return Err(self);
        };

        let message: std::result::Result<LayershellCustomActionsWithId, P::Message> =
            message.try_into();

        match message {
            Ok(action) => Ok(action),
            Err(message) => Err(Self::Program(message)),
        }
    }
}

#[allow(unused)]
fn attach(program: impl Program + 'static) -> impl Program {
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

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            state.update(&self.program, message)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            state.view(&self.program, window)
        }

        fn namespace(&self) -> String {
            self.program.namespace()
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            state.subscription(&self.program)
        }

        fn theme(&self, state: &Self::State, window: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state.state(), window)
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            window: iced_core::window::Id,
        ) -> iced::theme::Style {
            self.program.style(state.state(), theme, window)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state.state(), window)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state.state_mut(), id)
        }
    }

    Attach { program }
}

// layershell application
pub trait Program: Sized {
    /// The [`Executor`] that will run commands and subscriptions.
    ///
    /// The [default executor] can be a good starting point!
    ///
    /// [`Executor`]: Self::Executor
    /// [default executor]: iced::executor::Default
    type Executor: iced::Executor;
    type State;
    type Renderer: Renderer;

    /// The type of __messages__ your [`Application`] will produce.
    type Message: std::fmt::Debug
        + Send
        + 'static
        + TryInto<LayershellCustomActionsWithId, Error = Self::Message>;

    /// The theme of your [`Application`].
    type Theme: Default + DefaultStyle;
    fn remove_id(&self, _state: &mut Self::State, _id: iced_core::window::Id);
    /// Initializes the [`Application`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Task`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    ///
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn namespace(&self) -> String {
        "A cool iced application".to_string()
    }

    fn name() -> &'static str;
    fn boot(&self) -> (Self::State, Task<Self::Message>);
    ///
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Task`] returned will be executed immediately in the background.
    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message>;

    /// Returns the widgets to display in the [`Application`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view<'a>(
        &self,
        state: &'a Self::State,
        _window: iced_core::window::Id,
    ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer>;

    /// Returns the current [`Theme`] of the [`Application`].
    ///
    /// [`Theme`]: Self::Theme
    fn theme(&self, _state: &Self::State, _id: iced_core::window::Id) -> Self::Theme {
        Self::Theme::default()
    }

    /// Returns the current `Style` of the [`Theme`].
    ///
    /// [`Theme`]: Self::Theme
    fn style(
        &self,
        _state: &Self::State,
        theme: &Self::Theme,
        _id: iced_core::window::Id,
    ) -> crate::Appearance {
        theme.base()
    }

    /// Returns the event [`Subscription`] for the current state of the
    /// application.
    ///
    /// A [`Subscription`] will be kept alive as long as you keep returning it,
    /// and the __messages__ produced will be handled by
    /// [`update`](#tymethod.update).
    ///
    /// By default, this method returns an empty [`Subscription`].
    fn subscription(&self, _state: &Self::State) -> iced::Subscription<Self::Message> {
        iced::Subscription::none()
    }

    /// Returns the scale factor of the [`Application`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self, _state: &Self::State, _window: iced_core::window::Id) -> f64 {
        1.0
    }

    fn run(self, settings: Settings) -> Result
    where
        Self: 'static,
    {
        #[cfg(all(feature = "debug", not(target_arch = "wasm32")))]
        let program = {
            iced_debug::init(iced_debug::Metadata {
                name: Self::name(),
                theme: None,
                can_time_travel: cfg!(feature = "time-travel"),
            });

            attach(self)
        };

        #[cfg(any(not(feature = "debug"), target_arch = "wasm32"))]
        let program = self;
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
}

pub trait NameSpace {
    /// Produces the title of the [`Application`].
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

pub trait RemoveId<State> {
    fn remove_id(&self, state: &mut State, id: iced_core::window::Id);
}

impl<T, State> RemoveId<State> for T
where
    T: Fn(&mut State, iced_core::window::Id),
{
    fn remove_id(&self, state: &mut State, id: iced_core::window::Id) {
        self(state, id)
    }
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
pub struct Daemon<A: Program> {
    raw: A,
    settings: Settings,
}

pub fn daemon<State, Message, Theme, Renderer>(
    boot: impl Boot<State, Message>,
    namespace: impl NameSpace,
    update: impl Update<State, Message>,
    view: impl for<'a> self::View<'a, State, Message, Theme, Renderer>,
    remove_id: impl RemoveId<State>,
) -> Daemon<impl Program<Message = Message, Theme = Theme, State = State>>
where
    State: 'static,
    Message:
        'static + TryInto<LayershellCustomActionsWithId, Error = Message> + Send + std::fmt::Debug,
    Theme: Default + DefaultStyle,
    Renderer: self::Renderer,
{
    use std::marker::PhantomData;
    struct Instance<State, Message, Theme, Renderer, Update, View, RemoveId, Boot> {
        update: Update,
        view: View,
        remove_id: RemoveId,
        boot: Boot,
        _state: PhantomData<State>,
        _message: PhantomData<Message>,
        _theme: PhantomData<Theme>,
        _renderer: PhantomData<Renderer>,
    }
    impl<State, Message, Theme, Renderer, Update, View, RemoveId, Boot> Program
        for Instance<State, Message, Theme, Renderer, Update, View, RemoveId, Boot>
    where
        Message: 'static
            + TryInto<LayershellCustomActionsWithId, Error = Message>
            + Send
            + std::fmt::Debug,
        Theme: Default + DefaultStyle,
        Renderer: self::Renderer,
        Update: self::Update<State, Message>,
        RemoveId: self::RemoveId<State>,
        Boot: self::Boot<State, Message>,
        View: for<'a> self::View<'a, State, Message, Theme, Renderer>,
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

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.remove_id.remove_id(state, id);
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
    Daemon {
        raw: Instance {
            update,
            view,
            remove_id,
            boot,

            _state: PhantomData,
            _message: PhantomData,
            _theme: PhantomData,
            _renderer: PhantomData,
        },
        settings: Settings::default(),
    }
    .namespace(namespace)
}

fn with_namespace<P: Program>(
    program: P,
    namespace: impl Fn() -> String,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithNamespace<P, NameSpace> {
        program: P,
        namespace: NameSpace,
    }
    impl<P, Namespace> Program for WithNamespace<P, Namespace>
    where
        P: Program,
        Namespace: Fn() -> String,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;
        fn namespace(&self) -> String {
            (self.namespace)()
        }

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }

        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.program.view(state, window)
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state, id)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn name() -> &'static str {
            P::name()
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            self.program.style(state, theme, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
        }
    }

    WithNamespace { program, namespace }
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
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
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

        fn namespace(&self) -> String {
            self.program.namespace()
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state, id)
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            self.program.style(state, theme, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
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

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            (self.theme)(state, id)
        }
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }
        fn namespace(&self) -> String {
            self.program.namespace()
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

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            self.program.style(state, theme, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
        }
    }

    WithTheme { program, theme: f }
}

pub fn with_style<P: Program>(
    program: P,
    f: impl Fn(&P::State, &P::Theme, iced_core::window::Id) -> crate::Appearance,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme> {
    struct WithStyle<P, F> {
        program: P,
        style: F,
    }

    impl<P: Program, F> Program for WithStyle<P, F>
    where
        F: Fn(&P::State, &P::Theme, iced_core::window::Id) -> crate::Appearance,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            (self.style)(state, theme, id)
        }

        fn namespace(&self) -> String {
            self.program.namespace()
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
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
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

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
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

        fn namespace(&self) -> String {
            self.program.namespace()
        }
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
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

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            self.program.subscription(state)
        }

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state, id)
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            self.program.style(state, theme, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            (self.scale_factor)(state, window)
        }
    }

    WithScaleFactor {
        program,
        scale_factor: f,
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

        fn namespace(&self) -> String {
            self.program.namespace()
        }
        fn name() -> &'static str {
            P::name()
        }
        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.program.update(state, message)
        }
        fn boot(&self) -> (Self::State, Task<Self::Message>) {
            self.program.boot()
        }
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
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

        fn theme(&self, state: &Self::State, id: iced_core::window::Id) -> Self::Theme {
            self.program.theme(state, id)
        }

        fn style(
            &self,
            state: &Self::State,
            theme: &Self::Theme,
            id: iced_core::window::Id,
        ) -> crate::Appearance {
            self.program.style(state, theme, id)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
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
    {
        self.raw.run(self.settings)
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

    pub fn layer_settings(self, layer_settings: LayerShellSettings) -> Self {
        Self {
            settings: Settings {
                layer_settings,
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

    pub fn namespace(
        self,
        namespace: impl NameSpace,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_namespace(self.raw, move || namespace.namespace()),
            settings: self.settings,
        }
    }
    /// Sets the style logic of the [`Application`].
    pub fn style(
        self,
        f: impl Fn(&P::State, &P::Theme, iced_core::window::Id) -> crate::Appearance,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_style(self.raw, f),
            settings: self.settings,
        }
    }
    /// Sets the subscription logic of the [`Application`].
    pub fn subscription(
        self,
        f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_subscription(self.raw, f),
            settings: self.settings,
        }
    }

    /// Sets the theme logic of the [`Application`].
    pub fn theme(
        self,
        f: impl Fn(&P::State, iced_core::window::Id) -> P::Theme,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_theme(self.raw, f),
            settings: self.settings,
        }
    }

    /// Sets the scale factor of the [`Application`].
    pub fn scale_factor(
        self,
        f: impl Fn(&P::State, iced_core::window::Id) -> f64,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>> {
        Daemon {
            raw: with_scale_factor(self.raw, f),
            settings: self.settings,
        }
    }
    /// Sets the executor of the [`Application`].
    pub fn executor<E>(
        self,
    ) -> Daemon<impl Program<State = P::State, Message = P::Message, Theme = P::Theme>>
    where
        E: iced_futures::Executor,
    {
        Daemon {
            raw: with_executor::<P, E>(self.raw),
            settings: self.settings,
        }
    }
}

use iced::Subscription;
use iced::theme;
use iced::window;

#[allow(missing_debug_implementations)]
pub struct Instance<P: Program> {
    program: P,
    state: P::State,
}

impl<P: Program> Instance<P> {
    /// Creates a new [`Instance`] of the given [`Program`].
    pub fn new(program: P) -> (Self, Task<P::Message>) {
        let (state, task) = program.boot();

        (Self { program, state }, task)
    }

    /// Returns the current title of the [`Instance`].
    pub fn namespace(&self) -> String {
        self.program.namespace()
    }

    /// Processes the given message and updates the [`Instance`].
    pub fn update(&mut self, message: P::Message) -> Task<P::Message> {
        self.program.update(&mut self.state, message)
    }

    /// Produces the current widget tree of the [`Instance`].
    pub fn view(&self, window: window::Id) -> Element<'_, P::Message, P::Theme, P::Renderer> {
        self.program.view(&self.state, window)
    }

    /// Returns the current [`Subscription`] of the [`Instance`].
    pub fn subscription(&self) -> Subscription<P::Message> {
        self.program.subscription(&self.state)
    }

    /// Returns the current theme of the [`Instance`].
    pub fn theme(&self, window: window::Id) -> P::Theme {
        self.program.theme(&self.state, window)
    }

    /// Returns the current [`theme::Style`] of the [`Instance`].
    pub fn style(&self, theme: &P::Theme, window: window::Id) -> theme::Style {
        self.program.style(&self.state, theme, window)
    }

    /// Returns the current scale factor of the [`Instance`].
    pub fn scale_factor(&self, window: window::Id) -> f64 {
        self.program.scale_factor(&self.state, window)
    }

    pub fn remove_id(&mut self, id: iced_core::window::Id) {
        self.program.remove_id(&mut self.state, id);
    }
}
