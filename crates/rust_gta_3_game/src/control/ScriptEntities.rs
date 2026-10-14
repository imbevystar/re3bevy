//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Split from control/Script.rs (verbatim, parity-preserving).

#[allow(unused_imports)]
use crate::peds::PedLocomotion::PedLocomotion;

use super::Script::SamePassOverlay;
use crate::vehicles::Automobile::VehicleCollisionState;
use crate::vehicles::Automobile::{
    AutomobileAudioMotion, AutomobileMut, AutomobileRef, AutomobileRuntimeView, AutomobileState,
};
use crate::vehicles::Boat::{BoatMut, BoatRef, BoatRuntimeView, BoatState};
use crate::vehicles::Heli::{HeliMut, HeliRef, HeliRuntimeView, HeliState};
use crate::vehicles::Plane::{PlaneMut, PlaneRef, PlaneRuntimeView, PlaneState};
use crate::vehicles::Vehicle::{
    VehicleCore, VehicleDriveState, VehicleAppearance, VehicleEntryState, VehicleAudioState, VehicleMut, VehicleRef, VehicleRuntime, VehicleVitality,
};
use bevy::prelude::Mut;

#[allow(unused_imports)]
use super::ScriptSnapshots::{
    remove_script_vehicle_and_occupants, script_vehicle_base_can_be_deleted,
};
#[allow(unused_imports)]
use super::ScriptTypes::{
    CTheScripts, ScriptCommandEffect, ScriptCreateCarRequest, ScriptCreateCharRequest,
    ScriptEntityMutation, ScriptEntityMutationResult, ScriptEntityRuntimeAccess,
    ScriptEntitySnapshot, ScriptHandle, ScriptPlayerSnapshot,
};
#[allow(unused_imports)]
use crate::animation::CutsceneMgr::{
    CCutsceneMgr, load_cutscene_head_animation, load_cutscene_runtime_data,
    restore_after_cutscene_clear,
};
#[allow(unused_imports)]
use crate::animation::PedVehicleAnimation::CPedVehicleAnimation;
#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::collision::ColModel::{CColBox, CColSphere, CColVector};
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::control::AutoPilot::{
    eCarDrivingStyle, eCarMission, eCarTempAction, script_car_mission,
};
#[allow(unused_imports)]
use crate::control::Darkel::CDarkel;
#[allow(unused_imports)]
use crate::control::GameLogic::{CGameLogic, GameLogicIntent, ResurrectionKind};
#[allow(unused_imports)]
use crate::control::Garages::{CGarages, garage_contains_vehicle};
#[allow(unused_imports)]
use crate::control::Garages::{CStoredCar, destroy_script_vehicle_and_occupants};
#[allow(unused_imports)]
use crate::control::OnscreenTimer::COnscreenTimer;
#[allow(unused_imports)]
use crate::control::PathFind::CPathFind;
#[allow(unused_imports)]
use crate::control::Pickups::{
    CPacManPickups, PICKUP_COLLECTABLE1, PICKUP_FLOATINGPACKAGE, remove_pacman_object,
    reset_vehicle_pacman_load, spawn_pacman_objects,
};
#[allow(unused_imports)]
use crate::control::Record::{CHASE_CARS, CRecordDataForChase};
#[allow(unused_imports)]
use crate::control::Remote::CRemote;
#[allow(unused_imports)]
use crate::control::Replay::{CReplay, ReplayMode};
#[allow(unused_imports)]
use crate::control::Restart::CRestart;
#[allow(unused_imports)]
use crate::core::Camera::{CCamera, CameraScriptMode};
#[allow(unused_imports)]
use crate::core::EventList::CEventList;
#[allow(unused_imports)]
use crate::core::Fire::CFireManager;
#[allow(unused_imports)]
use crate::core::Frontend::CMenuManager;
#[allow(unused_imports)]
use crate::core::Game::{
    CGameBeforeRw, cutscene_model_snapshot, find_links_to_go_with_these_nodes,
    focused_player_vehicle_handle, join_car_with_road_system, join_car_with_road_system_goto_coors,
    snap_follow_camera_resource,
};
#[allow(unused_imports)]
use crate::core::Pad::{CPads, PLAYERCONTROL_CAMERA};
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use crate::core::Pools::CPoolHandle;
#[allow(unused_imports)]
use crate::core::References::CReferences;
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::core::Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY};
#[allow(unused_imports)]
use crate::core::World::EEntityType;
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::entities::Entity::script_entity_status;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_DEADDODO;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::script_vehicle_base_z;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::{choose_vehicle_optional_components, eVehicleType};
#[allow(unused_imports)]
use crate::objects::Object::TEMP_OBJECT;
#[allow(unused_imports)]
#[allow(unused_imports)]
use crate::objects::ParticleObject::CParticleObjectRuntime;
#[allow(unused_imports)]
use crate::peds::Gangs::CGangs;
#[allow(unused_imports)]
use crate::peds::Ped::{
    RANDOM_CHAR, add_ped_ammo, apply_ped_objective, apply_ped_wait_state,
    apply_script_ped_wander_path, give_ped_weapon, set_ped_ammo, set_script_ped_chat,
};
#[allow(unused_imports)]
use crate::peds::Ped::{ped_seat_world_position, player_seat_index};
#[allow(unused_imports)]
use crate::peds::PedRoutes::CPedRoutes;
#[allow(unused_imports)]
use crate::peds::PedType::script_ped_type;
#[allow(unused_imports)]
use crate::peds::PedType::{CPedTypeRuntime, PED_STATS_TYPES, ePedType};
#[allow(unused_imports)]
use crate::peds::PlayerPed::{CPlayerPed, CWorldPlayers, WorldPlayerAccess};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::{
    InVehicle, PedEntity, VehicleEntity, queue_clear_vehicle_relation,
    queue_replace_vehicle_relation, queue_set_vehicle_relation,
};
use crate::peds::{PedAnimation::PedAnimation, PedVitality::PedVitality};
#[allow(unused_imports)]
use crate::renderer::Coronas::CCoronas;
#[allow(unused_imports)]
use crate::renderer::Credits::CCredits;
#[allow(unused_imports)]
use crate::renderer::Font::CFont;
#[allow(unused_imports)]
use crate::renderer::Hud::CHud;
#[allow(unused_imports)]
use crate::renderer::Particle::CParticle;
#[allow(unused_imports)]
use crate::renderer::PointLights::CPointLights;
#[allow(unused_imports)]
use crate::renderer::Rubbish::CRubbish;
#[allow(unused_imports)]
use crate::renderer::Shadows::{CShadows, ScriptShadow};
#[allow(unused_imports)]
use crate::renderer::SpecialFX::{C3dMarkerState, CSpecialFX, MARKERTYPE_CYLINDER};
#[allow(unused_imports)]
use crate::renderer::WaterLevel::CWaterLevel;
#[allow(unused_imports)]
use crate::renderer::Weather::CWeather;
#[allow(unused_imports)]
use crate::text::Messages::CMessages;
#[allow(unused_imports)]
use crate::text::Pager::CPager;
#[allow(unused_imports)]
use crate::text::Text::CText;
#[allow(unused_imports)]
use crate::vehicles::Automobile::{script_automobile_base_z, script_bomb_type};
#[allow(unused_imports)]
use crate::vehicles::Boat::CBoat;
#[allow(unused_imports)]
use crate::vehicles::CarGen::CTheCarGenerators;
#[allow(unused_imports)]
use crate::vehicles::Cranes::CCranes;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Heli::CHeliRuntime;
#[allow(unused_imports)]
use crate::vehicles::Heli::{CHeli, HELI_TYPE_CATALINA};
#[allow(unused_imports)]
use crate::vehicles::Plane::CPlane;
#[allow(unused_imports)]
use crate::vehicles::Plane::{CPlaneRuntime, CesnaMissionStatus};
#[allow(unused_imports)]
use crate::vehicles::Vehicle::PERMANENT_VEHICLE;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::{
    MISSION_VEHICLE, PARKED_VEHICLE, RANDOM_VEHICLE, eCarLock, script_car_lock,
};
#[allow(unused_imports)]
use crate::weapons::Explosion::{CExplosionPool, eExplosionType, script_explosion_type};
#[allow(unused_imports)]
use crate::weapons::ProjectileInfo::CProjectileInfoPool;
#[allow(unused_imports)]
use crate::weapons::Weapon::CWeapon;
#[allow(unused_imports)]
use crate::weapons::WeaponInfo::CWeaponInfoStore;
#[allow(unused_imports)]
use crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS;
#[allow(unused_imports)]
use crate::weapons::WeaponType::script_weapon_type;
/// Script-driven gameplay bridge until full CWorld player/cutscene ECS is wired.
/// Values come only from opcode effects that mirror re3 side effects.
use bevy::ecs::system::SystemParam;
#[allow(unused_imports)]
use bevy::input::{ButtonInput, mouse::MouseButton};
#[allow(unused_imports)]
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};
#[allow(unused_imports)]
use bevy::prelude::{KeyCode, error};
#[allow(unused_imports)]
use rust_gta_3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use rust_gta_3_assets::streaming::CdStream::CdStream;
#[allow(unused_imports)]
use rust_gta_3_core::core::GameState::{CGameState, ELevelName};
#[allow(unused_imports)]
use rust_gta_3_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral};

#[allow(unused_imports)]
use crate::audio::soundlist::*;
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
#[allow(unused_imports)]
use crate::control::Pickups::{CPickups, PICKUP_IN_SHOP, PICKUP_IN_SHOP_OUT_OF_STOCK};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
// FAZ8a bridge consumed by FAZ8d: ScriptEntityIndex, ScriptEntityWorld,
// ScriptObjectEntity, ScriptPedEntity, ScriptVehicleEntity, spawn_script_object
// and remove_script_object now live in this module.
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPools};
#[allow(unused_imports)]
use crate::entities::Physical::{GTA_PHYSICS_HZ, physical_collision_records};
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_PICKUP_BONUS;
#[allow(unused_imports)]
use crate::objects::Object::{CObject, MISSION_OBJECT};
#[allow(unused_imports)]
use crate::peds::Ped::{
    CPed, MISSION_CHAR, PedState, eMoveState, eObjective, ped_weapon_ammo_snapshot,
    script_ped_can_see_player,
};
#[allow(unused_imports)]
use crate::peds::Population::{CPopulation, remove_ped_from_population_counts};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::VehicleSeat;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::CVehicle;
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use rust_gta_3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Resource)]
pub struct ScriptGameplayBridge {
    /// Ammo commands retained because the original WEAPON.DAT store is absent.
    pub weapon_data_deferrals: u64,
    pub player_spawned: bool,
    pub player_index: i32,
    pub player_position: rust_gta_3_core::core::Coordinates::CVector,
    pub player_heading_degrees: f32,
    pub player_visible: bool,
    pub player_control_enabled: bool,
    pub cutscene_name: String,
    pub cutscene_requested: bool,
    pub cutscene_started: bool,
    pub widescreen: bool,
    pub playing_intro: bool,
    pub splash_screen_name: String,
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct ScriptTaxiLightState {
    pub all_enabled: bool,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct ScriptPickupIndex(pub HashMap<i32, i32>);

#[derive(Debug, Clone, Default, Resource)]
pub struct ScriptRadarBlipIndex(pub HashMap<i32, i32>);

/// Borrowed views join identity metadata with the single live ped component.
/// They are transient access values, never persisted gameplay copies.
pub struct ScriptPedRef<'a> {
    pub handle: i32,
    pub model_id: i32,
    pub ped: &'a CPed,
    pub vitality: &'a PedVitality,
    pub locomotion: &'a PedLocomotion,
    pub animation: &'a PedAnimation,
}

pub struct ScriptPedMut<'a> {
    pub handle: i32,
    pub model_id: i32,
    pub ped: &'a mut CPed,
    pub vitality: &'a mut PedVitality,
    pub locomotion: &'a mut PedLocomotion,
    pub animation: &'a mut PedAnimation,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct ScriptPedIdentity {
    pub handle: i32,
    pub model_id: i32,
}

#[derive(SystemParam)]
pub struct ScriptPedStore<'w, 's> {
    entities: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptPedIdentity,
            &'static mut CPed,
            &'static mut PedVitality,
            &'static mut PedLocomotion,
            &'static mut PedAnimation,
        ),
        bevy::prelude::Without<crate::peds::PlayerPed::PlayerPedState>,
    >,
}

