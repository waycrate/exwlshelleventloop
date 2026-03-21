//! XKB keymap.

use crate::keyboard::PhysicalKey;

pub use winit_common::xkb::scancode_to_physicalkey;

/// Map the raw X11-style keycode to the `KeyCode` enum.
///
/// X11-style keycodes are offset by 8 from the keycodes the Linux kernel uses.
pub fn raw_keycode_to_physicalkey(keycode: u32) -> PhysicalKey {
    scancode_to_physicalkey(keycode.saturating_sub(8))
}
