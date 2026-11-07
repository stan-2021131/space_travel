pub mod clipping;
pub mod rasterizer;
pub mod shaders;

use crate::buffer::{DepthBuffer, Framebuffer};
use crate::material::Material;
use crate::math::Mat4;
use crate::mesh::Mesh;
use crate::stats::FrameStats;
use clipping::{ClipTriangle, ClipVertex, ClippedTriangles, clip_triangle_near_plane};
pub use rasterizer::{VertexOut, is_front_facing, rasterize_triangle, rasterize_triangle_band};
pub use shaders as shader;
pub use shaders::*;

/// Renderizador 3D por software ejecutado íntegramente en CPU con paralelismo multinúcleo.
///
/// Administra buffers temporales preasignados para transformar los vértices únicos de una malla
/// una sola vez por cuadro/instancia y orquestar las 13 etapas explícitas del pipeline en paralelo
/// mediante partición horizontal de pantalla (Scanline Slicing).
pub struct SoftwareRenderer {
    /// Buffer temporal de vértices transformados a espacio de recorte, reutilizado entre llamadas.
    vertex_clip_cache: Vec<ClipVertex>,
    /// Buffer temporal de triángulos listos para rasterizar tras clipping y backface culling.
    triangles_cache: Vec<(VertexOut, VertexOut, VertexOut)>,
}

/// Contexto de transformación y sombreado para el renderizado de una malla.
pub struct RenderContext<'a, S: FragmentShader> {
    pub model_matrix: &'a Mat4,
    pub rotation_matrix: &'a Mat4,
    pub view_proj_matrix: &'a Mat4,
    pub material: &'a Material,
    pub shader: &'a S,
}

/// Objetivos de renderizado en CPU (framebuffer y depth buffer).
pub struct RenderTarget<'a> {
    pub framebuffer: &'a mut Framebuffer,
    pub depth_buffer: &'a mut DepthBuffer,
}

impl SoftwareRenderer {
    pub fn new() -> Self {
        Self {
            vertex_clip_cache: Vec::with_capacity(1024),
            triangles_cache: Vec::with_capacity(1024),
        }
    }

    /// Renderiza una malla indexada ejecutando las 13 etapas completas del pipeline.
    /// Distribuye la rasterización de fragmentos entre múltiples hilos de CPU mediante franjas horizontales.
    pub fn render_mesh<S: FragmentShader>(
        &mut self,
        mesh: &Mesh,
        ctx: &RenderContext<'_, S>,
        target: &mut RenderTarget<'_>,
        stats: &mut FrameStats,
    ) {
        stats.modelos_procesados += 1;

        // Reutilización del buffer temporal de vértices únicos
        self.vertex_clip_cache.clear();
        if self.vertex_clip_cache.capacity() < mesh.vertices.len() {
            self.vertex_clip_cache
                .reserve(mesh.vertices.len() - self.vertex_clip_cache.capacity());
        }

        // --- Etapa 1 a 4: Transformación de vértices únicos ---
        for v in &mesh.vertices {
            stats.vertices_procesados += 1;

            let local_pos = v.position;
            let world_pos = ctx.model_matrix.transform_point(local_pos);

            let world_normal = ctx
                .rotation_matrix
                .transform_direction(v.normal)
                .normalize();

            let world_tangent = ctx
                .rotation_matrix
                .transform_direction(v.tangent)
                .normalize();

            let clip_pos = ctx.view_proj_matrix.transform_vec4(world_pos.to_vec4(1.0));

            self.vertex_clip_cache.push(ClipVertex::new(
                clip_pos,
                world_pos,
                world_normal,
                world_tangent,
                v.uv,
            ));
        }

        let width = target.framebuffer.width as f32;
        let height = target.framebuffer.height as f32;

        self.triangles_cache.clear();

        // --- Procesamiento de triángulos indexados, clipping y backface culling ---
        for chunk in mesh.indices.chunks_exact(3) {
            stats.triangulos_entrada += 1;

            let cv0 = self.vertex_clip_cache[chunk[0] as usize];
            let cv1 = self.vertex_clip_cache[chunk[1] as usize];
            let cv2 = self.vertex_clip_cache[chunk[2] as usize];

            let tri = ClipTriangle::new(cv0, cv1, cv2);

            match clip_triangle_near_plane(&tri) {
                ClippedTriangles::Zero => {
                    stats.triangulos_descartados_clipping += 1;
                }
                ClippedTriangles::One(t) => {
                    self.collect_visible_triangle(&t, width, height, stats);
                }
                ClippedTriangles::Two(t1, t2) => {
                    stats.triangulos_generados_clipping += 1;
                    self.collect_visible_triangle(&t1, width, height, stats);
                    self.collect_visible_triangle(&t2, width, height, stats);
                }
            }
        }

        if self.triangles_cache.is_empty() || target.framebuffer.height == 0 {
            return;
        }

        stats.triangulos_rasterizados += self.triangles_cache.len();

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, target.framebuffer.height);

