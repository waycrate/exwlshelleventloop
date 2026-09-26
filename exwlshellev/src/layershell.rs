use super::{
    Margin, RefreshRequest, Shell, Size, WindowState, WindowStateUnitBuilder,
    warn_if_exclusive_zone_ignored,
};
use crate::NewLayerShellSettings;

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

impl<T: 'static> WindowState<T> {
    pub fn create_layershell(
        &mut self,
        NewLayerShellSettings {
            size,
            layer,
            anchor,
            exclusive_zone,
            margin,
            keyboard_interactivity,
            output_option: output_type,
            events_transparent,
            namespace,
            blur_option,
        }: NewLayerShellSettings,
        binding: impl Into<Option<T>>,
    ) -> Result<crate::id::Id, crate::ExWlShellEventError> {
        let output = self.resolve_output(output_type);
        let layer_shell =
            self.layer_shell
                .as_ref()
                .ok_or(crate::ExWlShellEventError::Unsupported(
                    crate::ProtocolType::LayerShell,
                ))?;

        let qh = self.queue_handle.clone();
        let wire_anchor = size.resolve_anchor(anchor);

        let wl_surface = self.wl_compositor.create_surface(&qh, ());

        let id = crate::id::Id::unique();
        let layer = layer_shell.get_layer_surface(
            &wl_surface,
            output.as_ref(),
            layer,
            namespace.unwrap_or_else(|| self.default_namespace.clone()),
            &qh,
            (),
        );
        layer.set_anchor(wire_anchor);
        layer.set_keyboard_interactivity(keyboard_interactivity);
        let (init_w, init_h) = size.to_set();
        layer.set_size(init_w, init_h);

        if let Some(zone) = exclusive_zone {
            warn_if_exclusive_zone_ignored(zone, wire_anchor);
            layer.set_exclusive_zone(zone);
        }

        if let Some(Margin {
            top,
            right,
            bottom,
            left,
        }) = margin
        {
            layer.set_margin(top, right, bottom, left);
        }

        if events_transparent {
            let region = self.wl_compositor.create_region(&qh, ());
            wl_surface.set_input_region(Some(&region));
            region.destroy();
        }

        wl_surface.commit();

        let mut effect = None;
        if let Some(effect_manger) = &self.background_effect_manager {
            effect = Some(effect_manger.get_background_effect(&wl_surface, &qh, ()));
        }
        let mut fractional_scale = None;
        if let Some(ref fractional_scale_manager) = self.fractional_scale_manager {
            fractional_scale =
                Some(fractional_scale_manager.get_fractional_scale(&wl_surface, &qh, ()));
        }
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
                Shell::LayerShell(layer),
            )
            .layout(self.anchor, self.size)
            .viewport(viewport)
            .blur_option(blur_option)
            .effect_surface(effect)
            .fractional_scale(fractional_scale)
            .wl_output(output)
            .binding(binding.into())
            .build(),
        );
        Ok(id)
    }
}
