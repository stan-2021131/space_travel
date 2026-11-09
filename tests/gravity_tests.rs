use space_travel::config::{PlanetConfig, ShipConfig};
use space_travel::math::Vec3;
use space_travel::physics::ShipState;

fn dummy_ship_config() -> ShipConfig {
    ShipConfig {
        model_path: "assets/models/nave.obj".to_string(),
        target_length_wu: 1.0,
        initial_position: [0.0, 0.0, 50.0],
        initial_rotation_deg: [0.0, 0.0, 0.0],
        acceleration: 14.0,
        reverse_acceleration: 7.0,
        braking_force: 12.0,
        drag: 0.0, // Drag cero para medir aceleración puramente gravitatoria
        max_speed: 100.0,
        yaw_speed: 1.8,
        pitch_speed: 1.5,
        roll_speed: 2.0,
        collision_radius: 0.6,
    }
}

#[test]
fn test_gravity_zero_strength_identity() {
    let config = dummy_ship_config();
    let mut ship = ShipState::new(&config);
    ship.position = Vec3::new(0.0, 0.0, 20.0);
    ship.velocity = Vec3::zero();

    let accel = ship.apply_gravitational_attraction(
        Vec3::zero(),
        10.0,
        30.0,
        0.0, // Fuerza cero
        0.1,
    );

    assert!(accel.is_none());
    assert_eq!(ship.velocity, Vec3::zero());
}

#[test]
fn test_gravity_outside_radius_no_attraction() {
    let config = dummy_ship_config();
    let mut ship = ShipState::new(&config);
    // Posición en z=50, con radio gravitatorio=40
    ship.position = Vec3::new(0.0, 0.0, 50.0);
    ship.velocity = Vec3::zero();

    let accel = ship.apply_gravitational_attraction(Vec3::zero(), 15.0, 40.0, 20.0, 0.1);

    assert!(accel.is_none(), "Fuera del radio no debe haber atracción");
    assert_eq!(ship.velocity, Vec3::zero());
}

#[test]
fn test_gravity_inward_acceleration() {
    let config = dummy_ship_config();
    let mut ship = ShipState::new(&config);
    let body_center = Vec3::new(100.0, 0.0, 0.0);
    ship.position = Vec3::new(80.0, 0.0, 0.0); // A la izquierda del cuerpo
    ship.velocity = Vec3::zero();

    let accel = ship.apply_gravitational_attraction(body_center, 10.0, 30.0, 15.0, 0.1);

    assert!(accel.is_some());
    let a = accel.unwrap();
    assert!(
        a.x > 0.0,
        "La aceleración debe apuntar hacia +X (hacia el cuerpo)"
    );
    assert_eq!(a.y, 0.0);
    assert_eq!(a.z, 0.0);
    assert!(
        ship.velocity.x > 0.0,
        "La velocidad debe incrementarse hacia el centro"
    );
}

#[test]
fn test_gravity_smooth_falloff_continuity() {
    let config = dummy_ship_config();
    let body_center = Vec3::zero();
    let body_radius = 10.0;
    let gravity_radius = 50.0;
    let surface_gravity = 20.0;

    // Medición a r = 49.99 (justo dentro) vs r = 50.01 (justo fuera)
    let mut ship_inside = ShipState::new(&config);
    ship_inside.position = Vec3::new(0.0, 0.0, 49.99);

    let mut ship_outside = ShipState::new(&config);
    ship_outside.position = Vec3::new(0.0, 0.0, 50.01);

    let accel_inside = ship_inside.apply_gravitational_attraction(
        body_center,
        body_radius,
        gravity_radius,
        surface_gravity,
        0.016,
    );

    let accel_outside = ship_outside.apply_gravitational_attraction(
        body_center,
        body_radius,
        gravity_radius,
        surface_gravity,
        0.016,
    );

    assert!(accel_outside.is_none());
    assert!(accel_inside.is_some());

    // Debido al smoothstep, la aceleración cerca del borde debe ser suavemente infinitesimal (< 0.01)
    let a_mag = accel_inside.unwrap().length();
    assert!(
        a_mag < 0.01,
        "La aceleración cerca del límite debe acercarse suavemente a cero (obtenido: {})",
        a_mag
    );
}

#[test]
fn test_gravity_no_singularity_at_surface() {
    let config = dummy_ship_config();
    let mut ship = ShipState::new(&config);
    let body_radius = 15.0;
    // Posición justo en la superficie física
    ship.position = Vec3::new(body_radius, 0.0, 0.0);
    ship.velocity = Vec3::zero();

    let accel = ship.apply_gravitational_attraction(Vec3::zero(), body_radius, 50.0, 25.0, 0.1);

    assert!(accel.is_some());
    let a = accel.unwrap();
    assert!(a.length().is_finite(), "La aceleración debe ser finita");
    // En la superficie con distancia clamped a body_radius, ratio es 1.0 y t=1.0 (smoothstep=1.0)
    // por lo que a = surface_gravity = 25.0
    let diff = (a.length() - 25.0).abs();
    assert!(
        diff < 1e-3,
        "En la superficie nominal debe coincidir con surface_gravity"
    );
}

#[test]
fn test_planet_effective_gravity_radius_default_and_explicit() {
    let p_default = PlanetConfig {
        id: "p_def".to_string(),
        mesh_type: "model".to_string(),
        model_path: "dummy.obj".to_string(),
        subdivisions: 0,
        deformation_amplitude: 0.0,
        deformation_frequency: 0.5,
        radius: 12.0,
        orbital_radius: 50.0,
        orbital_period: 100.0,
        rotation_period: 20.0,
        initial_orbital_angle_deg: 0.0,
        collision_margin: 1.0,
        shader: "normal_mapped".to_string(),
        gravity_strength: 10.0,
        gravity_radius: None, // No especificado
        visible_gravity: false,
        axial_tilt_deg: 0.0,
        surface_objects: Vec::new(),
        debris_rings: Vec::new(),
    };
    assert_eq!(
        p_default.effective_gravity_radius(),
        16.0,
        "Por defecto debe ser radius + 4.0"
    );

    let p_explicit = PlanetConfig {
        gravity_radius: Some(35.0),
        ..p_default
    };
    assert_eq!(
        p_explicit.effective_gravity_radius(),
        35.0,
        "Si es explícito debe respetar el valor especificado"
    );
}
