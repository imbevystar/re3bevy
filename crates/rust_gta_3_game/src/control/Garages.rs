//! re3 reference: references/re-GTA/src/control/Garages.cpp
//! re3 reference: references/re-GTA/src/control/Garages.h
//!
//! `CGarages::Init` state.

use bevy::{prelude::*, window::PrimaryWindow};
use rust_gta_3_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::{
    core::{
        FileLoader::CFileLoader,
        World::{CWorld, EEntityType, NUMSECTORS_X},
    },
    modelinfo::ModelIndices::*,
    renderer::Font::CFont,
    save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter},
    text::{Messages::CMessages, Text::CText},
};

pub const NUM_GARAGES: usize = 32;
pub const NUM_GARAGE_STORED_CARS: usize = 6;
pub const TOTAL_COLLECTCARS_GARAGES: usize = 3;
pub const TOTAL_COLLECTCARS_CARS: usize = 16;
pub const MESSAGE_LENGTH: usize = 8;
pub const AEHANDLE_NONE: i32 = -5;
pub const HALFPI: f32 = std::f32::consts::FRAC_PI_2;
pub const ROTATED_DOOR_OPEN_SPEED: f32 = 0.015;
pub const ROTATED_DOOR_CLOSE_SPEED: f32 = 0.02;
pub const DEFAULT_DOOR_OPEN_SPEED: f32 = 0.035;
pub const DEFAULT_DOOR_CLOSE_SPEED: f32 = 0.04;
pub const DISTANCE_TO_ACTIVATE_GARAGE: f32 = 8.0;
pub const DISTANCE_TO_ACTIVATE_KEEPCAR_GARAGE: f32 = 17.0;
pub const DISTANCE_TO_CLOSE_MISSION_GARAGE: f32 = 30.0;
pub const TIME_TO_PROCESS_KEEPCAR_GARAGE: u32 = 2_000;
pub const TIME_TO_RESPRAY: u32 = 2_000;
pub const RESPRAY_PRICE: i32 = 1_000;
pub const BOMB_PRICE: i32 = 1_000;
pub const TIME_TO_SETUP_BOMB: u32 = 2_000;
pub const REWARD_FOR_FIRST_POLICE_CAR: i32 = 5_000;
pub const REWARD_FOR_FIRST_BANK_VAN: i32 = 5_000;
pub const MAX_POLICE_CARS_TO_COLLECT: i32 = 10;
pub const MAX_BANK_VANS_TO_COLLECT: i32 = 10;
pub const IMPORT_REWARD: i32 = 1_000;
pub const IMPORT_ALLCARS_REWARD: i32 = 200_000;
pub const MAX_SPEED_TO_SHOW_COLLECTED_MESSAGE: f32 = 0.03;
pub const CRUSHER_VEHICLE_TEST_SPAN: usize = 8;
pub const CRUSHER_CRANE_SPEED: f32 = 0.005;
pub const TIME_TO_CRUSH_CAR: u32 = 3_000;
pub const CRUSHER_MIN_REWARD: f32 = 25.0;
pub const CRUSHER_MAX_REWARD: f32 = 125.0;
pub const CRUSHER_REWARD_COEFFICIENT: f32 = 1.0 / 500_000.0;
pub const HIDEOUT_DOOR_SPEED_COEFFICIENT: f32 = 1.7;
pub const TIME_BETWEEN_HIDEOUT_MESSAGES: u32 = 18_000;
pub const STORED_CAR_SAVE_SIZE: usize = 0x28;
pub const GARAGE_SAVE_SIZE: usize = 140;
pub const GARAGES_COMPATIBLE_SAVE_SIZE: usize = 6 * 4
    + TOTAL_COLLECTCARS_GARAGES * 4
    + 4
    + 3 * NUM_GARAGE_STORED_CARS * STORED_CAR_SAVE_SIZE
    + NUM_GARAGES * GARAGE_SAVE_SIZE;
pub const GARAGES_SAVE_SIZE: usize = 5484;

pub const CRUSHER_GARAGE_X1: f32 = 1135.5;
pub const CRUSHER_GARAGE_Y1: f32 = 57.0;
pub const CRUSHER_GARAGE_Z1: f32 = -1.0;
pub const CRUSHER_GARAGE_X2: f32 = 1149.5;
pub const CRUSHER_GARAGE_Y2: f32 = 63.7;
pub const CRUSHER_GARAGE_Z2: f32 = 3.5;

