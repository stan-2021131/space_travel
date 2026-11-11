use crate::buffer::{DepthBuffer, Framebuffer};
use crate::camera::Camera;
use crate::combat::{CombatSystem, Enemy, ProjectileOwner, create_projectile_mesh};
use crate::config::{AppConfig, LoreConfig, ThrusterFlameConfig};
use crate::hud::{
    CelestialInspectionData, HudContext, PlanetMarker, draw_health_bar, draw_orbital_lock_reticle,
    draw_speed_streaks, project_world_to_screen, render_hud,
};
use crate::material::Material;
use crate::math::{Mat4, Vec3, deg_to_rad};
use crate::mesh::Mesh;
use crate::particles::SparkSystem;
use crate::physics::{
    CollisionSphere, ShipInput, ShipState, resolve_sphere_collision_with_rebound,
    solar_damage_per_second,
};
use crate::pipeline::{
    AlienEnergyShader, BlendMode, DepthDebugShader, DiffuseShader, EarthLikeShader, EmissiveShader,
    ForceFieldShader, FragmentShader, NormalMappedShader, NormalsDebugShader, PlasmaShader,
    ProceduralRockShader, RenderContext, RenderTarget, ShaderMode, ShipShader, SkyShader,
    SoftwareRenderer, TexturedDiffuseShader, ThrusterFlameShader, ToonShader,
};
use crate::scene::{
    CelestialBody, CelestialComponent, DebrisRing, DysonSphere, Moon, ThrusterFlame,
    surface::{
        SurfaceGroup, calculate_fade_factor, calculate_surface_embedding,
        generate_spherical_distribution, generate_torus_surface_distribution,
    },
};
use crate::stats::{FrameStats, StatsTracker};
use raylib::prelude::*;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Identificador de astro objetivo para interacción y vista panorámica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectedCelestial {
    Sun,
    Planet(usize),
}

/// Aplicación principal que orquesta el ciclo de simulación espacial, control de nave,
/// cámara de persecución y renderizado por software en CPU.
pub struct App {
    config: AppConfig,
    pub lore_config: LoreConfig,
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
    thruster_flame_mesh: Arc<Mesh>,
    ship_thruster_flames: Vec<ThrusterFlame>,
    thruster_flame_material: Material,
    pub combat: CombatSystem,
    pub sparks: SparkSystem,

    sun_body: CelestialBody,
    pub dyson_sphere: Option<DysonSphere>,
    planets: Vec<CelestialBody>,

    // Estado de simulación
    sim_time: f32,
    accumulator: f32,
    pub shader_mode: ShaderMode,
    pub gravity_enabled: bool,
    pub local_flight_mode: bool,
    pub nearby_planet_idx: Option<usize>,
    pub nearby_celestial: Option<InspectedCelestial>,
    pub story_box_open: bool,
    pub story_box_page: usize,
    ship_thrust_state: f32,
    pub impact_shake_timer: f32,
    pub impact_shake_intensity: f32,

    // Shaders reutilizables
    planet_shader: TexturedDiffuseShader,
    normal_mapped_shader: NormalMappedShader,
    rock_shader: ProceduralRockShader,
    earth_shader: EarthLikeShader,
    plasma_shader: PlasmaShader,
    diffuse_shader: DiffuseShader,
    force_field_shader: ForceFieldShader,
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
    pub show_controls: bool,
    pub show_stats_panel: bool,
    pub first_person: bool,
}

/// Contexto de parámetros para la construcción de datos de inspección telescópica / orbital.
pub struct InspectionContext<'a> {
    pub nearby: Option<InspectedCelestial>,
    pub planets: &'a [CelestialBody],
    pub planet_configs: &'a [crate::config::PlanetConfig],
    pub sun_body: &'a CelestialBody,
    pub sun_config: &'a crate::config::SunConfig,
    pub dyson_sphere: Option<&'a DysonSphere>,
    pub lore_config: &'a LoreConfig,
    pub story_box_page: usize,
    pub story_box_open: bool,
}

impl App {
    pub const WINDOW_WIDTH: usize = 1280;
    pub const WINDOW_HEIGHT: usize = 720;

