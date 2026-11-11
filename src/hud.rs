use crate::buffer::{Color, Framebuffer};
use crate::combat::EncounterState;
use crate::math::{Mat4, Vec3};
use crate::pipeline::ShaderMode;
use crate::stats::FrameStats;

// ============================================================================
// 1. Proyección de Coordenadas 3D a Pantalla
// ============================================================================

/// Proyecta una posición mundial a coordenadas de píxel. Retorna `None` si el
/// punto está detrás de la cámara o fuera del volumen visible NDC.
pub fn project_world_to_screen(
    view_projection: &Mat4,
    world_position: Vec3,
    width: usize,
    height: usize,
) -> Option<(i32, i32)> {
    let clip = view_projection.transform_vec4(world_position.to_vec4(1.0));
    if clip.w <= 1e-5 {
        return None;
    }

    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    let ndc_z = clip.z / clip.w;
    if !(-1.0..=1.0).contains(&ndc_x)
        || !(-1.0..=1.0).contains(&ndc_y)
        || !(-1.0..=1.0).contains(&ndc_z)
    {
        return None;
    }

    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32).round() as i32;
    let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32).round() as i32;
    Some((screen_x, screen_y))
}

// ============================================================================
// 2. Micro-Fuente Bitmap 5x7 en CPU (Renderizado Autónomo sin GPU/Raylib)
// ============================================================================

/// Retorna la matriz de 7 filas (5 bits por fila) para caracteres ASCII y de puntuación.
#[inline]
fn get_glyph_rows(c: char) -> [u8; 7] {
    let normalized = match c {
        'á' | 'Á' => 'A',
        'é' | 'É' => 'E',
        'í' | 'Í' => 'I',
        'ó' | 'Ó' => 'O',
        'ú' | 'Ú' => 'U',
        'ñ' | 'Ñ' => 'N',
        other => other.to_ascii_uppercase(),
    };
    match normalized {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '[' => [
            0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110,
        ],
        ']' => [
            0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110,
        ],
        '%' => [
            0b11001, 0b11010, 0b00100, 0b01000, 0b01011, 0b10011, 0b00000,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        '<' => [
            0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
        ],
        '>' => [
            0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000,
        ],
        ';' => [
            0b00000, 0b00110, 0b00110, 0b00000, 0b00110, 0b00100, 0b01000,
        ],
        '?' => [
            0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        '\'' => [
            0b00110, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '"' => [
            0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '=' => [
            0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        _ => [0b00000; 7],
    }
}

/// Dibuja texto ASCII en CPU con escala de píxeles directamente sobre el Framebuffer.
pub fn draw_text(
    framebuffer: &mut Framebuffer,
    start_x: i32,
    start_y: i32,
    text: &str,
    color: Color,
    scale: i32,
) {
    let scale = scale.max(1);
    let mut cursor_x = start_x;

    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += 4 * scale;
            continue;
        }

        let rows = get_glyph_rows(ch);
        for (row_idx, &row_bits) in rows.iter().enumerate() {
            for col_idx in 0..5 {
                let mask = 1 << (4 - col_idx);
                if (row_bits & mask) != 0 {
                    let px = cursor_x + col_idx * scale;
                    let py = start_y + row_idx as i32 * scale;
                    framebuffer.fill_rect(px, py, scale, scale, color);
                }
            }
        }
        cursor_x += 6 * scale;
    }
}

/// Calcula el ancho en píxeles de una cadena de texto renderizada con draw_text.
#[inline]
pub fn measure_text_width(text: &str, scale: i32) -> i32 {
    let scale = scale.max(1);
    let mut width = 0;
    for ch in text.chars() {
        if ch == ' ' {
            width += 4 * scale;
        } else {
            width += 6 * scale;
        }
    }
    width
}

// ============================================================================
// 3. Contexto Global del HUD y Marcadores Planetarios
// ============================================================================

/// Información de proyección en pantalla de un cuerpo celeste para su indicador visual dinámico.
#[derive(Debug, Clone)]
pub struct PlanetMarker<'a> {
    pub id: &'a str,
    pub screen_x: i32,
    pub screen_y: i32,
    pub projected_radius: i32,
    pub distance_wu: f32,
    pub is_near: bool,
    pub in_interaction_range: bool,
    pub is_panoramic_active: bool,
}

/// Datos de inspección telescópica / orbital y registro de historia para la vista panorámica.
#[derive(Debug, Clone)]
pub struct CelestialInspectionData<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub body_type: &'a str,
    pub radius_wu: f32,
    pub orbital_period_s: Option<f32>,
    pub rotation_period_s: f32,
    pub gravity_strength: f32,
    pub gravity_radius: f32,
    pub surface_objects_summary: Option<String>,
    pub moons_summary: Option<String>,
    pub rings_summary: Option<String>,
    pub shader_name: &'a str,
    pub lore_pages: &'a [String],
    pub current_page: usize,
    pub is_story_open: bool,
}

/// Datos de telemetría y estado de la simulación requeridos para renderizar el HUD.
#[derive(Debug, Clone)]
pub struct HudContext<'a> {
    pub player_health: f32,
    pub player_max_health: f32,
    pub player_alive: bool,
    pub ship_speed_wu: f32,
    pub ship_max_speed_wu: f32,
    pub combat_state: EncounterState,
    pub nearest_body_name: Option<&'a str>,
    pub nearest_body_distance_wu: f32,
    pub is_near_orbit: bool,
    pub local_flight_mode: bool,
    pub gravity_enabled: bool,
    pub shader_mode: ShaderMode,
    pub show_controls: bool,
    pub show_stats: bool,
    pub current_fps: f32,
    pub frame_time_ms: f32,
    pub frame_stats: FrameStats,
    pub first_person: bool,
    pub aim_screen_pos: Option<(i32, i32)>,
    pub sim_time: f32,
    pub planet_markers: &'a [PlanetMarker<'a>],
    pub inspection_data: Option<CelestialInspectionData<'a>>,
}

// ============================================================================
// 4. Componentes Visuales del HUD
// ============================================================================

