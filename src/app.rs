use crate::buffer::{Color, DepthBuffer, Framebuffer};
use crate::camera::Camera;
use crate::math::{Mat4, Transform, Vec3, deg_to_rad};
use crate::mesh::Mesh;
use crate::pipeline::{
    DepthDebugShader, DiffuseShader, NormalsDebugShader, ShaderMode, SoftwareRenderer,
};
use crate::stats::{FrameStats, StatsTracker};
use raylib::prelude::*;
use std::path::Path;
use std::time::Instant;

/// Aplicación principal que orquesta el ciclo de simulación, la entrada del usuario
/// y la presentación del framebuffer en la ventana 2D mediante Raylib.
pub struct App {
    renderer: SoftwareRenderer,
    framebuffer: Framebuffer,
    depth_buffer: DepthBuffer,
    camera: Camera,
    mesh: Mesh,
    preview_transform: Transform,
    rotation_y: f32,
    is_paused: bool,
    shader_mode: ShaderMode,
    diffuse_shader: DiffuseShader,
    depth_shader: DepthDebugShader,
    normals_shader: NormalsDebugShader,
    stats_tracker: StatsTracker,
}

impl App {
    pub const WINDOW_WIDTH: usize = 960;
    pub const WINDOW_HEIGHT: usize = 720;

    pub fn new(model_path: &Path) -> Result<Self, String> {
        // Carga de la nave OBJ y reporte inicial en consola
        println!("[APP] Cargando modelo: {:?}", model_path);
        let mesh = Mesh::load_from_obj(model_path)?;

        let aabb = &mesh.aabb;
        let size = aabb.size();
        println!("============================================================");
        println!("[MODELO CARGADO]");
        println!("  Archivo:      {:?}", model_path);
        println!("  Vértices:     {}", mesh.vertex_count());
        println!("  Triángulos:   {}", mesh.triangle_count());
        println!(
            "  AABB Mínimo:  ({:.3}, {:.3}, {:.3})",
            aabb.min.x, aabb.min.y, aabb.min.z
        );
        println!(
            "  AABB Máximo:  ({:.3}, {:.3}, {:.3})",
            aabb.max.x, aabb.max.y, aabb.max.z
        );
        println!(
            "  Dimensiones:  ({:.3}, {:.3}, {:.3}) | Extensión máx: {:.3}",
            size.x,
            size.y,
            size.z,
            aabb.max_extent()
        );
        println!("============================================================");

        // Transformación de preview (centrado en el origen y escala proporcional uniforme para la cámara)
        // El radio objetivo es 2.0 unidades frente a la cámara
        let preview_transform = mesh.calculate_preview_transform(2.0);
        println!("[PREVIEW] Transformación calculada:");
        println!(
            "  Offset centrado: ({:.3}, {:.3}, {:.3})",
            preview_transform.position.x,
            preview_transform.position.y,
            preview_transform.position.z
        );
        println!("  Escala uniforme: {:.4}", preview_transform.scale.x);

        let aspect = Self::WINDOW_WIDTH as f32 / Self::WINDOW_HEIGHT as f32;
        // Cámara situada en (0, 1.5, 4.5) mirando al origen (0, 0, 0)
        let camera = Camera::new(
            Vec3::new(0.0, 1.2, 4.5),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::up(),
            45.0, // FoV 45 grados
            aspect,
            0.1,   // Near plane
            100.0, // Far plane
        );

        Ok(Self {
            renderer: SoftwareRenderer::new(),
            framebuffer: Framebuffer::new(Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT),
            depth_buffer: DepthBuffer::new(Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT),
            camera,
            mesh,
            preview_transform,
            rotation_y: 0.0,
            is_paused: false,
            shader_mode: ShaderMode::Diffuse,
            diffuse_shader: DiffuseShader::new(),
            depth_shader: DepthDebugShader::new(camera.z_near, camera.z_far, 2.5, 6.5),
            normals_shader: NormalsDebugShader,
            stats_tracker: StatsTracker::new(),
        })
    }