    pub fn new(config_path: &Path) -> Result<Self, String> {
        println!("[APP] Cargando configuración desde '{:?}'...", config_path);
        let config = AppConfig::load_from_file(config_path)?;

        let lore_config = match LoreConfig::load_from_file(Path::new("config/lore.toml")) {
            Ok(l) => {
                println!(
                    "[LORE] Archivo de bitácora y lore 'config/lore.toml' cargado exitosamente."
                );
                l
            }
            Err(e) => {
                println!(
                    "[LORE] Advertencia: No se pudo cargar 'config/lore.toml': {}. Se usarán valores predeterminados.",
                    e
                );
                LoreConfig::default()
            }
        };

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
        let thruster_flame_mesh = Arc::new(crate::procedural::generate_thruster_flame(12));
        let ship_thruster_flames = config
            .ship
            .thruster_flames
            .iter()
            .map(Self::create_thruster_flame)
            .collect();

        println!(
            "[RECURSOS] Cargando modelo del sol: {}",
            config.sun.model_path
        );
        let sun_mesh = Arc::new(Mesh::load_from_obj(Path::new(&config.sun.model_path))?);
        let sun_geo_radius = sun_mesh.geometric_radius().max(1e-4);
        let sun_scale = config.sun.radius / sun_geo_radius;

        let sun_effective_grav_radius = config.sun.effective_gravity_radius();
        let sun_grav_mesh = if config.sun.visible_gravity {
            Some(Arc::new(crate::procedural::generate_icosphere(1.0, 2)))
        } else {
            None
        };

        let sun_body = CelestialBody {
            id: config.sun.id.clone(),
            parent_id: None,
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
            debris_rings: Vec::new(),
            components: Vec::new(),
            thruster_flames: Vec::new(),
            moons: Vec::new(),
            gravity_strength: config.sun.gravity_strength,
            gravity_radius: sun_effective_grav_radius,
            visible_gravity: config.sun.visible_gravity,
            axial_tilt_rad: 0.0,
            gravity_mesh: sun_grav_mesh,
        };

        // 1.b Esfera de Dyson (si está habilitada en la configuración)
        let dyson_sphere = if let Some(ref d_cfg) = config.dyson_sphere {
            if d_cfg.enabled {
                println!(
                    "[RECURSOS] Cargando modelo de Esfera de Dyson: {}",
                    d_cfg.model_path
                );
                let dyson_submeshes = Mesh::load_models_from_obj(Path::new(&d_cfg.model_path))?;
                let dyson_meshes = dyson_submeshes.into_iter().map(Arc::new).collect();

                Some(DysonSphere::new(
                    dyson_meshes,
                    d_cfg.radius,
                    d_cfg.rotation_speed,
                    d_cfg.effective_collision_radius(),
                    d_cfg.effective_gravity_radius(),
                    d_cfg.gravity_strength,
                ))
            } else {
                None
            }
        } else {
            None
        };

        let mut planets = Vec::new();
        for (planet_index, p_cfg) in config.planets.iter().enumerate() {
            let mut auto_components = Vec::new();
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
                let mut loaded_models = Mesh::load_models_from_obj(Path::new(&p_cfg.model_path))?;
                let mut primary_mesh = loaded_models.remove(0);
                for _ in 0..p_cfg.subdivisions {
                    primary_mesh.subdivide();
                }
                for (sub_idx, mut sub_mesh) in loaded_models.into_iter().enumerate() {
                    for _ in 0..p_cfg.subdivisions {
                        sub_mesh.subdivide();
                    }
                    auto_components.push(CelestialComponent::new(
                        format!("{}_part_{}", p_cfg.id, sub_idx + 1),
                        Arc::new(sub_mesh),
                    ));
                }
                primary_mesh
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

            let mut max_geo_radius = mesh.geometric_radius();
            for comp in &auto_components {
                max_geo_radius = max_geo_radius.max(comp.origin_radius);
            }
            let geo_radius = max_geo_radius.max(1e-4);
            let p_scale = p_cfg.radius / geo_radius;
            let p_mesh = Arc::new(mesh);

            let mut surface_groups = Vec::new();
            let is_toroidal = p_mesh.aabb.size().y < p_mesh.aabb.size().x * 0.6;

            for (group_idx, s_cfg) in p_cfg.surface_objects.iter().enumerate() {
                println!(
                    "[RECURSOS] Cargando objeto de superficie '{}' para planeta '{}': {}",
                    s_cfg.name, p_cfg.id, s_cfg.model_path
                );
                let s_mesh = Arc::new(Mesh::load_from_obj(Path::new(&s_cfg.model_path))?);
                let embedding =
                    calculate_surface_embedding(p_cfg.radius, &s_mesh.aabb, s_cfg.scale);
                let effective_radius = (p_cfg.radius - embedding).max(1.0);

                let group_seed = config
                    .simulation
                    .seed
                    .wrapping_add((group_idx as u32).wrapping_mul(1013904223));

                let instances = if is_toroidal {
                    generate_torus_surface_distribution(
                        group_seed,
                        s_cfg.count,
                        &p_mesh,
                        p_scale,
                        s_cfg.scale,
                        embedding,
                    )
                } else {
                    generate_spherical_distribution(
                        group_seed,
                        s_cfg.count,
                        effective_radius,
                        s_cfg.scale,
                    )
                };

                surface_groups.push(SurfaceGroup::new(
                    s_cfg.name.clone(),
                    s_mesh,
                    instances,
                    s_cfg.max_distance,
                    s_cfg.fade_range,
                ));
            }

            let p_effective_grav_radius = p_cfg.effective_gravity_radius();
            let p_grav_mesh = if p_cfg.visible_gravity {
                Some(Arc::new(crate::procedural::generate_icosphere(1.0, 2)))
            } else {
                None
            };

            let mut debris_rings = Vec::new();
            for (ring_index, ring_cfg) in p_cfg.debris_rings.iter().enumerate() {
                let mut meshes = Vec::with_capacity(ring_cfg.model_paths.len());
                for model_path in &ring_cfg.model_paths {
                    println!(
                        "[RECURSOS] Cargando escombro orbital para planeta '{}': {}",
                        p_cfg.id, model_path
                    );
                    meshes.push(Arc::new(Mesh::load_from_obj(Path::new(model_path))?));
                }
                let ring_seed = config.simulation.seed
                    ^ (planet_index as u32).wrapping_mul(0x9E37_79B9)
                    ^ (ring_index as u32).wrapping_mul(0x85EB_CA6B);
                debris_rings.push(DebrisRing::generate(
                    meshes,
                    ring_seed,
                    ring_cfg.count,
                    ring_cfg.inner_radius,
                    ring_cfg.outer_radius,
                    ring_cfg.tilt_deg,
                    ring_cfg.rotation_period,
                    ring_cfg.min_scale,
                    ring_cfg.max_scale,
                    ring_cfg.height_variation,
                    ring_cfg.min_spin_period,
                    ring_cfg.max_spin_period,
                    ring_cfg.collision_margin,
                ));
            }

            let mut components = Vec::with_capacity(auto_components.len() + p_cfg.components.len());
            components.extend(auto_components);
            for component_cfg in &p_cfg.components {
                println!(
                    "[RECURSOS] Cargando componente '{}' para planeta '{}': {}",
                    component_cfg.name, p_cfg.id, component_cfg.model_path
                );
                let component_mesh =
                    Arc::new(Mesh::load_from_obj(Path::new(&component_cfg.model_path))?);
                components.push(CelestialComponent::new(
                    component_cfg.name.clone(),
                    component_mesh,
                ));
            }
            let thruster_flames = p_cfg
                .thruster_flames
                .iter()
                .map(Self::create_thruster_flame)
                .collect();
            let mut moons = Vec::with_capacity(p_cfg.moons.len());
            for moon_cfg in &p_cfg.moons {
                println!(
                    "[RECURSOS] Cargando luna '{}' para planeta '{}': {}",
                    moon_cfg.name, p_cfg.id, moon_cfg.model_path
                );
                let moon_mesh = Arc::new(Mesh::load_from_obj(Path::new(&moon_cfg.model_path))?);
                let moon_scale = moon_cfg.radius / moon_mesh.geometric_radius().max(1e-4);
                moons.push(Moon {
                    name: moon_cfg.name.clone(),
                    material: moon_mesh.material.clone(),
                    mesh: moon_mesh,
                    scale: moon_scale,
                    radius: moon_cfg.radius,
                    orbital_radius: moon_cfg.orbital_radius,
                    orbital_period: moon_cfg.orbital_period,
                    rotation_period: moon_cfg.rotation_period,
                    initial_orbital_angle_rad: deg_to_rad(moon_cfg.initial_orbital_angle_deg),
                    orbit_tilt_rad: deg_to_rad(moon_cfg.orbit_tilt_deg),
                    collision_margin: moon_cfg.collision_margin,
                });
            }

            planets.push(CelestialBody {
                id: p_cfg.id.clone(),
                parent_id: p_cfg.parent_id.clone(),
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
                debris_rings,
                components,
                thruster_flames,
                moons,
                gravity_strength: p_cfg.gravity_strength,
                gravity_radius: p_effective_grav_radius,
                visible_gravity: p_cfg.visible_gravity,
                axial_tilt_rad: deg_to_rad(p_cfg.axial_tilt_deg),
                gravity_mesh: p_grav_mesh,
            });
        }

        // Posiciones orbitales iniciales: primero planetas primarios, luego lunas dependientes
        for planet in &mut planets {
            if planet.parent_id.is_none() {
                planet.update(0.0);
            }
        }
        for i in 0..planets.len() {
            let center = if let Some(ref pid) = planets[i].parent_id {
                planets
                    .iter()
                    .position(|p| &p.id == pid)
                    .map(|idx| planets[idx].position)
            } else {
                None
            };
            if let Some(center) = center {
                planets[i].update_with_parent_position(0.0, center);
            }
        }

        // 2. Estado de la nave y cámara
        let ship_state = ShipState::new(&config.ship);
        let combat = CombatSystem::new(&config.enemies, config.ship.max_speed);
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
        let earth_shader =
            EarthLikeShader::new(config.simulation.seed, sun_body.position, camera.eye);
        let plasma_shader = PlasmaShader::new();
        let diffuse_shader = DiffuseShader::new();
        let toon_shader = ToonShader::new(sun_body.position, camera.eye);
        let ship_shader = ShipShader::new(sun_body.position, camera.eye);
        let normal_mapped_shader = NormalMappedShader::new(sun_body.position, camera.eye);
        let enemy_shader = AlienEnergyShader::new(sun_body.position, camera.eye);
        let sky_shader = SkyShader::new(config.simulation.seed);

        Ok(Self {
            config,
            lore_config,
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
            thruster_flame_mesh,
            ship_thruster_flames,
            thruster_flame_material: Material::new("ThrusterFlame", Vec3::new(1.0, 1.0, 1.0), None),
            combat,
            sparks: SparkSystem::new(512),
            sun_body,
            dyson_sphere,
            planets,
            sim_time: 0.0,
            accumulator: 0.0,
            shader_mode: ShaderMode::Normal,
            gravity_enabled: true,
            local_flight_mode: false,
            nearby_planet_idx: None,
            nearby_celestial: None,
            story_box_open: true,
            story_box_page: 0,
            ship_thrust_state: 0.0,
            impact_shake_timer: 0.0,
            impact_shake_intensity: 0.0,
            planet_shader: TexturedDiffuseShader::new(Vec3::zero()),
            normal_mapped_shader,
            rock_shader,
            earth_shader,
            plasma_shader,
            diffuse_shader,
            force_field_shader: ForceFieldShader::new(camera.eye),
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
            show_controls: true,
            show_stats_panel: false,
            first_person: false,
        })
    }

