use crate::application::Application;
use iced_core::{mouse as IcedMouse, Color, Point, Size};
use iced_graphics::Viewport;
use iced_style::application::{self, StyleSheet};

use crate::event::WindowEvent;

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
    mouse_position: Option<Point>,
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
            mouse_position: None,
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
        self.mouse_position
            .map(IcedMouse::Cursor::Available)
            .unwrap_or(IcedMouse::Cursor::Unavailable)
    }

    pub fn update(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorLeft => {
                self.mouse_position = None;
            }
            WindowEvent::CursorMoved { x, y } => {
                self.mouse_position = Some(Point::new(*x as f32, *y as f32));
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &A) {
        let new_scale_factor = application.scale_factor();
        if self.scale_factor != new_scale_factor {
            self.viewport =
                Viewport::with_physical_size(self.physical_size(), 1. * new_scale_factor);
            self.viewport_version = self.viewport_version.wrapping_add(1);
            self.scale_factor = new_scale_factor;
        }
        self.theme = application.theme();
        self.appearance = self.theme.appearance(&application.style());
    }
}
