use crate::material::Material;
use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;
use std::sync::Arc;

pub mod surface;
pub use surface::{SurfaceGroup, SurfaceInstance, calculate_surface_embedding};

/// Representa un cuerpo celeste (Sol o Planeta) en la simulación orbital.
#[derive(Debug, Clone)]
pub struct CelestialBody {
    pub id: String,
    pub mesh: Arc<Mesh>,
    pub material: Material,
    pub scale: f32,
    pub radius: f32,
    pub collision_margin: f32,
    pub orbital_radius: f32,
    pub orbital_period: f32,
    pub rotation_period: f32,
    pub initial_orbital_angle_rad: f32,
    pub position: Vec3,
    pub rotation_rad: f32,
    pub shader_type: String,
    pub surface_groups: Vec<SurfaceGroup>,
    pub gravity_strength: f32,
    pub gravity_radius: f32,
    pub visible_gravity: bool,
    pub gravity_mesh: Option<Arc<Mesh>>,
}

impl CelestialBody {
    /// Actualiza la posición orbital en el plano XZ y la rotación axial en Y según el tiempo de simulación.
    pub fn update(&mut self, sim_time: f32) {
        let tau = std::f32::consts::TAU;
        if self.orbital_radius > 0.0 && self.orbital_period > 0.0 {
            let orbit_angle =
                self.initial_orbital_angle_rad + (tau / self.orbital_period) * sim_time;
            self.position = Vec3::new(
                self.orbital_radius * orbit_angle.cos(),
                0.0,
                self.orbital_radius * orbit_angle.sin(),
            );
        }
        if self.rotation_period > 0.0 {
            self.rotation_rad = (tau / self.rotation_period) * sim_time;
        }
    }

    /// Retorna la matriz de modelo completa para colocar el cuerpo en el mundo: $M = T \cdot R_y \cdot S$.
    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::rotation_y(self.rotation_rad)
            * Mat4::uniform_scaling(self.scale)
    }

    /// Retorna la matriz de rotación pura para transformar normales con escala uniforme.
    pub fn rotation_matrix(&self) -> Mat4 {
        Mat4::rotation_y(self.rotation_rad)
    }

    /// Retorna la matriz de modelo para la esfera de campo gravitatorio (centrada en el cuerpo, escalada a gravity_radius).
    pub fn gravity_field_model_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::uniform_scaling(self.gravity_radius)
    }
}
