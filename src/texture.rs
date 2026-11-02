use crate::buffer::Color;
use std::path::Path;

/// Textura 2D en memoria RAM para muestreo en CPU.
#[derive(Debug, Clone)]
pub struct CpuTexture {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Color>,
}

impl CpuTexture {
    /// Crea una nueva textura con dimensiones dadas y píxeles en formato Color RGBA.
    pub fn new(width: usize, height: usize, pixels: Vec<Color>) -> Self {
        assert_eq!(
            pixels.len(),
            width * height,
            "La cantidad de píxeles debe coincidir exactamente con width * height"
        );
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Crea una textura monocromática de un solo color sólido (útil para pruebas o fallbacks).
    pub fn solid(width: usize, height: usize, color: Color) -> Self {
        Self {
            width,
            height,
            pixels: vec![color; width * height],
        }
    }

    /// Carga una textura desde un archivo PNG o JPG en disco usando el crate `image`.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let img = image::open(path)
            .map_err(|e| format!("Error al abrir imagen '{:?}': {}", path, e))?
            .to_rgba8();

        let width = img.width() as usize;
        let height = img.height() as usize;
        let raw_bytes = img.into_raw();

        let mut pixels = Vec::with_capacity(width * height);
        for chunk in raw_bytes.chunks_exact(4) {
            pixels.push(Color::new(chunk[0], chunk[1], chunk[2], chunk[3]));
        }

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Muestra un texel dadas coordenadas UV normalizadas.
    ///
    /// # Convenciones y Reglas de Muestreo:
    /// - **Coordenada U**: Utiliza **repetición / wrap fraccionario** ($u - \lfloor u \rfloor$)
    ///   para resolver suavemente la costura horizontal de envolturas cilíndricas o esferoidales.
    /// - **Coordenada V**: Utiliza **clamp / saturación estricta** en $[0.0, 1.0]$. No se repite,
    ///   impidiendo que los polos del planeta intercambien la parte superior e inferior.
    /// - **Inversión Vertical de V**: En formatos OBJ/Blender, $V = 0.0$ corresponde a la parte inferior
    ///   y $V = 1.0$ a la superior. En la memoria matricial de la imagen, la fila 0 corresponde al
    ///   borde superior; por lo tanto, se aplica la inversión: $v_{img} = 1.0 - V_{clamped}$.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Color {
        if self.width == 0 || self.height == 0 || self.pixels.is_empty() {
            return Color::WHITE;
        }

        // 1. Repetición (wrap) en el eje horizontal U
        let u_wrapped = u - u.floor();

        // 2. Saturación (clamp) en el eje vertical V
        let v_clamped = v.clamp(0.0, 1.0);

        // 3. Inversión vertical para alinear convención OBJ (origen abajo) con memoria (origen arriba)
        let v_flipped = 1.0 - v_clamped;

        let x = ((u_wrapped * (self.width as f32 - 1e-4)).floor() as usize).min(self.width - 1);
        let y = ((v_flipped * (self.height as f32 - 1e-4)).floor() as usize).min(self.height - 1);

        self.pixels[y * self.width + x]
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            Color::BLACK
        }
    }
}
