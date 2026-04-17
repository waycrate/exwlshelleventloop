use super::WindowState;
use sctk::seat::{Capability as SeatCapability, SeatHandler};
use waycrate_xkbkeycode::xkb_keyboard;
use wayland_backend::client::ObjectId;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_keyboard::{self, KeyState, KeymapFormat},
        wl_pointer::{self, WlPointer},
        wl_seat::{self, WlSeat},
        wl_touch::{self, WlTouch},
    },
};

use crate::{AxisScroll, DispatchMessageInner, KeyboardTokenState, RepeatInfo};

use std::time::Duration;

impl<T> WindowState<T> {
    /// get a seat from state
    pub fn get_seat(&self) -> &WlSeat {
        self.seat_back.as_ref().unwrap()
    }
    pub fn get_keyboard_state_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut xkb_keyboard::KeyboardState> {
        self.seats
            .values_mut()
            .flat_map(|seat| &mut seat.keyboard_state)
    }
    pub fn get_keyboard_state_by_id(
        &mut self,
        id: ObjectId,
    ) -> Option<&mut xkb_keyboard::KeyboardState> {
        self.seats
            .values_mut()
            .find(|seat| {
                seat.keyboard_state
                    .as_ref()
                    .is_some_and(|state| state.keyboard.id() == id)
            })
            .map(|storage| storage.keyboard_state.as_mut().unwrap())
    }
    pub fn get_pointers(&self) -> Vec<WlPointer> {
        self.seats
            .values()
            .flat_map(|seat| &seat.pointer)
            .cloned()
            .collect()
    }
    /// get the pointer
    pub fn get_pointers_iter(&self) -> impl Iterator<Item = &WlPointer> {
        self.seats.values().flat_map(|seat| &seat.pointer)
    }
    pub fn get_touchers(&self) -> Vec<WlTouch> {
        self.seats
            .values()
            .flat_map(|seat| &seat.touch)
            .cloned()
            .collect()
    }
    /// get the touch
    pub fn get_touches_iter(&self) -> impl Iterator<Item = &WlTouch> {
        self.seats.values().flat_map(|seat| &seat.touch)
    }
}

#[derive(Debug, Default)]
pub(crate) struct SeatStorage {
    pub touch: Option<WlTouch>,
    pub pointer: Option<WlPointer>,
    pub keyboard_state: Option<xkb_keyboard::KeyboardState>,
}

impl Drop for SeatStorage {
    fn drop(&mut self) {
        if let Some(touch) = self.touch.take()
            && touch.version() >= 3
        {
            touch.release();
        }
        if let Some(pointer) = self.pointer.take()
            && pointer.version() >= 3
        {
            pointer.release();
        }
    }
}

impl SeatStorage {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl<T: 'static> SeatHandler for WindowState<T> {
    fn seat_state(&mut self) -> &mut sctk::seat::SeatState {
        self.seat_state.as_mut().unwrap()
    }
    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seats.insert(seat.id(), SeatStorage::new());
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        let _ = self.seats.remove(&seat.id());
    }
    fn new_capability(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: sctk::seat::Capability,
    ) {
        let seat_state = match self.seats.get_mut(&seat.id()) {
            Some(seat_state) => seat_state,
            None => {
                log::warn!("Received wl_seat::new_capability for unknown seat");
                return;
            }
        };

        use xkb_keyboard::KeyboardState;
        match capability {
            SeatCapability::Touch if seat_state.touch.is_none() => {
                seat_state.touch = Some(seat.get_touch(queue_handle, ()));
            }
            SeatCapability::Keyboard if seat_state.keyboard_state.is_none() => {
                seat_state.keyboard_state =
                    Some(KeyboardState::new(seat.get_keyboard(queue_handle, ())));
            }
            SeatCapability::Pointer if seat_state.pointer.is_none() => {
                seat_state.pointer = Some(seat.get_pointer(queue_handle, ()));
            }
            _ => (),
        }
    }
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: sctk::seat::Capability,
    ) {
        let seat_state = match self.seats.get_mut(&seat.id()) {
            Some(seat_state) => seat_state,
            None => {
                log::warn!("Received wl_seat::new_capability for unknown seat");
                return;
            }
        };
        match capability {
            SeatCapability::Touch => {
                if let Some(touch) = seat_state.touch.take()
                    && touch.version() >= 3
                {
                    touch.release();
                }
            }
            SeatCapability::Pointer => {
                if let Some(pointer) = seat_state.pointer.take()
                    && pointer.version() >= 3
                {
                    pointer.release();
                }
            }
            SeatCapability::Keyboard => {
                seat_state.keyboard_state = None;
            }
            _ => (),
        }
    }
}

