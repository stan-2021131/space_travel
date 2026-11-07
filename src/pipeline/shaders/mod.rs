pub mod debug;
pub mod diffuse;
pub mod normal_mapped;
pub mod plasma;
pub mod procedural_rock;
pub mod ship;
pub mod sky;
pub mod toon;

use crate::buffer::Color;
use crate::material::Material;
use crate::math::{Vec2, Vec3};

pub use debug::{DepthDebugShader, NormalsDebugShader};
pub use diffuse::{DiffuseShader, EmissiveShader, TexturedDiffuseShader};
pub use normal_mapped::NormalMappedShader;
pub use plasma::PlasmaShader;
pub use procedural_rock::ProceduralRockShader;
pub use ship::ShipShader;
pub use sky::SkyShader;
pub use toon::ToonShader;

/// Datos interpolados que recibe el shader de fragmentos en cada píxel evaluado en CPU.
#[derive(Debug, Clone)]
pub struct FragmentInput<'a> {
    pub screen_x: usize,
    pub screen_y: usize,
    pub depth_ndc: f32,
    pub world_pos: Vec3,
    pub world_normal: Vec3,
    pub world_tangent: Vec3,
    pub uv: Vec2,
    pub barycentric: Vec3,
    pub material: &'a Material,
}

/// Modo de renderizado para alternar entre el sombreado visual estándar, estilo anime y modos diagnósticos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderMode {
    /// Sombreado estándar de la escena (plasma solar, procedural/texturas según material).
    Normal,
    /// Sombreado estilizado Toon / Anime universal cuantizado en 3 bandas con silueta.
    Toon,
    /// Modo de depuración que visualiza el búfer de profundidad linealizado en escala de grises.
    DepthDebug,
    /// Modo de depuración que visualiza las normales en espacio de mundo mapeadas a colores RGB.
    NormalsDebug,
}

impl ShaderMode {
    /// Alterna cíclicamente el modo de sombreado (Normal -> Toon -> DepthDebug -> NormalsDebug -> Normal).
    pub fn cycle(self) -> Self {
        match self {
            ShaderMode::Normal => ShaderMode::Toon,
            ShaderMode::Toon => ShaderMode::DepthDebug,
            ShaderMode::DepthDebug => ShaderMode::NormalsDebug,
            ShaderMode::NormalsDebug => ShaderMode::Normal,
        }
    }
}

/// Trait para shaders de fragmentos ejecutados íntegramente en CPU.
pub trait FragmentShader: Sync {
    fn shade(&self, input: &FragmentInput<'_>) -> Color;
}
