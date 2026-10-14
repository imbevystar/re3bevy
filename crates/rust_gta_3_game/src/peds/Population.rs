//! re3 reference: references/re-GTA/src/peds/Population.cpp
//! re3 reference: references/re-GTA/src/peds/Population.h
//!
//! `CPopulation::Initialise` state and `PEDGRP.DAT` loading.

use std::io;
use crate::{objects::Object::CObject, collision::Collision::col_bound_spheres_overlap};

use bevy::prelude::Resource;
use rust_gta_3_assets::install::FileMgr::CFileMgr;
use rust_gta_3_core::core::{Coordinates::CVector, GameState::ELevelName};

use crate::peds::Ped::CPed;
use crate::peds::PedType::CPedTypeRuntime;
use crate::{modelinfo::ModelInfo::CModelInfo, peds::PedType::ePedType};

pub const NUMPEDGROUPS: usize = 31;
pub const NUMMODELSPERPEDGROUP: usize = 8;
pub const DEFAULT_MAX_NUMBER_OF_PEDS: i32 = 25;
pub const MIN_CREATION_DIST: f32 = 40.0;
pub const CREATION_RANGE: f32 = 10.0;
pub const OFFSCREEN_CREATION_MULT: f32 = 0.5;
pub const PED_REMOVE_DIST: f32 = 51.0;
pub const PED_REMOVE_DIST_SPECIAL: f32 = 65.0;

fn refresh_population_totals(population: &mut CPopulation) {
    population.total_civ_peds = population.num_civ_male + population.num_civ_female;
    population.total_gang_peds = population.gang_counts.iter().copied().sum();
    population.total_peds = population.num_dummy
        + population.num_emergency
        + population.num_cop
        + population.total_gang_peds
        + population.total_civ_peds;
}

pub(crate) fn add_ped_to_population_counts(population: &mut CPopulation, ped_type: ePedType) {
    use ePedType::*;

    match ped_type {
        PEDTYPE_PLAYER1 | PEDTYPE_PLAYER2 | PEDTYPE_PLAYER3 | PEDTYPE_PLAYER4 | PEDTYPE_UNUSED1
        | PEDTYPE_SPECIAL => return,
        PEDTYPE_CIVMALE => {
            population.num_civ_male = population.num_civ_male.saturating_add(1);
        }
        PEDTYPE_CIVFEMALE | PEDTYPE_PROSTITUTE => {
            population.num_civ_female = population.num_civ_female.saturating_add(1);
        }
        PEDTYPE_CRIMINAL => population.num_civ_male = population.num_civ_male.saturating_add(1),
        PEDTYPE_COP => population.num_cop = population.num_cop.saturating_add(1),
        PEDTYPE_EMERGENCY | PEDTYPE_FIREMAN => {
            population.num_emergency = population.num_emergency.saturating_add(1);
        }
        PEDTYPE_GANG1 | PEDTYPE_GANG2 | PEDTYPE_GANG3 | PEDTYPE_GANG4 | PEDTYPE_GANG5
        | PEDTYPE_GANG6 | PEDTYPE_GANG7 | PEDTYPE_GANG8 | PEDTYPE_GANG9 => {
            let index = (ped_type as usize).saturating_sub(PEDTYPE_GANG1 as usize);
            if let Some(count) = population.gang_counts.get_mut(index) {
                *count = count.saturating_add(1);
            }
        }
        _ => population.num_dummy = population.num_dummy.saturating_add(1),
    }
    refresh_population_totals(population);
}

pub(crate) fn remove_ped_from_population_counts(population: &mut CPopulation, ped_type: ePedType) {
    use ePedType::*;

    match ped_type {
        PEDTYPE_PLAYER1 | PEDTYPE_PLAYER2 | PEDTYPE_PLAYER3 | PEDTYPE_PLAYER4 | PEDTYPE_UNUSED1
        | PEDTYPE_SPECIAL => return,
        PEDTYPE_CIVMALE | PEDTYPE_CRIMINAL => {
            population.num_civ_male = population.num_civ_male.saturating_sub(1);
        }
        PEDTYPE_CIVFEMALE | PEDTYPE_PROSTITUTE => {
            population.num_civ_female = population.num_civ_female.saturating_sub(1);
        }
        PEDTYPE_COP => population.num_cop = population.num_cop.saturating_sub(1),
        PEDTYPE_EMERGENCY | PEDTYPE_FIREMAN => {
            population.num_emergency = population.num_emergency.saturating_sub(1);
        }
        PEDTYPE_GANG1 | PEDTYPE_GANG2 | PEDTYPE_GANG3 | PEDTYPE_GANG4 | PEDTYPE_GANG5
        | PEDTYPE_GANG6 | PEDTYPE_GANG7 | PEDTYPE_GANG8 | PEDTYPE_GANG9 => {
            let index = (ped_type as usize).saturating_sub(PEDTYPE_GANG1 as usize);
            if let Some(count) = population.gang_counts.get_mut(index) {
                *count = count.saturating_sub(1);
            }
        }
        _ => population.num_dummy = population.num_dummy.saturating_sub(1),
    }
    refresh_population_totals(population);
}

