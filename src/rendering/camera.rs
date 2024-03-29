use glam::{Vec3, Mat4};
use crate::state::Sensitivity;

#[derive(Debug)]
pub struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Camera {
        let mut new_camera = Camera {
            fov_y: 0.2 * std::f32::consts::PI,
            z_near: 0.1,
            z_far: 250.0,
            eye: Vec3::default(),
            target: Vec3::default(),
            up: Vec3::default(),
        };
        new_camera.default_view();
        new_camera
    }
}

impl Camera {
    pub fn default_view(&mut self) {
        self.eye = Vec3::new(6.0, 4.0, 4.0);
        self.target = Vec3::new(0.0, 0.0, 0.0);
        let relative_pos = self.eye - self.target;
        let right = relative_pos.cross(Vec3::Z);
        self.up = right.cross(relative_pos).normalize();
    }

    pub fn build_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn build_projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect, self.z_near, self.z_far)
    }

    pub fn build_ortho_matrix(&self, aspect: f32) -> Mat4 {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        // the constant that premultiplies the distance was chosen because on my monitor
        // it is the one that makes the switch between orthographic and perspective almost seamless
        let half_h = 0.25 * distance;
        let half_w = half_h * aspect;
        Mat4::orthographic_lh(-half_w, half_w, -half_h, half_h, 128.0, -128.0)
    }

    pub fn set_xz_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = -distance * Vec3::Y;
        self.up = Vec3::Z;
    }

    pub fn set_xy_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = distance * Vec3::Z;
        self.up = Vec3::Y;
    }

    pub fn set_yz_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = distance * Vec3::X;
        self.up = Vec3::Z;
    }

    pub fn set_minus_xz_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = distance * Vec3::Y;
        self.up = Vec3::Z;
    }

    pub fn set_minus_xy_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = -distance * Vec3::Z;
        self.up = Vec3::Y;
    }

    pub fn set_minus_yz_plane(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = -distance * Vec3::X;
        self.up = Vec3::Z;
    }

    pub fn set_x1_y1_z1_point(&mut self) {
        let relative_pos = self.eye - self.target;
        let distance = relative_pos.length();
        self.target = Vec3::ZERO;
        self.eye = distance * Vec3::ONE.normalize();
        self.up = Vec3::Z;
    }

    pub fn set_x1_y1_z1_wide(&mut self) {
        let relative_pos = self.eye - self.target;
        self.target = Vec3::ZERO;
        self.eye = 14.0 * Vec3::ONE.normalize();
        self.up = Vec3::Z;
    }
}

#[derive(Debug)]
pub struct InputState {
    pub reset_to_xy: bool,
    pub reset_to_xz: bool,
    pub reset_to_yz: bool,
    pub reset_to_minus_xy: bool,
    pub reset_to_minus_xz: bool,
    pub reset_to_minus_yz: bool,
    pub reset_to_xyz: bool,
    pub mouse_middle_click: bool,
    pub mouse_left_click: bool,
    pub mouse_motion: (f64, f64),
    pub mouse_wheel: f32,
}

impl InputState {
    pub fn reset_deltas(&mut self) {
        self.reset_to_xy = false;
        self.reset_to_xz = false;
        self.reset_to_yz = false;
        self.reset_to_minus_xy = false;
        self.reset_to_minus_xz = false;
        self.reset_to_minus_yz = false;
        self.reset_to_xyz = false;
        self.mouse_motion = (0.0, 0.0);
        self.mouse_wheel = 0.0;
    }
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            reset_to_xy: false,
            reset_to_xz: false,
            reset_to_yz: false,
            reset_to_minus_xy: false,
            reset_to_minus_xz: false,
            reset_to_minus_yz: false,
            reset_to_xyz: false,
            mouse_wheel: 0.0,
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
        if inputs.reset_to_xz {
            camera.set_xz_plane();
            return;
        }
        if inputs.reset_to_xy {
            camera.set_xy_plane();
            return;
        }
        if inputs.reset_to_yz {
            camera.set_yz_plane();
            return;
        }
        if inputs.reset_to_minus_xz {
            camera.set_minus_xz_plane();
            return;
        }
        if inputs.reset_to_minus_xy {
            camera.set_minus_xy_plane();
            return;
        }
        if inputs.reset_to_minus_yz {
            camera.set_minus_yz_plane();
            return;
        }
        if inputs.reset_to_xyz {
            camera.set_x1_y1_z1_point();
            return;
        }

