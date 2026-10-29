use crate::error::RegionError;
use crate::reexport::{Anchor, Layer, WlRegion};
use exwlshellev::blur::BlurOption;
use exwlshellev::reexport::xdg_positioner::{
    Anchor as PopupAnchor, ConstraintAdjustment as PopupConstraintAdjustment,
    Gravity as PopupGravity,
};
use exwlshellev::{
    ActivationTokenSink, LayerSize, Margin, NewInputPanelSettings, NewLayerShellSettings,
    NewXdgWindowSettings, PixelSize, PopupPlacement, Position,
};
use iced_core::window::Id as IcedId;
use std::sync::Arc;

/// the [InputRegion] will delete the rectangle first, then you can add it later
pub struct InputRegion {
    region: Option<WlRegion>,
    width: u32,
    height: u32,
}

impl InputRegion {
    pub(crate) fn region(&self) -> Option<&WlRegion> {
        self.region.as_ref()
    }
    pub(crate) fn new(region: WlRegion, width: u32, height: u32) -> Self {
        Self {
            region: Some(region),
            width,
            height,
        }
    }
    /// return the height of the surface
    pub fn surface_height(&self) -> u32 {
        self.height
    }

    /// return the width of the layer
    pub fn surface_width(&self) -> u32 {
        self.width
    }

    /// return the size of the layer, width, height
    pub fn surface_size(&self) -> (u32, u32) {
        (self.surface_width(), self.surface_height())
    }

    /// check if region is still exist
    pub fn alive(&self) -> bool {
        self.region.is_some()
    }

    /// wrapper of the add function of [WlRegion], if failed, return [RegionError]
    pub fn add(&self, x: i32, y: i32, width: i32, height: i32) -> Result<(), RegionError> {
        let region = self.region.as_ref().ok_or(RegionError::RegionDeleted)?;
        region.add(x, y, width, height);
        Ok(())
    }

    /// wrapper of the subtract function of [WlRegion], if failed, return [RegionError]
    pub fn subtract(&self, x: i32, y: i32, width: i32, height: i32) -> Result<(), RegionError> {
        let region = self.region.as_ref().ok_or(RegionError::RegionDeleted)?;
        region.subtract(x, y, width, height);
        Ok(())
    }

    /// destroy the inner region, then will reset the region
    pub fn remove(&mut self) {
        let Some(region) = self.region.take() else {
            return;
        };
        region.destroy();
    }
}

impl Drop for InputRegion {
    fn drop(&mut self) {
        let Some(region) = self.region.take() else {
            return;
        };
        region.destroy();
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct IcedXdgWindowSettings {
    /// The initial window size.
    pub size: Option<PixelSize>,
    /// Request client-side decorations instead of the default server-side mode.
    pub client_side_decorations: bool,
    /// The xdg `app_id`
    pub app_id: Option<String>,
}

impl From<IcedXdgWindowSettings> for NewXdgWindowSettings {
    fn from(val: IcedXdgWindowSettings) -> Self {
        NewXdgWindowSettings {
            title: None,
            size: val.size,
            client_side_decorations: val.client_side_decorations,
            app_id: val.app_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IcedNewPopupSettings {
    pub size: PixelSize,
    pub parent: Option<IcedId>,
    pub placement: PopupPlacement,
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub constraint_adjustment: PopupConstraintAdjustment,
}

impl IcedNewPopupSettings {
    pub fn new(
        parent: IcedId,
        size: PixelSize,
        anchor_position: Position,
        anchor_size: PixelSize,
    ) -> Self {
        Self::build(
            Some(parent),
            size,
            PopupPlacement::Anchored {
                position: anchor_position,
                size: anchor_size,
            },
        )
    }

    pub fn on_current_surface(
        size: PixelSize,
        anchor_position: Position,
        anchor_size: PixelSize,
    ) -> Self {
        Self::build(
            None,
            size,
            PopupPlacement::Anchored {
                position: anchor_position,
                size: anchor_size,
            },
        )
    }

    pub fn at_position(parent: IcedId, size: PixelSize, position: Position) -> Self {
        Self::build(Some(parent), size, PopupPlacement::Position(position))
    }

    pub fn at_position_on_current_surface(size: PixelSize, position: Position) -> Self {
        Self::build(None, size, PopupPlacement::Position(position))
    }

    fn build(parent: Option<IcedId>, size: PixelSize, placement: PopupPlacement) -> Self {
        Self {
            size,
            parent,
            placement,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IcedNewMenuSettings {
    pub size: PixelSize,
    pub gravity: PopupGravity,
}

/// region callback
type RegionCB = Arc<dyn Fn(&mut InputRegion) + Send + Sync>;

// Callback wrapper around dyn Fn(&WlRegion)
#[derive(Clone)]
pub struct RegionActionCB(pub RegionCB);

impl std::fmt::Debug for RegionActionCB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "callback function")
    }
}

impl RegionActionCB {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&mut InputRegion) + Send + Sync + 'static,
    {
        RegionActionCB(Arc::new(callback))
    }
}

/// NOTE: DO NOT USE THIS ENUM DIERCTLY
/// use macro to_layer_message
#[derive(Debug, Clone)]
pub enum ExWlShellCustomAction {
    LayoutChange {
        anchor: Anchor,
        size: LayerSize,
    },
    LayerChange(Layer),
    MarginChange(Margin),
    ExclusiveZoneChange(i32),
    KeyboardInteractivityChange(exwlshellev::reexport::KeyboardInteractivity),
    VirtualKeyboardPressed {
        key: u32,
    },
    BlurOptionChange(BlurOption),
    // settings, info, single_tone
    NewLayerShell {
        settings: NewLayerShellSettings,
        id: IcedId,
    },
    SetInputRegion(RegionActionCB),
    /// Mint an `xdg_activation_v1` token on behalf of the addressed surface and
    /// hand it to the sink. A layer shell has no toplevel to raise itself, so
    /// the token is only useful once passed to the client that should be
    /// focused.
    RequestActivationToken(ActivationTokenSink),
    NewPopUp {
        settings: IcedNewPopupSettings,
        id: IcedId,
    },
    /// During move/resize of a mapped popup `settings.parent` is ignored. Can't be reparented as per spec
    PopUpReposition {
        settings: IcedNewPopupSettings,
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
    RedrawWindow,
    ForgetLastOutput,
    Lock,
    UnLock,
}

/// Please do not use this struct directly
/// Use macro to_layer_message instead
#[derive(Debug, Clone)]
pub struct ExWlShellCustomActionWithId(pub Option<IcedId>, pub ExWlShellCustomAction);

impl ExWlShellCustomActionWithId {
    pub fn new(id: Option<IcedId>, action: ExWlShellCustomAction) -> Self {
        Self(id, action)
    }
}
