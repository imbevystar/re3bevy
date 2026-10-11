//! Audio-owned actor access. Components remain canonical; no SCM mutation buffer
//! or renderer projection is acquired by sound consumers.
use crate::{
    control::ScriptEntities::ScriptEntityIndex,
    control::ScriptEntities::ScriptPedIdentity,
    peds::{
        Ped::CPed,
        PedVitality::PedVitality,
        PlayerPed::{CWorldPlayerState, MAX_PLAYERS, PlayerPedState},
        VehicleRelations::InVehicle,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(Debug, Default, Resource)]
pub struct AudioActorDiagnostics {
    pub missing_listener_frames: u64,
    pub missing_audio_manager_frames: u64,
    pub missing_handling_frames: u64,
    pub missing_weather_frames: u64,
    pub missing_rain_world_frames: u64,
    pub missing_model_handling: u64,
    pub actor_allocation_failures: u64,
    pub invalid_actor_audio_ids: u64,
    pub missing_script_positions: u64,
}

impl AudioActorDiagnostics {
    /// Camera is authoritative; FileLoader's synchronized renderer camera is
    /// the valid alternate input used by the existing game integration.
    pub(crate) fn listener_position(
        &mut self,
        camera: Option<&crate::core::Camera::CCamera>,
        loader: Option<&crate::core::FileLoader::CFileLoader>,
    ) -> Option<re3bevy_core::core::Coordinates::CVector> {
        let position = camera
            .map(|camera| camera.position)
            .or_else(|| loader.map(|loader| loader.renderer.camera_position));
        if position.is_none() {
            self.missing_listener_frames += 1;
            if self.missing_listener_frames == 1 {
                warn!("audio_actor_missing_listener");
            }
        }
        position
    }
    pub(crate) fn observe_actor_audio_id(&mut self, id: i32, capacity: usize) {
        if id < 0 {
            self.actor_allocation_failures += 1;
            if self.actor_allocation_failures == 1 {
                warn!(error_code = id, "audio_actor_allocation_failed");
            }
        } else if id as usize >= capacity {
            self.invalid_actor_audio_ids += 1;
            if self.invalid_actor_audio_ids == 1 {
                warn!(audio_id = id, "audio_actor_invalid_audio_id");
            }
        }
    }

    pub(crate) fn audio_manager_available(&mut self, present: bool) -> bool {
        if !present {
            self.missing_audio_manager_frames += 1;
            if self.missing_audio_manager_frames == 1 {
                warn!("audio_actor_missing_manager");
            }
        }
        present
    }
}

/// The focused player's audio inputs need neither player mutation nor animation.
#[derive(SystemParam)]
pub(crate) struct AudioFocusedPlayer<'w, 's> {
    state: Option<Res<'w, CWorldPlayerState>>,
    players: Query<'w, 's, (&'static CPed, Option<&'static InVehicle>), With<PlayerPedState>>,
}
impl AudioFocusedPlayer<'_, '_> {
    fn focused(&self) -> Option<(&CPed, Option<&InVehicle>)> {
        let state = self.state.as_deref()?;
        self.players
            .get(
                state
                    .entities
                    .get(state.player_in_focus)
                    .copied()
                    .flatten()?,
            )
            .ok()
    }
    pub(crate) fn surface(&self) -> Option<u8> {
        Some(self.focused()?.0.physical.surface_touched)
    }
    pub(crate) fn vehicle(&self) -> Option<Entity> {
        Some(self.focused()?.1?.vehicle)
    }
}

/// The event processor mutates only ped-owned audio identity and queued footstep
/// state. It never borrows PedAnimation, player controls, vitality or vehicles.
#[derive(SystemParam)]
pub(crate) struct AudioPedEvents<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    state: Option<Res<'w, CWorldPlayerState>>,
    players: Query<'w, 's, (&'static mut CPed, &'static PlayerPedState), With<PlayerPedState>>,
    peds: Query<
        'w,
        's,
        (Entity, &'static ScriptPedIdentity, &'static mut CPed),
        Without<PlayerPedState>,
    >,
}
impl AudioPedEvents<'_, '_> {
    pub(crate) fn for_each_player(&mut self, mut visit: impl FnMut(i32, &mut CPed)) {
        let Some(state) = self.state.as_deref() else {
            return;
        };
        for entity in state.entities.iter().take(MAX_PLAYERS).flatten() {
            if let Ok((mut ped, player)) = self.players.get_mut(*entity) {
                visit(player.pool_handle, &mut ped);
            }
        }
    }
    pub(crate) fn for_each_script_ped(&mut self, mut visit: impl FnMut(i32, &mut CPed)) {
        for (entity, identity, mut ped) in &mut self.peds {
            if self.index.ped(identity.handle) == Some(entity) {
                visit(identity.handle, &mut ped);
            }
        }
    }
}

