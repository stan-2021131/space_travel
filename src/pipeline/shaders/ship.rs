use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

/// Shader especializado para el fuselaje metálico de la nave espacial en CPU.
///
/// Implementa:
/// 1. Iluminación física real proveniente de la posición dinámica del Sol (`light_pos`).
/// 2. Destello especular metálico Blinn-Phong según la dirección del observador (`camera_pos`).
/// 3. Emisión base ambiental calibrada para que la nave sea siempre visible contra el fondo espacial.
/// 4. Acabado de titanio/acero pulido que resalta la geometría de las alas y cabina.
pub struct ShipShader {
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub ambient_intensity: f32,
    pub specular_power: f32,
}

impl ShipShader {
    pub fn new(light_pos: Vec3, camera_pos: Vec3) -> Self {
        Self {
            light_pos,
            camera_pos,
            ambient_intensity: 0.16,
            specular_power: 32.0,
        }
    }
}

impl FragmentShader for ShipShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        let n = input.world_normal.normalize();

        // 1. Vector director incidente desde la posición dinámica del Sol
        let to_light = self.light_pos - input.world_pos;
        let l = if to_light.length_squared() > 1e-4 {
            to_light.normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        // 2. Vector director hacia la cámara (observador)
        let to_cam = self.camera_pos - input.world_pos;
        let v = if to_cam.length_squared() > 1e-4 {
            to_cam.normalize()
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };

        // 3. Atenuación difusa lambertiana frente al Sol
        let n_dot_l = n.dot(l).max(0.0);

        // 4. Reflejo especular metálico Blinn-Phong
        let h = (l + v).normalize();
        let n_dot_h = n.dot(h).max(0.0);
        let specular = if n_dot_l > 0.0 {
            n_dot_h.powf(self.specular_power)
        } else {
            0.0
        };

        // 5. Color base del material con sutil tinte metálico de titanio
        let kd = input.material.kd;
        let metal_r = kd.x * 0.94;
        let metal_g = kd.y * 0.97;
        let metal_b = kd.z * 1.05;

        // 6. Síntesis de color:
        // Ambiente base (conserva silueta visible en penumbra) + Difusa solar + Especular metálico
        let diffuse_weight = 0.84;
        let lit_r = metal_r * (self.ambient_intensity + n_dot_l * diffuse_weight) + specular * 0.45;
        let lit_g = metal_g * (self.ambient_intensity + n_dot_l * diffuse_weight) + specular * 0.48;
        let lit_b = metal_b * (self.ambient_intensity + n_dot_l * diffuse_weight) + specular * 0.55;

        Color::from_f32(lit_r.min(1.0), lit_g.min(1.0), lit_b.min(1.0), 1.0)
    }
}
