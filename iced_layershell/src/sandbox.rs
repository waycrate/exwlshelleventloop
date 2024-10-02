use super::Appearance;
use super::DefaultStyle;
use iced::theme::Theme;
use iced::{Element, Task};
use iced_futures::Subscription;

use crate::actions::LayershellCustomActions;
use crate::error;
use crate::settings::Settings;
use crate::Application;

pub trait LayerShellSandbox {
    /// The type of __messages__ your [`LayerShellSandbox`] will produce.
    type Message: std::fmt::Debug + Send;

    /// Initializes the [`LayerShellSandbox`].
    ///
    /// Here is where you should return the initial state of your app.
    fn new() -> Self;

    /// Returns the current namespace of the [`LayerShellSandbox`].
    ///
    /// This title can be dynamic! The runtime will automatically update the
    /// title of your application when necessary.
    fn namespace(&self) -> String;

    /// Handles a __message__ and updates the state of the [`LayerShellSandbox`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by user interactions, will be handled by this method.
    fn update(&mut self, message: Self::Message);

    /// Returns the widgets to display in the [`LayerShellSandbox`].
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(&self) -> Element<'_, Self::Message>;

    /// Returns the current [`Theme`] of the [`LayerShellSandbox`].
    ///
    /// If you want to use your own custom theme type, you will have to use an
    /// [`Application`].
    ///
    /// By default, it returns [`Theme::default`].
    fn theme(&self) -> Theme {
        Theme::default()
    }

    /// Returns the current style variant of [`Appearance`].
    ///
    /// By default, it returns [`Theme::default_style()`].
    fn style(&self, theme: &Theme) -> Appearance {
        theme.default_style()
    }

    /// Returns the scale factor of the [`LayerShellSandbox`].
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

    /// Runs the [`LayerShellSandbox`].
    ///
    /// On native platforms, this method will take control of the current thread
    /// and __will NOT return__.
    ///
    /// It should probably be that last thing you call in your `main` function.
    fn run(settings: Settings<()>) -> Result<(), error::Error>
    where
        Self: 'static + Sized,
        Self::Message: 'static + TryInto<LayershellCustomActions, Error = Self::Message>,
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

    fn new(_flags: ()) -> (Self, Task<T::Message>) {
        (T::new(), Task::none())
    }

    fn namespace(&self) -> String {
        T::namespace(self)
    }

    fn update(&mut self, message: T::Message) -> Task<T::Message> {
        T::update(self, message);

        Task::none()
    }

    fn view(&self) -> Element<'_, T::Message> {
        T::view(self)
    }

    fn theme(&self) -> Self::Theme {
        T::theme(self)
    }

    fn style(&self, theme: &Self::Theme) -> Appearance {
        T::style(self, theme)
    }

    fn subscription(&self) -> Subscription<T::Message> {
        Subscription::none()
    }

    fn scale_factor(&self) -> f64 {
        T::scale_factor(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSandbox {
        counter: i32,
    }

    #[derive(Debug)]
    enum MockMessage {
        Increment,
        Decrement,
    }

    impl LayerShellSandbox for MockSandbox {
        type Message = MockMessage;

        fn new() -> Self {
            Self { counter: 0 }
        }

        fn namespace(&self) -> String {
            "MockSandbox".into()
        }

        fn scale_factor(&self) -> f64 {
            2.0
        }

        fn update(&mut self, message: Self::Message) {
            match message {
                MockMessage::Increment => self.counter += 1,
                MockMessage::Decrement => self.counter -= 1,
            }
        }

        fn view(&self) -> Element<'_, Self::Message> {
            iced::widget::text("Mock view").into()
        }
    }

    #[test]
    fn test_namespace() {
        let app = <MockSandbox as LayerShellSandbox>::new();
        assert_eq!(LayerShellSandbox::namespace(&app), "MockSandbox");
    }

    #[test]
    fn test_scale_factor() {
        let app = <MockSandbox as LayerShellSandbox>::new();
        assert_eq!(LayerShellSandbox::scale_factor(&app), 2.0);
    }

    #[test]
    fn test_update_increment() {
        let mut app = <MockSandbox as LayerShellSandbox>::new();
        LayerShellSandbox::update(&mut app, MockMessage::Increment);
        assert_eq!(app.counter, 1);
    }

    #[test]
    fn test_update_decrement() {
        let mut app = <MockSandbox as LayerShellSandbox>::new();
        LayerShellSandbox::update(&mut app, MockMessage::Decrement);
        assert_eq!(app.counter, -1);
    }
}
