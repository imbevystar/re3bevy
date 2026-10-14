//! Shared CVehicle audio/control signalling, separate from physical/AI state.
use crate::{
    collision::{ColModel::LEVEL_GENERIC, Collision::CCollision},
    control::ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity, ScriptVehicleKind},
    entities::Entity::eEntityStatus,
    vehicles::{Automobile::AutomobileState, Vehicle::VehicleCore},
};
use bevy::prelude::*;
use rust_gta_3_core::core::Timer::CTimer;
#[derive(Debug, Clone, PartialEq, Default, Component)]
pub struct VehicleAudioState {
    pub alarm_state: i16,
    pub radio_station: u8,
    pub rain_audio_counter: u8,
    pub rain_samples_counter: u8,
    pub car_horn_timer: u8,
    pub car_horn_pattern: u8,
    pub siren_or_alarm: bool,
}

impl VehicleAudioState {
    /// Baseline CVehicle::IsAlarmOn (Vehicle.h); -1 is armed, not sounding.
    pub fn is_alarm_on(&self) -> bool {
        self.alarm_state != 0 && self.alarm_state != -1
    }

    /// CAutomobile::ReduceHornCounter.
    pub fn reduce_horn_counter(&mut self) {
        self.car_horn_timer = self.car_horn_timer.saturating_sub(1);
    }

    /// CVehicle::ProcessCarAlarm: signed storage, unsigned remaining time.
    pub fn process_car_alarm(&mut self, step_ms: u32) {
        if self.alarm_state == 0 || self.alarm_state == -1 {
            return;
        }
        let remaining = u32::from(self.alarm_state as u16);
        self.alarm_state = if remaining < step_ms {
            0
        } else {
            (remaining - step_ms) as i16
        };
    }
}

#[derive(Default, Resource)]
pub struct VehicleAlarmDiagnostics {
    pub countdown_updates: u64,
    pub expired_alarms: u64,
    pub waiting_collision_model: u64,
}

