#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub mod AudioCollision;
pub mod AudioLogic;
pub mod AudioManager;
pub mod AudioSamples;
pub mod AudioScriptObject;
pub mod DMAudio;
pub mod MusicManager;
pub mod PedTalk;
mod PedCommentPlayback;
pub use PedCommentPlayback::PedCommentPlaybackDiagnostics;
pub mod PolRadio;
pub mod audio_enums;
pub mod eax;
pub mod oal;
pub mod sampman;
pub mod sampman_miles;
pub mod sampman_null;
pub mod sampman_oal;
pub mod soundlist;

pub mod AudioPlugin;
pub mod AudioActorAccess;
mod PhysicalAudioLifecycle;
mod AudioSpatial;
pub use AudioSpatial::SfxSpatialDiagnostics;

/// Explicit compatibility path to the canonical player engine domain.
pub mod PlayerEngineAudio {
    pub use super::AudioLogic::{PlayerCesnaAudioState, PlayerVehicleAudioState, PlayerEngineAudioState,
        process_players_vehicle_engine, process_cesna, add_player_car_sample};
}
