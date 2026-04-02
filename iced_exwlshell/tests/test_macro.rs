use iced_exwlshell::to_exwlshell_message;

#[test]
fn test_layer_message_macro() {
    #[to_exwlshell_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    let e = TestEnum::SizeChange {
        id: iced::window::Id::unique(),
        size: ((1, 2)),
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