/// CAutomobile::ProcessControl calls the shared alarm before status dispatch,
/// after the original collision island gate. Boats call it in their control.
pub(crate) fn process_automobile_audio_control(
    mut cars: Query<
        (
            Entity,
            &ScriptVehicleIdentity,
            &VehicleCore,
            &mut VehicleAudioState,
        ),
        With<AutomobileState>,
    >,
    index: Res<ScriptEntityIndex>,
    timer: Res<CTimer>,
    collision: Res<CCollision>,
    loader: Res<crate::core::FileLoader::CFileLoader>,
    mut diagnostics: ResMut<VehicleAlarmDiagnostics>,
) {
    for (entity, identity, core, mut audio) in &mut cars {
        if index.vehicle(identity.handle) != Some(entity)
            || identity.kind != ScriptVehicleKind::Automobile
        {
            continue;
        }
        let model = core.physical.entity.model_index.max(0) as usize;
        let Some(col) = loader
            .model_info
            .get_model_info(model)
            .and_then(|model| model.base().col_model.as_ref())
        else {
            diagnostics.waiting_collision_model += 1;
            bevy::log::warn_once!(model, "vehicle_audio_control_waiting_collision_model");
            continue;
        };
        if col.level > LEVEL_GENERIC && col.level != collision.collision_in_memory as i32 {
            continue;
        }
        if matches!(
            core.physical.entity.status,
            eEntityStatus::STATUS_ABANDONED
                | eEntityStatus::STATUS_WRECKED
                | eEntityStatus::STATUS_PLAYER_DISABLED
        ) && audio.car_horn_timer != 0
        {
            audio.car_horn_timer = 0;
        }
        if audio.alarm_state == 0 || audio.alarm_state == -1 {
            continue;
        }
        audio.process_car_alarm(timer.time_step_in_milliseconds());
        diagnostics.countdown_updates += 1;
        if audio.alarm_state == 0 {
            diagnostics.expired_alarms += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entities::Physical::CPhysical, vehicles::Vehicle::CVehicle};
    use rust_gta_3_core::core::General::CGeneral;

    #[test]
    fn control_uses_model_col_level_and_rejects_retired_indexed_actors() {
        use crate::{
            collision::ColModel::{CColBox, CColModel, CColSphere, CColVector},
            control::ScriptEntities::{ScriptVehicleEntity, spawn_script_test_vehicle},
            core::FileLoader::CFileLoader,
            vehicles::{Automobile::CAutomobile, Vehicle::MISSION_VEHICLE},
        };
        let mut app = App::new();
        app.init_resource::<ScriptEntityIndex>()
            .init_resource::<VehicleAlarmDiagnostics>()
            .init_resource::<CTimer>()
            .init_resource::<CCollision>();
        let mut loader = CFileLoader::new();
        let zero = CColVector::new(0.0, 0.0, 0.0);
        loader
            .model_info
            .add_vehicle_model(90)
            .clump
            .base
            .set_col_model(
                CColModel::new(
                    CColSphere {
                        center: zero,
                        radius: 0.0,
                        surface: 0,
                        piece: 0,
                    },
                    CColBox::new(zero, zero, 0, 0),
                    Vec::new(),
                ),
                true,
            );
        app.insert_resource(loader);
        let mut general = CGeneral::with_seed(9);
        let mut car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        car.vehicle.core.physical.zone_level = 99;
        car.vehicle.core.physical.entity.status = eEntityStatus::STATUS_ABANDONED;
        car.vehicle.audio.alarm_state = 1000;
        car.vehicle.audio.car_horn_timer = 42;
        let handle = 0x501;
        let entity = spawn_script_test_vehicle(
            app.world_mut(),
            ScriptVehicleEntity::Automobile { handle, car },
        );
        app.add_systems(Update, process_automobile_audio_control
            .run_if(crate::core::World::world_actor_processing_enabled));
        let elapsed = app.world().resource::<CTimer>().time_step_in_milliseconds() as i16;
        app.world_mut().resource_mut::<CFileLoader>().world.process_cutscene_only = true;
        for _ in 0..8 {
            app.update();
        }
        let audio = app.world().get::<VehicleAudioState>(entity).unwrap();
        assert_eq!(audio.alarm_state, 1000, "normal actor control is frozen during cutscene-only dispatch");
        assert_eq!(audio.car_horn_timer, 42);
        assert_eq!(app.world().resource::<VehicleAlarmDiagnostics>().countdown_updates, 0);
        app.world_mut().resource_mut::<CFileLoader>().world.process_cutscene_only = false;
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleAudioState>(entity)
                .unwrap()
                .alarm_state,
            1000 - elapsed
        );
        assert_eq!(
            app.world()
                .get::<VehicleAudioState>(entity)
                .unwrap()
                .car_horn_timer,
            0
        );
        // The model's actual COL level, rather than the actor's zone, gates control.
        app.world_mut()
            .resource_mut::<CFileLoader>()
            .model_info
            .get_model_info_mut(90)
            .unwrap()
            .base_mut()
            .col_model
            .as_mut()
            .unwrap()
            .level = 99;
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleAudioState>(entity)
                .unwrap()
                .alarm_state,
            1000 - elapsed
        );
        app.world_mut()
            .resource_mut::<CFileLoader>()
            .model_info
            .get_model_info_mut(90)
            .unwrap()
            .base_mut()
            .col_model
            .as_mut()
            .unwrap()
            .level = LEVEL_GENERIC;
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .remove(&handle);
        app.update();
        assert_eq!(
            app.world()
                .resource::<VehicleAlarmDiagnostics>()
                .countdown_updates,
            1
        );
    }

    #[test]
    fn horn_counter_expires_without_underflow() {
        let mut audio = VehicleAudioState {
            car_horn_timer: 45,
            ..Default::default()
        };
        for expected in (0..45).rev() {
            audio.reduce_horn_counter();
            assert_eq!(audio.car_horn_timer, expected);
        }
        audio.reduce_horn_counter();
        assert_eq!(audio.car_horn_timer, 0);
    }

    #[test]
    fn alarm_countdown_matches_native_sentinels_unsigned_storage_and_expiry() {
        for (start, elapsed, expected) in [
            (0, 100, 0),
            (-1, 100, -1),
            (30, 20, 10),
            (20, 20, 0),
            (19, 20, 0),
            (-2, 20, -22),
            (-32768, 20, 32748),
            (100, 0, 100),
            (100, 70000, 0),
        ] {
            let mut audio = VehicleAudioState {
                alarm_state: start,
                ..Default::default()
            };
            assert_eq!(audio.is_alarm_on(), start != 0 && start != -1);
            audio.process_car_alarm(elapsed);
            assert_eq!(audio.alarm_state, expected);
        }
    }

    #[test]
    fn constructor_preserves_physical_then_radio_random_draw_order() {
        for seed in [1, 7, 29, 0xffff] {
            let mut expected = CGeneral::with_seed(seed);
            let _physical = CPhysical::new(&mut expected);
            let station =
                (expected.get_random_number() % crate::audio::audio_enums::USERTRACK as u16) as u8;
            let mut actual = CGeneral::with_seed(seed);
            let vehicle = CVehicle::new(0, 0, &mut actual);
            assert_eq!(vehicle.audio.radio_station, station);
            assert_eq!(actual.get_random_number(), expected.get_random_number());
            let (_, _, _, _, _, audio) = vehicle.into_parts();
            assert_eq!(
                audio,
                VehicleAudioState {
                    radio_station: station,
                    ..Default::default()
                }
            );
        }
    }
}
