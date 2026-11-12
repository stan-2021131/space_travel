use space_travel::app::App;
use space_travel::config::AppConfig;
use space_travel::pipeline::ShaderMode;
use space_travel::stats::{DiagnosticControls, StatsDisplayMode};
use std::path::Path;

#[test]
fn test_valid_configuration_loading() {
    let config_path = Path::new("config/solar_system.toml");
    assert!(
        config_path.exists(),
        "config/solar_system.toml debe existir"
    );

    let config = AppConfig::load_from_file(config_path)
        .expect("La configuración solar_system.toml debe ser válida");

    assert!(config.simulation.time_scale > 0.0);
    assert!(config.simulation.fixed_timestep > 0.0);
    assert!(config.ship.max_speed > 0.0);
    assert_eq!(config.ship.thruster_flames.len(), 2);
    assert_eq!(config.ship.boost_max_speed, 48.0);
    assert_eq!(config.sun.damage_radius, 65.0);
    assert!(config.enemies.initial_count > 0);
    assert!(config.enemies.player_max_health > 0.0);
    assert_eq!(config.enemies.peace_after_no_fire_seconds, 20.0);
    assert!(config.enemies.player_respawn_delay > 0.0);
    assert_eq!(config.enemies.approaching_speed_ratio, 1.00);
    assert_eq!(config.enemies.supervising_speed_ratio, 0.30);
    assert_eq!(config.enemies.attacking_speed_ratio, 0.80);
    assert!(config.enemies.supervising_distance > config.enemies.preferred_distance);
    assert!(config.sun.radius > 0.0);
    assert!(!config.planets.is_empty());
    let rocoso = config
        .planets
        .iter()
        .find(|planet| planet.id == "Forja Basaltica")
        .expect("Debe existir el planeta rocoso");
    assert_eq!(rocoso.shader, "normal_mapped");

    let tierra = config
        .planets
        .iter()
        .find(|planet| planet.id == "Eden Replicado")
        .expect("Debe existir el planeta tierra");
    assert_eq!(tierra.shader, "earth_like");
    assert_eq!(
        tierra.surface_objects.len(),
        5,
        "Debe tener los 5 tipos de objetos de árboles y ciudad"
    );
    for obj in &tierra.surface_objects {
        assert!(
            obj.max_distance <= 40.0,
            "Los objetos de corteza de la Tierra deben tener distancia de descarte <= 40.0 para rendimiento en CPU"
        );
        assert!(
            obj.fade_range <= 15.0,
            "El rango de transición debe ser suave y proporcionado"
        );
    }
    let destroyed = config
        .planets
        .iter()
        .find(|planet| planet.id == "Cenotafio Cero")
        .expect("Debe existir el planeta destruido");
    assert_eq!(destroyed.debris_rings.len(), 1);
    assert_eq!(destroyed.debris_rings[0].count, 20);
    assert_eq!(destroyed.debris_rings[0].model_paths.len(), 2);
    let kirby = config
        .planets
        .iter()
        .find(|planet| planet.id == "Parque kirby")
        .expect("Debe existir el planeta Kirby");
    assert_eq!(kirby.components.len(), 7);
    let coaster_train = kirby
        .components
        .iter()
        .find(|component| component.name == "Tren de montaña rusa")
        .expect("Kirby debe incluir el tren animado");
    assert_eq!(coaster_train.local_rotation_period, 14.0);
    assert_eq!(kirby.thruster_flames.len(), 1);
    assert_eq!(kirby.moons.len(), 1);
    assert_eq!(kirby.gravity_strength, 8.0);
    assert_eq!(kirby.radius + kirby.collision_margin, 13.5);
    assert!(config.music.enabled);
    assert_eq!(config.music.general_path, "assets/audio/music/General.ogg");
    assert_eq!(config.music.crossfade_seconds, 2.5);
    assert_eq!(config.music.zones.len(), 6);
    let music_body_ids: Vec<&str> = config
        .music
        .zones
        .iter()
        .map(|zone| zone.body_id.as_str())
        .collect();
    assert_eq!(
        music_body_ids,
        [
            "Helios Tipo-II",
            "Eden Replicado",
            "Forja Basaltica",
            "Cenotafio Cero",
            "Parque kirby",
            "Anillo Glaseado"
        ]
    );

    let homero = config
        .planets
        .iter()
        .find(|planet| planet.id == "Homero")
        .expect("Debe existir el cuerpo Homero como luna");
    assert_eq!(homero.parent_id.as_deref(), Some("Anillo Glaseado"));
    assert!(
        homero.radius < 45.0,
        "Homero debe ser más pequeño que el planeta dona"
    );
    assert!(
        homero.orbital_radius > 45.0,
        "La órbita de Homero debe ser exterior a la dona"
    );
}

