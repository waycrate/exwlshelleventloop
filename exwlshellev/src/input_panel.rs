use super::{NewInputPanelSettings, Shell, WindowState, WindowStateUnitBuilder};
use wayland_client::delegate_noop;
use wayland_protocols::wp::input_method::zv1::client::{
    zwp_input_panel_surface_v1::{Position as ZwpInputPanelPosition, ZwpInputPanelSurfaceV1},
    zwp_input_panel_v1::ZwpInputPanelV1,
};

delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelSurfaceV1);
delegate_noop!(@<T> WindowState<T>: ignore ZwpInputPanelV1);

impl<T: 'static> WindowState<T> {
    pub fn create_input_panel(
        &mut self,
        NewInputPanelSettings {
            size,
            keyboard,
            output_option: output_type,
        }: NewInputPanelSettings,
        info: impl Into<Option<T>>,
    ) -> Option<crate::id::Id> {
        let info = info.into();
        let output = self.resolve_output(output_type);

        let Some(output) = output else {
            log::warn!("no WlOutput, skip creating input panel");
            return None;
        };

        let id = crate::id::Id::unique();
        let qh = self.queue_handle.clone();
        let wl_surface = self.wl_compositor.create_surface(&qh, ());
        let input_panel = self.input_panel.as_ref()?;
        let input_panel_surface = input_panel.get_input_panel_surface(&wl_surface, &qh, ());
        if keyboard {
            input_panel_surface.set_toplevel(&output, ZwpInputPanelPosition::CenterBottom as u32);
        } else {
            input_panel_surface.set_overlay_panel();
        }
        wl_surface.commit();

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
                Shell::InputPanel(input_panel_surface),
            )
            .size(size.to_size())
            .viewport(viewport)
            .fractional_scale(fractional_scale)
            .binding(info)
            .build(),
        );
        Some(id)
    }
}