impl ScriptPedStore<'_, '_> {
    pub(crate) fn entity(&self, index: &ScriptEntityIndex, handle: i32) -> Option<Entity> {
        let entity = index.ped(handle)?;
        self.entities
            .get(entity)
            .ok()
            .filter(|(_, identity, _, _, _, _)| identity.handle == handle)
            .map(|(entity, _, _, _, _, _)| entity)
    }
    pub fn contains_key(&self, index: &ScriptEntityIndex, handle: &i32) -> bool {
        self.entity(index, *handle).is_some()
    }
    pub fn get(&self, index: &ScriptEntityIndex, handle: &i32) -> Option<ScriptPedRef<'_>> {
        self.get_entity(self.entity(index, *handle)?)
    }
    pub(crate) fn get_entity(&self, entity: Entity) -> Option<ScriptPedRef<'_>> {
        self.entities
            .get(entity)
            .ok()
            .map(
                |(_, identity, ped, vitality, locomotion, animation)| ScriptPedRef {
                    handle: identity.handle,
                    model_id: identity.model_id,
                    ped,
                    vitality,
                    locomotion,
                    animation,
                },
            )
    }
    pub fn get_mut(&mut self, index: &ScriptEntityIndex, handle: &i32) -> Option<ScriptPedMut<'_>> {
        let entity = self.entity(index, *handle)?;
        self.get_entity_mut(entity)
    }
    pub(crate) fn get_entity_mut(&mut self, entity: Entity) -> Option<ScriptPedMut<'_>> {
        self.entities.get_mut(entity).ok().map(
            |(_, identity, ped, vitality, locomotion, animation)| ScriptPedMut {
                handle: identity.handle,
                model_id: identity.model_id,
                ped: ped.into_inner(),
                vitality: vitality.into_inner(),
                locomotion: locomotion.into_inner(),
                animation: animation.into_inner(),
            },
        )
    }
    pub fn values(&self) -> impl Iterator<Item = ScriptPedRef<'_>> + '_ {
        self.entities
            .iter()
            .map(
                |(_, identity, ped, vitality, locomotion, animation)| ScriptPedRef {
                    handle: identity.handle,
                    model_id: identity.model_id,
                    ped,
                    vitality,
                    locomotion,
                    animation,
                },
            )
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = ScriptPedMut<'_>> + '_ {
        self.entities
            .iter_mut()
            .map(
                |(_, identity, ped, vitality, locomotion, animation)| ScriptPedMut {
                    handle: identity.handle,
                    model_id: identity.model_id,
                    ped: ped.into_inner(),
                    vitality: vitality.into_inner(),
                    locomotion: locomotion.into_inner(),
                    animation: animation.into_inner(),
                },
            )
    }
    pub fn iter(&self) -> impl Iterator<Item = (&i32, ScriptPedRef<'_>)> + '_ {
        self.entities
            .iter()
            .map(|(_, identity, ped, vitality, locomotion, animation)| {
                (
                    &identity.handle,
                    ScriptPedRef {
                        handle: identity.handle,
                        model_id: identity.model_id,
                        ped,
                        vitality,
                        locomotion,
                        animation,
                    },
                )
            })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, ScriptPedMut<'_>)> + '_ {
        self.entities
            .iter_mut()
            .map(|(_, identity, ped, vitality, locomotion, animation)| {
                (
                    identity.handle,
                    ScriptPedMut {
                        handle: identity.handle,
                        model_id: identity.model_id,
                        ped: ped.into_inner(),
                        vitality: vitality.into_inner(),
                        locomotion: locomotion.into_inner(),
                        animation: animation.into_inner(),
                    },
                )
            })
    }
    pub fn keys(&self) -> impl Iterator<Item = &i32> + '_ {
        self.entities
            .iter()
            .map(|(_, identity, _, _, _, _)| &identity.handle)
    }
    pub fn len(&self) -> usize {
        self.entities.iter().count()
    }
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

pub(crate) type VehicleReadComponents = (
    Entity,
    &'static ScriptVehicleIdentity,
    &'static VehicleCore,
    &'static VehicleVitality,
    &'static VehicleDriveState,
    &'static VehicleAppearance,
    &'static VehicleEntryState,
    &'static VehicleAudioState,
    Option<&'static AutomobileState>,
    Option<&'static AutomobileAudioMotion>,
    Option<&'static BoatState>,
    Option<&'static HeliState>,
    Option<&'static PlaneState>,
);
/// Immutable live vehicle components; construction data never enters this query.
#[derive(SystemParam)]
pub struct VehicleReadStore<'w, 's> {
    pub(crate) entities: Query<'w, 's, VehicleReadComponents>,
}
impl VehicleReadStore<'_, '_> {
    pub fn iter(&self) -> impl Iterator<Item = ScriptVehicleRef<'_>> {
        self.entities.iter().filter_map(vehicle_ref)
    }
    pub fn get(&self, entity: Entity) -> Result<ScriptVehicleRef<'_>, ()> {
        vehicle_ref(self.entities.get(entity).map_err(|_| ())?).ok_or(())
    }
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
pub(crate) type VehicleComponents = (
    Entity,
    &'static ScriptVehicleIdentity,
    &'static mut VehicleCore,
    &'static mut VehicleVitality,
    &'static mut VehicleDriveState,
    &'static mut VehicleAppearance,
    &'static mut VehicleEntryState,
    &'static mut VehicleAudioState,
    Option<&'static mut AutomobileState>,
    Option<&'static mut AutomobileAudioMotion>,
    Option<&'static mut BoatState>,
    Option<&'static mut HeliState>,
    Option<&'static mut PlaneState>,
);

/// Immutable common vehicle input for systems that do not need concrete-kind,
/// vitality or collision-history access. The canonical index owns generation.
#[derive(SystemParam)]
pub(crate) struct VehicleCoreRead<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    vehicles: Query<'w, 's, (&'static ScriptVehicleIdentity, &'static VehicleCore)>,
}

impl VehicleCoreRead<'_, '_> {
    pub(crate) fn get(&self, handle: i32) -> Option<&VehicleCore> {
        let entity = self.index.vehicle(handle)?;
        let (identity, core) = self.vehicles.get(entity).ok()?;
        (identity.handle == handle).then_some(core)
    }

    pub(crate) fn get_entity(&self, entity: Entity) -> Option<&VehicleCore> {
        let (identity, core) = self.vehicles.get(entity).ok()?;
        (self.index.vehicle(identity.handle) == Some(entity)).then_some(core)
    }
}

#[cfg(test)]
mod vehicle_core_read_tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn readonly_vehicle_inputs_share_live_pose_and_respect_index_retirement() {
        let mut world = bevy::prelude::World::new();
        world.init_resource::<ScriptEntityIndex>();
        let mut general = CGeneral::with_seed(23);
        let handle = 0x401;
        let (core, vitality, drive, appearance, entry, audio) = CVehicle::new(0, 0, &mut general).into_parts();
        let entity = world.spawn((
            ScriptVehicleIdentity { handle, kind: ScriptVehicleKind::Automobile },
            core, vitality, drive, appearance, entry, audio,
        )).id();
        world.resource_mut::<ScriptEntityIndex>().vehicles.insert(handle, entity);
        let mut state = SystemState::<(VehicleCoreRead, Query<&mut VehicleVitality>)>::new(&mut world);
        let expected = CVector::new(12.0, 34.0, 5.0);
        world.get_mut::<VehicleCore>(entity).unwrap().physical.entity.set_position(expected);
        {
            let (reader, mut vitality) = state.get_mut(&mut world).unwrap();
            vitality.get_mut(entity).unwrap().health = 31.0;
            assert_eq!(reader.get(handle).unwrap().physical.entity.get_position(), expected);
            assert_eq!(reader.get_entity(entity).unwrap().physical.entity.get_position(), expected);
            assert!(reader.get(handle + 1).is_none());
        }
        // Retirement takes effect before the deferred entity despawn.
        world.resource_mut::<ScriptEntityIndex>().vehicles.remove(&handle);
        assert!(world.get_entity(entity).is_ok());
        let (reader, _) = state.get_mut(&mut world).unwrap();
        assert!(reader.get(handle).is_none());
        assert!(reader.get_entity(entity).is_none());
        // A mismatched generation in the index cannot revive this identity.
        world.resource_mut::<ScriptEntityIndex>().vehicles.insert(handle + 1, entity);
        let (reader, _) = state.get_mut(&mut world).unwrap();
        assert!(reader.get(handle + 1).is_none());
        assert!(reader.get_entity(entity).is_none());
    }
}

