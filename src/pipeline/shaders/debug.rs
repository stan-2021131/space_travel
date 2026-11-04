use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;

/// Shader de diagnóstico para visualizar la profundidad linealizada (Depth Debug).
pub struct DepthDebugShader {
    pub z_near: f32,
    pub z_far: f32,
    pub display_min: f32,
    pub display_max: f32,
}

impl DepthDebugShader {
    pub fn new(z_near: f32, z_far: f32, display_min: f32, display_max: f32) -> Self {
        Self {
            z_near,
            z_far,
            display_min,
            display_max,
        }
    }

    /// Convierte la coordenada de profundidad no lineal z_ndc in [-1, 1] a distancia lineal positiva.
    #[inline]
    pub fn linearize_depth(&self, z_ndc: f32) -> f32 {
        let denominator = (self.z_far + self.z_near) - z_ndc * (self.z_far - self.z_near);
        if denominator.abs() > 1e-7 {
            (2.0 * self.z_near * self.z_far) / denominator
        } else {
            self.z_far
        }
    }
}

impl Default for DepthDebugShader {
    fn default() -> Self {
        Self::new(0.1, 500.0, 1.0, 80.0)
    }
}

impl FragmentShader for DepthDebugShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let z_linear = self.linearize_depth(input.depth_ndc);
        let range = self.display_max - self.display_min;
        let normalized = if range.abs() > 1e-6 {
            ((z_linear - self.display_min) / range).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let brightness = 1.0 - normalized;
        Color::from_f32(brightness, brightness, brightness, 1.0)
    }
}

/// Shader de diagnóstico para visualizar normales de superficie mapeadas a colores RGB.
pub struct NormalsDebugShader;

impl FragmentShader for NormalsDebugShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n = input.world_normal.normalize();
        let r = n.x * 0.5 + 0.5;
        let g = n.y * 0.5 + 0.5;
        let b = n.z * 0.5 + 0.5;
        Color::from_f32(r, g, b, 1.0)
    }
}
