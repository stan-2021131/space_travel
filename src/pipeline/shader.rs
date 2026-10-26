use crate::buffer::Color;
use crate::math::{Vec2, Vec3};

/// Datos interpolados que recibe el shader de fragmentos en cada píxel evaluado.
#[derive(Debug, Clone, Copy)]
pub struct FragmentInput {
    pub screen_x: usize,
    pub screen_y: usize,
    pub depth_ndc: f32,
    pub world_pos: Vec3,
    pub world_normal: Vec3,
    pub uv: Vec2,
    pub barycentric: Vec3,
}

/// Modo de renderizado para alternar entre el sombreado visual y visualizaciones diagnósticas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderMode {
    /// Sombreado estándar difuso con luz direccional y ambiental.
    Diffuse,
    /// Modo de depuración que visualiza el búfer de profundidad en escala de grises.
    DepthDebug,
    /// Modo de depuración que visualiza las normales en espacio de mundo mapeadas a colores RGB.
    NormalsDebug,
}

/// Trait para shaders de fragmentos ejecutados íntegramente en CPU.
pub trait FragmentShader {
    fn shade(&self, input: &FragmentInput) -> Color;
}

/// Shader de iluminación difusa básica en CPU con luz direccional y ambiental.
pub struct DiffuseShader {
    pub light_dir: Vec3,
    pub base_color: Color,
    pub ambient_intensity: f32,
}

impl DiffuseShader {
    pub fn new() -> Self {
        Self {
            // Luz que viene desde arriba y adelante hacia abajo
            light_dir: Vec3::new(0.5, 0.8, 0.3).normalize(),
            // Tono base de nave espacial (gris metálico cálido)
            base_color: Color::new(210, 215, 225, 255),
            ambient_intensity: 0.20,
        }
    }
}

impl Default for DiffuseShader {
    fn default() -> Self {
        Self::new()
    }
}

impl FragmentShader for DiffuseShader {
    fn shade(&self, input: &FragmentInput) -> Color {
        let n = input.world_normal.normalize();
        let n_dot_l = n.dot(self.light_dir).max(0.0);
        let intensity =
            (self.ambient_intensity + n_dot_l * (1.0 - self.ambient_intensity)).clamp(0.0, 1.0);

        Color::from_f32(
            (self.base_color.r as f32 / 255.0) * intensity,
            (self.base_color.g as f32 / 255.0) * intensity,
            (self.base_color.b as f32 / 255.0) * intensity,
            1.0,
        )
    }
}

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
        // Near y far del frustum de la cámara (0.1 y 100.0).
        // La nave se sitúa en el origen frente a la cámara en (0, 1.2, 4.5), por lo que
        // sus partes cercanas y lejanas oscilan entre 2.5 y 6.5 unidades de distancia lineal.
        Self::new(0.1, 100.0, 2.5, 6.5)
    }
}

impl FragmentShader for DepthDebugShader {
    fn shade(&self, input: &FragmentInput) -> Color {
        // 1. Linealización de profundidad NDC
        let z_linear = self.linearize_depth(input.depth_ndc);

        // 2. Normalización en el rango visible de la nave
        let range = self.display_max - self.display_min;
        let normalized = if range.abs() > 1e-6 {
            ((z_linear - self.display_min) / range).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // 3. Rampa de escala de grises: cercano brillante (1.0), lejano oscuro (0.0)
        let brightness = 1.0 - normalized;
        Color::from_f32(brightness, brightness, brightness, 1.0)
    }
}

/// Shader de diagnóstico para visualizar normales de superficie mapeadas a colores RGB.
pub struct NormalsDebugShader;

impl FragmentShader for NormalsDebugShader {
    fn shade(&self, input: &FragmentInput) -> Color {
        let n = input.world_normal.normalize();
        // Mapear [-1, 1] a [0, 1]
        let r = n.x * 0.5 + 0.5;
        let g = n.y * 0.5 + 0.5;
        let b = n.z * 0.5 + 0.5;
        Color::from_f32(r, g, b, 1.0)
    }
}
