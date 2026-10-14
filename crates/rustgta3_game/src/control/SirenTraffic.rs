//! Native siren response over canonical actors and incremental ECS sectors.
//! TODO(parity): CCarAI::MakeWayForCarWithSiren bike candidates require the
//! native bike actor/control domain; this automobile boundary does not invent it.

use crate::{
    control::{
        CarAI::{CCarAI, SirenVehicleSnapshot},
        ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity, ScriptVehicleKind},
    },
    core::Pools::CPoolHandle,
    entities::Entity::eEntityStatus,
    vehicles::{
        Automobile::AutomobileState,
        Vehicle::{RANDOM_VEHICLE, VehicleCore},
    },
};
use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, Timer::CTimer};


/// Siren-domain metrics and point membership over the canonical world grid.
#[derive(Default, Resource)]
pub struct AutomobileSpatialIndex {
    grid: crate::core::EntitySectorIndex::EntitySectorIndex,
    pub invalid_positions: u64,
    pub invalid_sources: u64,
    pub candidate_visits: u64,
    pub applied_actions: u64,
}
impl AutomobileSpatialIndex {
    pub fn remove(&mut self, entity: Entity) { self.grid.remove(entity); }
    pub fn update(&mut self, entity: Entity, position: CVector) {
        if !self.grid.update(entity, position) {
            self.invalid_positions += 1;
            bevy::log::warn_once!(target: "rustgta3::siren_traffic", "invalid automobile position cannot enter spatial membership");
        }
    }
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ { self.grid.entities() }
    pub fn candidates(&self, position: CVector, radius: f32, output: &mut Vec<Entity>) {
        if !self.grid.candidates(position, radius, output) {
            bevy::log::warn_once!(target: "rustgta3::siren_traffic", "invalid siren sector query");
        }
    }
}
/// Consume lifecycle removal messages during pause/restart/loading too.
/// Only derived membership changes; gameplay response keeps its native guard.
pub(crate) fn sync_automobile_spatial_index(
    mut spatial: ResMut<AutomobileSpatialIndex>,
    changed: Query<
        (Entity, &VehicleCore),
        (
            With<ScriptVehicleIdentity>,
            Or<(
                Changed<VehicleCore>,
                Changed<ScriptVehicleIdentity>,
                Added<AutomobileState>,
            )>,
        ),
    >,
    mut removed_identity: RemovedComponents<ScriptVehicleIdentity>,
    mut removed_core: RemovedComponents<VehicleCore>,
    mut removed_kind: RemovedComponents<AutomobileState>,
) {
    for entity in removed_identity
        .read()
        .chain(removed_core.read())
        .chain(removed_kind.read())
    {
        spatial.remove(entity);
    }
    for (entity, core) in &changed {
        spatial.update(entity, core.physical.entity.get_position());
    }
}

#[derive(Default)]
pub(crate) struct SirenTrafficScratch {
    sources: Vec<SirenSource>,
    candidates: Vec<Entity>,
}

#[derive(Clone, Copy)]
struct SirenSource {
    pool_slot: usize,
    entity: Entity,
}

