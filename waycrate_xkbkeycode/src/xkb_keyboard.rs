use memmap2::MmapOptions;
use smol_str::SmolStr;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    env,
    ffi::{CString, c_char},
    ops::Deref,
    os::{fd::OwnedFd, unix::ffi::OsStringExt},
    ptr::{self, NonNull},
    time::Duration,
};
use wayland_client::{Proxy, protocol::wl_keyboard::WlKeyboard};

use crate::keymap;

use xkbcommon_dl::{
    self as xkb, XkbCommon, XkbCommonCompose, xkb_compose_compile_flags, xkb_compose_feed_result,
    xkb_compose_state, xkb_compose_state_flags, xkb_compose_status, xkb_compose_table,
    xkb_keycode_t, xkb_keysym_t, xkb_layout_index_t, xkbcommon_compose_handle, xkbcommon_handle,
};

use crate::keyboard::ModifiersState;
use xkb::{
    xkb_context, xkb_context_flags, xkb_keymap, xkb_keymap_compile_flags, xkb_state,
    xkb_state_component,
};

use crate::keyboard::{Key, KeyLocation, PhysicalKey};

use calloop::RegistrationToken;

static RESET_DEAD_KEYS: AtomicBool = AtomicBool::new(false);

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

    pub fn state_mut(&mut self) -> Option<&mut XkbState> {
        self.state.as_mut()
    }

    pub fn keymap_mut(&mut self) -> Option<&mut XkbKeymap> {
        self.keymap.as_mut()
    }
    /// Key builder context with the user provided xkb state.
    pub fn key_context(&mut self) -> Option<KeyContext<'_>> {
        let state = self.state.as_mut()?;
        let keymap = self.keymap.as_mut()?;
        let compose_state1 = self.compose_state1.as_mut();
        let compose_state2 = self.compose_state2.as_mut();
        let scratch_buffer = &mut self.scratch_buffer;
        Some(KeyContext {
            state,
            keymap,
            compose_state1,
            compose_state2,
            scratch_buffer,
        })
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

impl Default for XkbContext {
    fn default() -> Self {
        Self::new()
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
    modifiers: ModifiersStateXkb,
}

impl XkbState {
    pub fn new_wayland(keymap: &XkbKeymap) -> Option<Self> {
        let state = NonNull::new(unsafe { (XKBH.xkb_state_new)(keymap.as_ptr()) })?;
        Some(Self::new_inner(state))
    }

