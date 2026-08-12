use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures::channel::mpsc::{UnboundedSender, unbounded};
use futures::{SinkExt, StreamExt};
use iced_futures::Subscription;
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use wayland_client::{
    Connection, Dispatch, DispatchError, EventQueue, QueueHandle,
    backend::WaylandError,
    globals::{GlobalError, GlobalList, GlobalListContents, registry_queue_init},
    protocol::{wl_callback, wl_registry},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("registry error: {0}")]
    RegistryErr(#[from] GlobalError),
    #[error("dispatch error: {0}")]
    DispatchErr(#[from] DispatchError),
    /// A `Dispatch` impl panicked on the worker thread.
    #[error("worker thread panicked")]
    Panicked,
    /// The worker thread could not be started.
    #[error("could not spawn the wayland subscriber thread: {0}")]
    Spawn(#[from] std::io::Error),
}

impl Error {
    /// Whether the connection this came from is now unusable.
    pub fn is_fatal(&self) -> bool {
        let backend = match self {
            Error::RegistryErr(GlobalError::Backend(err))
            | Error::DispatchErr(DispatchError::Backend(err)) => err,
            Error::Spawn(_) => return true,
            Error::RegistryErr(GlobalError::InvalidId(_))
            | Error::DispatchErr(DispatchError::BadMessage { .. })
            | Error::Panicked => return false,
        };
        !matches!(backend, WaylandError::Io(err) if err.kind() == std::io::ErrorKind::WouldBlock)
    }
}

/// A dispatch state the worker can drive.
pub(crate) trait Worker: Sized + Send + 'static
where
    Self: Dispatch<wl_callback::WlCallback, ()>
        + Dispatch<wl_registry::WlRegistry, GlobalListContents>,
{
    /// What this worker hands to the application.
    type Event: Send + 'static;

    fn disposition(_event: &Self::Event) -> Disposition {
        Disposition::Incremental
    }

    /// Build the initial state
    fn init(conn: &Connection, globals: &GlobalList, qh: &QueueHandle<Self>)
    -> Result<Self, Error>;

    /// Drain everything queued for the app since last call.
    fn take_events(&mut self) -> Vec<Self::Event>;

    /// Only the worker knows how to retract what it announced.
    fn reset_events(&mut self) -> Vec<Self::Event> {
        Vec::new()
    }

    /// Build the event reporting a failure, so a protocol error does not look
    /// identical to the compositor simply going quiet.
    fn stop_event(error: Error) -> Self::Event;

    /// Release protocol objects. Called once, after the loop exits.
    fn teardown(&mut self, queue: &mut EventQueue<Self>);
}

/// What an event means for the worker driving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    /// Carries a change; earlier events still matter.
    Incremental,
    /// Carries whole state, so anything queued before it is stale.
    #[cfg_attr(not(feature = "workspace"), allow(dead_code))]
    Supersedes,
    /// The protocol is over, deliver it, then stop the worker.
    #[cfg_attr(not(feature = "workspace"), allow(dead_code))]
    Terminal,
}

impl Disposition {
    fn supersedes(self) -> bool {
        matches!(self, Self::Supersedes | Self::Terminal)
    }
}

/// The worker's queue handle, once it has one.
type Handoff<S> = Arc<Mutex<Option<QueueHandle<S>>>>;

fn lock<S>(handoff: &Handoff<S>) -> MutexGuard<'_, Option<QueueHandle<S>>> {
    handoff.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Wakes the worker when the subscription drops, so its protocol objects are
/// destroyed, not leaked.
pub(crate) struct ShutdownGuard<S: Worker> {
    conn: Connection,
    stop: Arc<AtomicBool>,
    handoff: Handoff<S>,
}

impl<S: Worker> Drop for ShutdownGuard<S> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let handoff = lock(&self.handoff);
        let Some(qh) = handoff.as_ref() else { return };
        // A sync callback on the worker's queue makes blocking_dispatch return.
        self.conn.display().sync(qh, ());
        drop(handoff);
        let Err(WaylandError::Io(err)) = self.conn.flush() else {
            return;
        };
        if err.kind() != std::io::ErrorKind::WouldBlock {
            return;
        }
        // Copied logic from `Connection::blocking_read`, mirrored for writes:
        // poll until writable, then flush(`wl_display_flush`).
        let conn = self.conn.clone();
        let _ = std::thread::Builder::new()
            .name("iced-wayland-subscriber-flush".into())
            .spawn(move || {
                let fd = conn.as_fd();
                let mut fds = [PollFd::new(&fd, PollFlags::OUT)];
                // Only `EINTR` is worth retrying; anything else is an answer.
                while let Err(Errno::INTR) = rustix::event::poll(&mut fds, None) {}
                let _ = conn.flush();
            });
    }
}

/// How many times a worker is rebuilt after a failure it might survive.
const MAX_RESTARTS: usize = 5;

/// Grows with each attempt.
fn restart_delay(attempt: usize) -> Duration {
    Duration::from_millis(50 << attempt.min(4))
}

