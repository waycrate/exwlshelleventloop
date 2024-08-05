use std::fs::File;
use std::os::fd::AsFd;

use layershellev::keyboard::{KeyCode, PhysicalKey};
use layershellev::reexport::*;
use layershellev::*;

fn main() {
    let ev: WindowState<(), ()> = WindowState::new("Hello")
        .with_single(false)
        .with_size((0, 400))
        .with_layer(Layer::Top)
        .with_margin((20, 20, 100, 20))
        .with_anchor(Anchor::Bottom | Anchor::Left | Anchor::Right)
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
        .with_exclusize_zone(-1)
        .build()
        .unwrap();

    let mut virtual_keyboard_manager = None;
    ev.running(|event, ev, index| {
        match event {
            // NOTE: this will send when init, you can request bind extra object from here
            LayerEvent::InitRequest => ReturnData::RequestBind,
            LayerEvent::BindProvide(globals, qh) => {
                // NOTE: you can get implied wayland object from here
                virtual_keyboard_manager = Some(
                    globals
                        .bind::<zwp_virtual_keyboard_v1::ZwpVirtualKeyboardManagerV1, _, _>(
                            qh,
                            1..=1,
                            (),
                        )
                        .unwrap(),
                );
                println!("{:?}", virtual_keyboard_manager);
                ReturnData::None
            }
            LayerEvent::XdgInfoChanged(_) => {
                let index = index.unwrap();
                let unit = ev.get_unit(index);
                println!("{:?}", unit.get_xdgoutput_info());
                ReturnData::None
            }
            LayerEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
                draw(file, (init_w, init_h));
                let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
                ReturnData::WlBuffer(pool.create_buffer(
                    0,
                    init_w as i32,
                    init_h as i32,
                    (init_w * 4) as i32,
                    wl_shm::Format::Argb8888,
                    qh,
                    (),
                ))
            }
            LayerEvent::RequestMessages(DispatchMessage::RequestRefresh {
                width, height, ..
            }) => {
                println!("{width}, {height}");
                ReturnData::None
            }
            LayerEvent::RequestMessages(DispatchMessage::MouseButton { .. }) => ReturnData::None,
            LayerEvent::RequestMessages(DispatchMessage::MouseEnter {
                serial, pointer, ..
            }) => ReturnData::RequestSetCursorShape((
                "crosshair".to_owned(),
                pointer.clone(),
                *serial,
            )),
            LayerEvent::RequestMessages(DispatchMessage::MouseMotion {
                time,
                surface_x,
                surface_y,
            }) => {
                println!("{time}, {surface_x}, {surface_y}");
                ReturnData::None
            }
            LayerEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
                if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                    ReturnData::RequestExist
                } else {
                    ReturnData::None
                }
            }
            _ => ReturnData::None,
        }
    })
    .unwrap();
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
