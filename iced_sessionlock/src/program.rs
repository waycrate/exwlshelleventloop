use iced_core::Element;
use iced_core::Renderer;
use iced_core::text;
use iced_core::window;
use iced_runtime::Task;
pub trait Program: Sized {
    /// The graphics backend to use to draw the [`Program`].
    type Renderer: Renderer + text::Renderer;

    /// The type of __messages__ your [`Program`] will produce.
    type Message: std::fmt::Debug + Send;

    /// The theme used to draw the [`Program`].
    type Theme;

    /// Handles a __message__ and updates the state of the [`Program`].
    ///
    /// This is where you define your __update logic__. All the __messages__,
    /// produced by either user interactions or commands, will be handled by
    /// this method.
    ///
    /// Any [`Task`] returned will be executed immediately in the background by the
    /// runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message>;

    /// Returns the widgets to display in the [`Program`] for the `window`.
    ///
    /// These widgets can produce __messages__ based on user interaction.
    fn view(&self, window: window::Id) -> Element<'_, Self::Message, Self::Theme, Self::Renderer>;
}
