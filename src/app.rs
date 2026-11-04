use crate::buffer::{DepthBuffer, Framebuffer};
use crate::camera::Camera;
use crate::config::AppConfig;
use crate::material::Material;
use crate::math::{Mat4, Vec3, deg_to_rad};
use crate::mesh::Mesh;
use crate::physics::{ShipInput, ShipState};
use crate::pipeline::{
    DepthDebugShader, FragmentShader, NormalsDebugShader, PlasmaShader, ProceduralRockShader,
    RenderContext, RenderTarget, ShaderMode, ShipShader, SkyShader, SoftwareRenderer,
    TexturedDiffuseShader, ToonShader,
};
use crate::scene::CelestialBody;
use crate::stats::{FrameStats, StatsTracker};
use raylib::prelude::*;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Aplicación principal que orquesta el ciclo de simulación espacial, control de nave,
/// cámara de persecución y renderizado por software en CPU.
pub struct App {
    config: AppConfig,
    renderer: SoftwareRenderer,
    framebuffer: Framebuffer,
    depth_buffer: DepthBuffer,
    camera: Camera,

    // Entidades
    ship_mesh: Arc<Mesh>,
    ship_scale: f32,
    ship_state: ShipState,

    sun_body: CelestialBody,
    planets: Vec<CelestialBody>,

    // Estado de simulación
    sim_time: f32,
    accumulator: f32,
    pub shader_mode: ShaderMode,

    // Shaders reutilizables
    planet_shader: TexturedDiffuseShader,
    rock_shader: ProceduralRockShader,
    plasma_shader: PlasmaShader,
    ship_shader: ShipShader,
    toon_shader: ToonShader,
    sky_shader: SkyShader,
    depth_shader: DepthDebugShader,
    normals_shader: NormalsDebugShader,

    pub stats_tracker: StatsTracker,
}

impl App {
    pub const WINDOW_WIDTH: usize = 960;
    pub const WINDOW_HEIGHT: usize = 720;

