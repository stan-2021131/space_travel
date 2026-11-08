use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader con mapeo de normales (Normal Mapping) evaluado en CPU en espacio tangente (TBN).
///
/// Si el material cuenta con textura de normales en `material.normal_texture`, decodifica el texel
/// RGB de espacio tangente a [-1, 1], construye la matriz ortonormal TBN = [T | B | N] en espacio
/// de mundo y perturba la normal antes de calcular la iluminación difusa Lambert y especular Blinn-Phong.
pub struct NormalMappedShader {
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub ambient_intensity: f32,
    pub specular_power: f32,
    pub specular_intensity: f32,
}

impl NormalMappedShader {
    pub fn new(light_pos: Vec3, camera_pos: Vec3) -> Self {
        Self {
            light_pos,
            camera_pos,
            ambient_intensity: 0.12,
            specular_power: 32.0,
            specular_intensity: 0.35,
        }
    }
}

impl FragmentShader for NormalMappedShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n_geom = input.world_normal;
        let t_geom = input.world_tangent;

        // Construcción de la matriz TBN en espacio de mundo:
        // Se aplica Gram-Schmidt para garantizar ortonormalidad estricta entre T y N
        let t = (t_geom - n_geom * t_geom.dot(n_geom)).normalize();
        let b = n_geom.cross(t).normalize();
        let n = n_geom;

        // Perturbación de normal según Normal Map (si existe en el material)
        let world_normal = if let Some(ref norm_tex) = input.material.normal_texture {
            let sample = norm_tex.sample(input.uv.x, input.uv.y);
            // Decodificación de canales [0, 255] a rango [-1.0, 1.0] en espacio tangente
            let n_tangent = Vec3::new(
                (sample.r as f32 * (1.0 / 255.0)) * 2.0 - 1.0,
                (sample.g as f32 * (1.0 / 255.0)) * 2.0 - 1.0,
                (sample.b as f32 * (1.0 / 255.0)) * 2.0 - 1.0,
            )
            .normalize();

            // Transformación de espacio tangente a espacio de mundo:
            // N_world = T * n_tangent.x + B * n_tangent.y + N * n_tangent.z
            (t * n_tangent.x + b * n_tangent.y + n * n_tangent.z).normalize()
        } else {
            n
        };

        // Color difuso base de la textura o reflectancia Kd del material
        let base_color = if let Some(ref tex) = input.material.texture {
            let sample = tex.sample(input.uv.x, input.uv.y);
            Vec3::new(
                sample.r as f32 * (1.0 / 255.0),
                sample.g as f32 * (1.0 / 255.0),
                sample.b as f32 * (1.0 / 255.0),
            )
        } else {
            input.material.kd
        };

        // Vector de incidencia de luz solar
        let l = (self.light_pos - input.world_pos).normalize();
        let n_dot_l = world_normal.dot(l).max(0.0);

        // Componente especular Blinn-Phong con la normal perturbada
        let v = (self.camera_pos - input.world_pos).normalize();
        let h = (l + v).normalize();
        let spec = if n_dot_l > 0.0 {
            world_normal.dot(h).max(0.0).powf(self.specular_power) * self.specular_intensity
        } else {
            0.0
        };

        let diffuse = self.ambient_intensity + (1.0 - self.ambient_intensity) * n_dot_l;

        let r = (base_color.x * diffuse + spec).clamp(0.0, 1.0);
        let g = (base_color.y * diffuse + spec).clamp(0.0, 1.0);
        let b = (base_color.z * diffuse + spec).clamp(0.0, 1.0);

        Color::from_f32(r, g, b, 1.0)
    }
}
