//! re3 reference: references/re-GTA/src/audio/sampman.h

use super::AudioManager::MAX_VOLUME;
use super::AudioSamples::TOTAL_AUDIO_SAMPLES;

pub const CAR_SFX_BANKS_OFFSET: usize = 1;
pub const SFX_BANK_PACARD: usize = CAR_SFX_BANKS_OFFSET;
pub const SFX_BANK_PATHFINDER: usize = 2;
pub const SFX_BANK_PORSCHE: usize = 3;
pub const SFX_BANK_SPIDER: usize = 4;
pub const SFX_BANK_MERC: usize = 5;
pub const SFX_BANK_TRUCK: usize = 6;
pub const SFX_BANK_HOTROD: usize = 7;
pub const SFX_BANK_COBRA: usize = 8;
pub const SFX_BANK_NONE: usize = 9;
pub const SFX_BANK_PED_COMMENTS: usize = 1;
pub const MAX_SFX_BANKS: usize = 10;
pub const INVALID_SFX_BANK: usize = usize::MAX;
pub const MAX_PEDSFX: usize = 7;

#[derive(Clone, Copy)]
pub(crate) struct PedCommentLoadContext {
    pub code_paused: bool,
    pub cutscene_music: bool,
    pub game_completed_music: bool,
}

/// Exact 20-byte entry used by GTA III `AUDIO/SFX.SDT` (`tSample` in re3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SampleMetadata {
    pub offset: u32,
    pub size: u32,
    pub frequency: u32,
    pub loop_start: u32,
    pub loop_end: i32,
}

