//! re3 reference: references/re-GTA/src/core/Game.cpp
//! re3 reference: references/re-GTA/src/core/Game.h
//!
//! Shared game state names derived from `CGame` and `eLevelName`.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum GtaAppState {
    #[default]
    Boot,
    LogoMpeg,
    IntroMpeg,
    LoadingFrontend,
    Frontend,
    LoadingGame,
    InGame,
    PauseMenu,
    Error,
}

/// Selects the executable's initial route without forking any game systems.
///
/// `NewGame` mirrors choosing "Start New Game" in re3's frontend: the common
/// one-time startup still runs, then the normal `LoadingGame`/MAIN.SCM path is
/// entered directly. It does not provide a bootstrap world or a test fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub enum GtaBootRoute {
    #[default]
    Frontend,
    NewGame,
}

/// Selects whether the next level initialisation should execute MAIN.SCM's
/// new-game entry point. Developer world teleports still need the real
/// DAT/IDE/IPL/streaming initialisation, but must not launch the story intro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub enum GtaLevelBootMode {
    #[default]
    NormalNewGame,
    DeveloperWorldTeleport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(i32)]
pub enum ELevelName {
    Ignore = -1,
    #[default]
    Generic = 0,
    Industrial = 1,
    Commercial = 2,
    Suburban = 3,
}

#[derive(Debug, Clone, Resource)]
pub struct CGameState {
    pub curr_level: ELevelName,
    pub demo_mode: bool,
    pub nasty_game: bool,
    pub french_game: bool,
    pub german_game: bool,
    pub no_prostitutes: bool,
    pub playing_intro: bool,
    pub dat_file: String,
}

