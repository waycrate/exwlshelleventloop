use crate::{NewPopUpSettings, PopUpRepositionSettings};

use super::{RefreshRequest, Shell, Size, WindowState, WindowStateUnitBuilder, build_positioner};
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

impl<T: 'static> WindowState<T> {
    pub fn popup_reposition(
        &mut self,
        PopUpRepositionSettings {
            size,
            placement,
            anchor,
            gravity,
            constraint_adjustment,
        }: PopUpRepositionSettings,
        id: crate::id::Id,
    ) {
        let Some(unit) = self.units.iter_mut().find(|unit| unit.id == id) else {
            return;
        };
        let Shell::PopUp((popup, _)) = &unit.shell else {
            log::warn!(
                target: "exwlshellev",
                "reposition target {id:?} is not a popup; only popups can be repositioned"
            );
            return;
        };
        if popup.version() < 3 {
            log::warn!(
                target: "exwlshellev",
                "compositor offers xdg_popup v{}, reposition needs v3; leaving popup {id:?} as it is",
                popup.version()
            );
            return;
        }
        let qh = self.queue_handle.clone();
        let wmbase = &self.wmbase;
        let positioner = build_positioner(
            &wmbase,
            &qh,
            size,
            placement,
            anchor,
            gravity,
            constraint_adjustment,
        );
        let token = unit.pending_reposition.unwrap_or(0).wrapping_add(1);
        popup.reposition(&positioner, token);
        positioner.destroy();
        unit.pending_reposition = Some(token);
    }
    pub fn create_popup(
        &mut self,
        NewPopUpSettings {
            size,
            id: parent_id,
            placement,
            anchor,
            gravity,
            constraint_adjustment,
            grab_serial,
        }: NewPopUpSettings,
        binding: impl Into<Option<T>>,
    ) -> Option<crate::id::Id> {
        let index = self.units.iter().position(|unit| unit.id == parent_id)?;

        let qh = self.queue_handle.clone();
        let id = crate::id::Id::unique();
        let wl_surface = self.wl_compositor.create_surface(&qh, ());
        let wmbase = &self.wmbase;
        let positioner = build_positioner(
            &wmbase,
            &qh,
            size,
            placement,
            anchor,
            gravity,
            constraint_adjustment,
        );
        let wl_xdg_surface = wmbase.get_xdg_surface(&wl_surface, &qh, ());

        let popup = match &self.units[index].shell {
            Shell::LayerShell(shell) => {
                let popup = wl_xdg_surface.get_popup(None, &positioner, &qh, ());
                shell.get_popup(&popup);
                popup
            }
            Shell::PopUp((_, parent_xdg_surface)) => {
                wl_xdg_surface.get_popup(Some(parent_xdg_surface), &positioner, &qh, ())
            }
            Shell::XdgTopLevel((_, parent_xdg_surface, _)) => {
                wl_xdg_surface.get_popup(Some(parent_xdg_surface), &positioner, &qh, ())
            }
            _ => {
                log::warn!(
                    target: "exwlshellev",
                    "cannot create popup: parent {:?} must be a layer surface, an xdg_toplevel or a popup",
                    id
                );
                positioner.destroy();
                wl_xdg_surface.destroy();
                wl_surface.destroy();
                return None;
            }
        };
        positioner.destroy();

        match (self.seat_back.as_ref(), grab_serial) {
            (Some(seat), Some(serial)) => popup.grab(seat, serial),
            (None, Some(_)) => log::warn!(
                target: "exwlshellev",
                "popup {id:?} wants a grab but no seat is available; it will not dismiss on click-outside"
            ),
            (_, None) => {}
        }

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
        self.push_window(
            WindowStateUnitBuilder::new(
                id,
                qh.clone(),
                self.display.clone(),
                wl_surface,
                self.wl_compositor.clone(),
                Shell::PopUp((popup, wl_xdg_surface)),
            )
            .parent(Some(parent_id))
            .size(size.to_size())
            .viewport(viewport)
            .fractional_scale(fractional_scale)
            .binding(binding.into())
            .build(),
        );
        Some(id)
    }
}
