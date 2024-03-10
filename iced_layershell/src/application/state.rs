use crate::application::Application;
use iced_core::{mouse as IcedMouse, Color, Size};
use iced_graphics::Viewport;
use iced_style::application::{self, StyleSheet};

#[allow(unused)]
pub struct State<A: Application>
where
    A::Theme: application::StyleSheet,
{
    scale_factor: f64,
    viewport: Viewport,
    viewport_version: usize,
    theme: A::Theme,
    appearance: application::Appearance,
}

impl<A: Application> State<A>
where
    A::Theme: application::StyleSheet,
{
    pub fn new(application: &A, window: &layershellev::WindowState<()>) -> Self {
        let scale_factor = application.scale_factor();
        let theme = application.theme();
        let appearance = theme.appearance(&application.style());

        let viewport = {
            let (width, height) = window.main_window().get_size();

            Viewport::with_physical_size(iced_core::Size::new(width, height), 1. * scale_factor)
        };
        Self {
            scale_factor,
            viewport,
            viewport_version: 0,
            theme,
            appearance,
        }
    }

    pub fn update_view_port(&mut self, width: u32, height: u32) {
        self.viewport = Viewport::with_physical_size(
            iced_core::Size::new(width, height),
            1. * self.scale_factor(),
        )
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn physical_size(&self) -> Size<u32> {
        self.viewport.physical_size()
    }

    pub fn logical_size(&self) -> Size<f32> {
        self.viewport.logical_size()
    }

    pub fn scale_factor(&self) -> f64 {
        self.viewport.scale_factor()
    }

    pub fn text_color(&self) -> Color {
        self.appearance.text_color
    }

    pub fn background_color(&self) -> Color {
        self.appearance.background_color
    }

    pub fn theme(&self) -> &A::Theme {
        &self.theme
    }

    pub fn cursor(&self) -> IcedMouse::Cursor {
        IcedMouse::Cursor::Unavailable
    }
}
