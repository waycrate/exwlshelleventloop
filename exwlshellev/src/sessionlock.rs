use super::{DispatchMessage, RefreshRequest, Size, WindowState};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1, ext_session_lock_surface_v1,
    ext_session_lock_v1::ExtSessionLockV1,
};

delegate_noop!(@<T> WindowState<T>: ignore ExtSessionLockManagerV1);

impl<T> Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        surface: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
        event: <ext_session_lock_surface_v1::ExtSessionLockSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            surface.ack_configure(serial);

            let Some(unit_index) = state.units.iter().position(|unit| unit.shell == *surface)
            else {
                return;
            };
            state.units[unit_index].size = Size { width, height };
            state.units[unit_index].configured = true;
            state.units[unit_index].request_refresh(RefreshRequest::NextFrame);
        }
    }
}

impl<T> Dispatch<ExtSessionLockV1, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: <ExtSessionLockV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::Event;
        match event {
            Event::Locked => {
                state.messages.push((None, DispatchMessage::Locked));
            }
            Event::Finished => {
                state.messages.push((None, DispatchMessage::LockFinished));
            }
            _ => unreachable!(),
        }
    }
}
