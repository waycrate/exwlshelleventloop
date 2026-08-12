use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use sctk::{
    delegate_dispatch2, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    backend::{ObjectId, WaylandError},
    delegate_noop, event_created_child,
    globals::GlobalList,
    protocol::{wl_callback::WlCallback, wl_output::WlOutput},
};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};

use crate::Error;
use crate::info::{OutputId, OutputInfo};
use crate::worker::{self, Disposition, Worker};

pub use ext_workspace_group_handle_v1::GroupCapabilities;
pub use ext_workspace_handle_v1::{State, WorkspaceCapabilities};

/// Identity of a workspace, stable for the lifetime of the protocol object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(ObjectId);

/// Identity of a workspace group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupId(ObjectId);

#[derive(Debug, Clone)]
pub struct WorkspaceGroup {
    pub id: GroupId,
    /// The outputs this group is displayed on, resolved at the manager `done`
    /// event so carry full info
    pub outputs: Vec<OutputInfo>,
    pub capabilities: GroupCapabilities,
    handle: ExtWorkspaceGroupHandleV1,
    /// Every `wl_output` the compositor named for this group.
    bound_outputs: Vec<WlOutput>,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub protocol_id: Option<String>,
    pub name: String,
    pub coordinates: Vec<u32>,
    pub state: State,
    pub capabilities: WorkspaceCapabilities,
    pub group: Option<GroupId>,
    handle: ExtWorkspaceHandleV1,
}

/// The workspace tree as of the last manager `done` event.
#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    pub groups: Vec<WorkspaceGroup>,
    pub workspaces: Vec<Workspace>,
    manager: Option<ExtWorkspaceManagerV1>,
    conn: Connection,
    /// Blocks application requests once the protocol is over
    stopped: Arc<Mutex<bool>>,
    /// Set the instant the protocol ends, ahead of `stopped`, so a request
    /// waiting on that lock cannot fire at an already-destroyed manager.
    dead: Arc<AtomicBool>,
}

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    /// The compositor did not advertise the capability for this request
    #[error("the compositor does not support this request")]
    Unsupported,
    /// No such workspace or group in this snapshot, or the manager is gone.
    #[error("no such workspace or group")]
    Gone,
    #[error("failed to flush the connection")]
    Io(#[from] wayland_client::backend::WaylandError),
}

impl WorkspaceSnapshot {
    pub fn workspace(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|workspace| &workspace.id == id)
    }

    pub fn group(&self, id: &GroupId) -> Option<&WorkspaceGroup> {
        self.groups.iter().find(|group| &group.id == id)
    }

    /// Workspaces currently assigned to `group`
    pub fn workspaces_in(&self, group: &GroupId) -> impl Iterator<Item = &Workspace> {
        self.workspaces
            .iter()
            .filter(move |workspace| workspace.group.as_ref() == Some(group))
    }

    /// The active workspace of `group`
    pub fn active_in(&self, group: &GroupId) -> Option<&Workspace> {
        self.workspaces_in(group)
            .find(|workspace| workspace.state.contains(State::Active))
    }

    /// The group `output` belongs to
    pub fn group_for_output(&self, output: OutputId) -> Option<&WorkspaceGroup> {
        self.groups.iter().find(|group| {
            group
                .outputs
                .iter()
                .any(|known| OutputId::from(known) == output)
        })
    }

    fn live_workspace(&self, id: &WorkspaceId) -> Result<&Workspace, RequestError> {
        self.workspace(id)
            .filter(|workspace| workspace.handle.is_alive())
            .ok_or(RequestError::Gone)
    }

    fn live_group(&self, id: &GroupId) -> Result<&WorkspaceGroup, RequestError> {
        self.group(id)
            .filter(|group| group.handle.is_alive())
            .ok_or(RequestError::Gone)
    }
}

impl WorkspaceSnapshot {
    /// Send `request` and the `commit` that makes the compositor apply it
    fn send(&self, alive: impl Fn() -> bool, request: impl FnOnce()) -> Result<(), RequestError> {
        if self.dead.load(Ordering::Acquire) {
            return Err(RequestError::Gone);
        }
        let stopped = self.stopped.lock().unwrap_or_else(PoisonError::into_inner);
        if *stopped || self.dead.load(Ordering::Acquire) {
            return Err(RequestError::Gone);
        }
        let manager = self
            .manager
            .as_ref()
            .filter(|manager| manager.is_alive())
            .ok_or(RequestError::Gone)?;
        if !alive() {
            return Err(RequestError::Gone);
        }

        request();
        manager.commit();

        match self.conn.flush() {
            Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(error) => Err(error.into()),
            Ok(()) => Ok(()),
        }
    }