pub fn parse_sfx_sdt(bytes: &[u8]) -> Vec<SampleMetadata> {
    bytes
        .chunks_exact(20)
        .map(|entry| SampleMetadata {
            offset: u32::from_le_bytes(entry[0..4].try_into().unwrap()),
            size: u32::from_le_bytes(entry[4..8].try_into().unwrap()),
            frequency: u32::from_le_bytes(entry[8..12].try_into().unwrap()),
            loop_start: u32::from_le_bytes(entry[12..16].try_into().unwrap()),
            loop_end: i32::from_le_bytes(entry[16..20].try_into().unwrap()),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Stopped,
    Playing { sample: u32, bank: u8 },
}

#[derive(Debug, Clone)]
pub struct cSampleManager {
    pub initialised: bool,
    pub effects_master_volume: u8,
    pub music_master_volume: u8,
    pub effects_fade_volume: u8,
    pub music_fade_volume: u8,
    pub mono_mode: bool,
    pub channels: Vec<ChannelState>,
    pub streams: Vec<Option<u32>>,
    pub bank_start_offset: [u32; MAX_SFX_BANKS],
    pub bank_loaded: [bool; MAX_SFX_BANKS],
    sample_metadata: Vec<SampleMetadata>,
    ped_comment_slots: [Option<u32>; MAX_PEDSFX],
    next_ped_comment_slot: usize,
}

impl Default for cSampleManager {
    fn default() -> Self {
        Self {
            initialised: false,
            effects_master_volume: MAX_VOLUME,
            music_master_volume: MAX_VOLUME,
            effects_fade_volume: MAX_VOLUME,
            music_fade_volume: MAX_VOLUME,
            mono_mode: false,
            channels: vec![ChannelState::Stopped; 28],
            streams: vec![None; 2],
            bank_start_offset: [0; MAX_SFX_BANKS],
            bank_loaded: [false; MAX_SFX_BANKS],
            sample_metadata: Vec::new(),
            ped_comment_slots: [None; MAX_PEDSFX],
            next_ped_comment_slot: 0,
        }
    }
}

impl cSampleManager {
    pub fn Initialise(&mut self) -> bool {
        self.ped_comment_slots.fill(None);
        self.next_ped_comment_slot = 0;
        self.initialised = true;
        true
    }

    pub fn Terminate(&mut self) {
        self.initialised = false;
        self.channels.fill(ChannelState::Stopped);
        self.streams.fill(None);
        self.bank_loaded = [false; MAX_SFX_BANKS];
        self.ped_comment_slots.fill(None);
        self.next_ped_comment_slot = 0;
    }

    pub fn GetSampleBaseFrequency(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.frequency)
            .unwrap_or_else(|| {
                (sample < TOTAL_AUDIO_SAMPLES as u32)
                    .then_some(22_050)
                    .unwrap_or(0)
            })
    }

    pub fn GetSampleLength(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.size / 2)
            .unwrap_or(0)
    }

    pub fn GetSampleLoopStartOffset(&self, sample: u32) -> u32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.loop_start)
            .unwrap_or(0)
    }

    pub fn GetSampleLoopEndOffset(&self, sample: u32) -> i32 {
        self.sample_metadata
            .get(sample as usize)
            .map(|sample| sample.loop_end)
            .unwrap_or(-1)
    }

    pub fn LoadSampleMetadata(&mut self, bytes: &[u8]) {
        self.sample_metadata = parse_sfx_sdt(bytes);
    }

    pub fn sample_metadata(&self) -> &[SampleMetadata] {
        &self.sample_metadata
    }

    pub(crate) fn is_ped_comment_loaded(&self, sample: u32) -> bool {
        (1..=3).any(|offset| {
            let slot = (self.next_ped_comment_slot + MAX_PEDSFX - offset) % MAX_PEDSFX;
            self.ped_comment_slots[slot] == Some(sample)
        })
    }

    /// sampman_oal.cpp::LoadPedComment, after the installed PCM owner has
    /// validated the complete sample. Slots reflect native load admission;
    /// the canonical Bevy backend retains the actual AudioSource bytes.
    pub(crate) fn load_ped_comment(&mut self, sample: u32, context: PedCommentLoadContext,
        original_pcm_available: bool) -> bool
    {
        if context.code_paused || context.cutscene_music || context.game_completed_music
            || !original_pcm_available
        { return false; }
        self.ped_comment_slots[self.next_ped_comment_slot] = Some(sample);
        self.next_ped_comment_slot = (self.next_ped_comment_slot + 1) % MAX_PEDSFX;
        true
    }

    pub fn SetSampleBankStartOffset(&mut self, bank: usize, offset: u32) {
        if bank < MAX_SFX_BANKS {
            self.bank_start_offset[bank] = offset;
        }
    }

    pub fn IsSampleBankLoaded(&self, bank: usize) -> bool {
        self.bank_loaded.get(bank).copied().unwrap_or(false)
    }

    pub fn LoadSampleBank(&mut self, bank: usize) -> bool {
        if bank < MAX_SFX_BANKS {
            self.bank_loaded[bank] = true;
            true
        } else {
            false
        }
    }

    pub fn UnloadSampleBank(&mut self, bank: usize) {
        if bank < MAX_SFX_BANKS {
            self.bank_loaded[bank] = false;
        }
    }

    pub fn GetStreamedFileLength(&self, _track: u32) -> u32 {
        1
    }

    pub fn SetEffectsMasterVolume(&mut self, volume: u8) {
        self.effects_master_volume = volume.min(MAX_VOLUME);
    }

    pub fn SetMusicMasterVolume(&mut self, volume: u8) {
        self.music_master_volume = volume.min(MAX_VOLUME);
    }

    pub fn SetEffectsFadeVolume(&mut self, volume: u8) {
        self.effects_fade_volume = volume.min(MAX_VOLUME);
    }

    pub fn SetMusicFadeVolume(&mut self, volume: u8) {
        self.music_fade_volume = volume.min(MAX_VOLUME);
    }

    pub fn SetMonoMode(&mut self, mono: bool) {
        self.mono_mode = mono;
    }

    pub fn StartChannel(&mut self, channel: usize, sample: u32, bank: u8) -> bool {
        if let Some(slot) = self.channels.get_mut(channel) {
            *slot = ChannelState::Playing { sample, bank };
            true
        } else {
            false
        }
    }

    pub fn StopChannel(&mut self, channel: usize) {
        if let Some(slot) = self.channels.get_mut(channel) {
            *slot = ChannelState::Stopped;
        }
    }

    pub fn GetChannelUsedFlag(&self, channel: usize) -> bool {
        matches!(
            self.channels.get(channel),
            Some(ChannelState::Playing { .. })
        )
    }

    pub fn StartStreamedFile(&mut self, track: u32, stream: usize) -> bool {
        if let Some(slot) = self.streams.get_mut(stream) {
            *slot = Some(track);
            true
        } else {
            false
        }
    }

    pub fn StopStreamedFile(&mut self, stream: usize) {
        if let Some(slot) = self.streams.get_mut(stream) {
            *slot = None;
        }
    }

    pub fn IsStreamPlaying(&self, stream: usize) -> bool {
        self.streams.get(stream).is_some_and(Option::is_some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdt_metadata_matches_re3_tsample_layout() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1234_u32.to_le_bytes());
        bytes.extend_from_slice(&5678_u32.to_le_bytes());
        bytes.extend_from_slice(&32_000_u32.to_le_bytes());
        bytes.extend_from_slice(&222_u32.to_le_bytes());
        bytes.extend_from_slice(&444_i32.to_le_bytes());

        let parsed = parse_sfx_sdt(&bytes);
        assert_eq!(
            parsed,
            vec![SampleMetadata {
                offset: 1234,
                size: 5678,
                frequency: 32_000,
                loop_start: 222,
                loop_end: 444,
            }]
        );

        let mut manager = cSampleManager::default();
        manager.LoadSampleMetadata(&bytes);
        assert_eq!(manager.GetSampleBaseFrequency(0), 32_000);
        assert_eq!(manager.GetSampleLength(0), 2839);
        assert_eq!(manager.GetSampleLoopStartOffset(0), 222);
        assert_eq!(manager.GetSampleLoopEndOffset(0), 444);
    }
}
