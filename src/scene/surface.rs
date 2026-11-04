use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::{AABB, Mesh};
use std::sync::Arc;

/// Instancia de un objeto posicionado y orientado sobre la corteza esférica de un planeta.
#[derive(Debug, Clone)]
pub struct SurfaceInstance {
    /// Posición en coordenadas locales del planeta (sobre la corteza esférica)
    pub local_pos: Vec3,
    /// Normal unitaria hacia afuera de la superficie
    pub local_normal: Vec3,
    /// Matriz de orientación ortonormal con yaw determinista
    pub local_basis: Mat4,
    /// Escala base del objeto
    pub base_scale: f32,
}

/// Grupo de instancias de superficie que comparten la misma malla y parámetros de renderizado.
#[derive(Debug, Clone)]
pub struct SurfaceGroup {
    pub name: String,
    pub mesh: Arc<Mesh>,
    pub instances: Vec<SurfaceInstance>,
    pub max_distance: f32,
    pub fade_range: f32,
}

impl SurfaceGroup {
    pub fn new(
        name: String,
        mesh: Arc<Mesh>,
        instances: Vec<SurfaceInstance>,
        max_distance: f32,
        fade_range: f32,
    ) -> Self {
        Self {
            name,
            mesh,
            instances,
            max_distance,
            fade_range,
        }
    }
}

/// Genera una distribución pseudoaleatoria uniforme y determinista de instancias
/// sobre una superficie esférica utilizando el algoritmo de Espiral de Fibonacci.
pub fn generate_spherical_distribution(
    seed: u32,
    count: usize,
    planet_radius: f32,
    base_scale: f32,
) -> Vec<SurfaceInstance> {
    if count == 0 {
        return Vec::new();
    }

    let mut instances = Vec::with_capacity(count);
    // Ángulo áureo en radianes: pi * (3.0 - sqrt(5.0))
    let golden_angle = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());

    // Orientación global de la distribución esférica gobernada de forma determinista por la semilla
    let seed_yaw = hash_f32(0x9E37_79B9, seed) * std::f32::consts::TAU;
    let seed_pitch = (hash_f32(0x85EB_CA6B, seed) - 0.5) * std::f32::consts::PI;
    let seed_rot = Mat4::rotation_y(seed_yaw) * Mat4::rotation_x(seed_pitch);

    for i in 0..count {
        // Coordenada Y uniforme en [-1.0, 1.0]
        let y = 1.0 - (i as f32 * 2.0 + 1.0) / (count as f32);
        let radius_at_y = (1.0 - y * y).max(0.0).sqrt();
        let theta = golden_angle * (i as f32);

        let x = theta.cos() * radius_at_y;
        let z = theta.sin() * radius_at_y;

        let unrotated_dir = Vec3::new(x, y, z);
        let unit_dir = seed_rot.transform_direction(unrotated_dir).normalize();

        // Posición asentada sobre la superficie del planeta
        let local_pos = unit_dir * planet_radius;
        let local_normal = unit_dir;

        // Construcción de la base ortonormal:
        // El eje +Y del modelo coincidirá con la normal esférica hacia afuera
        let up = local_normal;
        let reference = if up.y.abs() < 0.99 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        let right = reference.cross(up).normalize();
        let forward = right.cross(up).normalize();

        // Base ortonormal como matriz de columnas:
        // Columna 0: right (eje +X)
        // Columna 1: up (eje +Y, normal del planeta)
        // Columna 2: forward (eje +Z)
        // Columna 3: componente homogénea afín sin traslación
        let basis = Mat4::from_cols(
            Vec4::new(right.x, right.y, right.z, 0.0),
            Vec4::new(up.x, up.y, up.z, 0.0),
            Vec4::new(forward.x, forward.y, forward.z, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        // Giro pseudoaleatorio sobre la normal (yaw local)
        let yaw_angle = hash_f32(i as u32, seed) * std::f32::consts::TAU;
        let yaw_rot = Mat4::rotation_y(yaw_angle);

        // La rotación en Y local gira el objeto y luego basis lo orienta sobre la esfera
        let local_basis = basis * yaw_rot;

        instances.push(SurfaceInstance {
            local_pos,
            local_normal,
            local_basis,
            base_scale,
        });
    }

    instances
}

/// Función hash pseudoaleatoria determinista para números en [0.0, 1.0).
#[inline]
pub fn hash_f32(index: u32, seed: u32) -> f32 {
    let mut h = index.wrapping_mul(374761393) ^ seed.wrapping_mul(1103515245);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h as f32) / (u32::MAX as f32)
}

/// Calcula el factor de transición suave (0.0 a 1.0) según la distancia a la superficie del planeta.
#[inline]
pub fn calculate_fade_factor(distance_to_surface: f32, max_distance: f32, fade_range: f32) -> f32 {
    if fade_range <= 1e-6 {
        return if distance_to_surface < max_distance {
            1.0
        } else {
            0.0
        };
    }

    if distance_to_surface >= max_distance {
        0.0
    } else if distance_to_surface <= (max_distance - fade_range) {
        1.0
    } else {
        let t = (max_distance - distance_to_surface) / fade_range;
        // Suavizado cúbico smoothstep
        t * t * (3.0 - 2.0 * t)
    }
}

/// Calcula la profundidad geométrica de enraizamiento necesaria para que un objeto
/// con base plana no flote sobre una superficie esférica curva ni sobre sus facetas poligonales.
pub fn calculate_surface_embedding(planet_radius: f32, mesh_aabb: &AABB, scale: f32) -> f32 {
    let half_w_x = mesh_aabb.size().x * 0.5 * scale;
    let half_w_z = mesh_aabb.size().z * 0.5 * scale;
    let r_base = half_w_x.max(half_w_z).min(planet_radius * 0.85);

    // Sagita de curvatura esférica: R - sqrt(R^2 - r^2)
    let sagitta = planet_radius
        - (planet_radius * planet_radius - r_base * r_base)
            .max(0.0)
            .sqrt();

    // Margen de seguridad para absorber la distancia entre las cuerdas de las facetas poligonales del planeta
    let facet_margin = 0.015 * planet_radius;

    // Compensación si el origen de la malla en Y no se encuentra en su base inferior
    let base_offset = (-mesh_aabb.min.y * scale).max(0.0);

    sagitta + facet_margin + base_offset
}