    /// Switch to `id` - workspace-dispatch entry point
    pub fn activate(&self, id: &WorkspaceId) -> Result<(), RequestError> {
        let workspace = self.live_workspace(id)?;
        if !workspace
            .capabilities
            .contains(WorkspaceCapabilities::Activate)
        {
            return Err(RequestError::Unsupported);
        }
        self.send(
            || workspace.handle.is_alive(),
            || workspace.handle.activate(),
        )
    }

    pub fn deactivate(&self, id: &WorkspaceId) -> Result<(), RequestError> {
        let workspace = self.live_workspace(id)?;
        if !workspace
            .capabilities
            .contains(WorkspaceCapabilities::Deactivate)
        {
            return Err(RequestError::Unsupported);
        }
        self.send(
            || workspace.handle.is_alive(),
            || workspace.handle.deactivate(),
        )
    }

    pub fn remove(&self, id: &WorkspaceId) -> Result<(), RequestError> {
        let workspace = self.live_workspace(id)?;
        if !workspace
            .capabilities
            .contains(WorkspaceCapabilities::Remove)
        {
            return Err(RequestError::Unsupported);
        }
        self.send(|| workspace.handle.is_alive(), || workspace.handle.remove())
    }

    /// Move a `workspace` to another group
    pub fn assign(&self, workspace: &WorkspaceId, group: &GroupId) -> Result<(), RequestError> {
        let workspace = self.live_workspace(workspace)?;
        let group = self.live_group(group)?;
        if !workspace
            .capabilities
            .contains(WorkspaceCapabilities::Assign)
        {
            return Err(RequestError::Unsupported);
        }
        self.send(
            || workspace.handle.is_alive() && group.handle.is_alive(),
            || workspace.handle.assign(&group.handle),
        )
    }

    pub fn create_workspace(&self, group: &GroupId, name: &str) -> Result<(), RequestError> {
        let group = self.live_group(group)?;
        if !group
            .capabilities
            .contains(GroupCapabilities::CreateWorkspace)
        {
            return Err(RequestError::Unsupported);
        }
        self.send(
            || group.handle.is_alive(),
            || group.handle.create_workspace(name.to_owned()),
        )
    }
}

/// Coordinates arrive as raw byte array
fn decode_coordinates(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()))
        .collect()
}

// The generated `TryFrom` is `from_bits`, one unknown bit rejects value. Truncate instead.
fn flags<T>(value: WEnum<T>, from_bits_truncate: fn(u32) -> T) -> T {
    match value {
        WEnum::Value(flags) => flags,
        WEnum::Unknown(raw) => from_bits_truncate(raw),
    }
}

