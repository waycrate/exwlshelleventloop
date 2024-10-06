use crate::multi_window::Application;
use crate::{Appearance, DefaultStyle};
use iced_core::{mouse as IcedMouse, Color, Point, Size};
use iced_graphics::Viewport;
use layershellev::keyboard::ModifiersState;

use crate::event::WindowEvent;
use iced::window;

pub struct State<A: Application>
where
    A::Theme: DefaultStyle,
{
    id: window::Id,
    application_scale_factor: f64,
    wayland_scale_factor: f64,
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
    pub fn new(
        id: window::Id,
        application: &A,
        (width, height): (u32, u32),
        wayland_scale_factor: f64,
    ) -> Self {
        let application_scale_factor = application.scale_factor(id);
        let theme = application.theme();
        let appearance = application.style(&theme);

        let viewport = Viewport::with_physical_size(
            iced_core::Size::new(width, height),
            wayland_scale_factor * application_scale_factor,
        );
        Self {
            id,
            application_scale_factor,
            wayland_scale_factor,
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
        self.wayland_scale_factor
    }
    pub fn update_view_port(&mut self, width: u32, height: u32, scale: f64) {
        self.wayland_scale_factor = scale;
        self.viewport = Viewport::with_physical_size(
            iced_core::Size::new(width, height),
            self.current_wayland_scale() * self.application_scale_factor,
        );
        self.viewport_version = self.viewport_version.wrapping_add(1);
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

    pub fn mouse_position(&self) -> Option<&Point> {
        self.mouse_position.as_ref()
    }

    pub fn cursor(&self) -> IcedMouse::Cursor {
        self.mouse_position
            .map(|point| Point {
                x: point.x / self.scale_factor() as f32,
                y: point.y / self.scale_factor() as f32,
            })
            .map(IcedMouse::Cursor::Available)
            .unwrap_or(IcedMouse::Cursor::Unavailable)
    }

    pub fn update(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorLeft | WindowEvent::TouchUp { .. } => {
                self.mouse_position = None;
            }
            WindowEvent::CursorMoved { x, y }
            | WindowEvent::CursorEnter { x, y }
            | WindowEvent::TouchMotion { x, y, .. }
            | WindowEvent::TouchDown { x, y, .. } => {
                self.mouse_position = Some(Point::new(*x as f32, *y as f32));
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
            }
            WindowEvent::ScaleFactorChanged {
                scale_float,
                scale_u32: _,
            } => {
                let size = self.viewport.physical_size();

                self.viewport =
                    Viewport::with_physical_size(size, scale_float * self.application_scale_factor);

                self.viewport_version = self.viewport_version.wrapping_add(1);
                self.wayland_scale_factor = *scale_float;
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &A) {
        let new_scale_factor = application.scale_factor(self.id);
        if self.application_scale_factor != new_scale_factor {
            self.viewport = Viewport::with_physical_size(
                self.physical_size(),
                self.wayland_scale_factor * new_scale_factor,
            );
            self.viewport_version = self.viewport_version.wrapping_add(1);
            self.application_scale_factor = new_scale_factor;
        }
        self.theme = application.theme();
        self.appearance = application.style(&self.theme);
    }
}
