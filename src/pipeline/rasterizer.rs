use crate::buffer::{DepthBuffer, Framebuffer};
use crate::material::Material;
use crate::math::{Vec2, Vec3, Vec4};
use crate::pipeline::clipping::ClipVertex;
use crate::pipeline::shaders::{FragmentInput, FragmentShader};
use crate::stats::FrameStats;

/// Vértice resultante de la división de perspectiva y conversión a pantalla,
/// con atributos precalculados para interpolación afín y con corrección de perspectiva.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexOut {
    pub clip_pos: Vec4,
    pub screen_pos: Vec2,
    pub inv_w: f32,
    pub depth_ndc: f32,
    pub world_pos: Vec3,
    pub normal: Vec3,
    pub tangent: Vec3,
    pub uv: Vec2,
    // Atributos divididos por w para interpolación con corrección de perspectiva en pantalla:
    pub normal_div_w: Vec3,
    pub tangent_div_w: Vec3,
    pub world_pos_div_w: Vec3,
    pub uv_div_w: Vec2,
}

/// Modo de mezcla para la rasterización de fragmentos en el framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Opaco estándar: Z-Test con Z-Write y sobrescritura del píxel.
    #[default]
    Opaque,
    /// Semitransparente: Z-Test sin Z-Write y mezcla alfa en el framebuffer.
    AlphaBlend,
}

impl VertexOut {
    /// Convierte un `ClipVertex` al espacio de pantalla (Viewport) realizando la división de perspectiva.
    pub fn from_clip_vertex(cv: &ClipVertex, width: f32, height: f32) -> Self {
        let w = cv.clip_pos.w;
        let inv_w = if w.abs() > 1e-7 { 1.0 / w } else { 1.0 };

        let ndc_x = cv.clip_pos.x * inv_w;
        let ndc_y = cv.clip_pos.y * inv_w;
        let depth_ndc = cv.clip_pos.z * inv_w;

        // Viewport transform: mapea NDC [-1, 1] a [0, width] x [0, height].
        // Nótese que NDC +Y apunta hacia arriba, mientras que en coordenadas de pantalla de framebuffer
        // la fila 0 está en el borde superior (Y crece hacia abajo).
        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - ndc_y) * 0.5 * height;

        Self {
            clip_pos: cv.clip_pos,
            screen_pos: Vec2::new(screen_x, screen_y),
            inv_w,
            depth_ndc,
            world_pos: cv.world_pos,
            normal: cv.normal,
            tangent: cv.tangent,
            uv: cv.uv,
            normal_div_w: cv.normal * inv_w,
            tangent_div_w: cv.tangent * inv_w,
            world_pos_div_w: cv.world_pos * inv_w,
            uv_div_w: cv.uv * inv_w,
        }
    }
}

/// Determina si un triángulo es frontal (Front-Facing) en el espacio de pantalla.
///
/// # Convención de devanado (Winding):
/// - En el espacio de mundo / NDC, las caras frontales tienen orden antihorario (CCW).
/// - Al mapear a la pantalla donde el eje Y está invertido (fila 0 arriba),
///   el área signada en pantalla resulta negativa para caras CCW frontales:
///   `cross_2d = (x1 - x0)*(y2 - y0) - (y1 - y0)*(x2 - x0)`.
/// - Si `cross_2d < -1e-6`, el triángulo está orientado hacia la cámara (front-facing).
/// - Si `cross_2d >= -1e-6`, es cara trasera (backface) o degenerado.
#[inline]
pub fn is_front_facing(v0: &VertexOut, v1: &VertexOut, v2: &VertexOut) -> bool {
    let cross_2d = (v1.screen_pos.x - v0.screen_pos.x) * (v2.screen_pos.y - v0.screen_pos.y)
        - (v1.screen_pos.y - v0.screen_pos.y) * (v2.screen_pos.x - v0.screen_pos.x);
    cross_2d < -1e-6
}

/// Calcula las coordenadas baricéntricas de un punto P respecto a los vértices A, B, C en pantalla.
///
/// Retorna `Some((lambda_0, lambda_1, lambda_2))` si el triángulo no es degenerado.
#[inline]
pub fn calculate_barycentric_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> Option<(f32, f32, f32)> {
    let delta = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if delta.abs() < 1e-7 {
        return None;
    }

    let inv_delta = 1.0 / delta;
    let w_a = ((b.x - p.x) * (c.y - p.y) - (b.y - p.y) * (c.x - p.x)) * inv_delta;
    let w_b = ((c.x - p.x) * (a.y - p.y) - (c.y - p.y) * (a.x - p.x)) * inv_delta;
    let w_c = 1.0 - w_a - w_b;

    Some((w_a, w_b, w_c))
}