impl Dispatch<ExtWorkspaceManagerV1, ()> for Workspaces {
    fn event(
        state: &mut Self,
        _: &ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                state.snapshot.groups.push(WorkspaceGroup {
                    id: GroupId(workspace_group.id()),
                    outputs: Vec::new(),
                    capabilities: GroupCapabilities::empty(),
                    handle: workspace_group,
                    bound_outputs: Vec::new(),
                });
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                state.snapshot.workspaces.push(Workspace {
                    id: WorkspaceId(workspace.id()),
                    protocol_id: None,
                    name: String::new(),
                    coordinates: Vec::new(),
                    state: State::empty(),
                    capabilities: WorkspaceCapabilities::empty(),
                    group: None,
                    handle: workspace,
                });
            }
            ext_workspace_manager_v1::Event::Done => {
                state.published = Arc::new(state.snapshot.clone());
                state.refresh_published_outputs();
                state.saw_done = true;
            }
            ext_workspace_manager_v1::Event::Finished => {
                state.finished = true;
                state.snapshot.dead.store(true, Ordering::Release);
                state.snapshot.manager = None;
                let published = Arc::make_mut(&mut state.published);
                published.manager = None;
                published.groups.clear();
                published.workspaces.clear();
                state.saw_finished = true;
            }
            _ => {}
        }
    }

    // Both events above carry a `new_id`. Without this, client panics at runtime
    // on first workspace compositor announces.
    event_created_child!(Workspaces, ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ExtWorkspaceGroupHandleV1, ()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for Workspaces {
    fn event(
        state: &mut Self,
        handle: &ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let group_id = GroupId(handle.id());

        match event {
            ext_workspace_group_handle_v1::Event::Capabilities { capabilities } => {
                if let Some(group) = state.group_mut(handle) {
                    group.capabilities = flags(capabilities, GroupCapabilities::from_bits_truncate);
                }
            }
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                if let Some(group) = state.group_mut(handle)
                    && !group.bound_outputs.contains(&output)
                {
                    group.bound_outputs.push(output);
                }
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                if let Some(group) = state.group_mut(handle) {
                    group.bound_outputs.retain(|existing| existing != &output);
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                if let Some(workspace) = state.workspace_mut(&workspace) {
                    workspace.group = Some(group_id);
                }
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                // Nothing orders leave(old) before enter(new) on a move.
                if let Some(workspace) = state.workspace_mut(&workspace)
                    && workspace.group == Some(group_id)
                {
                    workspace.group = None;
                }
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                if !state.stopping {
                    let _lifetime = state
                        .snapshot
                        .stopped
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner);
                    handle.destroy();
                }
                state.snapshot.groups.retain(|group| group.id != group_id);
                for workspace in &mut state.snapshot.workspaces {
                    if workspace.group.as_ref() == Some(&group_id) {
                        workspace.group = None;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for Workspaces {
    fn event(
        state: &mut Self,
        handle: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_workspace_handle_v1::Event::Id { id } => {
                if let Some(workspace) = state.workspace_mut(handle) {
                    workspace.protocol_id = Some(id);
                }
            }
            ext_workspace_handle_v1::Event::Name { name } => {
                if let Some(workspace) = state.workspace_mut(handle) {
                    workspace.name = name;
                }
            }
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                if let Some(workspace) = state.workspace_mut(handle) {
                    workspace.coordinates = decode_coordinates(&coordinates);
                }
            }
            ext_workspace_handle_v1::Event::State { state: new_state } => {
                if let Some(workspace) = state.workspace_mut(handle) {
                    workspace.state = flags(new_state, State::from_bits_truncate);
                }
            }
            ext_workspace_handle_v1::Event::Capabilities { capabilities } => {
                if let Some(workspace) = state.workspace_mut(handle) {
                    workspace.capabilities =
                        flags(capabilities, WorkspaceCapabilities::from_bits_truncate);
                }
            }
            ext_workspace_handle_v1::Event::Removed => {
                let workspace_id = WorkspaceId(handle.id());
                if !state.stopping {
                    let _lifetime = state
                        .snapshot
                        .stopped
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner);
                    handle.destroy();
                }
                state
                    .snapshot
                    .workspaces
                    .retain(|workspace| workspace.id != workspace_id);
            }
            _ => {}
        }
    }
}

/// What the workspace subscription hands to the application
#[derive(Debug)]
pub enum WorkspaceEvent {
    /// The workspace tree, emitted once per manager `done` event.
    Updated(Arc<WorkspaceSnapshot>),
    /// The compositor does not implement `ext-workspace-v1`. Emitted once, at startup.
    Unsupported,
    /// The compositor ended the protocol.
    Finished,
    Stop(Error),
}

/// Dispatch state for the workspace worker.
#[derive(Debug)]
pub(crate) struct Workspaces {
    registry_state: RegistryState,
    /// Resolves group outputs on publish.
    output_state: OutputState,
    /// Working copy that protocol events mutate as they arrive.
    snapshot: WorkspaceSnapshot,
    /// The snapshot as of the last manager `done`.
    published: Arc<WorkspaceSnapshot>,
    /// Report `Unsupported` once, before any snapshot.
    announce_unsupported: bool,
    /// `published` changed since the last drain, protocol end sets `saw_finished`.
    saw_done: bool,
    /// Compositor answered `stop` or ended the protocol.
    finished: bool,
    /// `stop` has been sent, so no further request may go out.
    stopping: bool,
    /// The protocol ended and the application has not been told yet.
    saw_finished: bool,
}

impl Workspaces {
    fn workspace_mut(&mut self, handle: &ExtWorkspaceHandleV1) -> Option<&mut Workspace> {
        let id = handle.id();
        self.snapshot
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.id.0 == id)
    }

    fn group_mut(&mut self, handle: &ExtWorkspaceGroupHandleV1) -> Option<&mut WorkspaceGroup> {
        let id = handle.id();
        self.snapshot
            .groups
            .iter_mut()
            .find(|group| group.id.0 == id)
    }

    /// Re-resolve the published groups' outputs from `output_state`.
    fn refresh_published_outputs(&mut self) {
        for group in &mut Arc::make_mut(&mut self.published).groups {
            group.outputs = group
                .bound_outputs
                .iter()
                .filter_map(|output| self.output_state.info(output))
                .collect();
        }
    }

    /// Republish if `output` belongs to a published group.
    fn output_info_changed(&mut self, output: &WlOutput) {
        if self
            .published
            .groups
            .iter()
            .any(|group| group.bound_outputs.contains(output))
        {
            self.refresh_published_outputs();
            self.saw_done = true;
        }
    }
}

impl Worker for Workspaces {
    type Event = WorkspaceEvent;

    fn disposition(event: &WorkspaceEvent) -> Disposition {
        match event {
            // Carries the whole tree, so only the newest describes reality.
            WorkspaceEvent::Updated(_) => Disposition::Supersedes,
            // `finished` destroys the manager, and `Stop` only ever precedes
            // the worker returning. `Unsupported` is not terminal: the global
            // may still be registered later, and an `Updated` follows if it is.
            WorkspaceEvent::Finished | WorkspaceEvent::Stop(_) => Disposition::Terminal,
            WorkspaceEvent::Unsupported => Disposition::Incremental,
        }
    }

    fn init(
        conn: &Connection,
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
    ) -> Result<Self, Error> {
        // Absent is not final, compositor may register the global after connect
        let manager = globals
            .bind::<ExtWorkspaceManagerV1, _, _>(qh, 1..=1, ())
            .ok();
        let output_state = OutputState::new(globals, qh);
        let unsupported = manager.is_none();
        let snapshot = WorkspaceSnapshot {
            groups: Vec::new(),
            workspaces: Vec::new(),
            manager,
            conn: conn.clone(),
            stopped: Arc::new(Mutex::new(false)),
            dead: Arc::new(AtomicBool::new(false)),
        };
        Ok(Self {
            registry_state: RegistryState::new(globals),
            output_state,
            published: Arc::new(snapshot.clone()),
            snapshot,
            announce_unsupported: unsupported,
            saw_done: false,
            finished: false,
            stopping: false,
            saw_finished: false,
        })
    }

    fn take_events(&mut self) -> Vec<WorkspaceEvent> {
        if std::mem::take(&mut self.announce_unsupported) {
            return vec![WorkspaceEvent::Unsupported];
        }
        if std::mem::take(&mut self.saw_finished) {
            self.saw_done = false;
            return vec![WorkspaceEvent::Finished];
        }
        if std::mem::take(&mut self.saw_done) {
            vec![WorkspaceEvent::Updated(self.published.clone())]
        } else {
            Vec::new()
        }
    }

    fn reset_events(&mut self) -> Vec<WorkspaceEvent> {
        if self.published.groups.is_empty() && self.published.workspaces.is_empty() {
            return Vec::new();
        }
        let published = Arc::make_mut(&mut self.published);
        published.groups.clear();
        published.workspaces.clear();
        vec![WorkspaceEvent::Updated(self.published.clone())]
    }

    fn stop_event(error: Error) -> WorkspaceEvent {
        WorkspaceEvent::Stop(error)
    }

    fn teardown(&mut self, queue: &mut EventQueue<Self>) {
        crate::output::release_outputs(&self.output_state);
        let mut stopped = self
            .snapshot
            .stopped
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *stopped = true;
        self.snapshot.dead.store(true, Ordering::Release);

        for workspace in &self.snapshot.workspaces {
            workspace.handle.destroy();
        }
        for group in &self.snapshot.groups {
            group.handle.destroy();
        }
        self.snapshot.workspaces.clear();
        self.snapshot.groups.clear();

        if let Some(manager) = self.snapshot.manager.take() {
            self.stopping = true;
            manager.stop();
            drop(stopped);
            if queue.roundtrip(self).is_ok() && !self.finished {
                let _ = queue.roundtrip(self);
            }
        }
    }
}

impl OutputHandler for Workspaces {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.output_info_changed(&output);
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.output_info_changed(&output);
    }
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        // Fallback for the case the group's output_leave never lands.
        for group in &mut self.snapshot.groups {
            group.bound_outputs.retain(|existing| existing != &output);
        }
        let mut affected = false;
        for group in &mut Arc::make_mut(&mut self.published).groups {
            let before = group.bound_outputs.len();
            group.bound_outputs.retain(|existing| existing != &output);
            affected |= group.bound_outputs.len() != before;
        }
        if affected {
            self.refresh_published_outputs();
            self.saw_done = true;
        }
    }
}

impl sctk::registry::RegistryHandler<Workspaces> for Workspaces {
    /// The compositor registered the manager after we connected.
    fn new_global(
        state: &mut Workspaces,
        _conn: &Connection,
        qh: &QueueHandle<Workspaces>,
        _name: u32,
        interface: &str,
        _version: u32,
    ) {
        if state.snapshot.manager.is_some() || interface != ExtWorkspaceManagerV1::interface().name
        {
            return;
        }
        if let Ok(manager) = state.registry_state.bind_one(qh, 1..=1, ()) {
            state.snapshot.manager = Some(manager);
        }
    }
}

impl ProvidesRegistryState for Workspaces {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, Workspaces];
}

delegate_registry!(Workspaces);
delegate_dispatch2!(Workspaces);
delegate_noop!(Workspaces: ignore WlCallback);

/// Watch the compositor's workspaces.
pub fn listen(connection: Connection) -> iced_futures::Subscription<WorkspaceEvent> {
    worker::listen::<Workspaces>(connection)
}
