use crate::math::{Vec2, Vec3, Vec4};

/// Vértice en espacio de recorte homogéneo (clip space) antes de la división de perspectiva.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipVertex {
    pub clip_pos: Vec4,
    pub world_pos: Vec3,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub uv: Vec2,
}

impl ClipVertex {
    pub fn new(clip_pos: Vec4, world_pos: Vec3, normal: Vec3, tangent: Vec3, uv: Vec2) -> Self {
        Self {
            clip_pos,
            world_pos,
            normal,
            tangent,
            uv,
        }
    }

    /// Interpola linealmente entre dos vértices de recorte con parámetro t in [0, 1].
    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            clip_pos: a.clip_pos.lerp(b.clip_pos, t),
            world_pos: a.world_pos.lerp(b.world_pos, t),
            normal: a.normal.lerp(b.normal, t).normalize(),
            tangent: a.tangent.lerp(b.tangent, t).normalize(),
            uv: a.uv.lerp(b.uv, t),
        }
    }
}

/// Triángulo en espacio de recorte.
#[derive(Debug, Clone, Copy)]
pub struct ClipTriangle {
    pub v: [ClipVertex; 3],
}

impl ClipTriangle {
    pub fn new(v0: ClipVertex, v1: ClipVertex, v2: ClipVertex) -> Self {
        Self { v: [v0, v1, v2] }
    }
}

/// Resultado de la etapa de recorte contra el plano cercano (0, 1 o 2 triángulos).
#[derive(Debug)]
pub enum ClippedTriangles {
    Zero,
    One(ClipTriangle),
    Two(ClipTriangle, ClipTriangle),
}

/// Recorta un triángulo contra el plano cercano del frustum de visión (Near Plane Clipping).
///
/// En la convención de proyección con NDC z in [-1, 1], la condición del plano cercano es:
///   z_clip >= -w_clip   =>   z_clip + w_clip >= 0
///
/// Distancia con signo: d = z_clip + w_clip.
/// - Si d >= 0: el vértice está dentro (frente al plano cercano).
/// - Si d < 0: el vértice está fuera (detrás del plano cercano).
pub fn clip_triangle_near_plane(tri: &ClipTriangle) -> ClippedTriangles {
    // Calculamos distancias con signo para cada vértice
    let d0 = tri.v[0].clip_pos.z + tri.v[0].clip_pos.w;
    let d1 = tri.v[1].clip_pos.z + tri.v[1].clip_pos.w;
    let d2 = tri.v[2].clip_pos.z + tri.v[2].clip_pos.w;

    let in0 = d0 >= 0.0;
    let in1 = d1 >= 0.0;
    let in2 = d2 >= 0.0;

    let in_count = in0 as u8 + in1 as u8 + in2 as u8;

    match in_count {
        3 => {
            // Todos los vértices están dentro: el triángulo pasa intacto
            ClippedTriangles::One(*tri)
        }
        0 => {
            // Todos los vértices están detrás del plano cercano: se descarta por completo
            ClippedTriangles::Zero
        }
        1 => {
            // Un vértice dentro y dos fuera: se recorta produciendo 1 triángulo nuevo
            let (a, b, c, da, db, dc) = if in0 {
                (tri.v[0], tri.v[1], tri.v[2], d0, d1, d2)
            } else if in1 {
                (tri.v[1], tri.v[2], tri.v[0], d1, d2, d0)
            } else {
                (tri.v[2], tri.v[0], tri.v[1], d2, d0, d1)
            };

            let t_ab = da / (da - db);
            let t_ac = da / (da - dc);

            let i_ab = ClipVertex::lerp(&a, &b, t_ab);
            let i_ac = ClipVertex::lerp(&a, &c, t_ac);

            ClippedTriangles::One(ClipTriangle::new(a, i_ab, i_ac))
        }
        2 => {
            // Dos vértices dentro y uno fuera: se recorta el cuadrilátero produciendo 2 triángulos
            let (a, b, c, da, db, dc) = if !in2 {
                (tri.v[0], tri.v[1], tri.v[2], d0, d1, d2)
            } else if !in0 {
                (tri.v[1], tri.v[2], tri.v[0], d1, d2, d0)
            } else {
                (tri.v[2], tri.v[0], tri.v[1], d2, d0, d1)
            };
            // 'a' y 'b' están dentro, 'c' está fuera

            let t_ac = da / (da - dc);
            let t_bc = db / (db - dc);

            let i_ac = ClipVertex::lerp(&a, &c, t_ac);
            let i_bc = ClipVertex::lerp(&b, &c, t_bc);

            // Cuadrilátero a -> b -> i_bc -> i_ac descompuesto en dos triángulos CCW:
            let t1 = ClipTriangle::new(a, b, i_ac);
            let t2 = ClipTriangle::new(b, i_bc, i_ac);

            ClippedTriangles::Two(t1, t2)
        }
        _ => unreachable!(),
    }
}
