use space_travel::CelestialBody;
use space_travel::config::ShipConfig;
use space_travel::material::Material;
use space_travel::math::Vec3;
use space_travel::mesh::Mesh;
use space_travel::physics::{ShipInput, ShipState};
use std::sync::Arc;

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
fn test_ship_acceleration_and_drag() {
    let cfg = dummy_ship_config();
    let mut ship = ShipState::new(&cfg);

    let input_fwd = ShipInput {
        throttle_forward: true,
        ..Default::default()
    };

    // Un paso de aceleración
    ship.update(&input_fwd, &cfg, 1.0);
    assert!(ship.velocity.length() > 0.0);

    // Resistencia espacial (drag) sin acelerar
    let vel_before = ship.velocity.length();
    ship.update(&ShipInput::default(), &cfg, 1.0);
    let vel_after = ship.velocity.length();
    assert!(vel_after < vel_before, "El drag debe reducir la velocidad");
}

#[test]
fn test_boost_increases_acceleration_and_speed_limit() {
    let cfg = dummy_ship_config();
    let mut normal = ShipState::new(&cfg);
    let mut boosted = ShipState::new(&cfg);
    let normal_input = ShipInput {
        throttle_forward: true,
        ..Default::default()
    };
    let boost_input = ShipInput {
        throttle_forward: true,
        boost: true,
        ..Default::default()
    };

    for _ in 0..300 {
        normal.update(&normal_input, &cfg, 1.0 / 60.0);
        boosted.update(&boost_input, &cfg, 1.0 / 60.0);
    }

    assert!(boosted.velocity.length() > normal.velocity.length());
    assert!(boosted.velocity.length() <= cfg.boost_max_speed + 1e-4);
}

#[test]
fn test_boost_release_returns_gradually_to_normal_limit() {
    let cfg = dummy_ship_config();
    let mut ship = ShipState::new(&cfg);
    ship.velocity = ship.forward() * cfg.boost_max_speed;
    ship.update(&ShipInput::default(), &cfg, 1.0 / 60.0);
    let first_release_speed = ship.velocity.length();
    assert!(first_release_speed < cfg.boost_max_speed);
    assert!(first_release_speed > cfg.max_speed);

    for _ in 0..180 {
        ship.update(&ShipInput::default(), &cfg, 1.0 / 60.0);
    }
    assert!(ship.velocity.length() <= cfg.max_speed + 1e-3);
}

#[test]
fn test_solar_damage_curve_is_monotonic_and_bounded() {
    let surface = 31.0;
    let danger = 65.0;
    let max_damage = 30.0;
    let outside = space_travel::solar_damage_per_second(65.0, surface, danger, max_damage);
    let middle = space_travel::solar_damage_per_second(48.0, surface, danger, max_damage);
    let surface_damage = space_travel::solar_damage_per_second(31.0, surface, danger, max_damage);

    assert_eq!(outside, 0.0);
    assert!(middle > 0.0 && middle < max_damage);
    assert!((surface_damage - max_damage).abs() < 1e-4);
}