    fn new_inner(state: NonNull<xkb_state>) -> Self {
        let modifiers = ModifiersStateXkb::default();
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
    pub fn modifiers(&self) -> ModifiersStateXkb {
        self.modifiers
    }
    pub fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        depressed_group: u32,
        latched_group: u32,
        locked_group: u32,
    ) {
        let mask = unsafe {
            (XKBH.xkb_state_update_mask)(
                self.state.as_ptr(),
                mods_depressed,
                mods_latched,
                mods_locked,
                depressed_group,
                latched_group,
                locked_group,
            )
        };

        if mask.contains(xkb_state_component::XKB_STATE_MODS_EFFECTIVE) {
            // Effective value of mods have changed, we need to update our state.
            self.reload_modifiers();
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

    pub fn get_one_sym_raw(&mut self, keycode: xkb_keycode_t) -> xkb_keysym_t {
        unsafe { (XKBH.xkb_state_key_get_one_sym)(self.state.as_ptr(), keycode) }
    }

    pub fn layout(&mut self, key: xkb_keycode_t) -> xkb_layout_index_t {
        unsafe { (XKBH.xkb_state_key_get_layout)(self.state.as_ptr(), key) }
    }

    pub fn get_utf8_raw(
        &mut self,
        keycode: xkb_keycode_t,
        scratch_buffer: &mut Vec<u8>,
    ) -> Option<SmolStr> {
        make_string_with(scratch_buffer, |ptr, len| unsafe {
            (XKBH.xkb_state_key_get_utf8)(self.state.as_ptr(), keycode, ptr, len)
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ModifiersStateXkb {
    ctrl: bool,
    alt: bool,
    shift: bool,
    caps_lock: bool,
    logo: bool,
    num_lock: bool,
}

impl From<ModifiersStateXkb> for ModifiersState {
    fn from(mods: ModifiersStateXkb) -> ModifiersState {
        let mut to_mods = ModifiersState::empty();
        to_mods.set(ModifiersState::SHIFT, mods.shift);
        to_mods.set(ModifiersState::CONTROL, mods.ctrl);
        to_mods.set(ModifiersState::ALT, mods.alt);
        to_mods.set(ModifiersState::SUPER, mods.logo);
        to_mods
    }
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

pub struct KeyContext<'a> {
    pub state: &'a mut XkbState,
    pub keymap: &'a mut XkbKeymap,
    compose_state1: Option<&'a mut XkbComposeState>,
    compose_state2: Option<&'a mut XkbComposeState>,
    scratch_buffer: &'a mut Vec<u8>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
    pub text_with_all_modifiers: Option<SmolStr>,
    pub key_without_modifiers: Key,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEvent {
    /// Represents the position of a key independent of the currently active layout.
    ///
    /// It also uniquely identifies the physical key (i.e. it's mostly synonymous with a scancode).
    /// The most prevalent use case for this is games. For example the default keys for the player
    /// to move around might be the W, A, S, and D keys on a US layout. The position of these keys
    /// is more important than their label, so they should map to Z, Q, S, and D on an "AZERTY"
    /// layout. (This value is `KeyCode::KeyW` for the Z key on an AZERTY layout.)
    ///
    /// ## Caveats
    ///
    /// - Certain niche hardware will shuffle around physical key positions, e.g. a keyboard that
    ///   implements DVORAK in hardware (or firmware)
    /// - Your application will likely have to handle keyboards which are missing keys that your
    ///   own keyboard has.
    /// - Certain `KeyCode`s will move between a couple of different positions depending on what
    ///   layout the keyboard was manufactured to support.
    ///
    ///  **Because of these caveats, it is important that you provide users with a way to configure
    ///  most (if not all) keybinds in your application.**
    ///
    /// ## `Fn` and `FnLock`
    ///
    /// `Fn` and `FnLock` key events are *exceedingly unlikely* to be emitted by Winit. These keys
    /// are usually handled at the hardware or OS level, and aren't surfaced to applications. If
    /// you somehow see this in the wild, we'd like to know :)
    pub physical_key: PhysicalKey,

    /// This value is affected by all modifiers except <kbd>Ctrl</kbd>.
    ///
    /// This has two use cases:
    /// - Allows querying whether the current input is a Dead key.
    /// - Allows handling key-bindings on platforms which don't
    ///   support `key_without_modifiers`.
    ///
    /// If you use this field (or `key_without_modifiers` for that matter) for keyboard
    /// shortcuts, **it is important that you provide users with a way to configure your
    /// application's shortcuts so you don't render your application unusable for users with an
    /// incompatible keyboard layout.**
    ///
    /// ## Platform-specific
    /// - **Web:** Dead keys might be reported as the real key instead
    ///   of `Dead` depending on the browser/OS.
    ///
    pub logical_key: Key,

    /// Contains the text produced by this keypress.
    ///
    /// In most cases this is identical to the content
    /// of the `Character` variant of `logical_key`.
    /// However, on Windows when a dead key was pressed earlier
    /// but cannot be combined with the character from this
    /// keypress, the produced text will consist of two characters:
    /// the dead-key-character followed by the character resulting
    /// from this keypress.
    ///
    /// An additional difference from `logical_key` is that
    /// this field stores the text representation of any key
    /// that has such a representation. For example when
    /// `logical_key` is `Key::Named(NamedKey::Enter)`, this field is `Some("\r")`.
    ///
    /// This is `None` if the current keypress cannot
    /// be interpreted as text.
    ///
    /// See also: `text_with_all_modifiers()`
    pub text: Option<SmolStr>,

    /// Contains the location of this key on the keyboard.
    ///
    /// Certain keys on the keyboard may appear in more than once place. For example, the "Shift"
    /// key appears on the left side of the QWERTY keyboard as well as the right side. However,
    /// both keys have the same symbolic value. Another example of this phenomenon is the "1"
    /// key, which appears both above the "Q" key and as the "Keypad 1" key.
    ///
    /// This field allows the user to differentiate between keys like this that have the same
    /// symbolic value but different locations on the keyboard.
    ///
    /// See the [`KeyLocation`] type for more details.
    ///
    /// [`KeyLocation`]: crate::keyboard::KeyLocation
    pub location: KeyLocation,

    /// Whether the key is being pressed or released.
    ///
    /// See the [`ElementState`] type for more details.
    pub state: ElementState,

    /// Whether or not this key is a key repeat event.
    ///
    /// On some systems, holding down a key for some period of time causes that key to be repeated
    /// as though it were being pressed and released repeatedly. This field is `true` if and only
    /// if this event is the result of one of those repeats.
    ///
    pub repeat: bool,

    /// Platform-specific key event information.
    ///
    /// On Windows, Linux and macOS, this type contains the key without modifiers and the text with
    /// all modifiers applied.
    ///
    /// On Android, iOS, Redox and Web, this type is a no-op.
    pub(crate) platform_specific: KeyEventExtra,
}

impl KeyEvent {
    #[inline]
    pub fn text_with_all_modifiers(&self) -> Option<&str> {
        self.platform_specific
            .text_with_all_modifiers
            .as_ref()
            .map(|s| s.as_str())
    }

    #[inline]
    pub fn key_without_modifiers(&self) -> Key {
        self.platform_specific.key_without_modifiers.clone()
    }
}

impl KeyContext<'_> {
    pub fn process_key_event(
        &mut self,
        keycode: u32,
        state: ElementState,
        repeat: bool,
    ) -> KeyEvent {
        let mut event =
            KeyEventResults::new(self, keycode, !repeat && state == ElementState::Pressed);
        let physical_key = keymap::raw_keycode_to_physicalkey(keycode);
        let (logical_key, location) = event.key();
        let text = event.text();
        let (key_without_modifiers, _) = event.key_without_modifiers();
        let text_with_all_modifiers = event.text_with_all_modifiers();

        let platform_specific = KeyEventExtra {
            text_with_all_modifiers,
            key_without_modifiers,
        };

        KeyEvent {
            physical_key,
            logical_key,
            text,
            location,
            state,
            repeat,
            platform_specific,
        }
    }

    fn keysym_to_utf8_raw(&mut self, keysym: u32) -> Option<SmolStr> {
        self.scratch_buffer.clear();
        self.scratch_buffer.reserve(8);
        loop {
            let bytes_written = unsafe {
                (XKBH.xkb_keysym_to_utf8)(
                    keysym,
                    self.scratch_buffer.as_mut_ptr().cast(),
                    self.scratch_buffer.capacity(),
                )
            };
            if bytes_written == 0 {
                return None;
            } else if bytes_written == -1 {
                self.scratch_buffer.reserve(8);
            } else {
                unsafe {
                    self.scratch_buffer
                        .set_len(bytes_written.try_into().unwrap())
                };
                break;
            }
        }

        // Remove the null-terminator
        self.scratch_buffer.pop();
        byte_slice_to_smol_str(self.scratch_buffer)
    }
}

struct KeyEventResults<'a, 'b> {
    context: &'a mut KeyContext<'b>,
    keycode: u32,
    keysym: u32,
    compose: ComposeStatus,
}

impl<'a, 'b> KeyEventResults<'a, 'b> {
    fn new(context: &'a mut KeyContext<'b>, keycode: u32, compose: bool) -> Self {
        let keysym = context.state.get_one_sym_raw(keycode);

        let compose = if let Some(state) = context.compose_state1.as_mut().filter(|_| compose) {
            if RESET_DEAD_KEYS.swap(false, Ordering::SeqCst) {
                state.reset();
                context.compose_state2.as_mut().unwrap().reset();
            }
            state.feed(keysym)
        } else {
            ComposeStatus::None
        };

        KeyEventResults {
            context,
            keycode,
            keysym,
            compose,
        }
    }

    pub fn key(&mut self) -> (Key, KeyLocation) {
        let (key, location) = match self.keysym_to_key(self.keysym) {
            Ok(known) => return known,
            Err(undefined) => undefined,
        };

        if let ComposeStatus::Accepted(xkb_compose_status::XKB_COMPOSE_COMPOSING) = self.compose {
            let compose_state = self.context.compose_state2.as_mut().unwrap();
            // When pressing a dead key twice, the non-combining variant of that character will
            // be produced. Since this function only concerns itself with a single keypress, we
            // simulate this double press here by feeding the keysym to the compose state
            // twice.

            compose_state.feed(self.keysym);
            if matches!(compose_state.feed(self.keysym), ComposeStatus::Accepted(_)) {
                // Extracting only a single `char` here *should* be fine, assuming that no
                // dead key's non-combining variant ever occupies more than one `char`.
                let text = compose_state.get_string(self.context.scratch_buffer);
                let key = Key::Dead(text.and_then(|s| s.chars().next()));
                (key, location)
            } else {
                (key, location)
            }
        } else {
            let key = self
                .composed_text()
                .unwrap_or_else(|_| self.context.keysym_to_utf8_raw(self.keysym))
                .map(Key::Character)
                .unwrap_or(key);
            (key, location)
        }
    }

    pub fn key_without_modifiers(&mut self) -> (Key, KeyLocation) {
        // This will become a pointer to an array which libxkbcommon owns, so we don't need to
        // deallocate it.
        let layout = self.context.state.layout(self.keycode);
        let keysym = self
            .context
            .keymap
            .first_keysym_by_level(layout, self.keycode);

        match self.keysym_to_key(keysym) {
            Ok((key, location)) => (key, location),
            Err((key, location)) => {
                let key = self
                    .context
                    .keysym_to_utf8_raw(keysym)
                    .map(Key::Character)
                    .unwrap_or(key);
                (key, location)
            }
        }
    }

    fn keysym_to_key(&self, keysym: u32) -> Result<(Key, KeyLocation), (Key, KeyLocation)> {
        let location = keymap::keysym_location(keysym);
        let key = keymap::keysym_to_key(keysym);
        if matches!(key, Key::Unidentified(_)) {
            Err((key, location))
        } else {
            Ok((key, location))
        }
    }

    pub fn text(&mut self) -> Option<SmolStr> {
        self.composed_text()
            .unwrap_or_else(|_| self.context.keysym_to_utf8_raw(self.keysym))
    }

    // The current behaviour makes it so composing a character overrides attempts to input a
    // control character with the `Ctrl` key. We can potentially add a configuration option
    // if someone specifically wants the opposite behaviour.
    pub fn text_with_all_modifiers(&mut self) -> Option<SmolStr> {
        match self.composed_text() {
            Ok(text) => text,
            Err(_) => self
                .context
                .state
                .get_utf8_raw(self.keycode, self.context.scratch_buffer),
        }
    }

    fn composed_text(&mut self) -> Result<Option<SmolStr>, ()> {
        match self.compose {
            ComposeStatus::Accepted(status) => match status {
                xkb_compose_status::XKB_COMPOSE_COMPOSED => {
                    let state = self.context.compose_state1.as_mut().unwrap();
                    Ok(state.get_string(self.context.scratch_buffer))
                }
                xkb_compose_status::XKB_COMPOSE_COMPOSING
                | xkb_compose_status::XKB_COMPOSE_CANCELLED => Ok(None),
                xkb_compose_status::XKB_COMPOSE_NOTHING => Err(()),
            },
            _ => Err(()),
        }
    }
}
