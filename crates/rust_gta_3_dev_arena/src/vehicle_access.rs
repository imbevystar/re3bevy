//! Arena queries borrow the production vehicle owners; construction consumes them.
use bevy::{ecs::query::QueryData, prelude::*};

use rust_gta_3_game::vehicles::{
    Automobile::{
        AutomobileAudioMotion, AutomobileMut, AutomobileRef, AutomobileRuntimeView,
        AutomobileState, CAutomobile, VehicleCollisionState,
    },
    HandlingMgr::tHandlingData,
    Vehicle::{
        VehicleAppearance, VehicleAudioState, VehicleCore, VehicleDriveState, VehicleEntryState,
        VehicleRuntime, VehicleVitality,
    },
};

#[derive(Component, Debug, Clone)]
pub(crate) struct ArenaVehicleSettings {
    pub handling: tHandlingData,
    pub wheel_scale: f32,
    pub wheel_friction: f32,
    pub surface_adhesion: f32,
}

/// Only exists while constructing an actor, never as a live Component.
pub(crate) struct ArenaAutomobileSpawn {
    pub car: CAutomobile,
    pub settings: ArenaVehicleSettings,
}

impl ArenaAutomobileSpawn {
    pub fn into_bundle(self, history: VehicleCollisionState) -> impl Bundle {
        let (core, vitality, drive, appearance, entry, audio, state, motion) =
            self.car.into_parts();
        (
            core,
            vitality,
            drive,
            appearance,
            entry,
            audio,
            state,
            motion,
            self.settings,
            history,
        )
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct ArenaAutomobileRuntime {
    core: &'static mut VehicleCore,
    vitality: &'static mut VehicleVitality,
    drive: &'static mut VehicleDriveState,
    appearance: &'static mut VehicleAppearance,
    entry: &'static mut VehicleEntryState,
    audio: &'static mut VehicleAudioState,
    state: &'static mut AutomobileState,
    motion: &'static mut AutomobileAudioMotion,
    settings: &'static mut ArenaVehicleSettings,
}

pub(crate) struct ArenaAutomobileView<Car, Handling> {
    pub car: Car,
    pub handling: Handling,
    pub wheel_scale: f32,
    pub wheel_friction: f32,
    pub surface_adhesion: f32,
}

pub(crate) type ArenaAutomobileMut<'a> =
    ArenaAutomobileView<AutomobileMut<'a>, &'a mut tHandlingData>;
pub(crate) type ArenaAutomobileRef<'a> =
    ArenaAutomobileView<AutomobileRef<'a>, &'a tHandlingData>;

impl<'w, 's> ArenaAutomobileRuntimeItem<'w, 's> {
    pub fn into_view(self) -> ArenaAutomobileMut<'w> {
        let settings = self.settings.into_inner();
        ArenaAutomobileView {
            car: AutomobileRuntimeView {
                vehicle: VehicleRuntime {
                    core: self.core.into_inner(),
                    vitality: self.vitality.into_inner(),
                    drive: self.drive.into_inner(),
                    appearance: self.appearance.into_inner(),
                    entry: self.entry.into_inner(),
                    audio: self.audio.into_inner(),
                },
                state: self.state.into_inner(),
                audio_motion: self.motion.into_inner(),
            },
            handling: &mut settings.handling,
            wheel_scale: settings.wheel_scale,
            wheel_friction: settings.wheel_friction,
            surface_adhesion: settings.surface_adhesion,
        }
    }
}

impl<'w, 's> ArenaAutomobileRuntimeReadOnlyItem<'w, 's> {
    pub fn into_view(self) -> ArenaAutomobileRef<'w> {
        let settings = self.settings;
        ArenaAutomobileView {
            car: AutomobileRuntimeView {
                vehicle: VehicleRuntime {
                    core: self.core,
                    vitality: self.vitality,
                    drive: self.drive,
                    appearance: self.appearance,
                    entry: self.entry,
                    audio: self.audio,
                },
                state: self.state,
                audio_motion: self.motion,
            },
            handling: &settings.handling,
            wheel_scale: settings.wheel_scale,
            wheel_friction: settings.wheel_friction,
            surface_adhesion: settings.surface_adhesion,
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct ArenaAutomobileAudio {
    pub core: &'static VehicleCore,
    vitality: &'static VehicleVitality,
    drive: &'static VehicleDriveState,
    appearance: &'static VehicleAppearance,
    entry: &'static VehicleEntryState,
    audio: &'static mut VehicleAudioState,
    state: &'static AutomobileState,
    motion: &'static mut AutomobileAudioMotion,
    settings: &'static ArenaVehicleSettings,
}

/// PreRender writes wheel skid history; physical, control and audio inputs are readonly.
#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct ArenaAutomobileEffects {
    pub core: &'static VehicleCore,
    pub drive: &'static VehicleDriveState,
    pub state: &'static mut AutomobileState,
    pub settings: &'static ArenaVehicleSettings,
}

impl<'w, 's> ArenaAutomobileAudioItem<'w, 's> {
    pub fn into_car(
        self,
    ) -> AutomobileRuntimeView<
        &'w VehicleCore,
        &'w VehicleVitality,
        &'w AutomobileState,
        &'w VehicleDriveState,
        &'w VehicleAppearance,
        &'w VehicleEntryState,
        &'w mut VehicleAudioState,
        &'w mut AutomobileAudioMotion,
    > {
        AutomobileRuntimeView {
            vehicle: VehicleRuntime {
                core: self.core,
                vitality: self.vitality,
                drive: self.drive,
                appearance: self.appearance,
                entry: self.entry,
                audio: self.audio.into_inner(),
            },
            state: self.state,
            audio_motion: self.motion.into_inner(),
        }
    }
    pub fn handling(&self) -> &'w tHandlingData {
        &self.settings.handling
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_gta_3_core::core::General::CGeneral;
    use bevy::ecs::system::SystemState;

