use crate::material::Material;
use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;
use std::sync::Arc;

/// Satélite que acompaña la traslación de su planeta padre con revolución y giro propios.
#[derive(Debug, Clone)]
pub struct Moon {
    pub name: String,
    pub mesh: Arc<Mesh>,
    pub material: Material,
    pub scale: f32,
    pub radius: f32,
    pub orbital_radius: f32,
    pub orbital_period: f32,
    pub rotation_period: f32,
    pub initial_orbital_angle_rad: f32,
    pub orbit_tilt_rad: f32,
    pub collision_margin: f32,
}

impl Moon {
    fn orbit_rotation(&self, sim_time: f32) -> Mat4 {
        let angle =
            self.initial_orbital_angle_rad + std::f32::consts::TAU * sim_time / self.orbital_period;
        Mat4::rotation_z(self.orbit_tilt_rad) * Mat4::rotation_y(angle)
    }

    pub fn world_position(&self, parent_position: Vec3, sim_time: f32) -> Vec3 {
        parent_position
            + self.orbit_rotation(sim_time).transform_direction(Vec3::new(
                self.orbital_radius,
                0.0,
                0.0,
            ))
    }

    /// Mantiene la cara de la estrella hacia afuera y aplica el giro sobre su normal local +Y.
    pub fn matrices(&self, parent_position: Vec3, sim_time: f32) -> (Mat4, Mat4) {
        let spin = std::f32::consts::TAU * sim_time / self.rotation_period;
        let rotation = self.orbit_rotation(sim_time)
            * Mat4::rotation_z(-std::f32::consts::FRAC_PI_2)
            * Mat4::rotation_y(spin);
        let model = Mat4::from_translation(self.world_position(parent_position, sim_time))
            * rotation
            * Mat4::uniform_scaling(self.scale);
        (model, rotation)
    }

    pub fn culling_radius(&self) -> f32 {
        self.orbital_radius + self.radius
    }
}
