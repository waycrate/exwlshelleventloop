use crate::reexport::{Anchor, Layer};
#[derive(Debug, Clone, Copy)]
pub enum LayershellActions {
    AnchorChange(Anchor),
    LayerChange(Layer),
    SizeChange((u32, u32))
}