pub const CARS_TO_COLLECT_IN_CRAIGS_GARAGES: [[i32; TOTAL_COLLECTCARS_CARS];
    TOTAL_COLLECTCARS_GARAGES] = [
    [
        MI_SECURICA as i32,
        MI_MOONBEAM as i32,
        MI_COACH as i32,
        MI_FLATBED as i32,
        MI_LINERUN as i32,
        MI_TRASH as i32,
        MI_PATRIOT as i32,
        MI_MRWHOOP as i32,
        MI_BLISTA as i32,
        MI_MULE as i32,
        MI_YANKEE as i32,
        MI_BOBCAT as i32,
        MI_DODO as i32,
        MI_BUS as i32,
        MI_RUMPO as i32,
        MI_PONY as i32,
    ],
    [
        MI_SENTINEL as i32,
        MI_CHEETAH as i32,
        MI_BANSHEE as i32,
        MI_IDAHO as i32,
        MI_INFERNUS as i32,
        MI_TAXI as i32,
        MI_KURUMA as i32,
        MI_STRETCH as i32,
        MI_PEREN as i32,
        MI_STINGER as i32,
        MI_MANANA as i32,
        MI_LANDSTAL as i32,
        MI_STALLION as i32,
        MI_BFINJECT as i32,
        MI_CABBIE as i32,
        MI_ESPERANT as i32,
    ],
    [
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_LANDSTAL as i32,
        MI_CHEETAH as i32,
        MI_TAXI as i32,
        MI_ESPERANT as i32,
        MI_SENTINEL as i32,
        MI_IDAHO as i32,
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EGarageState {
    FullyClosed = 0,
    Opened = 1,
    Closing = 2,
    Opening = 3,
    OpenedContainsCar = 4,
    ClosedContainsCar = 5,
    AfterDropoff = 6,
}

impl EGarageState {
    fn from_re3(value: u8) -> Self {
        match value {
            1 => Self::Opened,
            2 => Self::Closing,
            3 => Self::Opening,
            4 => Self::OpenedContainsCar,
            5 => Self::ClosedContainsCar,
            6 => Self::AfterDropoff,
            _ => Self::FullyClosed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EGarageType {
    None = 0,
    Mission = 1,
    Bombshop1 = 2,
    Bombshop2 = 3,
    Bombshop3 = 4,
    Respray = 5,
    CollectorsItems = 6,
    CollectSpecificCars = 7,
    CollectCars1 = 8,
    CollectCars2 = 9,
    CollectCars3 = 10,
    ForCarToComeOutOf = 11,
    SixtySeconds = 12,
    Crusher = 13,
    MissionKeepCar = 14,
    ForScriptToOpen = 15,
    HideoutOne = 16,
    HideoutTwo = 17,
    HideoutThree = 18,
    ForScriptToOpenAndClose = 19,
    KeepsOpeningForSpecificCar = 20,
    MissionKeepCarRemainClosed = 21,
}

/// Live-world predicates read by the three mission-garage branches of
/// `CGarage::Update`. Geometry is calculated by the world adapter from the
/// original collision spheres before this value is constructed.
#[derive(Debug, Clone, Copy, Default)]
pub struct MissionGarageUpdateContext {
    pub time_step: f32,
    pub time_ms: u32,
    pub frame_counter: u32,
    pub player_position: CVector,
    pub player_vehicle_slot: Option<usize>,
    pub player_vehicle_position: Option<CVector>,
    pub target_exists: bool,
    pub target_entirely_inside: bool,
    pub target_entirely_outside: bool,
    pub player_entirely_outside_margin_2: bool,
    pub no_car_touching_garage: bool,
    pub no_other_car_touching_target: bool,
    pub static_player_target_entirely_inside: bool,
    pub no_car_blocking_door: bool,
}

/// Side effects which `CGarage::Update` performs outside the garage object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MissionGarageUpdateResult {
    /// `Some(false)` is `SetDisablePlayerControls(PLAYERCONTROL_GARAGE)`;
    /// `Some(true)` is the matching `SetEnablePlayerControls`.
    pub player_controls_enabled: Option<bool>,
    pub player_ignored_by_cops: Option<bool>,
    /// Normal mission garages destroy the target after the door reaches zero.
    pub destroy_vehicle_slot: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResprayGarageUpdateContext {
    pub time_step: f32,
    pub time_ms: u32,
    pub static_player_car_entirely_inside: bool,
    pub no_other_car_touching_player_car: bool,
    pub player_car_sprayable: bool,
    pub player_can_afford: bool,
    pub player_vehicle_near: bool,
    pub player_outside_garage: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResprayGarageUpdateResult {
    pub player_controls_enabled: Option<bool>,
    pub player_ignored_by_cops: Option<bool>,
    pub reset_player_car_fire_timer: bool,
    pub complete_respray: bool,
    pub checkpoint_reached: bool,
    pub call_off_chase: bool,
    /// `GA_1` (unsprayable) or `GA_3` (cannot afford).
    pub rejection_message: Option<[u8; 4]>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BombshopGarageUpdateContext {
    pub time_step: f32,
    pub time_ms: u32,
    pub static_player_car_entirely_inside: bool,
    pub no_other_car_touching_player_car: bool,
    pub player_car_already_has_bomb: bool,
    pub player_can_afford: bool,
    pub player_outside_garage: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BombshopGarageUpdateResult {
    pub player_controls_enabled: Option<bool>,
    pub player_ignored_by_cops: Option<bool>,
    pub install_bomb_type: Option<i32>,
    pub rejection_message: Option<[u8; 4]>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CollectGarageUpdateContext {
    pub time_step: f32,
    pub player_position: CVector,
    pub player_vehicle_slot: Option<usize>,
    pub player_vehicle_model: Option<i32>,
    pub player_vehicle_position: Option<CVector>,
    pub player_vehicle_speed: f32,
    pub player_vehicle_is_mission: bool,
    pub target_exists: bool,
    pub target_model: Option<i32>,
    pub target_entirely_inside: bool,
    pub no_other_car_touching_target: bool,
    pub player_entirely_outside_margin_2: bool,
    pub target_condition_acceptable: bool,
    pub craig_needs_player_car: bool,
    pub craig_has_collected_player_car: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollectGarageUpdateResult {
    pub player_controls_enabled: Option<bool>,
    pub player_ignored_by_cops: Option<bool>,
    pub destroy_vehicle_slot: Option<usize>,
    pub delivered_model: Option<i32>,
    pub camera_directly_behind: bool,
    pub message: Option<([u8; MESSAGE_LENGTH], u32)>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CrusherGarageUpdateContext {
    pub time_step: f32,
    pub time_ms: u32,
    pub candidate_vehicle_slot: Option<usize>,
    pub target_exists: bool,
    pub player_on_foot_inside: bool,
    pub frame_counter: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CrusherGarageUpdateResult {
    pub damp_target_velocity_by: Option<f32>,
    pub immobilize_target: bool,
    pub crush_vehicle_slot: Option<usize>,
    pub shake: bool,
    pub damage_player: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HideoutGarageUpdateContext {
    pub time_step: f32,
    pub time_ms: u32,
    pub player_position: CVector,
    pub player_vehicle_position: Option<CVector>,
    pub player_outside_garage: bool,
    pub no_car_blocking_door: bool,
    pub cars_inside_excluding_player: usize,
    pub stored_car_count: usize,
    pub last_help_message_time: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HideoutGarageUpdateResult {
    pub store_and_remove_cars: bool,
    pub restore_cars: bool,
    pub remove_blocking_cars_not_inside: bool,
    pub show_full_message: bool,
}

pub(crate) fn message_key(value: &[u8]) -> [u8; MESSAGE_LENGTH] {
    let mut key = [0_u8; MESSAGE_LENGTH];
    let len = value.len().min(MESSAGE_LENGTH);
    key[..len].copy_from_slice(&value[..len]);
    key
}

pub(crate) fn draw_garage_messages(
    garages: Option<Res<CGarages>>,
    timer: Res<CTimer>,
    text: Option<Res<CText>>,
    mut font: Option<ResMut<CFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let (Some(garages), Some(text), Some(font)) = (garages.as_ref(), text.as_ref(), font.as_mut())
    else {
        return;
    };
    let now = timer.time_in_milliseconds();
    if now <= garages.message_start_time || now >= garages.message_end_time {
        return;
    }
    let (width, height) = windows
        .single()
        .map(|window| (window.width(), window.height()))
        .unwrap_or((
            crate::core::common::DEFAULT_SCREEN_WIDTH,
            crate::core::common::DEFAULT_SCREEN_HEIGHT,
        ));
    let scale_x = width / crate::core::common::DEFAULT_SCREEN_WIDTH;
    let scale_y = height / crate::core::common::DEFAULT_SCREEN_HEIGHT;
    let raw = text.get_key_bytes(&garages.message_id_string);
    let displayed = if garages.message_number_in_string2 >= 0 {
        CMessages::insert_number_in_string(
            Some(&raw),
            [
                garages.message_number_in_string,
                garages.message_number_in_string2,
                -1,
                -1,
                -1,
                -1,
            ],
        )
    } else if garages.message_number_in_string >= 0 {
        CMessages::insert_number_in_string(
            Some(&raw),
            [garages.message_number_in_string, -1, -1, -1, -1, -1],
        )
    } else {
        raw
    };
    font.details.scale_x = 1.2 * scale_x;
    font.details.scale_y = 1.5 * scale_y;
    font.details.proportional = true;
    font.details.justify = false;
    font.details.background = false;
    font.details.centre_size = (crate::core::common::DEFAULT_SCREEN_WIDTH - 50.0) * scale_x;
    font.details.centre = true;
    font.details.style = crate::renderer::Font::FONT_BANK;
    font.details.color = crate::objects::ParticleObject::CRGBA::new(0, 0, 0, 255);
    let center_x = width * 0.5;
    let y = height / 3.0
        - if garages.message_number_in_string >= 0 {
            40.0 * scale_y
        } else {
            0.0
        };
    let numbered = garages.message_number_in_string >= 0;
    let shadow_x = center_x + if numbered { 2.0 } else { -2.0 } * scale_x;
    let shadow_y = y + if numbered { 2.0 } else { -2.0 } * scale_y;
    font.print_string(shadow_x, shadow_y, &displayed);
    font.details.color = crate::objects::ParticleObject::CRGBA::new(89, 115, 150, 255);
    font.print_string(center_x, y, &displayed);
}

const GARAGE_DOOR_MODEL_NAMES: &[&str] = &[
    "oddjgaragdoor",
    "bombdoor",
    "door_bombshop",
    "vheistlocdoor",
    "door2_garage",
    "ind_slidedoor",
    "bankjobdoor",
    "door_jmsgrage",
    "jamesgrge_kb",
    "door_sfehousegrge",
    "shedgaragedoor",
    "door4_garage",
    "door_col_compnd_01",
    "door_col_compnd_02",
    "door_col_compnd_03",
    "door_col_compnd_04",
    "door_col_compnd_05",
    "impex_door",
    "SalvGarage",
    "door3_garage",
    "leveldoor2",
    "double_garage_dr",
    "amcogaragedoor",
    "towergaragedoor1",
    "towergaragedoor2",
    "towergaragedoor3",
    "plysve_gragedoor",
    "impexpsubgrgdoor",
    "Sub_sprayshopdoor",
    "ind_plyrwoor",
    "8ballsuburbandoor",
    "crushercrush",
    "crushertop",
];

fn garage_door_model_name(file_loader: &CFileLoader, model_index: usize) -> Option<&str> {
    Some(
        &file_loader
            .model_info
            .get_model_info(model_index)?
            .base()
            .name,
    )
}

fn is_garage_door_world_entity(file_loader: &CFileLoader, entity_index: usize) -> bool {
    let Some(entity) = file_loader.world.entities.get(entity_index) else {
        return false;
    };
    if !matches!(entity.entity_type, EEntityType::Object | EEntityType::Dummy) {
        return false;
    }
    garage_door_model_name(file_loader, entity.model_index).is_some_and(|name| {
        GARAGE_DOOR_MODEL_NAMES
            .iter()
            .any(|door| name.eq_ignore_ascii_case(door))
    })
}

pub(crate) fn find_garage_door_entities(
    garage: &CGarage,
    file_loader: &CFileLoader,
) -> (Option<usize>, Option<usize>) {
    let xstart = CWorld::get_sector_index_x(garage.x1);
    let xend = CWorld::get_sector_index_x(garage.x2);
    let ystart = CWorld::get_sector_index_y(garage.y1);
    let yend = CWorld::get_sector_index_y(garage.y2);
    let mut scanned = Vec::<usize>::new();

    for y in ystart..=yend {
        for x in xstart..=xend {
            let Some(sector) = file_loader.world.sectors.get(y * NUMSECTORS_X + x) else {
                continue;
            };
            for list in [
                &sector.objects,
                &sector.objects_overlap,
                &sector.dummies,
                &sector.dummies_overlap,
            ] {
                for &entity_index in list {
                    if !scanned.contains(&entity_index) {
                        scanned.push(entity_index);
                    }
                }
            }
        }
    }

    let center_x = (garage.x1 + garage.x2) * 0.5;
    let center_y = (garage.y1 + garage.y2) * 0.5;
    let mut door1 = None;
    let mut door2 = None;
    for entity_index in scanned {
        if !is_garage_door_world_entity(file_loader, entity_index) {
            continue;
        }
        let entity = &file_loader.world.entities[entity_index];
        if (entity.position[0] - center_x).abs() >= 20.0
            || (entity.position[1] - center_y).abs() >= 20.0
        {
            continue;
        }
        let model_name =
            garage_door_model_name(file_loader, entity.model_index).unwrap_or_default();
        if model_name.eq_ignore_ascii_case("crushercrush") {
            door1 = Some(entity_index);
            continue;
        }
        if model_name.eq_ignore_ascii_case("crushertop") {
            door2 = Some(entity_index);
            continue;
        }
        if door1.is_none() {
            door1 = Some(entity_index);
        } else {
            door2 = Some(entity_index);
        }
    }

    let (Some(first), Some(second)) = (door1, door2) else {
        return (door1, door2);
    };
    let first_name =
        garage_door_model_name(file_loader, file_loader.world.entities[first].model_index)
            .unwrap_or_default();
    if first_name.eq_ignore_ascii_case("crushercrush")
        || first_name.eq_ignore_ascii_case("crushertop")
    {
        return (door1, door2);
    }

    let first_pos = file_loader.world.entities[first].position;
    let second_pos = file_loader.world.entities[second].position;
    let first_vec = [first_pos[0] - center_x, first_pos[1] - center_y];
    let second_vec = [second_pos[0] - center_x, second_pos[1] - center_y];
    if first_vec[0] * second_vec[0] + first_vec[1] * second_vec[1] > 0.0 {
        if first_vec[0] * first_vec[0] + first_vec[1] * first_vec[1]
            >= second_vec[0] * second_vec[0] + second_vec[1] * second_vec[1]
        {
            door1 = door2;
        }
        door2 = None;
    }
    (door1, door2)
}

pub(crate) fn refresh_garage_door_entities(
    garages: &mut CGarages,
    file_loader: &CFileLoader,
    count: usize,
) {
    for garage in garages.garages.iter_mut().take(count) {
        let stale_door1 = garage
            .door1_world_index
            .is_some_and(|index| !is_garage_door_world_entity(file_loader, index));
        let stale_door2 = garage
            .door2_world_index
            .is_some_and(|index| !is_garage_door_world_entity(file_loader, index));
        if garage.door_world_lookup_complete
            && !garage.recreate_door_on_next_refresh
            && !stale_door1
            && !stale_door2
        {
            continue;
        }

        garage.recreate_door_on_next_refresh = false;
        let (door1, door2) = find_garage_door_entities(garage, file_loader);
        garage.door1_world_index = door1;
        garage.door2_world_index = door2;
        garage.door1_pool_index = door1.map_or(0, |index| (index & 0x7f) as u8);
        garage.door2_pool_index = door2.map_or(0, |index| (index & 0x7f) as u8);
        garage.door1_is_dummy = door1.is_some_and(|index| {
            file_loader.world.entities[index].entity_type == EEntityType::Dummy
        });
        garage.door2_is_dummy = door2.is_some_and(|index| {
            file_loader.world.entities[index].entity_type == EEntityType::Dummy
        });
        garage.door_world_lookup_complete = true;

        if garage.door_geometry_initialized {
            continue;
        }
        if let Some(index) = door1 {
            let entity = &file_loader.world.entities[index];
            garage.door1_x = entity.position[0];
            garage.door1_y = entity.position[1];
            garage.door1_z = entity.position[2];
            let raw_height = file_loader
                .model_info
                .col_model(entity.model_index)
                .map_or(4.0, |col| {
                    col.bounding_box.max.z - col.bounding_box.min.z - 0.1
                });
            garage.door_height = if garage.rotated_door {
                raw_height / 2.0 - 0.1
            } else {
                raw_height
            };
        } else {
            garage.door_height = if garage.rotated_door { 1.9 } else { 4.0 };
        }
        if let Some(index) = door2 {
            let entity = &file_loader.world.entities[index];
            garage.door2_x = entity.position[0];
            garage.door2_y = entity.position[1];
            garage.door2_z = entity.position[2];
        }
        garage.door_pos = match garage.garage_type {
            EGarageType::Bombshop1
            | EGarageType::Bombshop2
            | EGarageType::Bombshop3
            | EGarageType::Respray => garage.door_height,
            EGarageType::Crusher => std::f32::consts::FRAC_PI_2,
            _ => 0.0,
        };
        garage.door_geometry_initialized = true;
    }
}

fn gta_entity_forward(entity: &crate::core::World::CWorldEntity) -> Vec3 {
    if let Some([_, forward, _]) = entity.matrix_basis_override {
        return Vec3::from_array(forward);
    }
    let axis = Vec3::from_array(entity.rotation_axis);
    if axis.length_squared() <= f32::EPSILON {
        Vec3::Y
    } else {
        Quat::from_axis_angle(axis.normalize(), entity.rotation_degrees.to_radians()) * Vec3::Y
    }
}

pub(crate) fn apply_rotated_garage_door_matrix(
    entity: &mut crate::core::World::CWorldEntity,
    fraction: f32,
) {
    let forward = gta_entity_forward(entity);
    let angle = -fraction * std::f32::consts::FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    let up = Vec3::new(-sin * forward.y, sin * forward.x, cos);
    let right = up.cross(forward);
    entity.matrix_basis_override = Some([right.to_array(), forward.to_array(), up.to_array()]);
}

pub(crate) fn apply_crusher_render_shake(
    garage: &CGarage,
    file_loader: &mut CFileLoader,
    x: f32,
    y: f32,
) {
    for index in [garage.door1_world_index, garage.door2_world_index]
        .into_iter()
        .flatten()
    {
        if let Some(entity) = file_loader.world.entities.get_mut(index) {
            entity.render_position_offset = [x, y, 0.0];
        }
    }
}

pub(crate) fn apply_garage_door_transforms(
    garages: &CGarages,
    file_loader: &mut CFileLoader,
    count: usize,
    actors: &mut ScriptEntityWorld<'_, '_>,
) {
    for garage in garages.garages.iter().take(count) {
        // Population replaces a CDummyObject with a live ECS CObject near
        // the player. RefreshDoorPointers in re3 follows that replacement;
        // the hidden IPL provenance must not receive the live door matrix.
        for (index, base_z) in [
            (garage.door1_world_index, garage.door1_z),
            (garage.door2_world_index, garage.door2_z),
        ] {
            let Some(index) = index else { continue };
            let Some(&handle) = file_loader.world.ecs_lifecycle_objects.get(&index) else {
                continue;
            };
            let Some(object) = actors.object_get_mut(handle) else {
                bevy::log::warn_once!(index, handle, "garage_door_live_object_missing");
                continue;
            };
            let matrix = &mut object.object.physical.entity.matrix;
            if garage.garage_type == EGarageType::Crusher {
                if garage.door2_world_index == Some(index) {
                    let (sin, cos) = (std::f32::consts::TAU - garage.door_pos).sin_cos();
                    matrix.right = CVector::new(1.0, 0.0, 0.0);
                    matrix.forward = CVector::new(0.0, cos, sin);
                    matrix.up = CVector::new(0.0, -sin, cos);
                }
            } else {
                matrix.position.z = base_z + garage.door_pos;
                if garage.rotated_door && garage.door_height.abs() > f32::EPSILON {
                    let angle = -garage.door_pos / garage.door_height * HALFPI;
                    let (sin, cos) = angle.sin_cos();
                    matrix.up = CVector::new(-sin * matrix.forward.y, sin * matrix.forward.x, cos);
                    matrix.right = matrix.up.cross(matrix.forward);
                }
            }
        }
        if garage.garage_type == EGarageType::Crusher {
            if let Some(index) = garage.door2_world_index
                && !file_loader.world.ecs_lifecycle_objects.contains_key(&index)
                && let Some(entity) = file_loader.world.entities.get_mut(index)
            {
                let angle = std::f32::consts::TAU - garage.door_pos;
                let (sin, cos) = angle.sin_cos();
                entity.matrix_basis_override =
                    Some([[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]]);
            }
            continue;
        }
        for (index, base_z) in [
            (garage.door1_world_index, garage.door1_z),
            (garage.door2_world_index, garage.door2_z),
        ] {
            let Some(index) = index else {
                continue;
            };
            if file_loader.world.ecs_lifecycle_objects.contains_key(&index) {
                continue;
            }
            let Some(entity) = file_loader.world.entities.get_mut(index) else {
                continue;
            };
            entity.position[2] = base_z + garage.door_pos;
            if garage.rotated_door {
                let fraction = if garage.door_height.abs() > f32::EPSILON {
                    garage.door_pos / garage.door_height
                } else {
                    0.0
                };
                apply_rotated_garage_door_matrix(entity, fraction);
            } else {
                entity.matrix_basis_override = None;
            }
        }
    }
}

pub(crate) fn garage_col_model<'a>(
    entity: &crate::entities::Entity::CEntity,
    file_loader: &'a CFileLoader,
) -> Option<&'a crate::collision::ColModel::CColModel> {
    file_loader
        .model_info
        .col_model(entity.model_index.max(0) as usize)
}

pub(crate) fn garage_bound_sphere_intersects(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    file_loader: &CFileLoader,
) -> bool {
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return false;
    };
    let position = entity.get_position();
    let radius = col_model.bounding_sphere.radius;
    radius + position.x >= garage.x1
        && position.x - radius <= garage.x2
        && radius + position.y >= garage.y1
        && position.y - radius <= garage.y2
        && radius + position.z >= garage.z1
        && position.z - radius <= garage.z2
}

pub(crate) fn garage_contains_vehicle<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    garage: &CGarage,
    vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>,
    file_loader: &CFileLoader,
) -> bool
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
    let entity = &vehicle.physical.entity;
    let position = entity.get_position();
    if position.x < garage.x1
        || position.x > garage.x2
        || position.y < garage.y1
        || position.y > garage.y2
        || position.z < garage.z1
        || position.z > garage.z2
    {
        return false;
    }
    let Some(col_model) = file_loader
        .model_info
        .col_model(entity.model_index.max(0) as usize)
    else {
        return false;
    };
    for sphere in &col_model.spheres {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        let radius = sphere.radius;
        if center.x + radius < garage.x1
            || center.x - radius > garage.x2
            || center.y + radius < garage.y1
            || center.y - radius > garage.y2
            || center.z + radius < garage.z1
            || center.z - radius > garage.z2
        {
            return false;
        }
    }
    true
}

pub(crate) fn garage_distance_to_rectangle_squared(garage: &CGarage, position: CVector) -> f32 {
    let dx = if position.x < garage.x1 {
        garage.x1 - position.x
    } else if position.x > garage.x2 {
        position.x - garage.x2
    } else {
        0.0
    };
    let dy = if position.y < garage.y1 {
        garage.y1 - position.y
    } else if position.y > garage.y2 {
        position.y - garage.y2
    } else {
        0.0
    };
    dx * dx + dy * dy
}

pub(crate) fn is_car_sprayable(model_id: i32) -> bool {
    !matches!(
        usize::try_from(model_id).ok(),
        Some(
            MI_FIRETRUCK
                | MI_AMBULAN
                | MI_POLICE
                | MI_ENFORCER
                | MI_BUS
                | MI_RHINO
                | MI_BARRACKS
                | MI_DODO
                | MI_COACH
        )
    )
}

pub(crate) fn garage_entity_entirely_inside_3d(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    margin: f32,
    file_loader: &CFileLoader,
) -> bool {
    let position = entity.get_position();
    if position.x < garage.x1 - margin
        || position.x > garage.x2 + margin
        || position.y < garage.y1 - margin
        || position.y > garage.y2 + margin
        || position.z < garage.z1 - margin
        || position.z > garage.z2 + margin
    {
        return false;
    }
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return false;
    };
    col_model.spheres.iter().all(|sphere| {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        let radius = sphere.radius;
        center.x + radius >= garage.x1 - margin
            && center.x - radius <= garage.x2 + margin
            && center.y + radius >= garage.y1 - margin
            && center.y - radius <= garage.y2 + margin
            && center.z + radius >= garage.z1 - margin
            && center.z - radius <= garage.z2 + margin
    })
}

pub(crate) fn garage_entity_entirely_inside_2d_strict(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    file_loader: &CFileLoader,
) -> bool {
    let position = entity.get_position();
    if position.x < garage.x1
        || position.x > garage.x2
        || position.y < garage.y1
        || position.y > garage.y2
    {
        return false;
    }
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return false;
    };
    col_model.spheres.iter().all(|sphere| {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        center.x - sphere.radius >= garage.x1
            && center.x + sphere.radius <= garage.x2
            && center.y - sphere.radius >= garage.y1
            && center.y + sphere.radius <= garage.y2
    })
}

pub(crate) fn garage_entity_entirely_outside_2d(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    margin: f32,
    file_loader: &CFileLoader,
) -> bool {
    let position = entity.get_position();
    if position.x > garage.x1 - margin
        && position.x < garage.x2 + margin
        && position.y > garage.y1 - margin
        && position.y < garage.y2 + margin
    {
        return false;
    }
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return true;
    };
    col_model.spheres.iter().all(|sphere| {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        let radius = sphere.radius;
        center.x + radius <= garage.x1 - margin
            || center.x - radius >= garage.x2 + margin
            || center.y + radius <= garage.y1 - margin
            || center.y - radius >= garage.y2 + margin
    })
}

pub(crate) fn garage_entity_touching_3d(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    file_loader: &CFileLoader,
) -> bool {
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return false;
    };
    let position = entity.get_position();
    let bound_radius = col_model.bounding_sphere.radius;
    if garage.x1 - bound_radius > position.x
        || garage.x2 + bound_radius < position.x
        || garage.y1 - bound_radius > position.y
        || garage.y2 + bound_radius < position.y
        || garage.z1 - bound_radius > position.z
        || garage.z2 + bound_radius < position.z
    {
        return false;
    }
    col_model.spheres.iter().any(|sphere| {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        let radius = sphere.radius;
        center.x + radius > garage.x1
            && center.x - radius < garage.x2
            && center.y + radius > garage.y1
            && center.y - radius < garage.y2
            && center.z + radius > garage.z1
            && center.z - radius < garage.z2
    })
}

pub(crate) fn garage_entity_has_sphere_way_outside(
    garage: &CGarage,
    entity: &crate::entities::Entity::CEntity,
    margin: f32,
    file_loader: &CFileLoader,
) -> bool {
    let Some(col_model) = garage_col_model(entity, file_loader) else {
        return true;
    };
    col_model.spheres.iter().any(|sphere| {
        let center = entity.matrix.transform_point(CVector::new(
            sphere.center.x,
            sphere.center.y,
            sphere.center.z,
        ));
        let radius = sphere.radius;
        center.x + radius + margin < garage.x1
            || center.x - radius - margin > garage.x2
            || center.y + radius + margin < garage.y1
            || center.y - radius - margin > garage.y2
            || center.z + radius + margin < garage.z1
            || center.z - radius - margin > garage.z2
    })
}

impl EGarageType {
    pub fn from_re3(value: u8) -> Self {
        match value {
            1 => Self::Mission,
            2 => Self::Bombshop1,
            3 => Self::Bombshop2,
            4 => Self::Bombshop3,
            5 => Self::Respray,
            6 => Self::CollectorsItems,
            7 => Self::CollectSpecificCars,
            8 => Self::CollectCars1,
            9 => Self::CollectCars2,
            10 => Self::CollectCars3,
            11 => Self::ForCarToComeOutOf,
            12 => Self::SixtySeconds,
            13 => Self::Crusher,
            14 => Self::MissionKeepCar,
            15 => Self::ForScriptToOpen,
            16 => Self::HideoutOne,
            17 => Self::HideoutTwo,
            18 => Self::HideoutThree,
            19 => Self::ForScriptToOpenAndClose,
            20 => Self::KeepsOpeningForSpecificCar,
            21 => Self::MissionKeepCarRemainClosed,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CStoredCar {
    pub model_index: i32,
    pub position: CVector,
    pub angle: CVector,
    pub flags: i32,
    pub primary_color: i8,
    pub secondary_color: i8,
    pub radio_station: i8,
    pub variation_a: i8,
    pub variation_b: i8,
    pub car_bomb_type: i8,
}

impl CStoredCar {
    pub const FLAG_BULLETPROOF: i32 = 0x01;
    pub const FLAG_FIREPROOF: i32 = 0x02;
    pub const FLAG_EXPLOSIONPROOF: i32 = 0x04;
    pub const FLAG_COLLISIONPROOF: i32 = 0x08;
    pub const FLAG_MELEEPROOF: i32 = 0x10;

    pub const fn init() -> Self {
        Self {
            model_index: 0,
            position: CVector::ZERO,
            angle: CVector::ZERO,
            flags: 0,
            primary_color: 0,
            secondary_color: 0,
            radio_station: 0,
            variation_a: 0,
            variation_b: 0,
            car_bomb_type: 0,
        }
    }

    pub fn clear(&mut self) {
        self.model_index = 0;
    }

    pub const fn has_car(self) -> bool {
        self.model_index != 0
    }

    fn write_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.model_index);
        writer.write(self.position.x);
        writer.write(self.position.y);
        writer.write(self.position.z);
        writer.write(self.angle.x);
        writer.write(self.angle.y);
        writer.write(self.angle.z);
        writer.write(self.flags);
        writer.write(self.primary_color);
        writer.write(self.secondary_color);
        writer.write(self.radio_station);
        writer.write(self.variation_a);
        writer.write(self.variation_b);
        writer.write(self.car_bomb_type);
        writer.zero(2);
    }

    fn read_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let car = Self {
            model_index: reader.read::<i32>()?,
            position: CVector::new(
                reader.read::<f32>()?,
                reader.read::<f32>()?,
                reader.read::<f32>()?,
            ),
            angle: CVector::new(
                reader.read::<f32>()?,
                reader.read::<f32>()?,
                reader.read::<f32>()?,
            ),
            flags: reader.read::<i32>()?,
            primary_color: reader.read::<i8>()?,
            secondary_color: reader.read::<i8>()?,
            radio_station: reader.read::<i8>()?,
            variation_a: reader.read::<i8>()?,
            variation_b: reader.read::<i8>()?,
            car_bomb_type: reader.read::<i8>()?,
        };
        reader.skip(2)?;
        Ok(car)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CGarage {
    pub garage_type: EGarageType,
    pub garage_state: EGarageState,
    pub field_2: bool,
    pub closing_without_target_car: bool,
    pub deactivated: bool,
    pub respray_happened: bool,
    pub target_model_index: i32,
    pub door1_pool_index: u8,
    pub door2_pool_index: u8,
    pub door1_world_index: Option<usize>,
    pub door2_world_index: Option<usize>,
    /// Runtime counterpart of the initial `RefreshDoorPointers(true)` call.
    /// `None` is also a valid completed lookup result, so the indices alone
    /// cannot distinguish "not searched yet" from "garage has no door".
    pub door_world_lookup_complete: bool,
    /// The base coordinates and collision-derived height are captured only
    /// by `CGarages::AddOne`. Pointer refreshes (including save restore) must
    /// retain the already stored geometry.
    pub door_geometry_initialized: bool,
    pub door1_is_dummy: bool,
    pub door2_is_dummy: bool,
    pub recreate_door_on_next_refresh: bool,
    pub rotated_door: bool,
    pub camera_follows_player: bool,
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
    pub door_pos: f32,
    pub door_height: f32,
    pub door1_x: f32,
    pub door1_y: f32,
    pub door2_x: f32,
    pub door2_y: f32,
    pub door1_z: f32,
    pub door2_z: f32,
    pub time_to_start_action: u32,
    pub collected_cars_state: u8,
    /// ECS counterpart of re3's transient `CVehicle *m_pTarget`.
    ///
    /// re3 clears this pointer after loading instead of serialising it.  The
    /// pool slot is therefore runtime-only here as well.
    pub target_vehicle_slot: Option<usize>,
    pub stored_car: CStoredCar,
}

impl CGarage {
    pub const fn unused() -> Self {
        Self {
            garage_type: EGarageType::None,
            garage_state: EGarageState::FullyClosed,
            field_2: false,
            closing_without_target_car: false,
            deactivated: false,
            respray_happened: false,
            target_model_index: 0,
            door1_pool_index: 0,
            door2_pool_index: 0,
            door1_world_index: None,
            door2_world_index: None,
            door_world_lookup_complete: false,
            door_geometry_initialized: false,
            door1_is_dummy: false,
            door2_is_dummy: false,
            recreate_door_on_next_refresh: false,
            rotated_door: false,
            camera_follows_player: false,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            z1: 0.0,
            z2: 0.0,
            door_pos: 0.0,
            door_height: 0.0,
            door1_x: 0.0,
            door1_y: 0.0,
            door2_x: 0.0,
            door2_y: 0.0,
            door1_z: 0.0,
            door2_z: 0.0,
            time_to_start_action: 0,
            collected_cars_state: 0,
            target_vehicle_slot: None,
            stored_car: CStoredCar::init(),
        }
    }

    pub fn add_one(p1: CVector, p2: CVector, garage_type: EGarageType, target_id: i32) -> Self {
        let door_height = 4.0;
        let mut garage = Self {
            garage_type,
            garage_state: EGarageState::FullyClosed,
            field_2: false,
            closing_without_target_car: false,
            deactivated: false,
            respray_happened: false,
            target_model_index: target_id,
            recreate_door_on_next_refresh: false,
            rotated_door: false,
            camera_follows_player: false,
            x1: p1.x.min(p2.x),
            x2: p1.x.max(p2.x),
            y1: p1.y.min(p2.y),
            y2: p1.y.max(p2.y),
            z1: p1.z.min(p2.z),
            z2: p1.z.max(p2.z),
            door1_z: p1.z,
            door2_z: p1.z,
            door_height,
            time_to_start_action: 0,
            collected_cars_state: 0,
            target_vehicle_slot: None,
            stored_car: CStoredCar::init(),
            ..Self::unused()
        };

        match garage_type {
            EGarageType::Bombshop1
            | EGarageType::Bombshop2
            | EGarageType::Bombshop3
            | EGarageType::Respray => {
                garage.garage_state = EGarageState::Opened;
                garage.door_pos = door_height;
            }
            EGarageType::Crusher => {
                garage.garage_state = EGarageState::Opened;
                garage.door_pos = HALFPI;
            }
            _ => {
                garage.garage_state = EGarageState::FullyClosed;
                garage.door_pos = 0.0;
            }
        }

        garage
    }

    pub fn player_arrested_or_died(&mut self) {
        match self.garage_type {
            EGarageType::Mission
            | EGarageType::CollectorsItems
            | EGarageType::CollectSpecificCars
            | EGarageType::CollectCars1
            | EGarageType::CollectCars2
            | EGarageType::CollectCars3
            | EGarageType::ForCarToComeOutOf
            | EGarageType::SixtySeconds
            | EGarageType::MissionKeepCar
            | EGarageType::ForScriptToOpen
            | EGarageType::HideoutOne
            | EGarageType::HideoutTwo
            | EGarageType::HideoutThree
            | EGarageType::ForScriptToOpenAndClose
            | EGarageType::KeepsOpeningForSpecificCar
            | EGarageType::MissionKeepCarRemainClosed => {
                if matches!(
                    self.garage_state,
                    EGarageState::Opened | EGarageState::Closing | EGarageState::Opening
                ) {
                    self.garage_state = EGarageState::Closing;
                }
            }
            EGarageType::Bombshop1
            | EGarageType::Bombshop2
            | EGarageType::Bombshop3
            | EGarageType::Respray
            | EGarageType::Crusher => {
                if matches!(
                    self.garage_state,
                    EGarageState::FullyClosed | EGarageState::Closing | EGarageState::Opening
                ) {
                    self.garage_state = EGarageState::Opening;
                }
            }
            EGarageType::None => {}
        }
    }

    /// re3 `CGarage::OpenThisGarage`.
    pub fn open_this_garage(&mut self) {
        if matches!(
            self.garage_state,
            EGarageState::FullyClosed | EGarageState::Closing | EGarageState::ClosedContainsCar
        ) {
            self.garage_state = EGarageState::Opening;
        }
    }

    /// re3 `CGarage::CloseThisGarage`.
    pub fn close_this_garage(&mut self) {
        if matches!(
            self.garage_state,
            EGarageState::Opened | EGarageState::Opening
        ) {
            self.garage_state = EGarageState::Closing;
        }
    }

    fn update_door_opening(&mut self, time_step: f32) {
        let speed = if self.rotated_door {
            ROTATED_DOOR_OPEN_SPEED
        } else {
            DEFAULT_DOOR_OPEN_SPEED
        };
        self.door_pos = (self.door_pos + speed * time_step).min(self.door_height);
        if self.door_pos == self.door_height {
            self.garage_state = EGarageState::Opened;
        }
    }

    fn update_door_closing(&mut self, time_step: f32) {
        let speed = if self.rotated_door {
            ROTATED_DOOR_CLOSE_SPEED
        } else {
            DEFAULT_DOOR_CLOSE_SPEED
        };
        self.door_pos = (self.door_pos - speed * time_step).max(0.0);
        if self.door_pos == 0.0 {
            self.garage_state = EGarageState::FullyClosed;
        }
    }

    /// The two source branches whose state is controlled exclusively by SCM.
    fn update_script_controlled_door(&mut self, time_step: f32) {
        match (self.garage_type, self.garage_state) {
            (EGarageType::ForScriptToOpen, EGarageState::Opening)
            | (EGarageType::ForScriptToOpenAndClose, EGarageState::Opening) => {
                self.update_door_opening(time_step);
            }
            (EGarageType::ForScriptToOpenAndClose, EGarageState::Closing) => {
                self.update_door_closing(time_step);
            }
            _ => {}
        }
    }

    fn distance_to_rectangle_squared(&self, position: CVector) -> f32 {
        let dx = if position.x < self.x1 {
            self.x1 - position.x
        } else if position.x > self.x2 {
            position.x - self.x2
        } else {
            0.0
        };
        let dy = if position.y < self.y1 {
            self.y1 - position.y
        } else if position.y > self.y2 {
            position.y - self.y2
        } else {
            0.0
        };
        dx * dx + dy * dy
    }

    fn player_is_far_from_center(&self, position: CVector) -> bool {
        let dx = position.x - (self.x1 + self.x2) * 0.5;
        let dy = position.y - (self.y1 + self.y2) * 0.5;
        dx * dx + dy * dy > DISTANCE_TO_CLOSE_MISSION_GARAGE.powi(2)
    }

    fn player_is_far_from_center_axis(&self, position: CVector, distance: f32) -> bool {
        (position.x - (self.x1 + self.x2) * 0.5).abs() > distance
            || (position.y - (self.y1 + self.y2) * 0.5).abs() > distance
    }

    fn smallest_distance_to_door_squared(&self, position: CVector) -> f32 {
        let distance1 = (self.door1_x - position.x).powi(2) + (self.door1_y - position.y).powi(2);
        let distance2 = (self.door2_x - position.x).powi(2) + (self.door2_y - position.y).powi(2);
        match (self.door1_world_index, self.door2_world_index) {
            (Some(_), Some(_)) => distance1.min(distance2),
            (Some(_), None) => distance1,
            (None, Some(_)) => distance2,
            (None, None) => 10_000_000.0,
        }
    }

    /// re3 `CGarage::Update`, `GARAGE_MISSION`,
    /// `GARAGE_MISSION_KEEPCAR`, and
    /// `GARAGE_MISSION_KEEPCAR_REMAINCLOSED`.
    pub fn update_mission_garage(
        &mut self,
        context: MissionGarageUpdateContext,
    ) -> MissionGarageUpdateResult {
        let mut result = MissionGarageUpdateResult::default();
        if self.deactivated && self.garage_state == EGarageState::FullyClosed {
            return result;
        }

        match self.garage_type {
            EGarageType::Mission => match self.garage_state {
                EGarageState::Opened => {
                    if self.player_is_far_from_center(context.player_position) {
                        if context.frame_counter & 0x1F == 0 && context.no_car_touching_garage {
                            self.garage_state = EGarageState::Closing;
                            self.closing_without_target_car = true;
                        }
                    } else if context.player_vehicle_slot.is_none()
                        && context.target_exists
                        && context.target_entirely_inside
                        && context.no_other_car_touching_target
                        && context.player_entirely_outside_margin_2
                    {
                        result.player_controls_enabled = Some(false);
                        result.player_ignored_by_cops = Some(true);
                        self.garage_state = EGarageState::Closing;
                        self.closing_without_target_car = false;
                    }
                }
                EGarageState::Closing => {
                    self.update_door_closing(context.time_step);
                    if self.door_pos == 0.0 {
                        if self.closing_without_target_car {
                            self.garage_state = EGarageState::FullyClosed;
                        } else {
                            if context.target_exists {
                                self.garage_state = EGarageState::ClosedContainsCar;
                                result.destroy_vehicle_slot = self.target_vehicle_slot;
                                self.target_vehicle_slot = None;
                            } else {
                                self.garage_state = EGarageState::FullyClosed;
                            }
                            result.player_controls_enabled = Some(true);
                            result.player_ignored_by_cops = Some(false);
                        }
                    }
                }
                EGarageState::FullyClosed => {
                    if self.target_vehicle_slot.is_some()
                        && self.target_vehicle_slot == context.player_vehicle_slot
                        && context.player_vehicle_position.is_some_and(|position| {
                            self.distance_to_rectangle_squared(position)
                                < DISTANCE_TO_ACTIVATE_GARAGE.powi(2)
                        })
                    {
                        self.garage_state = EGarageState::Opening;
                    }
                }
                EGarageState::Opening => self.update_door_opening(context.time_step),
                _ => {}
            },
            EGarageType::MissionKeepCar | EGarageType::MissionKeepCarRemainClosed => {
                match self.garage_state {
                    EGarageState::Opened => {
                        if self.player_is_far_from_center(context.player_position)
                            && context.no_car_touching_garage
                        {
                            self.garage_state = EGarageState::Closing;
                            self.closing_without_target_car = true;
                        } else if self.target_vehicle_slot.is_some()
                            && self.target_vehicle_slot == context.player_vehicle_slot
                            && context.static_player_target_entirely_inside
                            && context.no_car_blocking_door
                        {
                            result.player_controls_enabled = Some(false);
                            result.player_ignored_by_cops = Some(true);
                            self.garage_state = EGarageState::Closing;
                            self.closing_without_target_car = false;
                        }
                    }
                    EGarageState::Closing => {
                        self.update_door_closing(context.time_step);
                        if self.door_pos == 0.0 {
                            if self.closing_without_target_car {
                                self.garage_state = EGarageState::FullyClosed;
                            } else {
                                if context.target_exists {
                                    self.garage_state = EGarageState::ClosedContainsCar;
                                    self.time_to_start_action = context
                                        .time_ms
                                        .wrapping_add(TIME_TO_PROCESS_KEEPCAR_GARAGE);
                                    self.target_vehicle_slot = None;
                                } else {
                                    self.garage_state = EGarageState::FullyClosed;
                                }
                                result.player_controls_enabled = Some(true);
                                result.player_ignored_by_cops = Some(false);
                            }
                        }
                    }
                    EGarageState::FullyClosed => {
                        if self.target_vehicle_slot.is_some()
                            && self.target_vehicle_slot == context.player_vehicle_slot
                            && context.player_vehicle_position.is_some_and(|position| {
                                self.distance_to_rectangle_squared(position)
                                    < DISTANCE_TO_ACTIVATE_KEEPCAR_GARAGE.powi(2)
                            })
                        {
                            self.garage_state = EGarageState::Opening;
                        }
                    }
                    EGarageState::Opening => self.update_door_opening(context.time_step),
                    EGarageState::ClosedContainsCar => {
                        if self.garage_type == EGarageType::MissionKeepCar
                            && context.time_ms > self.time_to_start_action
                        {
                            self.garage_state = EGarageState::Opening;
                        }
                    }
                    _ => {}
                }
            }
            EGarageType::KeepsOpeningForSpecificCar => match self.garage_state {
                EGarageState::Opened => {
                    if self.player_is_far_from_center(context.player_position)
                        && context.target_exists
                        && context.target_entirely_outside
                        && context.no_car_touching_garage
                    {
                        self.garage_state = EGarageState::Closing;
                        self.closing_without_target_car = true;
                    }
                }
                EGarageState::Closing => self.update_door_closing(context.time_step),
                EGarageState::FullyClosed => {
                    if self.target_vehicle_slot.is_some()
                        && self.target_vehicle_slot == context.player_vehicle_slot
                        && context.player_vehicle_position.is_some_and(|position| {
                            self.distance_to_rectangle_squared(position)
                                < DISTANCE_TO_ACTIVATE_GARAGE.powi(2)
                        })
                    {
                        self.garage_state = EGarageState::Opening;
                    }
                }
                EGarageState::Opening => self.update_door_opening(context.time_step),
                _ => {}
            },
            _ => {}
        }
        result
    }

    /// re3 `GARAGE_COLLECTSPECIFICCARS` state machine.
    pub fn update_collect_specific_cars(
        &mut self,
        context: CollectGarageUpdateContext,
    ) -> CollectGarageUpdateResult {
        let mut result = CollectGarageUpdateResult::default();
        if self.garage_type != EGarageType::CollectSpecificCars
            || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        match self.garage_state {
            EGarageState::Opened => {
                if context.player_vehicle_model == Some(self.target_model_index) {
                    self.target_vehicle_slot = context.player_vehicle_slot;
                }
                if context.player_vehicle_slot.is_none() {
                    if context.target_exists
                        && context.target_entirely_inside
                        && context.no_other_car_touching_target
                        && context.player_entirely_outside_margin_2
                    {
                        result.player_controls_enabled = Some(false);
                        result.player_ignored_by_cops = Some(true);
                        self.garage_state = EGarageState::Closing;
                    } else if self.player_is_far_from_center_axis(context.player_position, 25.0) {
                        self.garage_state = EGarageState::Closing;
                        self.target_vehicle_slot = None;
                    }
                }
            }
            EGarageState::Closing => {
                self.update_door_closing(context.time_step);
                if self.door_pos == 0.0 {
                    self.garage_state = EGarageState::FullyClosed;
                    if context.target_exists {
                        result.destroy_vehicle_slot = self.target_vehicle_slot;
                        result.delivered_model = Some(self.target_model_index);
                        self.target_vehicle_slot = None;
                        result.player_controls_enabled = Some(true);
                        result.player_ignored_by_cops = Some(false);
                    }
                }
            }
            EGarageState::FullyClosed => {
                if context.player_vehicle_model == Some(self.target_model_index)
                    && context.player_vehicle_position.is_some_and(|position| {
                        self.distance_to_rectangle_squared(position)
                            < DISTANCE_TO_ACTIVATE_GARAGE.powi(2)
                    })
                {
                    self.garage_state = EGarageState::Opening;
                }
            }
            EGarageState::Opening => {
                if context.player_vehicle_model == Some(self.target_model_index) {
                    self.target_vehicle_slot = context.player_vehicle_slot;
                }
                self.update_door_opening(context.time_step);
            }
            _ => {}
        }
        result
    }

    /// re3 `GARAGE_COLLECTCARS_1..3` state machine.
    pub fn update_collect_cars(
        &mut self,
        context: CollectGarageUpdateContext,
    ) -> CollectGarageUpdateResult {
        let mut result = CollectGarageUpdateResult::default();
        if !matches!(
            self.garage_type,
            EGarageType::CollectCars1 | EGarageType::CollectCars2 | EGarageType::CollectCars3
        ) || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        match self.garage_state {
            EGarageState::Opened => {
                if context.craig_needs_player_car {
                    self.target_vehicle_slot = context.player_vehicle_slot;
                }
                if self.player_is_far_from_center_axis(context.player_position, 40.0) {
                    self.garage_state = EGarageState::Closing;
                    self.target_vehicle_slot = None;
                } else if context.target_exists
                    && context.player_vehicle_slot.is_none()
                    && context.target_entirely_inside
                    && context.no_other_car_touching_target
                    && context.player_entirely_outside_margin_2
                    && context.target_condition_acceptable
                {
                    result.player_controls_enabled = Some(false);
                    result.player_ignored_by_cops = Some(true);
                    result.camera_directly_behind = true;
                    self.garage_state = EGarageState::Closing;
                }
            }
            EGarageState::Closing => {
                self.update_door_closing(context.time_step);
                if self.door_pos == 0.0 {
                    self.garage_state = EGarageState::FullyClosed;
                    if context.target_exists {
                        result.destroy_vehicle_slot = self.target_vehicle_slot;
                        result.delivered_model = context.target_model;
                        self.target_vehicle_slot = None;
                        result.player_controls_enabled = Some(true);
                        result.player_ignored_by_cops = Some(false);
                    }
                }
            }
            EGarageState::FullyClosed => {
                if context.player_vehicle_position.is_some_and(|position| {
                    self.smallest_distance_to_door_squared(position)
                        < DISTANCE_TO_ACTIVATE_GARAGE.powi(2)
                }) {
                    if context.craig_needs_player_car {
                        if context.player_vehicle_is_mission {
                            result.message = Some((message_key(b"GA_1A"), 5_000));
                        } else {
                            self.garage_state = EGarageState::Opening;
                        }
                    } else if context.craig_has_collected_player_car {
                        result.message = Some((message_key(b"GA_20"), 5_000));
                    } else if context.player_vehicle_speed < MAX_SPEED_TO_SHOW_COLLECTED_MESSAGE {
                        result.message = Some((message_key(b"GA_19"), 5_000));
                    }
                }
                self.target_vehicle_slot = None;
            }
            EGarageState::Opening => {
                if context.craig_needs_player_car {
                    self.target_vehicle_slot = context.player_vehicle_slot;
                }
                self.update_door_opening(context.time_step);
            }
            _ => {}
        }
        result
    }

    /// re3 `GARAGE_CRUSHER` state machine. Pool slicing and world mutations
    /// are supplied/emitted by the ECS adapter.
    pub fn update_crusher(
        &mut self,
        context: CrusherGarageUpdateContext,
    ) -> CrusherGarageUpdateResult {
        let mut result = CrusherGarageUpdateResult::default();
        if self.garage_type != EGarageType::Crusher
            || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        match self.garage_state {
            EGarageState::Opened => {
                if let Some(slot) = context.candidate_vehicle_slot {
                    self.garage_state = EGarageState::Closing;
                    self.target_vehicle_slot = Some(slot);
                }
            }
            EGarageState::Closing => {
                if context.target_exists {
                    self.door_pos =
                        (self.door_pos - CRUSHER_CRANE_SPEED * context.time_step).max(0.0);
                    if self.door_pos < std::f32::consts::TAU / 5.0 {
                        result.immobilize_target = true;
                    } else {
                        result.damp_target_velocity_by = Some(0.8_f32.powf(context.time_step));
                    }
                    if self.door_pos == 0.0 {
                        result.crush_vehicle_slot = self.target_vehicle_slot;
                        self.target_vehicle_slot = None;
                        self.garage_state = EGarageState::AfterDropoff;
                        self.time_to_start_action = context.time_ms.wrapping_add(TIME_TO_CRUSH_CAR);
                    }
                } else {
                    self.garage_state = EGarageState::Opening;
                }
            }
            EGarageState::AfterDropoff => {
                if context.time_ms <= self.time_to_start_action {
                    result.shake = true;
                } else {
                    self.garage_state = EGarageState::Opening;
                }
            }
            EGarageState::Opening => {
                self.door_pos = (self.door_pos + context.time_step * CRUSHER_CRANE_SPEED)
                    .min(std::f32::consts::FRAC_PI_2);
                if self.door_pos == std::f32::consts::FRAC_PI_2 {
                    self.garage_state = EGarageState::Opened;
                }
            }
            _ => {}
        }
        result.damage_player =
            context.player_on_foot_inside && context.frame_counter & 0x1f == 0x17;
        result
    }

    pub fn max_stored_cars(&self) -> usize {
        match self.garage_type {
            EGarageType::HideoutOne => 1,
            EGarageType::HideoutTwo => 2,
            EGarageType::HideoutThree => 3,
            _ => 0,
        }
    }

    /// re3 `GARAGE_HIDEOUT_ONE..THREE` state machine.
    pub fn update_hideout(
        &mut self,
        context: HideoutGarageUpdateContext,
    ) -> HideoutGarageUpdateResult {
        let mut result = HideoutGarageUpdateResult::default();
        if !matches!(
            self.garage_type,
            EGarageType::HideoutOne | EGarageType::HideoutTwo | EGarageType::HideoutThree
        ) || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        let distance = self.distance_to_rectangle_squared(context.player_position);
        match self.garage_state {
            EGarageState::Opened => {
                if (distance > 15.0_f32.powi(2)
                    || context.player_vehicle_position.is_none() && distance > 2.2_f32.powi(2))
                    && context.no_car_blocking_door
                {
                    self.garage_state = EGarageState::Closing;
                } else if context.player_vehicle_position.is_some()
                    && context.cars_inside_excluding_player >= self.max_stored_cars()
                {
                    self.garage_state = EGarageState::Closing;
                } else if distance > 70.0_f32.powi(2) {
                    self.garage_state = EGarageState::Closing;
                    result.remove_blocking_cars_not_inside = true;
                }
            }
            EGarageState::Closing => {
                let speed = HIDEOUT_DOOR_SPEED_COEFFICIENT
                    * if self.rotated_door {
                        ROTATED_DOOR_CLOSE_SPEED
                    } else {
                        DEFAULT_DOOR_CLOSE_SPEED
                    };
                self.door_pos = (self.door_pos - speed * context.time_step).max(0.0);
                if !context.player_outside_garage {
                    self.garage_state = EGarageState::Opening;
                } else if self.door_pos == 0.0 {
                    self.garage_state = EGarageState::FullyClosed;
                    result.store_and_remove_cars = true;
                }
            }
            EGarageState::FullyClosed => {
                if distance < 1.7_f32.powi(2)
                    || distance < 10.0_f32.powi(2) && context.player_vehicle_position.is_some()
                {
                    if context.player_vehicle_position.is_some()
                        && context.stored_car_count >= self.max_stored_cars()
                    {
                        if self.door1_world_index.is_some()
                            && context.player_vehicle_position.is_some_and(|position| {
                                (position.x - self.door1_x).powi(2)
                                    + (position.y - self.door1_y).powi(2)
                                    < 5.0_f32.powi(2)
                            })
                            && context.time_ms.wrapping_sub(context.last_help_message_time)
                                > TIME_BETWEEN_HIDEOUT_MESSAGES
                        {
                            result.show_full_message = true;
                        }
                    } else {
                        result.restore_cars = true;
                    }
                }
            }
            EGarageState::Opening => {
                let speed = HIDEOUT_DOOR_SPEED_COEFFICIENT
                    * if self.rotated_door {
                        ROTATED_DOOR_OPEN_SPEED
                    } else {
                        DEFAULT_DOOR_OPEN_SPEED
                    };
                self.door_pos = (self.door_pos + speed * context.time_step).min(self.door_height);
                if self.door_pos == self.door_height {
                    self.garage_state = EGarageState::Opened;
                }
            }
            _ => {}
        }
        result
    }

    /// re3 `GARAGE_FORCARTOCOMEOUTOF` branch.
    pub fn update_for_car_to_come_out_of(&mut self, time_step: f32, garage_empty: bool) {
        if self.garage_type != EGarageType::ForCarToComeOutOf
            || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return;
        }
        match self.garage_state {
            EGarageState::Opened if garage_empty => {
                self.garage_state = EGarageState::Closing;
            }
            EGarageState::Closing => {
                self.update_door_closing(time_step);
                if !garage_empty {
                    self.garage_state = EGarageState::Opening;
                }
            }
            EGarageState::Opening => self.update_door_opening(time_step),
            _ => {}
        }
    }

    /// re3 `GARAGE_RESPRAY` state machine. Vehicle repair, colour choice,
    /// wanted reset and payment are emitted for the world adapter.
    pub fn update_respray(
        &mut self,
        context: ResprayGarageUpdateContext,
    ) -> ResprayGarageUpdateResult {
        let mut result = ResprayGarageUpdateResult::default();
        if self.garage_type != EGarageType::Respray
            || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        match self.garage_state {
            EGarageState::Opened => {
                if context.static_player_car_entirely_inside
                    && context.no_other_car_touching_player_car
                {
                    if !context.player_car_sprayable {
                        self.garage_state = EGarageState::OpenedContainsCar;
                        result.rejection_message = Some(*b"GA_1");
                    } else if !context.player_can_afford {
                        self.garage_state = EGarageState::OpenedContainsCar;
                        result.rejection_message = Some(*b"GA_3");
                    } else {
                        self.garage_state = EGarageState::Closing;
                        result.player_controls_enabled = Some(false);
                        result.player_ignored_by_cops = Some(true);
                    }
                }
                result.call_off_chase = context.player_vehicle_near;
            }
            EGarageState::Closing => {
                self.update_door_closing(context.time_step);
                if self.door_pos == 0.0 {
                    self.garage_state = EGarageState::FullyClosed;
                    self.time_to_start_action = context.time_ms.wrapping_add(TIME_TO_RESPRAY);
                    result.checkpoint_reached = true;
                }
                result.reset_player_car_fire_timer = true;
                result.call_off_chase = true;
            }
            EGarageState::FullyClosed => {
                if context.time_ms > self.time_to_start_action {
                    self.garage_state = EGarageState::Opening;
                    result.player_controls_enabled = Some(true);
                    result.player_ignored_by_cops = Some(false);
                    result.complete_respray = true;
                    self.respray_happened = true;
                }
                result.call_off_chase = true;
            }
            EGarageState::Opening => {
                let speed = if self.rotated_door {
                    ROTATED_DOOR_OPEN_SPEED
                } else {
                    DEFAULT_DOOR_OPEN_SPEED
                };
                self.door_pos = (self.door_pos + speed * context.time_step).min(self.door_height);
                if self.door_pos == self.door_height {
                    self.garage_state = EGarageState::OpenedContainsCar;
                }
            }
            EGarageState::OpenedContainsCar if context.player_outside_garage => {
                self.garage_state = EGarageState::Opened;
            }
            _ => {}
        }
        result
    }

    /// re3 `GARAGE_BOMBSHOP1..3` state machine.
    pub fn update_bombshop(
        &mut self,
        context: BombshopGarageUpdateContext,
    ) -> BombshopGarageUpdateResult {
        let mut result = BombshopGarageUpdateResult::default();
        if !matches!(
            self.garage_type,
            EGarageType::Bombshop1 | EGarageType::Bombshop2 | EGarageType::Bombshop3
        ) || self.deactivated && self.garage_state == EGarageState::FullyClosed
        {
            return result;
        }
        match self.garage_state {
            EGarageState::Opened => {
                if context.static_player_car_entirely_inside
                    && context.no_other_car_touching_player_car
                {
                    if context.player_car_already_has_bomb {
                        self.garage_state = EGarageState::OpenedContainsCar;
                        result.rejection_message = Some(*b"GA_5");
                    } else if !context.player_can_afford {
                        self.garage_state = EGarageState::OpenedContainsCar;
                        result.rejection_message = Some(*b"GA_4");
                    } else {
                        self.garage_state = EGarageState::Closing;
                        result.player_controls_enabled = Some(false);
                        result.player_ignored_by_cops = Some(true);
                    }
                }
            }
            EGarageState::Closing => {
                self.update_door_closing(context.time_step);
                if self.door_pos == 0.0 {
                    self.garage_state = EGarageState::FullyClosed;
                    self.time_to_start_action = context.time_ms.wrapping_add(TIME_TO_SETUP_BOMB);
                }
            }
            EGarageState::FullyClosed => {
                if context.time_ms > self.time_to_start_action {
                    self.garage_state = EGarageState::Opening;
                    result.install_bomb_type = Some(match self.garage_type {
                        EGarageType::Bombshop1 => 1,
                        EGarageType::Bombshop2 => 2,
                        _ => 3,
                    });
                    result.player_controls_enabled = Some(true);
                    result.player_ignored_by_cops = Some(false);
                }
            }
            EGarageState::Opening => {
                let speed = if self.rotated_door {
                    ROTATED_DOOR_OPEN_SPEED
                } else {
                    DEFAULT_DOOR_OPEN_SPEED
                };
                self.door_pos = (self.door_pos + speed * context.time_step).min(self.door_height);
                if self.door_pos == self.door_height {
                    self.garage_state = EGarageState::OpenedContainsCar;
                }
            }
            EGarageState::OpenedContainsCar if context.player_outside_garage => {
                self.garage_state = EGarageState::Opened;
            }
            _ => {}
        }
        result
    }

    fn write_compatible_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.garage_type as u8);
        writer.write(self.garage_state as u8);
        writer.write(bool_to_u8(self.field_2));
        writer.write(bool_to_u8(self.closing_without_target_car));
        writer.write(bool_to_u8(self.deactivated));
        writer.write(bool_to_u8(self.respray_happened));
        writer.zero(2);
        writer.write(self.target_model_index);
        writer.zero(4 + 4);
        writer.write(self.door1_pool_index);
        writer.write(self.door2_pool_index);
        writer.write(bool_to_u8(self.door1_is_dummy));
        writer.write(bool_to_u8(self.door2_is_dummy));
        writer.write(bool_to_u8(self.recreate_door_on_next_refresh));
        writer.write(bool_to_u8(self.rotated_door));
        writer.write(bool_to_u8(self.camera_follows_player));
        writer.zero(1);
        writer.write(self.x1);
        writer.write(self.x2);
        writer.write(self.y1);
        writer.write(self.y2);
        writer.write(self.z1);
        writer.write(self.z2);
        writer.write(self.door_pos);
        writer.write(self.door_height);
        writer.write(self.door1_x);
        writer.write(self.door1_y);
        writer.write(self.door2_x);
        writer.write(self.door2_y);
        writer.write(self.door1_z);
        writer.write(self.door2_z);
        writer.write(self.time_to_start_action);
        writer.write(self.collected_cars_state);
        writer.zero(3 + 4 + 4);
        writer.zero(STORED_CAR_SAVE_SIZE);
    }

    fn read_compatible_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let garage_type = EGarageType::from_re3(reader.read::<u8>()?);
        let garage_state = EGarageState::from_re3(reader.read::<u8>()?);
        let field_2 = reader.read::<u8>()? != 0;
        let closing_without_target_car = reader.read::<u8>()? != 0;
        let deactivated = reader.read::<u8>()? != 0;
        let respray_happened = reader.read::<u8>()? != 0;
        reader.skip(2)?;
        let target_model_index = reader.read::<i32>()?;
        reader.skip(4 + 4)?;
        let garage = Self {
            garage_type,
            garage_state,
            field_2,
            closing_without_target_car,
            deactivated,
            respray_happened,
            target_model_index,
            door1_pool_index: reader.read::<u8>()?,
            door2_pool_index: reader.read::<u8>()?,
            door1_world_index: None,
            door2_world_index: None,
            door_world_lookup_complete: false,
            door_geometry_initialized: true,
            door1_is_dummy: reader.read::<u8>()? != 0,
            door2_is_dummy: reader.read::<u8>()? != 0,
            recreate_door_on_next_refresh: true,
            rotated_door: reader.read::<u8>()? != 0,
            camera_follows_player: reader.read::<u8>()? != 0,
            x1: {
                reader.skip(1)?;
                reader.read::<f32>()?
            },
            x2: reader.read::<f32>()?,
            y1: reader.read::<f32>()?,
            y2: reader.read::<f32>()?,
            z1: reader.read::<f32>()?,
            z2: reader.read::<f32>()?,
            door_pos: reader.read::<f32>()?,
            door_height: reader.read::<f32>()?,
            door1_x: reader.read::<f32>()?,
            door1_y: reader.read::<f32>()?,
            door2_x: reader.read::<f32>()?,
            door2_y: reader.read::<f32>()?,
            door1_z: reader.read::<f32>()?,
            door2_z: reader.read::<f32>()?,
            time_to_start_action: reader.read::<u32>()?,
            collected_cars_state: reader.read::<u8>()?,
            // re3 Load explicitly assigns `m_pTarget = nil`.
            target_vehicle_slot: None,
            stored_car: CStoredCar::init(),
        };
        reader.skip(3 + 4 + 4)?;
        reader.skip(STORED_CAR_SAVE_SIZE)?;
        Ok(garage)
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CGarages {
    pub bank_vans_collected: i32,
    pub bombs_are_free: bool,
    pub resprays_are_free: bool,
    pub cars_collected: i32,
    pub car_types_collected: [i32; TOTAL_COLLECTCARS_GARAGES],
    pub crushed_car_id: i32,
    pub last_time_help_message: u32,
    pub message_number_in_string: i32,
    pub message_id_string: [u8; MESSAGE_LENGTH],
    pub message_number_in_string2: i32,
    pub message_start_time: u32,
    pub message_end_time: u32,
    pub num_garages: u32,
    pub player_in_garage: bool,
    pub police_cars_collected: i32,
    pub garages: [CGarage; NUM_GARAGES],
    pub cars_in_safe_house1: [CStoredCar; NUM_GARAGE_STORED_CARS],
    pub cars_in_safe_house2: [CStoredCar; NUM_GARAGE_STORED_CARS],
    pub cars_in_safe_house3: [CStoredCar; NUM_GARAGE_STORED_CARS],
    pub cam_should_be_outside: bool,
    pub h_garages: i32,
    pub audio_entity_created: bool,
    /// Function-static `GarageToBeTidied` from `CGarages::Update`.
    pub garage_to_be_tidied: usize,
}

impl CGarages {
    pub fn init() -> Self {
        let mut garages = [CGarage::unused(); NUM_GARAGES];
        garages[0] = CGarage::add_one(
            CVector::new(CRUSHER_GARAGE_X1, CRUSHER_GARAGE_Y1, CRUSHER_GARAGE_Z1),
            CVector::new(CRUSHER_GARAGE_X2, CRUSHER_GARAGE_Y2, CRUSHER_GARAGE_Z2),
            EGarageType::Crusher,
            0,
        );

        Self {
            bank_vans_collected: 0,
            bombs_are_free: false,
            resprays_are_free: false,
            cars_collected: 0,
            car_types_collected: [0; TOTAL_COLLECTCARS_GARAGES],
            crushed_car_id: -1,
            last_time_help_message: 0,
            message_number_in_string: 0,
            message_id_string: [0; MESSAGE_LENGTH],
            message_number_in_string2: 0,
            message_start_time: 0,
            message_end_time: 0,
            num_garages: 1,
            player_in_garage: false,
            police_cars_collected: 0,
            garages,
            cars_in_safe_house1: [CStoredCar::init(); NUM_GARAGE_STORED_CARS],
            cars_in_safe_house2: [CStoredCar::init(); NUM_GARAGE_STORED_CARS],
            cars_in_safe_house3: [CStoredCar::init(); NUM_GARAGE_STORED_CARS],
            cam_should_be_outside: false,
            h_garages: AEHANDLE_NONE,
            audio_entity_created: false,
            garage_to_be_tidied: 0,
        }
    }

    pub fn add_one(
        &mut self,
        p1: CVector,
        p2: CVector,
        garage_type: EGarageType,
        target_id: i32,
    ) -> i32 {
        let index = self.num_garages as usize;
        if index >= NUM_GARAGES {
            self.num_garages = self.num_garages.saturating_add(1);
            return index as i32;
        }
        self.garages[index] = CGarage::add_one(p1, p2, garage_type, target_id);
        self.num_garages += 1;
        index as i32
    }

    pub fn set_leave_camera_for_this_garage(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.camera_follows_player = true;
        true
    }

    pub fn change_garage_type(&mut self, garage: i32, garage_type: i32, target_model: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.garage_type = EGarageType::from_re3(garage_type as u8);
        garage.target_model_index = target_model;
        garage.garage_state = EGarageState::FullyClosed;
        true
    }

    pub fn set_garage_door_to_rotate(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        if !garage.rotated_door {
            garage.rotated_door = true;
            garage.door_height = garage.door_height / 2.0 - 0.1;
        }
        true
    }

    /// re3 `CGarages::SetTargetCarForMissonGarage`.
    pub fn set_target_car_for_mission_garage(
        &mut self,
        garage: i32,
        target_vehicle_slot: Option<usize>,
    ) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.target_vehicle_slot = target_vehicle_slot;
        if target_vehicle_slot.is_some() && garage.garage_state == EGarageState::ClosedContainsCar {
            garage.garage_state = EGarageState::FullyClosed;
        }
        true
    }

    /// re3 `CGarages::HasCarBeenDroppedOffYet`.
    pub fn has_car_been_dropped_off_yet(&self, garage: i32) -> bool {
        usize::try_from(garage)
            .ok()
            .and_then(|index| self.garages.get(index))
            .is_some_and(|garage| garage.garage_state == EGarageState::ClosedContainsCar)
    }

    pub fn open_garage(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.open_this_garage();
        true
    }

    pub fn close_garage(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.close_this_garage();
        true
    }

    /// re3 `CGarages::DeActivateGarage`.
    pub fn deactivate_garage(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.deactivated = true;
        true
    }

    /// re3 `CGarages::ActivateGarage`.
    pub fn activate_garage(&mut self, garage: i32) -> bool {
        let Ok(index) = usize::try_from(garage) else {
            return false;
        };
        let Some(garage) = self.garages.get_mut(index) else {
            return false;
        };
        garage.deactivated = false;
        if garage.garage_type == EGarageType::ForCarToComeOutOf
            && garage.garage_state == EGarageState::FullyClosed
        {
            garage.garage_state = EGarageState::Opening;
        }
        true
    }

    /// re3 `CGarages::HasResprayHappened` consumes the one-shot flag.
    pub fn take_respray_happened(&mut self, garage: i32) -> Option<bool> {
        let garage = self.garages.get_mut(usize::try_from(garage).ok()?)?;
        Some(std::mem::take(&mut garage.respray_happened))
    }

    /// re3 `CGarages::TriggerMessage`.
    pub fn trigger_message(
        &mut self,
        text: &[u8],
        number1: i32,
        duration_ms: u32,
        number2: i32,
        time_ms: u32,
    ) {
        let end = text
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(text.len());
        let same =
            self.message_id_string[..end.min(MESSAGE_LENGTH)] == text[..end.min(MESSAGE_LENGTH)];
        if same && time_ms >= self.message_start_time && time_ms <= self.message_end_time {
            if time_ms.wrapping_sub(self.message_start_time) <= 500 {
                return;
            }
            self.message_start_time = time_ms.wrapping_sub(500);
            self.message_end_time = time_ms.wrapping_sub(500).wrapping_add(duration_ms);
        } else {
            self.message_id_string.fill(0);
            let count = end.min(MESSAGE_LENGTH.saturating_sub(1));
            self.message_id_string[..count].copy_from_slice(&text[..count]);
            self.message_start_time = time_ms;
            self.message_end_time = time_ms.wrapping_add(duration_ms);
        }
        self.message_number_in_string = number1;
        self.message_number_in_string2 = number2;
    }

    fn craig_garage_index(garage_type: EGarageType) -> Option<usize> {
        match garage_type {
            EGarageType::CollectCars1 => Some(0),
            EGarageType::CollectCars2 => Some(1),
            EGarageType::CollectCars3 => Some(2),
            _ => None,
        }
    }

    /// re3 `CGarages::HasImportExportGarageCollectedThisCar` backing mask.
    ///
    /// The SCM garage handle addresses `aGarages`; that garage's type then
    /// selects one of the three `CarTypesCollected` bit fields.
    pub fn import_export_collection_mask(&self, garage_handle: i32) -> Option<i32> {
        let garage_index = usize::try_from(garage_handle).ok()?;
        if garage_index >= self.num_garages as usize {
            return None;
        }
        let garage = self.garages.get(garage_index)?;
        let collection_index = Self::craig_garage_index(garage.garage_type)?;
        self.car_types_collected.get(collection_index).copied()
    }

    pub fn has_import_export_garage_collected_slot(
        &self,
        garage_handle: i32,
        car_slot: i32,
    ) -> bool {
        let Ok(bit) = u32::try_from(car_slot) else {
            return false;
        };
        bit < i32::BITS
            && self
                .import_export_collection_mask(garage_handle)
                .is_some_and(|mask| mask & (1_i32 << bit) != 0)
    }

    pub fn craig_car_index(garage_type: EGarageType, model: i32) -> Option<(usize, usize)> {
        let model = if model == MI_CORPSE as i32 {
            MI_MANANA as i32
        } else {
            model
        };
        let garage = Self::craig_garage_index(garage_type)?;
        let car = CARS_TO_COLLECT_IN_CRAIGS_GARAGES[garage]
            .iter()
            .position(|candidate| *candidate == model)?;
        Some((garage, car))
    }

    pub fn does_craig_need_this_car(&self, garage_type: EGarageType, model: i32) -> bool {
        Self::craig_car_index(garage_type, model)
            .is_some_and(|(garage, car)| self.car_types_collected[garage] & (1_i32 << car) == 0)
    }

    pub fn has_craig_collected_this_car(&self, garage_type: EGarageType, model: i32) -> bool {
        Self::craig_car_index(garage_type, model)
            .is_some_and(|(garage, car)| self.car_types_collected[garage] & (1_i32 << car) != 0)
    }

    /// Marks one import/export delivery and returns whether all sixteen bits
    /// for that garage are now present.
    pub fn mark_this_car_as_collected_for_craig(
        &mut self,
        garage_type: EGarageType,
        model: i32,
    ) -> Option<bool> {
        let (garage, car) = Self::craig_car_index(garage_type, model)?;
        self.car_types_collected[garage] |= 1_i32 << car;
        let all_collected = (0..TOTAL_COLLECTCARS_CARS)
            .all(|index| self.car_types_collected[garage] & (1_i32 << index) != 0);
        Some(all_collected)
    }

    /// The matching subset of `CGarages::Update` for script-only garages.
    pub fn update_script_controlled_doors(&mut self, time_step: f32) {
        for garage in self
            .garages
            .iter_mut()
            .take(self.num_garages.min(NUM_GARAGES as u32) as usize)
        {
            if garage.garage_type != EGarageType::None {
                garage.update_script_controlled_door(time_step);
            }
        }
    }

    /// re3 `CGarages::IsPointInAGarageCameraZone`.
    pub fn is_point_in_camera_zone(&self, point: CVector) -> bool {
        self.garages
            .iter()
            .take(self.num_garages.min(NUM_GARAGES as u32) as usize)
            .any(|garage| {
                if garage.garage_type == EGarageType::None {
                    return false;
                }
                let margin = if matches!(
                    garage.garage_type,
                    EGarageType::CollectCars1
                        | EGarageType::CollectCars2
                        | EGarageType::CollectCars3
                ) {
                    1.3
                } else {
                    4.0
                };
                garage.x1 - margin <= point.x
                    && garage.x2 + margin >= point.x
                    && garage.y1 - margin <= point.y
                    && garage.y2 + margin >= point.y
            })
    }

    pub fn player_arrested_or_died(&mut self) {
        for garage in &mut self.garages {
            if garage.garage_type != EGarageType::None {
                garage.player_arrested_or_died();
            }
        }
        self.message_end_time = 0;
        self.message_start_time = 0;
    }

    pub fn save(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write(self.num_garages);
        writer.write(bool_to_u32(self.bombs_are_free));
        writer.write(bool_to_u32(self.resprays_are_free));
        writer.write(self.cars_collected);
        writer.write(self.bank_vans_collected);
        writer.write(self.police_cars_collected);
        for collected in self.car_types_collected {
            writer.write(collected);
        }
        writer.write(self.last_time_help_message);
        for index in 0..NUM_GARAGE_STORED_CARS {
            self.cars_in_safe_house1[index].write_save(&mut writer);
            self.cars_in_safe_house2[index].write_save(&mut writer);
            self.cars_in_safe_house3[index].write_save(&mut writer);
        }
        for garage in self.garages {
            garage.write_compatible_save(&mut writer);
        }
        debug_assert_eq!(writer.offset(), GARAGES_COMPATIBLE_SAVE_SIZE);
        writer.zero(GARAGES_SAVE_SIZE - writer.offset());
        writer.into_inner()
    }

    pub fn load(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let mut garages = Self::init();
        garages.num_garages = reader.read::<u32>()?;
        garages.bombs_are_free = reader.read::<i32>()? != 0;
        garages.resprays_are_free = reader.read::<i32>()? != 0;
        garages.cars_collected = reader.read::<i32>()?;
        garages.bank_vans_collected = reader.read::<i32>()?;
        garages.police_cars_collected = reader.read::<i32>()?;
        for collected in &mut garages.car_types_collected {
            *collected = reader.read::<i32>()?;
        }
        garages.last_time_help_message = reader.read::<u32>()?;
        for index in 0..NUM_GARAGE_STORED_CARS {
            garages.cars_in_safe_house1[index] = CStoredCar::read_save(&mut reader)?;
            garages.cars_in_safe_house2[index] = CStoredCar::read_save(&mut reader)?;
            garages.cars_in_safe_house3[index] = CStoredCar::read_save(&mut reader)?;
        }
        for garage in &mut garages.garages {
            *garage = CGarage::read_compatible_save(&mut reader)?;
        }
        if reader.offset() < bytes.len() {
            reader.skip(bytes.len() - reader.offset())?;
        }
        garages.message_end_time = 0;
        garages.cam_should_be_outside = false;
        garages.message_start_time = 0;
        Ok(garages)
    }
}

fn bool_to_u8(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn bool_to_u32(value: bool) -> u32 {
    if value { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garage_moves_live_door_after_population_promotion_and_dummy_after_demotion() {
        use crate::control::ScriptEntities::{
            ScriptObjectEntity, script_entity_test_world, with_script_entity_test_world,
        };
        use crate::objects::Object::CObject;
        use rust_gta_3_formats::formats::FileLoader::GtaIplInstance;

        let mut loader = CFileLoader::default();
        let model = loader.model_info.add_simple_model(1000);
        model.base.set_model_name("ind_plyrwoor");
        model.base.object_id = 0;
        let index = loader.world.load_object_instance(&GtaIplInstance {
            id: 1000, name: "ind_plyrwoor".into(), position: [0.0, 0.0, 10.0],
            scale: [1.0; 3], rotation_axis: [0.0, 0.0, 1.0], angle: 1.0,
        }, &loader.model_info, &Default::default()).unwrap();
        let mut garages = CGarages::init();
        garages.garages[0] = CGarage::add_one(CVector::new(-5.0, -5.0, 0.0), CVector::new(5.0, 5.0, 20.0),
            EGarageType::ForScriptToOpenAndClose, 0);
        refresh_garage_door_entities(&mut garages, &loader, 1);
        assert_eq!(garages.garages[0].door1_world_index, Some(index));
        for frame in 0..32 {
            loader.world.update_object_dummy_collision_lifecycle(CVector::ZERO, frame, &loader.model_info);
        }
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(4);
        with_script_entity_test_world(&mut world, |actors| {
            let mut object = CObject::new_with_model(1000, false, &mut general);
            object.physical.entity.set_position(CVector::new(0.0, 0.0, 10.0));
            actors.object_spawn(ScriptObjectEntity { handle: 1, object });
        });
        loader.world.ecs_lifecycle_objects.insert(index, 1);
        for rotated in [false, true] {
            garages.garages[0].rotated_door = rotated;
            garages.garages[0].door_height = 4.0;
            garages.garages[0].door_pos = 2.0;
            with_script_entity_test_world(&mut world, |actors| {
                apply_garage_door_transforms(&garages, &mut loader, 1, actors);
                let door = actors.object_get(1).unwrap();
                assert_eq!(door.object.physical.entity.get_position().z, 12.0);
                if rotated {
                    assert!((door.object.physical.entity.matrix.up.z - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
                }
            });
            assert_eq!(loader.world.entities[index].position[2], 10.0);
            assert!(!loader.world.entities[index].is_visible);
        }
        loader.world.ecs_lifecycle_objects.remove(&index);
        for frame in 0..32 {
            loader.world.update_object_dummy_collision_lifecycle(CVector::new(200.0, 0.0, 0.0), frame, &loader.model_info);
        }
        with_script_entity_test_world(&mut world, |actors| {
            actors.object_remove(None, 1);
            apply_garage_door_transforms(&garages, &mut loader, 1, actors);
        });
        assert!(loader.world.entities[index].is_visible);
        assert_eq!(loader.world.entities[index].position[2], 12.0);
    }

    #[test]
    fn script_open_close_door_uses_reference_speeds_and_terminal_states() {
        let mut garages = CGarages::init();
        let handle = garages.add_one(
            CVector::ZERO,
            CVector::new(5.0, 5.0, 5.0),
            EGarageType::ForScriptToOpenAndClose,
            0,
        );
        assert!(garages.open_garage(handle));
        garages.update_script_controlled_doors(100.0);
        assert_eq!(
            garages.garages[handle as usize].garage_state,
            EGarageState::Opening
        );
        assert_eq!(
            garages.garages[handle as usize].door_pos,
            DEFAULT_DOOR_OPEN_SPEED * 100.0
        );
        garages.update_script_controlled_doors(100.0);
        assert_eq!(
            garages.garages[handle as usize].garage_state,
            EGarageState::Opened
        );
        assert_eq!(garages.garages[handle as usize].door_pos, 4.0);

        assert!(garages.close_garage(handle));
        garages.update_script_controlled_doors(100.0);
        assert_eq!(
            garages.garages[handle as usize].garage_state,
            EGarageState::FullyClosed
        );
        assert_eq!(garages.garages[handle as usize].door_pos, 0.0);
    }

    #[test]
    fn save_load_drops_transient_target_like_cpp() {
        let mut garages = CGarages::init();
        let handle = garages.add_one(
            CVector::ZERO,
            CVector::new(5.0, 5.0, 5.0),
            EGarageType::Mission,
            0,
        );
        assert!(garages.set_target_car_for_mission_garage(handle, Some(7)));
        let loaded = CGarages::load(&garages.save()).unwrap();
        assert_eq!(loaded.garages[handle as usize].target_vehicle_slot, None);
    }

    fn mission_context() -> MissionGarageUpdateContext {
        MissionGarageUpdateContext {
            time_step: 1.0,
            time_ms: 1_000,
            frame_counter: 1,
            player_position: CVector::new(5.0, 5.0, 0.0),
            player_vehicle_slot: None,
            player_vehicle_position: None,
            target_exists: true,
            target_entirely_inside: true,
            target_entirely_outside: false,
            player_entirely_outside_margin_2: true,
            no_car_touching_garage: false,
            no_other_car_touching_target: true,
            static_player_target_entirely_inside: false,
            no_car_blocking_door: true,
        }
    }

    #[test]
    fn mission_garage_dropoff_destroys_target_at_fully_closed_boundary() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Mission,
            0,
        );
        garage.garage_state = EGarageState::Opened;
        garage.door_pos = garage.door_height;
        garage.target_vehicle_slot = Some(7);

        let result = garage.update_mission_garage(mission_context());
        assert_eq!(garage.garage_state, EGarageState::Closing);
        assert!(!garage.closing_without_target_car);
        assert_eq!(result.player_controls_enabled, Some(false));
        assert_eq!(result.player_ignored_by_cops, Some(true));

        let result = garage.update_mission_garage(MissionGarageUpdateContext {
            time_step: 100.0,
            ..mission_context()
        });
        assert_eq!(garage.garage_state, EGarageState::ClosedContainsCar);
        assert_eq!(garage.target_vehicle_slot, None);
        assert_eq!(result.destroy_vehicle_slot, Some(7));
        assert_eq!(result.player_controls_enabled, Some(true));
        assert_eq!(result.player_ignored_by_cops, Some(false));
    }

    #[test]
    fn mission_garage_far_close_preserves_target_and_waits_for_frame_slice() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Mission,
            0,
        );
        garage.garage_state = EGarageState::Opened;
        garage.door_pos = garage.door_height;
        garage.target_vehicle_slot = Some(7);
        let far = MissionGarageUpdateContext {
            player_position: CVector::new(100.0, 100.0, 0.0),
            frame_counter: 31,
            no_car_touching_garage: true,
            ..mission_context()
        };
        garage.update_mission_garage(far);
        assert_eq!(garage.garage_state, EGarageState::Opened);

        garage.update_mission_garage(MissionGarageUpdateContext {
            frame_counter: 32,
            ..far
        });
        assert_eq!(garage.garage_state, EGarageState::Closing);
        assert!(garage.closing_without_target_car);
        let result = garage.update_mission_garage(MissionGarageUpdateContext {
            time_step: 100.0,
            ..far
        });
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);
        assert_eq!(garage.target_vehicle_slot, Some(7));
        assert_eq!(result.destroy_vehicle_slot, None);
    }

    #[test]
    fn keep_car_garages_release_reference_and_only_normal_variant_reopens() {
        for (garage_type, should_reopen) in [
            (EGarageType::MissionKeepCar, true),
            (EGarageType::MissionKeepCarRemainClosed, false),
        ] {
            let mut garage =
                CGarage::add_one(CVector::ZERO, CVector::new(10.0, 10.0, 5.0), garage_type, 0);
            garage.garage_state = EGarageState::Opened;
            garage.door_pos = garage.door_height;
            garage.target_vehicle_slot = Some(9);
            let context = MissionGarageUpdateContext {
                player_vehicle_slot: Some(9),
                player_vehicle_position: Some(CVector::new(5.0, 5.0, 0.0)),
                static_player_target_entirely_inside: true,
                ..mission_context()
            };

            let entering = garage.update_mission_garage(context);
            assert_eq!(garage.garage_state, EGarageState::Closing);
            assert_eq!(entering.player_controls_enabled, Some(false));
            let closed = garage.update_mission_garage(MissionGarageUpdateContext {
                time_step: 100.0,
                time_ms: 4_000,
                ..context
            });
            assert_eq!(garage.garage_state, EGarageState::ClosedContainsCar);
            assert_eq!(garage.target_vehicle_slot, None);
            assert_eq!(garage.time_to_start_action, 6_000);
            assert_eq!(closed.destroy_vehicle_slot, None);
            assert_eq!(closed.player_controls_enabled, Some(true));

            garage.update_mission_garage(MissionGarageUpdateContext {
                time_ms: 6_001,
                target_exists: false,
                ..context
            });
            assert_eq!(
                garage.garage_state,
                if should_reopen {
                    EGarageState::Opening
                } else {
                    EGarageState::ClosedContainsCar
                }
            );
        }
    }

    #[test]
    fn specific_car_garage_cycles_only_after_target_has_left() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::KeepsOpeningForSpecificCar,
            0,
        );
        garage.garage_state = EGarageState::Opened;
        garage.door_pos = garage.door_height;
        garage.target_vehicle_slot = Some(4);
        let far = MissionGarageUpdateContext {
            player_position: CVector::new(100.0, 100.0, 0.0),
            no_car_touching_garage: true,
            target_entirely_outside: false,
            ..mission_context()
        };
        garage.update_mission_garage(far);
        assert_eq!(garage.garage_state, EGarageState::Opened);
        garage.update_mission_garage(MissionGarageUpdateContext {
            target_entirely_outside: true,
            ..far
        });
        assert_eq!(garage.garage_state, EGarageState::Closing);
        garage.update_mission_garage(MissionGarageUpdateContext {
            time_step: 100.0,
            ..far
        });
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);

        garage.update_mission_garage(MissionGarageUpdateContext {
            player_vehicle_slot: Some(4),
            player_vehicle_position: Some(CVector::new(-1.0, 5.0, 0.0)),
            ..mission_context()
        });
        assert_eq!(garage.garage_state, EGarageState::Opening);
    }

    #[test]
    fn car_exit_garage_reopens_if_an_entity_blocks_closing() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::ForCarToComeOutOf,
            0,
        );
        garage.garage_state = EGarageState::Opened;
        garage.door_pos = garage.door_height;
        garage.update_for_car_to_come_out_of(1.0, true);
        assert_eq!(garage.garage_state, EGarageState::Closing);
        let position = garage.door_pos;
        garage.update_for_car_to_come_out_of(1.0, false);
        assert_eq!(garage.garage_state, EGarageState::Opening);
        assert_eq!(garage.door_pos, position - DEFAULT_DOOR_CLOSE_SPEED);
        garage.update_for_car_to_come_out_of(100.0, false);
        assert_eq!(garage.garage_state, EGarageState::Opened);
    }

    #[test]
    fn respray_runs_two_second_close_repair_open_cycle() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Respray,
            0,
        );
        let context = ResprayGarageUpdateContext {
            time_step: 1.0,
            time_ms: 1_000,
            static_player_car_entirely_inside: true,
            no_other_car_touching_player_car: true,
            player_car_sprayable: true,
            player_can_afford: true,
            player_vehicle_near: true,
            player_outside_garage: false,
        };
        let closing = garage.update_respray(context);
        assert_eq!(garage.garage_state, EGarageState::Closing);
        assert_eq!(closing.player_controls_enabled, Some(false));
        assert_eq!(closing.player_ignored_by_cops, Some(true));
        assert!(closing.call_off_chase);

        let closed = garage.update_respray(ResprayGarageUpdateContext {
            time_step: 100.0,
            time_ms: 2_000,
            ..context
        });
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);
        assert_eq!(garage.time_to_start_action, 4_000);
        assert!(closed.checkpoint_reached);
        assert!(closed.reset_player_car_fire_timer);

        let waiting = garage.update_respray(ResprayGarageUpdateContext {
            time_ms: 4_000,
            ..context
        });
        assert!(!waiting.complete_respray);
        let repaired = garage.update_respray(ResprayGarageUpdateContext {
            time_ms: 4_001,
            ..context
        });
        assert_eq!(garage.garage_state, EGarageState::Opening);
        assert!(garage.respray_happened);
        assert!(repaired.complete_respray);
        assert_eq!(repaired.player_controls_enabled, Some(true));

        garage.update_respray(ResprayGarageUpdateContext {
            time_step: 200.0,
            ..context
        });
        assert_eq!(garage.garage_state, EGarageState::OpenedContainsCar);
        garage.update_respray(ResprayGarageUpdateContext {
            player_outside_garage: true,
            ..context
        });
        assert_eq!(garage.garage_state, EGarageState::Opened);
    }

    #[test]
    fn respray_rejections_and_message_retrigger_timing_match_cpp() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Respray,
            0,
        );
        let rejected = garage.update_respray(ResprayGarageUpdateContext {
            static_player_car_entirely_inside: true,
            no_other_car_touching_player_car: true,
            player_car_sprayable: false,
            player_can_afford: true,
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::OpenedContainsCar);
        assert_eq!(rejected.rejection_message, Some(*b"GA_1"));

        let mut garages = CGarages::init();
        garages.trigger_message(b"GA_1", -1, 4_000, -1, 1_000);
        assert_eq!(garages.message_start_time, 1_000);
        garages.trigger_message(b"GA_1", -1, 4_000, -1, 1_400);
        assert_eq!(garages.message_start_time, 1_000);
        garages.trigger_message(b"GA_1", -1, 4_000, -1, 1_600);
        assert_eq!(garages.message_start_time, 1_100);
        assert_eq!(garages.message_end_time, 5_100);
    }

    #[test]
    fn bombshop_installs_type_after_two_second_closed_interval() {
        for (garage_type, bomb_type) in [
            (EGarageType::Bombshop1, 1),
            (EGarageType::Bombshop2, 2),
            (EGarageType::Bombshop3, 3),
        ] {
            let mut garage =
                CGarage::add_one(CVector::ZERO, CVector::new(10.0, 10.0, 5.0), garage_type, 0);
            let context = BombshopGarageUpdateContext {
                time_step: 1.0,
                time_ms: 1_000,
                static_player_car_entirely_inside: true,
                no_other_car_touching_player_car: true,
                player_car_already_has_bomb: false,
                player_can_afford: true,
                player_outside_garage: false,
            };
            let closing = garage.update_bombshop(context);
            assert_eq!(garage.garage_state, EGarageState::Closing);
            assert_eq!(closing.player_controls_enabled, Some(false));
            garage.update_bombshop(BombshopGarageUpdateContext {
                time_step: 100.0,
                time_ms: 2_000,
                ..context
            });
            assert_eq!(garage.garage_state, EGarageState::FullyClosed);
            assert_eq!(garage.time_to_start_action, 4_000);
            assert_eq!(
                garage
                    .update_bombshop(BombshopGarageUpdateContext {
                        time_ms: 4_000,
                        ..context
                    })
                    .install_bomb_type,
                None
            );
            let installed = garage.update_bombshop(BombshopGarageUpdateContext {
                time_ms: 4_001,
                ..context
            });
            assert_eq!(installed.install_bomb_type, Some(bomb_type));
            assert_eq!(installed.player_controls_enabled, Some(true));
            assert_eq!(garage.garage_state, EGarageState::Opening);
        }
    }

    #[test]
    fn bombshop_rejects_existing_bomb_before_money_test() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Bombshop1,
            0,
        );
        let result = garage.update_bombshop(BombshopGarageUpdateContext {
            static_player_car_entirely_inside: true,
            no_other_car_touching_player_car: true,
            player_car_already_has_bomb: true,
            player_can_afford: false,
            ..Default::default()
        });
        assert_eq!(result.rejection_message, Some(*b"GA_5"));
        assert_eq!(garage.garage_state, EGarageState::OpenedContainsCar);
    }

    #[test]
    fn collect_specific_car_disables_controls_then_destroys_delivered_target() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::CollectSpecificCars,
            MI_POLICE as i32,
        );
        garage.garage_state = EGarageState::Opened;
        garage.door_pos = garage.door_height;
        garage.update_collect_specific_cars(CollectGarageUpdateContext {
            player_vehicle_slot: Some(7),
            player_vehicle_model: Some(MI_POLICE as i32),
            ..Default::default()
        });
        assert_eq!(garage.target_vehicle_slot, Some(7));

        let closing = garage.update_collect_specific_cars(CollectGarageUpdateContext {
            target_exists: true,
            target_model: Some(MI_POLICE as i32),
            target_entirely_inside: true,
            no_other_car_touching_target: true,
            player_entirely_outside_margin_2: true,
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::Closing);
        assert_eq!(closing.player_controls_enabled, Some(false));
        assert_eq!(closing.player_ignored_by_cops, Some(true));

        let delivered = garage.update_collect_specific_cars(CollectGarageUpdateContext {
            time_step: 100.0,
            target_exists: true,
            target_model: Some(MI_POLICE as i32),
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);
        assert_eq!(delivered.destroy_vehicle_slot, Some(7));
        assert_eq!(delivered.delivered_model, Some(MI_POLICE as i32));
        assert_eq!(delivered.player_controls_enabled, Some(true));
    }

    #[test]
    fn import_garage_rejects_mission_vehicle_before_opening() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::CollectCars2,
            0,
        );
        garage.door1_world_index = Some(1);
        garage.door1_x = 0.0;
        garage.door1_y = 0.0;
        let context = CollectGarageUpdateContext {
            player_vehicle_slot: Some(3),
            player_vehicle_model: Some(MI_SENTINEL as i32),
            player_vehicle_position: Some(CVector::ZERO),
            player_vehicle_is_mission: true,
            craig_needs_player_car: true,
            ..Default::default()
        };
        let rejected = garage.update_collect_cars(context);
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);
        assert_eq!(rejected.message, Some((message_key(b"GA_1A"), 5_000)));

        let accepted = garage.update_collect_cars(CollectGarageUpdateContext {
            player_vehicle_is_mission: false,
            ..context
        });
        assert_eq!(accepted.message, None);
        assert_eq!(garage.garage_state, EGarageState::Opening);
    }

    #[test]
    fn craig_collection_maps_corpse_to_manana_and_reports_completion() {
        let mut garages = CGarages::init();
        let (garage_index, manana_index) =
            CGarages::craig_car_index(EGarageType::CollectCars2, MI_MANANA as i32).unwrap();
        assert_eq!(
            CGarages::craig_car_index(EGarageType::CollectCars2, MI_CORPSE as i32),
            Some((garage_index, manana_index))
        );
        assert_eq!(
            garages
                .mark_this_car_as_collected_for_craig(EGarageType::CollectCars2, MI_CORPSE as i32,),
            Some(false)
        );
        assert!(garages.has_craig_collected_this_car(EGarageType::CollectCars2, MI_MANANA as i32));

        garages.car_types_collected[garage_index] =
            ((1_u32 << TOTAL_COLLECTCARS_CARS) - 1) as i32 & !(1_i32 << 0);
        assert_eq!(
            garages.mark_this_car_as_collected_for_craig(
                EGarageType::CollectCars2,
                MI_SENTINEL as i32,
            ),
            Some(true)
        );
    }

    #[test]
    fn crusher_immobilizes_crushes_shakes_then_reopens() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Crusher,
            0,
        );
        garage.update_crusher(CrusherGarageUpdateContext {
            candidate_vehicle_slot: Some(6),
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::Closing);
        assert_eq!(garage.target_vehicle_slot, Some(6));

        let immobilized = garage.update_crusher(CrusherGarageUpdateContext {
            time_step: 100.0,
            target_exists: true,
            ..Default::default()
        });
        assert!(immobilized.immobilize_target);
        assert_eq!(immobilized.crush_vehicle_slot, None);

        let crushed = garage.update_crusher(CrusherGarageUpdateContext {
            time_step: 300.0,
            time_ms: 2_000,
            target_exists: true,
            player_on_foot_inside: true,
            frame_counter: 0x17,
            ..Default::default()
        });
        assert_eq!(crushed.crush_vehicle_slot, Some(6));
        assert!(crushed.damage_player);
        assert_eq!(garage.garage_state, EGarageState::AfterDropoff);
        assert_eq!(garage.time_to_start_action, 5_000);

        assert!(
            garage
                .update_crusher(CrusherGarageUpdateContext {
                    time_ms: 5_000,
                    ..Default::default()
                })
                .shake
        );
        let finished = garage.update_crusher(CrusherGarageUpdateContext {
            time_ms: 5_001,
            ..Default::default()
        });
        assert!(!finished.shake);
        assert_eq!(garage.garage_state, EGarageState::Opening);
        garage.update_crusher(CrusherGarageUpdateContext {
            time_step: 1_000.0,
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::Opened);
        assert_eq!(garage.door_pos, std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn hideout_closing_reopens_when_player_is_not_outside() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::HideoutOne,
            0,
        );
        garage.garage_state = EGarageState::Closing;
        garage.door_pos = 2.0;
        let result = garage.update_hideout(HideoutGarageUpdateContext {
            time_step: 1.0,
            player_position: CVector::new(5.0, 5.0, 1.0),
            player_outside_garage: false,
            ..Default::default()
        });
        assert_eq!(garage.garage_state, EGarageState::Opening);
        assert_eq!(
            garage.door_pos,
            2.0 - HIDEOUT_DOOR_SPEED_COEFFICIENT * DEFAULT_DOOR_CLOSE_SPEED
        );
        assert_eq!(result, HideoutGarageUpdateResult::default());
    }

    #[test]
    fn hideout_closed_requests_restore_and_opens_only_after_adapter_succeeds() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::HideoutTwo,
            0,
        );
        garage.garage_state = EGarageState::FullyClosed;
        let result = garage.update_hideout(HideoutGarageUpdateContext {
            player_position: CVector::new(5.0, 5.0, 1.0),
            player_outside_garage: true,
            ..Default::default()
        });
        assert!(result.restore_cars);
        assert_eq!(garage.garage_state, EGarageState::FullyClosed);
    }

    #[test]
    fn full_hideout_shows_ga21_only_near_door_after_eighteen_seconds() {
        let mut garage = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::HideoutThree,
            0,
        );
        garage.garage_state = EGarageState::FullyClosed;
        garage.door1_world_index = Some(4);
        garage.door1_x = 5.0;
        garage.door1_y = -1.0;
        let context = HideoutGarageUpdateContext {
            time_ms: TIME_BETWEEN_HIDEOUT_MESSAGES + 1,
            player_position: CVector::new(5.0, -1.0, 1.0),
            player_vehicle_position: Some(CVector::new(5.0, -1.0, 1.0)),
            player_outside_garage: true,
            stored_car_count: 3,
            ..Default::default()
        };
        let result = garage.update_hideout(context);
        assert!(result.show_full_message);
        assert!(!result.restore_cars);

        let too_soon = garage.update_hideout(HideoutGarageUpdateContext {
            last_help_message_time: 2,
            time_ms: TIME_BETWEEN_HIDEOUT_MESSAGES + 2,
            ..context
        });
        assert!(!too_soon.show_full_message);
    }

    #[test]
    fn garage_camera_zone_uses_collect_and_default_cpp_margins() {
        let mut collect = CGarages::init();
        collect.garages[0] = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::CollectCars1,
            0,
        );
        assert!(collect.is_point_in_camera_zone(CVector::new(11.2, 5.0, 0.0)));
        assert!(!collect.is_point_in_camera_zone(CVector::new(11.4, 5.0, 0.0)));

        let mut normal = CGarages::init();
        normal.garages[0] = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::Mission,
            0,
        );
        assert!(normal.is_point_in_camera_zone(CVector::new(13.9, 5.0, 0.0)));
        assert!(!normal.is_point_in_camera_zone(CVector::new(14.1, 5.0, 0.0)));
    }

    #[test]
    fn import_export_slot_uses_the_garage_types_collection_mask() {
        let mut garages = CGarages::init();
        garages.garages[1] = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::CollectCars2,
            0,
        );
        garages.num_garages = 2;
        garages.car_types_collected[1] = 1 << 3;

        assert_eq!(garages.import_export_collection_mask(1), Some(1 << 3));
        assert!(garages.has_import_export_garage_collected_slot(1, 3));
        assert!(!garages.has_import_export_garage_collected_slot(1, 2));
        assert!(!garages.has_import_export_garage_collected_slot(0, 3));
    }

    #[test]
    fn activate_and_deactivate_garage_match_force_car_out_contract() {
        let mut garages = CGarages::init();
        garages.garages[1] = CGarage::add_one(
            CVector::ZERO,
            CVector::new(10.0, 10.0, 5.0),
            EGarageType::ForCarToComeOutOf,
            0,
        );
        garages.num_garages = 2;

        assert!(garages.deactivate_garage(1));
        assert!(garages.garages[1].deactivated);
        assert_eq!(garages.garages[1].garage_state, EGarageState::FullyClosed);

        assert!(garages.activate_garage(1));
        assert!(!garages.garages[1].deactivated);
        assert_eq!(garages.garages[1].garage_state, EGarageState::Opening);
    }
}

