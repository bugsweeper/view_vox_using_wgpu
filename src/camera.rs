use glam::{Mat4, Vec3, Vec4};
use std::f32::consts;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_projection: [f32; 16],
    view_position: [f32; 4],
}

impl From<(&OrbitCamera, &Projection)> for CameraUniform {
    fn from((camera, projection): (&OrbitCamera, &Projection)) -> Self {
        Self {
            view_projection: *(projection.calc_matrix() * camera.calc_matrix()).as_ref(),
            view_position: camera.view_position().into(),
        }
    }
}

/// Spherical orbit camera. Rotates around `target` at a given `distance`.
#[derive(Debug)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl OrbitCamera {
    /// World-space position of the camera eye.
    pub fn position(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        self.target
            + self.distance * Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw)
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    pub fn view_position(&self) -> Vec4 {
        self.position().extend(1.0)
    }

    /// Camera right vector (horizontal, no Y component).
    fn right(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(sin_yaw, 0.0, -cos_yaw)
    }

    /// Camera up vector in world space.
    fn up(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(-cos_yaw * sin_pitch, cos_pitch, -sin_yaw * sin_pitch)
    }
}

pub struct Projection {
    aspect_ratio: f32,
    fov_y: f32,
    z_near: f32,
    z_far: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fov_y: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            aspect_ratio: width as f32 / height as f32,
            fov_y,
            z_near,
            z_far,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect_ratio = width as f32 / height as f32;
    }

    pub fn set_z_far(&mut self, z_far: f32) {
        self.z_far = z_far;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect_ratio, self.z_near, self.z_far)
    }
}

pub struct OrbitController {
    rotate_horizontal: f32,
    rotate_vertical: f32,
    pan_horizontal: f32,
    pan_vertical: f32,
    scroll: f32,
    // keyboard amounts, range [0, 1]
    kb_orbit_left: f32,
    kb_orbit_right: f32,
    kb_orbit_up: f32,
    kb_orbit_down: f32,
    kb_pan_left: f32,
    kb_pan_right: f32,
    kb_pan_up: f32,
    kb_pan_down: f32,
    kb_zoom_in: f32,
    kb_zoom_out: f32,
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
}

impl OrbitController {
    pub fn new(orbit_sensitivity: f32, pan_sensitivity: f32, zoom_sensitivity: f32) -> Self {
        Self {
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            pan_horizontal: 0.0,
            pan_vertical: 0.0,
            scroll: 0.0,
            kb_orbit_left: 0.0,
            kb_orbit_right: 0.0,
            kb_orbit_up: 0.0,
            kb_orbit_down: 0.0,
            kb_pan_left: 0.0,
            kb_pan_right: 0.0,
            kb_pan_up: 0.0,
            kb_pan_down: 0.0,
            kb_zoom_in: 0.0,
            kb_zoom_out: 0.0,
            orbit_sensitivity,
            pan_sensitivity,
            zoom_sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let v = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            KeyCode::KeyW => { self.kb_orbit_up    = v; true }
            KeyCode::KeyS => { self.kb_orbit_down  = v; true }
            KeyCode::KeyA => { self.kb_orbit_left  = v; true }
            KeyCode::KeyD => { self.kb_orbit_right = v; true }
            KeyCode::ArrowUp    => { self.kb_pan_up    = v; true }
            KeyCode::ArrowDown  => { self.kb_pan_down  = v; true }
            KeyCode::ArrowLeft  => { self.kb_pan_left  = v; true }
            KeyCode::ArrowRight => { self.kb_pan_right = v; true }
            KeyCode::ShiftLeft  => { self.kb_zoom_in   = v; true }
            KeyCode::ControlLeft => { self.kb_zoom_out = v; true }
            _ => false,
        }
    }

    pub fn process_orbit(&mut self, dx: f64, dy: f64) {
        self.rotate_horizontal = dx as f32;
        self.rotate_vertical = dy as f32;
    }

    pub fn process_pan(&mut self, dx: f64, dy: f64) {
        self.pan_horizontal = dx as f32;
        self.pan_vertical = dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => -*y as f32 * 0.01,
        };
    }

    pub fn update_camera(&mut self, camera: &mut OrbitCamera, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        // Orbit (mouse + keyboard)
        let orbit_dx = self.rotate_horizontal
            + (self.kb_orbit_right - self.kb_orbit_left) * 90.0 * dt;
        let orbit_dy = self.rotate_vertical
            + (self.kb_orbit_down - self.kb_orbit_up) * 90.0 * dt;
        camera.yaw += orbit_dx * self.orbit_sensitivity;
        camera.pitch += orbit_dy * self.orbit_sensitivity;
        camera.pitch = camera
            .pitch
            .clamp(-consts::FRAC_PI_2 + 0.01, consts::FRAC_PI_2 - 0.01);

        // Pan (mouse + keyboard), scaled by distance
        let pan_scale = camera.distance * self.pan_sensitivity;
        let pan_dx = self.pan_horizontal
            + (self.kb_pan_right - self.kb_pan_left) * 60.0 * dt;
        let pan_dy = self.pan_vertical
            + (self.kb_pan_down - self.kb_pan_up) * 60.0 * dt;
        camera.target -= camera.right() * pan_dx * pan_scale;
        camera.target += camera.up() * pan_dy * pan_scale;

        // Zoom (scroll + keyboard)
        let zoom = self.scroll + (self.kb_zoom_out - self.kb_zoom_in) * 2.0 * dt;
        camera.distance = (camera.distance * (1.0 + zoom * self.zoom_sensitivity)).max(0.1);

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;
        self.pan_horizontal = 0.0;
        self.pan_vertical = 0.0;
        self.scroll = 0.0;
    }
}
