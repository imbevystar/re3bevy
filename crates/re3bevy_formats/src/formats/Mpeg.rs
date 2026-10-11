//! MPEG-1/MP2 program streams used by the original GTA III startup movies.
//! PL_MPEG is compiled into the application; no player process or media cache.

use std::{ffi::c_int, ptr::NonNull};

#[repr(C)]
struct Plane {
    width: u32,
    height: u32,
    data: *mut u8,
}

#[repr(C)]
struct Frame {
    time: f64,
    width: u32,
    height: u32,
    y: Plane,
    cr: Plane,
    cb: Plane,
}

#[repr(C)]
struct Samples {
    time: f64,
    count: u32,
    interleaved: [f32; 2304],
}

unsafe extern "C" {
    fn plm_create_with_memory(
        bytes: *mut u8,
        length: usize,
        free_when_done: c_int,
    ) -> *mut std::ffi::c_void;
    fn plm_destroy(decoder: *mut std::ffi::c_void);
    fn plm_has_headers(decoder: *mut std::ffi::c_void) -> c_int;
    fn plm_get_width(decoder: *mut std::ffi::c_void) -> c_int;
    fn plm_get_height(decoder: *mut std::ffi::c_void) -> c_int;
    fn plm_get_framerate(decoder: *mut std::ffi::c_void) -> f64;
    fn plm_get_num_audio_streams(decoder: *mut std::ffi::c_void) -> c_int;
    fn plm_get_samplerate(decoder: *mut std::ffi::c_void) -> c_int;
    fn plm_set_video_enabled(decoder: *mut std::ffi::c_void, enabled: c_int);
    fn plm_set_audio_enabled(decoder: *mut std::ffi::c_void, enabled: c_int);
    fn plm_rewind(decoder: *mut std::ffi::c_void);
    fn re3bevy_mpeg_decode_video(decoder: *mut std::ffi::c_void) -> *mut Frame;
    fn plm_decode_audio(decoder: *mut std::ffi::c_void) -> *mut Samples;
    fn plm_frame_to_rgba(frame: *mut Frame, destination: *mut u8, stride: c_int);
}

#[derive(Clone, Copy, Debug)]
pub struct MovieInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub sample_rate: Option<u32>,
}

#[derive(Debug)]
pub struct MovieFrame {
    pub time: f64,
    pub rgba: Vec<u8>,
}

/// A decoder owns its immutable input allocation and is confined to its worker.
pub struct MpegDecoder {
    decoder: NonNull<std::ffi::c_void>,
    _input: Vec<u8>,
    pub info: MovieInfo,
}

impl MpegDecoder {
    pub fn new(mut input: Vec<u8>) -> Result<Self, String> {
        if input.len() < 12
            || !input
                .windows(4)
                .take(4096)
                .any(|bytes| bytes == [0, 0, 1, 0xba])
        {
            return Err("not an MPEG program stream".into());
        }
        // SAFETY: PL_MPEG borrows this fixed input buffer without writing or
        // freeing it. Moving the Vec never moves its allocation. Drop destroys
        // the decoder before Rust releases that allocation.
        let decoder =
            NonNull::new(unsafe { plm_create_with_memory(input.as_mut_ptr(), input.len(), 0) })
                .ok_or("cannot allocate MPEG decoder")?;
        let mut result = Self {
            decoder,
            _input: input,
            info: MovieInfo {
                width: 0,
                height: 0,
                fps: 0.0,
                sample_rate: None,
            },
        };
        // SAFETY: the decoder is live and exclusively owned by this wrapper.
        unsafe {
            let ptr = result.decoder.as_ptr();
            if plm_has_headers(ptr) == 0 {
                return Err("incomplete MPEG-1/MP2 headers".into());
            }
            let width = plm_get_width(ptr);
            let height = plm_get_height(ptr);
            let fps = plm_get_framerate(ptr);
            if !(1..=4095).contains(&width)
                || !(1..=4095).contains(&height)
                || !fps.is_finite()
                || !(1.0..=120.0).contains(&fps)
            {
                return Err("invalid MPEG video dimensions or frame rate".into());
            }
            let sample_rate = if plm_get_num_audio_streams(ptr) > 0 {
                let rate = plm_get_samplerate(ptr);
                if ![32000, 44100, 48000].contains(&rate) {
                    return Err("invalid MP2 sample rate".into());
                }
                Some(rate as u32)
            } else {
                None
            };
            result.info = MovieInfo {
                width: width as u32,
                height: height as u32,
                fps,
                sample_rate,
            };
        }
        Ok(result)
    }

