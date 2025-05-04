use std::{collections::BTreeMap, sync::Arc};

use super::state::State;
use crate::DefaultStyle;
use crate::build_pattern::Instance;
use crate::build_pattern::Program;
use iced::mouse;
use iced::window::Id as IcedId;
use iced_graphics::Compositor;
use sessionlockev::{WindowWrapper, id::Id as SessionId};

pub struct Window<A, C>
where
    A: Program,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    pub id: SessionId,
    pub renderer: A::Renderer,
    pub surface: C::Surface,
    pub state: State<A>,
    pub mouse_interaction: mouse::Interaction,
}

pub struct WindowManager<A: Program, C: Compositor>
where
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    aliases: BTreeMap<SessionId, IcedId>,
    back_aliases: BTreeMap<IcedId, SessionId>,
    entries: BTreeMap<IcedId, Window<A, C>>,
}

impl<A, C> Default for WindowManager<A, C>
where
    A: Program,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<A, C> WindowManager<A, C>
where
    A: Program,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
            back_aliases: BTreeMap::new(),
            entries: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        id: IcedId,
        size: (u32, u32),
        fractal_scale: f64,
        window: Arc<WindowWrapper>,
        application: &Instance<A>,
        compositor: &mut C,
    ) -> &mut Window<A, C> {
        let layerid = window.id();
        let state = State::new(id, application, size, fractal_scale, &window);
        let physical_size = state.viewport().physical_size();
        let surface = compositor.create_surface(window, physical_size.width, physical_size.height);
        let renderer = compositor.create_renderer();
        let _ = self.aliases.insert(layerid, id);
        let _ = self.back_aliases.insert(id, layerid);

        let _ = self.entries.insert(
            id,
            Window {
                id: layerid,
                renderer,
                surface,
                state,
                mouse_interaction: mouse::Interaction::Idle,
            },
        );
        self.entries
            .get_mut(&id)
            .expect("Get window that was just inserted")
    }
    pub fn first(&self) -> Option<&Window<A, C>> {
        self.entries.first_key_value().map(|(_, v)| v)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IcedId, &mut Window<A, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn get_mut_alias(&mut self, id: SessionId) -> Option<(IcedId, &mut Window<A, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get_mut(id)?))
    }

    pub fn get_iced_id(&self, id: IcedId) -> Option<SessionId> {
        self.back_aliases.get(&id).copied()
    }

    pub fn get_mut(&mut self, id: IcedId) -> Option<&mut Window<A, C>> {
        self.entries.get_mut(&id)
    }

    pub fn get(&mut self, id: IcedId) -> Option<&Window<A, C>> {
        self.entries.get(&id)
    }
}
