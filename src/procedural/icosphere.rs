use std::collections::HashMap;
use std::f32::consts::PI;

use crate::math::{Vec2, Vec3};
use crate::mesh::{Mesh, Vertex};

/// Genera analíticamente una esfera geodésica (Icosphere) procedural mediante subdivisión recursiva
/// de un icosaedro regular de 12 vértices y 20 caras.
///
/// # Propiedades matemáticas:
/// - Cada nivel de subdivisión cuadruplica el número de caras ($F_N = 20 \cdot 4^N$).
/// - Los vértices siguen la progresión exacta $V_N = 10 \cdot 4^N + 2$.
/// - Deduplica aristas compartidas mediante un caché asociativo garantizando una variedad cerrada ($V - E + F = 2$).
/// - Todas las normales son puramente radiales unitarias y el devanado es antihorario (CCW) hacia el exterior.
pub fn generate_icosphere(radius: f32, subdivisions: u32) -> Mesh {
    // Razón áurea para la construcción del icosaedro regular
    let phi = (1.0 + 5.0f32.sqrt()) * 0.5;

    let base_positions = [
        Vec3::new(-1.0, phi, 0.0),
        Vec3::new(1.0, phi, 0.0),
        Vec3::new(-1.0, -phi, 0.0),
        Vec3::new(1.0, -phi, 0.0),
        Vec3::new(0.0, -1.0, phi),
        Vec3::new(0.0, 1.0, phi),
        Vec3::new(0.0, -1.0, -phi),
        Vec3::new(0.0, 1.0, -phi),
        Vec3::new(phi, 0.0, -1.0),
        Vec3::new(phi, 0.0, 1.0),
        Vec3::new(-phi, 0.0, -1.0),
        Vec3::new(-phi, 0.0, 1.0),
    ];

    #[inline(always)]
    fn calc_uv(n: Vec3) -> Vec2 {
        let u = 0.5 + n.z.atan2(n.x) / (2.0 * PI);
        let v = 0.5 - n.y.clamp(-1.0, 1.0).asin() / PI;
        Vec2::new(u, v)
    }

    let mut vertices: Vec<Vertex> = base_positions
        .iter()
        .map(|&p| {
            let norm = p.normalize();
            let pos = norm * radius;
            let uv = calc_uv(norm);
            Vertex::new(pos, norm, uv)
        })
        .collect();

    // 20 caras con devanado CCW mirando desde el exterior
    let mut indices: Vec<u32> = vec![
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, 1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7,
        1, 8, 3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, 4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9,
        8, 1,
    ];

    let mut edge_cache: HashMap<(u32, u32), u32> = HashMap::new();

    for _ in 0..subdivisions {
        edge_cache.clear();
        let mut next_indices = Vec::with_capacity(indices.len() * 4);

        for chunk in indices.chunks_exact(3) {
            let i0 = chunk[0];
            let i1 = chunk[1];
            let i2 = chunk[2];

            let mut get_midpoint = |a: u32, b: u32| -> u32 {
                let key = (a.min(b), a.max(b));
                if let Some(&idx) = edge_cache.get(&key) {
                    idx
                } else {
                    let pa = vertices[a as usize].position;
                    let pb = vertices[b as usize].position;
                    let mid_norm = (pa + pb).normalize();
                    let mid_pos = mid_norm * radius;
                    let mid_uv = calc_uv(mid_norm);
                    let new_v = Vertex::new(mid_pos, mid_norm, mid_uv);
                    vertices.push(new_v);
                    let idx = (vertices.len() - 1) as u32;
                    edge_cache.insert(key, idx);
                    idx
                }
            };

            let m01 = get_midpoint(i0, i1);
            let m12 = get_midpoint(i1, i2);
            let m20 = get_midpoint(i2, i0);

            next_indices.extend_from_slice(&[i0, m01, m20]);
            next_indices.extend_from_slice(&[i1, m12, m01]);
            next_indices.extend_from_slice(&[i2, m20, m12]);
            next_indices.extend_from_slice(&[m01, m12, m20]);
        }

        indices = next_indices;
    }

    let mut mesh = Mesh::new(vertices, indices);
    mesh.calculate_tangents();
    mesh.update_aabb();
    mesh
}