/// Keep re3's constructor/destructor population counters derived from the ECS
/// entity set. This makes mission peds and vehicle occupants participate in
/// the same police/gang/civilian budgets without duplicating lifetime hooks in
/// every SCM and traffic creation path.
pub(crate) fn refresh_population_counts_from_entities(
    mut population: ResMut<CPopulation>,
    peds: Query<&CPed, With<crate::control::ScriptEntities::ScriptPedIdentity>>,
) {
    population.num_civ_male = 0;
    population.num_civ_female = 0;
    population.num_cop = 0;
    population.num_emergency = 0;
    population.gang_counts = [0; 9];
    population.num_dummy = 0;
    population.total_peds = 0;
    population.total_gang_peds = 0;
    population.total_civ_peds = 0;

    population.total_mission_peds = 0;

    for entity in &peds {
        add_ped_to_population_counts(&mut population, entity.ped_type);
        if entity.char_created_by == MISSION_CHAR {
            population.total_mission_peds += 1;
        }
    }
    refresh_population_totals(&mut population);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegenerationPoint {
    pub src_level: ELevelName,
    pub dest_level: ELevelName,
    pub min: CVector,
    pub max: CVector,
    pub dest_pos_a: CVector,
    pub dest_pos_b: CVector,
    pub src_pos_a: CVector,
    pub src_pos_b: CVector,
}

pub const SAFE_ZONES: [RegenerationPoint; 8] = [
    regeneration(
        ELevelName::Industrial,
        ELevelName::Commercial,
        400.0,
        814.0,
        -954.0,
        -903.0,
        30.0,
        100.0,
        [790.0, -917.0, 39.0],
        [775.0, -921.0, 39.0],
        [424.0, -942.0, 38.0],
        [439.0, -938.0, 38.0],
    ),
    regeneration(
        ELevelName::Industrial,
        ELevelName::Commercial,
        555.0,
        711.0,
        118.0,
        186.0,
        -30.0,
        -10.0,
        [698.0, 182.0, -20.0],
        [681.0, 178.0, -20.0],
        [586.0, 144.0, -20.0],
        [577.0, 135.0, -20.0],
    ),
    regeneration(
        ELevelName::Industrial,
        ELevelName::Commercial,
        626.0,
        744.0,
        -124.0,
        -87.0,
        -20.0,
        -6.0,
        [736.0, -117.0, -13.0],
        [730.0, -115.0, -13.0],
        [635.0, -93.0, -12.5],
        [650.0, -89.0, -12.5],
    ),
    regeneration(
        ELevelName::Industrial,
        ELevelName::Commercial,
        645.0,
        734.0,
        -780.0,
        -750.0,
        -25.0,
        -6.0,
        [729.0, -764.0, -18.0],
        [720.0, -769.0, -17.0],
        [652.0, -774.0, -10.5],
        [659.0, -770.0, -10.5],
    ),
    regeneration(
        ELevelName::Commercial,
        ELevelName::Suburban,
        -532.0,
        -136.0,
        -668.0,
        -599.0,
        34.0,
        60.0,
        [-172.0, -619.0, 44.0],
        [-183.0, -623.0, 44.0],
        [-511.0, -645.0, 41.0],
        [-493.0, -639.0, 41.5],
    ),
    regeneration(
        ELevelName::Commercial,
        ELevelName::Suburban,
        -325.0,
        -175.0,
        27.0,
        75.0,
        -30.0,
        -10.0,
        [-185.0, 40.8, -20.5],
        [-202.0, 37.0, -20.5],
        [-315.0, 65.5, -20.5],
        [-306.0, 62.4, -20.5],
    ),
    regeneration(
        ELevelName::Commercial,
        ELevelName::Suburban,
        -410.0,
        -310.0,
        -1055.0,
        -1030.0,
        -20.0,
        -6.0,
        [-321.0, -1043.0, -13.2],
        [-328.0, -1045.0, -13.2],
        [-398.0, -1044.0, -13.5],
        [-390.0, -1040.5, -13.5],
    ),
    regeneration(
        ELevelName::Commercial,
        ELevelName::Suburban,
        -425.0,
        -280.0,
        -471.0,
        -447.0,
        -20.0,
        -5.0,
        [-292.0, -457.0, -11.6],
        [-310.0, -461.0, -11.6],
        [-413.0, -461.0, -11.5],
        [-399.0, -457.0, -11.3],
    ),
];

const fn regeneration(
    src_level: ELevelName,
    dest_level: ELevelName,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    z1: f32,
    z2: f32,
    dest_a: [f32; 3],
    dest_b: [f32; 3],
    src_a: [f32; 3],
    src_b: [f32; 3],
) -> RegenerationPoint {
    RegenerationPoint {
        src_level,
        dest_level,
        min: CVector::new(x1, y1, z1),
        max: CVector::new(x2, y2, z2),
        dest_pos_a: CVector::new(dest_a[0], dest_a[1], dest_a[2]),
        dest_pos_b: CVector::new(dest_b[0], dest_b[1], dest_b[2]),
        src_pos_a: CVector::new(src_a[0], src_a[1], src_a[2]),
        src_pos_b: CVector::new(src_b[0], src_b[1], src_b[2]),
    }
}

#[cfg(test)]
mod zone_time_tests {
    use super::*;

    #[test]
    fn native_zone_density_and_threshold_blend_truncates_at_the_authored_hour() {
        let day = ZonePedInfo { car_density: 10, ped_density: 10,
            car_threshold: [10; 6], gang_density: [10; 9], ped_group: 1, ..Default::default() };
        let night = ZonePedInfo { car_density: 0, ped_density: 0,
            car_threshold: [0; 6], gang_density: [0; 9], ped_group: 2, ..Default::default() };
        let dawn = interpolate_zone_info(day, night, 6);
        assert_eq!(dawn.car_density, 3);
        assert_eq!(dawn.ped_density, 3);
        assert_eq!(dawn.car_threshold, [3; 6]);
        assert_eq!(dawn.gang_density, [3; 9]);
        assert_eq!(dawn.ped_group, 1);
        let dusk = interpolate_zone_info(day, night, 20);
        assert_eq!(dusk.car_density, 6);
        assert_eq!(dusk.ped_density, 6);
        assert_eq!(dusk.ped_group, 2);
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPopulation {
    pub ped_groups: Vec<PedGroup>,
    pub give_peds_weapons: bool,
    pub all_random_peds_this_type: i32,
    pub ped_density_multiplier: f32,
    pub total_mission_peds: u32,
    pub max_number_of_peds_in_use: i32,
    pub num_civ_male: u32,
    pub num_civ_female: u32,
    pub num_cop: u32,
    pub zone_change_has_happened: bool,
    pub num_emergency: u32,
    pub countdown_to_peds_at_start: i8,
    pub gang_counts: [u32; 9],
    pub num_dummy: u32,
    pub total_peds: u32,
    pub total_gang_peds: u32,
    pub total_civ_peds: u32,
    pub initial_zone_change: Option<(ELevelName, ELevelName)>,
    pub regeneration_point_a: CVector,
    pub regeneration_point_b: CVector,
    pub regeneration_front: CVector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedGroup {
    pub models: [i32; NUMMODELSPERPEDGROUP],
}

impl CPopulation {
    pub fn initialise(file_mgr: &mut CFileMgr, model_info: &CModelInfo) -> io::Result<Self> {
        let mut population = Self {
            ped_groups: load_ped_groups(file_mgr, model_info)?,
            give_peds_weapons: false,
            all_random_peds_this_type: -1,
            ped_density_multiplier: 1.0,
            total_mission_peds: 0,
            max_number_of_peds_in_use: DEFAULT_MAX_NUMBER_OF_PEDS,
            num_civ_male: 0,
            num_civ_female: 0,
            num_cop: 0,
            zone_change_has_happened: false,
            num_emergency: 0,
            countdown_to_peds_at_start: 2,
            gang_counts: [0; 9],
            num_dummy: 0,
            total_peds: 0,
            total_gang_peds: 0,
            total_civ_peds: 0,
            initial_zone_change: None,
            regeneration_point_a: CVector::ZERO,
            regeneration_point_b: CVector::ZERO,
            regeneration_front: CVector::ZERO,
        };
        population.initial_zone_change = Some((ELevelName::Commercial, ELevelName::Industrial));
        Ok(population)
    }
}

fn safe_zone_for_position(position: CVector) -> Option<usize> {
    SAFE_ZONES.iter().position(|zone| {
        position.x > zone.min.x
            && position.x < zone.max.x
            && position.y > zone.min.y
            && position.y < zone.max.y
            && position.z > zone.min.z
            && position.z < zone.max.z
    })
}

fn level_name_from_zone_level(level: i32) -> ELevelName {
    match level {
        1 => ELevelName::Industrial,
        2 => ELevelName::Commercial,
        3 => ELevelName::Suburban,
        _ => ELevelName::Generic,
    }
}

fn collision_level_for_position(file_loader: &CFileLoader, position: CVector) -> ELevelName {
    if safe_zone_for_position(position).is_some() {
        ELevelName::Generic
    } else {
        level_name_from_zone_level(
            file_loader
                .world
                .zones
                .get_level_from_position([position.x, position.y, position.z]),
        )
    }
}

fn update_regeneration_points(population: &mut CPopulation, player_position: CVector) {
    let Some((old_level, new_level)) = population.initial_zone_change.take() else {
        return;
    };
    population.zone_change_has_happened = true;

    // Population::Initialise forces the Callahan Bridge industrial transition
    // once so the first collision residency change has a valid recovery area.
    let first_initialisation = population.regeneration_point_a == CVector::ZERO
        && old_level == ELevelName::Commercial
        && new_level == ELevelName::Industrial;
    let probe = if first_initialisation {
        CVector::new(690.0, -920.0, 42.0)
    } else {
        player_position
    };

    let safe_zone = safe_zone_for_position(probe).or_else(|| {
        SAFE_ZONES
            .iter()
            .enumerate()
            .filter(|(_, zone)| {
                (zone.src_level == old_level || zone.dest_level == old_level)
                    && (zone.src_level == new_level || zone.dest_level == new_level)
            })
            .min_by(|(_, a), (_, b)| {
                let ax = probe.x - (a.min.x + a.max.x) * 0.5;
                let ay = probe.y - (a.min.y + a.max.y) * 0.5;
                let bx = probe.x - (b.min.x + b.max.x) * 0.5;
                let by = probe.y - (b.min.y + b.max.y) * 0.5;
                (ax * ax + ay * ay)
                    .partial_cmp(&(bx * bx + by * by))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
    });
    let fallback = match new_level {
        ELevelName::Industrial => Some(0),
        ELevelName::Suburban => Some(4),
        _ => None,
    };
    let Some(zone) = safe_zone
        .or(fallback)
        .and_then(|index| SAFE_ZONES.get(index))
    else {
        return;
    };

    let (point_a, point_b, front) = if zone.src_level == new_level {
        (
            zone.src_pos_a,
            zone.src_pos_b,
            CVector::new(
                zone.dest_pos_a.x - zone.src_pos_a.x,
                zone.dest_pos_a.y - zone.src_pos_a.y,
                zone.dest_pos_a.z - zone.src_pos_a.z,
            ),
        )
    } else if zone.dest_level == new_level {
        (
            zone.dest_pos_a,
            zone.dest_pos_b,
            CVector::new(
                zone.src_pos_a.x - zone.dest_pos_a.x,
                zone.src_pos_a.y - zone.dest_pos_a.y,
                zone.src_pos_a.z - zone.dest_pos_a.z,
            ),
        )
    } else {
        return;
    };
    let length = (front.x * front.x + front.y * front.y + front.z * front.z).sqrt();
    population.regeneration_point_a = point_a;
    population.regeneration_point_b = point_b;
    population.regeneration_front = if length > f32::EPSILON {
        CVector::new(front.x / length, front.y / length, front.z / length)
    } else {
        CVector::new(1.0, 0.0, 0.0)
    };
}

fn load_ped_groups(file_mgr: &mut CFileMgr, model_info: &CModelInfo) -> io::Result<Vec<PedGroup>> {
    let bytes = file_mgr.load_file("DATA\\PEDGRP.DAT", 64 * 1024, "r")?;
    let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
    let mut groups = Vec::with_capacity(NUMPEDGROUPS);

    for raw in text.lines() {
        if groups.len() >= NUMPEDGROUPS {
            break;
        }
        let line = normalise_line(raw);
        let mut models = [-1; NUMMODELSPERPEDGROUP];
        let mut count = 0;

        for part in line.split_whitespace() {
            if part.starts_with('#') {
                break;
            }
            if count >= NUMMODELSPERPEDGROUP {
                break;
            }
            if let Some((model_id, _)) = model_info.get_model_info_by_name(part) {
                models[count] = model_id as i32;
            }
            count += 1;
        }

        if count == NUMMODELSPERPEDGROUP {
            groups.push(PedGroup { models });
        }
    }

    while groups.len() < NUMPEDGROUPS {
        groups.push(PedGroup {
            models: [-1; NUMMODELSPERPEDGROUP],
        });
    }

    Ok(groups)
}

fn normalise_line(raw: &str) -> String {
    raw.chars()
        .map(|ch| if ch == ',' || ch == '\r' { ' ' } else { ch })
        .collect()
}

// === FAZ5: moved from core/Game.rs (verbatim, parity-preserving) ===
// Ambient population model pools, generation and abandoned-zone management.
// PedWander, PedChat and PedPopulationLifecycle own on-foot runtime control;
// the ped plugin registers these systems. CarCtrl uses traffic-model helpers.

#[allow(unused_imports)]
use crate::animation::CutsceneMgr::CCutsceneMgr;
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::control::CarCtrl::CCarCtrl;
#[allow(unused_imports)]
use crate::control::Script::{
    ScriptCreateCharRequest, remove_script_vehicle_and_occupants,
    script_vehicle_base_can_be_deleted,
};
#[allow(unused_imports)]
use crate::core::Camera::CCamera;
#[allow(unused_imports)]
use crate::core::Cam::{LOOKING_FORWARD, eCamMode};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Game::{
    GameplayAuditTimer, ScriptEntityWorld, ScriptObjectEntity, ScriptVehicleEntity,
    remove_script_object, spawn_script_object,
};
#[allow(unused_imports)]
use crate::core::Game::join_car_with_road_system;
#[allow(unused_imports)]
use crate::core::Pools::{CPoolEntry, CPoolEntryKind, CPoolHandle, CPools};
#[allow(unused_imports)]
use crate::core::Streaming::CStreaming;
#[allow(unused_imports)]
use crate::core::World::{
    ObjectDummyLifecycleChange, ZonePedInfo, is_glass_dummy, world_entity_gta_matrix,
};
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::{
    MI_AMBULAN, MI_ARMY, MI_BARRACKS, MI_CABBIE, MI_COP, MI_ENFORCER, MI_FBI, MI_FBICAR,
    MI_FIREMAN, MI_FIRETRUCK, MI_MALE01, MI_MEDIC, MI_POLICE, MI_RCBANDIT, MI_RHINO, MI_SWAT,
    MI_TAXI, MI_TAXI_D,
};
#[allow(unused_imports)]
use crate::peds::Gangs::CGangs;
#[allow(unused_imports)]
use crate::peds::Ped::{MISSION_CHAR, RANDOM_CHAR};
#[allow(unused_imports)]
use crate::peds::Ped::{
    PedState, apply_script_ped_wander_path, eMoveState, eObjective, give_ped_weapon_with_info,
};
#[allow(unused_imports)]
use crate::peds::PlayerPed::CWorldPlayers;
#[allow(unused_imports)]
use crate::vehicles::Automobile::CAutomobile;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use bevy::{ecs::system::SystemParam, prelude::*};
#[allow(unused_imports)]
use rust_gta_3_core::core::{General::CGeneral, Timer::CTimer};

/// re3 `CCarCtrl::CarArrays` / `TotalNumOfCarsOfRating`
/// (`control/CarCtrl.cpp:1935` zeroes them; the class buckets themselves are
/// filled from the loaded vehicle model set). They are built once per session
/// because the IDE model table never changes at runtime.
#[derive(Debug, Clone, Default, Resource)]
pub struct AmbientTrafficModels {
    pub built: bool,
    pub car_models: Vec<i32>,
    pub models_by_class: [Vec<i32>; 7],
}

impl AmbientTrafficModels {
    pub(crate) fn build(&mut self, file_loader: &CFileLoader, car_ctrl: &mut CCarCtrl) {
        self.car_models.clear();
        self.models_by_class = std::array::from_fn(|_| Vec::new());
        car_ctrl.car_arrays = [[0; crate::control::CarCtrl::MAX_CAR_MODELS_IN_ARRAY]; 7];
        car_ctrl.total_num_of_cars_of_rating = [0; 7];
        car_ctrl.next_car_of_rating = [0; 7];
        for id in 0..6500usize {
            let Some(slot) = file_loader.model_info.get_model_info(id) else {
                continue;
            };
            let crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) = slot else {
                continue;
            };
            // re3 `CVehicleModelInfo` keeps boats, planes and helis out of the
            // road-traffic pools; only wheeled cars join the road system.
            if vehicle.vehicle_type
                != crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_CAR as i32
            {
                continue;
            }
            let class = match vehicle.vehicle_class.to_ascii_lowercase().as_str() {
                "poorfamily" => 0,
                "richfamily" => 1,
                "executive" => 2,
                "worker" => 3,
                "special" => 4,
                "big" => 5,
                "taxi" => 6,
                // Mission, emergency, law-enforcement and gang vehicles are
                // selected by their authored population class, never by a
                // uniform scan of every CAR entry in the IDE table.
                _ => continue,
            };
            let capacity = car_ctrl.car_arrays[class].len();
            let frequency = (vehicle.frequency as usize)
                .min(capacity.saturating_sub(self.models_by_class[class].len()));
            for _ in 0..frequency {
                self.models_by_class[class].push(id as i32);
                self.car_models.push(id as i32);
            }
        }
        for (class, models) in self.models_by_class.iter().enumerate() {
            let count = models.len().min(car_ctrl.car_arrays[class].len());
            car_ctrl.car_arrays[class][..count].copy_from_slice(&models[..count]);
            car_ctrl.total_num_of_cars_of_rating[class] = count as i32;
        }
        self.built = true;
    }
}

#[derive(Debug, Clone, Default, Resource)]
pub struct AmbientPedModels {
    pub built: bool,
    pub ped_models: Vec<i32>,
}

/// Cumulative creation-gate diagnostics for re3 `CPopulation::AddToPopulation`.
/// All counters belong to one Bevy app lifetime; failures must remain visible
/// even when a later successful spawn hides the original cause.
#[derive(Debug, Default, Resource)]
pub struct AmbientPedSpawnDiagnostics {
    pub unavailable_streaming: u64,
    pub unavailable_models: u64,
    pub unavailable_player: u64,
    pub population_cap: u64,
    pub waiting_model_residency: u64,
    pub no_path_spawn_position: u64,
    pub blocked_spawn_position: u64,
    pub construction_failure: u64,
    pub spawned: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AmbientSpawnCollisionCandidate {
    pub(crate) matrix: CMatrix,
    pub(crate) radius: f32,
    pub(crate) is_vehicle: bool,
}

#[derive(Debug, Default)]
pub(crate) struct AmbientTrafficGenerationScratch {
    pub(crate) collision_candidates: Vec<AmbientSpawnCollisionCandidate>,
}

impl AmbientPedModels {
    fn build(&mut self, population: &CPopulation) {
        self.ped_models.clear();
        for group in &population.ped_groups {
            for &model_id in &group.models {
                if model_id >= 0 {
                    self.ped_models.push(model_id);
                }
            }
        }
        self.ped_models.sort_unstable();
        self.ped_models.dedup();
        self.built = true;
    }
}

fn ambient_ped_type_for_model(file_loader: &CFileLoader, model_id: i32) -> ePedType {
    let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Ped(model)) = file_loader
        .model_info
        .get_model_info(model_id.max(0) as usize)
    else {
        return ePedType::PEDTYPE_CIVMALE;
    };
    crate::peds::PedType::CPedTypeRuntime::FindPedType(&model.ped_type)
        .unwrap_or(ePedType::PEDTYPE_CIVMALE)
}

fn gang_ped_type(gang: usize) -> ePedType {
    use ePedType::*;
    match gang {
        0 => PEDTYPE_GANG1,
        1 => PEDTYPE_GANG2,
        2 => PEDTYPE_GANG3,
        3 => PEDTYPE_GANG4,
        4 => PEDTYPE_GANG5,
        5 => PEDTYPE_GANG6,
        6 => PEDTYPE_GANG7,
        7 => PEDTYPE_GANG8,
        _ => PEDTYPE_GANG9,
    }
}

fn ped_type_gang_index(ped_type: ePedType) -> Option<usize> {
    use ePedType::*;
    Some(match ped_type {
        PEDTYPE_GANG1 => 0,
        PEDTYPE_GANG2 => 1,
        PEDTYPE_GANG3 => 2,
        PEDTYPE_GANG4 => 3,
        PEDTYPE_GANG5 => 4,
        PEDTYPE_GANG6 => 5,
        PEDTYPE_GANG7 => 6,
        PEDTYPE_GANG8 => 7,
        PEDTYPE_GANG9 => 8,
        _ => return None,
    })
}

fn choose_gang_occupation(gang: usize, gangs: Option<&CGangs>, general: &mut CGeneral) -> i32 {
    // GTA III ped model IDs MI_GANG01..MI_GANG18 are contiguous.
    let first_model = 10 + (gang.min(8) as i32 * 2);
    match gangs
        .and_then(|gangs| gangs.gangs.get(gang))
        .map_or(-1, |info| info.ped_model_override)
    {
        -1 => first_model + general.get_random_number_in_range_i32(0, 2),
        0 => first_model,
        _ => first_model + 1,
    }
}

/// re3 `CTheZones::GetZoneInfoForTimeOfDay`: use the current navigation
/// zone's day/night population values and blend densities/thresholds at dawn
/// and dusk. Ped-group selection switches at 19:00/05:00, as in the original.
pub(crate) fn ambient_zone_info(
    file_loader: &CFileLoader,
    position: CVector,
    clock: &rust_gta_3_core::core::Clock::CClock,
) -> ZonePedInfo {
    let Some(zone) = file_loader
        .world
        .zones
        .find_zone_for_point([position.x, position.y, position.z])
    else {
        return ZonePedInfo::default();
    };
    let day = zone.ped_info_day;
    let night = zone.ped_info_night;
    if clock.get_is_time_in_range(8, 19) {
        return day;
    }
    if clock.get_is_time_in_range(22, 5) {
        return night;
    }
    interpolate_zone_info(day, night, clock.hours())
}

/// CTheZones::GetZoneInfoForTimeOfDay blends at whole-hour boundaries and
/// assigns float results to the native integer fields by truncation.
fn interpolate_zone_info(day: ZonePedInfo, night: ZonePedInfo, hour: u8) -> ZonePedInfo {
    let (day_weight, night_weight) = if hour >= 19 {
        let night_weight = (f32::from(hour) - 19.0) / 3.0;
        (1.0 - night_weight, night_weight)
    } else {
        let day_weight = (f32::from(hour) - 5.0) / 3.0;
        (day_weight, 1.0 - day_weight)
    };
    let blend_i16 =
        |a: i16, b: i16| (f32::from(a) * day_weight + f32::from(b) * night_weight) as i16;
    let blend_u16 = |a: u16, b: u16| {
        (f32::from(a) * day_weight + f32::from(b) * night_weight) as u16
    };
    ZonePedInfo {
        car_density: blend_i16(day.car_density, night.car_density),
        car_threshold: std::array::from_fn(|i| {
            blend_i16(day.car_threshold[i], night.car_threshold[i])
        }),
        cop_threshold: blend_i16(day.cop_threshold, night.cop_threshold),
        gang_threshold: std::array::from_fn(|i| {
            blend_i16(day.gang_threshold[i], night.gang_threshold[i])
        }),
        ped_density: blend_u16(day.ped_density, night.ped_density),
        cop_density: blend_u16(day.cop_density, night.cop_density),
        gang_density: std::array::from_fn(|i| {
            blend_u16(day.gang_density[i], night.gang_density[i])
        }),
        ped_group: if (5..19).contains(&hour) {
            day.ped_group
        } else {
            night.ped_group
        },
    }
}

#[derive(SystemParam)]
pub(crate) struct AmbientPedSpawnTables<'w> {
    weapon_info: Res<'w, crate::weapons::WeaponInfo::CWeaponInfoStore>,
    ped_types: Res<'w, crate::peds::PedType::CPedTypeRuntime>,
}

pub(crate) fn process_ambient_ped_generation(
    mut spawn_diagnostics: ResMut<AmbientPedSpawnDiagnostics>,
    mut ped_models: ResMut<AmbientPedModels>,
    mut population: ResMut<CPopulation>,
    car_ctrl: Res<CCarCtrl>,
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    mut file_loader: Option<ResMut<CFileLoader>>,
    players: CWorldPlayers,
    camera: Option<Res<CCamera>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    mut streaming: Option<ResMut<CStreaming>>,
    gangs: Option<Res<CGangs>>,
    clock: Res<rust_gta_3_core::core::Clock::CClock>,
    timer: Res<CTimer>,
    tables: AmbientPedSpawnTables,
) {
    if cutscene.is_some_and(|c| c.running) {
        return;
    }
    let Some(streaming) = streaming.as_deref_mut() else {
        spawn_diagnostics.unavailable_streaming += 1;
        return;
    };
    if !ped_models.built {
        ped_models.build(&population);
    }
    if ped_models.ped_models.is_empty() {
        spawn_diagnostics.unavailable_models += 1;
        return;
    }
    let Some(player) = players.focused() else {
        spawn_diagnostics.unavailable_player += 1;
        return;
    };
    let position = player.position();
    let focused_player_on_foot = players.relation(players.player_in_focus() as i32).is_none();
    let on_foot_cops = script_entities
        .peds
        .iter()
        .filter(|(handle, entity)| {
            entity.ped.ped_type == ePedType::PEDTYPE_COP
                && entity.vitality.health > 0.0
                && script_entities.relation_for_char(**handle).is_none()
        })
        .count();
    let creation_dist_multiplier = players
        .relation(players.player_in_focus() as i32)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle))
        .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
        .map(|vehicle| {
            let speed = vehicle.base().physical.move_speed;
            ((speed.x * speed.x + speed.y * speed.y).sqrt() + 0.9).clamp(1.0, 1.5)
        })
        .unwrap_or(1.0);
    let generation_dist_multiplier = camera
        .as_deref()
        .map_or(1.0, |camera| camera.generation_dist_multiplier);
    let zone_info = file_loader
        .as_deref()
        .map(|loader| ambient_zone_info(loader, position, &clock))
        .unwrap_or_default();
    // CStreaming::StreamZoneModels admits the authored palette as a group.
    // Loading only the model sampled by generation biases cold streets toward
    // the one already-resident model and pins old palettes indefinitely.
    if streaming.current_ped_grp != i32::from(zone_info.ped_group)
        && let Some(loader) = file_loader.as_deref_mut()
    {
        if let Some(old_group) = usize::try_from(streaming.current_ped_grp).ok()
            .and_then(|group| population.ped_groups.get(group))
        {
            for &model in &old_group.models {
                if model >= 0 {
                    streaming.set_model_is_deletable(model as usize, loader);
                    streaming.set_model_txd_is_deletable(model as usize, loader);
                }
            }
        }
        streaming.current_ped_grp = i32::from(zone_info.ped_group);
        if let Some(group) = population.ped_groups.get(zone_info.ped_group as usize) {
            for &model in &group.models {
                if model >= 0 {
                    streaming.request_model_with_adapter(model as usize,
                        crate::core::Streaming::STREAMFLAGS_DONT_REMOVE, loader);
                }
            }
        }
        streaming.request_model_with_adapter(crate::modelinfo::ModelIndices::MI_MALE01,
            crate::core::Streaming::STREAMFLAGS_DONT_REMOVE, loader);
    }
    let road_density = file_loader
        .as_deref()
        .map(|loader| loader.path_find.calc_road_density(position.x, position.y))
        .unwrap_or_default();
    // Population.cpp only forces the extra on-foot cop when vehicle
    // population pressure prevents another law-enforcement car from being
    // generated.  Keeping all three native pressure terms prevents wanted
    // gameplay from silently replacing ordinary street pedestrians.
    let traffic_limit =
        (f32::from(zone_info.car_density.max(0)) * road_density * car_ctrl.car_density_multiplier)
            .ceil() as i32;
    let vehicle_population_pressure = car_ctrl.num_law_enforcer_cars
        >= i32::from(player.wanted.m_MaximumLawEnforcerVehicles)
        || car_ctrl.num_random_cars >= traffic_limit
        || car_ctrl.population_car_budget_full();
    let force_cop = player.wanted.get_wanted_level() > 2
        && focused_player_on_foot
        && on_foot_cops < usize::from(player.wanted.m_MaxCops)
        && vehicle_population_pressure
        && !file_loader
            .as_deref()
            .is_some_and(|loader| loader.world.cull_zones.player_no_police());
    let density_total = u32::from(zone_info.ped_density)
        + u32::try_from(zone_info.car_density.max(0)).unwrap_or_default();
    let max_peds = (density_total as f32 * road_density * population.ped_density_multiplier)
        .min(population.max_number_of_peds_in_use.max(0) as f32) as usize;
    let startup_generation = if population.countdown_to_peds_at_start != 0 {
        population.countdown_to_peds_at_start -= 1;
        if population.countdown_to_peds_at_start != 0 {
            return;
        }
        true
    } else {
        false
    };
    let attempts = if startup_generation { 50 } else { 1 };
    let ped_group = zone_info.ped_group as usize;
    'attempts: for _ in 0..attempts {
        // re3 gates `ms_nTotalPeds`, a population counter which excludes the
        // player and non-population ped types.  Bevy's raw entity count also
        // includes the player and would incorrectly consume a pedestrian slot.
        // Wanted-level `addCop` explicitly overrides this density limit.
        if population.total_peds as usize >= max_peds && !force_cop {
            spawn_diagnostics.population_cap += 1;
            break;
        }
    let mut decision = general.get_random_number_in_range_i32(0, 1000);
    let (mut ped_type, mut requested_model, mut gang_index) =
        if force_cop || decision < i32::from(zone_info.cop_density) {
            // ChoosePolicePedOccupation consumes a random number even though
            // GTA III always returns COP_STREET.
            let _ = general.get_random_number();
            (ePedType::PEDTYPE_COP, MI_COP as i32, None)
        } else {
            decision -= i32::from(zone_info.cop_density);
            let mut gang = None;
            for (index, density) in zone_info.gang_density.iter().enumerate() {
                if decision < i32::from(*density) {
                    gang = Some(index);
                    break;
                }
                decision -= i32::from(*density);
            }
            if let Some(gang) = gang {
                (
                    gang_ped_type(gang),
                    choose_gang_occupation(gang, gangs.as_deref(), &mut general),
                    Some(gang),
                )
            } else {
                let Some(models) = population
                    .ped_groups
                    .get(ped_group)
                    .map(|group| &group.models)
                else {
                    continue 'attempts;
                };
                let choice = general.get_random_number_in_range_i32(0, models.len() as i32) as usize;
                let model = models[choice];
                if model < 0 {
                    continue 'attempts;
                }
                let Some(loader) = file_loader.as_deref() else {
                    return;
                };
                (ambient_ped_type_for_model(loader, model), model, None)
            }
        };
    if !force_cop && population.all_random_peds_this_type > ePedType::PEDTYPE_PLAYER1 as i32 {
        ped_type = crate::peds::PedType::script_ped_type(population.all_random_peds_this_type);
        // re3 applies m_AllRandomPedsThisType before deciding whether this is
        // a gang group. Do not keep the density roll's stale gang identity:
        // that could turn a script-forced civilian/special population into a
        // formation of gang models.
        gang_index = ped_type_gang_index(ped_type);
    }
    let ped_amount = if gang_index.is_some() {
        let roll = general.get_random_number() % 100;
        if roll < 50 {
            continue 'attempts;
        }
        if roll < 57 {
            1
        } else if roll < 74 {
            2
        } else if roll < 85 {
            3
        } else {
            4
        }
    } else {
        1
    };
    if !streaming.has_model_loaded(requested_model.max(0) as usize) {
        spawn_diagnostics.waiting_model_residency += 1;
        if let Some(loader) = file_loader.as_deref_mut() {
            streaming.request_model_with_adapter(
                requested_model.max(0) as usize,
                crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
                loader,
            );
        }
        continue 'attempts;
    }
    let (mut min_dist, mut max_dist, min_dist_off_screen, max_dist_off_screen) =
        if startup_generation {
            let startup_max = creation_dist_multiplier * (MIN_CREATION_DIST + CREATION_RANGE);
            (10.0, startup_max, 10.0, startup_max)
        } else {
            (
                creation_dist_multiplier * (MIN_CREATION_DIST * generation_dist_multiplier),
                creation_dist_multiplier
                    * ((MIN_CREATION_DIST + CREATION_RANGE) * generation_dist_multiplier),
                creation_dist_multiplier
                    * (MIN_CREATION_DIST + CREATION_RANGE)
                    * OFFSCREEN_CREATION_MULT
                    - CREATION_RANGE,
                creation_dist_multiplier
                    * (MIN_CREATION_DIST + CREATION_RANGE)
                    * OFFSCREEN_CREATION_MULT,
            )
        };
    if force_cop {
        // Population.cpp resets only the on-screen creation ring when a cop
        // is forced because traffic pressure prevents another law-enforcer
        // vehicle. The off-screen limits keep the caller-provided values.
        min_dist = creation_dist_multiplier * MIN_CREATION_DIST;
        max_dist = creation_dist_multiplier * (MIN_CREATION_DIST + CREATION_RANGE);
    }

    let Some(mut spawn_pos) = file_loader.as_deref_mut().and_then(|loader| {
        let path_find = &loader.path_find;
        let world = &loader.world;
        let model_info = &loader.model_info;
        path_find
            .generate_ped_creation_coors(
                position.x,
                position.y,
                min_dist,
                max_dist,
                min_dist_off_screen,
                max_dist_off_screen,
                &mut general,
                |center, radius| {
                    camera
                        .as_deref()
                        .is_none_or(|camera| camera.is_sphere_visible(center, radius))
                },
                |x, y, z| world.find_ground_z_for_3d_coord(x, y, z, model_info),
            )
            .map(|coors| coors.position)
    }) else {
        spawn_diagnostics.no_path_spawn_position += 1;
        continue 'attempts;
    };
    let mut leader: Option<(i32, CVector)> = None;
    for member in 0..ped_amount {
        if let Some(gang) = gang_index {
            requested_model = choose_gang_occupation(gang, gangs.as_deref(), &mut general);
            if !streaming.has_model_loaded(requested_model.max(0) as usize) {
                if let Some(loader) = file_loader.as_deref_mut() {
                    streaming.request_model_with_adapter(
                        requested_model.max(0) as usize,
                        crate::core::Streaming::STREAMFLAGS_DONT_REMOVE,
                        loader,
                    );
                }
                continue 'attempts;
            }
        }
        // re3 mutates one `generatedCoors` across the whole gang loop. In
        // particular Z keeps the previous iteration's ground adjustment and
        // receives another +0.7 before each member is tested/spawned.
        spawn_pos.z += 0.7;
        if member > 0
            && let Some((_, leader_pos)) = leader
        {
            let offset_min = member as f32 * 0.75;
            let mut x_offset =
                general.get_random_number_in_range_f32(offset_min, offset_min + 0.75);
            let mut y_offset =
                general.get_random_number_in_range_f32(offset_min, offset_min + 0.75);
            if general.get_random_number() & 1 != 0 {
                x_offset = -x_offset;
            }
            if general.get_random_number() & 1 != 0 {
                y_offset = -y_offset;
            }
            spawn_pos.x = leader_pos.x + x_offset;
            spawn_pos.y = leader_pos.y + y_offset;
        }
        let Some(loader) = file_loader.as_deref() else {
            return;
        };
        if !ambient_ped_spawn_is_clear(spawn_pos, &script_entities, &players, loader) {
            spawn_diagnostics.blocked_spawn_position += 1;
            break;
        }
        // Population.cpp only re-snaps to ground when another gang member is
        // still due. Preserve that oddity for GTA III parity.
        if member + 1 < ped_amount {
            let Some(ground) = loader.world.find_ground_z_for_3d_coord(
                spawn_pos.x,
                spawn_pos.y,
                spawn_pos.z + 2.0,
                &loader.model_info,
            ) else {
                continue 'attempts;
            };
            spawn_pos.z = spawn_pos.z.max(ground + 0.7);
        }
        if camera
            .as_deref()
            .is_some_and(|camera| camera.is_sphere_visible(spawn_pos, 2.0))
        {
            let dx = spawn_pos.x - position.x;
            let dy = spawn_pos.y - position.y;
            if creation_dist_multiplier * MIN_CREATION_DIST > (dx * dx + dy * dy).sqrt() {
                break;
            }
        }
        // The Entity is constructed with deferred Bevy commands, so initialize
        // its ownership, original animation and wander state before spawn.
        // A same-frame query for the just-spawned handle cannot see it.
        let give_peds_weapons = population.give_peds_weapons;
        let now_ms = timer.time_in_milliseconds();
        let handle = script_entities.insert_population_ped(
            &mut pools,
            ScriptCreateCharRequest {
                ped_type: ped_type as i32,
                model_id: requested_model,
                position: spawn_pos,
                vehicle_handle: None,
                passenger_seat: None,
            },
            &mut general,
            loader,
            |entity, general| {
                let ped = &mut entity.ped;
                ped.char_created_by = RANDOM_CHAR;
                // insert_population_ped deliberately has no script ped-type
                // argument. Give ambient actors CPed's native PED.DAT threats
                // before their first CivilianAI scan.
                ped.initialise_fear_flags(&tables.ped_types);
                // CPopulation::AddToPopulation starts with a transparent clump;
                // CPed::ProcessControl fades it in over subsequent frames.
                ped.physical.entity.clump_alpha = 0;
                ped.physical.zone_level = crate::collision::ColModel::LEVEL_GENERIC as i8;
                ped.flags.responds_to_threats = true;
                ped.flags.allow_medics_to_revive_me = true;
                if let Some(gang) = ped_type_gang_index(ped_type) {
                    if let Some(info) = gangs.as_deref().and_then(|gangs| gangs.gangs.get(gang)) {
                        let weapon = if general.get_random_number_in_range_i32(0, 100) >= 50 {
                            info.weapon2
                        } else {
                            info.weapon1
                        };
                        if let Some(type_) = give_ped_weapon_with_info(ped, weapon, 25_001, &tables.weapon_info) {
                            ped.current_weapon = type_;
                        }
                    }
                } else if give_peds_weapons
                    && matches!(ped_type, ePedType::PEDTYPE_CIVMALE | ePedType::PEDTYPE_CIVFEMALE)
                {
                    let weapon = general.get_random_number_in_range_i32(
                        crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED as i32,
                        crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_DETONATOR as i32,
                    );
                    if weapon != crate::weapons::WeaponType::eWeaponType::WEAPONTYPE_UNARMED as i32 {
                        if let Some(type_) = give_ped_weapon_with_info(ped, weapon, 25_001, &tables.weapon_info) {
                            ped.current_weapon = type_;
                        }
                    }
                }
                if member > 0 && let Some((leader_handle, _)) = leader {
                    ped.leader = Some(CPoolHandle(leader_handle).index());
                }
                let path = general.get_random_number_in_range_i32(0, 8);
                apply_script_ped_wander_path(
                    ped, &entity.vitality, path,
                    loader, general, now_ms,
                );
            },
        );
        let Some(handle) = handle else {
            spawn_diagnostics.construction_failure += 1;
            break;
        };
        spawn_diagnostics.spawned += 1;
        add_ped_to_population_counts(&mut population, ped_type);
        if leader.is_none() {
            leader = Some((handle, spawn_pos));
        }
    }
    }
}

/// re3 `CPopulation::MoveCarsAndPedsOutOfAbandonedZones`.
///
/// Collision for only one island is resident at a time. Mission/locked
/// entities that cannot be deleted must therefore be moved to the active
/// transition regeneration area instead of being left to drive/fall through
/// geometry whose COL has been unloaded.
pub(crate) fn process_abandoned_zone_entities(
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut population: ResMut<CPopulation>,
    mut car_ctrl: ResMut<CCarCtrl>,
    collision: Res<CCollision>,
    mut file_loader: ResMut<CFileLoader>,
    players: CWorldPlayers,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
) {
    let player_position = players
        .focused()
        .map_or(CVector::ZERO, |player| player.position());
    update_regeneration_points(&mut population, player_position);

    match timer.frame_counter() & 7 {
        1 => {
            let mut moved_vehicle_count = 0usize;
            let mut relocate = Vec::new();
            let mut remove = Vec::new();
            for (&handle, vehicle) in script_entities.vehicles.iter() {
                let base = vehicle.base();
                if base.physical.zone_level != crate::collision::ColModel::LEVEL_GENERIC as i8
                    || !matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. })
                    || matches!(
                        base.physical.entity.status,
                        eEntityStatus::STATUS_ABANDONED
                            | eEntityStatus::STATUS_WRECKED
                            | eEntityStatus::STATUS_PLAYER
                            | eEntityStatus::STATUS_PLAYER_REMOTE
                    )
                {
                    continue;
                }
                let position = base.physical.entity.get_position();
                let level = collision_level_for_position(&file_loader, position);
                if level == ELevelName::Generic
                    || level == collision.collision_in_memory
                    || position.z <= -4.0
                {
                    continue;
                }
                if base.flags.is_locked
                    || !script_vehicle_base_can_be_deleted(&script_entities, handle, &base)
                {
                    relocate.push(handle);
                } else {
                    remove.push(handle);
                }
            }
            for handle in remove {
                remove_script_vehicle_and_occupants(
                    &mut script_entities,
                    &mut pools,
                    handle,
                    Some(&mut population),
                    Some(&mut car_ctrl),
                );
            }
            for handle in relocate {
                let slot = moved_vehicle_count;
                let mut destination = match slot & 3 {
                    0 => population.regeneration_point_a,
                    1 => population.regeneration_point_b,
                    2 => CVector::new(
                        population.regeneration_point_a.x,
                        population.regeneration_point_b.y,
                        population.regeneration_point_a.z,
                    ),
                    _ => CVector::new(
                        population.regeneration_point_b.x,
                        population.regeneration_point_a.y,
                        population.regeneration_point_a.z,
                    ),
                };
                destination.z += (slot / 4) as f32 * 7.0;

                let relocated = {
                    let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                        script_entities.vehicle_get_mut(handle)
                    else {
                        continue;
                    };
                    let mut matrix = car.vehicle.core.physical.entity.matrix;
                    matrix.position = destination;
                    matrix.forward = population.regeneration_front;
                    matrix.right = CVector::new(matrix.forward.y, -matrix.forward.x, 0.0);
                    matrix.up = matrix.right.cross(matrix.forward);
                    car.vehicle.core.physical.entity.set_matrix(matrix);
                    car.vehicle.core.physical.move_speed = CVector::ZERO;

                    let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
                    if let Some(col) = file_loader.model_info.col_model(model_id) {
                        let bbox_min = CVector::new(
                            col.bounding_box.min.x,
                            col.bounding_box.min.y,
                            col.bounding_box.min.z,
                        );
                        let bbox_max = CVector::new(
                            col.bounding_box.max.x,
                            col.bounding_box.max.y,
                            col.bounding_box.max.z,
                        );
                        let world = &file_loader.world;
                        let model_info = &file_loader.model_info;
                        car.place_on_road_properly(bbox_min, bbox_max, |probe, low_z| {
                            world
                                .process_vertical_line_buildings_only_with_entity(
                                    [probe.x, probe.y, probe.z],
                                    low_z,
                                    model_info,
                                )
                                .map(|(point, entity)| (point.point.z, entity))
                        });
                    }
                    join_car_with_road_system(
                        &mut file_loader.path_find,
                        &mut car.vehicle,
                        &mut general,
                    );
                    (car.vehicle.core.physical.entity.get_position(), model_id)
                };
                // re3 calls ClearSpaceForMissionEntity after PlaceOnRoadProperly
                // and the road join.  This matters when several persistent
                // cars are regenerated at the same transition area: ordinary
                // population entities must be removed instead of leaving the
                // mission car interpenetrating them on the next physics tick.
                script_entities.clear_space_for_mission_entity_except(
                    &mut pools,
                    relocated.0,
                    relocated.1,
                    &file_loader,
                    Some(handle),
                );
                moved_vehicle_count += 1;
            }
        }
        5 => {
            let mut remove = Vec::new();
            let mut relocate = Vec::new();
            for (&handle, entity) in script_entities.peds.iter() {
                let ped = &entity.ped;
                if ped.physical.zone_level != crate::collision::ColModel::LEVEL_GENERIC as i8
                    || script_entities.relation_for_char(handle).is_some()
                {
                    continue;
                }
                let position = ped.physical.entity.get_position();
                let level = collision_level_for_position(&file_loader, position);
                if level == ELevelName::Generic
                    || level == collision.collision_in_memory
                    || position.z <= -4.0
                {
                    continue;
                }
                if ped.char_created_by == MISSION_CHAR {
                    relocate.push(handle);
                } else {
                    remove.push((handle, ped.ped_type));
                }
            }
            for (handle, ped_type) in remove {
                remove_ped_from_population_counts(&mut population, ped_type);
                script_entities.remove_char(&mut pools, handle);
            }
            for handle in relocate {
                let mut destination = population.regeneration_point_a;
                if let Some(ground_z) = file_loader.world.find_ground_z_for_3d_coord(
                    destination.x,
                    destination.y,
                    destination.z + 2.0,
                    &file_loader.model_info,
                ) {
                    destination.z = ground_z + 1.0;
                }
                let model_id = if let Some(entity) = script_entities.ped_get_mut(handle) {
                    entity.ped.physical.entity.set_position(destination);
                    entity.ped.physical.move_speed = CVector::ZERO;
                    Some(entity.ped.physical.entity.model_index.max(0) as usize)
                } else {
                    None
                };
                if let Some(model_id) = model_id {
                    // Same ordering as Population.cpp: move, ground, then
                    // ClearSpaceForMissionEntity. Mission chars are protected
                    // by the helper while deletable ambient blockers are
                    // cleared from the regeneration point.
                    script_entities.clear_space_for_mission_entity(
                        &mut pools,
                        destination,
                        model_id,
                        &file_loader,
                    );
                }
            }
        }
        _ => {}
    }
}

