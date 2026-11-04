use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader estilizado Toon / Anime (Cel Shading) ejecutado en CPU.
///
/// Implementa una estética manga/anime auténtica con:
/// 1. Estrella solar radiante con bandas concéntricas cálidas y contorno estilizado.
/// 2. Cuantización de luz difusa en 3 escalones de alto contraste (luz plena, semitono y sombra limpia).
/// 3. Delineado de tinta negra anime (*ink outlines*) en los bordes de la silueta.
pub struct ToonShader {
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub is_sun: bool,
}

impl ToonShader {
    pub fn new(light_pos: Vec3, camera_pos: Vec3) -> Self {
        Self {
            light_pos,
            camera_pos,
            is_sun: false,
        }
    }
}

impl FragmentShader for ToonShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        // Vector hacia la cámara
        let to_cam = self.camera_pos - input.world_pos;
        let v = if to_cam.length_squared() > 1e-4 {
            to_cam.normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        // --- CASO 1: El Sol (Estrella radiante estilo anime) ---
        if self.is_sun {
            let offset = input.world_pos - self.light_pos;
            let n = if offset.length_squared() > 1e-4 {
                offset.normalize()
            } else {
                input.world_normal.normalize()
            };

            let n_dot_v = n.dot(v).abs();

            // Bandas concéntricas de energía estelar anime:
            return if n_dot_v < 0.12 {
                // Trazo exterior oscuro de silueta cómic
                Color::from_f32(0.18, 0.04, 0.0, 1.0)
            } else if n_dot_v < 0.35 {
                // Corona exterior de fuego naranja manga
                Color::from_f32(1.0, 0.36, 0.02, 1.0)
            } else if n_dot_v < 0.72 {
                // Disco medio amarillo dorado radiante
                Color::from_f32(1.0, 0.82, 0.05, 1.0)
            } else {
                // Núcleo central incandescente blanco-amarillo
                Color::from_f32(1.0, 1.0, 0.85, 1.0)
            };
        }

        // --- CASO 2: Cuerpos iluminados (Nave y Planetas) ---
        let n = input.world_normal.normalize();
        let to_light = self.light_pos - input.world_pos;
        let l = if to_light.length_squared() > 1e-4 {
            to_light.normalize()
        } else {
            Vec3::new(0.4, 0.8, 0.4).normalize()
        };

        // 1. Delineado de tinta negra anime (Ink Outline en el borde exterior estricto)
        let n_dot_v = n.dot(v).abs();
        if n_dot_v < 0.14 {
            return Color::new(18, 18, 24, 255); // Trazo de tinta oscura
        }

        // 2. Cuantización de iluminación difusa en 3 niveles de alto contraste (Cel Shading)
        let n_dot_l = n.dot(l);
        let toon_factor = if n_dot_l > 0.40 {
            1.00 // Luz plena directa
        } else if n_dot_l > 0.02 {
            0.62 // Semitono vibrante
        } else {
            0.28 // Sombra dura limpia
        };

        // Color base del objeto (textura o Kd del material)
        let base_color = if let Some(ref tex) = input.material.texture {
            tex.sample(input.uv.x, input.uv.y)
        } else {
            let kd = input.material.kd;
            Color::from_f32(kd.x, kd.y, kd.z, 1.0)
        };

        let r = (base_color.r as f32 / 255.0) * toon_factor;
        let g = (base_color.g as f32 / 255.0) * toon_factor;
        let b = (base_color.b as f32 / 255.0) * toon_factor;

        Color::from_f32(r.min(1.0), g.min(1.0), b.min(1.0), 1.0)
    }
}