/// Speech also reads independent vitality, while mutating canonical talking
/// timestamps and queue fields on CPed. NPC/player order belongs to its caller.
#[derive(SystemParam)]
pub(crate) struct AudioPedSpeech<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    state: Option<Res<'w, CWorldPlayerState>>,
    players: Query<
        'w,
        's,
        (
            &'static mut CPed,
            &'static PedVitality,
            &'static PlayerPedState,
        ),
        With<PlayerPedState>,
    >,
    peds: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptPedIdentity,
            &'static mut CPed,
            &'static PedVitality,
        ),
        Without<PlayerPedState>,
    >,
}
impl AudioPedSpeech<'_, '_> {
    pub(crate) fn for_each_player(&mut self, mut visit: impl FnMut(i32, &mut CPed, &PedVitality)) {
        let Some(state) = self.state.as_deref() else {
            return;
        };
        for entity in state.entities.iter().take(MAX_PLAYERS).flatten() {
            if let Ok((mut ped, vitality, player)) = self.players.get_mut(*entity) {
                visit(player.pool_handle, &mut ped, vitality);
            }
        }
    }
    pub(crate) fn for_each_script_ped(
        &mut self,
        mut visit: impl FnMut(i32, &mut CPed, &PedVitality),
    ) {
        for (entity, identity, mut ped, vitality) in &mut self.peds {
            if self.index.ped(identity.handle) == Some(entity) {
                visit(identity.handle, &mut ped, vitality);
            }
        }
    }
}

/// The reusable live vehicle query contains no SCM pending store or actor-wide
/// mutations. Sound processing retains the vehicle writes required by re3.
use crate::vehicles::{
    Automobile::{AutomobileAudioMotion, AutomobileRuntimeView, AutomobileState},
    Boat::{BoatRuntimeView, BoatState},
    Heli::{HeliRuntimeView, HeliState},
    Plane::{PlaneRuntimeView, PlaneState},
    Vehicle::{
        VehicleAppearance, VehicleCore, VehicleDriveState, VehicleRef, VehicleRuntime,
        VehicleVitality,
    },
};

/// CVehicle's physical audio identity is allocated once, before readonly sound
/// projection. CPhysical owns the ID; PhysicalAudioLifecycle validates and
/// retires its derived Entity/allocation binding.
pub(crate) fn allocate_vehicle_audio_entities(
    mut dmaudio: Option<ResMut<crate::audio::DMAudio::cDMAudio>>,
    index: Res<ScriptEntityIndex>,
    loader: Option<Res<crate::core::FileLoader::CFileLoader>>,
    handling: Option<Res<crate::vehicles::HandlingMgr::cHandlingDataMgr>>,
    weather: Option<Res<crate::renderer::Weather::CWeather>>,
    mut bindings: ResMut<super::PhysicalAudioLifecycle::PhysicalAudioBindings>,
    mut vehicles: Query<
        (
            &crate::control::ScriptEntities::ScriptVehicleIdentity,
            &mut VehicleCore,
            Has<AutomobileState>,
            Has<BoatState>,
            Has<HeliState>,
            Has<PlaneState>,
        ),
        (
            With<VehicleVitality>,
            With<VehicleDriveState>,
            With<VehicleAppearance>,
            With<crate::vehicles::Vehicle::VehicleEntryState>,
            With<crate::vehicles::Vehicle::VehicleAudioState>,
        ),
    >,
) {
    // process_vehicle_audio reports these same dependency waits. Allocation
    // follows its gates so waiting frames cannot consume audio entity slots.
    let (Some(dmaudio), Some(handling)) = (dmaudio.as_deref_mut(), handling.as_deref()) else {
        return;
    };
    if weather.is_none() || loader.is_none() {
        return;
    }
    for &handle in index.vehicles.keys() {
        let Some(entity) = index.vehicle(handle) else {
            continue;
        };
        let Ok((identity, mut core, car, boat, heli, plane)) = vehicles.get_mut(entity) else {
            continue;
        };
        if identity.handle != handle {
            continue;
        }
        use crate::control::ScriptEntities::ScriptVehicleKind;
        let ready = match identity.kind {
            ScriptVehicleKind::Automobile => {
                car && core
                    .handling_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|id| handling.get_handling_data(id))
                    .is_some()
            }
            ScriptVehicleKind::Boat => boat,
            ScriptVehicleKind::Heli => heli,
            ScriptVehicleKind::Plane => plane,
        };
        if ready {
            bindings.ensure_actor(entity, handle, &mut core.physical.audio_entity_id, dmaudio);
        }
    }
}

