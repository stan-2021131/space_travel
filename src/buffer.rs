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

    /// Mezcla alfa estándar: color_salida = src * alpha + dst * (1 - alpha).
    #[inline]
    pub fn blend(dst: Self, src: Self) -> Self {
        let a = src.a as u32;
        let inv_a = 255 - a;
        Self {
            r: ((src.r as u32 * a + dst.r as u32 * inv_a) / 255) as u8,
            g: ((src.g as u32 * a + dst.g as u32 * inv_a) / 255) as u8,
            b: ((src.b as u32 * a + dst.b as u32 * inv_a) / 255) as u8,
            a: 255,
        }
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

    /// Rellena un rectángulo 2D recortándolo contra los límites del framebuffer.
    pub fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) {
        if width <= 0 || height <= 0 {
            return;
        }
        let min_x = x.max(0) as usize;
        let min_y = y.max(0) as usize;
        let max_x = (x + width).min(self.width as i32).max(0) as usize;
        let max_y = (y + height).min(self.height as i32).max(0) as usize;

        for py in min_y..max_y {
            let row_start = py * self.width;
            for px in min_x..max_x {
                self.pixels[row_start + px] = color;
            }
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

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Color] {
        &mut self.pixels
    }

    /// Divide el framebuffer en `num_chunks` franjas horizontales contiguas y disjuntas.
    /// Retorna tuplas con `(y_start, y_end, slice)`.
    pub fn split_horizontal_chunks(
        &mut self,
        num_chunks: usize,
    ) -> Vec<(usize, usize, &mut [Color])> {
        let n = num_chunks.clamp(1, self.height.max(1));
        let height = self.height;
        let width = self.width;

        let mut chunks = Vec::with_capacity(n);
        let mut rem = &mut self.pixels[..];

        for i in 0..n {
            let y_start = (i * height) / n;
            let y_end = ((i + 1) * height) / n;
            let pixel_count = (y_end - y_start) * width;

            let (chunk, next_rem) = rem.split_at_mut(pixel_count);
            chunks.push((y_start, y_end, chunk));
            rem = next_rem;
        }

        chunks
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

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.buffer
    }

    /// Divide el buffer de profundidad en `num_chunks` franjas horizontales contiguas y disjuntas.
    /// Retorna tuplas con `(y_start, y_end, slice)`.
    pub fn split_horizontal_chunks(
        &mut self,
        num_chunks: usize,
    ) -> Vec<(usize, usize, &mut [f32])> {
        let n = num_chunks.clamp(1, self.height.max(1));
        let height = self.height;
        let width = self.width;

        let mut chunks = Vec::with_capacity(n);
        let mut rem = &mut self.buffer[..];

        for i in 0..n {
            let y_start = (i * height) / n;
            let y_end = ((i + 1) * height) / n;
            let pixel_count = (y_end - y_start) * width;

            let (chunk, next_rem) = rem.split_at_mut(pixel_count);
            chunks.push((y_start, y_end, chunk));
            rem = next_rem;
        }

        chunks
    }
}

/// Franja horizontal de renderizado en CPU acotada al rango vertical `[y_start, y_end)`.
///
/// Otorga acceso exclusivo a porciones disjuntas de color y profundidad sin condiciones de carrera.
pub struct RenderBand<'a> {
    pub width: usize,
    pub height: usize,
    pub y_start: usize,
    pub y_end: usize,
    pub pixels: &'a mut [Color],
    pub depth: &'a mut [f32],
}

impl<'a> RenderBand<'a> {
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y >= self.y_start && y < self.y_end {
            let local_y = y - self.y_start;
            self.pixels[local_y * self.width + x] = color;
        }
    }

    #[inline]
    pub fn blend_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y >= self.y_start && y < self.y_end {
            let local_y = y - self.y_start;
            let idx = local_y * self.width + x;
            self.pixels[idx] = Color::blend(self.pixels[idx], color);
        }
    }

    #[inline]
    pub fn test_depth_only(&self, x: usize, y: usize, depth: f32) -> bool {
        if x < self.width && y >= self.y_start && y < self.y_end {
            let local_y = y - self.y_start;
            let idx = local_y * self.width + x;
            return depth < self.depth[idx];
        }
        false
    }

    #[inline]
    pub fn test_and_set_depth(&mut self, x: usize, y: usize, depth: f32) -> bool {
        if x < self.width && y >= self.y_start && y < self.y_end {
            let local_y = y - self.y_start;
            let idx = local_y * self.width + x;
            if depth < self.depth[idx] {
                self.depth[idx] = depth;
                return true;
            }
        }
        false
    }
}

/// Divide sincrónicamente un Framebuffer y un DepthBuffer en `num_bands` franjas horizontales.
pub fn split_render_bands<'a>(
    framebuffer: &'a mut Framebuffer,
    depth_buffer: &'a mut DepthBuffer,
    num_bands: usize,
) -> Vec<RenderBand<'a>> {
    assert_eq!(framebuffer.width, depth_buffer.width);
    assert_eq!(framebuffer.height, depth_buffer.height);

    let n = num_bands.clamp(1, framebuffer.height.max(1));
    let height = framebuffer.height;
    let width = framebuffer.width;

    let mut bands = Vec::with_capacity(n);
    let mut fb_rem = &mut framebuffer.pixels[..];
    let mut db_rem = &mut depth_buffer.buffer[..];

    for i in 0..n {
        let y_start = (i * height) / n;
        let y_end = ((i + 1) * height) / n;
        let pixel_count = (y_end - y_start) * width;

        let (fb_chunk, fb_next) = fb_rem.split_at_mut(pixel_count);
        let (db_chunk, db_next) = db_rem.split_at_mut(pixel_count);

        bands.push(RenderBand {
            width,
            height,
            y_start,
            y_end,
            pixels: fb_chunk,
            depth: db_chunk,
        });

        fb_rem = fb_next;
        db_rem = db_next;
    }

    bands
}
