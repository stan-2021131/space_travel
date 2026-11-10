use super::{FragmentInput, FragmentShader};
use crate::buffer::Color;
use crate::math::Vec3;

const INV_U32_MAX: f32 = 1.0 / 4294967295.0;

/// Shader procedural en CPU para planetas de tipo terrestre o biosfera (verde con manchas azules).
///
/// Sintetiza de forma analítica continentes, masas oceánicas y casquetes polares mediante
/// ruido multifractal (fBm) continuo en coordenadas esféricas tridimensionales, eliminando
/// cualquier artefacto de facetas poligonales y rotando solidariamente con el astro.
pub struct EarthLikeShader {
    pub seed: u32,
    pub light_pos: Vec3,
    pub camera_pos: Vec3,
    pub ambient_intensity: f32,
    pub body_center: Vec3,
    pub rotation_rad: f32,
    pub axial_tilt_rad: f32,
    pub rot_cos: f32,
    pub rot_sin: f32,
    pub tilt_cos: f32,
    pub tilt_sin: f32,
}

impl EarthLikeShader {
    pub fn new(seed: u32, light_pos: Vec3, camera_pos: Vec3) -> Self {
        Self {
            seed,
            light_pos,
            camera_pos,
            ambient_intensity: 0.14,
            body_center: Vec3::zero(),
            rotation_rad: 0.0,
            axial_tilt_rad: 0.0,
            rot_cos: 1.0,
            rot_sin: 0.0,
            tilt_cos: 1.0,
            tilt_sin: 0.0,
        }
    }

    /// Actualiza la orientación y precalcula los factores trigonométricos una sola vez
    /// por cuadro antes del renderizado de la malla, evitando millones de llamadas en CPU.
    #[inline]
    pub fn set_orientation(&mut self, rotation_rad: f32, axial_tilt_rad: f32) {
        self.rotation_rad = rotation_rad;
        self.axial_tilt_rad = axial_tilt_rad;
        self.rot_cos = rotation_rad.cos();
        self.rot_sin = rotation_rad.sin();
        self.tilt_cos = (-axial_tilt_rad).cos();
        self.tilt_sin = (-axial_tilt_rad).sin();
    }

    /// Ruido de valor continuo tridimensional con interpolación cúbica (smoothstep).
    #[inline]
    fn value_noise(p: Vec3, seed: u32) -> f32 {
        let ix = p.x.floor() as i32;
        let iy = p.y.floor() as i32;
        let iz = p.z.floor() as i32;

        let fx = p.x - ix as f32;
        let fy = p.y - iy as f32;
        let fz = p.z - iz as f32;

        // Curva cúbica: 3t^2 - 2t^3
        let ux = fx * fx * (3.0 - 2.0 * fx);
        let uy = fy * fy * (3.0 - 2.0 * fy);
        let uz = fz * fz * (3.0 - 2.0 * fz);

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

    /// Ruido fractal multifrecuencia (fBm) de 3 capas deterministas.
    #[inline]
    fn fbm(p: Vec3, seed: u32) -> f32 {
        let n1 = Self::value_noise(p * 2.2, seed);
        let n2 = Self::value_noise(p * 4.8, seed.wrapping_add(101));
        let n3 = Self::value_noise(p * 10.5, seed.wrapping_add(202));
        0.55 * n1 + 0.30 * n2 + 0.15 * n3
    }
}

impl FragmentShader for EarthLikeShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color {
        // Dirección radial esférica continua desde el centro para eliminar facetas
        let offset = input.world_pos - self.body_center;
        let len_sq = offset.length_squared();
        let n = if len_sq > 1e-4 {
            offset * (1.0 / len_sq.sqrt())
        } else {
            input.world_normal.normalize()
        };

        // Rotación axial inversa y tilt precalculados para que el terreno gire solidariamente con el planeta
        let cos_r = self.rot_cos;
        let sin_r = self.rot_sin;
        let unrotated_y = Vec3::new(cos_r * n.x - sin_r * n.z, n.y, sin_r * n.x + cos_r * n.z);

