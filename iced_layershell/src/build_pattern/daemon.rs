use std::borrow::Cow;

use iced::Font;
use iced::{Element, Task};

use crate::actions::{IsSingleton, LayershellCustomActionsWithIdAndInfo};

use crate::settings::LayerShellSettings;
use crate::DefaultStyle;

use super::Renderer;
use crate::Settings;

use crate::Result;

use super::MainSettings;

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

    type WindowInfo: Clone + PartialEq + IsSingleton;
    /// The type of __messages__ your [`Application`] will produce.
    type Message: std::fmt::Debug
        + Send
        + 'static
        + TryInto<LayershellCustomActionsWithIdAndInfo<Self::WindowInfo>, Error = Self::Message>;

    /// The theme of your [`Application`].
    type Theme: Default + DefaultStyle;
    fn id_info(&self, _state: &Self::State, _id: iced_core::window::Id)
        -> Option<Self::WindowInfo>;

    fn set_id_info(
        &self,
        _state: &mut Self::State,
        _id: iced_core::window::Id,
        _info: Self::WindowInfo,
    );
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
    fn namespace(&self, _state: &Self::State) -> String {
        "A cool iced application".to_string()
    }

    /// Handles a __message__ and updates the state of the [`Application`].
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
    fn theme(&self, _state: &Self::State) -> Self::Theme {
        Self::Theme::default()
    }

    /// Returns the current `Style` of the [`Theme`].
    ///
    /// [`Theme`]: Self::Theme
    fn style(&self, _state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
        theme.default_style()
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

    fn run_with<I>(self, settings: MainSettings, initialize: I) -> Result
    where
        Self: 'static,
        I: FnOnce() -> (Self::State, Task<Self::Message>) + 'static,
    {
        use std::marker::PhantomData;
        struct Instance<P: Program, I> {
            program: P,
            state: P::State,
            _initialize: PhantomData<I>,
        }

        impl<P: Program, I: FnOnce() -> (P::State, Task<P::Message>)>
            iced_runtime::multi_window::Program for Instance<P, I>
        {
            type Message = P::Message;
            type Theme = P::Theme;
            type Renderer = P::Renderer;
            fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
                self.program.update(&mut self.state, message)
            }

            fn view(
                &self,
                window: iced_core::window::Id,
            ) -> crate::Element<'_, Self::Message, Self::Theme, Self::Renderer> {
                self.program.view(&self.state, window)
            }
        }

        impl<P: Program, I: FnOnce() -> (P::State, Task<P::Message>)>
            crate::multi_window::Application for Instance<P, I>
        {
            type Flags = (P, I);
            type WindowInfo = P::WindowInfo;

            fn new((program, initialize): Self::Flags) -> (Self, Task<Self::Message>) {
                let (state, task) = initialize();

                (
                    Self {
                        program,
                        state,
                        _initialize: PhantomData,
                    },
                    task,
                )
            }

            fn id_info(&self, id: iced_core::window::Id) -> Option<Self::WindowInfo> {
                self.program.id_info(&self.state, id)
            }
            fn set_id_info(&mut self, id: iced_core::window::Id, info: Self::WindowInfo) {
                self.program.set_id_info(&mut self.state, id, info)
            }
            fn remove_id(&mut self, id: iced_core::window::Id) {
                self.program.remove_id(&mut self.state, id)
            }
            fn namespace(&self) -> String {
                self.program.namespace(&self.state)
            }

            fn subscription(&self) -> iced::Subscription<Self::Message> {
                self.program.subscription(&self.state)
            }

            fn theme(&self) -> Self::Theme {
                self.program.theme(&self.state)
            }

            fn style(&self, theme: &Self::Theme) -> crate::Appearance {
                self.program.style(&self.state, theme)
            }

            fn scale_factor(&self, window: iced_core::window::Id) -> f64 {
                self.program.scale_factor(&self.state, window)
            }
        }

        let real_settings = Settings {
            flags: (self, initialize),
            id: settings.id,
            default_font: settings.default_font,
            layer_settings: settings.layer_settings,
            fonts: settings.fonts,
            default_text_size: settings.default_text_size,
            antialiasing: settings.antialiasing,
            virtual_keyboard_support: settings.virtual_keyboard_support,
        };
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

        crate::multi_window::run::<
            Instance<Self, I>,
            Self::Executor,
            <Self::Renderer as iced_graphics::compositor::Default>::Compositor,
        >(real_settings, renderer_settings)
    }

    fn run(self, settings: MainSettings) -> Result
    where
        Self: 'static,
        Self::State: Default,
    {
        self.run_with(settings, || (Self::State::default(), Task::none()))
    }
}