/// Static-map object half of re3 `CPopulation::ManagePopulation`.
///
/// OBJECT.DAT-backed IPL dummies become real colliding `CObject`s near the
/// player and revert to collisionless dummies beyond 80 m, mirroring
/// `ConvertToRealObject`/`ConvertToDummyObject`. While promoted, the live ECS
/// CObject owns the render slot, uproot thresholds, `ObjectDamage`, debris and
/// buoyancy; the stable world entity only keeps hidden provenance.
pub(crate) fn process_map_object_collision_lifecycle(
    mut file_loader: ResMut<CFileLoader>,
    mut script_entities: ScriptEntityWorld,
    mut pools: ResMut<CPools>,
    mut general: ResMut<CGeneral>,
    players: CWorldPlayers,
    timer: Res<CTimer>,
) {
    let Some(player) = players.focused() else {
        return;
    };
    let player_position = player.position();
    let changes = {
        let CFileLoader {
            world, model_info, ..
        } = &mut *file_loader;
        world.update_object_dummy_collision_lifecycle(
            player_position,
            timer.frame_counter(),
            model_info,
        )
    };
    for change in changes {
        match change {
            ObjectDummyLifecycleChange::Promoted {
                entity_index,
                model_index,
            } => {
                let blocked = {
                    let loader = &*file_loader;
                    promotion_is_blocked(loader, &script_entities, entity_index, model_index)
                };
                if blocked {
                    let CFileLoader {
                        world, model_info, ..
                    } = &mut *file_loader;
                    world.abort_object_dummy_promotion(entity_index, model_info);
                    continue;
                }
                let entity_snapshot = file_loader
                    .world
                    .entities
                    .get(entity_index)
                    .cloned();
                let Some(entity_snapshot) = entity_snapshot else {
                    continue;
                };
                let Some(handle) = pools
                    .object_pool
                    .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
                    .map(|entry| entry.0)
                else {
                    let CFileLoader {
                        world, model_info, ..
                    } = &mut *file_loader;
                    world.abort_object_dummy_promotion(entity_index, model_info);
                    continue;
                };
                let mut object = CObject::new_with_model(model_index as i32, false, &mut general);
                object.initialize_world_state(&file_loader);
                let matrix = world_entity_gta_matrix(&entity_snapshot);
                object.physical.entity.set_matrix(matrix);
                object.object_matrix = matrix;
                let model_name = file_loader
                    .model_info
                    .get_model_info(model_index)
                    .map(|model| model.base().name.as_str());
                match model_name {
                    // re3 ConvertToRealObject: glass panes stay invisible and
                    // CGlass owns their presentation.
                    Some(name) if is_glass_dummy(name) => {
                        object.physical.entity.flags.is_visible = false;
                    }
                    // re3 ConvertToRealObject: buoys enter the world dynamic
                    // and sinking.
                    Some(name) if name.eq_ignore_ascii_case("bouy") => {
                        object.physical.entity.set_is_static(false);
                        object.physical.set_move_speed(0.0, 0.0, -0.001);
                        object.physical.entity.flags.touching_water = true;
                    }
                    _ => {}
                }
                let handle = spawn_script_object(
                    &mut script_entities.commands,
                    &mut script_entities.index,
                    ScriptObjectEntity { handle, object },
                );
                let CFileLoader { world, .. } = &mut *file_loader;
                world.ecs_lifecycle_objects.insert(entity_index, handle);
            }
            ObjectDummyLifecycleChange::Demoted { entity_index } => {
                let handle = {
                    let CFileLoader { world, .. } = &mut *file_loader;
                    world.ecs_lifecycle_objects.remove(&entity_index)
                };
                if let Some(handle) = handle
                    && !remove_script_object(
                        &mut script_entities.commands,
                        &mut script_entities.index,
                        Some(&mut pools),
                        handle,
                    )
                {
                    let CFileLoader { world, .. } = &mut *file_loader;
                    world.ecs_lifecycle_objects.insert(entity_index, handle);
                }
            }
        }
    }
}

