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
    assert_eq!(config.planets[0].id, "rocoso");
    let destroyed = config
        .planets
        .iter()
        .find(|planet| planet.id == "destruido")
        .expect("Debe existir el planeta destruido");
    assert_eq!(destroyed.debris_rings.len(), 1);
    assert_eq!(destroyed.debris_rings[0].count, 20);
    assert_eq!(destroyed.debris_rings[0].model_paths.len(), 2);
    let kirby = config
        .planets
        .iter()
        .find(|planet| planet.id == "kirby")
        .expect("Debe existir el planeta Kirby");
    assert_eq!(kirby.components.len(), 5);
    assert_eq!(kirby.radius + kirby.collision_margin, 13.5);
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

    let mut bad_enemies = valid.clone();
    bad_enemies.enemies.initial_count = 0;
    assert!(bad_enemies.validate(Path::new("test.toml")).is_err());

    let mut bad_attack_speed = valid.clone();
    bad_attack_speed.enemies.attacking_speed_ratio = 1.1;
    assert!(bad_attack_speed.validate(Path::new("test.toml")).is_err());

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
        .position(|planet| planet.id == "destruido")
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
        .position(|planet| planet.id == "kirby")
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
}

#[test]
fn test_diagnostic_keys_and_shader_modes() {
    let mut controls = DiagnosticControls::new();
    assert_eq!(controls.shader_mode, ShaderMode::Normal);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Compact);

    // La tecla 'D' no modifica el modo de shader ni de estadísticas
    controls.handle_key_action(false, false, true); // is_d = true
    assert_eq!(
        controls.shader_mode,
        ShaderMode::Normal,
        "La tecla 'D' no debe alterar el modo de shader"
    );
    assert_eq!(
        controls.stats_mode,
        StatsDisplayMode::Compact,
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

    // La tecla 'F3' continúa afectando únicamente las estadísticas
    controls.handle_key_action(false, true, false); // is_f3 = true
    assert_eq!(
        controls.stats_mode,
        StatsDisplayMode::Detailed,
        "F3 debe cambiar estadísticas a Detailed"
    );
    assert_eq!(
        controls.shader_mode,
        ShaderMode::Normal,
        "F3 no debe alterar el modo de shader"
    );

    controls.handle_key_action(false, true, false);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Off);
    assert_eq!(controls.shader_mode, ShaderMode::Normal);

    controls.handle_key_action(false, true, false);
    assert_eq!(controls.stats_mode, StatsDisplayMode::Compact);
    assert_eq!(controls.shader_mode, ShaderMode::Normal);

    // Validación directa sobre la entidad real App
    let mut app = App::new(Path::new("config/solar_system.toml")).unwrap();
    assert_eq!(app.shader_mode, ShaderMode::Normal);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Compact);

    // La tecla 'D' en App no altera modo de sombreado ni estadísticas
    app.handle_diagnostic_key(false, false, true);
    assert_eq!(app.shader_mode, ShaderMode::Normal);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Compact);

    // 'F2' cicla shader_mode en App
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::Toon);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::DepthDebug);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::NormalsDebug);
    app.handle_diagnostic_key(true, false, false);
    assert_eq!(app.shader_mode, ShaderMode::Normal);

    // 'F3' cicla modo de estadísticas en App
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Detailed);
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Off);
    app.handle_diagnostic_key(false, true, false);
    assert_eq!(app.stats_tracker.display_mode, StatsDisplayMode::Compact);
}
