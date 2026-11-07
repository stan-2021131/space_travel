use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Configuración global de la aplicación deserializada desde archivo TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub simulation: SimulationConfig,
    pub ship: ShipConfig,
    pub enemies: EnemyConfig,
    pub camera: CameraConfig,
    pub sun: SunConfig,
    pub planets: Vec<PlanetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemyConfig {
    pub model_path: String,
    pub target_length_wu: f32,
    pub model_yaw_offset_deg: f32,
    pub initial_count: usize,
    pub max_enemies: usize,
    pub wave_increment: usize,
    pub base_health: f32,
    pub health_growth_per_wave: f32,
    pub move_speed: f32,
    pub steering_response: f32,
    pub preferred_distance: f32,
    pub fire_range: f32,
    pub enemy_fire_interval: f32,
    pub enemy_projectile_speed: f32,
    pub enemy_projectile_damage: f32,
    pub player_fire_interval: f32,
    pub player_projectile_speed: f32,
    pub player_projectile_damage: f32,
    pub projectile_lifetime: f32,
    pub projectile_radius: f32,
    pub collision_radius: f32,
    pub player_collision_radius: f32,
    pub player_max_health: f32,
    pub spawn_behind_sun_clearance: f32,
    pub spawn_spread: f32,
    pub reinforcement_distance: f32,
    pub wave_delay: f32,
    pub peace_after_no_fire_seconds: f32,
    pub player_respawn_delay: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimulationConfig {
    #[serde(default = "default_seed")]
    pub seed: u32,
    pub time_scale: f32,
    pub fixed_timestep: f32,
    pub max_frame_delta: f32,
}

fn default_seed() -> u32 {
    67
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShipConfig {
    pub model_path: String,
    pub target_length_wu: f32,
    pub initial_position: [f32; 3],
    pub initial_rotation_deg: [f32; 3],
    pub acceleration: f32,
    pub reverse_acceleration: f32,
    pub braking_force: f32,
    pub drag: f32,
    pub max_speed: f32,
    pub yaw_speed: f32,
    pub pitch_speed: f32,
    pub roll_speed: f32,
    pub collision_radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraConfig {
    pub distance_behind: f32,
    pub height_above: f32,
    pub look_ahead: f32,
    pub position_smoothing: f32,
    pub rotation_smoothing: f32,
    pub fov_deg: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SunConfig {
    pub id: String,
    pub model_path: String,
    pub radius: f32,
    pub collision_margin: f32,
    pub rotation_period: f32,
    pub shader: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceObjectConfig {
    pub name: String,
    pub model_path: String,
    #[serde(default = "default_surface_count")]
    pub count: usize,
    #[serde(default = "default_surface_scale")]
    pub scale: f32,
    #[serde(default = "default_surface_max_distance")]
    pub max_distance: f32,
    #[serde(default = "default_surface_fade_range")]
    pub fade_range: f32,
}

fn default_surface_count() -> usize {
    6
}

fn default_surface_scale() -> f32 {
    0.035
}

fn default_surface_max_distance() -> f32 {
    55.0
}

fn default_surface_fade_range() -> f32 {
    15.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlanetConfig {
    pub id: String,
    pub model_path: String,
    pub radius: f32,
    pub orbital_radius: f32,
    pub orbital_period: f32,
    pub rotation_period: f32,
    pub initial_orbital_angle_deg: f32,
    pub collision_margin: f32,
    pub shader: String,
    #[serde(default)]
    pub surface_objects: Vec<SurfaceObjectConfig>,
}

impl AppConfig {
    /// Carga y deserializa el archivo TOML de configuración.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| {
            format!(
                "Error al leer archivo de configuración '{}': {}",
                path.display(),
                e
            )
        })?;

        let config: AppConfig = toml::from_str(&content).map_err(|e| {
            format!(
                "Error de sintaxis en archivo de configuración '{}': {}",
                path.display(),
                e
            )
        })?;

        config.validate(path)?;
        Ok(config)
    }

    /// Valida las relaciones físicas y semánticas fundamentales del sistema solar.
    pub fn validate(&self, source_path: &Path) -> Result<(), String> {
        let p = source_path.display();

        if self.simulation.time_scale <= 0.0 || self.simulation.fixed_timestep <= 0.0 {
            return Err(format!(
                "[{}] Parámetros de simulación deben ser mayores a 0",
                p
            ));
        }

        if self.sun.radius <= 0.0 {
            return Err(format!("[{}] El radio del Sol debe ser positivo", p));
        }

        if self.camera.near_plane <= 0.0 || self.camera.near_plane >= self.camera.far_plane {
            return Err(format!(
                "[{}] Planos de cámara inválidos: near ({}) debe ser menor que far ({})",
                p, self.camera.near_plane, self.camera.far_plane
            ));
        }

        if self.ship.max_speed <= 0.0 || self.ship.acceleration <= 0.0 {
            return Err(format!(
                "[{}] Parámetros de nave (max_speed, acceleration) deben ser positivos",
                p
            ));
        }

        let enemy = &self.enemies;
        if enemy.initial_count == 0
            || enemy.max_enemies < enemy.initial_count
            || enemy.target_length_wu <= 0.0
            || enemy.base_health <= 0.0
            || enemy.move_speed <= 0.0
            || enemy.steering_response <= 0.0
            || enemy.preferred_distance <= 0.0
            || enemy.fire_range <= 0.0
            || enemy.enemy_fire_interval <= 0.0
            || enemy.enemy_projectile_speed <= 0.0
            || enemy.enemy_projectile_damage <= 0.0
            || enemy.player_fire_interval <= 0.0
            || enemy.player_projectile_speed <= 0.0
            || enemy.player_projectile_damage <= 0.0
            || enemy.projectile_lifetime <= 0.0
            || enemy.projectile_radius <= 0.0
            || enemy.collision_radius <= 0.0
            || enemy.player_collision_radius <= 0.0
            || enemy.player_max_health <= 0.0
            || enemy.spawn_behind_sun_clearance <= 0.0
            || enemy.spawn_spread <= 0.0
            || enemy.reinforcement_distance <= 0.0
            || enemy.wave_delay <= 0.0
            || enemy.peace_after_no_fire_seconds <= 0.0
            || enemy.player_respawn_delay <= 0.0
            || !enemy.model_yaw_offset_deg.is_finite()
        {
            return Err(format!("[{}] Configuración de enemigos inválida", p));
        }

        if self.camera.position_smoothing <= 0.0 || self.camera.rotation_smoothing <= 0.0 {
            return Err(format!("[{}] Suavizado de cámara debe ser mayor a 0", p));
        }

        let mut ids = HashSet::new();
        ids.insert(self.sun.id.as_str());

        for planet in &self.planets {
            if planet.radius <= 0.0 {
                return Err(format!(
                    "[{}] Radio de planeta '{}' debe ser positivo",
                    p, planet.id
                ));
            }
            if planet.orbital_radius <= self.sun.radius {
                return Err(format!(
                    "[{}] Órbita de '{}' ({}) colisiona con el Sol (radio {})",
                    p, planet.id, planet.orbital_radius, self.sun.radius
                ));
            }
            if !ids.insert(planet.id.as_str()) {
                return Err(format!(
                    "[{}] ID duplicado en configuración: '{}'",
                    p, planet.id
                ));
            }
            for obj in &planet.surface_objects {
                if obj.count == 0
                    || obj.scale <= 0.0
                    || obj.max_distance <= 0.0
                    || obj.fade_range <= 0.0
                {
                    return Err(format!(
                        "[{}] Parámetros de objeto de superficie '{}' en planeta '{}' deben ser positivos",
                        p, obj.name, planet.id
                    ));
                }
                if obj.fade_range >= obj.max_distance {
                    return Err(format!(
                        "[{}] En objeto de superficie '{}' del planeta '{}': fade_range ({}) debe ser menor que max_distance ({})",
                        p, obj.name, planet.id, obj.fade_range, obj.max_distance
                    ));
                }
            }
        }

        Ok(())
    }
}
