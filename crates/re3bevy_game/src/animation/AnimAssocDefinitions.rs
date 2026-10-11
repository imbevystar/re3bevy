//! re3 reference: references/re-GTA/src/animation/AnimManager.cpp
//!
//! Animation association definition metadata generated from re3 PC 1.1 tables.

use crate::animation::{AnimBlendAssociation::*, AnimationId::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimAssocDesc {
    pub anim_id: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimAssocDefinition {
    pub name: &'static str,
    pub block_name: &'static str,
    pub model_index: i32,
    pub anim_names: &'static [&'static str],
    pub anim_descs: &'static [AnimAssocDesc],
}

pub const MI_COP: i32 = 1;

pub type AssocGroupId = usize;

pub const ASSOCGRP_STD: AssocGroupId = 0;
pub const ASSOCGRP_PLAYER: AssocGroupId = 1;
pub const ASSOCGRP_PLAYERROCKET: AssocGroupId = 2;
pub const ASSOCGRP_PLAYER1ARMED: AssocGroupId = 3;
pub const ASSOCGRP_PLAYER2ARMED: AssocGroupId = 4;
pub const ASSOCGRP_PLAYERBBBAT: AssocGroupId = 5;
pub const ASSOCGRP_SHUFFLE: AssocGroupId = 6;
pub const ASSOCGRP_OLD: AssocGroupId = 7;
pub const ASSOCGRP_GANG1: AssocGroupId = 8;
pub const ASSOCGRP_GANG2: AssocGroupId = 9;
pub const ASSOCGRP_FAT: AssocGroupId = 10;
pub const ASSOCGRP_OLDFAT: AssocGroupId = 11;
pub const ASSOCGRP_WOMAN: AssocGroupId = 12;
pub const ASSOCGRP_WOMANSHOP: AssocGroupId = 13;
pub const ASSOCGRP_BUSYWOMAN: AssocGroupId = 14;
pub const ASSOCGRP_SEXYWOMAN: AssocGroupId = 15;
pub const ASSOCGRP_OLDWOMAN: AssocGroupId = 16;
pub const ASSOCGRP_FATWOMAN: AssocGroupId = 17;
pub const ASSOCGRP_PANICCHUNKY: AssocGroupId = 18;
pub const ASSOCGRP_PLAYERBACK: AssocGroupId = 19;
pub const ASSOCGRP_PLAYERLEFT: AssocGroupId = 20;
pub const ASSOCGRP_PLAYERRIGHT: AssocGroupId = 21;
pub const ASSOCGRP_ROCKETBACK: AssocGroupId = 22;
pub const ASSOCGRP_ROCKETLEFT: AssocGroupId = 23;
pub const ASSOCGRP_ROCKETRIGHT: AssocGroupId = 24;
pub const NUM_ANIM_ASSOC_GROUPS: usize = 25;

pub const A_STD_ANIM_DESCS: &[AnimAssocDesc] = &[
    AnimAssocDesc {
        anim_id: ANIM_STD_WALK,
        flags: ASSOC_REPEAT | ASSOC_MOVEMENT | ASSOC_HAS_TRANSLATION | ASSOC_WALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUN,
        flags: ASSOC_REPEAT | ASSOC_MOVEMENT | ASSOC_HAS_TRANSLATION | ASSOC_WALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUNFAST,
        flags: ASSOC_REPEAT | ASSOC_MOVEMENT | ASSOC_HAS_TRANSLATION | ASSOC_WALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE,
        flags: ASSOC_REPEAT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_STARTWALK,
        flags: ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUNSTOP1,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUNSTOP2,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE_CAM,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE_HBHB,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE_TIRED,
        flags: ASSOC_REPEAT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE_BIGGUN,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CHAT,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HAILTAXI,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_FRONT,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_LEFT,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_BACK,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_RIGHT,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_FACE,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_STOMACH,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_ARM_L,
        flags: ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_ARM_R,
        flags: ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_LEG_L,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KO_SHOT_LEG_R,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_SPINFORWARD_LEFT,
        flags: ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_SPINFORWARD_RIGHT,
        flags: ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIGHIMPACT_FRONT,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIGHIMPACT_LEFT,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIGHIMPACT_BACK,
        flags: ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIGHIMPACT_RIGHT,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HITBYGUN_FRONT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HITBYGUN_LEFT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HITBYGUN_BACK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HITBYGUN_RIGHT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_FRONT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_LEFT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_BACK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_RIGHT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_FLOOR,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_BODYBLOW,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_CHEST,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_HEAD,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_WALK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_WALL,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_FLOOR_FRONT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HIT_BEHIND,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PUNCH,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_KICKGROUND,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_BAT_H,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_BAT_V,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_HGUN_BODY,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_AK_BODY,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_PUMP,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_SNIPER,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_WEAPON_THROW,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_THROW_UNDER,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_START_THROW,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_DETONATE,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HGUN_RELOAD,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_AK_RELOAD,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_PUNCH,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_BAT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_UZI,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_PUMP,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_AK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_M16,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_FPS_ROCKET,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_IDLE,
        flags: ASSOC_REPEAT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_2IDLE,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_SHUFFLE_F,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_BODYBLOW,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_HEAD,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_KICK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_KNEE,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_LHOOK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_PUNCH,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_ROUNDHOUSE,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FIGHT_LONGKICK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PARTIAL_PUNCH,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JACKEDCAR_RHS,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JACKEDCAR_LO_RHS,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JACKEDCAR_LHS,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JACKEDCAR_LO_LHS,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_QUICKJACK,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_QUICKJACKED,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_ALIGN_DOOR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_ALIGNHI_DOOR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_OPEN_DOOR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CARDOOR_LOCKED_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_PULL_OUT_PED_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_PULL_OUT_PED_LO_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_GET_IN_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_GET_IN_LO_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_LO_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GETOUT_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GETOUT_LO_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_ALIGN_DOOR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_OPEN_DOOR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CARDOOR_LOCKED_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_PULL_OUT_PED_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_PULL_OUT_PED_LO_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_GET_IN_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_GET_IN_LO_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_DOOR_LO_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SHUFFLE_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SHUFFLE_LO_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SIT,
        flags: ASSOC_DELETEFADEDOUT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SIT_LO,
        flags: ASSOC_DELETEFADEDOUT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SIT_P,
        flags: ASSOC_DELETEFADEDOUT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_SIT_P_LO,
        flags: ASSOC_DELETEFADEDOUT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVE_LEFT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVE_RIGHT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVE_LEFT_LO,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVE_RIGHT_LO,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVEBY_LEFT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_DRIVEBY_RIGHT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_LOOKBEHIND,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_BOAT_DRIVE,
        flags: ASSOC_DELETEFADEDOUT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GETOUT_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GETOUT_LO_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_CLOSE_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CAR_HOOKERTALK,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_COACH_OPEN_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_COACH_OPEN_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_COACH_GET_IN_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_COACH_GET_IN_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_COACH_GET_OUT_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_TRAIN_GETIN,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_TRAIN_GETOUT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CRAWLOUT_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_CRAWLOUT_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_OPEN_DOOR_REAR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_GET_IN_REAR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_GET_OUT_REAR_LHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_OPEN_DOOR_REAR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_GET_IN_REAR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_VAN_GET_OUT_REAR_RHS,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GET_UP,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GET_UP_LEFT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GET_UP_RIGHT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_GET_UP_FRONT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JUMP_LAUNCH,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JUMP_GLIDE,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_JUMP_LAND,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FALL,
        flags: ASSOC_REPEAT | ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FALL_GLIDE,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FALL_LAND,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_FALL_COLLAPSE,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_EVADE_STEP,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_EVADE_DIVE,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION | ASSOC_FRONTAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_XPRESS_SCRATCH,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_IDLE,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_ROADCROSS,
        flags: ASSOC_REPEAT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_TURN180,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_ARREST,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_DROWN,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_MEDIC_CPR,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_DUCK_DOWN,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_DUCK_LOW,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RBLOCK_SHOOT,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_THROW_UNDER2,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HANDSUP,
        flags: ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_HANDSCOWER,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PARTIAL_FUCKU,
        flags: ASSOC_DELETEFADEDOUT | ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL | ASSOC_NOWALK,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PHONE_IN,
        flags: ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PHONE_OUT,
        flags: ASSOC_FADEOUTWHENDONE | ASSOC_PARTIAL,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_PHONE_TALK,
        flags: ASSOC_REPEAT | ASSOC_DELETEFADEDOUT | ASSOC_PARTIAL,
    },
];

pub const A_STD_ANIM_DESCS_SIDE: &[AnimAssocDesc] = &[
    AnimAssocDesc {
        anim_id: ANIM_STD_WALK,
        flags: ASSOC_REPEAT
            | ASSOC_MOVEMENT
            | ASSOC_HAS_TRANSLATION
            | ASSOC_WALK
            | ASSOC_HAS_X_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUN,
        flags: ASSOC_REPEAT
            | ASSOC_MOVEMENT
            | ASSOC_HAS_TRANSLATION
            | ASSOC_WALK
            | ASSOC_HAS_X_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_RUNFAST,
        flags: ASSOC_REPEAT
            | ASSOC_MOVEMENT
            | ASSOC_HAS_TRANSLATION
            | ASSOC_WALK
            | ASSOC_HAS_X_TRANSLATION,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_IDLE,
        flags: ASSOC_REPEAT,
    },
    AnimAssocDesc {
        anim_id: ANIM_STD_STARTWALK,
        flags: ASSOC_HAS_TRANSLATION | ASSOC_HAS_X_TRANSLATION,
    },
];

pub const A_STD_ANIMATIONS: &[&str] = &[
    "walk_civi",
    "run_civi",
    "sprint_panic",
    "idle_stance",
    "walk_start",
    "run_stop",
    "run_stopR",
    "idle_cam",
    "idle_hbhb",
    "idle_tired",
    "idle_armed",
    "idle_chat",
    "idle_taxi",
    "KO_shot_front",
    "KO_shot_front",
    "KO_shot_front",
    "KO_shot_front",
    "KO_shot_face",
    "KO_shot_stom",
    "KO_shot_arml",
    "KO_shot_armR",
    "KO_shot_legl",
    "KO_shot_legR",
    "KD_left",
    "KD_right",
    "KO_skid_front",
    "KO_spin_R",
    "KO_skid_back",
    "KO_spin_L",
    "SHOT_partial",
    "SHOT_leftP",
    "SHOT_partial",
    "SHOT_rightP",
    "HIT_front",
    "HIT_L",
    "HIT_back",
    "HIT_R",
    "FLOOR_hit",
    "HIT_bodyblow",
    "HIT_chest",
    "HIT_head",
    "HIT_walk",
    "HIT_wall",
    "FLOOR_hit_f",
    "HIT_behind",
    "punchR",
    "KICK_floor",
    "WEAPON_bat_h",
    "WEAPON_bat_v",
    "WEAPON_hgun_body",
    "WEAPON_AK_body",
    "WEAPON_pump",
    "WEAPON_sniper",
    "WEAPON_throw",
    "WEAPON_throwu",
    "WEAPON_start_throw",
    "bomber",
    "WEAPON_hgun_rload",
    "WEAPON_AK_rload",
    "FPS_PUNCH",
    "FPS_BAT",
    "FPS_UZI",
    "FPS_PUMP",
    "FPS_AK",
    "FPS_M16",
    "FPS_ROCKET",
    "FIGHTIDLE",
    "FIGHT2IDLE",
    "FIGHTsh_F",
    "FIGHTbodyblow",
    "FIGHThead",
    "FIGHTkick",
    "FIGHTknee",
    "FIGHTLhook",
    "FIGHTpunch",
    "FIGHTrndhse",
    "FIGHTlngkck",
    "FIGHTppunch",
    "car_jackedRHS",
    "car_LjackedRHS",
    "car_jackedLHS",
    "car_LjackedLHS",
    "CAR_Qjack",
    "CAR_Qjacked",
    "CAR_align_LHS",
    "CAR_alignHI_LHS",
    "CAR_open_LHS",
    "CAR_doorlocked_LHS",
    "CAR_pullout_LHS",
    "CAR_pulloutL_LHS",
    "CAR_getin_LHS",
    "CAR_getinL_LHS",
    "CAR_closedoor_LHS",
    "CAR_closedoorL_LHS",
    "CAR_rolldoor",
    "CAR_rolldoorLO",
    "CAR_getout_LHS",
    "CAR_getoutL_LHS",
    "CAR_close_LHS",
    "CAR_align_RHS",
    "CAR_alignHI_RHS",
    "CAR_open_RHS",
    "CAR_doorlocked_RHS",
    "CAR_pullout_RHS",
    "CAR_pulloutL_RHS",
    "CAR_getin_RHS",
    "CAR_getinL_RHS",
    "CAR_closedoor_RHS",
    "CAR_closedoorL_RHS",
    "CAR_shuffle_RHS",
    "CAR_Lshuffle_RHS",
    "CAR_sit",
    "CAR_Lsit",
    "CAR_sitp",
    "CAR_sitpLO",
    "DRIVE_L",
    "Drive_R",
    "Drive_LO_l",
    "Drive_LO_R",
    "Driveby_L",
    "Driveby_R",
    "CAR_LB",
    "DRIVE_BOAT",
    "CAR_getout_RHS",
    "CAR_getoutL_RHS",
    "CAR_close_RHS",
    "car_hookertalk",
    "COACH_opnL",
    "COACH_opnR",
    "COACH_inL",
    "COACH_inR",
    "COACH_outL",
    "TRAIN_getin",
    "TRAIN_getout",
    "CAR_crawloutRHS",
    "CAR_crawloutRHS",
    "VAN_openL",
    "VAN_getinL",
    "VAN_closeL",
    "VAN_getoutL",
    "VAN_open",
    "VAN_getin",
    "VAN_close",
    "VAN_getout",
    "Getup",
    "Getup",
    "Getup",
    "Getup_front",
    "JUMP_launch",
    "JUMP_glide",
    "JUMP_land",
    "FALL_fall",
    "FALL_glide",
    "FALL_land",
    "FALL_collapse",
    "EV_step",
    "EV_dive",
    "XPRESSscratch",
    "roadcross",
    "TURN_180",
    "ARRESTgun",
    "DROWN",
    "CPR",
    "DUCK_down",
    "DUCK_low",
    "RBLOCK_Cshoot",
    "WEAPON_throwu",
    "handsup",
    "handsCOWER",
    "FUCKU",
    "PHONE_in",
    "PHONE_out",
    "PHONE_talk",
];

pub const A_PLAYER_ANIMATIONS: &[&str] = &[
    "walk_player",
    "run_player",
    "SPRINT_civi",
    "IDLE_STANCE",
    "walk_start",
];

pub const A_PLAYER_WITH_ROCKET_ANIMATIONS: &[&str] = &[
    "walk_rocket",
    "run_rocket",
    "run_rocket",
    "idle_rocket",
    "walk_start_rocket",
];

pub const A_PLAYER_1_ARMED_ANIMATIONS: &[&str] = &[
    "walk_player",
    "run_1armed",
    "SPRINT_civi",
    "IDLE_STANCE",
    "walk_start",
];

pub const A_PLAYER_2_ARMED_ANIMATIONS: &[&str] = &[
    "walk_player",
    "run_armed",
    "run_armed",
    "idle_stance",
    "walk_start",
];

pub const A_PLAYER_BBBAT_ANIMATIONS: &[&str] = &[
    "walk_player",
    "run_player",
    "run_player",
    "IDLE_STANCE",
    "walk_start",
];

pub const A_SHUFFLE_ANIMATIONS: &[&str] =
    &["WALK_shuffle", "RUN_civi", "SPRINT_civi", "IDLE_STANCE"];

pub const A_OLD_ANIMATIONS: &[&str] = &["walk_old", "run_civi", "sprint_civi", "idle_stance"];

pub const A_GANG1_ANIMATIONS: &[&str] = &["walk_gang1", "run_gang1", "sprint_civi", "idle_stance"];

pub const A_GANG2_ANIMATIONS: &[&str] = &["walk_gang2", "run_gang1", "sprint_civi", "idle_stance"];

pub const A_FAT_ANIMATIONS: &[&str] = &["walk_fat", "run_civi", "woman_runpanic", "idle_stance"];

pub const A_OLD_FAT_ANIMATIONS: &[&str] =
    &["walk_fatold", "run_fatold", "woman_runpanic", "idle_stance"];

pub const A_STD_WOMAN_ANIMATIONS: &[&str] = &[
    "woman_walknorm",
    "woman_run",
    "woman_runpanic",
    "woman_idlestance",
];

pub const A_WOMAN_SHOP_ANIMATIONS: &[&str] = &[
    "woman_walkshop",
    "woman_run",
    "woman_run",
    "woman_idlestance",
];

pub const A_BUSY_WOMAN_ANIMATIONS: &[&str] = &[
    "woman_walkbusy",
    "woman_run",
    "woman_runpanic",
    "woman_idlestance",
];

pub const A_SEXY_WOMAN_ANIMATIONS: &[&str] = &[
    "woman_walksexy",
    "woman_run",
    "woman_runpanic",
    "woman_idlestance",
];

pub const A_OLD_WOMAN_ANIMATIONS: &[&str] = &[
    "woman_walkold",
    "woman_run",
    "woman_runpanic",
    "woman_idlestance",
];

pub const A_FAT_WOMAN_ANIMATIONS: &[&str] = &[
    "walk_fat",
    "woman_run",
    "woman_runpanic",
    "woman_idlestance",
];

pub const A_PANIC_CHUNKY_ANIMATIONS: &[&str] = &[
    "run_fatold",
    "woman_runpanic",
    "woman_runpanic",
    "idle_stance",
];

pub const A_PLAYER_STRAFE_BACK_ANIMATIONS: &[&str] = &[
    "walk_player_back",
    "run_player_back",
    "run_player_back",
    "IDLE_STANCE",
    "walk_start_back",
];

pub const A_PLAYER_STRAFE_LEFT_ANIMATIONS: &[&str] = &[
    "walk_player_left",
    "run_left",
    "run_left",
    "IDLE_STANCE",
    "walk_start_left",
];

pub const A_PLAYER_STRAFE_RIGHT_ANIMATIONS: &[&str] = &[
    "walk_player_right",
    "run_right",
    "run_right",
    "IDLE_STANCE",
    "walk_start_right",
];

pub const A_ROCKET_STRAFE_BACK_ANIMATIONS: &[&str] = &[
    "walk_rocket_back",
    "run_rocket_back",
    "run_rocket_back",
    "idle_rocket",
    "walkst_rocket_back",
];

pub const A_ROCKET_STRAFE_LEFT_ANIMATIONS: &[&str] = &[
    "walk_rocket_left",
    "run_rocket_left",
    "run_rocket_left",
    "idle_rocket",
    "walkst_rocket_left",
];

pub const A_ROCKET_STRAFE_RIGHT_ANIMATIONS: &[&str] = &[
    "walk_rocket_right",
    "run_rocket_right",
    "run_rocket_right",
    "idle_rocket",
    "walkst_rocket_right",
];

pub const MS_A_ANIM_ASSOC_DEFINITIONS: &[AnimAssocDefinition] = &[
    AnimAssocDefinition {
        name: "man",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_STD_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "player",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "playerrocket",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_WITH_ROCKET_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "player1armed",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_1_ARMED_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "player2armed",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_2_ARMED_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "playerBBBat",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_BBBAT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "shuffle",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_SHUFFLE_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "oldman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_OLD_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "gang1",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_GANG1_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "gang2",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_GANG2_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "fatman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_FAT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "oldfatman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_OLD_FAT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "woman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_STD_WOMAN_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "shopping",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_WOMAN_SHOP_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "busywoman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_BUSY_WOMAN_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "sexywoman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_SEXY_WOMAN_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "oldwoman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_OLD_WOMAN_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "fatwoman",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_FAT_WOMAN_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "panicchunky",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PANIC_CHUNKY_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "playerback",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_STRAFE_BACK_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "playerleft",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_STRAFE_LEFT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS_SIDE,
    },
    AnimAssocDefinition {
        name: "playerright",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_PLAYER_STRAFE_RIGHT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS_SIDE,
    },
    AnimAssocDefinition {
        name: "rocketback",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_ROCKET_STRAFE_BACK_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS,
    },
    AnimAssocDefinition {
        name: "rocketleft",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_ROCKET_STRAFE_LEFT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS_SIDE,
    },
    AnimAssocDefinition {
        name: "rocketright",
        block_name: "ped",
        model_index: MI_COP,
        anim_names: A_ROCKET_STRAFE_RIGHT_ANIMATIONS,
        anim_descs: A_STD_ANIM_DESCS_SIDE,
    },
];
