#![cfg(feature = "macros")]

use iced_layershell::reexport::{Anchor, LayerSize};
use iced_layershell::to_layer_message;

#[test]
fn test_layer_message_macro() {
    #[to_layer_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    let e = TestEnum::LayoutChange {
        anchor: Anchor::Bottom,
        size: LayerSize::fill_width(30),
    };
    let _ = e.clone();
}

#[test]
fn test_layer_message_macro_multi() {
    #[to_layer_message(multi)]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    use layershellev::*;
    let (_id, _message) = TestEnum::layershell_open(NewLayerShellSettings::default());
}