        let local_n = if self.axial_tilt_rad.abs() > 1e-4 {
            let cos_t = self.tilt_cos;
            let sin_t = self.tilt_sin;
            Vec3::new(
                unrotated_y.x,
                cos_t * unrotated_y.y - sin_t * unrotated_y.z,
                sin_t * unrotated_y.y + cos_t * unrotated_y.z,
            )
        } else {
            unrotated_y
        };

        // Evaluación de altura / bioma mediante ruido fractal
        let elevation = Self::fbm(local_n, self.seed);

        // Umbral marítimo (nivel del mar): zonas con elevación < 0.44 son manchas azules (océanos y lagos)
        let sea_level = 0.44;
        let is_water = elevation < sea_level;

        let (mut r, mut g, mut b) = if is_water {
            // Océanos y mares azules: de azul abisal profundo a azul celeste costero
            let t = (elevation / sea_level).clamp(0.0, 1.0);
            let ocean_deep = (0.04, 0.12, 0.45);
            let ocean_shallow = (0.10, 0.42, 0.72);
            (
                ocean_deep.0 + (ocean_shallow.0 - ocean_deep.0) * t,
                ocean_deep.1 + (ocean_shallow.1 - ocean_deep.1) * t,
                ocean_deep.2 + (ocean_shallow.2 - ocean_deep.2) * t,
            )
        } else if elevation < 0.47 {
            // Costa y orilla de transición suave
            let t = ((elevation - sea_level) / (0.47 - sea_level)).clamp(0.0, 1.0);
            let beach = (0.35, 0.45, 0.22);
            let grass = (0.12, 0.52, 0.16);
            (
                beach.0 + (grass.0 - beach.0) * t,
                beach.1 + (grass.1 - beach.1) * t,
                beach.2 + (grass.2 - beach.2) * t,
            )
        } else if elevation < 0.72 {
            // Continentes verdes (praderas, colinas, valles verdes templados)
            let t = ((elevation - 0.47) / (0.72 - 0.47)).clamp(0.0, 1.0);
            let meadow = (0.10, 0.56, 0.14);
            let forest = (0.07, 0.42, 0.10);
            (
                meadow.0 + (forest.0 - meadow.0) * t,
                meadow.1 + (forest.1 - meadow.1) * t,
                meadow.2 + (forest.2 - meadow.2) * t,
            )
        } else {
            // Bosques densos y tierras altas
            let t = ((elevation - 0.72) / (1.0 - 0.72)).clamp(0.0, 1.0);
            let highland = (0.08, 0.38, 0.11);
            let mountain_forest = (0.14, 0.28, 0.12);
            (
                highland.0 + (mountain_forest.0 - highland.0) * t,
                highland.1 + (mountain_forest.1 - highland.1) * t,
                highland.2 + (mountain_forest.2 - highland.2) * t,
            )
        };

        // Casquetes polares de hielo y nieve en latitudes extremas
        let abs_lat = local_n.y.abs();
        if abs_lat > 0.85 {
            let polar_t = ((abs_lat - 0.85) / 0.10).clamp(0.0, 1.0);
            let smooth_polar = polar_t * polar_t * (3.0 - 2.0 * polar_t);
            let snow = (0.92, 0.95, 0.98);
            r = r + (snow.0 - r) * smooth_polar;
            g = g + (snow.1 - g) * smooth_polar;
            b = b + (snow.2 - b) * smooth_polar;
        }

        // Iluminación difusa con luz proveniente del Sol
        let l = (self.light_pos - input.world_pos).normalize();
        let n_dot_l = n.dot(l).max(0.0);

        let mut lighting = self.ambient_intensity + n_dot_l * (1.0 - self.ambient_intensity);

        // Brillo especular sutil en los océanos azules hacia la cámara
        if is_water && n_dot_l > 0.0 {
            let v = (self.camera_pos - input.world_pos).normalize();
            let h = (l + v).normalize();
            let n_dot_h = n.dot(h).max(0.0);
            let s2 = n_dot_h * n_dot_h;
            let s4 = s2 * s2;
            let s8 = s4 * s4;
            let specular = (s8 * s8) * 0.35;
            lighting += specular;
        }

        let kd = input.material.kd;
        Color::from_f32(
            (r * kd.x * lighting).min(1.0),
            (g * kd.y * lighting).min(1.0),
            (b * kd.z * lighting).min(1.0),
            1.0,
        )
    }
}
