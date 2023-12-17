pub const LEFT: u32 = 105;
pub const RIGHT: u32 = 106;
pub const DOWN: u32 = 108;
pub const UP: u32 = 103;
pub const ESC: u32 = 1;
pub const SHIFT_LEFT: u32 = 42;
pub const SHIFT_RIGHT: u32 = 54;
pub const MENU: u32 = 139;
pub const CAPS_LOCK: u32 = 58;
pub const CTRL_LEFT: u32 = 29;
pub const CTRL_RIGHT: u32 = 97;
pub const ALT_LEFT: u32 = 56;
pub const ALT_RIGHT: u32 = 100;

pub const MIN_KEYBOARD: u32 = 999;
pub const CLOSE_KEYBOARD: u32 = 1000;

pub fn is_unique_key(key: u32) -> bool {
    key == MIN_KEYBOARD || key == CLOSE_KEYBOARD
}