/// Dibuja el marco angular superior que muestra el estado de alerta o el modo de los enemigos.
pub fn draw_top_mode_banner(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    let screen_w = framebuffer.width as i32;
    let banner_w = 640;
    let banner_h = 30;
    let left = (screen_w - banner_w) / 2;
    let top = 14;

    // Colores según el estado de alerta de la IA enemiga
    let (border_col, bg_col, text_col, title, subtitle) = if !ctx.player_alive {
        (
            Color::rgb(240, 50, 45),
            Color::rgb(45, 12, 14),
            Color::rgb(255, 90, 80),
            "NAVE DESTRUIDA",
            "REAPARICION EN CURSO...",
        )
    } else {
        match ctx.combat_state {
            EncounterState::Observing | EncounterState::Supervising => (
                Color::rgb(40, 220, 110),
                Color::rgb(14, 42, 24),
                Color::rgb(70, 245, 130),
                "MODO OBSERVACION",
                "TEN CUIDADO DE NO CAUSAR PROBLEMAS",
            ),
            EncounterState::Attacking => (
                Color::rgb(250, 45, 55),
                Color::rgb(50, 10, 16),
                Color::rgb(255, 80, 85),
                "ALERTA DE COMBATE",
                "HOSTILES EN MANIOBRA DE INTERCEPCION",
            ),
            EncounterState::Approaching => (
                Color::rgb(245, 185, 35),
                Color::rgb(45, 34, 10),
                Color::rgb(255, 210, 65),
                "ALERTA DE CONTACTO",
                "PATRULLA DESCONOCIDA APROXIMANDOSE",
            ),
            EncounterState::Dormant => (
                Color::rgb(55, 190, 245),
                Color::rgb(12, 30, 45),
                Color::rgb(110, 215, 255),
                "VUELO ESPACIAL",
                "SISTEMA SOLAR EN EXPLORACION LIBRE",
            ),
        }
    };

    // Fondo y borde biselado
    framebuffer.fill_rect(left, top, banner_w, banner_h, border_col);
    framebuffer.fill_rect(left + 2, top + 2, banner_w - 4, banner_h - 4, bg_col);

    // Decoraciones de bisel en esquinas
    framebuffer.fill_rect(left - 4, top + 4, 3, banner_h - 8, border_col);
    framebuffer.fill_rect(left + banner_w + 1, top + 4, 3, banner_h - 8, border_col);

    // Título y subtítulo en CPU centrados exactamente
    let banner_text = format!("{}: {}", title, subtitle);
    let text_w = measure_text_width(&banner_text, 1);
    let text_x = left + (banner_w - text_w) / 2;
    let text_y = top + (banner_h - 7) / 2;
    draw_text(framebuffer, text_x, text_y, &banner_text, text_col, 1);
}

/// Dibuja la barra de vida estilizada en la esquina inferior izquierda.
pub fn draw_player_health_panel(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    let screen_h = framebuffer.height as i32;
    let left = 24;
    let bottom = screen_h - 26;
    let bar_w = 260;
    let bar_h = 20;
    let top = bottom - bar_h;

    let fraction = (ctx.player_health / ctx.player_max_health.max(1e-4)).clamp(0.0, 1.0);

    let health_col = if fraction > 0.50 {
        Color::rgb(45, 220, 100) // Verde esmeralda
    } else if fraction > 0.25 {
        Color::rgb(245, 185, 40) // Amarillo ámbar
    } else {
        Color::rgb(240, 50, 45) // Rojo crítico
    };

    let panel_bg_w = bar_w + 95;
    let panel_bg_h = bar_h + 28;

    // Fondo sci-fi biselado
    framebuffer.fill_rect(
        left - 4,
        top - 22,
        panel_bg_w,
        panel_bg_h,
        Color::rgb(18, 24, 35),
    );
    framebuffer.fill_rect(
        left - 3,
        top - 21,
        panel_bg_w - 2,
        panel_bg_h - 2,
        Color::rgb(8, 12, 18),
    );

    // Etiqueta superior
    draw_text(
        framebuffer,
        left + 2,
        top - 16,
        "INTEGRIDAD DEL CASCO",
        Color::rgb(150, 185, 220),
        1,
    );

    // Contorno de la barra
    framebuffer.fill_rect(left, top, bar_w, bar_h, Color::rgb(45, 65, 90));
    framebuffer.fill_rect(
        left + 1,
        top + 1,
        bar_w - 2,
        bar_h - 2,
        Color::rgb(10, 15, 22),
    );

    // Relleno de salud
    let fill_w = ((bar_w - 4) as f32 * fraction).round() as i32;
    if fill_w > 0 {
        framebuffer.fill_rect(left + 2, top + 2, fill_w, bar_h - 4, health_col);
    }

    // Texto numérico 100/100 HP
    let hp_text = format!(
        "{}/{} HP",
        ctx.player_health as i32, ctx.player_max_health as i32
    );
    draw_text(
        framebuffer,
        left + bar_w + 8,
        top + 6,
        &hp_text,
        health_col,
        1,
    );
}

/// Dibuja el velocímetro en arco de cuadrante en la esquina inferior derecha con degradado.
pub fn draw_curved_speedometer(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    let screen_w = framebuffer.width as i32;
    let screen_h = framebuffer.height as i32;

    let center_x = screen_w - 35;
    let center_y = screen_h - 32;
    let radius = 140.0_f32;
    let fraction = (ctx.ship_speed_wu / ctx.ship_max_speed_wu.max(1e-4)).clamp(0.0, 1.0);

    // Ángulo desde 180° (izquierda) hasta 90° (arriba): cuadrante curvo
    let start_angle = std::f32::consts::PI;
    let total_arc = std::f32::consts::FRAC_PI_2; // 90 grados
    let steps = 60;

    // 1. Trazado del arco base (fondo tenue)
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle - t * total_arc;
        let x = (center_x as f32 + angle.cos() * radius).round() as i32;
        let y = (center_y as f32 - angle.sin() * radius).round() as i32;
        framebuffer.fill_rect(x - 1, y - 1, 3, 3, Color::rgb(35, 48, 68));
    }

    // 2. Trazado del arco activo con degradado continuo
    let active_steps = (steps as f32 * fraction).round() as usize;
    for i in 0..=active_steps {
        let t = i as f32 / steps as f32;
        let angle = start_angle - t * total_arc;
        let x = (center_x as f32 + angle.cos() * radius).round() as i32;
        let y = (center_y as f32 - angle.sin() * radius).round() as i32;

        // Degradado: de verde cian en baja velocidad a amarillo en media y rojo en máxima
        let col = if t < 0.5 {
            let k = t / 0.5;
            Color::rgb(
                (45.0 + 195.0 * k) as u8,
                (220.0 - 20.0 * k) as u8,
                (140.0 - 100.0 * k) as u8,
            )
        } else {
            let k = (t - 0.5) / 0.5;
            Color::rgb(240, (200.0 - 150.0 * k) as u8, (40.0 * (1.0 - k)) as u8)
        };

        framebuffer.fill_rect(x - 2, y - 2, 5, 5, col);
    }

    // 3. Lectura numérica de velocidad
    let kmh = (ctx.ship_speed_wu * 14.5).round() as i32;
    let speed_text = format!("{} KM/H", kmh);
    let speed_text_w = measure_text_width(&speed_text, 2);
    let label_w = measure_text_width("VELOCIDAD", 1);

    // Anclado a la derecha dentro del cuadrante, con espacio amplio respecto al arco para que 3 dígitos nunca se solapen
    let right_anchor = center_x - 15;
    let speed_x = right_anchor - speed_text_w;
    let speed_y = center_y - 30;
    let label_x = right_anchor - label_w;
    let label_y = speed_y - 14;

    draw_text(
        framebuffer,
        label_x,
        label_y,
        "VELOCIDAD",
        Color::rgb(140, 180, 220),
        1,
    );
    draw_text(
        framebuffer,
        speed_x,
        speed_y,
        &speed_text,
        Color::rgb(60, 235, 140),
        2,
    );
}