    /// Ejecuta el ciclo de vida de la aplicación con Raylib.
    pub fn run(&mut self) {
        let (mut rl, thread) = raylib::init()
            .size(Self::WINDOW_WIDTH as i32, Self::WINDOW_HEIGHT as i32)
            .title("Space Travel")
            .build();

        rl.set_target_fps(60);

        // Creación de textura 2D en Raylib para presentar el framebuffer de CPU
        let img = unsafe {
            Image::from_raw(raylib::ffi::GenImageColor(
                Self::WINDOW_WIDTH as i32,
                Self::WINDOW_HEIGHT as i32,
                raylib::ffi::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            ))
        };
        let mut screen_texture = rl
            .load_texture_from_image(&thread, &img)
            .expect("No se pudo crear la textura de presentación en Raylib");

        println!("[INSTRUCCIONES]");
        println!("  [Escape] : Cerrar la aplicación.");
        println!("  [Espacio]: Pausar / reanudar la rotación continua.");
        println!(
            "  [D]      : Alternar modo de shader (Difuso -> Profundidad Debug -> Normales Debug)."
        );
        println!("  [R]      : Restablecer ángulo de rotación.");
        println!("============================================================");

        while !rl.window_should_close() {
            let dt = rl.get_frame_time();

            // 1. Manejo de entradas del teclado
            self.handle_input(&rl);

            // 2. Actualización de simulación basada en tiempo transcurrido (dt)
            if !self.is_paused {
                // Rotación suave a 45 grados por segundo
                self.rotation_y += deg_to_rad(45.0) * dt;
                if self.rotation_y > std::f32::consts::TAU {
                    self.rotation_y -= std::f32::consts::TAU;
                }
            }

            // 3. Renderizado de la escena mediante el CPU software renderer
            let frame_start = Instant::now();
            let mut frame_stats = FrameStats::default();

            self.render_frame(&mut frame_stats);

            frame_stats.frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
            self.stats_tracker.record_frame(frame_stats);

            // 4. Presentación del framebuffer en la ventana Raylib
            screen_texture
                .update_texture(self.framebuffer.as_bytes())
                .expect("Error al transferir framebuffer a la textura de Raylib");

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(raylib::prelude::Color::BLACK);
            d.draw_texture(&screen_texture, 0, 0, raylib::prelude::Color::WHITE);
        }
    }

    fn handle_input(&mut self, rl: &RaylibHandle) {
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            self.is_paused = !self.is_paused;
            println!(
                "[CONTROLES] Rotación {}",
                if self.is_paused {
                    "PAUSADA"
                } else {
                    "REANUDADA"
                }
            );
        }

        if rl.is_key_pressed(KeyboardKey::KEY_D) {
            self.shader_mode = match self.shader_mode {
                ShaderMode::Diffuse => ShaderMode::DepthDebug,
                ShaderMode::DepthDebug => ShaderMode::NormalsDebug,
                ShaderMode::NormalsDebug => ShaderMode::Diffuse,
            };
            println!(
                "[CONTROLES] Modo de sombreado cambiado a: {:?}",
                self.shader_mode
            );
        }

        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            self.rotation_y = 0.0;
            println!("[CONTROLES] Orientación de rotación restablecida.");
        }
    }

    /// Ejecuta el renderizado de un fotograma completo en el framebuffer de CPU.
    pub fn render_frame(&mut self, stats: &mut FrameStats) {
        // Limpieza de buffers propios
        self.framebuffer.clear(Color::DARK_GRAY);
        self.depth_buffer.clear();

        // Matrices de modelo, rotación y vista-proyección
        // Modelo: R(t) * T_preview * S_preview
        let rot_matrix = Mat4::rotation_y(self.rotation_y);
        let preview_matrix = self.preview_transform.to_matrix();
        let model_matrix = rot_matrix * preview_matrix;

        let vp_matrix = self.camera.view_projection_matrix();

        let mut target = crate::pipeline::RenderTarget {
            framebuffer: &mut self.framebuffer,
            depth_buffer: &mut self.depth_buffer,
        };

        match self.shader_mode {
            ShaderMode::Diffuse => {
                let ctx = crate::pipeline::RenderContext {
                    model_matrix: &model_matrix,
                    rotation_matrix: &rot_matrix,
                    view_proj_matrix: &vp_matrix,
                    shader: &self.diffuse_shader,
                };
                self.renderer
                    .render_mesh(&self.mesh, &ctx, &mut target, stats);
            }
            ShaderMode::DepthDebug => {
                let ctx = crate::pipeline::RenderContext {
                    model_matrix: &model_matrix,
                    rotation_matrix: &rot_matrix,
                    view_proj_matrix: &vp_matrix,
                    shader: &self.depth_shader,
                };
                self.renderer
                    .render_mesh(&self.mesh, &ctx, &mut target, stats);
            }
            ShaderMode::NormalsDebug => {
                let ctx = crate::pipeline::RenderContext {
                    model_matrix: &model_matrix,
                    rotation_matrix: &rot_matrix,
                    view_proj_matrix: &vp_matrix,
                    shader: &self.normals_shader,
                };
                self.renderer
                    .render_mesh(&self.mesh, &ctx, &mut target, stats);
            }
        }
    }
}
