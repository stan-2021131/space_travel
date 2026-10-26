use crate::math::{Mat4, Vec3, deg_to_rad};

/// Cámara en el espacio 3D para la generación de matrices de vista y proyección.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_rad: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        fov_deg: f32,
        aspect: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up: up.normalize(),
            fov_y_rad: deg_to_rad(fov_deg),
            aspect,
            z_near,
            z_far,
        }
    }

    /// Calcula la matriz de vista (View Matrix) mediante Look-At.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye, self.target, self.up)
    }

    /// Calcula la matriz de proyección en perspectiva estándar (OpenGL-style NDC [-1, 1]).
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov_y_rad, self.aspect, self.z_near, self.z_far)
    }

    /// Calcula la matriz combinada de Vista-Proyección: $VP = P \cdot V$.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}
