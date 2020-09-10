use ultraviolet::{Vec3, Mat4};

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> ultraviolet::Mat4 {
        let lookat_matrix = Mat4::look_at(self.eye, self.target, self.up);
        let proj_matrix = ultraviolet::projection::perspective_dx(self.fov_y, self.aspect, self.z_near, self.z_far);

        proj_matrix * lookat_matrix
    }
}