    fn create_thruster_flame(config: &ThrusterFlameConfig) -> ThrusterFlame {
        ThrusterFlame::from_degrees(
            config.position,
            config.rotation_deg,
            config.radius,
            config.length,
            config.animation_speed,
            config.idle_intensity,
            config.active_intensity,
        )
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
        println!("  [Shift Izq. + W] : Activar acelerador de alta velocidad");
        println!("  [S]              : Frenar / Aplicar reversa");
        println!("  [A] / [D]        : Yaw (Giro horizontal)");
        println!("  [Flecha Arriba]  : Pitch arriba");
        println!("  [Flecha Abajo]   : Pitch abajo");
        println!("  [Q] / [E]        : Roll (Alabeo)");
        println!("  [R]              : Reiniciar posición y orientación de la nave");
        println!("  [G]              : Alternar gravedad planetaria (ON / OFF)");
        println!(
            "  [C]              : Alternar modo de control local orbital (cerca de un planeta)"
        );
        println!("  [V]              : Alternar cámara (Cabina 1ra Persona / Persecución 3ra)");
        println!("  [H] / [F1]       : Ocultar / Mostrar panel de controles HUD");
        println!("  [Espacio]        : Disparar");
        println!("  [F4]             : Activar/desactivar encuentro con enemigos");
        println!(
            "  [F2]             : Alternar modo de shader (Normal -> Toon -> Depth -> Normals)"
        );
        println!(
            "  [F3]             : Alternar panel de estadísticas HUD (FPS, tiempos, geometrías)"
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

            // 3. Suavizado de la cámara (modo cabina 1ra persona, panorámico centrado en planeta, o persecución 3ra persona)
            if self.first_person {
                let cockpit_pos = self.ship_state.position
                    + self.ship_state.forward() * 0.9
                    + self.ship_state.up() * 0.25;
                let look_target = cockpit_pos + self.ship_state.forward() * 50.0;
                self.camera.eye = cockpit_pos;
                self.camera.target = look_target;
                self.camera.up = self.ship_state.up();
            } else if self.local_flight_mode {
                match self.nearby_celestial {
                    Some(InspectedCelestial::Planet(idx)) => {
                        if let Some(planet) = self.planets.get(idx) {
                            self.camera.update_panoramic_planet_view(
                                planet.position,
                                planet.radius,
                                &self.config.camera,
                                dt,
                            );
                        }
                    }
                    Some(InspectedCelestial::Sun) => {
                        let sun_radius = if let Some(ref d) = self.dyson_sphere {
                            d.radius
                        } else {
                            self.sun_body.radius
                        };
                        self.camera.update_panoramic_planet_view(
                            self.sun_body.position,
                            sun_radius,
                            &self.config.camera,
                            dt,
                        );
                    }
                    None => {
                        self.camera.update_chase(
                            self.ship_state.position,
                            self.ship_state.forward(),
                            self.ship_state.up(),
                            &self.config.camera,
                            dt,
                        );
                    }
                }
            } else {
                self.camera.update_chase(
                    self.ship_state.position,
                    self.ship_state.forward(),
                    self.ship_state.up(),
                    &self.config.camera,
                    dt,
                );
            }

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
            self.show_stats_panel = !self.show_stats_panel;
            self.stats_tracker.cycle_mode();
        }
    }

    fn read_input(&mut self, rl: &RaylibHandle) -> ShipInput {
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            self.ship_state.reset(&self.config.ship);
            self.combat.restore_player_health(&self.config.enemies);
            println!("[SIMULACIÓN] Nave reiniciada a posición inicial.");
        }

        if rl.is_key_pressed(KeyboardKey::KEY_G) {
            self.gravity_enabled = !self.gravity_enabled;
            println!(
                "[FÍSICA] Gravedad planetaria: {}",
                if self.gravity_enabled {
                    "ACTIVADA"
                } else {
                    "DESACTIVADA"
                }
            );
        }

        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            if self.nearby_celestial.is_some() {
                self.local_flight_mode = !self.local_flight_mode;
                if self.local_flight_mode {
                    self.story_box_open = true;
                    self.story_box_page = 0;
                }
                let target_name = match self.nearby_celestial {
                    Some(InspectedCelestial::Planet(idx)) => self
                        .planets
                        .get(idx)
                        .map(|p| p.id.as_str())
                        .unwrap_or("planeta"),
                    Some(InspectedCelestial::Sun) => "Sol",
                    None => "cuerpo",
                };
                println!(
                    "[NAVEGACIÓN] Modo de control local orbital: {} (fijado a {})",
                    if self.local_flight_mode {
                        "ACTIVADO"
                    } else {
                        "DESACTIVADO"
                    },
                    target_name
                );
            } else {
                println!(
                    "[NAVEGACIÓN] No hay cuerpos celestes en rango de aproximación para modo de control local."
                );
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) && self.local_flight_mode {
            let total_pages = self.get_current_lore_total_pages();
            if self.story_box_open {
                if self.story_box_page + 1 < total_pages {
                    self.story_box_page += 1;
                } else {
                    // Última página alcanzada: cerrar el registro de bitácora
                    self.story_box_open = false;
                }
            } else {
                // Reabrir registro desde el inicio
                self.story_box_open = true;
                self.story_box_page = 0;
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_V) {
            self.first_person = !self.first_person;
            println!(
                "[CÁMARA] Vista: {}",
                if self.first_person {
                    "CABINA 1RA PERSONA"
                } else {
                    "PERSECUCIÓN 3RA PERSONA"
                }
            );
        }

        if rl.is_key_pressed(KeyboardKey::KEY_H) || rl.is_key_pressed(KeyboardKey::KEY_F1) {
            self.show_controls = !self.show_controls;
            println!(
                "[HUD] Panel de controles: {}",
                if self.show_controls {
                    "VISIBLE"
                } else {
                    "OCULTO"
                }
            );
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
                    "[ENCUENTRO] Activado: {} visitantes aparecieron detrás del Sol en aproximación inicial.",
                    self.combat.enemies.len()
                );
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F2) {
            self.cycle_shader_mode();
        }

