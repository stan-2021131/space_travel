use space_travel::camera::{Camera, Plane};
use space_travel::math::Vec3;
use space_travel::stats::{FrameStats, StatsDisplayMode, StatsTracker};

#[test]
fn test_plane_signed_distance_and_normalization() {
    // Plano Y = 5 orientado hacia +Y (arriba)
    // Coeficientes no normalizados: 0*x + 2*y + 0*z - 10 = 0
    let plane = Plane::from_coefficients(0.0, 2.0, 0.0, -10.0);

    // Debe haberse normalizado a normal (0, 1, 0) y distancia -5
    assert!((plane.normal.x - 0.0).abs() < 1e-5);
    assert!((plane.normal.y - 1.0).abs() < 1e-5);
    assert!((plane.normal.z - 0.0).abs() < 1e-5);
    assert!((plane.distance - (-5.0)).abs() < 1e-5);

    // Punto por encima (Y = 8): distancia +3
    assert!((plane.signed_distance(Vec3::new(0.0, 8.0, 0.0)) - 3.0).abs() < 1e-5);
    // Punto en el plano (Y = 5): distancia 0
    assert!(plane.signed_distance(Vec3::new(2.0, 5.0, -4.0)).abs() < 1e-5);
    // Punto por debajo (Y = 2): distancia -3
    assert!((plane.signed_distance(Vec3::new(0.0, 2.0, 0.0)) - (-3.0)).abs() < 1e-5);
}

#[test]
fn test_frustum_planes_extraction_and_inward_orientation() {
    // Cámara situada en (0, 0, 100) mirando hacia el origen (0, 0, 0)
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 100.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        1.333,
        1.0,
        500.0,
    );

    let frustum = camera.frustum();
    assert_eq!(frustum.planes.len(), 6);

    // Todos los planos deben tener normales unitarias
    for (i, plane) in frustum.planes.iter().enumerate() {
        let len = plane.normal.length();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Plano {} debe tener normal unitaria, longitud: {}",
            i,
            len
        );
    }

    // El punto objetivo (0, 0, 0) está a 100 WU de distancia frente a la cámara (entre near=1 y far=500),
    // por lo que debe estar estrictamente dentro de los 6 planos (distancia con signo positiva).
    let target = Vec3::new(0.0, 0.0, 0.0);
    for (i, plane) in frustum.planes.iter().enumerate() {
        let dist = plane.signed_distance(target);
        assert!(
            dist > 0.0,
            "El punto central objetivo debe estar dentro del plano {}, distancia: {}",
            i,
            dist
        );
    }
}

#[test]
fn test_frustum_sphere_culling_inside_and_outside() {
    // Cámara en (0, 0, 100) mirando a -Z hacia (0, 0, 0)
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 100.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        1.0,
        1.0,
        300.0,
    );
    let frustum = camera.frustum();

    // 1. Objeto centrado en el campo de visión (0, 0, 0) con radio 15.0 (como el planeta)
    assert!(
        frustum.intersects_sphere(Vec3::new(0.0, 0.0, 0.0), 15.0),
        "El planeta centrado en el cono debe ser visible"
    );

    // 2. Objeto completamente a espaldas de la cámara (en Z = 200, cámara en Z = 100 mirando a -Z)
    assert!(
        !frustum.intersects_sphere(Vec3::new(0.0, 0.0, 200.0), 15.0),
        "Un objeto a espaldas de la cámara debe descartarse"
    );

    // 3. Objeto lateral fuera de pantalla a la derecha (X = 300, Z = 0)
    assert!(
        !frustum.intersects_sphere(Vec3::new(300.0, 0.0, 0.0), 15.0),
        "Un objeto muy a la derecha debe descartarse"
    );

    // 4. Objeto lateral fuera de pantalla a la izquierda (X = -300, Z = 0)
    assert!(
        !frustum.intersects_sphere(Vec3::new(-300.0, 0.0, 0.0), 15.0),
        "Un objeto muy a la izquierda debe descartarse"
    );

    // 5. Objeto muy arriba fuera de pantalla (Y = 300, Z = 0)
    assert!(
        !frustum.intersects_sphere(Vec3::new(0.0, 300.0, 0.0), 15.0),
        "Un objeto muy arriba debe descartarse"
    );

    // 6. Objeto más allá del plano lejano (Z = -500, límite far = 300 -> pos Z = -200)
    assert!(
        !frustum.intersects_sphere(Vec3::new(0.0, 0.0, -500.0), 15.0),
        "Un objeto más allá del plano lejano debe descartarse"
    );
}

#[test]
fn test_frustum_grazing_sphere_intersection() {
    // Cámara en (0, 0, 50) mirando a (0, 0, 0)
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        1.0,
        1.0,
        200.0,
    );
    let frustum = camera.frustum();

    // Supongamos que en Z = 0 el semiancho del frustum es aproximadamente 50 * tan(30°) ≈ 28.8 WU.
    // Un objeto pequeño (r = 1.0) en X = 35 está fuera:
    assert!(!frustum.intersects_sphere(Vec3::new(35.0, 0.0, 0.0), 1.0));

    // Pero si el objeto es grande (r = 10.0) centrado en X = 35, su borde izquierdo penetra hasta 25 WU,
    // por lo que intersecta el frustum y NO debe recortarse:
    assert!(frustum.intersects_sphere(Vec3::new(35.0, 0.0, 0.0), 10.0));
}

#[test]
fn test_frame_stats_frustum_culling_accumulation() {
    let mut stats = FrameStats::default();
    assert_eq!(stats.objetos_descartados_frustum, 0);
    assert_eq!(stats.triangulos_descartados_frustum, 0);

    // Simulamos el descarte de 1 Sol (1280 tris) y 2 montañas (3016 tris)
    stats.objetos_descartados_frustum += 3;
    stats.triangulos_descartados_frustum += 1280 + 3016;

    let mut tracker = StatsTracker::new();
    tracker.display_mode = StatsDisplayMode::Detailed;
    tracker.record_frame(stats, 15.0, "rocoso", 20.0);

    assert_eq!(stats.objetos_descartados_frustum, 3);
    assert_eq!(stats.triangulos_descartados_frustum, 4296);
}

#[test]
fn test_surface_instance_bounding_sphere_enclosure() {
    // Cámara en (0, 0, 50) mirando hacia el origen (0, 0, 0)
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        60.0,
        1.0,
        1.0,
        200.0,
    );
    let frustum = camera.frustum();

    // En Z = 0, el semiancho del frustum es aprox 50 * tan(30°) ≈ 28.87 WU.
    // Una montaña con base en X = 33.0 (distancia perpendicular al plano ≈ -3.58 WU):
    let base_pos = Vec3::new(33.0, 0.0, 0.0);
    let scale = 0.055;
    let mesh_extent = 107.0;

    // Con radio subestimado (* 0.5 ≈ 2.94 WU), la esfera no alcanza el plano (-3.58 < -2.94) y se descarta prematuramente:
    let underestimated_radius = mesh_extent * scale * 0.5;
    assert!(!frustum.intersects_sphere(base_pos, underestimated_radius));

    // Con el radio completo (mesh_extent * scale ≈ 5.88 WU), la esfera penetra en el frustum (-3.58 > -5.88), cubriendo la cima visible:
    let full_radius = mesh_extent * scale;
    assert!(frustum.intersects_sphere(base_pos, full_radius));
}
