//! Touchpad gestures and scroll details that iced events cannot carry.
//!
//! iced scroll events do not include the source device, timestamp, or stop notifications, and
//! iced has no touchpad gesture events. Wayland compositors report these, so widgets can
//! implement kinetic scrolling, pinch to zoom, or swipe navigation.
//!
//! Enable [`ExWlSettings::gestures`] to expose them:
//!
//! - [`current`] returns the [`Frame`] of the [`mouse::Event::WheelScrolled`] being handled.
//! - A widget keeps a [`GestureReceiver`] in its state and exposes it from `Widget::operate`
//!   with `operation.custom(None, layout.bounds(), &mut receiver)`. It receives:
//!   - the [`Stop`] and [`Hold`] for the last scroll it claimed via `GestureReceiver::claim`
//!   - a [`Swipe`] or [`Pinch`] that began over it, if it is the innermost receiver at that point.
//!
//! The runtime requests a redraw after delivering a gesture, so the widget can
//! [`take`](GestureReceiver::take) it while handling the next event.
//!
//! With this setting disabled, widgets receive scroll deltas only.
//!
//! [`ExWlSettings::gestures`]: crate::settings::ExWlSettings::gestures

use std::any::Any;
use std::cell::Cell;
use std::collections::VecDeque;
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};

use exwlshellev::AxisScroll;
use exwlshellev::PointerGesture;
use exwlshellev::reexport::wayland_client::wl_pointer;
use iced_core::widget::operation::Scrollable;
use iced_core::widget::{Id, Operation};
use iced_core::{Point, Rectangle, Vector, mouse};

/// The device that produced a scroll.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A mouse wheel
    Wheel,
    /// Fingers on a touchpad. Scrolling ends with a stop.
    Finger,
    /// Continuous movement without a fixed step like trackpoint or button scrolling.
    Continuous,
}

impl Source {
    fn from_wayland(source: wl_pointer::AxisSource) -> Option<Self> {
        match source {
            wl_pointer::AxisSource::Wheel => Some(Self::Wheel),
            wl_pointer::AxisSource::Finger => Some(Self::Finger),
            wl_pointer::AxisSource::Continuous => Some(Self::Continuous),
            _ => None,
        }
    }
}

/// A value per scroll axis.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Axes<T> {
    /// The horizontal axis.
    pub x: T,
    /// The vertical axis.
    pub y: T,
}

impl Axes<bool> {
    /// Whether the value is `true` on either axis.
    pub fn any(self) -> bool {
        self.x || self.y
    }
}

/// One logical scroll event as reported by the compositor.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Compositor timestamp in milliseconds. Only differences between timestamps are
    /// meaningful. `None` if the compositor did not send a timestamp for this frame.
    pub time: Option<u32>,
    /// The device that produced the scroll.
    pub source: Option<Source>,
    /// The axes whose scroll direction is inverted (natural scrolling).
    pub inverted: Axes<bool>,
}

impl Frame {
    pub(crate) fn new(
        time: Option<u32>,
        source: Option<wl_pointer::AxisSource>,
        horizontal: &AxisScroll,
        vertical: &AxisScroll,
    ) -> Self {
        let inverted = |axis: &AxisScroll| {
            axis.relative_direction == Some(wl_pointer::AxisRelativeDirection::Inverted)
        };
        Self {
            time,
            source: source.and_then(Source::from_wayland),
            inverted: Axes {
                x: inverted(horizontal),
                y: inverted(vertical),
            },
        }
    }
}

/// Returns a frame's line delta followed by its pixel delta, in logical units.
pub(crate) fn deltas(
    horizontal: &AxisScroll,
    vertical: &AxisScroll,
) -> [Option<mouse::ScrollDelta>; 2] {
    enum Amount {
        None,
        Lines(f32),
        Pixels(f32),
    }
    // NOTE: Wayland's sign convention is the inverse of iced's.
    let amount = |axis: &AxisScroll| {
        if axis.value120 != 0 {
            Amount::Lines(-axis.value120 as f32 / 120.0)
        } else if axis.discrete != 0 {
            Amount::Lines(-axis.discrete as f32)
        } else if axis.absolute != 0.0 {
            Amount::Pixels(-axis.absolute as f32)
        } else {
            Amount::None
        }
    };
    let (x, y) = (amount(horizontal), amount(vertical));
    let lines = |amount: &Amount| match amount {
        Amount::Lines(lines) => Some(*lines),
        _ => None,
    };
    let pixels = |amount: &Amount| match amount {
        Amount::Pixels(pixels) => Some(*pixels),
        _ => None,
    };
    let delta = |x: Option<f32>, y: Option<f32>, make: fn(f32, f32) -> mouse::ScrollDelta| {
        (x.is_some() || y.is_some()).then(|| make(x.unwrap_or(0.0), y.unwrap_or(0.0)))
    };
    [
        delta(lines(&x), lines(&y), |x, y| mouse::ScrollDelta::Lines {
            x,
            y,
        }),
        delta(pixels(&x), pixels(&y), |x, y| mouse::ScrollDelta::Pixels {
            x,
            y,
        }),
    ]
}

