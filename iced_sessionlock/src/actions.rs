use iced_core::mouse::Interaction;
use sessionlockev::id::Id as SessionId;
#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum SessionShellAction {
    Mouse(Interaction),
    RedrawAll,
    RedrawWindow(SessionId),
}

#[derive(Debug, Clone, Copy)]
pub struct UnLockAction;
