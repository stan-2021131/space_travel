use space_travel::buffer::{Color, DepthBuffer, Framebuffer};
use space_travel::material::Material;
use space_travel::math::{Mat4, Vec2, Vec3, Vec4, deg_to_rad};
use space_travel::pipeline::clipping::ClipVertex;
use space_travel::pipeline::rasterizer::{
    VertexOut, calculate_barycentric_2d, is_front_facing, rasterize_triangle,
};
use space_travel::pipeline::shader::{
    DepthDebugShader, DiffuseShader, FragmentInput, FragmentShader,
};
use space_travel::stats::FrameStats;

fn dummy_clip_vertex(clip_pos: Vec4) -> ClipVertex {
    ClipVertex::new(
        clip_pos,
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    )
}

#[test]
fn test_barycentric_coordinates() {
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(10.0, 0.0);
    let c = Vec2::new(0.0, 10.0);

    let (w0, w1, w2) = calculate_barycentric_2d(a, a, b, c).unwrap();
    assert!((w0 - 1.0).abs() < 1e-5);
    assert!(w1.abs() < 1e-5);
    assert!(w2.abs() < 1e-5);

    let centroid = Vec2::new(10.0 / 3.0, 10.0 / 3.0);
    let (c0, c1, c2) = calculate_barycentric_2d(centroid, a, b, c).unwrap();
    assert!((c0 - 1.0 / 3.0).abs() < 1e-5);
    assert!((c1 - 1.0 / 3.0).abs() < 1e-5);
    assert!((c2 - 1.0 / 3.0).abs() < 1e-5);

    let deg_c = Vec2::new(5.0, 0.0);
    let deg_res = calculate_barycentric_2d(centroid, a, b, deg_c);
    assert!(deg_res.is_none());
}

