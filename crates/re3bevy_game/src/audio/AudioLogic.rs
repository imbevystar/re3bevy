use crate::audio::AudioManager::NUM_AUDIOENTITY_EVENTS;
use crate::audio::AudioScriptObject::cAudioScriptObjectPool;
use crate::core::Timer::CTimer;
use re3bevy_core::core::Coordinates::CVector;
use super::AudioActorAccess::{AudioActorDiagnostics, AudioFocusedPlayer, AudioPedEvents, AudioVehicleActors};

use super::AudioManager::{
    MAX_VOLUME, SFX_BANK_0, cAudioManager, tSound,
};
use super::AudioSamples::*;
use super::soundlist::*;
use crate::core::SurfaceTable::eSurfaceType;
use crate::weapons::WeaponType::eWeaponType;

pub const NO_SAMPLE: u32 = TOTAL_AUDIO_SAMPLES as u32;
pub const PED_COMMENT_MAX_DIST: f32 = 50.0;
pub const PED_COMMENT_POLICE_HELI_MAX_DIST: f32 = 400.0;
pub const PED_COMMENT_VOLUME: u8 = 127;
pub const PED_COMMENT_VOLUME_BEHIND_WALL: u8 = 31;
pub const VEHICLE_HORN_MAX_DIST: f32 = 40.0;
pub const VEHICLE_HORN_VOLUME: u8 = 80;
pub const VEHICLE_SIREN_MAX_DIST: f32 = 110.0;
pub const VEHICLE_SIREN_VOLUME: u8 = 80;
pub const VEHICLE_REVERSE_WARNING_MAX_DIST: f32 = 50.0;
pub const VEHICLE_REVERSE_WARNING_VOLUME: u8 = 60;
pub const VEHICLE_DOORS_MAX_DIST: f32 = 40.0;
pub const VEHICLE_DOORS_VOLUME: u8 = 100;
pub const AIR_BRAKES_MAX_DIST: f32 = 30.0;
pub const AIR_BRAKES_VOLUME: u8 = 70;
pub const VEHICLE_ROAD_NOISE_MAX_DIST: f32 = 95.0;
pub const VEHICLE_ROAD_NOISE_VOLUME: u8 = 30;
pub const WET_ROAD_NOISE_MAX_DIST: f32 = 30.0;
pub const WET_ROAD_NOISE_VOLUME: u8 = 23;
pub const VEHICLE_ENGINE_MAX_DIST: f32 = 50.0;
pub const VEHICLE_ENGINE_BASE_VOLUME: u8 = 80;
pub const VEHICLE_ENGINE_FULL_VOLUME: u8 = 120;
pub const VEHICLE_SKIDDING_MAX_DIST: f32 = 40.0;
pub const VEHICLE_SKIDDING_VOLUME: u8 = 50;
pub const CESNA_IDLE_MAX_DIST: f32 = 200.0;
pub const CESNA_REV_MAX_DIST: f32 = 90.0;
pub const CESNA_VOLUME: u8 = 80;
pub const PLAYER_VEHICLE_ENGINE_VOLUME: u8 = 85;
pub const ENGINE_DAMAGE_MAX_DIST: f32 = 40.0;
pub const ENGINE_DAMAGE_VOLUME: u8 = 6;
pub const ENGINE_DAMAGE_ON_FIRE_VOLUME: u8 = 60;
pub const CAR_BOMB_TICK_MAX_DIST: f32 = 40.0;
pub const CAR_BOMB_TICK_VOLUME: u8 = 60;
pub const REVERSE_GEAR_MAX_DIST: f32 = 30.0;
pub const REVERSE_GEAR_VOLUME: u8 = 24;
pub const MODEL_CAR_ENGINE_MAX_DIST: f32 = 30.0;
pub const MODEL_CAR_ENGINE_VOLUME: u8 = 90;
pub const RAIN_ON_VEHICLE_MAX_DIST: f32 = 22.0;
pub const RAIN_ON_VEHICLE_VOLUME: u8 = 30;
pub const VEHICLE_ONE_SHOT_DOOR_MAX_DIST: f32 = 50.0;
pub const VEHICLE_ONE_SHOT_DOOR_OPEN_VOLUME: u8 = 122;
pub const VEHICLE_ONE_SHOT_DOOR_CLOSE_VOLUME: u8 = 117;
pub const VEHICLE_ONE_SHOT_WINDSHIELD_CRACK_MAX_DIST: f32 = 30.0;
pub const VEHICLE_ONE_SHOT_WINDSHIELD_CRACK_VOLUME: u8 = 60;
pub const VEHICLE_ONE_SHOT_CAR_JUMP_MAX_DIST: f32 = 35.0;
pub const VEHICLE_ONE_SHOT_CAR_JUMP_VOLUME: u8 = 80;
pub const VEHICLE_ONE_SHOT_CAR_ENGINE_START_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_CAR_ENGINE_START_VOLUME: u8 = 60;
pub const VEHICLE_ONE_SHOT_CAR_LIGHT_BREAK_VOLUME: u8 = 30;
pub const VEHICLE_ONE_SHOT_CAR_HYDRAULIC_MAX_DIST: f32 = 35.0;
pub const VEHICLE_ONE_SHOT_CAR_HYDRAULIC_VOLUME: u8 = 55;
pub const VEHICLE_ONE_SHOT_CAR_SPLASH_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_CAR_SPLASH_VOLUME: u8 = 55;
pub const VEHICLE_ONE_SHOT_BOAT_SLOWDOWN_MAX_DIST: f32 = 50.0;
pub const VEHICLE_ONE_SHOT_TRAIN_DOOR_MAX_DIST: f32 = 35.0;
pub const VEHICLE_ONE_SHOT_TRAIN_DOOR_VOLUME: u8 = 70;
pub const VEHICLE_ONE_SHOT_CAR_TANK_TURRET_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_CAR_TANK_TURRET_VOLUME: u8 = 90;
pub const VEHICLE_ONE_SHOT_CAR_BOMB_TICK_MAX_DIST: f32 = 30.0;
pub const VEHICLE_ONE_SHOT_CAR_BOMB_TICK_VOLUME: u8 = CAR_BOMB_TICK_VOLUME;
pub const VEHICLE_ONE_SHOT_PLANE_ON_GROUND_MAX_DIST: f32 = 180.0;
pub const VEHICLE_ONE_SHOT_PLANE_ON_GROUND_VOLUME: u8 = 75;
pub const VEHICLE_ONE_SHOT_WEAPON_SHOT_FIRED_MAX_DIST: f32 = 120.0;
pub const VEHICLE_ONE_SHOT_WEAPON_SHOT_FIRED_VOLUME: u8 = 65;
pub const VEHICLE_ONE_SHOT_WEAPON_HIT_VEHICLE_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_WEAPON_HIT_VEHICLE_VOLUME: u8 = 90;
pub const VEHICLE_ONE_SHOT_BOMB_ARMED_MAX_DIST: f32 = 50.0;
pub const VEHICLE_ONE_SHOT_BOMB_ARMED_VOLUME: u8 = 50;
pub const VEHICLE_ONE_SHOT_WATER_FALL_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_WATER_FALL_VOLUME: u8 = 90;
pub const VEHICLE_ONE_SHOT_SPLATTER_MAX_DIST: f32 = 40.0;
pub const VEHICLE_ONE_SHOT_SPLATTER_VOLUME: u8 = 55;
pub const VEHICLE_ONE_SHOT_CAR_PED_COLLISION_MAX_DIST: f32 = 40.0;
pub const PED_ONE_SHOT_STEP_MAX_DIST: f32 = 20.0;
pub const PED_ONE_SHOT_STEP_VOLUME: u8 = 45;
pub const PED_ONE_SHOT_FALL_MAX_DIST: f32 = 30.0;
pub const PED_ONE_SHOT_FALL_VOLUME: u8 = 80;
pub const PED_ONE_SHOT_PUNCH_MAX_DIST: f32 = 30.0;
pub const PED_ONE_SHOT_PUNCH_VOLUME: u8 = 100;
pub const PED_ONE_SHOT_WEAPON_COLT45_MAX_DIST: f32 = 50.0;
pub const PED_ONE_SHOT_WEAPON_COLT45_VOLUME: u8 = 90;
pub const PED_ONE_SHOT_WEAPON_UZI_MAX_DIST: f32 = 80.0;
pub const PED_ONE_SHOT_WEAPON_UZI_VOLUME: u8 = 70;
pub const PED_ONE_SHOT_WEAPON_SHOTGUN_MAX_DIST: f32 = 60.0;
pub const PED_ONE_SHOT_WEAPON_SHOTGUN_VOLUME: u8 = 100;
pub const PED_ONE_SHOT_WEAPON_AK47_MAX_DIST: f32 = 80.0;
pub const PED_ONE_SHOT_WEAPON_AK47_VOLUME: u8 = 70;
pub const PED_ONE_SHOT_WEAPON_M16_MAX_DIST: f32 = 80.0;
pub const PED_ONE_SHOT_WEAPON_M16_VOLUME: u8 = 70;
pub const PED_ONE_SHOT_WEAPON_SNIPERRIFLE_MAX_DIST: f32 = 60.0;
pub const PED_ONE_SHOT_WEAPON_SNIPERRIFLE_VOLUME: u8 = 110;
pub const PED_ONE_SHOT_WEAPON_ROCKETLAUNCHER_MAX_DIST: f32 = 90.0;
pub const PED_ONE_SHOT_WEAPON_ROCKETLAUNCHER_VOLUME: u8 = 80;
pub const PED_ONE_SHOT_WEAPON_FLAMETHROWER_MAX_DIST: f32 = 60.0;
pub const PED_ONE_SHOT_WEAPON_FLAMETHROWER_VOLUME: u8 = 90;
pub const PED_ONE_SHOT_WEAPON_RELOAD_MAX_DIST: f32 = 30.0;
pub const PED_ONE_SHOT_WEAPON_RELOAD_VOLUME: u8 = 75;
pub const PED_ONE_SHOT_WEAPON_BULLET_ECHO_MAX_DIST: f32 = 80.0;
pub const PED_ONE_SHOT_WEAPON_BULLET_ECHO_VOLUME: u8 = 40;
pub const PED_ONE_SHOT_WEAPON_FLAMETHROWER_FIRE_MAX_DIST: f32 = 60.0;
pub const PED_ONE_SHOT_WEAPON_FLAMETHROWER_FIRE_VOLUME: u8 = 70;
pub const PED_ONE_SHOT_WEAPON_HIT_PED_MAX_DIST: f32 = 30.0;
pub const PED_ONE_SHOT_WEAPON_HIT_PED_VOLUME: u8 = 90;
pub const PED_ONE_SHOT_SPLASH_MAX_DIST: f32 = 40.0;
pub const PED_ONE_SHOT_SPLASH_PED_VOLUME: u8 = 70;
pub const SCRIPT_OBJECT_GATE_MAX_DIST: f32 = 40.0;
pub const SCRIPT_OBJECT_BULLET_HIT_GROUND_MAX_DIST: f32 = 50.0;
pub const SCRIPT_OBJECT_BULLET_HIT_GROUND_VOLUME: u8 = 90;
pub const SCRIPT_OBJECT_PAYPHONE_RINGING_MAX_DIST: f32 = 80.0;
pub const SCRIPT_OBJECT_PAYPHONE_RINGING_VOLUME: u8 = 80;
pub const SCRIPT_OBJECT_GLASS_BREAK_MAX_DIST: f32 = 60.0;
pub const SCRIPT_OBJECT_GLASS_BREAK_LONG_VOLUME: u8 = 70;
pub const SCRIPT_OBJECT_GLASS_BREAK_SHORT_VOLUME: u8 = 60;
pub const SCRIPT_OBJECT_GLASS_LIGHT_BREAK_MAX_DIST: f32 = 55.0;
pub const SCRIPT_OBJECT_GLASS_LIGHT_BREAK_VOLUME: u8 = 25;
pub const SCRIPT_OBJECT_BOX_DESTROYED_MAX_DIST: f32 = 60.0;
pub const SCRIPT_OBJECT_BOX_DESTROYED_VOLUME: u8 = 80;
pub const SCRIPT_OBJECT_METAL_COLLISION_VOLUME: u8 = 70;
pub const SCRIPT_OBJECT_TIRE_COLLISION_VOLUME: u8 = 60;
pub const SCRIPT_OBJECT_GUNSHELL_MAX_DIST: f32 = 20.0;
pub const SCRIPT_OBJECT_GUNSHELL_VOLUME: u8 = 30;
pub const SCRIPT_OBJECT_SHORT_MAX_DIST: f32 = 30.0;
pub const SCRIPT_OBJECT_LONG_MAX_DIST: f32 = 80.0;
pub const SCRIPT_OBJECT_DEFAULT_VOLUME: u8 = MAX_VOLUME;
pub const SCRIPT_OBJECT_RESTAURANT_VOLUME: u8 = 110;
pub const SCRIPT_OBJECT_BANK_ALARM_VOLUME: u8 = 90;
pub const PORN_CINEMA_SHORT_MAX_DIST: f32 = 20.0;
pub const PORN_CINEMA_LONG_MAX_DIST: f32 = SCRIPT_OBJECT_LONG_MAX_DIST;
pub const PORN_CINEMA_VOLUME: u8 = SCRIPT_OBJECT_DEFAULT_VOLUME;
pub const PORN_CINEMA_MOAN_VOLUME: u8 = 90;
pub const WORK_SHOP_MAX_DIST: f32 = 20.0;
pub const WORK_SHOP_VOLUME: u8 = 30;
pub const SAWMILL_VOLUME: u8 = 30;
pub const SAWMILL_CUT_WOOD_VOLUME: u8 = 70;
pub const LAUNDERETTE_VOLUME: u8 = 45;
pub const LAUNDERETTE_SONG_VOLUME: u8 = 110;
pub const SHOP_VOLUME: u8 = 30;
pub const SHOP_TILL_VOLUME: u8 = 70;
pub const AIRPORT_VOLUME: u8 = 110;
pub const CINEMA_VOLUME: u8 = 30;
pub const DOCKS_VOLUME: u8 = 40;
pub const HOME_VOLUME: u8 = 40;
pub const POLICE_CELL_BEATING_VOLUME: u8 = 55;
pub const TRAIN_NOISE_FAR_MAX_DIST: f32 = 300.0;
pub const TRAIN_NOISE_NEAR_MAX_DIST: f32 = 70.0;
pub const TRAIN_NOISE_VOLUME: u8 = 75;
pub const BOAT_ENGINE_MAX_DIST: f32 = 50.0;
pub const BOAT_ENGINE_REEFER_IDLE_VOLUME: u8 = 50;
pub const BOAT_ENGINE_REEFER_ACCEL_MIN_VOLUME: u8 = 15;
pub const BOAT_ENGINE_REEFER_ACCEL_VOLUME_MULT: u8 = 100;
pub const BOAT_ENGINE_LOW_ACCEL_VOLUME: u8 = 45;
pub const BOAT_ENGINE_HIGH_ACCEL_MIN_VOLUME: u8 = 15;
pub const BOAT_ENGINE_HIGH_ACCEL_VOLUME_MULT: u8 = 105;
pub const BOAT_MOVING_OVER_WATER_MAX_DIST: f32 = 50.0;
pub const BOAT_MOVING_OVER_WATER_VOLUME: u8 = 30;
pub const JUMBO_MAX_DIST: f32 = 440.0;
pub const JUMBO_RUMBLE_SOUND_MAX_DIST: f32 = 240.0;
pub const JUMBO_ENGINE_SOUND_MAX_DIST: f32 = 180.0;
pub const JUMBO_WHINE_SOUND_MAX_DIST: f32 = 170.0;
pub const EXPLOSION_DEFAULT_MAX_DIST: f32 = 400.0;
pub const EXPLOSION_MOLOTOV_MAX_DIST: f32 = 200.0;
pub const EXPLOSION_MINE_MAX_DIST: f32 = 300.0;
pub const FIRE_DEFAULT_MAX_DIST: f32 = 50.0;
pub const FIRE_DEFAULT_VOLUME: u8 = 80;
pub const FIRE_BUILDING_MAX_DIST: f32 = 50.0;
pub const FIRE_BUILDING_VOLUME: u8 = 100;
pub const FIRE_PED_MAX_DIST: f32 = 25.0;
pub const FIRE_PED_VOLUME: u8 = 60;
pub const WATER_CANNON_MAX_DIST: f32 = 30.0;
pub const WATER_CANNON_VOLUME: u8 = 50;
pub const CRANE_MAX_DIST: f32 = 80.0;
pub const CRANE_VOLUME: u8 = 100;
pub const PROJECTILE_ROCKET_MAX_DIST: f32 = 90.0;
pub const PROJECTILE_ROCKET_VOLUME: u8 = MAX_VOLUME;
pub const PROJECTILE_MOLOTOV_MAX_DIST: f32 = 30.0;
pub const PROJECTILE_MOLOTOV_VOLUME: u8 = 50;
pub const GARAGES_MAX_DIST: f32 = 80.0;
pub const GARAGES_VOLUME: u8 = 90;
pub const GARAGES_DOOR_VOLUME: u8 = 60;
pub const FIRE_HYDRANT_MAX_DIST: f32 = 35.0;
pub const FIRE_HYDRANT_VOLUME: u8 = 40;
pub const BRIDGE_MOTOR_MAX_DIST: f32 = 400.0;
pub const BRIDGE_MOTOR_VOLUME: u8 = MAX_VOLUME;
pub const BRIDGE_MAX_DIST: f32 = BRIDGE_MOTOR_MAX_DIST + 50.0;
pub const BRIDGE_WARNING_VOLUME: u8 = 100;
pub const FRONTEND_VOLUME: u8 = 110;

pub const OLD_DOOR: u8 = 0;
pub const NEW_DOOR: u8 = 1;
pub const TRUCK_DOOR: u8 = 2;
pub const BUS_DOOR: u8 = 3;

pub const SFX_BANK_0_LOGIC: u8 = 0;
pub const SFX_BANK_GENERIC_EXTRA: u8 = SFX_BANK_0;
pub const SFX_BANK_FRONT_END_MENU: u8 = SFX_BANK_0;
pub const CAR_SFX_BANKS_OFFSET: u8 = 1;
pub const SFX_BANK_PACARD: u8 = CAR_SFX_BANKS_OFFSET;
pub const SFX_BANK_PATHFINDER: u8 = 2;
pub const SFX_BANK_PORSCHE: u8 = 3;
pub const SFX_BANK_SPIDER: u8 = 4;
pub const SFX_BANK_MERC: u8 = 5;
pub const SFX_BANK_TRUCK: u8 = 6;
pub const SFX_BANK_HOTROD: u8 = 7;
pub const SFX_BANK_COBRA: u8 = 8;
pub const SFX_BANK_NONE: u8 = 9;

pub const LANDSTAL: usize = 0;
pub const LINERUN: usize = 3;
pub const FIRETRUK: usize = 7;
pub const TRASH: usize = 8;
pub const AMBULAN: usize = 16;
pub const FBICAR: usize = 17;
pub const MRWHOOP: usize = 23;
pub const POLICE: usize = 26;
pub const ENFORCER: usize = 27;
pub const PREDATOR: usize = 30;
pub const BUS: usize = 31;
pub const COACH: usize = 37;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct tVehicleSampleData {
    pub m_nAccelerationSampleIndex: eSfxSample,
    pub m_nBank: u8,
    pub m_nHornSample: eSfxSample,
    pub m_nHornFrequency: i32,
    pub m_nSirenOrAlarmSample: eSfxSample,
    pub m_nSirenOrAlarmFrequency: i32,
    pub m_bDoorType: u8,
}

pub const MAX_CARS: usize = 70;

