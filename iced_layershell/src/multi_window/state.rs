use crate::multi_window::Application;
use crate::{Appearance, DefaultStyle};
use iced_core::{Color, Point, Size, mouse as IcedMouse};
use iced_graphics::Viewport;
use layershellev::WindowWrapper;
use layershellev::keyboard::ModifiersState;
use layershellev::reexport::wp_viewport::WpViewport;

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
    wpviewport: WpViewport,
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
        window: &WindowWrapper,
    ) -> Self {
        let application_scale_factor = application.scale_factor(id);
        let theme = application.theme(id);
        let appearance = application.style(&theme, id);

        let logical_size = Size::new(width, height);
        let viewport = viewport(logical_size, wayland_scale_factor, application_scale_factor);

        let wpviewport = window
            .viewport
            .clone()
            .expect("iced_layershell need viewport support to better wayland hidpi");
        set_wpviewport_destination(&wpviewport, logical_size);
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
            wpviewport,
        }
    }
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn update_view_port(&mut self, width: u32, height: u32, scale: f64) {
        let logical_size = Size::new(width, height);
        if self.logical_size_u32() == logical_size && self.wayland_scale_factor == scale {
            return;
        }
        self.wayland_scale_factor = scale;
        self.resize_viewport(logical_size);
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
                self.resize_viewport(self.logical_size_u32());
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &A) {
        let new_scale_factor = application.scale_factor(self.id);
        if self.application_scale_factor != new_scale_factor {
            self.application_scale_factor = new_scale_factor;
            self.resize_viewport(self.logical_size_u32());
        }
        self.theme = application.theme(self.id);
        self.appearance = application.style(&self.theme, self.id);
    }

    fn resize_viewport(&mut self, logical_size: Size<u32>) {
        self.viewport = viewport(
            logical_size,
            self.wayland_scale_factor,
            self.application_scale_factor,
        );

        self.viewport_version = self.viewport_version.wrapping_add(1);

        set_wpviewport_destination(&self.wpviewport, self.logical_size_u32());
    }

    fn logical_size_u32(&self) -> Size<u32> {
        // physical_size = (orig_logical_size as f64 * factor).ceil()
        // logical_sizea = physical_size as f64 / factor as f32
        // logical_size >= orig_logical_size
        let logical_size = self.viewport.logical_size();
        Size::new(
            logical_size.width.floor() as u32,
            logical_size.height.floor() as u32,
        )
    }
}

fn viewport(
    logical_size: Size<u32>,
    wayland_scale_factor: f64,
    application_scale_factor: f64,
) -> Viewport {
    let factor = wayland_scale_factor * application_scale_factor;
    let physical_size = Size::new(
        (logical_size.width as f64 * factor).ceil() as u32,
        (logical_size.height as f64 * factor).ceil() as u32,
    );
    Viewport::with_physical_size(physical_size, factor)
}

fn set_wpviewport_destination(wpviewport: &WpViewport, logical_size: Size<u32>) {
    if logical_size.width != 0 && logical_size.height != 0 {
        // set_destination(0, 0) will panic
        wpviewport.set_destination(logical_size.width as i32, logical_size.height as i32);
    }
}
