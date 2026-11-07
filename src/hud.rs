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
