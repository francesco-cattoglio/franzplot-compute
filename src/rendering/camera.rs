use glam::{Vec3, Mat4};
use winit::event::*;

#[derive(Debug)]
pub struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            eye: Vec3::new(3.0, 1.5, 1.5),
            target: Vec3::new(0.0, 0.0, 0.0),
            aspect: 1.0,
            up: Vec3::new(0.0, 0.0, 1.0),
            fov_y: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        }
    }
}

impl Camera {
    pub fn from_height_width(height: f32, width: f32) -> Self {
        Self {
            aspect: width/height,
            .. Default::default()
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let lookat_matrix = Mat4::look_at_lh(
            self.eye,
            self.target,
            self.up,
        );
        let proj_matrix = Mat4::perspective_lh(self.fov_y, self.aspect, self.z_near, self.z_far);
        // let proj_matrix = Mat4::orthographic_lh(-4.0, 4.0, -2.5, 2.5, -10.0, 10.0);

        proj_matrix * lookat_matrix
    }
}

#[derive(Debug)]
pub struct InputState {
    pub forward: bool,
    pub back: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub mouse_right_click: bool,
    pub mouse_left_click: bool,
    pub mouse_motion: (f64, f64),
    pub mouse_wheel: MouseScrollDelta,
}

impl InputState {
    pub fn reset_deltas(&mut self) {
        self.mouse_motion = (0.0, 0.0);
        self.mouse_wheel = MouseScrollDelta::LineDelta(0.0, 0.0);
    }
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            mouse_wheel: MouseScrollDelta::LineDelta(0.0, 0.0),
            forward: false,
            back: false,
            up: false,
            down: false,
            left: false,
            right: false,
            mouse_left_click: false,
            mouse_right_click: false,
            mouse_motion: (0.0, 0.0),
        }
    }
}

pub trait Controller {
    fn update_camera(&self, camera: &mut Camera, inputs: &InputState);
}

#[derive(Debug)]
pub struct VTKController {
    sensitivity_vertical: f32,
    sensitivity_horizontal: f32,
    sensitivity_zoom: f32,
    min_distance: f32,
}

impl VTKController {
    pub fn new(sensitivity_vertical: f32, sensitivity_horizontal: f32, sensitivity_zoom: f32) -> Self {
        Self {
            sensitivity_horizontal,
            sensitivity_vertical,
            sensitivity_zoom,
            min_distance: 0.25,
        }
    }
}

impl Controller for VTKController {
    fn update_camera(&self, camera: &mut Camera, inputs: &InputState) {
        let relative_pos = camera.eye - camera.target;
        let mut distance = relative_pos.length();
        let mut pos_on_sphere = relative_pos.normalize();
        match inputs.mouse_wheel {
            MouseScrollDelta::LineDelta(_x, y) => {
                distance += y * self.sensitivity_zoom;
                distance = distance.max(self.min_distance);
            },
            MouseScrollDelta::PixelDelta(physical_position) => {
                distance += physical_position.y as f32 * self.sensitivity_zoom;
                distance = distance.max(self.min_distance);
            }
        }
        if inputs.mouse_left_click {
            let x_delta = inputs.mouse_motion.0 as f32 * self.sensitivity_horizontal;
            let y_delta = inputs.mouse_motion.1 as f32 * self.sensitivity_vertical;
            let camera_right = camera.up.cross(pos_on_sphere.normalize());
            let position_delta =  y_delta * camera.up + x_delta * camera_right;
            // for small angles, sin(theta) = theta!
            pos_on_sphere = (pos_on_sphere + position_delta).normalize();
            camera.up = camera_right.cross(-pos_on_sphere).normalize();
        }
        camera.eye = pos_on_sphere * distance;
    }
}