/// Allocate the canonical physical audio identity before vehicle sound processing.
pub fn allocate_vehicle_audio_identity(
    dmaudio: &mut crate::audio::DMAudio::cDMAudio,
    core: &mut VehicleCore,
    handle: i32,
) {
    if core.physical.audio_entity_id >= 0 {
        return;
    }
    let id = dmaudio.CreateEntity(crate::audio::audio_enums::AUDIOTYPE_PHYSICAL, Some(handle));
    core.physical.audio_entity_id = id;
    if id >= 0 {
        dmaudio.SetEntityStatus(id, true);
    }
}

pub(crate) enum AudioVehicleMut<'a> {
    Automobile {
        car: AutomobileRuntimeView<
            &'a VehicleCore,
            &'a VehicleVitality,
            &'a AutomobileState,
            &'a VehicleDriveState,
            &'a crate::vehicles::Vehicle::VehicleAppearance,
            &'a crate::vehicles::Vehicle::VehicleEntryState,
            &'a mut crate::vehicles::Vehicle::VehicleAudioState,
         &'a mut AutomobileAudioMotion>,
    },
    Boat {
        boat: BoatRuntimeView<
            &'a VehicleCore,
            &'a VehicleVitality,
            &'a BoatState,
            &'a VehicleDriveState,
            &'a crate::vehicles::Vehicle::VehicleAppearance,
            &'a crate::vehicles::Vehicle::VehicleEntryState,
            &'a mut crate::vehicles::Vehicle::VehicleAudioState,
        >,
    },
    Heli {
        heli: HeliRuntimeView<
            &'a VehicleCore,
            &'a VehicleVitality,
            &'a HeliState,
            &'a VehicleDriveState,
            &'a crate::vehicles::Vehicle::VehicleAppearance,
            &'a crate::vehicles::Vehicle::VehicleEntryState,
            &'a mut crate::vehicles::Vehicle::VehicleAudioState,
        >,
    },
    Plane {
        plane: PlaneRuntimeView<
            &'a VehicleCore,
            &'a VehicleVitality,
            &'a PlaneState,
            &'a VehicleDriveState,
            &'a crate::vehicles::Vehicle::VehicleAppearance,
            &'a crate::vehicles::Vehicle::VehicleEntryState,
            &'a mut crate::vehicles::Vehicle::VehicleAudioState,
        >,
    },
}
impl AudioVehicleMut<'_> {
    pub(crate) fn base(&self) -> VehicleRef<'_> {
        match self {
            Self::Automobile { car } => car.vehicle.as_ref(),
            Self::Boat { boat } => boat.vehicle.as_ref(),
            Self::Heli { heli } => heli.vehicle.as_ref(),
            Self::Plane { plane } => plane.vehicle.as_ref(),
        }
    }
}

