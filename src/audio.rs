use crate::config::{MusicConfig, MusicZoneConfig};
use crate::math::Vec3;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use std::fs::File;
use std::path::Path;

struct ActiveTrack {
    zone_index: usize,
    sink: Sink,
    gain: f32,
}

/// Reproduce la música general y, como máximo, dos pistas planetarias durante un crossfade.
pub struct MusicSystem {
    stream: OutputStream,
    general_sink: Sink,
    general_gain: f32,
    active: Option<ActiveTrack>,
    outgoing: Option<ActiveTrack>,
    failed_zones: Vec<bool>,
    config: MusicConfig,
}

impl MusicSystem {
    /// Retorna `None` cuando la música está desactivada o no existe dispositivo de salida.
    pub fn initialize(config: &MusicConfig) -> Result<Option<Self>, String> {
        if !config.enabled {
            return Ok(None);
        }

        Self::validate_files(config)?;

        let stream = match OutputStreamBuilder::open_default_stream() {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!(
                    "[AUDIO] No se encontró un dispositivo de salida; la escena continuará sin música: {error}"
                );
                return Ok(None);
            }
        };

        let general_file = File::open(&config.general_path).map_err(|error| {
            format!(
                "No se pudo abrir la música general '{}': {error}",
                config.general_path
            )
        })?;
        let general_source = Decoder::new_looped(general_file).map_err(|error| {
            format!(
                "No se pudo decodificar la música general '{}': {error}",
                config.general_path
            )
        })?;
        let general_sink = Sink::connect_new(stream.mixer());
        general_sink.append(general_source);
        general_sink.set_volume(config.master_volume);

        Ok(Some(Self {
            stream,
            general_sink,
            general_gain: 1.0,
            active: None,
            outgoing: None,
            failed_zones: vec![false; config.zones.len()],
            config: config.clone(),
        }))
    }

    fn validate_files(config: &MusicConfig) -> Result<(), String> {
        if !Path::new(&config.general_path).is_file() {
            return Err(format!(
                "No existe la música general configurada: '{}'",
                config.general_path
            ));
        }
        for zone in &config.zones {
            if !Path::new(&zone.path).is_file() {
                return Err(format!(
                    "No existe la música configurada para '{}': '{}'",
                    zone.body_id, zone.path
                ));
            }
        }
        Ok(())
    }

    fn start_zone(&self, zone_index: usize) -> Result<ActiveTrack, String> {
        let zone = &self.config.zones[zone_index];
        let file = File::open(&zone.path)
            .map_err(|error| format!("No se pudo abrir '{}': {error}", zone.path))?;
        let source = Decoder::new_looped(file)
            .map_err(|error| format!("No se pudo decodificar '{}': {error}", zone.path))?;
        let sink = Sink::connect_new(self.stream.mixer());
        sink.set_volume(0.0);
        sink.append(source);
        Ok(ActiveTrack {
            zone_index,
            sink,
            gain: 0.0,
        })
    }

    /// Actualiza ganancias usando tiempo real, independiente del número de cuadros y del time scale.
    pub fn update(&mut self, selection: Option<(usize, f32)>, dt: f32) {
        let selection = selection.filter(|(index, influence)| {
            *index < self.config.zones.len()
                && influence.is_finite()
                && *influence > 0.0
                && !self.failed_zones[*index]
        });
        let selected_index = selection.map(|(index, _)| index);
        let active_index = self.active.as_ref().map(|track| track.zone_index);

        if selected_index != active_index {
            if let Some(previous) = self.active.take()
                && let Some(replaced) = self.outgoing.replace(previous)
            {
                replaced.sink.stop();
            }

            if let Some(index) = selected_index {
                match self.start_zone(index) {
                    Ok(track) => self.active = Some(track),
                    Err(error) => {
                        eprintln!("[AUDIO] {error}");
                        self.failed_zones[index] = true;
                    }
                }
            }
        }

        let effective_influence = match (&self.active, selection) {
            (Some(track), Some((index, influence))) if track.zone_index == index => {
                influence.clamp(0.0, 1.0)
            }
            _ => 0.0,
        };

        let general_floor = self.config.general_min_volume;
        let general_target = (general_floor * general_floor
            + (1.0 - general_floor * general_floor) * (1.0 - effective_influence))
            .sqrt();
        self.general_gain = smooth_gain(
            self.general_gain,
            general_target,
            dt,
            self.config.crossfade_seconds,
        );
        self.general_sink
            .set_volume(self.config.master_volume * self.general_gain);

        if let Some(track) = &mut self.active {
            let zone_volume = self.config.zones[track.zone_index].volume;
            let target = effective_influence.sqrt() * zone_volume;
            track.gain = smooth_gain(track.gain, target, dt, self.config.crossfade_seconds);
            track
                .sink
                .set_volume(self.config.master_volume * track.gain);
        }

        let mut remove_outgoing = false;
        if let Some(track) = &mut self.outgoing {
            track.gain = smooth_gain(track.gain, 0.0, dt, self.config.crossfade_seconds);
            track
                .sink
                .set_volume(self.config.master_volume * track.gain);
            remove_outgoing = track.gain <= 0.001;
        }
        if remove_outgoing && let Some(track) = self.outgoing.take() {
            track.sink.stop();
        }
    }
}

/// Influencia musical según la distancia a la superficie del cuerpo celeste.
pub fn music_influence(
    distance_to_center: f32,
    body_radius: f32,
    inner_distance: f32,
    outer_distance: f32,
) -> f32 {
    if !distance_to_center.is_finite()
        || !body_radius.is_finite()
        || body_radius < 0.0
        || inner_distance < 0.0
        || outer_distance <= inner_distance
    {
        return 0.0;
    }

    let surface_distance = (distance_to_center - body_radius).max(0.0);
    if surface_distance >= outer_distance {
        return 0.0;
    }
    if surface_distance <= inner_distance {
        return 1.0;
    }

    let t =
        ((outer_distance - surface_distance) / (outer_distance - inner_distance)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Elige una sola zona para impedir que varias músicas planetarias se mezclen a la vez.
pub fn strongest_music_zone<F>(
    ship_position: Vec3,
    zones: &[MusicZoneConfig],
    mut body_lookup: F,
) -> Option<(usize, f32)>
where
    F: FnMut(&str) -> Option<(Vec3, f32)>,
{
    let mut strongest: Option<(usize, f32)> = None;
    for (index, zone) in zones.iter().enumerate() {
        let Some((position, radius)) = body_lookup(&zone.body_id) else {
            continue;
        };
        let influence = music_influence(
            (ship_position - position).length(),
            radius,
            zone.inner_distance,
            zone.outer_distance,
        );
        if influence > strongest.map_or(0.0, |(_, current)| current) {
            strongest = Some((index, influence));
        }
    }
    strongest
}

pub fn smooth_gain(current: f32, target: f32, dt: f32, crossfade_seconds: f32) -> f32 {
    if !current.is_finite()
        || !target.is_finite()
        || !dt.is_finite()
        || dt <= 0.0
        || !crossfade_seconds.is_finite()
        || crossfade_seconds <= 0.0
    {
        return current;
    }
    let alpha = 1.0 - (-dt / crossfade_seconds).exp();
    current + (target - current) * alpha
}