/// re3 `CPopulation::TestSafeForRealObject`: a vehicle overlapping the dummy
/// blocks the conversion so the new CObject cannot materialise inside it. A
/// live ECS CObject already sitting on the same placement (restored save)
/// blocks the conversion permanently instead.
fn promotion_is_blocked(
    loader: &CFileLoader,
    script_entities: &ScriptEntityWorld<'_, '_>,
    entity_index: usize,
    model_index: usize,
) -> bool {
    let Some(entity) = loader.world.entities.get(entity_index) else {
        return true;
    };
    let Some(col) = loader.model_info.col_model(model_index) else {
        return false;
    };
    let matrix = world_entity_gta_matrix(entity);
    for (_, vehicle) in script_entities.vehicles.iter() {
        let base = vehicle.base();
        let vehicle_model_id = base.physical.entity.model_index.max(0) as usize;
        let Some(vehicle_col) = loader.model_info.col_model(vehicle_model_id) else {
            continue;
        };
        if col_bound_spheres_overlap(
            &matrix,
            col,
            &base.physical.entity.matrix,
            vehicle_col,
        ) {
            return true;
        }
    }
    let position = CVector::new(entity.position[0], entity.position[1], entity.position[2]);
    script_entities.objects.values().any(|entry| {
        let object = &entry.object;
        if object.physical.entity.model_index.max(0) as usize != model_index {
            return false;
        }
        let other = object.physical.entity.get_position();
        let dx = other.x - position.x;
        let dy = other.y - position.y;
        let dz = other.z - position.z;
        dx * dx + dy * dy + dz * dz < 0.05
    })
}