/// Audio writes shared/concrete audio state; physical pose, drive and health are
/// readonly inputs. No collision-history query is acquired.
#[derive(SystemParam)]
pub(crate) struct AudioVehicleStore<'w, 's> {
    entities: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::control::ScriptEntities::ScriptVehicleIdentity,
            &'static VehicleCore,
            &'static VehicleVitality,
            &'static VehicleDriveState,
            &'static VehicleAppearance,
            &'static crate::vehicles::Vehicle::VehicleEntryState,
            &'static mut crate::vehicles::Vehicle::VehicleAudioState,
            Option<&'static AutomobileState>,
            Option<&'static mut AutomobileAudioMotion>,
            Option<&'static BoatState>,
            Option<&'static HeliState>,
            Option<&'static PlaneState>,
        ),
    >,
}
impl AudioVehicleStore<'_, '_> {
    pub(crate) fn get_entity(
        &self,
        entity: Entity,
    ) -> Option<crate::control::ScriptEntities::ScriptVehicleRef<'_>> {
        crate::control::ScriptEntities::vehicle_ref(self.entities.get(entity).ok()?)
    }
    pub(crate) fn get_mut(
        &mut self,
        index: &ScriptEntityIndex,
        handle: &i32,
    ) -> Option<AudioVehicleMut<'_>> {
        let entity = index.vehicle(*handle)?;
        let Ok((
            _,
            identity,
            core,
            vitality,
            drive,
            appearance,
            entry,
            audio,
            car,
            audio_motion,
            boat,
            heli,
            plane,
        )) = self.entities.get_mut(entity)
        else {
            bevy::log::warn_once!(target: "re3bevy::audio_vehicle_lifecycle", handle,
                "indexed audio actor is waiting for required live components");
            return None;
        };
        if identity.handle != *handle {
            return None;
        }
        let vehicle = VehicleRuntime {
            core,
            vitality,
            drive,
            appearance,
            entry,
            audio: audio.into_inner(),
        };
        use crate::control::ScriptEntities::ScriptVehicleKind;
        let result = match identity.kind {
            ScriptVehicleKind::Automobile => car.zip(audio_motion).map(|(state, audio_motion)| AudioVehicleMut::Automobile {
                car: AutomobileRuntimeView {
                    vehicle,
                    state,
                    audio_motion: audio_motion.into_inner(),
                },
            }),
            ScriptVehicleKind::Boat => boat.map(|state| AudioVehicleMut::Boat {
                boat: BoatRuntimeView {
                    vehicle,
                    state,
                },
            }),
            ScriptVehicleKind::Heli => heli.map(|state| AudioVehicleMut::Heli {
                heli: HeliRuntimeView {
                    vehicle,
                    state,
                },
            }),
            ScriptVehicleKind::Plane => plane.map(|state| AudioVehicleMut::Plane {
                plane: PlaneRuntimeView {
                    vehicle,
                    state,
                },
            }),
        };
        if result.is_none() {
            bevy::log::warn_once!(target: "re3bevy::audio_vehicle_lifecycle", handle,
                "required concrete audio vehicle state is absent");
        }
        result
    }
}

#[derive(SystemParam)]
pub(crate) struct AudioVehicleActors<'w, 's> {
    pub(crate) index: Res<'w, ScriptEntityIndex>,
    pub(crate) vehicles: AudioVehicleStore<'w, 's>,
}
impl AudioVehicleActors<'_, '_> {
    pub(crate) fn handle_for_entity(&self, entity: Entity) -> Option<i32> {
        let handle = self.vehicles.get_entity(entity)?.handle();
        (self.index.vehicle(handle) == Some(entity)).then_some(handle)
    }
}

