use space_travel::config::AppConfig;
use space_travel::generate_thruster_flame;
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::pipeline::{FragmentInput, FragmentShader, ThrusterFlameShader};
use space_travel::scene::ThrusterFlame;
use std::path::Path;

#[test]
fn procedural_flame_has_expected_budget_and_finite_geometry() {
    let mesh = generate_thruster_flame(12);

    assert_eq!(mesh.vertex_count(), 14);
    assert_eq!(mesh.triangle_count(), 24);
    assert!((mesh.aabb.min.z + 1.0).abs() < 1e-6);
    assert!(mesh.aabb.max.z.abs() < 1e-6);
    assert!(mesh.vertices.iter().all(|vertex| {
        vertex.position.x.is_finite()
            && vertex.position.y.is_finite()
            && vertex.position.z.is_finite()
            && vertex.normal.length().is_finite()
    }));
}

#[test]
fn flame_shader_is_deterministic_and_animated() {
    let material = Material::default_diffuse();
    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 0.0, 1.0),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::new(0.31, 0.52),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &material,
    };
    let shader_t0 = ThrusterFlameShader::new(0.0, 8.0, 1.0);
    let shader_t1 = ThrusterFlameShader::new(0.7, 8.0, 1.0);

    assert_eq!(shader_t0.shade(&input), shader_t0.shade(&input));
    assert_ne!(shader_t0.shade(&input), shader_t1.shade(&input));
}

#[test]
fn configured_flames_point_away_from_their_thrusters() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap();
    assert_eq!(config.ship.thruster_flames.len(), 2);
    let kirby = config
        .planets
        .iter()
        .find(|planet| planet.id == "kirby")
        .unwrap();
    assert_eq!(kirby.thruster_flames.len(), 1);

    let ship_flame = &config.ship.thruster_flames[0];
    let ship = ThrusterFlame::from_degrees(
        ship_flame.position,
        ship_flame.rotation_deg,
        ship_flame.radius,
        ship_flame.length,
        ship_flame.animation_speed,
        ship_flame.idle_intensity,
        ship_flame.active_intensity,
    );
    let ship_direction = ship.rotation.transform_direction(Vec3::new(0.0, 0.0, -1.0));
    assert!(ship_direction.z < -0.999);

    let kirby_flame = &kirby.thruster_flames[0];
    let kirby = ThrusterFlame::from_degrees(
        kirby_flame.position,
        kirby_flame.rotation_deg,
        kirby_flame.radius,
        kirby_flame.length,
        kirby_flame.animation_speed,
        kirby_flame.idle_intensity,
        kirby_flame.active_intensity,
    );
    let kirby_direction = kirby
        .rotation
        .transform_direction(Vec3::new(0.0, 0.0, -1.0));
    assert!(kirby_direction.y < -0.999);
}

#[test]
fn boost_multiplier_extends_ship_flame() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap();
    let flame_config = &config.ship.thruster_flames[0];
    let flame = ThrusterFlame::from_degrees(
        flame_config.position,
        flame_config.rotation_deg,
        flame_config.radius,
        flame_config.length,
        flame_config.animation_speed,
        flame_config.idle_intensity,
        flame_config.active_intensity,
    );
    let tip = Vec3::new(0.0, 0.0, -1.0);
    let normal_tip = flame.local_model_matrix(1.0, 1.0).transform_point(tip);
    let boost_tip = flame
        .local_model_matrix(config.ship.boost_flame_multiplier, 1.0)
        .transform_point(tip);
    assert!((boost_tip - flame.position).length() > (normal_tip - flame.position).length());
}
