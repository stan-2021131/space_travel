use crate::math::{Mat4, Vec3, deg_to_rad};
use crate::mesh::Mesh;
use crate::scene::surface::hash_f32;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DebrisInstance {
    pub mesh_index: usize,
    pub base_angle_rad: f32,
    pub orbital_radius: f32,
    pub height: f32,
    pub scale: f32,
    pub initial_rotation: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Debug, Clone)]
pub struct DebrisRing {
    pub meshes: Vec<Arc<Mesh>>,
    pub instances: Vec<DebrisInstance>,
    pub tilt_rad: f32,
    pub rotation_period: f32,
    pub outer_radius: f32,
    pub max_scale: f32,
    pub collision_margin: f32,
}

impl DebrisRing {
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        meshes: Vec<Arc<Mesh>>,
        seed: u32,
        count: usize,
        inner_radius: f32,
        outer_radius: f32,
        tilt_deg: f32,
        rotation_period: f32,
        min_scale: f32,
        max_scale: f32,
        height_variation: f32,
        min_spin_period: f32,
        max_spin_period: f32,
        collision_margin: f32,
    ) -> Self {
        assert!(!meshes.is_empty(), "Un anillo requiere al menos una malla");
        assert!(count > 0, "Un anillo requiere al menos una instancia");
        let mut instances = Vec::with_capacity(count);
        let angle_step = std::f32::consts::TAU / count as f32;

        for index in 0..count {
            let i = index as u32;
            let angle_jitter = (hash_f32(i, seed ^ 0xA511_E9B3) - 0.5) * angle_step * 0.55;
            let radius_t = hash_f32(i, seed ^ 0x63D8_35A7);
            let height_t = hash_f32(i, seed ^ 0xC2B2_AE35);
            let scale_t = hash_f32(i, seed ^ 0x27D4_EB2F);
            let spin_t = hash_f32(i, seed ^ 0x1656_67B1);
            let spin_period = min_spin_period + (max_spin_period - min_spin_period) * spin_t;
            let spin_speed = std::f32::consts::TAU / spin_period;

            let sign_x = if hash_f32(i, seed ^ 0x9E37_79B9) < 0.5 {
                -1.0
            } else {
                1.0
            };
            let sign_y = if hash_f32(i, seed ^ 0x85EB_CA6B) < 0.5 {
                -1.0
            } else {
                1.0
            };
            let sign_z = if hash_f32(i, seed ^ 0xD3A2_646C) < 0.5 {
                -1.0
            } else {
                1.0
            };

            instances.push(DebrisInstance {
                mesh_index: index % meshes.len(),
                base_angle_rad: index as f32 * angle_step + angle_jitter,
                orbital_radius: inner_radius + (outer_radius - inner_radius) * radius_t,
                height: (height_t * 2.0 - 1.0) * height_variation,
                scale: min_scale + (max_scale - min_scale) * scale_t,
                initial_rotation: Vec3::new(
                    hash_f32(i, seed ^ 0x94D0_49BB) * std::f32::consts::TAU,
                    hash_f32(i, seed ^ 0x369D_EA0F) * std::f32::consts::TAU,
                    hash_f32(i, seed ^ 0xDB4F_0B91) * std::f32::consts::TAU,
                ),
                angular_velocity: Vec3::new(
                    sign_x * spin_speed * 0.47,
                    sign_y * spin_speed * 0.73,
                    sign_z * spin_speed,
                ),
            });
        }

        Self {
            meshes,
            instances,
            tilt_rad: deg_to_rad(tilt_deg),
            rotation_period,
            outer_radius,
            max_scale,
            collision_margin,
        }
    }

    fn orbital_rotation(&self, instance: &DebrisInstance, sim_time: f32) -> Mat4 {
        let ring_angle = std::f32::consts::TAU * sim_time / self.rotation_period;
        Mat4::rotation_z(self.tilt_rad) * Mat4::rotation_y(instance.base_angle_rad + ring_angle)
    }

    pub fn instance_world_position(
        &self,
        body_position: Vec3,
        instance: &DebrisInstance,
        sim_time: f32,
    ) -> Vec3 {
        let local_position = Vec3::new(instance.orbital_radius, instance.height, 0.0);
        body_position
            + self
                .orbital_rotation(instance, sim_time)
                .transform_point(local_position)
    }

    pub fn instance_matrices(
        &self,
        body_position: Vec3,
        instance: &DebrisInstance,
        sim_time: f32,
    ) -> (Mat4, Mat4) {
        let world_position = self.instance_world_position(body_position, instance, sim_time);
        let rotation = Vec3::new(
            instance.initial_rotation.x + instance.angular_velocity.x * sim_time,
            instance.initial_rotation.y + instance.angular_velocity.y * sim_time,
            instance.initial_rotation.z + instance.angular_velocity.z * sim_time,
        );
        let spin = Mat4::rotation_euler(rotation.y, rotation.x, rotation.z);
        let model =
            Mat4::from_translation(world_position) * spin * Mat4::uniform_scaling(instance.scale);
        (model, spin)
    }

    pub fn culling_radius(&self) -> f32 {
        let max_mesh_radius = self
            .meshes
            .iter()
            .map(|mesh| mesh.geometric_radius())
            .fold(0.0f32, f32::max);
        self.outer_radius + max_mesh_radius * self.max_scale
    }

    pub fn triangle_count(&self) -> usize {
        self.instances
            .iter()
            .map(|instance| self.meshes[instance.mesh_index].triangle_count())
            .sum()
    }
}
