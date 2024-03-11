use iced_core::SmolStr;

use iced_core::keyboard::Key as IcedKey;
pub fn key_from_u32(value: u32) -> IcedKey {
    use iced_core::keyboard::key::Named;
    match value {
        0 => IcedKey::Unidentified,
        1 => IcedKey::Named(Named::Escape),
        code @ 2..=10 => IcedKey::Character(SmolStr::new((code - 1).to_string())),
        11 => IcedKey::Character(SmolStr::new("0")),
        12 => IcedKey::Character(SmolStr::new("-")),
        13 => IcedKey::Character(SmolStr::new("=")),
        14 => IcedKey::Named(Named::Backspace),
        15 => IcedKey::Named(Named::Tab),
        16 => IcedKey::Character(SmolStr::new("q")),
        17 => IcedKey::Character(SmolStr::new("w")),
        18 => IcedKey::Character(SmolStr::new("e")),
        19 => IcedKey::Character(SmolStr::new("r")),
        20 => IcedKey::Character(SmolStr::new("t")),
        _ => IcedKey::Unidentified,
    }
}

pub fn text_from_u32(value: u32) -> Option<SmolStr> {
    match value {
        0 => None,
        1 => None,
        code @ 2..=10 => Some(SmolStr::new((code - 1).to_string())),
        11 => Some(SmolStr::new("0")),
        12 => Some(SmolStr::new("-")),
        13 => Some(SmolStr::new("=")),
        16 => Some(SmolStr::new("q")),
        17 => Some(SmolStr::new("w")),
        18 => Some(SmolStr::new("e")),
        19 => Some(SmolStr::new("r")),
        20 => Some(SmolStr::new("t")),
        _ => None,
    }
}
