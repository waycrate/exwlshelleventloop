use sctk::{
    delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle, delegate_noop,
    globals::GlobalList,
    protocol::{wl_callback::WlCallback, wl_output::WlOutput},
};

use crate::Error;
use crate::info::OutputInfo;
use crate::worker::{self, Worker};

#[derive(Debug)]
pub enum OutputEvent {
    Insert(OutputInfo),
    Changed(OutputInfo),
    Removed(OutputInfo),
    Stop(Error),
}

#[derive(Debug)]
pub(crate) struct Outputs {
    registry_state: RegistryState,
    output_state: OutputState,
    events: Vec<OutputEvent>,
}

impl Outputs {
    fn push(&mut self, make: fn(OutputInfo) -> OutputEvent, output: WlOutput) {
        if let Some(inner) = self.output_state.info(&output) {
            self.events.push(make(inner));
        }
    }
}

/// Release every output this state bound; a restarted subscription would otherwise accumulate them
pub(crate) fn release_outputs(output_state: &OutputState) {
    for output in output_state.outputs() {
        if output.version() >= 3 {
            output.release();
        }
    }
}

impl Worker for Outputs {
    type Event = OutputEvent;

    fn init(_: &Connection, globals: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self, Error> {
        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, qh),
            events: Vec::new(),
        })
    }

    fn take_events(&mut self) -> Vec<OutputEvent> {
        std::mem::take(&mut self.events)
    }

    fn reset_events(&mut self) -> Vec<OutputEvent> {
        self.output_state
            .outputs()
            .filter_map(|output| self.output_state.info(&output).map(OutputEvent::Removed))
            .collect()
    }

    fn stop_event(error: Error) -> OutputEvent {
        OutputEvent::Stop(error)
    }

    fn teardown(&mut self, _: &mut EventQueue<Self>) {
        release_outputs(&self.output_state);
    }
}

impl OutputHandler for Outputs {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.push(OutputEvent::Insert, output);
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.push(OutputEvent::Changed, output);
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.push(OutputEvent::Removed, output);
    }
}

impl ProvidesRegistryState for Outputs {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(Outputs);
delegate_output!(Outputs);
delegate_noop!(Outputs: ignore WlCallback);

/// Watch the compositor's outputs
pub fn listen(connection: Connection) -> iced_futures::Subscription<OutputEvent> {
    worker::listen::<Outputs>(connection)
}
