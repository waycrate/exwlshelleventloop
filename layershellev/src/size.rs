//! Surface sizes, in the shapes the wayland protocols accept.
//!
//! A `0` is legal only on a layer surface axis, where `set_size` reads it as
//! "the compositor assigns this one" and requires both opposite edges to be
//! anchored. No other size here takes `0`.
use std::num::NonZeroU32;

use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Anchor;

const HORIZONTAL_EDGES: Anchor = Anchor::Left.union(Anchor::Right);
const VERTICAL_EDGES: Anchor = Anchor::Top.union(Anchor::Bottom);

/// The extent requested for a layer surface along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Extent {
    /// exact extent as surface-local pixels
    Exact(NonZeroU32),
    /// let the compositor pick
    Fill,
}

impl Extent {
    /// exact extent, panics if 0. Use [`Extent::try_px`] for a run-time value
    pub const fn px(value: u32) -> Self {
        match NonZeroU32::new(value) {
            Some(value) => Extent::Exact(value),
            None => panic!(
                "a layer surface extent must be non-zero. Use Extent::Fill to let the compositor pick it"
            ),
        }
    }

    /// exact extent, None on 0.
    pub const fn try_px(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Extent::Exact(value)),
            None => None,
        }
    }

    /// if extent is left to the compositor
    pub const fn is_fill(self) -> bool {
        matches!(self, Extent::Fill)
    }

    /// the value to send in `set_size`: the extent, or `0` for [`Extent::Fill`]
    pub const fn to_set(self) -> u32 {
        match self {
            Extent::Exact(value) => value.get(),
            Extent::Fill => 0,
        }
    }
}

/// size requested for layer surface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerSize {
    pub width: Extent,
    pub height: Extent,
}

impl LayerSize {
    /// both axes left to the compositor, which needs all four anchor edges
    pub const FILL: Self = LayerSize {
        width: Extent::Fill,
        height: Extent::Fill,
    };

    /// both axes exact. panics if either is 0.
    pub const fn px(width: u32, height: u32) -> Self {
        LayerSize {
            width: Extent::px(width),
            height: Extent::px(height),
        }
    }

    /// compositor-chosen width, exact height. panic if height is 0
    pub const fn fill_width(height: u32) -> Self {
        LayerSize {
            width: Extent::Fill,
            height: Extent::px(height),
        }
    }

    /// exact width, compositor-chosen height. panic if width is 0
    pub const fn fill_height(width: u32) -> Self {
        LayerSize {
            width: Extent::px(width),
            height: Extent::Fill,
        }
    }

    /// anchor edges size requires
    const fn required_anchor(self) -> Anchor {
        let mut anchor = Anchor::empty();
        if self.width.is_fill() {
            anchor = anchor.union(HORIZONTAL_EDGES);
        }
        if self.height.is_fill() {
            anchor = anchor.union(VERTICAL_EDGES);
        }
        anchor
    }

    /// edges that size requires and anchor does not have
    pub const fn missing_edges(self, anchor: Anchor) -> Anchor {
        self.required_anchor().difference(anchor)
    }

    /// add missing anchors
    pub const fn resolve_anchor(self, anchor: Anchor) -> Anchor {
        anchor.union(self.missing_edges(anchor))
    }

    /// width and height to set in set_size
    pub const fn to_set(self) -> (u32, u32) {
        (self.width.to_set(), self.height.to_set())
    }
}

/// Exact, non-zero size in surface-local pixels, for sizes no protocol
/// lets you ask the compositor to choose.
///
/// there is no `0` here that means "you pick", the way `set_size` has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelSize {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
}

impl PixelSize {
    /// panics if either is 0. use [`PixelSize::try_px`] for run-time values
    pub const fn px(width: u32, height: u32) -> Self {
        match (NonZeroU32::new(width), NonZeroU32::new(height)) {
            (Some(width), Some(height)) => PixelSize { width, height },
            _ => panic!("a surface size must be non-zero on both axes"),
        }
    }

    /// exact size, if either is 0 then None
    pub const fn try_px(width: u32, height: u32) -> Option<Self> {
        match (NonZeroU32::new(width), NonZeroU32::new(height)) {
            (Some(width), Some(height)) => Some(PixelSize { width, height }),
            _ => None,
        }
    }

    pub const fn to_set(self) -> (u32, u32) {
        (self.width.get(), self.height.get())
    }

    /// Same size for signed integer requests, capped at [`i32::MAX`]
    /// so the value can never become negative on the set.
    pub const fn to_set_i32(self) -> (i32, i32) {
        const fn clamp(value: u32) -> i32 {
            if value > i32::MAX as u32 {
                i32::MAX
            } else {
                value as i32
            }
        }
        (clamp(self.width.get()), clamp(self.height.get()))
    }
}

/// check if positive exclusive zone is meaningful for anchor
pub(crate) fn exclusive_zone_is_meaningful(anchor: Anchor) -> bool {
    let spans_horizontally = anchor.contains(HORIZONTAL_EDGES);
    let spans_vertically = anchor.contains(VERTICAL_EDGES);
    match (spans_horizontally, spans_vertically) {
        // all four edges: no free edge to reserve space from
        (true, true) => false,
        // an edge plus both perpendicular edges
        (true, false) => anchor.intersects(VERTICAL_EDGES),
        (false, true) => anchor.intersects(HORIZONTAL_EDGES),
        // a corner, or nothing, invalid
        (false, false) => anchor.bits().count_ones() == 1,
    }
}

/// warn on positive zone paired with anchor the compositor clamps to zero.
pub(crate) fn warn_if_exclusive_zone_ignored(zone: i32, anchor: Anchor) {
    if zone > 0 && !exclusive_zone_is_meaningful(anchor) {
        log::warn!(
            "exclusive zone {zone} treated as 0: {anchor:?} is not a single edge, \
             or an edge with both perpendicular ones"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Anchor = HORIZONTAL_EDGES.union(VERTICAL_EDGES);

    #[test]
    fn exclusive_zone_meaningfulness_follows_the_spec() {
        // one edge, and an edge with both perpendicular ones - either axis
        for anchor in [
            Anchor::Bottom,
            Anchor::Bottom | Anchor::Left | Anchor::Right,
            Anchor::Left | Anchor::Top | Anchor::Bottom,
        ] {
            assert!(exclusive_zone_is_meaningful(anchor), "{anchor:?}");
        }
        // nothing, a corner, two parallel edges, all four
        for anchor in [
            Anchor::empty(),
            Anchor::Top | Anchor::Left,
            Anchor::Left | Anchor::Right,
            Anchor::Top | Anchor::Bottom,
            ALL,
        ] {
            assert!(!exclusive_zone_is_meaningful(anchor), "{anchor:?}");
        }
    }

    /// `0` at run-time is an `Option` to handle, not a panic
    #[test]
    fn a_zero_is_rejected_or_spelled_out() {
        assert_eq!(Extent::try_px(0), None);
        assert_eq!(PixelSize::try_px(0, 10), None);
        assert_eq!(PixelSize::try_px(10, 0), None);
        assert!(PixelSize::try_px(10, 10).is_some());
    }
}
