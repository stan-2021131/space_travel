use crate::buffer::{Color, DepthBuffer, Framebuffer};
use crate::math::{Mat4, Vec3};

/// Partícula individual de chispa incandescente generada por impacto mecánico.
#[derive(Debug, Clone, Copy)]
pub struct SparkParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Color,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub size: u8,
}

/// Sistema de partículas liviano en CPU para efectos de colisión sin asignaciones dinámicas.
#[derive(Debug, Clone)]
pub struct SparkSystem {
    particles: Vec<SparkParticle>,
    max_particles: usize,
    rng_seed: u64,
}

impl SparkSystem {
    pub fn new(max_particles: usize) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            max_particles,
            rng_seed: 0xCAFEBABE12345678,
        }
    }

    /// Genera un número pseudoaleatorio determinista rápido (LCG 64-bit).
    fn next_f32(&mut self) -> f32 {
        self.rng_seed = self
            .rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng_seed >> 32) as u32 as f32) / (u32::MAX as f32)
    }

    /// Emite un grupo vistoso de chispas y un destello incandescente en el punto de contacto.
    pub fn emit_sparks(
        &mut self,
        contact_point: Vec3,
        normal: Vec3,
        count: usize,
        impact_speed: f32,
    ) {
        let n = if normal.length_squared() > 1e-6 {
            normal.normalize()
        } else {
            Vec3::up()
        };

        // Base ortonormal para dispersión en cono alrededor de la normal
        let tangent = if n.y.abs() < 0.9 {
            n.cross(Vec3::up()).normalize()
        } else {
            n.cross(Vec3::new(1.0, 0.0, 0.0)).normalize()
        };
        let bitangent = n.cross(tangent).normalize();

        // 1. Destello nuclear central de impacto (flash incandescente blanco)
        let target_flash = if count >= 16 { 5 } else { 2 };
        let flash_count = target_flash.min(count);
        for _ in 0..flash_count {
            if self.particles.len() >= self.max_particles {
                break;
            }
            let flash_life = 0.16 + self.next_f32() * 0.12;
            let flash_vel = n * (1.0 + self.next_f32() * 2.5);
            self.particles.push(SparkParticle {
                position: contact_point + n * 0.12,
                velocity: flash_vel,
                color: Color::new(255, 255, 255, 255),
                lifetime: flash_life,
                max_lifetime: flash_life,
                size: 3,
            });
        }

        // 2. Chispas voladoras con dispersión angular y velocidades variadas
        let base_speed = (impact_speed * 2.2).clamp(12.0, 52.0);
        let spark_count = count.saturating_sub(flash_count);

        for _ in 0..spark_count {
            if self.particles.len() >= self.max_particles {
                break;
            }

            // Dispersión cónica aleatoria
            let angle = self.next_f32() * std::f32::consts::TAU;
            let spread = 0.15 + self.next_f32() * 0.95; // Apertura del cono
            let speed_factor = 0.40 + self.next_f32() * 1.50;
            let speed = base_speed * speed_factor;

            let dir = (n + tangent * (angle.cos() * spread) + bitangent * (angle.sin() * spread))
                .normalize();
            let velocity = dir * speed;

            // Vida útil prolongada para que se aprecien claramente sus trayectorias
            let lifetime = 0.40 + self.next_f32() * 0.55;

            // Tamaño variado entre fragmentos densos y microchispas
            let size = if self.next_f32() > 0.40 { 2 } else { 1 };

            // Tinte inicial de alta temperatura
            let color_rnd = self.next_f32();
            let color = if color_rnd > 0.50 {
                Color::new(255, 255, 240, 255) // Blanco cálido brillante
            } else if color_rnd > 0.20 {
                Color::new(255, 220, 80, 255) // Dorado intenso
            } else {
                Color::new(255, 150, 30, 255) // Ámbar vivo
            };

            self.particles.push(SparkParticle {
                position: contact_point + n * 0.20,
                velocity,
                color,
                lifetime,
                max_lifetime: lifetime,
                size,
            });
        }
    }

    /// Actualiza la posición y decaimiento de las partículas en función del tiempo ($dt$).
    pub fn update(&mut self, dt: f32) {
        // Resistencia espacial suave que permite a las chispas viajar lejos antes de apagarse
        let drag = (-1.4 * dt).exp();
        for p in &mut self.particles {
            p.position += p.velocity * dt;
            p.velocity *= drag;
            p.lifetime -= dt;
        }

        self.particles.retain(|p| p.lifetime > 0.0);
    }

    /// Cantidad de partículas activas actualmente.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Retorna una vista de las partículas activas actualmente.
    pub fn particles(&self) -> &[SparkParticle] {
        &self.particles
    }

    /// Elimina todas las partículas activas.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Renderiza las chispas sobre el framebuffer aplicando prueba de profundidad,
    /// gradiente térmico de enfriamiento y mezcla aditiva saturada.
    pub fn render(
        &self,
        framebuffer: &mut Framebuffer,
        depth_buffer: &DepthBuffer,
        view_projection: &Mat4,
    ) {
        let width = framebuffer.width;
        let height = framebuffer.height;

        for p in &self.particles {
            let clip = view_projection.transform_vec4(p.position.to_vec4(1.0));
            if clip.w <= 1e-4 {
                continue;
            }

            let ndc_x = clip.x / clip.w;
            let ndc_y = clip.y / clip.w;
            let ndc_z = clip.z / clip.w;

            if !(-1.0..=1.0).contains(&ndc_x)
                || !(-1.0..=1.0).contains(&ndc_y)
                || !(-1.0..=1.0).contains(&ndc_z)
            {
                continue;
            }

            let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32).round() as i32;
            let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32).round() as i32;

            if screen_x < 0 || screen_x >= width as i32 || screen_y < 0 || screen_y >= height as i32
            {
                continue;
            }

            let px = screen_x as usize;
            let py = screen_y as usize;

            // Prueba de profundidad contra el Z-buffer con tolerancia para que no se ocluyan por su propia superficie
            let buf_depth = depth_buffer.get(px, py);
            if ndc_z > buf_depth + 0.025 {
                continue;
            }

            let fade = (p.lifetime / p.max_lifetime.max(1e-5)).clamp(0.0, 1.0);
            // Decaimiento visual cóncavo: se mantienen brillantes y llamativas casi todo su recorrido
            let intensity = (fade * 1.4).min(1.0);

            // Enfriamiento térmico dinámico del color: blanco -> dorado -> ámbar -> rojo anaranjado
            let (cr, cg, cb) = if fade > 0.50 {
                let t = (fade - 0.50) / 0.50;
                (
                    255,
                    (210.0 + 45.0 * t) as u8,
                    (p.color.b as f32 * (0.5 + 0.5 * t)) as u8,
                )
            } else {
                let t = (fade / 0.50).clamp(0.0, 1.0);
                (255, (70.0 + 140.0 * t) as u8, (15.0 + 55.0 * t) as u8)
            };

            let spark_r = ((cr as f32) * intensity) as u16;
            let spark_g = ((cg as f32) * intensity) as u16;
            let spark_b = ((cb as f32) * intensity) as u16;

            // 1. Píxel central con adición saturada
            let dst = framebuffer.get_pixel(px, py);
            framebuffer.set_pixel(
                px,
                py,
                Color::new(
                    (dst.r as u16 + spark_r).min(255) as u8,
                    (dst.g as u16 + spark_g).min(255) as u8,
                    (dst.b as u16 + spark_b).min(255) as u8,
                    255,
                ),
            );

            // 2. Halo voluminoso y visible: cada chispa tiene un radio de 2 a 4 píxeles para destacar claramente
            let radius: i32 = match p.size {
                1 => 2, // 5x5 píxeles de resplandor
                2 => 3, // 7x7 píxeles de rescoldo brillante
                _ => 4, // 9x9 píxeles de destello de impacto
            };

            let halo_factor = match p.size {
                1 => 0.55,
                2 => 0.70,
                _ => 0.85,
            };
            let h_r = ((spark_r as f32) * halo_factor) as u16;
            let h_g = ((spark_g as f32) * halo_factor) as u16;
            let h_b = ((spark_b as f32) * halo_factor) as u16;

            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq > radius * radius {
                        continue;
                    }

                    let nx = screen_x + dx;
                    let ny = screen_y + dy;
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let n_px = nx as usize;
                        let n_py = ny as usize;
                        if ndc_z <= depth_buffer.get(n_px, n_py) + 0.025 {
                            let falloff = 1.0 - (dist_sq as f32).sqrt() / (radius as f32 + 0.5);
                            let n_dst = framebuffer.get_pixel(n_px, n_py);
                            framebuffer.set_pixel(
                                n_px,
                                n_py,
                                Color::new(
                                    (n_dst.r as u16 + ((h_r as f32 * falloff) as u16)).min(255)
                                        as u8,
                                    (n_dst.g as u16 + ((h_g as f32 * falloff) as u16)).min(255)
                                        as u8,
                                    (n_dst.b as u16 + ((h_b as f32 * falloff) as u16)).min(255)
                                        as u8,
                                    255,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}
