use crate::buffer::Color;
use crate::material::Material;
use crate::math::{Vec2, Vec3};

/// Datos interpolados que recibe el shader de fragmentos en cada píxel evaluado.
#[derive(Debug, Clone)]
pub struct FragmentInput<'a> {
    pub screen_x: usize,
    pub screen_y: usize,
    pub depth_ndc: f32,
    pub world_pos: Vec3,
    pub world_normal: Vec3,
    pub uv: Vec2,
    pub barycentric: Vec3,
    pub material: &'a Material,
}

/// Modo de renderizado para alternar entre el sombreado visual y visualizaciones diagnósticas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderMode {
    /// Sombreado estándar de la escena (difuso/texturado según el material).
    Normal,
    /// Modo de depuración que visualiza el búfer de profundidad linealizado en escala de grises.
    DepthDebug,
    /// Modo de depuración que visualiza las normales en espacio de mundo mapeadas a colores RGB.
    NormalsDebug,
}

impl ShaderMode {
    /// Alterna cíclicamente el modo de sombreado (Normal -> DepthDebug -> NormalsDebug -> Normal).
    pub fn cycle(self) -> Self {
        match self {
            ShaderMode::Normal => ShaderMode::DepthDebug,
            ShaderMode::DepthDebug => ShaderMode::NormalsDebug,
            ShaderMode::NormalsDebug => ShaderMode::Normal,
        }
    }
}

/// Trait para shaders de fragmentos ejecutados íntegramente en CPU.
pub trait FragmentShader {
    fn shade(&self, input: &FragmentInput<'_>) -> Color;
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
        // Dirección de la luz: desde la superficie hacia el sol
        let l = (self.light_pos - input.world_pos).normalize();
        let n_dot_l = n.dot(l).max(0.0);
        let lighting =
            (self.ambient_intensity + n_dot_l * (1.0 - self.ambient_intensity)).clamp(0.0, 1.0);

        // Muestreo de textura o color blanco de textura si no existe
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

/// Shader emisivo para cuerpos autoiluminados como el sol.
///
/// No se oscurece con fuentes direccionales externas y emite un color constante basado en Kd.
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

/// Shader de iluminación difusa básica en CPU con luz direccional y ambiental (utilizado para la nave).
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
