//! re3 reference: references/re-GTA/src/vehicles/Vehicle.cpp
//! re3 reference: references/re-GTA/src/vehicles/Vehicle.h
//!
//! Vehicle base state.

use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};
use crate::entities::EntityReference::EntityReference;

use crate::{
    collision::ColModel::CColBox,
    control::AutoPilot::{CAutoPilot, eCarMission, eCarTempAction},
    entities::{
        Entity::{eEntityStatus, eEntityType},
        Physical::CPhysical,
    },
    modelinfo::{ModelIndices::MI_RCBANDIT, VehicleModelInfo::eVehicleType},
    vehicles::{
        DamageManager::eWheelStatus,
        HandlingMgr::{HANDLING_HAS_NO_ROOF, cHandlingDataMgr},
    },
};

pub const RANDOM_VEHICLE: u8 = 1;
pub const MISSION_VEHICLE: u8 = 2;
pub const PARKED_VEHICLE: u8 = 3;
pub const PERMANENT_VEHICLE: u8 = 4;
pub const MAX_VEHICLE_PASSENGERS: usize = 8;
pub const USERTRACK: u8 = 9;
pub const FBURST_SPEED_MAX: f32 = 0.3;
pub const FBURST_TYRE_MOD: f32 = 0.1;

/// `CVehicle::UsesSiren` takes original model IDs, including the ice-cream van.
/// Audio's independent predicate takes indices relative to MI_LANDSTAL.
pub fn vehicle_uses_siren(model_index: usize) -> bool {
    use crate::modelinfo::ModelIndices::{
        MI_AMBULAN, MI_ENFORCER, MI_FBICAR, MI_FIRETRUCK, MI_MRWHOOP, MI_POLICE, MI_PREDATOR,
    };
    matches!(
        model_index,
        MI_FIRETRUCK | MI_AMBULAN | MI_FBICAR | MI_MRWHOOP | MI_POLICE | MI_ENFORCER | MI_PREDATOR
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eCarLock {
    CARLOCK_NOT_USED = 0,
    CARLOCK_UNLOCKED = 1,
    CARLOCK_LOCKED = 2,
    CARLOCK_LOCKOUT_PLAYER_ONLY = 3,
    CARLOCK_LOCKED_PLAYER_INSIDE = 4,
    CARLOCK_LOCKED_INITIALLY = 5,
    CARLOCK_FORCE_SHUT_DOORS = 6,
}

pub(crate) fn script_car_lock(value: i32) -> eCarLock {
    match value {
        1 => eCarLock::CARLOCK_UNLOCKED,
        2 => eCarLock::CARLOCK_LOCKED,
        3 => eCarLock::CARLOCK_LOCKOUT_PLAYER_ONLY,
        4 => eCarLock::CARLOCK_LOCKED_PLAYER_INSIDE,
        5 => eCarLock::CARLOCK_LOCKED_INITIALLY,
        6 => eCarLock::CARLOCK_FORCE_SHUT_DOORS,
        _ => eCarLock::CARLOCK_NOT_USED,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum tWheelState {
    WHEEL_STATE_NORMAL = 0,
    WHEEL_STATE_SPINNING = 1,
    WHEEL_STATE_SKIDDING = 2,
    WHEEL_STATE_FIXED = 3,
}

pub use crate::collision::Collision::CStoredCollPoly;

#[derive(Debug, Clone, PartialEq)]
pub struct CVehicleFlags {
    pub is_law_enforcer: bool,
    pub is_ambulance_on_duty: bool,
    pub is_fire_truck_on_duty: bool,
    pub is_locked: bool,
    pub freebies: bool,
    pub is_van: bool,
    pub is_bus: bool,
    pub is_big: bool,
    pub low_vehicle: bool,
    pub warned_peds: bool,
    pub crane_message_done: bool,
    pub extended_range: bool,
    pub has_been_owned_by_player: bool,
    pub create_road_block_peds: bool,
    pub using_special_col_model: bool,
    pub occupants_have_been_generated: bool,
    pub gun_switched_off: bool,
    pub vehicle_col_processed: bool,
    pub is_car_park_vehicle: bool,
    pub has_already_been_recorded: bool,
}

impl Default for CVehicleFlags {
    fn default() -> Self {
        Self {
            is_law_enforcer: false,
            is_ambulance_on_duty: false,
            is_fire_truck_on_duty: false,
            is_locked: false,
            freebies: true,
            is_van: false,
            is_bus: false,
            is_big: false,
            low_vehicle: false,
            warned_peds: false,
            crane_message_done: false,
            extended_range: false,
            has_been_owned_by_player: false,
            create_road_block_peds: false,
            using_special_col_model: false,
            occupants_have_been_generated: false,
            gun_switched_off: false,
            vehicle_col_processed: false,
            is_car_park_vehicle: false,
            has_already_been_recorded: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CVehicleWheelHandling {
    pub mass: f32,
    pub traction_loss: f32,
    pub wheel_friction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CVehicleProcessWheelInput {
    pub wheel_fwd: CVector,
    pub wheel_right: CVector,
    pub wheel_contact_speed: CVector,
    pub wheel_contact_point: CVector,
    pub wheels_on_ground: i32,
    pub thrust: f32,
    pub brake: f32,
    pub adhesion: f32,
    pub wheel_id: i8,
    pub wheel_speed: f32,
    pub wheel_state: tWheelState,
    pub wheel_status: eWheelStatus,
    pub already_skidding: bool,
    pub time_step: f32,
    pub handling: CVehicleWheelHandling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CVehicleProcessWheelResult {
    pub wheel_id: i8,
    pub wheel_speed: f32,
    pub wheel_state: tWheelState,
    pub already_skidding: bool,
    pub braking: bool,
    pub driving: bool,
    pub fwd_force: f32,
    pub right_force: f32,
    pub adhesion: f32,
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct VehicleCore {
    pub physical: CPhysical,
    pub handling_id: Option<i32>,
    pub auto_pilot: CAutoPilot,
    pub mission_value: i16,
    pub field_1d0: [f32; 4],
    pub current_ground_entity: Option<usize>,
    pub vehicle_created_by: u8,
    pub flags: CVehicleFlags,
    pub num_peds_use_it_as_cover: i8,
    pub ammo_in_clip: u8,
    pub pac_man_pickups_carried: i8,
    pub roadblock_type: u8,
    pub roadblock_node: i16,
    pub gun_firing_time: u32,
    pub time_blocked: u16,
    pub map_object_height_ahead: f32,
    pub map_object_height_behind: f32,
    pub coll_polys: [CStoredCollPoly; 2],
    pub vehicle_type: eVehicleType,
}

pub use super::VehicleAppearance::VehicleAppearance;
pub use super::VehicleDrive::VehicleDriveState;
pub use super::VehicleEntry::VehicleEntryState;
pub use super::VehicleAudio::VehicleAudioState;

/// Owned construction storage; borrowed runtime views use references instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Owned<T>(pub T);
impl<T> std::ops::Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> std::ops::DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// Shared live health/fire/explosion state, independently borrowed by all kinds.
#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct VehicleVitality {
    pub health: f32,
    pub car_fire: Option<usize>,
    pub time_of_death: u32,
    pub bomb_timer: i16,
    pub blow_up_entity: Option<EntityReference>,
    pub last_weapon_damage: i8,
    pub can_be_damaged: bool,
    pub take_less_damage: bool,
}
impl Default for VehicleVitality {
    fn default() -> Self {
        Self {
            health: 1000.0,
            car_fire: None,
            time_of_death: 0,
            bomb_timer: 0,
            blow_up_entity: None,
            last_weapon_damage: -1,
            can_be_damaged: true,
            take_less_damage: false,
        }
    }
}

/// A construction DTO or joined borrowed view, according to its storage types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleRuntime<C, V, Drive, Appearance, Entry, Audio> {
    pub core: C,
    pub vitality: V,
    pub drive: Drive,
    pub appearance: Appearance,
    pub entry: Entry,
    pub audio: Audio,
}
pub type CVehicle = VehicleRuntime<
    Owned<VehicleCore>,
    Owned<VehicleVitality>,
    Owned<VehicleDriveState>,
    Owned<VehicleAppearance>,
    Owned<VehicleEntryState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAudioState>>;
pub type VehicleRef<'a> = VehicleRuntime<
    &'a VehicleCore,
    &'a VehicleVitality,
    &'a VehicleDriveState,
    &'a VehicleAppearance,
    &'a VehicleEntryState, &'a crate::vehicles::Vehicle::VehicleAudioState>;
pub type VehicleMut<'a> = VehicleRuntime<
    &'a mut VehicleCore,
    &'a mut VehicleVitality,
    &'a mut VehicleDriveState,
    &'a mut VehicleAppearance,
    &'a mut VehicleEntryState, &'a mut crate::vehicles::Vehicle::VehicleAudioState>;
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::Deref for VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    type Target = VehicleCore;
    fn deref(&self) -> &VehicleCore {
        &self.core
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::DerefMut for VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    fn deref_mut(&mut self) -> &mut VehicleCore {
        &mut self.core
    }
}
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn as_ref(&self) -> VehicleRef<'_> {
        VehicleRuntime {
            core: &self.core,
            vitality: &self.vitality,
            drive: &self.drive,
            appearance: &self.appearance,
            entry: &self.entry,
            audio: &self.audio,
        }
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn as_mut(&mut self) -> VehicleMut<'_> {
        VehicleRuntime {
            core: &mut self.core,
            vitality: &mut self.vitality,
            drive: &mut self.drive,
            appearance: &mut self.appearance,
            entry: &mut self.entry,
            audio: &mut self.audio,
        }
    }
}
impl CVehicle {
    pub fn into_parts(
        self,
    ) -> (
        VehicleCore,
        VehicleVitality,
        VehicleDriveState,
        VehicleAppearance,
        VehicleEntryState,
        VehicleAudioState,
    ) {
        (
            self.core.0,
            self.vitality.0,
            self.drive.0,
            self.appearance.0,
            self.entry.0,
            self.audio.0,
        )
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    /// CCarCtrl::SwitchVehicleToRealPhysics resets autopilot actions/timestamps.
    pub(crate) fn switch_to_real_physics(&mut self, now_ms: u32) {
        self.core.switch_to_real_physics(now_ms);
    }
}

impl VehicleCore {
    /// Native autopilot hand-off needs only the core owner.
    pub(crate) fn switch_to_real_physics(&mut self, now_ms: u32) {
        self.auto_pilot.switch_to_real_physics(now_ms);
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    /// Weapon branch of re3 `CVehicle::InflictDamage`.
    ///
    /// The automobile damage lifecycle observes the resulting health and
    /// performs the authored fire/explosion transition in its ECS system.
    pub(crate) fn inflict_weapon_damage(
        &mut self,
        weapon_type: crate::weapons::WeaponType::eWeaponType,
        damage: f32,
    ) -> bool {
        self.inflict_weapon_damage_from_source(weapon_type, damage, true)
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub(crate) fn inflict_weapon_damage_from_source(
        &mut self,
        weapon_type: crate::weapons::WeaponType::eWeaponType,
        damage: f32,
        damaged_by_player: bool,
    ) -> bool {
        self.vitality.inflict_weapon_damage_from_source(
            &self.core, weapon_type, damage, damaged_by_player,
        )
    }
}

impl VehicleVitality {
    /// Weapon damage writes health/damage history and reads protection flags
    /// from the physical owner. Drive, appearance, seating and audio owners
    /// are independent of this state calculation.
    pub(crate) fn inflict_weapon_damage_from_source(
        &mut self,
        core: &VehicleCore,
        weapon_type: crate::weapons::WeaponType::eWeaponType,
        damage: f32,
        damaged_by_player: bool,
    ) -> bool {
        use crate::weapons::WeaponType::eWeaponType;

        if !self.can_be_damaged
            || self.health <= 0.0
            || (core.physical.entity.flags.only_damaged_by_player && !damaged_by_player)
        {
            return false;
        }
        let proofed = match weapon_type {
            eWeaponType::WEAPONTYPE_UNARMED | eWeaponType::WEAPONTYPE_BASEBALLBAT => {
                core.physical.entity.flags.melee_proof
            }
            eWeaponType::WEAPONTYPE_COLT45
            | eWeaponType::WEAPONTYPE_UZI
            | eWeaponType::WEAPONTYPE_SHOTGUN
            | eWeaponType::WEAPONTYPE_AK47
            | eWeaponType::WEAPONTYPE_M16
            | eWeaponType::WEAPONTYPE_SNIPERRIFLE
            | eWeaponType::WEAPONTYPE_UZI_DRIVEBY => core.physical.entity.flags.bullet_proof,
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
            | eWeaponType::WEAPONTYPE_MOLOTOV
            | eWeaponType::WEAPONTYPE_GRENADE
            | eWeaponType::WEAPONTYPE_EXPLOSION => core.physical.entity.flags.explosion_proof,
            eWeaponType::WEAPONTYPE_FLAMETHROWER => core.physical.entity.flags.fire_proof,
            eWeaponType::WEAPONTYPE_RAMMEDBYCAR | eWeaponType::WEAPONTYPE_RUNOVERBYCAR => {
                core.physical.entity.flags.collision_proof
            }
            _ => false,
        };
        if proofed {
            return false;
        }

        self.last_weapon_damage = weapon_type as i8;
        self.health = (self.health - damage).max(0.0);
        true
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    /// re3 `CVehicle::CarHasRoof`.
    ///
    /// `HANDLING_HAS_NO_ROOF` marks convertibles/open vehicles. The original
    /// still treats them as roofed unless both optional roof extras were
    /// actually instantiated on this vehicle.
    pub fn car_has_roof(&self, handling_manager: &cHandlingDataMgr) -> bool {
        self.core.car_has_roof(&self.appearance, handling_manager)
    }
}

impl VehicleCore {
    /// Roof decisions borrow only the authored handling binding and extras.
    pub fn car_has_roof(
        &self,
        appearance: &VehicleAppearance,
        handling_manager: &cHandlingDataMgr,
    ) -> bool {
        let Some(handling_id) = self
            .handling_id
            .and_then(|id| usize::try_from(id).ok())
        else {
            return true;
        };
        let Some(handling) = handling_manager.get_handling_data(handling_id) else {
            return true;
        };
        if handling.flags & HANDLING_HAS_NO_ROOF == 0 {
            return true;
        }
        !(appearance.extras[0] != 0 && appearance.extras[1] != 0)
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn can_ped_enter_car(&self) -> bool {
        let up_z = self.core.physical.entity.get_up().z;
        (up_z > 0.1 || up_z < -0.1)
            && self.core.physical.move_speed.magnitude_sqr() <= 0.2_f32.powi(2)
            && self.core.physical.turn_speed.magnitude_sqr() <= 0.2_f32.powi(2)
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    /// re3 `CVehicle::CanPedOpenLocks`.
    pub fn can_ped_open_locks(&self, is_player: bool) -> bool {
        !matches!(
            self.entry.door_lock,
            eCarLock::CARLOCK_LOCKED
                | eCarLock::CARLOCK_LOCKED_INITIALLY
                | eCarLock::CARLOCK_LOCKED_PLAYER_INSIDE
        ) && !(is_player && self.entry.door_lock == eCarLock::CARLOCK_LOCKOUT_PLAYER_ONLY)
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn can_ped_exit_car(&self) -> bool {
        let up_z = self.core.physical.entity.get_up().z;
        let upright_or_inverted = up_z > 0.1 || up_z < -0.1;
        let move_speed = self.core.physical.move_speed.magnitude_sqr();
        let turn_speed = self.core.physical.turn_speed;
        if upright_or_inverted {
            move_speed <= 0.005
                && turn_speed.x.abs() <= 0.01
                && turn_speed.y.abs() <= 0.01
                && turn_speed.z.abs() <= 0.01
        } else {
            move_speed < 0.005
                && turn_speed.x.abs() < 0.01
                && turn_speed.y.abs() < 0.01
                && turn_speed.z.abs() < 0.01
        }
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    /// re3 `CVehicle::CanBeDeleted`. Occupant ownership lives in ECS
    /// `InVehicle` relations, so the runtime supplies the derived aggregate
    /// instead of asking `CVehicle` to own a reverse ped-slot list.
    pub(crate) fn can_be_deleted_with(&self, occupants_can_be_deleted: bool) -> bool {
        if self.entry.num_getting_in != 0 || self.entry.getting_out_flags != 0 {
            return false;
        }
        if !occupants_can_be_deleted {
            return false;
        }
        match self.core.vehicle_created_by {
            RANDOM_VEHICLE | PARKED_VEHICLE => true,
            MISSION_VEHICLE | PERMANENT_VEHICLE => false,
            _ => true,
        }
    }
}

impl CVehicle {
    pub fn new(created_by: u8, time_ms: u32, general: &mut CGeneral) -> Self {
        let mut physical = CPhysical::new(general);
        physical.entity.set_type(eEntityType::ENTITY_TYPE_VEHICLE);

        let mut vehicle = Self {
            vitality: Owned(VehicleVitality::default()),
            drive: Owned(VehicleDriveState::default()),
            appearance: Owned(VehicleAppearance::default()),
            entry: Owned(VehicleEntryState::default()),
            audio: Owned(VehicleAudioState::default()),
            core: Owned(VehicleCore {
                physical,
                handling_id: None,
                auto_pilot: {
                    let mut auto_pilot = CAutoPilot::new(time_ms);
                    auto_pilot.car_mission = eCarMission::MISSION_NONE;
                    auto_pilot.temp_action = eCarTempAction::TEMPACT_NONE;
                    auto_pilot.stay_in_current_level = false;
                    auto_pilot.ignore_pathfinding = false;
                    auto_pilot
                },
                mission_value: 0,
                field_1d0: [0.0; 4],
                current_ground_entity: None,
                vehicle_created_by: created_by,
                flags: CVehicleFlags::default(),
                num_peds_use_it_as_cover: 0,
                ammo_in_clip: 0,
                pac_man_pickups_carried: 0,
                roadblock_type: 0,
                roadblock_node: 0,
                gun_firing_time: 0,
                time_blocked: 0,
                map_object_height_ahead: 0.0,
                map_object_height_behind: 0.0,
                coll_polys: [CStoredCollPoly::default(), CStoredCollPoly::default()],
                vehicle_type: eVehicleType::VEHICLE_TYPE_CAR,
            }),
        };
        vehicle.audio.radio_station = (general.get_random_number() % u16::from(USERTRACK)) as u8;
        vehicle
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn set_status(&mut self, status: eEntityStatus) {
        self.core.physical.entity.set_status(status);
    }
}

impl VehicleCore {
    /// CVehicle::IsLawEnforcementVehicle uses original gameplay model IDs.
    pub(crate) fn is_law_enforcement_vehicle(&self) -> bool {
        use crate::modelinfo::ModelIndices::{MI_FBICAR, MI_POLICE, MI_ENFORCER, MI_PREDATOR, MI_RHINO, MI_BARRACKS};
        matches!(self.physical.entity.model_index as usize,
            MI_FBICAR | MI_POLICE | MI_ENFORCER | MI_PREDATOR | MI_RHINO | MI_BARRACKS)
    }

    /// re3 `CVehicle::IsSphereTouchingVehicle`. This deliberately tests the
    /// model bounding box in the vehicle's local right/forward/up basis rather
    /// than using a world-space distance approximation.
    pub fn is_sphere_touching_vehicle(
        &self,
        sphere: CVector,
        radius: f32,
        bounding_box: &CColBox,
    ) -> bool {
        let position = self.physical.entity.get_position();
        let relative = CVector::new(
            sphere.x - position.x,
            sphere.y - position.y,
            sphere.z - position.z,
        );
        let right = self.physical.entity.get_right();
        let x = relative.x * right.x + relative.y * right.y + relative.z * right.z;
        if bounding_box.min.x - radius > x || bounding_box.max.x + radius < x {
            return false;
        }
        let forward = self.physical.entity.get_forward();
        let y = relative.x * forward.x + relative.y * forward.y + relative.z * forward.z;
        if bounding_box.min.y - radius > y || bounding_box.max.y + radius < y {
            return false;
        }
        let up = self.physical.entity.get_up();
        let z = relative.x * up.x + relative.y * up.y + relative.z * up.z;
        if bounding_box.min.z - radius > z || bounding_box.max.z + radius < z {
            return false;
        }
        true
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    Drive: std::ops::Deref<Target = VehicleDriveState>,
    Appearance: std::ops::Deref<Target = VehicleAppearance>,
    Entry: std::ops::Deref<Target = VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn is_car(&self) -> bool {
        self.core.vehicle_type == eVehicleType::VEHICLE_TYPE_CAR
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    Drive: std::ops::DerefMut<Target = VehicleDriveState>,
    Appearance: std::ops::DerefMut<Target = VehicleAppearance>,
    Entry: std::ops::DerefMut<Target = VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>
{
    pub fn process_wheel(
        &mut self,
        mut input: CVehicleProcessWheelInput,
        general: &mut CGeneral,
    ) -> CVehicleProcessWheelResult {
        let mut fwd = 0.0;
        let mut right = 0.0;
        let braking = input.brake != 0.0;
        if braking {
            input.thrust = 0.0;
        }
        let driving = input.thrust != 0.0;

        let contact_speed_fwd = dot_vec3(input.wheel_contact_speed, input.wheel_fwd);
        let contact_speed_right = dot_vec3(input.wheel_contact_speed, input.wheel_right);

        if input.wheel_state != tWheelState::WHEEL_STATE_NORMAL {
            input.already_skidding = true;
        }
        input.wheel_state = tWheelState::WHEEL_STATE_NORMAL;

        input.adhesion *= input.time_step;
        if input.already_skidding {
            input.adhesion *= input.handling.traction_loss;
        }

        if input.wheels_on_ground <= 0 {
            return CVehicleProcessWheelResult {
                wheel_id: input.wheel_id,
                wheel_speed: input.wheel_speed,
                wheel_state: input.wheel_state,
                already_skidding: input.already_skidding,
                braking,
                driving,
                fwd_force: fwd,
                right_force: right,
                adhesion: input.adhesion,
            };
        }

        let wheels_on_ground = input.wheels_on_ground as f32;
        if contact_speed_right != 0.0 {
            right = -contact_speed_right / wheels_on_ground;
            if input.wheel_status == eWheelStatus::WHEEL_STATUS_BURST {
                let fwd_speed = contact_speed_fwd.min(FBURST_SPEED_MAX);
                right += fwd_speed
                    * general.get_random_number_in_range_f32(-FBURST_TYRE_MOD, FBURST_TYRE_MOD);
            }
        }

        if driving {
            fwd = input.thrust;
            right = clamp_signed(right, input.adhesion);
        } else if contact_speed_fwd != 0.0 {
            fwd = -contact_speed_fwd / wheels_on_ground;
            if !braking && self.drive.gas_pedal < 0.01 {
                input.brake = input.handling.wheel_friction / input.handling.mass;
                if self.core.physical.entity.model_index == MI_RCBANDIT as i16 {
                    input.brake *= 0.2;
                }
                // re3 FIX_BUGS applies GetTimeStepFix to passive wheel brake.
                input.brake *= input.time_step / (50.0 / 30.0);
            }

            if input.brake > input.adhesion {
                if contact_speed_fwd.abs() > 0.005 {
                    input.wheel_state = tWheelState::WHEEL_STATE_FIXED;
                }
            } else {
                fwd = clamp_signed(fwd, input.brake);
            }
        }

        let speed_sqr = right * right + fwd * fwd;
        if input.adhesion * input.adhesion < speed_sqr {
            if input.wheel_state != tWheelState::WHEEL_STATE_FIXED {
                if driving && contact_speed_fwd < 0.2 {
                    input.wheel_state = tWheelState::WHEEL_STATE_SPINNING;
                } else {
                    input.wheel_state = tWheelState::WHEEL_STATE_SKIDDING;
                }
            }

            let length = speed_sqr.sqrt();
            let traction_loss = if input.already_skidding {
                1.0
            } else {
                input.handling.traction_loss
            };
            right *= input.adhesion * traction_loss / length;
            fwd *= input.adhesion * traction_loss / length;
        }

        if fwd != 0.0 || right != 0.0 {
            let mut direction = add_vec3(
                scale_vec3(input.wheel_fwd, fwd),
                scale_vec3(input.wheel_right, right),
            );
            let speed = direction.magnitude();
            if speed != 0.0 {
                direction = scale_vec3(direction, 1.0 / speed);
                let impulse = speed * self.core.physical.mass;
                let turn_impulse = speed
                    * self
                        .core
                        .physical
                        .get_mass(input.wheel_contact_point, direction);
                self.core
                    .physical
                    .apply_move_force(scale_vec3(direction, impulse));
                self.core.physical.apply_turn_force(
                    scale_vec3(direction, turn_impulse),
                    input.wheel_contact_point,
                );
            }
        }

        CVehicleProcessWheelResult {
            wheel_id: input.wheel_id,
            wheel_speed: input.wheel_speed,
            wheel_state: input.wheel_state,
            already_skidding: input.already_skidding,
            braking,
            driving,
            fwd_force: fwd,
            right_force: right,
            adhesion: input.adhesion,
        }
    }
}

impl CVehicle {
    pub fn process_wheel_rotation(
        state: tWheelState,
        fwd: CVector,
        speed: CVector,
        radius: f32,
        time_step: f32,
    ) -> f32 {
        let angular_velocity = match state {
            tWheelState::WHEEL_STATE_SPINNING => -1.1,
            tWheelState::WHEEL_STATE_FIXED => 0.0,
            _ => -dot_vec3(fwd, speed) / radius,
        };
        angular_velocity * time_step
    }
}

fn clamp_signed(value: f32, limit: f32) -> f32 {
    if value > 0.0 {
        value.min(limit)
    } else {
        value.max(-limit)
    }
}

fn add_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn scale_vec3(a: CVector, scale: f32) -> CVector {
    CVector::new(a.x * scale, a.y * scale, a.z * scale)
}

fn dot_vec3(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
