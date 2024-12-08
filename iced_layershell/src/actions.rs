use crate::reexport::{Anchor, Layer};
use iced::window::Id as IcedId;
use iced_core::mouse::Interaction;
use layershellev::id::Id as LayerId;
use layershellev::NewLayerShellSettings;

pub(crate) type LayerShellActionVec = Vec<LayerShellAction>;

#[derive(Debug, Clone)]
pub(crate) enum LayerShellAction {
    Mouse(Interaction),
    CustomActions(LayershellCustomActions),
    CustomActionsWithId(LayershellCustomActionsWithIdInner),
    RedrawAll,
    RedrawWindow(LayerId), // maybe one day it is useful, but now useless
    NewMenu((IcedNewPopupSettings, iced_core::window::Id)),
}

pub trait IsSingleton {
    fn is_singleton(&self) -> bool {
        false
    }
}

pub struct MainWindowInfo;

impl TryFrom<MainWindowInfo> for () {
    type Error = ();
    fn try_from(_: MainWindowInfo) -> Result<(), Self::Error> {
        Err(())
    }
}

impl IsSingleton for () {}

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

/// NOTE: DO NOT USE THIS ENUM DIERCTLY
/// use macro to_layer_message
#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    AnchorSizeChange(Anchor, (u32, u32)),
    MarginChange((i32, i32, i32, i32)),
    SizeChange((u32, u32)),
    VirtualKeyboardPressed {
        time: u32,
        key: u32,
    },
    // settings, info, single_tone
    NewLayerShell {
        settings: NewLayerShellSettings,
        info: IcedId,
    },
    NewPopUp {
        settings: IcedNewPopupSettings,
        info: IcedId,
    },
    NewMenu {
        settings: IcedNewMenuSettings,
        info: IcedId,
    },
    /// is same with WindowAction::Close(id)
    RemoveWindow(IcedId),
    ForgetLastOutput,
}

/// Please do not use this struct directly
/// Use macro to_layer_message instead
#[derive(Debug, Clone, Copy)]
pub struct LayershellCustomActionsWithId(pub Option<IcedId>, pub LayershellCustomActions);

impl LayershellCustomActionsWithId {
    pub fn new(id: Option<IcedId>, actions: LayershellCustomActions) -> Self {
        Self(id, actions)
    }
}

// first one means
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayershellCustomActionsWithIdInner(
    pub Option<LayerId>,         // come from
    pub Option<LayerId>,         // target if has one
    pub LayershellCustomActions, // actions
);
