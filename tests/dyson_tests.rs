use space_travel::config::AppConfig;
use space_travel::material::Material;
use space_travel::math::{Vec2, Vec3};
use space_travel::mesh::Mesh;
use space_travel::pipeline::{DiffuseShader, FragmentInput, FragmentShader};
use space_travel::scene::DysonSphere;
use std::path::Path;
use std::sync::Arc;

#[test]
fn test_load_dyson_obj_asset() {
    let dyson_path = Path::new("assets/models/dyson/dyson.obj");
    assert!(dyson_path.exists(), "El archivo dyson.obj debe existir");

    let dyson_mesh = Mesh::load_from_obj(dyson_path).expect("dyson.obj debe cargar correctamente");
    assert!(
        dyson_mesh.vertices.len() >= 500,
        "La esfera de Dyson debe contener suficiente detalle poligonal"
    );
    assert!(
        !dyson_mesh.indices.is_empty(),
        "dyson.obj debe tener índices triangulares"
    );
    assert!(
        dyson_mesh.geometric_radius() > 0.5,
        "El radio geométrico de dyson.obj debe ser positivo"
    );
}

#[test]
fn test_load_models_from_obj_generic_multimaterial() {
    let dyson_path = Path::new("assets/models/dyson/dyson.obj");
    let submeshes = Mesh::load_models_from_obj(dyson_path)
        .expect("dyson.obj debe cargar todas sus submallas con MTL");

    assert_eq!(
        submeshes.len(),
        4,
        "dyson.obj contiene 4 grupos de materiales definidos en dyson.mtl"
    );

    let mat_names: Vec<&str> = submeshes.iter().map(|m| m.material.name.as_str()).collect();
    assert!(
        mat_names.contains(&"M_Structure_DarkMetal"),
        "Debe contener el material M_Structure_DarkMetal"
    );
    assert!(
        mat_names.contains(&"M_Collector_Bronze"),
        "Debe contener el material M_Collector_Bronze"
    );
    assert!(
        mat_names.contains(&"M_Radiator_Copper"),
        "Debe contener el material M_Radiator_Copper"
    );
    assert!(
        mat_names.contains(&"M_Systems_Amber"),
        "Debe contener el material M_Systems_Amber"
    );

    // Verificar reflectancias Kd cargadas desde dyson.mtl
    for submesh in &submeshes {
        assert!(
            submesh.triangle_count() > 0,
            "Cada submalla debe contener geometría triangular"
        );
        match submesh.material.name.as_str() {
            "M_Collector_Bronze" => {
                assert!((submesh.material.kd.x - 0.23).abs() < 1e-3);
                assert!((submesh.material.kd.y - 0.095).abs() < 1e-3);
                assert!((submesh.material.kd.z - 0.018).abs() < 1e-3);
            }
            "M_Radiator_Copper" => {
                assert!((submesh.material.kd.x - 0.19).abs() < 1e-3);
                assert!((submesh.material.kd.y - 0.035).abs() < 1e-3);
                assert!((submesh.material.kd.z - 0.012).abs() < 1e-3);
            }
            "M_Structure_DarkMetal" => {
                assert!((submesh.material.kd.x - 0.22).abs() < 1e-3);
                assert!((submesh.material.kd.y - 0.26).abs() < 1e-3);
                assert!((submesh.material.kd.z - 0.30).abs() < 1e-3);
            }
            "M_Systems_Amber" => {
                assert!((submesh.material.kd.x - 0.012).abs() < 1e-3);
                assert!((submesh.material.ke.x - 0.08).abs() < 1e-3);
                assert!((submesh.material.ke.y - 2.20).abs() < 1e-3);
                assert!((submesh.material.ke.z - 4.00).abs() < 1e-3);
            }
            _ => panic!("Material no reconocido: {}", submesh.material.name),
        }
    }
}

#[test]
fn test_dyson_sphere_model_matrix() {
    let dyson_path = Path::new("assets/models/dyson/dyson.obj");
    let submeshes = Mesh::load_models_from_obj(dyson_path).unwrap();
    let meshes = submeshes.into_iter().map(Arc::new).collect();
    let sphere = DysonSphere::new(meshes, 60.0, 0.02, 60.0, 72.0, 8.0);

    let center = Vec3::new(10.0, -5.0, 30.0);
    let m = sphere.model_matrix(center);

    // El origen local transformado debe posicionarse en el centro dado
    let transformed_center = m.transform_point(Vec3::zero());
    assert!((transformed_center.x - 10.0).abs() < 1e-3);
    assert!((transformed_center.y - (-5.0)).abs() < 1e-3);
    assert!((transformed_center.z - 30.0).abs() < 1e-3);
    assert!(sphere.scale > 0.0, "La escala debe ser positiva");
}

#[test]
fn test_dyson_sphere_rotation() {
    let dyson_path = Path::new("assets/models/dyson/dyson.obj");
    let submeshes = Mesh::load_models_from_obj(dyson_path).unwrap();
    let meshes = submeshes.into_iter().map(Arc::new).collect();
    let mut sphere = DysonSphere::new(meshes, 60.0, 0.02, 60.0, 72.0, 8.0);

    let rot_init = sphere.current_rotation_rad;
    sphere.update(3.0);
    let rot_after = sphere.current_rotation_rad;

    assert!(
        rot_after > rot_init,
        "La rotación de la esfera debe avanzar conforme transcurre el tiempo"
    );
    assert!(
        (rot_after - 0.06).abs() < 1e-4,
        "La velocidad de rotación (0.02 rad/s) por 3s debe ser 0.06 rad"
    );
}

#[test]
fn test_toml_dyson_sphere_parsing() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml"))
        .expect("solar_system.toml debe deserializarse sin errores");

    let dyson_cfg = config
        .dyson_sphere
        .as_ref()
        .expect("Debe existir la sección [dyson_sphere] en solar_system.toml");

    assert!(
        dyson_cfg.enabled,
        "La esfera de Dyson debe estar habilitada"
    );
    assert_eq!(dyson_cfg.radius, 60.0);
    assert_eq!(dyson_cfg.gravity_strength, 8.0);
    assert_eq!(dyson_cfg.effective_gravity_radius(), 72.0);
    assert_eq!(dyson_cfg.effective_collision_radius(), 60.0);
}

#[test]
fn test_diffuse_shader_with_emissive_material() {
    let shader = DiffuseShader::new();
    let mat = Material::with_emission(
        "EmissiveMat",
        Vec3::new(0.01, 0.05, 0.07),
        Vec3::new(0.2, 0.8, 1.0),
        None,
        None,
    );

    let input = FragmentInput {
        screen_x: 0,
        screen_y: 0,
        depth_ndc: 0.5,
        world_pos: Vec3::zero(),
        world_normal: Vec3::up(),
        world_tangent: Vec3::new(1.0, 0.0, 0.0),
        uv: Vec2::zero(),
        barycentric: Vec3::new(0.33, 0.33, 0.34),
        material: &mat,
    };

    let color = shader.shade(&input);
    // El color final debe incluir el componente emisivo Ke
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;

    assert!(
        r >= 0.20 && g >= 0.80 && b >= 0.99,
        "DiffuseShader debe sumar la emisión Ke al color evaluado"
    );
}
