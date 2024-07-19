Simple MagicaVox viewer.
Uses:
- wgpu for rendering
- dot_vox for parsing MagicaVox files
- winit for window/input management
- glam for 3d math

Opens snow.vox by default, reads a command line argument to determine the path of the file to open, supports drag-and-drop files on window.
After file loading adds rotating light cube.

## Controls

Mouse controls:
- Left Mouse - Rotate camera
- Scroll Wheel - Zoom
- Drop down vox-file on window - Open file, turn camera to models center

Keyboard controls:
- Left Shift - Move camera up
- Left Control - Move camera down
- W/Up - Move camera forward
- S/Down - Move camera backward
- A/Left - Move camera left
- D/Right - Move camera right