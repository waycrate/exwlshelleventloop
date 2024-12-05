use iced_layershell::actions::IsSingleton;
use iced_layershell::{to_layer_message, windowinfo_marker};

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
fn test_layersingleton_derive() {
    #[allow(unused)]
    #[derive(windowinfo_marker)]
    enum SingleToneTest {
        #[singleton]
        SingleTon,
        NotSingleTon,
        #[singleton]
        SingleTonTwo {
            field: bool,
        },
        #[singleton]
        SingleTonThird(i32),
    }
    assert!(SingleToneTest::SingleTon.is_singleton());
    assert!(!SingleToneTest::NotSingleTon.is_singleton());
    assert!(SingleToneTest::SingleTonTwo { field: false }.is_singleton());
    assert!(SingleToneTest::SingleTonThird(10).is_singleton());
}
