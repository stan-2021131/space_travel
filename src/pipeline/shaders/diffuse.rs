use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader emisivo para cuerpos autoiluminados.
pub struct EmissiveShader {
    pub emission_intensity: f32,
}

impl EmissiveShader {
    pub fn new(emission_intensity: f32) -> Self {
        Self { emission_intensity }
    }
}

impl Default for EmissiveShader {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl FragmentShader for EmissiveShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let kd = input.material.kd;
        Color::from_f32(
            kd.x * self.emission_intensity,
            kd.y * self.emission_intensity,
            kd.z * self.emission_intensity,
            1.0,
        )
    }
}
pub struct DiffuseShader {
    pub light_dir: Vec3,
    pub ambient_intensity: f32,
}

impl DiffuseShader {
    pub fn new() -> Self {
        Self {
            light_dir: Vec3::new(0.5, 0.8, 0.3).normalize(),
            ambient_intensity: 0.25,
        }
    }
}

impl Default for DiffuseShader {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentShader for DiffuseShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n = input.world_normal.normalize();
        let n_dot_l = n.dot(self.light_dir).max(0.0);
        let lighting =
            (self.ambient_intensity + n_dot_l * (1.0 - self.ambient_intensity)).clamp(0.0, 1.0);

        let kd = input.material.kd;
        Color::from_f32(kd.x * lighting, kd.y * lighting, kd.z * lighting, 1.0)
    }
}

/// Shader de iluminación difusa texturada con luz direccional/puntual desde el sol y luz ambiental.
pub struct TexturedDiffuseShader {
    pub light_pos: Vec3,
    pub ambient_intensity: f32,
}

impl TexturedDiffuseShader {
    pub fn new(light_pos: Vec3) -> Self {
        Self {
            light_pos,
            ambient_intensity: 0.15,
        }
    }
}

impl Default for TexturedDiffuseShader {
    fn default() -> Self {
        Self::new(Vec3::zero())
    }
}

impl FragmentShader for TexturedDiffuseShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n = input.world_normal.normalize();
        let l = (self.light_pos - input.world_pos).normalize();
        let n_dot_l = n.dot(l).max(0.0);
        let lighting =
            (self.ambient_intensity + n_dot_l * (1.0 - self.ambient_intensity)).clamp(0.0, 1.0);

        let tex_color = if let Some(ref tex) = input.material.texture {
            tex.sample(input.uv.x, input.uv.y)
        } else {
            Color::WHITE
        };

        let kd = input.material.kd;
        let r = (tex_color.r as f32 / 255.0) * kd.x * lighting;
        let g = (tex_color.g as f32 / 255.0) * kd.y * lighting;
        let b = (tex_color.b as f32 / 255.0) * kd.z * lighting;

        Color::from_f32(r, g, b, 1.0)
    }
}
