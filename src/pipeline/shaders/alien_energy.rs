use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Recubrimiento energético animado para las naves alienígenas, evaluado en CPU.
///
/// Combina iluminación solar difusa con un borde Fresnel y bandas de energía que
/// recorren el fuselaje en el espacio de mundo. El tiempo proviene de la simulación,
/// por lo que la animación no depende de la cantidad de cuadros renderizados.
pub struct AlienEnergyShader {
    pub time: f32,
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub ambient_intensity: f32,
    pub scan_speed: f32,
    pub scan_frequency: f32,
    pub rim_power: f32,
}

impl AlienEnergyShader {
    pub fn new(light_pos: Vec3, camera_pos: Vec3) -> Self {
        Self {
            time: 0.0,
            light_pos,
            camera_pos,
            ambient_intensity: 0.28,
            scan_speed: 4.0,
            scan_frequency: 3.5,
            rim_power: 2.2,
        }
    }

    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }
}

impl FragmentShader for AlienEnergyShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n = input.world_normal.normalize();

        let to_light = self.light_pos - input.world_pos;
        let l = if to_light.length_squared() > 1e-4 {
            to_light.normalize()
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };

        let to_camera = self.camera_pos - input.world_pos;
        let v = if to_camera.length_squared() > 1e-4 {
            to_camera.normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        let diffuse = n.dot(l).max(0.0);
        let lighting =
            (self.ambient_intensity + diffuse * (1.0 - self.ambient_intensity)).clamp(0.0, 1.0);

        // El borde Fresnel hace visible la silueta energética sin requerir transparencia.
        let view_alignment = n.dot(v).abs().clamp(0.0, 1.0);
        let rim = (1.0 - view_alignment).powf(self.rim_power);

        // Bandas estrechas que ascienden por el fuselaje y varían también en X/Z
        // para evitar que el efecto parezca un simple gradiente horizontal.
        let phase = input.world_pos.y * self.scan_frequency + input.world_pos.x * 0.45
            - input.world_pos.z * 0.25
            - self.time * self.scan_speed;
        let scan_wave = phase.sin() * 0.5 + 0.5;
        let scan = scan_wave.powi(10);
        let pulse = (self.time * 2.1).sin() * 0.5 + 0.5;
        let energy = (rim * 0.70 + scan * (0.30 + pulse * 0.25)).clamp(0.0, 1.0);

        let kd = input.material.kd;
        let base_r = kd.x * lighting;
        let base_g = kd.y * lighting;
        let base_b = kd.z * lighting;

        // Emisión cian-violeta: altera tanto la silueta como las bandas y produce
        // un resultado claramente distinto del metal Blinn-Phong de la nave principal.
        Color::from_f32(
            base_r * 0.55 + energy * 0.20,
            base_g * 0.70 + energy * 0.72,
            base_b * 0.85 + energy,
            1.0,
        )
    }
}
