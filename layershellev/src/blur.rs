use super::WindowState;
use wayland_client::delegate_noop;
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};
// TODO: handle callback
delegate_noop!(@<T> WindowState<T>: ignore ExtBackgroundEffectSurfaceV1);
delegate_noop!(@<T> WindowState<T>: ignore ExtBackgroundEffectManagerV1);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlurRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum BlurOption {
    #[default]
    None,
    FullRegion,
    Region(Vec<BlurRegion>),
}