/// The end of a scroll gesture: the fingers were lifted off the device.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stop {
    pub time: Option<u32>,
    pub axes: Axes<bool>,
}

impl Stop {
    pub(crate) fn new(
        time: Option<u32>,
        horizontal: &AxisScroll,
        vertical: &AxisScroll,
    ) -> Option<Self> {
        let axes = Axes {
            x: horizontal.stop,
            y: vertical.stop,
        };
        axes.any().then_some(Self { time, axes })
    }
}

/// Fingers were put down on a touchpad and held there, e.g. to stop kinetic scrolling.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    pub time: u32,
    pub fingers: u32,
}

/// Fingers moving together on a touchpad. Times are compositor timestamps in milliseconds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Swipe {
    Begin {
        time: u32,
        fingers: u32,
    },
    /// The fingers moved by `delta`, in logical pixels, since the previous event.
    Update {
        time: u32,
        delta: Vector,
    },
    /// The fingers were lifted, or the compositor `cancelled` the swipe.
    End {
        time: u32,
        cancelled: bool,
    },
}

/// Fingers moving towards or away from each other, or rotating, on a touchpad. Times are
/// compositor timestamps in milliseconds.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pinch {
    Begin {
        time: u32,
        fingers: u32,
    },
    /// The center of the fingers moved by `delta`, in logical pixels, since the previous event.
    /// `scale` is the distance between the fingers relative to the start of the gesture.
    /// `rotation` is the clockwise angle in degrees since the previous event.
    Update {
        time: u32,
        delta: Vector,
        scale: f32,
        rotation: f32,
    },
    /// The fingers were lifted, or the compositor `cancelled` the pinch.
    End {
        time: u32,
        cancelled: bool,
    },
}

/// A touchpad gesture delivered to a [`GestureReceiver`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// The claimed scroll stopped.
    Stop(Stop),
    /// Fingers were put down during or after the claimed scroll.
    Hold(Hold),
    /// A swipe that began over the receiver.
    Swipe(Swipe),
    /// A pinch that began over the receiver.
    Pinch(Pinch),
}

impl Gesture {
    pub(crate) fn from_pointer(gesture: PointerGesture, scale_factor: f64) -> Self {
        let delta =
            |dx: f64, dy: f64| Vector::new((dx / scale_factor) as f32, (dy / scale_factor) as f32);
        match gesture {
            PointerGesture::HoldBegin { time, fingers } => Self::Hold(Hold { time, fingers }),
            PointerGesture::SwipeBegin { time, fingers } => {
                Self::Swipe(Swipe::Begin { time, fingers })
            }
            PointerGesture::SwipeUpdate { time, dx, dy } => Self::Swipe(Swipe::Update {
                time,
                delta: delta(dx, dy),
            }),
            PointerGesture::SwipeEnd { time, cancelled } => {
                Self::Swipe(Swipe::End { time, cancelled })
            }
            PointerGesture::PinchBegin { time, fingers } => {
                Self::Pinch(Pinch::Begin { time, fingers })
            }
            PointerGesture::PinchUpdate {
                time,
                dx,
                dy,
                scale,
                rotation,
            } => Self::Pinch(Pinch::Update {
                time,
                delta: delta(dx, dy),
                scale: scale as f32,
                rotation: rotation as f32,
            }),
            PointerGesture::PinchEnd { time, cancelled } => {
                Self::Pinch(Pinch::End { time, cancelled })
            }
        }
    }
}

/// Receives list of [`Gesture`] widget owns.
#[derive(Debug)]
pub struct GestureReceiver {
    owner: Owner,
    pending: VecDeque<Gesture>,
}

impl GestureReceiver {
    /// Creates a receiver with no pending gesture.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self {
            owner: Owner(NEXT.fetch_add(1, Ordering::Relaxed)),
            pending: VecDeque::new(),
        }
    }

    /// Claims the scroll of the [`mouse::Event::WheelScrolled`] being handled, so its
    /// [`Stop`] and any [`Hold`] come to this receiver.
    pub fn claim(&self) -> bool {
        let claimed = CURRENT.get().is_some();
        if claimed {
            CLAIM.set(Some(self.owner));
        }
        claimed
    }

    /// Takes the oldest pending gesture.
    pub fn take(&mut self) -> Option<Gesture> {
        self.pending.pop_front()
    }
}

