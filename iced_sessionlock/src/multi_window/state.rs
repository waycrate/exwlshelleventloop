use crate::{Appearance, DefaultStyle};
use iced_core::{Color, Point, Size, mouse as IcedMouse, window};
use iced_graphics::Viewport;
use iced_program::Instance;
use iced_program::Program;
use sessionlockev::WindowWrapper;
use sessionlockev::keyboard::ModifiersState;
use sessionlockev::reexport::wp_viewport::WpViewport;

use crate::event::WindowEvent;

pub struct State<P: Program>
where
    P::Theme: DefaultStyle,
{
    id: window::Id,
    application_scale_factor: f64,
    wayland_scale_factor: f64,
    /// viewport_logical_size = window_size / application_scale_factor,
    /// viewport_physical_size = window_size * wayland_scal_factor,
    window_size: Size<u32>,
    viewport: Viewport,
    viewport_version: usize,
    theme: Option<P::Theme>,
    default_theme: P::Theme,
    theme_mode: iced_core::theme::Mode,
    style: Appearance,
    mouse_position: Option<Point>,
    modifiers: ModifiersState,
    wpviewport: WpViewport,
}

impl<P: Program> State<P>
where
    P::Theme: DefaultStyle,
{
    pub fn new(
        id: window::Id,
        application: &Instance<P>,
        (width, height): (u32, u32),
        wayland_scale_factor: f64,
        window: &WindowWrapper,
        system_theme: iced_core::theme::Mode,
    ) -> Self {
        let application_scale_factor = application.scale_factor(id) as f64;
        let theme = application.theme(id);
        let theme_mode = theme
            .as_ref()
            .map(iced_core::theme::Base::mode)
            .unwrap_or_default();
        let default_theme = <P::Theme as iced_core::theme::Base>::default(system_theme);
        let style = application.style(&theme.as_ref().unwrap_or(&default_theme));

        let window_size = Size::new(width, height);
        let viewport = viewport(window_size, wayland_scale_factor, application_scale_factor);

        let wpviewport = window
            .viewport
            .clone()
            .expect("iced_sessionlock need viewport support to better wayland hidpi");
        set_wpviewport_destination(&wpviewport, window_size);
        Self {
            id,
            application_scale_factor,
            wayland_scale_factor,
            window_size,
            viewport,
            viewport_version: 0,
            theme,
            style,
            default_theme,
            theme_mode,
            mouse_position: None,
            modifiers: ModifiersState::default(),
            wpviewport,
        }
    }
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn update_view_port(&mut self, width: u32, height: u32, scale: f64) {
        let window_size = Size::new(width, height);
        if self.window_size == window_size && self.wayland_scale_factor == scale {
            return;
        }
        self.window_size = window_size;
        self.wayland_scale_factor = scale;
        self.resize_viewport();
        set_wpviewport_destination(&self.wpviewport, self.window_size);
    }

    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    pub fn window_size(&self) -> Size<u32> {
        self.window_size
    }

    /// using viewport physical size and wayland scale factor to calculate the actual window size.
    /// The result may contain fractions.
    pub fn window_size_f32(&self) -> Size<f32> {
        let physical_size = self.viewport.physical_size();
        Size::new(
            (physical_size.width as f64 / self.wayland_scale_factor) as f32,
            (physical_size.height as f64 / self.wayland_scale_factor) as f32,
        )
    }

    pub fn wayland_scale_factor(&self) -> f64 {
        self.wayland_scale_factor
    }

    pub fn application_scale_factor(&self) -> f64 {
        self.application_scale_factor
    }

    pub fn text_color(&self) -> Color {
        self.style.text_color
    }

    pub fn background_color(&self) -> Color {
        self.style.background_color
    }

    pub fn theme(&self) -> &P::Theme {
        self.theme.as_ref().unwrap_or(&self.default_theme)
    }

    #[allow(unused)]
    pub fn theme_mode(&self) -> iced_core::theme::Mode {
        self.theme_mode
    }

    pub fn cursor(&self) -> IcedMouse::Cursor {
        self.mouse_position
            .map(IcedMouse::Cursor::Available)
            .unwrap_or(IcedMouse::Cursor::Unavailable)
    }

    pub fn update(&mut self, event: &WindowEvent, application: &Instance<P>) {
        match event {
            WindowEvent::CursorLeft => {
                self.mouse_position = None;
            }
            WindowEvent::CursorMoved { x, y }
            | WindowEvent::CursorEnter { x, y }
            | WindowEvent::TouchMotion { x, y, .. }
            | WindowEvent::TouchDown { x, y, .. }
            | WindowEvent::TouchUp { x, y, .. } => {
                self.mouse_position = Some(Point::new(
                    (*x / self.application_scale_factor) as f32,
                    (*y / self.application_scale_factor) as f32,
                ));
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = *modifiers;
            }
            WindowEvent::ScaleFactorChanged {
                scale_float,
                scale_u32: _,
            } => {
                self.wayland_scale_factor = *scale_float;
                self.resize_viewport();
            }
            WindowEvent::ThemeChanged(mode) => {
                self.default_theme = <P::Theme as iced_core::theme::Base>::default(*mode);
                if self.theme.is_none() {
                    self.style = application.style(&self.default_theme);
                }
            }
            _ => {}
        }
    }

    pub fn synchronize(&mut self, application: &Instance<P>) {
        let new_scale_factor = application.scale_factor(self.id) as f64;
        if self.application_scale_factor != new_scale_factor {
            self.application_scale_factor = new_scale_factor;
            self.resize_viewport();
        }
        self.theme = application.theme(self.id);
        self.style = application.style(self.theme());
    }

    fn resize_viewport(&mut self) {
        tracing::debug!(
            "before resizing viewport, window_size: {:?}, viewport physical size: {:?}, viewport logical size: {:?}",
            self.window_size,
            self.viewport.physical_size(),
            self.viewport.logical_size()
        );
        self.viewport = viewport(
            self.window_size,
            self.wayland_scale_factor,
            self.application_scale_factor,
        );
        tracing::debug!(
            "after resizing viewport, window_size: {:?}, viewport physical size: {:?}, viewport logical size: {:?}, wayland scale factor: {}, application scale factor: {}",
            self.window_size,
            self.viewport.physical_size(),
            self.viewport.logical_size(),
            self.wayland_scale_factor,
            self.application_scale_factor
        );

        self.viewport_version = self.viewport_version.wrapping_add(1);
    }
}

fn viewport(
    window_size: Size<u32>,
    wayland_scale_factor: f64,
    application_scale_factor: f64,
) -> Viewport {
    let factor = wayland_scale_factor * application_scale_factor;
    let physical_size = Size::new(
        (window_size.width as f64 * wayland_scale_factor).ceil() as u32,
        (window_size.height as f64 * wayland_scale_factor).ceil() as u32,
    );
    Viewport::with_physical_size(physical_size, factor as f32)
}

fn set_wpviewport_destination(wpviewport: &WpViewport, window_size: Size<u32>) {
    if window_size.width != 0 && window_size.height != 0 {
        // set_destination(0, 0) will panic
        wpviewport.set_destination(window_size.width as i32, window_size.height as i32);
    }
}
