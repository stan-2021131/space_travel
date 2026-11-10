use space_travel::combat::{CombatSystem, EncounterState, Projectile, ProjectileOwner};
use space_travel::config::AppConfig;
use space_travel::math::Vec3;
use space_travel::mesh::Mesh;
use space_travel::physics::CollisionSphere;
use std::path::Path;

fn config() -> AppConfig {
    AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap()
}

#[test]
fn peaceful_mode_starts_without_enemies_and_with_full_health() {
    let cfg = config();
    let combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);

    assert_eq!(combat.state, EncounterState::Dormant);
    assert!(combat.enemies.is_empty());
    assert!(combat.projectiles.is_empty());
    assert_eq!(combat.player_health, cfg.enemies.player_max_health);
}

#[test]
fn activation_spawns_approachers_behind_the_sun() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 200.0);
    let sun = Vec3::zero();

    combat.activate(ship, sun, cfg.sun.radius, &cfg.enemies);

    assert_eq!(combat.state, EncounterState::Approaching);
    assert_eq!(combat.enemies.len(), cfg.enemies.initial_count);
    assert!(
        combat
            .enemies
            .iter()
            .all(|enemy| enemy.position.z < -cfg.sun.radius),
        "Los enemigos deben aparecer en el lado opuesto del Sol respecto a la nave"
    );
}

#[test]
fn supervisors_follow_but_never_fire() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 20.0);
    combat.activate(ship, Vec3::zero(), cfg.sun.radius, &cfg.enemies);

    // Colocarlos dentro del alcance de fuego no debe cambiar la regla diplomática.
    for enemy in &mut combat.enemies {
        enemy.position = ship + Vec3::new(0.0, 0.0, 10.0);
    }
    for step in 0..120 {
        combat.update(step as f32 * 0.1, 0.1, ship, &cfg.enemies);
    }

    assert_eq!(combat.state, EncounterState::Supervising);
    assert!(
        combat
            .projectiles
            .iter()
            .all(|projectile| projectile.owner != ProjectileOwner::Enemy)
    );
}

#[test]
fn destroying_first_enemy_starts_attack_mode() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::zero();
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);

    combat.enemies[0].health = cfg.enemies.player_projectile_damage;
    combat.enemies[0].position = Vec3::new(0.0, 0.0, 1.0);
    assert!(combat.try_player_fire(
        0.0,
        ship,
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::zero(),
        &cfg.enemies
    ));

    let update = combat.update(0.0, 0.0, ship, &cfg.enemies);
    assert_eq!(update.enemies_destroyed, 1);
    assert!(update.attack_started);
    assert_eq!(combat.state, EncounterState::Attacking);
}

#[test]
fn enemies_only_fire_after_attack_mode_begins() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::zero();
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);
    for enemy in &mut combat.enemies {
        enemy.position = Vec3::new(0.0, 0.0, 10.0);
    }

    combat.update(1.0, 0.0, ship, &cfg.enemies);
    assert!(
        combat
            .projectiles
            .iter()
            .all(|projectile| projectile.owner != ProjectileOwner::Enemy)
    );

    combat.state = EncounterState::Attacking;
    combat.update(2.0, 0.0, ship, &cfg.enemies);
    assert!(
        combat
            .projectiles
            .iter()
            .any(|projectile| projectile.owner == ProjectileOwner::Enemy)
    );
}

#[test]
fn enemy_projectile_reduces_player_health() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.projectiles.push(Projectile {
        owner: ProjectileOwner::Enemy,
        position: Vec3::zero(),
        velocity: Vec3::zero(),
        damage: 12.0,
        radius: cfg.enemies.projectile_radius,
        remaining_lifetime: 1.0,
    });

    let update = combat.update(0.0, 0.0, Vec3::zero(), &cfg.enemies);
    assert_eq!(update.player_damage, 12.0);
    assert_eq!(combat.player_health, cfg.enemies.player_max_health - 12.0);
}

#[test]
fn twenty_seconds_without_player_fire_starts_static_observation() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::zero();
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);
    combat.state = EncounterState::Attacking;
    assert!(combat.try_player_fire(
        0.0,
        ship,
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::zero(),
        &cfg.enemies
    ));
    combat.projectiles.clear();

    combat.update(19.9, 0.1, ship, &cfg.enemies);
    assert_eq!(combat.state, EncounterState::Attacking);
    let positions_before = combat
        .enemies
        .iter()
        .map(|enemy| enemy.position)
        .collect::<Vec<_>>();

    let update = combat.update(20.1, 0.5, ship, &cfg.enemies);
    assert!(update.peace_started);
    assert_eq!(combat.state, EncounterState::Observing);
    for (enemy, previous_position) in combat.enemies.iter().zip(positions_before) {
        assert_eq!(enemy.position, previous_position);
        assert_eq!(enemy.velocity, Vec3::zero());
    }
    assert!(
        combat
            .projectiles
            .iter()
            .all(|projectile| projectile.owner != ProjectileOwner::Enemy)
    );
}

