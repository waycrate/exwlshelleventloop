use std::{collections::BTreeMap, sync::Arc};

use super::state::State;
use crate::DefaultStyle;
use crate::ime_preedit::{ImeState, Preedit};
use enumflags2::{BitFlag, BitFlags};
use iced_core::InputMethod;
use iced_core::input_method;
use iced_graphics::Compositor;
use layershellev::{WindowWrapper, id::Id as LayerId};

use iced::mouse;
use iced::window::Id as IcedId;
use iced_program::Instance;
use iced_program::Program;

pub struct Window<P, C>
where
    P: Program,
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    pub id: LayerId,
    #[allow(unused)]
    pub iced_id: IcedId,
    pub renderer: P::Renderer,
    pub surface: C::Surface,
    pub state: State<P>,
    pub mouse_interaction: mouse::Interaction,
    preedit: Option<Preedit<P::Renderer>>,
    ime_state: Option<(iced_core::Rectangle, input_method::Purpose)>,
}

pub struct WindowManager<P: Program, C: Compositor>
where
    C: Compositor<Renderer = P::Renderer>,
    P::Theme: DefaultStyle,
{
    aliases: BTreeMap<LayerId, IcedId>,
    back_aliases: BTreeMap<IcedId, LayerId>,
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

    pub fn first(&self) -> Option<&Window<P, C>> {
        self.entries.first_key_value().map(|(_, v)| v)
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
                iced_id: id,
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IcedId, &mut Window<P, C>)> {
        self.entries.iter_mut().map(|(k, v)| (*k, v))
    }

    pub fn first_window(&self) -> Option<(&IcedId, &Window<P, C>)> {
        self.entries.iter().next()
    }

    pub fn last_window(&self) -> Option<(&IcedId, &Window<P, C>)> {
        self.entries.iter().last()
    }

    pub fn get_mut_alias(&mut self, id: LayerId) -> Option<(IcedId, &mut Window<P, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get_mut(id)?))
    }

    pub fn get_alias(&self, id: LayerId) -> Option<(IcedId, &Window<P, C>)> {
        let id = self.aliases.get(&id).copied()?;

        Some((id, self.get(id)?))
    }

    pub fn get_mut(&mut self, id: IcedId) -> Option<&mut Window<P, C>> {
        self.entries.get_mut(&id)
    }
    pub fn get(&self, id: IcedId) -> Option<&Window<P, C>> {
        self.entries.get(&id)
    }
}

impl<A, C> Window<A, C>
where
    A: Program,
    C: Compositor<Renderer = A::Renderer>,
    A::Theme: DefaultStyle,
{
    pub fn request_input_method(&mut self, input_method: InputMethod) -> BitFlags<ImeState> {
        match input_method {
            InputMethod::Disabled => self.disable_ime(),
            InputMethod::Enabled {
                purpose,
                preedit,
                cursor,
            } => {
                let mut flags = ImeState::empty();
                if self.ime_state.is_none() {
                    flags.insert(ImeState::Allowed);
                }
                if self.ime_state != Some((cursor, purpose)) {
                    flags.insert(ImeState::Update);
                }
                self.update_ime(cursor, purpose);

                if let Some(preedit) = preedit {
                    if preedit.content.is_empty() {
                        self.preedit = None;
                    } else {
                        let mut overlay = self.preedit.take().unwrap_or_else(Preedit::new);

                        overlay.update(
                            cursor,
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

    fn update_ime(&mut self, position: iced_core::Rectangle, purpose: input_method::Purpose) {
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
