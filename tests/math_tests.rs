use space_travel::math::{Mat4, Transform, Vec2, Vec3, Vec4, deg_to_rad};

#[test]
fn test_vec3_operations_and_edge_cases() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
    assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
    assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
    assert_eq!(a / 2.0, Vec3::new(0.5, 1.0, 1.5));

    let div_zero = a / 0.0;
    assert_eq!(div_zero, Vec3::zero());

    assert_eq!(a.dot(b), 32.0);

    let cross = a.cross(b);
    assert!((cross.dot(a)).abs() < 1e-6);
    assert!((cross.dot(b)).abs() < 1e-6);

    let unit = a.normalize();
    assert!((unit.length() - 1.0).abs() < 1e-6);

    let zero_norm = Vec3::zero().normalize();
    assert_eq!(zero_norm, Vec3::zero());
}

#[test]
fn test_vec2_operations() {
    let v1 = Vec2::new(3.0, 4.0);
    assert_eq!(v1.length(), 5.0);
    assert_eq!(v1.normalize(), Vec2::new(0.6, 0.8));

    let v_zero = v1 / 0.0;
    assert_eq!(v_zero, Vec2::zero());
}

#[test]
fn test_vec4_operations_and_edge_cases() {
    let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);

    assert_eq!(v1 + v2, Vec4::new(6.0, 8.0, 10.0, 12.0));
    assert_eq!(v2 - v1, Vec4::new(4.0, 4.0, 4.0, 4.0));
    assert_eq!(v1 * 2.0, Vec4::new(2.0, 4.0, 6.0, 8.0));
    assert_eq!(v1 / 2.0, Vec4::new(0.5, 1.0, 1.5, 2.0));

    let div_zero = v1 / 0.0;
    assert_eq!(div_zero, Vec4::zero());

    let lerped = v1.lerp(v2, 0.5);
    assert_eq!(lerped, Vec4::new(3.0, 4.0, 5.0, 6.0));
}

#[test]
fn test_matrix_identity_and_translation() {
    let id = Mat4::IDENTITY;
    let p = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(id.transform_point(p), p);

    let t = Mat4::translation(10.0, -5.0, 2.0);
    let transformed = t.transform_point(p);
    assert_eq!(transformed, Vec3::new(11.0, -3.0, 5.0));

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

    let near_pt = Vec4::new(0.0, 0.0, -z_near, 1.0);
    let near_clip = p_mat.transform_vec4(near_pt);
    let near_ndc_z = near_clip.z / near_clip.w;
    assert!((near_ndc_z - (-1.0)).abs() < 1e-5);
    assert!((near_clip.w - z_near).abs() < 1e-5);

    let far_pt = Vec4::new(0.0, 0.0, -z_far, 1.0);
    let far_clip = p_mat.transform_vec4(far_pt);
    let far_ndc_z = far_clip.z / far_clip.w;
    assert!((far_ndc_z - 1.0).abs() < 1e-5);
}

#[test]
fn test_transform_matrix_generation() {
    let transform = Transform::new(
        Vec3::new(10.0, 20.0, 30.0),
        Vec3::new(0.0, deg_to_rad(90.0), 0.0),
        Vec3::new(2.0, 2.0, 2.0),
    );

    let m = transform.to_matrix();
    let pt = m.transform_point(Vec3::new(1.0, 0.0, 0.0));
    assert!((pt.x - 10.0).abs() < 1e-4);
    assert!((pt.y - 20.0).abs() < 1e-4);
    assert!((pt.z - 28.0).abs() < 1e-4);
}
