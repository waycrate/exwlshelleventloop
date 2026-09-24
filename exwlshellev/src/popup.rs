use super::{RefreshRequest, Size, WindowState};
use wayland_protocols::xdg::shell::client::xdg_popup;

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

impl<T> Dispatch<xdg_popup::XdgPopup, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &xdg_popup::XdgPopup,
        event: <xdg_popup::XdgPopup as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_popup::Event::Configure { width, height, .. } => {
                let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
                else {
                    return;
                };
                state.units[unit_index].size = Size {
                    width: width as u32,
                    height: height as u32,
                };
                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            xdg_popup::Event::PopupDone => {
                if let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
                {
                    state.units[unit_index].request_close();
                }
            }
            xdg_popup::Event::Repositioned { token } => {
                if let Some(unit) = state.units.iter_mut().find(|unit| unit.shell == *surface)
                    && unit.pending_reposition == Some(token)
                {
                    unit.pending_reposition = None;
                }
            }
            _ => {}
        }
    }
}
