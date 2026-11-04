use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader animado de plasma solar para cuerpos autoiluminados en CPU.
///
/// Modula la radiación térmica superficial mediante ondas sinusoidales tridimensionales
/// continuas que cambian de forma continua con el tiempo transcurrido (`sim_time`),
/// eliminando facetas poligonales y simulando convección y llamaradas estelares.
pub struct PlasmaShader {
    pub time: f32,
    pub speed: f32,
    pub scale: f32,
    pub sun_center: Vec3,
}

impl PlasmaShader {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            speed: 1.5,
            scale: 1.0,
            sun_center: Vec3::zero(),
        }
    }

    /// Actualiza el tiempo de simulación para animar el plasma.
    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }
}

impl Default for PlasmaShader {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentShader for PlasmaShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        // Cálculo de dirección esférica continua desde el centro del Sol para eliminar facetas
        let offset = input.world_pos - self.sun_center;
        let n = if offset.length_squared() > 1e-4 {
            offset.normalize()
        } else {
            input.world_normal.normalize()
        };

        let t = self.time * self.speed;

        // 1. Grandes ondas de convección de plasma (macro-llamaradas)
        let v1 = (n.x * 3.5 * self.scale + t * 1.6).sin();
        let v2 = (n.y * 3.5 * self.scale + t * 1.3).sin();
        let v3 = (n.z * 3.5 * self.scale + (n.x + n.y) * 2.2 + t * 2.0).sin();
        let macro_wave = (v1 + v2 + v3 + 3.0) / 6.0;

        // 2. Micro-granulación solar (células de ebullición térmica)
        let micro_wave = ((n.x * 12.0 - t * 2.5).sin()
            * (n.y * 12.0 + t * 2.1).sin()
            * (n.z * 12.0 - t * 1.8).sin())
            * 0.5
            + 0.5;

        // Combinación equilibrada de llamaradas y granulación
        let s = (macro_wave * 0.75 + micro_wave * 0.25).clamp(0.0, 1.0);

        // Rampa de radiación térmica de alto contraste:
        // [0.0 .. 0.45]: Rojo carmesí profundo (filamentos fríos)
        // [0.45 .. 0.80]: Naranja dorado solar intenso
        // [0.80 .. 1.00]: Amarillo brillante y blanco incandescente (erupciones coronales)
        let (r, g, b) = if s < 0.45 {
            let f = s / 0.45;
            let r = 0.65 + 0.35 * f;
            let g = 0.05 + 0.35 * f;
            let b = 0.0;
            (r, g, b)
        } else if s < 0.80 {
            let f = (s - 0.45) / 0.35;
            let r = 1.0;
            let g = 0.40 + 0.45 * f;
            let b = 0.0 + 0.15 * f;
            (r, g, b)
        } else {
            let f = (s - 0.80) / 0.20;
            let r = 1.0;
            let g = 0.85 + 0.15 * f;
            let b = 0.15 + 0.65 * f;
            (r, g, b)
        };

        let kd = input.material.kd;
        Color::from_f32(
            (r * kd.x).min(1.0),
            (g * kd.y).min(1.0),
            (b * kd.z).min(1.0),
            1.0,
        )
    }
}