/// What ended an attempt.
enum Ended {
    /// Asked to stop, or the application dropped the receiver. Do not restart.
    Done,
    /// Failed. Report it and rebuild, unless it is fatal or attempts ran out.
    Failed(Error),
}

/// The worker thread: rebuild the state on failure, up to a point.
fn run<S: Worker>(
    connection: Connection,
    tx: UnboundedSender<S::Event>,
    stop: Arc<AtomicBool>,
    handoff: Handoff<S>,
) {
    // Loop instead of delegating to the client as it would create extra complexity.
    // It shouldn't be a problem, but if related issue created, keep this as the default
    // and expose for manual management
    for attempt in 0..=MAX_RESTARTS {
        match attempt_once::<S>(&connection, &tx, &stop, &handoff) {
            Ended::Done => return,
            Ended::Failed(error) => {
                if stop.load(Ordering::Acquire) || tx.is_closed() {
                    return;
                }
                if error.is_fatal() || attempt == MAX_RESTARTS {
                    let _ = tx.unbounded_send(S::stop_event(error));
                    return;
                }
                std::thread::sleep(restart_delay(attempt));
                if stop.load(Ordering::Acquire) || tx.is_closed() {
                    return;
                }
            }
        }
    }
}

/// One init/dispatch/teardown cycle.
fn attempt_once<S: Worker>(
    connection: &Connection,
    tx: &UnboundedSender<S::Event>,
    stop: &Arc<AtomicBool>,
    handoff: &Handoff<S>,
) -> Ended {
    // Before the init roundtrips, which are the one place this thread can
    // block without the guard being able to wake it.
    if stop.load(Ordering::Acquire) {
        return Ended::Done;
    }
    let started = (|| {
        let (globals, queue) = registry_queue_init::<S>(connection)?;
        let qh = queue.handle();
        let state = S::init(connection, &globals, &qh)?;
        Ok::<_, Error>((queue, qh, state))
    })();

    let (mut queue, qh, mut state) = match started {
        Ok((queue, qh, state)) => (queue, qh, state),
        Err(error) => return Ended::Failed(error),
    };
    {
        // Publish the handle so the guard can wake us. If it has already run,
        // nothing will, and `blocking_dispatch` parks forever.
        let mut handoff = lock(handoff);
        if stop.load(Ordering::Acquire) {
            drop(handoff);
            state.teardown(&mut queue);
            let _ = queue.flush();
            return Ended::Done;
        }
        *handoff = Some(qh);
    }

    // Same shape as winit wrapping its event handler
    let dispatched = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        loop {
            if stop.load(Ordering::Acquire) {
                return Ended::Done;
            }
            if let Err(error) = queue.blocking_dispatch(&mut state) {
                for event in state.take_events() {
                    let _ = tx.unbounded_send(event);
                }
                return Ended::Failed(error.into());
            }
            for event in state.take_events() {
                let terminal = S::disposition(&event) == Disposition::Terminal;
                if tx.unbounded_send(event).is_err() || terminal {
                    return Ended::Done;
                }
            }
        }
    }));
    let ended = match dispatched {
        Ok(ended) => ended,
        Err(_) => {
            for event in state.take_events() {
                let _ = tx.unbounded_send(event);
            }
            Ended::Failed(Error::Panicked)
        }
    };

    if matches!(ended, Ended::Failed(_)) {
        let reset = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| state.reset_events()));
        for event in reset.unwrap_or_default() {
            let _ = tx.unbounded_send(event);
        }
    }

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| state.teardown(&mut queue)));
    let _ = queue.flush();
    *lock(handoff) = None;
    ended
}

/// Run `S` as a subscription on its own event queue of `connection`.
pub(crate) fn listen<S: Worker>(connection: Connection) -> Subscription<S::Event> {
    let connection: crate::HashConnection = connection.into();
    Subscription::run_with(connection, |conn| {
        let conn = conn.clone();
        iced_futures::stream::channel(
            100,
            |mut output: futures::channel::mpsc::Sender<S::Event>| async move {
                let connection = conn.into_inner();
                let (tx, mut rx) = unbounded();
                let stop = Arc::new(AtomicBool::new(false));
                let handoff: Handoff<S> = Arc::new(Mutex::new(None));
                let _guard = ShutdownGuard::<S> {
                    conn: connection.clone(),
                    stop: stop.clone(),
                    handoff: handoff.clone(),
                };
                // Report a failed spawn the same way every other failure is
                // reported, rather than panicking on the executor thread.
                if let Err(error) = std::thread::Builder::new()
                    .name("iced-wayland-subscriber".into())
                    .spawn(move || run::<S>(connection, tx, stop, handoff))
                {
                    let _ = output.send(S::stop_event(error.into())).await;
                    return;
                }
                while let Some(mut event) = rx.next().await {
                    while let Ok(newer) = rx.try_recv() {
                        if !S::disposition(&newer).supersedes() && output.send(event).await.is_err()
                        {
                            return;
                        }
                        event = newer;
                    }
                    if output.send(event).await.is_err() {
                        break;
                    }
                }
            },
        )
    })
}
