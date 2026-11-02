use crate::math::Vec3;
use crate::texture::CpuTexture;
use std::sync::Arc;

/// Representa las propiedades ópticas y de superficie de un material para el renderizador por software en CPU.
#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    /// Color reflectivo difuso Kd en formato [0.0, 1.0].
    pub kd: Vec3,
    /// Textura difusa opcional cargada en RAM.
    pub texture: Option<Arc<CpuTexture>>,
}

impl Material {
    pub fn new(name: impl Into<String>, kd: Vec3, texture: Option<Arc<CpuTexture>>) -> Self {
        Self {
            name: name.into(),
            kd,
            texture,
        }
    }

    /// Material por defecto documentado para mallas que carecen de archivo MTL o definiciones de material.
    pub fn default_diffuse() -> Self {
        Self {
            name: "DefaultMaterial".to_string(),
            kd: Vec3::new(0.85, 0.85, 0.90),
            texture: None,
        }
    }

    pub fn textured(name: impl Into<String>, kd: Vec3, texture: Arc<CpuTexture>) -> Self {
        Self {
            name: name.into(),
            kd,
            texture: Some(texture),
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::default_diffuse()
    }
}
