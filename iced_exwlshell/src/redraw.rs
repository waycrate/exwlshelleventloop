//! Per-message redraw targeting.
//!
//! [`Scope`] is evaluated before the application update and only decides which
//! windows are rendered. To also skip rebuilding (`view` + layout) the windows
//! an update did not touch, install a callback with `Daemon::dirty_windows`:
//! it runs after the update with the new state and takes precedence over this
//! policy.

use iced_core::window::Id;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    None,
    Window(Id),
    All,
}

pub(crate) enum Targets {
    None,
    Window(Id),
    All,
    Windows(Vec<Id>),
}

type RedrawScopeFn<Message> = dyn Fn(&Message) -> Scope;

pub(crate) struct Policy<Message> {
    scope: Option<Box<RedrawScopeFn<Message>>>,
}

impl<Message> Policy<Message> {
    pub(crate) fn new(scope: impl Fn(&Message) -> Scope + 'static) -> Self {
        Self {
            scope: Some(Box::new(scope)),
        }
    }

    #[cfg(any(all(feature = "debug", not(target_arch = "wasm32")), test))]
    pub(crate) fn for_wrapped_messages<Wrapped>(
        self,
        inner_message: impl for<'a> Fn(&'a Wrapped) -> Option<&'a Message> + 'static,
    ) -> Policy<Wrapped>
    where
        Message: 'static,
    {
        let Some(scope) = self.scope else {
            return Policy::default();
        };

        Policy::new(move |message| inner_message(message).map_or(Scope::All, &scope))
    }

    pub(crate) fn targets(&self, messages: &[Message]) -> Targets {
        let Some(scope) = self.scope.as_ref() else {
            return Targets::All;
        };

        let mut first_window = None;
        let mut messages = messages.iter();
        let mut windows = None;

        for message in messages.by_ref() {
            match scope(message) {
                Scope::All => return Targets::All,
                Scope::None => {}
                Scope::Window(id) => match first_window {
                    None => first_window = Some(id),
                    Some(current) if current == id => {}
                    Some(current) => {
                        windows = Some(vec![current, id]);
                        break;
                    }
                },
            }
        }

        if let Some(mut windows) = windows {
            for message in messages {
                match scope(message) {
                    Scope::Window(id) if !windows.contains(&id) => windows.push(id),
                    Scope::All => return Targets::All,
                    Scope::Window(_) | Scope::None => {}
                }
            }

            return Targets::Windows(windows);
        }

        first_window.map_or(Targets::None, Targets::Window)
    }
}

impl<Message> Default for Policy<Message> {
    fn default() -> Self {
        Self { scope: None }
    }
}

impl<Message> Debug for Policy<Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Policy").finish_non_exhaustive()
    }
}
