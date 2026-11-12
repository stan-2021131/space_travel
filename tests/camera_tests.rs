use space_travel::camera::Camera;
use space_travel::config::{CameraConfig, ShipConfig};
use space_travel::math::Vec3;
use space_travel::physics::ShipState;

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

#[test]
fn test_chase_camera_tracking_and_dot_products() {
    let cfg = dummy_ship_config(); // initial_rotation_deg = [0.0, 180.0, 0.0]
    let ship = ShipState::new(&cfg);
    let cam_cfg = CameraConfig {
        distance_behind: 4.0,
        height_above: 1.5,
        look_ahead: 5.0,
        position_smoothing: 1000.0, // Muy alto para evaluar la posición ideal directa
        rotation_smoothing: 1000.0,
        fov_deg: 50.0,
        near_plane: 0.1,
        far_plane: 100.0,
    };

    let mut camera = Camera::new(
        Vec3::zero(),
        Vec3::zero(),
        Vec3::up(),
        50.0,
        1.0,
        0.1,
        100.0,
    );

    camera.update_chase(ship.position, ship.forward(), ship.up(), &cam_cfg, 1.0);

    // El vector desde la nave hacia la cámara tiene producto punto negativo con forward
    let to_camera = camera.eye - ship.position;
    let dot_camera = to_camera.dot(ship.forward());
    assert!(
        dot_camera < 0.0,
        "El vector hacia la cámara debe tener producto punto negativo con forward"
    );

    // El vector desde la nave hacia el objetivo de cámara tiene producto punto positivo con forward
    let to_target = camera.target - ship.position;
    let dot_target = to_target.dot(ship.forward());
    assert!(
        dot_target > 0.0,
        "El vector hacia el target debe tener producto punto positivo con forward"
    );

    // La cámara debe estar por encima de la nave para proyectarla ligeramente debajo del centro
    assert!(
        to_camera.y > 0.0,
        "La cámara debe estar por encima de la nave"
    );
}