/// Dibuja el panel lateral izquierdo con los controles de vuelo (plegable con [H]).
pub fn draw_controls_panel(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    let left = 20;
    let top = 50;
    let panel_w = 245;
    let panel_h = 195;

    if !ctx.show_controls {
        // Pestaña mínima de recordatorio cuando está oculto
        framebuffer.fill_rect(left, top, 140, 18, Color::rgb(20, 30, 44));
        draw_text(
            framebuffer,
            left + 6,
            top + 5,
            "[H] VER CONTROLES",
            Color::rgb(110, 160, 210),
            1,
        );
        return;
    }

    // Fondo translúcido con marco biselado
    framebuffer.fill_rect(left, top, panel_w, panel_h, Color::rgb(35, 50, 72));
    framebuffer.fill_rect(
        left + 1,
        top + 1,
        panel_w - 2,
        panel_h - 2,
        Color::rgb(10, 16, 24),
    );

    // Cabecera
    framebuffer.fill_rect(left + 2, top + 2, panel_w - 4, 18, Color::rgb(20, 34, 52));
    draw_text(
        framebuffer,
        left + 8,
        top + 6,
        "CONTROLES DE VUELO [H]",
        Color::rgb(70, 215, 255),
        1,
    );

    let rows = [
        ("W / S", "Acelerar / Frenar"),
        ("SHIFT + W", "Acelerador Turbo"),
        ("A / D", "Guinada (Yaw)"),
        ("FLECHAS", "Cabeceo (Pitch)"),
        ("Q / E", "Alabeo (Roll)"),
        ("ESPACIO", "Disparar proyectiles"),
        ("V", "Camara 1ra/3ra"),
        ("F3", "Panel Stats (Dev)"),
        ("R", "Reiniciar posicion"),
    ];

    let mut row_y = top + 26;
    for (key, desc) in rows {
        draw_text(
            framebuffer,
            left + 8,
            row_y,
            key,
            Color::rgb(245, 205, 75),
            1,
        );
        draw_text(
            framebuffer,
            left + 80,
            row_y,
            desc,
            Color::rgb(180, 200, 220),
            1,
        );
        row_y += 18;
    }
}

/// Dibuja el panel lateral derecho con las estadísticas detalladas y modo dev (activado con [F3]).
pub fn draw_stats_panel(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    if !ctx.show_stats {
        return;
    }

    let screen_w = framebuffer.width as i32;
    let panel_w = 250;
    let panel_h = 275;
    let left = screen_w - panel_w - 20;
    let top = 50;

    // Fondo translúcido con marco biselado
    framebuffer.fill_rect(left, top, panel_w, panel_h, Color::rgb(42, 60, 85));
    framebuffer.fill_rect(
        left + 1,
        top + 1,
        panel_w - 2,
        panel_h - 2,
        Color::rgb(12, 18, 26),
    );

    // Cabecera
    framebuffer.fill_rect(left + 2, top + 2, panel_w - 4, 18, Color::rgb(22, 38, 58));
    draw_text(
        framebuffer,
        left + 8,
        top + 6,
        "STATS DETALLADOS [F3]",
        Color::rgb(80, 230, 255),
        1,
    );

    let shader_str = match ctx.shader_mode {
        ShaderMode::Normal => "NORMAL",
        ShaderMode::Toon => "TOON ANIME",
        ShaderMode::DepthDebug => "PROFUNDIDAD",
        ShaderMode::NormalsDebug => "NORMALES",
    };

    let stat_rows = [
        ("FPS REAL:", format!("{:.1}", ctx.current_fps)),
        ("FRAME TIME:", format!("{:.2} MS", ctx.frame_time_ms)),
        (
            "TRIS RASTER:",
            format!("{}", ctx.frame_stats.triangulos_rasterizados),
        ),
        (
            "TRIS CULL:",
            format!("{}", ctx.frame_stats.triangulos_descartados_frustum),
        ),
        (
            "OBJS CULL:",
            format!("{}", ctx.frame_stats.objetos_descartados_frustum),
        ),
        (
            "DEPTH PASS:",
            format!("{}", ctx.frame_stats.fragmentos_aceptados_depth),
        ),
    ];

    let mut row_y = top + 26;
    for (label, val) in &stat_rows {
        draw_text(
            framebuffer,
            left + 8,
            row_y,
            label,
            Color::rgb(160, 190, 220),
            1,
        );
        draw_text(
            framebuffer,
            left + 130,
            row_y,
            val,
            Color::rgb(75, 240, 140),
            1,
        );
        row_y += 16;
    }

    // Sección Modo Dev / Opciones
    row_y += 4;
    framebuffer.fill_rect(left + 6, row_y, panel_w - 12, 1, Color::rgb(45, 65, 90));
    row_y += 6;
    draw_text(
        framebuffer,
        left + 8,
        row_y,
        "OPCIONES MODO DEV",
        Color::rgb(240, 195, 60),
        1,
    );
    row_y += 18;

    let dev_rows = [
        ("[F2] SHADER", shader_str.to_string()),
        (
            "[G] GRAVEDAD",
            if ctx.gravity_enabled {
                "ACTIVA"
            } else {
                "DESACTIVA"
            }
            .to_string(),
        ),
        (
            "[C] ORBITA",
            if ctx.local_flight_mode {
                "BLOQUEADA"
            } else {
                "LIBRE"
            }
            .to_string(),
        ),
        (
            "[V] CAMARA",
            if ctx.first_person {
                "1RA CABINA"
            } else {
                "3RA EXTERIOR"
            }
            .to_string(),
        ),
        (
            "[F4] ENCUENTRO",
            match ctx.combat_state {
                EncounterState::Attacking => "COMBATE",
                EncounterState::Approaching | EncounterState::Supervising => "ALERTA",
                EncounterState::Observing => "OBSERVACION",
                EncounterState::Dormant => "PACIFICO",
            }
            .to_string(),
        ),
        (
            "[H] CONTROLES",
            if ctx.show_controls {
                "VISIBLE"
            } else {
                "OCULTO"
            }
            .to_string(),
        ),
    ];

    for (action, state) in &dev_rows {
        draw_text(
            framebuffer,
            left + 8,
            row_y,
            action,
            Color::rgb(245, 215, 85),
            1,
        );
        draw_text(
            framebuffer,
            left + 130,
            row_y,
            state,
            Color::rgb(90, 225, 255),
            1,
        );
        row_y += 16;
    }
}

