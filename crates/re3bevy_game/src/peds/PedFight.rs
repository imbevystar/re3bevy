//! re3 reference: references/re-GTA/src/peds/PedFight.cpp
//! re3 reference: references/re-GTA/src/peds/Ped.h
//!
//! Ped melee state and `DATA\\fistfite.dat` move table.

use std::io;

use bevy::prelude::{Component, Entity, Resource};
use re3bevy_assets::install::FileMgr::CFileMgr;

use crate::animation::{
    AnimAssocDefinitions::ASSOCGRP_STD,
    AnimBlendAssociation::{ASSOC_DELETEFADEDOUT, ASSOC_REPEAT},
    AnimBlendClumpData::CAnimBlendClumpData,
    AnimManager::CAnimManager,
    AnimationId::{
        ANIM_STD_FIGHT_2IDLE, ANIM_STD_FIGHT_IDLE, ANIM_STD_NUM, ANIM_STD_RUNSTOP1,
        ANIM_STD_RUNSTOP2, ANIM_STD_STARTWALK, ANIM_STD_WALK, AnimationId,
    },
};
use crate::peds::Ped::{CPed, PedFightMoves, PedState, eMoveState};

pub const NUM_FIGHTMOVES: usize = 24;
pub const FIGHTSTATE_MOVE_FINISHED: i8 = -2;
pub const FIGHTSTATE_JUST_ATTACKED: i8 = -1;
pub const FIGHTSTATE_NO_MOVE: i8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightEndType {
    Normal,
    WithStep,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum FightMoveHitLevel {
    HITLEVEL_NULL = 0,
    HITLEVEL_GROUND,
    HITLEVEL_LOW,
    HITLEVEL_MEDIUM,
    HITLEVEL_HIGH,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FightMove {
    pub anim_id: AnimationId,
    pub start_fire_time: f32,
    pub end_fire_time: f32,
    pub combo_follow_on_time: f32,
    pub strike_radius: f32,
    pub hit_level: FightMoveHitLevel,
    pub damage: u8,
    pub flags: u8,
}

impl Default for FightMove {
    fn default() -> Self {
        Self {
            anim_id: ANIM_STD_NUM as AnimationId,
            start_fire_time: 0.0,
            end_fire_time: 0.0,
            combo_follow_on_time: 0.0,
            strike_radius: 0.0,
            hit_level: FightMoveHitLevel::HITLEVEL_NULL,
            damage: 0,
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CFightMoveTable {
    pub moves: [FightMove; NUM_FIGHTMOVES],
}

impl CFightMoveTable {
    /// `CPed::LoadFightData`: animation names are resolved against ASSOCGRP_STD
    /// after PED.IFP association groups have been created.
    pub fn load(file_mgr: &mut CFileMgr, anim_manager: &CAnimManager) -> io::Result<Self> {
        let bytes = file_mgr.load_file("DATA\\fistfite.dat", 64 * 1024, "r")?;
        Self::from_fistfite_dat(&bytes, anim_manager)
    }

    pub fn from_fistfite_dat(bytes: &[u8], anim_manager: &CAnimManager) -> io::Result<Self> {
        let text = std::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes))
            .map_err(|_| invalid_data("DATA\\fistfite.dat is not text"))?;
        let standard_group = anim_manager
            .anim_assoc_groups
            .first()
            .ok_or_else(|| invalid_data("ASSOCGRP_STD was not created before LoadFightData"))?;
        let mut table = Self {
            moves: [FightMove::default(); NUM_FIGHTMOVES],
        };
        let mut move_id = 0usize;
        let mut found_end = false;

        for source_line in text.lines() {
            let line = source_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields[0].starts_with("ENDWEAPONDATA") {
                found_end = true;
                break;
            }
            if fields.len() < 9 {
                return Err(invalid_data(format!(
                    "invalid fistfite.dat row: {source_line}"
                )));
            }
            if move_id >= NUM_FIGHTMOVES {
                return Err(invalid_data(
                    "fistfite.dat has more than NUM_FIGHTMOVES rows",
                ));
            }
            let parse_f32 = |index: usize| {
                fields[index].parse::<f32>().map_err(|_| {
                    invalid_data(format!("invalid fistfite.dat number: {source_line}"))
                })
            };
            let hit_level = match fields[5].as_bytes().first().copied() {
                Some(b'G') => FightMoveHitLevel::HITLEVEL_GROUND,
                Some(b'H') => FightMoveHitLevel::HITLEVEL_HIGH,
                Some(b'L') => FightMoveHitLevel::HITLEVEL_LOW,
                Some(b'M') => FightMoveHitLevel::HITLEVEL_MEDIUM,
                Some(b'N') => FightMoveHitLevel::HITLEVEL_NULL,
                _ => {
                    return Err(invalid_data(format!(
                        "invalid fistfite.dat hit level: {source_line}"
                    )));
                }
            };
            let anim_id = if fields[6] == "null" {
                ANIM_STD_WALK
            } else {
                standard_group
                    .get_animation_by_name(fields[6])
                    .map(|association| association.anim_id)
                    .ok_or_else(|| {
                        invalid_data(format!(
                            "fistfite.dat animation {} is absent from ASSOCGRP_STD",
                            fields[6]
                        ))
                    })?
            };
            table.moves[move_id] = FightMove {
                anim_id,
                start_fire_time: parse_f32(1)? / 30.0,
                end_fire_time: parse_f32(2)? / 30.0,
                combo_follow_on_time: parse_f32(3)? / 30.0,
                strike_radius: parse_f32(4)?,
                hit_level,
                damage: fields[7].parse::<u8>().map_err(|_| {
                    invalid_data(format!("invalid fistfite.dat damage: {source_line}"))
                })?,
                flags: fields[8].parse::<u8>().map_err(|_| {
                    invalid_data(format!("invalid fistfite.dat flags: {source_line}"))
                })?,
            };
            move_id += 1;
        }

        if !found_end || move_id != NUM_FIGHTMOVES {
            return Err(invalid_data(format!(
                "fistfite.dat loaded {move_id} moves; expected {NUM_FIGHTMOVES} before ENDWEAPONDATA"
            )));
        }
        Ok(table)
    }

    pub fn get(&self, move_id: usize) -> Option<&FightMove> {
        self.moves.get(move_id)
    }
}

pub fn fight_move_from_index(move_id: usize) -> Option<PedFightMoves> {
    Some(match move_id {
        0 => PedFightMoves::FIGHTMOVE_NULL,
        1 => PedFightMoves::FIGHTMOVE_STDPUNCH,
        2 => PedFightMoves::FIGHTMOVE_IDLE,
        3 => PedFightMoves::FIGHTMOVE_SHUFFLE_F,
        4 => PedFightMoves::FIGHTMOVE_KNEE,
        5 => PedFightMoves::FIGHTMOVE_HEADBUTT,
        6 => PedFightMoves::FIGHTMOVE_PUNCHJAB,
        7 => PedFightMoves::FIGHTMOVE_PUNCHHOOK,
        8 => PedFightMoves::FIGHTMOVE_KICK,
        9 => PedFightMoves::FIGHTMOVE_LONGKICK,
        10 => PedFightMoves::FIGHTMOVE_ROUNDHOUSE,
        11 => PedFightMoves::FIGHTMOVE_BODYBLOW,
        12 => PedFightMoves::FIGHTMOVE_GROUNDKICK,
        13 => PedFightMoves::FIGHTMOVE_HITFRONT,
        14 => PedFightMoves::FIGHTMOVE_HITBACK,
        15 => PedFightMoves::FIGHTMOVE_HITRIGHT,
        16 => PedFightMoves::FIGHTMOVE_HITLEFT,
        17 => PedFightMoves::FIGHTMOVE_HITBODY,
        18 => PedFightMoves::FIGHTMOVE_HITCHEST,
        19 => PedFightMoves::FIGHTMOVE_HITHEAD,
        20 => PedFightMoves::FIGHTMOVE_HITBIGSTEP,
        21 => PedFightMoves::FIGHTMOVE_HITONFLOOR,
        22 => PedFightMoves::FIGHTMOVE_HITBEHIND,
        23 => PedFightMoves::FIGHTMOVE_IDLE2NORM,
        _ => return None,
    })
}

pub fn fight_animation_ids(table: &CFightMoveTable) -> Vec<AnimationId> {
    let mut ids = vec![ANIM_STD_FIGHT_IDLE, ANIM_STD_FIGHT_2IDLE];
    ids.extend(
        table
            .moves
            .iter()
            .skip(1)
            .take(NUM_FIGHTMOVES - 1)
            .map(|fight_move| fight_move.anim_id),
    );
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Shared Bevy-side setup for re3 `CPed::StartFightAttack`.
///
/// Target selection remains with the ECS caller; this function owns the ped
/// state and authored animation associations that are common to player, co-op,
/// and the developer arena.
pub fn start_fight_attack(
    ped: &mut CPed, vitality: &crate::peds::PedVitality::PedVitality,
    clump: &mut CAnimBlendClumpData,
    anim_manager: &CAnimManager,
    table: &CFightMoveTable,
    initial_move: PedFightMoves,
    now_ms: u32,
) -> bool {
    if !ped.is_ped_in_control(vitality) || ped.attack_timer > now_ms {
        return false;
    }
    if ped.ped_state == PedState::PED_FIGHT {
        return true;
    }

    if ped.ped_state != PedState::PED_AIM_GUN {
        ped.last_ped_state = ped.ped_state;
    }
    ped.ped_state = PedState::PED_FIGHT;
    ped.move_state = eMoveState::PEDMOVE_NONE;
    ped.stored_move_state = eMoveState::PEDMOVE_NONE;
    ped.fight_button_pressure = 0;
    ped.fight_state = FIGHTSTATE_NO_MOVE;
    ped.take_step_after_attack = false;
    ped.fight_combo_move = false;
    ped.flags.is_attacking = true;

    clump
        .associations
        .retain(|association| association.flags & ASSOC_REPEAT == 0);
    for association in &mut clump.associations {
        if matches!(
            association.anim_id,
            ANIM_STD_STARTWALK | ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2
        ) {
            association.blend_delta = -1000.0;
            association.flags |= ASSOC_DELETEFADEDOUT;
        }
    }

    if let Some(mut idle) =
        anim_manager.create_anim_association(ASSOCGRP_STD, ANIM_STD_FIGHT_IDLE as usize)
    {
        idle.set_blend(1.0, 0.0);
        idle.start(0.0);
        clump.associations.push(idle);
    } else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            "StartFightAttack: ANIM_STD_FIGHT_IDLE is unavailable"
        );
    }

    if !start_fight_move(ped, clump, anim_manager, table, initial_move, 4.0, None) {
        end_fight(ped, clump, anim_manager, table, FightEndType::Fast);
        return false;
    }
    true
}

/// Start one authored fight move, matching the association ownership used by
/// `CPed::Fight` and `FinishFightMoveCB`.
pub fn start_fight_move(
    ped: &mut CPed,
    clump: &mut CAnimBlendClumpData,
    anim_manager: &CAnimManager,
    table: &CFightMoveTable,
    move_id: PedFightMoves,
    blend_delta: f32,
    start_fraction: Option<f32>,
) -> bool {
    let Some(fight_move) = table.get(move_id as usize) else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            move_id = move_id as usize,
            "fight move is absent from fistfite.dat"
        );
        return false;
    };
    let fight_ids = fight_animation_ids(table);
    for association in &mut clump.associations {
        if association.anim_id != ANIM_STD_FIGHT_IDLE
            && fight_ids.contains(&association.anim_id)
            && association.blend_delta >= 0.0
        {
            association.blend_delta = -8.0;
            association.flags |= ASSOC_DELETEFADEDOUT;
        }
    }

    let Some(mut association) =
        anim_manager.create_anim_association(ASSOCGRP_STD, fight_move.anim_id as usize)
    else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            anim_id = fight_move.anim_id,
            "fight animation is unavailable"
        );
        return false;
    };
    let start_time = start_fraction
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
        * association.hierarchy.total_length;
    association.set_blend(0.0, blend_delta);
    association.start(start_time);
    association.set_finish_callback();
    clump.associations.push(association);
    ped.cur_fight_move = move_id;
    ped.fight_state = FIGHTSTATE_NO_MOVE;
    true
}