pub trait NameSpace<State> {
    /// Produces the title of the [`Application`].
    fn namespace(&self, state: &State) -> String;
}

impl<State> NameSpace<State> for &'static str {
    fn namespace(&self, _state: &State) -> String {
        self.to_string()
    }
}

impl<T, State> NameSpace<State> for T
where
    T: Fn(&State) -> String,
{
    fn namespace(&self, state: &State) -> String {
        self(state)
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

pub trait IdInfo<State, WindowInfo> {
    fn id_info(&self, state: &State, id: iced_core::window::Id) -> Option<WindowInfo>;
}

impl<T, State, WindowInfo> IdInfo<State, WindowInfo> for T
where
    T: Fn(&State, iced_core::window::Id) -> Option<WindowInfo>,
{
    fn id_info(&self, state: &State, id: iced_core::window::Id) -> Option<WindowInfo> {
        self(state, id)
    }
}
pub trait SetIdInfo<State, WindowInfo> {
    fn set_id_info(&self, state: &mut State, id: iced_core::window::Id, window_info: WindowInfo);
}

impl<T, State, WindowInfo> SetIdInfo<State, WindowInfo> for T
where
    T: Fn(&mut State, iced_core::window::Id, WindowInfo),
{
    fn set_id_info(&self, state: &mut State, id: iced_core::window::Id, window_info: WindowInfo) {
        self(state, id, window_info)
    }
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

#[derive(Debug)]
pub struct Daemon<A: Program> {
    raw: A,
    settings: MainSettings,
}

pub fn daemon<State, Message, Theme, Renderer, WindowInfo>(
    namespace: impl NameSpace<State>,
    update: impl Update<State, Message>,
    view: impl for<'a> self::View<'a, State, Message, Theme, Renderer>,
    id_info: impl IdInfo<State, WindowInfo>,
    set_id_info: impl SetIdInfo<State, WindowInfo>,
    remove_id: impl RemoveId<State>,
) -> Daemon<impl Program<Message = Message, Theme = Theme, State = State, WindowInfo = WindowInfo>>
where
    State: 'static,
    WindowInfo: Clone + PartialEq + IsSingleton,
    Message: 'static
        + TryInto<LayershellCustomActionsWithIdAndInfo<WindowInfo>, Error = Message>
        + Send
        + std::fmt::Debug,
    Theme: Default + DefaultStyle,
    Renderer: self::Renderer,
{
    use std::marker::PhantomData;
    struct Instance<
        State,
        Message,
        Theme,
        Renderer,
        Update,
        View,
        IdInfo,
        SetIdInfo,
        RemoveId,
        WindowInfo,
    > {
        update: Update,
        view: View,
        id_info: IdInfo,
        set_id_info: SetIdInfo,
        remove_id: RemoveId,
        _state: PhantomData<State>,
        _message: PhantomData<Message>,
        _theme: PhantomData<Theme>,
        _renderer: PhantomData<Renderer>,
        _windowinfo: PhantomData<WindowInfo>,
    }
    impl<
            State,
            Message,
            Theme,
            Renderer,
            Update,
            View,
            WindowInfo,
            IdInfo,
            SetIdInfo,
            RemoveId,
        > Program
        for Instance<
            State,
            Message,
            Theme,
            Renderer,
            Update,
            View,
            IdInfo,
            SetIdInfo,
            RemoveId,
            WindowInfo,
        >
    where
        WindowInfo: Clone + PartialEq + IsSingleton,
        Message: 'static
            + TryInto<LayershellCustomActionsWithIdAndInfo<WindowInfo>, Error = Message>
            + Send
            + std::fmt::Debug,
        Theme: Default + DefaultStyle,
        IdInfo: self::IdInfo<State, WindowInfo>,
        SetIdInfo: self::SetIdInfo<State, WindowInfo>,
        RemoveId: self::RemoveId<State>,
        Renderer: self::Renderer,
        Update: self::Update<State, Message>,
        View: for<'a> self::View<'a, State, Message, Theme, Renderer>,
    {
        type State = State;
        type Renderer = Renderer;
        type Message = Message;
        type Theme = Theme;
        type WindowInfo = WindowInfo;
        type Executor = iced_futures::backend::default::Executor;

        fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
            self.update.update(state, message).into()
        }
        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.id_info.id_info(state, id)
        }
        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.remove_id.remove_id(state, id)
        }
        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.set_id_info.set_id_info(state, id, info)
        }

        fn view<'a>(
            &self,
            state: &'a Self::State,
            window: iced_core::window::Id,
        ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
            self.view.view(state, window).into()
        }
    }
    Daemon {
        raw: Instance {
            update,
            view,
            id_info,
            set_id_info,
            remove_id,

            _state: PhantomData,
            _message: PhantomData,
            _theme: PhantomData,
            _renderer: PhantomData,
            _windowinfo: PhantomData,
        },
        settings: MainSettings::default(),
    }
    .namespace(namespace)
}