#[test]
fn test_inelastic_collision_with_tangential_sliding() {
    let cfg = dummy_ship_config();
    let mut ship = ShipState::new(&cfg);

    // Cuerpo esférico en (0, 0, 0) con radio 10 y margen 1 (radio efectivo = 10 + 0.6 + 1.0 = 11.6)
    let body_center = Vec3::zero();
    let body_radius = 10.0;
    let margin = 1.0;
    let effective_radius = 11.6;

    // 1. Nave fuera del radio efectivo: no hay colisión
    ship.position = Vec3::new(0.0, 0.0, 20.0);
    assert!(!ship.resolve_collision(body_center, body_radius, margin));
    assert_eq!(ship.position.z, 20.0);

    // 2. Nave penetrando con velocidad frontal hacia el cuerpo
    ship.position = Vec3::new(0.0, 0.0, 5.0); // Dentro de la esfera
    // Velocidad con componente entrante hacia -Z y tangencial en +X
    ship.velocity = Vec3::new(8.0, 0.0, -10.0);

    let collided = ship.resolve_collision(body_center, body_radius, margin);
    assert!(collided);

    // La nave debe reposicionarse en la superficie del radio efectivo
    assert!((ship.position.length() - effective_radius).abs() < 1e-4);

    // La velocidad hacia el cuerpo debe eliminarse, y la tangencial en X debe preservarse
    assert!(
        (ship.velocity.x - 8.0).abs() < 1e-4,
        "Velocidad tangencial debe conservarse"
    );
    assert!(
        ship.velocity.z >= -1e-5,
        "Velocidad entrante hacia el cuerpo debe extinguirse"
    );

    // 3. Caso límite: centros exactamente coincidentes no produce NaN
    ship.position = Vec3::zero();
    let coincident_collided = ship.resolve_collision(body_center, body_radius, margin);
    assert!(coincident_collided);
    assert!(
        ship.position.x.is_finite() && ship.position.y.is_finite() && ship.position.z.is_finite()
    );
    assert!((ship.position.length() - effective_radius).abs() < 1e-4);
}