// === FAZ7a: moved from core/Game.rs (verbatim, parity-preserving) ===
// Garage runtime: update tick plus respray/hideout/door helpers.
// Schedule stays in core/Game.rs.

#[allow(unused_imports)]
use self::message_key as garage_text_key;
#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::core::Camera::{CCamera, apply_garage_camera_override};
#[allow(unused_imports)]
use crate::core::EventList::CEventList;
#[allow(unused_imports)]
use crate::core::Game::{
    GameplayAuditTimer, ScriptEntityWorld, ScriptPedEntity, ScriptTaxiLightState,
    ScriptVehicleEntity, script_vehicle_base_can_be_deleted,
};
#[allow(unused_imports)]
use crate::core::Pad::{CPads, PLAYERCONTROL_GARAGE};
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use crate::core::Pools::{CPoolHandle, CPools};
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::core::Streaming::{CStreaming, STREAMFLAGS_DEPENDENCY};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::CVehicleColours;
#[allow(unused_imports)]
use crate::peds::Ped::{PedState, eObjective, give_ped_weapon_with_info};
use crate::weapons::WeaponInfo::CWeaponInfoStore;
#[allow(unused_imports)]
use crate::peds::PlayerPed::{CWorldPlayers, WorldPlayerAccess};
#[allow(unused_imports)]
use crate::renderer::Hud::CHud;
#[allow(unused_imports)]
use crate::renderer::Particle::{CParticle, ParticleEmitter};
#[allow(unused_imports)]
use crate::vehicles::Automobile::{CAutomobile, script_bomb_type};
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::MISSION_VEHICLE;
#[allow(unused_imports)]
use crate::weapons::WeaponType::eWeaponType;
#[allow(unused_imports)]
use rust_gta_3_core::core::General::CGeneral;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

