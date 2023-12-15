use std::fs::File;
use std::os::fd::AsFd;

use layershellev::reexport::*;
use layershellev::*;

const Q_KEY: u32 = 16;
const W_KEY: u32 = 17;
const E_KEY: u32 = 18;
const A_KEY: u32 = 30;
const S_KEY: u32 = 31;
const D_KEY: u32 = 32;
const Z_KEY: u32 = 44;
const X_KEY: u32 = 45;
const C_KEY: u32 = 46;
const ESC_KEY: u32 = 1;

fn main() {
    let mut ev = LayerEventLoop::new()
        .with_size((400, 400))
        .with_anchor(Anchor::Left)
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
        .with_exclusize_zone(-1);

    ev.running(|event, ev| {
        println!("{:?}", event);
        match event {
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
            LayerEvent::RequestMessages(DispatchMessage::RefreshSurface { width, height }) => {
                println!("{width}, {height}");
                ReturnData::None
            }
            LayerEvent::RequestMessages(DispatchMessage::Button { .. }) => ReturnData::None,
            LayerEvent::RequestMessages(DispatchMessage::KeyBoard { key, .. }) => {
                match *key {
                    Q_KEY => ev.set_anchor(Anchor::Top | Anchor::Left),
                    W_KEY => ev.set_anchor(Anchor::Top),
                    E_KEY => ev.set_anchor(Anchor::Top | Anchor::Right),
                    A_KEY => ev.set_anchor(Anchor::Left),
                    S_KEY => {
                        ev.set_anchor(Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom)
                    }
                    D_KEY => ev.set_anchor(Anchor::Right),
                    Z_KEY => ev.set_anchor(Anchor::Left | Anchor::Bottom),
                    X_KEY => ev.set_anchor(Anchor::Bottom),
                    C_KEY => ev.set_anchor(Anchor::Bottom | Anchor::Right),
                    ESC_KEY => return ReturnData::RequestExist,
                    _ => {}
                }
                ReturnData::None
            }
            _ => unreachable!(),
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
