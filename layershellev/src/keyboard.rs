use memmap2::MmapOptions;
use smol_str::SmolStr;
use std::sync::LazyLock;
use std::{
    env,
    ffi::{c_char, CString},
    ops::Deref,
    os::{fd::OwnedFd, unix::ffi::OsStringExt},
    ptr::{self, NonNull},
    time::Duration,
};
use wayland_client::{protocol::wl_keyboard::WlKeyboard, Proxy};

use xkbcommon_dl::{
    self as xkb, xkb_compose_compile_flags, xkb_compose_feed_result, xkb_compose_state,
    xkb_compose_state_flags, xkb_compose_status, xkb_compose_table, xkb_keysym_t,
    xkbcommon_compose_handle, xkbcommon_handle, XkbCommon, XkbCommonCompose,
};

use xkb::{
    xkb_context, xkb_context_flags, xkb_keymap, xkb_keymap_compile_flags, xkb_state,
    xkb_state_component,
};

static XKBH: LazyLock<&'static XkbCommon> = LazyLock::new(xkbcommon_handle);
static XKBCH: LazyLock<&'static XkbCommonCompose> = LazyLock::new(xkbcommon_compose_handle);

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
    pub current_repeat: Option<u32>,
}

impl KeyboardState {
    pub fn new(keyboard: WlKeyboard) -> Self {
        Self {
            keyboard,
            xkb_context: Context::new().unwrap(),
            repeat_info: RepeatInfo::default(),
            current_repeat: None,
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
pub struct Context {
    // NOTE: field order matters.
    state: Option<XkbState>,
    keymap: Option<XkbKeymap>,
    compose_state1: Option<XkbComposeState>,
    compose_state2: Option<XkbComposeState>,
    _compose_table: Option<XkbComposeTable>,
    context: XkbContext,
    scratch_buffer: Vec<u8>,
}

impl Context {
    pub fn new() -> Result<Self, Error> {
        if xkb::xkbcommon_option().is_none() {
            return Err(Error::XKBNotFound);
        }

        let context = XkbContext::new();
        let mut compose_table = XkbComposeTable::new(&context);
        let mut compose_state1 = compose_table.as_ref().and_then(|table| table.new_state());
        let mut compose_state2 = compose_table.as_ref().and_then(|table| table.new_state());

        // Disable compose if anything compose related failed to initialize.
        if compose_table.is_none() || compose_state1.is_none() || compose_state2.is_none() {
            compose_state2 = None;
            compose_state1 = None;
            compose_table = None;
        }

        Ok(Self {
            state: None,
            keymap: None,
            compose_state1,
            compose_state2,
            _compose_table: compose_table,
            context,
            scratch_buffer: Vec::with_capacity(8),
        })
    }
    pub fn set_keymap_from_fd(&mut self, fd: OwnedFd, size: usize) {
        let keymap = XkbKeymap::from_fd(&self.context, fd, size);
        let state = keymap.as_ref().and_then(XkbState::new_wayland);
        if keymap.is_none() || state.is_none() {
            log::warn!("failed to update xkb keymap");
        }
        self.state = state;
        self.keymap = keymap;
    }
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

#[derive(Debug)]
pub struct XkbComposeTable {
    table: NonNull<xkb_compose_table>,
}

impl XkbComposeTable {
    pub fn new(context: &XkbContext) -> Option<Self> {
        let locale = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into());
        let locale = CString::new(locale.into_vec()).unwrap();

        let table = unsafe {
            (XKBCH.xkb_compose_table_new_from_locale)(
                context.as_ptr(),
                locale.as_ptr(),
                xkb_compose_compile_flags::XKB_COMPOSE_COMPILE_NO_FLAGS,
            )
        };

        let table = NonNull::new(table)?;
        Some(Self { table })
    }

    /// Create new state with the given compose table.
    pub fn new_state(&self) -> Option<XkbComposeState> {
        let state = unsafe {
            (XKBCH.xkb_compose_state_new)(
                self.table.as_ptr(),
                xkb_compose_state_flags::XKB_COMPOSE_STATE_NO_FLAGS,
            )
        };

        let state = NonNull::new(state)?;
        Some(XkbComposeState { state })
    }
}

impl Deref for XkbComposeTable {
    type Target = NonNull<xkb_compose_table>;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

impl Drop for XkbComposeTable {
    fn drop(&mut self) {
        unsafe {
            (XKBCH.xkb_compose_table_unref)(self.table.as_ptr());
        }
    }
}

#[derive(Debug)]
pub struct XkbComposeState {
    state: NonNull<xkb_compose_state>,
}

// NOTE: This is track_caller so we can have more informative line numbers when logging
#[track_caller]
fn byte_slice_to_smol_str(bytes: &[u8]) -> Option<SmolStr> {
    std::str::from_utf8(bytes)
        .map(SmolStr::new)
        .map_err(|e| {
            log::warn!(
                "UTF-8 received from libxkbcommon ({:?}) was invalid: {e}",
                bytes
            )
        })
        .ok()
}

/// Shared logic for constructing a string with `xkb_compose_state_get_utf8` and
/// `xkb_state_key_get_utf8`.
fn make_string_with<F>(scratch_buffer: &mut Vec<u8>, mut f: F) -> Option<SmolStr>
where
    F: FnMut(*mut c_char, usize) -> i32,
{
    let size = f(ptr::null_mut(), 0);
    if size == 0 {
        return None;
    }
    let size = usize::try_from(size).unwrap();
    scratch_buffer.clear();
    // The allocated buffer must include space for the null-terminator.
    scratch_buffer.reserve(size + 1);
    unsafe {
        let written = f(
            scratch_buffer.as_mut_ptr().cast(),
            scratch_buffer.capacity(),
        );
        if usize::try_from(written).unwrap() != size {
            // This will likely never happen.
            return None;
        }
        scratch_buffer.set_len(size);
    };

    byte_slice_to_smol_str(scratch_buffer)
}

impl XkbComposeState {
    pub fn get_string(&mut self, scratch_buffer: &mut Vec<u8>) -> Option<SmolStr> {
        make_string_with(scratch_buffer, |ptr, len| unsafe {
            (XKBCH.xkb_compose_state_get_utf8)(self.state.as_ptr(), ptr, len)
        })
    }

    #[inline]
    pub fn feed(&mut self, keysym: xkb_keysym_t) -> ComposeStatus {
        let feed_result = unsafe { (XKBCH.xkb_compose_state_feed)(self.state.as_ptr(), keysym) };
        match feed_result {
            xkb_compose_feed_result::XKB_COMPOSE_FEED_IGNORED => ComposeStatus::Ignored,
            xkb_compose_feed_result::XKB_COMPOSE_FEED_ACCEPTED => {
                ComposeStatus::Accepted(self.status())
            }
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        unsafe {
            (XKBCH.xkb_compose_state_reset)(self.state.as_ptr());
        }
    }

    #[inline]
    pub fn status(&mut self) -> xkb_compose_status {
        unsafe { (XKBCH.xkb_compose_state_get_status)(self.state.as_ptr()) }
    }
}

impl Drop for XkbComposeState {
    fn drop(&mut self) {
        unsafe {
            (XKBCH.xkb_compose_state_unref)(self.state.as_ptr());
        };
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ComposeStatus {
    Accepted(xkb_compose_status),
    Ignored,
    None,
}
