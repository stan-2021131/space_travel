use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

const INV_U32_MAX: f32 = 1.0 / 4294967295.0;

/// Shader procedural para superficies planetarias rocosas sin necesidad de texturas de imagen.
///
/// Sintetiza analíticamente la geología mediante ruido determinista tridimensional evaluado
/// sobre coordenadas esféricas continuas (eliminando facetas poligonales). Genera cráteres
/// de impacto con crestas elevadas, mares de basalto oscuro, vetas minerales y grano fino.
pub struct ProceduralRockShader {
    pub seed: u32,
    pub light_pos: Vec3,
    pub ambient_intensity: f32,
    pub body_center: Vec3,
    pub rotation_rad: f32,
}

impl ProceduralRockShader {
    pub fn new(seed: u32, light_pos: Vec3) -> Self {
        Self {
            seed,
            light_pos,
            ambient_intensity: 0.12,
            body_center: Vec3::zero(),
            rotation_rad: 0.0,
        }
    }

    /// Hash pseudoaleatorio determinista en el espacio de celda entera tridimensional.
    #[inline(always)]
    fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
        let mut h = (x as u32).wrapping_mul(374761393)
            ^ (y as u32).wrapping_mul(668265263)
            ^ (z as u32).wrapping_mul(951214213)
            ^ seed.wrapping_mul(1103515245);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h as f32) * INV_U32_MAX
    }

    /// Ruido de valor continuo tridimensional con interpolación cúbica (smoothstep).
    /// Optimizado: precomputa factores de coordenadas por eje y reutiliza componentes enteros.
    #[inline]
    fn value_noise(p: Vec3, seed: u32) -> f32 {
        let ix = p.x.floor() as i32;
        let iy = p.y.floor() as i32;
        let iz = p.z.floor() as i32;

        let fx = p.x - ix as f32;
        let fy = p.y - iy as f32;
        let fz = p.z - iz as f32;

        // Curva cúbica de interpolación: 3t^2 - 2t^3
        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uy = fy * fy * (3.0 - 2.0 * fy);
        let uz = fz * fz * (3.0 - 2.0 * fz);

        // Precomputamos las multiplicaciones de los ejes por separado para evitar recalcularlas en las 8 esquinas
        let hx0 = (ix as u32).wrapping_mul(374761393);
        let hx1 = ((ix + 1) as u32).wrapping_mul(374761393);
        let hy0 = (iy as u32).wrapping_mul(668265263);
        let hy1 = ((iy + 1) as u32).wrapping_mul(668265263);
        let hz0 = (iz as u32).wrapping_mul(951214213);
        let hz1 = ((iz + 1) as u32).wrapping_mul(951214213);
        let hs = seed.wrapping_mul(1103515245);

        #[inline(always)]
        fn mix_hash(h_in: u32) -> f32 {
            let h = (h_in ^ (h_in >> 13)).wrapping_mul(1274126177);
            (h as f32) * INV_U32_MAX
        }

        let c000 = mix_hash(hx0 ^ hy0 ^ hz0 ^ hs);
        let c100 = mix_hash(hx1 ^ hy0 ^ hz0 ^ hs);
        let c010 = mix_hash(hx0 ^ hy1 ^ hz0 ^ hs);
        let c110 = mix_hash(hx1 ^ hy1 ^ hz0 ^ hs);
        let c001 = mix_hash(hx0 ^ hy0 ^ hz1 ^ hs);
        let c101 = mix_hash(hx1 ^ hy0 ^ hz1 ^ hs);
        let c011 = mix_hash(hx0 ^ hy1 ^ hz1 ^ hs);
        let c111 = mix_hash(hx1 ^ hy1 ^ hz1 ^ hs);

        let x00 = c000 + ux * (c100 - c000);
        let x10 = c010 + ux * (c110 - c010);
        let x01 = c001 + ux * (c101 - c001);
        let x11 = c011 + ux * (c111 - c011);

        let y0 = x00 + uy * (x10 - x00);
        let y1 = x01 + uy * (x11 - x01);

        y0 + uz * (y1 - y0)
    }
}