#[derive(SystemParam)]
pub struct ScriptVehicleStore<'w, 's> {
    pub(crate) entities: Query<'w, 's, VehicleComponents>,
    collision_states: Query<'w, 's, &'static mut VehicleCollisionState>,
}
pub(crate) fn vehicle_ref<'a>(
    item: (
        Entity,
        &'a ScriptVehicleIdentity,
        &'a VehicleCore,
        &'a VehicleVitality,
        &'a VehicleDriveState,
        &'a VehicleAppearance,
    &'a VehicleEntryState,
        &'a VehicleAudioState,
        Option<&'a AutomobileState>,
        Option<&'a AutomobileAudioMotion>,
        Option<&'a BoatState>,
        Option<&'a HeliState>,
        Option<&'a PlaneState>,
    ),
) -> Option<ScriptVehicleRef<'a>> {
    let (_, identity, core, vitality, drive, appearance, entry, audio, car, audio_motion, boat, heli, plane) = item;
    let handle = identity.handle;
    let vehicle = VehicleRuntime {
        core: core,
        vitality: vitality,
        drive: drive,
        appearance: appearance,
        entry,
        audio,
    };
    let result = match identity.kind {
        ScriptVehicleKind::Automobile => car.zip(audio_motion).map(|(state, audio_motion)| ScriptVehicleRef::Automobile {
            handle,
            car: AutomobileRuntimeView {
                vehicle,
                state: state,
                audio_motion,
            },
        }),
        ScriptVehicleKind::Boat => boat.map(|state| ScriptVehicleRef::Boat {
            handle,
            boat: BoatRuntimeView {
                vehicle,
                state: state,
            },
        }),
        ScriptVehicleKind::Heli => heli.map(|state| ScriptVehicleRef::Heli {
            handle,
            heli: HeliRuntimeView {
                vehicle,
                state: state,
            },
        }),
        ScriptVehicleKind::Plane => plane.map(|state| ScriptVehicleRef::Plane {
            handle,
            plane: PlaneRuntimeView {
                vehicle,
                state: state,
            },
        }),
    };
    if result.is_none() {
        bevy::log::warn_once!(target:"rust_gta_3::vehicle_lifecycle","required concrete vehicle component absent");
    }
    result
}
pub(crate) fn vehicle_mut<'a>(
    item: (
        Entity,
        &'a ScriptVehicleIdentity,
        Mut<'a, VehicleCore>,
        Mut<'a, VehicleVitality>,
        Mut<'a, VehicleDriveState>,
        Mut<'a, VehicleAppearance>,
        Mut<'a, VehicleEntryState>,
        Mut<'a, VehicleAudioState>,
        Option<Mut<'a, AutomobileState>>,
        Option<Mut<'a, AutomobileAudioMotion>>,
        Option<Mut<'a, BoatState>>,
        Option<Mut<'a, HeliState>>,
        Option<Mut<'a, PlaneState>>,
    ),
) -> Option<ScriptVehicleMut<'a>> {
    let (_, identity, core, vitality, drive, appearance, entry, audio, car, audio_motion, boat, heli, plane) = item;
    let handle = identity.handle;
    let vehicle = VehicleRuntime {
        core: core.into_inner(),
        vitality: vitality.into_inner(),
        drive: drive.into_inner(),
        appearance: appearance.into_inner(),
        entry: entry.into_inner(),
        audio: audio.into_inner(),
    };
    let result = match identity.kind {
        ScriptVehicleKind::Automobile => car.zip(audio_motion).map(|(state, audio_motion)| ScriptVehicleMut::Automobile {
            handle,
            car: AutomobileRuntimeView {
                vehicle,
                state: state.into_inner(),
                audio_motion: audio_motion.into_inner(),
            },
        }),
        ScriptVehicleKind::Boat => boat.map(|state| ScriptVehicleMut::Boat {
            handle,
            boat: BoatRuntimeView {
                vehicle,
                state: state.into_inner(),
            },
        }),
        ScriptVehicleKind::Heli => heli.map(|state| ScriptVehicleMut::Heli {
            handle,
            heli: HeliRuntimeView {
                vehicle,
                state: state.into_inner(),
            },
        }),
        ScriptVehicleKind::Plane => plane.map(|state| ScriptVehicleMut::Plane {
            handle,
            plane: PlaneRuntimeView {
                vehicle,
                state: state.into_inner(),
            },
        }),
    };
    if result.is_none() {
        bevy::log::warn_once!(target:"rust_gta_3::vehicle_lifecycle","required concrete vehicle component absent");
    }
    result
}
impl ScriptVehicleStore<'_, '_> {
    pub(crate) fn capture_collision_pose(&mut self, index: &ScriptEntityIndex, handle: i32) {
        let Some(entity) = self.entity(index, handle) else {
            return;
        };
        let pose = self
            .entities
            .get(entity)
            .expect("resolved vehicle identity")
            .2
            .physical
            .entity
            .matrix;
        self.collision_states
            .get_mut(entity)
            .expect("required VehicleCollisionState")
            .previous_matrix = pose;
    }
    pub(crate) fn collision_pose(&self, index: &ScriptEntityIndex, handle: i32) -> Option<CMatrix> {
        let entity = self.entity(index, handle)?;
        Some(
            self.collision_states
                .get(entity)
                .expect("required VehicleCollisionState")
                .previous_matrix,
        )
    }
    pub(crate) fn entity(&self, index: &ScriptEntityIndex, handle: i32) -> Option<Entity> {
        let entity = index.vehicle(handle)?;
        self.entities
            .get(entity)
            .ok()
            .filter(|item| item.1.handle == handle)
            .map(|item| item.0)
    }
    pub fn contains_key(&self, index: &ScriptEntityIndex, handle: &i32) -> bool {
        self.entity(index, *handle).is_some()
    }
    pub fn get(&self, index: &ScriptEntityIndex, handle: &i32) -> Option<ScriptVehicleRef<'_>> {
        self.get_entity(self.entity(index, *handle)?)
    }
    pub(crate) fn get_entity(&self, entity: Entity) -> Option<ScriptVehicleRef<'_>> {
        vehicle_ref(self.entities.get(entity).ok()?)
    }
    pub fn get_mut(
        &mut self,
        index: &ScriptEntityIndex,
        handle: &i32,
    ) -> Option<ScriptVehicleMut<'_>> {
        let entity = self.entity(index, *handle)?;
        self.get_entity_mut(entity)
    }
    pub(crate) fn get_entity_mut(&mut self, entity: Entity) -> Option<ScriptVehicleMut<'_>> {
        vehicle_mut(self.entities.get_mut(entity).ok()?)
    }
    pub fn get_disjoint_mut(
        &mut self,
        index: &ScriptEntityIndex,
        handles: [&i32; 2],
    ) -> [Option<ScriptVehicleMut<'_>>; 2] {
        let Some(a) = self.entity(index, *handles[0]) else {
            return [None, None];
        };
        let Some(b) = self.entity(index, *handles[1]) else {
            return [None, None];
        };
        if a == b {
            return [None, None];
        }
        match self.entities.get_many_mut([a, b]) {
            Ok([a, b]) => [vehicle_mut(a), vehicle_mut(b)],
            Err(_) => [None, None],
        }
    }
    pub fn values(&self) -> impl Iterator<Item = ScriptVehicleRef<'_>> + '_ {
        self.entities.iter().filter_map(vehicle_ref)
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = ScriptVehicleMut<'_>> + '_ {
        self.entities.iter_mut().filter_map(vehicle_mut)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&i32, ScriptVehicleRef<'_>)> + '_ {
        self.entities.iter().filter_map(|item| {
            let handle = &item.1.handle;
            Some((handle, vehicle_ref(item)?))
        })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, ScriptVehicleMut<'_>)> + '_ {
        self.entities.iter_mut().filter_map(|item| {
            let handle = item.1.handle;
            Some((handle, vehicle_mut(item)?))
        })
    }
    pub fn keys(&self) -> impl Iterator<Item = &i32> + '_ {
        self.entities.iter().map(|item| &item.1.handle)
    }
    pub fn len(&self) -> usize {
        self.entities.iter().count()
    }
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

// SamePassOverlay impls + ScriptEntityWorld impl (script ECS access layer).
impl ScriptEntityWorld<'_, '_> {
    pub fn object_get(&self, handle: i32) -> Option<ScriptObjectRef<'_>> {
        self.objects.get(&self.index, &handle)
    }
    pub fn object_get_mut(&mut self, handle: i32) -> Option<ScriptObjectMut<'_>> {
        self.objects.get_mut(&self.index, &handle)
    }
    pub fn object_spawn(&mut self, entity: ScriptObjectEntity) -> i32 {
        spawn_script_object(&mut self.commands, &mut self.index, entity)
    }
    pub fn object_remove(&mut self, pools: Option<&mut CPools>, handle: i32) -> bool {
        remove_script_object(&mut self.commands, &mut self.index, pools, handle)
    }

    /// Fast handle lookup through the permitted SCM handle -> Entity index.
    /// Store access requires this same index, preserving generation-bearing
    /// handle identity without scanning live component queries.
    pub fn ped_get(&self, handle: i32) -> Option<ScriptPedRef<'_>> {
        self.peds.get(&self.index, &handle)
    }

    pub fn ped_get_mut(&mut self, handle: i32) -> Option<ScriptPedMut<'_>> {
        self.peds.get_mut(&self.index, &handle)
    }

    pub fn vehicle_get(&self, handle: i32) -> Option<ScriptVehicleRef<'_>> {
        self.vehicles.get(&self.index, &handle)
    }

    pub fn vehicle_get_mut(&mut self, handle: i32) -> Option<ScriptVehicleMut<'_>> {
        self.vehicles.get_mut(&self.index, &handle)
    }

    /// Number of live script vehicles, queried from Bevy components.
    pub fn vehicle_count(&self) -> usize {
        self.vehicles.len()
    }

    /// Small diagnostic projection used by non-production parity harnesses.
    pub fn vehicle_position_and_engine(&self, handle: i32) -> Option<(f32, bool)> {
        let vehicle = self.vehicles.get(&self.index, &handle)?;
        Some((
            vehicle.base().physical.entity.get_position().z,
            vehicle.base().drive.engine_on,
        ))
    }

    pub(crate) fn spawn_ped(&mut self, entity: ScriptPedEntity) -> i32 {
        let handle = entity.handle;
        let bevy_entity = self
            .commands
            .spawn((PedEntity, ScriptHandle(handle), entity.into_bundle()))
            .id();
        self.index.peds.insert(handle, bevy_entity);
        handle
    }

    pub(crate) fn spawn_vehicle(&mut self, entity: ScriptVehicleEntity) -> i32 {
        let handle = entity.handle();
        let mut commands = self.commands.spawn_empty();
        let bevy_entity = commands.id();
        entity.insert_into(&mut commands);
        self.index.vehicles.insert(handle, bevy_entity);
        handle
    }

    pub(crate) fn despawn_vehicle(&mut self, handle: i32) -> bool {
        let Some(entity) = self.index.vehicles.remove(&handle) else {
            return false;
        };
        self.commands.entity(entity).despawn();
        true
    }

    pub fn char_snapshot(&self, handle: i32) -> Option<ScriptEntitySnapshot> {
        let entity = self.ped_get(handle)?;
        let ped = entity.ped;
        let vitality = entity.vitality;
        Some(ScriptEntitySnapshot {
            position: ped.physical.entity.get_position(),
            bound_center: ped.physical.entity.get_position(),
            bound_radius: 0.8,
            heading_degrees: ped.physical.entity.get_heading_degrees(),
            up_z: ped.physical.entity.get_up().z,
            model_index: i32::from(ped.physical.entity.model_index),
            entity_status: ped.physical.entity.status as i32,
            alive: vitality.health > 0.0,
            current_vehicle: self.current_vehicle_handle_for_char(handle),
            current_vehicle_model: None,
            vehicle_colour1: None,
            vehicle_colour2: None,
            vehicle_is_boat: false,
            vehicle_in_water: false,
            automobile_drive_wheels_on_ground: None,
            stopped: matches!(
                ped.move_state,
                eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
            ),
            is_shooting: ped.flags.is_shooting,
            collision_records: physical_collision_records(&ped.physical),
            speed: ped.physical.move_speed.magnitude(),
            health: vitality.health,
            current_weapon: ped.current_weapon as i32,
            weapon_ammo: ped_weapon_ammo_snapshot(ped),
            script_objective_completed: ped.flags.script_objective_completed,
            leader_slot: ped.leader,
            ped_type: Some(ped.ped_type as i32),
            vehicle_bomb_type: None,
            vehicle_damaged: false,
            last_weapon_damage: i32::from(vitality.last_wep_dam),
            vehicle_num_passengers: 0,
            vehicle_num_max_passengers: 0,
            vehicle_passenger_handles: [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS],
            vehicle_closed_doors: 0,
            can_see_player: false,
        })
    }

    pub fn vehicle_handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.index.vehicle_handle_for_slot(slot)
    }

    pub fn ped_handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.index.ped_handle_for_slot(slot)
    }

    pub fn vehicle_entity(&self, handle: i32) -> Option<Entity> {
        self.index.vehicle(handle)
    }

    pub(crate) fn object_entity(&self, handle: i32) -> Option<Entity> {
        self.index.object(handle)
    }

    pub(crate) fn object_get_entity(&self, entity: Entity) -> Option<ScriptObjectRef<'_>> {
        let object = self.objects.get_entity(entity)?;
        (self.index.object(object.handle) == Some(entity)).then_some(object)
    }

    pub fn ped_entity(&self, handle: i32) -> Option<Entity> {
        self.index.ped(handle)
    }

    pub(crate) fn relation_for_ped_entity(&self, ped: Entity) -> Option<InVehicle> {
        self.relations
            .get(ped)
            .ok()
            .and_then(|(_, relation)| relation.copied())
    }

    pub(crate) fn relation_for_char(&self, handle: i32) -> Option<InVehicle> {
        self.index
            .ped(handle)
            .and_then(|ped| self.relation_for_ped_entity(ped))
    }

    pub fn vehicle_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        let handle = self.vehicles.get_entity(entity)?.handle();
        (self.index.vehicle(handle) == Some(entity)).then_some(handle)
    }

    pub fn ped_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        let handle = self.peds.get_entity(entity)?.handle;
        (self.index.ped(handle) == Some(entity)).then_some(handle)
    }

    pub(crate) fn current_vehicle_handle_for_char(&self, handle: i32) -> Option<i32> {
        let relation = self.relation_for_char(handle)?;
        self.vehicle_handle_for_entity(relation.vehicle)
    }

    pub(crate) fn relation_occupants(
        &self,
        vehicle: Entity,
    ) -> impl Iterator<Item = (Entity, VehicleSeat)> + '_ {
        self.relations.iter().filter_map(move |(ped, relation)| {
            let relation = relation.copied()?;
            (relation.vehicle == vehicle).then_some((ped, relation.seat))
        })
    }

    pub(crate) fn script_occupant_handles(
        &self,
        vehicle: Entity,
    ) -> impl Iterator<Item = (i32, VehicleSeat)> + '_ {
        self.relation_occupants(vehicle)
            .filter_map(|(ped, seat)| self.ped_handle_for_entity(ped).map(|handle| (handle, seat)))
    }

    pub(crate) fn script_relations(&self) -> impl Iterator<Item = (i32, InVehicle)> + '_ {
        self.relations.iter().filter_map(|(ped, relation)| {
            let relation = relation.copied()?;
            self.ped_handle_for_entity(ped)
                .map(|handle| (handle, relation))
        })
    }

    fn set_relation_for_ped_entity(&mut self, ped: Entity, relation: InVehicle) -> bool {
        queue_set_vehicle_relation(&mut self.commands, &self.relations, ped, relation)
    }

    pub(crate) fn set_char_relation(
        &mut self,
        char_handle: i32,
        vehicle_handle: i32,
        seat: VehicleSeat,
    ) -> bool {
        let (Some(ped), Some(vehicle)) = (
            self.index.ped(char_handle),
            self.index.vehicle(vehicle_handle),
        ) else {
            return false;
        };
        self.set_relation_for_ped_entity(ped, InVehicle { vehicle, seat })
    }

    pub(crate) fn clear_char_relation(&mut self, char_handle: i32) -> bool {
        let Some(ped) = self.index.ped(char_handle) else {
            return false;
        };
        queue_clear_vehicle_relation(&mut self.commands, ped);
        true
    }

    pub(crate) fn clear_relation_for_ped_entity(&mut self, ped: Entity) {
        queue_clear_vehicle_relation(&mut self.commands, ped);
    }

    pub fn vehicle_snapshot(&self, handle: i32) -> Option<ScriptEntitySnapshot> {
        let vehicle = self.vehicle_get(handle)?;
        let base = vehicle.base();
        let mut passenger_handles = [None; crate::vehicles::Vehicle::MAX_VEHICLE_PASSENGERS];
        for (ped_handle, seat) in self.script_occupant_handles(self.vehicle_entity(handle)?) {
            if let VehicleSeat::Passenger(index) = seat
                && let Some(slot) = passenger_handles.get_mut(usize::from(index))
            {
                *slot = Some(ped_handle);
            }
        }
        let closed_doors = match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                let mut bits = 0_u8;
                for door in 0..crate::vehicles::Automobile::NUM_CAR_DOORS {
                    if car.damage.get_door_status(door as u8)
                        != crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_MISSING as u32
                        && car.doors[door].is_closed()
                    {
                        bits |= 1 << door;
                    }
                }
                bits
            }
            crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. } => 0,
            crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. } => 0,
            crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => 0,
        };
        Some(ScriptEntitySnapshot {
            position: base.physical.entity.get_position(),
            bound_center: base.physical.entity.get_position(),
            bound_radius: 1.2,
            heading_degrees: base.physical.entity.get_heading_degrees(),
            up_z: base.physical.entity.get_up().z,
            model_index: i32::from(base.physical.entity.model_index),
            entity_status: base.physical.entity.status as i32,
            alive: base.vitality.health > 0.0,
            current_vehicle: None,
            current_vehicle_model: Some(i32::from(base.physical.entity.model_index)),
            vehicle_colour1: Some(i32::from(base.appearance.current_colour1)),
            vehicle_colour2: Some(i32::from(base.appearance.current_colour2)),
            vehicle_is_boat: matches!(
                vehicle,
                crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
            ),
            vehicle_in_water: base.physical.flags.is_in_water,
            automobile_drive_wheels_on_ground: match vehicle {
                crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                    Some(car.drive_wheels_on_ground)
                }
                _ => None,
            },
            // Generic instantaneous snapshot. Script STOPPED opcodes replace
            // this with re3 `CTheScripts::IsVehicleStopped`, which uses the
            // current frame's `m_fDistanceTravelled` and timer timestep.
            stopped: base.physical.move_speed.magnitude() <= 0.01,
            is_shooting: false,
            collision_records: physical_collision_records(&base.physical),
            speed: base.physical.move_speed.magnitude(),
            health: base.vitality.health,
            current_weapon: 0,
            weapon_ammo: [0; WEAPONTYPE_TOTAL_INVENTORY_WEAPONS],
            script_objective_completed: false,
            leader_slot: None,
            ped_type: None,
            vehicle_bomb_type: match vehicle {
                crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                    Some(car.bomb_type as i32)
                }
                crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
                | crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. }
                | crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => None,
            },
            vehicle_damaged: base.appearance.is_damaged,
            last_weapon_damage: i32::from(base.vitality.last_weapon_damage),
            vehicle_num_passengers: passenger_handles.iter().flatten().count() as u8,
            vehicle_num_max_passengers: base.entry.num_max_passengers,
            vehicle_passenger_handles: passenger_handles,
            vehicle_closed_doors: closed_doors,
            can_see_player: false,
        })
    }

    pub fn build_script_object(
        &mut self,
        pools: &mut CPools,
        handle: Option<i32>,
        model_id: i32,
        mut position: rust_gta_3_core::core::Coordinates::CVector,
        model_base_offset: f32,
        general: &mut CGeneral,
        file_loader: &CFileLoader,
    ) -> Option<ScriptObjectEntity> {
        let handle = match handle {
            Some(h) => {
                let _ = pools
                    .object_pool
                    .new_at(CPoolHandle(h), CPoolEntry::new(CPoolEntryKind::Object));
                h
            }
            None => pools
                .object_pool
                .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
                .map(|entry| entry.0)?,
        };
        let mut object = CObject::new_with_model(model_id, false, general);
        object.initialize_world_state(file_loader);
        object.object_created_by = MISSION_OBJECT;
        position.z += model_base_offset;
        object.physical.entity.set_position(position);
        object.physical.entity.matrix.reset_orientation();
        object.object_matrix = object.physical.entity.matrix;
        // re3 COMMAND_CREATE_OBJECT (Script.cpp:2681): ClearSpaceForMissionEntity
        // runs before CWorld::Add.
        self.clear_space_for_mission_entity(pools, position, model_id.max(0) as usize, file_loader);
        Some(ScriptObjectEntity { handle, object })
    }

    /// re3 `CTheScripts::ClearSpaceForMissionEntity` (Script5.cpp:2167-2231).
    /// Mission spawns never stack: whatever actually collides with the new
    /// entity's COL at the spawn point is deleted. Locked vehicles, vehicles
    /// with getting-in/out activity, mission entities and player occupants are
    /// protected exactly like re3 (`bIsLocked`/`CanBeDeleted` rules).
    pub fn clear_space_for_mission_entity(
        &mut self,
        pools: &mut CPools,
        position: CVector,
        model_index: usize,
        file_loader: &CFileLoader,
    ) {
        self.clear_space_for_mission_entity_except(pools, position, model_index, file_loader, None);
    }

    /// re3 ClearSpaceForMissionEntity with the already-worlded entity omitted
    /// from its collision candidates, as required by COMMAND_SET_CAR_COORDINATES.
    pub fn clear_space_for_mission_entity_except(
        &mut self,
        pools: &mut CPools,
        position: CVector,
        model_index: usize,
        file_loader: &CFileLoader,
        excluded_vehicle: Option<i32>,
    ) {
        let Some(new_col) = file_loader.model_info.col_model(model_index) else {
            return;
        };
        let radius = new_col.bounding_sphere.radius;
        let mut new_matrix = CMatrix::unity();
        new_matrix.position = position;

        // FindObjectsKindaColliding broad phase: 2D bound-sphere overlap for
        // vehicles and peds. Peds in vehicles are excluded up front like re3.
        let mut vehicle_candidates = Vec::new();
        for (&handle, vehicle) in self.vehicles.iter() {
            if Some(handle) == excluded_vehicle {
                continue;
            }
            let base = vehicle.base();
            let centre = base.physical.entity.get_position();
            let candidate_radius = file_loader
                .model_info
                .col_model(base.physical.entity.model_index.max(0) as usize)
                .map_or(0.0, |col| col.bounding_sphere.radius);
            let dx = centre.x - position.x;
            let dy = centre.y - position.y;
            if dx * dx + dy * dy < (radius + candidate_radius).powi(2) {
                vehicle_candidates.push(handle);
            }
        }
        let mut ped_candidates = Vec::new();
        for (&handle, ped) in self.peds.iter() {
            if self.relation_for_char(handle).is_some() {
                continue;
            }
            let centre = ped.ped.physical.entity.get_position();
            let candidate_radius = file_loader
                .model_info
                .col_model(ped.ped.physical.entity.model_index.max(0) as usize)
                .map_or(0.5, |col| col.bounding_sphere.radius);
            let dx = centre.x - position.x;
            let dy = centre.y - position.y;
            if dx * dx + dy * dy < (radius + candidate_radius).powi(2) {
                ped_candidates.push(handle);
            }
        }

        // Precise phase: re3 runs CCollision::ProcessColModels between the new
        // entity and each candidate before deleting anything.
        let collides = |entity: &crate::entities::Entity::CEntity| -> bool {
            let Some(candidate_col) = file_loader
                .model_info
                .col_model(entity.model_index.max(0) as usize)
            else {
                // Without a COL model re3 could not have spawned the entity at
                // all; treat the broad-phase overlap as the collision result.
                return true;
            };
            CCollision::test_col_models_spheres(&new_matrix, new_col, &entity.matrix, candidate_col)
        };

        for handle in vehicle_candidates {
            let Some(vehicle) = self.vehicles.get(&self.index, &handle) else {
                continue;
            };
            let base = vehicle.base();
            if base.flags.is_locked || !collides(&base.physical.entity) {
                continue;
            }
            // re3 `CVehicle::CanBeDeleted` (getting-in/out activity, mission
            // chars and player occupants block deletion; only RANDOM/PARKED
            // vehicles are deletable) — the shared script-deletion helper.
            if !script_vehicle_base_can_be_deleted(self, handle, &base) {
                continue;
            }
            // re3 prints exactly this case; it is how stacked mission spawns
            // are audited in the original.
            bevy::log::debug!(
                "ClearSpaceForMissionEntity: deleting vehicle handle={} at ({:.1},{:.1},{:.1})",
                handle,
                position.x,
                position.y,
                position.z
            );
            // re3 removes every occupant via CPopulation::RemovePed first.
            destroy_script_vehicle_and_occupants(self, pools, handle);
        }

        for handle in ped_candidates {
            let Some(ped) = self.peds.get(&self.index, &handle) else {
                continue;
            };
            // re3 CPed::CanBeDeleted: mission chars and in-vehicle peds stay.
            if ped.ped.char_created_by == MISSION_CHAR || !collides(&ped.ped.physical.entity) {
                continue;
            }
            self.remove_char(pools, handle);
        }
    }

    pub fn insert_script_char(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCharRequest,
        general: &mut CGeneral,
        ped_types: Option<&CPedTypeRuntime>,
        file_loader: &CFileLoader,
    ) -> Option<i32> {
        self.insert_script_char_entity(pools, request, general, ped_types, file_loader, None)
            .map(|entity| entity.handle)
    }

    pub(crate) fn insert_script_char_entity(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCharRequest,
        general: &mut CGeneral,
        ped_types: Option<&CPedTypeRuntime>,
        file_loader: &CFileLoader,
        pending_vehicle: Option<&mut ScriptVehicleEntity>,
    ) -> Option<ScriptPedEntity> {
        self.insert_script_char_entity_with(
            pools, request, general, ped_types, file_loader, pending_vehicle, true, |_, _| {},
        )
    }

    /// Configure an ambient actor before its deferred Bevy spawn. The caller
    /// owns the same CPed/locomotion/animation data that the Entity receives;
    /// a newly allocated Entity is not yet queryable until the apply barrier.
    pub(crate) fn insert_population_ped<F>(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCharRequest,
        general: &mut CGeneral,
        file_loader: &CFileLoader,
        initialise: F,
    ) -> Option<i32>
    where
        F: FnOnce(&mut ScriptPedEntity, &mut CGeneral),
    {
        self.insert_script_char_entity_with(
            pools, request, general, None, file_loader, None, false, initialise,
        )
        .map(|entity| entity.handle)
    }

    fn insert_script_char_entity_with<F>(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCharRequest,
        general: &mut CGeneral,
        ped_types: Option<&CPedTypeRuntime>,
        file_loader: &CFileLoader,
        mut pending_vehicle: Option<&mut ScriptVehicleEntity>,
        clear_mission_space: bool,
        initialise: F,
    ) -> Option<ScriptPedEntity>
    where
        F: FnOnce(&mut ScriptPedEntity, &mut CGeneral),
    {
        let mut position = request.position;
        if let Some(vehicle_handle) = request.vehicle_handle {
            position = if let Some(vehicle) = pending_vehicle
                .as_deref()
                .filter(|vehicle| vehicle.handle() == vehicle_handle)
            {
                vehicle.base().physical.entity.get_position()
            } else {
                self.vehicles
                    .get(&self.index, &vehicle_handle)?
                    .base()
                    .physical
                    .entity
                    .get_position()
            };
        }

        let mut ped = CPed::new(script_ped_type(request.ped_type), general);
        let mut vitality = PedVitality::default();
        if let Some(ped_types) = ped_types {
            ped.initialise_fear_flags(ped_types);
        }
        ped.physical
            .entity
            .set_model_index_no_create(request.model_id.max(0) as usize);
        if ped.ped_type == ePedType::PEDTYPE_COP {
            let cop_type =
                crate::peds::CopPed::cop_type_for_model(request.model_id.max(0) as usize);
            crate::peds::CopPed::initialise_cop_ped(&mut ped, &mut vitality, cop_type);
        }
        ped.char_created_by = MISSION_CHAR;
        ped.flags.responds_to_threats = false;
        ped.flags.allow_medics_to_revive_me = false;
        ped.physical.entity.set_position(position);
        // Script.cpp assigns every mission-created ped to the island level at
        // its spawn point. CPed::ProcessControl then stays dormant while that
        // island's collision is not resident, preventing remote mission actors
        // from walking/falling through unloaded geometry.
        ped.physical.zone_level = file_loader
            .world
            .zones
            .get_level_from_position([position.x, position.y, position.z])
            as i8;
        ped.physical.entity.set_orientation(0.0, 0.0, 0.0);
        // re3 COMMAND_CREATE_CHAR (Script.cpp:1861): ClearSpaceForMissionEntity
        // runs before CWorld::Add. Chars created inside a car skip it like re3.
        if request.vehicle_handle.is_none() && clear_mission_space {
            self.clear_space_for_mission_entity(
                pools,
                position,
                request.model_id.max(0) as usize,
                file_loader,
            );
        }
        let handle = pools
            .ped_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Ped))?
            .0;

        if let Some(vehicle_handle) = request.vehicle_handle {
            let pending_matches = pending_vehicle
                .as_deref()
                .is_some_and(|vehicle| vehicle.handle() == vehicle_handle);
            let mut vehicle = if pending_matches {
                pending_vehicle.as_deref_mut()?.as_mut()
            } else {
                self.vehicles.get_mut(&self.index, &vehicle_handle)?
            };
            let is_boat = matches!(
                vehicle,
                crate::control::ScriptEntities::ScriptVehicleMut::Boat { .. }
            );
            let mut base = vehicle.base_mut();
            // Both CVehicle::SetUpDriver/SetupPassenger and
            // COMMAND_CREATE_CHAR_AS_PASSENGER put ordinary vehicle
            // occupants in PED_DRIVING. Seat ownership lives in InVehicle;
            // PED_PASSENGER is not the generic GTA III car-passenger state.
            ped.ped_state = PedState::PED_DRIVING;
            ped.physical.entity.flags.uses_collision = false;
            base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PHYSICS);
            // Occupancy is represented solely by the ped's `InVehicle`
            // component below.  Vehicle-side driver/passenger slots are not
            // maintained as a second source of truth.
            if request.passenger_seat.is_none() {
                if !is_boat {
                    base.auto_pilot.car_mission =
                        crate::control::AutoPilot::eCarMission::MISSION_CRUISE;
                }
                base.drive.engine_on = true;
            }
            if pending_matches {
                let bevy_entity = self.index.vehicle(vehicle_handle)?;
                drop(vehicle);
                pending_vehicle
                    .as_deref()?
                    .clone()
                    .insert_into(&mut self.commands.entity(bevy_entity));
            }
        }

        let mut entity = ScriptPedEntity {
            vitality,
            locomotion: PedLocomotion::default(),
            animation: crate::peds::PedAnimation::PedAnimation::default(),
            handle,
            ped,
            model_id: request.model_id,
        };
        initialise(&mut entity, general);
        let pending = entity.clone();
        let handle = entity.handle;
        self.spawn_ped(entity);
        // The ped-owned ECS relation is installed at the same creation
        // boundary as the native occupant link.  The vehicle component is
        // never given a reverse authoritative occupant field by this path.
        if let Some(vehicle_handle) = request.vehicle_handle
            && let (Some(ped_entity), Some(vehicle_entity)) =
                (self.index.ped(handle), self.index.vehicle(vehicle_handle))
        {
            let seat = request
                .passenger_seat
                .and_then(|seat| u8::try_from(seat).ok())
                .map_or(VehicleSeat::Driver, VehicleSeat::Passenger);
            let _ = self.set_relation_for_ped_entity(
                ped_entity,
                InVehicle {
                    vehicle: vehicle_entity,
                    seat,
                },
            );
        }
        Some(pending)
    }

    pub fn insert_script_car(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCarRequest,
        model_info: Option<&ModelInfoSlot>,
        handling_manager: Option<&cHandlingDataMgr>,
        general: &mut CGeneral,
        file_loader: &CFileLoader,
        all_taxi_lights: bool,
    ) -> Option<i32> {
        self.insert_script_car_entity(
            pools,
            request,
            model_info,
            handling_manager,
            general,
            file_loader,
            all_taxi_lights,
        )
        .map(|entity| entity.handle())
    }

    pub(crate) fn insert_script_car_entity(
        &mut self,
        pools: &mut CPools,
        request: ScriptCreateCarRequest,
        model_info: Option<&ModelInfoSlot>,
        handling_manager: Option<&cHandlingDataMgr>,
        general: &mut CGeneral,
        file_loader: &CFileLoader,
        all_taxi_lights: bool,
    ) -> Option<ScriptVehicleEntity> {
        let Some(ModelInfoSlot::Vehicle(vehicle_model)) = model_info else {
            bevy::log::warn_once!(target: "rust_gta_3::script_vehicle_creation", model_id = request.model_id,
                "script_vehicle_missing_metadata: required original IDE vehicle record is unavailable");
            return None;
        };
        let handling_id = cHandlingDataMgr::get_handling_id(vehicle_model.handling_id.as_str());
        let Some(handling) = handling_manager
            .and_then(|manager| handling_id.and_then(|id| manager.automobile_snapshot(id))) else {
            bevy::log::warn_once!(target: "rust_gta_3::script_vehicle_creation", model_id = request.model_id,
                "script_vehicle_missing_handling: required original HANDLING.CFG record is unavailable");
            return None;
        };
        let Some(handle) = pools.vehicle_pool.new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle)) else {
            bevy::log::warn_once!(target: "rust_gta_3::script_vehicle_creation", "script_vehicle_pool_exhausted");
            return None;
        };
        let handle = handle.0;
        let colours = if vehicle_model.num_colours > 0 {
            let index = general.get_random_number() as usize % vehicle_model.num_colours;
            Some((vehicle_model.colours1[index], vehicle_model.colours2[index]))
        } else {
            None
        };
        let vehicle_type = vehicle_model.vehicle_type;
        let mut position = request.position;

        let entity = if vehicle_type == eVehicleType::VEHICLE_TYPE_BOAT as i32 {
            position.z += script_vehicle_base_z(model_info);
            let mut boat = CBoat::new(
                request.model_id,
                MISSION_VEHICLE,
                request.time_ms,
                general,
                Some(crate::vehicles::Boat::CBoatHandlingSnapshot {
                    mass: handling.mass,
                    turn_mass: handling.turn_mass,
                    centre_of_mass: handling.centre_of_mass,
                    dimension: handling.dimension,
                    buoyancy: handling.buoyancy,
                    handling_id: handling.handling_id,
                }),
                colours,
            );
            boat.vehicle.core.physical.entity.set_position(position);
            boat.vehicle
                .set_status(crate::entities::Entity::eEntityStatus::STATUS_ABANDONED);
            boat.vehicle.core.flags.is_locked = true;
            boat.vehicle.core.auto_pilot.car_mission =
                crate::control::AutoPilot::eCarMission::MISSION_NONE;
            boat.vehicle.core.auto_pilot.temp_action =
                crate::control::AutoPilot::eCarTempAction::TEMPACT_NONE;
            boat.vehicle.core.auto_pilot.cruise_speed = 20;
            boat.vehicle.core.auto_pilot.max_traffic_speed = 20.0;
            ScriptVehicleEntity::Boat { handle, boat }
        } else {
            let handling_data = handling_manager
                .and_then(|mgr| handling_id.and_then(|id| mgr.get_handling_data(id)))
                .copied();
            let mut car = CAutomobile::new(
                request.model_id,
                MISSION_VEHICLE,
                request.time_ms,
                general,
                Some(handling),
                colours,
                all_taxi_lights,
            );
            car.vehicle.appearance.extras =
                choose_vehicle_optional_components(model_info, general, false);
            if let (Some(ModelInfoSlot::Vehicle(vehicle_model)), Some(handling_data)) =
                (model_info, handling_data)

            {
                car.setup_suspension_from_model(vehicle_model, &handling_data);
            }
            position.z += script_automobile_base_z(model_info, &car);
            car.vehicle.core.physical.entity.set_position(position);
            // COMMAND_CREATE_CAR stores the mission car's island just after
            // CWorld::Add. Keep that lifetime information on the ECS physical
            // instead of leaving all script cars at LEVEL_GENERIC.
            car.vehicle.core.physical.zone_level = file_loader
                .world
                .zones
                .get_level_from_position([position.x, position.y, position.z])
                as i8;
            car.vehicle
                .set_status(crate::entities::Entity::eEntityStatus::STATUS_ABANDONED);
            car.vehicle.core.flags.is_locked = true;
            // re3 COMMAND_CREATE_CAR calls JoinCarWithRoadSystem immediately,
            // even while the car's mission remains MISSION_NONE. This seeds a
            // valid road/lane history for later SCM objectives and teleports.
            join_car_with_road_system(&file_loader.path_find, &mut car.vehicle, general);
            car.vehicle.core.auto_pilot.car_mission =
                crate::control::AutoPilot::eCarMission::MISSION_NONE;
            car.vehicle.core.auto_pilot.temp_action =
                crate::control::AutoPilot::eCarTempAction::TEMPACT_NONE;
            car.vehicle.core.auto_pilot.driving_style =
                crate::control::AutoPilot::eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS;
            car.vehicle.core.auto_pilot.cruise_speed = 9;
            car.vehicle.core.auto_pilot.max_traffic_speed = 9.0;
            car.vehicle.core.auto_pilot.current_lane = 0;
            car.vehicle.core.auto_pilot.next_lane = 0;
            car.vehicle.drive.engine_on = false;
            car.vehicle.core.flags.has_been_owned_by_player = true;
            ScriptVehicleEntity::Automobile { handle, car }
        };
        // re3 COMMAND_CREATE_CAR (Script.cpp:2077/2095):
        // ClearSpaceForMissionEntity runs before CWorld::Add so scripted
        // spawns can never stack. CCarCtrl::GenerateOneRandomCar uses the
        // separate FindObjectsKindaColliding gate and simply aborts instead
        // of deleting an existing RANDOM_VEHICLE; callers select that path
        // explicitly through the request flag.
        if request.clear_space_for_mission_entity {
            self.clear_space_for_mission_entity(
                pools,
                position,
                request.model_id.max(0) as usize,
                file_loader,
            );
        }
        bevy::log::debug!(
            "SCM CREATE_CAR model={} pos=({:.2},{:.2},{:.2}) handle={}",
            request.model_id,
            position.x,
            position.y,
            position.z,
            handle
        );
        let pending = entity.clone();
        self.spawn_vehicle(entity);
        Some(pending)
    }

    /// re3 `CHeli::GenerateHeli(true)` Catalina branch.
    pub fn insert_catalina_heli(
        &mut self,
        pools: &mut CPools,
        time_ms: u32,
        general: &mut CGeneral,
    ) -> Option<i32> {
        use crate::{modelinfo::ModelIndices::MI_ESCAPE, vehicles::Vehicle::PERMANENT_VEHICLE};

        let handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?
            .0;
        let mut heli = CHeli::new(MI_ESCAPE as i32, PERMANENT_VEHICLE, time_ms, general);
        heli.vehicle
            .core
            .physical
            .entity
            .set_position(CVector::new(-224.0, 201.0, 83.0));
        heli.vehicle
            .core
            .physical
            .entity
            .set_heading(270.0_f32.to_radians());
        heli.vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
        heli.vehicle.core.flags.is_locked = true;
        heli.heli_id = self
            .vehicles
            .values()
            .filter_map(|vehicle| match vehicle {
                crate::control::ScriptEntities::ScriptVehicleRef::Heli { heli, .. } => {
                    Some(heli.heli_id)
                }
                _ => None,
            })
            .fold(0_i8, |candidate, used| {
                if used == candidate {
                    candidate.saturating_add(1)
                } else {
                    candidate
                }
            });
        heli.heli_type = HELI_TYPE_CATALINA;
        self.spawn_vehicle(ScriptVehicleEntity::Heli { handle, heli });
        Some(handle)
    }

    /// re3 `CPlane::InitPlanes` / mission Cesna construction path.
    pub fn insert_plane(
        &mut self,
        pools: &mut CPools,
        model_id: usize,
        plane_id: i16,
        time_ms: u32,
        general: &mut CGeneral,
        drug_run: bool,
        drop_off: bool,
    ) -> Option<i32> {
        let handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?
            .0;
        let mut plane = CPlane::new(model_id as i32, PERMANENT_VEHICLE, time_ms, general);
        plane
            .vehicle
            .core
            .physical
            .entity
            .set_position(CVector::ZERO);
        plane.vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
        plane.vehicle.core.flags.is_locked = true;
        plane.plane_id = plane_id;
        plane.cur_path_node = 0;
        plane.is_drug_run_cesna = drug_run;
        plane.is_drop_off_cesna = drop_off;
        self.spawn_vehicle(ScriptVehicleEntity::Plane { handle, plane });
        Some(handle)
    }

    /// re3 `CCarGenerator::DoInternalProcessing` parked automobile path (partial).
    pub fn insert_parked_car_from_generator(
        &mut self,
        pools: &mut CPools,
        model_id: i32,
        mut position: rust_gta_3_core::core::Coordinates::CVector,
        angle_degrees: f32,
        time_ms: u32,
        model_info: Option<&ModelInfoSlot>,
        handling_manager: Option<&cHandlingDataMgr>,
        general: &mut CGeneral,
        ground_z: Option<f32>,
        all_taxi_lights: bool,
    ) -> Option<i32> {
        use crate::vehicles::Vehicle::PARKED_VEHICLE;
        if let Some(z) = ground_z {
            position.z = z;
        }
        let handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?
            .0;
        let (handling_name, colours) = match model_info {
            Some(ModelInfoSlot::Vehicle(vehicle)) => {
                let colours = if vehicle.num_colours > 0 {
                    let idx = (general.get_random_number() as usize) % vehicle.num_colours;
                    Some((vehicle.colours1[idx], vehicle.colours2[idx]))
                } else {
                    None
                };
                (vehicle.handling_id.as_str(), colours)
            }
            _ => ("", None),
        };
        let handling_id = cHandlingDataMgr::get_handling_id(handling_name);
        let handling =
            handling_manager.and_then(|mgr| handling_id.and_then(|id| mgr.automobile_snapshot(id)));
        let handling_data = handling_manager
            .and_then(|mgr| handling_id.and_then(|id| mgr.get_handling_data(id)))
            .copied();
        let mut car = CAutomobile::new(
            model_id,
            PARKED_VEHICLE,
            time_ms,
            general,
            handling,
            colours,
            all_taxi_lights,
        );
        car.vehicle.appearance.extras = choose_vehicle_optional_components(model_info, general, false);
        if let (Some(ModelInfoSlot::Vehicle(vehicle_model)), Some(handling_data)) =
            (model_info, handling_data)

        {
            car.setup_suspension_from_model(vehicle_model, &handling_data);
        }
        position.z += script_automobile_base_z(model_info, &car);
        car.vehicle.core.physical.entity.set_position(position);
        car.vehicle
            .core
            .physical
            .entity
            .set_heading(angle_degrees.to_radians());
        car.vehicle
            .set_status(crate::entities::Entity::eEntityStatus::STATUS_ABANDONED);
        car.vehicle.drive.engine_on = false;
        car.vehicle.core.flags.is_locked = false;
        car.vehicle.core.auto_pilot.car_mission =
            crate::control::AutoPilot::eCarMission::MISSION_NONE;
        car.vehicle.core.auto_pilot.cruise_speed = 0;
        self.spawn_vehicle(ScriptVehicleEntity::Automobile { handle, car });
        Some(handle)
    }

    /// re3 `COMMAND_CREATE_RANDOM_CAR_FOR_CAR_PARK` automobile constructor.
    pub fn insert_random_car_for_car_park(
        &mut self,
        pools: &mut CPools,
        model_id: i32,
        mut position: CVector,
        heading_degrees: f32,
        time_ms: u32,
        model_info: Option<&ModelInfoSlot>,
        handling_manager: Option<&cHandlingDataMgr>,
        paths: &mut CPathFind,
        zone_level: i8,
        general: &mut CGeneral,
        all_taxi_lights: bool,
    ) -> Option<i32> {
        let ModelInfoSlot::Vehicle(vehicle_model) = model_info? else {
            return None;
        };
        if vehicle_model.vehicle_type != eVehicleType::VEHICLE_TYPE_CAR as i32 {
            return None;
        }
        let handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?
            .0;
        let colours = if vehicle_model.num_colours > 0 {
            let index = (general.get_random_number() as usize) % vehicle_model.num_colours;
            Some((vehicle_model.colours1[index], vehicle_model.colours2[index]))
        } else {
            None
        };
        let handling_id = cHandlingDataMgr::get_handling_id(vehicle_model.handling_id.as_str());
        let handling = handling_manager
            .and_then(|manager| handling_id.and_then(|id| manager.automobile_snapshot(id)));
        let handling_data = handling_manager
            .and_then(|manager| handling_id.and_then(|id| manager.get_handling_data(id)))
            .copied();
        let mut car = CAutomobile::new(
            model_id,
            RANDOM_VEHICLE,
            time_ms,
            general,
            handling,
            colours,
            all_taxi_lights,
        );
        car.vehicle.appearance.extras = choose_vehicle_optional_components(model_info, general, false);
        if let Some(handling_data) = handling_data {
            car.setup_suspension_from_model(vehicle_model, &handling_data);
        }
        position.z += script_automobile_base_z(model_info, &car);
        car.vehicle.core.physical.entity.set_position(position);
        car.vehicle
            .core
            .physical
            .entity
            .set_heading(heading_degrees.to_radians());
        car.vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
        car.vehicle.core.flags.is_locked = false;
        car.vehicle.core.flags.is_car_park_vehicle = true;
        join_car_with_road_system(paths, &mut car.vehicle, general);
        car.vehicle.core.auto_pilot.car_mission = eCarMission::MISSION_NONE;
        car.vehicle.core.auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
        car.vehicle.core.auto_pilot.driving_style = eCarDrivingStyle::DRIVINGSTYLE_STOP_FOR_CARS;
        car.vehicle.core.auto_pilot.cruise_speed = 9;
        car.vehicle.core.auto_pilot.max_traffic_speed = 9.0;
        car.vehicle.core.auto_pilot.current_lane = 0;
        car.vehicle.core.auto_pilot.next_lane = 0;
        car.vehicle.drive.engine_on = false;
        car.vehicle.core.physical.zone_level = zone_level;
        self.spawn_vehicle(ScriptVehicleEntity::Automobile { handle, car });
        Some(handle)
    }

    /// re3 `CStoredCar::RestoreCar`. The stored Z is already the vehicle's
    /// world-space origin, so unlike script/car-generator creation no model
    /// base offset is applied a second time.
    pub fn restore_stored_car(
        &mut self,
        pools: &mut CPools,
        stored: CStoredCar,
        time_ms: u32,
        model_info: Option<&ModelInfoSlot>,
        handling_manager: Option<&cHandlingDataMgr>,
        general: &mut CGeneral,
        player_ped_slot: Option<usize>,
        all_taxi_lights: bool,
    ) -> Option<i32> {
        let ModelInfoSlot::Vehicle(model) = model_info? else {
            return None;
        };
        let handle = pools
            .vehicle_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))?
            .0;
        let handling_id = cHandlingDataMgr::get_handling_id(&model.handling_id);
        let handling =
            handling_manager.and_then(|mgr| handling_id.and_then(|id| mgr.automobile_snapshot(id)));
        let colours = Some((stored.primary_color as u8, stored.secondary_color as u8));

        let mut entity = if model.vehicle_type == eVehicleType::VEHICLE_TYPE_BOAT as i32 {
            let boat = CBoat::new(
                stored.model_index,
                RANDOM_VEHICLE,
                time_ms,
                general,
                handling.map(|snapshot| crate::vehicles::Boat::CBoatHandlingSnapshot {
                    mass: snapshot.mass,
                    turn_mass: snapshot.turn_mass,
                    centre_of_mass: snapshot.centre_of_mass,
                    dimension: snapshot.dimension,
                    buoyancy: snapshot.buoyancy,
                    handling_id: snapshot.handling_id,
                }),
                colours,
            );
            ScriptVehicleEntity::Boat { handle, boat }
        } else {
            let mut car = CAutomobile::new(
                stored.model_index,
                RANDOM_VEHICLE,
                time_ms,
                general,
                handling,
                colours,
                all_taxi_lights,
            );
            car.bomb_type = script_bomb_type(i32::from(stored.car_bomb_type));
            if car.bomb_type != crate::vehicles::Automobile::eBombType::CARBOMB_NONE {
                car.bomb_rigger = player_ped_slot;
            }
            ScriptVehicleEntity::Automobile { handle, car }
        };

        let base = entity.base_mut();
        base.physical.entity.set_position(stored.position);
        base.physical.entity.matrix.forward = stored.angle;
        base.physical.entity.matrix.right = CVector::new(stored.angle.y, -stored.angle.x, 0.0);
        base.physical.entity.matrix.up = CVector::new(0.0, 0.0, 1.0);
        base.set_status(eEntityStatus::STATUS_ABANDONED);
        base.appearance.current_colour1 = stored.primary_color as u8;
        base.appearance.current_colour2 = stored.secondary_color as u8;
        base.audio.radio_station = stored.radio_station as u8;
        base.appearance.extras = [stored.variation_a, stored.variation_b];
        base.flags.freebies = false;
        base.flags.has_been_owned_by_player = true;
        base.flags.is_locked = false;
        base.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;
        let flags = &mut base.physical.entity.flags;
        flags.bullet_proof = stored.flags & CStoredCar::FLAG_BULLETPROOF != 0;
        flags.fire_proof = stored.flags & CStoredCar::FLAG_FIREPROOF != 0;
        flags.explosion_proof = stored.flags & CStoredCar::FLAG_EXPLOSIONPROOF != 0;
        flags.collision_proof = stored.flags & CStoredCar::FLAG_COLLISIONPROOF != 0;
        flags.melee_proof = stored.flags & CStoredCar::FLAG_MELEEPROOF != 0;

        self.spawn_vehicle(entity);
        Some(handle)
    }

    pub fn remove_char(&mut self, pools: &mut CPools, handle: i32) {
        if self.index.ped(handle).is_some() {
            for mut ped in self.peds.values_mut() {
                crate::peds::PedNearby::remove_near_ped_reference(&mut ped.ped, handle);
            }
        }
        retire_script_ped(&mut self.commands, &mut self.index, pools, handle);
    }

    pub fn remove_car(&mut self, pools: &mut CPools, handle: i32) {
        self.despawn_vehicle(handle);
        let pool_handle = CPoolHandle(handle);
        if pools.vehicle_pool.get_at(pool_handle).is_some() {
            pools.vehicle_pool.delete_by_slot(pool_handle.index());
        }
    }

    pub fn warp_char_into_car(&mut self, char_handle: i32, vehicle_handle: i32) -> bool {
        let Some(position) = self
            .vehicles
            .get(&self.index, &vehicle_handle)
            .map(|vehicle| vehicle.base().physical.entity.get_position())
        else {
            return false;
        };
        if !self.set_char_relation(char_handle, vehicle_handle, VehicleSeat::Driver) {
            return false;
        }
        let Some(ped_entity) = self.peds.get_mut(&self.index, &char_handle) else {
            return false;
        };
        ped_entity.ped.physical.entity.set_position(position);
        ped_entity.ped.ped_state = PedState::PED_DRIVING;
        ped_entity.ped.physical.entity.flags.uses_collision = false;
        let Some(mut vehicle) = self.vehicles.get_mut(&self.index, &vehicle_handle) else {
            return false;
        };
        let is_boat = matches!(
            vehicle,
            crate::control::ScriptEntities::ScriptVehicleMut::Boat { .. }
        );
        let mut base = vehicle.base_mut();
        base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PHYSICS);
        if !is_boat {
            base.auto_pilot.car_mission = crate::control::AutoPilot::eCarMission::MISSION_CRUISE;
        }
        base.drive.engine_on = true;
        true
    }

    /// Remove the authoritative ped-owned occupant relation and apply the
    /// vehicle-side state transition that re3's `RemoveDriver` produced. The
    /// relationship itself is never resolved through CPools integer slots.
    pub(crate) fn detach_ped_from_vehicle(&mut self, ped: Entity) -> Option<InVehicle> {
        let relation = self.relation_for_ped_entity(ped)?;
        queue_clear_vehicle_relation(&mut self.commands, ped);
        if relation.seat == VehicleSeat::Driver
            && let Some(vehicle_handle) = self.vehicle_handle_for_entity(relation.vehicle)
            && let Some(mut vehicle) = self.vehicles.get_mut(&self.index, &vehicle_handle)
        {
            let mut base = vehicle.base_mut();
            base.set_status(crate::entities::Entity::eEntityStatus::STATUS_ABANDONED);
            base.drive.engine_on = false;
            base.auto_pilot.cruise_speed = 0;
        }
        Some(relation)
    }

    pub(crate) fn place_player_in_car(
        &mut self,
        player_entity: Entity,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        vehicle_handle: i32,
        seat: VehicleSeat,
        file_loader: &CFileLoader,
    ) -> bool {
        self.place_player_in_car_replacing_occupant(
            player_entity,
            player,
            vehicle_handle,
            seat,
            file_loader,
            None,
        )
    }

    /// re3 `CVehicle::SetDriver` can replace the driver pointer while the old
    /// driver's dragged-out animation is still alive. The caller supplies that
    /// exact old occupant; ordinary placement keeps the strict seat check.
    pub(crate) fn place_player_in_car_replacing_occupant(
        &mut self,
        player_entity: Entity,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        vehicle_handle: i32,
        seat: VehicleSeat,
        file_loader: &CFileLoader,
        replaced_occupant: Option<Entity>,
    ) -> bool {
        let Some(vehicle_entity) = self.vehicle_entity(vehicle_handle) else {
            return false;
        };
        let passenger_seat = player_seat_index(seat);
        let Some(position) = self
            .vehicles
            .get(&self.index, &vehicle_handle)
            .map(|vehicle| {
                let is_boat = matches!(
                    vehicle,
                    crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
                );
                // re3 `CPed::SetPedPositionInCar` (Ped.cpp:6991-7011): the ped rides at
                // the authored seat dummy, not the vehicle origin.
                ped_seat_world_position(&vehicle.base(), passenger_seat, is_boat, file_loader)
            })
        else {
            return false;
        };
        let relation = InVehicle {
            vehicle: vehicle_entity,
            seat,
        };
        let relation_set = if let Some(replaced_occupant) = replaced_occupant {
            queue_replace_vehicle_relation(
                &mut self.commands,
                &self.relations,
                player_entity,
                relation,
                replaced_occupant,
            )
        } else {
            self.set_relation_for_ped_entity(player_entity, relation)
        };
        if !relation_set {
            return false;
        }
        if self
            .vehicles
            .get(&self.index, &vehicle_handle)
            .is_some_and(|vehicle| {
                vehicle.base().vehicle_created_by != crate::vehicles::Vehicle::MISSION_VEHICLE
            })
        {
            crate::control::CarCtrl::queue_register_vehicle_of_interest(
                &mut self.commands,
                vehicle_entity,
            );
        }
        player.set_position(position);
        // re3 uses PED_DRIVING for ordinary occupants regardless of whether
        // the ECS seat relation is driver or passenger. PED_PASSENGER is a
        // separate special-purpose state, not the generic car passenger state.
        player.ped.ped_state = PedState::PED_DRIVING;
        player.ped.physical.entity.flags.uses_collision = false;
        player.ped.physical.move_speed = CVector::ZERO;
        player.move_speed = 0.0;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_standing = true;
        true
    }

    pub fn warp_player_into_car(
        &mut self,
        player_entity: Entity,
        player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
        vehicle_handle: i32,
        seat: VehicleSeat,
        file_loader: &CFileLoader,
    ) -> bool {
        if !self.place_player_in_car(player_entity, player, vehicle_handle, seat, file_loader) {
            return false;
        }
        let is_boat = self
            .vehicles
            .get(&self.index, &vehicle_handle)
            .is_some_and(|vehicle| {
                matches!(
                    vehicle,
                    crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
                )
            });
        let Some(mut vehicle) = self.vehicles.get_mut(&self.index, &vehicle_handle) else {
            return false;
        };
        let mut base = vehicle.base_mut();
        if seat == VehicleSeat::Driver {
            base.set_status(crate::entities::Entity::eEntityStatus::STATUS_PLAYER);
        }
        if seat == VehicleSeat::Driver && !is_boat {
            base.auto_pilot.car_mission = crate::control::AutoPilot::eCarMission::MISSION_NONE;
        }
        base.drive.engine_on = true;
        base.flags.has_been_owned_by_player = true;
        true
    }
}
impl<T> Default for SamePassOverlay<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> SamePassOverlay<T> {
    pub(crate) fn get(&self, handle: &i32) -> Option<&T> {
        self.entries
            .iter()
            .find_map(|(candidate, value)| (candidate == handle).then_some(value))
    }

