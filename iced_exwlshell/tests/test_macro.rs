use iced_exwlshell::reexport::{Anchor, LayerSize};
use iced_exwlshell::to_exwlshell_message;

#[test]
fn test_layer_message_macro() {
    #[to_exwlshell_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    let e = TestEnum::LayoutChange {
        id: iced::window::Id::unique(),
        anchor: Anchor::Bottom,
        size: LayerSize::fill_width(30),
    };
    let _ = e.clone();
}

#[test]
fn test_layer_message_macro_multi() {
    #[to_exwlshell_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    use exwlshellev::*;
    let (_id, _message) = TestEnum::layershell_open(NewLayerShellSettings::default());
}
