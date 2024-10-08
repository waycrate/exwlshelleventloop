use crate::application::Application;
use crate::{Appearance, DefaultStyle};
use iced_core::{mouse as IcedMouse, Color, Point, Size};
use iced_graphics::Viewport;
use layershellev::keyboard::ModifiersState;

use crate::event::WindowEvent;
use layershellev::reexport::wp_viewport::WpViewport;

pub struct State<A: Application>
where
    A::Theme: DefaultStyle,
{
    application_scale_factor: f64,
    wayland_scale_factor: f64,
    real_window_size: Size<u32>,
    viewport: Viewport,
    viewport_version: usize,
    theme: A::Theme,
    appearance: Appearance,
    mouse_position: Option<Point>,
    modifiers: ModifiersState,
    wpviewport: WpViewport,
}

impl<A: Application> State<A>
where
    A::Theme: DefaultStyle,
{
    pub fn new(application: &A, window: &layershellev::WindowStateSimple) -> Self {
        let application_scale_factor = application.scale_factor();
        let theme = application.theme();
        let appearance = application.style(&theme);

        let (width, height) = window.main_window().get_size();
        let real_window_size = Size::new(width, height);
        let wayland_scale_factor = 1.;

        let viewport = { Viewport::with_physical_size(real_window_size, wayland_scale_factor) };
        Self {
            application_scale_factor,
            real_window_size,
            wayland_scale_factor,
            viewport,
            viewport_version: 0,
            theme,
            appearance,
            mouse_position: None,
            modifiers: ModifiersState::default(),
            wpviewport: window
                .gen_main_wrapper()
                .viewport
                .expect("iced_layershell need viewport support to better wayland dpi"),
        }
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn current_wayland_scale(&self) -> f64 {
        self.wayland_scale_factor
    }

    pub fn update_view_port(&mut self, width: u32, height: u32, scale: f64) {
        let real_window_size = Size::new(width, height);
        if self.real_window_size == real_window_size && self.wayland_scale_factor == scale {
            return;
        }
        self.real_window_size = real_window_size;
        self.wayland_scale_factor = scale;
        self.viewport = Viewport::with_physical_size(
            self.adjusted_physical_size(),
            self.current_wayland_scale() * self.application_scale_factor,
        );
        let logical_size = self.viewport.logical_size();

        self.wpviewport.set_destination(
            logical_size.width.ceil() as i32,
            logical_size.height.ceil() as i32,
        );
        self.viewport_version = self.viewport_version.wrapping_add(1);
    }

    fn adjusted_physical_size(&self) -> Size<u32> {
        let mut size = self.real_window_size;
        let factor = self.wayland_scale_factor * self.application_scale_factor;
        size.width = (size.width as f64 * factor).ceil() as u32;
        size.height = (size.height as f64 * factor).ceil() as u32;
        size
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
    pub fn application_scale_factor(&self) -> f64 {
        1.
    }
    pub fn cursor(&self) -> IcedMouse::Cursor {
        self.mouse_position
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
                self.wayland_scale_factor = *scale_float;
                self.viewport = Viewport::with_physical_size(
                    self.adjusted_physical_size(),
                    self.application_scale_factor * scale_float,
                );

                self.viewport_version = self.viewport_version.wrapping_add(1);
                let logical_size = self.viewport.logical_size();

                self.wpviewport.set_destination(
                    logical_size.width.ceil() as i32,
                    logical_size.height.ceil() as i32,
                );
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &A) {
        let new_scale_factor = application.scale_factor();
        if self.application_scale_factor != new_scale_factor {
            self.application_scale_factor = new_scale_factor;
            self.viewport = Viewport::with_physical_size(
                self.adjusted_physical_size(),
                self.current_wayland_scale() * new_scale_factor,
            );
            self.viewport_version = self.viewport_version.wrapping_add(1);
        }
        self.theme = application.theme();
        self.appearance = application.style(&self.theme);
    }
}