        if num_threads == 1 || self.triangles_cache.len() <= 2 {
            // Ejecución secuencial en un solo hilo
            for (vo0, vo1, vo2) in &self.triangles_cache {
                let fb_width = target.framebuffer.width;
                let fb_height = target.framebuffer.height;
                let mut band = crate::buffer::RenderBand {
                    width: fb_width,
                    height: fb_height,
                    y_start: 0,
                    y_end: fb_height,
                    pixels: target.framebuffer.as_mut_slice(),
                    depth: target.depth_buffer.as_mut_slice(),
                };
                rasterize_triangle_band(vo0, vo1, vo2, &mut band, ctx.material, ctx.shader, stats);
            }
        } else {
            // Rasterización paralela por franjas horizontales disjuntas (Scanline Slicing)
            let bands = crate::buffer::split_render_bands(
                target.framebuffer,
                target.depth_buffer,
                num_threads,
            );

            let triangles = &self.triangles_cache;
            let material = ctx.material;
            let shader = ctx.shader;

            let thread_stats: Vec<FrameStats> = std::thread::scope(|s| {
                let mut handles = Vec::with_capacity(bands.len());
                for mut band in bands {
                    handles.push(s.spawn(move || {
                        let mut local_stats = FrameStats::default();
                        for (vo0, vo1, vo2) in triangles {
                            rasterize_triangle_band(
                                vo0,
                                vo1,
                                vo2,
                                &mut band,
                                material,
                                shader,
                                &mut local_stats,
                            );
                        }
                        local_stats
                    }));
                }
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap_or_default())
                    .collect()
            });

            for ts in thread_stats {
                stats.fragmentos_evaluados += ts.fragmentos_evaluados;
                stats.fragmentos_aceptados_depth += ts.fragmentos_aceptados_depth;
            }
        }
    }

    /// Realiza división de perspectiva, conversión a pantalla y backface culling, recolectando triángulos visibles.
    #[inline]
    fn collect_visible_triangle(
        &mut self,
        tri: &ClipTriangle,
        width: f32,
        height: f32,
        stats: &mut FrameStats,
    ) {
        let vo0 = VertexOut::from_clip_vertex(&tri.v[0], width, height);
        let vo1 = VertexOut::from_clip_vertex(&tri.v[1], width, height);
        let vo2 = VertexOut::from_clip_vertex(&tri.v[2], width, height);

        if !is_front_facing(&vo0, &vo1, &vo2) {
            stats.triangulos_descartados_backface += 1;
            return;
        }

        self.triangles_cache.push((vo0, vo1, vo2));
    }

    /// Realiza división de perspectiva, conversión al viewport, backface culling y rasterización directa monohilo.
    #[allow(clippy::too_many_arguments)]
    pub fn process_and_rasterize_triangle<S: FragmentShader>(
        &self,
        tri: &ClipTriangle,
        width: f32,
        height: f32,
        target: &mut RenderTarget<'_>,
        material: &Material,
        shader: &S,
        stats: &mut FrameStats,
    ) {
        let vo0 = VertexOut::from_clip_vertex(&tri.v[0], width, height);
        let vo1 = VertexOut::from_clip_vertex(&tri.v[1], width, height);
        let vo2 = VertexOut::from_clip_vertex(&tri.v[2], width, height);

        if !is_front_facing(&vo0, &vo1, &vo2) {
            stats.triangulos_descartados_backface += 1;
            return;
        }

        rasterize_triangle(
            &vo0,
            &vo1,
            &vo2,
            target.framebuffer,
            target.depth_buffer,
            material,
            shader,
            stats,
        );
    }
}

impl Default for SoftwareRenderer {
    fn default() -> Self {
        Self::new()
    }
}