    pub(crate) fn get_mut(&mut self, handle: &i32) -> Option<&mut T> {
        self.entries
            .iter_mut()
            .find_map(|(candidate, value)| (candidate == handle).then_some(value))
    }

    pub(crate) fn insert(&mut self, handle: i32, value: T) {
        if let Some(existing) = self.get_mut(&handle) {
            *existing = value;
        } else {
            self.entries.push((handle, value));
        }
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|(_, value)| value)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&i32, &T)> {
        self.entries.iter().map(|(handle, value)| (handle, value))
    }
}

// === FAZ9c: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn script_key_string(key: [u8; 8]) -> String {
    let length = key.iter().position(|&byte| byte == 0).unwrap_or(key.len());
    String::from_utf8_lossy(&key[..length]).into_owned()
}

// FAZ8c-darkel: moved to control/Darkel.rs verbatim (2 items): fn apply_script_darkel_effects, fn update_darkel

// FAZ8c-bridge: moved to control/Bridge.rs verbatim (1 items): fn update_bridge

// FAZ9b: moved to core/Fire.rs verbatim (4 items): fn apply_script_fire_effects, fn clear_script_fire_target, fn process_script_fires, fn script_fire_vehicle_flame_position

pub(crate) fn apply_script_player_info_effects(
    mut scripts: ResMut<CTheScripts>,
    mut player_info: Option<ResMut<CPlayerInfo>>,
    mut bridge: ResMut<ScriptGameplayBridge>,
    mut world_players: CWorldPlayers,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::SetCollectableTotal(total) => {
                if let Some(info) = player_info.as_mut() {
                    info.m_nTotalPackages = total;
                } else {
                    leftover.push(ScriptCommandEffect::SetCollectableTotal(total));
                }
            }
            ScriptCommandEffect::SwitchTaxiTimer { enabled, time_ms } => {
                if let Some(info) = player_info.as_mut() {
                    if enabled {
                        info.m_nUnusedTaxiTimer = time_ms;
                        info.m_bUnusedTaxiThing = true;
                    } else {
                        info.m_bUnusedTaxiThing = false;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SwitchTaxiTimer { enabled, time_ms });
                }
            }
            ScriptCommandEffect::AddScore {
                player_index,
                amount,
            } => {
                if player_index == 0 {
                    if let Some(info) = player_info.as_mut() {
                        info.m_nMoney = info.m_nMoney.saturating_add(amount);
                    }
                } else {
                    leftover.push(ScriptCommandEffect::AddScore {
                        player_index,
                        amount,
                    });
                }
            }
            ScriptCommandEffect::SetPlayerNeverGetsTired {
                player_index,
                enabled,
            } => {
                if player_index == 0 {
                    if let Some(info) = player_info.as_mut() {
                        info.m_bInfiniteSprint = enabled;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerNeverGetsTired {
                        player_index,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetPlayerFastReload {
                player_index,
                enabled,
            } => {
                if player_index == 0 {
                    if let Some(info) = player_info.as_mut() {
                        info.m_bFastReload = enabled;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerFastReload {
                        player_index,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetPlayerVisible {
                player_index,
                visible,
            } => {
                if player_index == 0 {
                    bridge.player_visible = visible;
                    if let Some(player) = world_players.player_mut(player_index) {
                        player.ped.physical.entity.flags.is_visible = visible;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetPlayerVisible {
                        player_index,
                        visible,
                    });
                }
            }
            ScriptCommandEffect::SetGetOutOfJailFree {
                player_index,
                enabled,
            } => {
                if player_index == 0 {
                    if let Some(info) = player_info.as_mut() {
                        info.m_bGetOutOfJailFree = enabled;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetGetOutOfJailFree {
                        player_index,
                        enabled,
                    });
                }
            }
            ScriptCommandEffect::SetFreeHealthCare {
                player_index,
                enabled,
            } => {
                if player_index == 0 {
                    if let Some(info) = player_info.as_mut() {
                        info.m_bGetOutOfHospitalFree = enabled;
                    }
                } else {
                    leftover.push(ScriptCommandEffect::SetFreeHealthCare {
                        player_index,
                        enabled,
                    });
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

// FAZ8c-dmaudio: moved to audio/DMAudio.rs verbatim (1 items): fn apply_script_audio_effects

// === FAZ10e-scriptmisc: moved from core/Game.rs (verbatim, parity-preserving) ===

/// re3 `CTheScripts::IsVehicleStopped` (`control/Script5.cpp`).
#[inline]
pub(crate) fn script_vehicle_is_stopped(distance_travelled: f32, time_step: f32) -> bool {
    0.01 * time_step >= distance_travelled
}

// === FAZ8d: moved from core/Game.rs (verbatim, parity-preserving) ===
// Remaining script ECS-access stores plus script object spawn helpers.

/// SCM handle lookup only. Gameplay payloads live on Bevy entities; this
/// resource is deliberately just the script-facing handle -> Entity index.
#[derive(Debug, Clone, Default, Resource)]
pub struct ScriptEntityIndex {
    pub peds: super::ScriptIndex::ScriptHandleIndex,
    pub vehicles: super::ScriptIndex::ScriptHandleIndex,
    pub objects: super::ScriptIndex::ScriptHandleIndex,
}

/// Retire the indexed ECS identity and its matching CPools generation. Domain
/// callers handle native reference cleanup without acquiring unrelated actor
/// owners. The index changes immediately; component retirement is deferred.
pub(crate) fn retire_script_ped(
    commands: &mut Commands,
    index: &mut ScriptEntityIndex,
    pools: &mut CPools,
    handle: i32,
) -> bool {
    let retired = if let Some(entity) = index.peds.remove(&handle) {
        commands.entity(entity).despawn();
        true
    } else {
        false
    };
    let pool_handle = CPoolHandle(handle);
    if pools.ped_pool.get_at(pool_handle).is_some() {
        pools.ped_pool.delete_by_slot(pool_handle.index());
    }
    retired
}

impl ScriptEntityIndex {
    pub fn ped(&self, handle: i32) -> Option<Entity> {
        self.peds.get(&handle).copied()
    }
    pub fn vehicle(&self, handle: i32) -> Option<Entity> {
        self.vehicles.get(&handle).copied()
    }
    pub fn object(&self, handle: i32) -> Option<Entity> {
        self.objects.get(&handle).copied()
    }
    pub fn vehicle_handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.vehicles.handle_for_slot(slot)
    }
    pub fn ped_handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.peds.handle_for_slot(slot)
    }
    pub fn ped_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        self.peds.handle_for_entity(entity)
    }
    pub fn vehicle_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        self.vehicles.handle_for_entity(entity)
    }
}

#[derive(Debug, Clone)]
pub struct ScriptPedEntity {
    pub vitality: PedVitality,
    pub locomotion: PedLocomotion,
    pub animation: PedAnimation,
    pub handle: i32,
    pub ped: CPed,
    pub model_id: i32,
}

impl ScriptPedEntity {
    pub fn as_ref(&self) -> ScriptPedRef<'_> {
        ScriptPedRef {
            handle: self.handle,
            model_id: self.model_id,
            ped: &self.ped,
            vitality: &self.vitality,
            locomotion: &self.locomotion,
            animation: &self.animation,
        }
    }
    /// Consume constructor/staging data into the authoritative ECS components.
    pub fn into_bundle(self) -> impl Bundle {
        let death_lifecycle = crate::peds::PedDeath::PedDeathLifecycle::restored(&self.ped);
        (
            ScriptPedIdentity {
                handle: self.handle,
                model_id: self.model_id,
            },
            self.ped,
            self.vitality,
            self.locomotion,
            self.animation,
            death_lifecycle,
            crate::peds::PedObstacle::PedObstacleAvoidance::default(),
        )
    }
}

#[derive(Debug, Clone)]
pub enum ScriptVehicleEntity {
    Automobile { handle: i32, car: CAutomobile },
    Boat { handle: i32, boat: CBoat },
    Heli { handle: i32, heli: CHeli },
    Plane { handle: i32, plane: CPlane },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptVehicleKind {
    Automobile,
    Boat,
    Heli,
    Plane,
}
#[derive(Debug, Clone, Copy, Component)]
#[require(VehicleCollisionState)]
pub struct ScriptVehicleIdentity {
    pub handle: i32,
    pub kind: ScriptVehicleKind,
}
#[derive(Clone, Copy)]
pub enum ScriptVehicleRef<'a> {
    Automobile { handle: i32, car: AutomobileRef<'a> },
    Boat { handle: i32, boat: BoatRef<'a> },
    Heli { handle: i32, heli: HeliRef<'a> },
    Plane { handle: i32, plane: PlaneRef<'a> },
}
impl<'a> ScriptVehicleRef<'a> {
    pub(crate) fn handle(&self) -> i32 {
        match self {
            Self::Automobile { handle, .. }
            | Self::Boat { handle, .. }
            | Self::Heli { handle, .. }
            | Self::Plane { handle, .. } => *handle,
        }
    }
    pub(crate) fn base(&self) -> VehicleRef<'a> {
        match self {
            Self::Automobile { car, .. } => car.vehicle,
            Self::Boat { boat, .. } => boat.vehicle,
            Self::Heli { heli, .. } => heli.vehicle,
            Self::Plane { plane, .. } => plane.vehicle,
        }
    }
}
pub enum ScriptVehicleMut<'a> {
    Automobile { handle: i32, car: AutomobileMut<'a> },
    Boat { handle: i32, boat: BoatMut<'a> },
    Heli { handle: i32, heli: HeliMut<'a> },
    Plane { handle: i32, plane: PlaneMut<'a> },
}
impl ScriptVehicleMut<'_> {
    pub(crate) fn base(&self) -> VehicleRef<'_> {
        match self {
            Self::Automobile { car, .. } => car.vehicle.as_ref(),
            Self::Boat { boat, .. } => boat.vehicle.as_ref(),
            Self::Heli { heli, .. } => heli.vehicle.as_ref(),
            Self::Plane { plane, .. } => plane.vehicle.as_ref(),
        }
    }
    pub(crate) fn base_mut(&mut self) -> VehicleMut<'_> {
        match self {
            Self::Automobile { car, .. } => car.vehicle.as_mut(),
            Self::Boat { boat, .. } => boat.vehicle.as_mut(),
            Self::Heli { heli, .. } => heli.vehicle.as_mut(),
            Self::Plane { plane, .. } => plane.vehicle.as_mut(),
        }
    }
    pub fn as_ref(&self) -> ScriptVehicleRef<'_> {
        match self {
            Self::Automobile { handle, car } => ScriptVehicleRef::Automobile {
                handle: *handle,
                car: car.as_ref(),
            },
            Self::Boat { handle, boat } => ScriptVehicleRef::Boat {
                handle: *handle,
                boat: boat.as_ref(),
            },
            Self::Heli { handle, heli } => ScriptVehicleRef::Heli {
                handle: *handle,
                heli: heli.as_ref(),
            },
            Self::Plane { handle, plane } => ScriptVehicleRef::Plane {
                handle: *handle,
                plane: plane.as_ref(),
            },
        }
    }
    pub(crate) fn teleport(&mut self, position: CVector) {
        let mut base = self.base_mut();
        base.physical.entity.set_position(position);
        base.physical.entity.set_orientation(0.0, 0.0, 0.0);
        base.physical.move_speed = CVector::ZERO;
        base.physical.turn_speed = CVector::ZERO;
        if let Self::Automobile { car, .. } = self {
            car.reset_suspension();
        }
    }
}
impl ScriptVehicleEntity {
    pub fn as_ref(&self) -> ScriptVehicleRef<'_> {
        match self {
            Self::Automobile { handle, car } => ScriptVehicleRef::Automobile {
                handle: *handle,
                car: car.as_ref(),
            },
            Self::Boat { handle, boat } => ScriptVehicleRef::Boat {
                handle: *handle,
                boat: boat.as_ref(),
            },
            Self::Heli { handle, heli } => ScriptVehicleRef::Heli {
                handle: *handle,
                heli: heli.as_ref(),
            },
            Self::Plane { handle, plane } => ScriptVehicleRef::Plane {
                handle: *handle,
                plane: plane.as_ref(),
            },
        }
    }

    pub fn as_mut(&mut self) -> ScriptVehicleMut<'_> {
        match self {
            Self::Automobile { handle, car } => ScriptVehicleMut::Automobile {
                handle: *handle,
                car: car.as_mut(),
            },
            Self::Boat { handle, boat } => ScriptVehicleMut::Boat {
                handle: *handle,
                boat: boat.as_mut(),
            },
            Self::Heli { handle, heli } => ScriptVehicleMut::Heli {
                handle: *handle,
                heli: heli.as_mut(),
            },
            Self::Plane { handle, plane } => ScriptVehicleMut::Plane {
                handle: *handle,
                plane: plane.as_mut(),
            },
        }
    }
    pub(crate) fn insert_into(self, commands: &mut bevy::ecs::system::EntityCommands) {
        let handle = self.handle();
        let history = VehicleCollisionState {
            previous_matrix: self.base().physical.entity.matrix,
        };
        match self {
            Self::Automobile { car, .. } => {
                let (core, vitality, drive, appearance, entry, audio, state, audio_motion) = car.into_parts();
                commands.insert((
                    VehicleEntity,
                    ScriptHandle(handle),
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
                    history,
                ));
            }
            Self::Boat { boat, .. } => {
                let (core, vitality, drive, appearance, entry, audio, state) = boat.into_parts();
                commands.insert((
                    VehicleEntity,
                    ScriptHandle(handle),
                    ScriptVehicleIdentity {
                        handle,
                        kind: ScriptVehicleKind::Boat,
                    },
                    core,
                    vitality,
                    drive,
                    appearance,
                    entry,
                audio,
                    state,
                    history,
                ));
            }
            Self::Heli { heli, .. } => {
                let (core, vitality, drive, appearance, entry, audio, state) = heli.into_parts();
                commands.insert((
                    VehicleEntity,
                    ScriptHandle(handle),
                    ScriptVehicleIdentity {
                        handle,
                        kind: ScriptVehicleKind::Heli,
                    },
                    core,
                    vitality,
                    drive,
                    appearance,
                    entry,
                audio,
                    state,
                    history,
                ));
            }
            Self::Plane { plane, .. } => {
                let (core, vitality, drive, appearance, entry, audio, state) = plane.into_parts();
                commands.insert((
                    VehicleEntity,
                    ScriptHandle(handle),
                    ScriptVehicleIdentity {
                        handle,
                        kind: ScriptVehicleKind::Plane,
                    },
                    core,
                    vitality,
                    drive,
                    appearance,
                    entry,
                audio,
                    state,
                    history,
                ));
            }
        }
    }
}

/// Construction and same-pass staging data; consumed when the actor materializes.
#[derive(Debug, Clone)]
pub struct ScriptObjectEntity {
    pub handle: i32,
    pub object: CObject,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct ScriptObjectIdentity {
    pub handle: i32,
}

pub struct ScriptObjectRef<'a> {
    pub handle: i32,
    pub object: &'a CObject,
}

pub struct ScriptObjectMut<'a> {
    pub handle: i32,
    pub object: &'a mut CObject,
}

impl ScriptObjectEntity {
    pub fn into_bundle(self) -> impl Bundle {
        (
            ScriptObjectIdentity {
                handle: self.handle,
            },
            crate::objects::Object::ObjectCollisionState {
                previous_matrix: self.object.physical.entity.matrix,
            },
            self.object,
        )
    }

    pub fn as_ref(&self) -> ScriptObjectRef<'_> {
        ScriptObjectRef {
            handle: self.handle,
            object: &self.object,
        }
    }

    pub fn as_mut(&mut self) -> ScriptObjectMut<'_> {
        ScriptObjectMut {
            handle: self.handle,
            object: &mut self.object,
        }
    }
}

#[derive(SystemParam)]
pub struct ScriptObjectStore<'w, 's> {
    pub(crate) entities:
        Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static mut CObject)>,
}

impl ScriptObjectStore<'_, '_> {
    pub(crate) fn entity(&self, index: &ScriptEntityIndex, handle: i32) -> Option<Entity> {
        let entity = index.object(handle)?;
        self.entities
            .get(entity)
            .ok()
            .filter(|(_, identity, _)| identity.handle == handle)
            .map(|(entity, _, _)| entity)
    }

    pub fn get(&self, index: &ScriptEntityIndex, handle: &i32) -> Option<ScriptObjectRef<'_>> {
        self.get_entity(self.entity(index, *handle)?)
    }

    pub(crate) fn get_entity(&self, entity: Entity) -> Option<ScriptObjectRef<'_>> {
        self.entities
            .get(entity)
            .ok()
            .map(|(_, identity, object)| ScriptObjectRef {
                handle: identity.handle,
                object,
            })
    }

    pub fn get_mut(
        &mut self,
        index: &ScriptEntityIndex,
        handle: &i32,
    ) -> Option<ScriptObjectMut<'_>> {
        let entity = self.entity(index, *handle)?;
        self.get_entity_mut(entity).ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScriptObjectRef<'_>> + '_ {
        self.entities
            .iter()
            .map(|(_, identity, object)| ScriptObjectRef {
                handle: identity.handle,
                object,
            })
    }

    pub fn get_entity_mut(
        &mut self,
        entity: Entity,
    ) -> Result<ScriptObjectMut<'_>, bevy::ecs::query::QueryEntityError> {
        self.entities
            .get_mut(entity)
            .map(|(_, identity, object)| ScriptObjectMut {
                handle: identity.handle,
                object: object.into_inner(),
            })
    }

    pub fn get_many_mut(
        &mut self,
        entities: [Entity; 2],
    ) -> Result<[ScriptObjectMut<'_>; 2], bevy::ecs::query::QueryEntityError> {
        self.entities.get_many_mut(entities).map(
            |[(.., identity_a, object_a), (.., identity_b, object_b)]| {
                [
                    ScriptObjectMut {
                        handle: identity_a.handle,
                        object: object_a.into_inner(),
                    },
                    ScriptObjectMut {
                        handle: identity_b.handle,
                        object: object_b.into_inner(),
                    },
                ]
            },
        )
    }

    pub fn values(&self) -> impl Iterator<Item = ScriptObjectRef<'_>> + '_ {
        self.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = ScriptObjectMut<'_>> + '_ {
        self.entities
            .iter_mut()
            .map(|(_, identity, object)| ScriptObjectMut {
                handle: identity.handle,
                object: object.into_inner(),
            })
    }
}

