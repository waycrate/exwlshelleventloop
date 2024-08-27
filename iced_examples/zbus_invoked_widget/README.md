# Background widget service

It allow the program not to show at once, but it can be invoked by dbus or etc.

use

```bash
gdbus call --session --dest zbus.iced.MyGreeter1 --object-path /org/zbus/MyGreeter1 --method org.zbus.MyGreeter1.SayHello hello
```

It can be used in make a xdg-desktop-portal filechooser
