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

        // Giro pseudoaleatorio sobre la normal (yaw local)
        let yaw_angle = hash_f32(i as u32, seed) * std::f32::consts::TAU;

        instances.push(create_surface_instance(
            local_pos,
            local_normal,
            base_scale,
            yaw_angle,
        ));
    }

    instances
}

/// Construye una instancia de superficie orientada con base ortonormal estable a lo largo de la normal local.
pub fn create_surface_instance(
    local_pos: Vec3,
    local_normal: Vec3,
    base_scale: f32,
    yaw_angle: f32,
) -> SurfaceInstance {
    let up = local_normal.normalize();
    let reference = if up.y.abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let right = reference.cross(up).normalize();
    let forward = right.cross(up).normalize();

    let basis = Mat4::from_cols(
        Vec4::new(right.x, right.y, right.z, 0.0),
        Vec4::new(up.x, up.y, up.z, 0.0),
        Vec4::new(forward.x, forward.y, forward.z, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    );

    let yaw_rot = Mat4::rotation_y(yaw_angle);
    let local_basis = basis * yaw_rot;

    SurfaceInstance {
        local_pos,
        local_normal: up,
        local_basis,
        base_scale,
    }
}

/// Genera una distribución uniforme y determinista de instancias asentadas sobre la corteza superior
/// de un planeta toroidal (dona), orientadas con la normal de la superficie y con base ortonormal estable.
pub fn generate_torus_surface_distribution(
    seed: u32,
    count: usize,
    planet_mesh: &Mesh,
    planet_scale: f32,
    base_scale: f32,
    embedding: f32,
) -> Vec<SurfaceInstance> {
    if count == 0 || planet_mesh.vertices.is_empty() {
        return Vec::new();
    }

    // Filtrar vértices del lomo superior exterior de la dona (y > 0.04)
    let candidates: Vec<&crate::mesh::Vertex> = planet_mesh
        .vertices
        .iter()
        .filter(|v| v.position.y > 0.04)
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    let seed_offset = hash_f32(0x85EB_CA6B, seed) * std::f32::consts::TAU;
    let mut instances = Vec::with_capacity(count);

    for i in 0..count {
        let target_angle = seed_offset + (i as f32 * std::f32::consts::TAU / (count as f32));
        // Encontrar el vértice candidato más cercano al ángulo azimutal objetivo
        let mut best_vertex = candidates[0];
        let mut best_diff = f32::MAX;

        for &v in &candidates {
            let angle = v.position.z.atan2(v.position.x);
            let mut diff = (angle - target_angle).rem_euclid(std::f32::consts::TAU);
            if diff > std::f32::consts::PI {
                diff = std::f32::consts::TAU - diff;
            }
            if diff < best_diff {
                best_diff = diff;
                best_vertex = v;
            }
        }

        let normal = if best_vertex.normal.length_squared() > 1e-4 {
            best_vertex.normal.normalize()
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };

        let local_pos = best_vertex.position * planet_scale - normal * embedding;
        let yaw_angle = hash_f32(i as u32, seed) * std::f32::consts::TAU;

        instances.push(create_surface_instance(
            local_pos, normal, base_scale, yaw_angle,
        ));
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
