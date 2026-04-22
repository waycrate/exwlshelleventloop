use memmap2::MmapOptions;
use std::sync::LazyLock;
use std::{
    ops::Deref,
    os::fd::OwnedFd,
    ptr::{self, NonNull},
    time::Duration,
};
use wayland_client::{Proxy, protocol::wl_keyboard::WlKeyboard};

use xkbcommon_dl::{
    self as xkb, XkbCommon, XkbCommonCompose, xkb_keycode_t, xkb_keysym_t, xkb_layout_index_t,
    xkbcommon_compose_handle, xkbcommon_handle,
};

use xkb::{xkb_keymap, xkb_keymap_compile_flags};

pub use winit_common::xkb::{Context, KeyContext, XkbContext, XkbState};
pub use winit_core::event::{ElementState, KeyEvent};

use calloop::RegistrationToken;

pub static XKBH: LazyLock<&'static XkbCommon> = LazyLock::new(xkbcommon_handle);
pub static XKBCH: LazyLock<&'static XkbCommonCompose> = LazyLock::new(xkbcommon_compose_handle);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatInfo {
    /// Keys will be repeated at the specified rate and delay.
    Repeat {
        /// The time between the key repeats.
        gap: Duration,

        /// Delay (in milliseconds) between a key press and the start of repetition.
        delay: Duration,
    },

    /// Keys should not be repeated.
    Disable,
}

impl Default for RepeatInfo {
    /// The default repeat rate is 25 keys per second with the delay of 200ms.
    ///
    /// The values are picked based on the default in various compositors and Xorg.
    fn default() -> Self {
        Self::Repeat {
            gap: Duration::from_millis(40),
            delay: Duration::from_millis(200),
        }
    }
}

#[derive(Debug)]
pub struct KeyboardState {
    pub keyboard: WlKeyboard,

    pub xkb_context: Context,
    pub repeat_info: RepeatInfo,
    pub repeat_token: Option<RegistrationToken>,
    pub current_repeat: Option<u32>,
}

impl KeyboardState {
    pub fn new(keyboard: WlKeyboard) -> Self {
        Self {
            keyboard,
            xkb_context: Context::new().unwrap(),
            repeat_info: RepeatInfo::default(),
            current_repeat: None,
            repeat_token: None,
        }
    }
}

impl Drop for KeyboardState {
    fn drop(&mut self) {
        if self.keyboard.version() >= 3 {
            self.keyboard.release();
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// libxkbcommon is not available
    XKBNotFound,
}

#[derive(Debug)]
pub struct XkbKeymap {
    keymap: NonNull<xkb_keymap>,
}

impl XkbKeymap {
    pub fn from_fd(context: &XkbContext, fd: OwnedFd, size: usize) -> Option<Self> {
        let map = MmapOptions::new().len(size).map_raw_read_only(&fd).ok()?;
        let keymap = unsafe {
            let keymap = (XKBH.xkb_keymap_new_from_string)(
                (*context).as_ptr(),
                map.as_ptr() as *const _,
                xkb::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS,
            );

            NonNull::new(keymap)?
        };
        Some(Self { keymap })
    }

    pub fn first_keysym_by_level(
        &mut self,
        layout: xkb_layout_index_t,
        keycode: xkb_keycode_t,
    ) -> xkb_keysym_t {
        unsafe {
            let mut keysyms = ptr::null();
            let count = (XKBH.xkb_keymap_key_get_syms_by_level)(
                self.keymap.as_ptr(),
                keycode,
                layout,
                // NOTE: The level should be zero to ignore modifiers.
                0,
                &mut keysyms,
            );

            if count == 1 { *keysyms } else { 0 }
        }
    }
    /// Check whether the given key repeats.
    pub fn key_repeats(&mut self, keycode: xkb_keycode_t) -> bool {
        unsafe { (XKBH.xkb_keymap_key_repeats)(self.keymap.as_ptr(), keycode) == 1 }
    }
}

impl Drop for XkbKeymap {
    fn drop(&mut self) {
        unsafe { (XKBH.xkb_keymap_unref)(self.keymap.as_ptr()) }
    }
}

impl Deref for XkbKeymap {
    type Target = NonNull<xkb_keymap>;
    fn deref(&self) -> &Self::Target {
        &self.keymap
    }
}
