use crate::reexport::{Anchor, Layer};
use iced_core::mouse::Interaction;
use iced_runtime::command::Action;

#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum LayerShellActions {
    Mouse(Interaction),
    CustomActions(Vec<LayershellCustomActions>),
    RedrawAll,
    RedrawWindow(usize)
}

#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32)),
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
