use crate::math::{Vec2, Vec3};
use crate::mesh::{Mesh, Vertex};

/// Genera una llama cónica unitaria orientada desde `z = 0` hacia `-Z`.
/// La escala y orientación finales se aplican mediante la matriz de modelo.
pub fn generate_thruster_flame(segments: u32) -> Mesh {
    let segments = segments.max(3);
    let mut vertices = Vec::with_capacity(segments as usize + 2);
    let mut indices = Vec::with_capacity(segments as usize * 6);

    for index in 0..segments {
        let u = index as f32 / segments as f32;
        let angle = u * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        let normal = Vec3::new(cos, sin, 1.0).normalize();
        vertices.push(Vertex::new(
            Vec3::new(cos, sin, 0.0),
            normal,
            Vec2::new(u, 0.0),
        ));
    }

    let tip_index = vertices.len() as u32;
    vertices.push(Vertex::new(
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec2::new(0.5, 1.0),
    ));
    let cap_index = vertices.len() as u32;
    vertices.push(Vertex::new(
        Vec3::zero(),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.5, 0.0),
    ));

    for index in 0..segments {
        let next = (index + 1) % segments;
        // Devanado exterior de la pared y tapa frontal del cono.
        indices.extend_from_slice(&[index, tip_index, next]);
        indices.extend_from_slice(&[cap_index, index, next]);
    }

    Mesh::new(vertices, indices)
}
