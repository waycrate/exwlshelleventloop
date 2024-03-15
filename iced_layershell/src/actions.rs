use crate::reexport::{Anchor, Layer};
use iced_core::mouse::Interaction;

#[derive(Debug, Clone)]
pub(crate) enum LayerShellActions {
    Mouse(Interaction),
    CustomActions(Vec<LayershellCustomActions>),
}

#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32)),
}