fn with_namespace<P: Program>(
    program: P,
    namespace: impl Fn(&P::State) -> String,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
    struct WithNamespace<P, NameSpace> {
        program: P,
        namespace: NameSpace,
    }
    impl<P, Namespace> Program for WithNamespace<P, Namespace>
    where
        P: Program,
        Namespace: Fn(&P::State) -> String,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;
        type WindowInfo = P::WindowInfo;

        fn namespace(&self, state: &Self::State) -> String {
            (self.namespace)(state)
        }

        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
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

        fn theme(&self, state: &Self::State) -> Self::Theme {
            self.program.theme(state)
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
    }

    WithNamespace { program, namespace }
}

pub fn with_subscription<P: Program>(
    program: P,
    f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
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
        type WindowInfo = P::WindowInfo;
        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
        }

        fn subscription(&self, state: &Self::State) -> iced::Subscription<Self::Message> {
            (self.subscription)(state)
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

        fn namespace(&self, state: &Self::State) -> String {
            self.program.namespace(state)
        }

        fn theme(&self, state: &Self::State) -> Self::Theme {
            self.program.theme(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
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
    f: impl Fn(&P::State) -> P::Theme,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
    struct WithTheme<P, F> {
        program: P,
        theme: F,
    }

    impl<P: Program, F> Program for WithTheme<P, F>
    where
        F: Fn(&P::State) -> P::Theme,
    {
        type State = P::State;
        type Message = P::Message;
        type Theme = P::Theme;
        type Renderer = P::Renderer;
        type Executor = P::Executor;
        type WindowInfo = P::WindowInfo;
        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
        }

        fn theme(&self, state: &Self::State) -> Self::Theme {
            (self.theme)(state)
        }

        fn namespace(&self, state: &Self::State) -> String {
            self.program.namespace(state)
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

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
        }

        fn scale_factor(&self, state: &Self::State, window: iced_core::window::Id) -> f64 {
            self.program.scale_factor(state, window)
        }
    }

    WithTheme { program, theme: f }
}

pub fn with_style<P: Program>(
    program: P,
    f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
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
        type WindowInfo = P::WindowInfo;
        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            (self.style)(state, theme)
        }

        fn namespace(&self, state: &Self::State) -> String {
            self.program.namespace(state)
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

        fn theme(&self, state: &Self::State) -> Self::Theme {
            self.program.theme(state)
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
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
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
        type WindowInfo = P::WindowInfo;
        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
        }

        fn namespace(&self, state: &Self::State) -> String {
            self.program.namespace(state)
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

        fn theme(&self, state: &Self::State) -> Self::Theme {
            self.program.theme(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
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
) -> impl Program<State = P::State, Message = P::Message, Theme = P::Theme, WindowInfo = P::WindowInfo>
{
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
        type WindowInfo = P::WindowInfo;

        fn id_info(
            &self,
            state: &Self::State,
            id: iced_core::window::Id,
        ) -> Option<Self::WindowInfo> {
            self.program.id_info(state, id)
        }

        fn remove_id(&self, state: &mut Self::State, id: iced_core::window::Id) {
            self.program.remove_id(state, id)
        }

        fn set_id_info(
            &self,
            state: &mut Self::State,
            id: iced_core::window::Id,
            info: Self::WindowInfo,
        ) {
            self.program.set_id_info(state, id, info)
        }

        fn namespace(&self, state: &Self::State) -> String {
            self.program.namespace(state)
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

        fn theme(&self, state: &Self::State) -> Self::Theme {
            self.program.theme(state)
        }

        fn style(&self, state: &Self::State, theme: &Self::Theme) -> crate::Appearance {
            self.program.style(state, theme)
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
        P::State: Default,
    {
        self.raw.run(self.settings)
    }

    pub fn run_with<I>(self, initialize: I) -> Result
    where
        Self: 'static,
        I: FnOnce() -> (P::State, Task<P::Message>) + 'static,
    {
        self.raw.run_with(self.settings, initialize)
    }
    pub fn settings(self, settings: MainSettings) -> Self {
        Self { settings, ..self }
    }

    /// Sets the [`Settings::antialiasing`] of the [`Application`].
    pub fn antialiasing(self, antialiasing: bool) -> Self {
        Self {
            settings: MainSettings {
                antialiasing,
                ..self.settings
            },
            ..self
        }
    }

    /// Sets the default [`Font`] of the [`Application`].
    pub fn default_font(self, default_font: Font) -> Self {
        Self {
            settings: MainSettings {
                default_font,
                ..self.settings
            },
            ..self
        }
    }

    pub fn layer_settings(self, layer_settings: LayerShellSettings) -> Self {
        Self {
            settings: MainSettings {
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

    pub fn namespace(
        self,
        namespace: impl NameSpace<P::State>,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    > {
        Daemon {
            raw: with_namespace(self.raw, move |state| namespace.namespace(state)),
            settings: self.settings,
        }
    }
    /// Sets the style logic of the [`Application`].
    pub fn style(
        self,
        f: impl Fn(&P::State, &P::Theme) -> crate::Appearance,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    > {
        Daemon {
            raw: with_style(self.raw, f),
            settings: self.settings,
        }
    }
    /// Sets the subscription logic of the [`Application`].
    pub fn subscription(
        self,
        f: impl Fn(&P::State) -> iced::Subscription<P::Message>,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    > {
        Daemon {
            raw: with_subscription(self.raw, f),
            settings: self.settings,
        }
    }

    /// Sets the theme logic of the [`Application`].
    pub fn theme(
        self,
        f: impl Fn(&P::State) -> P::Theme,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    > {
        Daemon {
            raw: with_theme(self.raw, f),
            settings: self.settings,
        }
    }

    /// Sets the scale factor of the [`Application`].
    pub fn scale_factor(
        self,
        f: impl Fn(&P::State, iced_core::window::Id) -> f64,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    > {
        Daemon {
            raw: with_scale_factor(self.raw, f),
            settings: self.settings,
        }
    }
    /// Sets the executor of the [`Application`].
    pub fn executor<E>(
        self,
    ) -> Daemon<
        impl Program<
            State = P::State,
            Message = P::Message,
            Theme = P::Theme,
            WindowInfo = P::WindowInfo,
        >,
    >
    where
        E: iced_futures::Executor,
    {
        Daemon {
            raw: with_executor::<P, E>(self.raw),
            settings: self.settings,
        }
    }
}
