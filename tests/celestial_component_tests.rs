use space_travel::material::Material;
use space_travel::math::Vec3;
use space_travel::mesh::Mesh;
use space_travel::scene::{CelestialBody, CelestialComponent};
use std::path::Path;
use std::sync::Arc;

#[test]
fn kirby_components_load_expected_materials_and_triangle_budget() {
    let cases = [
        ("kirby_body/kirby_body.obj", true, false),
        ("kirby_eyes/eyes.obj", false, false),
        ("kirby_feet/feet.obj", true, false),
        ("kirby_mouth/mouth.obj", false, false),
        ("platform/platformobj.obj", true, true),
        ("thruster/thruster.obj", true, true),
    ];
    let mut triangles = 0;

    for (relative, has_albedo, has_normal) in cases {
        let path = format!("assets/models/kirby_planet/{relative}");
        let mesh = Mesh::load_from_obj(Path::new(&path))
            .unwrap_or_else(|error| panic!("No se pudo cargar {path}: {error}"));
        assert_eq!(mesh.material.texture.is_some(), has_albedo, "{path}");
        assert_eq!(mesh.material.normal_texture.is_some(), has_normal, "{path}");
        triangles += mesh.triangle_count();
    }

    assert_eq!(triangles, 7_140);
}

#[test]
fn kirby_composite_radius_fits_approved_collision_sphere() {
    let platform = Arc::new(
        Mesh::load_from_obj(Path::new(
            "assets/models/kirby_planet/platform/platformobj.obj",
        ))
        .unwrap(),
    );
    let scale = 10.0 / platform.geometric_radius();
    let component_paths = [
        ("Propulsor", "thruster/thruster.obj"),
        ("Cuerpo y brazos", "kirby_body/kirby_body.obj"),
        ("Pies", "kirby_feet/feet.obj"),
        ("Ojos", "kirby_eyes/eyes.obj"),
        ("Boca", "kirby_mouth/mouth.obj"),
    ];
    let components = component_paths
        .into_iter()
        .map(|(name, relative)| {
            let path = format!("assets/models/kirby_planet/{relative}");
            let mesh = Arc::new(Mesh::load_from_obj(Path::new(&path)).unwrap());
            CelestialComponent::new(name.to_string(), mesh)
        })
        .collect();

    let body = CelestialBody {
        id: "kirby".to_string(),
        parent_id: None,
        mesh: platform.clone(),
        material: platform.material.clone(),
        scale,
        radius: 10.0,
        collision_margin: 3.5,
        orbital_radius: 205.0,
        orbital_period: 210.0,
        rotation_period: 42.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::new(205.0, 0.0, 0.0),
        rotation_rad: 0.0,
        shader_type: "normal_mapped".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components,
        thruster_flames: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 14.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    assert!(body.composite_radius() > body.radius);
    assert!(body.composite_radius() <= body.radius + body.collision_margin);
}

#[test]
fn homero_composite_radius_fits_approved_collision_sphere() {
    let mut loaded =
        Mesh::load_models_from_obj(Path::new("assets/models/homer/homero.obj")).unwrap();
    let primary = loaded.remove(0);
    let mut auto_components = Vec::new();
    for (idx, sub_mesh) in loaded.into_iter().enumerate() {
        auto_components.push(CelestialComponent::new(
            format!("homero_part_{}", idx + 1),
            Arc::new(sub_mesh),
        ));
    }
    let mut max_geo_radius = primary.geometric_radius();
    for comp in &auto_components {
        max_geo_radius = max_geo_radius.max(comp.origin_radius);
    }
    let radius = 8.0;
    let collision_margin = 1.0;
    let scale = radius / max_geo_radius.max(1e-4);

    let body = CelestialBody {
        id: "homero".to_string(),
        parent_id: Some("dona".to_string()),
        mesh: Arc::new(primary),
        material: Material::default_diffuse(),
        scale,
        radius,
        collision_margin,
        orbital_radius: 80.0,
        orbital_period: 45.0,
        rotation_period: 20.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "diffuse".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: auto_components,
        thruster_flames: Vec::new(),
        gravity_strength: 5.0,
        gravity_radius: 16.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };

    assert_eq!(body.components.len(), 4);
    assert!(body.composite_radius() <= body.radius + body.collision_margin);
    assert!((body.composite_radius() - body.radius).abs() < 1e-4);
}

#[test]
fn shared_parent_matrix_preserves_component_assembly() {
    let mesh = Arc::new(Mesh::new(Vec::new(), Vec::new()));
    let mut body = CelestialBody {
        id: "compuesto".to_string(),
        parent_id: None,
        mesh: mesh.clone(),
        material: Material::default_diffuse(),
        scale: 2.0,
        radius: 4.0,
        collision_margin: 1.0,
        orbital_radius: 20.0,
        orbital_period: 40.0,
        rotation_period: 20.0,
        initial_orbital_angle_rad: 0.0,
        position: Vec3::zero(),
        rotation_rad: 0.0,
        shader_type: "texture".to_string(),
        surface_groups: Vec::new(),
        debris_rings: Vec::new(),
        components: vec![CelestialComponent::new("hijo".to_string(), mesh)],
        thruster_flames: Vec::new(),
        gravity_strength: 0.0,
        gravity_radius: 8.0,
        visible_gravity: false,
        axial_tilt_rad: 0.0,
        gravity_mesh: None,
    };
    body.update(5.0);

    let local_anchor = Vec3::new(1.0, 2.0, 0.0);
    let transformed = body.model_matrix().transform_point(local_anchor);
    let expected = body.position
        + body
            .rotation_matrix()
            .transform_point(local_anchor * body.scale);
    assert!((transformed - expected).length() < 1e-4);
}
