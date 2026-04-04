Simple MagicaVox viewer.
Uses:
- wgpu for rendering
- dot_vox for parsing MagicaVox files
- winit for window/input management
- glam for 3d math

Opens snow.vox by default, reads a command line argument to determine the path of the file to open, supports drag-and-drop files on window.
After file loading adds rotating light cube.

## Known Limitations

**Drag-and-drop on Wayland:** winit does not currently implement the Wayland drag-and-drop protocol (`wl_data_device`), so `DroppedFile` events are never delivered under a native Wayland session. To use drag-and-drop, force the X11 backend by unsetting `WAYLAND_DISPLAY`:

```sh
WAYLAND_DISPLAY="" cargo run
```

Note: if your file manager runs as a native Wayland application it may also refuse to drop files onto an XWayland window. In that case use the CLI argument instead:

```sh
WAYLAND_DISPLAY="" cargo run -- path/to/model.vox
```

## Controls

Mouse controls:
- Left Mouse drag - Orbit around model
- Right Mouse drag - Pan
- Scroll Wheel - Zoom
- Drop vox-file on window - Open file, reset camera to model center

Keyboard controls:
- W/S/A/D - Orbit
- Arrow keys - Pan
- Left Shift / Left Control - Zoom in / out