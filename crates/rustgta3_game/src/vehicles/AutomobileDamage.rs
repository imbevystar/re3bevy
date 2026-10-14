//! State-changing VehicleDamage calculations over canonical component owners.
use super::{
    Automobile::{AutomobileState, eCarPiece},
    DamageManager::tComponent,
    Vehicle::{VehicleAppearance, VehicleCore, VehicleEntryState, VehicleVitality, eCarLock},
};
use crate::{
    entities::Entity::eEntityStatus,
    modelinfo::{ModelIndices::MI_DODO, VehicleModelInfo::VehicleDamageNodes},
};
use rustgta3_core::core::General::CGeneral;

pub(crate) fn apply_collision_piece_damage(
    core: &VehicleCore,
    vitality: &mut VehicleVitality,
    appearance: &mut VehicleAppearance,
    entry: &VehicleEntryState,
    state: &mut AutomobileState,
    impulse: f32,
    piece: eCarPiece,
    collision_damage_multiplier: f32,
    damage_multiplier: f32,
    general: &mut CGeneral,
    damage_nodes: Option<&VehicleDamageNodes>,
) -> bool {
    if !vitality.can_be_damaged
        || impulse <= 25.0
        || core.physical.entity.status == eEntityStatus::STATUS_WRECKED
        || core.physical.entity.flags.collision_proof
    {
        return false;
    }
    let component_damage_permitted =
        core.physical.entity.get_up().z > 0.0 || core.physical.move_speed.magnitude_sqr() > 0.1;
    let impulse_multiplier = if state.more_resistant_to_damage {
        0.5
    } else {
        4.0
    };
    let scaled_impulse = impulse * impulse_multiplier;
    let changed = if !component_damage_permitted {
        false
    } else if matches!(
        piece,
        eCarPiece::CAR_PIECE_BUMP_FRONT | eCarPiece::CAR_PIECE_BUMP_REAR
    ) {
        struct BumperFollowup {
            bumper: tComponent,
            panel: crate::vehicles::DamageManager::ePanels,
            component: tComponent,
            node: Option<crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition>,
            permitted: bool,
        }
        let followup = if piece == eCarPiece::CAR_PIECE_BUMP_FRONT {
            BumperFollowup {
                bumper: tComponent::COMPONENT_BUMPER_FRONT,
                panel: crate::vehicles::DamageManager::ePanels::VEHBUMPER_FRONT,
                component: tComponent::COMPONENT_DOOR_BONNET,
                node: damage_nodes.map(|nodes| nodes.bonnet),
                permitted: core.physical.entity.model_index != MI_DODO as i16,
            }
        } else {
            BumperFollowup {
                bumper: tComponent::COMPONENT_BUMPER_REAR,
                panel: crate::vehicles::DamageManager::ePanels::VEHBUMPER_REAR,
                component: tComponent::COMPONENT_DOOR_BOOT,
                node: damage_nodes.map(|nodes| nodes.boot),
                permitted: true,
            }
        };
        let mut changed = state.damage.apply_damage(
            followup.bumper,
            scaled_impulse,
            collision_damage_multiplier,
            general,
        );
        // Intentional switch fallthrough in re3: once a bumper reaches
        // MISSING, the same collision also damages bonnet/boot if present.
        let has_node = match followup.node {
            Some(crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Position(_)) => true,
            Some(crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Absent) => false,
            Some(crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Invalid) => {
                bevy::log::warn_once!("automobile_damage_followup_original_node_invalid");
                false
            }
            None => {
                bevy::log::warn_once!("automobile_damage_followup_waiting_original_nodes");
                false
            }
        };
        if has_node
            && followup.permitted
            && state.damage.get_panel_status(followup.panel as u8)
                == crate::vehicles::DamageManager::ePanelStatus::PANEL_STATUS_MISSING as u32
        {
            changed |= state.damage.apply_damage(
                followup.component,
                scaled_impulse,
                collision_damage_multiplier,
                general,
            );
        }
        changed
    } else {
        let component = match piece {
            eCarPiece::CAR_PIECE_BONNET if core.physical.entity.model_index != MI_DODO as i16 => {
                tComponent::COMPONENT_DOOR_BONNET
            }
            eCarPiece::CAR_PIECE_BOOT => tComponent::COMPONENT_DOOR_BOOT,
            eCarPiece::CAR_PIECE_DOOR_LF
                if matches!(
                    entry.door_lock,
                    eCarLock::CARLOCK_NOT_USED | eCarLock::CARLOCK_UNLOCKED
                ) =>
            {
                tComponent::COMPONENT_DOOR_FRONT_LEFT
            }
            eCarPiece::CAR_PIECE_DOOR_RF
                if matches!(
                    entry.door_lock,
                    eCarLock::CARLOCK_NOT_USED | eCarLock::CARLOCK_UNLOCKED
                ) =>
            {
                tComponent::COMPONENT_DOOR_FRONT_RIGHT
            }
            eCarPiece::CAR_PIECE_DOOR_LR
                if matches!(
                    entry.door_lock,
                    eCarLock::CARLOCK_NOT_USED | eCarLock::CARLOCK_UNLOCKED
                ) =>
            {
                tComponent::COMPONENT_DOOR_REAR_LEFT
            }
            eCarPiece::CAR_PIECE_DOOR_RR
                if matches!(
                    entry.door_lock,
                    eCarLock::CARLOCK_NOT_USED | eCarLock::CARLOCK_UNLOCKED
                ) =>
            {
                tComponent::COMPONENT_DOOR_REAR_RIGHT
            }
            eCarPiece::CAR_PIECE_WING_LF => tComponent::COMPONENT_PANEL_FRONT_LEFT,
            eCarPiece::CAR_PIECE_WING_RF => tComponent::COMPONENT_PANEL_FRONT_RIGHT,
            eCarPiece::CAR_PIECE_WING_LR => tComponent::COMPONENT_PANEL_REAR_LEFT,
            eCarPiece::CAR_PIECE_WING_RR => tComponent::COMPONENT_PANEL_REAR_RIGHT,
            eCarPiece::CAR_PIECE_WINDSCREEN => tComponent::COMPONENT_PANEL_WINDSCREEN,
            // Wheel pieces do not progress a component in GTA III, but the
            // common VehicleDamage health calculation below still runs.
            _ => tComponent::COMPONENT_DEFAULT,
        };
        component != tComponent::COMPONENT_DEFAULT
            && state.damage.apply_damage(
                component,
                scaled_impulse,
                collision_damage_multiplier,
                general,
            )
    };
    if changed {
        appearance.is_damaged = true;
    }

    // Common tail of re3 CAutomobile::VehicleDamage.  Component damage
    // alone is cosmetic; this is the state transition that eventually
    // drives ENGINE_STATUS_ON_FIRE and the five-second explosion timer.
    // Calls carrying an explicit collision impulse enter VehicleDamage
    // with damageMultiplier=0.2 (only the deferred impulse==0 path uses
    // 1.0); both paths share these canonical owners.
    let damage = (impulse - 25.0) * collision_damage_multiplier * 0.6 * damage_multiplier;
    if damage > 0.0 {
        let divisor = if core.physical.entity.status == eEntityStatus::STATUS_PLAYER {
            if vitality.take_less_damage { 6.0 } else { 2.0 }
        } else if vitality.take_less_damage {
            12.0
        } else {
            4.0
        };
        let old_health = vitality.health;
        vitality.health -= damage / divisor;
        if vitality.health <= 0.0 && old_health > 0.0 {
            vitality.health = 1.0;
        }
    }
    if vitality.health < 250.0 {
        if state.damage.get_engine_status() < 225 {
            state.damage.set_engine_status(225);
            state.fire_blow_up_timer = 0.0;
            state.set_on_fire_entity = core.physical.damage.entity;
        }
    }
    changed
}
