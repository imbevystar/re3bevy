//! re3 reference: references/re-GTA/src/peds/CivilianPed.cpp
//! re3 reference: references/re-GTA/src/peds/CivilianPed.h
//!
//! Civilian ped subtype. re3 adds no fields beyond `CPed`; behaviour is in
//! `CivilianAI`/`ProcessControl`, not in an invented civilian state payload.

use bevy::prelude::*;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use super::{Ped::CPed, PedType::ePedType};
use crate::{
    collision::Collision::CCollision,
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
    core::{
        EventList::{CEventList, EEventEntity, EEventType},
        FileLoader::CFileLoader,
        Pools::CPoolHandle,
    },
    peds::{
        Ped::{eMoveState, eObjective, script_ped_can_see_entity, RANDOM_CHAR, MISSION_CHAR},
        PedLocomotion::PedLocomotion,
        PedType::{CPedStatsRuntime, CPedTypeRuntime, PED_FLAG_EXPLOSION, PED_FLAG_GUN},
        PedVitality::PedVitality,
        PlayerPed::PlayerPedState,
        PlayerRead::CWorldPlayersRead,
        VehicleRelations::InVehicle,
    },
};

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CCivilianPed {
    pub ped: CPed,
}

impl CCivilianPed {
    pub fn new(ped_type: ePedType, model_index: usize, general: &mut CGeneral) -> Self {
        let mut ped = CPed::new(ped_type, general);
        ped.physical.entity.set_model_index_no_create(model_index);
        ped.near_peds.fill(None);
        Self { ped }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CivilianThreat {
    slot: usize,
    position: CVector,
    ped_type: ePedType,
    temper: i8,
    melee: bool,
    is_player: bool,
    wanted_cops: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CivilianReaction {
    Fight,
    Flee { duration_ms: u32, movement: eMoveState },
    None,
}

#[derive(Clone, Copy)]
pub(crate) struct CivilianIntent {
    entity: Entity,
    threat: Option<CivilianThreat>,
    source: CVector,
    reaction: CivilianReaction,
    gunshot: bool,
}

fn is_gang(ped_type: ePedType) -> bool {
    (ePedType::PEDTYPE_GANG1 as u8..=ePedType::PEDTYPE_GANG9 as u8)
        .contains(&(ped_type as u8))
}

/// CCivilianPed::CivilianAI's two ped-threat branches. C++ uses the authored
/// PEDSTATS.DAT personality, not a global hostile-to-player flag.
fn react_to_ped(
    gunshot: bool,
    fear: i8,
    temper: i8,
    lawfulness: i8,
    our_melee: bool,
    threat: CivilianThreat,
    distance_sq: f32,
) -> CivilianReaction {
    let flee = |duration_ms, movement| CivilianReaction::Flee { duration_ms, movement };
    if gunshot {
        if fear <= lawfulness {
            if temper <= fear {
                return if distance_sq < 100.0 {
                    flee(10_000, eMoveState::PEDMOVE_RUN)
                } else {
                    flee(5_000, eMoveState::PEDMOVE_RUN)
                };
            }
            if our_melee {
                return flee(5_000, if distance_sq < 400.0 { eMoveState::PEDMOVE_RUN } else { eMoveState::PEDMOVE_WALK });
            }
            if threat.is_player && threat.wanted_cops {
                return flee(5_000, if distance_sq < 100.0 { eMoveState::PEDMOVE_RUN } else { eMoveState::PEDMOVE_WALK });
            }
            return CivilianReaction::Fight;
        }
        return if distance_sq < 100.0 {
            flee(10_000, eMoveState::PEDMOVE_SPRINT)
        } else {
            flee(5_000, eMoveState::PEDMOVE_RUN)
        };
    }
    if i32::from(fear) <= 100 - i32::from(threat.temper)
        && threat.ped_type != ePedType::PEDTYPE_COP
    {
        if threat.melee || !our_melee {
            // In re3 a civilian without an existing kill objective does not
            // start another fight with a player already pursued by police.
            if threat.is_player && threat.wanted_cops { return CivilianReaction::None; }
            return CivilianReaction::Fight;
        }
        return CivilianReaction::None;
    }
    flee(10_000, eMoveState::PEDMOVE_RUN)
}

/// Scan the native near-ped cache and live CEventList, then commit only the
/// decisions to canonical ped components. Script/mission objectives retain
/// priority; generated civilians run this after their 16-frame near list pass.
pub(crate) fn process_civilian_ai(
    mut peds: ParamSet<(
        Query<(Entity, &ScriptPedIdentity, &CPed, &PedVitality, Has<InVehicle>), Without<PlayerPedState>>,
        Query<(&mut CPed, &PedVitality, &mut PedLocomotion), Without<PlayerPedState>>,
    )>,
    players: CWorldPlayersRead,
    index: Res<ScriptEntityIndex>,
    types: Res<CPedTypeRuntime>,
    stats: Res<CPedStatsRuntime>,
    events: Res<CEventList>,
    loader: Res<CFileLoader>,
    collision: Res<CCollision>,
    timer: Res<CTimer>,
    mut intents: Local<Vec<CivilianIntent>>,
) {
    let now = timer.time_in_milliseconds();
    intents.clear();
    {
        let read = peds.p0();
        let resolve = |handle: i32| -> Option<CivilianThreat> {
            if let Some(entity) = index.ped(handle)
                && let Ok((_, identity, ped, vitality, _)) = read.get(entity)
                && identity.handle == handle && vitality.health > 0.0
            {
                return Some(CivilianThreat {
                    slot: CPoolHandle(handle).index(),
                    position: ped.physical.entity.get_position(),
                    ped_type: ped.ped_type,
                    temper: stats.GetPedStats(ped.ped_stats).m_temper,
                    melee: ped.weapons[ped.current_weapon as usize].IsTypeMelee(),
                    is_player: false,
                    wanted_cops: false,
                });
            }
            let player = players.iter().find(|player| player.pool_handle == handle)?;
            if player.vitality.health <= 0.0 { return None; }
            Some(CivilianThreat {
                slot: CPoolHandle(handle).index(),
                position: player.position(),
                ped_type: player.ped.ped_type,
                temper: stats.GetPedStats(player.ped.ped_stats).m_temper,
                melee: player.ped.weapons[player.ped.current_weapon as usize].IsTypeMelee(),
                is_player: true,
                wanted_cops: player.state.wanted.m_CurrentCops != 0,
            })
        };

        for (entity, identity, ped, vitality, seated) in &read {
            if index.ped(identity.handle) != Some(entity)
                || ped.char_created_by != RANDOM_CHAR || seated
                || !ped.is_ped_in_control(vitality)
                || !ped.flags.responds_to_threats
                || ped.objective != eObjective::OBJECTIVE_NONE
                || now <= ped.flee_timer
                || (ped.physical.zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
                    && ped.physical.zone_level != collision.collision_in_memory as i8)
                || matches!(ped.ped_type, ePedType::PEDTYPE_COP | ePedType::PEDTYPE_EMERGENCY | ePedType::PEDTYPE_FIREMAN)
            { continue; }

            let position = ped.physical.entity.get_position();
            if ped.fear_flags & PED_FLAG_EXPLOSION != 0
                && let Some((slot, _)) = events.find_closest_event(EEventType::Explosion, position)
            {
                let source = events.events[slot].position;
                if (source.x - position.x).hypot(source.y - position.y) < 20.0 {
                    intents.push(CivilianIntent { entity, threat: None, source,
                        reaction: CivilianReaction::Flee { duration_ms: 2_000, movement: eMoveState::PEDMOVE_RUN }, gunshot: false });
                    continue;
                }
            }

            let mut detected = None;
            if ped.fear_flags & PED_FLAG_GUN != 0
                && let Some((slot, _)) = events.find_closest_event(EEventType::Gunshot, position)
            {
                let event = &events.events[slot];
                if event.entity_type == EEventEntity::Ped
                    && let Some(shooter) = resolve(event.entity_ref)
                    && shooter.slot != CPoolHandle(identity.handle).index()
                    && (ped.ped_type != shooter.ped_type
                        || matches!(ped.ped_type, ePedType::PEDTYPE_CIVMALE | ePedType::PEDTYPE_CIVFEMALE))
                {
                    if !is_gang(ped.ped_type) {
                        detected = Some((shooter, true));
                    } else if types.GetFlag(shooter.ped_type as usize) & ped.fear_flags != 0 {
                        detected = Some((shooter, false));
                    }
                }
            }

            if detected.is_none() {
                let mut closest_sq = 60.0_f32.powi(2);
                for &handle in ped.near_peds.iter().flatten() {
                    let Some(other) = resolve(handle) else { continue; };
                    if !other.is_player && index.ped(handle).and_then(|entity| read.get(entity).ok())
                        .is_some_and(|(_, _, actor, _, _)| actor.char_created_by == MISSION_CHAR)
                    { continue; }
                    if types.GetFlag(other.ped_type as usize) & ped.fear_flags == 0 { continue; }
                    let dx = other.position.x - position.x;
                    let dy = other.position.y - position.y;
                    let distance_sq = dx * dx + dy * dy;
                    if distance_sq >= closest_sq || !script_ped_can_see_entity(
                        position, ped.physical.entity.get_forward(), other.position, &loader,
                    ) { continue; }
                    closest_sq = distance_sq;
                    detected = Some((other, false));
                }
            }
            let Some((threat, gunshot)) = detected else { continue; };
            let dx = threat.position.x - position.x;
            let dy = threat.position.y - position.y;
            let personality = stats.GetPedStats(ped.ped_stats);
            let reaction = react_to_ped(gunshot, personality.m_fear, personality.m_temper,
                personality.m_lawfulness, ped.weapons[ped.current_weapon as usize].IsTypeMelee(),
                threat, dx * dx + dy * dy);
            if reaction != CivilianReaction::None {
                intents.push(CivilianIntent { entity, threat: Some(threat), source: threat.position, reaction, gunshot });
            }
        }
    }

    let mut write = peds.p1();
    for intent in intents.iter().copied() {
        let Ok((mut ped, vitality, mut locomotion)) = write.get_mut(intent.entity) else { continue; };
        if ped.objective != eObjective::OBJECTIVE_NONE || !ped.is_ped_in_control(vitality) { continue; }
        if let Some(threat) = intent.threat {
            ped.threat_entity = Some(threat.slot);
            if intent.gunshot && now > ped.look_timer {
                ped.look_target = Some(threat.slot);
                ped.flags.is_looking = true;
                ped.look_timer = now.wrapping_add(500);
            }
        }
        match intent.reaction {
            CivilianReaction::Fight => {
                if let Some(threat) = intent.threat {
                    ped.set_objective_entity(eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT, threat.slot, now, None);
                    ped.flags.is_running = true;
                }
            }
            CivilianReaction::Flee { duration_ms, movement } => {
                if ped.set_flee_from_position(vitality, &mut locomotion, intent.source, duration_ms, now) {
                    ped.move_state = movement;
                    ped.flags.is_running = true;
                }
            }
            CivilianReaction::None => {}
        }
    }
}

#[cfg(test)]
mod civilian_ai_tests {
    use super::*;

    fn threat() -> CivilianThreat {
        CivilianThreat { slot: 1, position: CVector::ZERO, ped_type: ePedType::PEDTYPE_PLAYER1,
            temper: 60, melee: false, is_player: true, wanted_cops: false }
    }

    #[test]
    fn gunshots_split_panic_from_armed_retaliation() {
        assert!(matches!(react_to_ped(true, 80, 20, 40, false, threat(), 25.0), CivilianReaction::Flee { .. }));
        assert_eq!(react_to_ped(true, 20, 80, 70, false, threat(), 25.0), CivilianReaction::Fight);
        assert!(matches!(react_to_ped(true, 20, 80, 70, true, threat(), 25.0), CivilianReaction::Flee { .. }));
    }

    #[test]
    fn visible_threat_respects_temper_and_police_pursuit() {
        let hostile = CivilianThreat { melee: true, ..threat() };
        assert_eq!(react_to_ped(false, 25, 60, 40, true, hostile, 25.0), CivilianReaction::Fight);
        let pursued = CivilianThreat { wanted_cops: true, ..hostile };
        assert_eq!(react_to_ped(false, 25, 60, 40, true, pursued, 25.0), CivilianReaction::None);
        assert!(matches!(react_to_ped(false, 95, 20, 40, true, hostile, 25.0), CivilianReaction::Flee { .. }));
    }
}
