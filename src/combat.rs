use crate::config::EnemyConfig;
use crate::material::Material;
use crate::math::{Mat4, Vec2, Vec3, Vec4};
use crate::mesh::{Mesh, Vertex};
use crate::physics::{CollisionSphere, resolve_sphere_collision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterState {
    Dormant,
    Approaching,
    Supervising,
    Attacking,
    Observing,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub id: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub forward: Vec3,
    pub health: f32,
    pub max_health: f32,
    pub next_fire_time: f32,
}

impl Enemy {
    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    /// Orienta el eje +Z local del modelo hacia la dirección de avance.
    pub fn rotation_matrix(&self) -> Mat4 {
        orientation_from_forward(self.forward)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileOwner {
    Player,
    Enemy,
}

#[derive(Debug, Clone)]
pub struct Projectile {
    pub owner: ProjectileOwner,
    pub position: Vec3,
    pub velocity: Vec3,
    pub damage: f32,
    pub radius: f32,
    pub remaining_lifetime: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CombatUpdate {
    pub enemies_destroyed: usize,
    pub player_damage: f32,
    pub attack_started: bool,
    pub wave_spawned: Option<u32>,
    pub peace_started: bool,
    pub player_destroyed: bool,
    pub player_respawned: bool,
}

#[derive(Debug, Clone)]
pub struct CombatSystem {
    pub state: EncounterState,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub player_health: f32,
    pub collision_immunity_timer: f32,
    pub wave: u32,
    next_enemy_id: u32,
    next_player_fire_time: f32,
    next_wave_time: Option<f32>,
    last_player_shot_time: Option<f32>,
    player_destroyed_at: Option<f32>,
    player_max_speed: f32,
}

impl CombatSystem {
    pub fn new(config: &EnemyConfig, player_max_speed: f32) -> Self {
        Self {
            state: EncounterState::Dormant,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            player_health: config.player_max_health,
            collision_immunity_timer: 0.0,
            wave: 0,
            next_enemy_id: 1,
            next_player_fire_time: 0.0,
            next_wave_time: None,
            last_player_shot_time: None,
            player_destroyed_at: None,
            player_max_speed,
        }
    }

    pub fn encounter_active(&self) -> bool {
        self.state != EncounterState::Dormant
    }

    pub fn player_is_alive(&self) -> bool {
        self.player_health > 0.0
    }

    pub fn restore_player_health(&mut self, config: &EnemyConfig) {
        self.player_health = config.player_max_health;
        self.player_destroyed_at = None;
        self.collision_immunity_timer = 0.0;
    }

    /// Aplica un daño leve a la nave tras una colisión contra un obstáculo rígido
    /// si la velocidad de impacto supera un umbral mínimo. Cuenta con tiempo de gracia (inmunidad)
    /// para evitar que múltiples contactos por cuadro drenen toda la vida.
    pub fn apply_collision_damage(&mut self, impact_speed: f32) -> Option<f32> {
        if self.collision_immunity_timer <= 0.0 && impact_speed > 2.0 && self.player_is_alive() {
            let damage = (3.0 + (impact_speed - 2.0) * 0.4).clamp(3.0, 10.0);
            self.player_health = (self.player_health - damage).max(0.0);
            self.collision_immunity_timer = 0.45;
            Some(damage)
        } else {
            None
        }
    }

    pub fn deactivate(&mut self) {
        self.state = EncounterState::Dormant;
        self.enemies.clear();
        self.projectiles.clear();
        self.wave = 0;
        self.next_wave_time = None;
        self.last_player_shot_time = None;
    }

    /// Inicia el encuentro detrás del Sol visto desde la nave. De esta forma
    /// los visitantes aparecen inicialmente ocultos por el cuerpo solar.
    pub fn activate(
        &mut self,
        ship_position: Vec3,
        sun_position: Vec3,
        sun_radius: f32,
        config: &EnemyConfig,
    ) {
        self.deactivate();
        self.state = EncounterState::Approaching;
        self.wave = 1;
        self.last_player_shot_time = None;

        let sun_to_ship = (ship_position - sun_position).normalize();
        let direction = if sun_to_ship.length_squared() > 1e-6 {
            sun_to_ship
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        let spawn_center =
            sun_position - direction * (sun_radius + config.spawn_behind_sun_clearance);
        self.spawn_wave_at(
            spawn_center,
            config.initial_count,
            config.base_health,
            config,
        );
    }

    pub fn try_player_fire(
        &mut self,
        now: f32,
        position: Vec3,
        forward: Vec3,
        inherited_velocity: Vec3,
        config: &EnemyConfig,
    ) -> bool {
        if now < self.next_player_fire_time || self.player_health <= 0.0 {
            return false;
        }

        let direction = forward.normalize();
        self.projectiles.push(Projectile {
            owner: ProjectileOwner::Player,
            position: position + direction * 1.0,
            velocity: inherited_velocity + direction * config.player_projectile_speed,
            damage: config.player_projectile_damage,
            radius: config.projectile_radius,
            remaining_lifetime: config.projectile_lifetime,
        });
        self.next_player_fire_time = now + config.player_fire_interval;
        self.last_player_shot_time = Some(now);
        if self.state == EncounterState::Observing {
            self.state = EncounterState::Attacking;
        }
        true
    }

    pub fn update(
        &mut self,
        now: f32,
        dt: f32,
        ship_position: Vec3,
        config: &EnemyConfig,
    ) -> CombatUpdate {
        let mut result = CombatUpdate::default();

        if self.collision_immunity_timer > 0.0 {
            self.collision_immunity_timer = (self.collision_immunity_timer - dt).max(0.0);
        }

        if let Some(destroyed_at) = self.player_destroyed_at
            && now - destroyed_at >= config.player_respawn_delay
        {
            self.player_health = config.player_max_health;
            self.player_destroyed_at = None;
            result.player_respawned = true;
        }

        if self.state == EncounterState::Attacking
            && self
                .last_player_shot_time
                .is_some_and(|last_shot| now - last_shot >= config.peace_after_no_fire_seconds)
        {
            self.state = EncounterState::Observing;
            for enemy in &mut self.enemies {
                enemy.velocity = Vec3::zero();
            }
            self.projectiles
                .retain(|projectile| projectile.owner == ProjectileOwner::Player);
            result.peace_started = true;
        }

        self.update_enemies(now, dt, ship_position, config);
        self.update_projectiles(dt);
        self.resolve_projectile_hits(ship_position, config, &mut result);

        if self.player_health <= 0.0 && self.player_destroyed_at.is_none() {
            self.player_destroyed_at = Some(now);
            self.projectiles
                .retain(|projectile| projectile.owner == ProjectileOwner::Player);
            result.player_destroyed = true;
        }

        if result.enemies_destroyed > 0
            && matches!(
                self.state,
                EncounterState::Approaching | EncounterState::Supervising
            )
        {
            self.state = EncounterState::Attacking;
            result.attack_started = true;
        }

        self.enemies.retain(Enemy::is_alive);
        self.projectiles
            .retain(|projectile| projectile.remaining_lifetime > 0.0);

        if self.state == EncounterState::Attacking && self.enemies.is_empty() {
            let spawn_time = self.next_wave_time.get_or_insert(now + config.wave_delay);
            if now >= *spawn_time {
                self.wave += 1;
                let count = (config.initial_count
                    + config.wave_increment * (self.wave.saturating_sub(1) as usize))
                    .min(config.max_enemies);
                let health = config.base_health
                    * (1.0 + config.health_growth_per_wave * (self.wave - 1) as f32);
                let spawn_center =
                    ship_position + Vec3::new(0.0, 8.0, -config.reinforcement_distance);
                self.spawn_wave_at(spawn_center, count, health, config);
                self.next_wave_time = None;
                result.wave_spawned = Some(self.wave);
            }
        } else {
            self.next_wave_time = None;
        }

        result
    }

    /// Impide que las naves enemigas atraviesen cuerpos celestes u objetos de
    /// superficie. Si el impacto es casi frontal, agrega una velocidad lateral
    /// determinista para que la IA pueda deslizarse alrededor del obstáculo.
    pub fn resolve_obstacle_collisions(
        &mut self,
        obstacles: &[CollisionSphere],
        config: &EnemyConfig,
    ) -> usize {
        let mut collision_count = 0;
        let observing = self.state == EncounterState::Observing;
        let movement_speed = self.enemy_speed(config);
        for enemy in &mut self.enemies {
            for &obstacle in obstacles {
                if resolve_sphere_collision(
                    &mut enemy.position,
                    &mut enemy.velocity,
                    config.collision_radius,
                    obstacle,
                ) {
                    collision_count += 1;
                    let normal = (enemy.position - obstacle.center).normalize();
                    if observing {
                        enemy.velocity = Vec3::zero();
                    } else if enemy.velocity.length() < movement_speed * 0.25 {
                        let reference = if normal.dot(Vec3::up()).abs() < 0.95 {
                            Vec3::up()
                        } else {
                            Vec3::new(1.0, 0.0, 0.0)
                        };
                        let side_sign = if enemy.id % 2 == 0 { 1.0 } else { -1.0 };
                        enemy.velocity = reference.cross(normal).normalize()
                            * (movement_speed * 0.55 * side_sign);
                    }
                }
            }
        }
        collision_count
    }

    fn update_enemies(&mut self, now: f32, dt: f32, ship_position: Vec3, config: &EnemyConfig) {
        if self.state == EncounterState::Dormant {
            return;
        }

        if self.state == EncounterState::Approaching
            && self.enemies.iter().any(|enemy| {
                (ship_position - enemy.position).length() <= config.supervising_distance
            })
        {
            self.state = EncounterState::Supervising;
        }

        let may_fire = self.state == EncounterState::Attacking;
        let movement_speed = self.enemy_speed(config);
        let preferred_distance = if self.state == EncounterState::Supervising {
            config.supervising_distance
        } else {
            config.preferred_distance
        };
        let mut spawned_projectiles = Vec::new();

        for enemy in &mut self.enemies {
            let to_ship = ship_position - enemy.position;
            let distance = to_ship.length();
            let direction = if distance > 1e-6 {
                to_ship / distance
            } else {
                enemy.forward
            };
            enemy.forward = direction;

            if self.state == EncounterState::Observing {
                enemy.velocity = Vec3::zero();
                continue;
            }

            let desired_velocity = if distance > preferred_distance * 1.15 {
                direction * movement_speed
            } else if distance < preferred_distance * 0.70 {
                -direction * (movement_speed * 0.65)
            } else {
                // En la distancia de supervisión se desplazan lateralmente para
                // no amontonarse frente a la nave.
                let tangent = direction.cross(Vec3::up()).normalize();
                tangent * (movement_speed * 0.35)
            };
            let steering = 1.0 - (-config.steering_response * dt).exp();
            enemy.velocity = enemy.velocity.lerp(desired_velocity, steering);
            enemy.position += enemy.velocity * dt;

            if may_fire
                && distance <= config.fire_range
                && now >= enemy.next_fire_time
                && self.player_health > 0.0
            {
                spawned_projectiles.push(Projectile {
                    owner: ProjectileOwner::Enemy,
                    position: enemy.position + direction * config.collision_radius,
                    velocity: direction * config.enemy_projectile_speed,
                    damage: config.enemy_projectile_damage,
                    radius: config.projectile_radius,
                    remaining_lifetime: config.projectile_lifetime,
                });
                enemy.next_fire_time = now + config.enemy_fire_interval;
            }
        }

        self.projectiles.extend(spawned_projectiles);
    }

    fn enemy_speed(&self, config: &EnemyConfig) -> f32 {
        let ratio = match self.state {
            EncounterState::Approaching => config.approaching_speed_ratio,
            EncounterState::Supervising => config.supervising_speed_ratio,
            EncounterState::Attacking => config.attacking_speed_ratio,
            EncounterState::Dormant | EncounterState::Observing => 0.0,
        };
        self.player_max_speed * ratio
    }

    fn update_projectiles(&mut self, dt: f32) {
        for projectile in &mut self.projectiles {
            projectile.position += projectile.velocity * dt;
            projectile.remaining_lifetime -= dt;
        }
    }

    fn resolve_projectile_hits(
        &mut self,
        ship_position: Vec3,
        config: &EnemyConfig,
        result: &mut CombatUpdate,
    ) {
        for projectile in &mut self.projectiles {
            if projectile.remaining_lifetime <= 0.0 {
                continue;
            }

            match projectile.owner {
                ProjectileOwner::Player => {
                    for enemy in &mut self.enemies {
                        if !enemy.is_alive() {
                            continue;
                        }
                        let hit_radius = projectile.radius + config.collision_radius;
                        if (projectile.position - enemy.position).length_squared()
                            <= hit_radius * hit_radius
                        {
                            enemy.health -= projectile.damage;
                            projectile.remaining_lifetime = 0.0;
                            if !enemy.is_alive() {
                                result.enemies_destroyed += 1;
                            }
                            break;
                        }
                    }
                }
                ProjectileOwner::Enemy => {
                    let hit_radius = projectile.radius + config.player_collision_radius;
                    if (projectile.position - ship_position).length_squared()
                        <= hit_radius * hit_radius
                    {
                        let damage = projectile.damage.min(self.player_health);
                        self.player_health -= damage;
                        result.player_damage += damage;
                        projectile.remaining_lifetime = 0.0;
                    }
                }
            }
        }
    }

    fn spawn_wave_at(&mut self, center: Vec3, count: usize, health: f32, config: &EnemyConfig) {
        for index in 0..count {
            let angle = std::f32::consts::TAU * index as f32 / count.max(1) as f32;
            let offset = Vec3::new(
                angle.cos() * config.spawn_spread,
                (angle * 1.7).sin() * config.spawn_spread * 0.35,
                angle.sin() * config.spawn_spread,
            );
            self.enemies.push(Enemy {
                id: self.next_enemy_id,
                position: center + offset,
                velocity: Vec3::zero(),
                forward: Vec3::new(0.0, 0.0, 1.0),
                health,
                max_health: health,
                next_fire_time: 0.0,
            });
            self.next_enemy_id += 1;
        }
    }
}

pub fn orientation_from_forward(forward: Vec3) -> Mat4 {
    let forward = forward.normalize();
    if forward.length_squared() <= 1e-6 {
        return Mat4::IDENTITY;
    }
    let reference_up = if forward.dot(Vec3::up()).abs() < 0.98 {
        Vec3::up()
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let right = reference_up.cross(forward).normalize();
    let up = forward.cross(right).normalize();
    Mat4::from_cols(
        Vec4::new(right.x, right.y, right.z, 0.0),
        Vec4::new(up.x, up.y, up.z, 0.0),
        Vec4::new(forward.x, forward.y, forward.z, 0.0),
        Vec4::new(0.0, 0.0, 0.0, 1.0),
    )
}

/// Octaedro pequeño para proyectiles, procesado por el mismo pipeline CPU.
pub fn create_projectile_mesh() -> Mesh {
    let positions = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    let vertices = positions
        .iter()
        .map(|&position| Vertex::new(position, position.normalize(), Vec2::zero()))
        .collect();
    let indices = vec![
        2, 0, 4, 2, 4, 1, 2, 1, 5, 2, 5, 0, 3, 4, 0, 3, 1, 4, 3, 5, 1, 3, 0, 5,
    ];
    Mesh::with_material(vertices, indices, Material::default_diffuse())
}
