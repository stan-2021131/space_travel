use crate::config::ShipConfig;
use crate::math::{Mat4, Vec3, deg_to_rad};

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

        if input.throttle_forward {
            self.velocity += fwd * (config.acceleration * dt);
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
        if speed > config.max_speed {
            self.velocity = (self.velocity / speed) * config.max_speed;
        }

        // 5. Integración de posición
        self.position += self.velocity * dt;
    }

    /// Resuelve la colisión inelástica con deslizamiento tangencial contra un cuerpo esférico.
    ///
    /// Retorna `true` si hubo contacto y se corrigió la trayectoria.
    pub fn resolve_collision(&mut self, body_center: Vec3, body_radius: f32, margin: f32) -> bool {
        let effective_radius = body_radius + self.collision_radius + margin;
        let delta = self.position - body_center;
        let dist = delta.length();

        if dist < effective_radius {
            // 1. Normal de contacto (evita divisiones por cero si los centros coinciden)
            let normal = if dist > 1e-6 {
                delta / dist
            } else {
                Vec3::up()
            };

            // 2. Reposición instantánea sobre el límite seguro
            self.position = body_center + normal * effective_radius;

            // 3. Respuesta inelástica: eliminar exclusivamente la velocidad hacia el cuerpo
            let normal_vel = self.velocity.dot(normal);
            if normal_vel < 0.0 {
                // Se anula la componente normal, conservando al 100% la velocidad tangencial
                self.velocity -= normal * normal_vel;
            }

            return true;
        }

        false
    }
}
