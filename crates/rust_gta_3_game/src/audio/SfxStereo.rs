//! Fixed native 2D pan endpoints for SetupJumboRumbleSound's original PCM.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum StereoSide {
    Left,
    Right,
}
impl StereoSide {
    pub(super) fn from_native_pan(pan: u8) -> Option<Self> {
        match pan {
            0 => Some(Self::Left),
            127 => Some(Self::Right),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct StereoSourceKey {
    sample: u32,
    loop_region: bool,
    side: StereoSide,
}

/// Route installed mono PCM16 to a stereo endpoint without modifying its samples.
fn stereo_endpoint_wav(pcm: &[u8], frequency: u32, side: StereoSide) -> Option<Vec<u8>> {
    if pcm.is_empty() || pcm.len() % 2 != 0 || frequency == 0 {
        return None;
    }
    let mut stereo = Vec::with_capacity(pcm.len().checked_mul(2)?);
    const PCM16_SILENCE: [u8; 2] = [0, 0];
    for sample in pcm.chunks_exact(2) {
        match side {
            StereoSide::Left => {
                stereo.extend_from_slice(sample);
                stereo.extend_from_slice(&PCM16_SILENCE);
            }
            StereoSide::Right => {
                stereo.extend_from_slice(&PCM16_SILENCE);
                stereo.extend_from_slice(sample);
            }
        }
    }
    Some(pcm16_wav(&stereo, frequency, 2))
}

pub(super) fn stereo_source(
    playback: &mut GtaSfxPlayback,
    audio_sources: &mut Assets<AudioSource>,
    sample: u32,
    descriptor: SampleMetadata,
    loop_region: bool,
    side: StereoSide,
) -> Option<Handle<AudioSource>> {
    let key = StereoSourceKey {
        sample,
        loop_region,
        side,
    };
    if let Some(source) = playback.stereo_sources.get(&key) {
        return Some(source.clone());
    }
    let wav = sample_pcm(&playback.raw, descriptor, loop_region)
        .and_then(|pcm| stereo_endpoint_wav(pcm, descriptor.frequency, side));
    let Some(wav) = wav else {
        playback.invalid_pcm_events += 1;
        bevy::log::warn_once!(sample, loop_region, "sfx_stereo_invalid_original_pcm");
        return None;
    };
    let source = audio_sources.add(AudioSource { bytes: wav.into() });
    playback.stereo_sources.insert(key, source.clone());
    Some(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stereo_routing_preserves_signed_original_samples_and_silences_only_opposite_side() {
        let samples = [i16::MIN, -123, 0, 123, i16::MAX];
        let pcm: Vec<_> = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        for side in [StereoSide::Left, StereoSide::Right] {
            let wav = stereo_endpoint_wav(&pcm, 22050, side).unwrap();
            assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2);
            assert_eq!(u16::from_le_bytes(wav[32..34].try_into().unwrap()), 4);
            assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 88200);
            for (frame, original) in wav[44..].chunks_exact(4).zip(samples) {
                let left = i16::from_le_bytes(frame[..2].try_into().unwrap());
                let right = i16::from_le_bytes(frame[2..].try_into().unwrap());
                match side {
                    StereoSide::Left => {
                        assert_eq!(left, original);
                        assert_eq!(right, 0);
                    }
                    StereoSide::Right => {
                        assert_eq!(left, 0);
                        assert_eq!(right, original);
                    }
                }
            }
        }
        assert!(stereo_endpoint_wav(&[1], 22050, StereoSide::Left).is_none());
        assert!(stereo_endpoint_wav(&pcm, 0, StereoSide::Right).is_none());
        assert!(StereoSide::from_native_pan(63).is_none());
    }
}
