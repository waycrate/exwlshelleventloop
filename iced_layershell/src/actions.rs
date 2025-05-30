use crate::ime_preedit::ImeState;
use crate::reexport::{Anchor, Layer, WlRegion};
use enumflags2::BitFlags;
use iced::window::Id as IcedId;
use iced_core::input_method;
use iced_core::mouse::Interaction;
use layershellev::id::Id as LayerId;
use layershellev::{NewInputPanelSettings, NewLayerShellSettings};

use std::sync::Arc;

pub(crate) type LayerShellActionVec = Vec<LayerShellAction>;

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum LayerShellAction {
    Mouse(Interaction),
    CustomActionWithId(LayershellCustomActionWithIdInner),
    RedrawAll,
    RedrawWindow(LayerId), // maybe one day it is useful, but now useless
    NewMenu(IcedNewPopupSettings, iced_core::window::Id),
    ImeWithId(LayerId, input_method::InputMethod, BitFlags<ImeState>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IcedNewPopupSettings {
    pub size: (u32, u32),
    pub position: (i32, i32),
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
pub enum LayershellCustomAction {
    AnchorChange(Anchor),
    LayerChange(Layer),
    AnchorSizeChange(Anchor, (u32, u32)),
    MarginChange((i32, i32, i32, i32)),
    SizeChange((u32, u32)),
    ExclusiveZoneChange(i32),
    VirtualKeyboardPressed {
        time: u32,
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
    NewInputPanel {
        settings: NewInputPanelSettings,
        id: IcedId,
    },
    /// is same with WindowAction::Close(id)
    RemoveWindow(IcedId),
    ForgetLastOutput,
}

/// Please do not use this struct directly
/// Use macro to_layer_message instead
#[derive(Debug, Clone)]
pub struct LayershellCustomActionWithId(pub Option<IcedId>, pub LayershellCustomAction);

impl LayershellCustomActionWithId {
    pub fn new(id: Option<IcedId>, action: LayershellCustomAction) -> Self {
        Self(id, action)
    }
}

// first one means
#[derive(Debug, Clone)]
pub(crate) struct LayershellCustomActionWithIdInner(
    pub Option<LayerId>,        // come from
    pub Option<LayerId>,        // target if has one
    pub LayershellCustomAction, // actions
);
