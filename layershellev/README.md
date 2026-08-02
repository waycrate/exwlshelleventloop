# Layershellev

Layershelleventloop, take lot reference from winit, amin to make a easy
way to create layershell window.

you can take `./examples/simplelayer.rs` for example

The shape is a callback over `LayerShellEvent`:

```rust, no_run
let ev: WindowState<()> = WindowState::new("Hello").build().unwrap();

ev.running(|event, ev, index| match event {
    // Sent once at start-up; the place to bind extra globals.
    LayerShellEvent::InitRequest => ReturnData::RequestBind,
    LayerShellEvent::BindProvide(globals, qh) => ReturnData::None,
    LayerShellEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
        // hand back a WlBuffer for the surface
        ReturnData::None
    }
    LayerShellEvent::RequestMessages(message) => ReturnData::None,
    _ => ReturnData::None,
})
.unwrap();
```

## Monitors

The loop binds the outputs itself, so it reports them rather than making you
open a second connection:

- `DispatchMessage::OutputAdded(OutputInfo)`: a monitor was connected
- `DispatchMessage::OutputUpdated(OutputInfo)`: its mode, scale, name or
  position changed
- `DispatchMessage::OutputRemoved(OutputInfo)`: it was disconnected, and the
  surfaces that covered it are closed
- `DispatchMessage::OutputChanged(Option<WlOutput>)`: a surface is now on a
  different monitor

`WindowStateUnit::get_wloutput()` gives the monitor a surface is on and
`get_wloutputs()` every monitor it overlaps, since a surface can span more than
one.

## Choosing a monitor

`NewLayerShellSettings::output_option` takes an `OutputOption`:

- `Active`: let the compositor decide (the default)
`LastOutput`: the monitor the previous surface went to
`OutputName(String)`: by name, such as `DP-2`
`GlobalName(u32)`: by `wl_registry` global name, as carried by `OutputInfo::id`
`Output(WlOutput)`: a monitor you already hold, on the same connection

A name that matches nothing connected logs a warning and falls back to letting
the compositor choose, so a surface is still created.

For more example, please take a look at [exwlshelleventloop](https://github.com/waycrate/exwlshelleventloop)
