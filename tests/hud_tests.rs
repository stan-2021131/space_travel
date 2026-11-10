use space_travel::buffer::{Color, Framebuffer};
use space_travel::hud::{
    draw_aim_reticle, draw_health_bar, draw_speed_streaks, project_world_to_screen,
};
use space_travel::math::{Mat4, Vec3};

#[test]
fn identity_projection_maps_origin_to_screen_center() {
    let projected = project_world_to_screen(&Mat4::IDENTITY, Vec3::zero(), 100, 80);
    assert_eq!(projected, Some((50, 40)));
}

#[test]
fn speed_streaks_are_deterministic_and_clip_safely() {
    let mut first = Framebuffer::new(96, 72);
    let mut second = Framebuffer::new(96, 72);
    draw_speed_streaks(&mut first, 1.25, 16);
    draw_speed_streaks(&mut second, 1.25, 16);
    assert_eq!(first.as_bytes(), second.as_bytes());
    assert!(first.as_bytes().chunks_exact(4).any(|pixel| pixel[0] > 0));
}

#[test]
fn health_bar_draws_filled_and_missing_sections() {
    let mut framebuffer = Framebuffer::new(20, 10);
    framebuffer.clear(Color::BLACK);
    draw_health_bar(&mut framebuffer, 10, 2, 10, 5, 0.5);

    assert_eq!(framebuffer.get_pixel(5, 2), Color::WHITE);
    assert_eq!(framebuffer.get_pixel(6, 3), Color::rgb(245, 190, 35));
    assert_eq!(framebuffer.get_pixel(10, 3), Color::rgb(100, 18, 22));
}

#[test]
fn aim_reticle_is_symmetric_and_keeps_an_inner_gap() {
    let mut framebuffer = Framebuffer::new(41, 41);
    framebuffer.clear(Color::BLACK);
    draw_aim_reticle(&mut framebuffer, 20, 20);

    let core = Color::rgb(150, 168, 205);
    assert_eq!(framebuffer.get_pixel(20, 20), core);
    assert_eq!(framebuffer.get_pixel(20, 10), core);
    assert_eq!(framebuffer.get_pixel(20, 30), core);
    assert_eq!(framebuffer.get_pixel(10, 20), core);
    assert_eq!(framebuffer.get_pixel(30, 20), core);
    assert_eq!(framebuffer.get_pixel(12, 12), core);
    assert_eq!(framebuffer.get_pixel(28, 28), core);
    assert_eq!(framebuffer.get_pixel(20, 14), Color::BLACK);
}

#[test]
fn aim_reticle_clips_safely_at_framebuffer_edges() {
    let mut framebuffer = Framebuffer::new(5, 5);
    framebuffer.clear(Color::BLACK);
    draw_aim_reticle(&mut framebuffer, 0, 0);

    assert_eq!(framebuffer.get_pixel(0, 0), Color::rgb(150, 168, 205));
    assert!(
        framebuffer
            .as_bytes()
            .chunks_exact(4)
            .any(|pixel| pixel != [0, 0, 0, 255])
    );
}
