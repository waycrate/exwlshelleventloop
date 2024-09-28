use iced_sessionlock_macros::to_session_message;

#[test]
fn test_macro() {
    #[allow(dead_code)]
    #[to_session_message]
    #[derive(Debug, Clone)]
    enum TestEnum {
        TestA,
    }
    let e = TestEnum::UnLock;
    let _ = e.clone();
}