#[test]
fn peaceful_observers_stay_still_and_keep_looking_at_ship() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.activate(
        Vec3::new(0.0, 0.0, 100.0),
        Vec3::zero(),
        cfg.sun.radius,
        &cfg.enemies,
    );
    combat.state = EncounterState::Observing;
    let original_position = combat.enemies[0].position;
    let moved_ship = Vec3::new(40.0, 15.0, 80.0);

    combat.update(30.0, 1.0, moved_ship, &cfg.enemies);

    let enemy = &combat.enemies[0];
    let expected_direction = (moved_ship - original_position).normalize();
    assert_eq!(enemy.position, original_position);
    assert_eq!(enemy.velocity, Vec3::zero());
    assert!((enemy.forward - expected_direction).length() < 1e-4);
}

#[test]
fn firing_again_reactivates_peaceful_observers() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.activate(
        Vec3::zero(),
        Vec3::new(0.0, 0.0, -100.0),
        10.0,
        &cfg.enemies,
    );
    combat.state = EncounterState::Observing;

    assert!(combat.try_player_fire(
        25.0,
        Vec3::zero(),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::zero(),
        &cfg.enemies
    ));
    assert_eq!(combat.state, EncounterState::Attacking);
}

#[test]
fn destroyed_player_respawns_instead_of_remaining_soft_locked() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.projectiles.push(Projectile {
        owner: ProjectileOwner::Enemy,
        position: Vec3::zero(),
        velocity: Vec3::zero(),
        damage: cfg.enemies.player_max_health,
        radius: cfg.enemies.projectile_radius,
        remaining_lifetime: 1.0,
    });

    let destroyed = combat.update(0.0, 0.0, Vec3::zero(), &cfg.enemies);
    assert!(destroyed.player_destroyed);
    assert_eq!(combat.player_health, 0.0);
    assert!(!combat.player_is_alive());

    let still_destroyed = combat.update(
        cfg.enemies.player_respawn_delay - 0.01,
        0.0,
        Vec3::zero(),
        &cfg.enemies,
    );
    assert!(!still_destroyed.player_respawned);
    assert!(!combat.player_is_alive());

    let respawned = combat.update(
        cfg.enemies.player_respawn_delay + 0.1,
        0.0,
        Vec3::zero(),
        &cfg.enemies,
    );
    assert!(respawned.player_respawned);
    assert_eq!(combat.player_health, cfg.enemies.player_max_health);
    assert!(combat.player_is_alive());
}

#[test]
fn supervising_enemies_are_slow_enough_for_the_player_to_escape() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 1_000.0);
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);
    combat.state = EncounterState::Supervising;
    combat.enemies.truncate(1);
    combat.enemies[0].position = Vec3::zero();

    for step in 0..200 {
        combat.update(step as f32 * 0.05, 0.05, ship, &cfg.enemies);
    }

    let expected_speed = cfg.ship.max_speed * cfg.enemies.supervising_speed_ratio;
    let actual_speed = combat.enemies[0].velocity.length();
    assert!((actual_speed - expected_speed).abs() < 0.01);
    assert!(actual_speed < cfg.ship.max_speed);
}

#[test]
fn approaching_enemies_use_full_player_max_speed() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 1_000.0);
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);
    combat.enemies.truncate(1);
    combat.enemies[0].position = Vec3::zero();

    for step in 0..200 {
        combat.update(step as f32 * 0.05, 0.05, ship, &cfg.enemies);
    }

    assert_eq!(combat.state, EncounterState::Approaching);
    let actual_speed = combat.enemies[0].velocity.length();
    assert!((actual_speed - cfg.ship.max_speed).abs() < 0.01);
}

#[test]
fn reaching_player_changes_to_supervising_without_reapproaching() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 100.0);
    combat.activate(ship, Vec3::zero(), cfg.sun.radius, &cfg.enemies);
    combat.enemies.truncate(1);
    combat.enemies[0].position = ship - Vec3::new(0.0, 0.0, cfg.enemies.supervising_distance);

    combat.update(0.0, 0.0, ship, &cfg.enemies);
    assert_eq!(combat.state, EncounterState::Supervising);

    combat.update(1.0, 0.1, ship + Vec3::new(0.0, 0.0, 500.0), &cfg.enemies);
    assert_eq!(combat.state, EncounterState::Supervising);
}