impl Default for GestureReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifies the [`GestureReceiver`] that owns a gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Owner(u64);

/// Delivers a [`Gesture`] to the [`GestureReceiver`] that owns it.
struct DeliverGesture {
    owner: Owner,
    gesture: Gesture,
    delivered: bool,
}

impl DeliverGesture {
    fn new(owner: Owner, gesture: Gesture) -> Self {
        Self {
            owner,
            gesture,
            delivered: false,
        }
    }

    /// Whether the owner was found.
    fn delivered(&self) -> bool {
        self.delivered
    }
}

impl Operation for DeliverGesture {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if !self.delivered {
            operate(self);
        }
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if let Some(receiver) = state.downcast_mut::<GestureReceiver>()
            && receiver.owner == self.owner
        {
            receiver.pending.push_back(self.gesture);
            self.delivered = true;
        }
    }
}

/// Finds the innermost visible [`GestureReceiver`] at a point.
struct FindReceiver {
    point: Point,
    found: Option<Owner>,
    scrollable: Option<(bool, Vector)>,
}

impl FindReceiver {
    fn new(point: Point) -> Self {
        Self {
            point,
            found: None,
            scrollable: None,
        }
    }
}

impl Operation for FindReceiver {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        match self.scrollable.take() {
            None => operate(self),
            // Skip the content when the point is outside the scroll viewport.
            Some((false, _)) => {}
            // The layout doesn't include the scroll offset, so adjust the point to match.
            Some((true, translation)) => {
                let point = self.point;
                self.point = point + translation;
                operate(self);
                self.point = point;
            }
        }
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.scrollable = Some((bounds.contains(self.point), translation));
    }

    fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Any) {
        // Children are visited after their parents, so the last match is the innermost.
        if let Some(receiver) = state.downcast_mut::<GestureReceiver>()
            && bounds.contains(self.point)
        {
            self.found = Some(receiver.owner);
        }
    }
}

/// The receivers that own a window's gestures.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Owners {
    /// Claimed the latest scroll event.
    pub(crate) scroll: Option<Owner>,
    /// Was under the cursor when the current swipe or pinch began.
    pointer: Option<Owner>,
}

impl Owners {
    /// Delivers `gesture` to its owner through `operate`, and returns whether it was received.
    /// `cursor` is the cursor position when `gesture` arrived.
    pub(crate) fn deliver(
        &mut self,
        gesture: Gesture,
        cursor: Option<Point>,
        mut operate: impl FnMut(&mut dyn Operation),
    ) -> bool {
        let owner = match gesture {
            Gesture::Stop(_) | Gesture::Hold(_) => self.scroll,
            Gesture::Swipe(Swipe::Begin { .. }) | Gesture::Pinch(Pinch::Begin { .. }) => {
                self.pointer = cursor.and_then(|point| {
                    let mut find = FindReceiver::new(point);
                    operate(&mut find);
                    find.found
                });
                self.pointer
            }
            Gesture::Swipe(Swipe::End { .. }) | Gesture::Pinch(Pinch::End { .. }) => {
                mem::take(&mut self.pointer)
            }
            Gesture::Swipe(_) | Gesture::Pinch(_) => self.pointer,
        };
        let Some(owner) = owner else {
            return false;
        };
        let mut deliver = DeliverGesture::new(owner, gesture);
        operate(&mut deliver);
        deliver.delivered()
    }
}

thread_local! {
    static CURRENT: Cell<Option<Frame>> = const { Cell::new(None) };
    static CLAIM: Cell<Option<Owner>> = const { Cell::new(None) };
}

/// The [`Frame`] of the [`mouse::Event::WheelScrolled`] being handled right now.
pub fn current() -> Option<Frame> {
    CURRENT.get()
}

/// Runs `f` with [`current`] returning `frame`. Also returns the owner that claimed the frame.
pub(crate) fn with_current<R>(frame: Option<Frame>, f: impl FnOnce() -> R) -> (R, Option<Owner>) {
    struct Restore(Option<Frame>, Option<Owner>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.set(self.0);
            CLAIM.set(self.1);
        }
    }
    let restore = Restore(CURRENT.replace(frame), CLAIM.replace(None));
    let result = f();
    let claim = CLAIM.get();
    drop(restore);
    (result, claim)
}