impl FragmentShader for ProceduralRockShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        // Dirección radial esférica continua desde el centro del planeta para eliminar facetas
        let offset = input.world_pos - self.body_center;
        let len_sq = offset.length_squared();
        let n = if len_sq > 1e-4 {
            offset * (1.0 / len_sq.sqrt())
        } else {
            input.world_normal.normalize()
        };

        // Rotación axial inversa para que las coordenadas de ruido giren solidariamente con el planeta
        let cos_r = self.rotation_rad.cos();
        let sin_r = self.rotation_rad.sin();
        let local_n = Vec3::new(cos_r * n.x - sin_r * n.z, n.y, sin_r * n.x + cos_r * n.z);

        // 1. Manchas continentales y mares basálticos oscuros (baja frecuencia)
        let maria = Self::value_noise(local_n * 2.8, self.seed);

        // 2. Anillos y crestas de cráteres de impacto (ridged noise a media frecuencia)
        let crater_raw = Self::value_noise(local_n * 7.5, self.seed.wrapping_add(10));
        let rim_base = 1.0 - (crater_raw - 0.5).abs() * 2.0;
        let crater_rim = rim_base * rim_base; // Crestas afiladas sin llamar a powi(2)

        // 3. Micro-aspereza y granularidad de grava/roca (alta frecuencia)
        // Al ser grano fino (escala 26.0), una celda de hash 3D aporta la dispersión granular sin requerir
        // el costo de una interpolación cúbica trilineal completa de 8 esquinas.
        let gp = local_n * 26.0;
        let grain = Self::hash3(
            gp.x.floor() as i32,
            gp.y.floor() as i32,
            gp.z.floor() as i32,
            self.seed.wrapping_add(20),
        );

        // Combinación estratigráfica
        let rock_val = (0.50 * maria + 0.35 * crater_rim + 0.15 * grain).clamp(0.0, 1.0);

        // Paleta geológica mineral de alto contraste (con factores inversos precalculados):
        // - [0.00 .. 0.40]: Basalto volcánico y cuencas oscuras (factor 1.0 / 0.40 = 2.5)
        // - [0.40 .. 0.72]: Arenisca ocre/terracota marciana (factor 1.0 / 0.32 = 3.125)
        // - [0.72 .. 1.00]: Crestas de impacto claras, cuarzo y regolito blanquecino (factor 1.0 / 0.28 = 3.5714285)
        let (r, g, b) = if rock_val < 0.40 {
            let t = rock_val * 2.5;
            let r = 0.12 + 0.16 * t;
            let g = 0.10 + 0.10 * t;
            let b = 0.09 + 0.08 * t;
            (r, g, b)
        } else if rock_val < 0.72 {
            let t = (rock_val - 0.40) * 3.125;
            let r = 0.28 + 0.34 * t;
            let g = 0.20 + 0.14 * t;
            let b = 0.17 + 0.06 * t;
            (r, g, b)
        } else {
            let t = (rock_val - 0.72) * 3.5714285;
            let r = 0.62 + 0.28 * t;
            let g = 0.48 + 0.36 * t;
            let b = 0.35 + 0.37 * t;
            (r, g, b)
        };

        // Iluminación difusa con luz proveniente del Sol
        let l = (self.light_pos - input.world_pos).normalize();
        let n_dot_l = n.dot(l).max(0.0);

        // Modulación sutil de iluminación por relieve (las crestas reciben más luz)
        let relief_mult = 0.85 + 0.30 * crater_rim;
        let lighting = (self.ambient_intensity
            + n_dot_l * (1.0 - self.ambient_intensity) * relief_mult)
            .clamp(0.0, 1.0);

        let kd = input.material.kd;
        Color::from_f32(
            (r * kd.x * lighting).min(1.0),
            (g * kd.y * lighting).min(1.0),
            (b * kd.z * lighting).min(1.0),
            1.0,
        )
    }
}