    /// Decode the short startup soundtrack once, then rewind for bounded video
    /// streaming. At 48 kHz stereo this retains 192 KB per second of PCM.
    pub fn soundtrack(&mut self, cancelled: impl Fn() -> bool) -> Result<Option<Vec<u8>>, String> {
        let Some(rate) = self.info.sample_rate else {
            return Ok(None);
        };
        let mut pcm = Vec::new();
        // SAFETY: no FFI pointers escape this method. Disabling video prevents
        // its compressed packets accumulating during the audio-only pass.
        unsafe {
            let ptr = self.decoder.as_ptr();
            plm_set_video_enabled(ptr, 0);
            loop {
                if cancelled() {
                    return Err("movie cancelled".into());
                }
                let samples = plm_decode_audio(ptr);
                if samples.is_null() {
                    break;
                }
                let samples = &*samples;
                if samples.count > 1152 {
                    return Err("invalid MP2 sample count".into());
                }
                // GTA's longest startup movie is 95 seconds; reject unbounded
                // soundtracks instead of exhausting application memory.
                if pcm.len() + samples.count as usize * 4 > rate as usize * 4 * 600 {
                    return Err("startup movie soundtrack exceeds ten minutes".into());
                }
                for &sample in &samples.interleaved[..samples.count as usize * 2] {
                    let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
                    pcm.extend_from_slice(&value.to_le_bytes());
                }
            }
            plm_rewind(ptr);
            plm_set_audio_enabled(ptr, 0);
            plm_set_video_enabled(ptr, 1);
        }
        if pcm.is_empty() {
            return Err("MPEG soundtrack produced no samples".into());
        }
        let size = pcm.len() as u32;
        let mut wav = Vec::with_capacity(pcm.len() + 44);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(size + 36).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 4).to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&size.to_le_bytes());
        wav.extend_from_slice(&pcm);
        Ok(Some(wav))
    }

    pub fn next_frame(&mut self) -> Result<Option<MovieFrame>, String> {
        // SAFETY: frame storage is decoder-owned and is copied before the next
        // decode. Conversion writes exactly width*height pixels at this stride.
        unsafe {
            let frame = re3bevy_mpeg_decode_video(self.decoder.as_ptr());
            if frame.is_null() {
                return Ok(None);
            }
            if (*frame).width != self.info.width
                || (*frame).height != self.info.height
                || !(*frame).time.is_finite()
            {
                return Err("MPEG video changed dimensions or has an invalid timestamp".into());
            }
            // PL_MPEG writes RGB only, leaving alpha unchanged.
            let mut rgba = vec![255; self.info.width as usize * self.info.height as usize * 4];
            plm_frame_to_rgba(frame, rgba.as_mut_ptr(), (self.info.width * 4) as c_int);
            Ok(Some(MovieFrame {
                time: (*frame).time,
                rgba,
            }))
        }
    }
}

impl Drop for MpegDecoder {
    fn drop(&mut self) {
        // SAFETY: unique ownership; decoder destroyed while input is still live.
        unsafe {
            plm_destroy(self.decoder.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_truncated_streams() {
        assert!(MpegDecoder::new(Vec::new()).is_err());
        assert!(MpegDecoder::new(vec![0, 0, 1, 0xba, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }

    #[test]
    #[ignore = "requires RE3BEVY_MOVIE_TEST_DIR pointing to original GTA III movies"]
    fn original_startup_movies_decode_with_audio_and_opaque_frames() {
        let root = std::path::PathBuf::from(
            std::env::var_os("RE3BEVY_MOVIE_TEST_DIR").expect("movie directory"),
        );
        for name in ["Logo.mpg", "GTAtitles.mpg", "GTAtitlesGER.mpg"] {
            let mut decoder = MpegDecoder::new(std::fs::read(root.join(name)).unwrap()).unwrap();
            assert_eq!((decoder.info.width, decoder.info.height), (640, 480));
            assert_eq!(decoder.info.fps, 30.0);
            assert_eq!(decoder.info.sample_rate, Some(48000));
            let wav = decoder.soundtrack(|| false).unwrap().unwrap();
            let audio_seconds = (wav.len() - 44) as f64 / 192000.0;
            let mut frames = 0;
            let mut last_time = 0.0;
            while let Some(frame) = decoder.next_frame().unwrap() {
                assert!((frame.time - frames as f64 / decoder.info.fps).abs() < 0.001);
                assert_eq!(frame.rgba.len(), 640 * 480 * 4);
                if frames % 1000 == 0 {
                    assert!(frame.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
                }
                last_time = frame.time;
                frames += 1;
            }
            let video_seconds = last_time + 1.0 / decoder.info.fps;
            assert_eq!(frames, if name == "Logo.mpg" { 476 } else { 2850 });
            assert!((video_seconds - audio_seconds).abs() < 0.15);
            eprintln!(
                "{name}: {frames} frames, video={video_seconds:.3}s audio={audio_seconds:.3}s"
            );
        }
    }
}