pub(crate) fn update_garages(
    timer: Res<CTimer>,
    garages: Option<ResMut<CGarages>>,
    mut file_loader: ResMut<CFileLoader>,
    mut script_entities: ScriptEntityWorld,
    garage_vehicle_runtime: (
        Option<ResMut<CPools>>,
        Option<ResMut<CStreaming>>,
        Option<Res<cHandlingDataMgr>>,
        Res<ScriptTaxiLightState>,
        Res<CWeaponInfoStore>,
    ),
    mut world_players: CWorldPlayers,
    mut pads: ResMut<CPads>,
    mut player_info: Option<ResMut<CPlayerInfo>>,
    mut stats: Option<ResMut<CStats>>,
    mut general: ResMut<CGeneral>,
    particle_inputs: (
        Option<ResMut<CParticle>>,
        Option<Res<crate::renderer::ParticleMgr::cParticleSystemMgr>>,
    ),
    vehicle_colours: Option<Res<CVehicleColours>>,
    text: Option<Res<CText>>,
    mut hud: Option<ResMut<CHud>>,
    mut camera: Option<ResMut<CCamera>>,
    mut events: ResMut<CEventList>,
) {
    let (mut particles, particle_systems) = particle_inputs;
    let Some(mut garages) = garages else {
        return;
    };
    let (mut pools, mut streaming, handling_manager, taxi_lights, weapon_info) = garage_vehicle_runtime;
    // re3 `CGarages::Update` clears these raw pointers at the beginning of
    // every frame; individual used garages repopulate them below.
    garages.cam_should_be_outside = false;
    if let Some(camera) = camera.as_mut() {
        camera.garage_we_are_in = None;
        camera.garage_for_first_person_hack = None;
    }

    let player_relation = world_players.relation(world_players.player_in_focus() as i32);
    let player_vehicle_handle = player_relation
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
    let player_vehicle_slot = player_vehicle_handle.map(|handle| CPoolHandle(handle).index());
    let player_vehicle_position = player_relation
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
        .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
        .map(|vehicle| vehicle.base().physical.entity.get_position());
    // re3 `FindPlayerCoors`: while driving, garage distance checks use the
    // vehicle origin rather than the seated ped's transform.
    let player_position = player_vehicle_position.unwrap_or_else(|| {
        world_players
            .focused()
            .map(|player| player.position())
            .unwrap_or(CVector::ZERO)
    });

    let mut aggregate = MissionGarageUpdateResult::default();
    let mut destroyed_slots = Vec::new();
    let mut collect_deliveries = Vec::<(EGarageType, i32)>::new();
    let mut crushed_car_id = None;
    let mut garage_messages = Vec::<([u8; 8], i32, u32, i32)>::new();
    let mut hideout_store_requests = Vec::<(usize, EGarageType)>::new();
    let mut hideout_restore_requests = Vec::<(usize, EGarageType)>::new();
    let mut hideout_blocker_removal_requests = Vec::<usize>::new();
    let mut hideout_full_message = false;
    let resprays_are_free = garages.resprays_are_free;
    let bombs_are_free = garages.bombs_are_free;
    let car_types_collected = garages.car_types_collected;
    let last_time_help_message = garages.last_time_help_message;
    let safehouse_counts = [
        garages
            .cars_in_safe_house1
            .iter()
            .filter(|car| car.has_car())
            .count(),
        garages
            .cars_in_safe_house2
            .iter()
            .filter(|car| car.has_car())
            .count(),
        garages
            .cars_in_safe_house3
            .iter()
            .filter(|car| car.has_car())
            .count(),
    ];
    let count = garages
        .num_garages
        .min(crate::control::Garages::NUM_GARAGES as u32) as usize;
    refresh_garage_door_entities(&mut garages, &file_loader, count);
    apply_garage_door_transforms(&garages, &mut file_loader, count, &mut script_entities);
    let mut cam_should_be_outside = false;
    for (garage_index, garage) in garages.garages.iter_mut().take(count).enumerate() {
        if garage.garage_type != EGarageType::Crusher
            && !garage.camera_follows_player
            && matches!(
                garage.garage_state,
                EGarageState::FullyClosed
                    | EGarageState::Opened
                    | EGarageState::Closing
                    | EGarageState::Opening
                    | EGarageState::OpenedContainsCar
                    | EGarageState::ClosedContainsCar
            )
        {
            let ped_inside = world_players.focused().is_some_and(|player| {
                garage_entity_entirely_inside_3d(
                    garage,
                    &player.ped.physical.entity,
                    0.25,
                    &file_loader,
                )
            });
            if ped_inside {
                cam_should_be_outside = true;
                if let Some(camera) = camera.as_mut() {
                    camera.garage_we_are_in = Some(garage_index);
                }
            }
            if let Some(slot) = player_vehicle_slot
                && let Some((_, vehicle)) = script_entities
                    .vehicles
                    .iter()
                    .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
            {
                let entity = &vehicle.base().physical.entity;
                if !garage_entity_entirely_outside_2d(garage, entity, 0.0, &file_loader)
                    && let Some(camera) = camera.as_mut()
                {
                    camera.garage_for_first_person_hack = Some(garage_index);
                }
                if usize::try_from(entity.model_index).ok()
                    == Some(crate::modelinfo::ModelIndices::MI_MRWHOOP)
                {
                    let position = entity.get_position();
                    if position.x >= garage.x1 - 4.0
                        && position.x <= garage.x2 - 4.0
                        && position.y >= garage.y1 + 4.0
                        && position.y <= garage.y2 + 4.0
                    {
                        cam_should_be_outside = true;
                        if let Some(camera) = camera.as_mut() {
                            camera.garage_we_are_in = Some(garage_index);
                        }
                    }
                }
            }
        }
        if matches!(
            garage.garage_type,
            EGarageType::HideoutOne | EGarageType::HideoutTwo | EGarageType::HideoutThree
        ) {
            let player_entity = player_vehicle_slot
                .and_then(|slot| {
                    script_entities
                        .vehicles
                        .iter()
                        .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                        .map(|(_, vehicle)| &vehicle.base().core.physical.entity)
                })
                .or_else(|| {
                    world_players
                        .focused()
                        .map(|player| &player.ped.physical.entity)
                });
            let player_outside_garage = player_entity.is_none_or(|entity| {
                garage_entity_entirely_outside_2d(garage, entity, 0.0, &file_loader)
            });
            let cars_inside_excluding_player = script_entities
                .vehicles
                .iter()
                .filter(|(handle, vehicle)| {
                    let position = vehicle.base().physical.entity.get_position();
                    player_vehicle_slot != Some(CPoolHandle(**handle).index())
                        && position.x > garage.x1
                        && position.x < garage.x2
                        && position.y > garage.y1
                        && position.y < garage.y2
                        && position.z > garage.z1
                        && position.z < garage.z2
                })
                .count();
            let stored_car_count = match garage.garage_type {
                EGarageType::HideoutOne => safehouse_counts[0],
                EGarageType::HideoutTwo => safehouse_counts[1],
                EGarageType::HideoutThree => safehouse_counts[2],
                _ => 0,
            };
            let result = garage.update_hideout(HideoutGarageUpdateContext {
                time_step: timer.time_step(),
                time_ms: timer.time_in_milliseconds(),
                player_position,
                player_vehicle_position,
                player_outside_garage,
                no_car_blocking_door: !garage_any_car_blocking_door(
                    garage,
                    &script_entities,
                    &file_loader,
                ),
                cars_inside_excluding_player,
                stored_car_count,
                last_help_message_time: last_time_help_message,
            });
            if result.store_and_remove_cars {
                hideout_store_requests.push((garage_index, garage.garage_type));
            }
            if result.restore_cars {
                hideout_restore_requests.push((garage_index, garage.garage_type));
            }
            if result.remove_blocking_cars_not_inside {
                hideout_blocker_removal_requests.push(garage_index);
            }
            hideout_full_message |= result.show_full_message;
            continue;
        }
        if garage.garage_type == EGarageType::Respray {
            let player_vehicle = player_vehicle_slot.and_then(|slot| {
                script_entities
                    .vehicles
                    .iter()
                    .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                    .map(|(_, vehicle)| vehicle)
            });
            let static_inside = world_players.focused().is_some_and(|player| {
                player.ped.ped_state == PedState::PED_DRIVING
                    && player.ped.objective != eObjective::OBJECTIVE_LEAVE_CAR
            }) && player_vehicle.is_some_and(|vehicle| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                    return false;
                };
                let position = car.vehicle.core.physical.entity.get_position();
                let speed = car.vehicle.core.physical.move_speed;
                position.x >= garage.x1
                    && position.x <= garage.x2
                    && position.y >= garage.y1
                    && position.y <= garage.y2
                    && speed.x.abs() <= 0.01
                    && speed.y.abs() <= 0.01
                    && speed.z.abs() <= 0.01
                    && speed.magnitude_sqr() <= 0.01_f32.powi(2)
                    && garage_entity_entirely_inside_3d(
                        garage,
                        &car.vehicle.core.physical.entity,
                        0.0,
                        &file_loader,
                    )
            });
            let player_model =
                player_vehicle.map(|vehicle| i32::from(vehicle.base().physical.entity.model_index));
            let sprayable = player_model.is_some_and(is_car_sprayable);
            let can_afford = resprays_are_free
                || player_info
                    .as_ref()
                    .is_some_and(|info| info.m_nMoney >= RESPRAY_PRICE);
            let near = player_vehicle_position.is_some_and(|position| {
                garage_distance_to_rectangle_squared(garage, position)
                    < crate::control::Garages::DISTANCE_TO_ACTIVATE_GARAGE.powi(2)
            });
            let player_outside = match player_vehicle {
                Some(vehicle) => garage_entity_entirely_outside_2d(
                    garage,
                    &vehicle.base().physical.entity,
                    0.0,
                    &file_loader,
                ),
                None => world_players.focused().is_none_or(|player| {
                    garage_entity_entirely_outside_2d(
                        garage,
                        &player.ped.physical.entity,
                        0.0,
                        &file_loader,
                    )
                }),
            };
            let result = garage.update_respray(ResprayGarageUpdateContext {
                time_step: timer.time_step(),
                time_ms: timer.time_in_milliseconds(),
                static_player_car_entirely_inside: static_inside,
                no_other_car_touching_player_car: !garage_any_other_car_touching(
                    garage,
                    player_vehicle_slot,
                    &script_entities,
                    &file_loader,
                ),
                player_car_sprayable: sprayable,
                player_can_afford: can_afford,
                player_vehicle_near: near,
                player_outside_garage: player_outside,
            });
            if result.player_controls_enabled.is_some() {
                aggregate.player_controls_enabled = result.player_controls_enabled;
            }
            if result.player_ignored_by_cops.is_some() {
                aggregate.player_ignored_by_cops = result.player_ignored_by_cops;
            }
            if let Some(message) = result.rejection_message {
                let mut key = [0_u8; 8];
                key[..message.len()].copy_from_slice(&message);
                garage_messages.push((key, -1, 4_000, -1));
            }
            if result.checkpoint_reached
                && let Some(stats) = stats.as_mut()
            {
                stats.total_legitimate_kills = stats
                    .total_legitimate_kills
                    .saturating_add(stats.kills_since_last_checkpoint);
                stats.kills_since_last_checkpoint = 0;
            }
            if result.reset_player_car_fire_timer {
                if let Some(slot) = player_vehicle_slot
                    && let Some((_, crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. })) = script_entities
                        .vehicles
                        .iter_mut()
                        .find(|(handle, _)| CPoolHandle(*handle).index() == slot)
                {
                    car.fire_blow_up_timer = 0.0;
                }
            }
            if result.complete_respray {
                if let Some(key) = complete_respray(
                    garage,
                    player_vehicle_slot,
                    resprays_are_free,
                    &mut script_entities,
                    &mut world_players,
                    player_info.as_deref_mut(),
                    &mut file_loader,
                    &mut general,
                    particles.as_deref_mut().map(|particles| {
                        particles.emitter_with_context(particle_systems.as_deref(),
                            camera.as_ref().map(|camera| camera.position))
                    }).as_mut(),
                    vehicle_colours.as_deref(),
                    timer.time_in_milliseconds(),
                ) {
                    garage_messages.push((key, -1, 4_000, -1));
                }
            }
            continue;
        }
        if garage.garage_type == EGarageType::Crusher {
            let pool_size = pools
                .as_ref()
                .map_or(crate::core::Pools::NUMVEHICLES, |pools| {
                    pools.vehicle_pool.get_size()
                });
            let slice = timer.frame_counter() as usize % CRUSHER_VEHICLE_TEST_SPAN;
            let start = pool_size * slice / CRUSHER_VEHICLE_TEST_SPAN;
            let end = pool_size * (slice + 1) / CRUSHER_VEHICLE_TEST_SPAN;
            let candidate_vehicle_slot = (start..end)
                .filter_map(|slot| {
                    script_entities
                        .vehicles
                        .iter()
                        .find(|(handle, vehicle)| {
                            CPoolHandle(**handle).index() == slot
                                && matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. })
                                && garage_entity_entirely_inside_3d(
                                    garage,
                                    &vehicle.base().physical.entity,
                                    0.0,
                                    &file_loader,
                                )
                        })
                        .map(|_| slot)
                })
                .last();
            let target_exists = garage.target_vehicle_slot.is_some_and(|slot| {
                script_entities
                    .vehicles
                    .keys()
                    .any(|handle| CPoolHandle(*handle).index() == slot)
            });
            let player_on_foot_inside = player_vehicle_slot.is_none()
                && world_players.focused().is_some_and(|player| {
                    garage_entity_entirely_inside_2d_strict(
                        garage,
                        &player.ped.physical.entity,
                        &file_loader,
                    )
                });
            let target_slot = garage.target_vehicle_slot;
            let result = garage.update_crusher(CrusherGarageUpdateContext {
                time_step: timer.time_step(),
                time_ms: timer.time_in_milliseconds(),
                candidate_vehicle_slot,
                target_exists,
                player_on_foot_inside,
                frame_counter: timer.frame_counter(),
            });

            if let Some(slot) = target_slot
                && let Some((_, mut vehicle)) = script_entities
                    .vehicles
                    .iter_mut()
                    .find(|(handle, _)| CPoolHandle(*handle).index() == slot)
            {
                let physical = &mut vehicle.base_mut().physical;
                if result.immobilize_target {
                    physical.entity.flags.uses_collision = false;
                    physical.flags.affected_by_gravity = false;
                    physical.move_speed = CVector::ZERO;
                } else if let Some(factor) = result.damp_target_velocity_by {
                    physical.move_speed = CVector::new(
                        physical.move_speed.x * factor,
                        physical.move_speed.y * factor,
                        physical.move_speed.z * factor,
                    );
                }
            }
            if let Some(slot) = result.crush_vehicle_slot {
                let handle = script_entities.vehicle_handle_for_slot(slot);
                let reward = script_entities
                    .vehicles
                    .iter()
                    .find(|(vehicle_handle, _)| CPoolHandle(**vehicle_handle).index() == slot)
                    .and_then(|(_, vehicle)| {
                        let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                            return None;
                        };
                        let monetary_value = car.handling_monetary_value;
                        Some(
                            (CRUSHER_MIN_REWARD
                                + monetary_value as f32
                                    * car.vehicle.vitality.health
                                    * CRUSHER_REWARD_COEFFICIENT)
                                .min(CRUSHER_MAX_REWARD) as i32,
                        )
                    })
                    .unwrap_or(CRUSHER_MIN_REWARD as i32);
                if let Some(info) = player_info.as_mut() {
                    info.m_nMoney = info.m_nMoney.saturating_add(reward);
                }
                if let Some(stats) = stats.as_mut() {
                    stats.cars_crushed = stats.cars_crushed.saturating_add(1);
                }
                crushed_car_id = Some(handle.unwrap_or(slot as i32));
                destroyed_slots.push(slot);
            }
            let (shake_x, shake_y) = if result.shake {
                (
                    f32::from(general.get_random_number() & 0x7f) * 0.0002,
                    f32::from(general.get_random_number() & 0x7f) * 0.0002,
                )
            } else {
                (0.0, 0.0)
            };
            apply_crusher_render_shake(garage, &mut file_loader, shake_x, shake_y);
            if result.damage_player
                && let Some(player) = world_players.focused_mut()
            {
                let armour_damage = player.vitality.armour.min(300.0);
                player.vitality.armour -= armour_damage;
                player.vitality.health = (player.vitality.health - (300.0 - armour_damage)).max(0.0);
                if player.vitality.health == 0.0 {
                    player.ped.set_dead(player.vitality, 
                        player.pool_handle,
                        timer.time_in_milliseconds(),
                        &mut events,
                    );
                }
            }
            continue;
        }
        if matches!(
            garage.garage_type,
            EGarageType::Bombshop1 | EGarageType::Bombshop2 | EGarageType::Bombshop3
        ) {
            let player_vehicle = player_vehicle_slot.and_then(|slot| {
                script_entities
                    .vehicles
                    .iter()
                    .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                    .map(|(_, vehicle)| vehicle)
            });
            let static_inside = world_players.focused().is_some_and(|player| {
                player.ped.ped_state == PedState::PED_DRIVING
                    && player.ped.objective != eObjective::OBJECTIVE_LEAVE_CAR
            }) && player_vehicle.is_some_and(|vehicle| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                    return false;
                };
                let position = car.vehicle.core.physical.entity.get_position();
                let speed = car.vehicle.core.physical.move_speed;
                position.x >= garage.x1
                    && position.x <= garage.x2
                    && position.y >= garage.y1
                    && position.y <= garage.y2
                    && speed.x.abs() <= 0.01
                    && speed.y.abs() <= 0.01
                    && speed.z.abs() <= 0.01
                    && speed.magnitude_sqr() <= 0.01_f32.powi(2)
                    && garage_entity_entirely_inside_3d(
                        garage,
                        &car.vehicle.core.physical.entity,
                        0.0,
                        &file_loader,
                    )
            });
            let player_outside = match player_vehicle {
                Some(vehicle) => garage_entity_entirely_outside_2d(
                    garage,
                    &vehicle.base().physical.entity,
                    0.0,
                    &file_loader,
                ),
                None => world_players.focused().is_none_or(|player| {
                    garage_entity_entirely_outside_2d(
                        garage,
                        &player.ped.physical.entity,
                        0.0,
                        &file_loader,
                    )
                }),
            };
            let result = garage.update_bombshop(BombshopGarageUpdateContext {
                time_step: timer.time_step(),
                time_ms: timer.time_in_milliseconds(),
                static_player_car_entirely_inside: static_inside,
                no_other_car_touching_player_car: !garage_any_other_car_touching(
                    garage,
                    player_vehicle_slot,
                    &script_entities,
                    &file_loader,
                ),
                player_car_already_has_bomb: player_vehicle.is_some_and(|vehicle| {
                    matches!(
                        vehicle,
                        crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }
                            if car.bomb_type
                                != crate::vehicles::Automobile::eBombType::CARBOMB_NONE
                    )
                }),
                player_can_afford: bombs_are_free
                    || player_info
                        .as_ref()
                        .is_some_and(|info| info.m_nMoney >= BOMB_PRICE),
                player_outside_garage: player_outside,
            });
            if result.player_controls_enabled.is_some() {
                aggregate.player_controls_enabled = result.player_controls_enabled;
            }
            if result.player_ignored_by_cops.is_some() {
                aggregate.player_ignored_by_cops = result.player_ignored_by_cops;
            }
            if let Some(message) = result.rejection_message {
                let mut key = [0_u8; 8];
                key[..message.len()].copy_from_slice(&message);
                garage_messages.push((key, -1, 4_000, -1));
            }
            if let Some(bomb_type) = result.install_bomb_type
                && let Some(handle) = player_vehicle_handle
                && let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) = script_entities
                    .vehicles
                    .get_mut(&script_entities.index, &handle)
            {
                car.bomb_type = script_bomb_type(bomb_type);
                car.bomb_rigger = world_players
                    .focused()
                    .map(|player| CPoolHandle(player.pool_handle).index());
                if !bombs_are_free && let Some(info) = player_info.as_mut() {
                    info.m_nMoney = info.m_nMoney.saturating_sub(BOMB_PRICE).max(0);
                }
                if let Some(stats) = stats.as_mut() {
                    stats.kgs_of_explosives_used = stats.kgs_of_explosives_used.saturating_add(10);
                }
                if bomb_type == 3
                    && let Some(mut player) = world_players.focused_mut()
                {
                    let _ = give_ped_weapon_with_info(
                        &mut player.ped,
                        eWeaponType::WEAPONTYPE_DETONATOR as i32,
                        1,
                        &weapon_info,
                    );
                    player.ped.weapons[eWeaponType::WEAPONTYPE_DETONATOR as usize].weapon_state =
                        crate::weapons::WeaponType::eWeaponState::WEAPONSTATE_READY;
                }
                if let (Some(text), Some(hud)) = (text.as_ref(), hud.as_mut()) {
                    let key = match garage.garage_type {
                        EGarageType::Bombshop1 if pads.get_pad(0).mode == 3 => "GA_6B",
                        EGarageType::Bombshop1 => "GA_6",
                        EGarageType::Bombshop2 if pads.get_pad(0).mode == 3 => "GA_7B",
                        EGarageType::Bombshop2 => "GA_7",
                        _ => "GA_8",
                    };
                    hud.set_help_message(&text.get(key), false);
                }
            }
            continue;
        }
        if matches!(
            garage.garage_type,
            EGarageType::CollectSpecificCars
                | EGarageType::CollectCars1
                | EGarageType::CollectCars2
                | EGarageType::CollectCars3
        ) {
            let player_vehicle = player_vehicle_slot.and_then(|slot| {
                script_entities
                    .vehicles
                    .iter()
                    .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                    .map(|(_, vehicle)| vehicle)
            });
            let player_vehicle_model =
                player_vehicle.map(|vehicle| i32::from(vehicle.base().physical.entity.model_index));
            let player_vehicle_position =
                player_vehicle.map(|vehicle| vehicle.base().physical.entity.get_position());
            let player_vehicle_speed = player_vehicle.map_or(0.0, |vehicle| {
                vehicle.base().physical.move_speed.magnitude()
            });
            let player_vehicle_is_mission = player_vehicle
                .is_some_and(|vehicle| vehicle.base().vehicle_created_by == MISSION_VEHICLE);

            let target_vehicle = garage.target_vehicle_slot.and_then(|slot| {
                script_entities
                    .vehicles
                    .iter()
                    .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                    .map(|(_, vehicle)| vehicle)
            });
            let target_exists = target_vehicle.is_some();
            let target_model =
                target_vehicle.map(|vehicle| i32::from(vehicle.base().physical.entity.model_index));
            let target_entirely_inside = target_vehicle.is_some_and(|vehicle| {
                garage_entity_entirely_inside_3d(
                    garage,
                    &vehicle.base().physical.entity,
                    0.0,
                    &file_loader,
                )
            });
            let no_other_car_touching_target = !garage_any_other_car_touching(
                garage,
                garage.target_vehicle_slot,
                &script_entities,
                &file_loader,
            );
            let player_entirely_outside_margin_2 = world_players.focused().is_none_or(|player| {
                garage_entity_entirely_outside_2d(
                    garage,
                    &player.ped.physical.entity,
                    2.0,
                    &file_loader,
                )
            });
            let target_condition_acceptable = target_vehicle.is_some_and(|vehicle| {
                if vehicle.base().physical.entity.status == eEntityStatus::STATUS_WRECKED {
                    return false;
                }
                match vehicle {
                    crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                        car.damage.get_engine_status()
                            <= crate::vehicles::DamageManager::eEngineStatus::ENGINE_STATUS_ON_FIRE
                                as u32
                            && car.fire_blow_up_timer == 0.0
                    }
                    crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
                    | crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. }
                    | crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => true,
                }
            });
            let (craig_needs_player_car, craig_has_collected_player_car) = player_vehicle_model
                .and_then(|model| CGarages::craig_car_index(garage.garage_type, model))
                .map_or((false, false), |(garage_index, car_index)| {
                    let collected = car_types_collected[garage_index] & (1_i32 << car_index) != 0;
                    (!collected, collected)
                });
            let context = CollectGarageUpdateContext {
                time_step: timer.time_step(),
                player_position,
                player_vehicle_slot,
                player_vehicle_model,
                player_vehicle_position,
                player_vehicle_speed,
                player_vehicle_is_mission,
                target_exists,
                target_model,
                target_entirely_inside,
                no_other_car_touching_target,
                player_entirely_outside_margin_2,
                target_condition_acceptable,
                craig_needs_player_car,
                craig_has_collected_player_car,
            };
            let result = if garage.garage_type == EGarageType::CollectSpecificCars {
                garage.update_collect_specific_cars(context)
            } else {
                garage.update_collect_cars(context)
            };
            if result.player_controls_enabled.is_some() {
                aggregate.player_controls_enabled = result.player_controls_enabled;
            }
            if result.player_ignored_by_cops.is_some() {
                aggregate.player_ignored_by_cops = result.player_ignored_by_cops;
            }
            if let Some((key, duration)) = result.message {
                garage_messages.push((key, -1, duration, -1));
            }
            if result.camera_directly_behind
                && let Some(camera) = camera.as_mut()
            {
                let heading = world_players
                    .focused()
                    .map(|player| player.heading_degrees())
                    .unwrap_or(0.0)
                    .to_radians();
                camera.set_camera_directly_behind_for_follow_ped(heading);
            }
            if let Some(slot) = result.destroy_vehicle_slot {
                destroyed_slots.push(slot);
                if let Some(model) = result.delivered_model {
                    collect_deliveries.push((garage.garage_type, model));
                }
            }
            continue;
        }
        if garage.garage_type == EGarageType::ForCarToComeOutOf {
            let garage_empty =
                garage_is_empty(garage, &script_entities, &world_players, &file_loader);
            garage.update_for_car_to_come_out_of(timer.time_step(), garage_empty);
            continue;
        }
        if !matches!(
            garage.garage_type,
            EGarageType::Mission
                | EGarageType::MissionKeepCar
                | EGarageType::MissionKeepCarRemainClosed
                | EGarageType::KeepsOpeningForSpecificCar
        ) {
            continue;
        }

        let target_vehicle = garage.target_vehicle_slot.and_then(|slot| {
            script_entities
                .vehicles
                .iter()
                .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                .map(|(_, vehicle)| vehicle)
        });
        let target_exists = target_vehicle.is_some();
        let target_entirely_inside = target_vehicle.is_some_and(|vehicle| {
            garage_entity_entirely_inside_3d(
                garage,
                &vehicle.base().physical.entity,
                0.0,
                &file_loader,
            )
        });
        let target_entirely_outside = target_vehicle.is_some_and(|vehicle| {
            garage_entity_entirely_outside_2d(
                garage,
                &vehicle.base().physical.entity,
                0.0,
                &file_loader,
            )
        });
        let no_other_car_touching_target = !garage_any_other_car_touching(
            garage,
            garage.target_vehicle_slot,
            &script_entities,
            &file_loader,
        );
        let no_car_touching_garage =
            !garage_any_other_car_touching(garage, None, &script_entities, &file_loader);
        let player_entirely_outside_margin_2 = world_players.focused().is_none_or(|player| {
            garage_entity_entirely_outside_2d(
                garage,
                &player.ped.physical.entity,
                2.0,
                &file_loader,
            )
        });
        let static_player_target_entirely_inside = player_vehicle_slot
            .zip(garage.target_vehicle_slot)
            .is_some_and(|(player_slot, target_slot)| player_slot == target_slot)
            && world_players.focused().is_some_and(|player| {
                player.ped.ped_state == PedState::PED_DRIVING
                    && player.ped.objective != eObjective::OBJECTIVE_LEAVE_CAR
            })
            && player_vehicle_slot
                .and_then(|slot| {
                    script_entities
                        .vehicles
                        .iter()
                        .find(|(handle, _)| CPoolHandle(**handle).index() == slot)
                        .map(|(_, vehicle)| vehicle)
                })
                .is_some_and(|vehicle| {
                    let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                        return false;
                    };
                    let position = car.vehicle.core.physical.entity.get_position();
                    let speed = car.vehicle.core.physical.move_speed;
                    position.x >= garage.x1
                        && position.x <= garage.x2
                        && position.y >= garage.y1
                        && position.y <= garage.y2
                        && speed.x.abs() <= 0.01
                        && speed.y.abs() <= 0.01
                        && speed.z.abs() <= 0.01
                        && speed.magnitude_sqr() <= 0.01_f32.powi(2)
                        && garage_entity_entirely_inside_3d(
                            garage,
                            &car.vehicle.core.physical.entity,
                            0.0,
                            &file_loader,
                        )
                });

        let result = garage.update_mission_garage(MissionGarageUpdateContext {
            time_step: timer.time_step(),
            time_ms: timer.time_in_milliseconds(),
            frame_counter: timer.frame_counter(),
            player_position,
            player_vehicle_slot,
            player_vehicle_position,
            target_exists,
            target_entirely_inside,
            target_entirely_outside,
            player_entirely_outside_margin_2,
            no_car_touching_garage,
            no_other_car_touching_target,
            static_player_target_entirely_inside,
            no_car_blocking_door: !garage_any_car_blocking_door(
                garage,
                &script_entities,
                &file_loader,
            ),
        });
        if result.player_controls_enabled.is_some() {
            aggregate.player_controls_enabled = result.player_controls_enabled;
        }
        if result.player_ignored_by_cops.is_some() {
            aggregate.player_ignored_by_cops = result.player_ignored_by_cops;
        }
        if let Some(slot) = result.destroy_vehicle_slot {
            destroyed_slots.push(slot);
        }
    }
    if timer.frame_counter() & 0x0f == 0x0c {
        garages.garage_to_be_tidied =
            (garages.garage_to_be_tidied + 1) % crate::control::Garages::NUM_GARAGES;
        let garage = garages.garages[garages.garage_to_be_tidied];
        if garage.garage_type != EGarageType::None {
            let camera_position = camera
                .as_ref()
                .map(|camera| camera.position)
                .unwrap_or(player_position);
            let center_x = (garage.x1 + garage.x2) * 0.5;
            let center_y = (garage.y1 + garage.y2) * 0.5;
            let far = (camera_position.x - center_x).abs() > 40.0
                || (camera_position.y - center_y).abs() > 40.0;
            destroyed_slots.extend(garage_tidy_vehicle_slots(
                &garage,
                far,
                &script_entities,
                &file_loader,
            ));
        }
    }
    garages.cam_should_be_outside = cam_should_be_outside;

    if let Some(pools) = pools.as_mut() {
        for garage_index in hideout_blocker_removal_requests {
            let garage = garages.garages[garage_index];
            remove_hideout_blocking_cars_not_inside(
                &garage,
                &mut script_entities,
                pools,
                &file_loader,
            );
        }
        for (garage_index, garage_type) in hideout_store_requests {
            let garage = garages.garages[garage_index];
            match garage_type {
                EGarageType::HideoutOne => store_and_remove_hideout_cars(
                    &garage,
                    &mut garages.cars_in_safe_house1,
                    &mut script_entities,
                    pools,
                    &mut world_players,
                    &file_loader,
                ),
                EGarageType::HideoutTwo => store_and_remove_hideout_cars(
                    &garage,
                    &mut garages.cars_in_safe_house2,
                    &mut script_entities,
                    pools,
                    &mut world_players,
                    &file_loader,
                ),
                EGarageType::HideoutThree => store_and_remove_hideout_cars(
                    &garage,
                    &mut garages.cars_in_safe_house3,
                    &mut script_entities,
                    pools,
                    &mut world_players,
                    &file_loader,
                ),
                _ => {}
            }
        }
        if let Some(streaming) = streaming.as_mut() {
            let player_ped_slot = world_players
                .focused()
                .map(|player| CPoolHandle(player.pool_handle).index());
            for (garage_index, garage_type) in hideout_restore_requests {
                let restored_all = match garage_type {
                    EGarageType::HideoutOne => restore_hideout_cars(
                        &mut garages.cars_in_safe_house1,
                        &mut script_entities,
                        pools,
                        streaming,
                        &file_loader,
                        handling_manager.as_deref(),
                        &mut general,
                        timer.time_in_milliseconds(),
                        player_ped_slot,
                        taxi_lights.all_enabled,
                    ),
                    EGarageType::HideoutTwo => restore_hideout_cars(
                        &mut garages.cars_in_safe_house2,
                        &mut script_entities,
                        pools,
                        streaming,
                        &file_loader,
                        handling_manager.as_deref(),
                        &mut general,
                        timer.time_in_milliseconds(),
                        player_ped_slot,
                        taxi_lights.all_enabled,
                    ),
                    EGarageType::HideoutThree => restore_hideout_cars(
                        &mut garages.cars_in_safe_house3,
                        &mut script_entities,
                        pools,
                        streaming,
                        &file_loader,
                        handling_manager.as_deref(),
                        &mut general,
                        timer.time_in_milliseconds(),
                        player_ped_slot,
                        taxi_lights.all_enabled,
                    ),
                    _ => false,
                };
                if restored_all {
                    garages.garages[garage_index].garage_state = EGarageState::Opening;
                }
            }
        }
    }
    if hideout_full_message {
        garages.last_time_help_message = timer.time_in_milliseconds();
        if let (Some(text), Some(hud)) = (text.as_ref(), hud.as_mut()) {
            hud.set_help_message(&text.get("GA_21"), false);
        }
    }

    // The remaining two branches are controlled exclusively by SCM.
    garages.update_script_controlled_doors(timer.time_step());
    if let Some(handle) = crushed_car_id {
        garages.crushed_car_id = handle;
    }
    for (garage_type, model) in collect_deliveries {
        if garage_type == EGarageType::CollectSpecificCars {
            let reward = if model == crate::modelinfo::ModelIndices::MI_POLICE as i32 {
                let reward = REWARD_FOR_FIRST_POLICE_CAR
                    * (MAX_POLICE_CARS_TO_COLLECT - garages.police_cars_collected)
                    / MAX_POLICE_CARS_TO_COLLECT;
                garages.police_cars_collected = garages.police_cars_collected.saturating_add(1);
                reward
            } else if model == crate::modelinfo::ModelIndices::MI_SECURICA as i32 {
                let reward = REWARD_FOR_FIRST_BANK_VAN
                    * (MAX_BANK_VANS_TO_COLLECT - garages.bank_vans_collected)
                    / MAX_BANK_VANS_TO_COLLECT;
                garages.bank_vans_collected = garages.bank_vans_collected.saturating_add(1);
                reward
            } else {
                0
            };
            if reward > 0 {
                if let Some(info) = player_info.as_mut() {
                    info.m_nMoney = info.m_nMoney.saturating_add(reward);
                }
                garage_messages.push((garage_text_key(b"GA_10"), reward, 4_000, -1));
            } else {
                garage_messages.push((garage_text_key(b"GA_11"), -1, 4_000, -1));
            }
        } else if let Some(all_collected) =
            garages.mark_this_car_as_collected_for_craig(garage_type, model)
        {
            if let Some(info) = player_info.as_mut() {
                info.m_nMoney = info.m_nMoney.saturating_add(IMPORT_REWARD);
                if all_collected {
                    info.m_nMoney = info.m_nMoney.saturating_add(IMPORT_ALLCARS_REWARD);
                }
            }
            garage_messages.push((
                garage_text_key(if all_collected { b"GA_14" } else { b"GA_13" }),
                -1,
                5_000,
                -1,
            ));
        }
    }
    for (key, number1, duration, number2) in garage_messages {
        garages.trigger_message(
            &key,
            number1,
            duration,
            number2,
            timer.time_in_milliseconds(),
        );
    }

    if let Some(enabled) = aggregate.player_controls_enabled {
        if enabled {
            pads.get_pad_mut(0)
                .set_enable_player_controls(PLAYERCONTROL_GARAGE);
        } else {
            pads.get_pad_mut(0)
                .set_disable_player_controls(PLAYERCONTROL_GARAGE);
        }
    }
    if let Some(ignored) = aggregate.player_ignored_by_cops
        && let Some(mut player) = world_players.focused_mut()
    {
        player.wanted.m_bIgnoredByCops = ignored;
    }
    if let Some(pools) = pools.as_mut() {
        for slot in destroyed_slots {
            if let Some(handle) = script_entities.vehicle_handle_for_slot(slot) {
                destroy_script_vehicle_and_occupants(&mut script_entities, pools, handle);
            }
            // re3 registered every garage target as a CEntity reference, so
            // deleting a vehicle nulls every outstanding pointer to it.
            for garage in garages.garages.iter_mut().take(count) {
                if garage.target_vehicle_slot == Some(slot) {
                    garage.target_vehicle_slot = None;
                }
            }
        }
    }
    if let Some(camera) = camera.as_mut() {
        apply_garage_camera_override(camera, &garages, player_position, &file_loader);
    }
    apply_garage_door_transforms(&garages, &mut file_loader, count, &mut script_entities);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn complete_respray(
    garage: &CGarage,
    player_vehicle_slot: Option<usize>,
    resprays_are_free: bool,
    entities: &mut ScriptEntityWorld,
    world_players: &mut CWorldPlayers,
    player_info: Option<&mut CPlayerInfo>,
    file_loader: &mut CFileLoader,
    general: &mut CGeneral,
    particles: Option<&mut ParticleEmitter<'_>>,
    vehicle_colours: Option<&CVehicleColours>,
    _time_ms: u32,
) -> Option<[u8; 8]> {
    let vehicle_slot = player_vehicle_slot?;
    let vehicle_handle = entities.vehicle_handle_for_slot(vehicle_slot)?;
    let (model_id, old_colours, fixed_colour, old_health) = {
        let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = entities.vehicles.get(&entities.index, &vehicle_handle)?
        else {
            return None;
        };
        (
            car.vehicle.core.physical.entity.model_index.max(0) as usize,
            (car.vehicle.appearance.current_colour1, car.vehicle.appearance.current_colour2),
            car.fixed_colour,
            car.vehicle.vitality.health,
        )
    };
    let wanted_before = world_players
        .focused()
        .map_or(0, |player| player.wanted.m_nWantedLevel);
    if let Some(mut player) = world_players.focused_mut() {
        player.wanted.Initialise();
        player.wanted.m_bIgnoredByCops = false;
    }

    let mut chosen = old_colours;
    let mut changed_colour = false;
    if !fixed_colour {
        if let Some(ModelInfoSlot::Vehicle(model)) =
            file_loader.model_info.get_model_info_mut(model_id)
        {
            for attempt in 0..10 {
                chosen = model.choose_vehicle_colour(Some(old_colours));
                if chosen != old_colours {
                    changed_colour = true;
                    let _ = attempt;
                    break;
                }
            }
        }
    }

    let other_car_touching =
        garage_any_other_car_touching(garage, Some(vehicle_slot), entities, file_loader);
    let player_ped_slot = world_players
        .focused()
        .map(|player| CPoolHandle(player.pool_handle).index());
    let other_ped_touching = entities.peds.iter().any(|(handle, ped)| {
        player_ped_slot != Some(CPoolHandle(*handle).index())
            && garage_entity_touching_3d(garage, &ped.ped.physical.entity, file_loader)
    });
    {
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } =
            entities.vehicles.get_mut(&entities.index, &vehicle_handle)?
        else {
            return None;
        };
        car.vehicle.vitality.health = 1_000.0;
        car.fix();
        if car.vehicle.core.physical.entity.get_up().z < 0.0 {
            let entity = &mut car.vehicle.core.physical.entity;
            entity.matrix.up = CVector::new(
                -entity.matrix.up.x,
                -entity.matrix.up.y,
                -entity.matrix.up.z,
            );
            entity.matrix.right = CVector::new(
                -entity.matrix.right.x,
                -entity.matrix.right.y,
                -entity.matrix.right.z,
            );
        }
        car.vehicle.appearance.current_colour1 = chosen.0;
        car.vehicle.appearance.current_colour2 = chosen.1;

        if !other_car_touching && !other_ped_touching {
            let original = car.vehicle.core.physical.entity.get_position();
            let offset = CVector::new(
                (garage.x1 + garage.x2) * 0.5 - original.x,
                (garage.y1 + garage.y2) * 0.5 - original.y,
                0.0,
            );
            let distance = offset.magnitude();
            let mut centered = original;
            if distance < 0.75 {
                centered.x = (garage.x1 + garage.x2) * 0.5;
                centered.y = (garage.y1 + garage.y2) * 0.5;
            } else if distance > 0.0 {
                centered.x += offset.x * 0.75 / distance;
                centered.y += offset.y * 0.75 / distance;
            }
            car.vehicle.core.physical.entity.set_position(centered);
            if !garage_entity_entirely_inside_3d(
                garage,
                &car.vehicle.core.physical.entity,
                0.1,
                file_loader,
            ) {
                car.vehicle.core.physical.entity.set_position(original);
            }
        }
    }

    if changed_colour
        && let (Some(particles), Some(table)) = (particles, vehicle_colours)
        && let Some(colour) = table.vehicle_colour_table.get(chosen.0 as usize)
    {
        let colour = crate::renderer::Particle::ParticleColour {
            red: colour.red,
            green: colour.green,
            blue: colour.blue,
            alpha: colour.alpha,
        };
        for _ in 0..200 {
            particles.add_particle_coloured(
                crate::renderer::ParticleType::PARTICLE_GARAGEPAINT_SPRAY,
                CVector::new(
                    general.get_random_number_in_range_f32(garage.x1 + 0.5, garage.x2 - 0.5),
                    general.get_random_number_in_range_f32(garage.y1 + 0.5, garage.y2 - 0.5),
                    general
                        .get_random_number_in_range_f32(garage.door1_z - 3.0, garage.door1_z + 1.0),
                ),
                CVector::ZERO,
                None,
                0.0,
                colour,
                0,
                0,
                0,
                0,
            );
        }
    }

    let take_money = wanted_before != 0 || old_health < 970.0;
    if take_money {
        if !resprays_are_free && let Some(info) = player_info {
            info.m_nMoney = info.m_nMoney.saturating_sub(RESPRAY_PRICE).max(0);
        }
        let mut key = [0_u8; 8];
        key[..4].copy_from_slice(b"GA_2");
        Some(key)
    } else if changed_colour {
        let text = if general.get_random_true_false() {
            b"GA_15".as_slice()
        } else {
            b"GA_16".as_slice()
        };
        let mut key = [0_u8; 8];
        key[..text.len()].copy_from_slice(text);
        Some(key)
    } else {
        None
    }
}

