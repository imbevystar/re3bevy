//! Keep CChannel's native gain authoritative at the Bevy spatial boundary.
use super::DMAudio::{GtaAudioListener, GtaSfxVoice};
use bevy::{
    audio::{AudioSinkPlayback, SpatialAudioSink, SpatialListener, SpatialScale},
    prelude::*,
};

#[derive(Default, Resource)]
pub struct SfxSpatialDiagnostics {
    pub projected_voices: u64,
    pub unavailable_listener: u64,
    pub invalid_positions: u64,
}

struct SpatialProjection {
    scale: SpatialScale,
    emitter: Vec3,
    left_ear: Vec3,
    right_ear: Vec3,
}

fn projection(
    listener: &Transform,
    ears: &SpatialListener,
    emitter: Vec3,
) -> Option<SpatialProjection> {
    let left = listener.transform_point(ears.left_ear_offset);
    let right = listener.transform_point(ears.right_ear_offset);
    let distance = emitter.distance(listener.translation);
    let ear_radius = left
        .distance(listener.translation)
        .max(right.distance(listener.translation));
    if !emitter.is_finite()
        || !left.is_finite()
        || !right.is_finite()
        || !distance.is_finite()
        || !ear_radius.is_finite()
        || left == right
    {
        return None;
    }
    // Rodio multiplies each channel by min(1, 1 / ear_distance_squared).
    // Uniformly project source AND ears inside that backend's unit radius:
    // angular pan ratios are unchanged; only its additional distance loss is
    // neutralized. CChannel::SetDistances/SetVolume remain the gain calculation.
    // Half a unit leaves room for floating-point roundoff at distant GTA poses.
    let extent = distance + ear_radius;
    let scale = SpatialScale(Vec3::splat(if extent > 0.0 { 0.5 / extent } else { 1.0 }));
    Some(SpatialProjection {
        scale,
        emitter: emitter * scale.0,
        left_ear: left * scale.0,
        right_ear: right * scale.0,
    })
}

/// After SFX commands materialize, before Bevy starts or updates their sinks.
/// Actor positions and original distance/gain inputs retain their native owners.
pub(crate) fn normalize_sfx_spatial_backend(
    listeners: Query<(&Transform, &SpatialListener), With<GtaAudioListener>>,
    mut voices: Query<
        (&Transform, &mut PlaybackSettings, Option<&SpatialAudioSink>),
        With<GtaSfxVoice>,
    >,
    mut diagnostics: ResMut<SfxSpatialDiagnostics>,
) {
    if !voices.iter().any(|(_, settings, _)| settings.spatial) {
        return;
    }
    let Ok((listener, ears)) = listeners.single() else {
        diagnostics.unavailable_listener += 1;
        warn_once!("sfx_spatial_waiting_unique_native_listener");
        return;
    };
    for (transform, mut settings, sink) in &mut voices {
        if !settings.spatial {
            continue;
        }
        let Some(projected) = projection(listener, ears, transform.translation) else {
            diagnostics.invalid_positions += 1;
            warn_once!("sfx_spatial_invalid_native_positions");
            if let Some(sink) = sink {
                sink.stop();
            }
            continue;
        };
        if settings.spatial_scale.as_ref().map(|scale| scale.0) != Some(projected.scale.0) {
            settings.spatial_scale = Some(projected.scale);
        }
        if let Some(sink) = sink {
            sink.set_ears_position(projected.left_ear, projected.right_ear);
            sink.set_emitter_position(projected.emitter);
        }
        diagnostics.projected_voices += 1;
    }
    // TODO(parity): CChannel::SetPosition/OpenAL directional mixing and HRTF.
    // Rodio's angular mixer differs; this boundary corrects distance gain only.
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn near_and_far_original_emitters_cannot_receive_backend_distance_loss() {
        let listener =
            Transform::from_xyz(1000.0, 40.0, -900.0).with_rotation(Quat::from_rotation_y(0.7));
        let ears = SpatialListener::default();
        for distance in [0.0, 1.0, 50.0, 200.0, 400.0, 2000.0] {
            let emitter = listener.translation + Vec3::X * distance;
            let p = projection(&listener, &ears, emitter).unwrap();
            assert!(p.emitter.distance(p.left_ear) < 1.0);
            assert!(p.emitter.distance(p.right_ear) < 1.0);
            assert_ne!(p.left_ear, p.right_ear);
        }
    }

    #[test]
    fn invalid_audio_pose_has_no_fabricated_projection() {
        assert!(projection(&Transform::IDENTITY, &SpatialListener::default(), Vec3::NAN).is_none());
    }
}
