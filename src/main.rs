use std::fs::File;
use std::os::fd::AsFd;

use layershellev::reexport::*;
use layershellev::*;

fn main() {
    let mut ev = EventLoop::new()
        .with_size((400, 400))
        .with_anchor(Anchor::Left)
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive);

    ev.running(|event, ev| {
        println!("{:?}", event);
        match event {
            Event::RequestBuffer(file, shm, qh, init_w, init_h) => {
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
            Event::RequestMessages(DispatchMessage::Button { .. }) => {
                ev.set_anchor(Anchor::Right);
                ReturnData::None
            }
            Event::RequestMessages(DispatchMessage::KeyBoard { key, .. }) => {
                if *key == 1 {
                    return ReturnData::RequestExist;
                }
                ReturnData::None
            }
        }
    });
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
