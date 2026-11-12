//! Módulo de matrices 4x4 y transformaciones geométricas.
//!
//! # Convenciones matemáticas obligatorias del proyecto:
//! - **Handedness (Regla de la mano derecha)**:
//!   - Eje +X: Derecha.
//!   - Eje +Y: Arriba.
//!   - Eje +Z: Hacia el observador (fuera de la pantalla).
//! - **Dirección frontal de la cámara**:
//!   - La cámara mira hacia el eje -Z (`forward = (0, 0, -1)`).
//! - **Convención de vectores**:
//!   - Vectores columna: $v = [x, y, z, w]^T$.
//!   - Transformación: $v' = M \cdot v$.
//! - **Orden de multiplicación de matrices**:
//!   - De derecha a izquierda: $v_{clip} = P \cdot V \cdot M \cdot v_{local}$.
//!   - Al componer matrices $A$ aplicada después de $B$: $M = A \cdot B$.
//! - **Rango de profundidad en NDC**:
//!   - $z_{ndc} \in [-1.0, 1.0]$.
//!   - El plano cercano ($z_{near}$) mapea a $z_{ndc} = -1.0$.
//!   - El plano lejano ($z_{far}$) mapea a $z_{ndc} = +1.0$.
//! - **Convención de devanado (Winding) de caras frontales**:
//!   - Sentido antihorario (Counter-Clockwise / CCW).

use super::vec::{Vec3, Vec4};
use std::ops::Mul;

/// Matriz de 4x4 representada internamente en orden columna-mayor (column-major)
/// como un arreglo plano `[f32; 16]`.
///
/// El elemento en la fila `r` y columna `c` se accede como `data[c * 4 + r]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub data: [f32; 16],
}

impl Mat4 {
    /// Matriz identidad de 4x4.
    pub const IDENTITY: Self = Self {
        data: [
            1.0, 0.0, 0.0, 0.0, // Columna 0
            0.0, 1.0, 0.0, 0.0, // Columna 1
            0.0, 0.0, 1.0, 0.0, // Columna 2
            0.0, 0.0, 0.0, 1.0, // Columna 3
        ],
    };

    /// Matriz de ceros.
    pub const ZERO: Self = Self { data: [0.0; 16] };

    #[inline]
    pub const fn from_cols(c0: Vec4, c1: Vec4, c2: Vec4, c3: Vec4) -> Self {
        Self {
            data: [
                c0.x, c0.y, c0.z, c0.w, c1.x, c1.y, c1.z, c1.w, c2.x, c2.y, c2.z, c2.w, c3.x, c3.y,
                c3.z, c3.w,
            ],
        }
    }

    #[inline]
    pub fn get(&self, row: usize, col: usize) -> f32 {
        self.data[col * 4 + row]
    }

    #[inline]
    pub fn set(&mut self, row: usize, col: usize, val: f32) {
        self.data[col * 4 + row] = val;
    }

    /// Matriz de traslación.
    pub fn translation(tx: f32, ty: f32, tz: f32) -> Self {
        let mut m = Self::IDENTITY;
        m.data[12] = tx;
        m.data[13] = ty;
        m.data[14] = tz;
        m
    }

    /// Matriz de traslación desde un Vec3.
    #[inline]
    pub fn from_translation(t: Vec3) -> Self {
        Self::translation(t.x, t.y, t.z)
    }

    /// Matriz de escala no uniforme o uniforme.
    pub fn scaling(sx: f32, sy: f32, sz: f32) -> Self {
        let mut m = Self::IDENTITY;
        m.data[0] = sx;
        m.data[5] = sy;
        m.data[10] = sz;
        m
    }

    #[inline]
    pub fn from_scale(s: Vec3) -> Self {
        Self::scaling(s.x, s.y, s.z)
    }

    #[inline]
    pub fn uniform_scaling(s: f32) -> Self {
        Self::scaling(s, s, s)
    }

    /// Rotación alrededor del eje X (en radianes, sentido antihorario mirando a lo largo de +X).
    pub fn rotation_x(rad: f32) -> Self {
        let c = rad.cos();
        let s = rad.sin();
        let mut m = Self::IDENTITY;
        m.data[5] = c;
        m.data[6] = s;
        m.data[9] = -s;
        m.data[10] = c;
        m
    }

    /// Rotación alrededor del eje Y (en radianes, sentido antihorario mirando a lo largo de +Y).
    pub fn rotation_y(rad: f32) -> Self {
        let c = rad.cos();
        let s = rad.sin();
        let mut m = Self::IDENTITY;
        m.data[0] = c;
        m.data[2] = -s;
        m.data[8] = s;
        m.data[10] = c;
        m
    }

    /// Rotación alrededor del eje Z (en radianes, sentido antihorario mirando a lo largo de +Z).
    pub fn rotation_z(rad: f32) -> Self {
        let c = rad.cos();
        let s = rad.sin();
        let mut m = Self::IDENTITY;
        m.data[0] = c;
        m.data[1] = s;
        m.data[4] = -s;
        m.data[5] = c;
        m
    }

