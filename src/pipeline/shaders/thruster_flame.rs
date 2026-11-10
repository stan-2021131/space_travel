use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;

/// Shader emisivo animado para llamas de propulsores, evaluado completamente en CPU.
pub struct ThrusterFlameShader {
    pub time: f32,
    pub animation_speed: f32,
    pub intensity: f32,
}

impl ThrusterFlameShader {
    pub const fn new(time: f32, animation_speed: f32, intensity: f32) -> Self {
        Self {
            time,
            animation_speed,
            intensity,
        }
    }
}

impl FragmentShader for ThrusterFlameShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let longitudinal = input.uv.y.clamp(0.0, 1.0);
        let phase = self.time * self.animation_speed
            + input.uv.x * std::f32::consts::TAU * 2.0
            + longitudinal * 5.0;
        let flicker = 0.82 + 0.18 * (phase.sin() * 0.5 + 0.5);
        let brightness = (self.intensity * flicker).clamp(0.0, 1.0);

        let (r, g, b) = if longitudinal < 0.22 {
            let t = longitudinal / 0.22;
            (0.72 + 0.28 * t, 0.92 - 0.20 * t, 1.0 - 0.20 * t)
        } else if longitudinal < 0.68 {
            let t = (longitudinal - 0.22) / 0.46;
            (1.0, 0.72 - 0.30 * t, 0.80 - 0.68 * t)
        } else {
            let t = (longitudinal - 0.68) / 0.32;
            (1.0 - 0.25 * t, 0.42 - 0.34 * t, 0.12 - 0.10 * t)
        };

        Color::from_f32(r * brightness, g * brightness, b * brightness, 1.0)
    }
}
