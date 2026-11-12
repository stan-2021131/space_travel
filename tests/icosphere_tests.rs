use space_travel::generate_icosphere;

#[test]
fn test_icosphere_subdivision_vertex_and_face_counts() {
    // Progresión teórica del icosaedro subdividido:
    // Nivel N: V(N) = 10 * 4^N + 2, F(N) = 20 * 4^N
    let expected = [(0, 12, 20), (1, 42, 80), (2, 162, 320), (3, 642, 1280)];

    for &(level, exp_v, exp_f) in &expected {
        let mesh = generate_icosphere(10.0, level);
        assert_eq!(
            mesh.vertex_count(),
            exp_v,
            "Nivel {}: se esperaban {} vértices, se obtuvieron {}",
            level,
            exp_v,
            mesh.vertex_count()
        );
        assert_eq!(
            mesh.triangle_count(),
            exp_f,
            "Nivel {}: se esperaban {} triángulos, se obtuvieron {}",
            level,
            exp_f,
            mesh.triangle_count()
        );
        assert_eq!(mesh.indices.len(), exp_f * 3);

        // Característica de Euler para una esfera cerrada: V - E + F = 2
        // En una malla triangular cerrada cada cara tiene 3 aristas y cada arista se comparte entre 2 caras: E = 3F / 2
        // Por tanto: V - (3F / 2) + F = V - F / 2 = 2
        let euler = mesh.vertex_count() as i32 - (mesh.triangle_count() as i32) / 2;
        assert_eq!(
            euler, 2,
            "Nivel {}: la característica de Euler debe ser exactamente 2 para una variedad esférica cerrada, se obtuvo {}",
            level, euler
        );
    }
}

#[test]
fn test_icosphere_radial_distance_and_normals() {
    let radius = 17.5;
    let mesh = generate_icosphere(radius, 2);

    for (i, v) in mesh.vertices.iter().enumerate() {
        let dist = v.position.length();
        assert!(
            (dist - radius).abs() < 1e-4,
            "Vértice {}: distancia al origen ({}) difiere del radio esperado ({})",
            i,
            dist,
            radius
        );

        let norm_len = v.normal.length();
        assert!(
            (norm_len - 1.0).abs() < 1e-4,
            "Vértice {}: normal no es unitaria ({})",
            i,
            norm_len
        );

        let radial_dir = v.position.normalize();
        assert!(
            v.normal.dot(radial_dir) > 0.9999,
            "Vértice {}: normal debe apuntar en dirección radial hacia el exterior",
            i
        );
    }
}

#[test]
fn test_icosphere_outward_facing_winding() {
    let mesh = generate_icosphere(5.0, 2);

    for (tri_idx, chunk) in mesh.indices.chunks_exact(3).enumerate() {
        let p0 = mesh.vertices[chunk[0] as usize].position;
        let p1 = mesh.vertices[chunk[1] as usize].position;
        let p2 = mesh.vertices[chunk[2] as usize].position;

        let face_normal = (p1 - p0).cross(p2 - p0);
        let face_center = (p0 + p1 + p2) * (1.0 / 3.0);

        let dot = face_normal.dot(face_center);
        assert!(
            dot > 1e-5,
            "Triángulo {}: devanado no es CCW hacia el exterior (dot = {})",
            tri_idx,
            dot
        );
    }
}

#[test]
fn test_icosphere_uv_coordinates_bounds() {
    let mesh = generate_icosphere(8.0, 2);

    for (i, v) in mesh.vertices.iter().enumerate() {
        assert!(
            v.uv.x >= 0.0 && v.uv.x <= 1.0,
            "Vértice {}: U fuera de [0, 1]: {}",
            i,
            v.uv.x
        );
        assert!(
            v.uv.y >= 0.0 && v.uv.y <= 1.0,
            "Vértice {}: V fuera de [0, 1]: {}",
            i,
            v.uv.y
        );
    }
}

#[test]
fn test_icosphere_aabb_dimensions() {
    let radius = 12.0;
    let mesh = generate_icosphere(radius, 3);

    let center = mesh.aabb.center();
    assert!(
        center.length() < 1e-3,
        "El centro del AABB de la icosphere debe estar en el origen, obtenido: {:?}",
        center
    );

    assert!((mesh.aabb.max.x - radius).abs() < 0.2);
    assert!((mesh.aabb.max.y - radius).abs() < 0.2);
    assert!((mesh.aabb.max.z - radius).abs() < 0.2);
    assert!((mesh.aabb.min.x - (-radius)).abs() < 0.2);
    assert!((mesh.aabb.min.y - (-radius)).abs() < 0.2);
    assert!((mesh.aabb.min.z - (-radius)).abs() < 0.2);
}
