use iced_core::window::Id;
use iced_core::{Element, Settings, theme, window};
use iced_futures::Subscription;
use iced_program::Program;
use iced_runtime::Task;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared slot between [`WithDirtyWindows`] and the runtime event loop.
///
/// `Some(ids)` accumulates the windows that changed since the runtime last
/// looked. `None` means "rebuild everything" and is sticky for the batch.
#[derive(Clone)]
pub(crate) struct DirtyWindows(Rc<RefCell<Option<Vec<Id>>>>);

impl Default for DirtyWindows {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(Some(Vec::new()))))
    }
}

impl DirtyWindows {
    fn record(&self, dirty: Option<Vec<Id>>) {
        let mut slot = self.0.borrow_mut();

        match (&mut *slot, dirty) {
            (Some(known), Some(ids)) => known.extend(ids),
            (known @ Some(_), None) => *known = None,
            // Already flagged as "all": keep it.
            (None, _) => {}
        }
    }

    /// Returns and resets the accumulated set. `None` means "rebuild all".
    pub(crate) fn take(&self) -> Option<Vec<Id>> {
        self.0.borrow_mut().replace(Vec::new())
    }
}

/// Wraps a [`Program`] and records which windows have changed right after each
/// `update`, using the post-update state.
pub(crate) struct WithDirtyWindows<P, F> {
    program: P,
    dirty_windows: F,
    shared: DirtyWindows,
}

impl<P, F> WithDirtyWindows<P, F>
where
    P: Program,
    F: Fn(&mut P::State) -> Option<Vec<Id>>,
{
    pub(crate) fn new(program: P, dirty_windows: F, shared: DirtyWindows) -> Self {
        Self {
            program,
            dirty_windows,
            shared,
        }
    }
}

impl<P, F> Program for WithDirtyWindows<P, F>
where
    P: Program,
    F: Fn(&mut P::State) -> Option<Vec<Id>>,
{
    type State = P::State;
    type Message = P::Message;
    type Theme = P::Theme;
    type Renderer = P::Renderer;
    type Executor = P::Executor;

    fn name() -> &'static str {
        P::name()
    }

    fn settings(&self) -> Settings {
        self.program.settings()
    }

    fn window(&self) -> Option<window::Settings> {
        self.program.window()
    }

    fn boot(&self) -> (Self::State, Task<Self::Message>) {
        self.program.boot()
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        let task = self.program.update(state, message);
        self.shared.record((self.dirty_windows)(state));
        task
    }

    fn view<'a>(
        &self,
        state: &'a Self::State,
        window: window::Id,
    ) -> Element<'a, Self::Message, Self::Theme, Self::Renderer> {
        self.program.view(state, window)
    }

    fn title(&self, state: &Self::State, window: window::Id) -> String {
        self.program.title(state, window)
    }

    fn subscription(&self, state: &Self::State) -> Subscription<Self::Message> {
        self.program.subscription(state)
    }

    fn theme(&self, state: &Self::State, window: window::Id) -> Option<Self::Theme> {
        self.program.theme(state, window)
    }

    fn style(&self, state: &Self::State, theme: &Self::Theme) -> theme::Style {
        self.program.style(state, theme)
    }

    fn scale_factor(&self, state: &Self::State, window: window::Id) -> f32 {
        self.program.scale_factor(state, window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Id {
        Id::unique()
    }

    #[test]
    fn records_union_of_dirty_sets() {
        let shared = DirtyWindows::default();
        let a = id();
        let b = id();

        shared.record(Some(vec![a]));
        shared.record(Some(vec![b]));

        let popped = shared.take().expect("partial rebuild");
        assert!(popped.contains(&a));
        assert!(popped.contains(&b));
        assert_eq!(popped.len(), 2);
    }

    #[test]
    fn take_resets_the_set() {
        let shared = DirtyWindows::default();
        shared.record(Some(vec![id()]));

        assert!(shared.take().unwrap().len() == 1);
        assert_eq!(shared.take(), Some(Vec::new()));
    }

    #[test]
    fn a_full_rebuild_flag_is_sticky() {
        let shared = DirtyWindows::default();

        shared.record(Some(vec![id()]));
        shared.record(None);
        shared.record(Some(vec![id()]));

        assert_eq!(shared.take(), None);
        assert_eq!(shared.take(), Some(Vec::new()));
    }
}
