use std::{hash::Hash, mem::ManuallyDrop};

use futures::SinkExt;
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_output::{self, WlOutput},
        wl_registry,
    },
};

use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

#[derive(Debug)]
struct BaseState;

// so interesting, it is just need to invoke once, it just used to get the globals
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for BaseState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

#[derive(Debug)]
struct SubscribeState {
    xdg_output_manager: ZxdgOutputManagerV1,
    events: Vec<WaylandEvents>,
    pending_events: Vec<OutputInfo>,
}

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub description: String,
    zxdg_output: ZxdgOutputV1,
    is_ready: bool,
}

impl OutputInfo {
    fn new(wl_output: WlOutput, zxdg_output: ZxdgOutputV1) -> Self {
        Self {
            wl_output,
            name: "".to_owned(),
            description: "".to_owned(),
            zxdg_output,
            is_ready: false,
        }
    }
    fn check_is_ready(&mut self) {
        self.is_ready = !self.name.is_empty() && !self.description.is_empty()
    }
}

impl SubscribeState {
    fn new(xdg_output_manager: ZxdgOutputManagerV1) -> Self {
        Self {
            xdg_output_manager,
            events: Vec::new(),
            pending_events: Vec::new(),
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for SubscribeState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == wl_output::WlOutput::interface().name {
                    let output = proxy.bind::<wl_output::WlOutput, _, _>(name, version, qh, ());
                    let zxdg_output = state.xdg_output_manager.get_xdg_output(&output, qh, ());
                    state
                        .pending_events
                        .push(OutputInfo::new(output, zxdg_output));
                }
            }
            wl_registry::Event::GlobalRemove { .. } => {}
            _ => unreachable!(),
        }
    }
}
impl Dispatch<zxdg_output_v1::ZxdgOutputV1, ()> for SubscribeState {
    fn event(
        state: &mut Self,
        zxdg_output: &zxdg_output_v1::ZxdgOutputV1,
        event: <zxdg_output_v1::ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let Some((index, pending)) = state
            .pending_events
            .iter_mut()
            .enumerate()
            .find(|(_, event)| event.zxdg_output == *zxdg_output)
        else {
            return;
        };
        match event {
            zxdg_output_v1::Event::Name { name } => {
                pending.name = name;
            }
            zxdg_output_v1::Event::Description { description } => {
                pending.description = description;
            }
            _ => {}
        }
        pending.check_is_ready();
        let is_ready = pending.is_ready;
        let _ = pending;
        if is_ready {
            let pending = state.pending_events.remove(index);
            state.events.push(WaylandEvents::OutputInsert(pending));
        }
    }
}
delegate_noop!(SubscribeState: ignore WlOutput); // output is need to place layer_shell, although here
delegate_noop!(SubscribeState: ignore ZxdgOutputManagerV1);

#[derive(Debug, Clone)]
pub enum WaylandEvents {
    OutputInsert(OutputInfo),
    DispatchError(String),
}

#[derive(Debug, Clone)]
pub struct HashConnection {
    conn: Connection,
}

impl Hash for HashConnection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.conn.display().hash(state);
    }
}

impl From<Connection> for HashConnection {
    fn from(value: Connection) -> Self {
        Self { conn: value }
    }
}

#[derive(Debug)]
struct QueuePoll<'a, 'b> {
    queue: ManuallyDrop<&'a mut EventQueue<SubscribeState>>,
    state: ManuallyDrop<&'b mut SubscribeState>,
}

impl<'a, 'b> QueuePoll<'a, 'b> {
    fn new(queue: &'a mut EventQueue<SubscribeState>, state: &'b mut SubscribeState) -> Self {
        Self {
            queue: ManuallyDrop::new(queue),
            state: ManuallyDrop::new(state),
        }
    }
}

async fn async_dispatch(
    event_queue: &mut EventQueue<SubscribeState>,
    state: &mut SubscribeState,
) -> Result<WaylandEvents, wayland_client::DispatchError> {
    let poll_conn = QueuePoll::new(event_queue, state);
    poll_conn.await
}

use std::task::Poll;

impl<'a, 'b> Future for QueuePoll<'a, 'b> {
    type Output = Result<WaylandEvents, wayland_client::DispatchError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let mut poll_init = self.as_mut();
        let state = unsafe { ManuallyDrop::take(&mut poll_init.state) };
        let queue = unsafe { ManuallyDrop::take(&mut poll_init.queue) };
        if state.events.is_empty() {
            if let Err(e) = queue.roundtrip(state) {
                return Poll::Ready(Err(e));
            };
            poll_init.queue = ManuallyDrop::new(queue);
            poll_init.state = ManuallyDrop::new(state);
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            let event = state.events.pop().unwrap();
            poll_init.queue = ManuallyDrop::new(queue);
            poll_init.state = ManuallyDrop::new(state);
            Poll::Ready(Ok(event))
        }
    }
}

pub fn listen(connection: HashConnection) -> iced::Subscription<WaylandEvents> {
    iced::Subscription::run_with(connection, |conn| {
        use iced::futures::channel::mpsc::Sender;
        let conn = conn.clone();
        iced::stream::channel(100, |mut output: Sender<WaylandEvents>| async move {
            let connection = conn.conn.clone();
            let (globals, _) = registry_queue_init::<BaseState>(&connection).unwrap();

            let mut event_queue = connection.new_event_queue::<SubscribeState>();
            let qhandle = event_queue.handle();
            let display = connection.display();

            let xdg_output_manager = globals
                .bind::<ZxdgOutputManagerV1, _, _>(&qhandle, 1..=3, ())
                .unwrap(); // 
            let mut state = SubscribeState::new(xdg_output_manager);

            display.get_registry(&qhandle, ());
            loop {
                match async_dispatch(&mut event_queue, &mut state).await {
                    Ok(event) => {
                        output.send(event).await.ok();
                    }
                    Err(e) => {
                        output
                            .send(WaylandEvents::DispatchError(e.to_string()))
                            .await
                            .ok();
                        break;
                    }
                }
            }
        })
    })
}
