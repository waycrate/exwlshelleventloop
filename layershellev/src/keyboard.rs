use std::{ffi::c_char, ops::Deref, os::fd::OwnedFd, ptr::NonNull};

use memmap2::MmapOptions;
use once_cell::sync::Lazy;

use xkbcommon_dl::{self as xkb, xkbcommon_handle, XkbCommon};

use xkb::{
    xkb_context, xkb_context_flags, xkb_keymap, xkb_keymap_compile_flags, xkb_state,
    xkb_state_component,
};

static XKBH: Lazy<&'static XkbCommon> = Lazy::new(xkbcommon_handle);

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

#[derive(Debug)]
pub struct XkbContext {
    context: NonNull<xkb_context>,
}

impl Drop for XkbContext {
    fn drop(&mut self) {
        unsafe { (XKBH.xkb_context_unref)(self.context.as_ptr()) }
    }
}

impl Deref for XkbContext {
    type Target = NonNull<xkb_context>;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl XkbContext {
    pub fn new() -> Self {
        let context = unsafe { (XKBH.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS) };
        let context = NonNull::new(context).unwrap();
        Self { context }
    }
}

#[derive(Debug)]
pub struct XkbState {
    state: NonNull<xkb_state>,
    modifiers: ModifiersState,
}

impl XkbState {
    pub fn new_wayland(keymap: &XkbKeymap) -> Option<Self> {
        let state = NonNull::new(unsafe { (XKBH.xkb_state_new)(keymap.as_ptr()) })?;
        Some(Self::new_inner(state))
    }

    fn new_inner(state: NonNull<xkb_state>) -> Self {
        let modifiers = ModifiersState::default();
        let mut this = Self { state, modifiers };
        this.reload_modifiers();
        this
    }
    // NOTE: read here
    /// Check if the modifier is active within xkb.
    fn mod_name_is_active(&mut self, name: &[u8]) -> bool {
        unsafe {
            (XKBH.xkb_state_mod_name_is_active)(
                self.state.as_ptr(),
                name.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE,
            ) > 0
        }
    }
    fn reload_modifiers(&mut self) {
        self.modifiers.ctrl = self.mod_name_is_active(xkb::XKB_MOD_NAME_CTRL);
        self.modifiers.alt = self.mod_name_is_active(xkb::XKB_MOD_NAME_ALT);
        self.modifiers.shift = self.mod_name_is_active(xkb::XKB_MOD_NAME_SHIFT);
        self.modifiers.caps_lock = self.mod_name_is_active(xkb::XKB_MOD_NAME_CAPS);
        println!("caps: {}", self.modifiers.caps_lock);
        self.modifiers.logo = self.mod_name_is_active(xkb::XKB_MOD_NAME_LOGO);
        self.modifiers.num_lock = self.mod_name_is_active(xkb::XKB_MOD_NAME_NUM);
    }
}

#[derive(Debug, Default)]
pub struct ModifiersState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    caps_lock: bool,
    logo: bool,
    num_lock: bool,
}