    /// Rotación combinada de Euler en orden Y * X * Z (Yaw * Pitch * Roll).
    pub fn rotation_euler(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self::rotation_y(yaw) * Self::rotation_x(pitch) * Self::rotation_z(roll)
    }

    /// Matriz de vista Look-At para un sistema de mano derecha con la cámara mirando hacia -Z.
    ///
    /// - `eye`: Posición de la cámara en el mundo.
    /// - `target`: Punto al que mira la cámara.
    /// - `up`: Vector que indica la dirección "hacia arriba" del mundo.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        // En sistema mano derecha, forward mira hacia -Z, por lo que el eje Z de la cámara apunta hacia atrás:
        let forward = (target - eye).normalize();
        let z_axis = -forward; // Hacia atrás de la cámara
        let x_axis = up.cross(z_axis).normalize(); // Hacia la derecha
        let y_axis = z_axis.cross(x_axis); // Hacia arriba real de la cámara

        let mut m = Self::IDENTITY;
        // Fila 0
        m.data[0] = x_axis.x;
        m.data[4] = x_axis.y;
        m.data[8] = x_axis.z;
        m.data[12] = -x_axis.dot(eye);
        // Fila 1
        m.data[1] = y_axis.x;
        m.data[5] = y_axis.y;
        m.data[9] = y_axis.z;
        m.data[13] = -y_axis.dot(eye);
        // Fila 2
        m.data[2] = z_axis.x;
        m.data[6] = z_axis.y;
        m.data[10] = z_axis.z;
        m.data[14] = -z_axis.dot(eye);

        m
    }

    /// Matriz de proyección en perspectiva estándar para sistema de mano derecha.
    ///
    /// - `fov_y_rad`: Campo de visión vertical en radianes.
    /// - `aspect`: Relación de aspecto (ancho / alto).
    /// - `z_near`: Distancia al plano cercano (debe ser > 0).
    /// - `z_far`: Distancia al plano lejano (debe ser > z_near).
    ///
    /// Mapea el volumen de vista frustum a NDC con $x, y, z \in [-1, 1]$.
    /// Para puntos visibles ($z_{view} \in [-z_{far}, -z_{near}]$), se produce $w_{clip} = -z_{view} > 0$.
    pub fn perspective(fov_y_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        assert!(z_near > 0.0, "z_near debe ser mayor a 0");
        assert!(z_far > z_near, "z_far debe ser mayor a z_near");
        let tan_half_fov = (fov_y_rad * 0.5).tan();
        let inv_aspect_tan = 1.0 / (aspect * tan_half_fov);
        let inv_tan = 1.0 / tan_half_fov;
        let depth_range = z_near - z_far;

        let mut m = Self::ZERO;
        m.data[0] = inv_aspect_tan;
        m.data[5] = inv_tan;
        m.data[10] = (z_far + z_near) / depth_range;
        m.data[11] = -1.0;
        m.data[14] = (2.0 * z_far * z_near) / depth_range;
        m
    }

    /// Transforma un vector cuadridimensional homogéneo: $v' = M \cdot v$.
    #[inline]
    pub fn transform_vec4(&self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.data[0] * v.x + self.data[4] * v.y + self.data[8] * v.z + self.data[12] * v.w,
            self.data[1] * v.x + self.data[5] * v.y + self.data[9] * v.z + self.data[13] * v.w,
            self.data[2] * v.x + self.data[6] * v.y + self.data[10] * v.z + self.data[14] * v.w,
            self.data[3] * v.x + self.data[7] * v.y + self.data[11] * v.z + self.data[15] * v.w,
        )
    }

    /// Transforma un punto 3D asumiendo $w = 1.0$. No realiza división de perspectiva.
    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.transform_vec4(p.to_vec4(1.0)).to_vec3()
    }

    /// Transforma un vector de dirección 3D asumiendo $w = 0.0$ (ignora traslación).
    #[inline]
    pub fn transform_direction(&self, d: Vec3) -> Vec3 {
        self.transform_vec4(d.to_vec4(0.0)).to_vec3()
    }

    /// Traspuesta de la matriz 4x4.
    pub fn transpose(&self) -> Self {
        let mut res = Self::ZERO;
        for r in 0..4 {
            for c in 0..4 {
                res.data[c * 4 + r] = self.data[r * 4 + c];
            }
        }
        res
    }
}

impl Mul for Mat4 {
    type Output = Self;

    /// Multiplicación de matrices: $C = A \cdot B$.
    fn mul(self, rhs: Self) -> Self {
        let mut res = Self::ZERO;
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.data[k * 4 + row] * rhs.data[col * 4 + k];
                }
                res.data[col * 4 + row] = sum;
            }
        }
        res
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    #[inline]
    fn mul(self, rhs: Vec4) -> Vec4 {
        self.transform_vec4(rhs)
    }
}

impl Default for Mat4 {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}