/// Dibuja la mira táctica sci-fi con corchetes envolventes y retícula central.
pub fn draw_modern_reticle(framebuffer: &mut Framebuffer, center_x: i32, center_y: i32) {
    let core_col = Color::rgb(80, 230, 255);
    let bracket_col = Color::rgb(140, 190, 235);

    // Cruceta central fina con apertura
    framebuffer.fill_rect(center_x - 12, center_y, 7, 2, core_col);
    framebuffer.fill_rect(center_x + 6, center_y, 7, 2, core_col);
    framebuffer.fill_rect(center_x, center_y - 12, 2, 7, core_col);
    framebuffer.fill_rect(center_x, center_y + 6, 2, 7, core_col);

    // Punto central de impacto
    framebuffer.fill_rect(center_x - 1, center_y - 1, 3, 3, Color::rgb(255, 240, 100));

    // Corchetes angulares exteriores estilo visor de caza
    let b_dist = 26;
    let b_len = 10;

    // Izquierda
    framebuffer.fill_rect(
        center_x - b_dist,
        center_y - b_len,
        2,
        b_len * 2 + 1,
        bracket_col,
    );
    framebuffer.fill_rect(center_x - b_dist, center_y - b_len, 6, 2, bracket_col);
    framebuffer.fill_rect(center_x - b_dist, center_y + b_len - 1, 6, 2, bracket_col);

    // Derecha
    framebuffer.fill_rect(
        center_x + b_dist - 1,
        center_y - b_len,
        2,
        b_len * 2 + 1,
        bracket_col,
    );
    framebuffer.fill_rect(center_x + b_dist - 5, center_y - b_len, 6, 2, bracket_col);
    framebuffer.fill_rect(
        center_x + b_dist - 5,
        center_y + b_len - 1,
        6,
        2,
        bracket_col,
    );
}

/// Dibuja el visor táctico rojo centrado y compacto de cabina en primera persona.
pub fn draw_cockpit_visor(framebuffer: &mut Framebuffer) {
    let w = framebuffer.width as i32;
    let h = framebuffer.height as i32;
    let cx = w / 2;
    let cy = h / 2;
    let line_col = Color::rgb(240, 50, 55); // Rojo brillante táctico

    // 1. Marco central compacto alrededor del centro de puntería (cx, cy)
    // Lado izquierdo
    draw_thick_segment(
        framebuffer,
        cx - 170,
        cy - 65,
        cx - 110,
        cy - 65,
        1,
        line_col,
    );
    draw_thick_segment(
        framebuffer,
        cx - 110,
        cy - 65,
        cx - 85,
        cy - 35,
        1,
        line_col,
    );
    draw_thick_segment(framebuffer, cx - 85, cy - 35, cx - 85, cy + 35, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx - 85,
        cy + 35,
        cx - 110,
        cy + 65,
        1,
        line_col,
    );
    draw_thick_segment(
        framebuffer,
        cx - 110,
        cy + 65,
        cx - 170,
        cy + 65,
        1,
        line_col,
    );

    // Lado derecho
    draw_thick_segment(
        framebuffer,
        cx + 170,
        cy - 65,
        cx + 110,
        cy - 65,
        1,
        line_col,
    );
    draw_thick_segment(
        framebuffer,
        cx + 110,
        cy - 65,
        cx + 85,
        cy - 35,
        1,
        line_col,
    );
    draw_thick_segment(framebuffer, cx + 85, cy - 35, cx + 85, cy + 35, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx + 85,
        cy + 35,
        cx + 110,
        cy + 65,
        1,
        line_col,
    );
    draw_thick_segment(
        framebuffer,
        cx + 110,
        cy + 65,
        cx + 170,
        cy + 65,
        1,
        line_col,
    );

    // 2. Alas horizontales de alineación a los costados del marco central
    draw_thick_segment(framebuffer, cx - 170, cy, cx - 230, cy, 1, line_col);
    draw_thick_segment(framebuffer, cx - 230, cy, cx - 250, cy - 20, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx - 250,
        cy - 20,
        cx - 290,
        cy - 20,
        1,
        line_col,
    );

    draw_thick_segment(framebuffer, cx + 170, cy, cx + 230, cy, 1, line_col);
    draw_thick_segment(framebuffer, cx + 230, cy, cx + 250, cy - 20, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx + 250,
        cy - 20,
        cx + 290,
        cy - 20,
        1,
        line_col,
    );

    // 3. Graduaciones verticales de cabeceo (Pitch Ladder) centradas
    draw_thick_segment(framebuffer, cx - 24, cy - 85, cx + 24, cy - 85, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx - 14,
        cy - 100,
        cx + 14,
        cy - 100,
        1,
        line_col,
    );

    draw_thick_segment(framebuffer, cx - 24, cy + 85, cx + 24, cy + 85, 1, line_col);
    draw_thick_segment(
        framebuffer,
        cx - 14,
        cy + 100,
        cx + 14,
        cy + 100,
        1,
        line_col,
    );

    // 4. Chevrons superiores e inferiores de encuadre
    framebuffer.fill_rect(cx - 1, cy - 115, 3, 8, line_col);
    framebuffer.fill_rect(cx - 1, cy + 108, 3, 8, line_col);
}