/// The fixed outside viewpoint selected by the garage branch in
/// `CCamera::CamControl`. This updates the live camera transform without
/// claiming SCM ownership, so normal follow mode resumes on the first frame
/// after the player leaves the garage.
pub(crate) fn stored_car_from_script_vehicle(vehicle: crate::control::ScriptEntities::ScriptVehicleRef<'_>) -> CStoredCar {
    let base = vehicle.base();
    let entity_flags = base.physical.entity.flags;
    let mut flags = 0;
    if entity_flags.bullet_proof {
        flags |= CStoredCar::FLAG_BULLETPROOF;
    }
    if entity_flags.fire_proof {
        flags |= CStoredCar::FLAG_FIREPROOF;
    }
    if entity_flags.explosion_proof {
        flags |= CStoredCar::FLAG_EXPLOSIONPROOF;
    }
    if entity_flags.collision_proof {
        flags |= CStoredCar::FLAG_COLLISIONPROOF;
    }
    if entity_flags.melee_proof {
        flags |= CStoredCar::FLAG_MELEEPROOF;
    }
    CStoredCar {
        model_index: i32::from(base.physical.entity.model_index),
        position: base.physical.entity.get_position(),
        angle: base.physical.entity.get_forward(),
        flags,
        primary_color: base.appearance.current_colour1 as i8,
        secondary_color: base.appearance.current_colour2 as i8,
        radio_station: base.audio.radio_station as i8,
        variation_a: base.appearance.extras[0],
        variation_b: base.appearance.extras[1],
        car_bomb_type: match vehicle {
            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => car.bomb_type as i8,
            crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }
            | crate::control::ScriptEntities::ScriptVehicleRef::Heli { .. }
            | crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. } => 0,
        },
    }
}