#[test]
fn test_ship_orientation_and_forward_vector() {
    let mut cfg = dummy_ship_config();
    cfg.initial_rotation_deg = [0.0, 0.0, 0.0];
    let mut ship = ShipState::new(&cfg);

    // 1. Con rotación cero, el frente local de la nave es +Z (morro del asset)
    let fwd_zero = ship.forward();
    assert!(
        (fwd_zero - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-4,
        "Con rotación cero, forward() debe ser +Z local"
    );

    // 2. Con yaw de 180 grados, el frente queda orientado hacia -Z del mundo
    ship.yaw_deg = 180.0;
    let fwd_180 = ship.forward();
    assert!(
        (fwd_180 - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4,
        "Con yaw de 180 grados, forward() debe ser -Z del mundo"
    );

    // 3. La aceleración con 'W' produce velocidad con producto punto positivo respecto a forward
    let input_w = ShipInput {
        throttle_forward: true,
        ..Default::default()
    };
    ship.velocity = Vec3::zero();
    ship.update(&input_w, &cfg, 0.1);
    assert!(
        ship.velocity.dot(ship.forward()) > 0.0,
        "La aceleración con W debe tener producto punto positivo con forward()"
    );
}

#[test]
fn test_orbital_mechanics_deterministic() {
    let mut body = CelestialBody {
        id: "rocoso".to_string(),
        parent_id: None,
        mesh: Arc::new(Mesh::new(vec![], vec![])),
        material: Material::default_diffuse(),
        scale: 1.0,
        radius: 10.0,
        collision_margin: 1.0,
        orbital_radius: 70.0,
        orbital_period: 80.0,
        rotation_period: 20.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "textured_diffuse".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: Vec::new(),
        thruster_flames: Vec::new(),
        moons: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 14.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    // t = 0: (R, 0, 0)
    body.update(0.0);
    assert!((body.position.x - 70.0).abs() < 1e-4);
    assert!(body.position.z.abs() < 1e-4);
    assert!(body.rotation_rad.abs() < 1e-4);

    // t = T / 4: (0, 0, R)
    body.update(20.0);
    assert!(body.position.x.abs() < 1e-4);
    assert!((body.position.z - 70.0).abs() < 1e-4);
    assert!((body.position.length() - 70.0).abs() < 1e-4);
    assert!((body.rotation_rad - std::f32::consts::TAU).abs() < 1e-4);

    // t = T / 2: (-R, 0, 0)
    body.update(40.0);
    assert!((body.position.x - (-70.0)).abs() < 1e-4);
    assert!(body.position.z.abs() < 1e-4);
    assert!((body.position.length() - 70.0).abs() < 1e-4);
}

#[test]
fn test_celestial_body_model_and_rotation_matrices() {
    let mut body = CelestialBody {
        id: "rocoso".to_string(),
        parent_id: None,
        mesh: Arc::new(Mesh::new(vec![], vec![])),
        material: Material::default_diffuse(),
        scale: 2.0,
        radius: 10.0,
        collision_margin: 1.0,
        orbital_radius: 70.0,
        orbital_period: 80.0,
        rotation_period: 40.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "textured_diffuse".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: Vec::new(),
        thruster_flames: Vec::new(),
        moons: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 14.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    // Actualizar a t = 10s (rotación axial de 90°: TAU / 4)
    body.update(10.0);
    assert!((body.rotation_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-4);

    let model = body.model_matrix();
    let rot = body.rotation_matrix();

    // Rotación pura debe rotar una normal (0, 0, 1) en Y en 90° hacia (1, 0, 0)
    let n = Vec3::new(0.0, 0.0, 1.0);
    let rot_n = rot.transform_direction(n);
    assert!((rot_n.x - 1.0).abs() < 1e-4);
    assert!(rot_n.y.abs() < 1e-4);
    assert!(rot_n.z.abs() < 1e-4);

    // Matriz de modelo completa aplica T * R * S sobre un punto local (0, 0, 1):
    // 1. Escala por 2.0 -> (0, 0, 2)
    // 2. Rotación Y 90° -> (2, 0, 0)
    // 3. Traslación a body.position
    let pt = Vec3::new(0.0, 0.0, 1.0);
    let transformed_pt = model.transform_point(pt);
    assert!((transformed_pt.x - (body.position.x + 2.0)).abs() < 1e-4);
    assert!((transformed_pt.y - body.position.y).abs() < 1e-4);
    assert!((transformed_pt.z - body.position.z).abs() < 1e-4);
}

#[test]
fn test_resolve_sphere_collision_with_rebound() {
    use space_travel::physics::{CollisionSphere, resolve_sphere_collision_with_rebound};

    let obstacle = CollisionSphere::new(Vec3::zero(), 10.0);
    let mut ship_pos = Vec3::new(0.0, 0.0, 9.5); // penetración de 1.1 con radio 0.6
    let mut ship_vel = Vec3::new(2.0, 0.0, -10.0); // moviéndose hacia el centro en -Z
    let ship_radius = 0.6;
    let restitution = 0.5;

    let contact = resolve_sphere_collision_with_rebound(
        &mut ship_pos,
        &mut ship_vel,
        ship_radius,
        obstacle,
        restitution,
    );

    assert!(contact.is_some(), "Debe detectar contacto y colisión");
    let c = contact.unwrap();

    // Normal debe apuntar hacia afuera desde el centro del obstáculo (+Z)
    assert!((c.normal.z - 1.0).abs() < 1e-4);
    assert!(
        (c.impact_speed - 10.0).abs() < 1e-4,
        "Velocidad de impacto entrante debe ser 10.0"
    );

    // Posición corregida en el radio combinado 10.6
    assert!((ship_pos.z - 10.6).abs() < 1e-4);

    // Velocidad tangencial en X (2.0) debe preservarse
    assert!((ship_vel.x - 2.0).abs() < 1e-4);

    // Velocidad en Z con restitución e = 0.5: -10.0 - 1.0 * (-10.0 * 1.5) = +5.0
    assert!(
        (ship_vel.z - 5.0).abs() < 1e-4,
        "Velocidad tras rebote elástico parcial debe ser +5.0"
    );
}

#[test]
fn test_resolve_sphere_collision_rebound_zero_is_inelastic() {
    use space_travel::physics::{CollisionSphere, resolve_sphere_collision_with_rebound};

    let obstacle = CollisionSphere::new(Vec3::zero(), 10.0);
    let mut ship_pos = Vec3::new(0.0, 0.0, 9.0);
    let mut ship_vel = Vec3::new(3.0, 0.0, -8.0);
    let ship_radius = 0.6;

    let contact = resolve_sphere_collision_with_rebound(
        &mut ship_pos,
        &mut ship_vel,
        ship_radius,
        obstacle,
        0.0,
    );

    assert!(contact.is_some());
    // Con e = 0.0, la velocidad normal entrante se vuelve exactamente 0.0 (deslizamiento tangencial puro)
    assert!(ship_vel.z.abs() < 1e-5);
    assert!((ship_vel.x - 3.0).abs() < 1e-4);
}

#[test]
fn test_resolve_sphere_collision_with_rebound_separating_velocity() {
    use space_travel::physics::{CollisionSphere, resolve_sphere_collision_with_rebound};

    let obstacle = CollisionSphere::new(Vec3::zero(), 10.0);
    // Nave penetrando a distancia 9.5 con radio 0.6, pero su velocidad ya apunta hacia afuera (+Z)
    let mut ship_pos = Vec3::new(0.0, 0.0, 9.5);
    let mut ship_vel = Vec3::new(2.0, 0.0, 5.0); // inward_speed = +5.0 (separándose)
    let ship_radius = 0.6;

    let contact = resolve_sphere_collision_with_rebound(
        &mut ship_pos,
        &mut ship_vel,
        ship_radius,
        obstacle,
        0.70,
    );

    assert!(contact.is_some());
    let c = contact.unwrap();
    // La velocidad de impacto entrante es 0.0 porque ya se estaba separando
    assert_eq!(c.impact_speed, 0.0);
    // La posición es corregida a la superficie (10.6)
    assert!((ship_pos.z - 10.6).abs() < 1e-4);
    // La velocidad saliente no debe ser alterada
    assert!((ship_vel.z - 5.0).abs() < 1e-4);
    assert!((ship_vel.x - 2.0).abs() < 1e-4);
}

#[test]
fn test_moon_orbital_mechanics_relative_to_parent() {
    let mut parent = CelestialBody {
        id: "dona".to_string(),
        parent_id: None,
        mesh: Arc::new(Mesh::new(vec![], vec![])),
        material: Material::default_diffuse(),
        scale: 1.0,
        radius: 45.0,
        collision_margin: 1.0,
        orbital_radius: 450.0,
        orbital_period: 400.0,
        rotation_period: 40.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "diffuse".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: Vec::new(),
        thruster_flames: Vec::new(),
        moons: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 50.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    let mut moon = CelestialBody {
        id: "homero".to_string(),
        parent_id: Some("dona".to_string()),
        mesh: Arc::new(Mesh::new(vec![], vec![])),
        material: Material::default_diffuse(),
        scale: 1.0,
        radius: 8.0,
        collision_margin: 1.0,
        orbital_radius: 80.0,
        orbital_period: 40.0,
        rotation_period: 20.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "diffuse".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: Vec::new(),
        thruster_flames: Vec::new(),
        moons: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 12.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    // t = 0: Padre en (450, 0, 0), Luna a +80 en X del padre -> (530, 0, 0)
    parent.update(0.0);
    moon.update_with_parent_position(0.0, parent.position);
    assert!((parent.position.x - 450.0).abs() < 1e-4);
    assert!((moon.position.x - 530.0).abs() < 1e-4);
    assert!(moon.position.z.abs() < 1e-4);
    assert!(((moon.position - parent.position).length() - 80.0).abs() < 1e-4);

    // t = 10s: Luna ha recorrido T_luna / 4 (90° alrededor del padre)
    // El padre se ha movido según su propia órbita
    parent.update(10.0);
    moon.update_with_parent_position(10.0, parent.position);
    let relative_pos = moon.position - parent.position;
    assert!(
        (relative_pos.length() - 80.0).abs() < 1e-4,
        "La distancia orbital al padre debe conservarse exactamente"
    );
    assert!(relative_pos.x.abs() < 1e-4);
    assert!((relative_pos.z - 80.0).abs() < 1e-4);
}
