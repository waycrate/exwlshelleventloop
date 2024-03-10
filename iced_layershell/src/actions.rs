use iced_core::mouse::Interaction;
use crate::reexport::{Anchor, Layer};

#[derive(Debug, Clone)]
pub(crate) enum LayerShellActions {
    Mouse(Interaction),
    CustomActions(Vec<LayershellCustomActions>)
}

#[derive(Debug, Clone, Copy)]
pub enum LayershellCustomActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32)),
    CloseWindow,
}