pub(crate) fn script_vehicle_can_be_deleted(
    handle: i32,
    vehicle: crate::control::ScriptEntities::ScriptVehicleRef<'_>,
    entities: &ScriptEntityWorld,
) -> bool {
    let base = vehicle.base();
    script_vehicle_base_can_be_deleted(entities, handle, &base)
}

pub(crate) fn cancel_player_entering_removed_car(
    world_players: &mut dyn WorldPlayerAccess,
    removed_vehicle: Entity,
) {
    let player_index = world_players.player_in_focus() as i32;
    let is_related_to_removed_vehicle = world_players
        .vehicle_relation(player_index)
        .is_some_and(|relation| relation.vehicle == removed_vehicle);
    let Some(player) = world_players.focused_mut() else {
        return;
    };
    if is_related_to_removed_vehicle {
        player.ped.flags.cancel_entering_car = true;
    }
    if matches!(
        player.ped.objective,
        eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
    ) {
        player.ped.objective = eObjective::OBJECTIVE_NONE;
        player.ped.car_in_objective = None;
    }
}

pub(crate) fn store_and_remove_hideout_cars(
    garage: &CGarage,
    stored_cars: &mut [CStoredCar; NUM_GARAGE_STORED_CARS],
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    world_players: &mut dyn WorldPlayerAccess,
    file_loader: &CFileLoader,
) {
    stored_cars.fill(CStoredCar::init());
    let mut handles = entities.vehicles.keys().copied().collect::<Vec<_>>();
    handles.sort_unstable_by_key(|handle| std::cmp::Reverse(CPoolHandle(*handle).index()));
    let mut stored_count = 0;
    for handle in handles {
        let Some(vehicle) = entities.vehicles.get(&entities.index, &handle) else {
            continue;
        };
        let base = vehicle.base();
        let position = base.physical.entity.get_position();
        if position.x <= garage.x1
            || position.x >= garage.x2
            || position.y <= garage.y1
            || position.y >= garage.y2
            || position.z <= garage.z1
            || position.z >= garage.z2
            || base.vehicle_created_by == MISSION_VEHICLE
        {
            continue;
        }
        if stored_count < NUM_GARAGE_STORED_CARS
            && !garage_entity_has_sphere_way_outside(
                garage,
                &base.physical.entity,
                1.0,
                file_loader,
            )
        {
            stored_cars[stored_count] = stored_car_from_script_vehicle(vehicle);
            stored_count += 1;
        }
        let vehicle_entity = entities.vehicle_entity(handle);
        if let Some(vehicle_entity) = vehicle_entity {
            cancel_player_entering_removed_car(world_players, vehicle_entity);
        }
        destroy_script_vehicle_and_occupants(entities, pools, handle);
    }
}