/// Dibuja los indicadores interactivos y flechas proporcionales sobre cada planeta visible.
pub fn draw_planet_markers(framebuffer: &mut Framebuffer, markers: &[PlanetMarker<'_>]) {
    for marker in markers {
        let (tip_x, tip_y) = (marker.screen_x, marker.screen_y);

        // Si está fuera de los límites visibles de pantalla, se omite
        if tip_x < -50
            || tip_x > framebuffer.width as i32 + 50
            || tip_y < -50
            || tip_y > framebuffer.height as i32 + 50
        {
            continue;
        }

        // Tamaño proporcional al radio proyectado del planeta en pantalla
        let arrow_half_w = (marker.projected_radius / 6).clamp(5, 18);
        let arrow_h = (arrow_half_w * 7 / 5).clamp(7, 24);

        // Colores según estado de proximidad e interacción
        let (arrow_col, text_col) = if marker.in_interaction_range {
            (Color::rgb(60, 235, 255), Color::rgb(100, 245, 255))
        } else if marker.is_near {
            (Color::rgb(255, 220, 60), Color::rgb(255, 230, 90))
        } else {
            (Color::rgb(210, 195, 120), Color::rgb(220, 210, 150))
        };

        let bottom_y = tip_y - 4; // Justo arriba de la cúspide del planeta
        let top_y = bottom_y - arrow_h;

        // Dibuja el triángulo invertido hacia abajo: \/
        for y in top_y..=bottom_y {
            let dy = bottom_y - y;
            let hw = if arrow_h > 0 {
                (arrow_half_w * dy) / arrow_h
            } else {
                0
            };
            // Relleno interior semi-oscuro para contraste
            if hw > 1 {
                framebuffer.fill_rect(
                    tip_x - hw + 1,
                    y,
                    (hw - 1) * 2 + 1,
                    1,
                    Color::rgb(15, 20, 28),
                );
            }
            // Bordes exteriores del triángulo
            framebuffer.fill_rect(tip_x - hw, y, 1, 1, arrow_col);
            framebuffer.fill_rect(tip_x + hw, y, 1, 1, arrow_col);
        }
        // Base superior del triángulo
        framebuffer.fill_rect(
            tip_x - arrow_half_w,
            top_y,
            arrow_half_w * 2 + 1,
            1,
            arrow_col,
        );

        // Si está en rango cercano, muestra nombre y distancia encima de la flecha
        if marker.is_near {
            let name_dist = format!(
                "{} [{:.0} WU]",
                marker.id.to_uppercase(),
                marker.distance_wu
            );
            let name_w = measure_text_width(&name_dist, 1);
            let name_x = tip_x - name_w / 2;
            let name_y = top_y - 10;

            // Pequeña pastilla de fondo para legibilidad contra estrellas
            framebuffer.fill_rect(
                name_x - 3,
                name_y - 2,
                name_w + 6,
                11,
                Color::rgb(10, 15, 22),
            );
            draw_text(framebuffer, name_x, name_y, &name_dist, text_col, 1);

            // Si además está en rango de interacción para vista panorámica [C]
            if marker.in_interaction_range {
                let hint = if marker.is_panoramic_active {
                    "[C] SALIR PANORAMICA"
                } else {
                    "[C] VISTA PANORAMICA"
                };
                let hint_w = measure_text_width(hint, 1);
                let hint_x = tip_x - hint_w / 2;
                let hint_y = name_y - 12;

                framebuffer.fill_rect(
                    hint_x - 3,
                    hint_y - 2,
                    hint_w + 6,
                    11,
                    Color::rgb(15, 30, 45),
                );
                draw_text(
                    framebuffer,
                    hint_x,
                    hint_y,
                    hint,
                    Color::rgb(60, 245, 255),
                    1,
                );
            }
        }
    }
}

// ============================================================================
// 5. HUD de Inspección Panorámica y Componente de Bitácora / Historia
// ============================================================================

/// Divide un texto largo en líneas que no excedan el ancho máximo en píxeles.
pub fn wrap_text_to_lines(text: &str, max_width_px: i32, scale: i32) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let candidate = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        if measure_text_width(&candidate, scale) <= max_width_px {
            current_line = candidate;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Divide las páginas de lore en páginas de líneas que quepan en la caja de diálogo sin desbordarse.
pub fn paginate_lore(
    pages: &[String],
    max_width_px: i32,
    max_lines_per_page: usize,
    scale: i32,
) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for raw_page in pages {
        let lines = wrap_text_to_lines(raw_page, max_width_px, scale);
        if lines.is_empty() {
            continue;
        }
        for chunk in lines.chunks(max_lines_per_page) {
            result.push(chunk.to_vec());
        }
    }
    result
}

/// Traza el contorno rectangular de 1 píxel sobre el Framebuffer.
#[inline]
pub fn draw_rect_outline(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, color: Color) {
    if w <= 0 || h <= 0 {
        return;
    }
    fb.fill_rect(x, y, w, 1, color);
    fb.fill_rect(x, y + h - 1, w, 1, color);
    fb.fill_rect(x, y, 1, h, color);
    fb.fill_rect(x + w - 1, y, 1, h, color);
}

/// Dibuja el panel lateral derecho con las características astronómicas del cuerpo celeste.
pub fn draw_general_data_panel(
    framebuffer: &mut Framebuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    data: &CelestialInspectionData<'_>,
) {
    // 1. Fondo oscuro translúcido
    framebuffer.fill_rect(x, y, width, height, Color::rgb(8, 12, 22));

    // 2. Borde tecnológico (doble marco azul/cian)
    draw_rect_outline(framebuffer, x, y, width, height, Color::rgb(45, 85, 160));
    draw_rect_outline(
        framebuffer,
        x + 2,
        y + 2,
        width - 4,
        height - 4,
        Color::rgb(18, 32, 60),
    );

    // 3. Encabezado de la tarjeta "DATOS GENERALES"
    let header_h = 24;
    framebuffer.fill_rect(x, y, width, header_h, Color::rgb(22, 45, 95));
    draw_rect_outline(framebuffer, x, y, width, header_h, Color::rgb(65, 125, 230));
    draw_text(
        framebuffer,
        x + 14,
        y + 8,
        "DATOS GENERALES",
        Color::rgb(170, 225, 255),
        1,
    );

    // 4. Filas telemétricas
    let mut cur_y = y + header_h + 14;
    let row_spacing = 20;
    let label_x = x + 14;
    let value_x = x + 125;

    let mut draw_row = |fb: &mut Framebuffer, label: &str, val: &str| {
        draw_text(fb, label_x, cur_y, label, Color::rgb(125, 155, 195), 1);
        draw_text(fb, value_x, cur_y, val, Color::rgb(235, 245, 255), 1);
        cur_y += row_spacing;
    };

    draw_row(framebuffer, "ID / NOMBRE:", data.id);
    draw_row(framebuffer, "TIPO:", data.body_type);

    let radius_str = format!("{:.1} WU", data.radius_wu);
    draw_row(framebuffer, "RADIO:", &radius_str);

    let orbit_str = match data.orbital_period_s {
        Some(p) => format!("{:.1} S", p),
        None => "N/A (CENTRO)".to_string(),
    };
    draw_row(framebuffer, "PERIODO ORB:", &orbit_str);

    let rot_str = format!("{:.1} S", data.rotation_period_s);
    draw_row(framebuffer, "ROTACION:", &rot_str);

    let grav_str = if data.gravity_strength > 0.0 {
        format!(
            "{:.1} G (R: {:.1})",
            data.gravity_strength, data.gravity_radius
        )
    } else {
        "DESACTIVADA".to_string()
    };
    draw_row(framebuffer, "GRAVEDAD:", &grav_str);

    if let Some(ref obj_str) = data.surface_objects_summary {
        draw_row(framebuffer, "SUPERFICIE:", obj_str);
    }

    if let Some(ref moons_str) = data.moons_summary {
        draw_row(framebuffer, "SATELITES:", moons_str);
    }

    if let Some(ref rings_str) = data.rings_summary {
        draw_row(framebuffer, "ANILLOS:", rings_str);
    }

    draw_row(framebuffer, "SHADER CPU:", data.shader_name);
}

/// Propiedades de configuración y contenido para el componente de cuadro de diálogo StoryBox.
#[derive(Debug, Clone)]
pub struct StoryBoxProps<'a> {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub tab_title: &'a str,
    pub lines: &'a [String],
    pub current_page_idx: usize,
    pub total_pages: usize,
    pub is_open: bool,
}

