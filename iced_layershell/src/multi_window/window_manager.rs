use std::collections::BTreeMap;

use super::state::State;
use crate::multi_window::Application;
use iced_graphics::Compositor;
use iced_style::application::StyleSheet;
use layershellev::id::Id;

pub struct Window<A, C>
where
    A: Application,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: StyleSheet,
{
    pub id: Id,
    pub renderer: A::Renderer,
    pub surface: C::Surface,
    pub state: State<A>,
}

pub struct WindowManager<A: Application, C: Compositor>
where
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: StyleSheet,
{
    entries: BTreeMap<Id, Window<A, C>>,
}

impl<A, C> WindowManager<A, C>
where
    A: Application,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: StyleSheet,
{
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        window: &layershellev::WindowStateUnit<()>,
        application: &A,
        compositor: &mut C,
    ) -> &mut Window<A, C> {
        use std::sync::Arc;
        let id = window.id();
        let state = State::new(application, window);
        let physical_size = state.physical_size();
        let surface = compositor.create_surface(
            Arc::new(window.gen_wrapper()),
            physical_size.width,
            physical_size.height,
        );
        let renderer = compositor.create_renderer();
        let _ = self.entries.insert(
            id,
            Window {
                id,
                renderer,
                surface,
                state,
            },
        );
        self.entries
            .get_mut(&id)
            .expect("Get window that was just inserted")
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id, &mut Window<A, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Window<A, C>> {
        self.entries.get_mut(&id)
    }
}