/// Live SCM actors are Bevy entities. This SystemParam is only an access
/// facade over components plus the permitted handle -> Entity index; it owns
/// no parallel gameplay payloads.
#[derive(SystemParam)]
pub struct ScriptEntityWorld<'w, 's> {
    pub peds: ScriptPedStore<'w, 's>,
    pub vehicles: ScriptVehicleStore<'w, 's>,
    pub objects: ScriptObjectStore<'w, 's>,
    pub(crate) index: ResMut<'w, ScriptEntityIndex>,
    pub(crate) relations: Query<'w, 's, (Entity, Option<&'static InVehicle>), With<PedEntity>>,
    pub(crate) commands: Commands<'w, 's>,
}

// FAZ8b3: moved to control/Script.rs verbatim (see control/Script.rs).

#[cfg(test)]
pub(crate) fn script_entity_test_world() -> bevy::prelude::World {
    let mut world = bevy::prelude::World::new();
    world.init_resource::<ScriptEntityIndex>();
    world
}

#[cfg(test)]
pub(crate) fn with_script_entity_test_world<R>(
    world: &mut bevy::prelude::World,
    f: impl FnOnce(&mut ScriptEntityWorld<'_, '_>) -> R,
) -> R {
    let mut state = bevy::ecs::system::SystemState::<ScriptEntityWorld>::new(world);
    let result = {
        let mut entities = state
            .get_mut(world)
            .expect("script entity test SystemParam should validate");
        f(&mut entities)
    };
    state.apply(world);
    result
}

#[cfg(test)]
pub(crate) fn spawn_script_test_vehicle(
    world: &mut bevy::prelude::World,
    vehicle: ScriptVehicleEntity,
) -> Entity {
    let handle = vehicle.handle();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let entity = {
        let mut commands = Commands::new(&mut queue, world);
        let mut actor = commands.spawn_empty();
        let entity = actor.id();
        vehicle.insert_into(&mut actor);
        entity
    };
    queue.apply(world);
    world
        .resource_mut::<ScriptEntityIndex>()
        .vehicles
        .insert(handle, entity);
    entity
}

#[cfg(test)]
pub(crate) fn spawn_script_test_ped(
    world: &mut bevy::prelude::World,
    ped: ScriptPedEntity,
) -> Entity {
    let handle = ped.handle;
    let entity = world
        .spawn((PedEntity, ScriptHandle(handle), ped.into_bundle()))
        .id();
    world
        .resource_mut::<ScriptEntityIndex>()
        .peds
        .insert(handle, entity);
    entity
}

// FAZ8a: moved to control/Script.rs verbatim (17 items): fn script_ped_snapshot_from_entity, fn script_vehicle_snapshot_from_entity, fn script_object_snapshot, fn script_char_entity_snapshot, fn script_vehicle_entity_snapshot, fn script_object_entity_snapshot, fn script_object_handle_for_pool_slot, fn script_vehicle_base_can_be_deleted, fn set_all_occupants_of_car_leave_car, fn remove_script_vehicle_and_occupants, fn script_sound_frontend_sound, fn mission_audio_sample_from_key, fn script_char_say_sound, fn queue_ped_script_say, fn script_key_to_string, fn materialize_pickup_object, fn remove_materialized_pickup

pub(crate) fn spawn_script_object(
    commands: &mut Commands,
    index: &mut ScriptEntityIndex,
    entity: ScriptObjectEntity,
) -> i32 {
    let handle = entity.handle;
    let bevy_entity = commands
        .spawn((ScriptHandle(handle), entity.into_bundle()))
        .id();
    index.objects.insert(handle, bevy_entity);
    handle
}

pub(crate) fn remove_script_object(
    commands: &mut Commands,
    index: &mut ScriptEntityIndex,
    pools: Option<&mut CPools>,
    handle: i32,
) -> bool {
    if let Some(pools) = pools {
        let pool_handle = CPoolHandle(handle);
        if pools.object_pool.get_at(pool_handle).is_some() {
            pools.object_pool.delete_by_slot(pool_handle.index());
        }
    }
    let Some(entity) = index.objects.remove(&handle) else {
        return false;
    };
    commands.entity(entity).despawn();
    true
}

impl ScriptVehicleEntity {
    pub(crate) fn handle(&self) -> i32 {
        *self.handle_ref()
    }

