use crate::config::ShipConfig;
use crate::math::{Mat4, Vec3, deg_to_rad};

/// Obstáculo esférico usado por todas las entidades móviles. Los cuerpos
/// celestes y objetos de superficie se aproximan mediante esta envolvente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl CollisionSphere {
    pub const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

/// Información de contacto detallada producida al resolver una colisión física.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionContact {
    /// Punto de impacto en la superficie del obstáculo en coordenadas de mundo.
    pub point: Vec3,
    /// Normal de colisión unitaria que apunta hacia afuera del obstáculo.
    pub normal: Vec3,
    /// Magnitud de la velocidad perpendicular con la que la entidad impactó antes del rebote.
    pub impact_speed: f32,
}

/// Resuelve una penetración entre una entidad móvil y un obstáculo esférico con restitución (rebote/retroceso).
///
/// Si `restitution > 0.0`, la velocidad perpendicular se invierte proporcionalmente amortiguada.
/// Retorna `Some(CollisionContact)` si existió colisión y penetración.
pub fn resolve_sphere_collision_with_rebound(
    position: &mut Vec3,
    velocity: &mut Vec3,
    entity_radius: f32,
    obstacle: CollisionSphere,
    restitution: f32,
) -> Option<CollisionContact> {
    let effective_radius = obstacle.radius + entity_radius;
    let delta = *position - obstacle.center;
    let distance = delta.length();
    if distance >= effective_radius {
        return None;
    }

    let normal = if distance > 1e-6 {
        delta / distance
    } else {
        Vec3::up()
    };
    *position = obstacle.center + normal * effective_radius;

    let inward_speed = velocity.dot(normal);
    let impact_speed = if inward_speed < 0.0 {
        let e = restitution.clamp(0.0, 1.0);
        // Rebote físico: retroceso en la dirección normal de la superficie
        *velocity -= normal * (inward_speed * (1.0 + e));
        -inward_speed
    } else {
        0.0
    };

    Some(CollisionContact {
        point: obstacle.center + normal * obstacle.radius,
        normal,
        impact_speed,
    })
}

/// Resuelve una penetración entre una entidad móvil y un obstáculo esférico sin restitución (deslizamiento inelástico puro).
pub fn resolve_sphere_collision(
    position: &mut Vec3,
    velocity: &mut Vec3,
    entity_radius: f32,
    obstacle: CollisionSphere,
) -> bool {
    resolve_sphere_collision_with_rebound(position, velocity, entity_radius, obstacle, 0.0)
        .is_some()
}

/// Estado de entrada del usuario para el control de la nave.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShipInput {
    pub throttle_forward: bool, // Tecla W
    pub throttle_reverse: bool, // Tecla S (freno / reversa)
    pub yaw_left: bool,         // Tecla A
    pub yaw_right: bool,        // Tecla D
    pub pitch_up: bool,         // Flecha Arriba
    pub pitch_down: bool,       // Flecha Abajo
    pub roll_left: bool,        // Tecla Q
    pub roll_right: bool,       // Tecla E
    pub boost: bool,            // Shift izquierdo junto con W
    pub fire: bool,             // Barra espaciadora
}

/// Estado dinámico de la nave espacial con cinemática arcade e inercia.
#[derive(Debug, Clone)]
pub struct ShipState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub pitch_deg: f32,
    pub yaw_deg: f32,
    pub roll_deg: f32,
    pub collision_radius: f32,
}

impl ShipState {
    pub fn new(config: &ShipConfig) -> Self {
        Self {
            position: Vec3::new(
                config.initial_position[0],
                config.initial_position[1],
                config.initial_position[2],
            ),
            velocity: Vec3::zero(),
            pitch_deg: config.initial_rotation_deg[0],
            yaw_deg: config.initial_rotation_deg[1],
            roll_deg: config.initial_rotation_deg[2],
            collision_radius: config.collision_radius,
        }
    }

    /// Reinicia la nave a su estado inicial configurado.
    pub fn reset(&mut self, config: &ShipConfig) {
        self.position = Vec3::new(
            config.initial_position[0],
            config.initial_position[1],
            config.initial_position[2],
        );
        self.velocity = Vec3::zero();
        self.pitch_deg = config.initial_rotation_deg[0];
        self.yaw_deg = config.initial_rotation_deg[1];
        self.roll_deg = config.initial_rotation_deg[2];
    }

    /// Retorna la matriz de rotación pura de la nave (Yaw * Pitch * Roll).
    pub fn rotation_matrix(&self) -> Mat4 {
        Mat4::rotation_euler(
            deg_to_rad(self.yaw_deg),
            deg_to_rad(self.pitch_deg),
            deg_to_rad(self.roll_deg),
        )
    }

    /// Vector frontal hacia el que avanza la nave.
    /// El modelo de la nave fue creado con su morro orientado hacia +Z local:
    /// local_forward = (0, 0, +1), local_up = (0, 1, 0), local_right = (1, 0, 0).
    pub fn forward(&self) -> Vec3 {
        let r = self.rotation_matrix();
        r.transform_direction(Vec3::new(0.0, 0.0, 1.0)).normalize()
    }

    /// Vector superior local de la nave (up = (0, 1, 0)).
    pub fn up(&self) -> Vec3 {
        let r = self.rotation_matrix();
        r.transform_direction(Vec3::new(0.0, 1.0, 0.0)).normalize()
    }

    /// Vector lateral derecho de la nave (right = (1, 0, 0)).
    pub fn right(&self) -> Vec3 {
        let r = self.rotation_matrix();
        r.transform_direction(Vec3::new(1.0, 0.0, 0.0)).normalize()
    }