    #[test]
    fn audio_motion_writes_coexist_with_concrete_readers_and_observe_live_drive() {
        let mut world = World::new();
        let mut general = CGeneral::default();
        let car = CAutomobile::new(
            rust_gta_3_game::modelinfo::ModelIndices::MI_KURUMA as i32,
            0,
            0,
            &mut general,
            None,
            None,
            false,
        );
        let history = VehicleCollisionState {
            previous_matrix: car.vehicle.core.physical.entity.matrix,
        };
        let entity = world
            .spawn(
                ArenaAutomobileSpawn {
                    car,
                    settings: ArenaVehicleSettings {
                        handling: Default::default(),
                        wheel_scale: 1.0,
                        wheel_friction: 1.0,
                        surface_adhesion: 1.0,
                    },
                }
                .into_bundle(history),
            )
            .id();
        world
            .get_mut::<VehicleDriveState>(entity)
            .unwrap()
            .gas_pedal = 0.7;

        // This join would fail Bevy's access validation if audio acquired
        // mutable concrete or drive state merely to update its own history.
        let mut state = SystemState::<(
            Query<(&AutomobileState, &VehicleDriveState)>,
            Query<ArenaAutomobileAudio>,
        )>::new(&mut world);
        {
            let (readers, mut audio) = state.get_mut(&mut world).unwrap();
            let (concrete, drive) = readers.get(entity).unwrap();
            let car = audio.get_mut(entity).unwrap().into_car();
            assert_eq!(car.vehicle.drive.gas_pedal, drive.gas_pedal);
            assert_eq!(car.vehicle.drive.gas_pedal, 0.7);
            assert_eq!(car.state.wheels_on_ground, concrete.wheels_on_ground);
            car.audio_motion.velocity_change_for_audio = 0.2;
        }
        assert_eq!(
            world
                .get::<AutomobileAudioMotion>(entity)
                .unwrap()
                .velocity_change_for_audio,
            0.2
        );
        // PreRender must coexist with readonly physical/control/audio readers.
        let mut effects_state = SystemState::<(
            Query<(Ref<VehicleCore>, Ref<VehicleDriveState>, Ref<VehicleAudioState>, Ref<AutomobileAudioMotion>)>,
            Query<ArenaAutomobileEffects>,
        )>::new(&mut world);
        {
            let (readers, mut effects) = effects_state.get_mut(&mut world).unwrap();
            let (core, drive, audio, motion) = readers.get(entity).unwrap();
            let input_ticks = (core.last_changed(), drive.last_changed(), audio.last_changed(), motion.last_changed());
            let mut car = effects.get_mut(entity).unwrap();
            assert_eq!(car.drive.gas_pedal, drive.gas_pedal);
            car.state.wheel_skidmark_bloody[0] = true;
            assert_eq!(input_ticks, (core.last_changed(), drive.last_changed(), audio.last_changed(), motion.last_changed()),
                "wheel effect history must not mark physical/control/audio inputs changed");
        }
        assert!(world.get::<AutomobileState>(entity).unwrap().wheel_skidmark_bloody[0]);
    }
}
