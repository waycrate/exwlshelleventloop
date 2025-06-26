mod consts;
mod keyboardlayouts;
#[allow(unused)]
mod otherkeys;
mod pangoui;
use std::{ffi::CString, fs::File, io::Write, os::fd::AsFd, path::PathBuf};

use consts::EXCLUDE_ZONE_TOP;
use keyboardlayouts::Layouts;

use layershellev::reexport::wayland_client::KeyState;
use xkbcommon::xkb;

use layershellev::reexport::wayland_client::{ButtonState, WEnum, wl_keyboard::KeymapFormat};
use layershellev::reexport::*;
use layershellev::*;
use pangoui::PangoUi;

use bitflags::bitflags;

bitflags! {
    #[allow(unused)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct KeyModifierType : u32 {
        const NoMod = 0;
        const Shift = 1;
        const CapsLock = 2;
        const Ctrl = 4;
        const Alt = 8;
        const Super = 64;
        const AltGr = 128;
    }
}

impl From<u32> for KeyModifierType {
    fn from(value: u32) -> Self {
        match value {
            otherkeys::CAPS_LOCK => KeyModifierType::CapsLock,
            otherkeys::SHIFT_LEFT | otherkeys::SHIFT_RIGHT => KeyModifierType::Shift,
            otherkeys::MENU => KeyModifierType::Super,
            otherkeys::CTRL_LEFT | otherkeys::CTRL_RIGHT => KeyModifierType::Ctrl,
            otherkeys::ALT_LEFT | otherkeys::ALT_RIGHT => KeyModifierType::Alt,
            _ => KeyModifierType::NoMod,
        }
    }
}

impl From<usize> for KeyModifierType {
    fn from(value: usize) -> Self {
        let value = value as u32;
        value.into()
    }
}

pub fn get_keymap_as_file() -> (File, u32) {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);

    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        Layouts::EnglishUs.to_layout_name(), // if no , it is norwegian
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )
    .expect("xkbcommon keymap panicked!");
    let xkb_state = xkb::State::new(&keymap);
    let keymap = xkb_state
        .get_keymap()
        .get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
    let keymap = CString::new(keymap).expect("Keymap should not contain interior nul bytes");
    let keymap = keymap.as_bytes_with_nul();
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut file = tempfile::tempfile_in(dir).expect("File could not be created!");
    file.write_all(keymap).unwrap();
    file.flush().unwrap();
    (file, keymap.len() as u32)
}

