use glam::{Vec3, Mat4};
use winit::event::*;

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    yaw: f32,
    pitch: f32,
    pub up: Vec3,
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            position: Vec3::new(-1.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
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
        let direction =
            Vec3::new(
                self.yaw.cos()*self.pitch.cos(),
                self.yaw.sin()*self.pitch.cos(),
                self.pitch.sin(),
            ).normalize();
        let lookat_matrix = Mat4::look_at_lh(
            self.position,
            self.position + direction,
            self.up,
        );
        let proj_matrix = Mat4::perspective_lh(self.fov_y, self.aspect, self.z_near, self.z_far);
        // let proj_matrix = Mat4::orthographic_lh(-4.0, 4.0, -2.5, 2.5, -10.0, 10.0);

        proj_matrix * lookat_matrix
    }
}

// TODO: camera controller movement currently depends on the frame dt. However,
// rotation should NOT depend on it, since it depends on how many pixel I dragged
// over the rendered scene, which kinda makes it already framerate-agnostic
// OTOH, we might want to make it frame *dimension* agnostic!
#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) {
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
            }
            VirtualKeyCode::LShift => {
                self.amount_down = amount;
            }
            _ => {},
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.rotate_horizontal = mouse_dx;
        self.rotate_vertical = mouse_dy;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vec3::new(yaw_cos, yaw_sin, 0.0).normalize();
        let right = Vec3::new(-yaw_sin, yaw_cos, 0.0).normalize();
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        camera.position.z += (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += self.rotate_horizontal * self.sensitivity * dt;
        camera.pitch += -self.rotate_vertical * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -std::f32::consts::FRAC_PI_2 {
            camera.pitch = -std::f32::consts::FRAC_PI_2;
        } else if camera.pitch > std::f32::consts::FRAC_PI_2 {
            camera.pitch = std::f32::consts::FRAC_PI_2;
        }
    }
}

