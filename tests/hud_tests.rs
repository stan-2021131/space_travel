use space_travel::buffer::{Color, Framebuffer};
use space_travel::hud::{draw_health_bar, project_world_to_screen};
use space_travel::math::{Mat4, Vec3};

#[test]
fn identity_projection_maps_origin_to_screen_center() {
    let projected = project_world_to_screen(&Mat4::IDENTITY, Vec3::zero(), 100, 80);
    assert_eq!(projected, Some((50, 40)));
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