fn main() {
    let ev: WindowState<PangoUi> = WindowState::new("precure")
        .with_allscreens()
        .with_size((0, 300))
        .with_layer(Layer::Top)
        .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
        .with_keyboard_interacivity(KeyboardInteractivity::None)
        .with_exclusive_zone(300)
        .build()
        .unwrap();

    let mut current_keytype = KeyModifierType::NoMod;
    let mut virtuan_keyboard = None;
    let mut button_pos: (f64, f64) = (0., 0.);
    let mut is_min = false;

    let mut touch_id = -1;
    let mut touch_key = 0;

    ev.running(move |event, ev, index| match event {
        LayerShellEvent::InitRequest => ReturnData::RequestBind,
        LayerShellEvent::BindProvide(globals, qh) => {
            let virtual_keyboard_manager = globals
                .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(qh, 1..=1, ())
                .unwrap();
            let seat = ev.get_seat();
            let virtual_keyboard_in =
                virtual_keyboard_manager.create_virtual_keyboard(seat, qh, ());
            let (file, size) = get_keymap_as_file();
            virtual_keyboard_in.keymap(KeymapFormat::XkbV1.into(), file.as_fd(), size);
            virtuan_keyboard = Some(virtual_keyboard_in);
            ReturnData::None
        }
        LayerShellEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
            let index = index.unwrap();
            let mut pangoui = pangoui::PangoUi::default();
            pangoui.set_size((init_w as i32, init_h as i32));
            pangoui.init_draw(current_keytype, file);
            *ev.get_binding_mut(index).unwrap() = pangoui;
            let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
            ReturnData::WlBuffer(pool.create_buffer(
                0,
                init_w as i32,
                init_h as i32,
                (init_w * 4) as i32,
                wl_shm::Format::Argb8888,
                qh,
                (),
            ))
        }
        LayerShellEvent::RequestMessages(DispatchMessage::RequestRefresh {
            width, height, ..
        }) => {
            let index = index.unwrap();
            let pangoui = ev.get_binding_mut(index).unwrap();
            pangoui.set_size((*width as i32, *height as i32));
            pangoui.repaint(current_keytype);
            ev.request_next_present(index);
            let windowunit = ev.get_unit_with_id(index).unwrap();
            windowunit.refresh();
            ReturnData::None
        }
        LayerShellEvent::RequestMessages(DispatchMessage::MouseButton { state, .. }) => {
            let index = index.unwrap();
            let key = ev.get_binding_mut(index).unwrap().get_key(button_pos);
            let windowunit = ev.get_unit_with_id(index).unwrap();
            match key {
                Some(otherkeys::CLOSE_KEYBOARD) => {
                    if let WEnum::Value(ButtonState::Pressed) = *state {
                        return ReturnData::None;
                    }
                    ReturnData::RequestExit
                }
                Some(otherkeys::MIN_KEYBOARD) => {
                    if let WEnum::Value(ButtonState::Pressed) = *state {
                        return ReturnData::None;
                    }
                    if is_min {
                        windowunit.set_size((0, EXCLUDE_ZONE_TOP as u32));
                        windowunit.set_exclusive_zone(EXCLUDE_ZONE_TOP as i32);
                    } else {
                        windowunit.set_size((0, 300));
                        windowunit.set_exclusive_zone(300);
                    }
                    is_min = !is_min;
                    ReturnData::None
                }
                Some(key) => {
                    let keystate = match state {
                        WEnum::Value(ButtonState::Pressed) => KeyState::Pressed,
                        WEnum::Value(ButtonState::Released) => KeyState::Released,
                        _ => unreachable!(),
                    };

                    let virtuan_keyboard = virtuan_keyboard.as_ref().unwrap();
                    virtuan_keyboard.key(100, key, keystate.into());
                    let keymod: KeyModifierType = key.into();
                    if keymod != KeyModifierType::NoMod && keystate == KeyState::Pressed {
                        return ReturnData::None;
                    }
                    let keytype_now = current_keytype ^ keymod;
                    if keytype_now != current_keytype {
                        current_keytype = keytype_now;
                        virtuan_keyboard.modifiers(current_keytype.bits(), 0, 0, 0);
                        let ids: Vec<_> = ev.get_unit_iter().map(WindowStateUnit::id).collect();
                        for id in ids {
                            ev.get_binding_mut(id).unwrap().repaint(current_keytype);
                        }
                        ev.request_refresh_all(RefreshRequest::NextFrame);
                    }
                    ReturnData::None
                }
                None => ReturnData::None,
            }
        }
        LayerShellEvent::RequestMessages(DispatchMessage::TouchDown { x, y, id, .. }) => {
            if *id != touch_id || touch_id == -1 {
                touch_id = *id;
            }
            let index = index.unwrap();
            let pangoui = ev.get_binding_mut(index).unwrap();
            let Some(touch_getkey) = pangoui.get_key((*x, *y)) else {
                return ReturnData::None;
            };
            let windowunit = ev.get_unit_with_id(index).unwrap();
            touch_key = touch_getkey;
            match touch_getkey {
                otherkeys::CLOSE_KEYBOARD => ReturnData::RequestExit,
                otherkeys::MIN_KEYBOARD => {
                    if is_min {
                        windowunit.set_size((0, EXCLUDE_ZONE_TOP as u32));
                        windowunit.set_exclusive_zone(EXCLUDE_ZONE_TOP as i32);
                    } else {
                        windowunit.set_size((0, 300));
                        windowunit.set_exclusive_zone(300);
                    }
                    is_min = !is_min;
                    ReturnData::None
                }
                key => {
                    let keystate = KeyState::Pressed;

                    let virtuan_keyboard = virtuan_keyboard.as_ref().unwrap();
                    virtuan_keyboard.key(100, key, keystate.into());
                    let keymod: KeyModifierType = key.into();
                    if keymod != KeyModifierType::NoMod && keystate == KeyState::Pressed {
                        return ReturnData::None;
                    }
                    let keytype_now = current_keytype ^ keymod;
                    if keytype_now != current_keytype {
                        current_keytype = keytype_now;
                        virtuan_keyboard.modifiers(current_keytype.bits(), 0, 0, 0);
                        let ids: Vec<_> = ev.get_unit_iter().map(WindowStateUnit::id).collect();
                        for id in ids {
                            ev.get_binding_mut(id).unwrap().repaint(current_keytype);
                        }
                        ev.request_refresh_all(RefreshRequest::NextFrame);
                    }
                    ReturnData::None
                }
            }
        }
        LayerShellEvent::RequestMessages(DispatchMessage::TouchUp { id, .. }) => {
            if *id != touch_id {
                return ReturnData::None;
            }
            let virtuan_keyboard = virtuan_keyboard.as_ref().unwrap();
            virtuan_keyboard.key(100, touch_key, KeyState::Released.into());
            ReturnData::None
        }
        LayerShellEvent::RequestMessages(DispatchMessage::MouseEnter {
            surface_x,
            surface_y,
            ..
        }) => {
            button_pos = (*surface_x, *surface_y);
            ReturnData::None
        }
        LayerShellEvent::RequestMessages(DispatchMessage::MouseMotion {
            surface_x,
            surface_y,
            ..
        }) => {
            button_pos = (*surface_x, *surface_y);
            ReturnData::None
        }
        _ => ReturnData::None,
    })
    .unwrap();
}
