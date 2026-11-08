use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader de campo de fuerza / atmósfera gravitatoria con efecto Fresnel Rim y pulsaciones dinámicas.
///
/// Diseñado para ejecutarse en CPU en modo de mezcla alfa (Alpha Blending) sobre esferas de delimitación.
/// Produce una apariencia translúcida en el centro frontal y un brillo energético en los bordes de la silueta.
#[derive(Debug, Clone)]
pub struct ForceFieldShader {
    pub camera_pos: Vec3,
    pub field_color: Vec3,
    pub base_alpha: f32,
    pub rim_power: f32,
    pub rim_intensity: f32,
    pub sim_time: f32,
    pub pulse_speed: f32,
}

impl ForceFieldShader {
    pub fn new(camera_pos: Vec3) -> Self {
        Self {
            camera_pos,
            field_color: Vec3::new(0.25, 0.75, 1.0), // Cian energético espacial
            base_alpha: 0.02,
            rim_power: 3.0,
            rim_intensity: 0.35,
            sim_time: 0.0,
            pulse_speed: 2.0,
        }
    }

    pub fn with_color(mut self, color: Vec3) -> Self {
        self.field_color = color;
        self
    }
}

impl FragmentShader for ForceFieldShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        // 1. Vector de visión normalizado desde la superficie hacia la cámara
        let view_dir = (self.camera_pos - input.world_pos).normalize();
        let normal = input.world_normal.normalize();

        // 2. Efecto Fresnel Rim: máximo en la silueta tangencial, mínimo en el centro frontal
        let ndotv = normal.dot(view_dir).abs().clamp(0.0, 1.0);
        let fresnel = 1.0 - ndotv;
        let rim = fresnel.powf(self.rim_power);

        // 3. Ondas radiales de energía pulsante a lo largo de la latitud esférica
        let wave =
            (input.uv.y * std::f32::consts::PI * 8.0 - self.sim_time * self.pulse_speed).sin();
        let pulse = (wave * 0.5 + 0.5) * 0.04;

        // 4. Cálculo del canal alfa combinado para Alpha Blending (indicador translúcido y sutil)
        let alpha = (self.base_alpha + rim * self.rim_intensity + pulse).clamp(0.0, 0.95);

        // 5. Modulación del color emisivo final (brillo cian más intenso en el contorno)
        let intensity = 0.70 + 0.30 * rim + pulse;
        let r = (self.field_color.x * intensity).clamp(0.0, 1.0);
        let g = (self.field_color.y * intensity).clamp(0.0, 1.0);
        let b = (self.field_color.z * intensity).clamp(0.0, 1.0);

        Color::from_f32(r, g, b, alpha)
    }
}
