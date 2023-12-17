mod mainkeyboard;
mod smallkeyboard;
//use std::f64::consts::PI;

use std::fs::File;

use cairo::Context;
use memmap2::MmapMut;
use smallkeyboard::{draw_number_keyboard, find_keycode_from_smallkeyboard};

use crate::{
    consts::{FONT_FAMILY, FONT_SIZE, KEYBOARD_TITLE},
    otherkeys,
    pangoui::mainkeyboard::draw_main_keyboard,
};

use mainkeyboard::find_keycode_from_mainkeyboard;

use super::KeyModifierType;

use crate::consts::{EXCULDE_ZONE_RIGHT, EXCULDE_ZONE_TOP};

#[derive(Debug, Default)]
pub struct PangoUi {
    width: i32,
    height: i32,
    context: Option<Context>,
}

fn contain_mode(key_type: KeyModifierType, mode: KeyModifierType) -> bool {
    key_type == key_type | mode
}

fn draw_title(context: &Context, pangolayout: &pango::Layout, width: i32) {
    pangolayout.set_text(KEYBOARD_TITLE);
    let (textwidth, _) = pangolayout.pixel_size();
    let start_pos = (width - textwidth) / 2;
    context.save().unwrap();
    context.move_to(start_pos as f64, 0.0);
    pangocairo::show_layout(context, pangolayout);
    context.restore().unwrap()
}

impl PangoUi {
    pub(crate) fn init_draw(&mut self, key_type: KeyModifierType, tmp: &mut File) {
        let cairo_fmt = cairo::Format::ARgb32;
        let stride = cairo_fmt.stride_for_width(self.width as u32).unwrap();
        tmp.set_len((stride * self.height) as u64).unwrap();
        let mmmap: MmapMut = unsafe { MmapMut::map_mut(&*tmp).unwrap() };

        let surface =
            cairo::ImageSurface::create_for_data(mmmap, cairo_fmt, self.width, self.height, stride)
                .unwrap();
        let cairoinfo = cairo::Context::new(&surface).unwrap();
        self.context = Some(cairoinfo);
        self.repaint(key_type);
    }

    pub(crate) fn repaint(&self, key_type: KeyModifierType) {
        let height = self.height;
        let width = self.width;

        let cr = self.context.as_ref().unwrap();
        cr.set_operator(cairo::Operator::Source);
        cr.set_source_rgba(0.8_f64, 0.8_f64, 0.8_f64, 0.3);
        cr.paint().unwrap();
        cr.set_source_rgb(1_f64, 1_f64, 1_f64);
        let font_size = FONT_SIZE;
        let pangolayout = pangocairo::create_layout(cr);
        let mut desc = pango::FontDescription::new();
        desc.set_family(FONT_FAMILY);
        desc.set_weight(pango::Weight::Bold);

        desc.set_size(font_size * pango::SCALE);
        pangolayout.set_font_description(Some(&desc));

        draw_number_keyboard(cr, &pangolayout, width, height, 27, key_type);
        draw_main_keyboard(cr, &pangolayout, height, 27, key_type);
        draw_title(cr, &pangolayout, width);
    }

    pub fn set_size(&mut self, (width, height): (i32, i32)) {
        self.width = width;
        self.height = height;
    }

    pub fn get_key(&self, (pos_x, pos_y): (f64, f64)) -> Option<u32> {
        let (pos_x, pos_y) = (pos_x as i32, pos_y as i32);
        let exclude_zone = EXCULDE_ZONE_TOP as i32;
        let step = (self.height - exclude_zone) / 3;
        let x_1 = self.width - 4 * step;
        let x_4 = self.width - step;
        let x_exclude = self.width - EXCULDE_ZONE_RIGHT as i32;
        if pos_y < EXCULDE_ZONE_TOP as i32 {
            if pos_x < x_exclude {
                return None;
            }
            let step_right = EXCULDE_ZONE_TOP as i32;
            let right_w = pos_x - x_exclude;
            if right_w / step_right == 0 {
                return Some(otherkeys::MIN_KEYBOARD);
            } else {
                return Some(otherkeys::CLOSE_KEYBOARD);
            }
        }
        if pos_x < x_1 {
            let step = (self.height - exclude_zone) / 4;
            return find_keycode_from_mainkeyboard((pos_x, pos_y), step);
        } else if pos_x > x_4 {
            match (pos_y - exclude_zone) / step {
                0 => return Some(12),
                1 => return Some(11),
                2 => return Some(13),
                _ => return None,
            }
        }
        Some(find_keycode_from_smallkeyboard((pos_x, pos_y), x_1, step))
    }
}