/// Componente modular y reutilizable de cuadro de diálogo / historia (StoryBox).
///
/// Recibe posición, dimensiones, título de pestaña, líneas formateadas de la página actual,
/// índice de página, total de páginas y si está abierto o cerrado mediante `StoryBoxProps`.
pub fn draw_story_dialog_box(framebuffer: &mut Framebuffer, props: &StoryBoxProps<'_>) {
    let x = props.x;
    let y = props.y;
    let width = props.width;
    let height = props.height;
    let tab_title = props.tab_title;
    let lines = props.lines;
    let current_page_idx = props.current_page_idx;
    let total_pages = props.total_pages;
    let is_open = props.is_open;

    if !is_open {
        // Barra discreta de reapertura y navegación en la parte inferior cuando está cerrado
        let bar_width = 460.min(width);
        let bar_x = x + (width - bar_width) / 2;
        let bar_y = y + height - 28;
        let bar_height = 24;

        framebuffer.fill_rect(bar_x, bar_y, bar_width, bar_height, Color::rgb(12, 18, 30));
        draw_rect_outline(
            framebuffer,
            bar_x,
            bar_y,
            bar_width,
            bar_height,
            Color::rgb(45, 80, 140),
        );

        let prompt = "[C] SALIR DE VISTA | [ENTER] REABRIR REGISTRO";
        let prompt_w = measure_text_width(prompt, 1);
        let tx = bar_x + (bar_width - prompt_w) / 2;
        let ty = bar_y + (bar_height - 7) / 2;
        draw_text(framebuffer, tx, ty, prompt, Color::rgb(140, 195, 245), 1);
        return;
    }

    // 1. Fondo oscuro translúcido
    framebuffer.fill_rect(x, y, width, height, Color::rgb(8, 12, 22));

    // 2. Borde exterior tecnológico doble
    draw_rect_outline(framebuffer, x, y, width, height, Color::rgb(45, 85, 160));
    draw_rect_outline(
        framebuffer,
        x + 2,
        y + 2,
        width - 4,
        height - 4,
        Color::rgb(18, 32, 60),
    );

    // 3. Pestaña superior izquierda (Header Tab)
    let tab_w = 270.min(width - 20);
    let tab_h = 22;
    framebuffer.fill_rect(x, y, tab_w, tab_h, Color::rgb(22, 45, 95));
    draw_rect_outline(framebuffer, x, y, tab_w, tab_h, Color::rgb(65, 125, 230));
    draw_text(
        framebuffer,
        x + 12,
        y + 7,
        tab_title,
        Color::rgb(160, 220, 255),
        1,
    );

    // Indicador de página en la esquina derecha del tab
    let page_indicator = format!(
        "{}/{}",
        (current_page_idx + 1).min(total_pages),
        total_pages
    );
    let p_w = measure_text_width(&page_indicator, 1);
    draw_text(
        framebuffer,
        x + tab_w - p_w - 10,
        y + 7,
        &page_indicator,
        Color::rgb(255, 220, 100),
        1,
    );

    // 4. Texto de la bitácora / historia
    let text_start_x = x + 20;
    let mut text_y = y + tab_h + 12;
    let line_height = 15;

    for line in lines {
        draw_text(
            framebuffer,
            text_start_x,
            text_y,
            line,
            Color::rgb(225, 240, 255),
            1,
        );
        text_y += line_height;
    }

    // 5. Botón / Instrucción en la esquina inferior derecha
    let is_last_page = current_page_idx + 1 >= total_pages;
    let (action_label, action_color) = if is_last_page {
        ("[ENTER] CERRAR REGISTRO [X]", Color::rgb(255, 190, 70))
    } else {
        ("[ENTER] SIGUIENTE PAGINA [>]", Color::rgb(70, 230, 180))
    };

    let action_w = measure_text_width(action_label, 1);
    let action_x = x + width - action_w - 20;
    let action_y = y + height - 20;

    // Pequeño marco de realce para el botón de acción
    framebuffer.fill_rect(
        action_x - 6,
        action_y - 3,
        action_w + 12,
        14,
        Color::rgb(18, 30, 55),
    );
    draw_rect_outline(
        framebuffer,
        action_x - 6,
        action_y - 3,
        action_w + 12,
        14,
        action_color,
    );
    draw_text(
        framebuffer,
        action_x,
        action_y,
        action_label,
        action_color,
        1,
    );

    // Instrucción para salir con [C] en la esquina inferior izquierda
    draw_text(
        framebuffer,
        text_start_x,
        action_y,
        "[C] SALIR DE VISTA PANORAMICA",
        Color::rgb(120, 145, 175),
        1,
    );
}

