//! re3 reference: references/re-GTA/src/audio/MusicManager.cpp
//! re3 reference: references/re-GTA/src/audio/MusicManager.h

use super::audio_enums::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct tStreamedSample {
    pub m_nLength: u32,
    pub m_nPosition: u32,
    pub m_nLastPosCheckTimer: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicServiceContext {
    pub time_ms_pause_mode: u32,
    pub player_in_car: bool,
    pub player_vehicle_radio: u8,
    pub player_vehicle_uses_police_radio: bool,
    pub stream_playing: bool,
    pub streamed_file_position: u32,
    pub mp3_radio_available: bool,
    /// re3 `CPad::ChangeStationJustDown`, already gated by player/replay and
    /// police-radio conditions at the gameplay boundary.
    pub change_station_just_down: bool,
}

impl Default for MusicServiceContext {
    fn default() -> Self {
        Self {
            time_ms_pause_mode: 0,
            player_in_car: false,
            player_vehicle_radio: HEAD_RADIO as u8,
            player_vehicle_uses_police_radio: false,
            stream_playing: false,
            streamed_file_position: 0,
            mp3_radio_available: true,
            change_station_just_down: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicCommand {
    StopStreamedFile,
    PreloadStreamedFile {
        track: u8,
    },
    StartPreloadedStreamedFile,
    StartStreamedFile {
        track: u8,
        position: u32,
    },
    SetStreamVolumeAndPan {
        volume: u8,
        pan: u8,
        front_end: bool,
    },
}

#[derive(Debug, Clone)]
pub struct cMusicManager {
    pub m_bIsInitialised: bool,
    pub m_bDisabled: bool,
    pub m_nMusicMode: u8,
    pub m_nNextTrack: u8,
    pub m_nPlayingTrack: u8,
    pub m_bFrontendTrackFinished: bool,
    pub m_bPlayInFrontend: bool,
    pub m_bSetNextStation: bool,
    pub m_nAnnouncement: u8,
    pub m_bPreviousPlayerInCar: bool,
    pub m_bPlayerInCar: bool,
    pub m_bAnnouncementInProgress: bool,
    pub m_aTracks: Vec<tStreamedSample>,
    pub m_bResetTimers: bool,
    pub m_nResetTime: u32,
    pub m_nLastTrackServiceTime: u32,
    pub m_nTimer: u32,
    pub m_bDoTrackService: bool,
    pub m_bIgnoreTimeDelay: bool,
    pub m_bVerifyAmbienceTrackStartedToPlay: bool,
    pub m_bRadioSetByScript: bool,
    pub m_nRadioStationScript: u8,
    pub m_nRadioPosition: i32,
    pub m_nRadioInCar: u8,
    pub gNumRetunePresses: i32,
    pub gRetuneCounter: i32,
    pub bHasStarted: bool,
    pub emitted_commands: Vec<MusicCommand>,
    pending_vehicle_radio_update: Option<u8>,
}

impl Default for cMusicManager {
    fn default() -> Self {
        Self::new()
    }
}

impl cMusicManager {
    pub fn new() -> Self {
        Self {
            m_bIsInitialised: false,
            m_bDisabled: false,
            m_nMusicMode: MUSICMODE_DISABLED as u8,
            m_nNextTrack: NO_TRACK as u8,
            m_nPlayingTrack: NO_TRACK as u8,
            m_bFrontendTrackFinished: false,
            m_bPlayInFrontend: false,
            m_bSetNextStation: false,
            m_nAnnouncement: NO_TRACK as u8,
            m_bPreviousPlayerInCar: false,
            m_bPlayerInCar: false,
            m_bAnnouncementInProgress: false,
            m_aTracks: vec![tStreamedSample::default(); TOTAL_STREAMED_SOUNDS as usize],
            m_bResetTimers: false,
            m_nResetTime: 0,
            m_nLastTrackServiceTime: 0,
            m_nTimer: 0,
            m_bDoTrackService: false,
            m_bIgnoreTimeDelay: false,
            m_bVerifyAmbienceTrackStartedToPlay: false,
            m_bRadioSetByScript: false,
            m_nRadioStationScript: HEAD_RADIO as u8,
            m_nRadioPosition: -1,
            m_nRadioInCar: NO_TRACK as u8,
            gNumRetunePresses: 0,
            gRetuneCounter: 0,
            bHasStarted: false,
            emitted_commands: Vec::new(),
            pending_vehicle_radio_update: None,
        }
    }

    pub fn IsInitialised(&self) -> bool {
        self.m_bIsInitialised
    }

    pub fn GetMusicMode(&self) -> u32 {
        self.m_nMusicMode as u32
    }

    pub fn GetNextTrack(&self) -> u8 {
        self.m_nNextTrack
    }

    pub fn Initialise(&mut self, track_lengths: &[u32], timer_ms: u32, seed_pos: u32) -> bool {
        if !self.IsInitialised() {
            for i in 0..TOTAL_STREAMED_SOUNDS as usize {
                let length = track_lengths.get(i).copied().unwrap_or(1).max(1);
                self.m_aTracks[i].m_nLength = length;
                self.m_aTracks[i].m_nPosition = seed_pos % length;
                self.m_aTracks[i].m_nLastPosCheckTimer = timer_ms;
            }
            self.m_bResetTimers = false;
            self.m_nResetTime = 0;
            self.m_nTimer = timer_ms;
            self.m_nLastTrackServiceTime = timer_ms;
            self.m_bDoTrackService = false;
            self.m_bIgnoreTimeDelay = false;
            self.m_bRadioSetByScript = false;
            self.m_nRadioStationScript = HEAD_RADIO as u8;
            self.m_nRadioPosition = -1;
            self.m_nRadioInCar = NO_TRACK as u8;
            self.gNumRetunePresses = 0;
            self.gRetuneCounter = 0;
            self.m_bIsInitialised = true;
        }
        self.m_bIsInitialised
    }

    pub fn Terminate(&mut self, stream_playing: bool) {
        if !self.IsInitialised() {
            return;
        }
        if stream_playing {
            self.emitted_commands.push(MusicCommand::StopStreamedFile);
            self.m_nNextTrack = NO_TRACK as u8;
            self.m_nPlayingTrack = NO_TRACK as u8;
        }
        self.m_bIsInitialised = false;
    }

    pub fn ChangeMusicMode(&mut self, mode: u8, ctx: &MusicServiceContext) {
        if !self.IsInitialised() {
            return;
        }

        let mode2 = match mode as i32 {
            MUSICMODE_FRONTEND => MUSICMODE_FRONTEND as u8,
            MUSICMODE_GAME => MUSICMODE_GAME as u8,
            MUSICMODE_CUTSCENE => MUSICMODE_CUTSCENE as u8,
            MUSICMODE_DISABLE => MUSICMODE_DISABLED as u8,
            _ => return,
        };

        if mode2 != self.m_nMusicMode
            || (mode as i32 == MUSICMODE_FRONTEND && mode2 == MUSICMODE_FRONTEND as u8)
        {
            if ctx.stream_playing {
                if (self.m_nNextTrack as i32) < TOTAL_STREAMED_SOUNDS {
                    let track = self.m_nNextTrack as usize;
                    self.m_aTracks[track].m_nPosition = ctx.streamed_file_position;
                    self.m_aTracks[track].m_nLastPosCheckTimer = ctx.time_ms_pause_mode;
                }
                self.emitted_commands.push(MusicCommand::StopStreamedFile);
            }
            self.m_nNextTrack = NO_TRACK as u8;
            self.m_nPlayingTrack = NO_TRACK as u8;
            self.m_bFrontendTrackFinished = false;
            self.m_bPlayInFrontend = false;
            self.m_bSetNextStation = false;
            self.m_bPreviousPlayerInCar = false;
            self.m_bPlayerInCar = false;
            self.m_bAnnouncementInProgress = false;
            self.m_nTimer = ctx.time_ms_pause_mode;
            self.m_nLastTrackServiceTime = ctx.time_ms_pause_mode;
            self.m_bDoTrackService = false;
            self.m_bIgnoreTimeDelay = true;
            self.m_bVerifyAmbienceTrackStartedToPlay = false;
            self.m_nMusicMode = mode2;
        }
    }

    pub fn StopFrontEndTrack(&mut self) {
        if self.m_nMusicMode == MUSICMODE_FRONTEND as u8 {
            self.m_nNextTrack = NO_TRACK as u8;
            self.m_nPlayingTrack = NO_TRACK as u8;
            self.m_bFrontendTrackFinished = true;
            self.emitted_commands.push(MusicCommand::StopStreamedFile);
        }
    }

    pub fn PlayAnnouncement(&mut self, announcement: u8) {
        if self.m_bIsInitialised {
            self.m_nAnnouncement = announcement;
            self.m_bAnnouncementInProgress = true;
        }
    }

    pub fn PlayFrontEndTrack(&mut self, track: u8, frontend_flag: bool) {
        if self.m_bIsInitialised {
            self.m_nNextTrack = track;
            self.m_nPlayingTrack = NO_TRACK as u8;
            self.m_bFrontendTrackFinished = false;
            self.m_bPlayInFrontend = frontend_flag;
            self.bHasStarted = false;
        }
    }

    pub fn PreloadCutSceneMusic(&mut self, track: u8, stream_playing: bool) {
        if self.m_bIsInitialised
            && !self.m_bDisabled
            && (track as i32) < TOTAL_STREAMED_SOUNDS
            && self.m_nMusicMode == MUSICMODE_CUTSCENE as u8
        {
            if stream_playing {
                self.emitted_commands.push(MusicCommand::StopStreamedFile);
            }
            self.emitted_commands
                .push(MusicCommand::PreloadStreamedFile { track });
            self.emitted_commands
                .push(MusicCommand::SetStreamVolumeAndPan {
                    volume: 127,
                    pan: 63,
                    front_end: true,
                });
            self.m_nNextTrack = track;
        }
    }

    pub fn PlayPreloadedCutSceneMusic(&mut self) {
        if self.m_bIsInitialised
            && !self.m_bDisabled
            && self.m_nMusicMode == MUSICMODE_CUTSCENE as u8
        {
            self.emitted_commands
                .push(MusicCommand::StartPreloadedStreamedFile);
        }
    }

    pub fn StopCutSceneMusic(&mut self) {
        if self.m_bIsInitialised
            && !self.m_bDisabled
            && self.m_nMusicMode == MUSICMODE_CUTSCENE as u8
        {
            self.emitted_commands.push(MusicCommand::StopStreamedFile);
            self.m_nNextTrack = NO_TRACK as u8;
        }
    }

    pub fn GetRadioInCar(&self, ctx: &MusicServiceContext) -> u8 {
        if !self.m_bIsInitialised {
            return HEAD_RADIO as u8;
        }
        if ctx.player_in_car {
            if ctx.player_vehicle_uses_police_radio {
                if self.m_nRadioInCar as i32 == NO_TRACK {
                    return POLICE_RADIO as u8;
                }
                return self.m_nRadioInCar;
            }
            return ctx.player_vehicle_radio;
        }
        if self.m_nRadioInCar as i32 == NO_TRACK {
            RADIO_OFF as u8
        } else {
            self.m_nRadioInCar
        }
    }

    pub fn SetRadioInCar(&mut self, station: u32, ctx: &MusicServiceContext) -> Option<u8> {
        if !self.m_bIsInitialised {
            return None;
        }
        if !ctx.player_in_car || ctx.player_vehicle_uses_police_radio {
            self.m_nRadioInCar = station as u8;
            None
        } else {
            Some(station as u8)
        }
    }

    pub fn SetRadioChannelByScript(&mut self, station: u8, pos: i32) {
        if self.m_bIsInitialised && (station as i32) < RADIO_OFF {
            self.m_bRadioSetByScript = true;
            self.m_nRadioStationScript = station;
            let length = self.m_aTracks[station as usize].m_nLength.max(1) as i32;
            self.m_nRadioPosition = if pos == -1 { -1 } else { pos % length };
        }
    }

    pub fn ResetMusicAfterReload(&mut self) {
        self.m_bRadioSetByScript = false;
        self.m_nRadioStationScript = 0;
        self.m_nRadioPosition = -1;
        self.m_nAnnouncement = NO_TRACK as u8;
        self.m_bAnnouncementInProgress = false;
        self.m_bSetNextStation = false;
        self.gRetuneCounter = 0;
        self.gNumRetunePresses = 0;
    }

    pub fn ResetTimers(&mut self, time: u32) {
        self.m_bResetTimers = true;
        self.m_nResetTime = time;
    }

    pub fn Service(&mut self, ctx: &MusicServiceContext) {
        if self.m_bResetTimers {
            self.m_bResetTimers = false;
            self.m_nLastTrackServiceTime = self.m_nResetTime;
        }
        if !self.m_bIsInitialised || self.m_bDisabled {
            return;
        }
        if self.m_nMusicMode == MUSICMODE_CUTSCENE as u8 {
            self.emitted_commands
                .push(MusicCommand::SetStreamVolumeAndPan {
                    volume: 127,
                    pan: 63,
                    front_end: true,
                });
            return;
        }
        self.m_nTimer = ctx.time_ms_pause_mode;
        if self.m_nTimer > self.m_nLastTrackServiceTime.saturating_add(2000)
            || self.m_bIgnoreTimeDelay
        {
            self.m_bIgnoreTimeDelay = false;
            self.m_bDoTrackService = true;
            self.m_nLastTrackServiceTime = self.m_nTimer;
        } else {
            self.m_bDoTrackService = false;
        }
        if self.m_nNextTrack as i32 == NO_TRACK && ctx.stream_playing {
            self.emitted_commands.push(MusicCommand::StopStreamedFile);
            return;
        }
        match self.m_nMusicMode as i32 {
            MUSICMODE_FRONTEND => self.ServiceFrontEndMode(ctx),
            MUSICMODE_GAME => self.ServiceGameMode(ctx),
            _ => {}
        }
    }

    pub fn ServiceFrontEndMode(&mut self, ctx: &MusicServiceContext) {
        if (self.m_nNextTrack as i32) < TOTAL_STREAMED_SOUNDS {
            if self.m_bFrontendTrackFinished {
                if !ctx.stream_playing {
                    self.m_nNextTrack = NO_TRACK as u8;
                    self.m_nPlayingTrack = NO_TRACK as u8;
                }
            } else if self.bHasStarted {
                if !ctx.stream_playing {
                    self.emitted_commands.push(MusicCommand::StartStreamedFile {
                        track: self.m_nNextTrack,
                        position: 0,
                    });
                }
            } else {
                self.emitted_commands
                    .push(MusicCommand::SetStreamVolumeAndPan {
                        volume: 0,
                        pan: 63,
                        front_end: false,
                    });
                let position = if (self.m_nNextTrack as i32) < NUM_RADIOS {
                    self.GetTrackStartPos(self.m_nNextTrack)
                } else {
                    0
                };
                self.emitted_commands.push(MusicCommand::StartStreamedFile {
                    track: self.m_nNextTrack,
                    position,
                });
                self.emitted_commands
                    .push(MusicCommand::SetStreamVolumeAndPan {
                        volume: 100,
                        pan: 63,
                        front_end: false,
                    });
                if self.m_bPlayInFrontend {
                    self.bHasStarted = true;
                } else {
                    self.m_bFrontendTrackFinished = true;
                }
            }
        }
    }

    pub fn ServiceGameMode(&mut self, ctx: &MusicServiceContext) {
        self.m_bPreviousPlayerInCar = self.m_bPlayerInCar;
        self.m_bPlayerInCar = ctx.player_in_car;
        self.m_nPlayingTrack = self.m_nNextTrack;
        if !self.m_bPlayerInCar {
            self.gRetuneCounter = 0;
            self.gNumRetunePresses = 0;
            self.m_bSetNextStation = false;
            self.ServiceAmbience();
        } else if self.m_bRadioSetByScript {
            self.m_nNextTrack = self.m_nRadioStationScript;
        } else {
            if ctx.change_station_just_down {
                // MusicManager.cpp:518-519. Multiple presses inside the
                // 30-service retune window accumulate and restart the delay.
                self.gRetuneCounter = 30;
                self.gNumRetunePresses = self.gNumRetunePresses.saturating_add(1);
            }
            if self.m_bPreviousPlayerInCar && self.gNumRetunePresses != 0 {
                if self.gRetuneCounter != 0 {
                    self.gRetuneCounter -= 1;
                } else {
                    self.m_bSetNextStation = true;
                }
            }
            if self.m_bSetNextStation {
                self.m_bSetNextStation = false;
                let station = next_radio_after_presses(
                    ctx.player_vehicle_radio,
                    self.gNumRetunePresses,
                    ctx.mp3_radio_available,
                );
                self.gNumRetunePresses = 0;
                self.m_nNextTrack = station;
                self.pending_vehicle_radio_update = Some(station);
            } else {
                self.m_nNextTrack = self.GetRadioInCar(ctx);
            }
        }
    }

    pub fn take_pending_vehicle_radio_update(&mut self) -> Option<u8> {
        self.pending_vehicle_radio_update.take()
    }

    pub fn ServiceAmbience(&mut self) {
        if self.m_nNextTrack as i32 == NO_TRACK || self.m_nNextTrack == RADIO_OFF as u8 {
            self.m_nNextTrack = STREAMED_SOUND_CITY_AMBIENT as u8;
        }
    }

    pub fn GetTrackStartPos(&self, track: u8) -> u32 {
        if (track as usize) < self.m_aTracks.len() {
            self.m_aTracks[track as usize].m_nPosition
        } else {
            0
        }
    }

    pub fn GetCarTuning(&self, ctx: &MusicServiceContext) -> u8 {
        self.GetRadioInCar(ctx)
    }

    pub fn GetNextCarTuning(&self, ctx: &MusicServiceContext) -> u8 {
        let mut station = self.GetCarTuning(ctx);
        if station == RADIO_OFF as u8 {
            station = HEAD_RADIO as u8;
        } else {
            station = station.saturating_add(1);
            if station as i32 == USERTRACK && !ctx.mp3_radio_available {
                station = RADIO_OFF as u8;
            } else if station as i32 > USERTRACK {
                station = RADIO_OFF as u8;
            }
        }
        station
    }

    pub fn ChangeRadioChannel(&mut self, ctx: &MusicServiceContext) -> u8 {
        let station = self.GetNextCarTuning(ctx);
        self.m_nNextTrack = station;
        station
    }
}

fn next_radio_after_presses(mut station: u8, presses: i32, mp3_available: bool) -> u8 {
    for _ in 0..presses.max(0) {
        if station == RADIO_OFF as u8 {
            station = HEAD_RADIO as u8;
            continue;
        }
        station = station.saturating_add(1);
        if station == USERTRACK as u8 && !mp3_available {
            station = RADIO_OFF as u8;
        } else if station > USERTRACK as u8 {
            station = RADIO_OFF as u8;
        }
    }
    station
}

#[cfg(test)]
mod radio_retune_tests {
    use super::*;

    #[test]
    fn retune_wraps_and_skips_unavailable_usertrack() {
        assert_eq!(next_radio_after_presses(RADIO_OFF as u8, 1, true), HEAD_RADIO as u8);
        assert_eq!(next_radio_after_presses(USERTRACK as u8, 1, true), RADIO_OFF as u8);
        assert_eq!(
            next_radio_after_presses((USERTRACK - 1) as u8, 1, false),
            RADIO_OFF as u8
        );
        assert_eq!(next_radio_after_presses(HEAD_RADIO as u8, 2, true), 2);
    }
}
