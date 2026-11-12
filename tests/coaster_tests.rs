use space_travel::{CelestialComponent, Mat4, Mesh, Vec3};
use std::path::Path;
use std::sync::Arc;

const TRACK_PATH: &str = "assets/models/objects/roller_coaster/roller_coaster.obj";
const TRAIN_PATH: &str = "assets/models/objects/coaster_train/coaster_train.obj";

#[test]
fn coaster_assets_load_with_texture_and_small_triangle_budget() {
    let track = Mesh::load_from_obj(Path::new(TRACK_PATH)).expect("Debe cargar el riel");
    let train = Mesh::load_from_obj(Path::new(TRAIN_PATH)).expect("Debe cargar el tren");

    assert_eq!(track.triangle_count(), 1_424);
    assert_eq!(train.triangle_count(), 688);
    assert_eq!(track.triangle_count() + train.triangle_count(), 2_112);
    assert!(track.material.texture.is_some());
    assert!(train.material.texture.is_some());
    assert!(Path::new("assets/models/objects/coaster_colormap.png").is_file());
    assert!(Path::new("assets/models/objects/roller_coaster/LICENSE.txt").is_file());
}

#[test]
fn train_returns_to_same_pose_after_one_period() {
    let mesh = Arc::new(Mesh::load_from_obj(Path::new(TRAIN_PATH)).unwrap());
    let component =
        CelestialComponent::new("Tren".to_string(), mesh.clone()).with_local_rotation(14.0, 0.0);
    let parent_model =
        Mat4::from_translation(Vec3::new(10.0, 3.0, -4.0)) * Mat4::uniform_scaling(1.75);
    let parent_rotation = Mat4::rotation_y(0.4);
    let anchor = mesh.aabb.center();

    let (start_model, start_rotation) = component.matrices(&parent_model, &parent_rotation, 0.0);
    let (end_model, end_rotation) = component.matrices(&parent_model, &parent_rotation, 14.0);

    assert!(
        (start_model.transform_point(anchor) - end_model.transform_point(anchor)).length() < 1e-4
    );
    let direction = Vec3::new(1.0, 0.0, 0.0);
    assert!(
        (start_rotation.transform_direction(direction)
            - end_rotation.transform_direction(direction))
        .length()
            < 1e-4
    );
}

#[test]
fn train_follows_vertical_circle_and_rotates_with_its_tangent() {
    let mesh = Arc::new(Mesh::load_from_obj(Path::new(TRAIN_PATH)).unwrap());
    let component =
        CelestialComponent::new("Tren".to_string(), mesh.clone()).with_local_rotation(14.0, 0.0);
    let anchor = mesh.aabb.center();
    let (quarter_model, quarter_rotation) =
        component.matrices(&Mat4::IDENTITY, &Mat4::IDENTITY, 3.5);
    let quarter_anchor = quarter_model.transform_point(anchor);

    assert!((quarter_anchor.x + anchor.y).abs() < 1e-4);
    assert!((quarter_anchor.y - anchor.x).abs() < 1e-4);
    assert!((quarter_anchor.z - anchor.z).abs() < 1e-4);
    assert!((quarter_anchor.length() - anchor.length()).abs() < 1e-4);

    let rotated_right = quarter_rotation.transform_direction(Vec3::new(1.0, 0.0, 0.0));
    assert!((rotated_right - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-4);
}

#[test]
fn analytical_train_animation_is_independent_of_frame_partition() {
    let mesh = Arc::new(Mesh::load_from_obj(Path::new(TRAIN_PATH)).unwrap());
    let component =
        CelestialComponent::new("Tren".to_string(), mesh).with_local_rotation(14.0, 0.0);

    let direct = component.local_rotation_matrix(5.25);
    let simulated_time = [1.0_f32, 0.5, 2.0, 1.75].into_iter().sum();
    let partitioned = component.local_rotation_matrix(simulated_time);
    assert_eq!(direct, partitioned);
}
