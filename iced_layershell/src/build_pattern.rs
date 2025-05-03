//! The build_pattern allow you to create application just with callback functions.
//! Similar with the one of origin iced.

mod application;
mod daemon;
use std::borrow::Cow;

use iced::{Font, Pixels};

use crate::settings::{LayerShellSettings, VirtualKeyboardSettings};

pub use daemon::Program as DaemonProgram;
/// The renderer of some Program.
pub trait Renderer: iced_core::text::Renderer + iced_graphics::compositor::Default {}

impl<T> Renderer for T where T: iced_core::text::Renderer + iced_graphics::compositor::Default {}

/// MainSettings for iced_layershell
/// different from [`crate::Settings`], it does not contain the field of flags
#[derive(Debug)]
pub struct Settings {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// settings for layer shell
    pub layer_settings: LayerShellSettings,
    /// The data needed to initialize an Application
    ///
    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,

    /// The default [`Font`] to be used.
    ///
    /// By default, it uses [`Family::SansSerif`](iced::font::Family::SansSerif).
    pub default_font: Font,

    /// The text size that will be used by default.
    ///
    /// The default value is `16.0`.
    pub default_text_size: Pixels,

    /// If set to true, the renderer will try to perform antialiasing for some
    /// primitives.
    ///
    /// Enabling it can produce a smoother result in some widgets, like the
    /// `Canvas`, at a performance cost.
    ///
    /// By default, it is disabled.
    ///
    pub antialiasing: bool,

    pub virtual_keyboard_support: Option<VirtualKeyboardSettings>,
}
impl Default for Settings {
    fn default() -> Self {
        Settings {
            id: None,
            fonts: Vec::new(),
            layer_settings: LayerShellSettings::default(),
            default_font: Font::default(),
            default_text_size: Pixels(16.0),
            antialiasing: false,
            virtual_keyboard_support: None,
        }
    }
}

use daemon::Program as LayerProgram;

use iced::Task;

use iced::Element;
use iced::Subscription;
use iced::theme;
use iced::window;

#[allow(missing_debug_implementations)]
pub struct Instance<P: LayerProgram> {
    program: P,
    state: P::State,
}

impl<P: LayerProgram> Instance<P> {
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

#[doc = include_str!("./build_pattern/application.md")]
pub use application::application;

#[doc = include_str!("./build_pattern/daemon.md")]
pub use daemon::daemon;
