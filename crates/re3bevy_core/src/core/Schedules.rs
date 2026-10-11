//! re3 reference: references/re-GTA/src/core/Game.cpp
//! re3 reference: references/re-GTA/src/core/main.cpp
//!
//! Bevy schedule labels ordered to preserve `CGame::Process` ownership.

use bevy::prelude::*;

use super::{GameState::GtaAppState, Timer::CTimer};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GtaFrameSet {
    Timer,
    GameProcess,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GtaProcessSet {
    Pad,
    Cutscene,
    Frontend,
    Streaming,
    Zones,
    FrameSetup,
    Clock,
    Weather,
    Script,
    Collision,
    Trains,
    Planes,
    Helis,
    Darkel,
    Skidmarks,
    Antennas,
    Glass,
    Events,
    Particles,
    Fires,
    Population,
    Weapons,
    CarGenerators,
    Cranes,
    Clouds,
    MovingThings,
    WaterCannons,
    UserDisplay,
    Replay,
    World,
    Accidents,
    PacManPickups,
    Pickups,
    Garages,
    Rubbish,
    SpecialFx,
    TimeCycle,
    Camera,
    CullZones,
    GameLogic,
    Bridge,
    Coronas,
    Shadows,
    Phones,
    CarCtrl,
    RoadBlocks,
    Audio,
    Render2d,
    RenderPrep,
}

pub struct GtaSchedulePlugin;

impl Plugin for GtaSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (GtaFrameSet::Timer, GtaFrameSet::GameProcess).chain(),
        )
        .configure_sets(
            Update,
            (
                GtaProcessSet::Pad,
                GtaProcessSet::Cutscene,
                GtaProcessSet::Frontend,
                GtaProcessSet::Streaming,
            )
                .chain()
                .in_set(GtaFrameSet::GameProcess)
                .run_if(gta_process_in_game),
        )
        .configure_sets(
            Update,
            (
                GtaProcessSet::Zones,
                GtaProcessSet::FrameSetup,
                GtaProcessSet::Clock,
                GtaProcessSet::Weather,
                GtaProcessSet::Script,
                GtaProcessSet::Collision,
                GtaProcessSet::Trains,
                GtaProcessSet::Planes,
                GtaProcessSet::Helis,
                GtaProcessSet::Darkel,
                GtaProcessSet::Skidmarks,
                GtaProcessSet::Antennas,
                GtaProcessSet::Glass,
            )
                .in_set(GtaFrameSet::GameProcess)
                .run_if(gta_process_in_game_and_not_paused),
        )
        .configure_sets(
            Update,
            (
                GtaProcessSet::Events,
                GtaProcessSet::Particles,
                GtaProcessSet::Fires,
                GtaProcessSet::Population,
                GtaProcessSet::Weapons,
                GtaProcessSet::CarGenerators,
                GtaProcessSet::Cranes,
                GtaProcessSet::Clouds,
                GtaProcessSet::MovingThings,
                GtaProcessSet::WaterCannons,
                GtaProcessSet::UserDisplay,
                GtaProcessSet::Replay,
                GtaProcessSet::World,
            )
                .in_set(GtaFrameSet::GameProcess)
                .run_if(gta_process_in_game_and_not_paused),
        )
        .configure_sets(
            Update,
            (
                GtaProcessSet::Accidents,
                GtaProcessSet::PacManPickups,
                GtaProcessSet::Pickups,
                GtaProcessSet::Garages,
                GtaProcessSet::Rubbish,
                GtaProcessSet::SpecialFx,
                GtaProcessSet::TimeCycle,
                GtaProcessSet::Camera,
                GtaProcessSet::CullZones,
                GtaProcessSet::GameLogic,
                GtaProcessSet::Bridge,
                GtaProcessSet::Coronas,
                GtaProcessSet::Shadows,
                GtaProcessSet::Phones,
                GtaProcessSet::CarCtrl,
                GtaProcessSet::RoadBlocks,
            )
                .chain()
                .in_set(GtaFrameSet::GameProcess)
                .run_if(gta_process_in_game_and_not_paused),
        )
        // DMAudio::Service is outside CGame::Process and still runs while the
        // gameplay timer is paused. Render preparation follows the service.
        .configure_sets(
            Update,
            (
                GtaProcessSet::Audio,
                GtaProcessSet::Render2d,
                GtaProcessSet::RenderPrep,
            )
                .chain()
                .in_set(GtaFrameSet::GameProcess)
                .run_if(gta_process_in_game),
        )
        .configure_sets(Update, GtaProcessSet::Cutscene.after(GtaProcessSet::Pad))
        .configure_sets(
            Update,
            GtaProcessSet::Frontend.after(GtaProcessSet::Cutscene),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Streaming.after(GtaProcessSet::Frontend),
        )
        .configure_sets(Update, GtaProcessSet::Zones.after(GtaProcessSet::Streaming))
        .configure_sets(
            Update,
            GtaProcessSet::FrameSetup.after(GtaProcessSet::Zones),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Clock.after(GtaProcessSet::FrameSetup),
        )
        .configure_sets(Update, GtaProcessSet::Weather.after(GtaProcessSet::Clock))
        .configure_sets(Update, GtaProcessSet::Script.after(GtaProcessSet::Weather))
        .configure_sets(
            Update,
            GtaProcessSet::Collision.after(GtaProcessSet::Script),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Trains.after(GtaProcessSet::Collision),
        )
        .configure_sets(Update, GtaProcessSet::Planes.after(GtaProcessSet::Trains))
        .configure_sets(Update, GtaProcessSet::Helis.after(GtaProcessSet::Planes))
        .configure_sets(Update, GtaProcessSet::Darkel.after(GtaProcessSet::Helis))
        .configure_sets(
            Update,
            GtaProcessSet::Skidmarks.after(GtaProcessSet::Darkel),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Antennas.after(GtaProcessSet::Skidmarks),
        )
        .configure_sets(Update, GtaProcessSet::Glass.after(GtaProcessSet::Antennas))
        .configure_sets(Update, GtaProcessSet::Events.after(GtaProcessSet::Glass))
        .configure_sets(
            Update,
            GtaProcessSet::Particles.after(GtaProcessSet::Events),
        )
        .configure_sets(Update, GtaProcessSet::Fires.after(GtaProcessSet::Particles))
        .configure_sets(
            Update,
            GtaProcessSet::Population.after(GtaProcessSet::Fires),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Weapons.after(GtaProcessSet::Population),
        )
        .configure_sets(
            Update,
            GtaProcessSet::CarGenerators.after(GtaProcessSet::Weapons),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Cranes.after(GtaProcessSet::CarGenerators),
        )
        .configure_sets(Update, GtaProcessSet::Clouds.after(GtaProcessSet::Cranes))
        .configure_sets(
            Update,
            GtaProcessSet::MovingThings.after(GtaProcessSet::Clouds),
        )
        .configure_sets(
            Update,
            GtaProcessSet::WaterCannons.after(GtaProcessSet::MovingThings),
        )
        .configure_sets(
            Update,
            GtaProcessSet::UserDisplay.after(GtaProcessSet::WaterCannons),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Replay.after(GtaProcessSet::UserDisplay),
        )
        .configure_sets(Update, GtaProcessSet::World.after(GtaProcessSet::Replay))
        .configure_sets(Update, GtaProcessSet::Accidents.after(GtaProcessSet::World))
        .configure_sets(
            Update,
            GtaProcessSet::PacManPickups.after(GtaProcessSet::Accidents),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Pickups.after(GtaProcessSet::PacManPickups),
        )
        .configure_sets(Update, GtaProcessSet::Garages.after(GtaProcessSet::Pickups))
        .configure_sets(Update, GtaProcessSet::Rubbish.after(GtaProcessSet::Garages))
        .configure_sets(
            Update,
            GtaProcessSet::SpecialFx.after(GtaProcessSet::Rubbish),
        )
        .configure_sets(
            Update,
            GtaProcessSet::TimeCycle.after(GtaProcessSet::SpecialFx),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Camera.after(GtaProcessSet::TimeCycle),
        )
        .configure_sets(
            Update,
            GtaProcessSet::CullZones.after(GtaProcessSet::Camera),
        )
        .configure_sets(
            Update,
            GtaProcessSet::GameLogic.after(GtaProcessSet::CullZones),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Bridge.after(GtaProcessSet::GameLogic),
        )
        .configure_sets(Update, GtaProcessSet::Coronas.after(GtaProcessSet::Bridge))
        .configure_sets(Update, GtaProcessSet::Shadows.after(GtaProcessSet::Coronas))
        .configure_sets(Update, GtaProcessSet::Phones.after(GtaProcessSet::Shadows))
        .configure_sets(Update, GtaProcessSet::CarCtrl.after(GtaProcessSet::Phones))
        .configure_sets(
            Update,
            GtaProcessSet::RoadBlocks.after(GtaProcessSet::CarCtrl),
        )
        .configure_sets(
            Update,
            GtaProcessSet::Audio.after(GtaProcessSet::RoadBlocks),
        )
        .configure_sets(Update, GtaProcessSet::Render2d.after(GtaProcessSet::Audio))
        .configure_sets(
            Update,
            GtaProcessSet::RenderPrep.after(GtaProcessSet::Render2d),
        );
    }
}