    pub(crate) fn handle_ref(&self) -> &i32 {
        match self {
            ScriptVehicleEntity::Automobile { handle, .. }
            | ScriptVehicleEntity::Boat { handle, .. }
            | ScriptVehicleEntity::Heli { handle, .. }
            | ScriptVehicleEntity::Plane { handle, .. } => handle,
        }
    }

    pub(crate) fn base(&self) -> &CVehicle {
        match self {
            ScriptVehicleEntity::Automobile { car, .. } => &car.vehicle,
            ScriptVehicleEntity::Boat { boat, .. } => &boat.vehicle,
            ScriptVehicleEntity::Heli { heli, .. } => &heli.vehicle,
            ScriptVehicleEntity::Plane { plane, .. } => &plane.vehicle,
        }
    }

    pub(crate) fn base_mut(&mut self) -> &mut CVehicle {
        match self {
            ScriptVehicleEntity::Automobile { car, .. } => &mut car.vehicle,
            ScriptVehicleEntity::Boat { boat, .. } => &mut boat.vehicle,
            ScriptVehicleEntity::Heli { heli, .. } => &mut heli.vehicle,
            ScriptVehicleEntity::Plane { plane, .. } => &mut plane.vehicle,
        }
    }
}

#[cfg(test)]
mod indexed_access_tests {
    use super::*;

