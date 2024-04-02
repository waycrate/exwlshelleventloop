use iced_core::mouse::Interaction;
use iced_runtime::command::Action;
use sessionlockev::id::Id as SessionId;
#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum SessionShellActions {
    Mouse(Interaction),
    RedrawAll,
    RedrawWindow(SessionId),
}

#[derive(Debug, Clone, Copy)]
pub struct UnLockAction;

impl<T> From<UnLockAction> for Action<T> {
    fn from(value: UnLockAction) -> Self {
        Action::Custom(Box::new(value))
    }
}

impl UnLockAction {
    pub fn to_action<T>(&self) -> Action<T> {
        (*self).into()
    }
}
