# Migration 0.19.1 -> HEAD

## iced_exwlshell
### Settings

```rust
application(new, namespace, update, view)
    .settings(Settings {
        .layersettings: LayerShellSettings {
            ...
        }
        ...
    })

->

application(new, namespace, update, view)
    .settings(Settings { ... })
    .layer_settings(LayerShellSettings {
        ...
    })
```

### Margin

```rust
(right, left, top, bottom) -> Margin { right, left, top, bottom }
```

## exwlshell

Now exwlshell abandon the callback way to handle the event, instead, used the similar way, trait, to address different events.

Now instead managering the buffer, you should use something like softbuffer to draw the surface

A simple example is follow.

```rust
use std::collections::HashMap;
use std::num::NonZeroU32;

use exwlshellev::keyboard::{KeyCode, PhysicalKey};
use exwlshellev::reexport::*;
use exwlshellev::*;

struct Window {
    context: softbuffer::Context<DisplayWrapper>,
    buffers: HashMap<id::Id, softbuffer::Surface<DisplayWrapper, WindowWrapper>>,
}

impl ExWlShellHandler for Window {
    fn on_init(
        &mut self,
        _state: &mut ExWlGlobalShellState,
        event: ExWlShellInitEvent,
    ) -> InitRequest {
        match event {
            // NOTE: this will send when init, you can request bind extra object from here
            ExWlShellInitEvent::Start => InitRequest::RequestBind,
            ExWlShellInitEvent::BindProvide(globals, qh) => {
                // NOTE: you can get implied wayland object from here
                let virtual_keyboard_manager = globals
                    .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                        qh,
                        1..=1,
                        (),
                    )
                    .unwrap();
                println!("{:?}", virtual_keyboard_manager);
                InitRequest::RequestCompositor
            }
            ExWlShellInitEvent::CompositorProvide(_compositor, _qh) => {
                // NOTE: this is an example to use the CompositorProvide,
                // but this is quite useless, because you can get the window_unit to set it directly
                // NOTE: you can set input region to limit area which gets input events
                // surface outside region becomes transparent for input events
                // To ignore all input events use region with (0,0) size
                // for x in state.get_unit_iter() {
                //     let region = _compositor.create_region(_qh, ());
                //     region.add(0, 0, 0, 0);
                //     x.get_wlsurface().set_input_region(Some(&region));
                // }
                InitRequest::None
            }
        }
    }

    fn on_normal_dispatch(&mut self, _context: NoIdEventContext<Self>) {}

    /// NOTE: you should know whether you have the buffer for this surface or not
    /// So no on created
    fn on_refresh(&mut self, context: RefreshEventContext<Self>) {
        let id = context.id();

        let mut resized = context.resized();
        let ex_wlshell_window = context.shell_state();

        let dpi::PhysicalSize { width, height } = ex_wlshell_window.physical_size();
        if !self.buffers.contains_key(&id) {
            self.buffers.insert(
                id,
                softbuffer::Surface::new(&self.context, ex_wlshell_window.gen_wrapper()).unwrap(),
            );
            resized = true;
        }
        let surface = self.buffers.get_mut(&id).unwrap();
        if resized {
            surface
                .resize(
                    NonZeroU32::new(width).unwrap(),
                    NonZeroU32::new(height).unwrap(),
                )
                .unwrap();
        }
        let mut buffer = surface.buffer_mut().unwrap();

        use std::cmp::min;
        let buf_x = buffer.width().get();
        let buf_y = buffer.height().get();
        for index in 0..(buffer.width().get() * buffer.height().get()) {
            let y = index / buffer.width().get();
            let x = index % buffer.width().get();
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);

            let color = (a << 24) + (r << 16) + (g << 8) + b;
            buffer[index as usize] = color;
        }
        buffer.present().unwrap();

        println!("{width}, {height}");
    }

    fn on_broadcast(&mut self, _context: NoIdEventContext<Self>, event: ExWlShellBroadcast) {
        if let ExWlShellBroadcast::OutputAdded(info) = event {
            println!("{info:?}");
        }
    }
    fn on_window_event(&mut self, mut context: HaveIdEventContext<Self>, event: ExWlShellEvent) {
        let id = context.id();
        let state = context.global_state_mut();
        match event {
            ExWlShellEvent::MouseEnter { pointer, .. } => {
                state.set_cursor(Cursor::Shape(CursorShape::Crosshair), pointer)
            }
            ExWlShellEvent::MouseMotion {
                time,
                surface_x,
                surface_y,
            } => {
                println!("{time}, {surface_x}, {surface_y}");
            }
            ExWlShellEvent::OutputChanged(output) => {
                // NOTE: sent when surface enters another output, or its output info changes
                let info = output.as_ref().and_then(|o| state.get_output_info_of(o));
                println!("{info:?}");
            }
            ExWlShellEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                    state.exit();
                }
            }
            ExWlShellEvent::Closed => {
                self.buffers.remove(&id);
            }
            _ => {}
        }
    }
}

fn main() {
    let prebuild = ExWlEventLoopBuilder::new("Hello")
        .with_allscreens()
        .with_size(LayerSize::fill_width(400))
        .with_layer(Layer::Top)
        .with_margin(Margin {
            top: 20,
            right: 20,
            bottom: 100,
            left: 20,
        })
        .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
        .with_exclusive_zone(-1)
        .prebuild()
        .unwrap();
    let display = prebuild.display_wrapper();
    let context = softbuffer::Context::new(display).unwrap();
    let window = Window {
        context,
        buffers: HashMap::new(),
    };
    let ev = prebuild.attach(window).unwrap();

    ev.run().unwrap()
}
```

Note: `XXXIdEventContext` and the `ExWlEventLoop` allow you to register custom subscription, and return you a Sender to send messages.

For example

```rust
let message_sender = wl_context
    .register(|window, mut shell_context, action: Action<P::Message>| {
        let ContextState::Context(context) = &mut window.context_state else {
            unreachable!("context state is not initialized");
        };
        context.handle_user_action(shell_context.global_state_mut(), action);
    })
    .unwrap();
```

The code about is in `iced_exwlshell/src/window.rs`

This will be of help if you consider to use something like a11y

About `on_refresh`,We do not have a callback for when a new shell come out, instead exwlshell thinks you know if you have the wlbuffer for it. The RefreshEventContext will provide the information whether the scale factor changed and whether the size of the surface changed.

During `on_init`, you can access something like globals and compositor, but in fact you can get it anytime. I just think it can be of some help, so I kept this design.

`on_normal_dispatch` is the callback called every time the poll of the events is finished, you can do things like resizing and reanchoring.

`on_broadcast` is the callback when the messages are not related to specific events, for example when a new Display is plugged or etc.
