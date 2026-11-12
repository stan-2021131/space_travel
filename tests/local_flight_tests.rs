use space_travel::buffer::Framebuffer;
use space_travel::camera::Camera;
use space_travel::config::{CameraConfig, ShipConfig};
use space_travel::hud::{draw_orbital_lock_reticle, draw_orbital_mode_badge};
use space_travel::math::Vec3;
use space_travel::physics::{ShipInput, ShipState};

fn dummy_ship_config() -> ShipConfig {
    ShipConfig {
        model_path: "assets/models/nave.obj".to_string(),
        target_length_wu: 1.0,
        initial_position: [0.0, 0.0, 48.0],
        initial_rotation_deg: [0.0, 180.0, 0.0],
        acceleration: 10.0,
        reverse_acceleration: 5.0,
        braking_force: 8.0,
        drag: 0.5,
        max_speed: 20.0,
        yaw_speed: 1.5,
        pitch_speed: 1.5,
        roll_speed: 1.5,
        collision_radius: 0.6,
        boost_acceleration: 20.0,
        boost_max_speed: 35.0,
        boost_release_drag: 1.8,
        boost_flame_multiplier: 1.65,
        thruster_flames: Vec::new(),
    }
}

fn dummy_cam_config() -> CameraConfig {
    CameraConfig {
        distance_behind: 4.0,
        height_above: 1.5,
        look_ahead: 5.0,
        position_smoothing: 8.0,
        rotation_smoothing: 8.0,
        fov_deg: 50.0,
        near_plane: 0.1,
        far_plane: 1000.0,
    }
}

#[test]
fn test_camera_panoramic_planet_view_targets_planet_center() {
    let cam_config = dummy_cam_config();
    let mut camera = Camera::new(
        Vec3::new(0.0, 10.0, 60.0),
        Vec3::new(0.0, 0.0, 48.0),
        Vec3::up(),
        60.0,
        1.33,
        0.5,
        1000.0,
    );

    let planet_center = Vec3::new(120.0, 0.0, -80.0);
    let planet_radius = 20.0;

    let dt = 0.05;
    for _ in 0..40 {
        camera.update_panoramic_planet_view(planet_center, planet_radius, &cam_config, dt);
    }

    // El objetivo (target) de la cámara debe converger exactamente al centro del planeta
    let dist_to_planet_center = (camera.target - planet_center).length();
    assert!(
        dist_to_planet_center < 1.0,
        "La cámara panorámica debe centrarse en el planeta: dist = {}",
        dist_to_planet_center
    );
}

#[test]
fn test_camera_panoramic_planet_view_maintains_panoramic_distance() {
    let cam_config = dummy_cam_config();
    let mut camera = Camera::new(
        Vec3::new(0.0, 5.0, 10.0),
        Vec3::zero(),
        Vec3::up(),
        60.0,
        1.33,
        0.5,
        1000.0,
    );

    let planet_center = Vec3::zero();
    let planet_radius = 20.0;

    let dt = 0.05;
    for _ in 0..50 {
        camera.update_panoramic_planet_view(planet_center, planet_radius, &cam_config, dt);
    }

    // La distancia de la cámara al centro del planeta debe enmarcar la panorámica completa
    let cam_distance = (camera.eye - planet_center).length();
    let min_dist = planet_radius * 2.5;
    let max_dist = planet_radius * 4.5;
    assert!(
        cam_distance >= min_dist && cam_distance <= max_dist,
        "La cámara debe posicionarse a distancia panorámica del planeta ({} <= {} <= {})",
        min_dist,
        cam_distance,
        max_dist
    );
}

#[test]
fn test_ship_manual_controls_near_planet() {
    let config = dummy_ship_config();
    let mut ship = ShipState::new(&config);

    // Nave posicionada al costado del planeta
    ship.position = Vec3::new(35.0, 0.0, 0.0);
    ship.velocity = Vec3::zero();

    // Aceleración manual y giro de guiñada (yaw)
    let input = ShipInput {
        throttle_forward: true,
        yaw_left: true,
        ..Default::default()
    };

    let dt = 0.05;
    for _ in 0..10 {
        ship.update(&input, &config, dt);
    }

    // La nave responde ágilmente a los controles del usuario
    assert!(
        ship.velocity.length() > 0.5,
        "La nave debe acelerar con los controles manuales"
    );
    assert_ne!(
        ship.yaw_deg, config.initial_rotation_deg[1],
        "El yaw debe cambiar con el control manual del piloto"
    );
}

#[test]
fn test_hud_orbital_reticle_and_badge_drawing() {
    let mut fb = Framebuffer::new(320, 240);

    // Debe dibujar retícula orbital de aproximación sin desbordamiento ni pánicos
    draw_orbital_lock_reticle(&mut fb, 160, 120, false);
    draw_orbital_lock_reticle(&mut fb, 160, 120, true);

    // Debe dibujar insignia orbital de estado
    draw_orbital_mode_badge(&mut fb, 10, 10, false);
    draw_orbital_mode_badge(&mut fb, 10, 10, true);

    // Verificar que el framebuffer fue modificado en la zona
    assert_ne!(
        fb.get_pixel(160, 120),
        space_travel::buffer::Color::BLACK,
        "El centro de la retícula debe tener color asignado"
    );
    assert_ne!(
        fb.get_pixel(14, 14),
        space_travel::buffer::Color::BLACK,
        "La insignia orbital debe tener color asignado"
    );
}
