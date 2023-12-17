use cairo::Context;

use serde::{Deserialize, Serialize};

use std::sync::OnceLock;

use crate::KeyModifierType;

use crate::consts::EXCULDE_ZONE_TOP;

use super::contain_mode;

static MAIN_LAYOUT_INFO: OnceLock<Vec<Vec<MainLayout>>> = OnceLock::new();

const MAIN_LAYOUT: &str = include_str!("../../asserts/mainkeylayout/enUS.json");

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MainLayout {
    text: String,
    cap: Option<String>,
    shift: Option<String>,
    width: usize,
    line: usize,
    start_pos: usize,
    key: usize,
}

// TODO: cap and shift
#[derive(Clone, Copy)]
enum KeyType {
    Normal,
    Cap,
    Shift,
}

fn contain_mode_special(keymode: KeyModifierType, key_type: KeyModifierType) -> bool {
    if key_type == KeyModifierType::NoMod {
        return false;
    }
    contain_mode(keymode, key_type)
}

impl MainLayout {
    fn get_info(&self, keymode: KeyModifierType, step: f64, font_size: i32) -> DrawInfo<'_> {
        let layout_keytype: KeyModifierType = self.key.into();
        let locked = contain_mode_special(keymode, layout_keytype);
        let keytype = keymode.into();
        match keytype {
            KeyType::Normal => DrawInfo {
                step,
                width: self.width as i32,
                font_size,
                line: self.line as i32,
                text: self.text.as_str(),
                start_pos: self.start_pos as i32,
                locked,
            },
            KeyType::Cap => DrawInfo {
                step,
                width: self.width as i32,
                font_size,
                line: self.line as i32,
                text: match &self.cap {
                    Some(text) => text,
                    None => self.text.as_str(),
                },
                start_pos: self.start_pos as i32,
                locked,
            },
            KeyType::Shift => DrawInfo {
                step,
                width: self.width as i32,
                font_size,
                line: self.line as i32,
                text: match &self.shift {
                    Some(text) => text,
                    None => match &self.cap {
                        Some(text) => text,
                        None => self.text.as_str(),
                    },
                },
                start_pos: self.start_pos as i32,
                locked,
            },
        }
    }
}

impl From<KeyModifierType> for KeyType {
    fn from(value: KeyModifierType) -> Self {
        if contain_mode(value, KeyModifierType::Shift) {
            KeyType::Shift
        } else if contain_mode(value, KeyModifierType::CapsLock) {
            KeyType::Cap
        } else {
            KeyType::Normal
        }
    }
}

fn get_main_layout() -> Vec<Vec<MainLayout>> {
    if let Some(layout_info) = MAIN_LAYOUT_INFO.get() {
        layout_info.clone()
    } else {
        let layout: Vec<Vec<MainLayout>> = serde_json::from_str(MAIN_LAYOUT).unwrap();
        MAIN_LAYOUT_INFO.set(layout.clone()).expect("Cannot set it");
        layout
    }
}

struct DrawInfo<'a> {
    step: f64,
    width: i32,
    font_size: i32,
    line: i32,
    text: &'a str,
    start_pos: i32,
    locked: bool,
}

fn draw_unit_key(
    pangolayout: &pango::Layout,
    content: &Context,
    DrawInfo {
        step,
        width,
        font_size,
        line,
        text,
        start_pos,
        locked,
    }: DrawInfo,
) {
    let exclude_zone = EXCULDE_ZONE_TOP;
    let start_x = step * start_pos as f64 / 2.0;
    let end_x = step * width as f64 / 2.0 + start_x;
    let start_y = step * line as f64 + exclude_zone;
    let end_y = step * (line + 1) as f64 + exclude_zone;
    if locked {
        content.rectangle(start_x, start_y, end_x - start_x, end_y - start_y);
        content.set_source_rgba(0.5, 0.5, 0.5, 0.8);
        content.fill().unwrap();
        content.set_source_rgb(0_f64, 0_f64, 0_f64);
    }
    content.move_to(start_x, start_y);
    content.line_to(start_x, end_y);
    content.move_to(end_x, start_y);
    content.line_to(end_x, end_y);

    content.move_to(start_x, start_y);
    content.line_to(end_x, start_y);
    content.move_to(start_x, end_y);
    content.line_to(end_x, end_y);

    content.stroke().unwrap();

    pangolayout.set_text(text);
    let font_adjusty = step / 2.0 - font_size as f64;
    content.save().unwrap();
    content.move_to(start_x + font_adjusty, start_y);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();
}

pub(crate) fn draw_main_keyboard(
    content: &Context,
    pangolayout: &pango::Layout,
    height: i32,
    font_size: i32,
    key_type: KeyModifierType,
) {
    let exclude_zone = EXCULDE_ZONE_TOP;
    let step = (height - exclude_zone as i32) / 4;

    for oneline in get_main_layout().iter() {
        for map in oneline.iter() {
            draw_unit_key(
                pangolayout,
                content,
                map.get_info(key_type, step as f64, font_size),
            );
        }
    }
}

pub fn find_keycode_from_mainkeyboard((pos_x, pos_y): (i32, i32), step: i32) -> Option<u32> {
    let exclude_zone = EXCULDE_ZONE_TOP;
    let main_layout = get_main_layout();
    let aby = (pos_y - exclude_zone as i32) / step;
    if aby >= main_layout.len() as i32 {
        return None;
    }

    for MainLayout {
        width,
        start_pos,
        key,
        ..
    } in main_layout[aby as usize].iter()
    {
        if pos_x > *start_pos as i32 * step / 2
            && pos_x < (*start_pos as i32 + *width as i32) * step / 2
        {
            return Some(*key as u32);
        }
    }
    None
}
