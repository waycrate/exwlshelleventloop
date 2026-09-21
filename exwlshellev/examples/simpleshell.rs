use std::fs::File;
use std::os::fd::AsFd;

use exwlshellev::keyboard::{KeyCode, PhysicalKey};
use exwlshellev::reexport::*;
use exwlshellev::*;

struct Window;
impl ExWlShellHandler<()> for Window {
    fn request_buffer(
        &mut self,
        state: &mut WindowState<()>,
        _id: id::Id,
        file: &mut std::fs::File,
        qh: &wayland_client::QueueHandle<WindowState<()>>,
        width: u32,
        height: u32,
    ) -> wayland_client::WlBuffer {
        draw(file, (width, height));
        let pool = state
            .get_shm()
            .create_pool(file.as_fd(), (width * height * 4) as i32, qh, ());
        pool.create_buffer(
            0,
            width as i32,
            height as i32,
            (width * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        )
    }
    fn on_init(
        &mut self,
        event: ExWlShellInitEvent<()>,
        _state: &mut WindowState<()>,
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
    fn on_normal_dispatch(&mut self, _state: &mut WindowState<()>) {}
    fn on_event(
        &mut self,
        event: ExWlShellEvent,
        state: &mut WindowState<()>,
        _id: Option<id::Id>,
    ) {
        match event {
            ExWlShellEvent::RequestRefresh { width, height, .. } => {
                println!("{width}, {height}");
            }
            ExWlShellEvent::MouseEnter { pointer, .. } => state.push_request(
                Request::RequestSetCursor((Cursor::Shape(CursorShape::Crosshair), pointer.clone())),
            ),
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
                    state.push_request(Request::RequestExit);
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let window = Window;
    let ev: EventContext<(), _> = WindowState::new("Hello")
        .with_allscreens()
        .with_size(LayerSize::fill_width(400))
        .with_layer(Layer::Top)
        .with_margin((20, 20, 100, 20))
        .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
        .with_exclusive_zone(-1)
        .build(window)
        .unwrap();

    ev.run().unwrap()
}

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32)) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
            let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);

            let color = (a << 24) + (r << 16) + (g << 8) + b;
            buf.write_all(&color.to_ne_bytes()).unwrap();
        }
    }
    buf.flush().unwrap();
}