    pub fn new(config_path: &Path) -> Result<Self, String> {
        println!("[APP] Cargando configuración desde '{:?}'...", config_path);
        let config = AppConfig::load_from_file(config_path)?;

        // 1. Carga de mallas compartidas
        println!(
            "[RECURSOS] Cargando modelo de la nave: {}",
            config.ship.model_path
        );
        let ship_mesh = Arc::new(Mesh::load_from_obj(Path::new(&config.ship.model_path))?);
        let ship_max_dim = ship_mesh.aabb.max_extent();
        let ship_scale = if ship_max_dim > 1e-6 {
            config.ship.target_length_wu / ship_max_dim
        } else {
            1.0
        };

        println!(
            "[RECURSOS] Cargando modelo del sol: {}",
            config.sun.model_path
        );
        let sun_mesh = Arc::new(Mesh::load_from_obj(Path::new(&config.sun.model_path))?);
        let sun_geo_radius = sun_mesh.geometric_radius().max(1e-4);
        let sun_scale = config.sun.radius / sun_geo_radius;

        let sun_body = CelestialBody {
            id: config.sun.id.clone(),
            material: sun_mesh.material.clone(),
            mesh: sun_mesh,
            scale: sun_scale,
            radius: config.sun.radius,
            collision_margin: config.sun.collision_margin,
            orbital_radius: 0.0,
            orbital_period: 1.0,
            rotation_period: config.sun.rotation_period,
            initial_orbital_angle_rad: 0.0,
            position: Vec3::zero(),
            rotation_rad: 0.0,
            shader_type: config.sun.shader.clone(),
        };

        let mut planets = Vec::new();
        for p_cfg in &config.planets {
            println!(
                "[RECURSOS] Cargando planeta '{}': {}",
                p_cfg.id, p_cfg.model_path
            );
            let p_mesh = Arc::new(Mesh::load_from_obj(Path::new(&p_cfg.model_path))?);
            let p_geo_radius = p_mesh.geometric_radius().max(1e-4);
            let p_scale = p_cfg.radius / p_geo_radius;

            planets.push(CelestialBody {
                id: p_cfg.id.clone(),
                material: p_mesh.material.clone(),
                mesh: p_mesh,
                scale: p_scale,
                radius: p_cfg.radius,
                collision_margin: p_cfg.collision_margin,
                orbital_radius: p_cfg.orbital_radius,
                orbital_period: p_cfg.orbital_period,
                rotation_period: p_cfg.rotation_period,
                initial_orbital_angle_rad: deg_to_rad(p_cfg.initial_orbital_angle_deg),
                position: Vec3::new(p_cfg.orbital_radius, 0.0, 0.0),
                rotation_rad: 0.0,
                shader_type: p_cfg.shader.clone(),
            });
        }

        // 2. Estado de la nave y cámara
        let ship_state = ShipState::new(&config.ship);
        let aspect = Self::WINDOW_WIDTH as f32 / Self::WINDOW_HEIGHT as f32;
        let mut camera = Camera::new(
            ship_state.position - ship_state.forward() * config.camera.distance_behind
                + ship_state.up() * config.camera.height_above,
            ship_state.position + ship_state.forward() * config.camera.look_ahead,
            ship_state.up(),
            config.camera.fov_deg,
            aspect,
            config.camera.near_plane,
            config.camera.far_plane,
        );
        camera.update_chase(
            ship_state.position,
            ship_state.forward(),
            ship_state.up(),
            &config.camera,
            1.0,
        );

        let depth_shader = DepthDebugShader::new(
            config.camera.near_plane,
            config.camera.far_plane,
            config.camera.near_plane,
            config.camera.far_plane * 0.70,
        );

        let rock_shader = ProceduralRockShader::new(config.simulation.seed, sun_body.position);
        let plasma_shader = PlasmaShader::new();
        let toon_shader = ToonShader::new(sun_body.position, camera.eye);
        let ship_shader = ShipShader::new(sun_body.position, camera.eye);
        let sky_shader = SkyShader::new(config.simulation.seed);

        Ok(Self {
            config,
            renderer: SoftwareRenderer::new(),
            framebuffer: Framebuffer::new(Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT),
            depth_buffer: DepthBuffer::new(Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT),
            camera,
            ship_mesh,
            ship_scale,
            ship_state,
            sun_body,
            planets,
            sim_time: 0.0,
            accumulator: 0.0,
            shader_mode: ShaderMode::Normal,
            planet_shader: TexturedDiffuseShader::new(Vec3::zero()),
            rock_shader,
            plasma_shader,
            ship_shader,
            toon_shader,
            sky_shader,
            depth_shader,
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

        println!("============================================================");
        println!("[CONTROLES DE VUELO]");
        println!("  [W]              : Acelerar hacia adelante");
        println!("  [S]              : Frenar / Aplicar reversa");
        println!("  [A] / [D]        : Yaw (Giro horizontal)");
        println!("  [Flecha Arriba]  : Pitch arriba");
        println!("  [Flecha Abajo]   : Pitch abajo");
        println!("  [Q] / [E]        : Roll (Alabeo)");
        println!("  [R]              : Reiniciar posición y orientación de la nave");
        println!(
            "  [F2]             : Alternar modo de shader (Normal -> Toon -> Depth -> Normals)"
        );
        println!(
            "  [F3]             : Alternar modo de estadísticas (Compacto -> Detallado -> Off)"
        );
        println!("  [Escape]         : Cerrar aplicación");
        println!("============================================================");

        while !rl.window_should_close() {
            let dt = rl
                .get_frame_time()
                .min(self.config.simulation.max_frame_delta)
                * self.config.simulation.time_scale;

            // 1. Manejo de entradas
            let input = self.read_input(&rl);

            // 2. Simulación de física y órbitas en pasos fijos acumulados
            self.accumulator += dt;
            let fixed_dt = self.config.simulation.fixed_timestep;
            let mut steps = 0;
            while self.accumulator >= fixed_dt && steps < 5 {
                self.update_simulation(&input, fixed_dt);
                self.accumulator -= fixed_dt;
                steps += 1;
            }

            // 3. Suavizado de la cámara de persecución
            self.camera.update_chase(
                self.ship_state.position,
                self.ship_state.forward(),
                self.ship_state.up(),
                &self.config.camera,
                dt,
            );

            // 4. Renderizado en CPU y medición de métricas
            let frame_start = Instant::now();
            let mut frame_stats = FrameStats::default();

            self.render_scene(&mut frame_stats);

            frame_stats.frame_time_ms = frame_start.elapsed().as_secs_f32() * 1000.0;

            // Datos de estadísticas
            let (nearest_name, nearest_dist) = Self::calculate_nearest_body_distance_from(
                self.ship_state.position,
                &self.sun_body,
                &self.planets,
            );
            self.stats_tracker.record_frame(
                frame_stats,
                self.ship_state.velocity.length(),
                nearest_name,
                nearest_dist,
            );

            // 5. Presentación del framebuffer en la ventana Raylib
            screen_texture
                .update_texture(self.framebuffer.as_bytes())
                .expect("Error al actualizar la textura de presentación");

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(raylib::prelude::Color::BLACK);
            d.draw_texture(&screen_texture, 0, 0, raylib::prelude::Color::WHITE);
        }
    }

    /// Alterna el modo de visualización de shaders (Normal -> DepthDebug -> NormalsDebug).
    pub fn cycle_shader_mode(&mut self) {
        self.shader_mode = self.shader_mode.cycle();
        println!("[CONTROLES] Modo de sombreado: {:?}", self.shader_mode);
    }

    /// Maneja acciones de teclas de diagnóstico (F2 para shaders, F3 para estadísticas).
    /// La tecla D es exclusivamente para navegación (yaw_right) y no debe alterar modos de diagnóstico.
    pub fn handle_diagnostic_key(&mut self, is_f2: bool, is_f3: bool, is_d: bool) {
        let _ = is_d; // D no altera el shader ni las estadísticas
        if is_f2 {
            self.cycle_shader_mode();
        }
        if is_f3 {
            self.stats_tracker.cycle_mode();
        }
    }

    fn read_input(&mut self, rl: &RaylibHandle) -> ShipInput {
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            self.ship_state.reset(&self.config.ship);
            println!("[SIMULACIÓN] Nave reiniciada a posición inicial.");
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F2) {
            self.cycle_shader_mode();
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F3) {
            self.stats_tracker.cycle_mode();
        }

        ShipInput {
            throttle_forward: rl.is_key_down(KeyboardKey::KEY_W),
            throttle_reverse: rl.is_key_down(KeyboardKey::KEY_S),
            yaw_left: rl.is_key_down(KeyboardKey::KEY_A),
            yaw_right: rl.is_key_down(KeyboardKey::KEY_D),
            pitch_up: rl.is_key_down(KeyboardKey::KEY_UP),
            pitch_down: rl.is_key_down(KeyboardKey::KEY_DOWN),
            roll_left: rl.is_key_down(KeyboardKey::KEY_Q),
            roll_right: rl.is_key_down(KeyboardKey::KEY_E),
        }
    }

    /// Actualiza la simulación física de la nave, órbitas, rotaciones axiales y colisiones.
    pub fn update_simulation(&mut self, input: &ShipInput, dt: f32) {
        self.sim_time += dt;

        // Actualización del tiempo para shaders animados
        self.plasma_shader.set_time(self.sim_time);

        // Movimiento de la nave
        self.ship_state.update(input, &self.config.ship, dt);

        // Órbitas y rotación axial calculadas analíticamente por tiempo
        self.sun_body.update(self.sim_time);
        for planet in &mut self.planets {
            planet.update(self.sim_time);
        }

        // Colisiones inelásticas con deslizamiento
        // 1. Sol
        self.ship_state.resolve_collision(
            self.sun_body.position,
            self.sun_body.radius,
            self.sun_body.collision_margin,
        );

        // 2. Planetas
        for planet in &self.planets {
            self.ship_state.resolve_collision(
                planet.position,
                planet.radius,
                planet.collision_margin,
            );
        }
    }

    /// Renderiza todos los cuerpos de la escena continua en el framebuffer propio en CPU.
    pub fn render_scene(&mut self, stats: &mut FrameStats) {
        if self.shader_mode == ShaderMode::DepthDebug
            || self.shader_mode == ShaderMode::NormalsDebug
        {
            self.framebuffer.clear(crate::buffer::Color::BLACK);
        } else {
            self.sky_shader.render(&mut self.framebuffer, &self.camera);
        }
        self.depth_buffer.clear();

        let vp_matrix = self.camera.view_projection_matrix();
        self.toon_shader.camera_pos = self.camera.eye;
        self.toon_shader.light_pos = self.sun_body.position;
        self.ship_shader.camera_pos = self.camera.eye;
        self.ship_shader.light_pos = self.sun_body.position;
        self.plasma_shader.sun_center = self.sun_body.position;
        self.rock_shader.light_pos = self.sun_body.position;
        self.planet_shader.light_pos = self.sun_body.position;

        let mut target = RenderTarget {
            framebuffer: &mut self.framebuffer,
            depth_buffer: &mut self.depth_buffer,
        };

        // 1. Sol (Shader animado de plasma)
        let sun_model = self.sun_body.model_matrix();
        let sun_rot = self.sun_body.rotation_matrix();
        self.toon_shader.is_sun = true;
        Self::draw_mesh(
            &mut self.renderer,
            self.shader_mode,
            &self.depth_shader,
            &self.normals_shader,
            &self.toon_shader,
            &self.plasma_shader,
            &self.sun_body.mesh,
            &sun_model,
            &sun_rot,
            &vp_matrix,
            &self.sun_body.material,
            &mut target,
            stats,
        );
        self.toon_shader.is_sun = false;

        // 2. Planetas (Shader procedural con semilla o textura)
        for planet in &self.planets {
            let p_model = planet.model_matrix();
            let p_rot = planet.rotation_matrix();
            self.rock_shader.body_center = planet.position;
            self.rock_shader.rotation_rad = planet.rotation_rad;
            if planet.shader_type == "procedural_rock" {
                Self::draw_mesh(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.rock_shader,
                    &planet.mesh,
                    &p_model,
                    &p_rot,
                    &vp_matrix,
                    &planet.material,
                    &mut target,
                    stats,
                );
            } else {
                Self::draw_mesh(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.planet_shader,
                    &planet.mesh,
                    &p_model,
                    &p_rot,
                    &vp_matrix,
                    &planet.material,
                    &mut target,
                    stats,
                );
            }
        }

        // 3. Nave
        let ship_center = self.ship_mesh.aabb.center();
        let ship_model = Mat4::from_translation(self.ship_state.position)
            * self.ship_state.rotation_matrix()
            * Mat4::uniform_scaling(self.ship_scale)
            * Mat4::from_translation(-ship_center);
        let ship_rot = self.ship_state.rotation_matrix();

        Self::draw_mesh(
            &mut self.renderer,
            self.shader_mode,
            &self.depth_shader,
            &self.normals_shader,
            &self.toon_shader,
            &self.ship_shader,
            &self.ship_mesh,
            &ship_model,
            &ship_rot,
            &vp_matrix,
            &self.ship_mesh.material,
            &mut target,
            stats,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_mesh<S: FragmentShader>(
        renderer: &mut SoftwareRenderer,
        shader_mode: ShaderMode,
        depth_shader: &DepthDebugShader,
        normals_shader: &NormalsDebugShader,
        toon_shader: &ToonShader,
        base_shader: &S,
        mesh: &Mesh,
        model_matrix: &Mat4,
        rotation_matrix: &Mat4,
        vp_matrix: &Mat4,
        material: &Material,
        target: &mut RenderTarget<'_>,
        stats: &mut FrameStats,
    ) {
        match shader_mode {
            ShaderMode::Normal => {
                let ctx = RenderContext {
                    model_matrix,
                    rotation_matrix,
                    view_proj_matrix: vp_matrix,
                    material,
                    shader: base_shader,
                };
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::Toon => {
                let ctx = RenderContext {
                    model_matrix,
                    rotation_matrix,
                    view_proj_matrix: vp_matrix,
                    material,
                    shader: toon_shader,
                };
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::DepthDebug => {
                let ctx = RenderContext {
                    model_matrix,
                    rotation_matrix,
                    view_proj_matrix: vp_matrix,
                    material,
                    shader: depth_shader,
                };
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::NormalsDebug => {
                let ctx = RenderContext {
                    model_matrix,
                    rotation_matrix,
                    view_proj_matrix: vp_matrix,
                    material,
                    shader: normals_shader,
                };
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
        }
    }

    /// Calcula la distancia a la superficie del cuerpo celeste más cercano dada una posición.
    pub fn calculate_nearest_body_distance_from<'a>(
        pos: Vec3,
        sun: &'a CelestialBody,
        planets: &'a [CelestialBody],
    ) -> (&'a str, f32) {
        let sun_dist = (pos - sun.position).length() - sun.radius;
        let mut min_dist = sun_dist;
        let mut min_name = sun.id.as_str();

        for p in planets {
            let p_dist = (pos - p.position).length() - p.radius;
            if p_dist < min_dist {
                min_dist = p_dist;
                min_name = p.id.as_str();
            }
        }

        (min_name, min_dist)
    }

    /// Calcula la distancia de la nave a la superficie del cuerpo celeste más cercano.
    pub fn calculate_nearest_body_distance(&self) -> (&str, f32) {
        Self::calculate_nearest_body_distance_from(
            self.ship_state.position,
            &self.sun_body,
            &self.planets,
        )
    }
}
