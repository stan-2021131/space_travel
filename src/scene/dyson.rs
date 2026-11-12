use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;
use std::sync::Arc;

/// Estructura central de la Esfera de Dyson que envuelve al Sol.
#[derive(Debug, Clone)]
pub struct DysonSphere {
    pub meshes: Vec<Arc<Mesh>>,
    pub scale: f32,
    pub radius: f32,
    pub rotation_speed: f32,
    pub current_rotation_rad: f32,
    pub collision_radius: f32,
    pub gravity_radius: f32,
    pub gravity_strength: f32,
}

impl DysonSphere {
    pub fn new(
        meshes: Vec<Arc<Mesh>>,
        radius: f32,
        rotation_speed: f32,
        collision_radius: f32,
        gravity_radius: f32,
        gravity_strength: f32,
    ) -> Self {
        let geo_radius = meshes
            .iter()
            .map(|m| m.geometric_radius())
            .fold(0.0f32, f32::max)
            .max(1e-4);
        let scale = radius / geo_radius;

        Self {
            meshes,
            scale,
            radius,
            rotation_speed,
            current_rotation_rad: 0.0,
            collision_radius,
            gravity_radius,
            gravity_strength,
        }
    }

    /// Actualiza la rotación lenta de la esfera.
    pub fn update(&mut self, dt: f32) {
        self.current_rotation_rad = (self.current_rotation_rad + self.rotation_speed * dt)
            .rem_euclid(std::f32::consts::TAU);
    }

    /// Matriz de modelo de la esfera centrada en el Sol: $M = T \cdot R_y \cdot S$.
    pub fn model_matrix(&self, center: Vec3) -> Mat4 {
        Mat4::from_translation(center) * self.rotation_matrix() * Mat4::uniform_scaling(self.scale)
    }

    /// Matriz de rotación pura de la esfera sobre su eje vertical.
    pub fn rotation_matrix(&self) -> Mat4 {
        Mat4::rotation_y(self.current_rotation_rad)
    }
}
