pub mod app;
pub mod buffer;
pub mod camera;
pub mod math;
pub mod mesh;
pub mod pipeline;
pub mod stats;

pub use app::App;
pub use buffer::{Color, DepthBuffer, Framebuffer};
pub use camera::Camera;
pub use math::{Mat4, Transform, Vec2, Vec3, Vec4};
pub use mesh::{AABB, Mesh, Vertex};
pub use pipeline::{
    DepthDebugShader, DiffuseShader, FragmentInput, FragmentShader, NormalsDebugShader,
    RenderContext, RenderTarget, ShaderMode, SoftwareRenderer,
};
pub use stats::{FrameStats, StatsTracker};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::deg_to_rad;
    use crate::pipeline::clipping::{
        ClipTriangle, ClipVertex, ClippedTriangles, clip_triangle_near_plane,
    };
    use crate::pipeline::rasterizer::{
        VertexOut, calculate_barycentric_2d, is_front_facing, rasterize_triangle,
    };
    use std::path::Path;

    // --- PRUEBAS DE VECTORES Y CASOS LÍMITE ---
    #[test]
    fn test_vec3_operations_and_edge_cases() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        // Suma y resta
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));

        // Multiplicación escalar y división
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(a / 2.0, Vec3::new(0.5, 1.0, 1.5));

        // Caso límite: división por cero no produce pánico ni valores descontrolados
        let div_zero = a / 0.0;
        assert_eq!(div_zero, Vec3::zero());

        // Producto punto
        assert_eq!(a.dot(b), 32.0);

        // Producto cruz
        let cross = a.cross(b);
        assert!((cross.dot(a)).abs() < 1e-6);
        assert!((cross.dot(b)).abs() < 1e-6);

        // Normalización
        let unit = a.normalize();
        assert!((unit.length() - 1.0).abs() < 1e-6);

        // Caso límite: vector nulo normalizado
        let zero_norm = Vec3::zero().normalize();
        assert_eq!(zero_norm, Vec3::zero());
    }

    #[test]
    fn test_vec2_operations() {
        let v1 = Vec2::new(3.0, 4.0);
        assert_eq!(v1.length(), 5.0);
        assert_eq!(v1.normalize(), Vec2::new(0.6, 0.8));

        // División por cero en Vec2
        let v_zero = v1 / 0.0;
        assert_eq!(v_zero, Vec2::zero());
    }

    // --- PRUEBAS DE MATRICES Y PROYECCIÓN ---
    #[test]
    fn test_matrix_identity_and_translation() {
        let id = Mat4::IDENTITY;
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(id.transform_point(p), p);

        let t = Mat4::translation(10.0, -5.0, 2.0);
        let transformed = t.transform_point(p);
        assert_eq!(transformed, Vec3::new(11.0, -3.0, 5.0));

        // Las direcciones no se trasladan
        let d = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(t.transform_direction(d), d);
    }

    #[test]
    fn test_matrix_rotations_and_euler() {
        let rot_y = Mat4::rotation_y(deg_to_rad(90.0));
        let fwd = Vec3::new(0.0, 0.0, 1.0);
        let rotated = rot_y.transform_direction(fwd);
        assert!((rotated.x - 1.0).abs() < 1e-5);
        assert!(rotated.y.abs() < 1e-5);
        assert!(rotated.z.abs() < 1e-5);

        let t = Mat4::translation(1.0, 2.0, 3.0);
        let s = Mat4::uniform_scaling(2.0);
        let m = t * s;

        let p = Vec3::new(1.0, 1.0, 1.0);
        let res = m.transform_point(p);
        assert_eq!(res, Vec3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn test_perspective_projection_conventions() {
        let fov = deg_to_rad(90.0);
        let aspect = 1.0;
        let z_near = 1.0;
        let z_far = 100.0;

        let p_mat = Mat4::perspective(fov, aspect, z_near, z_far);

        // Plano cercano en espacio de vista: z = -z_near
        let near_pt = Vec4::new(0.0, 0.0, -z_near, 1.0);
        let near_clip = p_mat.transform_vec4(near_pt);
        let near_ndc_z = near_clip.z / near_clip.w;
        assert!((near_ndc_z - (-1.0)).abs() < 1e-5);
        assert!((near_clip.w - z_near).abs() < 1e-5);

        // Plano lejano en espacio de vista: z = -z_far
        let far_pt = Vec4::new(0.0, 0.0, -z_far, 1.0);
        let far_clip = p_mat.transform_vec4(far_pt);
        let far_ndc_z = far_clip.z / far_clip.w;
        assert!((far_ndc_z - 1.0).abs() < 1e-5);
    }

    // --- PRUEBAS DE CLIPPING CONTRA EL PLANO CERCANO ---
    fn dummy_clip_vertex(clip_pos: Vec4) -> ClipVertex {
        ClipVertex::new(clip_pos, Vec3::zero(), Vec3::up(), Vec2::zero())
    }

    #[test]
    fn test_clipping_all_inside() {
        let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, 1.0, 2.0));
        let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, 1.0, 2.0));
        let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, 1.0, 2.0));

        let tri = ClipTriangle::new(v0, v1, v2);
        match clip_triangle_near_plane(&tri) {
            ClippedTriangles::One(_) => {}
            _ => panic!("Se esperaba 1 triángulo intacto"),
        }
    }

    #[test]
    fn test_clipping_all_outside() {
        let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, -5.0, 2.0));
        let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, -5.0, 2.0));
        let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, -5.0, 2.0));

        let tri = ClipTriangle::new(v0, v1, v2);
        match clip_triangle_near_plane(&tri) {
            ClippedTriangles::Zero => {}
            _ => panic!("Se esperaba descarte total (0 triángulos)"),
        }
    }

    #[test]
    fn test_clipping_one_inside_two_outside() {
        let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, 1.0, 2.0)); // d = +3
        let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, -5.0, 2.0)); // d = -3
        let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, -5.0, 2.0)); // d = -3

        let tri = ClipTriangle::new(v0, v1, v2);
        match clip_triangle_near_plane(&tri) {
            ClippedTriangles::One(new_tri) => {
                assert_eq!(new_tri.v[0].clip_pos, v0.clip_pos);
                let d1 = new_tri.v[1].clip_pos.z + new_tri.v[1].clip_pos.w;
                let d2 = new_tri.v[2].clip_pos.z + new_tri.v[2].clip_pos.w;
                assert!(d1.abs() < 1e-5, "d1 debe ser aprox 0: {}", d1);
                assert!(d2.abs() < 1e-5, "d2 debe ser aprox 0: {}", d2);
            }
            _ => panic!("Se esperaba 1 triángulo recortado"),
        }
    }

    #[test]
    fn test_clipping_two_inside_one_outside() {
        let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, 1.0, 2.0)); // d = +3
        let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, 1.0, 2.0)); // d = +3
        let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, -5.0, 2.0)); // d = -3

        let tri = ClipTriangle::new(v0, v1, v2);
        match clip_triangle_near_plane(&tri) {
            ClippedTriangles::Two(t1, t2) => {
                assert_eq!(t1.v[0].clip_pos, v0.clip_pos);
                assert_eq!(t1.v[1].clip_pos, v1.clip_pos);
                assert_eq!(t2.v[0].clip_pos, v1.clip_pos);
            }
            _ => panic!("Se esperaban 2 triángulos recortados"),
        }
    }

    // --- PRUEBAS DE BARICÉNTRICAS Y BACKFACE CULLING ---
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

        // Triángulo degenerado (área cero)
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
            Vec2::zero(),
        );
        let cv1 = ClipVertex::new(
            Vec4::new(0.5, -0.5, 0.0, 1.0),
            Vec3::zero(),
            Vec3::up(),
            Vec2::zero(),
        );
        let cv2 = ClipVertex::new(
            Vec4::new(0.0, 0.5, 0.0, 1.0),
            Vec3::zero(),
            Vec3::up(),
            Vec2::zero(),
        );

        let vo0 = VertexOut::from_clip_vertex(&cv0, 800.0, 600.0);
        let vo1 = VertexOut::from_clip_vertex(&cv1, 800.0, 600.0);
        let vo2 = VertexOut::from_clip_vertex(&cv2, 800.0, 600.0);

        assert!(
            is_front_facing(&vo0, &vo1, &vo2),
            "Triángulo CCW debe ser frontal"
        );
        assert!(
            !is_front_facing(&vo0, &vo2, &vo1),
            "Triángulo CW debe ser cara trasera"
        );
    }

    // --- PRUEBAS DE DEPTH BUFFER ---
    #[test]
    fn test_depth_buffer_test_and_set() {
        let mut db = DepthBuffer::new(10, 10);
        db.clear();

        assert_eq!(db.get(2, 3), f32::INFINITY);

        // Valor menor pasa y se actualiza
        assert!(db.test_and_set(2, 3, 0.5));
        assert_eq!(db.get(2, 3), 0.5);

        // Valor mayor falla y es descartado
        assert!(!db.test_and_set(2, 3, 0.8));
        assert_eq!(db.get(2, 3), 0.5);

        // Valor menor pasa y se actualiza
        assert!(db.test_and_set(2, 3, 0.2));
        assert_eq!(db.get(2, 3), 0.2);
    }

    #[test]
    fn test_depth_debug_shader_linearization() {
        let shader = DepthDebugShader::new(0.1, 100.0, 2.5, 6.5);
        // En plano cercano NDC (-1.0), z lineal debe ser z_near (0.1)
        let z_near_lin = shader.linearize_depth(-1.0);
        assert!((z_near_lin - 0.1).abs() < 1e-4);

        // En plano lejano NDC (+1.0), z lineal debe ser z_far (100.0)
        let z_far_lin = shader.linearize_depth(1.0);
        assert!((z_far_lin - 100.0).abs() < 1e-2);

        // En la zona de la nave: z_near = 0.1, z_far = 100.0
        // Para z lineal = 3.5 y z lineal = 5.5, los colores generados deben mostrar contraste visible
        // (z_ndc correspondiente a z_lin = 3.5 vs 5.5)
        let p_mat = Mat4::perspective(deg_to_rad(45.0), 1.0, 0.1, 100.0);
        let clip_close = p_mat.transform_vec4(Vec4::new(0.0, 0.0, -3.5, 1.0));
        let clip_far = p_mat.transform_vec4(Vec4::new(0.0, 0.0, -5.5, 1.0));

        let frag_close = FragmentInput {
            screen_x: 0,
            screen_y: 0,
            depth_ndc: clip_close.z / clip_close.w,
            world_pos: Vec3::zero(),
            world_normal: Vec3::up(),
            uv: Vec2::zero(),
            barycentric: Vec3::new(1.0, 0.0, 0.0),
        };
        let frag_far = FragmentInput {
            screen_x: 0,
            screen_y: 0,
            depth_ndc: clip_far.z / clip_far.w,
            world_pos: Vec3::zero(),
            world_normal: Vec3::up(),
            uv: Vec2::zero(),
            barycentric: Vec3::new(1.0, 0.0, 0.0),
        };

        let color_close = shader.shade(&frag_close);
        let color_far = shader.shade(&frag_far);

        // La parte cercana debe ser significativamente más brillante que la parte lejana
        assert!(
            color_close.r > color_far.r + 50,
            "Cercano ({}) debe ser más brillante que lejano ({})",
            color_close.r,
            color_far.r
        );
    }

    // --- PRUEBAS DE MALLA, AABB Y PREVIEW ---
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

    #[test]
    fn test_load_nave_obj() {
        let path = Path::new("assets/models/nave.obj");
        assert!(
            path.exists(),
            "El archivo assets/models/nave.obj debe existir"
        );

        let mesh = Mesh::load_from_obj(path).expect("Error al cargar assets/models/nave.obj");
        assert!(mesh.vertex_count() > 0, "La nave debe contener vértices");
        assert!(
            mesh.triangle_count() > 0,
            "La nave debe contener triángulos"
        );

        let aabb = &mesh.aabb;
        let size = aabb.size();
        assert!(size.x > 0.0 && size.y > 0.0 && size.z > 0.0);
        assert!(size.x.is_finite() && size.y.is_finite() && size.z.is_finite());
    }

    #[test]
    fn test_vec4_operations_and_edge_cases() {
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);

        assert_eq!(v1 + v2, Vec4::new(6.0, 8.0, 10.0, 12.0));
        assert_eq!(v2 - v1, Vec4::new(4.0, 4.0, 4.0, 4.0));
        assert_eq!(v1 * 2.0, Vec4::new(2.0, 4.0, 6.0, 8.0));
        assert_eq!(v1 / 2.0, Vec4::new(0.5, 1.0, 1.5, 2.0));

        // División por cero en Vec4 devuelve cero
        let div_zero = v1 / 0.0;
        assert_eq!(div_zero, Vec4::zero());

        // Lerp
        let lerped = v1.lerp(v2, 0.5);
        assert_eq!(lerped, Vec4::new(3.0, 4.0, 5.0, 6.0));
    }

    #[test]
    fn test_viewport_transform_and_perspective_divide() {
        let cv = ClipVertex::new(
            Vec4::new(2.0, -2.0, 1.0, 2.0),
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::up(),
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
        let shader = DiffuseShader::new();
        let mut stats = FrameStats::default();

        // Triángulo completamente a la izquierda de la pantalla: x in [-100, -20]
        let cv0 = ClipVertex::new(
            Vec4::new(-100.0, 50.0, 0.5, 1.0),
            Vec3::zero(),
            Vec3::up(),
            Vec2::zero(),
        );
        let cv1 = ClipVertex::new(
            Vec4::new(-20.0, 20.0, 0.5, 1.0),
            Vec3::zero(),
            Vec3::up(),
            Vec2::zero(),
        );
        let cv2 = ClipVertex::new(
            Vec4::new(-50.0, 80.0, 0.5, 1.0),
            Vec3::zero(),
            Vec3::up(),
            Vec2::zero(),
        );

        let mut vo0 = VertexOut::from_clip_vertex(&cv0, 100.0, 100.0);
        let mut vo1 = VertexOut::from_clip_vertex(&cv1, 100.0, 100.0);
        let mut vo2 = VertexOut::from_clip_vertex(&cv2, 100.0, 100.0);
        // Forzar screen_pos directos fuera de la pantalla
        vo0.screen_pos = Vec2::new(-100.0, 50.0);
        vo1.screen_pos = Vec2::new(-20.0, 20.0);
        vo2.screen_pos = Vec2::new(-50.0, 80.0);

        rasterize_triangle(&vo0, &vo1, &vo2, &mut fb, &mut db, &shader, &mut stats);

        // Debe descartarse de inmediato sin evaluar fragmentos
        assert_eq!(
            stats.fragmentos_evaluados, 0,
            "No debe evaluar fragmentos para triángulos fuera de pantalla"
        );
        assert_eq!(stats.fragmentos_aceptados_depth, 0);
    }

    #[test]
    fn test_rasterize_triangle_pipeline_execution() {
        let mut fb = Framebuffer::new(100, 100);
        let mut db = DepthBuffer::new(100, 100);
        let shader = DiffuseShader::new();
        let mut stats = FrameStats::default();

        // Triángulo visible y CCW centrado en (50, 50)
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

        rasterize_triangle(&vo0, &vo1, &vo2, &mut fb, &mut db, &shader, &mut stats);

        assert!(
            stats.fragmentos_evaluados > 0,
            "Debe evaluar fragmentos dentro del bounding box"
        );
        assert!(
            stats.fragmentos_aceptados_depth > 0,
            "Debe rasterizar fragmentos que superen el depth test"
        );
        // Verificar que el centro (50, 50) fue dibujado y tiene profundidad registrada
        assert!((db.get(50, 50) - 0.2).abs() < 1e-4);
        assert_ne!(fb.get_pixel(50, 50), Color::BLACK);
    }
}