/// Dibuja el HUD panorámico completo al entrar en la vista orbital de un astro.
pub fn draw_planet_inspection_hud(
    framebuffer: &mut Framebuffer,
    data: &CelestialInspectionData<'_>,
) {
    let width = framebuffer.width as i32;
    let height = framebuffer.height as i32;

    // 1. Título superior central con escala 2 y color dorado
    let title_w = measure_text_width(data.title, 2);
    let title_x = (width - title_w) / 2;
    let title_y = 26;

    // Decoración de acento alrededor del título
    let bracket_pad = 16;
    draw_text(
        framebuffer,
        title_x - bracket_pad - 12,
        title_y,
        "[",
        Color::rgb(100, 180, 255),
        2,
    );
    draw_text(
        framebuffer,
        title_x + title_w + bracket_pad,
        title_y,
        "]",
        Color::rgb(100, 180, 255),
        2,
    );
    draw_text(
        framebuffer,
        title_x,
        title_y,
        data.title,
        Color::rgb(255, 225, 75),
        2,
    );

    // Subtítulo pequeño debajo del título
    let sub_text = "SECTOR ORBITAL // REGISTRO TELEMETRICO";
    let sub_w = measure_text_width(sub_text, 1);
    draw_text(
        framebuffer,
        (width - sub_w) / 2,
        title_y + 20,
        sub_text,
        Color::rgb(120, 160, 200),
        1,
    );

    // 2. Panel derecho de "DATOS GENERALES"
    let panel_w = 340;
    let panel_h = 240;
    let panel_x = width - panel_w - 28;
    let panel_y = 65;
    draw_general_data_panel(framebuffer, panel_x, panel_y, panel_w, panel_h, data);

    // 3. Cuadro inferior de historia / bitácora (StoryBox)
    let story_w = 980.min(width - 56);
    let story_h = 135;
    let story_x = (width - story_w) / 2;
    let story_y = height - story_h - 24;

    // Paginación y envoltura de texto
    let max_text_width = story_w - 40;
    let paginated = paginate_lore(data.lore_pages, max_text_width, 4, 1);
    let total_pages = paginated.len().max(1);
    let current_idx = data.current_page.min(total_pages - 1);
    let current_lines = if !paginated.is_empty() {
        paginated[current_idx].clone()
    } else {
        vec!["Sin registros de bitacora disponibles para este sector.".to_string()]
    };

    let story_props = StoryBoxProps {
        x: story_x,
        y: story_y,
        width: story_w,
        height: story_h,
        tab_title: "HISTORIA / REGISTRO DE BITACORA",
        lines: &current_lines,
        current_page_idx: current_idx,
        total_pages,
        is_open: data.is_story_open,
    };

    draw_story_dialog_box(framebuffer, &story_props);
}

// ============================================================================
// 6. Función Principal Modular de Entrada: render_hud
// ============================================================================

/// Renderiza el HUD de juego completo desacoplado sobre el framebuffer.
pub fn render_hud(framebuffer: &mut Framebuffer, ctx: &HudContext<'_>) {
    // Si la nave está en modo panorámico orbital de inspección, renderiza la vista cinematográfica
    if let Some(ref insp) = ctx.inspection_data {
        draw_planet_inspection_hud(framebuffer, insp);
        return;
    }

    // 1. Visor de primera persona si está activo el modo cabina
    if ctx.first_person {
        draw_cockpit_visor(framebuffer);
    }

    // 2. Indicadores dinámicos y flechas proporcionales sobre planetas
    draw_planet_markers(framebuffer, ctx.planet_markers);

    // 3. Banner central superior (Modo Observación / Combate / Alerta)
    draw_top_mode_banner(framebuffer, ctx);

    // 4. Barra de vida inferior izquierda con texto 100/100
    draw_player_health_panel(framebuffer, ctx);

    // 5. Velocímetro curvo inferior derecho con degradado
    draw_curved_speedometer(framebuffer, ctx);

    // 6. Panel lateral izquierdo de controles (toggle con [H])
    draw_controls_panel(framebuffer, ctx);

    // 7. Panel lateral derecho de estadísticas y opciones dev (toggle con [F3])
    draw_stats_panel(framebuffer, ctx);

    // 8. Mira central moderna proyectada
    if let Some((mx, my)) = ctx.aim_screen_pos {
        draw_modern_reticle(framebuffer, mx, my);
    }
}

// ============================================================================
// 6. Funciones de compatibilidad heredadas
// ============================================================================

pub fn draw_health_bar(
    framebuffer: &mut Framebuffer,
    center_x: i32,
    top_y: i32,
    width: i32,
    height: i32,
    health_fraction: f32,
) {
    if width < 4 || height < 3 {
        return;
    }
    let left = center_x - width / 2;
    framebuffer.fill_rect(left, top_y, width, height, Color::WHITE);
    framebuffer.fill_rect(
        left + 1,
        top_y + 1,
        width - 2,
        height - 2,
        Color::rgb(100, 18, 22),
    );

    let inner_width = width - 2;
    let filled = (inner_width as f32 * health_fraction.clamp(0.0, 1.0)).round() as i32;
    if filled > 0 {
        let color = if health_fraction > 0.5 {
            Color::rgb(40, 220, 90)
        } else if health_fraction > 0.25 {
            Color::rgb(245, 190, 35)
        } else {
            Color::rgb(240, 55, 45)
        };
        framebuffer.fill_rect(left + 1, top_y + 1, filled, height - 2, color);
    }
}

