//! Scroll details that iced's [`mouse::Event::WheelScrolled`] cannot carry.
//!
//! iced scroll events do not include the source device, timestamp, or stop notifications.
//! Wayland compositors report these details so widgets can implement kinetic scrolling,
//! where content keeps moving after the user lifts their fingers.
//!
//! With this setting disabled, widgets receive scroll deltas only.
//!
//! [`ExWlSettings::scroll_frames`]: crate::settings::ExWlSettings::scroll_frames

use std::any::Any;
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use exwlshellev::AxisScroll;
use exwlshellev::reexport::wayland_client::wl_pointer;
use iced_core::widget::{Id, Operation};
use iced_core::{Rectangle, mouse};

/// The device that produced a scroll.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A mouse wheel: discrete steps.
    Wheel,
    /// Fingers on a touch surface such as a touchpad. Scrolling ends with a stop.
    Finger,
    /// Continuous movement without a fixed step, e.g. a trackpoint or button scrolling.
    Continuous,
    /// A sideways tilt of a mouse wheel.
    WheelTilt,
}

impl Source {
    fn from_wayland(source: wl_pointer::AxisSource) -> Option<Self> {
        match source {
            wl_pointer::AxisSource::Wheel => Some(Self::Wheel),
            wl_pointer::AxisSource::Finger => Some(Self::Finger),
            wl_pointer::AxisSource::Continuous => Some(Self::Continuous),
            wl_pointer::AxisSource::WheelTilt => Some(Self::WheelTilt),
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
    /// The device that produced the scroll
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

    fn merge(self, later: Self) -> Self {
        Self {
            time: later.time.or(self.time),
            axes: Axes {
                x: self.axes.x || later.axes.x,
                y: self.axes.y || later.axes.y,
            },
        }
    }
}

/// Receives the [`Stop`] of scroll gestures its widget owns.
#[derive(Debug)]
pub struct StopReceiver {
    owner: Owner,
    stop: Option<Stop>,
}

impl StopReceiver {
    /// Creates a receiver with no pending stop.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self {
            owner: Owner(NEXT.fetch_add(1, Ordering::Relaxed)),
            stop: None,
        }
    }

    /// Claims the gesture of the [`mouse::Event::WheelScrolled`] being handled.
    pub fn claim(&self) -> bool {
        let claimed = CURRENT.get().is_some();
        if claimed {
            CLAIM.set(Some(self.owner));
        }
        claimed
    }

    /// Takes the stop delivered since the last call. Stops delivered in between are merged.
    pub fn take_stop(&mut self) -> Option<Stop> {
        self.stop.take()
    }
}

impl Default for StopReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifies the [`StopReceiver`] that owns a gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Owner(u64);

/// Delivers a [`Stop`] to the [`StopReceiver`] of its owner.
pub(crate) struct DeliverStop {
    owner: Owner,
    stop: Stop,
    delivered: bool,
}

impl DeliverStop {
    pub(crate) fn new(owner: Owner, stop: Stop) -> Self {
        Self {
            owner,
            stop,
            delivered: false,
        }
    }

    /// Whether the owner was found.
    pub(crate) fn delivered(&self) -> bool {
        self.delivered
    }
}

impl Operation for DeliverStop {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if !self.delivered {
            operate(self);
        }
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if let Some(receiver) = state.downcast_mut::<StopReceiver>()
            && receiver.owner == self.owner
        {
            receiver.stop = Some(match receiver.stop {
                Some(earlier) => earlier.merge(self.stop),
                None => self.stop,
            });
            self.delivered = true;
        }
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