    #[test]
    fn object_components_share_scm_mutations_and_despawn_atomically() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(2);
        let handle = 0x302;
        let entity = world
            .spawn((
                ScriptHandle(handle),
                ScriptObjectEntity {
                    handle,
                    object: CObject::new(&mut general),
                }
                .into_bundle(),
            ))
            .id();
        world
            .resource_mut::<ScriptEntityIndex>()
            .objects
            .insert(handle, entity);
        world.get_mut::<CObject>(entity).unwrap().bonus_value = 17;
        let pose = world.get::<CObject>(entity).unwrap().physical.entity.matrix;
        assert_eq!(
            world
                .get::<crate::objects::Object::ObjectCollisionState>(entity)
                .unwrap()
                .previous_matrix,
            pose
        );
        with_script_entity_test_world(&mut world, |actors| {
            assert_eq!(actors.object_get(handle).unwrap().object.bonus_value, 17);
            actors.object_get_mut(handle).unwrap().object.bonus_value = 29;
            assert_eq!(actors.object_get(handle).unwrap().object.bonus_value, 29);
            assert!(actors.object_get(0x301).is_none());
        });
        assert_eq!(world.get::<CObject>(entity).unwrap().bonus_value, 29);
        // Corrupt/stale index mappings cannot resolve a different generation.
        world
            .get_mut::<ScriptObjectIdentity>(entity)
            .unwrap()
            .handle = 0x303;
        with_script_entity_test_world(&mut world, |actors| {
            assert!(actors.object_get(handle).is_none());
            assert!(actors.object_get_mut(handle).is_none());
            assert!(actors.object_remove(None, handle));
            assert!(actors.object_get(handle).is_none());
        });
        assert!(world.get::<CObject>(entity).is_none());
        assert!(world.get::<ScriptObjectIdentity>(entity).is_none());
        assert!(
            world
                .get::<crate::objects::Object::ObjectCollisionState>(entity)
                .is_none()
        );
    }

