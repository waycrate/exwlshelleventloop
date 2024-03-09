use std::borrow::Cow;

use iced::{Font, Pixels};

use crate::reexport::{Anchor, Layer};

#[derive(Debug, Clone)]
pub struct Settings<Flags> {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// The [`window::Settings`].

    /// The data needed to initialize an [`Application`].
    ///
    /// [`Application`]: crate::Application
    pub flags: Flags,

    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,

    pub layer_settings: LayerShellSettings,
    pub default_font: Font,

    /// The text size that will be used by default.
    ///
    /// The default value is `16.0`.
    pub default_text_size: Pixels,

    /// If set to true, the renderer will try to perform antialiasing for some
    /// primitives.
    ///
    /// Enabling it can produce a smoother result in some widgets, like the
    /// [`Canvas`], at a performance cost.
    ///
    /// By default, it is disabled.
    ///
    /// [`Canvas`]: crate::widget::Canvas
    pub antialiasing: bool,
}

impl<Flags> Default for Settings<Flags>
where
    Flags: Default,
{
    fn default() -> Self {
        Settings {
            id: None,
            flags: Default::default(),
            fonts: Vec::new(),
            layer_settings: LayerShellSettings::default(),
            default_font: Font::default(),
            default_text_size: Pixels(16.0),
            antialiasing: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayerShellSettings {
    pub anchor: Anchor,
    pub layer: Layer,
    pub exclsize_zone: i32,
    pub size: Option<(u32, u32)>,
}

impl Default for LayerShellSettings {
    fn default() -> Self {
        LayerShellSettings {
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer: Layer::Top,
            exclsize_zone: -1,
            size: None,
        }
    }
}
