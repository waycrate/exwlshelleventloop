const ENGLISH_LAYOUT: &str = include_str!("../asserts/layoutassert/us.json");

use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyBoardLayout {
    pub name: String,
    pub layoutname: String,
    pub keys: Vec<LayoutKey>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LayoutKey {
    pub mainkey: String,
    pub y: usize,
    pub index: usize,
    fillend: Option<bool>,
    modkey: Option<bool>,
    width: Option<usize>,
    pub menu: Option<String>,
    pub extra: Option<String>,
    pub caps: Option<String>,
}

#[allow(unused)]
impl LayoutKey {
    pub fn is_fillend(&self) -> bool {
        self.fillend.unwrap_or(false)
    }

    pub fn is_modkey(&self) -> bool {
        self.modkey.unwrap_or(false)
    }

    pub fn width(&self) -> usize {
        self.width.unwrap_or(2)
    }
}

#[allow(unused)]
#[derive(Default, Debug)]
pub enum Layouts {
    #[default]
    EnglishUs,
    //Norwegian,
}

#[allow(unused)]
impl Layouts {
    pub fn to_layout_name(&self) -> &str {
        match self {
            //Self::Norwegian => "no",
            Self::EnglishUs => "us",
        }
    }

    pub fn get_layout(&self) -> Result<KeyBoardLayout> {
        match self {
            Layouts::EnglishUs => Ok(serde_json::from_str(ENGLISH_LAYOUT)?),
        }
    }
}

#[test]
fn tst_layout_read() {
    let us_keyboard: KeyBoardLayout = serde_json::from_str(ENGLISH_LAYOUT).unwrap();
    assert_eq!(us_keyboard.name, "usbase".to_string());
    assert_eq!(us_keyboard.layoutname, "us".to_string());
    assert_eq!(us_keyboard.keys[0].mainkey, "`".to_string());
}
