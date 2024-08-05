use crate::reexport::{Anchor, Layer};
use iced::window::Id as IcedId;
use iced_core::mouse::Interaction;
use iced_runtime::command::Action;
use layershellev::id::Id as LayerId;
use layershellev::NewLayerShellSettings;
#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum LayerShellActions<INFO: Clone> {
    Mouse(Interaction),
    CustomActions(Vec<LayershellCustomActionsWithInfo<INFO>>),
    CustomActionsWithId(Vec<LayershellCustomActionsWithIdInner<INFO>>),
    RedrawAll,
    RedrawWindow(LayerId),
}

#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActionsWithInfo<INFO: Clone> {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32)),
    VirtualKeyboardPressed { time: u32, key: u32 },
    NewLayerShell((NewLayerShellSettings, INFO)),
    RemoveLayerShell(IcedId),
}

pub type LayershellCustomActions = LayershellCustomActionsWithInfo<()>;

#[derive(Debug, Clone, Copy)]
pub struct LayershellCustomActionsWithIdAndInfo<INFO: Clone>(
    pub IcedId,
    pub LayershellCustomActionsWithInfo<INFO>,
);

impl<INFO: Clone> LayershellCustomActionsWithIdAndInfo<INFO> {
    pub fn new(id: IcedId, actions: LayershellCustomActionsWithInfo<INFO>) -> Self {
        Self(id, actions)
    }
}

pub type LayershellCustomActionsWithId = LayershellCustomActionsWithIdAndInfo<()>;

// first one means
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayershellCustomActionsWithIdInner<INFO: Clone>(
    pub LayerId,
    pub Option<LayerId>,
    pub LayershellCustomActionsWithInfo<INFO>,
);

impl<T, INFO: Clone + 'static> From<LayershellCustomActionsWithIdAndInfo<INFO>> for Action<T> {
    fn from(value: LayershellCustomActionsWithIdAndInfo<INFO>) -> Self {
        Action::Custom(Box::new(value.clone()))
    }
}
impl<T, INFO: Clone + 'static> From<LayershellCustomActionsWithInfo<INFO>> for Action<T> {
    fn from(value: LayershellCustomActionsWithInfo<INFO>) -> Self {
        Action::Custom(Box::new(value.clone()))
    }
}

impl<INFO: Clone + 'static> LayershellCustomActionsWithInfo<INFO> {
    pub fn to_action<T>(&self) -> Action<T> {
        (*self).clone().into()
    }
}
