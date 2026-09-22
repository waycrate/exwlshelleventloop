use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer,
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
};

use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor as PopupAnchor, ConstraintAdjustment as PopupConstraintAdjustment,
    Gravity as PopupGravity,
};

use wayland_client::protocol::wl_output::{self};

use crate::size::{LayerSize, PixelSize};

use crate::{blur::BlurOption, id};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OutputOption {
    LastOutput,

    OutputName(String),

    /// The `wl_registry` global name, as carried by `iced_wayland_subscriber::OutputId`
    GlobalName(u32),

    /// NOTE: The output should be in the same connection with the layershellev, that means if you
    /// want to pass a [wl_output::WlOutput] to create a new layershell, you need to pass your
    /// connection to layershellev first
    Output(wl_output::WlOutput),

    /// Let the compositor decide which output to use.
    #[default]
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Margin {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position<T = i32> {
    pub x: T,
    pub y: T,
}

impl<T> Position<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size<T = u32> {
    pub width: T,
    pub height: T,
}

/// layershell settings to create a new layershell surface
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLayerShellSettings {
    /// the size of the layershell
    pub size: LayerSize,
    pub layer: Layer,
    pub anchor: Anchor,
    pub exclusive_zone: Option<i32>,
    pub margin: Option<Margin>,
    pub keyboard_interactivity: KeyboardInteractivity,
    /// follow the last output of the activated surface, used to create some thing like mako, who
    /// will show on the same window, only when the notifications is cleared, it will change the
    /// wl_output.
    pub output_option: OutputOption,
    pub events_transparent: bool,
    pub namespace: Option<String>,
    pub blur_option: BlurOption,
}

/// How a popup is positioned relative to its parent surface.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PopupPlacement {
    /// anchor rectangle in the parent surface's local coordinates
    Anchored {
        /// the top-left corner of the anchor rectangle
        position: Position,
        /// the extents of the anchor rectangle
        size: PixelSize,
    },
    /// Absolute position of the popup in the parent surface's local coordinates
    Position(Position),
}

/// be used to create a new popup
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NewPopUpSettings {
    /// the size of the popup
    pub size: PixelSize,
    /// the id of the parent surface
    pub id: id::Id,
    /// How a popup is positioned relative to its parent surface.
    pub placement: PopupPlacement,
    /// which point of the anchor rect the popup is anchored to
    pub anchor: PopupAnchor,
    /// the direction the popup grows from the anchor point
    pub gravity: PopupGravity,
    /// how the compositor may adjust (flip/slide/resize) the popup for off-screen cases
    pub constraint_adjustment: PopupConstraintAdjustment,
    /// Serial of the input event
    pub grab_serial: Option<u32>,
}

/// be used to move and resize a mapped popup
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PopUpRepositionSettings {
    /// the new size of the popup
    pub size: PixelSize,
    /// How the popup is positioned relative to its parent surface
    pub placement: PopupPlacement,
    /// which point of the anchor rect the popup is anchored to
    pub anchor: PopupAnchor,
    /// the direction the popup grows from the anchor point
    pub gravity: PopupGravity,
    /// how the compositor may adjust (flip/slide/resize) the popup for off-screen cases
    pub constraint_adjustment: PopupConstraintAdjustment,
}

/// Settings used to create a new xdg toplevel window.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct NewXdgWindowSettings {
    /// The window title.
    pub title: Option<String>,
    /// The initial window size.
    pub size: Option<PixelSize>,
    /// Request client-side decorations instead of the default server-side mode.
    pub client_side_decorations: bool,
}

/// Window state reported by `xdg_toplevel::configure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToplevelState {
    /// The surface is maximized.
    pub maximized: bool,
    /// The surface is fullscreen.
    pub fullscreen: bool,
    /// At least one edge is tiled against another surface or the output.
    pub tiled: bool,
    /// The compositor considers this surface active.
    pub activated: bool,
}

/// input panel settings to create a new input panel surface
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInputPanelSettings {
    pub size: PixelSize,
    /// set the surface type as a keyboard
    pub keyboard: bool,
    /// follow the last output of the activated surface, used to create some thing like mako, who
    /// will show on the same window, only when the notifications is cleared, it will change the
    /// wl_output.
    pub output_option: OutputOption,
}

impl Default for NewLayerShellSettings {
    fn default() -> Self {
        NewLayerShellSettings {
            anchor: Anchor::all(),
            layer: Layer::Top,
            exclusive_zone: None,
            size: LayerSize::FILL,
            margin: Some(Margin {
                right: 0,
                left: 0,
                top: 0,
                bottom: 0,
            }),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            output_option: OutputOption::Active,
            events_transparent: false,
            namespace: None,
            blur_option: BlurOption::None,
        }
    }
}
