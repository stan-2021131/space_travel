use space_travel::buffer::Color;
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::mesh::{AABB, Mesh, Vertex};
use space_travel::pipeline::shader::{
    EmissiveShader, FragmentInput, FragmentShader, TexturedDiffuseShader,
};
use space_travel::texture::CpuTexture;
use std::path::Path;
use std::sync::Arc;

#[test]
fn test_emissive_shader() {
    let shader = EmissiveShader::new(1.5);
    let mat = Material::new("SunMat", Vec3::new(1.0, 0.5, 0.0), None);
    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let color = shader.shade(&input);
    assert_eq!(color.r, 255);
    assert!((color.g as f32 / 255.0 - 0.75).abs() < 0.05);
}

#[test]
fn test_textured_diffuse_shader() {
    let white_tex = Arc::new(CpuTexture::solid(2, 2, Color::WHITE));
    let mat = Material::new("TexMat", Vec3::new(1.0, 1.0, 1.0), Some(white_tex));
    let shader = TexturedDiffuseShader::new(Vec3::new(0.0, 10.0, 0.0)); // Luz desde arriba

    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(), // Apunta hacia la luz -> iluminación máxima
        uv: Vec2::new(0.5, 0.5),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let color = shader.shade(&input);
    assert_eq!(color.r, 255);
    assert_eq!(color.g, 255);
    assert_eq!(color.b, 255);
}

#[test]
fn test_texture_sampling_rules() {
    // Textura 2x2:
    // Fila 0 (arriba): Rojo, Verde
    // Fila 1 (abajo):  Azul, Blanco
    let pixels = vec![Color::RED, Color::GREEN, Color::BLUE, Color::WHITE];
    let tex = CpuTexture::new(2, 2, pixels);

    // U wrap: u = 0.2 -> columna 0, u = 1.2 -> columna 0 (wrap)
    // V clamp: V = 1.0 (arriba en OBJ -> fila 0 en memoria) -> Rojo en x=0
    let sample_top_left = tex.sample(0.1, 1.0);
    assert_eq!(sample_top_left, Color::RED);

    // V = 0.0 (abajo en OBJ -> fila 1 en memoria) -> Azul en x=0
    let sample_bottom_left = tex.sample(0.1, 0.0);
    assert_eq!(sample_bottom_left, Color::BLUE);

    // V fuera de rango superior (> 1.0) se clampa a 1.0 (polo norte no invierte textura)
    let sample_clamp_over = tex.sample(0.1, 2.5);
    assert_eq!(sample_clamp_over, Color::RED);

    // V fuera de rango inferior (< 0.0) se clampa a 0.0 (polo sur no invierte textura)
    let sample_clamp_under = tex.sample(0.1, -1.0);
    assert_eq!(sample_clamp_under, Color::BLUE);

    // U repetido
    let sample_wrapped = tex.sample(3.1, 1.0);
    assert_eq!(sample_wrapped, Color::RED);
}

#[test]
fn test_load_rocky_texture_and_mtl() {
    let tex_path = Path::new("assets/models/rocky/rocky.png");
    assert!(tex_path.exists());
    let tex = CpuTexture::load_from_file(tex_path).expect("rocky.png debe cargar con éxito");
    assert_eq!(tex.width, 512);
    assert_eq!(tex.height, 512);

    let mesh = Mesh::load_from_obj(Path::new("assets/models/rocky/rocky.obj"))
        .expect("rocky.obj con MTL debe cargar con éxito");
    assert!(
        mesh.material.texture.is_some(),
        "rocky.obj debe tener textura asignada desde MTL"
    );
    assert_eq!(mesh.material.name, "Material.001");
}

#[test]
fn test_rocky_material_texture_brightness_unattenuated() {
    let rocky = Mesh::load_from_obj(Path::new("assets/models/rocky/rocky.obj")).unwrap();
    assert!(rocky.material.texture.is_some());
    assert!(
        (rocky.material.kd.x - 1.0).abs() < 1e-3,
        "Kd.x debe ser unitario (1.0), pero es {}",
        rocky.material.kd.x
    );
    assert!(
        (rocky.material.kd.y - 1.0).abs() < 1e-3,
        "Kd.y debe ser unitario (1.0), pero es {}",
        rocky.material.kd.y
    );
    assert!(
        (rocky.material.kd.z - 1.0).abs() < 1e-3,
        "Kd.z debe ser unitario (1.0), pero es {}",
        rocky.material.kd.z
    );
}

#[test]
fn test_load_all_models_integrity() {
    let nave = Mesh::load_from_obj(Path::new("assets/models/nave.obj")).unwrap();
    let rocky = Mesh::load_from_obj(Path::new("assets/models/rocky/rocky.obj")).unwrap();
    let sun = Mesh::load_from_obj(Path::new("assets/models/sun/sun.obj")).unwrap();

    assert!(nave.vertex_count() > 0 && nave.triangle_count() > 0);
    assert!(rocky.vertex_count() > 0 && rocky.triangle_count() > 0);
    assert!(sun.vertex_count() > 0 && sun.triangle_count() > 0);

    assert!((rocky.geometric_radius() - 1.0).abs() < 1e-4);
    assert!((sun.geometric_radius() - 1.0).abs() < 1e-4);
}

#[test]
fn test_aabb_calculations() {
    let pts = [
        Vec3::new(-2.0, 1.0, 3.0),
        Vec3::new(4.0, -5.0, 2.0),
        Vec3::new(0.0, 3.0, -1.0),
    ];
    let aabb = AABB::from_points(&pts);

    assert_eq!(aabb.min, Vec3::new(-2.0, -5.0, -1.0));
    assert_eq!(aabb.max, Vec3::new(4.0, 3.0, 3.0));
    assert_eq!(aabb.size(), Vec3::new(6.0, 8.0, 4.0));
    assert_eq!(aabb.center(), Vec3::new(1.0, -1.0, 1.0));
    assert_eq!(aabb.max_extent(), 8.0);
}

#[test]
fn test_preview_transform_logic() {
    let v0 = Vertex::new(Vec3::new(-10.0, -10.0, 0.0), Vec3::up(), Default::default());
    let v1 = Vertex::new(Vec3::new(10.0, -10.0, 0.0), Vec3::up(), Default::default());
    let v2 = Vertex::new(Vec3::new(0.0, 10.0, 0.0), Vec3::up(), Default::default());

    let mesh = Mesh::new(vec![v0, v1, v2], vec![0, 1, 2]);
    assert_eq!(mesh.aabb.max_extent(), 20.0);

    let preview = mesh.calculate_preview_transform(2.0);
    assert!((preview.scale.x - 0.2).abs() < 1e-5);
    assert!((preview.scale.y - 0.2).abs() < 1e-5);
    assert!((preview.scale.z - 0.2).abs() < 1e-5);
}
