//! System ownership for the weapon branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::control::CarCtrl::process_simple_automobile_control;
use crate::peds::PlayerPed::update_player_on_foot_control;
use crate::weapons::Explosion::process_explosion_runtime;
use crate::weapons::ProjectileInfo::process_projectile_runtime;
use crate::weapons::ShotInfo::process_shot_runtime;
use bevy::prelude::*;
use rust_gta_3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaWeaponSimulationPlugin;

impl Plugin for GtaWeaponSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::entities::EntityReference::EntityReferencesCleared>().add_systems(
            Update,
            crate::weapons::Explosion::retire_explosion_references
                .after(GtaProcessSet::Script)
                .before(GtaProcessSet::World)
                .before(process_explosion_runtime),
        ).add_systems(Update, crate::weapons::Explosion::clear_explosion_reference_targets
            .after(GtaProcessSet::GameLogic).before(GtaProcessSet::Bridge))
        .add_systems(
            Update,
            process_projectile_runtime
                .after(process_explosion_runtime)
                .before(process_simple_automobile_control)
                .in_set(GtaProcessSet::World)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            crate::weapons::BulletRuntime::process_bullet_runtime
                .after(process_projectile_runtime)
                .before(process_simple_automobile_control)
                .in_set(GtaProcessSet::World)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            process_shot_runtime
                .after(update_player_on_foot_control)
                .before(process_simple_automobile_control)
                .in_set(GtaProcessSet::World)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            process_explosion_runtime
                .after(process_shot_runtime)
                .before(process_simple_automobile_control)
                .in_set(GtaProcessSet::World)
                .run_if(in_state(GtaAppState::InGame)),
        );
    }
}
