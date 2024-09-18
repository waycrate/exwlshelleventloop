use iced_core::mouse::Interaction;
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