#[test]
fn test_invalid_configuration_rejections() {
    let valid = AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap();

    // Radio solar <= 0
    let mut bad_sun = valid.clone();
    bad_sun.sun.radius = -5.0;
    assert!(bad_sun.validate(Path::new("test.toml")).is_err());

    // Órbita que intersecta el sol
    let mut bad_orbit = valid.clone();
    bad_orbit.planets[0].orbital_radius = 20.0; // Menor que radio sol (30)
    assert!(bad_orbit.validate(Path::new("test.toml")).is_err());

    // ID duplicado
    let mut dup_id = valid.clone();
    dup_id.planets[0].id = dup_id.sun.id.clone();
    assert!(dup_id.validate(Path::new("test.toml")).is_err());

    // Near plane mayor o igual a Far plane
    let mut bad_planes = valid.clone();
    bad_planes.camera.near_plane = 100.0;
    bad_planes.camera.far_plane = 10.0;
    assert!(bad_planes.validate(Path::new("test.toml")).is_err());

    // Velocidad máxima de nave <= 0
    let mut bad_ship = valid.clone();
    bad_ship.ship.max_speed = -5.0;
    assert!(bad_ship.validate(Path::new("test.toml")).is_err());

    let mut bad_ship_flame = valid.clone();
    bad_ship_flame.ship.thruster_flames[0].length = 0.0;
    assert!(bad_ship_flame.validate(Path::new("test.toml")).is_err());

    let mut bad_boost = valid.clone();
    bad_boost.ship.boost_max_speed = bad_boost.ship.max_speed;
    assert!(bad_boost.validate(Path::new("test.toml")).is_err());

    let mut bad_solar_damage = valid.clone();
    bad_solar_damage.sun.damage_radius = bad_solar_damage.sun.radius;
    assert!(bad_solar_damage.validate(Path::new("test.toml")).is_err());

    let mut bad_enemies = valid.clone();
    bad_enemies.enemies.initial_count = 0;
    assert!(bad_enemies.validate(Path::new("test.toml")).is_err());

    let mut bad_attack_speed = valid.clone();
    bad_attack_speed.enemies.attacking_speed_ratio = 1.1;
    assert!(bad_attack_speed.validate(Path::new("test.toml")).is_err());

    // Luna con órbita que intersecta a su planeta padre
    let mut bad_moon_orbit = valid.clone();
    let moon = bad_moon_orbit
        .planets
        .iter_mut()
        .find(|p| p.id == "Homero")
        .unwrap();
    moon.orbital_radius = 20.0; // Menor que el radio de la dona (45.0)
    assert!(bad_moon_orbit.validate(Path::new("test.toml")).is_err());

    // Luna con planeta padre inexistente
    let mut bad_moon_parent = valid.clone();
    let moon2 = bad_moon_parent
        .planets
        .iter_mut()
        .find(|p| p.id == "Homero")
        .unwrap();
    moon2.parent_id = Some("planeta_fantasma".to_string());
    assert!(bad_moon_parent.validate(Path::new("test.toml")).is_err());

    // Luna con auto-referencia (su propio padre)
    let mut bad_moon_self = valid.clone();
    let moon_self = bad_moon_self
        .planets
        .iter_mut()
        .find(|p| p.id == "Homero")
        .unwrap();
    moon_self.parent_id = Some("Homero".to_string());
    assert!(bad_moon_self.validate(Path::new("test.toml")).is_err());

    // Luna cuyo astro padre es a su vez una luna
    let mut bad_nested_moon = valid.clone();
    let mut sub_moon = valid.planets[0].clone();
    sub_moon.id = "sub_luna".to_string();
    sub_moon.parent_id = Some("Homero".to_string());
    sub_moon.orbital_radius = 50.0;
    bad_nested_moon.planets.push(sub_moon);
    assert!(bad_nested_moon.validate(Path::new("test.toml")).is_err());

    let mut bad_approach_speed = valid.clone();
    bad_approach_speed.enemies.approaching_speed_ratio =
        bad_approach_speed.enemies.attacking_speed_ratio;
    assert!(bad_approach_speed.validate(Path::new("test.toml")).is_err());

    let mut bad_supervision_speed = valid.clone();
    bad_supervision_speed.enemies.supervising_speed_ratio =
        bad_supervision_speed.enemies.attacking_speed_ratio;
    assert!(
        bad_supervision_speed
            .validate(Path::new("test.toml"))
            .is_err()
    );

    // Suavizado de cámara <= 0
    let mut bad_cam = valid.clone();
    bad_cam.camera.position_smoothing = 0.0;
    assert!(bad_cam.validate(Path::new("test.toml")).is_err());

    // Objeto de superficie con fade_range >= max_distance
    if !valid.planets.is_empty() && !valid.planets[0].surface_objects.is_empty() {
        let mut bad_surface = valid.clone();
        bad_surface.planets[0].surface_objects[0].max_distance = 50.0;
        bad_surface.planets[0].surface_objects[0].fade_range = 60.0;
        assert!(bad_surface.validate(Path::new("test.toml")).is_err());
    }

    // Subdivisions > 5
    let mut bad_subdiv = valid.clone();
    bad_subdiv.planets[0].subdivisions = 6;
    assert!(bad_subdiv.validate(Path::new("test.toml")).is_err());

    // Modelo vacío en planeta no-icosphere
    let mut bad_model = valid.clone();
    bad_model.planets[0].mesh_type = "model".to_string();
    bad_model.planets[0].model_path = "   ".to_string();
    assert!(bad_model.validate(Path::new("test.toml")).is_err());

    // Inclinación axial no finita (NaN)
    let mut bad_tilt = valid.clone();
    bad_tilt.planets[0].axial_tilt_deg = f32::NAN;
    assert!(bad_tilt.validate(Path::new("test.toml")).is_err());

    let destroyed_index = valid
        .planets
        .iter()
        .position(|planet| planet.id == "Cenotafio Cero")
        .unwrap();
    let mut bad_ring_radius = valid.clone();
    bad_ring_radius.planets[destroyed_index].debris_rings[0].inner_radius =
        bad_ring_radius.planets[destroyed_index].radius;
    assert!(bad_ring_radius.validate(Path::new("test.toml")).is_err());

    let mut bad_ring_models = valid.clone();
    bad_ring_models.planets[destroyed_index].debris_rings[0]
        .model_paths
        .clear();
    assert!(bad_ring_models.validate(Path::new("test.toml")).is_err());

    let kirby_index = valid
        .planets
        .iter()
        .position(|planet| planet.id == "Parque kirby")
        .unwrap();
    let mut bad_component_path = valid.clone();
    bad_component_path.planets[kirby_index].components[0].model_path = "  ".to_string();
    assert!(bad_component_path.validate(Path::new("test.toml")).is_err());

    let mut duplicate_component = valid.clone();
    let duplicate_name = duplicate_component.planets[kirby_index].components[0]
        .name
        .clone();
    duplicate_component.planets[kirby_index].components[1].name = duplicate_name;
    assert!(
        duplicate_component
            .validate(Path::new("test.toml"))
            .is_err()
    );

    let mut bad_component_animation = valid.clone();
    bad_component_animation.planets[kirby_index].components[6].local_rotation_period = -1.0;
    assert!(
        bad_component_animation
            .validate(Path::new("test.toml"))
            .is_err()
    );

    let mut bad_music_fade = valid.clone();
    bad_music_fade.music.crossfade_seconds = 0.0;
    assert!(bad_music_fade.validate(Path::new("test.toml")).is_err());

    let mut bad_kirby_flame = valid.clone();
    bad_kirby_flame.planets[kirby_index].thruster_flames[0].animation_speed = f32::NAN;
    assert!(bad_kirby_flame.validate(Path::new("test.toml")).is_err());

    let mut bad_moon_orbit = valid.clone();
    bad_moon_orbit.planets[kirby_index].moons[0].orbital_radius = 1.0;
    assert!(bad_moon_orbit.validate(Path::new("test.toml")).is_err());
}

