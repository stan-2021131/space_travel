use space_travel::buffer::{Color, RenderBand};
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::pipeline::FragmentShader;
use space_travel::pipeline::shaders::{ForceFieldShader, FragmentInput};

#[test]
fn test_fresnel_rim_increases_at_silhouette() {
    let camera_pos = Vec3::new(0.0, 0.0, 10.0);
    let shader = ForceFieldShader::new(camera_pos);
    let material = Material::default();

    // 1. Fragmento en el centro frontal: la normal apunta directamente a la cámara (N . V = 1.0)
    let center_input = FragmentInput {
        screen_x: 100,
        screen_y: 100,
        depth_ndc: 0.5,
        world_pos: Vec3::new(0.0, 0.0, 5.0),
        world_normal: Vec3::new(0.0, 0.0, 1.0),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(0.33, 0.33, 0.34),
        material: &material,
    };
    let center_color = shader.shade(&center_input);

    // 2. Fragmento en la silueta tangencial: la normal es exactamente perpendicular a la línea de visión (N . V = 0.0)
    let rim_input = FragmentInput {
        screen_x: 150,
        screen_y: 100,
        depth_ndc: 0.5,
        world_pos: Vec3::new(0.0, 0.0, 5.0),
        world_normal: Vec3::new(1.0, 0.0, 0.0), // Perpendicular a la línea de visión (0, 0, 1)
        world_tangent: Vec3::new(0.0, 1.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(0.33, 0.33, 0.34),
        material: &material,
    };
    let rim_color = shader.shade(&rim_input);

    // El borde de la silueta debe tener significativamente mayor opacidad (alfa) que el centro frontal
    assert!(
        rim_color.a > center_color.a,
        "La silueta (a={}) debe ser más opaca que el centro frontal (a={}) por el efecto Fresnel",
        rim_color.a,
        center_color.a
    );

    // El centro frontal debe ser casi invisible para permitir ver con total claridad el planeta interior
    assert!(
        center_color.a < 30,
        "El centro frontal debe tener baja opacidad (indicador sutil)"
    );
    assert!(
        rim_color.a > center_color.a * 3,
        "La silueta debe destacar como indicador visible respecto al centro"
    );
}

#[test]
fn test_force_field_alpha_blending() {
    let fondo = Color::new(20, 30, 40, 255);
    // Capa frontal translúcida con 50% de opacidad (alfa = 128)
    let frente = Color::new(100, 200, 255, 128);

    let mezclado = Color::blend(fondo, frente);

    // Valor esperado: componente ponderada
    assert!(mezclado.r > fondo.r && mezclado.r < frente.r);
    assert!(mezclado.g > fondo.g && mezclado.g < frente.g);
    assert!(mezclado.b > fondo.b && mezclado.b < frente.b);
    assert_eq!(mezclado.a, 255);
}

#[test]
fn test_force_field_custom_color() {
    let camera_pos = Vec3::new(0.0, 0.0, 10.0);
    // Configuración con tinte dorado/solar
    let shader = ForceFieldShader::new(camera_pos).with_color(Vec3::new(1.0, 0.8, 0.2));
    let material = Material::default();

    let input = FragmentInput {
        screen_x: 100,
        screen_y: 100,
        depth_ndc: 0.5,
        world_pos: Vec3::new(0.0, 0.0, 5.0),
        world_normal: Vec3::new(1.0, 0.0, 0.0),
        world_tangent: Vec3::new(0.0, 1.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(0.33, 0.33, 0.34),
        material: &material,
    };
    let color = shader.shade(&input);

    assert!(
        color.r > color.b,
        "Canal rojo debe predominar en tono dorado"
    );
    assert!(color.g > color.b, "Canal verde debe superar al azul");
}

#[test]
fn test_render_band_depth_only_preserves_buffer() {
    let mut pixels = vec![Color::BLACK; 100];
    let mut depth = vec![0.5f32; 100];

    let band = RenderBand {
        width: 10,
        height: 10,
        y_start: 0,
        y_end: 10,
        pixels: &mut pixels,
        depth: &mut depth,
    };

    // Objeto más cercano (depth = 0.3 < 0.5): pasa la prueba
    assert!(band.test_depth_only(5, 5, 0.3));
    // La prueba test_depth_only NO debe modificar el Z-buffer
    assert_eq!(
        band.depth[5 * 10 + 5],
        0.5,
        "test_depth_only no debe sobrescribir el valor de profundidad"
    );

    // Objeto más lejano (depth = 0.8 > 0.5): no pasa la prueba
    assert!(!band.test_depth_only(5, 5, 0.8));
}

#[test]
fn test_render_band_blend_pixel() {
    let mut pixels = vec![Color::new(10, 10, 10, 255); 100];
    let mut depth = vec![1.0f32; 100];

    let mut band = RenderBand {
        width: 10,
        height: 10,
        y_start: 0,
        y_end: 10,
        pixels: &mut pixels,
        depth: &mut depth,
    };

    let semitransparente = Color::new(200, 100, 50, 128);
    band.blend_pixel(3, 4, semitransparente);

    let px = band.pixels[4 * 10 + 3];
    assert!(px.r > 10, "El píxel debió ser mezclado con el nuevo color");
    assert!(px.g > 10);
    assert!(px.b > 10);
}
