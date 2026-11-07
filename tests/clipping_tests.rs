use space_travel::math::{Vec2, Vec3, Vec4};
use space_travel::pipeline::clipping::{
    ClipTriangle, ClipVertex, ClippedTriangles, clip_triangle_near_plane,
};

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
    let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, 1.0, 2.0));
    let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, -5.0, 2.0));
    let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, -5.0, 2.0));

    let tri = ClipTriangle::new(v0, v1, v2);
    match clip_triangle_near_plane(&tri) {
        ClippedTriangles::One(new_tri) => {
            assert_eq!(new_tri.v[0].clip_pos, v0.clip_pos);
            let d1 = new_tri.v[1].clip_pos.z + new_tri.v[1].clip_pos.w;
            let d2 = new_tri.v[2].clip_pos.z + new_tri.v[2].clip_pos.w;
            assert!(d1.abs() < 1e-5);
            assert!(d2.abs() < 1e-5);
        }
        _ => panic!("Se esperaba 1 triángulo recortado"),
    }
}

#[test]
fn test_clipping_two_inside_one_outside() {
    let v0 = dummy_clip_vertex(Vec4::new(0.0, 0.0, 1.0, 2.0));
    let v1 = dummy_clip_vertex(Vec4::new(1.0, 0.0, 1.0, 2.0));
    let v2 = dummy_clip_vertex(Vec4::new(0.0, 1.0, -5.0, 2.0));

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
