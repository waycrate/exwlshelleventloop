use iced_layershell::to_layer_message;

#[test]
fn test_layer_message_macro() {
    #[to_layer_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    let e = TestEnum::SizeChange((10, 10));
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
    let (_id, message) = TestEnum::layershell_open(NewLayerShellSettings::default());
    assert!(matches!(message, TestEnum::NewLayerShell { .. }))
}
