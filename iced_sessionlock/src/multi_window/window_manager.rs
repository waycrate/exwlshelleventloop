use std::{collections::BTreeMap, sync::Arc};

use super::state::State;
use crate::DefaultStyle;
use iced::mouse;
use iced::window::Id as IcedId;
use iced_graphics::Compositor;
use iced_program::Instance;
use iced_program::Program;
use sessionlockev::{WindowWrapper, id::Id as SessionId};

pub struct Window<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub id: SessionId,
    pub renderer: P::Renderer,
    pub surface: C::Surface,
    pub state: State<P>,
    pub mouse_interaction: mouse::Interaction,
}

pub struct WindowManager<P: Program, C: Compositor>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    aliases: BTreeMap<SessionId, IcedId>,
    back_aliases: BTreeMap<IcedId, SessionId>,
    entries: BTreeMap<IcedId, Window<P, C>>,
}

impl<P, C> Default for WindowManager<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<P, C> WindowManager<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub fn new() -> Self {
        Self {
            aliases: BTreeMap::new(),
            back_aliases: BTreeMap::new(),
            entries: BTreeMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &mut self,
        id: IcedId,
        size: (u32, u32),
        fractal_scale: f64,
        window: Arc<WindowWrapper>,
        application: &Instance<P>,
        compositor: &mut C,
        system_theme: iced::theme::Mode,
    ) -> &mut Window<P, C> {
        let layerid = window.id();
        let state = State::new(id, application, size, fractal_scale, &window, system_theme);
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
    pub fn first(&self) -> Option<&Window<P, C>> {
        self.entries.first_key_value().map(|(_, v)| v)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IcedId, &mut Window<P, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn get_mut_alias(&mut self, id: SessionId) -> Option<(IcedId, &mut Window<P, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get_mut(id)?))
    }

    pub fn get_iced_id(&self, id: IcedId) -> Option<SessionId> {
        self.back_aliases.get(&id).copied()
    }

    pub fn get_mut(&mut self, id: IcedId) -> Option<&mut Window<P, C>> {
        self.entries.get_mut(&id)
    }

    pub fn get(&mut self, id: IcedId) -> Option<&Window<P, C>> {
        self.entries.get(&id)
    }
}
