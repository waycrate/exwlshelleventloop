use super::{RefreshRequest, Size, WindowState};

use wayland_client::{Connection, Dispatch, Proxy, delegate_noop};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1, zwlr_layer_surface_v1,
};

delegate_noop!(@<T> WindowState<T>: ignore ZwlrLayerShellV1); // it is similar with xdg_toplevel, also the

impl<T> Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let unit_index = state.units.iter().position(|unit| unit.shell == *surface);
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);

                let Some(unit_index) = unit_index else {
                    return;
                };
                state.units[unit_index].size = Size { width, height };
                state.units[unit_index].configured = true;
                state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
            }
            zwlr_layer_surface_v1::Event::Closed => {
                if let Some(i) = unit_index {
                    state.units[i].request_close();
                }
            }
            _ => log::info!("ignore zwlr_layer_surface_v1 event: {event:?}"),
        }
    }
}
