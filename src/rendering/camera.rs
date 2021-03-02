use glam::{Vec3, Mat4};
use winit::event::*;
use crate::state::Sensitivity;

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
            eye: Vec3::new(6.0, 4.0, 4.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            aspect: 1.0,
            up: Vec3::new(0.0, 0.0, 1.0),
            fov_y: 0.2 * std::f32::consts::PI,
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

    pub fn reset_view(&mut self) {
        self.eye = Vec3::new(6.0, 4.0, 4.0);
        self.target = Vec3::new(0.0, 0.0, 0.0);
        self.up = Vec3::new(0.0, 0.0, 1.0);
    }

    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn build_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn build_ortho_matrix(&self) -> Mat4 {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        // the constant that premultiplies the distance was chosen because on my monitor
        // it is the one that makes the switch between orthographic and perspective almost seamless
        let half_h = 0.25 * distance;
        let half_w = half_h * self.aspect;
        // need testing
        Mat4::orthographic_lh(-half_w, half_w, -half_h, half_h, 64.0, -64.0)
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
    pub mouse_middle_click: bool,
    pub mouse_left_click: bool,
    pub mouse_motion: (f64, f64),
    pub mouse_wheel: f32,
}

impl InputState {
    pub fn reset_deltas(&mut self) {
        self.mouse_motion = (0.0, 0.0);
        self.mouse_wheel = 0.0;
    }
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            mouse_wheel: 0.0,
            forward: false,
            back: false,
            up: false,
            down: false,
            left: false,
            right: false,
            mouse_left_click: false,
            mouse_middle_click: false,
            mouse_motion: (0.0, 0.0),
        }
    }
}

pub trait Controller {
    // TODO: when we introduce more than 1 kind of controllers, we might need
    // to change the signature and remove the "lock_z_up" boolean
    fn update_camera(&self, camera: &mut Camera, inputs: &InputState, sensitivity: &Sensitivity, lock_z_up: bool);
}

#[derive(Debug)]
pub struct VTKController {
    min_distance: f32,
}

impl VTKController {
    pub fn new() -> Self {
        Self {
            min_distance: 0.20,
        }
    }
}

impl Controller for VTKController {
    fn update_camera(&self, camera: &mut Camera, inputs: &InputState, sensitivity: &Sensitivity, lock_z_up: bool) {
        let relative_pos = camera.eye - camera.target;
        let mut distance = relative_pos.length();
        let mut pos_on_sphere = relative_pos.normalize();
        distance += inputs.mouse_wheel;
        distance = distance.max(self.min_distance);

        if inputs.mouse_left_click {
            let rot_coeff = 0.02;
            let x_delta = inputs.mouse_motion.0 as f32 * rot_coeff * sensitivity.camera_horizontal;
            let y_delta = inputs.mouse_motion.1 as f32 * rot_coeff * sensitivity.camera_vertical;
            let camera_right = camera.up.cross(pos_on_sphere.normalize());
            // we want to render the effect of the object rotating the same way
            // the mouse moves. For this reason, we need to rotate the camera
            // in the opposite direction along "camera.right"!
            // The movement along the "camera.up" direction does not need to be inverted
            // because screen coordinates are already "y points downwards"
            let position_delta =  y_delta * camera.up - x_delta * camera_right;
            // for small angles, sin(theta) = theta!
            pos_on_sphere = (pos_on_sphere + position_delta).normalize();
            if lock_z_up {
                camera.up = Vec3::unit_z();
            } else {
                camera.up = camera_right.cross(-pos_on_sphere).normalize();
            }
        } else if inputs.mouse_middle_click {
            // panning
            let camera_right = camera.up.cross(pos_on_sphere).normalize();
            // BEWARE: camera_up might be different from camera.up!
            // camera.up might be locked due to user settings (camera.up might be locked to z axis
            let camera_up = camera_right.cross(-pos_on_sphere).normalize();
            let pan_coeff = 0.002 * distance;
            let x_delta = inputs.mouse_motion.0 as f32 * pan_coeff * sensitivity.camera_horizontal;
            let y_delta = inputs.mouse_motion.1 as f32 * pan_coeff * sensitivity.camera_vertical;
            let position_delta = y_delta * camera_up - x_delta * camera_right;
            camera.target += position_delta;
        }
        camera.eye = camera.target + pos_on_sphere * distance;
    }
}


