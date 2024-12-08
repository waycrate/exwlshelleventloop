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
