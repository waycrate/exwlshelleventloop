use iced_layershell::actions::{IsSingleton, MainWindowInfo};
use iced_layershell::{to_layer_message, WindowInfoMarker};

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
    #[derive(WindowInfoMarker)]
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
        #[main]
        Main,
    }
    assert!(SingleToneTest::SingleTon.is_singleton());
    assert!(!SingleToneTest::NotSingleTon.is_singleton());
    assert!(SingleToneTest::SingleTonTwo { field: false }.is_singleton());
    assert!(SingleToneTest::SingleTonThird(10).is_singleton());
    assert!(matches!(
        MainWindowInfo.try_into().unwrap(),
        SingleToneTest::Main
    ))
}