        if rl.is_key_pressed(KeyboardKey::KEY_F3) {
            self.show_stats_panel = !self.show_stats_panel;
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
            boost: rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT),
            fire: rl.is_key_down(KeyboardKey::KEY_SPACE),
        }
    }

    /// Actualiza la simulación física de la nave, órbitas, rotaciones axiales y colisiones.
    pub fn update_simulation(&mut self, input: &ShipInput, dt: f32) {
        self.sim_time += dt;

        // Actualización del tiempo para shaders animados
        self.plasma_shader.set_time(self.sim_time);
        self.enemy_shader.set_time(self.sim_time);
        self.ship_thrust_state = if !self.combat.player_is_alive() {
            0.0
        } else if input.boost && input.throttle_forward {
            2.0
        } else if input.throttle_forward {
            1.0
        } else if input.throttle_reverse {
            -1.0
        } else {
            0.0
        };

        // Una nave destruida permanece ausente e inmóvil hasta reaparecer.
        if self.combat.player_is_alive() {
            self.ship_state.update(input, &self.config.ship, dt);
        } else {
            self.ship_state.velocity = Vec3::zero();
        }

        // Órbitas y rotación axial calculadas analíticamente por tiempo
        self.sun_body.update(self.sim_time);
        if let Some(ref mut dyson) = self.dyson_sphere {
            dyson.update(dt);
        }
        for planet in &mut self.planets {
            if planet.parent_id.is_none() {
                planet.update(self.sim_time);
            }
        }
        for i in 0..self.planets.len() {
            let center = if let Some(ref pid) = self.planets[i].parent_id {
                self.planets
                    .iter()
                    .position(|p| &p.id == pid)
                    .map(|idx| self.planets[idx].position)
            } else {
                None
            };
            if let Some(center) = center {
                self.planets[i].update_with_parent_position(self.sim_time, center);
            }
        }

        // Detección del cuerpo celeste más próximo para aproximación y modo local orbital
        let mut nearest_candidate: Option<(InspectedCelestial, f32)> = None;

        // Verificación del Sol / Esfera de Dyson
        let sun_dist = (self.ship_state.position - self.sun_body.position).length();
        let sun_radius = if let Some(ref d) = self.dyson_sphere {
            d.radius
        } else {
            self.sun_body.radius
        };
        let sun_engagement = (sun_radius * 2.8).max(90.0);
        if sun_dist <= sun_engagement {
            nearest_candidate = Some((InspectedCelestial::Sun, sun_dist));
        }

        // Verificación de planetas
        for (idx, planet) in self.planets.iter().enumerate() {
            let dist = (self.ship_state.position - planet.position).length();
            let engagement_range = planet.gravity_radius.max(planet.radius * 2.8);
            if dist <= engagement_range {
                match nearest_candidate {
                    Some((_, current_min)) if dist < current_min => {
                        nearest_candidate = Some((InspectedCelestial::Planet(idx), dist));
                    }
                    None => {
                        nearest_candidate = Some((InspectedCelestial::Planet(idx), dist));
                    }
                    _ => {}
                }
            }
        }

        self.nearby_celestial = nearest_candidate.map(|(c, _)| c);
        self.nearby_planet_idx = match self.nearby_celestial {
            Some(InspectedCelestial::Planet(idx)) => Some(idx),
            _ => None,
        };

        // Desacople automático si la nave se aleja fuera del radio orbital
        if self.local_flight_mode && self.nearby_celestial.is_none() {
            self.local_flight_mode = false;
            println!(
                "[NAVEGACIÓN] Nave fuera del rango orbital. Retornando a vista de persecución."
            );
        }

        // Atracción gravitatoria hacia cuerpos celestes si está activada
        if self.gravity_enabled && self.combat.player_is_alive() {
            self.ship_state.apply_gravitational_attraction(
                self.sun_body.position,
                self.sun_body.radius,
                self.sun_body.gravity_radius,
                self.sun_body.gravity_strength,
                dt,
            );
            if let Some(ref dyson) = self.dyson_sphere {
                self.ship_state.apply_gravitational_attraction(
                    self.sun_body.position,
                    dyson.radius,
                    dyson.gravity_radius,
                    dyson.gravity_strength,
                    dt,
                );
            }
            for planet in &self.planets {
                self.ship_state.apply_gravitational_attraction(
                    planet.position,
                    planet.radius,
                    planet.gravity_radius,
                    planet.gravity_strength,
                    dt,
                );
            }
        }

        self.impact_shake_timer = (self.impact_shake_timer - dt).max(0.0);

        // Envolventes compartidas: Sol, planetas y objetos de superficie.
        let obstacles = self.collect_collision_obstacles();
        if self.combat.player_is_alive() {
            for &obstacle in &obstacles {
                if let Some(contact) = resolve_sphere_collision_with_rebound(
                    &mut self.ship_state.position,
                    &mut self.ship_state.velocity,
                    self.ship_state.collision_radius,
                    obstacle,
                    0.70,
                ) && contact.impact_speed > 1.5
                {
                    let shake_power = (contact.impact_speed * 0.035).clamp(0.06, 0.35);
                    self.impact_shake_timer = 0.16;
                    self.impact_shake_intensity = shake_power;

                    if let Some(damage) = self.combat.apply_collision_damage(contact.impact_speed) {
                        println!(
                            "[COLISIÓN] Impacto a {:.1} u/s. Daño: {:.1}. Vida: {:.0}/{:.0}",
                            contact.impact_speed,
                            damage,
                            self.combat.player_health,
                            self.config.enemies.player_max_health
                        );
                    }
                    let spark_count = ((contact.impact_speed * 5.0) as usize).clamp(36, 72);
                    self.sparks.emit_sparks(
                        contact.point,
                        contact.normal,
                        spark_count,
                        contact.impact_speed,
                    );
                }
            }
        }

        self.sparks.update(dt);

        if input.fire && self.combat.player_is_alive() {
            self.combat.try_player_fire(
                self.sim_time,
                self.ship_state.position,
                self.ship_state.forward(),
                self.ship_state.velocity,
                &self.config.enemies,
            );
        }

        if self.combat.player_is_alive() {
            let sun_distance = (self.ship_state.position - self.sun_body.position).length();
            let damage_per_second = solar_damage_per_second(
                sun_distance,
                self.sun_body.radius + self.sun_body.collision_margin,
                self.config.sun.damage_radius,
                self.config.sun.max_damage_per_second,
            );
            self.combat.apply_player_damage(damage_per_second * dt);
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
            self.ship_state.velocity = Vec3::zero();
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
        let debris_count = self
            .planets
            .iter()
            .flat_map(|planet| &planet.debris_rings)
            .map(|ring| ring.instances.len())
            .sum::<usize>();
        let moon_count = self
            .planets
            .iter()
            .map(|planet| planet.moons.len())
            .sum::<usize>();
        let mut obstacles =
            Vec::with_capacity(1 + self.planets.len() + surface_count + debris_count + moon_count);

        obstacles.push(CollisionSphere::new(
            self.sun_body.position,
            self.sun_body.radius + self.sun_body.collision_margin,
        ));

        if let Some(ref dyson) = self.dyson_sphere {
            obstacles.push(CollisionSphere::new(
                self.sun_body.position,
                dyson.collision_radius,
            ));
        }

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

            for ring in &planet.debris_rings {
                for instance in &ring.instances {
                    let mesh = &ring.meshes[instance.mesh_index];
                    let (model, _) =
                        ring.instance_matrices(planet.position, instance, self.sim_time);
                    let center = model.transform_point(mesh.aabb.center());
                    let radius = mesh.geometric_radius() * instance.scale + ring.collision_margin;
                    obstacles.push(CollisionSphere::new(center, radius));
                }
            }

            for moon in &planet.moons {
                obstacles.push(CollisionSphere::new(
                    moon.world_position(planet.position, self.sim_time),
                    moon.radius + moon.collision_margin,
                ));
            }
        }

        obstacles
    }

    /// Renderiza todos los cuerpos de la escena continua en el framebuffer propio en CPU.
    pub fn render_scene(&mut self, stats: &mut FrameStats) {
        // Sacudida de impacto (Screen Shake) en la cámara para retroalimentación visual contundente
        let active_camera = if self.impact_shake_timer > 0.0 {
            let shake_amp = self.impact_shake_intensity * (self.impact_shake_timer / 0.16);
            let shake_freq = self.sim_time * 45.0;
            let forward = (self.camera.target - self.camera.eye).normalize();
            let right = forward.cross(self.camera.up).normalize();
            let shake_offset = self.camera.up * (shake_freq.sin() * shake_amp * 0.35)
                + right * ((shake_freq * 1.30).cos() * shake_amp * 0.45);

            let mut cam = self.camera;
            cam.eye += shake_offset;
            cam.target += shake_offset;
            cam
        } else {
            self.camera
        };

        if self.shader_mode == ShaderMode::DepthDebug
            || self.shader_mode == ShaderMode::NormalsDebug
        {
            self.framebuffer.clear(crate::buffer::Color::BLACK);
        } else {
            self.sky_shader
                .render(&mut self.framebuffer, &active_camera);
        }
        self.depth_buffer.clear();

        let vp_matrix = active_camera.view_projection_matrix();
        let frustum = active_camera.frustum();
        self.toon_shader.camera_pos = active_camera.eye;
        self.toon_shader.light_pos = self.sun_body.position;
        self.ship_shader.camera_pos = active_camera.eye;
        self.ship_shader.light_pos = self.sun_body.position;
        self.enemy_shader.camera_pos = active_camera.eye;
        self.enemy_shader.light_pos = self.sun_body.position;
        self.plasma_shader.sun_center = self.sun_body.position;
        self.rock_shader.light_pos = self.sun_body.position;
        self.earth_shader.light_pos = self.sun_body.position;
        self.earth_shader.camera_pos = active_camera.eye;
        self.planet_shader.light_pos = self.sun_body.position;
        self.normal_mapped_shader.light_pos = self.sun_body.position;
        self.normal_mapped_shader.camera_pos = active_camera.eye;

        let mut target = RenderTarget {
            framebuffer: &mut self.framebuffer,
            depth_buffer: &mut self.depth_buffer,
        };

        // 1. Esfera de Dyson (se dibuja antes del Sol para que su estructura opaca
        // escriba en el Z-buffer y descarte por Early-Z los fragmentos cubiertos del costoso PlasmaShader)
        if let Some(ref dyson) = self.dyson_sphere {
            let dyson_cull_radius = dyson.radius * 1.05;

            if frustum.intersects_sphere(self.sun_body.position, dyson_cull_radius) {
                let dyson_model = dyson.model_matrix(self.sun_body.position);
                let dyson_rot = dyson.rotation_matrix();

                // Cada submalla de la esfera se dibuja con su respectivo material cargado del MTL
                for mesh in &dyson.meshes {
                    Self::draw_mesh(
                        &mut self.renderer,
                        self.shader_mode,
                        &self.depth_shader,
                        &self.normals_shader,
                        &self.toon_shader,
                        &self.diffuse_shader,
                        mesh,
                        &dyson_model,
                        &dyson_rot,
                        &vp_matrix,
                        &mesh.material,
                        &mut target,
                        stats,
                    );
                }
            } else {
                stats.objetos_descartados_frustum += 1;
            }
        }

        // 2. Sol (Shader animado de plasma con Frustum Culling y descarte Early-Z tras la Esfera de Dyson)
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
            let ring_extent = planet
                .debris_rings
                .iter()
                .map(DebrisRing::culling_radius)
                .fold(0.0f32, f32::max);
            let moon_extent = planet
                .moons
                .iter()
                .map(Moon::culling_radius)
                .fold(0.0f32, f32::max);
            let planet_cull_radius = (planet.radius + max_surface_extent)
                .max(ring_extent)
                .max(moon_extent)
                .max(planet.composite_radius())
                .max(planet.thruster_flame_radius());
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
                for ring in &planet.debris_rings {
                    stats.objetos_descartados_frustum += ring.instances.len();
                    stats.triangulos_descartados_frustum += ring.triangle_count();
                }
                for component in &planet.components {
                    stats.objetos_descartados_frustum += 1;
                    stats.triangulos_descartados_frustum += component.mesh.triangle_count();
                }
                stats.objetos_descartados_frustum += planet.thruster_flames.len();
                stats.triangulos_descartados_frustum +=
                    self.thruster_flame_mesh.triangle_count() * planet.thruster_flames.len();
                for moon in &planet.moons {
                    stats.objetos_descartados_frustum += 1;
                    stats.triangulos_descartados_frustum += moon.mesh.triangle_count();
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
            } else if planet.shader_type == "earth_like" || planet.shader_type == "earth" {
                self.earth_shader.body_center = planet.position;
                self.earth_shader
                    .set_orientation(planet.rotation_rad, planet.axial_tilt_rad);
                Self::draw_mesh(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.earth_shader,
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

            // 2.1 Componentes rígidos con origen compartido. Usan la misma matriz
            // padre para conservar exactamente el ensamblaje creado en Blender.
            for component in &planet.components {
                let component_center = p_model.transform_point(component.mesh.aabb.center());
                let component_radius = component.mesh.geometric_radius() * planet.scale;
                if !frustum.intersects_sphere(component_center, component_radius) {
                    stats.objetos_descartados_frustum += 1;
                    stats.triangulos_descartados_frustum += component.mesh.triangle_count();
                    continue;
                }

                if component.mesh.material.normal_texture.is_some() {
                    Self::draw_mesh(
                        &mut self.renderer,
                        self.shader_mode,
                        &self.depth_shader,
                        &self.normals_shader,
                        &self.toon_shader,
                        &self.normal_mapped_shader,
                        &component.mesh,
                        &p_model,
                        &p_rot,
                        &vp_matrix,
                        &component.mesh.material,
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
                        &component.mesh,
                        &p_model,
                        &p_rot,
                        &vp_matrix,
                        &component.mesh.material,
                        &mut target,
                        stats,
                    );
                }
            }

            // 2.2 Llamas rígidas: heredan la misma órbita, inclinación y rotación del planeta.
            for flame in &planet.thruster_flames {
                Self::draw_thruster_flame(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.thruster_flame_mesh,
                    &self.thruster_flame_material,
                    flame,
                    &p_model,
                    &p_rot,
                    &vp_matrix,
                    &mut target,
                    stats,
                    self.sim_time,
                    flame.active_intensity,
                );
            }

            // 2.3 Lunas: acompañan la traslación del planeta con órbita y giro independientes.
            for moon in &planet.moons {
                let moon_position = moon.world_position(planet.position, self.sim_time);
                if !frustum.intersects_sphere(moon_position, moon.radius) {
                    stats.objetos_descartados_frustum += 1;
                    stats.triangulos_descartados_frustum += moon.mesh.triangle_count();
                    continue;
                }
                let (moon_model, moon_rotation) = moon.matrices(planet.position, self.sim_time);
                Self::draw_mesh(
                    &mut self.renderer,
                    self.shader_mode,
                    &self.depth_shader,
                    &self.normals_shader,
                    &self.toon_shader,
                    &self.planet_shader,
                    &moon.mesh,
                    &moon_model,
                    &moon_rotation,
                    &vp_matrix,
                    &moon.material,
                    &mut target,
                    stats,
                );
            }

            // 2.4 Objetos de superficie del planeta (con descarte por distancia, horizonte y frustum individual)
            for group in &planet.surface_groups {
                let dist_to_planet_center = (active_camera.eye - planet.position).length();
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
                    let to_cam_from_center = active_camera.eye - planet.position;
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

            // 2.5 Anillos de escombros: comparten la traslación orbital del planeta,
            // pero su revolución y el giro de cada pieza dependen de periodos propios.
            for ring in &planet.debris_rings {
                for instance in &ring.instances {
                    let mesh = &ring.meshes[instance.mesh_index];
                    let (model, rotation) =
                        ring.instance_matrices(planet.position, instance, self.sim_time);
                    let mesh_center = model.transform_point(mesh.aabb.center());
                    let mesh_radius = mesh.geometric_radius() * instance.scale;
                    if !frustum.intersects_sphere(mesh_center, mesh_radius) {
                        stats.objetos_descartados_frustum += 1;
                        stats.triangulos_descartados_frustum += mesh.triangle_count();
                        continue;
                    }

                    Self::draw_mesh(
                        &mut self.renderer,
                        self.shader_mode,
                        &self.depth_shader,
                        &self.normals_shader,
                        &self.toon_shader,
                        &self.normal_mapped_shader,
                        mesh,
                        &model,
                        &rotation,
                        &vp_matrix,
                        &mesh.material,
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

        // 4. Nave. Durante la espera de reaparición o en modo cabina (1ra persona) no se dibuja
        // la malla exterior de la nave; si está viva y en 3ra persona, conserva el descarte por frustum.
        if self.combat.player_is_alive() && !self.first_person {
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

                for flame in &self.ship_thruster_flames {
                    let intensity = if self.ship_thrust_state > 1.5 {
                        flame.active_intensity * self.config.ship.boost_flame_multiplier
                    } else if self.ship_thrust_state > 0.5 {
                        flame.active_intensity
                    } else if self.ship_thrust_state < -0.5 {
                        flame.idle_intensity * 0.45
                    } else {
                        flame.idle_intensity
                    };
                    Self::draw_thruster_flame(
                        &mut self.renderer,
                        self.shader_mode,
                        &self.depth_shader,
                        &self.normals_shader,
                        &self.toon_shader,
                        &self.thruster_flame_mesh,
                        &self.thruster_flame_material,
                        flame,
                        &ship_model,
                        &ship_rot,
                        &vp_matrix,
                        &mut target,
                        stats,
                        self.sim_time,
                        intensity,
                    );
                }
            }
        }

        // 5. Esferas semitransparentes de campo gravitatorio / atmósfera (Alpha Blending)
        self.force_field_shader.camera_pos = active_camera.eye;
        self.force_field_shader.sim_time = self.sim_time;
        let field_material = Material::default();

        if self.sun_body.visible_gravity
            && let Some(ref field_mesh) = self.sun_body.gravity_mesh
            && frustum.intersects_sphere(self.sun_body.position, self.sun_body.gravity_radius)
        {
            let field_model = self.sun_body.gravity_field_model_matrix();
            let field_rot = self.sun_body.rotation_matrix();
            let ctx = RenderContext::new(
                &field_model,
                &field_rot,
                &vp_matrix,
                &field_material,
                &self.force_field_shader,
            )
            .with_blend_mode(BlendMode::AlphaBlend);
            self.renderer
                .render_mesh(field_mesh, &ctx, &mut target, stats);
        }

        for planet in &self.planets {
            if planet.visible_gravity
                && let Some(ref field_mesh) = planet.gravity_mesh
                && frustum.intersects_sphere(planet.position, planet.gravity_radius)
            {
                let field_model = planet.gravity_field_model_matrix();
                let field_rot = planet.rotation_matrix();
                let ctx = RenderContext::new(
                    &field_model,
                    &field_rot,
                    &vp_matrix,
                    &field_material,
                    &self.force_field_shader,
                )
                .with_blend_mode(BlendMode::AlphaBlend);
                self.renderer
                    .render_mesh(field_mesh, &ctx, &mut target, stats);
            }
        }

        if self.ship_thrust_state > 1.5 && self.combat.player_is_alive() {
            draw_speed_streaks(&mut self.framebuffer, self.sim_time, 16);
        }

        // 6. Sistema de partículas (chispas incandescentes de colisión en CPU con Z-test)
        self.sparks
            .render(&mut self.framebuffer, &self.depth_buffer, &vp_matrix);

        // Las barras flotantes en el mundo se dibujan únicamente sobre las naves enemigas.
        Self::draw_enemy_health_bars(
            &mut self.framebuffer,
            &vp_matrix,
            &self.combat.enemies,
            self.config.enemies.target_length_wu,
        );

        let aim_pos = if self.combat.player_is_alive() {
            let forward = self.ship_state.forward();
            let projectile_velocity =
                self.ship_state.velocity + forward * self.config.enemies.player_projectile_speed;
            let aim_point = self.ship_state.position + forward + projectile_velocity;
            project_world_to_screen(
                &vp_matrix,
                aim_point,
                self.framebuffer.width,
                self.framebuffer.height,
            )
        } else {
            None
        };

        // Marcador visual de aproximación orbital sobre el centro del planeta visible
        if let Some(InspectedCelestial::Planet(idx)) = self.nearby_celestial
            && let Some(planet) = self.planets.get(idx)
            && !self.local_flight_mode
            && let Some((px, py)) = project_world_to_screen(
                &vp_matrix,
                planet.position,
                self.framebuffer.width,
                self.framebuffer.height,
            )
        {
            draw_orbital_lock_reticle(&mut self.framebuffer, px, py, false);
        }

        // Proyección analítica de indicadores planetarios dinámicos
        let mut planet_markers = Vec::with_capacity(self.planets.len() + 1);

        // Indicador del Sol
        let sun_dist = (self.ship_state.position - self.sun_body.position).length();
        let sun_radius = if let Some(ref d) = self.dyson_sphere {
            d.radius
        } else {
            self.sun_body.radius
        };
        let sun_dist_surface = (sun_dist - sun_radius).max(0.0);
        let sun_center_proj = project_world_to_screen(
            &vp_matrix,
            self.sun_body.position,
            self.framebuffer.width,
            self.framebuffer.height,
        );
        let sun_top_world = self.sun_body.position + active_camera.up * (sun_radius * 1.06);
        let sun_top_proj = project_world_to_screen(
            &vp_matrix,
            sun_top_world,
            self.framebuffer.width,
            self.framebuffer.height,
        );
        if let (Some((cx, cy)), Some((tx, ty))) = (sun_center_proj, sun_top_proj) {
            let dx = cx - tx;
            let dy = cy - ty;
            let proj_radius = (dx * dx + dy * dy).isqrt().max(4);
            let is_near = sun_dist_surface <= sun_radius * 6.5;
            let in_interaction = self.nearby_celestial == Some(InspectedCelestial::Sun);
            let is_panoramic = self.local_flight_mode && in_interaction;

            planet_markers.push(PlanetMarker {
                id: "sol",
                screen_x: tx,
                screen_y: ty,
                projected_radius: proj_radius,
                distance_wu: sun_dist_surface,
                is_near,
                in_interaction_range: in_interaction,
                is_panoramic_active: is_panoramic,
            });
        }

        for (idx, planet) in self.planets.iter().enumerate() {
            let dist = (self.ship_state.position - planet.position).length();
            let dist_surface = (dist - planet.radius).max(0.0);

            // Proyección del centro del planeta y de su cúspide superior (proporcional al radio del planeta)
            let center_proj = project_world_to_screen(
                &vp_matrix,
                planet.position,
                self.framebuffer.width,
                self.framebuffer.height,
            );
            // La cúspide superior relativa al vector "up" de la cámara asegura alineación vertical en pantalla
            let top_world = planet.position + active_camera.up * (planet.radius * 1.06);
            let top_proj = project_world_to_screen(
                &vp_matrix,
                top_world,
                self.framebuffer.width,
                self.framebuffer.height,
            );

            if let (Some((cx, cy)), Some((tx, ty))) = (center_proj, top_proj) {
                // Radio proyectado en píxeles: distancia euclidiana entre centro y cúspide
                let dx = cx - tx;
                let dy = cy - ty;
                let proj_radius = (dx * dx + dy * dy).isqrt().max(4);

                let is_near = dist_surface <= planet.radius * 6.5;
                let in_interaction = self.nearby_celestial == Some(InspectedCelestial::Planet(idx));
                let is_panoramic = self.local_flight_mode && in_interaction;

                planet_markers.push(PlanetMarker {
                    id: planet.id.as_str(),
                    screen_x: tx,
                    screen_y: ty,
                    projected_radius: proj_radius,
                    distance_wu: dist_surface,
                    is_near,
                    in_interaction_range: in_interaction,
                    is_panoramic_active: is_panoramic,
                });
            }
        }

        // Renderizado del HUD de juego modular (desacoplado y ejecutado en CPU)
        let (nearest_name, nearest_dist) = Self::calculate_nearest_body_distance_from(
            self.ship_state.position,
            &self.sun_body,
            &self.planets,
        );

        let inspection_data = if self.local_flight_mode {
            let insp_ctx = InspectionContext {
                nearby: self.nearby_celestial,
                planets: &self.planets,
                planet_configs: &self.config.planets,
                sun_body: &self.sun_body,
                sun_config: &self.config.sun,
                dyson_sphere: self.dyson_sphere.as_ref(),
                lore_config: &self.lore_config,
                story_box_page: self.story_box_page,
                story_box_open: self.story_box_open,
            };
            Self::build_inspection_data(&insp_ctx)
        } else {
            None
        };

        let hud_ctx = HudContext {
            player_health: self.combat.player_health,
            player_max_health: self.config.enemies.player_max_health,
            player_alive: self.combat.player_is_alive(),
            ship_speed_wu: self.ship_state.velocity.length(),
            ship_max_speed_wu: self.config.ship.max_speed,
            combat_state: self.combat.state,
            nearest_body_name: Some(nearest_name),
            nearest_body_distance_wu: nearest_dist,
            is_near_orbit: self.nearby_celestial.is_some(),
            local_flight_mode: self.local_flight_mode,
            gravity_enabled: self.gravity_enabled,
            shader_mode: self.shader_mode,
            show_controls: self.show_controls,
            show_stats: self.show_stats_panel,
            current_fps: self.stats_tracker.current_fps,
            frame_time_ms: self.stats_tracker.current_frame_time_ms,
            frame_stats: self.stats_tracker.last_frame_stats,
            first_person: self.first_person,
            aim_screen_pos: aim_pos,
            sim_time: self.sim_time,
            planet_markers: &planet_markers,
            inspection_data,
        };

        render_hud(&mut self.framebuffer, &hud_ctx);
    }

    fn draw_enemy_health_bars(
        framebuffer: &mut Framebuffer,
        view_projection: &Mat4,
        enemies: &[Enemy],
        enemy_height: f32,
    ) {
        let width = framebuffer.width;
        let height = framebuffer.height;

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
    fn draw_thruster_flame(
        renderer: &mut SoftwareRenderer,
        shader_mode: ShaderMode,
        depth_shader: &DepthDebugShader,
        normals_shader: &NormalsDebugShader,
        toon_shader: &ToonShader,
        mesh: &Mesh,
        material: &Material,
        flame: &ThrusterFlame,
        parent_model: &Mat4,
        parent_rotation: &Mat4,
        vp_matrix: &Mat4,
        target: &mut RenderTarget<'_>,
        stats: &mut FrameStats,
        sim_time: f32,
        intensity: f32,
    ) {
        let flicker = 0.90 + 0.10 * (sim_time * flame.animation_speed).sin();
        let model = *parent_model * flame.local_model_matrix(intensity, flicker);
        let rotation = *parent_rotation * flame.rotation;
        let shader = ThrusterFlameShader::new(sim_time, flame.animation_speed, intensity);
        Self::draw_mesh(
            renderer,
            shader_mode,
            depth_shader,
            normals_shader,
            toon_shader,
            &shader,
            mesh,
            &model,
            &rotation,
            vp_matrix,
            material,
            target,
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
                let ctx = RenderContext::new(
                    model_matrix,
                    rotation_matrix,
                    vp_matrix,
                    material,
                    base_shader,
                );
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::Toon => {
                let ctx = RenderContext::new(
                    model_matrix,
                    rotation_matrix,
                    vp_matrix,
                    material,
                    toon_shader,
                );
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::DepthDebug => {
                let ctx = RenderContext::new(
                    model_matrix,
                    rotation_matrix,
                    vp_matrix,
                    material,
                    depth_shader,
                );
                renderer.render_mesh(mesh, &ctx, target, stats);
            }
            ShaderMode::NormalsDebug => {
                let ctx = RenderContext::new(
                    model_matrix,
                    rotation_matrix,
                    vp_matrix,
                    material,
                    normals_shader,
                );
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

    /// Construye la información telemétrica y de lore para el HUD de inspección panorámica.
    pub fn build_inspection_data<'a>(
        ctx: &InspectionContext<'a>,
    ) -> Option<CelestialInspectionData<'a>> {
        match ctx.nearby {
            Some(InspectedCelestial::Planet(idx)) => {
                let planet = ctx.planets.get(idx)?;
                let p_cfg = ctx.planet_configs.get(idx);
                let lore_key = p_cfg
                    .and_then(|p| p.lore_id.as_deref())
                    .unwrap_or(planet.id.as_str());
                let lore_entry = ctx.lore_config.get_entry(lore_key);

                let title = lore_entry
                    .map(|e| e.nombre.as_str())
                    .unwrap_or(planet.id.as_str());

                let pages = lore_entry.map(|e| e.paginas.as_slice()).unwrap_or(&[]);

                let body_type = match planet.id.as_str() {
                    "tierra" => "Planeta Habitable",
                    "dona" => "Toroide Glaseado",
                    "kirby" => "Planeta Biomecanico",
                    "destruido" => "Planeta Fragmentado",
                    "rocoso" => "Planeta Rocoso Arido",
                    "asteroide" => "Asteroide Procedural",
                    "homero" => "Satelite Orbital",
                    _ => "Cuerpo Celeste",
                };

                let surface_summary = match planet.id.as_str() {
                    "tierra" => Some("48 (Robles, Pinos, Ciudad)".to_string()),
                    "dona" => Some("32 (Pasteles, Galletas, Helados)".to_string()),
                    "kirby" => Some("5 Modulos, 1 Propulsor".to_string()),
                    "destruido" => Some("Superficie fragmentada".to_string()),
                    "rocoso" => Some("6 Cadenas montanosas".to_string()),
                    "asteroide" => Some("Malla deformada procedural".to_string()),
                    _ => None,
                };

                let moons_summary = match planet.id.as_str() {
                    "dona" => Some("1 (Homero)".to_string()),
                    "kirby" => Some("1 (Estrella)".to_string()),
                    _ => None,
                };

                let rings_summary = if planet.id == "destruido" {
                    Some("20 Escombros orbitales".to_string())
                } else {
                    None
                };

                Some(CelestialInspectionData {
                    id: planet.id.as_str(),
                    title,
                    body_type,
                    radius_wu: planet.radius,
                    orbital_period_s: Some(planet.orbital_period),
                    rotation_period_s: planet.rotation_period,
                    gravity_strength: planet.gravity_strength,
                    gravity_radius: planet.gravity_radius,
                    surface_objects_summary: surface_summary,
                    moons_summary,
                    rings_summary,
                    shader_name: planet.shader_type.as_str(),
                    lore_pages: pages,
                    current_page: ctx.story_box_page,
                    is_story_open: ctx.story_box_open,
                })
            }
            Some(InspectedCelestial::Sun) => {
                let lore_key = ctx
                    .sun_config
                    .lore_id
                    .as_deref()
                    .unwrap_or(ctx.sun_body.id.as_str());
                let lore_entry = ctx.lore_config.get_entry(lore_key);

                let title = lore_entry
                    .map(|e| e.nombre.as_str())
                    .unwrap_or("Estrella Central (Sol)");

                let pages = lore_entry.map(|e| e.paginas.as_slice()).unwrap_or(&[]);

                let body_type = if ctx.dyson_sphere.is_some() {
                    "Enana Amarilla + Dyson"
                } else {
                    "Enana Amarilla (Plasma)"
                };

                let radius = if let Some(d) = ctx.dyson_sphere {
                    d.radius
                } else {
                    ctx.sun_body.radius
                };

                let surface_summary = if ctx.dyson_sphere.is_some() {
                    Some("Megaestructura Dyson activa".to_string())
                } else {
                    Some("Corona solar de plasma".to_string())
                };

                let moons_summary = Some(format!("{} Planetas en orbita", ctx.planets.len()));

                Some(CelestialInspectionData {
                    id: "sol",
                    title,
                    body_type,
                    radius_wu: radius,
                    orbital_period_s: None,
                    rotation_period_s: ctx.sun_body.rotation_period,
                    gravity_strength: ctx.sun_body.gravity_strength,
                    gravity_radius: ctx.sun_body.gravity_radius,
                    surface_objects_summary: surface_summary,
                    moons_summary,
                    rings_summary: None,
                    shader_name: "PLASMA SOLAR (CPU)",
                    lore_pages: pages,
                    current_page: ctx.story_box_page,
                    is_story_open: ctx.story_box_open,
                })
            }
            None => None,
        }
    }

    /// Construye la información telemétrica y de lore para el HUD de inspección panorámica actual.
    pub fn build_current_inspection_data(&self) -> Option<CelestialInspectionData<'_>> {
        let ctx = InspectionContext {
            nearby: self.nearby_celestial,
            planets: &self.planets,
            planet_configs: &self.config.planets,
            sun_body: &self.sun_body,
            sun_config: &self.config.sun,
            dyson_sphere: self.dyson_sphere.as_ref(),
            lore_config: &self.lore_config,
            story_box_page: self.story_box_page,
            story_box_open: self.story_box_open,
        };
        Self::build_inspection_data(&ctx)
    }

    /// Retorna el número total de páginas formateadas de lore para el astro actual.
    pub fn get_current_lore_total_pages(&self) -> usize {
        let lore_pages = match self.nearby_celestial {
            Some(InspectedCelestial::Planet(idx)) => {
                if let Some(planet) = self.planets.get(idx) {
                    let lore_key = self
                        .config
                        .planets
                        .get(idx)
                        .and_then(|p| p.lore_id.as_deref())
                        .unwrap_or(planet.id.as_str());
                    self.lore_config
                        .get_entry(lore_key)
                        .map(|e| e.paginas.as_slice())
                        .unwrap_or(&[])
                } else {
                    &[]
                }
            }
            Some(InspectedCelestial::Sun) => {
                let lore_key = self
                    .config
                    .sun
                    .lore_id
                    .as_deref()
                    .unwrap_or(self.sun_body.id.as_str());
                self.lore_config
                    .get_entry(lore_key)
                    .map(|e| e.paginas.as_slice())
                    .unwrap_or(&[])
            }
            None => &[],
        };

        let max_text_width = 980.min(Self::WINDOW_WIDTH as i32 - 56) - 40;
        let paginated = crate::hud::paginate_lore(lore_pages, max_text_width, 4, 1);
        paginated.len().max(1)
    }
}
