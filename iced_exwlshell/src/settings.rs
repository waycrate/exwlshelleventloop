use std::fs::File;

use iced_wayland_subscriber::shell;

use crate::reexport::{Anchor, KeyboardInteractivity, Layer, WithConnection};

pub use exwlshellev::{Extent, LayerSize, Margin, StartMode};

use exwlshellev::{blur::BlurOption, reexport::wayland_client::wl_keyboard::KeymapFormat};

#[derive(Debug)]
pub struct VirtualKeyboardSettings {
    pub file: File,
    pub keymap_size: u32,
    pub keymap_format: KeymapFormat,
}

#[derive(Debug)]
pub struct ExWlSettings {
    pub virtual_keyboard_support: Option<VirtualKeyboardSettings>,

    /// set the used wayland connection, all wayland object will share it, and they can be used by
    /// each other.
    pub with_connection: Option<WithConnection>,

    /// Where the runtime reports the surfaces it creates and the outputs they
    /// land on. Clone the same handle into `Broadcast::listen` to receive them.
    /// The default is a fresh one, which nothing is listening to.
    pub shell_broadcast: shell::ShellSender,

    pub layer_settings: LayerShellSettings,

    /// Keep the compositor alive when the last surface closes, instead of
    /// dropping it. Avoids cold-start delay at the cost of idle GPU/RAM.
    /// Defaults to `true`. Useful for daemons that show surfaces rarely.
    pub keep_compositor_alive: bool,

    /// Report scroll stops and per-scroll details to widgets; see [`crate::scroll`].
    /// Defaults to `false`.
    pub scroll_frames: bool,
}

impl Default for ExWlSettings {
    fn default() -> Self {
        ExWlSettings {
            virtual_keyboard_support: None,
            with_connection: None,
            shell_broadcast: shell::channel().0,
            layer_settings: LayerShellSettings::default(),
            keep_compositor_alive: true,
            scroll_frames: false
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayerShellSettings {
    pub anchor: Anchor,
    pub layer: Layer,
    pub exclusive_zone: i32,
    pub size: LayerSize,
    pub margin: Margin,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub start_mode: StartMode,
    pub blur_option: BlurOption,
    pub events_transparent: bool,
}

impl Default for LayerShellSettings {
    fn default() -> Self {
        LayerShellSettings {
            anchor: Anchor::all(),
            layer: Layer::Top,
            exclusive_zone: -1,
            size: LayerSize::FILL,
            margin: Margin::default(),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            events_transparent: false,
            start_mode: StartMode::default(),
            blur_option: BlurOption::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wlsettings_default() {
        let settings: ExWlSettings = ExWlSettings::default();

        // Test default layershellv settings
        assert_eq!(settings.layer_settings.anchor, Anchor::all());
        assert_eq!(settings.layer_settings.layer, Layer::Top);
        assert_eq!(settings.layer_settings.exclusive_zone, -1);
        assert_eq!(settings.layer_settings.size, LayerSize::FILL);
        assert_eq!(
            settings.layer_settings.margin,
            Margin {
                top: 0,
                right: 0,
                bottom: 0,
                left: 0
            }
        );
        assert_eq!(
            settings.layer_settings.keyboard_interactivity,
            KeyboardInteractivity::OnDemand
        );
        assert!(matches!(
            settings.layer_settings.start_mode,
            StartMode::Active
        ));
    }

    #[test]
    fn test_virtual_keyboard_settings() {
        let file = File::open("/dev/null").expect("Failed to open file");
        let keymap_size = 1024;
        let keymap_format = KeymapFormat::XkbV1;

        let virtual_keyboard_settings = VirtualKeyboardSettings {
            file,
            keymap_size,
            keymap_format,
        };

        assert_eq!(virtual_keyboard_settings.keymap_size, 1024);
        assert_eq!(virtual_keyboard_settings.keymap_format, KeymapFormat::XkbV1);
    }

    #[test]
    fn test_layer_shell_settings_custom() {
        let layer_settings = LayerShellSettings {
            anchor: Anchor::Top | Anchor::Left,
            layer: Layer::Background,
            exclusive_zone: 0,
            size: LayerSize::px(1920, 1080),
            margin: Margin {
                top: 10,
                right: 10,
                bottom: 10,
                left: 10,
            },
            keyboard_interactivity: KeyboardInteractivity::None,
            start_mode: StartMode::TargetScreen("HDMI-1".to_string()),
            events_transparent: false,
            blur_option: BlurOption::None,
        };

        assert_eq!(layer_settings.anchor, Anchor::Top | Anchor::Left);
        assert_eq!(layer_settings.layer, Layer::Background);
        assert_eq!(layer_settings.exclusive_zone, 0);
        assert_eq!(layer_settings.size, LayerSize::px(1920, 1080));
        assert_eq!(
            layer_settings.margin,
            Margin {
                top: 10,
                right: 10,
                bottom: 10,
                left: 10
            }
        );
        assert_eq!(
            layer_settings.keyboard_interactivity,
            KeyboardInteractivity::None
        );
        assert_eq!(
            layer_settings.start_mode,
            StartMode::TargetScreen("HDMI-1".to_string())
        );
    }
}
