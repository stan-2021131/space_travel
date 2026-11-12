use space_travel::{Mesh, Vec2, Vec3, Vertex};
use std::path::Path;

#[test]
fn test_subdivide_single_triangle_quadruples_faces() {
    let vertices = vec![
        Vertex::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        Vertex::new(
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec2::new(1.0, 0.0),
        ),
        Vertex::new(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec2::new(0.0, 1.0),
        ),
    ];
    let indices = vec![0, 1, 2];
    let mut mesh = Mesh::new(vertices, indices);

    assert_eq!(mesh.triangle_count(), 1);
    assert_eq!(mesh.vertex_count(), 3);

    mesh.subdivide();

    // 1 triángulo se subdivide en 4 triángulos (12 índices)
    assert_eq!(mesh.triangle_count(), 4);
    assert_eq!(mesh.indices.len(), 12);
    // 3 vértices originales + 3 puntos medios de aristas = 6 vértices
    assert_eq!(mesh.vertex_count(), 6);

    // Los puntos medios deben existir con posiciones interpoladas exactas:
    // (1, 0, 0), (1, 1, 0), (0, 1, 0)
    let positions: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position).collect();
    assert!(
        positions
            .iter()
            .any(|p| (*p - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5)
    );
    assert!(
        positions
            .iter()
            .any(|p| (*p - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5)
    );
    assert!(
        positions
            .iter()
            .any(|p| (*p - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5)
    );
}

#[test]
fn test_subdivide_shared_edge_deduplication() {
    // Cuadrilátero formado por 2 triángulos que comparten la hipotenusa (arista entre 1 y 2):
    // v2 (0, 2) ----- v3 (2, 2)
    // |          /    |
    // |       /       |
    // v0 (0, 0) ----- v1 (2, 0)
    let normal_z = Vec3::new(0.0, 0.0, 1.0);
    let vertices = vec![
        Vertex::new(Vec3::new(0.0, 0.0, 0.0), normal_z, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(2.0, 0.0, 0.0), normal_z, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(0.0, 2.0, 0.0), normal_z, Vec2::new(0.0, 1.0)),
        Vertex::new(Vec3::new(2.0, 2.0, 0.0), normal_z, Vec2::new(1.0, 1.0)),
    ];
    // Triángulo 1: (0, 1, 2), Triángulo 2: (1, 3, 2) - comparten arista (1, 2)
    let indices = vec![0, 1, 2, 1, 3, 2];
    let mut mesh = Mesh::new(vertices, indices);

    assert_eq!(mesh.triangle_count(), 2);
    assert_eq!(mesh.vertex_count(), 4);

    mesh.subdivide();

    // 2 triángulos -> 8 triángulos (24 índices)
    assert_eq!(mesh.triangle_count(), 8);
    assert_eq!(mesh.indices.len(), 24);

    // Aristas originales: 4 de perímetro + 1 diagonal compartida = 5 aristas.
    // Vértices esperados: 4 originales + 5 puntos medios = 9 vértices.
    // Si no deduplicara aristas compartidas, crearía 6 puntos medios en vez de 5 (10 vértices).
    assert_eq!(
        mesh.vertex_count(),
        9,
        "La subdivisión debe deduplicar vértices en aristas compartidas mediante el caché de aristas"
    );

    // El punto medio de la diagonal compartida (1.0, 1.0, 0.0) debe aparecer exactamente una vez
    let midpoint_count = mesh
        .vertices
        .iter()
        .filter(|v| (v.position - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5)
        .count();
    assert_eq!(
        midpoint_count, 1,
        "El punto medio en la arista compartida solo debe crearse una vez"
    );
}

#[test]
fn test_subdivide_interpolates_attributes() {
    let v0 = Vertex::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::new(0.0, 0.0),
    );
    let v1 = Vertex::new(
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec2::new(1.0, 0.0),
    );
    let v2 = Vertex::new(
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.0, 1.0),
    );

    let mut mesh = Mesh::new(vec![v0, v1, v2], vec![0, 1, 2]);
    mesh.subdivide();

    // Buscar el punto medio de v0 y v1:
    let mid_01 = mesh
        .vertices
        .iter()
        .find(|v| (v.position - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5)
        .expect("Debe existir el punto medio v0-v1");

    // UV interpolado: (0.5, 0.0)
    assert!((mid_01.uv.x - 0.5).abs() < 1e-5);
    assert!((mid_01.uv.y - 0.0).abs() < 1e-5);

    // Normal interpolada y normalizada: normalize( (1,0,0) + (0,1,0) ) = (1/sqrt(2), 1/sqrt(2), 0)
    let expected_norm = Vec3::new(1.0, 1.0, 0.0).normalize();
    assert!(
        (mid_01.normal - expected_norm).length() < 1e-5,
        "La normal intermedia debe estar correctamente promediada y normalizada"
    );
    assert!(
        (mid_01.normal.length() - 1.0).abs() < 1e-5,
        "La normal intermedia debe tener longitud 1.0"
    );
}

#[test]
fn test_subdivide_rocky_model() {
    let mut mesh = Mesh::load_from_obj(Path::new("assets/models/rocky/rocky.obj"))
        .expect("Debe cargar rocky.obj");

    let original_triangles = mesh.triangle_count();
    let original_indices = mesh.indices.len();

    mesh.subdivide();

    assert_eq!(
        mesh.triangle_count(),
        original_triangles * 4,
        "La cantidad de triángulos debe multiplicarse por 4 tras subdividir"
    );
    assert_eq!(
        mesh.indices.len(),
        original_indices * 4,
        "La cantidad de índices debe multiplicarse por 4"
    );

    // Todas las normales de los vértices deben estar normalizadas
    for (i, v) in mesh.vertices.iter().enumerate() {
        assert!(
            (v.normal.length() - 1.0).abs() < 1e-4,
            "Vértice {}: normal no es unitaria ({})",
            i,
            v.normal.length()
        );
    }
}
