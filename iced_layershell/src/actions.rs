use crate::reexport::{Anchor, Layer, WlRegion};
use iced_core::window::Id as IcedId;
use layershellev::reexport::xdg_positioner::{
    Anchor as PopupAnchor, ConstraintAdjustment as PopupConstraintAdjustment,
    Gravity as PopupGravity,
};
use layershellev::{
    NewInputPanelSettings, NewLayerShellSettings, NewXdgWindowSettings, PopupPlacement,
};

use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct IcedXdgWindowSettings {
    /// The initial window size.
    pub size: Option<(u32, u32)>,
    /// Request client-side decorations instead of the default server-side mode.
    pub client_side_decorations: bool,
}

impl From<IcedXdgWindowSettings> for NewXdgWindowSettings {
    fn from(val: IcedXdgWindowSettings) -> Self {
        NewXdgWindowSettings {
            title: None,
            size: val.size,
            client_side_decorations: val.client_side_decorations,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IcedNewPopupSettings {
    pub size: (u32, u32),
    pub parent: Option<IcedId>,
    pub placement: PopupPlacement,
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub constraint_adjustment: PopupConstraintAdjustment,
}

impl IcedNewPopupSettings {
    /// Create popup settings for a popup of `size` anchored at `anchor_rect`
    /// (in the parent surface's local coordinates) on the `parent` surface.
    ///
    /// Defaults are applied: anchored at the bottom-left of the anchor
    /// rect, growing toward the top-right, with the compositor free to flip
    /// or slide the popup on either axis to keep it on-screen. Override any of
    /// them with the builder methods.
    pub fn new(parent: IcedId, size: (u32, u32), anchor_rect: (i32, i32, i32, i32)) -> Self {
        Self::build(Some(parent), size, PopupPlacement::Anchored(anchor_rect))
    }

    pub fn on_current_surface(size: (u32, u32), anchor_rect: (i32, i32, i32, i32)) -> Self {
        Self::build(None, size, PopupPlacement::Anchored(anchor_rect))
    }

    pub fn at_position(parent: IcedId, size: (u32, u32), position: (i32, i32)) -> Self {
        Self::build(Some(parent), size, PopupPlacement::Position(position))
    }

    pub fn at_position_on_current_surface(size: (u32, u32), position: (i32, i32)) -> Self {
        Self::build(None, size, PopupPlacement::Position(position))
    }

    fn build(parent: Option<IcedId>, size: (u32, u32), placement: PopupPlacement) -> Self {
        Self {
            size,
            parent,
            placement,
            anchor: PopupAnchor::BottomLeft,
            gravity: PopupGravity::TopRight,
            constraint_adjustment: PopupConstraintAdjustment::FlipX
                | PopupConstraintAdjustment::FlipY
                | PopupConstraintAdjustment::SlideX
                | PopupConstraintAdjustment::SlideY,
        }
    }

    /// Set which point of the anchor rect the popup is anchored to.
    pub fn anchor(mut self, anchor: PopupAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the direction the popup grows from the anchor point.
    pub fn gravity(mut self, gravity: PopupGravity) -> Self {
        self.gravity = gravity;
        self
    }

    /// Set how the compositor may adjust (flip/slide/resize) the popup for off-screen cases
    pub fn constraint_adjustment(
        mut self,
        constraint_adjustment: PopupConstraintAdjustment,
    ) -> Self {
        self.constraint_adjustment = constraint_adjustment;
        self
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MenuDirection {
    Up,
    Down,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IcedNewMenuSettings {
    pub size: (u32, u32),
    pub direction: MenuDirection,
}

type Callback = Arc<dyn Fn(&WlRegion) + Send + Sync>;

// Callback wrapper around dyn Fn(&WlRegion)
#[derive(Clone)]
pub struct ActionCallback(pub Callback);

impl std::fmt::Debug for ActionCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "callback function")
    }
}

impl ActionCallback {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&WlRegion) + Send + Sync + 'static,
    {
        ActionCallback(Arc::new(callback))
    }
}

/// NOTE: DO NOT USE THIS ENUM DIERCTLY
/// use macro to_layer_message
#[derive(Debug, Clone)]
pub enum LayerShellCustomAction {
    AnchorChange(Anchor),
    LayerChange(Layer),
    AnchorSizeChange(Anchor, (u32, u32)),
    MarginChange((i32, i32, i32, i32)),
    SizeChange((u32, u32)),
    ExclusiveZoneChange(i32),
    KeyboardInteractivityChange(layershellev::reexport::KeyboardInteractivity),
    VirtualKeyboardPressed {
        key: u32,
    },
    // settings, info, single_tone
    NewLayerShell {
        settings: NewLayerShellSettings,
        id: IcedId,
    },
    SetInputRegion(ActionCallback),
    NewPopUp {
        settings: IcedNewPopupSettings,
        id: IcedId,
    },
    NewMenu {
        settings: IcedNewMenuSettings,
        id: IcedId,
    },
    NewBaseWindow {
        settings: IcedXdgWindowSettings,
        id: IcedId,
    },
    NewInputPanel {
        settings: NewInputPanelSettings,
        id: IcedId,
    },
    /// is same with WindowAction::Close(id)
    RemoveWindow,
    ForgetLastOutput,
}

/// Please do not use this struct directly
/// Use macro to_layer_message instead
#[derive(Debug, Clone)]
pub struct LayerShellCustomActionWithId(pub Option<IcedId>, pub LayerShellCustomAction);

impl LayerShellCustomActionWithId {
    pub fn new(id: Option<IcedId>, action: LayerShellCustomAction) -> Self {
        Self(id, action)
    }
}