#[test]
fn attacking_enemies_reach_eighty_percent_of_player_max_speed() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let ship = Vec3::new(0.0, 0.0, 1_000.0);
    combat.activate(ship, Vec3::new(0.0, 0.0, -100.0), 10.0, &cfg.enemies);
    combat.state = EncounterState::Attacking;
    combat.enemies.truncate(1);
    combat.enemies[0].position = Vec3::zero();

    for step in 0..200 {
        combat.update(step as f32 * 0.05, 0.05, ship, &cfg.enemies);
    }

    let expected_speed = cfg.ship.max_speed * 0.80;
    let actual_speed = combat.enemies[0].velocity.length();
    assert!((actual_speed - expected_speed).abs() < 0.01);
}

#[test]
fn enemy_model_loads_and_has_triangles() {
    let cfg = config();
    let mesh = Mesh::load_from_obj(Path::new(&cfg.enemies.model_path))
        .expect("El OBJ de enemigos debe poder cargarse");
    assert!(mesh.vertex_count() > 0);
    assert!(mesh.triangle_count() > 0);
}

#[test]
fn corrected_enemy_nose_points_at_player() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.activate(
        Vec3::new(20.0, 5.0, 30.0),
        Vec3::zero(),
        cfg.sun.radius,
        &cfg.enemies,
    );
    let enemy = &mut combat.enemies[0];
    enemy.forward = Vec3::new(0.4, 0.2, 0.8).normalize();

    let rotation = enemy.rotation_matrix()
        * space_travel::Mat4::rotation_y(space_travel::math::deg_to_rad(
            cfg.enemies.model_yaw_offset_deg,
        ));
    let visible_nose = rotation
        .transform_direction(Vec3::new(1.0, 0.0, 0.0))
        .normalize();

    assert!((visible_nose - enemy.forward).length() < 1e-4);
}

#[test]
fn enemies_cannot_cross_spherical_obstacles() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.activate(
        Vec3::zero(),
        Vec3::new(0.0, 0.0, -100.0),
        10.0,
        &cfg.enemies,
    );
    combat.enemies.truncate(1);
    combat.enemies[0].position = Vec3::new(0.0, 0.0, 5.0);
    combat.enemies[0].velocity = Vec3::new(0.0, 0.0, -8.0);
    let obstacle = CollisionSphere::new(Vec3::zero(), 10.0);

    assert_eq!(
        combat.resolve_obstacle_collisions(&[obstacle], &cfg.enemies),
        1
    );
    let enemy = &combat.enemies[0];
    let normal = (enemy.position - obstacle.center).normalize();
    assert!(
        (enemy.position - obstacle.center).length()
            >= obstacle.radius + cfg.enemies.collision_radius - 1e-4
    );
    assert!(enemy.velocity.dot(normal) >= -1e-5);
}

#[test]
fn observing_enemies_remain_static_after_collision_resolution() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    combat.activate(
        Vec3::zero(),
        Vec3::new(0.0, 0.0, -100.0),
        10.0,
        &cfg.enemies,
    );
    combat.state = EncounterState::Observing;
    combat.enemies.truncate(1);
    combat.enemies[0].position = Vec3::new(0.0, 0.0, 5.0);
    combat.enemies[0].velocity = Vec3::new(0.0, 0.0, -8.0);

    combat.resolve_obstacle_collisions(&[CollisionSphere::new(Vec3::zero(), 10.0)], &cfg.enemies);

    assert_eq!(combat.enemies[0].velocity, Vec3::zero());
}

#[test]
fn collision_damage_applied_on_high_speed_impact() {
    let cfg = config();
    let mut combat = CombatSystem::new(&cfg.enemies, cfg.ship.max_speed);
    let initial_health = combat.player_health;

    // Choque a baja velocidad (< 2.0 u/s) no debe infligir daño
    let no_dmg = combat.apply_collision_damage(1.5);
    assert!(no_dmg.is_none());
    assert_eq!(combat.player_health, initial_health);

    // Choque a velocidad moderada (5.0 u/s) inflige daño leve (3.0 + (5.0 - 2.0) * 0.4 = 4.2)
    let dmg = combat.apply_collision_damage(5.0);
    assert!(dmg.is_some());
    let damage_dealt = dmg.unwrap();
    assert!((damage_dealt - 4.2).abs() < 1e-4);
    assert!((combat.player_health - (initial_health - 4.2)).abs() < 1e-4);
    assert!(combat.collision_immunity_timer > 0.0);

    // Impacto subsiguiente inmediato durante el temporizador de gracia debe ser ignorado
    let blocked_dmg = combat.apply_collision_damage(10.0);
    assert!(blocked_dmg.is_none());

    // Tras el paso de tiempo suficiente en update, vuelve a ser vulnerable
    combat.update(1.0, 0.5, Vec3::zero(), &cfg.enemies);
    assert_eq!(combat.collision_immunity_timer, 0.0);

    let next_dmg = combat.apply_collision_damage(15.0);
    assert!(next_dmg.is_some());
    // El daño máximo está acotado en 10.0
    assert!((next_dmg.unwrap() - 8.2).abs() < 1e-4);
}
