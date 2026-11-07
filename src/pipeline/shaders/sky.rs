use crate::buffer::{Color, Framebuffer};
use crate::camera::Camera;

const INV_U32_MAX: f32 = 1.0 / 4294967295.0;

/// Shader procedural en CPU para el fondo cósmico del universo.
///
/// Evalúa en tiempo real estrellas puntuales deterministas y nebulosas suaves
/// según la dirección tridimensional del rayo de la cámara en el espacio de mundo,
/// garantizando rotación 3D realista con cero distorsión y rendimiento de 60 FPS.
pub struct SkyShader {
    pub seed: u32,
}

impl SkyShader {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    /// Hash pseudoaleatorio determinista en el espacio de celda angular tridimensional.
    #[inline]
    fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
        let mut h = (x as u32).wrapping_mul(374761393)
            ^ (y as u32).wrapping_mul(668265263)
            ^ (z as u32).wrapping_mul(951214213)
            ^ seed.wrapping_mul(1103515245);
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h as f32) * INV_U32_MAX
    }

    /// Renderiza el fondo cósmico sobre el framebuffer completo interpolando rayos directores de cámara.
    pub fn render(&self, framebuffer: &mut Framebuffer, camera: &Camera) {
        let width = framebuffer.width;
        let height = framebuffer.height;
        if width == 0 || height == 0 {
            return;
        }

        // Vectores directores ortonormales de la cámara en el espacio de mundo
        let forward = (camera.target - camera.eye).normalize();
        let right = forward.cross(camera.up).normalize();
        let up = right.cross(forward).normalize();

        let half_height = (camera.fov_y_rad * 0.5).tan();
        let half_width = half_height * camera.aspect;

        // Cuatro rayos directores correspondientes a las esquinas de la pantalla
        let tl = forward - right * half_width + up * half_height;
        let tr = forward + right * half_width + up * half_height;
        let bl = forward - right * half_width - up * half_height;
        let br = forward + right * half_width - up * half_height;

        let inv_h = 1.0 / height as f32;
        let inv_w = 1.0 / width as f32;

        // Barrido por scanlines con interpolación lineal de rayos
        for y in 0..height {
            let v = (y as f32 + 0.5) * inv_h;
            let left_ray = tl + (bl - tl) * v;
            let right_ray = tr + (br - tr) * v;
            let step_ray = (right_ray - left_ray) * inv_w;

            let mut ray = left_ray;

            for x in 0..width {
                let dir = ray.normalize();

                // 1. Detección de estrella en celda angular discreta
                let cx = (dir.x * 160.0).floor() as i32;
                let cy = (dir.y * 160.0).floor() as i32;
                let cz = (dir.z * 160.0).floor() as i32;
                let h = Self::hash3(cx, cy, cz, self.seed);

                let pixel_color = if h > 0.9986 {
                    // Píxel estrella con ligera variación de brillo estelar templado
                    let brightness = 150 + ((h - 0.9986) / 0.0014 * 60.0) as u8;
                    Color::new(
                        brightness,
                        brightness,
                        (brightness as f32 * 1.05).min(255.0) as u8,
                        255,
                    )
                } else {
                    // 2. Fondo cósmico profundo y oscuro con sutiles velos de nebulosas
                    // Velo violeta/púrpura muy tenue
                    let s1 = (dir.x * 2.2 + dir.y * 1.8).sin() * 0.5 + 0.5;
                    let neb1 = s1 * s1 * s1;
                    // Velo cian/turquesa suave
                    let s2 = (dir.z * 2.0 - dir.x * 1.6).cos() * 0.5 + 0.5;
                    let neb2 = s2 * s2 * s2;

                    let base_r = 4.0 + neb1 * 7.0;
                    let base_g = 5.0 + neb1 * 2.0 + neb2 * 6.0;
                    let base_b = 9.0 + neb1 * 9.0 + neb2 * 8.0;

                    Color::new(
                        base_r.min(255.0) as u8,
                        base_g.min(255.0) as u8,
                        base_b.min(255.0) as u8,
                        255,
                    )
                };

                framebuffer.set_pixel(x, y, pixel_color);
                ray += step_ray;
            }
        }
    }
}
