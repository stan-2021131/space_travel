pub mod mat;
pub mod transform;
pub mod vec;

pub use mat::Mat4;
pub use transform::Transform;
pub use vec::{Vec2, Vec3, Vec4};

/// Convierte grados sexagesimales a radianes.
#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    deg * std::f32::consts::PI / 180.0
}

/// Convierte radianes a grados sexagesimales.
#[inline]
pub fn rad_to_deg(rad: f32) -> f32 {
    rad * 180.0 / std::f32::consts::PI
}
