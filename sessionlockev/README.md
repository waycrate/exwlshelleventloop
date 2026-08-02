# sessionlockev

SessionLockEventloop, take lot reference from winit, amin to make a easy way to create layershell window.

you can take `./examples/simplelock.rs` for example

The shape is a callback over `SessionLockEvent`:

```rust, no_run
use sessionlockev::keyboard::{KeyCode, PhysicalKey};
use sessionlockev::reexport::*;
use sessionlockev::*;

let ev: WindowState<()> = WindowState::new().build().unwrap();

ev.running(|event, ev, index| match event {
    // Sent once at start-up; the place to bind extra globals.
    SessionLockEvent::InitRequest => ReturnData::RequestBind,
    SessionLockEvent::BindProvide(globals, qh) => ReturnData::None,
    SessionLockEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
        // hand back a WlBuffer for the surface
        ReturnData::None
    }
    SessionLockEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
        if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
            ReturnData::RequestUnlockAndExist
        } else {
            ReturnData::None
        }
    }
    _ => ReturnData::None,
})
.unwrap();
```

## Monitors

A lock surface is created for every connected monitor and destroyed with it, so
the loop reports the monitors themselves:

- `DispatchMessage::OutputAdded(OutputInfo)`: a monitor was connected, and a
  lock surface has been created for it
- `DispatchMessage::OutputUpdated(OutputInfo)`: its mode, scale, name or
  position changed
- `DispatchMessage::OutputRemoved(OutputInfo)`: it was disconnected, and the
  lock surface that covered it is closed

`WindowState::get_output_info_of(&wl_output)` resolves a monitor to its
`OutputInfo`, and `WindowStateUnit::get_wloutput()` gives the monitor a lock
surface covers.

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
