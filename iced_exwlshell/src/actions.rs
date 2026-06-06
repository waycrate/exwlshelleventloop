use crate::reexport::{Anchor, Layer, WlRegion};
use exwlshellev::reexport::xdg_positioner::{
    Anchor as PopupAnchor, ConstraintAdjustment as PopupConstraintAdjustment,
    Gravity as PopupGravity,
};
use exwlshellev::{NewInputPanelSettings, NewLayerShellSettings, NewXdgWindowSettings};
use iced_core::window::Id as IcedId;

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
    pub parent: IcedId,
    pub anchor_rect: (i32, i32, i32, i32),
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub constraint_adjustment: PopupConstraintAdjustment,
}

impl IcedNewPopupSettings {
    pub fn new(parent: IcedId, size: (u32, u32), anchor_rect: (i32, i32, i32, i32)) -> Self {
        Self {
            size,
            parent,
            anchor_rect,
            anchor: PopupAnchor::BottomLeft,
            gravity: PopupGravity::BottomRight,
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
pub enum ExwlShellCustomAction {
    AnchorChange(Anchor),
    LayerChange(Layer),
    AnchorSizeChange(Anchor, (u32, u32)),
    MarginChange((i32, i32, i32, i32)),
    SizeChange((u32, u32)),
    ExclusiveZoneChange(i32),
    KeyboardInteractivityChange(exwlshellev::reexport::KeyboardInteractivity),
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
    Lock,
    UnLock,
}

/// Please do not use this struct directly
/// Use macro to_layer_message instead
#[derive(Debug, Clone)]
pub struct ExwlShellCustomActionWithId(pub Option<IcedId>, pub ExwlShellCustomAction);

impl ExwlShellCustomActionWithId {
    pub fn new(id: Option<IcedId>, action: ExwlShellCustomAction) -> Self {
        Self(id, action)
    }
}
