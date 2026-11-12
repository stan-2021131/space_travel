use space_travel::config::AppConfig;
use space_travel::math::Vec3;
use space_travel::mesh::Mesh;
use space_travel::scene::Moon;
use std::path::Path;
use std::sync::Arc;

fn star_moon() -> (Moon, Vec3) {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap();
    let kirby = config
        .planets
        .iter()
        .find(|planet| planet.id == "Parque kirby")
        .unwrap();
    let moon_config = &kirby.moons[0];
    let mesh = Arc::new(Mesh::load_from_obj(Path::new(&moon_config.model_path)).unwrap());
    let scale = moon_config.radius / mesh.geometric_radius();
    let moon = Moon {
        name: moon_config.name.clone(),
        material: mesh.material.clone(),
        mesh,
        scale,
        radius: moon_config.radius,
        orbital_radius: moon_config.orbital_radius,
        orbital_period: moon_config.orbital_period,
        rotation_period: moon_config.rotation_period,
        initial_orbital_angle_rad: moon_config.initial_orbital_angle_deg.to_radians(),
        orbit_tilt_rad: moon_config.orbit_tilt_deg.to_radians(),
        collision_margin: moon_config.collision_margin,
    };
    (moon, Vec3::new(100.0, 20.0, -50.0))
}

#[test]
fn star_asset_loads_with_approved_budget_and_material() {
    let (moon, _) = star_moon();
    assert_eq!(moon.mesh.triangle_count(), 1_014);
    assert_eq!(moon.material.name, "mat_star_moon");
    assert!(moon.material.texture.is_none());
    assert!(moon.material.kd.x > moon.material.kd.y);
    assert!(moon.material.kd.y > moon.material.kd.z);
}

#[test]
fn moon_orbit_repeats_and_follows_parent_translation() {
    let (moon, parent) = star_moon();
    let start = moon.world_position(parent, 0.0);
    let repeated = moon.world_position(parent, moon.orbital_period);
    assert!((start - repeated).length() < 1e-3);

    let displacement = Vec3::new(7.0, -3.0, 11.0);
    let moved = moon.world_position(parent + displacement, 0.0);
    assert!((moved - start - displacement).length() < 1e-4);
}

#[test]
fn moon_face_points_radially_outward_while_spin_is_independent() {
    let (moon, parent) = star_moon();
    let position = moon.world_position(parent, 3.0);
    let (_, rotation) = moon.matrices(parent, 3.0);
    let face_normal = rotation.transform_direction(Vec3::up()).normalize();
    let radial = (position - parent).normalize();
    assert!(face_normal.dot(radial) > 0.999);

    let (model_a, _) = moon.matrices(parent, 0.0);
    let (model_b, _) = moon.matrices(parent, moon.rotation_period * 0.25);
    let local_tip = Vec3::new(-1.0, 0.0, 0.0);
    assert!(
        (model_a.transform_point(local_tip) - model_b.transform_point(local_tip)).length() > 0.1
    );
}

#[test]
fn moon_orbit_clears_kirby_collision_sphere() {
    let config = AppConfig::load_from_file(Path::new("config/solar_system.toml")).unwrap();
    let kirby = config
        .planets
        .iter()
        .find(|planet| planet.id == "Parque kirby")
        .unwrap();
    let moon = &kirby.moons[0];
    let clearance = moon.orbital_radius
        - moon.radius
        - moon.collision_margin
        - kirby.radius
        - kirby.collision_margin;
    assert!(clearance >= 5.0 - 1e-4);
}