impl<T> Dispatch<wl_keyboard::WlKeyboard, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        wl_keyboard: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        use crate::keyboard::*;
        use xkb_keyboard::ElementState;
        let surface_id = state.current_surface_id();

        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => match format {
                WEnum::Value(KeymapFormat::XkbV1) => {
                    let Some(keyboard_state) = state
                        .seats
                        .values_mut()
                        .find(|seat_storage| {
                            seat_storage
                                .keyboard_state
                                .as_ref()
                                .is_some_and(|state| state.keyboard == *wl_keyboard)
                        })
                        .map(|storage| storage.keyboard_state.as_mut().unwrap())
                    else {
                        return;
                    };
                    let context = &mut keyboard_state.xkb_context;
                    context.set_keymap_from_fd(fd, size as usize)
                }
                WEnum::Value(KeymapFormat::NoKeymap) => {
                    log::warn!("non-xkb compatible keymap")
                }
                _ => unreachable!(),
            },
            wl_keyboard::Event::Enter { surface, .. } => {
                state.update_current_surface(Some(surface));
                let Some(keyboard_state) = state
                    .seats
                    .values_mut()
                    .find(|seat_storage| {
                        seat_storage
                            .keyboard_state
                            .as_ref()
                            .is_some_and(|state| state.keyboard == *wl_keyboard)
                    })
                    .map(|storage| storage.keyboard_state.as_mut().unwrap())
                else {
                    return;
                };
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Leave { .. } => {
                let Some(keyboard_state) = state
                    .seats
                    .values_mut()
                    .find(|seat_storage| {
                        seat_storage
                            .keyboard_state
                            .as_ref()
                            .is_some_and(|state| state.keyboard == *wl_keyboard)
                    })
                    .map(|storage| storage.keyboard_state.as_mut().unwrap())
                else {
                    return;
                };
                keyboard_state.current_repeat = None;
                state.message.push((
                    surface_id,
                    DispatchMessageInner::ModifiersChanged(ModifiersState::empty()),
                ));
                state
                    .message
                    .push((surface_id, DispatchMessageInner::UnFocused));
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Key {
                state: keystate,
                key,
                ..
            } => {
                let pressed_state = match keystate {
                    WEnum::Value(KeyState::Pressed) => ElementState::Pressed,
                    WEnum::Value(KeyState::Released) => ElementState::Released,
                    _ => {
                        return;
                    }
                };
                let Some(keyboard_state) = state
                    .seats
                    .values_mut()
                    .find(|seat_storage| {
                        seat_storage
                            .keyboard_state
                            .as_ref()
                            .is_some_and(|state| state.keyboard == *wl_keyboard)
                    })
                    .map(|storage| storage.keyboard_state.as_mut().unwrap())
                else {
                    return;
                };
                let key = key + 8;
                if let Some(mut key_context) = keyboard_state.xkb_context.key_context() {
                    let event = key_context.process_key_event(key, pressed_state, false);
                    let event = DispatchMessageInner::KeyboardInput {
                        event,
                        is_synthetic: false,
                    };
                    state.message.push((surface_id, event));
                }

                match pressed_state {
                    ElementState::Pressed => {
                        let delay = match keyboard_state.repeat_info {
                            RepeatInfo::Repeat { delay, .. } => delay,
                            RepeatInfo::Disable => return,
                        };

                        if keyboard_state
                            .xkb_context
                            .keymap_mut()
                            .is_none_or(|keymap| !keymap.key_repeats(key))
                        {
                            return;
                        }

                        keyboard_state.current_repeat = Some(key);

                        if let Some(token) = keyboard_state.repeat_token.take() {
                            state.to_remove_tokens.push(token);
                        }

                        state.repeat_delay = Some(KeyboardTokenState {
                            delay,
                            key,
                            surface_id,
                            pressed_state,
                            object_id: wl_keyboard.id(),
                        });
                    }
                    ElementState::Released => {
                        if keyboard_state.repeat_info != RepeatInfo::Disable
                            && keyboard_state
                                .xkb_context
                                .keymap_mut()
                                .is_some_and(|keymap| keymap.key_repeats(key))
                            && Some(key) == keyboard_state.current_repeat
                        {
                            keyboard_state.current_repeat = None;
                            if let Some(token) = keyboard_state.repeat_token.take() {
                                state.to_remove_tokens.push(token);
                            }
                        }
                    }
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_locked,
                mods_latched,
                group,
                ..
            } => {
                let Some(keyboard_state) = state
                    .seats
                    .values_mut()
                    .find(|seat_storage| {
                        seat_storage
                            .keyboard_state
                            .as_ref()
                            .is_some_and(|state| state.keyboard == *wl_keyboard)
                    })
                    .map(|storage| storage.keyboard_state.as_mut().unwrap())
                else {
                    return;
                };
                let xkb_context = &mut keyboard_state.xkb_context;
                let xkb_state = match xkb_context.state_mut() {
                    Some(state) => state,
                    None => return,
                };
                xkb_state.update_modifiers(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                let modifiers = xkb_state.modifiers();

                state.message.push((
                    state.current_surface_id(),
                    DispatchMessageInner::ModifiersChanged(modifiers.into()),
                ))
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                let Some(keyboard_state) = state
                    .seats
                    .values_mut()
                    .find(|seat_storage| {
                        seat_storage
                            .keyboard_state
                            .as_ref()
                            .is_some_and(|state| state.keyboard == *wl_keyboard)
                    })
                    .map(|storage| storage.keyboard_state.as_mut().unwrap())
                else {
                    return;
                };
                keyboard_state.repeat_info = if rate == 0 {
                    // Stop the repeat once we get a disable event.
                    keyboard_state.current_repeat = None;
                    if let Some(token) = keyboard_state.repeat_token.take() {
                        state.to_remove_tokens.push(token);
                    }
                    RepeatInfo::Disable
                } else {
                    let gap = Duration::from_micros(1_000_000 / rate as u64);
                    let delay = Duration::from_millis(delay as u64);
                    RepeatInfo::Repeat { gap, delay }
                };
            }
            _ => {}
        }
    }
}

impl<T> Dispatch<wl_touch::WlTouch, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        _proxy: &wl_touch::WlTouch,
        event: <wl_touch::WlTouch as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_touch::Event::Down {
                serial,
                time,
                surface,
                id,
                x,
                y,
            } => {
                state.finger_locations.insert(id, (x, y));
                let surface_id = state.get_id_from_surface(&surface);
                state
                    .active_surfaces
                    .insert(Some(id), (surface.clone(), surface_id));
                state.update_current_surface(Some(surface));
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchDown {
                        serial,
                        time,
                        id,
                        x,
                        y,
                    },
                ))
            }
            wl_touch::Event::Cancel => {
                let mut mouse_surface = None;
                for (k, v) in state.active_surfaces.drain() {
                    if let Some(id) = k {
                        let (x, y) = state.finger_locations.remove(&id).unwrap_or_default();
                        state
                            .message
                            .push((v.1, DispatchMessageInner::TouchCancel { id, x, y }));
                    } else {
                        // keep the surface of mouse.
                        mouse_surface = Some(v);
                    }
                }
                if let Some(mouse_surface) = mouse_surface {
                    state.active_surfaces.insert(None, mouse_surface);
                }
            }
            wl_touch::Event::Up { serial, time, id } => {
                let surface_id = state
                    .active_surfaces
                    .remove(&Some(id))
                    .or_else(|| {
                        log::warn!("finger[{id}] hasn't been down.");
                        None
                    })
                    .and_then(|(_, id)| id);
                let (x, y) = state.finger_locations.remove(&id).unwrap_or_default();
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchUp {
                        serial,
                        time,
                        id,
                        x,
                        y,
                    },
                ));
            }
            wl_touch::Event::Motion { time, id, x, y } => {
                let surface_id = state
                    .active_surfaces
                    .get(&Some(id))
                    .or_else(|| {
                        log::warn!("finger[{id}] hasn't been down.");
                        None
                    })
                    .and_then(|(_, id)| *id);
                state.finger_locations.insert(id, (x, y));
                state.message.push((
                    surface_id,
                    DispatchMessageInner::TouchMotion { time, id, x, y },
                ));
            }
            _ => {}
        }
    }
}

