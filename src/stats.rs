use std::time::Instant;

/// Contadores y métricas de rendimiento del software renderer por cuadro y acumuladas.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub modelos_procesados: usize,
    pub vertices_procesados: usize,
    pub triangulos_entrada: usize,
    pub triangulos_generados_clipping: usize,
    pub triangulos_descartados_clipping: usize,
    pub triangulos_descartados_backface: usize,
    pub triangulos_rasterizados: usize,
    pub fragmentos_evaluados: usize,
    pub fragmentos_aceptados_depth: usize,
    pub frame_time_ms: f32,
}

impl FrameStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Administrador de estadísticas que acumula métricas de varios cuadros e imprime
/// un resumen legible en consola aproximadamente una vez por segundo.
pub struct StatsTracker {
    last_print_time: Instant,
    frame_count: usize,
    accumulated: FrameStats,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            last_print_time: Instant::now(),
            frame_count: 0,
            accumulated: FrameStats::default(),
        }
    }

    /// Registra las estadísticas de un cuadro renderizado y las reporta en consola si ha transcurrido un segundo.
    pub fn record_frame(&mut self, current: FrameStats) {
        self.frame_count += 1;
        self.accumulated.modelos_procesados += current.modelos_procesados;
        self.accumulated.vertices_procesados += current.vertices_procesados;
        self.accumulated.triangulos_entrada += current.triangulos_entrada;
        self.accumulated.triangulos_generados_clipping += current.triangulos_generados_clipping;
        self.accumulated.triangulos_descartados_clipping += current.triangulos_descartados_clipping;
        self.accumulated.triangulos_descartados_backface += current.triangulos_descartados_backface;
        self.accumulated.triangulos_rasterizados += current.triangulos_rasterizados;
        self.accumulated.fragmentos_evaluados += current.fragmentos_evaluados;
        self.accumulated.fragmentos_aceptados_depth += current.fragmentos_aceptados_depth;
        self.accumulated.frame_time_ms += current.frame_time_ms;

        let elapsed = self.last_print_time.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            let fps = self.frame_count as f32 / elapsed;
            let avg_frame_time = self.accumulated.frame_time_ms / self.frame_count.max(1) as f32;
            let avg_in_tris = self.accumulated.triangulos_entrada / self.frame_count.max(1);
            let avg_clip_gen =
                self.accumulated.triangulos_generados_clipping / self.frame_count.max(1);
            let avg_clip_disc =
                self.accumulated.triangulos_descartados_clipping / self.frame_count.max(1);
            let avg_backface_disc =
                self.accumulated.triangulos_descartados_backface / self.frame_count.max(1);
            let avg_rast_tris = self.accumulated.triangulos_rasterizados / self.frame_count.max(1);
            let avg_eval_frags = self.accumulated.fragmentos_evaluados / self.frame_count.max(1);
            let avg_depth_frags =
                self.accumulated.fragmentos_aceptados_depth / self.frame_count.max(1);

            println!(
                "[STATS] FPS: {:5.1} | Frame: {:5.2} ms | Tris In: {:4} | Clip (+{:2}, -{:2}) | Backface -{:4} | Rast: {:4} | Frags: {:6} -> Depth: {:6}",
                fps,
                avg_frame_time,
                avg_in_tris,
                avg_clip_gen,
                avg_clip_disc,
                avg_backface_disc,
                avg_rast_tris,
                avg_eval_frags,
                avg_depth_frags
            );

            self.last_print_time = Instant::now();
            self.frame_count = 0;
            self.accumulated = FrameStats::default();
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}