#[test]
fn test_diagnostic_keys_and_shader_modes() {
    let mut controls = DiagnosticControls::new();
    assert_eq!(controls.shader_mode, ShaderMode::Normal);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Off);

    // La tecla 'D' no modifica el modo de shader ni de estadísticas
    controls.handle_key_action(false, false, true); // is_d = true
    assert_eq!(
        controls.shader_mode,
        ShaderMode::Normal,
        "La tecla 'D' no debe alterar el modo de shader"
    );
    assert_eq!(
        controls.stats_mode,
        StatsDisplayMode::Off,
        "La tecla 'D' no debe alterar el modo de estadísticas"
    );

    // La tecla 'F2' cambia cíclicamente el modo de shader (Normal -> Toon -> Depth -> Normals)
    controls.handle_key_action(true, false, false); // is_f2 = true
    assert_eq!(controls.shader_mode, ShaderMode::Toon);

    controls.handle_key_action(true, false, false);
    assert_eq!(controls.shader_mode, ShaderMode::DepthDebug);

    controls.handle_key_action(true, false, false);
    assert_eq!(controls.shader_mode, ShaderMode::NormalsDebug);

    controls.handle_key_action(true, false, false);
    assert_eq!(controls.shader_mode, ShaderMode::Normal);

    // La tecla 'F3' continúa afectando únicamente las estadísticas (Off -> Compact -> Detailed -> Off)
    controls.handle_key_action(false, true, false); // is_f3 = true
    assert_eq!(
        controls.stats_mode,
        StatsDisplayMode::Compact,
        "F3 debe cambiar estadísticas a Compact"
    );
    assert_eq!(
        controls.shader_mode,
        ShaderMode::Normal,
        "F3 no debe alterar el modo de shader"
    );

    controls.handle_key_action(false, true, false);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Detailed);
    assert_eq!(controls.shader_mode, ShaderMode::Normal);

    controls.handle_key_action(false, true, false);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Off);
    assert_eq!(controls.shader_mode, ShaderMode::Normal);

    // Validación directa sobre la entidad real App
    let mut app = App::new(Path::new("config/solar_system.toml")).unwrap();
    assert_eq!(app.shader_mode, ShaderMode::Normal);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Off);

    // La tecla 'D' en App no altera modo de sombreado ni estadísticas
    app.handle_diagnostic_key(false, false, true);
    assert_eq!(app.shader_mode, ShaderMode::Normal);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Off);

    // 'F2' cicla shader_mode en App
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::Toon);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::DepthDebug);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::NormalsDebug);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::Normal);

    // 'F3' cicla modo de estadísticas en App (Off -> Compact -> Detailed -> Off)
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Compact);
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Detailed);
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Off);
}