/// Rasteriza un triángulo sobre una franja horizontal específica acotada al rango vertical `[band.y_start, band.y_end)`.
/// Retorna `true` si el triángulo intersectó la banda y fue procesado, o `false` si fue descartado.
#[allow(clippy::too_many_arguments)]
pub fn rasterize_triangle_band<S: FragmentShader>(
    v0: &VertexOut,
    v1: &VertexOut,
    v2: &VertexOut,
    band: &mut crate::buffer::RenderBand<'_>,
    material: &Material,
    shader: &S,
    blend_mode: BlendMode,
    stats: &mut FrameStats,
) -> bool {
    let raw_min_x = v0
        .screen_pos
        .x
        .min(v1.screen_pos.x)
        .min(v2.screen_pos.x)
        .floor() as isize;
    let raw_max_x = v0
        .screen_pos
        .x
        .max(v1.screen_pos.x)
        .max(v2.screen_pos.x)
        .ceil() as isize;
    let raw_min_y = v0
        .screen_pos
        .y
        .min(v1.screen_pos.y)
        .min(v2.screen_pos.y)
        .floor() as isize;
    let raw_max_y = v0
        .screen_pos
        .y
        .max(v1.screen_pos.y)
        .max(v2.screen_pos.y)
        .ceil() as isize;

    let fb_width = band.width as isize;
    let band_y_start = band.y_start as isize;
    let band_y_end = band.y_end as isize;

    // Descarte inmediato si el triángulo está fuera en X o fuera de las filas cubiertas por esta banda
    if raw_max_x < 0 || raw_min_x >= fb_width || raw_max_y < band_y_start || raw_min_y >= band_y_end
    {
        return false;
    }

    let min_x = raw_min_x.clamp(0, fb_width - 1) as usize;
    let max_x = raw_max_x.clamp(0, fb_width - 1) as usize;
    let min_y = raw_min_y.clamp(band_y_start, band_y_end - 1) as usize;
    let max_y = raw_max_y.clamp(band_y_start, band_y_end - 1) as usize;

    if min_x > max_x || min_y > max_y {
        return false;
    }

    let a = v0.screen_pos;
    let b = v1.screen_pos;
    let c = v2.screen_pos;
    let delta = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if delta.abs() < 1e-7 {
        return false;
    }
    let inv_delta = 1.0 / delta;

    let dw0_dx = (b.y - c.y) * inv_delta;
    let dw0_dy = (c.x - b.x) * inv_delta;
    let dw1_dx = (c.y - a.y) * inv_delta;
    let dw1_dy = (a.x - c.x) * inv_delta;

    let px0 = min_x as f32 + 0.5;
    let py0 = min_y as f32 + 0.5;

    let mut row_w0 = ((b.x - px0) * (c.y - py0) - (b.y - py0) * (c.x - px0)) * inv_delta;
    let mut row_w1 = ((c.x - px0) * (a.y - py0) - (c.y - py0) * (a.x - px0)) * inv_delta;

    for y in min_y..=max_y {
        let mut w0 = row_w0;
        let mut w1 = row_w1;

        for x in min_x..=max_x {
            stats.fragmentos_evaluados += 1;
            let w2 = 1.0 - w0 - w1;

            const EPS: f32 = -1e-4;
            if w0 >= EPS && w1 >= EPS && w2 >= EPS {
                let depth_ndc = w0 * v0.depth_ndc + w1 * v1.depth_ndc + w2 * v2.depth_ndc;

                let passes_depth = match blend_mode {
                    BlendMode::Opaque => band.test_and_set_depth(x, y, depth_ndc),
                    BlendMode::AlphaBlend => band.test_depth_only(x, y, depth_ndc),
                };

                if passes_depth {
                    stats.fragmentos_aceptados_depth += 1;

                    let inv_w = w0 * v0.inv_w + w1 * v1.inv_w + w2 * v2.inv_w;
                    let w_frag = if inv_w.abs() > 1e-8 { 1.0 / inv_w } else { 1.0 };

                    let normal =
                        (v0.normal_div_w * w0 + v1.normal_div_w * w1 + v2.normal_div_w * w2)
                            * w_frag;
                    let tangent =
                        (v0.tangent_div_w * w0 + v1.tangent_div_w * w1 + v2.tangent_div_w * w2)
                            * w_frag;
                    let world_pos = (v0.world_pos_div_w * w0
                        + v1.world_pos_div_w * w1
                        + v2.world_pos_div_w * w2)
                        * w_frag;
                    let uv = (v0.uv_div_w * w0 + v1.uv_div_w * w1 + v2.uv_div_w * w2) * w_frag;

                    let input = FragmentInput {
                        screen_x: x,
                        screen_y: y,
                        depth_ndc,
                        world_pos,
                        world_normal: normal.normalize(),
                        world_tangent: tangent.normalize(),
                        uv,
                        barycentric: Vec3::new(w0, w1, w2),
                        material,
                    };

                    let color = shader.shade(&input);
                    match blend_mode {
                        BlendMode::Opaque => band.set_pixel(x, y, color),
                        BlendMode::AlphaBlend => band.blend_pixel(x, y, color),
                    }
                }
            }

            w0 += dw0_dx;
            w1 += dw1_dx;
        }

        row_w0 += dw0_dy;
        row_w1 += dw1_dy;
    }

    true
}

/// Rasteriza un triángulo en CPU con prueba de profundidad e interpolación con corrección de perspectiva.
#[allow(clippy::too_many_arguments)]
pub fn rasterize_triangle<S: FragmentShader>(
    v0: &VertexOut,
    v1: &VertexOut,
    v2: &VertexOut,
    framebuffer: &mut Framebuffer,
    depth_buffer: &mut DepthBuffer,
    material: &Material,
    shader: &S,
    stats: &mut FrameStats,
) {
    let width = framebuffer.width;
    let height = framebuffer.height;
    let mut band = crate::buffer::RenderBand {
        width,
        height,
        y_start: 0,
        y_end: height,
        pixels: framebuffer.as_mut_slice(),
        depth: depth_buffer.as_mut_slice(),
    };
    if rasterize_triangle_band(
        v0,
        v1,
        v2,
        &mut band,
        material,
        shader,
        BlendMode::Opaque,
        stats,
    ) {
        stats.triangulos_rasterizados += 1;
    }
}
