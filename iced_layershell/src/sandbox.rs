use iced::theme;
use iced::Element;
use iced_futures::Subscription;
use iced_runtime::Command;
use iced_style::Theme;

use crate::settings::Settings;
use crate::Application;
use crate::error;

pub trait LayerShellSandbox {
    /// The type of __messages__ your [`Sandbox`] will produce.
    type Message: std::fmt::Debug + Send;

    /// Initializes the [`Sandbox`].
    ///
    /// Here is where you should return the initial state of your app.
    fn new() -> Self;

    /// Returns the current title of the [`Sandbox`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn namespace(&self) -> String;

    /// Handles a __message__ and updates the state of the [`Sandbox`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by user interactions, will be handled by this method.
    fn update(&mut self, message: Self::Message);

    /// Returns the widgets to display in the [`Sandbox`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(&self) -> Element<'_, Self::Message>;

    /// Returns the current [`Theme`] of the [`Sandbox`].
    ///
    /// If you want to use your own custom theme type, you will have to use an
    /// [`Application`].
    ///
    /// By default, it returns [`Theme::default`].
    fn theme(&self) -> Theme {
        Theme::default()
    }

    /// Returns the current style variant of [`theme::Application`].
    ///
    /// By default, it returns [`theme::Application::default`].
    fn style(&self) -> theme::Application {
        theme::Application::default()
    }

    /// Returns the scale factor of the [`Sandbox`].
    ///
    /// It can be used to dynamically control the size of the UI at runtime
    /// (i.e. zooming).
    ///
    /// For instance, a scale factor of `2.0` will make widgets twice as big,
    /// while a scale factor of `0.5` will shrink them to half their size.
    ///
    /// By default, it returns `1.0`.
    fn scale_factor(&self) -> f64 {
        1.0
    }

    /// Runs the [`Sandbox`].
    ///
    /// On native platforms, this method will take control of the current thread
    /// and __will NOT return__.
    ///
    /// It should probably be that last thing you call in your `main` function.
    fn run(settings: Settings<()>) -> Result<(), error::Error>
    where
        Self: 'static + Sized,
    {
        <Self as Application>::run(settings)
    }
}

impl<T> Application for T
where
    T: LayerShellSandbox,
{
    type Executor = iced_futures::backend::null::Executor;
    type Flags = ();
    type Message = T::Message;
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<T::Message>) {
        (T::new(), Command::none())
    }

    fn namespace(&self) -> String {
        T::namespace(self)
    }

    fn update(&mut self, message: T::Message) -> Command<T::Message> {
        T::update(self, message);

        Command::none()
    }

    fn view(&self) -> Element<'_, T::Message> {
        T::view(self)
    }

    fn theme(&self) -> Self::Theme {
        T::theme(self)
    }

    fn style(&self) -> theme::Application {
        T::style(self)
    }

    fn subscription(&self) -> Subscription<T::Message> {
        Subscription::none()
    }

    fn scale_factor(&self) -> f64 {
        T::scale_factor(self)
    }
}