impl Default for CGameState {
    fn default() -> Self {
        Self {
            curr_level: ELevelName::Industrial,
            demo_mode: true,
            nasty_game: true,
            french_game: false,
            german_game: false,
            no_prostitutes: false,
            playing_intro: false,
            dat_file: "DATA\\GTA3.DAT".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CGameInitialiseStep {
    FileMgrInitialise,
    CdStreamInit,
    ValidateVersion,
    TextLoad,
    AudioInitialise,
    TimerInitialise,
    TempColModelsInitialise,
    HandlingManagerInitialise,
    SurfaceTableInitialise,
    PedStatsInitialise,
    TimeCycleInitialise,
    PoolsInitialise,
    GameLogicInitAtStartOfGame,
    ReferencesInit,
    CameraInit,
    PathFindInit,
    WeatherInit,
    CullZonesInit,
    CullZonesVisibilityCache,
    CollisionInit,
    ZonesInit,
    UserDisplayInit,
    MessagesInit,
    MessagesClearAllDisplayed,
    RecordDataForGameInit,
    RestartInitialise,
    WorldInitialise,
    ParticleInitialise,
    AnimManagerInitialise,
    CutsceneMgrInitialise,
    CarCtrlInit,
    ModelInfoInitialise,
    PickupsInit,
    CarGeneratorsInit,
    FileLoaderLoadDefaultDat,
    FileLoaderLoadGtaDat,
    VehicleColoursLoad,
    WaterInitialise,
    DrawInitialise,
    StreamingInit,
    StreamingLoadInitialVehicles,
    StreamingLoadInitialPeds,
    StreamingRequestBigBuildings,
    StreamingLoadAllRequestedModels,
    AnimManagerLoadAnimFiles,
    PedInitialise,
    RouteNodeInitialise,
    EventListInitialise,
    RendererInit,
    RadarInitialise,
    WeaponInitialiseWeapons,
    TrafficLightsScan,
    RoadBlocksInit,
    PopulationInitialise,
    CoronasInit,
    ShadowsInit,
    WeaponEffectsInit,
    SkidmarksInit,
    AntennasInit,
    GlassInit,
    PhoneInfoInitialise,
    ScriptsInit,
    GangsInitialise,
    ClockInitialise,
    HeliInit,
    CranesInit,
    DarkelInit,
    StatsInit,
    PacManPickupsInit,
    RubbishInit,
    CloudsInit,
    SpecialFXInit,
    WaterCannonsInit,
    BridgeInit,
    GaragesInit,
    CreditsInit,
    TrainInit,
    PlaneInit,
    RecordDataForChaseInit,
    ReplayInit,
    ModelInfoRemoveColModelsFromOtherLevels,
}

pub const INITIALISE_SEQUENCE_PC: &[CGameInitialiseStep] = &[
    CGameInitialiseStep::FileMgrInitialise,
    CGameInitialiseStep::CdStreamInit,
    CGameInitialiseStep::ValidateVersion,
    CGameInitialiseStep::TextLoad,
    CGameInitialiseStep::AudioInitialise,
    CGameInitialiseStep::TimerInitialise,
    CGameInitialiseStep::TempColModelsInitialise,
    CGameInitialiseStep::HandlingManagerInitialise,
    CGameInitialiseStep::SurfaceTableInitialise,
    CGameInitialiseStep::PedStatsInitialise,
    CGameInitialiseStep::TimeCycleInitialise,
    CGameInitialiseStep::PoolsInitialise,
    CGameInitialiseStep::GameLogicInitAtStartOfGame,
    CGameInitialiseStep::ReferencesInit,
    CGameInitialiseStep::CameraInit,
    CGameInitialiseStep::PathFindInit,
    CGameInitialiseStep::WeatherInit,
    CGameInitialiseStep::CullZonesInit,
    CGameInitialiseStep::CollisionInit,
    CGameInitialiseStep::ZonesInit,
    CGameInitialiseStep::UserDisplayInit,
    CGameInitialiseStep::MessagesInit,
    CGameInitialiseStep::MessagesClearAllDisplayed,
    CGameInitialiseStep::RecordDataForGameInit,
    CGameInitialiseStep::RestartInitialise,
    CGameInitialiseStep::WorldInitialise,
    CGameInitialiseStep::ParticleInitialise,
    CGameInitialiseStep::AnimManagerInitialise,
    CGameInitialiseStep::CutsceneMgrInitialise,
    CGameInitialiseStep::CarCtrlInit,
    CGameInitialiseStep::ModelInfoInitialise,
    CGameInitialiseStep::PickupsInit,
    CGameInitialiseStep::CarGeneratorsInit,
    CGameInitialiseStep::FileLoaderLoadDefaultDat,
    CGameInitialiseStep::FileLoaderLoadGtaDat,
    CGameInitialiseStep::VehicleColoursLoad,
    CGameInitialiseStep::WaterInitialise,
    CGameInitialiseStep::DrawInitialise,
    CGameInitialiseStep::StreamingInit,
    CGameInitialiseStep::StreamingLoadInitialVehicles,
    CGameInitialiseStep::StreamingLoadInitialPeds,
    CGameInitialiseStep::StreamingRequestBigBuildings,
    CGameInitialiseStep::StreamingLoadAllRequestedModels,
    CGameInitialiseStep::AnimManagerLoadAnimFiles,
    CGameInitialiseStep::PedInitialise,
    CGameInitialiseStep::RouteNodeInitialise,
    CGameInitialiseStep::EventListInitialise,
    CGameInitialiseStep::RendererInit,
    CGameInitialiseStep::RadarInitialise,
    CGameInitialiseStep::WeaponInitialiseWeapons,
    CGameInitialiseStep::TrafficLightsScan,
    CGameInitialiseStep::RoadBlocksInit,
    CGameInitialiseStep::PopulationInitialise,
    CGameInitialiseStep::CoronasInit,
    CGameInitialiseStep::ShadowsInit,
    CGameInitialiseStep::WeaponEffectsInit,
    CGameInitialiseStep::SkidmarksInit,
    CGameInitialiseStep::AntennasInit,
    CGameInitialiseStep::GlassInit,
    CGameInitialiseStep::PhoneInfoInitialise,
    CGameInitialiseStep::ScriptsInit,
    CGameInitialiseStep::GangsInitialise,
    CGameInitialiseStep::ClockInitialise,
    CGameInitialiseStep::HeliInit,
    CGameInitialiseStep::CranesInit,
    CGameInitialiseStep::DarkelInit,
    CGameInitialiseStep::StatsInit,
    CGameInitialiseStep::PacManPickupsInit,
    CGameInitialiseStep::RubbishInit,
    CGameInitialiseStep::CloudsInit,
    CGameInitialiseStep::SpecialFXInit,
    CGameInitialiseStep::WaterCannonsInit,
    CGameInitialiseStep::BridgeInit,
    CGameInitialiseStep::GaragesInit,
    CGameInitialiseStep::CreditsInit,
    CGameInitialiseStep::TrainInit,
    CGameInitialiseStep::PlaneInit,
    CGameInitialiseStep::RecordDataForChaseInit,
    CGameInitialiseStep::ReplayInit,
    CGameInitialiseStep::ModelInfoRemoveColModelsFromOtherLevels,
];
