use crate::material::Material;
use crate::math::{Mat4, Vec3};
use crate::mesh::Mesh;
use std::sync::Arc;

pub mod debris_ring;
pub mod dyson;
pub mod surface;
pub use debris_ring::{DebrisInstance, DebrisRing};
pub use dyson::DysonSphere;
pub use surface::{
    SurfaceGroup, SurfaceInstance, calculate_surface_embedding, create_surface_instance,
    generate_torus_surface_distribution,
};

#[derive(Debug, Clone)]
pub struct CelestialComponent {
    pub name: String,
    pub mesh: Arc<Mesh>,
    /// Radio máximo desde el origen compartido del cuerpo compuesto.
    pub origin_radius: f32,
}

impl CelestialComponent {
    pub fn new(name: String, mesh: Arc<Mesh>) -> Self {
        let origin_radius = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position.length())
            .fold(0.0f32, f32::max);
        Self {
            name,
            mesh,
            origin_radius,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThrusterFlame {
    pub position: Vec3,
    pub rotation: Mat4,
    pub radius: f32,
    pub length: f32,
    pub animation_speed: f32,
    pub idle_intensity: f32,
    pub active_intensity: f32,
}

impl ThrusterFlame {
    pub fn from_degrees(
        position: [f32; 3],
        rotation_deg: [f32; 3],
        radius: f32,
        length: f32,
        animation_speed: f32,
        idle_intensity: f32,
        active_intensity: f32,
    ) -> Self {
        Self {
            position: Vec3::new(position[0], position[1], position[2]),
            rotation: Mat4::rotation_euler(
                rotation_deg[1].to_radians(),
                rotation_deg[0].to_radians(),
                rotation_deg[2].to_radians(),
            ),
            radius,
            length,
            animation_speed,
            idle_intensity,
            active_intensity,
        }
    }

    pub fn local_model_matrix(&self, intensity: f32, flicker: f32) -> Mat4 {
        let intensity = intensity.clamp(0.05, 1.0);
        Mat4::from_translation(self.position)
            * self.rotation
            * Mat4::scaling(
                self.radius * (0.75 + intensity * 0.25),
                self.radius * (0.75 + intensity * 0.25),
                self.length * intensity * flicker,
            )
    }

    pub fn origin_extent(&self) -> f32 {
        self.position.length() + (self.radius * self.radius + self.length * self.length).sqrt()
    }
}

/// Representa un cuerpo celeste (Sol o Planeta) en la simulación orbital.
#[derive(Debug, Clone)]
pub struct CelestialBody {
    pub id: String,
    pub parent_id: Option<String>,
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
    pub debris_rings: Vec<DebrisRing>,
    pub components: Vec<CelestialComponent>,
    pub thruster_flames: Vec<ThrusterFlame>,
    pub gravity_strength: f32,
    pub gravity_radius: f32,
    pub visible_gravity: bool,
    pub axial_tilt_rad: f32,
    pub gravity_mesh: Option<Arc<Mesh>>,
}

impl CelestialBody {
    /// Actualiza la posición orbital en el plano XZ y la rotación axial en Y según el tiempo de simulación,
    /// orbitando alrededor de un centro arbitrario (por defecto el origen para planetas, o la posición del padre para lunas).
    pub fn update_with_parent_position(&mut self, sim_time: f32, center: Vec3) {
        let tau = std::f32::consts::TAU;
        if self.orbital_radius > 0.0 && self.orbital_period > 0.0 {
            let orbit_angle =
                self.initial_orbital_angle_rad + (tau / self.orbital_period) * sim_time;
            self.position = center
                + Vec3::new(
                    self.orbital_radius * orbit_angle.cos(),
                    0.0,
                    self.orbital_radius * orbit_angle.sin(),
                );
        } else {
            self.position = center;
        }
        if self.rotation_period > 0.0 {
            self.rotation_rad = (tau / self.rotation_period) * sim_time;
        }
    }

    /// Actualiza la posición orbital en el plano XZ y la rotación axial en Y según el tiempo de simulación con centro en el origen.
    pub fn update(&mut self, sim_time: f32) {
        self.update_with_parent_position(sim_time, Vec3::zero());
    }

    /// Retorna la matriz de modelo completa para colocar el cuerpo en el mundo: $M = T \cdot R \cdot S$.
    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position)
            * self.rotation_matrix()
            * Mat4::uniform_scaling(self.scale)
    }

    /// Retorna la matriz de rotación pura para transformar normales con escala uniforme.
    pub fn rotation_matrix(&self) -> Mat4 {
        if self.axial_tilt_rad.abs() > 1e-4 {
            Mat4::rotation_x(self.axial_tilt_rad) * Mat4::rotation_y(self.rotation_rad)
        } else {
            Mat4::rotation_y(self.rotation_rad)
        }
    }

    /// Retorna la matriz de modelo para la esfera de campo gravitatorio (centrada en el cuerpo, escalada a gravity_radius).
    pub fn gravity_field_model_matrix(&self) -> Mat4 {
        Mat4::from_translation(self.position) * Mat4::uniform_scaling(self.gravity_radius)
    }

    /// Radio envolvente del cuerpo principal y todos sus componentes con origen compartido.
    pub fn composite_radius(&self) -> f32 {
        self.components
            .iter()
            .map(|component| component.origin_radius * self.scale)
            .fold(self.radius, f32::max)
    }

    /// Radio visual requerido para no descartar llamas que sobresalen del cuerpo sólido.
    pub fn thruster_flame_radius(&self) -> f32 {
        self.thruster_flames
            .iter()
            .map(|flame| flame.origin_extent() * self.scale)
            .fold(self.radius, f32::max)
    }
}