/// Finish the active fight association and restore the pre-fight player state.
pub fn end_fight(
    ped: &mut CPed,
    clump: &mut CAnimBlendClumpData,
    anim_manager: &CAnimManager,
    table: &CFightMoveTable,
    end_type: FightEndType,
) {
    if ped.ped_state != PedState::PED_FIGHT {
        return;
    }
    let fight_ids = fight_animation_ids(table);
    for association in &mut clump.associations {
        if fight_ids.contains(&association.anim_id) {
            association.blend_delta = -8.0;
            association.flags |= ASSOC_DELETEFADEDOUT;
        }
    }
    let restored = match ped.last_ped_state {
        PedState::PED_FIGHT | PedState::PED_ATTACK | PedState::PED_AIM_GUN => PedState::PED_IDLE,
        state => state,
    };
    ped.ped_state = restored;
    ped.cur_fight_move = PedFightMoves::FIGHTMOVE_NULL;
    ped.fight_button_pressure = 0;
    ped.fight_state = FIGHTSTATE_MOVE_FINISHED;
    ped.take_step_after_attack = false;
    ped.fight_combo_move = false;
    ped.flags.is_attacking = false;
    ped.wait_timer = 0;

    let idle_group = usize::try_from(ped.anim_group).unwrap_or(0);
    if let Some(mut idle) =
        anim_manager.create_anim_association(idle_group, crate::animation::AnimationId::ANIM_STD_IDLE as usize)
    {
        idle.set_blend(
            0.0,
            if end_type == FightEndType::WithStep { 1.0 } else { 8.0 },
        );
        idle.start(0.0);
        clump.associations.push(idle);
    }
    let transition_id = if end_type == FightEndType::WithStep {
        ANIM_STD_STARTWALK
    } else {
        ANIM_STD_FIGHT_2IDLE
    };
    if let Some(mut transition) =
        anim_manager.create_anim_association(ASSOCGRP_STD, transition_id as usize)
    {
        transition.set_blend(0.0, 8.0);
        transition.speed = if end_type == FightEndType::Fast { 2.0 } else { 1.0 };
        transition.start(0.0);
        clump.associations.push(transition);
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Component)]
pub struct CPedFight {
    pub target: Option<Entity>,
    pub attack_timer_ms: u32,
    pub hit_counter: u8,
}

impl Default for CPedFight {
    fn default() -> Self {
        Self {
            target: None,
            attack_timer_ms: 0,
            hit_counter: 0,
        }
    }
}
