pub mod actions;
mod clipboard;
mod conversion;
mod error;
mod event;
pub mod multi_window;
mod proxy;

pub mod settings;

use actions::UnLockAction;
use settings::Settings;

pub use error::Error;

use iced::{Color, Element, Theme};
use iced_futures::Subscription;
use iced_runtime::Task;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    /// The background [`Color`] of the application.
    pub background_color: Color,

    /// The default text [`Color`] of the application.
    pub text_color: Color,
}

/// The default style of a [`Program`].
pub trait DefaultStyle {
    /// Returns the default style of a [`Program`].
    fn default_style(&self) -> Appearance;
}

impl DefaultStyle for Theme {
    fn default_style(&self) -> Appearance {
        default(self)
    }
}

/// The default [`Appearance`] of a [`Program`] with the built-in [`Theme`].
pub fn default(theme: &Theme) -> Appearance {
    let palette = theme.extended_palette();

    Appearance {
        background_color: palette.background.base.color,
        text_color: palette.background.base.text,
    }
}

pub trait MultiApplication: Sized {
    /// The [`Executor`] that will run commands and subscriptions.
    ///
    /// The [default executor] can be a good starting point!
    ///
    /// [`Executor`]: Self::Executor
    /// [default executor]: iced::executor::Default
    type Executor: iced::Executor;

    /// The type of __messages__ your [`MultiApplication`] will produce.
    type Message: std::fmt::Debug + Send;

    /// The data needed to initialize your [`MultiApplication`].
    type Flags;

    type Theme: Default + DefaultStyle;

    /// Initializes the [`MultiApplication`] with the flags provided to
    /// [`run`] as part of the [`Settings`].
    ///
    /// Here is where you should return the initial state of your app.
    ///
    /// Additionally, you can return a [`Command`] if you need to perform some
    /// async action in the background on startup. This is useful if you want to
    /// load state from a file, perform an initial HTTP request, etc.
    ///
    /// [`run`]: Self::run
    fn new(flags: Self::Flags) -> (Self, Task<Self::Message>);

    /// Returns the current title of the `window` of the [`MultiApplication`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your window when necessary.
    fn namespace(&self) -> String;

    /// Handles a __message__ and updates the state of the [`MultiApplication`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Command`] returned will be executed immediately in the background.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message>;

    /// Returns the widgets to display in the `window` of the [`MultiApplication`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(
        &self,
        window: iced::window::Id,
    ) -> Element<'_, Self::Message, Self::Theme, iced::Renderer>;

    /// Returns the current [`Theme`] of the `window` of the [`MultiApplication`].
    ///
    /// [`Theme`]: Self::Theme
    #[allow(unused_variables)]
    fn theme(&self) -> Self::Theme {
        Self::Theme::default()
    }

    /// Returns the current `Style` of the [`Theme`].
    ///
    /// [`Theme`]: Self::Theme

    fn style(&self, theme: &Self::Theme) -> Appearance {
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
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::none()
    }

    /// Returns the scale factor of the `window` of the [`MultiApplication`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    #[allow(unused_variables)]
    fn scale_factor(&self, window: iced::window::Id) -> f64 {
        1.0
    }

    /// Runs the multi-window [`MultiApplication`].
    ///
    /// On native platforms, this method will take control of the current thread
    /// until the [`MultiApplication`] exits.
    ///
    /// On the web platform, this method __will NOT return__ unless there is an
    /// [`Error`] during startup.
    ///
    /// [`Error`]: crate::Error
    fn run(settings: Settings<Self::Flags>) -> Result<(), error::Error>
    where
        Self: 'static,
        Self::Message: 'static + TryInto<UnLockAction, Error = Self::Message>,
    {
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

        multi_window::run::<MultiInstance<Self>, Self::Executor, iced_renderer::Compositor>(
            settings,
            renderer_settings,
        )
    }
}

struct MultiInstance<A: MultiApplication>(A);

impl<A> iced_runtime::multi_window::Program for MultiInstance<A>
where
    A: MultiApplication,
{
    type Message = A::Message;
    type Theme = A::Theme;
    type Renderer = iced_renderer::Renderer;

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        self.0.update(message)
    }

    fn view(
        &self,
        window: iced::window::Id,
    ) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        self.0.view(window)
    }
}

impl<A> multi_window::Application for MultiInstance<A>
where
    A: MultiApplication,
{
    type Flags = A::Flags;

    fn new(flags: Self::Flags) -> (Self, Task<A::Message>) {
        let (app, command) = A::new(flags);

        (MultiInstance(app), command)
    }

    fn namespace(&self) -> String {
        self.0.namespace()
    }

    fn theme(&self) -> A::Theme {
        self.0.theme()
    }

    fn style(&self, theme: &Self::Theme) -> Appearance {
        self.0.style(theme)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        self.0.subscription()
    }

    fn scale_factor(&self, window: iced::window::Id) -> f64 {
        self.0.scale_factor(window)
    }
}
