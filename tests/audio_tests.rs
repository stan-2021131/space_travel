use rodio::Decoder;
use space_travel::audio::{music_influence, smooth_gain, strongest_music_zone};
use space_travel::config::MusicZoneConfig;
use space_travel::math::Vec3;
use std::fs::File;

fn zone(body_id: &str, inner: f32, outer: f32) -> MusicZoneConfig {
    MusicZoneConfig {
        body_id: body_id.to_string(),
        path: format!("assets/audio/music/{body_id}.ogg"),
        inner_distance: inner,
        outer_distance: outer,
        volume: 1.0,
    }
}

#[test]
fn influence_uses_distance_from_surface_with_smooth_limits() {
    assert_eq!(music_influence(35.0, 10.0, 25.0, 75.0), 1.0);
    assert_eq!(music_influence(85.0, 10.0, 25.0, 75.0), 0.0);

    let middle = music_influence(60.0, 10.0, 25.0, 75.0);
    assert!((middle - 0.5).abs() < 1e-6);
}

#[test]
fn strongest_zone_selects_only_the_most_influential_body() {
    let zones = vec![zone("kirby", 10.0, 80.0), zone("rocoso", 10.0, 80.0)];
    let selected = strongest_music_zone(Vec3::new(25.0, 0.0, 0.0), &zones, |id| match id {
        "kirby" => Some((Vec3::zero(), 10.0)),
        "rocoso" => Some((Vec3::new(100.0, 0.0, 0.0), 15.0)),
        _ => None,
    })
    .unwrap();

    assert_eq!(selected.0, 0);
    assert!(selected.1 > 0.9);
}

#[test]
fn crossfade_smoothing_is_independent_of_frame_partition() {
    let one_step = smooth_gain(0.0, 1.0, 1.0, 2.5);
    let half_step = smooth_gain(0.0, 1.0, 0.5, 2.5);
    let two_steps = smooth_gain(half_step, 1.0, 0.5, 2.5);
    assert!((one_step - two_steps).abs() < 1e-6);
}

#[test]
fn invalid_or_distant_music_zones_produce_no_influence() {
    assert_eq!(music_influence(f32::NAN, 10.0, 5.0, 20.0), 0.0);
    assert_eq!(music_influence(15.0, 10.0, 20.0, 10.0), 0.0);

    let zones = vec![zone("kirby", 10.0, 20.0)];
    assert!(
        strongest_music_zone(Vec3::new(100.0, 0.0, 0.0), &zones, |_| {
            Some((Vec3::zero(), 10.0))
        })
        .is_none()
    );
}

#[test]
fn configured_music_files_are_valid_loopable_ogg_streams() {
    let paths = [
        "assets/audio/music/General.ogg",
        "assets/audio/music/Dyson_fixed.ogg",
        "assets/audio/music/Tierra_Planet.ogg",
        "assets/audio/music/Rocky_Planet.ogg",
        "assets/audio/music/Destroyed_Planet_fixed.ogg",
        "assets/audio/music/Kirby_Planet.ogg",
        "assets/audio/music/Donut_Planet.ogg",
    ];

    for path in paths {
        let file =
            File::open(path).unwrap_or_else(|error| panic!("No se pudo abrir {path}: {error}"));
        Decoder::new_looped(file)
            .unwrap_or_else(|error| panic!("No se pudo decodificar {path}: {error}"));
    }
}