pub(crate) fn restore_hideout_cars(
    stored_cars: &mut [CStoredCar; NUM_GARAGE_STORED_CARS],
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    streaming: &mut CStreaming,
    file_loader: &CFileLoader,
    handling_manager: Option<&cHandlingDataMgr>,
    general: &mut CGeneral,
    time_ms: u32,
    player_ped_slot: Option<usize>,
    all_taxi_lights: bool,
) -> bool {
    for stored in stored_cars.iter_mut().filter(|stored| stored.has_car()) {
        let Ok(model_index) = usize::try_from(stored.model_index) else {
            continue;
        };
        streaming.request_model(model_index, STREAMFLAGS_DEPENDENCY);
        if !streaming.has_model_loaded(model_index) {
            continue;
        }
        if entities
            .restore_stored_car(
                pools,
                *stored,
                time_ms,
                file_loader.model_info.get_model_info(model_index),
                handling_manager,
                general,
                player_ped_slot,
                all_taxi_lights,
            )
            .is_some()
        {
            stored.clear();
        }
    }
    stored_cars.iter().all(|stored| !stored.has_car())
}

pub(crate) fn remove_hideout_blocking_cars_not_inside(
    garage: &CGarage,
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    file_loader: &CFileLoader,
) {
    let removable = entities
        .vehicles
        .iter()
        .filter_map(|(&handle, vehicle)| {
            let base = vehicle.base();
            let position = base.physical.entity.get_position();
            (garage_entity_touching_3d(garage, &base.physical.entity, file_loader)
                && (position.x < garage.x1
                    || position.x > garage.x2
                    || position.y < garage.y1
                    || position.y > garage.y2
                    || position.z < garage.z1
                    || position.z > garage.z2)
                && !base.flags.is_locked
                && script_vehicle_can_be_deleted(handle, vehicle, entities))
            .then_some(handle)
        })
        .collect::<Vec<_>>();
    for handle in removable {
        destroy_script_vehicle_and_occupants(entities, pools, handle);
    }
}