#[test]
fn test_backface_culling() {
    let cv0 = ClipVertex::new(
        Vec4::new(-0.5, -0.5, 0.0, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );
    let cv1 = ClipVertex::new(
        Vec4::new(0.5, -0.5, 0.0, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );
    let cv2 = ClipVertex::new(
        Vec4::new(0.0, 0.5, 0.0, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );

    let vo0 = VertexOut::from_clip_vertex(&cv0, 800.0, 600.0);
    let vo1 = VertexOut::from_clip_vertex(&cv1, 800.0, 600.0);
    let vo2 = VertexOut::from_clip_vertex(&cv2, 800.0, 600.0);

    assert!(is_front_facing(&vo0, &vo1, &vo2));
    assert!(!is_front_facing(&vo0, &vo2, &vo1));
}

#[test]
fn test_depth_buffer_test_and_set() {
    let mut db = DepthBuffer::new(10, 10);
    db.clear();

    assert_eq!(db.get(2, 3), f32::INFINITY);
    assert!(db.test_and_set(2, 3, 0.5));
    assert_eq!(db.get(2, 3), 0.5);
    assert!(!db.test_and_set(2, 3, 0.8));
    assert_eq!(db.get(2, 3), 0.5);
    assert!(db.test_and_set(2, 3, 0.2));
    assert_eq!(db.get(2, 3), 0.2);
}

#[test]
fn test_depth_debug_shader_linearization() {
    let shader = DepthDebugShader::new(0.1, 100.0, 2.5, 6.5);
    let z_near_lin = shader.linearize_depth(-1.0);
    assert!((z_near_lin - 0.1).abs() < 1e-4);

    let z_far_lin = shader.linearize_depth(1.0);
    assert!((z_far_lin - 100.0).abs() < 1e-2);

    let p_mat = Mat4::perspective(deg_to_rad(45.0), 1.0, 0.1, 100.0);
    let clip_close = p_mat.transform_vec4(Vec4::new(0.0, 0.0, -3.5, 1.0));
    let clip_far = p_mat.transform_vec4(Vec4::new(0.0, 0.0, -5.5, 1.0));

    let mat = Material::default_diffuse();
    let frag_close = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: clip_close.z / clip_close.w,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };
    let frag_far = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: clip_far.z / clip_far.w,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let color_close = shader.shade(&frag_close);
    let color_far = shader.shade(&frag_far);

    assert!(color_close.r > color_far.r + 50);
}

#[test]
fn test_viewport_transform_and_perspective_divide() {
    let cv = ClipVertex::new(
        Vec4::new(2.0, -2.0, 1.0, 2.0),
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::new(0.5, 0.5),
    );
    let vo = VertexOut::from_clip_vertex(&cv, 800.0, 600.0);

    // ndc_x = 2.0 / 2.0 = 1.0 -> screen_x = (1.0 + 1.0) * 0.5 * 800 = 800.0
    assert!((vo.screen_pos.x - 800.0).abs() < 1e-4);
    // ndc_y = -2.0 / 2.0 = -1.0 -> screen_y = (1.0 - (-1.0)) * 0.5 * 600 = 600.0
    assert!((vo.screen_pos.y - 600.0).abs() < 1e-4);
    // depth_ndc = 1.0 / 2.0 = 0.5
    assert!((vo.depth_ndc - 0.5).abs() < 1e-4);
    // inv_w = 0.5
    assert!((vo.inv_w - 0.5).abs() < 1e-4);
}

#[test]
fn test_rasterize_triangle_outside_screen_cull() {
    let mut fb = Framebuffer::new(100, 100);
    let mut db = DepthBuffer::new(100, 100);
    let mat = Material::default_diffuse();
    let shader = DiffuseShader::new();
    let mut stats = FrameStats::default();

    let cv0 = ClipVertex::new(
        Vec4::new(-100.0, 50.0, 0.5, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );
    let cv1 = ClipVertex::new(
        Vec4::new(-20.0, 20.0, 0.5, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );
    let cv2 = ClipVertex::new(
        Vec4::new(-50.0, 80.0, 0.5, 1.0),
        Vec3::zero(),
        Vec3::up(),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::zero(),
    );

    let mut vo0 = VertexOut::from_clip_vertex(&cv0, 100.0, 100.0);
    let mut vo1 = VertexOut::from_clip_vertex(&cv1, 100.0, 100.0);
    let mut vo2 = VertexOut::from_clip_vertex(&cv2, 100.0, 100.0);
    vo0.screen_pos = Vec2::new(-100.0, 50.0);
    vo1.screen_pos = Vec2::new(-20.0, 20.0);
    vo2.screen_pos = Vec2::new(-50.0, 80.0);

    rasterize_triangle(
        &vo0, &vo1, &vo2, &mut fb, &mut db, &mat, &shader, &mut stats,
    );

    assert_eq!(
        stats.fragmentos_evaluados, 0,
        "No debe evaluar fragmentos para triángulos fuera de pantalla"
    );
    assert_eq!(stats.fragmentos_aceptados_depth, 0);
    assert_eq!(
        stats.triangulos_rasterizados, 0,
        "No debe contabilizar como rasterizado un triángulo descartado fuera de pantalla"
    );
}

#[test]
fn test_rasterize_triangle_pipeline_execution() {
    let mut fb = Framebuffer::new(100, 100);
    let mut db = DepthBuffer::new(100, 100);
    let mat = Material::default_diffuse();
    let shader = DiffuseShader::new();
    let mut stats = FrameStats::default();

    let mut vo0 = VertexOut::from_clip_vertex(
        &dummy_clip_vertex(Vec4::new(0.0, 0.0, 0.2, 1.0)),
        100.0,
        100.0,
    );
    let mut vo1 = VertexOut::from_clip_vertex(
        &dummy_clip_vertex(Vec4::new(0.0, 0.0, 0.2, 1.0)),
        100.0,
        100.0,
    );
    let mut vo2 = VertexOut::from_clip_vertex(
        &dummy_clip_vertex(Vec4::new(0.0, 0.0, 0.2, 1.0)),
        100.0,
        100.0,
    );

    vo0.screen_pos = Vec2::new(30.0, 70.0);
    vo1.screen_pos = Vec2::new(70.0, 70.0);
    vo2.screen_pos = Vec2::new(50.0, 30.0);
    vo0.depth_ndc = 0.2;
    vo1.depth_ndc = 0.2;
    vo2.depth_ndc = 0.2;

    assert!(
        is_front_facing(&vo0, &vo1, &vo2),
        "Triángulo de prueba debe ser frontal"
    );

    rasterize_triangle(
        &vo0, &vo1, &vo2, &mut fb, &mut db, &mat, &shader, &mut stats,
    );

    assert!(
        stats.fragmentos_evaluados > 0,
        "Debe evaluar fragmentos dentro del bounding box"
    );
    assert!(
        stats.fragmentos_aceptados_depth > 0,
        "Debe rasterizar fragmentos que superen el depth test"
    );
    assert!((db.get(50, 50) - 0.2).abs() < 1e-4);
    assert_ne!(fb.get_pixel(50, 50), Color::BLACK);
}

#[test]
fn test_depth_debug_shader_far_body_visibility() {
    let near = 0.1;
    let far = 500.0;
    let shader = DepthDebugShader::new(near, far, near, far * 0.70);

    let p_mat = Mat4::perspective(deg_to_rad(50.0), 1.333, near, far);
    let sun_clip = p_mat.transform_vec4(Vec4::new(0.0, 0.0, -200.0, 1.0));
    let sun_depth_ndc = sun_clip.z / sun_clip.w;

    let mat = Material::default_diffuse();
    let frag_sun = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: sun_depth_ndc,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(1.0, 0.0, 0.0),
        material: &mat,
    };

    let color_sun = shader.shade(&frag_sun);
    assert!(
        color_sun.r > 40,
        "El cuerpo a 200 WU debe ser claramente visible en escala de grises (brillo r={}), no negro",
        color_sun.r
    );
}

#[test]
fn test_buffer_horizontal_splitting() {
    let mut fb = Framebuffer::new(80, 60);
    let mut db = DepthBuffer::new(80, 60);

    let bands = space_travel::buffer::split_render_bands(&mut fb, &mut db, 4);
    assert_eq!(bands.len(), 4);

    let mut current_y = 0;
    for band in bands {
        assert_eq!(band.width, 80);
        assert_eq!(band.height, 60);
        assert_eq!(band.y_start, current_y);
        assert!(band.y_end > band.y_start);
        assert_eq!(band.pixels.len(), (band.y_end - band.y_start) * 80);
        assert_eq!(band.depth.len(), (band.y_end - band.y_start) * 80);
        current_y = band.y_end;
    }
    assert_eq!(
        current_y, 60,
        "Todas las bandas deben cubrir exactamente [0, 60)"
    );
}

#[test]
fn test_multithreaded_vs_singlethreaded_pixel_identity() {
    use space_travel::mesh::{Mesh, Vertex};
    use space_travel::pipeline::{RenderContext, RenderTarget, SoftwareRenderer};

    // Crear una malla triangular simple con 2 caras
    let v0 = Vertex::new(
        Vec3::new(-1.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.0, 0.0),
    );
    let v1 = Vertex::new(
        Vec3::new(1.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(1.0, 0.0),
    );
    let v2 = Vertex::new(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.5, 1.0),
    );
    let v3 = Vertex::new(
        Vec3::new(-0.5, 0.0, -0.5),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.25, 0.5),
    );

    let mesh = Mesh::new(vec![v0, v1, v2, v3], vec![0, 1, 2, 0, 2, 3]);

    let model = Mat4::IDENTITY;
    let rot = Mat4::IDENTITY;
    let vp = Mat4::perspective(deg_to_rad(60.0), 1.0, 0.1, 50.0)
        * Mat4::look_at(Vec3::new(0.0, 0.0, 3.0), Vec3::zero(), Vec3::up());
    let mat = Material::default_diffuse();
    let mut shader = DiffuseShader::new();
    shader.light_dir = Vec3::new(2.0, 3.0, 5.0).normalize();

    let ctx = RenderContext::new(&model, &rot, &vp, &mat, &shader);

    // 1. Renderizado monohilo (1 banda)
    let mut fb_single = Framebuffer::new(64, 64);
    let mut db_single = DepthBuffer::new(64, 64);
    let mut stats_single = FrameStats::default();
    {
        let mut target_single = RenderTarget {
            framebuffer: &mut fb_single,
            depth_buffer: &mut db_single,
        };
        let mut renderer_single = SoftwareRenderer::new();
        renderer_single.render_mesh(&mesh, &ctx, &mut target_single, &mut stats_single);
    }

    // 2. Renderizado multihilo normal
    let mut fb_multi = Framebuffer::new(64, 64);
    let mut db_multi = DepthBuffer::new(64, 64);
    let mut stats_multi = FrameStats::default();
    {
        let mut target_multi = RenderTarget {
            framebuffer: &mut fb_multi,
            depth_buffer: &mut db_multi,
        };
        let mut renderer_multi = SoftwareRenderer::new();
        renderer_multi.render_mesh(&mesh, &ctx, &mut target_multi, &mut stats_multi);
    }

    assert!(stats_single.fragmentos_evaluados > 0);
    assert_eq!(
        stats_single.fragmentos_evaluados,
        stats_multi.fragmentos_evaluados
    );
    assert_eq!(
        stats_single.fragmentos_aceptados_depth,
        stats_multi.fragmentos_aceptados_depth
    );

    // Comparación pixel por pixel
    for y in 0..64 {
        for x in 0..64 {
            let col_s = fb_single.get_pixel(x, y);
            let col_m = fb_multi.get_pixel(x, y);
            assert_eq!(
                col_s, col_m,
                "Discrepancia de color en ({}, {}): single={:?}, multi={:?}",
                x, y, col_s, col_m
            );

            let d_s = db_single.get(x, y);
            let d_m = db_multi.get(x, y);
            assert!(
                d_s == d_m || (d_s - d_m).abs() < 1e-5,
                "Discrepancia de profundidad en ({}, {}): single={}, multi={}",
                x,
                y,
                d_s,
                d_m
            );
        }
    }
}