        let relative_pos = camera.eye - camera.target;
        let mut distance = relative_pos.length();
        let mut pos_on_sphere = relative_pos.normalize();
        distance += inputs.mouse_wheel;
        distance = distance.max(self.min_distance);

        if inputs.mouse_left_click {
            let rot_coeff = -0.005;
            let x_delta = inputs.mouse_motion.0 as f32 * rot_coeff * sensitivity.camera_horizontal;
            let y_delta = inputs.mouse_motion.1 as f32 * rot_coeff * sensitivity.camera_vertical;
            let camera_right = camera.up.cross(pos_on_sphere).normalize();
            // first compute the change in pitch
            let pitch = glam::Mat3::from_axis_angle(camera_right, y_delta);
            pos_on_sphere = pitch * pos_on_sphere;
            camera.up = pitch * camera.up;
            // now we do something different depending on whether the camera is locked
            if lock_z_up {
                // First, the camera might need to be reset. We can check on the z
                // component of the right axis to know if the camera is not straight up anymore
                let tilt_angle = camera_right.z.asin();
                if tilt_angle.abs() > 42.0 * std::f32::EPSILON {
                    // do not de-tilt in one single shot, de-tilt a bit for each frame, this way
                    // the user can at least understand what is happening
                    let untilt = glam::Mat3::from_axis_angle(pos_on_sphere, -0.2 * tilt_angle);
                    camera.up = untilt * camera.up;
                }
                // then we need to check if the camera went upside-down. If that happened,
                // we rotate around the camera_right axis to bring the camera and the position
                // on the sphere to the correct values
                if camera.up.z < 42.0 * std::f32::EPSILON {
                    let angle = pos_on_sphere.z.signum() * camera.up.z.atan2(Vec3::new(camera.up.x, camera.up.y, 0.0).length());
                    // same as before, apply rotation a bit per frame
                    let rot_around_right = glam::Mat3::from_axis_angle(camera_right, -0.25 * angle);
                    camera.up = rot_around_right * camera.up;
                    pos_on_sphere = rot_around_right * pos_on_sphere;
                }

                // after fixing the camera, we can do standard processing of user input
                let rot_z = glam::Mat3::from_rotation_z(x_delta);
                pos_on_sphere = rot_z * pos_on_sphere;
                camera.up = rot_z * camera.up;
            } else {
                let yaw = glam::Mat3::from_axis_angle(camera.up, x_delta);
                pos_on_sphere = yaw * pos_on_sphere;
            };
            pos_on_sphere = pos_on_sphere.normalize();
            camera.up = camera.up.normalize();
        } else if inputs.mouse_middle_click {
            // panning
            let camera_right = camera.up.cross(pos_on_sphere).normalize();
            // BEWARE: camera_up might be different from camera.up!
            // camera.up might be locked due to user settings (camera.up might be locked to z axis
            let camera_up = camera_right.cross(-pos_on_sphere).normalize();
            let pan_coeff = 0.001 * distance;
            let x_delta = inputs.mouse_motion.0 as f32 * pan_coeff * sensitivity.camera_horizontal;
            let y_delta = inputs.mouse_motion.1 as f32 * pan_coeff * sensitivity.camera_vertical;
            let position_delta = y_delta * camera_up - x_delta * camera_right;
            camera.target += position_delta;
        }
        camera.eye = camera.target + pos_on_sphere * distance;
    }
}


