use cairo::Context;

use crate::consts::{EXCLUDE_ZONE_RIGHT, EXCLUDE_ZONE_TOP};

use super::contain_mode;
use crate::KeyModifierType;

fn contain_shift(key_type: KeyModifierType) -> bool {
    contain_mode(key_type, KeyModifierType::Shift)
}

pub fn find_keycode_from_smallkeyboard((pos_x, pos_y): (i32, i32), start_x: i32, step: i32) -> u32 {
    let exclude_zone = EXCLUDE_ZONE_TOP as i32;
    let abx = (pos_x - start_x) / step;
    let aby = (pos_y - exclude_zone) / step;
    let code = aby * 3 + abx + 2;
    code as u32
}

fn draw_extra_btn(content: &Context, pangolayout: &pango::Layout, width: i32, font_size: i32) {
    let step = EXCLUDE_ZONE_RIGHT / 2.0;
    let x_1 = width as f64 - EXCLUDE_ZONE_RIGHT;
    let x_2 = width as f64 - step;
    let x_3 = width as f64;
    let y_1 = 0.0;
    let y_2 = step;
    content.set_source_rgb(0.0, 0.0, 0.0);
    content.move_to(x_1, y_1);
    content.line_to(x_1, y_2);
    content.move_to(x_2, y_1);
    content.line_to(x_2, y_2);
    content.move_to(x_3, y_1);
    content.line_to(x_3, y_2);

    content.move_to(x_1, y_1);
    content.line_to(x_3, y_1);
    content.move_to(x_1, y_2);
    content.line_to(x_3, y_2);

    content.stroke().unwrap();

    let font_adjustx = step / 2.0 - font_size as f64 / 2.0;
    pangolayout.set_text("-");
    content.save().unwrap();
    content.move_to(x_1 + font_adjustx, y_1);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    pangolayout.set_text("x");
    content.save().unwrap();
    content.move_to(x_2 + font_adjustx, y_1);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();
}

pub(super) fn draw_number_keyboard(
    content: &Context,
    pangolayout: &pango::Layout,
    width: i32,
    height: i32,
    font_size: i32,
    key_type: KeyModifierType,
) {
    // NOTE: here require width > height
    assert!(width - EXCLUDE_ZONE_RIGHT as i32 > height);

    let exclude_zone = EXCLUDE_ZONE_RIGHT / 2.0;

    let step = (height as f64 - exclude_zone) / 3.0;
    let x_1 = width as f64 - 4.0 * step;
    let x_2 = width as f64 - 3.0 * step;
    let x_3 = width as f64 - 2.0 * step;
    let x_4 = width as f64 - step;
    let x_5 = width as f64;

    let y_1 = 0.0 + exclude_zone;
    let y_2 = step + exclude_zone;
    let y_3 = 2.0 * step + exclude_zone;
    let y_4 = height as f64;

    let font_adjusty = step / 2.0 - font_size as f64;
    let font_adjustx = step / 2.0 - font_size as f64 / 2.0;
    content.set_source_rgb(0.0, 0.0, 0.0);
    content.move_to(x_1, y_1);
    content.line_to(x_1, y_4);
    content.move_to(x_2, y_1);
    content.line_to(x_2, y_4);
    content.move_to(x_3, y_1);
    content.line_to(x_3, y_4);
    content.move_to(x_4, y_1);
    content.line_to(x_4, y_4);
    content.move_to(x_5, y_1);
    content.line_to(x_5, y_4);

    content.move_to(x_1, y_1);
    content.line_to(x_5, y_1);
    content.move_to(x_1, y_2);
    content.line_to(x_5, y_2);
    content.move_to(x_1, y_3);
    content.line_to(x_5, y_3);
    content.move_to(x_1, y_4);
    content.line_to(x_5, y_4);

    content.stroke().unwrap();

    let shiftmode = contain_shift(key_type);

    if shiftmode {
        pangolayout.set_text("!");
    } else {
        pangolayout.set_text("1");
    }
    content.save().unwrap();
    content.move_to(x_1 + font_adjustx, y_1 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("@");
    } else {
        pangolayout.set_text("2");
    }
    content.save().unwrap();
    content.move_to(x_2 + font_adjustx, y_1 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("#");
    } else {
        pangolayout.set_text("3");
    }
    content.save().unwrap();
    content.move_to(x_3 + font_adjustx, y_1 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("$");
    } else {
        pangolayout.set_text("4");
    }
    content.save().unwrap();
    content.move_to(x_1 + font_adjustx, y_2 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("%");
    } else {
        pangolayout.set_text("5");
    }
    content.save().unwrap();
    content.move_to(x_2 + font_adjustx, y_2 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("^");
    } else {
        pangolayout.set_text("6");
    }
    content.save().unwrap();
    content.move_to(x_3 + font_adjustx, y_2 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("&");
    } else {
        pangolayout.set_text("7");
    }
    content.save().unwrap();
    content.move_to(x_1 + font_adjustx, y_3 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("*");
    } else {
        pangolayout.set_text("8");
    }
    content.save().unwrap();
    content.move_to(x_2 + font_adjustx, y_3 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("(");
    } else {
        pangolayout.set_text("9");
    }
    content.save().unwrap();
    content.move_to(x_3 + font_adjustx, y_3 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("_");
    } else {
        pangolayout.set_text("-");
    }
    content.save().unwrap();
    content.move_to(x_4 + font_adjustx, y_1 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text(")");
    } else {
        pangolayout.set_text("0");
    }
    content.save().unwrap();
    content.move_to(x_4 + font_adjustx, y_2 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    if shiftmode {
        pangolayout.set_text("+");
    } else {
        pangolayout.set_text("=");
    }
    content.save().unwrap();
    content.move_to(x_4 + font_adjustx, y_3 + font_adjusty);
    pangocairo::show_layout(content, pangolayout);
    content.restore().unwrap();

    draw_extra_btn(content, pangolayout, width, font_size);
}
