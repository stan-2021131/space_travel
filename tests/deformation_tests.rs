use space_travel::{Mesh, generate_icosphere};

fn get_base_mesh() -> Mesh {
    generate_icosphere(15.0, 2)
}

#[test]
fn test_mesh_deformation_determinism() {
    let mut mesh_a = get_base_mesh();
    let mut mesh_b = get_base_mesh();

    mesh_a.deform(2.0, 0.4, 67);
    mesh_b.deform(2.0, 0.4, 67);

    assert_eq!(mesh_a.vertices.len(), mesh_b.vertices.len());
    for i in 0..mesh_a.vertices.len() {
        assert_eq!(
            mesh_a.vertices[i].position, mesh_b.vertices[i].position,
            "Vértice {}: la deformación con la misma semilla debe ser idéntica",
            i
        );
        assert_eq!(
            mesh_a.vertices[i].normal, mesh_b.vertices[i].normal,
            "Vértice {}: las normales recalculadas deben ser idénticas",
            i
        );
    }
}

#[test]
fn test_mesh_deformation_seed_variance() {
    let mut mesh_a = get_base_mesh();
    let mut mesh_b = get_base_mesh();

    mesh_a.deform(2.0, 0.4, 42);
    mesh_b.deform(2.0, 0.4, 999);

    let mut any_diff = false;
    for i in 0..mesh_a.vertices.len() {
        if (mesh_a.vertices[i].position - mesh_b.vertices[i].position).length() > 1e-4 {
            any_diff = true;
            break;
        }
    }

    assert!(
        any_diff,
        "Semillas distintas deben producir deformaciones geométricas diferentes"
    );
}

#[test]
fn test_mesh_deformation_zero_amplitude_identity() {
    let orig = get_base_mesh();
    let mut deformed = orig.clone();

    deformed.deform(0.0, 0.5, 42);

    for i in 0..orig.vertices.len() {
        assert_eq!(orig.vertices[i].position, deformed.vertices[i].position);
    }
}

#[test]
fn test_mesh_deformation_recalculated_normals_are_unit_and_coherent() {
    let mut mesh = get_base_mesh();
    mesh.deform(2.0, 0.3, 101);

    for (i, v) in mesh.vertices.iter().enumerate() {
        let len = v.normal.length();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "Vértice {}: la normal recalculada debe tener longitud unitaria, obtenido: {}",
            i,
            len
        );

        let radial = v.position.normalize();
        assert!(
            v.normal.dot(radial) > 0.0,
            "Vértice {}: la normal no debe invertirse hacia el centro de la esfera",
            i
        );
    }
}

#[test]
fn test_mesh_deformation_tangent_orthogonality() {
    let mut mesh = get_base_mesh();
    mesh.deform(1.5, 0.5, 77);

    for (i, v) in mesh.vertices.iter().enumerate() {
        let t_len = v.tangent.length();
        assert!(
            (t_len - 1.0).abs() < 1e-4,
            "Vértice {}: el vector tangente debe ser unitario ({})",
            i,
            t_len
        );

        let dot = v.tangent.dot(v.normal);
        assert!(
            dot.abs() < 1e-4,
            "Vértice {}: la tangente debe ser estrictamente ortogonal a la normal (dot = {})",
            i,
            dot
        );
    }
}