pub(crate) fn ambient_ped_spawn_is_clear(
    position: CVector,
    script_entities: &ScriptEntityWorld<'_, '_>,
    players: &CWorldPlayers<'_, '_>,
    file_loader: &CFileLoader,
) -> bool {
    crate::peds::PedPlacement::CPedPlacement::is_position_clear_for_ped(
        position,
        |position, radius, _check_2d_only, max_entities| {
            ambient_ped_spawn_collider_count(
                position,
                radius,
                max_entities,
                script_entities,
                players,
                file_loader,
            )
        },
    )
}

pub(crate) fn ambient_ped_spawn_collider_count(
    position: CVector,
    radius: f32,
    max_entities: i16,
    script_entities: &ScriptEntityWorld<'_, '_>,
    players: &CWorldPlayers<'_, '_>,
    file_loader: &CFileLoader,
) -> i16 {
    let overlaps = |other_position: CVector, model_id: usize| {
        let Some(col) = file_loader.model_info.col_model(model_id) else {
            return false;
        };
        let dx = position.x - other_position.x;
        let dy = position.y - other_position.y;
        let combined = col.bounding_sphere.radius + radius;
        dx * dx + dy * dy < combined * combined
    };
    let mut count = 0i16;

    for vehicle in script_entities.vehicles.values() {
        let base = vehicle.base();
        if overlaps(
            base.physical.entity.get_position(),
            base.physical.entity.model_index.max(0) as usize,
        ) {
            count += 1;
            if count >= max_entities {
                return count;
            }
        }
    }
    for ped in script_entities.peds.values() {
        if overlaps(
            ped.ped.physical.entity.get_position(),
            ped.ped.physical.entity.model_index.max(0) as usize,
        ) {
            count += 1;
            if count >= max_entities {
                return count;
            }
        }
    }
    for player in players.iter() {
        if overlaps(
            player.ped.physical.entity.get_position(),
            player.ped.physical.entity.model_index.max(0) as usize,
        ) {
            count += 1;
            if count >= max_entities {
                return count;
            }
        }
    }
    count
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AmbientCarClass {
    Civilian(usize),
    Cops,
    Gang(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmbientCarModelChoice {
    pub model_id: i32,
    pub class: AmbientCarClass,
}

pub(crate) fn choose_police_car_model(
    wanted: &crate::core::Wanted::CWanted,
    streaming: &CStreaming,
    general: &mut CGeneral,
) -> i32 {
    use crate::modelinfo::ModelIndices::{
        MI_ARMY, MI_BARRACKS, MI_ENFORCER, MI_FBI, MI_FBICAR, MI_POLICE, MI_RHINO,
    };

    if wanted.are_swat_required()
        && streaming.has_model_loaded(MI_ENFORCER)
        && streaming.has_model_loaded(MI_POLICE)
    {
        return if general.get_random_number() & 0x0f == 0 {
            MI_ENFORCER as i32
        } else {
            MI_POLICE as i32
        };
    }
    if wanted.are_fbi_required()
        && streaming.has_model_loaded(MI_FBICAR)
        && streaming.has_model_loaded(MI_FBI)
    {
        return MI_FBICAR as i32;
    }
    if wanted.are_army_required()
        && streaming.has_model_loaded(MI_RHINO)
        && streaming.has_model_loaded(MI_BARRACKS)
        && streaming.has_model_loaded(MI_ARMY)
    {
        return if general.get_random_true_false() {
            MI_BARRACKS as i32
        } else {
            MI_RHINO as i32
        };
    }
    MI_POLICE as i32
}

fn choose_gang_car_model(
    gang: usize,
    gangs: Option<&CGangs>,
    streaming: &CStreaming,
) -> Option<i32> {
    // GTA III's MI_GANG01..MI_GANG18 are contiguous at model IDs 10..27.
    let first_ped = 10 + gang * 2;
    if !streaming.has_model_loaded(first_ped) || !streaming.has_model_loaded(first_ped + 1) {
        return None;
    }
    gangs
        .and_then(|gangs| gangs.gangs.get(gang))
        .map(|gang| gang.vehicle_model)
        .filter(|model| *model >= 0)
}

pub(crate) fn choose_ambient_car_model(
    car_ctrl: &mut CCarCtrl,
    zone_info: ZonePedInfo,
    gangs: Option<&CGangs>,
    wanted: &crate::core::Wanted::CWanted,
    streaming: &CStreaming,
    general: &mut CGeneral,
) -> Option<AmbientCarModelChoice> {
    // Native ChooseModel rerolls until it gets a resident model. During Bevy
    // streaming startup there can briefly be none, so keep the same reroll
    // semantics with a finite guard and try again next frame if necessary.
    for _ in 0..64 {
        let roll = general.get_random_number_in_range_i32(0, 1000) as i16;
        let class = if roll < zone_info.car_threshold[0] {
            AmbientCarClass::Civilian(0)
        } else if roll < zone_info.car_threshold[1] {
            AmbientCarClass::Civilian(1)
        } else if roll < zone_info.car_threshold[2] {
            AmbientCarClass::Civilian(2)
        } else if roll < zone_info.car_threshold[3] {
            AmbientCarClass::Civilian(3)
        } else if roll < zone_info.car_threshold[4] {
            AmbientCarClass::Civilian(4)
        } else if roll < zone_info.car_threshold[5] {
            AmbientCarClass::Civilian(5)
        } else if roll < zone_info.cop_threshold {
            AmbientCarClass::Cops
        } else if let Some(gang) = zone_info
            .gang_threshold
            .iter()
            .position(|threshold| roll < *threshold)
        {
            AmbientCarClass::Gang(gang)
        } else {
            AmbientCarClass::Civilian(6)
        };

        let model_id = match class {
            AmbientCarClass::Civilian(class) => car_ctrl.choose_car_model(class, general),
            AmbientCarClass::Cops => Some(choose_police_car_model(wanted, streaming, general)),
            AmbientCarClass::Gang(gang) => choose_gang_car_model(gang, gangs, streaming),
        };
        let Some(model_id) = model_id else {
            continue;
        };
        if model_id >= 0 && streaming.has_model_loaded(model_id as usize) {
            return Some(AmbientCarModelChoice { model_id, class });
        }
    }
    None
}

/// `CPopulation::AddPedInCar`'s model/type dispatch
/// (`peds/Population.cpp:745-838`).  A generic `MI_MALE01` driver is the
/// correct fallback for ordinary civilian traffic, but emergency/taxi models
/// carry authored occupants with different ped stats and meshes.  Keeping the
/// dispatch at spawn time also means the existing `SetUpDriver` seat/animation
/// wiring remains untouched.
pub(crate) fn ambient_driver_profile(
    vehicle_model_id: usize,
    general: &mut CGeneral,
    streaming: &CStreaming,
    file_loader: &CFileLoader,
    population: &CPopulation,
    gangs: Option<&CGangs>,
    ped_group: usize,
) -> (i32, i32) {
    let default_model = if matches!(
        vehicle_model_id,
        MI_TAXI | MI_CABBIE | crate::modelinfo::ModelIndices::MI_BORGNINE
    ) {
        MI_TAXI_D
    } else {
        MI_MALE01
    };
    let special = match vehicle_model_id {
        MI_FIRETRUCK => Some((ePedType::PEDTYPE_FIREMAN, MI_FIREMAN)),
        MI_AMBULAN => Some((ePedType::PEDTYPE_EMERGENCY, MI_MEDIC)),
        MI_FBICAR => Some((ePedType::PEDTYPE_COP, MI_FBI)),
        MI_POLICE => Some((ePedType::PEDTYPE_COP, MI_COP)),
        MI_ENFORCER => Some((ePedType::PEDTYPE_COP, MI_SWAT)),
        MI_RHINO | MI_BARRACKS => Some((ePedType::PEDTYPE_COP, MI_ARMY)),
        MI_TAXI | MI_CABBIE | crate::modelinfo::ModelIndices::MI_BORGNINE => {
            // The original alternates a taxi driver and the default civilian
            // branch with `GetRandomTrueFalse`; only select the special mesh
            // when it is resident, exactly like AddPedInCar's loaded-model
            // guard.
            general
                .get_random_true_false()
                .then_some((ePedType::PEDTYPE_CIVMALE, MI_TAXI_D))
        }
        _ => None,
    };
    if let Some((ped_type, model_id)) = special {
        if streaming.has_model_loaded(model_id) {
            return (ped_type as i32, model_id as i32);
        }
        return (
            ambient_ped_type_for_model(file_loader, default_model as i32) as i32,
            default_model as i32,
        );
    }

    // re3 AddPedInCar checks the gang vehicle table before falling back to a
    // civilian occupation. This is what puts Mafia/Triad/etc. members in their
    // authored traffic cars instead of generic civilians.
    if let Some(gang_index) = gangs.and_then(|gangs| {
        gangs
            .gangs
            .iter()
            .position(|gang| gang.vehicle_model == vehicle_model_id as i32)
    }) {
        let model_id = choose_gang_occupation(gang_index, gangs, general);
        if model_id >= 0 && streaming.has_model_loaded(model_id as usize) {
            return (gang_ped_type(gang_index) as i32, model_id);
        }
        return (
            ambient_ped_type_for_model(file_loader, default_model as i32) as i32,
            default_model as i32,
        );
    }

    // re3 AddPedInCar chooses an occupation from this zone's PEDGRP and tries
    // up to 16 models (`i = 15` through `i = 0`) until it finds a loaded ped whose carsCanDrive mask
    // includes the vehicle's authored class.
    let vehicle_class = file_loader
        .model_info
        .get_model_info(vehicle_model_id)
        .and_then(|model| match model {
            crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) => {
                Some(vehicle.vehicle_class.as_str())
            }
            _ => None,
        })
        .and_then(|class| match class {
            "poorfamily" => Some(0),
            "richfamily" => Some(1),
            "executive" => Some(2),
            "worker" => Some(3),
            "special" => Some(4),
            "big" => Some(5),
            "taxi" => Some(6),
            _ => None,
        });
    let Some(models) = population
        .ped_groups
        .get(ped_group)
        .map(|group| &group.models)
    else {
        return (
            ambient_ped_type_for_model(file_loader, default_model as i32) as i32,
            default_model as i32,
        );
    };
    for _ in 0..16 {
        let choice = general.get_random_number_in_range_i32(0, models.len() as i32) as usize;
        let selected = models[choice];
        let model_id = if selected < 0 { default_model as i32 } else { selected };
        let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Ped(model)) =
            file_loader.model_info.get_model_info(model_id as usize)
        else {
            continue;
        };
        if vehicle_class.is_some_and(|class| model.cars_can_drive & (1 << class) != 0) {
            // AddPedInCar finishes occupation selection before testing
            // residency. An unloaded eligible model selects the default;
            // it does not cause another occupation roll.
            if !streaming.has_model_loaded(model_id as usize) {
                return (
                    ambient_ped_type_for_model(file_loader, default_model as i32) as i32,
                    default_model as i32,
                );
            }
            let ped_type =
                CPedTypeRuntime::FindPedType(&model.ped_type).unwrap_or(ePedType::PEDTYPE_CIVMALE);
            return (ped_type as i32, model_id);
        }
    }
    (
        ambient_ped_type_for_model(file_loader, default_model as i32) as i32,
        default_model as i32,
    )
}

/// Resolve the car path node re3 would spawn on: `CPathFind::
/// FindNodeClosestToCoorsFavourDirection` with the outbound spawn direction.
pub(crate) fn ambient_car_spawn_position(
    file_loader: &mut CFileLoader,
    target: CVector,
    direction_x: f32,
    direction_y: f32,
    preferred_distance: f32,
    angle_limit: f32,
    forward: bool,
    general: &mut CGeneral,
) -> Option<crate::control::PathFind::CCarCreationCoors> {
    // This is the production equivalent of `ThePaths::NewGenerateCarCreationCoors`.
    // Returning the authored node pair and interpolation fraction is
    // important: using only the nearest node left every ambient car at a
    // junction with no valid curve history, which made the city look frozen.
    file_loader.path_find.new_generate_car_creation_coors(
        target.x,
        target.y,
        direction_x,
        direction_y,
        preferred_distance,
        angle_limit,
        forward,
        false,
        general,
    )
}

/// re3 `CWorld::FindObjectsKindaColliding` gate used before a random car is
/// created. The original query is deliberately broad phase: it compares the
/// spawn point against each existing vehicle/ped bound radius in 2D and adds
/// a fixed 10 m clearance. There is no COL narrow phase here.
pub(crate) fn ambient_car_spawn_is_blocked(
    position: CVector,
    candidates: &[AmbientSpawnCollisionCandidate],
) -> bool {
    candidates.iter().any(|candidate| {
        let dx = candidate.matrix.position.x - position.x;
        let dy = candidate.matrix.position.y - position.y;
        let radius = candidate.radius + 10.0;
        dx * dx + dy * dy < radius * radius
    })
}

pub(crate) fn ambient_car_spawn_is_blocked_with_extra_radius(
    position: CVector,
    model_id: usize,
    candidates: &[AmbientSpawnCollisionCandidate],
    file_loader: &CFileLoader,
    extra_radius: f32,
) -> bool {
    crate::profile_scope!("ambient_car_spawn_is_blocked");
    let Some(new_col) = file_loader.model_info.col_model(model_id) else {
        return true;
    };
    let query_radius = new_col.bounding_sphere.radius + extra_radius.max(0.0);
    candidates.iter().any(|candidate| {
        if !candidate.is_vehicle {
            return false;
        }
        let dx = candidate.matrix.position.x - position.x;
        let dy = candidate.matrix.position.y - position.y;
        let radius = query_radius + candidate.radius;
        dx * dx + dy * dy < radius * radius
    })
}

pub(crate) fn ambient_car_spawn_has_nearby_entity(
    position: CVector,
    model_id: usize,
    candidates: &[AmbientSpawnCollisionCandidate],
    file_loader: &CFileLoader,
    extra_radius: f32,
) -> bool {
    let Some(new_col) = file_loader.model_info.col_model(model_id) else {
        return true;
    };
    let query_radius = new_col.bounding_sphere.radius + extra_radius.max(0.0);
    candidates.iter().any(|candidate| {
        if !candidate.is_vehicle {
            return false;
        }
        let dx = candidate.matrix.position.x - position.x;
        let dy = candidate.matrix.position.y - position.y;
        let radius = query_radius + candidate.radius;
        dx * dx + dy * dy < radius * radius
    })
}
