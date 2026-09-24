use crate::NewXdgWindowSettings;

use super::{
    DispatchMessage, PixelSize, RefreshRequest, Shell, Size, ToplevelState, WindowState,
    WindowStateUnitBuilder,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;

use wayland_protocols::xdg::shell::client::xdg_toplevel;

fn toplevel_state_from_configure(states: &[u8]) -> ToplevelState {
    let mut toplevel_state = ToplevelState::default();
    for raw in states.as_chunks::<4>().0 {
        let raw = u32::from_ne_bytes([raw[0], raw[1], raw[2], raw[3]]);
        let Ok(state) = xdg_toplevel::State::try_from(raw) else {
            continue;
        };
        match state {
            xdg_toplevel::State::Maximized => toplevel_state.maximized = true,
            xdg_toplevel::State::Fullscreen => toplevel_state.fullscreen = true,
            xdg_toplevel::State::Activated => toplevel_state.activated = true,
            xdg_toplevel::State::TiledLeft
            | xdg_toplevel::State::TiledRight
            | xdg_toplevel::State::TiledTop
            | xdg_toplevel::State::TiledBottom => toplevel_state.tiled = true,
            _ => {}
        }
    }
    toplevel_state
}

impl<T> Dispatch<xdg_toplevel::XdgToplevel, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &xdg_toplevel::XdgToplevel,
        event: <xdg_toplevel::XdgToplevel as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let unit_index = state.units.iter().position(|unit| unit.shell == *surface);
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                let Some(unit_index) = unit_index else {
                    return;
                };
                if width != 0 && height != 0 {
                    state.units[unit_index].size = Size {
                        width: width as u32,
                        height: height as u32,
                    };
                }

                let toplevel_state = toplevel_state_from_configure(&states);
                if state.units[unit_index].toplevel_state != toplevel_state {
                    state.units[unit_index].toplevel_state = toplevel_state;
                    let id = state.units[unit_index].id;
                    state.messages.push((
                        Some(id),
                        DispatchMessage::ToplevelStateChanged(toplevel_state),
                    ));
                }

                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            xdg_toplevel::Event::Close => {
                let Some(unit_index) = unit_index else {
                    return;
                };
                state.units[unit_index].request_flag.close = true;
            }
            _ => {}
        }
    }
}

impl<T: 'static> WindowState<T> {
    pub fn create_xdg_base_window(
        &mut self,
        NewXdgWindowSettings {
            title,
            size,
            client_side_decorations,
        }: NewXdgWindowSettings,
        info: impl Into<Option<T>>,
    ) -> Option<crate::id::Id> {
        let qh = self.queue_handle.clone();
        let wmbase = &self.wmbase;
        let wl_surface = self.wl_compositor.create_surface(&qh, ());
        let wl_xdg_surface = wmbase.get_xdg_surface(&wl_surface, &qh, ());
        let toplevel = wl_xdg_surface.get_toplevel(&qh, ());

        toplevel.set_title(title.unwrap_or("".to_owned()));

        let decoration = if let Some(decoration_manager) = &self.xdg_decoration_manager {
            let decoration = decoration_manager.get_toplevel_decoration(&toplevel, &qh, ());
            use zxdg_toplevel_decoration_v1::Mode;
            decoration.set_mode(if client_side_decorations {
                Mode::ClientSide
            } else {
                Mode::ServerSide
            });
            Some(decoration)
        } else {
            None
        };
        let mut fractional_scale = None;
        if let Some(ref fractional_scale_manager) = self.fractional_scale_manager {
            fractional_scale =
                Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
        }
        wl_surface.commit();

        let viewport = self
            .viewporter
            .as_ref()
            .map(|viewport| viewport.get_viewport(&wl_surface, &qh, ()));

        let id = crate::id::Id::unique();
        self.push_window(
            WindowStateUnitBuilder::new(
                id,
                qh.clone(),
                self.display.clone(),
                wl_surface,
                self.wl_compositor.clone(),
                Shell::XdgTopLevel((toplevel, wl_xdg_surface, decoration)),
            )
            .size(size.unwrap_or(PixelSize::px(300, 300)).to_size())
            .viewport(viewport)
            .fractional_scale(fractional_scale)
            .binding(info.into())
            .build(),
        );
        Some(id)
    }
}
