use std::{collections::BTreeMap, sync::Arc};

use super::state::State;
use crate::DefaultStyle;
use crate::ime_preedit::{ImeState, Preedit};
use crate::multi_window::Application;
use enumflags2::{BitFlag, BitFlags};
use iced_core::InputMethod;
use iced_core::input_method;
use iced_graphics::Compositor;
use layershellev::{WindowWrapper, id::Id as LayerId};

use iced::mouse;
use iced::window::Id as IcedId;

pub struct Window<A, C>
where
    A: Application,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    pub id: LayerId,
    pub renderer: A::Renderer,
    pub surface: C::Surface,
    pub state: State<A>,
    pub mouse_interaction: mouse::Interaction,
    preedit: Option<Preedit<A::Renderer>>,
    ime_state: Option<(iced_core::Point, input_method::Purpose)>,
}

pub struct WindowManager<A: Application, C: Compositor>
where
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    aliases: BTreeMap<LayerId, IcedId>,
    back_aliases: BTreeMap<IcedId, LayerId>,
    entries: BTreeMap<IcedId, Window<A, C>>,
}

impl<A, C> Default for WindowManager<A, C>
where
    A: Application,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<A, C> WindowManager<A, C>
where
    A: Application,
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

    pub fn remove(&mut self, id: IcedId) {
        let remove_alias = self
            .aliases
            .iter()
            .find(|(_, oriid)| **oriid == id)
            .map(|(layid, _)| *layid);
        if let Some(oriid) = remove_alias {
            self.aliases.remove(&oriid);
        }
        self.back_aliases.remove(&id);
        self.entries.remove(&id);
    }

    pub fn insert(
        &mut self,
        id: IcedId,
        size: (u32, u32),
        fractal_scale: f64,
        window: Arc<WindowWrapper>,
        application: &A,
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
                preedit: None,
                ime_state: None,
            },
        );
        self.entries
            .get_mut(&id)
            .expect("Get window that was just inserted")
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IcedId, &mut Window<A, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn first_window(&self) -> Option<(&IcedId, &Window<A, C>)> {
        self.entries.iter().next()
    }

    pub fn get_mut_alias(&mut self, id: LayerId) -> Option<(IcedId, &mut Window<A, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get_mut(id)?))
    }
    pub fn get_alias(&self, id: LayerId) -> Option<(IcedId, &Window<A, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get(id)?))
    }
    pub fn get_layer_id(&self, id: IcedId) -> Option<LayerId> {
        self.back_aliases.get(&id).copied()
    }

    pub fn get_mut(&mut self, id: IcedId) -> Option<&mut Window<A, C>> {
        self.entries.get_mut(&id)
    }
    pub fn get(&self, id: IcedId) -> Option<&Window<A, C>> {
        self.entries.get(&id)
    }
}

impl<A, C> Window<A, C>
where
    A: Application,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    pub fn request_input_method(&mut self, input_method: InputMethod) -> BitFlags<ImeState> {
        match input_method {
            InputMethod::Disabled => self.disable_ime(),
            InputMethod::Enabled {
                position,
                purpose,
                preedit,
            } => {
                let mut flags = ImeState::empty();
                if self.ime_state.is_none() {
                    flags.insert(ImeState::Allowed);
                }
                if self.ime_state != Some((position, purpose)) {
                    flags.insert(ImeState::Update);
                }
                self.update_ime(position, purpose);

                if let Some(preedit) = preedit {
                    if preedit.content.is_empty() {
                        self.preedit = None;
                    } else {
                        let mut overlay = self.preedit.take().unwrap_or_else(Preedit::new);

                        overlay.update(
                            position,
                            &preedit,
                            self.state.background_color(),
                            &self.renderer,
                        );

                        self.preedit = Some(overlay);
                    }
                } else {
                    self.preedit = None;
                }

                flags
            }
        }
    }

    pub fn draw_preedit(&mut self) {
        use iced_core::Point;
        use iced_core::Rectangle;
        if let Some(preedit) = &self.preedit {
            preedit.draw(
                &mut self.renderer,
                self.state.text_color(),
                self.state.background_color(),
                &Rectangle::new(Point::ORIGIN, self.state.viewport().logical_size()),
            );
        }
    }

    fn update_ime(&mut self, position: iced_core::Point, purpose: input_method::Purpose) {
        if self.ime_state != Some((position, purpose)) {
            self.ime_state = Some((position, purpose));
        }
    }

    fn disable_ime(&mut self) -> BitFlags<ImeState> {
        let flags = if self.ime_state.is_some() {
            ImeState::Disabled.into()
        } else {
            ImeState::empty()
        };
        if self.ime_state.is_some() {
            self.ime_state = None;
        }

        self.preedit = None;
        flags
    }
}
