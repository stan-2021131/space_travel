use space_travel::math::Vec3;
use space_travel::mesh::Mesh;
use space_travel::scene::DebrisRing;
use std::path::Path;
use std::sync::Arc;

fn test_ring(seed: u32) -> DebrisRing {
    let meshes = vec![
        Arc::new(Mesh::new(Vec::new(), Vec::new())),
        Arc::new(Mesh::new(Vec::new(), Vec::new())),
    ];
    DebrisRing::generate(
        meshes, seed, 20, 17.0, 22.0, 18.0, 42.0, 0.65, 1.25, 1.2, 5.0, 12.0, 0.15,
    )
}

#[test]
fn debris_distribution_is_deterministic_and_bounded() {
    let first = test_ring(67);
    let second = test_ring(67);

    assert_eq!(first.instances.len(), 20);
    for (a, b) in first.instances.iter().zip(&second.instances) {
        assert_eq!(a.mesh_index, b.mesh_index);
        assert!((a.base_angle_rad - b.base_angle_rad).abs() < 1e-6);
        assert!((a.orbital_radius - b.orbital_radius).abs() < 1e-6);
        assert!((a.height - b.height).abs() < 1e-6);
        assert!((a.scale - b.scale).abs() < 1e-6);
        assert!((17.0..=22.0).contains(&a.orbital_radius));
        assert!((-1.2..=1.2).contains(&a.height));
        assert!((0.65..=1.25).contains(&a.scale));
        assert!(a.initial_rotation.x.is_finite());
        assert!(a.angular_velocity.z.is_finite());
    }

    assert!(
        first
            .instances
            .iter()
            .any(|instance| instance.mesh_index == 0)
    );
    assert!(
        first
            .instances
            .iter()
            .any(|instance| instance.mesh_index == 1)
    );
}

#[test]
fn debris_orbit_repeats_after_its_period_and_follows_parent_translation() {
    let ring = test_ring(67);
    let instance = &ring.instances[0];
    let parent = Vec3::new(10.0, -3.0, 7.0);
    let at_start = ring.instance_world_position(parent, instance, 0.0);
    let at_quarter = ring.instance_world_position(parent, instance, 10.5);
    let after_period = ring.instance_world_position(parent, instance, 42.0);

    assert!((at_start - after_period).length() < 1e-4);
    assert!((at_start - at_quarter).length() > 1.0);

    let translation = Vec3::new(-5.0, 8.0, 2.0);
    let moved = ring.instance_world_position(parent + translation, instance, 0.0);
    assert!((moved - at_start - translation).length() < 1e-5);
}

#[test]
fn individual_debris_spin_changes_orientation_without_changing_orbit_formula() {
    let ring = test_ring(67);
    let instance = &ring.instances[3];
    let parent = Vec3::zero();
    let (model_start, rotation_start) = ring.instance_matrices(parent, instance, 0.0);
    let (model_later, rotation_later) = ring.instance_matrices(parent, instance, 1.0);

    let direction = Vec3::new(1.0, 0.0, 0.0);
    assert!(
        (rotation_start.transform_direction(direction)
            - rotation_later.transform_direction(direction))
        .length()
            > 1e-3
    );
    assert!(model_start.transform_point(Vec3::zero()).x.is_finite());
    assert!(model_later.transform_point(Vec3::zero()).z.is_finite());
}

#[test]
fn destroyed_planet_and_debris_load_albedo_and_normal_maps() {
    let paths = [
        "assets/models/destroyed/PlanetaDestruido.obj",
        "assets/models/objects/escombro1/escombro1.obj",
        "assets/models/objects/escombro2/escombro2.obj",
    ];

    for path in paths {
        let mesh = Mesh::load_from_obj(Path::new(path))
            .unwrap_or_else(|error| panic!("No se pudo cargar {path}: {error}"));
        assert!(mesh.material.texture.is_some(), "Falta albedo en {path}");
        assert!(
            mesh.material.normal_texture.is_some(),
            "Falta normal map en {path}"
        );
    }
}