pub const A_VEHICLE_SETTINGS: [tVehicleSampleData; MAX_CARS] = [
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_2,
        m_nBank: SFX_BANK_PATHFINDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 11487,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_8,
        m_nBank: SFX_BANK_COBRA,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 11025,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 10928,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 29711,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 12893,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8941,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 10706,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 11922,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 29711,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 7948,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 29711,
        m_nSirenOrAlarmSample: SFX_POLICE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 11556,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 31478,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8941,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 9538,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 12220,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 10842,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_3,
        m_nBank: SFX_BANK_PORSCHE,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 12017,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_2,
        m_nBank: SFX_BANK_PATHFINDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 22295,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 12200,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 18000,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 13400,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_BUS,
        m_nHornFrequency: 18286,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_3,
        m_nBank: SFX_BANK_PORSCHE,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 11025,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 13600,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 22295,
        m_nSirenOrAlarmSample: SFX_AMBULANCE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 8795,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 9271,
        m_nSirenOrAlarmSample: SFX_POLICE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 16168,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 12170,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8000,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 12345,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_2,
        m_nBank: SFX_BANK_PATHFINDER,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 10796,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8543,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 9271,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_2,
        m_nBank: SFX_BANK_PATHFINDER,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 10924,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 11025,
        m_nSirenOrAlarmSample: SFX_ICE_CREAM_TUNE,
        m_nSirenOrAlarmFrequency: 11025,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_7,
        m_nBank: SFX_BANK_HOTROD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 10000,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 10706,
        m_nSirenOrAlarmSample: SFX_POLICE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 13596,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_BUS,
        m_nHornFrequency: 17260,
        m_nSirenOrAlarmSample: SFX_POLICE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 13000,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 8670,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_8,
        m_nBank: SFX_BANK_COBRA,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 10400,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 10123,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_POLICE_SIREN_SLOW,
        m_nSirenOrAlarmFrequency: 13596,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 11652,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 10554,
        m_bDoorType: BUS_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 29711,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8000,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 28043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 29711,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: BUS_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CESNA_IDLE,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 26513,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_BUS,
        m_nHornFrequency: 16291,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 7500,
        m_bDoorType: BUS_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 10842,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 10233,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 8670,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 8935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 2000,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 17000,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_BMW328,
        m_nHornFrequency: 9003,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_2,
        m_nBank: SFX_BANK_PATHFINDER,
        m_nHornSample: SFX_CAR_HORN_PORSCHE,
        m_nHornFrequency: 12375,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_5,
        m_nBank: SFX_BANK_MERC,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 15554,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_7,
        m_nBank: SFX_BANK_HOTROD,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 13857,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_7,
        m_nBank: SFX_BANK_HOTROD,
        m_nHornSample: SFX_CAR_HORN_PICKUP,
        m_nHornFrequency: 10924,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 20143,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_0_LOGIC,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9000,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_6,
        m_nBank: SFX_BANK_TRUCK,
        m_nHornSample: SFX_CAR_HORN_TRUCK,
        m_nHornFrequency: 28043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_BUS,
        m_nHornFrequency: 18286,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: TRUCK_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_56CHEV,
        m_nHornFrequency: 10842,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_4,
        m_nBank: SFX_BANK_SPIDER,
        m_nHornSample: SFX_CAR_HORN_BUS2,
        m_nHornFrequency: 18000,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 13400,
        m_bDoorType: NEW_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
    tVehicleSampleData {
        m_nAccelerationSampleIndex: SFX_CAR_REV_1,
        m_nBank: SFX_BANK_PACARD,
        m_nHornSample: SFX_CAR_HORN_JEEP,
        m_nHornFrequency: 21043,
        m_nSirenOrAlarmSample: SFX_CAR_ALARM_1,
        m_nSirenOrAlarmFrequency: 9935,
        m_bDoorType: OLD_DOOR,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleStatusForAudio {
    Player,
    Abandoned,
    Simple,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorAudioSnapshot {
    pub swinging: bool,
    pub open_or_closed: bool,
    pub angular_velocity: f32,
}

impl Default for DoorAudioSnapshot {
    fn default() -> Self {
        Self {
            swinging: false,
            open_or_closed: false,
            angular_velocity: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleAudioSnapshot {
    pub audio_entity: i32,
    pub model_index: usize,
    pub position: CVector,
    pub distance_squared: f32,
    pub distance_calculated: bool,
    pub horn_timer: i32,
    pub horn_pattern: usize,
    pub siren_or_alarm: bool,
    pub alarm_on: bool,
    pub status: VehicleStatusForAudio,
    pub engine_on: bool,
    pub gas_pedal: f32,
    pub gas_pedal_audio: f32,
    pub player_accelerate_state: Option<i16>,
    pub previous_velocity_change_for_audio: f32,
    pub velocity_change: f32,
    pub max_velocity: f32,
    pub current_gear: u8,
    pub gear_shift_down_velocity: f32,
    pub first_gear_shift_up_velocity: f32,
    pub max_reverse_velocity: f32,
    pub wheels_on_ground: u8,
    pub drive_wheels_on_ground: u8,
    pub surface_touched: u8,
    pub wet_roads: f32,
    pub skid_value: f32,
    pub train_speed: f32,
    pub train_wagon_position: f32,
    pub boat_is_reefer: bool,
    pub boat_propeller_in_water: bool,
    pub boat_in_water: bool,
    pub boat_anchored: bool,
    pub heli_id: u32,
    pub jumbo_speed: f32,
    pub doors: [DoorAudioSnapshot; 6],
    pub move_speed_z: f32,
    pub handbrake_on: bool,
    pub drive_type: char,
    pub num_gears: u8,
    pub wheel_states:
        [crate::vehicles::Vehicle::tWheelState; crate::vehicles::Automobile::NUM_CAR_WHEELS],
    pub wheel_missing: [bool; crate::vehicles::Automobile::NUM_CAR_WHEELS],
    pub turn_speed_z: f32,
    pub player_engine_channel_used: Option<bool>,
}

#[path = "PlayerEngineAudio.rs"]
mod player_engine_audio;
pub use player_engine_audio::{PlayerCesnaAudioState, PlayerVehicleAudioState, PlayerEngineAudioState,
    process_players_vehicle_engine, process_cesna, add_player_car_sample};
use player_engine_audio::VehicleAudioGlobals;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineDamageAudioState {
    None,
    Damaged,
    OnFire,
}

#[inline]
fn engine_damage_audio_state(status: u32) -> EngineDamageAudioState {
    match status {
        100..=224 => EngineDamageAudioState::Damaged,
        225..=250 => EngineDamageAudioState::OnFire,
        _ => EngineDamageAudioState::None,
    }
}

#[path = "VehicleOneShots.rs"]
mod vehicle_one_shots;
pub use vehicle_one_shots::{VehicleOneShotAudioState, VehicleOneShotOutcome, flush_vehicle_comments, process_vehicle_one_shot};
use vehicle_one_shots::drain_vehicle_one_shots;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedMoveStateForAudio {
    Still,
    Walk,
    Run,
    Sprint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PedAudioSnapshot {
    pub audio_entity: i32,
    pub position: CVector,
    pub distance_squared: f32,
    pub distance_calculated: bool,
    pub surface_touched: u8,
    pub is_in_air: bool,
    pub is_player: bool,
    pub move_state: PedMoveStateForAudio,
    pub weapon_type: eWeaponType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PedOneShotAudioState {
    pub sound_index: u8,
}

impl PedOneShotAudioState {
    pub const fn new() -> Self {
        Self { sound_index: 21 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedOneShotOutcome {
    None,
    Queued,
    RequestPedComment(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PedOneShotMatch {
    Sample(PedOneShotSample),
    Skip,
    PedComment,
}

impl Default for VehicleAudioSnapshot {
    fn default() -> Self {
        Self {
            audio_entity: 0,
            model_index: LANDSTAL,
            position: CVector::ZERO,
            distance_squared: 0.0,
            distance_calculated: false,
            horn_timer: 0,
            horn_pattern: 0,
            siren_or_alarm: false,
            alarm_on: false,
            status: VehicleStatusForAudio::Other,
            engine_on: false,
            gas_pedal: 0.0,
            gas_pedal_audio: 0.0,
            player_accelerate_state: None,
            previous_velocity_change_for_audio: 0.0,
            velocity_change: 0.0,
            max_velocity: 0.0,
            current_gear: 0,
            gear_shift_down_velocity: 0.0,
            first_gear_shift_up_velocity: 0.0,
            max_reverse_velocity: 0.0,
            wheels_on_ground: 0,
            drive_wheels_on_ground: 0,
            surface_touched: eSurfaceType::SURFACE_DEFAULT as u8,
            wet_roads: 0.0,
            skid_value: 0.0,
            train_speed: 0.0,
            train_wagon_position: 0.0,
            boat_is_reefer: false,
            boat_propeller_in_water: true,
            boat_in_water: false,
            boat_anchored: false,
            heli_id: 0,
            jumbo_speed: 0.0,
            doors: [DoorAudioSnapshot::default(); 6],
            move_speed_z: 0.0,
            handbrake_on: false,
            drive_type: '4',
            num_gears: 5,
            wheel_states: [crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL;
                crate::vehicles::Automobile::NUM_CAR_WHEELS],
            wheel_missing: [false; crate::vehicles::Automobile::NUM_CAR_WHEELS],
            turn_speed_z: 0.0,
            player_engine_channel_used: None,
        }
    }
}

// re3 `HornPattern[8][44]`, packed with bit N representing timer slot N.
const HORN_PATTERN_MASKS: [u64; 8] = [
    0xffff81fffc,
    0x3fffffffffc,
    0x7ffff1f0ffc,
    0x7ffc7fe3e7c,
    0x7fc,
    0x71c,
    0x3ff83f39c3c,
    0x7ffc3f1f3c,
];

#[inline]
fn horn_pattern_enabled(pattern: usize, slot: usize) -> bool {
    slot < 44 && (HORN_PATTERN_MASKS[pattern.min(7)] & (1_u64 << slot)) != 0
}

pub fn uses_siren(model: usize) -> bool {
    matches!(
        model,
        FIRETRUK | AMBULAN | FBICAR | POLICE | ENFORCER | PREDATOR
    )
}

pub fn uses_siren_switching(model: usize) -> bool {
    matches!(model, AMBULAN | POLICE | ENFORCER | PREDATOR)
}

pub fn uses_reverse_warning(model: usize) -> bool {
    matches!(model, LINERUN | FIRETRUK | TRASH | BUS | COACH)
}

pub fn has_air_brakes(model: usize) -> bool {
    uses_reverse_warning(model)
}

/// ProcessVehicleHorn's persistent timer/pattern writes occur only after its
/// distance and native model/siren gates; the remaining sample input is readonly.
fn prepare_vehicle_horn(
    audio: &cAudioManager,
    state: &mut crate::vehicles::Vehicle::VehicleAudioState,
    snapshot: &mut VehicleAudioSnapshot,
) {
    if snapshot.distance_squared >= VEHICLE_HORN_MAX_DIST * VEHICLE_HORN_MAX_DIST
        || (snapshot.siren_or_alarm && uses_siren_switching(snapshot.model_index))
        || snapshot.model_index == MRWHOOP
        || snapshot.status == VehicleStatusForAudio::Player
        || state.car_horn_timer == 0 {
        return;
    }
    state.car_horn_timer = state.car_horn_timer.min(44);
    if state.car_horn_timer == 44 {
        state.car_horn_pattern = ((audio.m_FrameCounter as i32 + snapshot.audio_entity) & 7) as u8;
    }
    snapshot.horn_timer = i32::from(state.car_horn_timer);
    snapshot.horn_pattern = usize::from(state.car_horn_pattern);
}

pub fn process_vehicle_horn(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= VEHICLE_HORN_MAX_DIST * VEHICLE_HORN_MAX_DIST {
        return false;
    }
    if snapshot.siren_or_alarm && uses_siren_switching(snapshot.model_index) {
        return true;
    }
    if snapshot.model_index == MRWHOOP {
        return true;
    }
    if snapshot.horn_timer <= 0 {
        return true;
    }
    if snapshot.status != VehicleStatusForAudio::Player {
        let timer = snapshot.horn_timer.min(44).max(1) as usize;
        let pattern = snapshot.horn_pattern.min(7);
        if !horn_pattern_enabled(pattern, 44 - timer) {
            return true;
        }
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(VEHICLE_HORN_VOLUME, VEHICLE_HORN_MAX_DIST, distance);
    if volume > 0 {
        let settings = A_VEHICLE_SETTINGS[snapshot.model_index.min(MAX_CARS - 1)];
        queue_vehicle_sample(
            audio,
            snapshot,
            4,
            settings.m_nHornSample as u32,
            SFX_BANK_0,
            2,
            settings.m_nHornFrequency as u32,
            VEHICLE_HORN_VOLUME,
            volume,
            0,
            5.0,
            VEHICLE_HORN_MAX_DIST,
            false,
            false,
            3.0,
        );
    }
    true
}

pub fn process_vehicle_siren_or_alarm(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
) -> bool {
    if snapshot.distance_squared >= VEHICLE_SIREN_MAX_DIST * VEHICLE_SIREN_MAX_DIST {
        return false;
    }
    if !snapshot.siren_or_alarm && !snapshot.alarm_on {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(VEHICLE_SIREN_VOLUME, VEHICLE_SIREN_MAX_DIST, distance);
    if volume > 0 {
        let settings = A_VEHICLE_SETTINGS[snapshot.model_index.min(MAX_CARS - 1)];
        let mut counter = 5;
        let (sample, frequency) = if uses_siren(snapshot.model_index) {
            if snapshot.status == VehicleStatusForAudio::Abandoned {
                return true;
            }
            if snapshot.horn_timer > 0 && snapshot.model_index != FIRETRUK {
                counter = 60;
                let freq = if snapshot.model_index == FBICAR {
                    16113
                } else {
                    audio.sample_base_frequency(SFX_SIREN_FAST as u32)
                };
                (SFX_SIREN_FAST as u32, freq)
            } else {
                (
                    settings.m_nSirenOrAlarmSample as u32,
                    settings.m_nSirenOrAlarmFrequency as u32,
                )
            }
        } else {
            (
                settings.m_nSirenOrAlarmSample as u32,
                settings.m_nSirenOrAlarmFrequency as u32,
            )
        };
        queue_vehicle_sample(
            audio,
            snapshot,
            counter,
            sample,
            SFX_BANK_0,
            1,
            frequency,
            VEHICLE_SIREN_VOLUME,
            volume,
            0,
            7.0,
            VEHICLE_SIREN_MAX_DIST,
            false,
            false,
            5.0,
        );
    }
    true
}

pub fn process_vehicle_reverse_warning(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
) -> bool {
    if snapshot.distance_squared
        >= VEHICLE_REVERSE_WARNING_MAX_DIST * VEHICLE_REVERSE_WARNING_MAX_DIST
    {
        return false;
    }
    if !snapshot.engine_on || snapshot.gas_pedal >= 0.0 {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(
        VEHICLE_REVERSE_WARNING_VOLUME,
        VEHICLE_REVERSE_WARNING_MAX_DIST,
        distance,
    );
    if volume > 0 {
        let frequency = (100 * audio.m_sQueueSample.m_nEntityIndex as u32 % 1024)
            + audio.sample_base_frequency(SFX_REVERSE_WARNING as u32);
        queue_vehicle_sample(
            audio,
            snapshot,
            12,
            SFX_REVERSE_WARNING as u32,
            SFX_BANK_0,
            2,
            frequency,
            VEHICLE_REVERSE_WARNING_VOLUME,
            volume,
            0,
            3.0,
            VEHICLE_REVERSE_WARNING_MAX_DIST,
            false,
            false,
            3.0,
        );
    }
    true
}

pub fn process_air_brakes(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= AIR_BRAKES_MAX_DIST * AIR_BRAKES_MAX_DIST {
        return false;
    }
    let crossed = (snapshot.previous_velocity_change_for_audio >= 0.025
        && snapshot.velocity_change < 0.025)
        || (snapshot.previous_velocity_change_for_audio <= -0.025
            && snapshot.velocity_change > 0.025);
    if !snapshot.engine_on || !crossed {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let emitting_volume =
        ((audio.m_anRandomTable[0].unsigned_abs() % 10) as u8) + AIR_BRAKES_VOLUME;
    let volume = audio.ComputeVolume(emitting_volume, AIR_BRAKES_MAX_DIST, distance);
    if volume > 0 {
        let base_frequency = audio.sample_base_frequency(SFX_AIR_BRAKES as u32);
        let frequency =
            (base_frequency as i32 + audio.RandomDisplacement(base_frequency >> 4)).max(0) as u32;
        queue_vehicle_sample(
            audio,
            snapshot,
            13,
            SFX_AIR_BRAKES as u32,
            SFX_BANK_0,
            10,
            frequency,
            emitting_volume,
            volume,
            1,
            0.0,
            AIR_BRAKES_MAX_DIST,
            true,
            true,
            0.0,
        );
    }
    true
}

pub fn process_vehicle_doors(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= VEHICLE_DOORS_MAX_DIST * VEHICLE_DOORS_MAX_DIST {
        return false;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    for (i, door) in snapshot.doors.iter().enumerate() {
        if !door.swinging || !door.open_or_closed {
            continue;
        }
        let velocity = door.angular_velocity.abs().min(0.3);
        if velocity <= 0.0035 {
            continue;
        }
        let emitting_volume = (VEHICLE_DOORS_VOLUME as f32 * velocity / 0.3) as u8;
        let volume = audio.ComputeVolume(emitting_volume, VEHICLE_DOORS_MAX_DIST, distance);
        if volume == 0 {
            continue;
        }
        let sample = SFX_COL_CAR_PANEL_1 as u32 + (audio.m_anRandomTable[1].unsigned_abs() % 6);
        let base_frequency = audio.sample_base_frequency(sample);
        let frequency = (base_frequency as i32 + audio.RandomDisplacement(1000)).max(0) as u32;
        queue_vehicle_sample(
            audio,
            snapshot,
            i as u32 + 6,
            sample,
            SFX_BANK_0,
            10,
            frequency,
            emitting_volume,
            volume,
            1,
            1.0,
            VEHICLE_DOORS_MAX_DIST,
            true,
            true,
            0.0,
        );
    }
    true
}

pub fn process_vehicle_road_noise(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
) -> bool {
    if snapshot.distance_squared >= VEHICLE_ROAD_NOISE_MAX_DIST * VEHICLE_ROAD_NOISE_MAX_DIST {
        return false;
    }
    if snapshot.max_velocity <= 0.0 || snapshot.drive_wheels_on_ground == 0 {
        return true;
    }
    let velocity = snapshot.velocity_change.abs();
    if velocity <= 0.0 {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let relative = (velocity / (0.5 * snapshot.max_velocity)).min(1.0);
    let emitting_volume = (VEHICLE_ROAD_NOISE_VOLUME as f32 * relative) as u8;
    let volume = audio.ComputeVolume(emitting_volume, VEHICLE_ROAD_NOISE_MAX_DIST, distance);
    if volume > 0 {
        let (sample, frequency) = if snapshot.surface_touched == eSurfaceType::SURFACE_WATER as u8 {
            (
                SFX_BOAT_WATER_LOOP as u32,
                6050 * emitting_volume as u32 / VEHICLE_ROAD_NOISE_VOLUME as u32 + 16000,
            )
        } else {
            let sample_freq = audio.sample_base_frequency(SFX_ROAD_NOISE as u32);
            let multiplier = (distance / VEHICLE_ROAD_NOISE_MAX_DIST) * 0.5;
            (
                SFX_ROAD_NOISE as u32,
                (sample_freq as f32 * multiplier) as u32 + ((3 * sample_freq) >> 2),
            )
        };
        queue_vehicle_sample(
            audio,
            snapshot,
            0,
            sample,
            SFX_BANK_0,
            3,
            frequency,
            emitting_volume,
            volume,
            0,
            6.0,
            VEHICLE_ROAD_NOISE_MAX_DIST,
            false,
            true,
            4.0,
        );
    }
    true
}

pub fn process_wet_road_noise(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= WET_ROAD_NOISE_MAX_DIST * WET_ROAD_NOISE_MAX_DIST {
        return false;
    }
    if snapshot.max_velocity <= 0.0 || snapshot.drive_wheels_on_ground == 0 {
        return true;
    }
    let vel_change = snapshot.velocity_change.abs();
    if vel_change <= 0.0 {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let relative = (vel_change / (0.5 * snapshot.max_velocity)).min(1.0);
    let emitting_volume = (WET_ROAD_NOISE_VOLUME as f32 * relative * snapshot.wet_roads) as u8;
    let volume = audio.ComputeVolume(emitting_volume, WET_ROAD_NOISE_MAX_DIST, distance);
    if volume > 0 {
        let freq = audio.sample_base_frequency(SFX_ROAD_NOISE as u32);
        let multiplier = (distance / WET_ROAD_NOISE_MAX_DIST) * 0.5;
        queue_vehicle_sample(
            audio,
            snapshot,
            1,
            SFX_ROAD_NOISE as u32,
            SFX_BANK_0,
            3,
            freq + (freq as f32 * multiplier) as u32,
            emitting_volume,
            volume,
            0,
            6.0,
            WET_ROAD_NOISE_MAX_DIST,
            false,
            true,
            4.0,
        );
    }
    true
}

pub fn process_vehicle_engine(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot, gas_pedal_audio: &mut f32) -> bool {
    if snapshot.distance_squared >= VEHICLE_ENGINE_MAX_DIST * VEHICLE_ENGINE_MAX_DIST {
        return false;
    }
    if !snapshot.engine_on {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let Some(settings) = A_VEHICLE_SETTINGS.get(snapshot.model_index) else {
        bevy::log::warn_once!(model = snapshot.model_index, "vehicle_engine_invalid_audio_model");
        return false;
    };
    let (modificator, mut frequency_base) = if snapshot.wheels_on_ground > 0 {
        let mut traction = if snapshot.handbrake_on && snapshot.velocity_change == 0.0 {
            0.9
        } else {
            0.0
        };
        if !snapshot.handbrake_on && snapshot.status != VehicleStatusForAudio::Simple {
            match snapshot.drive_type {
                '4' => {
                    for state in snapshot.wheel_states {
                        if state == crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SPINNING {
                            traction += 0.05;
                        }
                    }
                }
                'F' => {
                    for wheel in [
                        crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT,
                        crate::vehicles::Automobile::CARWHEEL_FRONT_RIGHT,
                    ] {
                        if snapshot.wheel_states[wheel]
                            == crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SPINNING
                        {
                            traction += 0.1;
                        }
                    }
                }
                'R' => {
                    for wheel in [
                        crate::vehicles::Automobile::CARWHEEL_REAR_LEFT,
                        crate::vehicles::Automobile::CARWHEEL_REAR_RIGHT,
                    ] {
                        if snapshot.wheel_states[wheel]
                            == crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SPINNING
                        {
                            traction += 0.1;
                        }
                    }
                }
                _ => {}
            }
        }

        let modificator = if snapshot.max_velocity > 0.0 {
            if snapshot.current_gear > 0 {
                let relative = ((snapshot.velocity_change - snapshot.gear_shift_down_velocity)
                    / snapshot.max_velocity
                    * 2.5)
                    .min(1.0);
                if traction == 0.0
                    && snapshot.status != VehicleStatusForAudio::Simple
                    && snapshot.velocity_change < snapshot.first_gear_shift_up_velocity
                {
                    traction = 0.7;
                }
                traction * *gas_pedal_audio * 0.95 + (1.0 - traction) * relative
            } else if snapshot.max_reverse_velocity != 0.0 {
                (1.0 - ((snapshot.velocity_change - snapshot.gear_shift_down_velocity)
                    / snapshot.max_reverse_velocity)
                    .abs())
                .min(1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        let frequency = if snapshot.current_gear == 0 {
            (13000.0 * modificator + 14000.0).max(1.0) as u32
        } else {
            (1200.0 * f32::from(snapshot.current_gear) + 18000.0 * modificator + 14000.0).max(1.0)
                as u32
        };
        (modificator, frequency)
    } else {
        if snapshot.drive_wheels_on_ground > 0 {
            *gas_pedal_audio *= 0.4;
        }
        let modificator = *gas_pedal_audio;
        (
            modificator,
            (1200.0 * f32::from(snapshot.current_gear) + 18000.0 * modificator + 14000.0).max(1.0)
                as u32,
        )
    };
    let emitting_volume = if modificator < 0.75 {
        (modificator / 0.75 * (VEHICLE_ENGINE_FULL_VOLUME - VEHICLE_ENGINE_BASE_VOLUME) as f32
            + VEHICLE_ENGINE_BASE_VOLUME as f32) as u8
    } else {
        VEHICLE_ENGINE_FULL_VOLUME
    };
    let volume = audio.ComputeVolume(emitting_volume, VEHICLE_ENGINE_MAX_DIST, distance);
    if volume > 0 {
        let idle_sample = settings.m_nBank as i32 - CAR_SFX_BANKS_OFFSET as i32 + SFX_CAR_IDLE_1;
        let (sample, counter) = if snapshot.status == VehicleStatusForAudio::Simple
            && modificator < 0.02
            || snapshot.status != VehicleStatusForAudio::Simple && snapshot.gas_pedal < 0.05
        {
            frequency_base = (modificator * 10000.0) as u32 + 22050;
            (idle_sample.max(0) as u32, 52)
        } else {
            (settings.m_nAccelerationSampleIndex as u32, 2)
        };
        let mut frequency = frequency_base + (100 * snapshot.audio_entity.max(0) as u32 % 1000);
        if sample == SFX_CAR_IDLE_6 as u32 || sample == SFX_CAR_REV_6 as u32 {
            frequency >>= 1;
        }
        queue_vehicle_sample(
            audio,
            snapshot,
            counter,
            sample,
            SFX_BANK_0,
            3,
            frequency,
            emitting_volume,
            volume,
            0,
            6.0,
            VEHICLE_ENGINE_MAX_DIST,
            false,
            true,
            8.0,
        );
    }
    true
}


pub fn process_model_car_engine(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    gas_pedal_audio: &mut f32,
) -> bool {
    if snapshot.distance_squared >= MODEL_CAR_ENGINE_MAX_DIST * MODEL_CAR_ENGINE_MAX_DIST {
        return false;
    }
    if !snapshot.engine_on {
        return true;
    }
    let velocity_change = if snapshot.wheels_on_ground > 0 {
        snapshot.velocity_change.abs()
    } else {
        if snapshot.drive_wheels_on_ground > 0 {
            *gas_pedal_audio *= 0.4;
        }
        *gas_pedal_audio * snapshot.max_velocity
    };
    if velocity_change > 0.001 {
        let allowed_velocity = 0.5 * snapshot.max_velocity;
        let vol = if allowed_velocity > 0.0 && velocity_change < allowed_velocity {
            (MODEL_CAR_ENGINE_VOLUME as f32 * velocity_change / allowed_velocity) as u8
        } else {
            MODEL_CAR_ENGINE_VOLUME
        };
        if vol > 0 {
            let distance = calculate_vehicle_distance(audio, snapshot);
            let volume = audio.ComputeVolume(vol, MODEL_CAR_ENGINE_MAX_DIST, distance);
            if volume > 0 {
                let freq = if snapshot.max_velocity > 0.0 {
                    (11025.0 * velocity_change / snapshot.max_velocity + 11025.0) as u32
                } else {
                    11025
                };
                queue_vehicle_sample(
                    audio,
                    snapshot,
                    2,
                    SFX_REMOTE_CONTROLLED_CAR as u32,
                    SFX_BANK_0,
                    1,
                    freq,
                    vol,
                    volume,
                    0,
                    6.0,
                    MODEL_CAR_ENGINE_MAX_DIST,
                    false,
                    true,
                    4.0,
                );
            }
        }
    }
    true
}

pub fn process_vehicle_skidding(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
) -> bool {
    if snapshot.distance_squared >= VEHICLE_SKIDDING_MAX_DIST * VEHICLE_SKIDDING_MAX_DIST {
        return false;
    }
    if snapshot.wheels_on_ground == 0 || snapshot.max_velocity <= 0.0 {
        return true;
    }
    let mut skid_val = 0.0_f32;
    for wheel in 0..crate::vehicles::Automobile::NUM_CAR_WHEELS {
        if snapshot.wheel_states[wheel] == crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_NORMAL
            || snapshot.wheel_missing[wheel]
        {
            continue;
        }
        let drive_wheel = match snapshot.drive_type {
            '4' => true,
            'F' => matches!(
                wheel,
                crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT
                    | crate::vehicles::Automobile::CARWHEEL_FRONT_RIGHT
            ),
            'R' => matches!(
                wheel,
                crate::vehicles::Automobile::CARWHEEL_REAR_LEFT
                    | crate::vehicles::Automobile::CARWHEEL_REAR_RIGHT
            ),
            _ => false,
        };
        let turn_skid = (snapshot.turn_speed_z.abs() * 20.0).min(1.0);
        let wheel_skid = if drive_wheel {
            match snapshot.wheel_states[wheel] {
                crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SPINNING => {
                    if snapshot.gas_pedal_audio > 0.4 {
                        (snapshot.gas_pedal_audio - 0.4) * (5.0 / 3.0) * 0.75
                    } else {
                        0.0
                    }
                }
                crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SKIDDING => {
                    (snapshot.velocity_change.abs() / snapshot.max_velocity).min(1.0)
                }
                crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_FIXED => {
                    let mut relative_velocity = snapshot.gas_pedal_audio;
                    if relative_velocity > 0.4 {
                        relative_velocity = (snapshot.gas_pedal_audio - 0.4) * (5.0 / 3.0);
                    }
                    let mut relative_velocity_change = 0.0;
                    let velocity_change = snapshot.velocity_change.abs();
                    if velocity_change > 0.04 {
                        relative_velocity_change =
                            (velocity_change / snapshot.max_velocity).min(1.0);
                    }
                    relative_velocity.max(relative_velocity_change)
                }
                _ => 0.0,
            }
        } else if snapshot.wheel_states[wheel]
            == crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SKIDDING
        {
            (snapshot.velocity_change.abs() / snapshot.max_velocity).min(1.0)
        } else {
            0.0
        };
        skid_val = skid_val.max(wheel_skid.max(turn_skid));
    }
    if skid_val <= 0.0 {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let skid_val = skid_val.clamp(0.0, 1.0);
    let mut emitting_volume = (VEHICLE_SKIDDING_VOLUME as f32 * skid_val) as u8;
    let mut volume = audio.ComputeVolume(emitting_volume, VEHICLE_SKIDDING_MAX_DIST, distance);
    if volume == 0 {
        return true;
    }
    let (sample, frequency) = match snapshot.surface_touched {
        x if x == eSurfaceType::SURFACE_GRASS as u8 || x == eSurfaceType::SURFACE_HEDGE as u8 => {
            emitting_volume >>= 2;
            volume >>= 2;
            if volume == 0 {
                return true;
            }
            (SFX_RAIN as u32, (13000.0 * skid_val + 35000.0) as u32)
        }
        x if x == eSurfaceType::SURFACE_GRAVEL as u8
            || x == eSurfaceType::SURFACE_MUD_DRY as u8
            || x == eSurfaceType::SURFACE_SAND as u8
            || x == eSurfaceType::SURFACE_WATER as u8 =>
        {
            (SFX_GRAVEL_SKID as u32, (6000.0 * skid_val + 10000.0) as u32)
        }
        _ => (SFX_SKID as u32, (5000.0 * skid_val + 11000.0) as u32),
    };
    queue_vehicle_sample(
        audio,
        snapshot,
        3,
        sample,
        SFX_BANK_0,
        8,
        frequency,
        emitting_volume,
        volume,
        0,
        3.0,
        VEHICLE_SKIDDING_MAX_DIST,
        false,
        true,
        3.0,
    );
    true
}


pub fn process_engine_damage(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    engine_on: bool,
    damage: EngineDamageAudioState,
) -> bool {
    if snapshot.distance_squared >= ENGINE_DAMAGE_MAX_DIST * ENGINE_DAMAGE_MAX_DIST {
        return false;
    }
    if !engine_on {
        return true;
    }
    let (sample, emitting_volume, frequency) = match damage {
        EngineDamageAudioState::None => return true,
        EngineDamageAudioState::OnFire => (
            SFX_CAR_ON_FIRE as u32,
            ENGINE_DAMAGE_ON_FIRE_VOLUME,
            audio.sample_base_frequency(SFX_CAR_ON_FIRE as u32),
        ),
        EngineDamageAudioState::Damaged => (SFX_JUMBO_TAXI as u32, ENGINE_DAMAGE_VOLUME, 40000),
    };
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(emitting_volume, ENGINE_DAMAGE_MAX_DIST, distance);
    if volume > 0 {
        queue_vehicle_sample(
            audio,
            snapshot,
            28,
            sample,
            SFX_BANK_0,
            7,
            frequency,
            emitting_volume,
            volume,
            0,
            2.0,
            ENGINE_DAMAGE_MAX_DIST,
            false,
            true,
            3.0,
        );
    }
    true
}

pub fn process_car_bomb_tick(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    engine_on: bool,
    timed_bomb_active: bool,
) -> bool {
    if snapshot.distance_squared >= CAR_BOMB_TICK_MAX_DIST * CAR_BOMB_TICK_MAX_DIST {
        return false;
    }
    if !engine_on || !timed_bomb_active {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let volume = audio.ComputeVolume(CAR_BOMB_TICK_VOLUME, CAR_BOMB_TICK_MAX_DIST, distance);
    if volume > 0 {
        queue_vehicle_sample(
            audio,
            snapshot,
            35,
            SFX_COUNTDOWN as u32,
            SFX_BANK_0,
            0,
            audio.sample_base_frequency(SFX_COUNTDOWN as u32),
            CAR_BOMB_TICK_VOLUME,
            volume,
            0,
            2.0,
            CAR_BOMB_TICK_MAX_DIST,
            false,
            true,
            3.0,
        );
    }
    true
}

/// re3 cAudioManager::ProcessReverseGear (AudioLogic.cpp).
/// Reverse-gear whine while the engine runs backwards or idles in gear 0.
pub fn process_reverse_gear(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    gas_pedal_audio: &mut f32,
) -> bool {
    if snapshot.distance_squared >= REVERSE_GEAR_MAX_DIST * REVERSE_GEAR_MAX_DIST {
        return false;
    }
    if !snapshot.engine_on || !(snapshot.gas_pedal < 0.0 || snapshot.current_gear == 0) {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    if snapshot.wheels_on_ground == 0 && snapshot.drive_wheels_on_ground > 0 {
        *gas_pedal_audio *= 0.4;
    }
    let modificator = if snapshot.wheels_on_ground > 0 {
        if snapshot.max_reverse_velocity != 0.0 {
            snapshot.velocity_change / snapshot.max_reverse_velocity
        } else {
            0.0
        }
    } else {
        *gas_pedal_audio
    }
    .abs();
    let emitting_volume = (REVERSE_GEAR_VOLUME as f32 * modificator) as u8;
    let volume = audio.ComputeVolume(emitting_volume, REVERSE_GEAR_MAX_DIST, distance);
    if volume == 0 {
        return true;
    }
    let (counter, sample) = if snapshot.gas_pedal < 0.0 {
        (61, SFX_REVERSE_GEAR as u32)
    } else {
        (62, SFX_REVERSE_GEAR_2 as u32)
    };
    queue_vehicle_sample(
        audio,
        snapshot,
        counter,
        sample,
        SFX_BANK_0,
        3,
        (6000.0 * modificator) as u32 + 7000,
        emitting_volume,
        volume,
        0,
        3.0,
        REVERSE_GEAR_MAX_DIST,
        false,
        true,
        5.0,
    );
    true
}

/// re3 cAudioManager::ProcessRainOnVehicle (AudioLogic.cpp).
/// Rain patter on the roof, sampled at most every second frame per vehicle.
pub fn process_rain_on_vehicle(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    rain: f32,
    rain_counter: &mut u8,
    rain_samples_counter: &mut u8,
) -> bool {
    if snapshot.distance_squared >= RAIN_ON_VEHICLE_MAX_DIST * RAIN_ON_VEHICLE_MAX_DIST
        || rain <= 0.01
    {
        return true;
    }
    *rain_counter = rain_counter.wrapping_add(1);
    if *rain_counter < 2 {
        return true;
    }
    *rain_counter = 0;
    let distance = calculate_vehicle_distance(audio, snapshot);
    let emitting_volume = (RAIN_ON_VEHICLE_VOLUME as f32 * rain) as u8;
    let volume = audio.ComputeVolume(emitting_volume, RAIN_ON_VEHICLE_MAX_DIST, distance);
    if volume == 0 {
        return true;
    }
    let counter = u32::from(*rain_samples_counter);
    *rain_samples_counter = rain_samples_counter.wrapping_add(1);
    if *rain_samples_counter > 4 {
        *rain_samples_counter = 68;
    }
    let sample = audio.m_anRandomTable[1].rem_euclid(4) as u32 + SFX_CAR_RAIN_1 as u32;
    queue_vehicle_sample(
        audio,
        snapshot,
        counter,
        sample,
        SFX_BANK_0,
        9,
        audio.m_anRandomTable[1].rem_euclid(4000) as u32 + 28_000,
        emitting_volume,
        volume,
        1,
        0.0,
        RAIN_ON_VEHICLE_MAX_DIST,
        true,
        false,
        0.0,
    );
    true
}

pub fn process_train_noise(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= TRAIN_NOISE_FAR_MAX_DIST * TRAIN_NOISE_FAR_MAX_DIST {
        return false;
    }
    if snapshot.velocity_change <= 0.0 {
        return true;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    let speed_multiplier = (snapshot.train_speed * 250.0 / 51.0).min(1.0);
    let emitting_volume = (TRAIN_NOISE_VOLUME as f32 * speed_multiplier) as u8;
    if snapshot.train_wagon_position == 0.0 {
        let volume = audio.ComputeVolume(emitting_volume, TRAIN_NOISE_FAR_MAX_DIST, distance);
        if volume > 0 {
            queue_vehicle_sample(
                audio,
                snapshot,
                32,
                SFX_TRAIN_FAR as u32,
                SFX_BANK_0,
                2,
                audio.sample_base_frequency(SFX_TRAIN_FAR as u32),
                emitting_volume,
                volume,
                0,
                3.0,
                TRAIN_NOISE_FAR_MAX_DIST,
                false,
                true,
                3.0,
            );
        }
    }
    if snapshot.distance_squared < TRAIN_NOISE_NEAR_MAX_DIST * TRAIN_NOISE_NEAR_MAX_DIST {
        let volume = audio.ComputeVolume(emitting_volume, TRAIN_NOISE_NEAR_MAX_DIST, distance);
        if volume > 0 {
            queue_vehicle_sample(
                audio,
                snapshot,
                33,
                SFX_TRAIN_NEAR as u32,
                SFX_BANK_0,
                5,
                audio.sample_base_frequency(SFX_TRAIN_NEAR as u32)
                    + 100 * snapshot.audio_entity.max(0) as u32 % 987,
                emitting_volume,
                volume,
                0,
                6.0,
                TRAIN_NOISE_NEAR_MAX_DIST,
                false,
                true,
                3.0,
            );
        }
    }
    true
}

pub fn process_boat_engine(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    if snapshot.distance_squared >= BOAT_ENGINE_MAX_DIST * BOAT_ENGINE_MAX_DIST {
        return false;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    if snapshot.boat_is_reefer {
        let idle_volume = audio.ComputeVolume(
            BOAT_ENGINE_REEFER_IDLE_VOLUME,
            BOAT_ENGINE_MAX_DIST,
            distance,
        );
        if idle_volume > 0 {
            queue_vehicle_sample(
                audio,
                snapshot,
                39,
                SFX_FISHING_BOAT_IDLE as u32,
                SFX_BANK_0,
                3,
                10386 + ((snapshot.audio_entity.max(0) as u32) << 16) % 1000,
                BOAT_ENGINE_REEFER_IDLE_VOLUME,
                idle_volume,
                0,
                2.0,
                BOAT_ENGINE_MAX_DIST,
                false,
                true,
                7.0,
            );
        }
        let gas = snapshot.gas_pedal.abs();
        let emitting_volume = (BOAT_ENGINE_REEFER_ACCEL_VOLUME_MULT as f32 * gas) as u8
            + BOAT_ENGINE_REEFER_ACCEL_MIN_VOLUME;
        let mut frequency = (3000.0 * gas) as u32 + 6000;
        if !snapshot.boat_propeller_in_water {
            frequency = 11 * frequency / 10;
        }
        queue_boat_accel(audio, snapshot, emitting_volume, frequency, distance);
    } else {
        let gas = snapshot.gas_pedal.abs();
        let (emitting_volume, frequency, counter, sample) = if gas > 0.0 {
            let mut frequency = (4000.0 * gas) as u32 + 8000;
            if !snapshot.boat_anchored {
                frequency = 11 * frequency / 10;
            }
            (
                (BOAT_ENGINE_HIGH_ACCEL_VOLUME_MULT as f32 * gas) as u8
                    + BOAT_ENGINE_HIGH_ACCEL_MIN_VOLUME,
                frequency,
                40,
                SFX_POLICE_BOAT_ACCEL as u32,
            )
        } else {
            (
                BOAT_ENGINE_LOW_ACCEL_VOLUME,
                11025,
                39,
                SFX_POLICE_BOAT_IDLE as u32,
            )
        };
        let volume = audio.ComputeVolume(emitting_volume, BOAT_ENGINE_MAX_DIST, distance);
        if volume > 0 {
            queue_vehicle_sample(
                audio,
                snapshot,
                counter,
                sample,
                SFX_BANK_0,
                3,
                frequency + ((snapshot.audio_entity.max(0) as u32) << 16) % 1000,
                emitting_volume,
                volume,
                0,
                2.0,
                BOAT_ENGINE_MAX_DIST,
                false,
                true,
                7.0,
            );
        }
    }
    true
}

fn queue_boat_accel(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    emitting_volume: u8,
    frequency: u32,
    distance: f32,
) {
    let volume = audio.ComputeVolume(emitting_volume, BOAT_ENGINE_MAX_DIST, distance);
    if volume > 0 {
        queue_vehicle_sample(
            audio,
            snapshot,
            40,
            SFX_POLICE_BOAT_ACCEL as u32,
            SFX_BANK_0,
            3,
            frequency + ((snapshot.audio_entity.max(0) as u32) << 16) % 1000,
            emitting_volume,
            volume,
            0,
            2.0,
            BOAT_ENGINE_MAX_DIST,
            false,
            true,
            7.0,
        );
    }
}

pub fn process_boat_moving_over_water(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
) -> bool {
    if snapshot.distance_squared
        >= BOAT_MOVING_OVER_WATER_MAX_DIST * BOAT_MOVING_OVER_WATER_MAX_DIST
    {
        return false;
    }
    let velocity = snapshot.velocity_change.abs();
    if velocity <= 0.0005 || !snapshot.boat_in_water {
        return true;
    }
    let velocity = velocity.min(0.75);
    let multiplier = (velocity - 0.0005) / (1499.0 / 2000.0);
    let distance = calculate_vehicle_distance(audio, snapshot);
    let emitting_volume = (BOAT_MOVING_OVER_WATER_VOLUME as f32 * multiplier) as u8;
    let volume = audio.ComputeVolume(emitting_volume, BOAT_MOVING_OVER_WATER_MAX_DIST, distance);
    if volume > 0 {
        queue_vehicle_sample(
            audio,
            snapshot,
            38,
            SFX_BOAT_WATER_LOOP as u32,
            SFX_BANK_0,
            3,
            (6050.0 * multiplier + 16000.0) as u32,
            emitting_volume,
            volume,
            0,
            2.0,
            BOAT_MOVING_OVER_WATER_MAX_DIST,
            false,
            true,
            3.0,
        );
    }
    true
}

pub fn process_helicopter(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> bool {
    // Native tHelicopterSampleData / gHeliSfxRanges.
    struct HelicopterSampleRange {
        max_distance: f32,
        base_distance: f32,
        base_volume: u8,
    }
    const HELI_RANGES: [HelicopterSampleRange; 3] = [
        HelicopterSampleRange { max_distance: 400.0, base_distance: 380.0, base_volume: 100 },
        HelicopterSampleRange { max_distance: 100.0, base_distance: 70.0, base_volume: MAX_VOLUME },
        HelicopterSampleRange { max_distance: 60.0, base_distance: 30.0, base_volume: MAX_VOLUME },
    ];
    if snapshot.distance_squared >= HELI_RANGES[0].max_distance.powi(2) {
        return false;
    }
    let distance = calculate_vehicle_distance(audio, snapshot);
    for (i, range) in HELI_RANGES.iter().enumerate() {
        let max_dist = range.max_distance;
        let base_dist = range.base_distance;
        let base_volume = range.base_volume;
        if distance >= max_dist {
            return true;
        }
        let emitting_volume = if distance < base_dist {
            base_volume
        } else {
            (base_volume as f32 * ((max_dist - distance) / (max_dist - base_dist))) as u8
        };
        let volume = audio.ComputeVolume(emitting_volume, max_dist, distance);
        if volume > 0 {
            let sample = SFX_HELI_1 as u32 + i as u32;
            queue_vehicle_sample(
                audio,
                snapshot,
                i as u32 + 65,
                sample,
                SFX_BANK_0,
                0,
                1200 * snapshot.heli_id + audio.sample_base_frequency(sample),
                emitting_volume,
                volume,
                0,
                6.0,
                max_dist,
                false,
                true,
                3.0,
            );
        }
    }
    true
}

#[path = "JumboAudio.rs"]
mod jumbo_audio;
pub use jumbo_audio::{JumboAudioPhase, JumboAudioState, JumboPathInput, jumbo_phase, process_jumbo};

fn calculate_vehicle_distance(audio: &mut cAudioManager, snapshot: &VehicleAudioSnapshot) -> f32 {
    let distance = if snapshot.distance_calculated {
        snapshot.distance_squared
    } else {
        snapshot.distance_squared.sqrt()
    };
    audio.m_sQueueSample.m_fDistance = distance;
    distance
}

#[allow(clippy::too_many_arguments)]
fn queue_vehicle_sample(
    audio: &mut cAudioManager,
    snapshot: &VehicleAudioSnapshot,
    counter: u32,
    sample: u32,
    bank: u8,
    priority: u32,
    frequency: u32,
    emitting_volume: u8,
    volume: u8,
    loop_count: u32,
    speed_multiplier: f32,
    max_distance: f32,
    static_sample: bool,
    reverb: bool,
    frames_to_play: f32,
) {
    audio.m_sQueueSample.m_nEntityIndex = snapshot.audio_entity;
    audio.m_sQueueSample.m_nCounter = counter;
    audio.m_sQueueSample.m_nSampleIndex = sample;
    audio.m_sQueueSample.m_nBankIndex = bank;
    audio.m_sQueueSample.m_bIs2D = false;
    audio.m_sQueueSample.m_nPriority = priority;
    audio.m_sQueueSample.m_nFrequency = frequency;
    audio.m_sQueueSample.m_nVolume = volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nEmittingVolume = emitting_volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nLoopCount = loop_count;
    audio.m_sQueueSample.m_nLoopStart = 0;
    audio.m_sQueueSample.m_nLoopEnd = -1;
    audio.m_sQueueSample.m_fSpeedMultiplier = speed_multiplier;
    audio.m_sQueueSample.m_MaxDistance = max_distance;
    audio.m_sQueueSample.m_bStatic = static_sample;
    audio.m_sQueueSample.m_vecPos = snapshot.position;
    audio.m_sQueueSample.m_bReverb = reverb;
    audio.m_sQueueSample.m_nFramesToPlay = frames_to_play;
    audio.AddSampleToRequestedQueue();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireAudioKind {
    Default,
    Building,
    Ped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplosionAudioKind {
    Grenade,
    Rocket,
    Barrel,
    TankGrenade,
    Mine,
    HeliBomb,
    Molotov,
    Heli,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionalAudioSnapshot {
    pub audio_entity: i32,
    pub index: u32,
    pub position: CVector,
    pub distance_squared: f32,
    pub distance_calculated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub kind: FireAudioKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub kind: ExplosionAudioKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherAudioSnapshot {
    pub audio_entity: i32,
    pub lightning_volume: Option<f32>,
    pub rain: f32,
    pub rain_allowed: bool,
    pub camera_sound_dist_up: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendAudioContext {
    pub frame_counter: u32,
    pub pickup_next_frame: u32,
    pub part_mission_next_frame: u32,
    pub counter: u8,
}

impl FrontendAudioContext {
    pub const fn new() -> Self {
        Self {
            frame_counter: 0,
            pickup_next_frame: 0,
            part_mission_next_frame: 0,
            counter: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraneAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub activated: bool,
    pub moving: bool,
    pub collision_event: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileAudioKind {
    Rocket,
    Molotov,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub kind: ProjectileAudioKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarageAudioState {
    Other,
    Opening,
    Closing,
    AfterDropoff,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GarageAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub state: GarageAudioState,
    pub crusher: bool,
    pub door_opened_event: bool,
    pub door_closed_event: bool,
    pub frame_counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeAudioState {
    Other,
    Locked,
    LiftPartIsUp,
    LiftPartIsDown,
    LiftPartAboutToMoveUp,
    LiftPartMovingUp,
    LiftPartMovingDown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BridgeAudioSnapshot {
    pub source: PositionalAudioSnapshot,
    pub state: BridgeAudioState,
    pub old_state: BridgeAudioState,
    pub commercial_passed: bool,
}

pub fn process_explosion(audio: &mut cAudioManager, snapshot: &ExplosionAudioSnapshot) -> bool {
    let (sample, bank, mut frequency, max_distance) = match snapshot.kind {
        ExplosionAudioKind::Grenade
        | ExplosionAudioKind::Rocket
        | ExplosionAudioKind::Barrel
        | ExplosionAudioKind::TankGrenade => (
            SFX_EXPLOSION_2 as u32,
            SFX_BANK_0,
            displaced_frequency(audio, 38000, 2000),
            EXPLOSION_DEFAULT_MAX_DIST,
        ),
        ExplosionAudioKind::Mine | ExplosionAudioKind::HeliBomb => (
            SFX_ROCKET_LEFT as u32,
            SFX_BANK_0,
            displaced_frequency(audio, 12347, 1000),
            EXPLOSION_MINE_MAX_DIST,
        ),
        ExplosionAudioKind::Molotov => (
            SFX_EXPLOSION_3 as u32,
            SFX_BANK_0,
            displaced_frequency(audio, 19000, 1000),
            EXPLOSION_MOLOTOV_MAX_DIST,
        ),
        ExplosionAudioKind::Heli | ExplosionAudioKind::Other => (
            SFX_EXPLOSION_1 as u32,
            SFX_BANK_GENERIC_EXTRA,
            displaced_frequency(audio, 38000, 2000),
            EXPLOSION_DEFAULT_MAX_DIST,
        ),
    };
    if snapshot.kind == ExplosionAudioKind::Heli {
        frequency = 8 * frequency / 10;
    }
    queue_positional_sample(
        audio,
        &snapshot.source,
        sample,
        bank,
        0,
        frequency,
        MAX_VOLUME,
        max_distance,
        1,
        2.0,
        true,
        true,
        0.0,
    )
}

pub fn process_fire(audio: &mut cAudioManager, snapshot: &FireAudioSnapshot) -> bool {
    let (sample, max_distance, emitting_volume, frequency, priority) = match snapshot.kind {
        FireAudioKind::Building => {
            let base = audio.sample_base_frequency(SFX_CAR_ON_FIRE as u32);
            (
                SFX_CAR_ON_FIRE as u32,
                FIRE_BUILDING_MAX_DIST,
                FIRE_BUILDING_VOLUME,
                8 * base / 10 + snapshot.source.index * ((8 * base / 10).max(1) >> 6),
                6,
            )
        }
        FireAudioKind::Ped => {
            let base = audio.sample_base_frequency(SFX_PED_ON_FIRE as u32);
            (
                SFX_PED_ON_FIRE as u32,
                FIRE_PED_MAX_DIST,
                FIRE_PED_VOLUME,
                base + snapshot.source.index * (base.max(1) >> 6),
                10,
            )
        }
        FireAudioKind::Default => {
            let base = audio.sample_base_frequency(SFX_CAR_ON_FIRE as u32);
            (
                SFX_CAR_ON_FIRE as u32,
                FIRE_DEFAULT_MAX_DIST,
                FIRE_DEFAULT_VOLUME,
                base + snapshot.source.index * (base.max(1) >> 6),
                8,
            )
        }
    };

    queue_positional_sample(
        audio,
        &snapshot.source,
        sample,
        SFX_BANK_0,
        priority,
        frequency,
        emitting_volume,
        max_distance,
        0,
        2.0,
        false,
        true,
        10.0,
    )
}

pub fn process_water_cannon(audio: &mut cAudioManager, snapshot: &PositionalAudioSnapshot) -> bool {
    queue_positional_sample(
        audio,
        snapshot,
        SFX_JUMBO_TAXI as u32,
        SFX_BANK_0,
        5,
        15591,
        WATER_CANNON_VOLUME,
        WATER_CANNON_MAX_DIST,
        0,
        2.0,
        false,
        true,
        8.0,
    )
}

pub fn process_weather(audio: &mut cAudioManager, snapshot: &WeatherAudioSnapshot) -> bool {
    let mut queued = false;
    if let Some(lightning_volume) = snapshot.lightning_volume {
        let (sample, bank, mut volume) = if lightning_volume < 10.0 {
            (
                SFX_EXPLOSION_2 as u32,
                SFX_BANK_0,
                (lightning_volume as u8).saturating_add(35),
            )
        } else {
            (
                SFX_EXPLOSION_1 as u32,
                SFX_BANK_GENERIC_EXTRA,
                ((lightning_volume - 10.0) as u8).saturating_add(40),
            )
        };
        if snapshot.camera_sound_dist_up < 20.0 {
            volume >>= 1;
        }
        audio.m_sQueueSample.m_nEntityIndex = snapshot.audio_entity;
        audio.m_sQueueSample.m_nCounter = random_mod(audio, 2, 4);
        audio.m_sQueueSample.m_nSampleIndex = sample;
        audio.m_sQueueSample.m_nBankIndex = bank;
        audio.m_sQueueSample.m_nFrequency = displaced_frequency(audio, 4000, 500);
        audio.m_sQueueSample.m_nVolume = volume.min(MAX_VOLUME);
        audio.m_sQueueSample.m_nPriority = 0;
        audio.m_sQueueSample.m_nPan = (random_mod(audio, 2, 16) as u8) + 55;
        audio.m_sQueueSample.m_bIs2D = true;
        audio.m_sQueueSample.m_nLoopCount = 1;
        audio.m_sQueueSample.m_bStatic = true;
        audio.m_sQueueSample.m_nLoopStart = 0;
        audio.m_sQueueSample.m_nLoopEnd = -1;
        audio.m_sQueueSample.m_bReverb = false;
        audio.AddSampleToRequestedQueue();
        queued = true;
    }

    if snapshot.rain > 0.0 && snapshot.rain_allowed {
        let volume = (25.0 * snapshot.rain).clamp(0.0, MAX_VOLUME as f32) as u8;
        if volume > 0 {
            audio.m_sQueueSample.m_nEntityIndex = snapshot.audio_entity;
            audio.m_sQueueSample.m_nCounter = 4;
            audio.m_sQueueSample.m_nSampleIndex = SFX_RAIN as u32;
            audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_RAIN as u32);
            audio.m_sQueueSample.m_nVolume = volume;
            audio.m_sQueueSample.m_nPriority = 0;
            audio.m_sQueueSample.m_nPan = 63;
            audio.m_sQueueSample.m_bIs2D = true;
            audio.m_sQueueSample.m_nLoopCount = 0;
            audio.m_sQueueSample.m_bStatic = false;
            audio.m_sQueueSample.m_nFramesToPlay = 30.0;
            audio.m_sQueueSample.m_bReverb = false;
            audio.AddSampleToRequestedQueue();
            queued = true;
        }
    }

    queued
}

pub fn process_frontend_sound(
    audio: &mut cAudioManager,
    audio_entity: i32,
    sound: i16,
    event_volume: f32,
    context: &mut FrontendAudioContext,
) -> bool {
    let Some(frontend) = frontend_sample_for_sound(audio, sound) else {
        return false;
    };

    if frontend.processed_pickup {
        if context.frame_counter <= context.pickup_next_frame {
            return false;
        }
        context.pickup_next_frame = context.frame_counter + 5;
    } else if frontend.processed_mission {
        if context.frame_counter <= context.part_mission_next_frame {
            return false;
        }
        context.part_mission_next_frame = context.frame_counter + 5;
    }

    let mut frequency = if sound == SFX_RAIN as i16 {
        28509
    } else if frontend.sample == SFX_PICKUP_1_LEFT as u32 {
        if event_volume == 1.0 { 48000 } else { 32000 }
    } else {
        audio.sample_base_frequency(frontend.sample)
    };

    if frequency == 0 {
        frequency = audio.sample_base_frequency(frontend.sample);
    }

    queue_frontend_sample(
        audio,
        audio_entity,
        context.counter,
        frontend.sample,
        frontend.bank,
        frequency,
        frontend.stereo,
        false,
    );
    context.counter = context.counter.wrapping_add(1);

    if frontend.stereo {
        let left_pan = audio.m_sQueueSample.m_nPan;
        queue_frontend_sample(
            audio,
            audio_entity,
            context.counter,
            frontend.sample + 1,
            frontend.bank,
            frequency,
            true,
            true,
        );
        audio.m_sQueueSample.m_nPan = 127 - left_pan;
        context.counter = context.counter.wrapping_add(1);
    }

    true
}

pub fn process_ped_one_shot(
    audio: &mut cAudioManager,
    snapshot: &PedAudioSnapshot,
    sound: i16,
    state: &mut PedOneShotAudioState,
) -> PedOneShotOutcome {
    let mapped = match match_ped_one_shot(audio, snapshot, sound, state) {
        PedOneShotMatch::Sample(mapped) => mapped,
        PedOneShotMatch::Skip => return PedOneShotOutcome::None,
        PedOneShotMatch::PedComment => return PedOneShotOutcome::RequestPedComment(sound as u16),
    };
    if snapshot.distance_squared >= mapped.max_distance * mapped.max_distance {
        return PedOneShotOutcome::None;
    }
    let distance = calculate_ped_distance(audio, snapshot);
    let volume = audio.ComputeVolume(mapped.emitting_volume, mapped.max_distance, distance);
    if volume == 0 {
        return PedOneShotOutcome::None;
    }
    queue_ped_sample(audio, snapshot, mapped, volume);
    if mapped.stereo && distance < 0.2 * mapped.max_distance {
        queue_ped_stereo_pair(audio, mapped, state, sound, snapshot.weapon_type);
    }
    PedOneShotOutcome::Queued
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PedOneShotSample {
    sample: u32,
    counter: u32,
    frequency: u32,
    priority: u32,
    emitting_volume: u8,
    max_distance: f32,
    speed_multiplier: f32,
    loop_count: u32,
    static_sample: bool,
    frames_to_play: f32,
    stereo: bool,
    flamethrower_weapon_shot: bool,
}

fn match_ped_one_shot(
    audio: &cAudioManager,
    snapshot: &PedAudioSnapshot,
    sound: i16,
    state: &mut PedOneShotAudioState,
) -> PedOneShotMatch {
    let mut mapped = match sound {
        x if x == SOUND_FALL_LAND as i16 || x == SOUND_FALL_COLLAPSE as i16 => {
            if snapshot.is_in_air {
                return PedOneShotMatch::Skip;
            }
            let sample = if snapshot.surface_touched == eSurfaceType::SURFACE_WATER as u8 {
                SFX_FOOTSTEP_WATER_1 as u32 + random_mod(audio, 3, 4)
            } else if sound == SOUND_FALL_LAND as i16 {
                SFX_BODY_LAND as u32
            } else {
                SFX_BODY_LAND_AND_FALL as u32
            };
            let base = audio.sample_base_frequency(sample);
            PedOneShotSample {
                sample,
                counter: 1,
                frequency: displaced_frequency(audio, base as i32, base / 17),
                priority: 2,
                emitting_volume: random_mod(audio, 3, 20) as u8 + PED_ONE_SHOT_FALL_VOLUME,
                max_distance: PED_ONE_SHOT_FALL_MAX_DIST,
                speed_multiplier: 0.0,
                loop_count: 1,
                static_sample: true,
                frames_to_play: 0.0,
                stereo: false,
                flamethrower_weapon_shot: false,
            }
        }
        x if x == SOUND_STEP_START as i16 || x == SOUND_STEP_END as i16 => {
            if snapshot.is_in_air {
                return PedOneShotMatch::Skip;
            }
            let mut emitting_volume = random_mod(audio, 3, 15) as u8 + PED_ONE_SHOT_STEP_VOLUME;
            if !snapshot.is_player {
                emitting_volume >>= 1;
            }
            let sample = ped_footstep_sample(audio, snapshot.surface_touched);
            let base = audio.sample_base_frequency(sample);
            let mut frequency = displaced_frequency(audio, base as i32, base / 17);
            match snapshot.move_state {
                PedMoveStateForAudio::Walk => {
                    emitting_volume >>= 2;
                    frequency = 9 * frequency / 10;
                }
                PedMoveStateForAudio::Run => {
                    emitting_volume >>= 1;
                    frequency = 11 * frequency / 10;
                }
                PedMoveStateForAudio::Sprint => {
                    frequency = 12 * frequency / 10;
                }
                PedMoveStateForAudio::Still => {}
            }
            PedOneShotSample {
                sample,
                counter: (sound - SOUND_STEP_START as i16 + 1) as u32,
                frequency,
                priority: 5,
                emitting_volume,
                max_distance: PED_ONE_SHOT_STEP_MAX_DIST,
                speed_multiplier: 0.0,
                loop_count: 1,
                static_sample: true,
                frames_to_play: 0.0,
                stereo: false,
                flamethrower_weapon_shot: false,
            }
        }
        x if x == SOUND_WEAPON_AK47_BULLET_ECHO as i16
            || x == SOUND_WEAPON_UZI_BULLET_ECHO as i16
            || x == SOUND_WEAPON_M16_BULLET_ECHO as i16 =>
        {
            let sample = SFX_UZI_END_LEFT as u32;
            next_ped_indexed_sample(
                audio,
                state,
                sample,
                PED_ONE_SHOT_WEAPON_BULLET_ECHO_VOLUME + random_mod(audio, 4, 10) as u8,
                PED_ONE_SHOT_WEAPON_BULLET_ECHO_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(sample) >> 4,
            )
        }
        x if x == SOUND_WEAPON_FLAMETHROWER_FIRE as i16 => {
            let sample = SFX_FLAMETHROWER_START_LEFT as u32;
            next_ped_indexed_sample(
                audio,
                state,
                sample,
                PED_ONE_SHOT_WEAPON_FLAMETHROWER_FIRE_VOLUME,
                PED_ONE_SHOT_WEAPON_FLAMETHROWER_FIRE_MAX_DIST,
                3,
                4.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(sample) >> 4,
            )
        }
        x if x == SOUND_WEAPON_SHOT_FIRED as i16 => match match_weapon_shot(audio, snapshot, state)
        {
            Some(sample) => sample,
            None => return PedOneShotMatch::Skip,
        },
        x if x == SOUND_WEAPON_RELOAD as i16 => match match_weapon_reload(audio, snapshot, state) {
            Some(sample) => sample,
            None => return PedOneShotMatch::Skip,
        },
        x if x == SOUND_WEAPON_HIT_PED as i16 => {
            let sample = SFX_BULLET_PED as u32;
            next_ped_indexed_sample(
                audio,
                state,
                sample,
                random_mod(audio, 0, 20) as u8 + PED_ONE_SHOT_WEAPON_HIT_PED_VOLUME,
                PED_ONE_SHOT_WEAPON_HIT_PED_MAX_DIST,
                7,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(sample) >> 3,
            )
        }
        x if x == SOUND_WEAPON_BAT_ATTACK as i16 => next_ped_indexed_sample_with_frequency(
            audio,
            state,
            SFX_BAT_HIT_LEFT as u32,
            displaced_frequency(audio, 22000, 2000),
            random_mod(audio, 2, 20) as u8 + PED_ONE_SHOT_PUNCH_VOLUME,
            PED_ONE_SHOT_PUNCH_MAX_DIST,
            3,
        ),
        x if (SOUND_FIGHT_PUNCH_33 as i16..=SOUND_FIGHT_KICK_44 as i16).contains(&x) => {
            let (sample, frequency) = fight_sample_and_frequency(sound);
            next_ped_indexed_sample_with_frequency(
                audio,
                state,
                sample,
                frequency,
                random_mod(audio, 3, 26) as u8 + PED_ONE_SHOT_PUNCH_VOLUME,
                PED_ONE_SHOT_PUNCH_MAX_DIST,
                3,
            )
        }
        x if x == SOUND_SPLASH as i16 => next_ped_indexed_sample_with_frequency(
            audio,
            state,
            SFX_SPLASH_1 as u32,
            displaced_frequency(audio, 20000, 1400),
            random_mod(audio, 2, 30) as u8 + PED_ONE_SHOT_SPLASH_PED_VOLUME,
            PED_ONE_SHOT_SPLASH_MAX_DIST,
            1,
        ),
        _ => return PedOneShotMatch::PedComment,
    };
    if state.sound_index > 60 {
        state.sound_index = 21;
    }
    mapped.counter %= 128;
    PedOneShotMatch::Sample(mapped)
}

fn ped_footstep_sample(audio: &cAudioManager, surface: u8) -> u32 {
    match surface {
        x if x == eSurfaceType::SURFACE_GRASS as u8 => {
            SFX_FOOTSTEP_GRASS_1 as u32 + random_mod(audio, 1, 5)
        }
        x if x == eSurfaceType::SURFACE_GRAVEL as u8
            || x == eSurfaceType::SURFACE_MUD_DRY as u8 =>
        {
            SFX_FOOTSTEP_GRAVEL_1 as u32 + random_mod(audio, 4, 5)
        }
        x if x == eSurfaceType::SURFACE_CAR as u8
            || x == eSurfaceType::SURFACE_GARAGE_DOOR as u8
            || x == eSurfaceType::SURFACE_CAR_PANEL as u8
            || x == eSurfaceType::SURFACE_THICK_METAL_PLATE as u8
            || x == eSurfaceType::SURFACE_SCAFFOLD_POLE as u8
            || x == eSurfaceType::SURFACE_LAMP_POST as u8
            || x == eSurfaceType::SURFACE_FIRE_HYDRANT as u8
            || x == eSurfaceType::SURFACE_GIRDER as u8
            || x == eSurfaceType::SURFACE_METAL_CHAIN_FENCE as u8
            || x == eSurfaceType::SURFACE_CONTAINER as u8
            || x == eSurfaceType::SURFACE_NEWS_VENDOR as u8 =>
        {
            SFX_FOOTSTEP_METAL_1 as u32 + random_mod(audio, 0, 5)
        }
        x if x == eSurfaceType::SURFACE_SAND as u8 => {
            SFX_FOOTSTEP_SAND_1 as u32 + (audio.m_anRandomTable[4].unsigned_abs() & 3)
        }
        x if x == eSurfaceType::SURFACE_WATER as u8 => {
            SFX_FOOTSTEP_WATER_1 as u32 + (audio.m_anRandomTable[3].unsigned_abs() & 3)
        }
        x if x == eSurfaceType::SURFACE_WOOD_CRATES as u8
            || x == eSurfaceType::SURFACE_WOOD_BENCH as u8
            || x == eSurfaceType::SURFACE_WOOD_SOLID as u8 =>
        {
            SFX_FOOTSTEP_WOOD_1 as u32 + random_mod(audio, 2, 5)
        }
        x if x == eSurfaceType::SURFACE_HEDGE as u8 => {
            SFX_COL_VEG_1 as u32 + random_mod(audio, 2, 5)
        }
        _ => SFX_FOOTSTEP_CONCRETE_1 as u32 + random_mod(audio, 2, 5),
    }
}

fn next_ped_counter(state: &mut PedOneShotAudioState) -> u32 {
    let counter = state.sound_index as u32;
    state.sound_index = state.sound_index.wrapping_add(1);
    if state.sound_index > 60 {
        state.sound_index = 21;
    }
    counter
}

#[allow(clippy::too_many_arguments)]
fn next_ped_indexed_sample(
    audio: &cAudioManager,
    state: &mut PedOneShotAudioState,
    sample: u32,
    emitting_volume: u8,
    max_distance: f32,
    priority: u32,
    speed_multiplier: f32,
    loop_count: u32,
    static_sample: bool,
    frames_to_play: f32,
    frequency_displacement_seed: u32,
) -> PedOneShotSample {
    let base = audio.sample_base_frequency(sample);
    PedOneShotSample {
        sample,
        counter: next_ped_counter(state),
        frequency: displaced_frequency(audio, base as i32, frequency_displacement_seed),
        priority,
        emitting_volume,
        max_distance,
        speed_multiplier,
        loop_count,
        static_sample,
        frames_to_play,
        stereo: false,
        flamethrower_weapon_shot: false,
    }
}

fn next_ped_indexed_sample_with_frequency(
    _audio: &cAudioManager,
    state: &mut PedOneShotAudioState,
    sample: u32,
    frequency: u32,
    emitting_volume: u8,
    max_distance: f32,
    priority: u32,
) -> PedOneShotSample {
    PedOneShotSample {
        sample,
        counter: next_ped_counter(state),
        frequency,
        priority,
        emitting_volume,
        max_distance,
        speed_multiplier: 0.0,
        loop_count: 1,
        static_sample: true,
        frames_to_play: 0.0,
        stereo: false,
        flamethrower_weapon_shot: false,
    }
}

fn match_weapon_shot(
    audio: &cAudioManager,
    snapshot: &PedAudioSnapshot,
    state: &mut PedOneShotAudioState,
) -> Option<PedOneShotSample> {
    let (sample, volume, max_distance, priority, speed, loop_count, static_sample, frames, seed) =
        match snapshot.weapon_type {
            eWeaponType::WEAPONTYPE_COLT45 => (
                SFX_COLT45_LEFT as u32,
                random_mod(audio, 1, 10) as u8 + PED_ONE_SHOT_WEAPON_COLT45_VOLUME,
                PED_ONE_SHOT_WEAPON_COLT45_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_COLT45_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => (
                SFX_ROCKET_LEFT as u32,
                random_mod(audio, 0, 20) as u8 + PED_ONE_SHOT_WEAPON_ROCKETLAUNCHER_VOLUME,
                PED_ONE_SHOT_WEAPON_ROCKETLAUNCHER_MAX_DIST,
                1,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_ROCKET_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_FLAMETHROWER => (
                SFX_FLAMETHROWER_LEFT as u32,
                PED_ONE_SHOT_WEAPON_FLAMETHROWER_VOLUME,
                PED_ONE_SHOT_WEAPON_FLAMETHROWER_MAX_DIST,
                3,
                4.0,
                0,
                false,
                6.0,
                0,
            ),
            eWeaponType::WEAPONTYPE_AK47 => (
                SFX_AK47_LEFT as u32,
                random_mod(audio, 1, 15) as u8 + PED_ONE_SHOT_WEAPON_AK47_VOLUME,
                PED_ONE_SHOT_WEAPON_AK47_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_AK47_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_UZI => (
                SFX_UZI_LEFT as u32,
                random_mod(audio, 3, 15) as u8 + PED_ONE_SHOT_WEAPON_UZI_VOLUME,
                PED_ONE_SHOT_WEAPON_UZI_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_UZI_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_SNIPERRIFLE => (
                SFX_SNIPER_LEFT as u32,
                random_mod(audio, 4, 10) as u8 + PED_ONE_SHOT_WEAPON_SNIPERRIFLE_VOLUME,
                PED_ONE_SHOT_WEAPON_SNIPERRIFLE_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_SNIPER_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_SHOTGUN => (
                SFX_SHOTGUN_LEFT as u32,
                random_mod(audio, 2, 10) as u8 + PED_ONE_SHOT_WEAPON_SHOTGUN_VOLUME,
                PED_ONE_SHOT_WEAPON_SHOTGUN_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_SHOTGUN_LEFT as u32) >> 5,
            ),
            eWeaponType::WEAPONTYPE_M16 => (
                SFX_M16_LEFT as u32,
                random_mod(audio, 4, 15) as u8 + PED_ONE_SHOT_WEAPON_M16_VOLUME,
                PED_ONE_SHOT_WEAPON_M16_MAX_DIST,
                3,
                0.0,
                1,
                true,
                0.0,
                audio.sample_base_frequency(SFX_M16_LEFT as u32) >> 5,
            ),
            _ => return None,
        };
    let frequency = if snapshot.weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER {
        (10 * snapshot.audio_entity.max(0) as u32 % 2048) + audio.sample_base_frequency(sample)
    } else {
        displaced_frequency(audio, audio.sample_base_frequency(sample) as i32, seed)
    };
    Some(PedOneShotSample {
        sample,
        counter: if snapshot.weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER {
            9
        } else {
            next_ped_counter(state)
        },
        frequency,
        priority,
        emitting_volume: volume,
        max_distance,
        speed_multiplier: speed,
        loop_count,
        static_sample,
        frames_to_play: frames,
        stereo: matches!(
            snapshot.weapon_type,
            eWeaponType::WEAPONTYPE_COLT45
                | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
                | eWeaponType::WEAPONTYPE_FLAMETHROWER
                | eWeaponType::WEAPONTYPE_SNIPERRIFLE
                | eWeaponType::WEAPONTYPE_SHOTGUN
        ),
        flamethrower_weapon_shot: snapshot.weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER,
    })
}

fn match_weapon_reload(
    audio: &cAudioManager,
    snapshot: &PedAudioSnapshot,
    state: &mut PedOneShotAudioState,
) -> Option<PedOneShotSample> {
    let (sample, base_frequency) = match snapshot.weapon_type {
        eWeaponType::WEAPONTYPE_COLT45 => (
            SFX_PISTOL_RELOAD as u32,
            audio.sample_base_frequency(SFX_PISTOL_RELOAD as u32),
        ),
        eWeaponType::WEAPONTYPE_UZI => (SFX_M16_RELOAD as u32, 39243),
        eWeaponType::WEAPONTYPE_AK47 => (
            SFX_AK47_RELOAD as u32,
            audio.sample_base_frequency(SFX_AK47_RELOAD as u32),
        ),
        eWeaponType::WEAPONTYPE_M16 => (
            SFX_M16_RELOAD as u32,
            audio.sample_base_frequency(SFX_M16_RELOAD as u32),
        ),
        eWeaponType::WEAPONTYPE_SHOTGUN => (SFX_AK47_RELOAD as u32, 30290),
        eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => (
            SFX_ROCKET_RELOAD as u32,
            audio.sample_base_frequency(SFX_ROCKET_RELOAD as u32),
        ),
        eWeaponType::WEAPONTYPE_SNIPERRIFLE => (
            SFX_RIFLE_RELOAD as u32,
            audio.sample_base_frequency(SFX_RIFLE_RELOAD as u32),
        ),
        _ => return None,
    };
    Some(PedOneShotSample {
        sample,
        counter: next_ped_counter(state),
        frequency: displaced_frequency(audio, base_frequency as i32, 300),
        priority: 5,
        emitting_volume: PED_ONE_SHOT_WEAPON_RELOAD_VOLUME,
        max_distance: PED_ONE_SHOT_WEAPON_RELOAD_MAX_DIST,
        speed_multiplier: 0.0,
        loop_count: 1,
        static_sample: true,
        frames_to_play: 0.0,
        stereo: false,
        flamethrower_weapon_shot: false,
    })
}

fn fight_sample_and_frequency(sound: i16) -> (u32, u32) {
    match sound {
        x if x == SOUND_FIGHT_PUNCH_33 as i16 => (SFX_FIGHT_1 as u32, 18000),
        x if x == SOUND_FIGHT_KICK_34 as i16 => (SFX_FIGHT_1 as u32, 16500),
        x if x == SOUND_FIGHT_HEADBUTT_35 as i16 => (SFX_FIGHT_1 as u32, 20000),
        x if x == SOUND_FIGHT_PUNCH_36 as i16 => (SFX_FIGHT_2 as u32, 18000),
        x if x == SOUND_FIGHT_PUNCH_37 as i16 => (SFX_FIGHT_2 as u32, 16500),
        x if x == SOUND_FIGHT_CLOSE_PUNCH_38 as i16 => (SFX_FIGHT_2 as u32, 20000),
        x if x == SOUND_FIGHT_PUNCH_39 as i16 => (SFX_FIGHT_4 as u32, 18000),
        x if x == SOUND_FIGHT_PUNCH_OR_KICK_BELOW_40 as i16 => (SFX_FIGHT_4 as u32, 16500),
        x if x == SOUND_FIGHT_PUNCH_41 as i16 => (SFX_FIGHT_4 as u32, 20000),
        x if x == SOUND_FIGHT_PUNCH_FROM_BEHIND_42 as i16 => (SFX_FIGHT_5 as u32, 18000),
        x if x == SOUND_FIGHT_KNEE_OR_KICK_43 as i16 => (SFX_FIGHT_5 as u32, 16500),
        _ => (SFX_FIGHT_5 as u32, 20000),
    }
}

fn queue_ped_sample(
    audio: &mut cAudioManager,
    snapshot: &PedAudioSnapshot,
    mapped: PedOneShotSample,
    volume: u8,
) {
    audio.m_sQueueSample.m_nEntityIndex = snapshot.audio_entity;
    audio.m_sQueueSample.m_nCounter = mapped.counter;
    audio.m_sQueueSample.m_nSampleIndex = mapped.sample;
    audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
    audio.m_sQueueSample.m_bIs2D = false;
    if mapped.stereo {
        audio.m_sQueueSample.m_bIs2D = true;
        audio.m_sQueueSample.m_nPan = 0;
    }
    audio.m_sQueueSample.m_nPriority = mapped.priority;
    audio.m_sQueueSample.m_nFrequency = mapped.frequency;
    audio.m_sQueueSample.m_nVolume = volume;
    audio.m_sQueueSample.m_nEmittingVolume = mapped.emitting_volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nLoopCount = mapped.loop_count;
    audio.m_sQueueSample.m_nLoopStart = 0;
    audio.m_sQueueSample.m_nLoopEnd = -1;
    audio.m_sQueueSample.m_fSpeedMultiplier = mapped.speed_multiplier;
    audio.m_sQueueSample.m_MaxDistance = mapped.max_distance;
    audio.m_sQueueSample.m_bStatic = mapped.static_sample;
    audio.m_sQueueSample.m_vecPos = snapshot.position;
    audio.m_sQueueSample.m_bReverb = true;
    audio.m_sQueueSample.m_nFramesToPlay = mapped.frames_to_play;
    audio.AddSampleToRequestedQueue();
}

fn queue_ped_stereo_pair(
    audio: &mut cAudioManager,
    mapped: PedOneShotSample,
    state: &mut PedOneShotAudioState,
    sound: i16,
    weapon_type: eWeaponType,
) {
    audio.m_sQueueSample.m_nPan = 127;
    audio.m_sQueueSample.m_nSampleIndex = mapped.sample + 1;
    if sound == SOUND_WEAPON_SHOT_FIRED as i16
        && weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER
    {
        audio.m_sQueueSample.m_nCounter = mapped.counter + 1;
    } else {
        audio.m_sQueueSample.m_nCounter = next_ped_counter(state);
    }
    audio.AddSampleToRequestedQueue();
}

fn calculate_ped_distance(audio: &mut cAudioManager, snapshot: &PedAudioSnapshot) -> f32 {
    let distance = if snapshot.distance_calculated {
        snapshot.distance_squared
    } else {
        snapshot.distance_squared.sqrt()
    };
    audio.m_sQueueSample.m_fDistance = distance;
    distance
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrontendSample {
    sample: u32,
    stereo: bool,
    processed_pickup: bool,
    processed_mission: bool,
    bank: u8,
}

fn frontend_sample_for_sound(audio: &cAudioManager, sound: i16) -> Option<FrontendSample> {
    let mut stereo = false;
    let mut processed_pickup = false;
    let mut processed_mission = false;
    let mut frontend_bank = false;
    let sample = match sound {
        x if x == SOUND_FRONTEND_RADIO_TURN_OFF as i16
            || x == SOUND_FRONTEND_RADIO_CHANGE as i16 =>
        {
            SFX_RADIO_CLICK as u32
        }
        x if x == SOUND_HUD as i16 => SFX_INFO as u32,
        x if x == SOUND_FRONTEND_MENU_STARTING as i16 => {
            stereo = true;
            SFX_START_BUTTON_LEFT as u32
        }
        x if x == SOUND_FRONTEND_MENU_NEW_PAGE as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_PAGE_CHANGE_AND_BACK_LEFT as u32
        }
        x if x == SOUND_FRONTEND_MENU_NAVIGATION as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_HIGHLIGHT_LEFT as u32
        }
        x if x == SOUND_FRONTEND_MENU_SETTING_CHANGE as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_SELECT_LEFT as u32
        }
        x if x == SOUND_FRONTEND_MENU_BACK as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_SUB_MENU_BACK_LEFT as u32
        }
        x if x == SOUND_FRONTEND_STEREO as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_STEREO_LEFT as u32
        }
        x if x == SOUND_FRONTEND_MONO as i16 => {
            frontend_bank = true;
            SFX_MONO as u32
        }
        x if x == SOUND_FRONTEND_AUDIO_TEST as i16 => {
            frontend_bank = true;
            SFX_NOISE_BURST_1 as u32 + random_mod(audio, 0, 3)
        }
        x if x == SOUND_FRONTEND_FAIL as i16 => {
            stereo = true;
            frontend_bank = true;
            SFX_ERROR_LEFT as u32
        }
        x if x == SOUND_RACE_START_GO as i16 => SFX_PART_MISSION_COMPLETE as u32,
        x if x == SOUND_PART_MISSION_COMPLETE as i16 => {
            processed_mission = true;
            SFX_PART_MISSION_COMPLETE as u32
        }
        x if x == SOUND_RACE_START_3 as i16
            || x == SOUND_RACE_START_2 as i16
            || x == SOUND_RACE_START_1 as i16
            || x == SOUND_CLOCK_TICK as i16 =>
        {
            SFX_TIMER_BEEP as u32
        }
        x if x == SOUND_PAGER as i16 => SFX_PAGER as u32,
        x if x == SOUND_WEAPON_SNIPER_SHOT_NO_ZOOM as i16 => SFX_ERROR_FIRE_RIFLE as u32,
        x if x == SOUND_WEAPON_ROCKET_SHOT_NO_ZOOM as i16 => SFX_ERROR_FIRE_ROCKET_LAUNCHER as u32,
        x if x == SOUND_PICKUP_WEAPON_BOUGHT as i16 || x == SOUND_PICKUP_WEAPON as i16 => {
            stereo = true;
            processed_pickup = true;
            SFX_PICKUP_1_LEFT as u32
        }
        x if x == SOUND_PICKUP_BONUS as i16
            || x == SOUND_PICKUP_MONEY as i16
            || x == SOUND_PICKUP_HIDDEN_PACKAGE as i16
            || x == SOUND_PICKUP_PACMAN_PILL as i16
            || x == SOUND_PICKUP_PACMAN_PACKAGE as i16
            || x == SOUND_PICKUP_FLOAT_PACKAGE as i16 =>
        {
            stereo = true;
            processed_pickup = true;
            SFX_PICKUP_3_LEFT as u32
        }
        x if x == SOUND_PICKUP_ERROR as i16 => {
            stereo = true;
            processed_pickup = true;
            SFX_PICKUP_ERROR_LEFT as u32
        }
        x if x == SOUND_GARAGE_NO_MONEY as i16
            || x == SOUND_GARAGE_BAD_VEHICLE as i16
            || x == SOUND_GARAGE_BOMB_ALREADY_SET as i16 =>
        {
            stereo = true;
            SFX_PICKUP_ERROR_LEFT as u32
        }
        x if x == SOUND_GARAGE_OPENING as i16
            || x == SOUND_GARAGE_BOMB1_SET as i16
            || x == SOUND_GARAGE_BOMB2_SET as i16
            || x == SOUND_GARAGE_BOMB3_SET as i16
            || x == SOUND_41 as i16
            || x == SOUND_GARAGE_VEHICLE_DECLINED as i16
            || x == SOUND_GARAGE_VEHICLE_ACCEPTED as i16
            || x == SOUND_PICKUP_HEALTH as i16
            || x == SOUND_4B as i16
            || x == SOUND_PICKUP_ADRENALINE as i16
            || x == SOUND_PICKUP_ARMOUR as i16
            || x == SOUND_EVIDENCE_PICKUP as i16
            || x == SOUND_UNLOAD_GOLD as i16 =>
        {
            stereo = true;
            processed_pickup = true;
            SFX_PICKUP_2_LEFT as u32
        }
        _ => return None,
    };
    Some(FrontendSample {
        sample,
        stereo,
        processed_pickup,
        processed_mission,
        bank: if frontend_bank {
            SFX_BANK_FRONT_END_MENU
        } else {
            SFX_BANK_0
        },
    })
}

fn queue_frontend_sample(
    audio: &mut cAudioManager,
    audio_entity: i32,
    counter: u8,
    sample: u32,
    bank: u8,
    frequency: u32,
    stereo: bool,
    right_channel: bool,
) {
    audio.m_sQueueSample.m_nEntityIndex = audio_entity;
    audio.m_sQueueSample.m_nCounter = counter as u32;
    audio.m_sQueueSample.m_nSampleIndex = sample;
    audio.m_sQueueSample.m_nFrequency = frequency;
    audio.m_sQueueSample.m_nVolume = FRONTEND_VOLUME;
    audio.m_sQueueSample.m_nLoopCount = 1;
    audio.m_sQueueSample.m_nLoopStart = 0;
    audio.m_sQueueSample.m_nLoopEnd = -1;
    audio.m_sQueueSample.m_bStatic = true;
    audio.m_sQueueSample.m_nBankIndex = bank;
    audio.m_sQueueSample.m_nPriority = 0;
    audio.m_sQueueSample.m_bIs2D = true;
    audio.m_sQueueSample.m_nPan = if stereo {
        if right_channel {
            127 - (audio.m_anRandomTable[0].unsigned_abs() as u8 & 31)
        } else {
            audio.m_anRandomTable[0].unsigned_abs() as u8 & 31
        }
    } else {
        63
    };
    audio.m_sQueueSample.m_bReverb = false;
    audio.AddSampleToRequestedQueue();
}

pub fn process_crane(audio: &mut cAudioManager, snapshot: &CraneAudioSnapshot) -> bool {
    if !snapshot.activated || !snapshot.moving {
        return false;
    }
    let mut queued = queue_positional_sample(
        audio,
        &snapshot.source,
        SFX_CRANE_MAGNET as u32,
        SFX_BANK_0,
        2,
        6000,
        CRANE_VOLUME,
        CRANE_MAX_DIST,
        0,
        4.0,
        false,
        true,
        3.0,
    );
    if queued && snapshot.collision_event {
        let volume = audio.m_sQueueSample.m_nVolume;
        queued |= queue_positional_sample_with_volume(
            audio,
            &snapshot.source,
            1,
            SFX_COL_CAR_2 as u32,
            SFX_BANK_0,
            2,
            audio.sample_base_frequency(SFX_COL_CAR_2 as u32),
            CRANE_VOLUME,
            volume,
            CRANE_MAX_DIST,
            1,
            4.0,
            true,
            true,
            3.0,
        );
    }
    queued
}

pub fn process_projectile(audio: &mut cAudioManager, snapshot: &ProjectileAudioSnapshot) -> bool {
    let (sample, frequency, priority, emitting_volume, max_distance) = match snapshot.kind {
        ProjectileAudioKind::Rocket => (
            SFX_ROCKET_FLY as u32,
            audio.sample_base_frequency(SFX_ROCKET_FLY as u32),
            3,
            PROJECTILE_ROCKET_VOLUME,
            PROJECTILE_ROCKET_MAX_DIST,
        ),
        ProjectileAudioKind::Molotov => (
            SFX_PED_ON_FIRE as u32,
            32 * audio.sample_base_frequency(SFX_PED_ON_FIRE as u32) / 25,
            7,
            PROJECTILE_MOLOTOV_VOLUME,
            PROJECTILE_MOLOTOV_MAX_DIST,
        ),
    };
    queue_positional_sample(
        audio,
        &snapshot.source,
        sample,
        SFX_BANK_0,
        priority,
        frequency,
        emitting_volume,
        max_distance,
        0,
        4.0,
        false,
        true,
        3.0,
    )
}

pub fn process_garage(audio: &mut cAudioManager, snapshot: &GarageAudioSnapshot) -> bool {
    let mut queued = false;
    if matches!(
        snapshot.state,
        GarageAudioState::Opening | GarageAudioState::Closing | GarageAudioState::AfterDropoff
    ) {
        let (sample, frequency, counter, loop_count, static_sample, frames) =
            if snapshot.crusher && snapshot.state == GarageAudioState::AfterDropoff {
                if snapshot.frame_counter & 1 == 0 {
                    (NO_SAMPLE, 0, snapshot.source.index, 1, true, 0.0)
                } else {
                    let use_panel = audio.m_anRandomTable[1] & 1 == 0;
                    let sample = if use_panel {
                        SFX_COL_CAR_PANEL_1 as u32 + random_mod(audio, 2, 6)
                    } else {
                        SFX_COL_CAR_1 as u32 + random_mod(audio, 2, 5)
                    };
                    (
                        sample,
                        displaced_frequency(
                            audio,
                            (audio.sample_base_frequency(sample) >> 1) as i32,
                            (audio.sample_base_frequency(sample) >> 5).max(1),
                        ),
                        32 + snapshot.source.index,
                        1,
                        true,
                        0.0,
                    )
                }
            } else if snapshot.crusher {
                (
                    SFX_FISHING_BOAT_IDLE as u32,
                    6543,
                    snapshot.source.index,
                    0,
                    false,
                    3.0,
                )
            } else {
                (
                    SFX_GARAGE_DOOR_LOOP as u32,
                    13961,
                    snapshot.source.index,
                    0,
                    false,
                    3.0,
                )
            };
        if sample != NO_SAMPLE {
            if snapshot.source.distance_squared < GARAGES_MAX_DIST * GARAGES_MAX_DIST {
                let distance = calculate_positional_distance(audio, &snapshot.source);
                queued |= queue_positional_sample_with_distance(
                    audio,
                    &snapshot.source,
                    distance,
                    counter,
                    sample,
                    SFX_BANK_0,
                    3,
                    frequency,
                    GARAGES_VOLUME,
                    GARAGES_MAX_DIST,
                    loop_count,
                    2.0,
                    static_sample,
                    true,
                    frames,
                );
            }
        }
    }

    if snapshot.door_opened_event || snapshot.door_closed_event {
        let (sample, frequency) = if snapshot.crusher || snapshot.door_opened_event {
            (
                SFX_COL_CAR_PANEL_2 as u32,
                if snapshot.crusher { 6735 } else { 22000 },
            )
        } else {
            (SFX_COL_GARAGE_DOOR_1 as u32, 18000)
        };
        queued |= queue_positional_sample(
            audio,
            &snapshot.source,
            sample,
            SFX_BANK_0,
            4,
            frequency,
            GARAGES_DOOR_VOLUME,
            GARAGES_MAX_DIST,
            1,
            0.0,
            true,
            true,
            0.0,
        );
    }
    queued
}

pub fn process_fire_hydrant(audio: &mut cAudioManager, snapshot: &PositionalAudioSnapshot) -> bool {
    queue_positional_sample(
        audio,
        snapshot,
        SFX_JUMBO_TAXI as u32,
        SFX_BANK_0,
        4,
        15591,
        FIRE_HYDRANT_VOLUME,
        FIRE_HYDRANT_MAX_DIST,
        0,
        2.0,
        false,
        true,
        3.0,
    )
}

pub fn process_bridge(audio: &mut cAudioManager, snapshot: &BridgeAudioSnapshot) -> bool {
    if snapshot.source.distance_squared >= BRIDGE_MAX_DIST * BRIDGE_MAX_DIST {
        return false;
    }
    let distance = calculate_positional_distance(audio, &snapshot.source);
    let mut queued = false;
    if matches!(
        snapshot.state,
        BridgeAudioState::Locked
            | BridgeAudioState::LiftPartIsUp
            | BridgeAudioState::LiftPartAboutToMoveUp
            | BridgeAudioState::LiftPartMovingUp
            | BridgeAudioState::LiftPartMovingDown
    ) {
        queued |= process_bridge_warning(audio, snapshot, distance);
    }
    if matches!(
        snapshot.state,
        BridgeAudioState::LiftPartMovingUp | BridgeAudioState::LiftPartMovingDown
    ) {
        queued |= process_bridge_motor(audio, snapshot, distance);
    }
    queued | process_bridge_one_shot(audio, snapshot, distance)
}

fn process_bridge_warning(
    audio: &mut cAudioManager,
    snapshot: &BridgeAudioSnapshot,
    distance: f32,
) -> bool {
    if !snapshot.commercial_passed || distance >= BRIDGE_MAX_DIST {
        return false;
    }
    queue_positional_sample_with_distance(
        audio,
        &snapshot.source,
        distance,
        0,
        SFX_BRIDGE_OPEN_WARNING as u32,
        SFX_BANK_GENERIC_EXTRA,
        1,
        audio.sample_base_frequency(SFX_BRIDGE_OPEN_WARNING as u32),
        BRIDGE_WARNING_VOLUME,
        BRIDGE_MAX_DIST,
        0,
        2.0,
        false,
        false,
        8.0,
    )
}

fn process_bridge_motor(
    audio: &mut cAudioManager,
    snapshot: &BridgeAudioSnapshot,
    distance: f32,
) -> bool {
    if distance >= BRIDGE_MOTOR_MAX_DIST {
        return false;
    }
    queue_positional_sample_with_distance(
        audio,
        &snapshot.source,
        distance,
        1,
        SFX_FISHING_BOAT_IDLE as u32,
        SFX_BANK_0,
        1,
        5500,
        BRIDGE_MOTOR_VOLUME,
        BRIDGE_MOTOR_MAX_DIST,
        0,
        2.0,
        false,
        false,
        3.0,
    )
}

fn process_bridge_one_shot(
    audio: &mut cAudioManager,
    snapshot: &BridgeAudioSnapshot,
    distance: f32,
) -> bool {
    let moving_edge = matches!(
        (snapshot.state, snapshot.old_state),
        (
            BridgeAudioState::LiftPartIsUp,
            BridgeAudioState::LiftPartMovingUp
        ) | (
            BridgeAudioState::LiftPartIsDown,
            BridgeAudioState::LiftPartMovingDown
        ) | (
            BridgeAudioState::LiftPartMovingUp,
            BridgeAudioState::LiftPartAboutToMoveUp
        ) | (
            BridgeAudioState::LiftPartMovingDown,
            BridgeAudioState::LiftPartIsUp
        )
    );
    if !moving_edge || distance >= BRIDGE_MOTOR_MAX_DIST {
        return false;
    }
    queue_positional_sample_with_distance(
        audio,
        &snapshot.source,
        distance,
        2,
        SFX_COL_CONTAINER_1 as u32,
        SFX_BANK_0,
        1,
        audio.sample_base_frequency(SFX_COL_CONTAINER_1 as u32),
        BRIDGE_MOTOR_VOLUME,
        BRIDGE_MOTOR_MAX_DIST,
        1,
        2.0,
        true,
        false,
        0.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn queue_positional_sample(
    audio: &mut cAudioManager,
    source: &PositionalAudioSnapshot,
    sample: u32,
    bank: u8,
    priority: u32,
    frequency: u32,
    emitting_volume: u8,
    max_distance: f32,
    loop_count: u32,
    speed_multiplier: f32,
    static_sample: bool,
    reverb: bool,
    frames_to_play: f32,
) -> bool {
    if source.distance_squared >= max_distance * max_distance {
        return false;
    }
    let distance = calculate_positional_distance(audio, source);
    queue_positional_sample_with_distance(
        audio,
        source,
        distance,
        source.index,
        sample,
        bank,
        priority,
        frequency,
        emitting_volume,
        max_distance,
        loop_count,
        speed_multiplier,
        static_sample,
        reverb,
        frames_to_play,
    )
}

#[allow(clippy::too_many_arguments)]
fn queue_positional_sample_with_distance(
    audio: &mut cAudioManager,
    source: &PositionalAudioSnapshot,
    distance: f32,
    counter: u32,
    sample: u32,
    bank: u8,
    priority: u32,
    frequency: u32,
    emitting_volume: u8,
    max_distance: f32,
    loop_count: u32,
    speed_multiplier: f32,
    static_sample: bool,
    reverb: bool,
    frames_to_play: f32,
) -> bool {
    let volume = audio.ComputeVolume(emitting_volume, max_distance, distance);
    if volume == 0 {
        return false;
    }
    queue_positional_sample_with_volume(
        audio,
        source,
        counter,
        sample,
        bank,
        priority,
        frequency,
        emitting_volume,
        volume,
        max_distance,
        loop_count,
        speed_multiplier,
        static_sample,
        reverb,
        frames_to_play,
    )
}

#[allow(clippy::too_many_arguments)]
fn queue_positional_sample_with_volume(
    audio: &mut cAudioManager,
    source: &PositionalAudioSnapshot,
    counter: u32,
    sample: u32,
    bank: u8,
    priority: u32,
    frequency: u32,
    emitting_volume: u8,
    volume: u8,
    max_distance: f32,
    loop_count: u32,
    speed_multiplier: f32,
    static_sample: bool,
    reverb: bool,
    frames_to_play: f32,
) -> bool {
    audio.m_sQueueSample.m_nEntityIndex = source.audio_entity;
    audio.m_sQueueSample.m_nCounter = counter;
    audio.m_sQueueSample.m_nSampleIndex = sample;
    audio.m_sQueueSample.m_nBankIndex = bank;
    audio.m_sQueueSample.m_bIs2D = false;
    audio.m_sQueueSample.m_nPriority = priority;
    audio.m_sQueueSample.m_nFrequency = frequency;
    audio.m_sQueueSample.m_nVolume = volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nEmittingVolume = emitting_volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nLoopCount = loop_count;
    audio.m_sQueueSample.m_nLoopStart = 0;
    audio.m_sQueueSample.m_nLoopEnd = -1;
    audio.m_sQueueSample.m_fSpeedMultiplier = speed_multiplier;
    audio.m_sQueueSample.m_MaxDistance = max_distance;
    audio.m_sQueueSample.m_bStatic = static_sample;
    audio.m_sQueueSample.m_vecPos = source.position;
    audio.m_sQueueSample.m_bReverb = reverb;
    audio.m_sQueueSample.m_nFramesToPlay = frames_to_play;
    audio.AddSampleToRequestedQueue();
    true
}

fn calculate_positional_distance(
    audio: &mut cAudioManager,
    source: &PositionalAudioSnapshot,
) -> f32 {
    let distance = if source.distance_calculated {
        source.distance_squared
    } else {
        source.distance_squared.sqrt()
    };
    audio.m_sQueueSample.m_fDistance = distance;
    distance
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptObjectAudioContext {
    pub player_surface: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScriptBuildingAudioState {
    pub porn_next_time: u32,
    pub sawmill_next_time: u32,
    pub shop_next_time: u32,
    pub airport_next_time: u32,
    pub cinema_next_time: u32,
    pub docks_next_time: u32,
    pub home_next_time: u32,
    pub cell_next_time: u32,
    pub airport_counter: u8,
    pub cinema_counter: u8,
    pub docks_counter: u8,
    pub home_counter: u8,
    pub cell_counter: u8,
}

impl ScriptBuildingAudioState {
    pub const fn new() -> Self {
        Self {
            porn_next_time: 0,
            sawmill_next_time: 0,
            shop_next_time: 0,
            airport_next_time: 0,
            cinema_next_time: 0,
            docks_next_time: 0,
            home_next_time: 0,
            cell_next_time: 0,
            airport_counter: 0,
            cinema_counter: 0,
            docks_counter: 0,
            home_counter: 0,
            cell_counter: 0,
        }
    }
}

pub fn process_one_shot_script_object(
    audio: &mut cAudioManager,
    sound: u8,
    position: CVector,
    camera_position: CVector,
    context: ScriptObjectAudioContext,
) -> bool {
    let volume: u8;
    audio.m_sQueueSample.m_vecPos = position;
    audio.m_sQueueSample.m_bIs2D = false;
    audio.m_sQueueSample.m_fSpeedMultiplier = 0.0;
    audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;

    match sound as i32 {
        SCRIPT_SOUND_BOX_DESTROYED_1 => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_BOX_DESTROYED_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_WOODEN_BOX_SMASH as u32;
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(audio, 18600, 1500);
            audio.m_sQueueSample.m_nPriority = 3;
            volume = random_mod(audio, 2, 20) as u8 + SCRIPT_OBJECT_BOX_DESTROYED_VOLUME;
        }
        SCRIPT_SOUND_BOX_DESTROYED_2 => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_BOX_DESTROYED_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_CARDBOARD_BOX_SMASH as u32;
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(audio, 18600, 1500);
            audio.m_sQueueSample.m_nPriority = 3;
            volume = random_mod(audio, 2, 20) as u8 + SCRIPT_OBJECT_BOX_DESTROYED_VOLUME;
        }
        SCRIPT_SOUND_METAL_COLLISION => {
            audio.m_sQueueSample.m_MaxDistance = 60.0;
            audio.m_sQueueSample.m_nSampleIndex = SFX_COL_CAR_1 as u32 + random_mod(audio, 3, 5);
            audio.m_sQueueSample.m_nFrequency =
                audio.sample_base_frequency(audio.m_sQueueSample.m_nSampleIndex);
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(
                audio,
                audio.m_sQueueSample.m_nFrequency as i32,
                (audio.m_sQueueSample.m_nFrequency >> 4).max(1),
            );
            audio.m_sQueueSample.m_nPriority = 3;
            volume = random_mod(audio, 2, 30) as u8 + SCRIPT_OBJECT_METAL_COLLISION_VOLUME;
        }
        SCRIPT_SOUND_TIRE_COLLISION => {
            audio.m_sQueueSample.m_MaxDistance = 60.0;
            audio.m_sQueueSample.m_nSampleIndex = SFX_TYRE_BUMP as u32;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_TYRE_BUMP as u32);
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(
                audio,
                audio.m_sQueueSample.m_nFrequency as i32,
                (audio.m_sQueueSample.m_nFrequency >> 4).max(1),
            );
            audio.m_sQueueSample.m_nPriority = 3;
            volume = random_mod(audio, 2, 30) as u8 + SCRIPT_OBJECT_TIRE_COLLISION_VOLUME;
        }
        SCRIPT_SOUND_GUNSHELL_DROP => {
            let soft_surface = context.player_surface.is_some_and(is_soft_shell_surface);
            if context.player_surface == Some(eSurfaceType::SURFACE_WATER as u8) {
                return false;
            }
            audio.m_sQueueSample.m_nSampleIndex = if soft_surface {
                SFX_BULLET_SHELL_HIT_GROUND_2 as u32
            } else {
                SFX_BULLET_SHELL_HIT_GROUND_1 as u32
            };
            audio.m_sQueueSample.m_nFrequency = if soft_surface {
                displaced_frequency(audio, 11000, 500)
            } else {
                displaced_frequency(audio, 18000, 750)
            };
            audio.m_sQueueSample.m_nPriority = if soft_surface { 18 } else { 15 };
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GUNSHELL_MAX_DIST;
            volume = random_mod(audio, 2, 20) as u8 + SCRIPT_OBJECT_GUNSHELL_VOLUME;
        }
        SCRIPT_SOUND_GUNSHELL_DROP_SOFT => {
            audio.m_sQueueSample.m_nSampleIndex = SFX_BULLET_SHELL_HIT_GROUND_2 as u32;
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(audio, 11000, 500);
            audio.m_sQueueSample.m_nPriority = 18;
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GUNSHELL_MAX_DIST;
            volume = random_mod(audio, 2, 20) as u8 + SCRIPT_OBJECT_GUNSHELL_VOLUME;
        }
        SCRIPT_SOUND_BULLET_HIT_GROUND_1
        | SCRIPT_SOUND_BULLET_HIT_GROUND_2
        | SCRIPT_SOUND_BULLET_HIT_GROUND_3 => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_BULLET_HIT_GROUND_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex =
                SFX_BULLET_WALL_1 as u32 + random_mod(audio, 0, 3);
            audio.m_sQueueSample.m_nFrequency =
                audio.sample_base_frequency(audio.m_sQueueSample.m_nSampleIndex);
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(
                audio,
                audio.m_sQueueSample.m_nFrequency as i32,
                (audio.m_sQueueSample.m_nFrequency >> 5).max(1),
            );
            audio.m_sQueueSample.m_nPriority = 9;
            volume = random_mod(audio, 2, 20) as u8 + SCRIPT_OBJECT_BULLET_HIT_GROUND_VOLUME;
        }
        SCRIPT_SOUND_PAYPHONE_RINGING => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_PAYPHONE_RINGING_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_PHONE_RING as u32;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_PHONE_RING as u32);
            audio.m_sQueueSample.m_nPriority = 1;
            audio.m_sQueueSample.m_fSpeedMultiplier = 2.0;
            volume = SCRIPT_OBJECT_PAYPHONE_RINGING_VOLUME;
        }
        SCRIPT_SOUND_GLASS_BREAK_L => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GLASS_BREAK_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_GLASS_SMASH as u32;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_GLASS_SMASH as u32);
            audio.m_sQueueSample.m_nPriority = 3;
            volume = SCRIPT_OBJECT_GLASS_BREAK_LONG_VOLUME;
        }
        SCRIPT_SOUND_GLASS_BREAK_S => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GLASS_BREAK_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_GLASS_SMASH as u32;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_GLASS_SMASH as u32);
            audio.m_sQueueSample.m_nPriority = 3;
            volume = SCRIPT_OBJECT_GLASS_BREAK_SHORT_VOLUME;
        }
        SCRIPT_SOUND_GLASS_CRACK => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GLASS_BREAK_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_GLASS_CRACK as u32;
            audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(SFX_GLASS_CRACK as u32);
            audio.m_sQueueSample.m_nPriority = 3;
            volume = SCRIPT_OBJECT_GLASS_BREAK_LONG_VOLUME;
        }
        SCRIPT_SOUND_GLASS_LIGHT_BREAK => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GLASS_LIGHT_BREAK_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex =
                SFX_GLASS_SHARD_1 as u32 + (audio.m_anRandomTable[4].unsigned_abs() & 3);
            audio.m_sQueueSample.m_nFrequency = displaced_frequency(audio, 19000, 2000);
            audio.m_sQueueSample.m_nPriority = 9;
            volume = (SCRIPT_OBJECT_GLASS_LIGHT_BREAK_VOLUME as i32
                + random_displacement(audio, 11))
            .clamp(0, MAX_VOLUME as i32) as u8;
        }
        SCRIPT_SOUND_GATE_START_CLUNK | SCRIPT_SOUND_GATE_STOP_CLUNK => {
            audio.m_sQueueSample.m_MaxDistance = SCRIPT_OBJECT_GATE_MAX_DIST;
            audio.m_sQueueSample.m_nSampleIndex = SFX_COL_GATE as u32;
            audio.m_sQueueSample.m_nFrequency = if sound as i32 == SCRIPT_SOUND_GATE_START_CLUNK {
                10600
            } else {
                9000
            };
            audio.m_sQueueSample.m_nPriority = 1;
            volume = (50 + random_displacement(audio, 10)).clamp(0, MAX_VOLUME as i32) as u8;
        }
        SCRIPT_SOUND_TRAIN_ANNOUNCEMENT_1 | SCRIPT_SOUND_TRAIN_ANNOUNCEMENT_2 => {
            audio.m_sQueueSample.m_MaxDistance = 40.0;
            audio.m_sQueueSample.m_nSampleIndex = SFX_TRAIN_STATION_ANNOUNCE as u32;
            audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
            audio.m_sQueueSample.m_nFrequency =
                audio.sample_base_frequency(SFX_TRAIN_STATION_ANNOUNCE as u32);
            audio.m_sQueueSample.m_nPriority = 0;
            audio.m_sQueueSample.m_fSpeedMultiplier = 2.0;
            volume = 110;
        }
        _ => return false,
    }

    let dist_squared = distance_squared(position, camera_position);
    if dist_squared < audio.m_sQueueSample.m_MaxDistance * audio.m_sQueueSample.m_MaxDistance {
        audio.m_sQueueSample.m_fDistance = dist_squared.sqrt();
        audio.m_sQueueSample.m_nVolume = audio.ComputeVolume(
            volume,
            audio.m_sQueueSample.m_MaxDistance,
            audio.m_sQueueSample.m_fDistance,
        );
        audio.m_sQueueSample.m_nEmittingVolume = volume.min(MAX_VOLUME);
        if audio.m_sQueueSample.m_nVolume > 0 {
            audio.m_sQueueSample.m_nCounter =
                audio.m_sQueueSample.m_nCounter.wrapping_add(1).max(1);
            audio.m_sQueueSample.m_nLoopCount = 1;
            audio.m_sQueueSample.m_bStatic = true;
            audio.m_sQueueSample.m_nLoopStart = 0;
            audio.m_sQueueSample.m_nLoopEnd = -1;
            audio.m_sQueueSample.m_bReverb = true;
            audio.AddSampleToRequestedQueue();
        }
    }
    true
}

pub fn process_looping_script_object(
    audio: &mut cAudioManager,
    sound: u8,
    position: CVector,
    camera_position: CVector,
) -> bool {
    let sound_i = sound as i32;
    let long_range = is_long_script_loop(sound_i);
    let max_distance = if long_range {
        SCRIPT_OBJECT_LONG_MAX_DIST
    } else {
        SCRIPT_OBJECT_SHORT_MAX_DIST
    };

    let Some((sample, volume, priority, frames_to_play, speed_multiplier)) =
        looping_script_sample(sound_i)
    else {
        return false;
    };

    let dist_squared = distance_squared(position, camera_position);
    if dist_squared >= max_distance * max_distance {
        return true;
    }

    let distance = dist_squared.sqrt();
    let computed_volume = audio.ComputeVolume(volume, max_distance, distance);
    if computed_volume == 0 {
        return true;
    }

    audio.m_sQueueSample.m_vecPos = position;
    audio.m_sQueueSample.m_fDistance = distance;
    audio.m_sQueueSample.m_nSampleIndex = sample;
    audio.m_sQueueSample.m_nBankIndex = SFX_BANK_0;
    audio.m_sQueueSample.m_bIs2D = false;
    audio.m_sQueueSample.m_nPriority = priority;
    audio.m_sQueueSample.m_nFrequency = audio.sample_base_frequency(sample);
    audio.m_sQueueSample.m_nVolume = computed_volume;
    audio.m_sQueueSample.m_nEmittingVolume = volume.min(MAX_VOLUME);
    audio.m_sQueueSample.m_nCounter = 0;
    audio.m_sQueueSample.m_nLoopCount = 0;
    audio.m_sQueueSample.m_nLoopStart = 0;
    audio.m_sQueueSample.m_nLoopEnd = -1;
    audio.m_sQueueSample.m_fSpeedMultiplier = speed_multiplier;
    audio.m_sQueueSample.m_MaxDistance = max_distance;
    audio.m_sQueueSample.m_bStatic = false;
    audio.m_sQueueSample.m_nFramesToPlay = frames_to_play;
    audio.m_sQueueSample.m_bReverb = true;
    audio.AddSampleToRequestedQueue();
    true
}

pub fn process_script_building_object(
    audio: &mut cAudioManager,
    sound: u8,
    position: CVector,
    camera_position: CVector,
    time_ms: u32,
    state: &mut ScriptBuildingAudioState,
) -> bool {
    let sound_i = sound as i32;
    let source = PositionalAudioSnapshot {
        audio_entity: audio.m_sQueueSample.m_nEntityIndex,
        index: 0,
        position,
        distance_squared: distance_squared(position, camera_position),
        distance_calculated: false,
    };
    match sound_i {
        SCRIPT_SOUND_PORN_CINEMA_1_S
        | SCRIPT_SOUND_PORN_CINEMA_1_L
        | SCRIPT_SOUND_PORN_CINEMA_2_S
        | SCRIPT_SOUND_PORN_CINEMA_2_L
        | SCRIPT_SOUND_PORN_CINEMA_3_S
        | SCRIPT_SOUND_PORN_CINEMA_3_L
        | SCRIPT_SOUND_MISTY_SEX_S
        | SCRIPT_SOUND_MISTY_SEX_L => {
            let Some((loop_sample, groan_base, max_distance)) = porn_samples(sound_i) else {
                return false;
            };
            let mut queued = false;
            let is_misty = matches!(sound_i, SCRIPT_SOUND_MISTY_SEX_S | SCRIPT_SOUND_MISTY_SEX_L);
            if !is_misty {
                queued |= queue_positional_sample(
                    audio,
                    &source,
                    loop_sample,
                    SFX_BANK_0,
                    3,
                    audio.sample_base_frequency(loop_sample),
                    PORN_CINEMA_VOLUME,
                    max_distance,
                    0,
                    2.0,
                    false,
                    true,
                    0.0,
                );
            }
            if time_ms > state.porn_next_time {
                let rand = random_mod(audio, 1, 2);
                queued |= queue_positional_sample(
                    audio,
                    &source,
                    groan_base + rand,
                    SFX_BANK_0,
                    6,
                    displaced_frequency(
                        audio,
                        audio.sample_base_frequency(groan_base + rand) as i32,
                        audio.sample_base_frequency(groan_base + rand) >> 4,
                    ),
                    PORN_CINEMA_MOAN_VOLUME,
                    max_distance,
                    1,
                    0.0,
                    true,
                    true,
                    0.0,
                );
                state.porn_next_time = time_ms + 2000 + random_mod(audio, 3, 6000);
            }
            queued
        }
        SCRIPT_SOUND_WORK_SHOP_LOOP_S | SCRIPT_SOUND_WORK_SHOP_LOOP_L => queue_positional_sample(
            audio,
            &source,
            SFX_WORKSHOP_1 as u32,
            SFX_BANK_0,
            5,
            audio.sample_base_frequency(SFX_WORKSHOP_1 as u32),
            WORK_SHOP_VOLUME,
            WORK_SHOP_MAX_DIST,
            0,
            2.0,
            false,
            true,
            0.0,
        ),
        SCRIPT_SOUND_SAWMILL_LOOP_S | SCRIPT_SOUND_SAWMILL_LOOP_L => {
            let mut queued = queue_positional_sample(
                audio,
                &source,
                SFX_SAWMILL_LOOP as u32,
                SFX_BANK_0,
                5,
                audio.sample_base_frequency(SFX_SAWMILL_LOOP as u32),
                SAWMILL_VOLUME,
                SCRIPT_OBJECT_SHORT_MAX_DIST,
                0,
                2.0,
                false,
                true,
                0.0,
            );
            if time_ms > state.sawmill_next_time {
                queued |= queue_positional_sample(
                    audio,
                    &source,
                    SFX_SAWMILL_CUT_WOOD as u32,
                    SFX_BANK_0,
                    3,
                    audio.sample_base_frequency(SFX_SAWMILL_CUT_WOOD as u32),
                    SAWMILL_CUT_WOOD_VOLUME,
                    SCRIPT_OBJECT_SHORT_MAX_DIST,
                    1,
                    2.0,
                    true,
                    true,
                    0.0,
                );
                state.sawmill_next_time = time_ms + 2000 + random_mod(audio, 3, 4000);
            }
            queued
        }
        SCRIPT_SOUND_LAUNDERETTE_LOOP_S | SCRIPT_SOUND_LAUNDERETTE_LOOP_L => {
            let mut queued = queue_positional_sample(
                audio,
                &source,
                SFX_LAUNDERETTE_LOOP as u32,
                SFX_BANK_0,
                5,
                audio.sample_base_frequency(SFX_LAUNDERETTE_LOOP as u32),
                LAUNDERETTE_VOLUME,
                SCRIPT_OBJECT_SHORT_MAX_DIST,
                0,
                2.0,
                false,
                true,
                0.0,
            );
            queued |= queue_positional_sample(
                audio,
                &source,
                SFX_LAUNDERETTE_SONG_LOOP as u32,
                SFX_BANK_0,
                3,
                audio.sample_base_frequency(SFX_LAUNDERETTE_SONG_LOOP as u32),
                LAUNDERETTE_SONG_VOLUME,
                SCRIPT_OBJECT_SHORT_MAX_DIST,
                0,
                2.0,
                false,
                true,
                0.0,
            );
            queued
        }
        SCRIPT_SOUND_SHOP_LOOP_S | SCRIPT_SOUND_SHOP_LOOP_L => {
            let mut queued = queue_positional_sample(
                audio,
                &source,
                SFX_SHOP_LOOP as u32,
                SFX_BANK_0,
                5,
                audio.sample_base_frequency(SFX_SHOP_LOOP as u32),
                SHOP_VOLUME,
                SCRIPT_OBJECT_SHORT_MAX_DIST,
                0,
                2.0,
                false,
                true,
                0.0,
            );
            if time_ms > state.shop_next_time {
                let rand = random_mod(audio, 1, 2);
                queued |= queue_positional_sample(
                    audio,
                    &source,
                    SFX_SHOP_TILL_1 as u32 + rand,
                    SFX_BANK_0,
                    3,
                    audio.sample_base_frequency(SFX_SHOP_TILL_1 as u32 + rand),
                    SHOP_TILL_VOLUME,
                    SCRIPT_OBJECT_SHORT_MAX_DIST,
                    1,
                    2.0,
                    true,
                    true,
                    0.0,
                );
                state.shop_next_time = time_ms + 3000 + random_mod(audio, 3, 7000);
            }
            queued
        }
        SCRIPT_SOUND_AIRPORT_LOOP_S | SCRIPT_SOUND_AIRPORT_LOOP_L => {
            let max_distance = script_sound_range(sound_i);
            if time_ms <= state.airport_next_time {
                return false;
            }
            let sample = SFX_AIRPORT_ANNOUNCEMENT_1 as u32 + random_mod(audio, 1, 4);
            let queued = queue_positional_sample(
                audio,
                &source,
                sample,
                SFX_BANK_0,
                3,
                audio.sample_base_frequency(sample),
                AIRPORT_VOLUME,
                max_distance,
                1,
                2.0,
                true,
                true,
                0.0,
            );
            if queued {
                state.airport_counter = state.airport_counter.wrapping_add(1);
                state.airport_next_time = time_ms + 10000 + random_mod(audio, 3, 20000);
            }
            queued
        }
        SCRIPT_SOUND_CINEMA_LOOP_S | SCRIPT_SOUND_CINEMA_LOOP_L => {
            if time_ms <= state.cinema_next_time {
                return false;
            }
            let max_distance = script_sound_range(sound_i);
            let volume = random_mod(audio, 0, 90) as u8 + CINEMA_VOLUME;
            let sample = SFX_CINEMA_BASS_1 as u32 + (state.cinema_counter as u32 % 3);
            let queued = queue_positional_sample(
                audio,
                &source,
                sample,
                SFX_BANK_0,
                3,
                displaced_frequency(
                    audio,
                    audio.sample_base_frequency(sample) as i32,
                    audio.sample_base_frequency(sample) >> 4,
                ),
                volume,
                max_distance,
                1,
                2.0,
                true,
                true,
                0.0,
            );
            if queued {
                state.cinema_counter = state.cinema_counter.wrapping_add(1);
                state.cinema_next_time = time_ms + 1000 + random_mod(audio, 3, 4000);
            }
            queued
        }
        SCRIPT_SOUND_DOCKS_LOOP_S | SCRIPT_SOUND_DOCKS_LOOP_L => {
            if time_ms <= state.docks_next_time {
                return false;
            }
            let max_distance = script_sound_range(sound_i);
            let volume = random_mod(audio, 0, 60) as u8 + DOCKS_VOLUME;
            let sample = SFX_DOCKS_FOGHORN as u32;
            let queued = queue_positional_sample(
                audio,
                &source,
                sample,
                SFX_BANK_0,
                3,
                displaced_frequency(
                    audio,
                    audio.sample_base_frequency(sample) as i32,
                    audio.sample_base_frequency(sample) >> 3,
                ),
                volume,
                max_distance,
                1,
                2.0,
                true,
                true,
                0.0,
            );
            if queued {
                state.docks_counter = state.docks_counter.wrapping_add(1);
                state.docks_next_time = time_ms + 10000 + random_mod(audio, 3, 40000);
            }
            queued
        }
        SCRIPT_SOUND_HOME_LOOP_S | SCRIPT_SOUND_HOME_LOOP_L => {
            if time_ms <= state.home_next_time {
                return false;
            }
            let max_distance = script_sound_range(sound_i);
            let volume = random_mod(audio, 0, 30) as u8 + HOME_VOLUME;
            let sample = SFX_HOME_1 as u32 + random_mod(audio, 0, 5);
            let queued = queue_positional_sample(
                audio,
                &source,
                sample,
                SFX_BANK_0,
                3,
                displaced_frequency(
                    audio,
                    audio.sample_base_frequency(sample) as i32,
                    audio.sample_base_frequency(sample) >> 4,
                ),
                volume,
                max_distance,
                1,
                0.0,
                true,
                true,
                0.0,
            );
            if queued {
                state.home_counter = state.home_counter.wrapping_add(1);
                state.home_next_time = time_ms + 1000 + random_mod(audio, 3, 4000);
            }
            queued
        }
        SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_S | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_L => {
            if time_ms <= state.cell_next_time {
                return false;
            }
            let max_distance = script_sound_range(sound_i);
            let volume = random_mod(audio, 0, 50) as u8 + POLICE_CELL_BEATING_VOLUME;
            let sample = SFX_COL_PED_1 as u32 + random_mod(audio, 2, 5);
            let queued = queue_positional_sample(
                audio,
                &source,
                sample,
                SFX_BANK_0,
                3,
                displaced_frequency(
                    audio,
                    audio.sample_base_frequency(sample) as i32,
                    audio.sample_base_frequency(sample) >> 4,
                ),
                volume,
                max_distance,
                1,
                0.0,
                true,
                true,
                0.0,
            );
            if queued {
                state.cell_counter = state.cell_counter.wrapping_add(1);
                state.cell_next_time = time_ms + 500 + random_mod(audio, 3, 1500);
            }
            queued
        }
        _ => false,
    }
}

fn porn_samples(sound: i32) -> Option<(u32, u32, f32)> {
    match sound {
        SCRIPT_SOUND_PORN_CINEMA_1_S | SCRIPT_SOUND_MISTY_SEX_S => Some((
            SFX_PORN_1_LOOP as u32,
            SFX_PORN_1_GROAN_1 as u32,
            PORN_CINEMA_SHORT_MAX_DIST,
        )),
        SCRIPT_SOUND_PORN_CINEMA_1_L | SCRIPT_SOUND_MISTY_SEX_L => Some((
            SFX_PORN_1_LOOP as u32,
            SFX_PORN_1_GROAN_1 as u32,
            PORN_CINEMA_LONG_MAX_DIST,
        )),
        SCRIPT_SOUND_PORN_CINEMA_2_S => Some((
            SFX_PORN_2_LOOP as u32,
            SFX_PORN_2_GROAN_1 as u32,
            PORN_CINEMA_SHORT_MAX_DIST,
        )),
        SCRIPT_SOUND_PORN_CINEMA_2_L => Some((
            SFX_PORN_2_LOOP as u32,
            SFX_PORN_2_GROAN_1 as u32,
            PORN_CINEMA_LONG_MAX_DIST,
        )),
        SCRIPT_SOUND_PORN_CINEMA_3_S => Some((
            SFX_PORN_3_LOOP as u32,
            SFX_PORN_3_GROAN_1 as u32,
            PORN_CINEMA_SHORT_MAX_DIST,
        )),
        SCRIPT_SOUND_PORN_CINEMA_3_L => Some((
            SFX_PORN_3_LOOP as u32,
            SFX_PORN_3_GROAN_1 as u32,
            PORN_CINEMA_LONG_MAX_DIST,
        )),
        _ => None,
    }
}

fn script_sound_range(sound: i32) -> f32 {
    match sound {
        SCRIPT_SOUND_AIRPORT_LOOP_L
        | SCRIPT_SOUND_CINEMA_LOOP_L
        | SCRIPT_SOUND_DOCKS_LOOP_L
        | SCRIPT_SOUND_HOME_LOOP_L
        | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_L => SCRIPT_OBJECT_LONG_MAX_DIST,
        _ => SCRIPT_OBJECT_SHORT_MAX_DIST,
    }
}

fn is_long_script_loop(sound: i32) -> bool {
    matches!(
        sound,
        SCRIPT_SOUND_BANK_ALARM_LOOP_L
            | SCRIPT_SOUND_POLICE_BALL_LOOP_L
            | SCRIPT_SOUND_WORK_SHOP_LOOP_L
            | SCRIPT_SOUND_SAWMILL_LOOP_L
            | SCRIPT_SOUND_LAUNDERETTE_LOOP_L
            | SCRIPT_SOUND_SHOP_LOOP_L
            | SCRIPT_SOUND_AIRPORT_LOOP_L
            | SCRIPT_SOUND_CINEMA_LOOP_L
            | SCRIPT_SOUND_DOCKS_LOOP_L
            | SCRIPT_SOUND_HOME_LOOP_L
            | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_L
    ) || (SCRIPT_SOUND_PARTY_1_LOOP_L..=SCRIPT_SOUND_PARTY_12_LOOP_L).contains(&sound)
        && (sound - SCRIPT_SOUND_PARTY_1_LOOP_S) % 2 == 1
}

fn looping_script_sample(sound: i32) -> Option<(u32, u8, u32, f32, f32)> {
    if (SCRIPT_SOUND_PARTY_1_LOOP_S..=SCRIPT_SOUND_PARTY_12_LOOP_L).contains(&sound) {
        let party = ((sound - SCRIPT_SOUND_PARTY_1_LOOP_S) / 2).clamp(0, 11) as u32;
        return Some((
            SFX_CLUB_1 as u32 + party,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            3,
            3.0,
            2.0,
        ));
    }

    match sound {
        SCRIPT_SOUND_BANK_ALARM_LOOP_S | SCRIPT_SOUND_BANK_ALARM_LOOP_L => Some((
            SFX_BANK_ALARM_1 as u32,
            SCRIPT_OBJECT_BANK_ALARM_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_POLICE_BALL_LOOP_S | SCRIPT_SOUND_POLICE_BALL_LOOP_L => Some((
            SFX_POLICE_BALL_1 as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_WORK_SHOP_LOOP_S | SCRIPT_SOUND_WORK_SHOP_LOOP_L => Some((
            SFX_WORKSHOP_1 as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_SAWMILL_LOOP_S | SCRIPT_SOUND_SAWMILL_LOOP_L => Some((
            SFX_SAWMILL_LOOP as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_LAUNDERETTE_LOOP_S | SCRIPT_SOUND_LAUNDERETTE_LOOP_L => Some((
            SFX_LAUNDERETTE_LOOP as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_SHOP_LOOP_S | SCRIPT_SOUND_SHOP_LOOP_L => Some((
            SFX_SHOP_LOOP as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_AIRPORT_LOOP_S | SCRIPT_SOUND_AIRPORT_LOOP_L => Some((
            SFX_AIRPORT_ANNOUNCEMENT_1 as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_CINEMA_LOOP_S | SCRIPT_SOUND_CINEMA_LOOP_L => Some((
            SFX_CINEMA_BASS_1 as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_DOCKS_LOOP_S | SCRIPT_SOUND_DOCKS_LOOP_L => Some((
            SFX_DOCKS_FOGHORN as u32,
            SCRIPT_OBJECT_DEFAULT_VOLUME,
            2,
            3.0,
            2.0,
        )),
        SCRIPT_SOUND_HOME_LOOP_S | SCRIPT_SOUND_HOME_LOOP_L => {
            Some((SFX_HOME_1 as u32, SCRIPT_OBJECT_DEFAULT_VOLUME, 2, 3.0, 2.0))
        }
        SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_S | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_L => {
            Some((SFX_COL_PED_1 as u32, 55, 2, 3.0, 2.0))
        }
        _ => None,
    }
}

fn is_soft_shell_surface(surface: u8) -> bool {
    matches!(
        surface,
        x if x == eSurfaceType::SURFACE_GRASS as u8
            || x == eSurfaceType::SURFACE_GRAVEL as u8
            || x == eSurfaceType::SURFACE_MUD_DRY as u8
            || x == eSurfaceType::SURFACE_TRANSPARENT_CLOTH as u8
            || x == eSurfaceType::SURFACE_PED as u8
            || x == eSurfaceType::SURFACE_SAND as u8
            || x == eSurfaceType::SURFACE_RUBBER as u8
            || x == eSurfaceType::SURFACE_HEDGE as u8
    )
}

fn random_mod(audio: &cAudioManager, index: usize, modulo: u32) -> u32 {
    if modulo == 0 {
        0
    } else {
        audio.m_anRandomTable[index % audio.m_anRandomTable.len()].unsigned_abs() % modulo
    }
}

fn random_displacement(audio: &cAudioManager, seed: u32) -> i32 {
    if seed == 0 {
        0
    } else {
        let value = random_mod(audio, 0, seed) as i32;
        if value & 1 == 0 { value } else { -value }
    }
}

fn displaced_frequency(audio: &cAudioManager, base: i32, seed: u32) -> u32 {
    (base + random_displacement(audio, seed)).max(0) as u32
}

fn distance_squared(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

// === FAZ7d: moved from core/Game.rs (verbatim, parity-preserving) ===
// Vehicle audio world adapter: physical-entity registration plus the
// per-frame AudioLogic snapshot feed. Schedule stays in core/Game.rs.

#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::audio::audio_enums::AUDIOTYPE_PHYSICAL;
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Game::ScriptVehicleEntity;
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::{MI_DODO, MI_LANDSTAL, MI_RCBANDIT};
#[allow(unused_imports)]
use crate::renderer::Weather::CWeather;
#[allow(unused_imports)]
use crate::vehicles::Automobile::{CAutomobile, NUM_CAR_WHEELS};
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use bevy::prelude::*;

#[path = "AutomobileAudio.rs"]
mod automobile_audio;
pub use automobile_audio::process_single_automobile_audio;
#[cfg(test)]
use automobile_audio::update_gas_pedal_audio;

/// ProcessVehicle converts authored model IDs to audio-setting indices.
fn vehicle_audio_model_index(
    core: &crate::vehicles::Vehicle::VehicleCore,
    sequences: &mut VehicleOneShotAudioState,
) -> Option<usize> {
    let index = usize::try_from(core.physical.entity.model_index).ok()
        .and_then(|model| model.checked_sub(MI_LANDSTAL));
    if index.is_none() {
        sequences.invalid_model_events += 1;
        bevy::log::warn_once!(model = core.physical.entity.model_index, "vehicle_one_shot_invalid_model");
    }
    index
}

/// Native non-automobile ProcessVehicleOneShots positional/model interface.
fn process_common_vehicle_one_shots(
    dmaudio: &mut cDMAudio,
    core: &crate::vehicles::Vehicle::VehicleCore,
    camera_pos: CVector,
    sequences: &mut VehicleOneShotAudioState,
) {
    let Some(model_index) = vehicle_audio_model_index(core, sequences) else { return; };
    let position = core.physical.entity.get_position();
    let offset = position - camera_pos;
    let snapshot = VehicleAudioSnapshot {
        audio_entity: core.physical.audio_entity_id, model_index, position,
        distance_squared: offset.dot(offset), distance_calculated: false,
        ..Default::default()
    };
    drain_vehicle_one_shots(&mut dmaudio.AudioManager, &snapshot, sequences);
}

/// Shared ProcessRainOnVehicle tail for the non-automobile branches.
fn process_live_vehicle_rain(
    dmaudio: &mut cDMAudio,
    core: &crate::vehicles::Vehicle::VehicleCore,
    state: &mut crate::vehicles::Vehicle::VehicleAudioState,
    camera_pos: CVector,
    rain: f32,
) {
    let position = core.physical.entity.get_position();
    let offset = position - camera_pos;
    let snapshot = VehicleAudioSnapshot {
        audio_entity: core.physical.audio_entity_id,
        position,
        distance_squared: offset.dot(offset),
        distance_calculated: false,
        ..Default::default()
    };
    process_rain_on_vehicle(&mut dmaudio.AudioManager, &snapshot, rain,
        &mut state.rain_audio_counter, &mut state.rain_samples_counter);
}

/// Build the exact re3 `cVehicleParams` snapshot from live ECS automobiles
/// and run the already-ported AudioLogic vehicle branches before Service
/// drains the requested queue.
pub fn process_single_boat_audio<BoatCore, BoatVitality, BoatState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    dmaudio: &mut cDMAudio,
    boat: &mut crate::vehicles::Boat::BoatRuntimeView<BoatCore, BoatVitality, BoatState, Drive, Appearance, Entry, Audio>,
    _entity_handle: i32,
    camera_pos: CVector,
    rain: f32,
    one_shots: &mut VehicleOneShotAudioState,
)
where
    BoatCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    BoatVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    BoatState: std::ops::Deref<Target = crate::vehicles::Boat::BoatState>,
{
    if boat.vehicle.core.physical.audio_entity_id < 0 {
        bevy::log::warn_once!("audio_vehicle_waiting_for_physical_identity");
        return;
    }
    let position = boat.vehicle.core.physical.entity.get_position();
    let dx = position.x - camera_pos.x;
    let dy = position.y - camera_pos.y;
    let dz = position.z - camera_pos.z;
    let distance_squared = dx * dx + dy * dy + dz * dz;

    let model_index = boat.vehicle.core.physical.entity.model_index.max(0) as usize;
    let is_reefer = model_index == crate::modelinfo::ModelIndices::MI_REEFER;

    let velocity = boat.vehicle.core.physical.move_speed;
    let speed =
        (velocity.x * velocity.x + velocity.y * velocity.y + velocity.z * velocity.z).sqrt();

    let snapshot = VehicleAudioSnapshot {
        audio_entity: boat.vehicle.core.physical.audio_entity_id,
        model_index,
        position,
        distance_squared,
        distance_calculated: false,
        // TODO(parity): cAudioManager::ProcessBoatEngine — focused pad
        // acceleration/brake input and native shared LastAccel/LastVol.
        gas_pedal: boat.accelerate - boat.brake,
        boat_is_reefer: is_reefer,
        boat_propeller_in_water: boat.propeller_in_water,
        boat_in_water: boat.boat_in_water,
        boat_anchored: boat.is_anchored,
        velocity_change: speed,
        ..VehicleAudioSnapshot::default()
    };

    process_boat_engine(&mut dmaudio.AudioManager, &snapshot);
    process_boat_moving_over_water(&mut dmaudio.AudioManager, &snapshot);

    process_common_vehicle_one_shots(dmaudio, &boat.vehicle.core, camera_pos, one_shots);
    process_live_vehicle_rain(dmaudio, &boat.vehicle.core, &mut boat.vehicle.audio, camera_pos, rain);
}

pub fn process_single_heli_audio<HeliCore, HeliVitality, HeliState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    dmaudio: &mut cDMAudio,
    heli: &mut crate::vehicles::Heli::HeliRuntimeView<HeliCore, HeliVitality, HeliState, Drive, Appearance, Entry, Audio>,
    _entity_handle: i32,
    camera_pos: CVector,
    rain: f32,
    one_shots: &mut VehicleOneShotAudioState,
)
where
    HeliCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    HeliVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    HeliState: std::ops::Deref<Target = crate::vehicles::Heli::HeliState>,
{
    if heli.vehicle.core.physical.audio_entity_id < 0 {
        bevy::log::warn_once!("audio_vehicle_waiting_for_physical_identity");
        return;
    }
    let position = heli.vehicle.core.physical.entity.get_position();
    let dx = position.x - camera_pos.x;
    let dy = position.y - camera_pos.y;
    let dz = position.z - camera_pos.z;
    let distance_squared = dx * dx + dy * dy + dz * dz;

    let snapshot = VehicleAudioSnapshot {
        audio_entity: heli.vehicle.core.physical.audio_entity_id,
        model_index: heli.vehicle.core.physical.entity.model_index.max(0) as usize,
        position,
        distance_squared,
        distance_calculated: false,
        heli_id: heli.heli_id.max(0) as u32,
        ..VehicleAudioSnapshot::default()
    };

    process_helicopter(&mut dmaudio.AudioManager, &snapshot);

    process_common_vehicle_one_shots(dmaudio, &heli.vehicle.core, camera_pos, one_shots);
    process_live_vehicle_rain(dmaudio, &heli.vehicle.core, &mut heli.vehicle.audio, camera_pos, rain);
}

pub fn process_single_plane_audio<PlaneCore, PlaneVitality, PlaneState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    dmaudio: &mut cDMAudio,
    plane: &mut crate::vehicles::Plane::PlaneRuntimeView<PlaneCore, PlaneVitality, PlaneState, Drive, Appearance, Entry, Audio>,
    _entity_handle: i32,
    camera_pos: CVector,
    rain: f32,
    one_shots: &mut VehicleOneShotAudioState,
    player_engine: &mut PlayerEngineAudioState,
    plane_runtime: Option<&crate::vehicles::Plane::CPlaneRuntime>,
    jumbo: &mut JumboAudioState,
)
where
    PlaneCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    PlaneVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    PlaneState: std::ops::Deref<Target = crate::vehicles::Plane::PlaneState>,
{
    if plane.vehicle.core.physical.audio_entity_id < 0 {
        bevy::log::warn_once!("audio_vehicle_waiting_for_physical_identity");
        return;
    }
    let position = plane.vehicle.core.physical.entity.get_position();
    let dx = position.x - camera_pos.x;
    let dy = position.y - camera_pos.y;
    let dz = position.z - camera_pos.z;
    let distance_squared = dx * dx + dy * dy + dz * dz;

    let model_index = plane.vehicle.core.physical.entity.model_index.max(0) as usize;
    if model_index == crate::modelinfo::ModelIndices::MI_AIRTRAIN {
        let path = if distance_squared < JUMBO_MAX_DIST * JUMBO_MAX_DIST {
            jumbo.path_input(plane_runtime, plane.plane_id)
        } else { None };
        let snapshot = VehicleAudioSnapshot {
            audio_entity: plane.vehicle.core.physical.audio_entity_id,
            model_index,
            position,
            distance_squared,
            distance_calculated: false,
            jumbo_speed: plane.speed,
            ..VehicleAudioSnapshot::default()
        };

        if let Some(path) = path {
            process_jumbo(&mut dmaudio.AudioManager, &snapshot, path, jumbo);
        }
    } else if model_index == crate::modelinfo::ModelIndices::MI_DEADDODO {
        let snapshot = VehicleAudioSnapshot {
            audio_entity: plane.vehicle.core.physical.audio_entity_id,
            model_index,
            position,
            distance_squared,
            distance_calculated: false,
            ..VehicleAudioSnapshot::default()
        };
        process_cesna(
            &mut dmaudio.AudioManager,
            &snapshot,
            false,
            false,
            plane.speed > 0.0,
            &mut player_engine.cesna,
        );
    } else {
        jumbo.unsupported_model_events += 1;
        bevy::log::warn_once!(model_index, "plane_audio_unsupported_original_model");
    }

    process_common_vehicle_one_shots(dmaudio, &plane.vehicle.core, camera_pos, one_shots);
    process_live_vehicle_rain(dmaudio, &plane.vehicle.core, &mut plane.vehicle.audio, camera_pos, rain);
}

fn allocate_ped_audio_identity(dmaudio: &mut cDMAudio, ped: &mut crate::peds::Ped::CPed, entity_handle: i32) {
    if ped.physical.audio_entity_id < 0 {
        let entity = dmaudio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(entity_handle));
        ped.physical.audio_entity_id = entity;
        if entity >= 0 { dmaudio.SetEntityStatus(entity, true); }
    }
}

pub(crate) fn queue_ped_animation_audio(
    mut commands: Commands,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    index: Res<crate::control::ScriptEntities::ScriptEntityIndex>,
    mut peds: Query<(Entity, &crate::control::ScriptEntities::ScriptPedIdentity,
        &mut crate::peds::Ped::CPed, &crate::peds::PedAnimation::PedAnimationSound)>,
) {
    if peds.is_empty() { return; }
    let Some(audio) = dmaudio.as_deref_mut() else {
        bevy::log::warn_once!("ped_animation_sound_waiting_audio_manager");
        return;
    };
    for (entity, identity, mut ped, sound) in &mut peds {
        if index.ped(identity.handle) != Some(entity) { continue; }
        allocate_ped_audio_identity(audio, &mut ped, identity.handle);
        if ped.physical.audio_entity_id < 0 {
            bevy::log::warn_once!("ped_animation_sound_waiting_audio_slot");
            continue;
        }
        audio.PlayOneShot(ped.physical.audio_entity_id, sound.0, 1.0);
        commands.entity(entity).remove::<crate::peds::PedAnimation::PedAnimationSound>();
    }
}

pub fn process_single_ped_audio(
    dmaudio: &mut cDMAudio,
    ped: &mut crate::peds::Ped::CPed,
    is_player: bool,
    camera_pos: CVector,
    entity_handle: i32,
) {
    allocate_ped_audio_identity(dmaudio, ped, entity_handle);
    if ped.physical.audio_entity_id < 0 {
        return;
    }
    if let Some(sound) = ped.queued_footstep.take() {
        dmaudio.PlayOneShot(ped.physical.audio_entity_id, sound, 1.0);
    }
    let audio_id = ped.physical.audio_entity_id as usize;
    if audio_id >= dmaudio.AudioManager.m_asAudioEntities.len() {
        return;
    }
    let num_events = dmaudio.AudioManager.m_asAudioEntities[audio_id].m_AudioEvents as usize;
    if num_events == 0 {
        return;
    }
    let events = dmaudio.AudioManager.m_asAudioEntities[audio_id].m_awAudioEvent;
    dmaudio.AudioManager.m_asAudioEntities[audio_id].m_AudioEvents = 0;

    let position = ped.physical.entity.get_position();
    let dx = position.x - camera_pos.x;
    let dy = position.y - camera_pos.y;
    let dz = position.z - camera_pos.z;
    let distance_squared = dx * dx + dy * dy + dz * dz;

    let move_state = match ped.move_state {
        crate::peds::Ped::eMoveState::PEDMOVE_NONE
        | crate::peds::Ped::eMoveState::PEDMOVE_STILL => PedMoveStateForAudio::Still,
        crate::peds::Ped::eMoveState::PEDMOVE_WALK => PedMoveStateForAudio::Walk,
        crate::peds::Ped::eMoveState::PEDMOVE_RUN => PedMoveStateForAudio::Run,
        crate::peds::Ped::eMoveState::PEDMOVE_SPRINT => PedMoveStateForAudio::Sprint,
    };
    let weapon_type = ped
        .weapons
        .get(ped.current_weapon as usize)
        .map(|w| w.weapon_type)
        .unwrap_or(crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED);

    let snapshot = PedAudioSnapshot {
        audio_entity: ped.physical.audio_entity_id,
        position,
        distance_squared,
        distance_calculated: false,
        surface_touched: ped.physical.surface_touched,
        is_in_air: ped.flags.is_in_the_air,
        is_player,
        move_state,
        weapon_type,
    };

    let mut state = dmaudio.AudioManager.ped_one_shot_state;
    for sound in &events[..num_events.min(NUM_AUDIOENTITY_EVENTS)] {
        if *sound != crate::audio::soundlist::SOUND_NO_SOUND as i16 {
            process_ped_one_shot(&mut dmaudio.AudioManager, &snapshot, *sound, &mut state);
        }
    }
    dmaudio.AudioManager.ped_one_shot_state = state;
}

pub(crate) fn process_ped_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut actors: AudioPedEvents,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
) {
    if !diagnostics.audio_manager_available(dmaudio.is_some()) { return; }
    let Some(dmaudio) = dmaudio.as_deref_mut() else { return; };
    let camera_pos = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref());
    actors.for_each_player(|handle, ped| {
        if let Some(camera_pos) = camera_pos {
            process_single_ped_audio(dmaudio, ped, true, camera_pos, handle);
            diagnostics.observe_actor_audio_id(ped.physical.audio_entity_id, dmaudio.AudioManager.m_asAudioEntities.len());
        }
    });
    actors.for_each_script_ped(|handle, ped| {
        if let Some(camera_pos) = camera_pos {
            process_single_ped_audio(dmaudio, ped, false, camera_pos, handle);
            diagnostics.observe_actor_audio_id(ped.physical.audio_entity_id, dmaudio.AudioManager.m_asAudioEntities.len());
        }
    });
}

pub(crate) fn process_weather_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    weather: Option<Res<CWeather>>,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
) {
    let (Some(dmaudio), Some(weather)) = (dmaudio.as_deref_mut(), weather.as_deref()) else {
        return;
    };
    let handle = if weather.sound_handle >= 0 {
        weather.sound_handle
    } else {
        dmaudio.AudioManager.m_nWeatherAudioEntity
    };
    if handle < 0 {
        return;
    }
    let audio_id = handle as usize;
    if audio_id >= dmaudio.AudioManager.m_asAudioEntities.len() {
        return;
    }

    let Some(camera_pos) = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref()) else { return; };
    let mut lightning_volume = None;
    let num_events = dmaudio.AudioManager.m_asAudioEntities[audio_id].m_AudioEvents as usize;
    for i in 0..num_events.min(NUM_AUDIOENTITY_EVENTS) {
        if dmaudio.AudioManager.m_asAudioEntities[audio_id].m_awAudioEvent[i]
            == crate::audio::soundlist::SOUND_LIGHTNING as i16
        {
            lightning_volume = Some(dmaudio.AudioManager.m_asAudioEntities[audio_id].m_afVolume[i]);
            break;
        }
    }
    dmaudio.AudioManager.m_asAudioEntities[audio_id].m_AudioEvents = 0;

    let camera_sound_dist_up = camera_pos.z;
    let snapshot = WeatherAudioSnapshot {
        audio_entity: handle,
        lightning_volume,
        rain: weather.rain,
        rain_allowed: true,
        camera_sound_dist_up,
    };

    process_weather(&mut dmaudio.AudioManager, &snapshot);
}

pub(crate) fn process_vehicle_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut actors: AudioVehicleActors,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    handling: Option<Res<cHandlingDataMgr>>,
    weather: Option<Res<CWeather>>,
    file_loader: Option<Res<CFileLoader>>,
    pads: Option<Res<crate::core::Pad::CPads>>,
    players: AudioFocusedPlayer,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
    mut globals: VehicleAudioGlobals,
    mut speech: ResMut<crate::audio::PedTalk::PedTalkState>,
    timer: Res<CTimer>,
) {
    if !diagnostics.audio_manager_available(dmaudio.is_some()) { return; }
    let Some(dmaudio) = dmaudio.as_deref_mut() else { return; };
    let handling = handling.as_deref();
    if handling.is_none() {
        diagnostics.missing_handling_frames += 1;
        if diagnostics.missing_handling_frames == 1 { warn!("audio_vehicle_missing_handling"); }
    }
    let camera_pos = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref());
    if weather.is_none() {
        diagnostics.missing_weather_frames += 1;
        if diagnostics.missing_weather_frames == 1 { warn!("audio_vehicle_missing_weather"); }
    }
    if file_loader.is_none() {
        diagnostics.missing_rain_world_frames += 1;
        if diagnostics.missing_rain_world_frames == 1 { warn!("audio_vehicle_missing_rain_world"); }
    }
    let wet_roads = weather.as_ref().map(|weather| weather.wet_roads);
    let rain = weather.as_ref().zip(file_loader.as_ref()).map(|(weather, loader)| {
        let cull = &loader.world.cull_zones;
        if !cull.camera_no_rain() || !cull.player_no_rain() { weather.rain } else { 0.0 }
    });
    let player_vehicle_handle = players.vehicle().and_then(|entity| actors.handle_for_entity(entity));
    let player_audio_id = player_vehicle_handle.and_then(|handle| actors.vehicles.get_mut(&actors.index, &handle))
        .map(|vehicle| vehicle.base().physical.audio_entity_id).filter(|id| *id >= 0);
    let player_actor = player_vehicle_handle.and_then(|handle| actors.index.vehicle(handle));
    globals.player_engine.bind_actor(&mut dmaudio.AudioManager, player_actor, player_audio_id);
    let channel_used = player_audio_id.and_then(|id| globals.channel.is_used(id));
    let VehicleAudioGlobals { mut one_shots, mut player_engine, plane_runtime, mut jumbo, .. } = globals;


    for &handle in actors.index.vehicles.keys() {
        let Some(vehicle) = actors.vehicles.get_mut(&actors.index, &handle) else { continue; };
        // Retirement remains live while dependency waits preserve actor events.
        let (Some(camera_pos), Some(handling), Some(wet_roads), Some(rain)) = (camera_pos, handling, wet_roads, rain) else { continue; };
        let is_player = player_vehicle_handle == Some(handle);
        match vehicle {
            crate::audio::AudioActorAccess::AudioVehicleMut::Automobile { mut car, .. } => {
                let Some(handling_data) = car
                    .vehicle.core.handling_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|id| handling.get_handling_data(id))
                else {
                    diagnostics.missing_model_handling += 1;
                    if diagnostics.missing_model_handling == 1 {
                        warn!(model_index = car.vehicle.core.physical.entity.model_index, "audio_vehicle_missing_model_handling");
                    }
                    continue;
                };
                process_single_automobile_audio(
                    dmaudio,
                    &mut car,
                    handling_data,
                    handle,
                    camera_pos,
                    wet_roads,
                    rain,
                    is_player,
                    if is_player {
                        pads.as_ref().map(|pads| pads.pads[0].get_accelerate())
                    } else {
                        None
                    },
                    channel_used, &mut one_shots, &mut player_engine);
                let audio_id = car.vehicle.core.physical.audio_entity_id;
                diagnostics.observe_actor_audio_id(audio_id, dmaudio.AudioManager.m_asAudioEntities.len());
            }
            crate::audio::AudioActorAccess::AudioVehicleMut::Boat { mut boat, .. } => {
                process_single_boat_audio(dmaudio, &mut boat, handle, camera_pos, rain, &mut one_shots);
                let audio_id = boat.vehicle.core.physical.audio_entity_id;
                diagnostics.observe_actor_audio_id(audio_id, dmaudio.AudioManager.m_asAudioEntities.len());
            }
            crate::audio::AudioActorAccess::AudioVehicleMut::Heli { mut heli, .. } => {
                process_single_heli_audio(dmaudio, &mut heli, handle, camera_pos, rain, &mut one_shots);
                let audio_id = heli.vehicle.core.physical.audio_entity_id;
                diagnostics.observe_actor_audio_id(audio_id, dmaudio.AudioManager.m_asAudioEntities.len());
            }
            crate::audio::AudioActorAccess::AudioVehicleMut::Plane { mut plane, .. } => {
                process_single_plane_audio(dmaudio, &mut plane, handle, camera_pos, rain, &mut one_shots, &mut player_engine, plane_runtime.as_deref(), &mut jumbo);
                let audio_id = plane.vehicle.core.physical.audio_entity_id;
                diagnostics.observe_actor_audio_id(audio_id, dmaudio.AudioManager.m_asAudioEntities.len());
            }
        }
    }

    // Special vehicle speech is queued in this same pass, preserving event order
    // and PedTalk's global bodycast cooldown. Position/LOS are real world inputs.
    if let (Some(loader), Some(listener)) = (file_loader.as_deref(), camera_pos) {
        flush_vehicle_comments(&mut dmaudio.AudioManager, &mut one_shots, &mut speech,
            loader, listener, timer.time_in_milliseconds());
    }

}

#[allow(unused_imports)]
use crate::core::Fire::CFireManager;
#[allow(unused_imports)]
use crate::weapons::Explosion::{CExplosionPool, eExplosionType};

/// Drive re3 cAudioManager::ProcessExplosions from the ECS explosion pool.
/// re3 emits the blast sample once when the slot counter reads 1, then resets it.
pub(crate) fn process_explosion_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut explosions: Option<ResMut<CExplosionPool>>,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
) {
    let (Some(dmaudio), Some(explosions)) = (dmaudio.as_deref_mut(), explosions.as_deref_mut())
    else {
        return;
    };
    let Some(camera_pos) = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref()) else { return; };
    for slot in 0..explosions.explosions.len() {
        let (explosion_type, position, counter) = match explosions.explosions.get(slot) {
            Some(Some(explosion)) => (
                explosion.explosion_type,
                explosion.position,
                explosion.active_counter,
            ),
            _ => continue,
        };
        if counter != 1 {
            continue;
        }
        let kind = match explosion_type {
            eExplosionType::EXPLOSION_GRENADE => ExplosionAudioKind::Grenade,
            eExplosionType::EXPLOSION_MOLOTOV => ExplosionAudioKind::Molotov,
            eExplosionType::EXPLOSION_ROCKET => ExplosionAudioKind::Rocket,
            eExplosionType::EXPLOSION_CAR | eExplosionType::EXPLOSION_CAR_QUICK => {
                ExplosionAudioKind::Other
            }
            eExplosionType::EXPLOSION_HELI => ExplosionAudioKind::Heli,
            eExplosionType::EXPLOSION_MINE => ExplosionAudioKind::Mine,
            eExplosionType::EXPLOSION_BARREL => ExplosionAudioKind::Barrel,
            eExplosionType::EXPLOSION_TANK_GRENADE => ExplosionAudioKind::TankGrenade,
            eExplosionType::EXPLOSION_HELI_BOMB => ExplosionAudioKind::HeliBomb,
        };
        let dx = position.x - camera_pos.x;
        let dy = position.y - camera_pos.y;
        let dz = position.z - camera_pos.z;
        let snapshot = ExplosionAudioSnapshot {
            source: PositionalAudioSnapshot {
                audio_entity: 0,
                index: slot as u32,
                position,
                distance_squared: dx * dx + dy * dy + dz * dz,
                distance_calculated: false,
            },
            kind,
        };
        process_explosion(&mut dmaudio.AudioManager, &snapshot);
        explosions.ResetExplosionActiveCounter(slot as u8);
    }
}

/// Drive re3 cAudioManager::ProcessFires from the ECS fire manager.
/// re3 keys the burning loop off the burning entity type; the ECS fire record
/// carries no entity kind, so every active fire uses the default car-fire loop.
pub(crate) fn process_fire_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    fires: Option<Res<CFireManager>>,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
) {
    let (Some(dmaudio), Some(fires)) = (dmaudio.as_deref_mut(), fires.as_deref()) else {
        return;
    };
    let Some(camera_pos) = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref()) else { return; };
    let audio_entity = dmaudio.AudioManager.m_nFireAudioEntity;
    for (slot, fire) in fires.fires.iter().enumerate() {
        if !fire.active || !fire.audio_set {
            continue;
        }
        let dx = fire.position.x - camera_pos.x;
        let dy = fire.position.y - camera_pos.y;
        let dz = fire.position.z - camera_pos.z;
        let snapshot = FireAudioSnapshot {
            source: PositionalAudioSnapshot {
                audio_entity,
                index: slot as u32,
                position: fire.position,
                distance_squared: dx * dx + dy * dy + dz * dz,
                distance_calculated: false,
            },
            kind: FireAudioKind::Default,
        };
        process_fire(&mut dmaudio.AudioManager, &snapshot);
    }
}

pub fn is_script_building_sound(sound: i32) -> bool {
    matches!(
        sound,
        SCRIPT_SOUND_PORN_CINEMA_1_S
            | SCRIPT_SOUND_PORN_CINEMA_1_L
            | SCRIPT_SOUND_PORN_CINEMA_2_S
            | SCRIPT_SOUND_PORN_CINEMA_2_L
            | SCRIPT_SOUND_PORN_CINEMA_3_S
            | SCRIPT_SOUND_PORN_CINEMA_3_L
            | SCRIPT_SOUND_MISTY_SEX_S
            | SCRIPT_SOUND_MISTY_SEX_L
            | SCRIPT_SOUND_WORK_SHOP_LOOP_S
            | SCRIPT_SOUND_WORK_SHOP_LOOP_L
            | SCRIPT_SOUND_SAWMILL_LOOP_S
            | SCRIPT_SOUND_SAWMILL_LOOP_L
            | SCRIPT_SOUND_LAUNDERETTE_LOOP_S
            | SCRIPT_SOUND_LAUNDERETTE_LOOP_L
            | SCRIPT_SOUND_SHOP_LOOP_S
            | SCRIPT_SOUND_SHOP_LOOP_L
            | SCRIPT_SOUND_AIRPORT_LOOP_S
            | SCRIPT_SOUND_AIRPORT_LOOP_L
            | SCRIPT_SOUND_CINEMA_LOOP_S
            | SCRIPT_SOUND_CINEMA_LOOP_L
            | SCRIPT_SOUND_DOCKS_LOOP_S
            | SCRIPT_SOUND_DOCKS_LOOP_L
            | SCRIPT_SOUND_HOME_LOOP_S
            | SCRIPT_SOUND_HOME_LOOP_L
            | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_S
            | SCRIPT_SOUND_POLICE_CELL_BEATING_LOOP_L
    )
}

pub fn process_front_end_audio(audio: &mut cAudioManager) {
    let entity_index = audio.m_nFrontEndEntity;
    if entity_index < 0 {
        return;
    }
    let idx = entity_index as usize;
    if idx >= audio.m_asAudioEntities.len() {
        return;
    }
    if !audio.m_asAudioEntities[idx].m_bStatus {
        return;
    }

    let num_events = audio.m_asAudioEntities[idx].m_AudioEvents as usize;
    let events = audio.m_asAudioEntities[idx].m_awAudioEvent;
    let volumes = audio.m_asAudioEntities[idx].m_afVolume;

    for i in 0..num_events.min(NUM_AUDIOENTITY_EVENTS) {
        let sound = events[i];
        let Some(frontend_sample) = frontend_sample_for_sound(audio, sound) else {
            continue;
        };

        if frontend_sample.processed_pickup {
            if audio.m_FrameCounter <= audio.m_nPickupNextFrame {
                continue;
            }
            audio.m_nPickupNextFrame = audio.m_FrameCounter + 5;
        } else if frontend_sample.processed_mission {
            if audio.m_FrameCounter <= audio.m_nPartMisComNextFrame {
                continue;
            }
            audio.m_nPartMisComNextFrame = audio.m_FrameCounter + 5;
        }

        let frequency = if sound == SFX_RAIN as i16 {
            28509
        } else if sound == SFX_PICKUP_1_LEFT as i16
            || sound == SOUND_PICKUP_WEAPON as i16
            || sound == SOUND_PICKUP_WEAPON_BOUGHT as i16
        {
            if volumes[i] == 1.0 { 48000 } else { 32000 }
        } else {
            audio.sample_base_frequency(frontend_sample.sample)
        };

        let counter = audio.m_nFrontEndSoundCounter;
        audio.m_nFrontEndSoundCounter = audio.m_nFrontEndSoundCounter.wrapping_add(1);

        queue_frontend_sample(
            audio,
            entity_index,
            counter,
            frontend_sample.sample,
            frontend_sample.bank,
            frequency,
            frontend_sample.stereo,
            false,
        );

        if frontend_sample.stereo {
            let right_counter = audio.m_nFrontEndSoundCounter;
            audio.m_nFrontEndSoundCounter = audio.m_nFrontEndSoundCounter.wrapping_add(1);
            queue_frontend_sample(
                audio,
                entity_index,
                right_counter,
                frontend_sample.sample + 1,
                frontend_sample.bank,
                frequency,
                true,
                true,
            );
        }
    }

    audio.m_asAudioEntities[idx].m_AudioEvents = 0;
}

pub(crate) fn process_script_audio(
    mut dmaudio: Option<ResMut<cDMAudio>>,
    script_objects: Option<Res<cAudioScriptObjectPool>>,
    camera: Option<Res<crate::core::Camera::CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    timer: Res<CTimer>,
    players: AudioFocusedPlayer,
    mut diagnostics: ResMut<AudioActorDiagnostics>,
    mut building_state: Local<ScriptBuildingAudioState>,
) {
    if !diagnostics.audio_manager_available(dmaudio.is_some()) { return; }
    let Some(dmaudio) = dmaudio.as_deref_mut() else { return; };
    let Some(camera_pos) = diagnostics.listener_position(camera.as_deref(), file_loader.as_deref()) else { return; };
    let player_context = ScriptObjectAudioContext { player_surface: players.surface() };

    let time_ms = timer.time_in_milliseconds();

    // 1. Process and drain one-shot script objects
    let one_shot_total = dmaudio
        .AudioManager
        .m_sAudioScriptObjectManager
        .m_nScriptObjectEntityTotal;
    let one_shot_indices = dmaudio
        .AudioManager
        .m_sAudioScriptObjectManager
        .m_anScriptObjectEntityIndices;
    dmaudio
        .AudioManager
        .m_sAudioScriptObjectManager
        .m_nScriptObjectEntityTotal = 0;

    for &entity_id in &one_shot_indices[..one_shot_total] {
        if entity_id < 0 {
            continue;
        }
        let idx = entity_id as usize;
        if idx >= dmaudio.AudioManager.m_asAudioEntities.len() {
            continue;
        }
        let sound = dmaudio.AudioManager.m_asAudioEntities[idx].m_awAudioEvent[0] as u8;
        let Some(pos) = dmaudio.AudioManager.GetEntityPosition(entity_id) else {
            diagnostics.missing_script_positions += 1;
            if diagnostics.missing_script_positions == 1 { warn!("audio_script_missing_position"); }
            dmaudio.DestroyEntity(entity_id);
            continue;
        };
        dmaudio.AudioManager.m_sQueueSample.m_nEntityIndex = entity_id;
        process_one_shot_script_object(
            &mut dmaudio.AudioManager,
            sound,
            pos,
            camera_pos,
            player_context,
        );
        dmaudio.DestroyEntity(entity_id);
    }

    // 2. Process looping script objects in the pool
    if let Some(pool) = script_objects {
        let pool_size = pool.pool.get_size();
        for i in 0..pool_size {
            if let Some(obj) = pool.pool.get_slot(i) {
                if obj.AudioId == SCRIPT_SOUND_INVALID as i16 {
                    continue;
                }
                let entity_id = obj.AudioEntity;
                if entity_id < 0 {
                    continue;
                }
                dmaudio.AudioManager.m_sQueueSample.m_nEntityIndex = entity_id;
                let sound_u8 = obj.AudioId as u8;
                let sound_i = obj.AudioId as i32;
                if is_script_building_sound(sound_i) {
                    process_script_building_object(
                        &mut dmaudio.AudioManager,
                        sound_u8,
                        obj.Posn,
                        camera_pos,
                        time_ms,
                        &mut *building_state,
                    );
                } else {
                    process_looping_script_object(
                        &mut dmaudio.AudioManager,
                        sound_u8,
                        obj.Posn,
                        camera_pos,
                    );
                }
            }
        }
    }

    // 3. Process Frontend audio
    process_front_end_audio(&mut dmaudio.AudioManager);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_manager_with_test_metadata() -> cAudioManager {
        let mut audio = cAudioManager::default();
        install_test_sample_metadata(&mut audio);
        audio
    }
    fn install_test_sample_metadata(audio: &mut cAudioManager) {
        audio.SetSampleMetadata(&vec![crate::audio::sampman::SampleMetadata {
            frequency: 22050, size: 44100, loop_end: -1, ..Default::default()
        }; TOTAL_AUDIO_SAMPLES as usize]);
    }

    #[test]
    fn rain_tail_updates_canonical_boat_heli_and_plane_counters() {
        let mut one_shots = VehicleOneShotAudioState::default();
        let mut player_engine = PlayerEngineAudioState::default();
        let mut general = re3bevy_core::core::General::CGeneral::with_seed(9);
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        dmaudio.AudioManager.SetSampleMetadata(&vec![crate::audio::sampman::SampleMetadata {
            frequency: 22050, loop_end: -1, ..Default::default()
        }; SFX_BOAT_SPLASH_1 as usize + 3]);
        let mut boat = crate::vehicles::Boat::CBoat::new(
            crate::modelinfo::ModelIndices::MI_PREDATOR as i32, 0, 0, &mut general, None, None);
        let mut heli = crate::vehicles::Heli::CHeli::new(
            crate::modelinfo::ModelIndices::MI_CHOPPER as i32, 0, 0, &mut general);
        let mut plane = crate::vehicles::Plane::CPlane::new(
            crate::modelinfo::ModelIndices::MI_AIRTRAIN as i32, 0, 0, &mut general);
        for (handle, vehicle) in [(10, &mut boat.vehicle), (20, &mut heli.vehicle), (30, &mut plane.vehicle)] {
            crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut vehicle.core, handle);
        }
        for vehicle in [&boat.vehicle, &heli.vehicle, &plane.vehicle] {
            dmaudio.PlayOneShot(vehicle.core.physical.audio_entity_id, SOUND_CAR_SPLASH as u16, 1200.0);
        }
        for _ in 0..2 {
            process_single_boat_audio(&mut dmaudio, &mut boat, 10, CVector::ZERO, 0.5, &mut one_shots);
            process_single_heli_audio(&mut dmaudio, &mut heli, 20, CVector::ZERO, 0.5, &mut one_shots);
            process_single_plane_audio(&mut dmaudio, &mut plane, 30, CVector::ZERO, 0.5, &mut one_shots, &mut player_engine, None, &mut JumboAudioState::default());
        }
        assert_eq!(one_shots.processed_events, 3);
        assert_eq!(one_shots.queued_events, 3);
        assert_eq!(one_shots.wave_index, 44);
        for audio in [&boat.vehicle.audio, &heli.vehicle.audio, &plane.vehicle.audio] {
            assert_eq!(audio.rain_audio_counter, 0);
            assert_eq!(audio.rain_samples_counter, 1);
        }
        let queue = dmaudio.AudioManager.m_nActiveQueue;
        let count = dmaudio.AudioManager.m_nRequestedCount[queue] as usize;
        assert_eq!(dmaudio.AudioManager.m_aRequestedQueue[queue][..count].iter()
            .filter(|sample| (SFX_CAR_RAIN_1 as u32..=SFX_CAR_RAIN_1 as u32 + 3).contains(&sample.m_nSampleIndex)).count(), 3);
    }

    #[test]
    fn horn_persistent_writes_follow_native_distance_and_model_gates() {
        let mut audio = audio_manager_with_test_metadata();
        audio.m_FrameCounter = 7;
        for (distance, model, siren, status, expected_timer) in [
            (VEHICLE_HORN_MAX_DIST * VEHICLE_HORN_MAX_DIST, 0, false, VehicleStatusForAudio::Other, 45),
            (0.0, MRWHOOP, false, VehicleStatusForAudio::Other, 45),
            (0.0, POLICE, true, VehicleStatusForAudio::Other, 45),
            (0.0, 0, false, VehicleStatusForAudio::Player, 45),
            (0.0, 0, false, VehicleStatusForAudio::Other, 44),
        ] {
            let mut state = crate::vehicles::Vehicle::VehicleAudioState {
                car_horn_timer: 45, car_horn_pattern: 3, ..Default::default()
            };
            let mut snapshot = VehicleAudioSnapshot {
                audio_entity: 3, distance_squared: distance, model_index: model,
                siren_or_alarm: siren, status, horn_timer: 45, horn_pattern: 3,
                ..Default::default()
            };
            prepare_vehicle_horn(&audio, &mut state, &mut snapshot);
            assert_eq!(state.car_horn_timer, expected_timer);
            assert_eq!(snapshot.horn_timer, i32::from(expected_timer));
            assert_eq!(state.car_horn_pattern, if expected_timer == 44 { 2 } else { 3 });
        }
    }

    #[test]
    fn npc_engine_persists_native_airborne_gas_reduction_and_preserves_filter_gates() {
        let mut audio = audio_manager_with_test_metadata();
        let mut snapshot = VehicleAudioSnapshot {
            engine_on: true, wheels_on_ground: 0, drive_wheels_on_ground: 1,
            model_index: LANDSTAL, ..Default::default()
        };
        let mut gas = 0.8;
        assert!(process_vehicle_engine(&mut audio, &snapshot, &mut gas));
        assert!((gas - 0.32).abs() < 1.0e-6);
        snapshot.engine_on = false;
        assert!(process_vehicle_engine(&mut audio, &snapshot, &mut gas));
        assert!((gas - 0.32).abs() < 1.0e-6);
        snapshot.engine_on = true;
        snapshot.distance_squared = VEHICLE_ENGINE_MAX_DIST * VEHICLE_ENGINE_MAX_DIST;
        assert!(!process_vehicle_engine(&mut audio, &snapshot, &mut gas));
        assert!((gas - 0.32).abs() < 1.0e-6);
        snapshot.distance_squared = 0.0;
        snapshot.model_index = MAX_CARS;
        assert!(!process_vehicle_engine(&mut audio, &snapshot, &mut gas));
        assert!((gas - 0.32).abs() < 1.0e-6);
    }

    #[test]
    fn gas_pedal_audio_uses_re3_fixed_attack_and_release_steps() {
        assert!((update_gas_pedal_audio(0.0, 1.0) - 0.09).abs() < f32::EPSILON);
        assert!((update_gas_pedal_audio(0.95, 1.0) - 1.0).abs() < f32::EPSILON);
        assert!((update_gas_pedal_audio(1.0, 0.0) - 0.93).abs() < f32::EPSILON);
        assert!((update_gas_pedal_audio(0.04, 0.0) - 0.0).abs() < f32::EPSILON);
        assert!((update_gas_pedal_audio(0.3, -0.8) - 0.39).abs() < f32::EPSILON);
    }

    #[test]
    fn engine_damage_audio_stops_above_re3_fire_range() {
        assert_eq!(engine_damage_audio_state(99), EngineDamageAudioState::None);
        assert_eq!(
            engine_damage_audio_state(100),
            EngineDamageAudioState::Damaged
        );
        assert_eq!(
            engine_damage_audio_state(224),
            EngineDamageAudioState::Damaged
        );
        assert_eq!(
            engine_damage_audio_state(225),
            EngineDamageAudioState::OnFire
        );
        assert_eq!(
            engine_damage_audio_state(250),
            EngineDamageAudioState::OnFire
        );
        assert_eq!(engine_damage_audio_state(251), EngineDamageAudioState::None);
        assert_eq!(engine_damage_audio_state(255), EngineDamageAudioState::None);
    }

    #[test]
    fn npc_horn_patterns_keep_re3_silence_slots() {
        assert!(!horn_pattern_enabled(0, 0));
        assert!(!horn_pattern_enabled(0, 1));
        assert!(horn_pattern_enabled(0, 2));
        assert!(!horn_pattern_enabled(0, 17));
        assert!(horn_pattern_enabled(4, 2));
        assert!(!horn_pattern_enabled(4, 11));
        assert!(!horn_pattern_enabled(7, 43));
    }

    #[test]
    fn vehicle_skid_uses_drive_wheel_state_and_ignores_missing_wheels() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(entity, true);
        let mut snapshot = VehicleAudioSnapshot {
            audio_entity: entity,
            distance_squared: 0.0,
            wheels_on_ground: 4,
            max_velocity: 40.0,
            velocity_change: 15.0,
            gas_pedal_audio: 1.0,
            drive_type: 'F',
            ..Default::default()
        };
        snapshot.wheel_states[crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT] =
            crate::vehicles::Vehicle::tWheelState::WHEEL_STATE_SPINNING;
        assert!(process_vehicle_skidding(&mut audio, &snapshot));
        let queue = audio.m_nActiveQueue;
        assert_eq!(audio.m_nRequestedCount[queue], 1);
        assert_eq!(
            audio.m_aRequestedQueue[queue][0].m_nSampleIndex,
            SFX_SKID as u32
        );

        audio.ClearRequestedQueue();
        snapshot.wheel_missing[crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT] = true;
        assert!(process_vehicle_skidding(&mut audio, &snapshot));
        assert_eq!(audio.m_nRequestedCount[audio.m_nActiveQueue], 0);
    }

    #[test]
    fn full_throttle_engine_queues_an_audible_sample() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let snapshot = VehicleAudioSnapshot {
            audio_entity: 7,
            position: CVector::ZERO,
            distance_squared: 0.0,
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(false),
            engine_on: true,
            gas_pedal: 1.0,
            gas_pedal_audio: 0.8,
            velocity_change: 10.0,
            max_velocity: 40.0,
            current_gear: 2,
            ..Default::default()
        };
        let mut gas_pedal_audio = snapshot.gas_pedal_audio;
        assert!(process_vehicle_engine(&mut audio, &snapshot, &mut gas_pedal_audio));
        audio.ProcessActiveQueues();
        assert!(
            audio.emitted_samples.iter().any(|s| s.m_nVolume > 0),
            "engine should emit an audible sample at full throttle"
        );
    }

    #[test]
    fn player_vehicle_engine_accelerating_emits_audio() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let audio_entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(audio_entity, true);
        let mut state = PlayerVehicleAudioState::default();
        let mut gas_pedal_audio = 0.0;
        let snapshot = VehicleAudioSnapshot {
            audio_entity,
            position: CVector::ZERO,
            distance_squared: 0.0,
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(false),
            engine_on: true,
            gas_pedal: 1.0,
            gas_pedal_audio: 0.8,
            velocity_change: 15.0,
            previous_velocity_change_for_audio: 10.0,
            max_velocity: 40.0,
            current_gear: 2,
            wheels_on_ground: 4,
            drive_wheels_on_ground: 2,
            num_gears: 5,
            ..Default::default()
        };
        assert!(process_players_vehicle_engine(
            &mut audio,
            &snapshot,
            &mut state,
            &mut gas_pedal_audio
        ));
        audio.ProcessActiveQueues();
        assert!(
            audio.emitted_samples.iter().any(|s| s.m_nVolume > 0),
            "player vehicle should emit audible audio during acceleration"
        );
    }

    #[test]
    fn player_vehicle_engine_idle_emits_audio() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let audio_entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(audio_entity, true);
        let mut state = PlayerVehicleAudioState::default();
        let mut gas_pedal_audio = 0.0;
        let snapshot = VehicleAudioSnapshot {
            audio_entity,
            position: CVector::ZERO,
            distance_squared: 0.0,
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(false),
            engine_on: true,
            gas_pedal: 0.0,
            gas_pedal_audio: 0.0,
            velocity_change: 0.0,
            max_velocity: 40.0,
            current_gear: 1,
            wheels_on_ground: 4,
            num_gears: 5,
            ..Default::default()
        };
        assert!(process_players_vehicle_engine(
            &mut audio,
            &snapshot,
            &mut state,
            &mut gas_pedal_audio
        ));
        audio.ProcessActiveQueues();
        assert!(
            audio
                .emitted_samples
                .iter()
                .any(|s| s.m_nVolume > 0 && s.m_nCounter == 52),
            "player vehicle should emit audible idle loop on counter 52"
        );
    }

    #[test]
    fn player_vehicle_engine_radius_filter_preserves_native_channel_history() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let mut state = PlayerVehicleAudioState {
            player_just_entered_car: false,
            accel_sample_stopped: false,
            cruising: 2,
            ..Default::default()
        };
        let mut gas_pedal_audio = 0.8;
        let snapshot = VehicleAudioSnapshot {
            audio_entity: 7,
            distance_squared: (VEHICLE_ENGINE_MAX_DIST + 1.0).powi(2),
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(false),
            engine_on: true,
            ..Default::default()
        };

        assert!(!process_players_vehicle_engine(
            &mut audio,
            &snapshot,
            &mut state,
            &mut gas_pedal_audio
        ));
        assert!(!state.accel_sample_stopped);
        assert_eq!(state.cruising, 2);
        assert!(!audio.stopped_voice_keys.contains(&(7, 65)));
    }

    #[test]
    fn player_vehicle_engine_throttle_off_emits_finger_off_accel() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let audio_entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(audio_entity, true);
        let mut state = PlayerVehicleAudioState {
            last_accel: 200,
            player_just_entered_car: false,
            ..Default::default()
        };
        let mut gas_pedal_audio = 0.5;
        let snapshot = VehicleAudioSnapshot {
            audio_entity,
            position: CVector::ZERO,
            distance_squared: 0.0,
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(false),
            engine_on: true,
            gas_pedal: 0.0,
            gas_pedal_audio: 0.5,
            velocity_change: 5.0,
            max_velocity: 40.0,
            current_gear: 2,
            wheels_on_ground: 4,
            num_gears: 5,
            ..Default::default()
        };
        assert!(process_players_vehicle_engine(
            &mut audio,
            &snapshot,
            &mut state,
            &mut gas_pedal_audio
        ));
        audio.ProcessActiveQueues();
        assert!(
            audio.emitted_samples.iter().any(|s| s.m_nCounter == 63),
            "releasing throttle should emit finger-off-accel sound (counter 63)"
        );
    }

    #[test]
    fn player_vehicle_engine_gear_progression_respects_physical_gear() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();
        let audio_entity = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(1));
        audio.SetEntityStatus(audio_entity, true);
        let accel_sample = 3
            * (A_VEHICLE_SETTINGS[LANDSTAL]
                .m_nBank
                .saturating_sub(CAR_SFX_BANKS_OFFSET) as usize)
            + SFX_CAR_ACCEL_1 as usize;
        let mut metadata = vec![crate::audio::sampman::SampleMetadata::default(); accel_sample + 1];
        metadata[accel_sample] = crate::audio::sampman::SampleMetadata {
            size: 186_480,
            frequency: 22_050,
            ..Default::default()
        };
        audio.SetSampleMetadata(&metadata);
        let mut state = PlayerVehicleAudioState {
            player_just_entered_car: false,
            accel_sample_stopped: true,
            current_pretend_gear: 1,
            ..Default::default()
        };
        let mut gas_pedal_audio = 0.8;
        // re3 keeps the dedicated acceleration clip alive until the authored
        // sample itself finishes; physical transmission changes do not force
        // the synthetic audio gear forward.
        for frame in 0..30 {
            let snapshot = VehicleAudioSnapshot {
                audio_entity,
                position: CVector::ZERO,
                distance_squared: 0.0,
                status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(frame != 0),
                engine_on: true,
                gas_pedal: 1.0,
                gas_pedal_audio: 0.8,
                velocity_change: 15.0,
                previous_velocity_change_for_audio: 10.0,
                max_velocity: 40.0,
                current_gear: 2,
                wheels_on_ground: 4,
                drive_wheels_on_ground: 2,
                num_gears: 5,
                ..Default::default()
            };
            assert!(process_players_vehicle_engine(
                &mut audio,
                &snapshot,
                &mut state,
                &mut gas_pedal_audio
            ));
        }
        // Pretend gear should still be gear 1 (not jumped to 2 or 3 or cruising after just 1 second)
        assert_eq!(state.current_pretend_gear, 1);
        assert_eq!(state.cruising, 0);

        // A physical shift to gear 3 still leaves the currently playing audio
        // clip alone.
        let snapshot_gear3 = VehicleAudioSnapshot {
            audio_entity,
            position: CVector::ZERO,
            distance_squared: 0.0,
            status: VehicleStatusForAudio::Player,
            player_engine_channel_used: Some(true),
            engine_on: true,
            gas_pedal: 1.0,
            gas_pedal_audio: 0.8,
            velocity_change: 25.0,
            previous_velocity_change_for_audio: 20.0,
            max_velocity: 40.0,
            current_gear: 3,
            wheels_on_ground: 4,
            drive_wheels_on_ground: 2,
            num_gears: 5,
            ..Default::default()
        };
        assert!(process_players_vehicle_engine(
            &mut audio,
            &snapshot_gear3,
            &mut state,
            &mut gas_pedal_audio
        ));
        assert_eq!(state.current_pretend_gear, 1);
        assert_eq!(state.cruising, 0);

        // Actual backend completion, rather than elapsed simulation time,
        // selects GetChannelUsedFlag(false) and advances the pretend gear.
        let finished = VehicleAudioSnapshot { player_engine_channel_used: Some(false), ..snapshot_gear3 };
        assert!(process_players_vehicle_engine(&mut audio, &finished, &mut state, &mut gas_pedal_audio));
        assert_eq!(state.current_pretend_gear, 2);
    }
    #[test]
    fn ped_footstep_and_weapon_audio_queues_samples() {
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut general = crate::core::General::CGeneral::default();
        let mut ped = crate::peds::Ped::CPed::new(
            crate::peds::PedType::ePedType::PEDTYPE_PLAYER1,
            &mut general,
        );
        ped.physical.surface_touched =
            crate::core::SurfaceTable::eSurfaceType::SURFACE_PAVEMENT as u8;
        ped.queued_footstep = Some(crate::audio::soundlist::SOUND_STEP_START as u16);

        process_single_ped_audio(&mut dmaudio, &mut ped, true, CVector::ZERO, 10);
        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] > 0);
        let sample = dmaudio.AudioManager.m_aRequestedQueue[queue][0].m_nSampleIndex;
        assert!(
            sample >= SFX_FOOTSTEP_CONCRETE_1 as u32
                && sample <= SFX_FOOTSTEP_CONCRETE_1 as u32 + 5
        );

        // Weapon firing
        dmaudio.AudioManager.m_nRequestedCount[queue] = 0;
        ped.current_weapon = crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_COLT45;
        ped.weapons[crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_COLT45 as usize]
            .weapon_type = crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_COLT45;
        dmaudio.PlayOneShot(
            ped.physical.audio_entity_id,
            crate::audio::soundlist::SOUND_WEAPON_SHOT_FIRED as u16,
            0.0,
        );
        process_single_ped_audio(&mut dmaudio, &mut ped, true, CVector::ZERO, 10);
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] > 0);
        assert_eq!(
            dmaudio.AudioManager.m_aRequestedQueue[queue][0].m_nSampleIndex,
            SFX_COLT45_LEFT as u32
        );
    }

    #[test]
    fn successive_weapon_shots_keep_distinct_native_channel_identities() {
        let mut audio = cDMAudio::default();
        audio.Initialise();
        let mut general = crate::core::General::CGeneral::default();
        let mut ped = crate::peds::Ped::CPed::new(
            crate::peds::PedType::ePedType::PEDTYPE_PLAYER1, &mut general);
        ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
        ped.weapons[ped.current_weapon as usize].weapon_type = ped.current_weapon;
        process_single_ped_audio(&mut audio, &mut ped, true, CVector::ZERO, 10);
        let mut identities = std::collections::HashSet::new();
        for _ in 0..8 {
            audio.AudioManager.ClearRequestedQueue();
            audio.PlayOneShot(ped.physical.audio_entity_id, SOUND_WEAPON_SHOT_FIRED as u16, 0.0);
            process_single_ped_audio(&mut audio, &mut ped, true, CVector::ZERO, 10);
            let queue = audio.AudioManager.m_nActiveQueue;
            assert_eq!(audio.AudioManager.m_nRequestedCount[queue], 2);
            for sample in &audio.AudioManager.m_aRequestedQueue[queue][..2] {
                assert!(identities.insert((sample.m_nEntityIndex, sample.m_nCounter)),
                    "new shot reused a still-playing stereo channel");
            }
        }
    }

    #[test]
    fn boat_and_heli_audio_queues_samples() {
        let mut one_shots = VehicleOneShotAudioState::default();
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut general = crate::core::General::CGeneral::default();
        let mut boat = crate::vehicles::Boat::CBoat::new(
            crate::modelinfo::ModelIndices::MI_PREDATOR as i32,
            0,
            0,
            &mut general,
            None,
            None,
        );
        boat.accelerate = 1.0;
        boat.boat_in_water = true;
        boat.vehicle.core.physical.move_speed = CVector::new(10.0, 0.0, 0.0);

        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut boat.vehicle.core, 0);

        process_single_boat_audio(&mut dmaudio, &mut boat, 20, CVector::ZERO, 0.0, &mut one_shots);
        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] > 0);

        // Helicopter audio
        dmaudio.AudioManager.m_nRequestedCount[queue] = 0;
        let mut heli = crate::vehicles::Heli::CHeli::new(
            crate::modelinfo::ModelIndices::MI_CHOPPER as i32,
            0,
            0,
            &mut general,
        );
        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut heli.vehicle.core, 0);
        process_single_heli_audio(&mut dmaudio, &mut heli, 30, CVector::ZERO, 0.0, &mut one_shots);
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] > 0);
    }

    #[test]
    fn weather_audio_queues_rain_and_lightning() {
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut weather = crate::renderer::Weather::CWeather::init();
        weather.sound_handle =
            dmaudio.CreateEntity(crate::audio::audio_enums::AUDIOTYPE_WEATHER, Some(1));
        weather.rain = 0.8;

        let snapshot = WeatherAudioSnapshot {
            audio_entity: weather.sound_handle,
            lightning_volume: Some(15.0),
            rain: weather.rain,
            rain_allowed: true,
            camera_sound_dist_up: 20.0,
        };
        process_weather(&mut dmaudio.AudioManager, &snapshot);
        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] >= 2);
    }

    #[test]
    fn test_one_shot_script_audio_queues_and_cleans_up() {
        use crate::audio::AudioScriptObject::cAudioScriptObject;

        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);

        let mut script_obj = cAudioScriptObject {
            AudioId: SCRIPT_SOUND_BOX_DESTROYED_1 as i16,
            Posn: CVector::new(10.0, 10.0, 5.0),
            AudioEntity: crate::audio::AudioScriptObject::AEHANDLE_NONE,
        };
        dmaudio.CreateOneShotScriptObject(&mut script_obj);
        assert_eq!(
            dmaudio
                .AudioManager
                .m_sAudioScriptObjectManager
                .m_nScriptObjectEntityTotal,
            1
        );

        let entity_id = dmaudio
            .AudioManager
            .m_sAudioScriptObjectManager
            .m_anScriptObjectEntityIndices[0];
        let pos = dmaudio.AudioManager.GetEntityPosition(entity_id);
        assert_eq!(pos, Some(CVector::new(10.0, 10.0, 5.0)));

        let one_shot_total = dmaudio
            .AudioManager
            .m_sAudioScriptObjectManager
            .m_nScriptObjectEntityTotal;
        let one_shot_indices = dmaudio
            .AudioManager
            .m_sAudioScriptObjectManager
            .m_anScriptObjectEntityIndices;
        dmaudio
            .AudioManager
            .m_sAudioScriptObjectManager
            .m_nScriptObjectEntityTotal = 0;

        for &eid in &one_shot_indices[..one_shot_total] {
            let sound =
                dmaudio.AudioManager.m_asAudioEntities[eid as usize].m_awAudioEvent[0] as u8;
            let entity_pos = dmaudio
                .AudioManager
                .GetEntityPosition(eid)
                .unwrap_or(CVector::ZERO);
            dmaudio.AudioManager.m_sQueueSample.m_nEntityIndex = eid;
            let queued = process_one_shot_script_object(
                &mut dmaudio.AudioManager,
                sound,
                entity_pos,
                CVector::ZERO,
                ScriptObjectAudioContext::default(),
            );
            assert!(queued);
            dmaudio.DestroyEntity(eid);
        }

        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert!(dmaudio.AudioManager.m_nRequestedCount[queue] > 0);
        assert_eq!(dmaudio.AudioManager.destroyed_entities, vec![entity_id]);
    }

    #[test]
    fn test_front_end_audio_stereo_pickup() {
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        dmaudio.AudioManager.m_FrameCounter = 1;

        let fe_id = dmaudio.AudioManager.m_nFrontEndEntity;
        assert!(fe_id >= 0);

        dmaudio.PlayFrontEndSound(SOUND_PICKUP_WEAPON as u16, 0);
        assert_eq!(
            dmaudio.AudioManager.m_asAudioEntities[fe_id as usize].m_AudioEvents,
            1
        );

        process_front_end_audio(&mut dmaudio.AudioManager);
        assert_eq!(
            dmaudio.AudioManager.m_asAudioEntities[fe_id as usize].m_AudioEvents,
            0
        );

        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert_eq!(dmaudio.AudioManager.m_nRequestedCount[queue], 2); // Left and Right channels for stereo
        assert_eq!(
            dmaudio.AudioManager.m_aRequestedQueue[queue][0].m_nSampleIndex,
            SFX_PICKUP_1_LEFT as u32
        );
        assert_eq!(
            dmaudio.AudioManager.m_aRequestedQueue[queue][1].m_nSampleIndex,
            (SFX_PICKUP_1_LEFT + 1) as u32
        );
        assert_eq!(
            dmaudio.AudioManager.m_aRequestedQueue[queue][0].m_nVolume,
            FRONTEND_VOLUME
        );

        // Next frame pickup is rate-limited (cooldown is frame + 5)
        dmaudio.AudioManager.m_FrameCounter = 2;
        dmaudio.PlayFrontEndSound(SOUND_PICKUP_WEAPON as u16, 0);
        dmaudio.AudioManager.m_nRequestedCount[queue] = 0;
        process_front_end_audio(&mut dmaudio.AudioManager);
        assert_eq!(dmaudio.AudioManager.m_nRequestedCount[queue], 0);
    }

    #[test]
    fn test_looping_script_object_bank_alarm() {
        let mut audio = audio_manager_with_test_metadata();
        audio.Initialise();

        let queued = process_looping_script_object(
            &mut audio,
            SCRIPT_SOUND_BANK_ALARM_LOOP_S as u8,
            CVector::new(5.0, 5.0, 0.0),
            CVector::ZERO,
        );
        assert!(queued);
        let queue = audio.m_nActiveQueue;
        assert_eq!(audio.m_nRequestedCount[queue], 1);
        assert_eq!(
            audio.m_aRequestedQueue[queue][0].m_nSampleIndex,
            SFX_BANK_ALARM_1 as u32
        );
        assert_eq!(audio.m_aRequestedQueue[queue][0].m_nLoopCount, 0); // infinite loop
    }

    #[test]
    fn engine_off_preserves_dedicated_channel_until_backend_completion() {
        let mut one_shots = VehicleOneShotAudioState::default();
        let mut player_engine = PlayerEngineAudioState::default();
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);

        let mut general = crate::core::General::CGeneral::default();
        let mut car = crate::vehicles::Automobile::CAutomobile::new(
            crate::modelinfo::ModelIndices::MI_KURUMA as i32,
            0,
            0,
            &mut general,
            None,
            None,
            false,
        );
        car.vehicle.drive.engine_on = false;

        let handling = crate::vehicles::HandlingMgr::tHandlingData::default();
        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut car.vehicle.core, 0);
        process_single_automobile_audio(
            &mut dmaudio,
            &mut car,
            &handling,
            15,
            CVector::ZERO,
            0.0,
            0.0,
            true,
            None,
            Some(false), &mut one_shots, &mut player_engine);

        assert!(car.vehicle.core.physical.audio_entity_id >= 0);
        assert!(!dmaudio.AudioManager.stopped_voice_keys.contains(&(car.vehicle.core.physical.audio_entity_id, 65)));
        assert!(!dmaudio.AudioManager.stopped_engine_entities.contains(&car.vehicle.core.physical.audio_entity_id));

        let intents = dmaudio.drain_frame_intents();
        assert!(!intents.stopped_engine_entities.contains(&car.vehicle.core.physical.audio_entity_id));
        assert!(!intents.stopped_voice_keys.contains(&(car.vehicle.core.physical.audio_entity_id, 65)));
    }

    #[test]
    fn plane_audio_matches_re3_airtrain_deaddodo_dispatch() {
        let mut one_shots = VehicleOneShotAudioState::default();
        let mut player_engine = PlayerEngineAudioState::default();
        let mut general = crate::core::General::CGeneral::default();

        // re3 ProcessPlane only routes AIRTRAIN and DEADDODO. A plane entity
        // carrying any other model must not be treated as a live Dodo engine.
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut generic_plane = crate::vehicles::Plane::CPlane::new(
            crate::modelinfo::ModelIndices::MI_DODO as i32,
            0,
            0,
            &mut general,
        );
        generic_plane
            .vehicle.core.physical
            .entity
            .set_position(CVector::new(5.0, 0.0, 0.0));
        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut generic_plane.vehicle.core, 0);
        process_single_plane_audio(&mut dmaudio, &mut generic_plane, 100, CVector::ZERO, 0.0, &mut one_shots, &mut player_engine, None, &mut JumboAudioState::default());
        assert_eq!(
            dmaudio.AudioManager.m_nRequestedCount[dmaudio.AudioManager.m_nActiveQueue], 0,
            "generic plane models must not emit the Dodo/Cesna engine loop"
        );

        // DEADDODO uses ProcessCesna's normal spatial branch. In particular,
        // its distance must come from the plane/camera positions rather than
        // the player-only AddPlayerCarSample path that leaves distance at 0.
        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut dead_dodo = crate::vehicles::Plane::CPlane::new(
            crate::modelinfo::ModelIndices::MI_DEADDODO as i32,
            0,
            0,
            &mut general,
        );
        dead_dodo
            .vehicle.core.physical
            .entity
            .set_position(CVector::new(100.0, 0.0, 0.0));
        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut dead_dodo.vehicle.core, 0);
        process_single_plane_audio(&mut dmaudio, &mut dead_dodo, 101, CVector::ZERO, 0.0, &mut one_shots, &mut player_engine, None, &mut JumboAudioState::default());
        let queue = dmaudio.AudioManager.m_nActiveQueue;
        assert_eq!(dmaudio.AudioManager.m_nRequestedCount[queue], 1);
        let sample = dmaudio.AudioManager.m_aRequestedQueue[queue][0];
        assert_eq!(sample.m_nSampleIndex, SFX_CESNA_IDLE as u32);
        assert!((sample.m_fDistance - 100.0).abs() < 0.001);
        assert_eq!(sample.m_MaxDistance, CESNA_IDLE_MAX_DIST);

        let mut dmaudio = cDMAudio::default();
        dmaudio.Initialise();
        install_test_sample_metadata(&mut dmaudio.AudioManager);
        let mut far_dead_dodo = crate::vehicles::Plane::CPlane::new(
            crate::modelinfo::ModelIndices::MI_DEADDODO as i32,
            0,
            0,
            &mut general,
        );
        far_dead_dodo
            .vehicle.core.physical
            .entity
            .set_position(CVector::new(CESNA_IDLE_MAX_DIST + 1.0, 0.0, 0.0));
        crate::audio::AudioActorAccess::allocate_vehicle_audio_identity(&mut dmaudio, &mut far_dead_dodo.vehicle.core, 0);
        process_single_plane_audio(&mut dmaudio, &mut far_dead_dodo, 102, CVector::ZERO, 0.0, &mut one_shots, &mut player_engine, None, &mut JumboAudioState::default());
        assert_eq!(
            dmaudio.AudioManager.m_nRequestedCount[dmaudio.AudioManager.m_nActiveQueue], 0,
            "DEADDODO engine must be inaudible outside re3's Cesna radius"
        );
    }
}
