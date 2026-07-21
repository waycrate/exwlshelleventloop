use futures::channel::mpsc;
use iced_core::{Point, Size, window::Id};
use iced_futures::Subscription;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// output where surface is displayed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputInfo {
    /// output name, like `HDMI-A-1`
    pub name: String,
    /// human readable output description
    pub description: String,
    /// global output pos
    pub logical_position: Point<i32>,
    /// output size
    pub logical_size: Size<i32>,
}

impl From<layershellev::OutputInfo> for OutputInfo {
    fn from(info: layershellev::OutputInfo) -> Self {
        Self {
            name: info.name.unwrap_or_default(),
            description: info.description.unwrap_or_default(),
            logical_position: info.logical_position.map(Point::from).unwrap_or_default(),
            logical_size: info.logical_size.map(Size::from).unwrap_or_default(),
        }
    }
}

/// window started being displayed on a different output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    /// moved window id
    pub window: Id,
    /// new output
    pub output: Option<OutputInfo>,
}

#[derive(Default)]
struct Registry {
    subscribers: Vec<mpsc::UnboundedSender<OutputEvent>>,
    /// last known output per window, replayed to new subscribers
    last: HashMap<Id, Option<OutputInfo>>,
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

/// deliver to every subscriber, dropping those whose receiver has gone away
pub(crate) fn broadcast(event: OutputEvent) {
    let Ok(mut registry) = registry().lock() else {
        return;
    };
    registry.last.insert(event.window, event.output.clone());
    registry
        .subscribers
        .retain(|sender| sender.unbounded_send(event.clone()).is_ok());
}

/// forget a closed window so it is not replayed to new subscribers
pub(crate) fn forget(window: Id) {
    if let Ok(mut registry) = registry().lock() {
        registry.last.remove(&window);
    }
}

/// listen for the output each window is displayed on
pub fn listen() -> Subscription<OutputEvent> {
    Subscription::run(|| {
        let (sender, receiver) = mpsc::unbounded();
        if let Ok(mut registry) = registry().lock() {
            for (window, output) in &registry.last {
                let _ = sender.unbounded_send(OutputEvent {
                    window: *window,
                    output: output.clone(),
                });
            }
            registry.subscribers.push(sender);
        }
        receiver
    })
}
