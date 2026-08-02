//! Output identity and information.
//!
//! [`OutputInfo`] is sctk's, re-exported

pub use sctk::output::OutputInfo;

/// Identity of an output, the `wl_registry` global name
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutputId(pub u32);

impl From<&OutputInfo> for OutputId {
    fn from(info: &OutputInfo) -> Self {
        Self(info.id)
    }
}

/// Dimensions of the current mode, in pixels
pub fn pixel_size(info: &OutputInfo) -> Option<(i32, i32)> {
    info.modes
        .iter()
        .find(|mode| mode.current)
        .map(|mode| mode.dimensions)
}
