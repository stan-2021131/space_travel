use crate::pipeline::ShaderMode;
use std::time::Instant;

/// Modos de presentación de estadísticas en consola controlados mediante la tecla F3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsDisplayMode {
    #[default]
    Off,
    Compact,
    Detailed,
}

impl StatsDisplayMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::Compact,
            Self::Compact => Self::Detailed,
            Self::Detailed => Self::Off,
        }
    }
}

/// Administrador de controles diagnósticos para garantizar el aislamiento de teclas:
/// F2 (modo de shader), F3 (modo de estadísticas) y D (exclusivamente navegación).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticControls {
    pub shader_mode: ShaderMode,
    pub stats_mode: StatsDisplayMode,
}

impl Default for DiagnosticControls {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticControls {
    pub fn new() -> Self {
        Self {
            shader_mode: ShaderMode::Normal,
            stats_mode: StatsDisplayMode::Off,
        }
    }

    /// Maneja eventos de teclado de diagnóstico.
    /// F2 alterna modo de shader, F3 alterna estadísticas, D no altera diagnósticos.
    pub fn handle_key_action(&mut self, is_f2: bool, is_f3: bool, is_d: bool) {
        let _ = is_d; // D es exclusivamente para navegación de la nave (yaw_right)
        if is_f2 {
            self.shader_mode = self.shader_mode.cycle();
        }
        if is_f3 {
            self.stats_mode = self.stats_mode.cycle();
        }
    }
}

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
    pub objetos_descartados_frustum: usize,
    pub triangulos_descartados_frustum: usize,
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
    pub display_mode: StatsDisplayMode,
    pub current_fps: f32,
    pub current_frame_time_ms: f32,
    pub last_frame_stats: FrameStats,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            last_print_time: Instant::now(),
            frame_count: 0,
            accumulated: FrameStats::default(),
            display_mode: StatsDisplayMode::Off,
            current_fps: 60.0,
            current_frame_time_ms: 16.6,
            last_frame_stats: FrameStats::default(),
        }
    }

    /// Alterna el modo de visualización de estadísticas (Off -> Compact -> Detailed -> Off).
    pub fn cycle_mode(&mut self) {
        self.display_mode = self.display_mode.cycle();
        println!(
            "[STATS] Modo de estadísticas cambiado a: {:?}",
            self.display_mode
        );
    }

    /// Registra las estadísticas de un cuadro renderizado y las reporta en consola si ha transcurrido un segundo.
    pub fn record_frame(
        &mut self,
        current: FrameStats,
        ship_speed_wu: f32,
        nearest_body_name: &str,
        surface_dist_wu: f32,
    ) {
        self.frame_count += 1;
        self.last_frame_stats = current;
        self.accumulated.modelos_procesados += current.modelos_procesados;
        self.accumulated.vertices_procesados += current.vertices_procesados;
        self.accumulated.triangulos_entrada += current.triangulos_entrada;
        self.accumulated.triangulos_generados_clipping += current.triangulos_generados_clipping;
        self.accumulated.triangulos_descartados_clipping += current.triangulos_descartados_clipping;
        self.accumulated.triangulos_descartados_backface += current.triangulos_descartados_backface;
        self.accumulated.triangulos_rasterizados += current.triangulos_rasterizados;
        self.accumulated.fragmentos_evaluados += current.fragmentos_evaluados;
        self.accumulated.fragmentos_aceptados_depth += current.fragmentos_aceptados_depth;
        self.accumulated.objetos_descartados_frustum += current.objetos_descartados_frustum;
        self.accumulated.triangulos_descartados_frustum += current.triangulos_descartados_frustum;
        self.accumulated.frame_time_ms += current.frame_time_ms;

        let elapsed = self.last_print_time.elapsed().as_secs_f32();
        if elapsed >= 0.5 {
            let count = self.frame_count.max(1);
            let fps = self.frame_count as f32 / elapsed;
            let avg_frame_time = self.accumulated.frame_time_ms / count as f32;
            self.current_fps = fps;
            self.current_frame_time_ms = avg_frame_time;

            if self.display_mode != StatsDisplayMode::Off {
                let avg_models = self.accumulated.modelos_procesados / count;
                let avg_in_tris = self.accumulated.triangulos_entrada / count;
                let avg_clip_gen = self.accumulated.triangulos_generados_clipping / count;
                let avg_clip_disc = self.accumulated.triangulos_descartados_clipping / count;
                let avg_backface_disc = self.accumulated.triangulos_descartados_backface / count;
                let avg_rast_tris = self.accumulated.triangulos_rasterizados / count;
                let avg_eval_frags = self.accumulated.fragmentos_evaluados / count;
                let avg_depth_frags = self.accumulated.fragmentos_aceptados_depth / count;
                let avg_cull_objs = self.accumulated.objetos_descartados_frustum / count;
                let avg_cull_tris = self.accumulated.triangulos_descartados_frustum / count;

                match self.display_mode {
                    StatsDisplayMode::Compact => {
                        println!(
                            "[{:2.0} FPS | {:4.1} ms] obj {} (cull {}) | tri {}→{} (cull {}) | nave {:4.1} WU/s | {} {:+5.1} WU",
                            fps,
                            avg_frame_time,
                            avg_models,
                            avg_cull_objs,
                            avg_in_tris,
                            avg_rast_tris,
                            avg_cull_tris,
                            ship_speed_wu,
                            nearest_body_name,
                            surface_dist_wu
                        );
                    }
                    StatsDisplayMode::Detailed => {
                        println!(
                            "[STATS DETALLADO] FPS: {:5.1} | Frame: {:5.2} ms | Objs: {} (Cull: {}) | Tris In: {:4} (Cull: {:4}) | Clip (+{:2}, -{:2}) | Backface -{:4} | Rast: {:4} | Frags: {:6} -> Depth: {:6} | Nave: {:4.1} WU/s | Prox: {} ({:+5.1} WU)",
                            fps,
                            avg_frame_time,
                            avg_models,
                            avg_cull_objs,
                            avg_in_tris,
                            avg_cull_tris,
                            avg_clip_gen,
                            avg_clip_disc,
                            avg_backface_disc,
                            avg_rast_tris,
                            avg_eval_frags,
                            avg_depth_frags,
                            ship_speed_wu,
                            nearest_body_name,
                            surface_dist_wu
                        );
                    }
                    StatsDisplayMode::Off => {}
                }
            }

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