#[cfg(test)]
mod audio_actor_integration_tests {
    use super::*;
    use crate::{
        audio::{
            AudioLogic::{process_ped_audio, process_script_audio},
            AudioSamples::{SFX_BULLET_SHELL_HIT_GROUND_1, SFX_BULLET_SHELL_HIT_GROUND_2},
            AudioScriptObject::cAudioScriptObject,
            DMAudio::cDMAudio,
            soundlist::{SCRIPT_SOUND_GUNSHELL_DROP, SOUND_STEP_START},
        },
        core::{SurfaceTable::eSurfaceType, Timer::CTimer},
        peds::{PedType::ePedType, PlayerPed::CPlayerPed, VehicleRelations::PedEntity},
    };
    use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};

    #[test]
    fn speech_uses_current_model_and_ignores_retired_script_identity() {
        use crate::audio::{PedTalk::{PedTalkFamily, PedTalkState, service_ped_talk},
            soundlist::SOUND_PED_EVADE};
        use crate::modelinfo::ModelIndices::{MI_COP, MI_MALE01};
        let mut app = App::new();
        app.init_resource::<ScriptEntityIndex>()
            .init_resource::<AudioActorDiagnostics>()
            .init_resource::<PedTalkState>()
            .init_resource::<crate::core::FileLoader::CFileLoader>()
            .init_resource::<crate::core::Camera::CCamera>()
            .init_resource::<CGeneral>()
            .init_resource::<CTimer>()
            .add_systems(Update, service_ped_talk);
        app.world_mut().resource_mut::<CTimer>().set_time_in_milliseconds(100);
        let mut audio = cDMAudio::default();
        audio.Initialise();
        app.insert_resource(audio);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(5));
        ped.physical.entity.model_index = MI_MALE01 as i16;
        ped.queued_sound = SOUND_PED_EVADE as i16;
        let handle = 0x102;
        let entity = app.world_mut().spawn((ped, PedVitality::default(),
            ScriptPedIdentity { handle, model_id: MI_COP as i32 })).id();
        app.world_mut().resource_mut::<ScriptEntityIndex>().peds.insert(handle, entity);
        app.update();
        let phrases = app.world().resource::<PedTalkState>();
        assert_ne!(phrases.last_sfx[PedTalkFamily::NormalMale as usize], u32::MAX);
        assert_eq!(phrases.last_sfx[PedTalkFamily::Cop as usize], u32::MAX);
        assert_eq!(app.world().get::<CPed>(entity).unwrap().queued_sound,
            crate::peds::Ped::SOUND_NO_SOUND);

        app.world_mut().resource_mut::<ScriptEntityIndex>().peds.remove(&handle);
        app.world_mut().get_mut::<CPed>(entity).unwrap().queued_sound = SOUND_PED_EVADE as i16;
        app.world_mut().resource_mut::<CTimer>().set_time_in_milliseconds(2000);
        app.update();
        assert_eq!(app.world().get::<CPed>(entity).unwrap().queued_sound, SOUND_PED_EVADE as i16);
    }

    #[test]
    fn audio_borrows_current_drive_and_health_without_collision_history_access() {
        use crate::control::ScriptEntities::{ScriptVehicleIdentity, ScriptVehicleKind};
        use crate::vehicles::Automobile::CAutomobile;
        use bevy::ecs::system::SystemState;
        let mut world = World::new();
        world.init_resource::<ScriptEntityIndex>();
        let mut general = CGeneral::with_seed(9);
        let (core, vitality, drive, appearance, entry, audio, car, audio_motion) = CAutomobile::new(
            90,
            crate::vehicles::Vehicle::MISSION_VEHICLE,
            0,
            &mut general,
            None,
            None,
            false,
        )
        .into_parts();
        let handle = 0x501;
        let entity = world
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
                car,
                audio_motion,
            ))
            .id();
        world
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .insert(handle, entity);
        let _join = SystemState::<(
            AudioVehicleActors,
            Query<&crate::vehicles::Automobile::AutomobileState>,
            Query<&crate::vehicles::Boat::BoatState>,
            Query<&crate::vehicles::Heli::HeliState>,
            Query<&crate::vehicles::Plane::PlaneState>,
            Query<&mut crate::vehicles::Automobile::VehicleCollisionState>,
            Query<&mut crate::peds::Ped::CPed>,
        )>::new(&mut world);
        world
            .get_mut::<VehicleDriveState>(entity)
            .unwrap()
            .gas_pedal = 0.65;
        world.get_mut::<VehicleVitality>(entity).unwrap().health = 325.0;
        let mut audio = SystemState::<AudioVehicleActors>::new(&mut world);
        {
            let mut actors = audio.get_mut(&mut world).unwrap();
            let vehicle = actors.vehicles.get_mut(&actors.index, &handle).unwrap();
            match vehicle {
                AudioVehicleMut::Automobile { car } => {
                    car.vehicle.audio.car_horn_timer = 44;
                    car.audio_motion.gas_pedal_audio = 0.75;
                    car.audio_motion.velocity_change_for_audio = 0.125;
                }
                _ => panic!("expected automobile"),
            }
            let vehicle = actors.vehicles.get_mut(&actors.index, &handle).unwrap();
            assert_eq!(vehicle.base().drive.gas_pedal, 0.65);
            assert_eq!(vehicle.base().vitality.health, 325.0);
        }
        assert_eq!(
            world
                .get::<crate::vehicles::Vehicle::VehicleAudioState>(entity)
                .unwrap()
                .car_horn_timer,
            44
        );
        let motion = world.get::<crate::vehicles::Automobile::AutomobileAudioMotion>(entity).unwrap();
        assert_eq!(motion.gas_pedal_audio, 0.75);
        assert_eq!(motion.velocity_change_for_audio, 0.125);
        world
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .remove(&handle);
        let mut actors = audio.get_mut(&mut world).unwrap();
        assert!(actors.vehicles.get_mut(&actors.index, &handle).is_none());
    }

    #[test]
    fn ped_events_keep_pending_input_without_listener_and_retire_audio_after_despawn() {
        let mut app = App::new();
        app.init_resource::<ScriptEntityIndex>()
            .init_resource::<AudioActorDiagnostics>();
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        app.insert_resource(dmaudio);
        let mut general = CGeneral::with_seed(1);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        ped.queued_footstep = Some(SOUND_STEP_START as u16);
        let handle = 0x102;
        let entity = app
            .world_mut()
            .spawn((
                ped,
                ScriptPedIdentity {
                    handle,
                    model_id: 0,
                },
                PedEntity,
            ))
            .id();
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .insert(handle, entity);
        // An alias to the same Entity is rejected by the identity component.
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .insert(0x101, entity);
        app.init_resource::<super::super::PhysicalAudioLifecycle::PhysicalAudioBindings>()
            .add_systems(Update, (
                super::super::PhysicalAudioLifecycle::retire_physical_audio,
                super::super::PhysicalAudioLifecycle::allocate_ped_audio_entities,
                process_ped_audio,
            ).chain());
        app.update();
        assert_eq!(
            app.world().get::<CPed>(entity).unwrap().queued_footstep,
            Some(SOUND_STEP_START as u16)
        );
        assert_eq!(
            app.world()
                .resource::<AudioActorDiagnostics>()
                .missing_listener_frames,
            1
        );
        app.insert_resource(crate::core::Camera::CCamera::default());
        app.update();
        let ped = app.world().get::<CPed>(entity).unwrap();
        assert!(ped.queued_footstep.is_none());
        let audio_id = ped.physical.audio_entity_id;
        assert!(audio_id >= 0);
        // Teardown does not wait for a camera or processing dependencies.
        app.world_mut()
            .remove_resource::<crate::core::Camera::CCamera>();
        app.world_mut().despawn(entity);
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .remove(&handle);
        app.update();
        assert!(
            app.world()
                .resource::<cDMAudio>()
                .AudioManager
                .destroyed_entities
                .contains(&audio_id)
        );
    }

    #[test]
    fn script_gunshell_uses_live_focused_surface_and_removes_one_shot_entities() {
        for (surface, expected) in [
            (
                eSurfaceType::SURFACE_PAVEMENT,
                Some(SFX_BULLET_SHELL_HIT_GROUND_1 as u32),
            ),
            (
                eSurfaceType::SURFACE_GRASS,
                Some(SFX_BULLET_SHELL_HIT_GROUND_2 as u32),
            ),
            (eSurfaceType::SURFACE_WATER, None),
        ] {
            let mut app = App::new();
            app.init_resource::<AudioActorDiagnostics>()
                .init_resource::<CWorldPlayerState>();
            app.insert_resource(CTimer::default())
                .insert_resource(crate::core::Camera::CCamera::default());
            let mut general = CGeneral::with_seed(2);
            let mut player = CPlayerPed::new(&mut general, None);
            player.ped.physical.surface_touched = surface as u8;
            let player_entity = app.world_mut().spawn(player.into_bundle()).id();
            app.world_mut().resource_mut::<CWorldPlayerState>().entities[0] = Some(player_entity);
            let mut audio = cDMAudio::default();
            audio.Initialise();
            let mut object = cAudioScriptObject {
                AudioId: SCRIPT_SOUND_GUNSHELL_DROP as i16,
                Posn: CVector::ZERO,
                AudioEntity: crate::audio::AudioScriptObject::AEHANDLE_NONE,
            };
            audio.CreateOneShotScriptObject(&mut object);
            let audio_id = audio
                .AudioManager
                .m_sAudioScriptObjectManager
                .m_anScriptObjectEntityIndices[0];
            app.insert_resource(audio)
                .add_systems(Update, process_script_audio);
            app.update();
            let audio = &app.world().resource::<cDMAudio>().AudioManager;
            let queue = audio.m_nActiveQueue;
            let count = audio.m_nRequestedCount[queue];
            match expected {
                Some(sample) => {
                    assert!(count > 0);
                    assert_eq!(audio.m_aRequestedQueue[queue][0].m_nSampleIndex, sample);
                }
                None => assert_eq!(count, 0),
            }
            assert!(audio.destroyed_entities.contains(&audio_id));
            assert_eq!(
                audio.m_sAudioScriptObjectManager.m_nScriptObjectEntityTotal,
                0
            );
        }
    }
}
