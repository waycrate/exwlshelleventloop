use crate::application::Application;
use crate::{Appearance, DefaultStyle};
use iced_core::{mouse as IcedMouse, Color, Point, Size};
use iced_graphics::Viewport;
use layershellev::keyboard::ModifiersState;

use crate::event::WindowEvent;

pub struct State<A: Application>
where
    A::Theme: DefaultStyle,
{
    scale_factor: f64,
    window_size: iced::Size<u32>,
    window_scale_factor: u32,
    viewport: Viewport,
    viewport_version: usize,
    theme: A::Theme,
    appearance: Appearance,
    mouse_position: Option<Point>,
    modifiers: ModifiersState,
}

impl<A: Application> State<A>
where
    A::Theme: DefaultStyle,
{
    pub fn new(application: &A, window: &layershellev::WindowStateSimple) -> Self {
        let scale_factor = application.scale_factor();
        let theme = application.theme();
        let appearance = application.style(&theme);

        let window_scale_factor = 120;
        let (window_size, viewport) = {
            let (width, height) = window.main_window().get_size();

            let realscale = window_scale_factor as f64 / 120.;
            (
                iced::Size::new(width, height),
                Viewport::with_physical_size(
                    iced_core::Size::new(width, height),
                    realscale * scale_factor,
                ),
            )
        };
        Self {
            scale_factor,
            window_size,
            window_scale_factor,
            viewport,
            viewport_version: 0,
            theme,
            appearance,
            mouse_position: None,
            modifiers: ModifiersState::default(),
        }
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn current_wayland_scale(&self) -> f64 {
        self.window_scale_factor as f64 / 120.
    }

    pub fn update_view_port(&mut self, width: u32, height: u32) {
        self.window_size = iced::Size::new(width, height);
        self.viewport = Viewport::with_physical_size(
            self.window_size(),
            self.current_wayland_scale() * self.scale_factor,
        );
        self.viewport_version = self.viewport_version.wrapping_add(1);
    }

    fn window_size(&self) -> iced::Size<u32> {
        let mut window_size = self.window_size;
        window_size.width = window_size.width * 140 / self.window_scale_factor;
        window_size.height = window_size.height * 140 / self.window_scale_factor;
        window_size
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
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
            }
            WindowEvent::ScaleFactorChanged {
                scale_float,
                scale_u32,
            } => {
                self.viewport = Viewport::with_physical_size(
                    self.window_size(),
                    self.scale_factor * scale_float,
                );

                self.viewport_version = self.viewport_version.wrapping_add(1);
                self.window_scale_factor = *scale_u32;
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &A) {
        let new_scale_factor = application.scale_factor();
        if self.scale_factor != new_scale_factor {
            self.viewport = Viewport::with_physical_size(
                self.window_size(),
                self.current_wayland_scale() * new_scale_factor,
            );
            self.viewport_version = self.viewport_version.wrapping_add(1);
            self.scale_factor = new_scale_factor;
        }
        self.theme = application.theme();
        self.appearance = application.style(&self.theme);
    }
}
