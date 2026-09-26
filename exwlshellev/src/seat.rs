use super::WindowState;
use crate::events::AxisFrame;
use crate::{DispatchMessage, KeyboardTokenState, RepeatInfo, TextInputData, id};
use sctk::seat::{Capability as SeatCapability, SeatHandler};
use waycrate_xkbkeycode::xkb_keyboard;
use wayland_backend::client::ObjectId;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop,
    protocol::{
        wl_keyboard::{self, KeyState, KeymapFormat, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_seat::{self, WlSeat},
        wl_touch::{self, WlTouch},
    },
};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::ZwpTextInputV3;

use std::time::Duration;
impl<T> WindowState<T> {
    /// get a seat from state
    pub fn get_seat(&self) -> &WlSeat {
        self.seat_back.as_ref().unwrap()
    }

    /// get the keyboard
    pub fn get_keyboards(&self) -> Vec<WlKeyboard> {
        self.seats
            .values()
            .flat_map(|seat| &seat.keyboard_state)
            .map(|keyboard_state| &keyboard_state.keyboard)
            .cloned()
            .collect()
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
    pub(crate) fn get_keyboard_state_mut(
        &mut self,
        wl_keyboard: &wl_keyboard::WlKeyboard,
    ) -> Option<&mut xkb_keyboard::KeyboardState> {
        self.seats
            .values_mut()
            .find(|seat_storage| {
                seat_storage
                    .keyboard_state
                    .as_ref()
                    .is_some_and(|state| state.keyboard == *wl_keyboard)
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

    fn pointer_frame(&mut self, pointer: &WlPointer) -> &mut PointerFrame {
        self.pending_pointer_frames.entry(pointer.id()).or_default()
    }
    /// Emits the events for `pointer` since its last `wl_pointer.frame`, in received order
    fn flush_pointer_frame(&mut self, pointer: &WlPointer) {
        if let Some(frame) = self.pending_pointer_frames.remove(&pointer.id()) {
            self.messages.extend(frame.into_messages());
        }
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
/// The events of one pointer since its last `wl_pointer.frame`, which belong together.
#[derive(Debug, Default)]
pub(crate) struct PointerFrame {
    messages: Vec<(Option<id::Id>, DispatchMessage)>,
    axis: Option<(usize, Option<id::Id>, AxisFrame)>,
}

impl PointerFrame {
    fn push(&mut self, surface_id: Option<id::Id>, message: DispatchMessage) {
        self.messages.push((surface_id, message));
    }

    fn accumulate_axis(&mut self, surface_id: Option<id::Id>, event: &wl_pointer::Event) {
        let index = self.messages.len();
        let (_, _, axis) = self
            .axis
            .get_or_insert_with(|| (index, surface_id, AxisFrame::default()));
        axis.accumulate(event);
    }

    fn into_messages(mut self) -> Vec<(Option<id::Id>, DispatchMessage)> {
        if let Some((index, surface_id, axis)) = self.axis {
            self.messages
                .insert(index, (surface_id, axis.into_message()));
        }
        self.messages
    }
}

#[derive(Debug, Default)]
pub(crate) struct SeatStorage {
    pub touch: Option<WlTouch>,
    pub pointer: Option<WlPointer>,
    pub keyboard_state: Option<xkb_keyboard::KeyboardState>,
    pub text_input: Option<ZwpTextInputV3>,
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
        // destroy text_input during drop
        if let Some(text_input) = self.text_input.take() {
            text_input.destroy();
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
        &mut self.seat_state
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
        // We should always allow a seat has a text_input, even there is not a keyboard, because
        // maybe we can have virtual-keyboard
        if seat_state.text_input.is_none() {
            let text_input = self.text_input_manager.as_ref().map(|manager| {
                manager.get_text_input(&seat, queue_handle, TextInputData::default())
            });
            seat_state.text_input = text_input;
        }

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
                if let Some(pointer) = seat_state.pointer.take() {
                    self.flush_pointer_frame(&pointer);
                    if pointer.version() >= 3 {
                        pointer.release();
                    }
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
                state.update_active_output(&surface);
                if state.keyboard_focus.as_ref() == Some(&surface) {
                    log::warn!("wl_keyboard::enter ignoring duplicate call");
                } else {
                    let surface_id = state.get_id_from_surface(&surface);
                    state.keyboard_focus = Some(surface);
                    if let Some(id) = surface_id {
                        state
                            .messages
                            .push((Some(id), DispatchMessage::Focused(id)));
                    }
                }
                let Some(keyboard_state) = state.get_keyboard_state_mut(wl_keyboard) else {
                    return;
                };
                keyboard_state.current_repeat = None;
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Leave { surface, .. } => {
                state.keyboard_focus = None;
                let surface_id = state.get_id_from_surface(&surface);
                if surface_id.is_some() {
                    state.messages.push((
                        surface_id,
                        DispatchMessage::ModifiersChanged(ModifiersState::empty()),
                    ));
                    state.messages.push((surface_id, DispatchMessage::Unfocus));
                }
                let Some(keyboard_state) = state.get_keyboard_state_mut(wl_keyboard) else {
                    return;
                };
                keyboard_state.current_repeat = None;
                if let Some(token) = keyboard_state.repeat_token.take() {
                    state.to_remove_tokens.push(token);
                }
            }
            wl_keyboard::Event::Key {
                state: keystate,
                key,
                ..
            } => {
                let surface_id = state.keyboard_focus_id();
                let pressed_state = match keystate {
                    WEnum::Value(KeyState::Pressed) => ElementState::Pressed,
                    WEnum::Value(KeyState::Released) => ElementState::Released,
                    _ => {
                        return;
                    }
                };
                let key = key + 8;

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
                if let Some(mut key_context) = keyboard_state.xkb_context.key_context() {
                    let event = key_context.process_key_event(key, pressed_state, false);
                    let event = DispatchMessage::KeyboardInput {
                        event,
                        is_synthetic: false,
                    };
                    state.messages.push((surface_id, event));
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
                let Some(keyboard_state) = state.get_keyboard_state_mut(wl_keyboard) else {
                    return;
                };

                let xkb_context = &mut keyboard_state.xkb_context;
                let xkb_state = match xkb_context.state_mut() {
                    Some(state) => state,
                    None => return,
                };
                xkb_state.update_modifiers(mods_depressed, mods_latched, mods_locked, 0, 0, group);
                let modifiers = xkb_state.modifiers();

                state.messages.push((
                    state.keyboard_focus_id(),
                    DispatchMessage::ModifiersChanged(modifiers.into()),
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
                state.popup_grab_serial = Some(serial);
                state.finger_locations.insert(id, (x, y));
                let surface_id = state.get_id_from_surface(&surface);
                state
                    .active_surfaces
                    .insert(Some(id), (surface.clone(), surface_id));
                state.update_active_output(&surface);
                state.messages.push((
                    surface_id,
                    DispatchMessage::TouchDown {
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
                            .messages
                            .push((v.1, DispatchMessage::TouchCancel { id, x, y }));
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
                state.messages.push((
                    surface_id,
                    DispatchMessage::TouchUp {
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
                state
                    .messages
                    .push((surface_id, DispatchMessage::TouchMotion { time, id, x, y }));
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
            .unwrap_or_else(|| (None, None));

        // Events up to the next `frame` belong together, so they are held and emitted in order
        // once it arrives.
        match event {
            wl_pointer::Event::Frame => state.flush_pointer_frame(pointer),
            event if AxisFrame::is_axis_event(&event) => {
                state
                    .pointer_frame(pointer)
                    .accumulate_axis(surface_id, &event);
            }
            wl_pointer::Event::Button {
                state: btnstate,
                serial,
                button,
                time,
            } => {
                if matches!(btnstate, WEnum::Value(wl_pointer::ButtonState::Pressed)) {
                    state.popup_grab_serial = Some(serial);
                }
                if let Some(mouse_surface) = mouse_surface.cloned() {
                    state.update_active_output(&mouse_surface);
                }
                state.pointer_frame(pointer).push(
                    surface_id,
                    DispatchMessage::MouseButton {
                        state: btnstate,
                        serial,
                        button,
                        time,
                    },
                );
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
                    .pointer_frame(pointer)
                    .push(surface_id, DispatchMessage::MouseLeave);
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
                state.pointer_frame(pointer).push(
                    surface_id,
                    DispatchMessage::MouseEnter {
                        pointer: pointer.clone(),
                        serial,
                        surface_x,
                        surface_y,
                    },
                );
            }
            wl_pointer::Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                state.pointer_frame(pointer).push(
                    surface_id,
                    DispatchMessage::MouseMotion {
                        time,
                        surface_x,
                        surface_y,
                    },
                );
            }
            _ => {
                // TODO: not now
            }
        }
        // Before version 5 there is no `frame` event, so each event is a frame of its .
        if pointer.version() < 5 {
            state.flush_pointer_frame(pointer);
        }
    }
}

delegate_noop!(@<T: 'static> WindowState<T>: ignore WlSeat);
