/// Representación de un color RGBA con componentes de 8 bits (0-255).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const RED: Self = Self::new(255, 0, 0, 255);
    pub const GREEN: Self = Self::new(0, 255, 0, 255);
    pub const BLUE: Self = Self::new(0, 0, 255, 255);
    pub const GRAY: Self = Self::new(128, 128, 128, 255);
    pub const DARK_GRAY: Self = Self::new(30, 30, 35, 255);

    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline]
    pub fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: (r.clamp(0.0, 1.0) * 255.0) as u8,
            g: (g.clamp(0.0, 1.0) * 255.0) as u8,
            b: (b.clamp(0.0, 1.0) * 255.0) as u8,
            a: (a.clamp(0.0, 1.0) * 255.0) as u8,
        }
    }

    /// Empaqueta el color en un entero de 32 bits en formato RGBA (little endian en memoria: R, G, B, A).
    #[inline]
    pub fn to_u32(self) -> u32 {
        u32::from_le_bytes([self.r, self.g, self.b, self.a])
    }
}

/// Framebuffer de color administrado íntegramente por el software renderer en CPU.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Color>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::BLACK; width * height],
        }
    }

    #[inline]
    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            Color::BLACK
        }
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        let ptr = self.pixels.as_ptr() as *const u8;
        let len = self.pixels.len() * std::mem::size_of::<Color>();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

/// Buffer de profundidad (Depth Buffer / Z-Buffer) en CPU.
#[derive(Debug, Clone)]
pub struct DepthBuffer {
    pub width: usize,
    pub height: usize,
    buffer: Vec<f32>,
}

impl DepthBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![f32::INFINITY; width * height],
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buffer.fill(f32::INFINITY);
    }

    #[inline]
    pub fn clear_value(&mut self, value: f32) {
        self.buffer.fill(value);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x]
        } else {
            f32::INFINITY
        }
    }

    /// Realiza la prueba de profundidad (Depth Test) y escribe el nuevo valor si pasa.
    ///
    /// Retorna `true` si la profundidad `depth` es menor que la registrada previamente.
    #[inline]
    pub fn test_and_set(&mut self, x: usize, y: usize, depth: f32) -> bool {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            if depth < self.buffer[idx] {
                self.buffer[idx] = depth;
                return true;
            }
        }
        false
    }
}