    /// Actualiza la orientación, aceleración, velocidad y posición de la nave para un paso dt.
    pub fn update(&mut self, input: &ShipInput, config: &ShipConfig, dt: f32) {
        // 1. Orientación progresiva basada en tiempo
        if input.yaw_left {
            self.yaw_deg += config.yaw_speed * (180.0 / std::f32::consts::PI) * dt;
        }
        if input.yaw_right {
            self.yaw_deg -= config.yaw_speed * (180.0 / std::f32::consts::PI) * dt;
        }

        if input.pitch_up {
            self.pitch_deg -= config.pitch_speed * (180.0 / std::f32::consts::PI) * dt;
        }
        if input.pitch_down {
            self.pitch_deg += config.pitch_speed * (180.0 / std::f32::consts::PI) * dt;
        }

        if input.roll_left {
            self.roll_deg -= config.roll_speed * (180.0 / std::f32::consts::PI) * dt;
        }
        if input.roll_right {
            self.roll_deg += config.roll_speed * (180.0 / std::f32::consts::PI) * dt;
        }

        // Normalización de ángulos de Euler
        self.yaw_deg = self.yaw_deg.rem_euclid(360.0);
        self.pitch_deg = self.pitch_deg.clamp(-89.0, 89.0);
        self.roll_deg = self.roll_deg.rem_euclid(360.0);

        let fwd = self.forward();

        // 2. Aceleración y frenado / reversa progresivos
        let current_forward_speed = self.velocity.dot(fwd);

        let boosting = input.boost && input.throttle_forward;
        if input.throttle_forward {
            let acceleration = if boosting {
                config.boost_acceleration
            } else {
                config.acceleration
            };
            self.velocity += fwd * (acceleration * dt);
        }

        if input.throttle_reverse {
            if current_forward_speed > 0.5 {
                // Frenado progresivo
                let brake_impulse = config.braking_force * dt;
                let new_speed = (current_forward_speed - brake_impulse).max(0.0);
                self.velocity += fwd * (new_speed - current_forward_speed);
            } else {
                // Reversa controlada
                self.velocity -= fwd * (config.reverse_acceleration * dt);
            }
        }

        // 3. Resistencia espacial (drag) suave
        let drag_factor = (1.0 - config.drag * dt).clamp(0.0, 1.0);
        self.velocity *= drag_factor;

        // 4. Límite de velocidad máxima
        let speed = self.velocity.length();
        let speed_limit = if boosting {
            config.boost_max_speed
        } else {
            config.max_speed
        };
        if speed > speed_limit {
            let new_speed = if boosting {
                speed_limit
            } else {
                (speed * (1.0 - config.boost_release_drag * dt).clamp(0.0, 1.0))
                    .max(config.max_speed)
            };
            self.velocity = (self.velocity / speed) * new_speed;
        }

        // 5. Integración de posición
        self.position += self.velocity * dt;
    }

    /// Resuelve la colisión inelástica con deslizamiento tangencial contra un cuerpo esférico.
    ///
    /// Retorna `true` si hubo contacto y se corrigió la trayectoria.
    pub fn resolve_collision(&mut self, body_center: Vec3, body_radius: f32, margin: f32) -> bool {
        resolve_sphere_collision(
            &mut self.position,
            &mut self.velocity,
            self.collision_radius,
            CollisionSphere::new(body_center, body_radius + margin),
        )
    }

    /// Aplica atracción gravitatoria newtoniana suave dirigida hacia el centro de un cuerpo celeste.
    ///
    /// - `body_center`: Posición 3D del centro de masa del cuerpo.
    /// - `body_radius`: Radio físico de la superficie (evita singularidades).
    /// - `gravity_radius`: Radio máximo de la esfera de influencia gravitatoria.
    /// - `surface_gravity`: Aceleración nominal en la superficie física del cuerpo (g0).
    /// - `dt`: Delta de tiempo en segundos.
    ///
    /// Retorna `Some(a)` con el vector de aceleración aplicado, o `None` si la nave está fuera del campo.
    pub fn apply_gravitational_attraction(
        &mut self,
        body_center: Vec3,
        body_radius: f32,
        gravity_radius: f32,
        surface_gravity: f32,
        dt: f32,
    ) -> Option<Vec3> {
        if surface_gravity <= 0.0 || gravity_radius <= body_radius || dt <= 0.0 {
            return None;
        }

        let to_center = body_center - self.position;
        let dist = to_center.length();

        // Fuera de la esfera de influencia o centro coincidente
        if dist > gravity_radius || dist < 1e-4 {
            return None;
        }

        let direction = to_center / dist;

        // 1. Ley de inverso del cuadrado protegida contra singularidad en la superficie
        let clamped_dist = dist.max(body_radius);
        let ratio = body_radius / clamped_dist;
        let base_accel = surface_gravity * (ratio * ratio);

        // 2. Factor de caída suave (smoothstep) en el borde exterior para evitar discontinuidades
        let t = ((gravity_radius - dist) / (gravity_radius - body_radius)).clamp(0.0, 1.0);
        let smooth_falloff = t * t * (3.0 - 2.0 * t);

        let accel = direction * (base_accel * smooth_falloff);
        self.velocity += accel * dt;

        Some(accel)
    }
}

/// Daño térmico por segundo: cero fuera del radio de peligro y máximo en la superficie.
pub fn solar_damage_per_second(
    distance_to_center: f32,
    surface_radius: f32,
    damage_radius: f32,
    max_damage_per_second: f32,
) -> f32 {
    if !distance_to_center.is_finite()
        || damage_radius <= surface_radius
        || max_damage_per_second <= 0.0
        || distance_to_center >= damage_radius
    {
        return 0.0;
    }

    let t =
        ((damage_radius - distance_to_center) / (damage_radius - surface_radius)).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    max_damage_per_second * smooth
}
