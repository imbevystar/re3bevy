//! re3 reference: references/re-GTA/src/peds/PedChat.h
//!
//! Ped speech/chat request queue.

use bevy::prelude::*;
use rust_gta_3_core::core::{General::CGeneral, Timer::CTimer};

use crate::{
    control::{
        Script::queue_ped_script_say,
        ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
    },
    core::{FileLoader::CFileLoader, Pools::CPoolHandle},
    peds::{
        Ped::{CPed, PedState, RANDOM_CHAR, eMoveState, set_script_ped_chat},
        PedLocomotion::PedLocomotion,
        PedType::ePedType,
        PedVitality::PedVitality,
        PlayerPed::PlayerPedState,
        VehicleRelations::InVehicle,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CPedChatLine {
    pub ped: Entity,
    pub text_key: String,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Resource)]
pub struct CPedChat {
    pub lines: Vec<CPedChatLine>,
}

impl CPedChat {
    #[allow(non_snake_case)]
    pub fn Add(&mut self, ped: Entity, text_key: impl Into<String>, priority: i32) {
        self.lines.push(CPedChatLine {
            ped,
            text_key: text_key.into(),
            priority,
        });
    }
}

/// Reusable membership lists only. Ped poses, timers and states are always
/// borrowed from their live components, including changes made earlier in this
/// pass. Candidates come from CPed's canonical generation-bearing near list.
#[derive(Default)]
pub(crate) struct AmbientPedChatScratch {
    actors: Vec<(i32, Entity)>,
}

fn ambient_chat_types_compatible(first: ePedType, second: ePedType) -> bool {
    use ePedType::*;
    if first == second {
        return true;
    }
    if first == PEDTYPE_CRIMINAL {
        return false;
    }
    let first_gang = matches!(
        first,
        PEDTYPE_GANG1
            | PEDTYPE_GANG2
            | PEDTYPE_GANG3
            | PEDTYPE_GANG4
            | PEDTYPE_GANG5
            | PEDTYPE_GANG6
            | PEDTYPE_GANG7
            | PEDTYPE_GANG8
            | PEDTYPE_GANG9
    );
    let second_gang = matches!(
        second,
        PEDTYPE_GANG1
            | PEDTYPE_GANG2
            | PEDTYPE_GANG3
            | PEDTYPE_GANG4
            | PEDTYPE_GANG5
            | PEDTYPE_GANG6
            | PEDTYPE_GANG7
            | PEDTYPE_GANG8
            | PEDTYPE_GANG9
    );
    !(first_gang || second_gang)
}

fn ambient_ped_will_chat(ped: &CPed, file_loader: &CFileLoader) -> bool {
    if ped.physical.surface_touched == crate::core::SurfaceTable::eSurfaceType::SURFACE_TARMAC as u8
    {
        return false;
    }
    if let (Some(next), Some(last)) = (ped.next_path_node, ped.last_path_node)
        && next != last
        && file_loader.path_find.test_crosses_road(next, last)
    {
        return false;
    }
    true
}

fn clear_ambient_ped_chat(ped: &mut CPed, now_ms: u32) {
    ped.flags.is_talking = false;
    ped.flags.is_looking = false;
    ped.flags.keep_trying_to_look = false;
    ped.look_target = None;
    ped.look_timer = 0;
    let previous = ped.last_ped_state;
    ped.last_ped_state = PedState::PED_NONE;
    ped.ped_state = if previous == PedState::PED_NONE || previous == PedState::PED_CHAT {
        PedState::PED_WANDER_PATH
    } else {
        previous
    };
    ped.move_state = if ped.ped_state == PedState::PED_WANDER_PATH {
        eMoveState::PEDMOVE_WALK
    } else {
        ped.stored_move_state
    };
    ped.chat_timer = now_ms.saturating_add(30_000);
}

/// Ambient `CPed::Chat` and wander-path chat discovery. This domain mutates
/// only CPed/heading; health, seating and the script index remain read-only.
/// Native near-ped ordering/admission is shared with the nearby domain.
pub(crate) fn process_ambient_ped_chat(
    index: Res<ScriptEntityIndex>,
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &PedVitality,
            &mut PedLocomotion,
            Option<&InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    file_loader: Res<CFileLoader>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut scratch: Local<AmbientPedChatScratch>,
) {
    let now_ms = timer.time_in_milliseconds();
    let scratch = &mut *scratch;
    scratch.actors.clear();

    for (entity, identity, ped, vitality, _, seated) in peds.iter() {
        if index.ped(identity.handle) != Some(entity)
            || ped.char_created_by != RANDOM_CHAR
            || vitality.health <= 0.0
            || seated.is_some()
        {
            continue;
        }
        scratch.actors.push((identity.handle, entity));
    }
    scratch.actors.sort_unstable_by_key(|(handle, _)| *handle);

    for &(_, actor) in &scratch.actors {
        let Ok((_, _, ped, _, _, _)) = peds.get(actor) else {
            continue;
        };
        if ped.ped_state != PedState::PED_CHAT {
            continue;
        }
        let position = ped.physical.entity.get_position();
        let chat_expired = now_ms > ped.chat_timer;
        // CPed's compatibility look field is a native pool slot. Resolve it
        // through the canonical index, then check the current ECS identity.
        let partner = ped
            .look_target
            .and_then(|slot| index.ped_handle_for_slot(slot))
            .and_then(|handle| Some((handle, index.ped(handle)?)))
            .and_then(|(handle, entity)| {
                let (_, identity, ped, _, _, _) = peds.get(entity).ok()?;
                (identity.handle == handle && ped.ped_state == PedState::PED_CHAT)
                    .then_some(ped.physical.entity.get_position())
            });
        let Ok((_, _, mut ped, _, mut locomotion, _)) = peds.get_mut(actor) else {
            continue;
        };
        if chat_expired || partner.is_none() {
            clear_ambient_ped_chat(&mut ped, now_ms);
            continue;
        }
        let Some(partner_position) = partner else {
            continue;
        };
        locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
            partner_position.x,
            partner_position.y,
            position.x,
            position.y,
        );
        if ped.flags.is_talking && general.get_random_number() < 512 {
            ped.flags.is_talking = false;
        } else {
            ped.flags.is_talking = true;
            queue_ped_script_say(&mut ped, crate::audio::soundlist::SOUND_PED_CHAT as i16);
        }
    }

    for &(handle, actor) in &scratch.actors {
        let Ok((_, _, ped, _, _, _)) = peds.get(actor) else {
            continue;
        };
        if ped.ped_state != PedState::PED_WANDER_PATH
            || now_ms <= ped.chat_timer
            || !ambient_ped_will_chat(ped, &file_loader)
        {
            continue;
        }
        let position = ped.physical.entity.get_position();
        let mut selected = None;
        let mut rejected_distance = false;
        // GTA III considers every second member of the canonical near list.
        // A compatible but distant neighbor postpones chat for 30 seconds;
        // looking only in adjacent 2 m cells removed that native admission.
        for other_handle in ped
            .near_peds
            .iter()
            .take(usize::from(ped.num_near_peds))
            .step_by(2)
            .flatten()
        {
            let Some(other) = index.ped(*other_handle) else {
                continue;
            };
            if other == actor {
                continue;
            }
            let Ok((_, other_identity, other_ped, _, _, _)) = peds.get(other) else {
                continue;
            };
            if other_ped.ped_state != PedState::PED_WANDER_PATH
                || now_ms <= other_ped.chat_timer
                || !ambient_chat_types_compatible(ped.ped_type, other_ped.ped_type)
            {
                continue;
            }
            let other_position = other_ped.physical.entity.get_position();
            let delta = other_position - position;
            if delta.x * delta.x + delta.y * delta.y + delta.z * delta.z >= 1.8 * 1.8 {
                rejected_distance = true;
                continue;
            }
            if delta.x * ped.physical.entity.get_forward().x
                + delta.y * ped.physical.entity.get_forward().y
                <= delta.x.hypot(delta.y) * 60.0_f32.to_radians().cos()
            {
                continue;
            }
            selected = Some((other, other_identity.handle));
            break;
        }
        if rejected_distance && let Ok((_, _, mut ped, _, _, _)) = peds.get_mut(actor) {
            ped.chat_timer = now_ms.wrapping_add(30_000);
        }
        let Some((other, other_handle)) = selected else {
            continue;
        };
        let Ok([(_, _, mut first, _, _, _), (_, _, mut second, _, _, _)]) =
            peds.get_many_mut([actor, other])
        else {
            continue;
        };
        let duration = 10_000 + u32::from(general.get_random_number() % 4_000);
        set_script_ped_chat(
            &mut first,
            CPoolHandle(other_handle).index(),
            duration,
            now_ms,
        );
        set_script_ped_chat(&mut second, CPoolHandle(handle).index(), duration, now_ms);
        // PED_CHAT immediately excludes both actors from subsequent pairings;
        // a mirrored paired-handle set is unnecessary.
    }
}
