use crate::config::CameraConfig;
use crate::math::{Mat4, Vec3, deg_to_rad};

/// Cámara en el espacio 3D para la generación de matrices de vista y proyección.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_rad: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        fov_deg: f32,
        aspect: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up: up.normalize(),
            fov_y_rad: deg_to_rad(fov_deg),
            aspect,
            z_near,
            z_far,
        }
    }

    /// Actualiza la posición y objetivo de la cámara de persecución suavemente según la nave.
    pub fn update_chase(
        &mut self,
        ship_pos: Vec3,
        ship_forward: Vec3,
        ship_up: Vec3,
        config: &CameraConfig,
        dt: f32,
    ) {
        // Posición ideal: detrás y arriba de la nave
        let ideal_eye =
            ship_pos - ship_forward * config.distance_behind + ship_up * config.height_above;
        // Punto de mira ideal: adelantado en la dirección hacia donde apunta la nave
        let ideal_target = ship_pos + ship_forward * config.look_ahead;

        // Suavizado temporal exponencial independiente del framerate
        let pos_blend = 1.0 - (-config.position_smoothing * dt).exp();
        let rot_blend = 1.0 - (-config.rotation_smoothing * dt).exp();

        self.eye += (ideal_eye - self.eye) * pos_blend;
        self.target += (ideal_target - self.target) * rot_blend;
        self.up = ship_up.normalize();
    }

    /// Calcula la matriz de vista (View Matrix) mediante Look-At.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye, self.target, self.up)
    }

    /// Calcula la matriz de proyección en perspectiva estándar (OpenGL-style NDC [-1, 1]).
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov_y_rad, self.aspect, self.z_near, self.z_far)
    }

    /// Calcula la matriz combinada de Vista-Proyección: $VP = P \cdot V$.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Extrae el frustum de visión (6 planos) en espacio de mundo correspondiente a la cámara actual.
    pub fn frustum(&self) -> Frustum {
        Frustum::from_view_projection(&self.view_projection_matrix())
    }
}

/// Plano tridimensional orientado en el espacio definido por normal unitaria y distancia signada.
///
/// Ecuación del plano: $\vec{n} \cdot \vec{p} + d = 0$.
/// Un punto $\vec{p}$ se encuentra en el semiespacio positivo (interior visible) si $\vec{n} \cdot \vec{p} + d \ge 0$.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub distance: f32,
}

impl Plane {
    /// Construye un plano normalizado a partir de los coeficientes algebraicos $(a, b, c, d)$.
    pub fn from_coefficients(a: f32, b: f32, c: f32, d: f32) -> Self {
        let length = (a * a + b * b + c * c).sqrt();
        if length > 1e-8 {
            Self {
                normal: Vec3::new(a / length, b / length, c / length),
                distance: d / length,
            }
        } else {
            Self {
                normal: Vec3::new(0.0, 1.0, 0.0),
                distance: 0.0,
            }
        }
    }

    /// Distancia signada desde un punto 3D al plano.
    ///
    /// Positiva si está en el semiespacio interior, negativa si está en el exterior.
    #[inline]
    pub fn signed_distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

/// Frustum de visión de la cámara compuesto por 6 planos delimitadores en espacio de mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extrae los 6 planos del frustum a partir de la matriz de Vista-Proyección ($VP = P \cdot V$)
    /// utilizando el método de Gribb-Hartmann para el espacio NDC $[-1, 1]$.
    pub fn from_view_projection(vp: &Mat4) -> Self {
        // Filas de la matriz VP
        let r0 = [vp.get(0, 0), vp.get(0, 1), vp.get(0, 2), vp.get(0, 3)];
        let r1 = [vp.get(1, 0), vp.get(1, 1), vp.get(1, 2), vp.get(1, 3)];
        let r2 = [vp.get(2, 0), vp.get(2, 1), vp.get(2, 2), vp.get(2, 3)];
        let r3 = [vp.get(3, 0), vp.get(3, 1), vp.get(3, 2), vp.get(3, 3)];

        // 1. Izquierdo (Left): r3 + r0
        let left =
            Plane::from_coefficients(r3[0] + r0[0], r3[1] + r0[1], r3[2] + r0[2], r3[3] + r0[3]);

        // 2. Derecho (Right): r3 - r0
        let right =
            Plane::from_coefficients(r3[0] - r0[0], r3[1] - r0[1], r3[2] - r0[2], r3[3] - r0[3]);

        // 3. Inferior (Bottom): r3 + r1
        let bottom =
            Plane::from_coefficients(r3[0] + r1[0], r3[1] + r1[1], r3[2] + r1[2], r3[3] + r1[3]);

        // 4. Superior (Top): r3 - r1
        let top =
            Plane::from_coefficients(r3[0] - r1[0], r3[1] - r1[1], r3[2] - r1[2], r3[3] - r1[3]);

        // 5. Cercano (Near): r3 + r2 (en NDC [-1, 1])
        let near =
            Plane::from_coefficients(r3[0] + r2[0], r3[1] + r2[1], r3[2] + r2[2], r3[3] + r2[3]);

        // 6. Lejano (Far): r3 - r2 (en NDC [-1, 1])
        let far =
            Plane::from_coefficients(r3[0] - r2[0], r3[1] - r2[1], r3[2] - r2[2], r3[3] - r2[3]);

        Self {
            planes: [left, right, bottom, top, near, far],
        }
    }

    /// Evalúa en tiempo constante $O(1)$ si una esfera delimitadora (centro y radio)
    /// interseca o se encuentra dentro del frustum de visión.
    ///
    /// Retorna `false` de inmediato si la esfera está completamente detrás de cualquiera de los 6 planos.
    #[inline]
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.signed_distance(center) < -radius {
                return false;
            }
        }
        true
    }
}