    #[test]
    fn vehicle_vitality_and_concrete_state_share_ecs_scm_ownership() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(9);
        let mut car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        car.vehicle.vitality.health = 850.0;
        car.vehicle.vitality.can_be_damaged = false;
        car.vehicle.drive.gas_pedal = 0.35;
        car.audio_motion.gas_pedal_audio = 0.45;
        car.audio_motion.velocity_change_for_audio = 0.12;
        car.vehicle.appearance.extras = [1, -1];
        let handle = 0x402;
        let entity =
            spawn_script_test_vehicle(&mut world, ScriptVehicleEntity::Automobile { handle, car });
        assert!(world.get::<VehicleCore>(entity).is_some());
        assert!(world.get::<AutomobileState>(entity).is_some());
        let motion = world.get::<AutomobileAudioMotion>(entity).unwrap();
        assert_eq!(motion.gas_pedal_audio, 0.45);
        assert_eq!(motion.velocity_change_for_audio, 0.12);
        let mut drive = world.get_mut::<VehicleDriveState>(entity).unwrap();
        assert_eq!(drive.gas_pedal, 0.35);
        drive.engine_on = false;
        drive.current_gear = 3;
        let mut appearance = world.get_mut::<VehicleAppearance>(entity).unwrap();
        assert_eq!(appearance.extras, [1, -1]);
        appearance.current_colour1 = 12;
        appearance.lights_on = true;
        let mut entry = world.get_mut::<VehicleEntryState>(entity).unwrap();
        entry.door_lock = eCarLock::CARLOCK_LOCKED;
        entry.getting_in_flags = 2;
        entry.num_getting_in = 1;
        world.get_mut::<VehicleAudioState>(entity).unwrap().alarm_state = 4321;
        let mut vitality = world.get_mut::<VehicleVitality>(entity).unwrap();
        assert_eq!(vitality.health, 850.0);
        assert!(!vitality.can_be_damaged);
        vitality.health = 612.0;
        vitality.car_fire = Some(3);
        with_script_entity_test_world(&mut world, |actors| {
            assert_eq!(actors.vehicle_snapshot(handle).unwrap().health, 612.0);
            let mut vehicle = actors.vehicle_get_mut(handle).unwrap();
            assert_eq!(vehicle.base().vitality.car_fire, Some(3));
            assert!(!vehicle.base().drive.engine_on);
            assert_eq!(vehicle.base().drive.current_gear, 3);
            assert_eq!(vehicle.base().appearance.current_colour1, 12);
            assert!(vehicle.base().appearance.lights_on);
            assert_eq!(vehicle.base().entry.door_lock, eCarLock::CARLOCK_LOCKED);
            assert_eq!(vehicle.base().entry.getting_in_flags, 2);
            assert_eq!(vehicle.base().entry.num_getting_in, 1);
            assert_eq!(vehicle.base().audio.alarm_state, 4321);
            vehicle.base_mut().audio.siren_or_alarm = true;
            vehicle.base_mut().vitality.health = 400.0;
            vehicle.base_mut().drive.brake_pedal = 0.75;
            vehicle.base_mut().appearance.current_colour2 = 27;
            vehicle.base_mut().entry.getting_out_flags = 4;
            if let ScriptVehicleMut::Automobile { car, .. } = &mut vehicle {
                car.taxi_light = true;
            }
        });
        assert_eq!(world.get::<VehicleVitality>(entity).unwrap().health, 400.0);
        assert!(world.get::<AutomobileState>(entity).unwrap().taxi_light);
        assert_eq!(world.get::<VehicleDriveState>(entity).unwrap().brake_pedal, 0.75);
        assert_eq!(world.get::<VehicleAppearance>(entity).unwrap().current_colour2, 27);
        assert_eq!(world.get::<VehicleEntryState>(entity).unwrap().getting_out_flags, 4);
        assert!(world.get::<VehicleAudioState>(entity).unwrap().siren_or_alarm);
        world.despawn(entity);
        assert!(world.get::<VehicleAudioState>(entity).is_none());
        assert!(world.get::<VehicleVitality>(entity).is_none());
        assert!(world.get::<AutomobileState>(entity).is_none());
        assert!(world.get::<AutomobileAudioMotion>(entity).is_none());
        assert!(world.get::<VehicleDriveState>(entity).is_none());
        assert!(world.get::<VehicleAppearance>(entity).is_none());
        assert!(world.get::<VehicleEntryState>(entity).is_none());
        with_script_entity_test_world(&mut world, |actors| {
            assert!(actors.vehicle_get(handle).is_none());
        });
    }

    #[test]
    fn vehicle_collision_history_has_independent_stage_and_identity_lifetime() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(9);
        let mut car = CAutomobile::new(90, MISSION_VEHICLE, 0, &mut general, None, None, false);
        car.vehicle.core.physical.entity.matrix.position = CVector::new(10.0, 20.0, 30.0);
        let handle = 0x402;
        let entity =
            spawn_script_test_vehicle(&mut world, ScriptVehicleEntity::Automobile { handle, car });
        assert_eq!(
            world
                .get::<VehicleCollisionState>(entity)
                .unwrap()
                .previous_matrix
                .position,
            CVector::new(10.0, 20.0, 30.0)
        );
        with_script_entity_test_world(&mut world, |actors| {
            actors
                .vehicles
                .capture_collision_pose(&actors.index, handle);
            actors
                .vehicle_get_mut(handle)
                .unwrap()
                .base_mut()
                .physical
                .entity
                .matrix
                .position
                .x = 25.0;
            assert_eq!(
                actors
                    .vehicles
                    .collision_pose(&actors.index, handle)
                    .unwrap()
                    .position
                    .x,
                10.0
            );
            actors
                .vehicles
                .capture_collision_pose(&actors.index, handle);
            assert_eq!(
                actors
                    .vehicles
                    .collision_pose(&actors.index, handle)
                    .unwrap()
                    .position
                    .x,
                25.0
            );
            actors.index.vehicles.insert(0x401, entity);
            assert!(
                actors
                    .vehicles
                    .collision_pose(&actors.index, 0x401)
                    .is_none()
            );
        });
        world.despawn(entity);
        assert!(world.get::<VehicleCollisionState>(entity).is_none());
        with_script_entity_test_world(&mut world, |actors| {
            assert!(
                actors
                    .vehicles
                    .collision_pose(&actors.index, handle)
                    .is_none()
            );
        });
    }

    #[test]
    fn removed_and_reused_pool_handles_never_resolve_to_a_live_ped() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(1);
        let old_handle = 0x101;
        let live_handle = 0x102;
        let entity = spawn_script_test_ped(
            &mut world,
            ScriptPedEntity {
                vitality: crate::peds::PedVitality::PedVitality::default(),
                locomotion: PedLocomotion::default(),
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: live_handle,
                ped: CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
                model_id: 0,
            },
        );
        // A stale index entry must not bypass the payload's generation identity.
        world
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .insert(old_handle, entity);
        // A normal ECS system and SCM must mutate the very same live component.
        world.get_mut::<PedVitality>(entity).unwrap().health = 61.0;
        world
            .get_mut::<PedAnimation>(entity)
            .unwrap()
            .movement_anim_clump
            .velocity2d = Some((2.0, 3.0));
        with_script_entity_test_world(&mut world, |actors| {
            assert!(actors.ped_get(old_handle).is_none());
            assert!(actors.ped_get_mut(old_handle).is_none());
            assert_eq!(actors.ped_get(live_handle).unwrap().vitality.health, 61.0);
            assert_eq!(
                actors
                    .ped_get(live_handle)
                    .unwrap()
                    .animation
                    .movement_anim_clump
                    .velocity2d,
                Some((2.0, 3.0))
            );
            actors.ped_get_mut(live_handle).unwrap().vitality.health = 37.0;
            actors
                .ped_get_mut(live_handle)
                .unwrap()
                .animation
                .movement_anim_clump
                .velocity2d = Some((4.0, 5.0));
            // Subsequent SCM accesses in this pass observe the live mutation.
            assert_eq!(actors.ped_get(live_handle).unwrap().vitality.health, 37.0);
            actors.index.peds.remove(&live_handle);
            // Removing identity is immediate even before deferred despawn applies.
            assert!(actors.ped_get(live_handle).is_none());
        });
        assert_eq!(world.get::<PedVitality>(entity).unwrap().health, 37.0);
        assert_eq!(
            world
                .get::<PedAnimation>(entity)
                .unwrap()
                .movement_anim_clump
                .velocity2d,
            Some((4.0, 5.0))
        );
        assert_eq!(
            world.get::<ScriptPedIdentity>(entity).unwrap().handle,
            live_handle
        );
        world.despawn(entity);
        with_script_entity_test_world(&mut world, |actors| {
            assert!(actors.ped_get(old_handle).is_none());
        });
    }
}
