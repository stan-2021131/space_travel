use crate::buffer::{Color, Framebuffer};
use crate::math::{Mat4, Vec3};

/// Proyecta una posición mundial a coordenadas de píxel. Retorna `None` si el
/// punto está detrás de la cámara o fuera del volumen visible.
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

/// Dibuja una barra compacta con borde blanco, fondo rojo y vida verde.
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

/// Dibuja una mira de ocho segmentos con alto contraste sobre el framebuffer CPU.
/// El centro se recibe ya proyectado para que represente la trayectoria real de disparo.
pub fn draw_aim_reticle(framebuffer: &mut Framebuffer, center_x: i32, center_y: i32) {
    let outline = Color::rgb(28, 36, 58);
    let core = Color::rgb(150, 168, 205);
    let segments = [
        ((0, -16), (0, -10)),
        ((0, 10), (0, 16)),
        ((-16, 0), (-10, 0)),
        ((10, 0), (16, 0)),
        ((-11, -11), (-7, -7)),
        ((11, -11), (7, -7)),
        ((-11, 11), (-7, 7)),
        ((11, 11), (7, 7)),
    ];

    for &((start_x, start_y), (end_x, end_y)) in &segments {
        draw_thick_segment(
            framebuffer,
            center_x + start_x,
            center_y + start_y,
            center_x + end_x,
            center_y + end_y,
            2,
            outline,
        );
    }
    for &((start_x, start_y), (end_x, end_y)) in &segments {
        draw_thick_segment(
            framebuffer,
            center_x + start_x,
            center_y + start_y,
            center_x + end_x,
            center_y + end_y,
            1,
            core,
        );
    }

    draw_diamond(framebuffer, center_x, center_y, 2, outline);
    draw_diamond(framebuffer, center_x, center_y, 1, core);
}

/// Dibuja líneas radiales animadas durante el acelerador directamente en el framebuffer CPU.
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

#[allow(clippy::too_many_arguments)]
fn draw_thick_segment(
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

fn draw_diamond(
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

/// Dibuja un indicador orbital sobre el centro proyectado de un planeta cercano.
///
/// Si `is_active` es true, dibuja un brillo/marcador de fijación orbital activo.
/// Si es false, dibuja corchetes sutiles de aproximación disponible.
pub fn draw_orbital_lock_reticle(
    framebuffer: &mut Framebuffer,
    center_x: i32,
    center_y: i32,
    is_active: bool,
) {
    let size = if is_active { 18 } else { 14 };
    let corner_len = if is_active { 7 } else { 5 };
    let color = if is_active {
        Color::rgb(60, 230, 255) // Cian brillante de enganche
    } else {
        Color::rgb(100, 160, 195) // Azul aero sutil
    };
    let outline = Color::rgb(18, 26, 42);

    // Superior izquierda
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

    // Superior derecha
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

    // Inferior izquierda
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

    // Inferior derecha
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

    // Núcleo central
    if is_active {
        draw_diamond(framebuffer, center_x, center_y, 3, outline);
        draw_diamond(framebuffer, center_x, center_y, 2, Color::rgb(255, 230, 80));
    } else {
        draw_diamond(framebuffer, center_x, center_y, 2, outline);
        draw_diamond(framebuffer, center_x, center_y, 1, color);
    }
}

/// Dibuja una insignia de estado del modo orbital en el HUD.
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
