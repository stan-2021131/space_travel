use space_travel::buffer::{Color, Framebuffer};
use space_travel::camera::Camera;
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::mesh::{AABB, Mesh, Vertex};
use space_travel::pipeline::{
    AlienEnergyShader, EmissiveShader, FragmentInput, FragmentShader, PlasmaShader,
    ProceduralRockShader, ShipShader, SkyShader, TexturedDiffuseShader, ToonShader,
};
use space_travel::texture::CpuTexture;
use std::path::Path;
use std::sync::Arc;

#[test]
fn test_plasma_shader_animation_and_thermal_range() {
    let mut shader = PlasmaShader::new();
    let mat = Material::new("SunMat", Vec3::new(1.0, 1.0, 1.0), None);
    let input = FragmentInput {
        screen_x: 100,
        screen_y: 100,
        depth_ndc: 0.0,
        world_pos: Vec3::new(10.0, 0.0, 0.0),
        world_normal: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    shader.set_time(0.0);
    let color_t0 = shader.shade(&input);

    shader.set_time(3.5);
    let color_t1 = shader.shade(&input);

    // Los colores deben ser distintos al evolucionar el tiempo
    assert_ne!(
        (color_t0.r, color_t0.g, color_t0.b),
        (color_t1.r, color_t1.g, color_t1.b),
        "El plasma solar debe variar continuamente con el tiempo"
    );

    // La tonalidad debe mantenerse en la paleta solar (rojo/naranja/amarillo, r >= g, g >= b)
    assert!(color_t0.r >= color_t0.b);
    assert!(color_t1.r >= color_t1.b);
}

#[test]
fn test_alien_energy_shader_animation_is_deterministic() {
    let mut shader = AlienEnergyShader::new(Vec3::new(0.0, 10.0, 10.0), Vec3::new(0.0, 0.0, 10.0));
    let mat = Material::new("EnemyHull", Vec3::new(0.55, 0.72, 1.0), None);
    let input = FragmentInput {
        screen_x: 12,
        screen_y: 20,
        depth_ndc: 0.0,
        world_pos: Vec3::new(0.7, 0.4, 0.0),
        world_normal: Vec3::new(0.0, 0.0, 1.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(0.2, 0.3, 0.5),
        material: &mat,
    };

    shader.set_time(0.35);
    let color_a = shader.shade(&input);
    let color_a_repeat = shader.shade(&input);
    shader.set_time(1.10);
    let color_b = shader.shade(&input);

    assert_eq!(
        color_a, color_a_repeat,
        "El mismo tiempo y fragmento deben producir exactamente el mismo color"
    );
    assert_ne!(
        color_a, color_b,
        "Las bandas energéticas deben desplazarse con el tiempo de simulación"
    );
}

#[test]
fn test_alien_energy_shader_fresnel_rim() {
    let shader = AlienEnergyShader::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(0.0, 0.0, 10.0));
    let mat = Material::new("EnemyHull", Vec3::new(0.45, 0.55, 0.70), None);
    let front_input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 0.0, 1.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let rim_input = FragmentInput {
        world_normal: Vec3::new(1.0, 0.0, 0.0),
        ..front_input.clone()
    };

    let front_color = shader.shade(&front_input);
    let rim_color = shader.shade(&rim_input);

    assert!(
        rim_color.b > front_color.b,
        "El borde de la silueta debe recibir una emisión azul más intensa"
    );
    assert_eq!(rim_color.a, 255);
}

#[test]
fn test_procedural_rock_shader_determinism_and_seed_variance() {
    let mat = Material::new("RockMat", Vec3::new(1.0, 1.0, 1.0), None);
    let input = FragmentInput {
        screen_x: 50,
        screen_y: 50,
        depth_ndc: 0.0,
        world_pos: Vec3::new(0.0, 0.0, 10.0),
        world_normal: Vec3::new(0.5, 0.5, 0.707).normalize(),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let shader_seed_67_a = ProceduralRockShader::new(67, Vec3::new(0.0, 0.0, 50.0));
    let shader_seed_67_b = ProceduralRockShader::new(67, Vec3::new(0.0, 0.0, 50.0));
    let shader_seed_999 = ProceduralRockShader::new(999, Vec3::new(0.0, 0.0, 50.0));

    let col_a = shader_seed_67_a.shade(&input);
    let col_b = shader_seed_67_b.shade(&input);
    let col_diff = shader_seed_999.shade(&input);

    // Mismo input y misma semilla -> exactamente el mismo color determinista
    assert_eq!(
        col_a, col_b,
        "La misma semilla debe producir colores 100% deterministas"
    );

    // Semilla diferente -> patrón geológico diferente
    assert_ne!(
        (col_a.r, col_a.g, col_a.b),
        (col_diff.r, col_diff.g, col_diff.b),
        "Semillas distintas deben producir patrones rocosos distintos"
    );
}

#[test]
fn test_procedural_rock_shader_axial_rotation() {
    let mat = Material::new("RockMat", Vec3::new(1.0, 1.0, 1.0), None);

    // Estado 1: Planeta sin rotación (rotation_rad = 0), evaluando punto en (10, 0, 0)
    let mut shader_r0 = ProceduralRockShader::new(67, Vec3::new(50.0, 0.0, 0.0));
    shader_r0.rotation_rad = 0.0;
    shader_r0.ambient_intensity = 1.0; // Luz ambiente pura para aislar el albedo del terreno

    let input_p0 = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::new(10.0, 0.0, 0.0),
        world_normal: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let col_p0 = shader_r0.shade(&input_p0);

    // Estado 2: Planeta rotado 90° (rotation_rad = PI/2), evaluando el punto rotado en (0, 0, -10)
    let mut shader_r90 = ProceduralRockShader::new(67, Vec3::new(0.0, 0.0, -50.0));
    shader_r90.rotation_rad = std::f32::consts::FRAC_PI_2;
    shader_r90.ambient_intensity = 1.0;

    let input_p90 = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::new(0.0, 0.0, -10.0),
        world_normal: Vec3::new(0.0, 0.0, -1.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let col_p90 = shader_r90.shade(&input_p90);

    // El elemento geológico debe haber rotado solidariamente con el planeta
    assert_eq!(
        col_p0, col_p90,
        "La geología procedural debe rotar solidariamente con la rotación axial del planeta"
    );
}

#[test]
fn test_toon_shader_quantization_and_rim() {
    let mat = Material::new("ToonMat", Vec3::new(1.0, 1.0, 1.0), None);
    let camera_pos = Vec3::new(0.0, 0.0, 10.0);
    let light_pos = Vec3::new(0.0, 0.0, 20.0);
    let shader = ToonShader::new(light_pos, camera_pos);

    // 1. Superficie de frente mirando a la luz y a la cámara: normal (0, 0, 1)
    let input_direct = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 0.0, 1.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let col_direct = shader.shade(&input_direct);
    assert_eq!(col_direct.r, 255, "Luz directa debe estar al 100%");

    // 2. Superficie en sombra propia: normal orientada opuesta a la luz
    let input_shadow = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.0, 0.0, -1.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let col_shadow = shader.shade(&input_shadow);
    assert!(
        col_shadow.r <= 75,
        "La banda de sombra propia debe estar en el escalón oscuro (<= 30%)"
    );

    // 3. Silueta / borde (Delineado de tinta negra anime)
    let input_ink = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::zero(),
        world_normal: Vec3::new(0.995, 0.0, 0.08), // n_dot_v = 0.08 (< 0.14)
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let col_ink = shader.shade(&input_ink);
    assert_eq!(
        col_ink,
        Color::new(18, 18, 24, 255),
        "El borde de la silueta debe entintarse de negro cómic"
    );

    // 4. Sol anime cel-shaded
    let mut sun_shader = ToonShader::new(Vec3::zero(), camera_pos);
    sun_shader.is_sun = true;
    let col_sun = sun_shader.shade(&input_direct);
    assert!(
        col_sun.r >= 240 && col_sun.g >= 200,
        "El sol anime debe ser una estrella radiante cálida"
    );
}

#[test]
fn test_sky_shader_determinism_and_rotation() {
    let camera_a = Camera::new(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::up(),
        50.0,
        1.333,
        0.1,
        500.0,
    );
    let camera_rotated = Camera::new(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(10.0, 0.0, 10.0), // Mirando hacia +X (giro de 90 grados)
        Vec3::up(),
        50.0,
        1.333,
        0.1,
        500.0,
    );

    let sky_seed_67_a = SkyShader::new(67);
    let sky_seed_67_b = SkyShader::new(67);
    let sky_seed_999 = SkyShader::new(999);

    let mut fb_a1 = Framebuffer::new(64, 48);
    let mut fb_a2 = Framebuffer::new(64, 48);
    let mut fb_rotated = Framebuffer::new(64, 48);
    let mut fb_seed_diff = Framebuffer::new(64, 48);

    sky_seed_67_a.render(&mut fb_a1, &camera_a);
    sky_seed_67_b.render(&mut fb_a2, &camera_a);
    sky_seed_67_a.render(&mut fb_rotated, &camera_rotated);
    sky_seed_999.render(&mut fb_seed_diff, &camera_a);

    // 1. Mismo frame, misma cámara, misma semilla -> determinismo exacto píxel a píxel
    assert_eq!(
        fb_a1.as_bytes(),
        fb_a2.as_bytes(),
        "Misma semilla debe generar cielo idéntico"
    );

    // 2. Al rotar la cámara 90 grados, el cielo 3D debe cambiar coherentemente
    assert_ne!(
        fb_a1.as_bytes(),
        fb_rotated.as_bytes(),
        "Al rotar la cámara el fondo debe cambiar en 3D"
    );

    // 3. Con semillas distintas, la distribución de estrellas debe variar
    assert_ne!(
        fb_a1.as_bytes(),
        fb_seed_diff.as_bytes(),
        "Semillas distintas deben generar constelaciones distintas"
    );
}

#[test]
fn test_ship_shader_specular_and_sun_lighting() {
    let sun_pos = Vec3::new(0.0, 0.0, 0.0);
    let camera_pos = Vec3::new(0.0, 5.0, 10.0);
    let ship_shader = ShipShader::new(sun_pos, camera_pos);

    let mat = Material::new("ShipMetal", Vec3::new(0.7, 0.7, 0.7), None);

    // Fragmento frontal hacia el Sol (en +Z del Sol, normal hacia el Sol en -Z)
    let input_front = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::new(0.0, 0.0, 20.0),
        world_normal: Vec3::new(0.0, 0.0, -1.0), // Mira directamente al Sol
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    // Fragmento posterior en sombra (normal opuesta al Sol)
    let input_back = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.0,
        world_pos: Vec3::new(0.0, 0.0, 20.0),
        world_normal: Vec3::new(0.0, 0.0, 1.0), // De espaldas al Sol
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let color_front = ship_shader.shade(&input_front);
    let color_back = ship_shader.shade(&input_back);

    // 1. La cara iluminada por el Sol debe ser notablemente más brillante que la posterior
    assert!(
        color_front.r > color_back.r && color_front.g > color_back.g,
        "La cara frontal al Sol debe recibir luz directa"
    );

    // 2. La cara posterior no debe ser negra absoluta (conserva emisión base para silueta visible)
    assert!(
        color_back.r > 20 && color_back.g > 20 && color_back.b > 20,
        "La cara posterior debe conservar brillo ambiental suficiente para no perderse en el espacio"
    );
}

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