pub fn gta_process_in_game(state: Option<Res<State<GtaAppState>>>) -> bool {
    state.is_some_and(|state| state.get() == &GtaAppState::InGame)
}

pub fn gta_process_in_game_and_not_paused(
    state: Option<Res<State<GtaAppState>>>,
    timer: Option<Res<CTimer>>,
) -> bool {
    gta_process_in_game(state) && timer.is_some_and(|timer| !timer.get_is_paused())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Trace(Vec<u8>);

    fn record<const N: u8>(mut trace: ResMut<Trace>) {
        trace.0.push(N);
    }

    #[test]
    fn process_sets_follow_cgame_process_order() {
        let mut app = App::new();
        app.insert_resource(State::new(GtaAppState::InGame))
            .insert_resource(CTimer::default())
            .init_resource::<Trace>()
            .add_plugins(GtaSchedulePlugin)
            .add_systems(Update, record::<0>.in_set(GtaProcessSet::Pad))
            .add_systems(Update, record::<1>.in_set(GtaProcessSet::Streaming))
            .add_systems(Update, record::<2>.in_set(GtaProcessSet::Zones))
            .add_systems(Update, record::<3>.in_set(GtaProcessSet::Script))
            .add_systems(Update, record::<4>.in_set(GtaProcessSet::Darkel))
            .add_systems(Update, record::<5>.in_set(GtaProcessSet::Glass))
            .add_systems(Update, record::<6>.in_set(GtaProcessSet::Fires))
            .add_systems(Update, record::<7>.in_set(GtaProcessSet::CarGenerators))
            .add_systems(Update, record::<8>.in_set(GtaProcessSet::Replay))
            .add_systems(Update, record::<9>.in_set(GtaProcessSet::World))
            .add_systems(Update, record::<10>.in_set(GtaProcessSet::Accidents))
            .add_systems(Update, record::<11>.in_set(GtaProcessSet::Rubbish))
            .add_systems(Update, record::<12>.in_set(GtaProcessSet::TimeCycle))
            .add_systems(Update, record::<13>.in_set(GtaProcessSet::Camera))
            .add_systems(Update, record::<14>.in_set(GtaProcessSet::GameLogic))
            .add_systems(Update, record::<15>.in_set(GtaProcessSet::Bridge))
            .add_systems(Update, record::<16>.in_set(GtaProcessSet::RoadBlocks))
            .add_systems(Update, record::<17>.in_set(GtaProcessSet::Audio))
            .add_systems(Update, record::<18>.in_set(GtaProcessSet::Render2d))
            .add_systems(Update, record::<19>.in_set(GtaProcessSet::RenderPrep));

        app.update();
        assert_eq!(
            app.world().resource::<Trace>().0,
            (0..20).collect::<Vec<_>>()
        );
    }

    #[test]
    fn audio_and_render_service_continue_while_gameplay_is_paused() {
        let mut timer = CTimer::default();
        timer.set_code_pause(true);
        let mut app = App::new();
        app.insert_resource(State::new(GtaAppState::InGame))
            .insert_resource(timer)
            .init_resource::<Trace>()
            .add_plugins(GtaSchedulePlugin)
            .add_systems(Update, record::<0>.in_set(GtaProcessSet::Pad))
            .add_systems(Update, record::<1>.in_set(GtaProcessSet::Zones))
            .add_systems(Update, record::<2>.in_set(GtaProcessSet::Audio))
            .add_systems(Update, record::<3>.in_set(GtaProcessSet::RenderPrep));

        app.update();
        assert_eq!(app.world().resource::<Trace>().0, vec![0, 2, 3]);
    }
}