/// CAutomobile::ProcessControl -> CCarAI::MakeWayForCarWithSiren, frame 5/8.
pub(crate) fn process_siren_traffic_response(
    index: Res<ScriptEntityIndex>,
    mut cars: Query<(Entity, &ScriptVehicleIdentity, &mut VehicleCore, &crate::vehicles::Vehicle::VehicleAudioState), With<AutomobileState>>,
    timer: Res<CTimer>,
    mut spatial: ResMut<AutomobileSpatialIndex>,
    mut scratch: Local<SirenTrafficScratch>,
) {
    if timer.frame_counter() & 7 != 5 {
        return;
    }
    scratch.sources.clear();
    for (entity, identity, core, audio) in cars.iter() {
        if identity.kind != ScriptVehicleKind::Automobile
            || index.vehicle(identity.handle) != Some(entity)
        {
            continue;
        }
        let model = core.physical.entity.model_index.max(0) as usize;
        if audio.siren_or_alarm
            && crate::vehicles::Vehicle::vehicle_uses_siren(model)
            && model != crate::modelinfo::ModelIndices::MI_MRWHOOP
        {
            scratch.sources.push(SirenSource {
                pool_slot: CPoolHandle(identity.handle).index(),
                entity,
            });
        }
    }
    scratch.sources.sort_by_key(|source| source.pool_slot);
    let SirenTrafficScratch {
        sources,
        candidates,
    } = &mut *scratch;
    for source_identity in sources.iter() {
        let Ok((_, _, source, _)) = cars.get(source_identity.entity) else {
            continue;
        };
        let position = source.physical.entity.get_position();
        let speed = source.physical.move_speed;
        let flat_speed = (speed.x * speed.x + speed.y * speed.y).sqrt();
        if !flat_speed.is_finite()
            || !position.x.is_finite()
            || !position.y.is_finite()
            || !position.z.is_finite()
        {
            spatial.invalid_sources += 1;
            bevy::log::warn_once!(target: "rustgta3::siren_traffic", "invalid siren source pose/speed");
            continue;
        }
        if flat_speed < 0.1 {
            continue;
        }
        spatial.candidates(position, flat_speed * 45.0 + 20.0, candidates);
        candidates.retain(|entity| {
            cars.get(*entity).is_ok_and(|(_, identity, _, _)| {
                identity.kind == ScriptVehicleKind::Automobile
                    && index.vehicle(identity.handle) == Some(*entity)
            })
        });
        candidates.sort_by_key(|entity| {
            std::cmp::Reverse(
                CPoolHandle(cars.get(*entity).expect("candidate validated").1.handle).index(),
            )
        });
        for &entity in candidates.iter() {
            let Ok((_, identity, mut core, _)) = cars.get_mut(entity) else {
                continue;
            };
            spatial.candidate_visits += 1;
            let input = SirenVehicleSnapshot {
                slot: CPoolHandle(identity.handle).index(),
                position: core.physical.entity.get_position(),
                move_speed: core.physical.move_speed,
                forward: core.physical.entity.get_forward(),
                is_car_or_bike: true,
                simple_or_physics: matches!(
                    core.physical.entity.status,
                    eEntityStatus::STATUS_SIMPLE | eEntityStatus::STATUS_PHYSICS
                ),
                random_vehicle: core.vehicle_created_by == RANDOM_VEHICLE,
                emergency_vehicle: core.flags.is_law_enforcer
                    || core.flags.is_ambulance_on_duty
                    || core.flags.is_fire_truck_on_duty,
                temp_action: core.auto_pilot.temp_action,
                time_temp_action: core.auto_pilot.time_temp_action,
            };
            if let Some(intent) = CCarAI::make_way_for_siren_vehicle(
                position,
                speed,
                source_identity.pool_slot,
                input,
                timer.time_in_milliseconds(),
            ) {
                core.auto_pilot.temp_action = intent.temp_action;
                core.auto_pilot.time_temp_action = intent.time_temp_action;
                if intent.set_status_physics {
                    core.physical
                        .entity
                        .set_status(eEntityStatus::STATUS_PHYSICS);
                }
                spatial.applied_actions += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::AutoPilot::eCarTempAction,
        modelinfo::ModelIndices::MI_POLICE,
        vehicles::Automobile::{CAutomobile, VehicleCollisionState},
        vehicles::Vehicle::VehicleVitality,
    };
    use bevy::ecs::system::SystemState;
    use rustgta3_core::core::General::CGeneral;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<AutomobileSpatialIndex>()
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<CTimer>()
            .add_systems(
                Update,
                (
                    sync_automobile_spatial_index,
                    process_siren_traffic_response,
                )
                    .chain(),
            );
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(5);
        app
    }

    #[test]
    fn gameplay_and_audio_siren_predicates_keep_their_native_namespaces() {
        use crate::{
            audio::AudioLogic,
            modelinfo::ModelIndices::{MI_MRWHOOP, MI_POLICE},
            vehicles::Vehicle::vehicle_uses_siren,
        };
        assert!(vehicle_uses_siren(MI_POLICE));
        assert!(!AudioLogic::uses_siren(MI_POLICE));
        assert!(AudioLogic::uses_siren(AudioLogic::POLICE));
        assert!(!vehicle_uses_siren(AudioLogic::POLICE));
        assert!(vehicle_uses_siren(MI_MRWHOOP));
        assert!(!AudioLogic::uses_siren(AudioLogic::MRWHOOP));
    }

    #[test]
    fn lifecycle_cleanup_survives_frames_without_a_gameplay_response() {
        let mut app = app();
        let target = spawn(&mut app, 0x201, CVector::new(10.0, 1.0, 0.0), false);
        app.update();
        assert!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
        // No native response on these frames. Membership maintenance must
        // consume despawn messages even across more than their retention window.
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(6);
        app.world_mut().despawn(target);
        for _ in 0..5 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
        let replacement = spawn(&mut app, 0x202, CVector::new(1000.0, 1.0, 0.0), false);
        app.update();
        assert!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&replacement)
        );
        assert_eq!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .applied_actions,
            0
        );
    }

    #[test]
    fn production_registration_consumes_removals_during_loading() {
        use rustgta3_core::core::GameState::GtaAppState;
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .insert_state(GtaAppState::LoadingGame)
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<CTimer>()
            .add_plugins(crate::vehicles::VehicleSimulationPlugin::GtaVehicleSimulationPlugin);
        let target = spawn(&mut app, 0x201, CVector::new(10.0, 1.0, 0.0), false);
        app.update();
        assert!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
        app.world_mut().despawn(target);
        for _ in 0..5 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
        assert_eq!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .applied_actions,
            0
        );
    }

    fn spawn(app: &mut App, handle: i32, position: CVector, siren: bool) -> Entity {
        let mut general = CGeneral::with_seed(handle as u32);
        let (mut core, vitality, drive, appearance, entry, mut audio, state, audio_motion) = CAutomobile::new(
            MI_POLICE as i32,
            RANDOM_VEHICLE,
            0,
            &mut general,
            None,
            None,
            false,
        )
        .into_parts();
        core.physical.entity.set_position(position);
        core.physical.entity.matrix.forward = CVector::new(1.0, 0.0, 0.0);
        core.physical.entity.matrix.right = CVector::new(0.0, -1.0, 0.0);
        core.physical.move_speed = CVector::new(0.5, 0.0, 0.0);
        core.physical.entity.status = eEntityStatus::STATUS_SIMPLE;
        audio.siren_or_alarm = siren;
        core.flags.is_law_enforcer = siren;
        core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
        let entity = app
            .world_mut()
            .spawn((
                ScriptVehicleIdentity {
                    handle,
                    kind: ScriptVehicleKind::Automobile,
                },
                core,
                vitality,
                drive,
                appearance,
                entry,
                audio,
                state,
                audio_motion,
            ))
            .id();
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .insert(handle, entity);
        entity
    }

    #[test]
    fn later_siren_reads_prior_same_pass_swerve_and_preserves_timestamp() {
        let mut app = app();
        spawn(&mut app, 0x101, CVector::ZERO, true);
        spawn(&mut app, 0x201, CVector::new(0.0, 2.0, 0.0), true);
        let target = spawn(&mut app, 0x301, CVector::new(10.0, 1.0, 0.0), false);
        // First source wants RIGHT; the second would want LEFT if it saw the
        // original NONE. Native code retains the first swerve and its deadline.
        app.update();
        let core = app.world().get::<VehicleCore>(target).unwrap();
        assert_eq!(
            core.auto_pilot.temp_action,
            eCarTempAction::TEMPACT_SWERVERIGHT
        );
        assert_eq!(core.auto_pilot.time_temp_action, 2000);
        assert_eq!(core.physical.entity.status, eEntityStatus::STATUS_PHYSICS);
        app.world_mut()
            .get_mut::<VehicleCore>(target)
            .unwrap()
            .auto_pilot
            .time_temp_action = 777;
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(13);
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleCore>(target)
                .unwrap()
                .auto_pilot
                .time_temp_action,
            777
        );
    }

    #[test]
    fn moved_removed_and_retired_entities_do_not_leave_live_spatial_candidates() {
        let mut app = app();
        spawn(&mut app, 0x101, CVector::ZERO, true);
        let target = spawn(&mut app, 0x201, CVector::new(1000.0, 1.0, 0.0), false);
        for slot in 3..67 {
            spawn(
                &mut app,
                slot << 8 | 1,
                CVector::new(1200.0, 1000.0, 0.0),
                false,
            );
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleCore>(target)
                .unwrap()
                .auto_pilot
                .temp_action,
            eCarTempAction::TEMPACT_NONE
        );
        assert!(
            app.world()
                .resource::<AutomobileSpatialIndex>()
                .candidate_visits
                < 5
        );
        app.world_mut()
            .get_mut::<VehicleCore>(target)
            .unwrap()
            .physical
            .entity
            .set_position(CVector::new(10.0, 1.0, 0.0));
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(13);
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleCore>(target)
                .unwrap()
                .auto_pilot
                .temp_action,
            eCarTempAction::TEMPACT_SWERVERIGHT
        );
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .remove(&0x201);
        app.world_mut()
            .get_mut::<VehicleCore>(target)
            .unwrap()
            .auto_pilot
            .temp_action = eCarTempAction::TEMPACT_NONE;
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(21);
        app.update();
        assert_eq!(
            app.world()
                .get::<VehicleCore>(target)
                .unwrap()
                .auto_pilot
                .temp_action,
            eCarTempAction::TEMPACT_NONE
        );
        app.world_mut()
            .entity_mut(target)
            .remove::<AutomobileState>();
        app.update();
        assert!(
            !app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
        app.world_mut().despawn(target);
        app.update();
        assert!(
            !app.world()
                .resource::<AutomobileSpatialIndex>()
                .grid
                .contains(&target)
        );
    }

    #[test]
    fn response_query_coexists_with_mutable_vitality_and_collision_history() {
        let mut world = World::new();
        let _state = SystemState::<(
            Query<(Entity, &ScriptVehicleIdentity, &mut VehicleCore, &crate::vehicles::Vehicle::VehicleAudioState), With<AutomobileState>>,
            Query<&mut VehicleVitality>,
            Query<&mut VehicleCollisionState>,
            Query<&mut crate::peds::Ped::CPed>,
        )>::new(&mut world);
    }
}
