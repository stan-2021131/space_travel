use space_travel::Mesh;
use space_travel::buffer::Color;
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::pipeline::shaders::{FragmentInput, FragmentShader, NormalMappedShader};
use space_travel::texture::CpuTexture;
use std::sync::Arc;

#[test]
fn test_normal_mapping_flat_texel_identity() {
    // Texel azul neutro (128, 128, 255) representa normal no perturbada (0, 0, 1) en espacio tangente
    let norm_tex = Arc::new(CpuTexture::solid(2, 2, Color::new(128, 128, 255, 255)));
    let mat = Material::with_normal_map("TestMat", Vec3::new(1.0, 1.0, 1.0), None, Some(norm_tex));

    let light_pos = Vec3::new(0.0, 10.0, 0.0);
    let camera_pos = Vec3::new(0.0, 10.0, 10.0);
    let shader = NormalMappedShader::new(light_pos, camera_pos);

    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 1.0, 0.0),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let col = shader.shade(&input);
    // Con luz directamente arriba en (0, 10, 0) y normal apuntando a (0, 1, 0), n_dot_l ≈ 1.0
    assert!(
        col.r > 200,
        "La normal no perturbada (128, 128, 255) debe recibir iluminación difusa frontal completa"
    );
}

#[test]
fn test_normal_mapping_tangent_deviation_redirects_lighting() {
    // Normal map con texel inclinado hacia la tangente +X:
    // R = 255, G = 128, B = 128 -> (1.0, 0.0, 0.0) en espacio tangente
    let norm_tex_tilted = Arc::new(CpuTexture::solid(2, 2, Color::new(255, 128, 128, 255)));
    let mat_tilted = Material::with_normal_map(
        "TiltedMat",
        Vec3::new(1.0, 1.0, 1.0),
        None,
        Some(norm_tex_tilted),
    );

    // Luz lateral situada en +X (10, 0, 0)
    let light_pos = Vec3::new(10.0, 0.0, 0.0);
    let camera_pos = Vec3::new(0.0, 10.0, 10.0);
    let shader = NormalMappedShader::new(light_pos, camera_pos);

    // Superficie plana mirando hacia arriba +Y (0, 1, 0), con tangente en +X (1, 0, 0)
    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 1.0, 0.0),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat_tilted,
    };

    let col_tilted = shader.shade(&input);

    // Sin normal map (normal geométrica = (0, 1, 0)), una luz en (10, 0, 0) daría n_dot_l = 0.0 (solo luz ambiente)
    let mat_flat = Material::default_diffuse();
    let mut input_flat = input.clone();
    input_flat.material = &mat_flat;
    let col_flat = shader.shade(&input_flat);

    // La superficie con normal map inclinado hacia +X debe recibir significativamente más luz lateral
    assert!(
        col_tilted.r > col_flat.r + 50,
        "El normal map inclinado debe captar luz lateral (+X) mucho más intensamente que la cara plana"
    );
}

#[test]
fn test_mesh_calculated_tangents_are_orthonormal() {
    let mut mesh = Mesh::load_from_obj(std::path::Path::new("assets/models/rocky/rocky.obj"))
        .expect("Debe cargar rocky.obj");
    mesh.calculate_tangents();

    for (i, v) in mesh.vertices.iter().enumerate() {
        let t_len = v.tangent.length();
        assert!(
            (t_len - 1.0).abs() < 1e-4,
            "Vértice {}: longitud de tangente ({}) debe ser 1.0",
            i,
            t_len
        );

        let dot = v.tangent.dot(v.normal);
        assert!(
            dot.abs() < 1e-4,
            "Vértice {}: la tangente calculada debe ser ortogonal a la normal (dot = {})",
            i,
            dot
        );
    }
}

#[test]
fn test_normal_mapped_shader_fallback_without_texture() {
    let mat = Material::default_diffuse();
    let shader = NormalMappedShader::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 5.0, 5.0));

    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 1.0, 0.0),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let col = shader.shade(&input);
    assert!(
        col.r > 0 && col.g > 0 && col.b > 0,
        "El shader debe calcular sombreado difuso válido cuando no hay textura de normales"
    );
}
