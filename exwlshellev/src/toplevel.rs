use super::{DispatchMessage, RefreshRequest, Size, ToplevelState, WindowState};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
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
