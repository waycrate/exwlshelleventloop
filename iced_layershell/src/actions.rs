use crate::reexport::{Anchor, Layer};
use iced::window::Id as IcedId;
use iced_core::mouse::Interaction;
use iced_runtime::command::Action;
use layershellev::id::Id as LayerId;
#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum LayerShellActions {
    Mouse(Interaction),
    CustomActions(Vec<LayershellCustomActions>),
    CustomActionsWithId(Vec<LayershellCustomActionsWithIdInner>),
    RedrawAll,
    RedrawWindow(LayerId),
}

#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32)),
}

#[derive(Debug, Clone, Copy)]
pub struct LayershellCustomActionsWithId(pub IcedId, pub LayershellCustomActions);
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayershellCustomActionsWithIdInner(pub LayerId, pub LayershellCustomActions);

impl<T> From<LayershellCustomActionsWithId> for Action<T> {
    fn from(value: LayershellCustomActionsWithId) -> Self {
        Action::Custom(Box::new(value))
    }
}
impl<T> From<LayershellCustomActions> for Action<T> {
    fn from(value: LayershellCustomActions) -> Self {
        Action::Custom(Box::new(value))
    }
}

impl LayershellCustomActions {
    pub fn to_action<T>(&self) -> Action<T> {
        (*self).into()
    }
}
