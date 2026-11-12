use super::mat::Mat4;
use super::vec::Vec3;

/// Representa la transformación geométrica de una entidad en el espacio 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3, // Ángulos de Euler en radianes: x = pitch, y = yaw, z = roll
    pub scale: Vec3,
}

impl Transform {
    pub const fn new(position: Vec3, rotation: Vec3, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            rotation: Vec3::zero(),
            scale: Vec3::one(),
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        let t = Mat4::from_translation(self.position);
        let r = Mat4::rotation_euler(self.rotation.y, self.rotation.x, self.rotation.z);
        let s = Mat4::from_scale(self.scale);
        // T * R * S
        t * r * s
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Vec3::zero(),
            scale: Vec3::one(),
        }
    }
}
