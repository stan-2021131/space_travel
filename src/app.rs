use crate::buffer::{DepthBuffer, Framebuffer};
use crate::camera::Camera;
use crate::combat::{CombatSystem, Enemy, ProjectileOwner, create_projectile_mesh};
use crate::config::AppConfig;
use crate::hud::{draw_health_bar, project_world_to_screen};
use crate::material::Material;
use crate::math::{Mat4, Vec3, deg_to_rad};
use crate::mesh::Mesh;
use crate::physics::{CollisionSphere, ShipInput, ShipState, resolve_sphere_collision};
use crate::pipeline::{
    AlienEnergyShader, DepthDebugShader, EmissiveShader, FragmentShader, NormalMappedShader,
    NormalsDebugShader, PlasmaShader, ProceduralRockShader, RenderContext, RenderTarget,
    ShaderMode, ShipShader, SkyShader, SoftwareRenderer, TexturedDiffuseShader, ToonShader,
};
use crate::scene::{
    CelestialBody,
    surface::{
        SurfaceGroup, calculate_fade_factor, calculate_surface_embedding,
        generate_spherical_distribution,
    },
};
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
    enemy_mesh: Arc<Mesh>,
    enemy_scale: f32,
    projectile_mesh: Arc<Mesh>,
    pub combat: CombatSystem,

    sun_body: CelestialBody,
    planets: Vec<CelestialBody>,

    // Estado de simulación
    sim_time: f32,
    accumulator: f32,
    pub shader_mode: ShaderMode,

    // Shaders reutilizables
    planet_shader: TexturedDiffuseShader,
    normal_mapped_shader: NormalMappedShader,
    rock_shader: ProceduralRockShader,
    plasma_shader: PlasmaShader,
    ship_shader: ShipShader,
    enemy_shader: AlienEnergyShader,
    enemy_material: Material,
    projectile_shader: EmissiveShader,
    player_projectile_material: Material,
    enemy_projectile_material: Material,
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
            "[RECURSOS] Cargando modelo de enemigos: {}",
            config.enemies.model_path
        );
        let enemy_mesh = Arc::new(Mesh::load_from_obj(Path::new(&config.enemies.model_path))?);
        let enemy_max_dim = enemy_mesh.aabb.max_extent();
        let enemy_scale = if enemy_max_dim > 1e-6 {
            config.enemies.target_length_wu / enemy_max_dim
        } else {
            1.0
        };
        let projectile_mesh = Arc::new(create_projectile_mesh());

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
            surface_groups: Vec::new(),
        };

        let mut planets = Vec::new();
        for p_cfg in &config.planets {
            let mut mesh = if p_cfg.mesh_type == "icosphere" {
                println!(
                    "[PROCEDURAL] Generando esfera geodésica para '{}' (radio: {}, subdiv: {})",
                    p_cfg.id, p_cfg.radius, p_cfg.subdivisions
                );
                crate::procedural::generate_icosphere(p_cfg.radius, p_cfg.subdivisions)
            } else {
                println!(
                    "[RECURSOS] Cargando cuerpo celeste '{}': {}",
                    p_cfg.id, p_cfg.model_path
                );
                let mut m = Mesh::load_from_obj(Path::new(&p_cfg.model_path))?;
                // Subdivisión procedural de geometría previa a rasterizar (+10 pts)
                for _ in 0..p_cfg.subdivisions {
                    m.subdivide();
                }
                m
            };

            // Deformación geométrica por ruido con semilla universal (+10 pts)
            if p_cfg.deformation_amplitude.abs() > 1e-6 {
                println!(
                    "[GEOMETRÍA] Aplicando deformación procedural a '{}' (amp: {}, freq: {}, seed universal: {})",
                    p_cfg.id,
                    p_cfg.deformation_amplitude,
                    p_cfg.deformation_frequency,
                    config.simulation.seed
                );
                mesh.deform(
                    p_cfg.deformation_amplitude,
                    p_cfg.deformation_frequency,
                    config.simulation.seed,
                );
            }

            let geo_radius = mesh.geometric_radius().max(1e-4);
            let p_scale = p_cfg.radius / geo_radius;
            let p_mesh = Arc::new(mesh);

            let mut surface_groups = Vec::new();
            for s_cfg in &p_cfg.surface_objects {
                println!(
                    "[RECURSOS] Cargando objeto de superficie '{}' para planeta '{}': {}",
                    s_cfg.name, p_cfg.id, s_cfg.model_path
                );
                let s_mesh = Arc::new(Mesh::load_from_obj(Path::new(&s_cfg.model_path))?);
                let embedding =
                    calculate_surface_embedding(p_cfg.radius, &s_mesh.aabb, s_cfg.scale);
                let effective_radius = (p_cfg.radius - embedding).max(1.0);
                let instances = generate_spherical_distribution(
                    config.simulation.seed,
                    s_cfg.count,
                    effective_radius,
                    s_cfg.scale,
                );
                surface_groups.push(SurfaceGroup::new(
                    s_cfg.name.clone(),
                    s_mesh,
                    instances,
                    s_cfg.max_distance,
                    s_cfg.fade_range,
                ));
            }

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
                surface_groups,
            });
        }

        // 2. Estado de la nave y cámara
        let ship_state = ShipState::new(&config.ship);
        let combat = CombatSystem::new(&config.enemies);
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
        let normal_mapped_shader = NormalMappedShader::new(sun_body.position, camera.eye);
        let enemy_shader = AlienEnergyShader::new(sun_body.position, camera.eye);
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
            enemy_mesh,
            enemy_scale,
            projectile_mesh,
            combat,
            sun_body,
            planets,
            sim_time: 0.0,
            accumulator: 0.0,
            shader_mode: ShaderMode::Normal,
            planet_shader: TexturedDiffuseShader::new(Vec3::zero()),
            normal_mapped_shader,
            rock_shader,
            plasma_shader,
            ship_shader,
            enemy_shader,
            enemy_material: Material::new("EnemyHull", Vec3::new(0.55, 0.72, 1.0), None),
            projectile_shader: EmissiveShader::new(1.0),
            player_projectile_material: Material::new(
                "PlayerProjectile",
                Vec3::new(0.15, 0.9, 1.0),
                None,
            ),
            enemy_projectile_material: Material::new(
                "EnemyProjectile",
                Vec3::new(1.0, 0.15, 0.05),
                None,
            ),
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
        println!("  [Espacio]        : Disparar");
        println!("  [F4]             : Activar/desactivar encuentro con enemigos");
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
            self.combat.restore_player_health(&self.config.enemies);
            println!("[SIMULACIÓN] Nave reiniciada a posición inicial.");
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F4) {
            if self.combat.encounter_active() {
                self.combat.deactivate();
                println!("[ENCUENTRO] Desactivado. El sistema vuelve a modo pacífico.");
            } else {
                self.combat.activate(
                    self.ship_state.position,
                    self.sun_body.position,
                    self.sun_body.radius,
                    &self.config.enemies,
                );
                println!(
                    "[ENCUENTRO] Activado: {} visitantes aparecieron detrás del Sol en modo supervisión.",
                    self.combat.enemies.len()
                );
            }
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
            fire: rl.is_key_down(KeyboardKey::KEY_SPACE),
        }
    }

    /// Actualiza la simulación física de la nave, órbitas, rotaciones axiales y colisiones.
    pub fn update_simulation(&mut self, input: &ShipInput, dt: f32) {
        self.sim_time += dt;

        // Actualización del tiempo para shaders animados
        self.plasma_shader.set_time(self.sim_time);
        self.enemy_shader.set_time(self.sim_time);

        // Movimiento de la nave
        self.ship_state.update(input, &self.config.ship, dt);

        // Órbitas y rotación axial calculadas analíticamente por tiempo
        self.sun_body.update(self.sim_time);
        for planet in &mut self.planets {
            planet.update(self.sim_time);
        }

        // Envolventes compartidas: Sol, planetas y objetos de superficie.
        let obstacles = self.collect_collision_obstacles();
        for &obstacle in &obstacles {
            resolve_sphere_collision(
                &mut self.ship_state.position,
                &mut self.ship_state.velocity,
                self.ship_state.collision_radius,
                obstacle,
            );
        }

        if input.fire {
            self.combat.try_player_fire(
                self.sim_time,
                self.ship_state.position,
                self.ship_state.forward(),
                self.ship_state.velocity,
                &self.config.enemies,
            );
        }

        let combat_update = self.combat.update(
            self.sim_time,
            dt,
            self.ship_state.position,
            &self.config.enemies,
        );
        self.combat
            .resolve_obstacle_collisions(&obstacles, &self.config.enemies);
        if combat_update.attack_started {
            println!(
                "[ENCUENTRO] Un visitante fue destruido. Todos los enemigos entran en modo ATAQUE."
            );
        }
        if combat_update.peace_started {
            println!(
                "[ENCUENTRO] 20 segundos sin disparos: los enemigos detienen su movimiento y vuelven a observación pacífica."
            );
        }
        if let Some(wave) = combat_update.wave_spawned {
            println!(
                "[ENCUENTRO] Oleada {} desplegada con {} enemigos.",
                wave,
                self.combat.enemies.len()
            );
        }
        if combat_update.player_damage > 0.0 {
            println!(
                "[NAVE] Daño recibido: {:.0}. Vida: {:.0}/{:.0}",
                combat_update.player_damage,
                self.combat.player_health,
                self.config.enemies.player_max_health
            );
        }
        if combat_update.player_destroyed {
            println!(
                "[NAVE] Nave destruida. Reaparición automática en {:.0} segundos.",
                self.config.enemies.player_respawn_delay
            );
        }
        if combat_update.player_respawned {
            self.ship_state.reset(&self.config.ship);
            println!("[NAVE] Nave restaurada con vida completa.");
        }
    }

    fn collect_collision_obstacles(&self) -> Vec<CollisionSphere> {
        let surface_count = self
            .planets
            .iter()
            .flat_map(|planet| &planet.surface_groups)
            .map(|group| group.instances.len())
            .sum::<usize>();
        let mut obstacles = Vec::with_capacity(1 + self.planets.len() + surface_count);

        obstacles.push(CollisionSphere::new(
            self.sun_body.position,
            self.sun_body.radius + self.sun_body.collision_margin,
        ));

        for planet in &self.planets {
            obstacles.push(CollisionSphere::new(
                planet.position,
                planet.radius + planet.collision_margin,
            ));

            let planet_rotation = planet.rotation_matrix();
            let planet_translation = Mat4::from_translation(planet.position);
            for group in &planet.surface_groups {
                let local_mesh_center = group.mesh.aabb.center();
                let mesh_radius = group.mesh.geometric_radius();
                for instance in &group.instances {
                    let object_transform = planet_translation
                        * planet_rotation
                        * Mat4::from_translation(instance.local_pos)
                        * instance.local_basis
                        * Mat4::uniform_scaling(instance.base_scale);
                    let center = object_transform.transform_point(local_mesh_center);
                    let radius = (mesh_radius * instance.base_scale).max(0.15);
                    obstacles.push(CollisionSphere::new(center, radius));
                }
            }
        }

        obstacles
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
        let frustum = self.camera.frustum();
        self.toon_shader.camera_pos = self.camera.eye;
        self.toon_shader.light_pos = self.sun_body.position;
        self.ship_shader.camera_pos = self.camera.eye;
        self.ship_shader.light_pos = self.sun_body.position;
        self.enemy_shader.camera_pos = self.camera.eye;
        self.enemy_shader.light_pos = self.sun_body.position;
        self.plasma_shader.sun_center = self.sun_body.position;
        self.rock_shader.light_pos = self.sun_body.position;
        self.planet_shader.light_pos = self.sun_body.position;
        self.normal_mapped_shader.light_pos = self.sun_body.position;
        self.normal_mapped_shader.camera_pos = self.camera.eye;

        let mut target = RenderTarget {
            framebuffer: &mut self.framebuffer,
            depth_buffer: &mut self.depth_buffer,
        };

        // 1. Sol (Shader animado de plasma con Frustum Culling)
        if !frustum.intersects_sphere(self.sun_body.position, self.sun_body.radius) {
            stats.objetos_descartados_frustum += 1;
            stats.triangulos_descartados_frustum += self.sun_body.mesh.indices.len() / 3;
        } else {
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
        }

        // 2. Planetas (con Frustum Culling a nivel de astro y de corteza)
        for planet in &self.planets {
            let max_surface_extent = planet
                .surface_groups
                .iter()
                .map(|g| g.mesh.aabb.size().y * g.instances.first().map_or(0.0, |i| i.base_scale))
                .fold(0.0f32, f32::max);
            let planet_cull_radius = planet.radius + max_surface_extent;
            let planet_visible = frustum.intersects_sphere(planet.position, planet_cull_radius);
            if !planet_visible {
                // Si el planeta completo está fuera de visión, descartarlo y omitir todas sus montañas
                stats.objetos_descartados_frustum += 1;
                stats.triangulos_descartados_frustum += planet.mesh.indices.len() / 3;
                for group in &planet.surface_groups {
                    stats.objetos_descartados_frustum += group.instances.len();
                    stats.triangulos_descartados_frustum +=
                        (group.mesh.indices.len() / 3) * group.instances.len();
                }
                continue;
            }

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
            } else if planet.shader_type == "normal_mapped"
                || (planet.shader_type == "texture" && planet.material.normal_texture.is_some())
            {
                Self::draw_mesh(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.normal_mapped_shader,
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

            // 2.1 Objetos de superficie del planeta (con descarte por distancia, horizonte y frustum individual)
            for group in &planet.surface_groups {
                let dist_to_planet_center = (self.camera.eye - planet.position).length();
                let dist_to_surface = (dist_to_planet_center - planet.radius).max(0.0);

                // Descarte lejano O(1): si la cámara está fuera del radio de visibilidad, omitir todo el grupo
                if dist_to_surface >= group.max_distance {
                    continue;
                }

                let fade =
                    calculate_fade_factor(dist_to_surface, group.max_distance, group.fade_range);
                if fade <= 1e-4 {
                    continue;
                }

                let p_rot = planet.rotation_matrix();
                let p_trans = Mat4::from_translation(planet.position);
                let mesh_extent = group.mesh.aabb.max_extent();

                for instance in &group.instances {
                    // Descarte de cara oculta/horizonte del planeta: si el objeto se ubica en el hemisferio
                    // posterior respecto a la cámara, el cuerpo sólido opaco del planeta lo oculta por completo.
                    let world_normal = p_rot.transform_direction(instance.local_normal);
                    let to_cam_from_center = self.camera.eye - planet.position;
                    if world_normal.dot(to_cam_from_center) < -0.15 {
                        continue;
                    }

                    // Frustum culling individual para cada montaña en pantalla
                    let current_scale = instance.base_scale * fade;
                    let instance_world_pos =
                        planet.position + p_rot.transform_point(instance.local_pos);
                    let instance_radius = mesh_extent * current_scale;
                    if !frustum.intersects_sphere(instance_world_pos, instance_radius) {
                        stats.objetos_descartados_frustum += 1;
                        stats.triangulos_descartados_frustum += group.mesh.indices.len() / 3;
                        continue;
                    }

                    let obj_model = p_trans
                        * p_rot
                        * Mat4::from_translation(instance.local_pos)
                        * instance.local_basis
                        * Mat4::uniform_scaling(current_scale);
                    let obj_rot = p_rot * instance.local_basis;

                    Self::draw_mesh(
                        &mut self.renderer,
                        self.shader_mode,
                        &self.depth_shader,
                        &self.normals_shader,
                        &self.toon_shader,
                        &self.planet_shader,
                        &group.mesh,
                        &obj_model,
                        &obj_rot,
                        &vp_matrix,
                        &group.mesh.material,
                        &mut target,
                        stats,
                    );
                }
            }
        }

        // 3. Enemigos y proyectiles (con Frustum Culling). En modo pacífico ambas colecciones están vacías.
        let enemy_center = self.enemy_mesh.aabb.center();
        let enemy_radius = self.enemy_mesh.geometric_radius() * self.enemy_scale;
        for enemy in &self.combat.enemies {
            if !frustum.intersects_sphere(enemy.position, enemy_radius) {
                stats.objetos_descartados_frustum += 1;
                stats.triangulos_descartados_frustum += self.enemy_mesh.indices.len() / 3;
                continue;
            }

            // El OBJ enemigo fue exportado con la punta hacia +X. La corrección
            // configurable alinea esa punta con la dirección exacta a la nave.
            let enemy_rotation = enemy.rotation_matrix()
                * Mat4::rotation_y(deg_to_rad(self.config.enemies.model_yaw_offset_deg));
            let enemy_model = Mat4::from_translation(enemy.position)
                * enemy_rotation
                * Mat4::uniform_scaling(self.enemy_scale)
                * Mat4::from_translation(-enemy_center);
            Self::draw_mesh(
                &mut self.renderer,
                self.shader_mode,
                &self.depth_shader,
                &self.normals_shader,
                &self.toon_shader,
                &self.enemy_shader,
                &self.enemy_mesh,
                &enemy_model,
                &enemy_rotation,
                &vp_matrix,
                &self.enemy_material,
                &mut target,
                stats,
            );
        }

        for projectile in &self.combat.projectiles {
            if !frustum.intersects_sphere(projectile.position, projectile.radius) {
                stats.objetos_descartados_frustum += 1;
                stats.triangulos_descartados_frustum += self.projectile_mesh.indices.len() / 3;
                continue;
            }

            let projectile_model = Mat4::from_translation(projectile.position)
                * Mat4::uniform_scaling(projectile.radius);
            let material = match projectile.owner {
                ProjectileOwner::Player => &self.player_projectile_material,
                ProjectileOwner::Enemy => &self.enemy_projectile_material,
            };
            Self::draw_mesh(
                &mut self.renderer,
                self.shader_mode,
                &self.depth_shader,
                &self.normals_shader,
                &self.toon_shader,
                &self.projectile_shader,
                &self.projectile_mesh,
                &projectile_model,
                &Mat4::IDENTITY,
                &vp_matrix,
                material,
                &mut target,
                stats,
            );
        }

        // 4. Nave (con Frustum Culling)
        let ship_radius = self.ship_mesh.geometric_radius() * self.ship_scale;
        if !frustum.intersects_sphere(self.ship_state.position, ship_radius) {
            stats.objetos_descartados_frustum += 1;
            stats.triangulos_descartados_frustum += self.ship_mesh.indices.len() / 3;
        } else {
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

        // Las barras se dibujan al final como overlay del framebuffer CPU.
        Self::draw_entity_health_bars(
            &mut self.framebuffer,
            &vp_matrix,
            self.ship_state.position,
            self.combat.player_health,
            self.config.enemies.player_max_health,
            &self.combat.enemies,
            self.config.enemies.target_length_wu,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_entity_health_bars(
        framebuffer: &mut Framebuffer,
        view_projection: &Mat4,
        ship_position: Vec3,
        ship_health: f32,
        ship_max_health: f32,
        enemies: &[Enemy],
        enemy_height: f32,
    ) {
        let width = framebuffer.width;
        let height = framebuffer.height;

        if let Some((x, y)) = project_world_to_screen(
            view_projection,
            ship_position + Vec3::up() * 1.0,
            width,
            height,
        ) {
            draw_health_bar(
                framebuffer,
                x,
                y - 8,
                54,
                7,
                ship_health / ship_max_health.max(1e-6),
            );
        }

        for enemy in enemies {
            let anchor = enemy.position + Vec3::up() * (enemy_height * 0.65);
            if let Some((x, y)) = project_world_to_screen(view_projection, anchor, width, height) {
                draw_health_bar(
                    framebuffer,
                    x,
                    y - 6,
                    40,
                    6,
                    enemy.health / enemy.max_health.max(1e-6),
                );
            }
        }
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
