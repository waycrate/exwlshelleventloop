use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use futures::channel::mpsc;
use iced_core::window::Id;
use iced_futures::Subscription;

pub use crate::info::OutputInfo;

/// What kind of surface a window is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    LayerShell,
    PopUp,
    XdgTopLevel,
    InputPanel,
    SessionLock,
}

/// A surface the runtime created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellInfo {
    pub window: Id,
    pub shell: ShellType,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ShellEvent {
    /// Surface was created.
    NewShell(ShellInfo),
    /// Window was closed.
    Closed(Id),
    /// Window changed output.
    WindowOutputChanged {
        window: Id,
        output: Option<OutputInfo>,
    },
    /// monitor was connected.
    OutputAdded(OutputInfo),
    /// monitor mode, scale, name or position changed.
    OutputUpdated(OutputInfo),
    /// monitor was disconnected.
    OutputRemoved(OutputInfo),
}

#[derive(Default)]
struct Registry {
    subscribers: Vec<mpsc::UnboundedSender<ShellEvent>>,
    /// last known output per window, replayed to new subscribers
    outputs: BTreeMap<Id, Option<OutputInfo>>,
    /// shells still alive, replayed to new subscribers
    shells: BTreeMap<Id, ShellType>,
    /// monitors still connected, keyed by `wl_registry` global name and
    /// replayed to new subscribers
    monitors: BTreeMap<u32, OutputInfo>,
}

/// Create the two ends of a shell broadcast.
pub fn channel() -> (ShellSender, ShellReceiver) {
    let registry = Arc::new(Mutex::new(Registry::default()));
    (ShellSender(registry.clone()), ShellReceiver(registry))
}

/// The runtime's end announces the surfaces it creates.
#[derive(Debug)]
pub struct ShellSender(Arc<Mutex<Registry>>);

/// The app end [`listen`](Self::listen) for what the runtime creates.
#[derive(Debug, Clone)]
pub struct ShellReceiver(Arc<Mutex<Registry>>);

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("subscribers", &self.subscribers.len())
            .field("outputs", &self.outputs.len())
            .field("shells", &self.shells.len())
            .finish()
    }
}

fn lock(registry: &Mutex<Registry>) -> MutexGuard<'_, Registry> {
    // The data is a plain map, valid after any panic.
    // Poisoning would only silently kill the broadcast forever.
    registry.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ShellSender {
    /// Record a surface the runtime created, or a window changing output.
    pub fn send(&self, event: ShellEvent) {
        publish(&mut lock(&self.0), event);
    }

    /// Report a closed window.
    pub fn forget(&self, window: Id) {
        let mut registry = lock(&self.0);
        if registry.shells.contains_key(&window) {
            publish(&mut registry, ShellEvent::Closed(window));
        }
    }
}

impl PartialEq for ShellReceiver {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ShellReceiver {}

impl Hash for ShellReceiver {
    /// Identity, so `listen` is one subscription per channel.
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

impl ShellReceiver {
    /// Listen for surfaces the runtime creates and the outputs they are shown on.
    pub fn listen(&self) -> Subscription<ShellEvent> {
        Subscription::run_with(self.clone(), |receiver| {
            let (sender, receiver_stream) = mpsc::unbounded();
            let mut registry = lock(&receiver.0);
            for monitor in registry.monitors.values() {
                let _ = sender.unbounded_send(ShellEvent::OutputAdded(monitor.clone()));
            }
            for (window, shell) in &registry.shells {
                let _ = sender.unbounded_send(ShellEvent::NewShell(ShellInfo {
                    window: *window,
                    shell: *shell,
                }));
            }
            for (window, output) in &registry.outputs {
                let _ = sender.unbounded_send(ShellEvent::WindowOutputChanged {
                    window: *window,
                    output: output.clone(),
                });
            }
            registry
                .subscribers
                .retain(|subscriber| !subscriber.is_closed());
            registry.subscribers.push(sender);
            receiver_stream
        })
    }
}

/// Record `event` and fan it out, dropping subscribers whose receiver is gone.
fn publish(registry: &mut Registry, event: ShellEvent) {
    match &event {
        ShellEvent::NewShell(info) => {
            registry.shells.insert(info.window, info.shell);
        }
        ShellEvent::Closed(window) => {
            registry.outputs.remove(window);
            registry.shells.remove(window);
        }
        ShellEvent::WindowOutputChanged { window, output } => {
            if registry.shells.contains_key(window) {
                registry.outputs.insert(*window, output.clone());
            }
        }
        ShellEvent::OutputAdded(info) | ShellEvent::OutputUpdated(info) => {
            registry.monitors.insert(info.id, info.clone());
            for shown_on in registry.outputs.values_mut() {
                if shown_on.as_ref().is_some_and(|known| known.id == info.id) {
                    *shown_on = Some(info.clone());
                }
            }
        }
        ShellEvent::OutputRemoved(info) => {
            registry.monitors.remove(&info.id);
        }
    }
    registry
        .subscribers
        .retain(|sender| sender.unbounded_send(event.clone()).is_ok());
}
