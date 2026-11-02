pub mod clipping;
pub mod rasterizer;
pub mod shader;

use crate::buffer::{DepthBuffer, Framebuffer};
use crate::material::Material;
use crate::math::Mat4;
use crate::mesh::Mesh;
use crate::stats::FrameStats;
use clipping::{ClipTriangle, ClipVertex, ClippedTriangles, clip_triangle_near_plane};
pub use rasterizer::{VertexOut, is_front_facing, rasterize_triangle};
pub use shader::{
    DepthDebugShader, DiffuseShader, EmissiveShader, FragmentInput, FragmentShader,
    NormalsDebugShader, ShaderMode, TexturedDiffuseShader,
};

/// Renderizador 3D por software ejecutado íntegramente en CPU.
///
/// Administra buffers temporales preasignados para transformar los vértices únicos de una malla
/// una sola vez por cuadro/instancia y orquestar las 13 etapas explícitas del pipeline.
pub struct SoftwareRenderer {
    /// Buffer temporal de vértices transformados a espacio de recorte, reutilizado entre llamadas
    /// para evitar asignaciones dinámicas por cuadro o transformaciones repetidas de vértices compartidos.
    vertex_clip_cache: Vec<ClipVertex>,
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
        }
    }

    /// Renderiza una malla indexada ejecutando las 13 etapas completas del pipeline.
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
        // Se transforman únicamente los vértices del modelo una sola vez por instancia.
        for v in &mesh.vertices {
            stats.vertices_procesados += 1;

            // 1. Posición local
            let local_pos = v.position;

            // 2. Transformación de modelo a espacio de mundo
            let world_pos = ctx.model_matrix.transform_point(local_pos);

            // Transformación de la normal:
            // NOTA: Para modelos con escala uniforme se utiliza únicamente rotación y normalización.
            let world_normal = ctx
                .rotation_matrix
                .transform_direction(v.normal)
                .normalize();

            // 3 y 4. Transformación combinada de vista y proyección: $v_{clip} = (P \cdot V) \cdot v_{world}$
            let clip_pos = ctx.view_proj_matrix.transform_vec4(world_pos.to_vec4(1.0));

            self.vertex_clip_cache
                .push(ClipVertex::new(clip_pos, world_pos, world_normal, v.uv));
        }

        let width = target.framebuffer.width as f32;
        let height = target.framebuffer.height as f32;

        // --- Procesamiento de triángulos indexados ---
        for chunk in mesh.indices.chunks_exact(3) {
            stats.triangulos_entrada += 1;

            let cv0 = self.vertex_clip_cache[chunk[0] as usize];
            let cv1 = self.vertex_clip_cache[chunk[1] as usize];
            let cv2 = self.vertex_clip_cache[chunk[2] as usize];

            let tri = ClipTriangle::new(cv0, cv1, cv2);

            // 5. Clipping real contra el plano cercano
            match clip_triangle_near_plane(&tri) {
                ClippedTriangles::Zero => {
                    stats.triangulos_descartados_clipping += 1;
                }
                ClippedTriangles::One(t) => {
                    self.process_and_rasterize_triangle(
                        &t,
                        width,
                        height,
                        target,
                        ctx.material,
                        ctx.shader,
                        stats,
                    );
                }
                ClippedTriangles::Two(t1, t2) => {
                    stats.triangulos_generados_clipping += 1;
                    self.process_and_rasterize_triangle(
                        &t1,
                        width,
                        height,
                        target,
                        ctx.material,
                        ctx.shader,
                        stats,
                    );
                    self.process_and_rasterize_triangle(
                        &t2,
                        width,
                        height,
                        target,
                        ctx.material,
                        ctx.shader,
                        stats,
                    );
                }
            }
        }
    }

    /// Realiza división de perspectiva, conversión al viewport, backface culling y rasterización.
    #[allow(clippy::too_many_arguments)]
    fn process_and_rasterize_triangle<S: FragmentShader>(
        &self,
        tri: &ClipTriangle,
        width: f32,
        height: f32,
        target: &mut RenderTarget<'_>,
        material: &Material,
        shader: &S,
        stats: &mut FrameStats,
    ) {
        // 6 y 7: División de perspectiva y mapeo a pantalla (Viewport)
        let vo0 = VertexOut::from_clip_vertex(&tri.v[0], width, height);
        let vo1 = VertexOut::from_clip_vertex(&tri.v[1], width, height);
        let vo2 = VertexOut::from_clip_vertex(&tri.v[2], width, height);

        // Etapa separada de Backface Culling
        if !is_front_facing(&vo0, &vo1, &vo2) {
            stats.triangulos_descartados_backface += 1;
            return;
        }

        // 8 a 13: Rasterización, baricéntricas, z-test, fragment shader y framebuffer
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
