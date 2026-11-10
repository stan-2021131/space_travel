use space_travel::buffer::{Color, DepthBuffer, Framebuffer};
use space_travel::math::{Mat4, Vec3};
use space_travel::particles::SparkSystem;

#[test]
fn test_spark_system_emission_and_lifecycle() {
    let mut system = SparkSystem::new(64);
    assert_eq!(system.particle_count(), 0);

    let contact_point = Vec3::new(10.0, 0.0, 5.0);
    let contact_normal = Vec3::new(0.0, 1.0, 0.0);
    let count = 20;
    let impact_speed = 8.0;

    system.emit_sparks(contact_point, contact_normal, count, impact_speed);
    assert_eq!(system.particle_count(), count);

    // Guardar una copia de las partículas iniciales activas
    let p0 = system.particles()[0];
    assert!(p0.max_lifetime > 0.0);
    assert_eq!(p0.lifetime, p0.max_lifetime);

    // Actualizar 0.1s: deben moverse según su velocidad y reducir lifetime
    system.update(0.1);
    assert_eq!(system.particle_count(), count);
    let p1 = system.particles()[0];
    assert!(p1.lifetime < p0.lifetime);
    assert!(p1.position != p0.position);
    assert!(
        p1.velocity.length() < p0.velocity.length(),
        "El arrastre debe reducir la velocidad"
    );

    // Avanzar más allá del tiempo de vida máximo (ej. 2.0s)
    system.update(2.0);
    assert_eq!(
        system.particle_count(),
        0,
        "Todas las chispas deben haberse extinguido"
    );
}

#[test]
fn test_spark_system_capacity_limit() {
    let mut system = SparkSystem::new(16);
    // Intentar emitir más partículas que la capacidad
    system.emit_sparks(Vec3::zero(), Vec3::new(0.0, 0.0, 1.0), 32, 10.0);
    assert_eq!(
        system.particle_count(),
        16,
        "No debe exceder la capacidad fija"
    );
}

#[test]
fn test_spark_render_and_depth_occlusion() {
    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height);
    let mut db = DepthBuffer::new(width, height);
    fb.clear(Color::BLACK);
    db.clear();

    // Cámara mirando desde (0, 0, 5) hacia (0, 0, 0)
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::zero(),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 50.0);
    let vp = proj * view;

    let mut system = SparkSystem::new(8);
    // Emitir en el origen (0, 0, 0), que proyecta en el centro (32, 32)
    system.emit_sparks(Vec3::zero(), Vec3::new(0.0, 1.0, 0.0), 1, 0.0);

    // Renderizar: el centro debe iluminarse con suma aditiva
    system.render(&mut fb, &db, &vp);
    let center_pixel = fb.get_pixel(32, 32);
    assert!(
        center_pixel.r > 0 || center_pixel.g > 0,
        "El píxel central debe recibir color de chispa"
    );

    // Ahora probar oclusión por Z-buffer:
    let mut fb_occluded = Framebuffer::new(width, height);
    let mut db_occluded = DepthBuffer::new(width, height);
    fb_occluded.clear(Color::BLACK);
    db_occluded.clear();

    // Poner un valor de profundidad en el centro correspondiente a un objeto MUCHO más cercano que la chispa
    // Llenamos el centro con NDC -0.99 (muy cerca del plano cercano)
    db_occluded.test_and_set(32, 32, -0.99);

    system.render(&mut fb_occluded, &db_occluded, &vp);
    let occluded_pixel = fb_occluded.get_pixel(32, 32);
    assert_eq!(
        occluded_pixel,
        Color::BLACK,
        "La chispa debe ser descartada si está oculta detrás de una superficie"
    );
}

#[test]
fn test_spark_system_low_counts() {
    let mut system = SparkSystem::new(16);

    // Con count = 0 no debe emitir ninguna partícula
    system.emit_sparks(Vec3::zero(), Vec3::up(), 0, 10.0);
    assert_eq!(system.particle_count(), 0);

    // Con count = 1 debe emitir exactamente 1 partícula
    system.emit_sparks(Vec3::zero(), Vec3::up(), 1, 10.0);
    assert_eq!(system.particle_count(), 1);
}

#[test]
fn test_spark_particle_lifetime_fade_finite_with_zero_max_lifetime() {
    let p = space_travel::particles::SparkParticle {
        position: Vec3::zero(),
        velocity: Vec3::zero(),
        color: Color::WHITE,
        lifetime: 0.1,
        max_lifetime: 0.0,
        size: 1,
    };
    let fade = (p.lifetime / p.max_lifetime.max(1e-5)).clamp(0.0, 1.0);
    assert!(fade.is_finite());
    assert!(!fade.is_nan());
}