impl<T> Dispatch<wl_pointer::WlPointer, ()> for WindowState<T> {
    fn event(
        state: &mut Self,
        pointer: &wl_pointer::WlPointer,
        event: <wl_pointer::WlPointer as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // All mouse events should be happened on the surface which is hovered by the mouse.
        let (mouse_surface, surface_id) = state
            .active_surfaces
            .get(&None)
            .map(|(surface, id)| (Some(surface), *id))
            .unwrap_or_else(|| {
                match &event {
                    wl_pointer::Event::Enter { .. } => {}
                    _ => {
                        log::warn!("mouse hasn't entered.");
                    }
                }
                (None, None)
            });
        let scale = surface_id
            .and_then(|id| state.get_unit_with_id(id))
            .map(|unit| unit.scale_float())
            .unwrap_or(1.0);
        match event {
            wl_pointer::Event::Axis { time, axis, value } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.absolute = value;
                        }
                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.absolute = value;
                        }
                        _ => unreachable!(),
                    };

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ))
                }
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::AxisStop { time, axis } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => vertical.stop = true,
                        wl_pointer::Axis::HorizontalScroll => horizontal.stop = true,

                        _ => unreachable!(),
                    }

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ));
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::AxisSource { axis_source } => match axis_source {
                WEnum::Value(source) => state.message.push((
                    surface_id,
                    DispatchMessageInner::Axis {
                        horizontal: AxisScroll::default(),
                        vertical: AxisScroll::default(),
                        source: Some(source),
                        time: 0,
                        scale,
                    },
                )),
                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "unknown pointer axis source: {unknown:x}");
                }
            },
            wl_pointer::Event::AxisDiscrete { axis, discrete } => match axis {
                WEnum::Value(axis) => {
                    let (mut horizontal, mut vertical) = <(AxisScroll, AxisScroll)>::default();
                    match axis {
                        wl_pointer::Axis::VerticalScroll => {
                            vertical.discrete = discrete;
                        }

                        wl_pointer::Axis::HorizontalScroll => {
                            horizontal.discrete = discrete;
                        }

                        _ => unreachable!(),
                    };

                    state.message.push((
                        surface_id,
                        DispatchMessageInner::Axis {
                            time: 0,
                            scale,
                            horizontal,
                            vertical,
                            source: None,
                        },
                    ));
                }

                WEnum::Unknown(unknown) => {
                    log::warn!(target: "sessionlockev", "{}: invalid pointer axis: {:x}", pointer.id(), unknown);
                }
            },
            wl_pointer::Event::Button {
                state: btnstate,
                serial,
                button,
                time,
            } => {
                let mouse_surface = mouse_surface.cloned();
                state.update_current_surface(mouse_surface);
                state.message.push((
                    surface_id,
                    DispatchMessageInner::MouseButton {
                        state: btnstate,
                        serial,
                        button,
                        time,
                    },
                ));
            }
            wl_pointer::Event::Leave { .. } => {
                let surface_id = state
                    .active_surfaces
                    .remove(&None)
                    .or_else(|| {
                        log::warn!("mouse hasn't entered.");
                        None
                    })
                    .and_then(|(_, id)| id);
                state
                    .message
                    .push((surface_id, DispatchMessageInner::MouseLeave));
            }
            wl_pointer::Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                let surface_id = state.get_id_from_surface(&surface);
                state
                    .active_surfaces
                    .insert(None, (surface.clone(), surface_id));
                state.enter_serial = Some(serial);
                state.message.push((
                    surface_id,
                    DispatchMessageInner::MouseEnter {
                        pointer: pointer.clone(),
                        serial,
                        surface_x,
                        surface_y,
                    },
                ));
                if let Some(id) = state.current_surface_id() {
                    state
                        .message
                        .push((Some(id), DispatchMessageInner::Focused(id)));
                }
            }
            wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                state.message.push((
                    surface_id,
                    DispatchMessageInner::MouseMotion {
                        time,
                        surface_x,
                        surface_y,
                    },
                ));
            }
            _ => {
                // TODO: not now
            }
        }
    }
}

sctk::delegate_seat!(@<T: 'static > WindowState<T>);
delegate_noop!(@<T: 'static> WindowState<T>: ignore WlSeat);