pub fn draw_aim_reticle(framebuffer: &mut Framebuffer, center_x: i32, center_y: i32) {
    let size = 10;
    let gap = 3;
    let core = Color::rgb(150, 168, 205);
    let outline = Color::rgb(20, 24, 38);

    draw_thick_segment(
        framebuffer,
        center_x - size,
        center_y,
        center_x - gap,
        center_y,
        1,
        outline,
    );
    draw_thick_segment(
        framebuffer,
        center_x + gap,
        center_y,
        center_x + size,
        center_y,
        1,
        outline,
    );
    draw_thick_segment(
        framebuffer,
        center_x,
        center_y - size,
        center_x,
        center_y - gap,
        1,
        outline,
    );
    draw_thick_segment(
        framebuffer,
        center_x,
        center_y + gap,
        center_x,
        center_y + size,
        1,
        outline,
    );

    let diag_size = 8;
    let diag_gap = 3;
    draw_thick_segment(
        framebuffer,
        center_x - diag_size,
        center_y - diag_size,
        center_x - diag_gap,
        center_y - diag_gap,
        1,
        core,
    );
    draw_thick_segment(
        framebuffer,
        center_x + diag_gap,
        center_y - diag_gap,
        center_x + diag_size,
        center_y - diag_size,
        1,
        core,
    );
    draw_thick_segment(
        framebuffer,
        center_x - diag_size,
        center_y + diag_size,
        center_x - diag_gap,
        center_y + diag_gap,
        1,
        core,
    );
    draw_thick_segment(
        framebuffer,
        center_x + diag_gap,
        center_y + diag_gap,
        center_x + diag_size,
        center_y + diag_size,
        1,
        core,
    );

    draw_diamond(framebuffer, center_x, center_y, 2, outline);
    draw_diamond(framebuffer, center_x, center_y, 1, core);
}

pub fn draw_speed_streaks(framebuffer: &mut Framebuffer, sim_time: f32, count: usize) {
    if count == 0 || framebuffer.width == 0 || framebuffer.height == 0 {
        return;
    }
    let center_x = framebuffer.width as f32 * 0.5;
    let center_y = framebuffer.height as f32 * 0.5;
    let max_radius = framebuffer.width.min(framebuffer.height) as f32 * 0.62;
    let min_radius = max_radius * 0.34;
    let travel = max_radius - min_radius;
    let golden_angle = 2.399_963_1_f32;

    for index in 0..count {
        let angle = index as f32 * golden_angle + (index % 3) as f32 * 0.17;
        let phase = (sim_time * 1.9 + index as f32 * 0.137).fract();
        let end_radius = min_radius + travel * phase;
        let length = 16.0 + 30.0 * phase;
        let start_radius = (end_radius - length).max(min_radius * 0.8);
        let (sin, cos) = angle.sin_cos();
        let start_x = (center_x + cos * start_radius) as i32;
        let start_y = (center_y + sin * start_radius * 0.75) as i32;
        let end_x = (center_x + cos * end_radius) as i32;
        let end_y = (center_y + sin * end_radius * 0.75) as i32;
        draw_thick_segment(
            framebuffer,
            start_x,
            start_y,
            end_x,
            end_y,
            1,
            Color::rgb(175, 220, 255),
        );
    }
}

pub fn draw_thick_segment(
    framebuffer: &mut Framebuffer,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    radius: i32,
    color: Color,
) {
    let delta_x = end_x - start_x;
    let delta_y = end_y - start_y;
    let steps = delta_x.abs().max(delta_y.abs());
    if steps == 0 {
        framebuffer.fill_rect(
            start_x - radius,
            start_y - radius,
            radius * 2 + 1,
            radius * 2 + 1,
            color,
        );
        return;
    }

    for step in 0..=steps {
        let x = start_x + (delta_x * step).div_euclid(steps);
        let y = start_y + (delta_y * step).div_euclid(steps);
        framebuffer.fill_rect(
            x - radius,
            y - radius,
            radius * 2 + 1,
            radius * 2 + 1,
            color,
        );
    }
}

pub fn draw_diamond(
    framebuffer: &mut Framebuffer,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: Color,
) {
    for offset_y in -radius..=radius {
        let half_width = radius - offset_y.abs();
        framebuffer.fill_rect(
            center_x - half_width,
            center_y + offset_y,
            half_width * 2 + 1,
            1,
            color,
        );
    }
}

pub fn draw_orbital_lock_reticle(
    framebuffer: &mut Framebuffer,
    center_x: i32,
    center_y: i32,
    is_active: bool,
) {
    let size = if is_active { 18 } else { 14 };
    let corner_len = if is_active { 7 } else { 5 };
    let color = if is_active {
        Color::rgb(60, 230, 255)
    } else {
        Color::rgb(100, 160, 195)
    };
    let outline = Color::rgb(18, 26, 42);

    draw_thick_segment(
        framebuffer,
        center_x - size,
        center_y - size,
        center_x - size + corner_len,
        center_y - size,
        1,
        color,
    );
    draw_thick_segment(
        framebuffer,
        center_x - size,
        center_y - size,
        center_x - size,
        center_y - size + corner_len,
        1,
        color,
    );

    draw_thick_segment(
        framebuffer,
        center_x + size,
        center_y - size,
        center_x + size - corner_len,
        center_y - size,
        1,
        color,
    );
    draw_thick_segment(
        framebuffer,
        center_x + size,
        center_y - size,
        center_x + size,
        center_y - size + corner_len,
        1,
        color,
    );

    draw_thick_segment(
        framebuffer,
        center_x - size,
        center_y + size,
        center_x - size + corner_len,
        center_y + size,
        1,
        color,
    );
    draw_thick_segment(
        framebuffer,
        center_x - size,
        center_y + size,
        center_x - size,
        center_y + size - corner_len,
        1,
        color,
    );

    draw_thick_segment(
        framebuffer,
        center_x + size,
        center_y + size,
        center_x + size - corner_len,
        center_y + size,
        1,
        color,
    );
    draw_thick_segment(
        framebuffer,
        center_x + size,
        center_y + size,
        center_x + size,
        center_y + size - corner_len,
        1,
        color,
    );

    if is_active {
        draw_diamond(framebuffer, center_x, center_y, 3, outline);
        draw_diamond(framebuffer, center_x, center_y, 2, Color::rgb(255, 230, 80));
    } else {
        draw_diamond(framebuffer, center_x, center_y, 2, outline);
        draw_diamond(framebuffer, center_x, center_y, 1, color);
    }
}

pub fn draw_orbital_mode_badge(framebuffer: &mut Framebuffer, x: i32, y: i32, is_active: bool) {
    let width = 76;
    let height = 12;
    let border_color = Color::rgb(35, 45, 65);
    framebuffer.fill_rect(x, y, width, height, border_color);

    if is_active {
        framebuffer.fill_rect(x + 1, y + 1, width - 2, height - 2, Color::rgb(20, 80, 110));
        framebuffer.fill_rect(x + 3, y + 3, 6, 6, Color::rgb(50, 240, 255));
    } else {
        framebuffer.fill_rect(x + 1, y + 1, width - 2, height - 2, Color::rgb(30, 35, 45));
        framebuffer.fill_rect(x + 3, y + 3, 6, 6, Color::rgb(140, 160, 180));
    }
}