pub(crate) fn garage_tidy_vehicle_slots(
    garage: &CGarage,
    far: bool,
    entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
) -> Vec<usize> {
    entities
        .vehicles
        .iter()
        .filter_map(|(&handle, vehicle)| {
            let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                return None;
            };
            let base = &car.vehicle;
            let entity = &base.physical.entity;
            let position = entity.get_position();
            let remove = if far {
                position.x > garage.x1
                    && position.x < garage.x2
                    && position.y > garage.y1
                    && position.y < garage.y2
                    && position.z > garage.z1
                    && position.z < garage.z2
                    && (entity.status == eEntityStatus::STATUS_WRECKED || entity.get_up().z < 0.5)
            } else {
                entity.status == eEntityStatus::STATUS_WRECKED
                    && garage_entity_touching_3d(garage, entity, file_loader)
                    && (garage.garage_state == EGarageState::FullyClosed
                        || garage_entity_has_sphere_way_outside(garage, entity, 0.0, file_loader))
            };
            remove.then_some(CPoolHandle(handle).index())
        })
        .collect()
}

pub(crate) fn garage_is_empty(
    garage: &CGarage,
    entities: &ScriptEntityWorld,
    world_players: &CWorldPlayers,
    file_loader: &CFileLoader,
) -> bool {
    !entities.vehicles.values().any(|vehicle| {
        garage_bound_sphere_intersects(garage, &vehicle.base().physical.entity, file_loader)
    }) && !entities
        .peds
        .values()
        .any(|ped| garage_bound_sphere_intersects(garage, &ped.ped.physical.entity, file_loader))
        && !(0..world_players.slot_count()).any(|index| {
            world_players.player(index as i32).is_some_and(|player| {
                garage_bound_sphere_intersects(garage, &player.ped.physical.entity, file_loader)
            })
        })
}

pub(crate) fn garage_any_other_car_touching(
    garage: &CGarage,
    exception_slot: Option<usize>,
    entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
) -> bool {
    entities.vehicles.iter().any(|(handle, vehicle)| {
        let slot = CPoolHandle(*handle).index();
        exception_slot != Some(slot)
            && garage_entity_touching_3d(garage, &vehicle.base().physical.entity, file_loader)
    })
}

pub(crate) fn garage_any_car_blocking_door(
    garage: &CGarage,
    entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
) -> bool {
    entities.vehicles.values().any(|vehicle| {
        let entity = &vehicle.base().physical.entity;
        if !garage_entity_touching_3d(garage, entity, file_loader) {
            return false;
        }
        garage_col_model(entity, file_loader).is_some_and(|col_model| {
            col_model.spheres.iter().any(|sphere| {
                let center = entity.matrix.transform_point(CVector::new(
                    sphere.center.x,
                    sphere.center.y,
                    sphere.center.z,
                ));
                let radius = sphere.radius;
                center.x + radius < garage.x1
                    || center.x - radius > garage.x2
                    || center.y + radius < garage.y1
                    || center.y - radius > garage.y2
                    || center.z + radius < garage.z1
                    || center.z - radius > garage.z2
            })
        })
    })
}

pub(crate) fn destroy_script_vehicle_and_occupants(
    entities: &mut ScriptEntityWorld,
    pools: &mut CPools,
    vehicle_handle: i32,
) {
    let occupant_handles = entities
        .vehicle_entity(vehicle_handle)
        .map(|vehicle_entity| {
            entities
                .script_occupant_handles(vehicle_entity)
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for handle in occupant_handles {
        entities.remove_char(pools, handle);
    }
    entities.remove_car(pools, vehicle_handle);
}
