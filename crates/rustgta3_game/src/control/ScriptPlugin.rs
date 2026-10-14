//! System ownership for the script branch of re3 `CGame::Process`.
//! Cross-domain dependencies preserve the production schedule and deferred-command barriers.

use crate::audio::DMAudio::apply_script_audio_effects;
use crate::control::Darkel::apply_script_darkel_effects;
use crate::control::Phones::apply_script_phone_effects;
use crate::control::Pickups::update_pacman_runtime;
use crate::control::ScriptDebug::finish_debug_mission_switch;
use crate::control::ScriptDebug::prepare_debug_mission_switch;
use crate::control::ScriptDomainEffects::apply_script_aircraft_effects;
use crate::control::ScriptDomainEffects::apply_script_chase_effects;
use crate::control::ScriptDomainEffects::apply_script_credit_effects;
use crate::control::ScriptDomainEffects::apply_script_hud_effects;
use crate::control::ScriptDomainEffects::apply_script_light_effects;
use crate::control::ScriptDomainEffects::apply_script_pacman_effects;
use crate::control::ScriptDomainEffects::apply_script_population_effects;
use crate::control::ScriptDomainEffects::apply_script_restart_effects;
use crate::control::ScriptDomainEffects::apply_script_weapon_area_effects;
use crate::control::ScriptDomainEffects::prepare_gta_frame;
use crate::control::ScriptDomainEffects::process_chase_scene;
use crate::control::ScriptDynamicEntities::apply_script_dynamic_entity_effects;
use crate::control::ScriptEntities::apply_script_player_info_effects;
use crate::control::ScriptRuntime::apply_script_runtime_effects;
use crate::control::ScriptRuntime::apply_script_timer_effects;
use crate::control::ScriptRuntime::process_active_scripts;
use crate::control::ScriptWorldEffects::ensure_player_special_model_loaded;
use crate::core::Fire::process_script_fires;
use bevy::prelude::*;
use rustgta3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};

pub struct GtaScriptPlugin;

impl Plugin for GtaScriptPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            prepare_gta_frame
                .in_set(GtaProcessSet::FrameSetup)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            prepare_debug_mission_switch
                .after(update_pacman_runtime)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            process_active_scripts
                .after(prepare_debug_mission_switch)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            finish_debug_mission_switch
                .after(process_active_scripts)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_runtime_effects
                .after(finish_debug_mission_switch)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_population_effects
                .after(apply_script_runtime_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_timer_effects
                .after(apply_script_population_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_hud_effects
                .after(apply_script_timer_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_chase_effects
                .after(apply_script_hud_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_light_effects
                .after(apply_script_chase_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_credit_effects
                .after(apply_script_light_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_restart_effects
                .after(apply_script_credit_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_darkel_effects
                .after(apply_script_restart_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_phone_effects
                .after(apply_script_darkel_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            ensure_player_special_model_loaded
                .after(apply_script_phone_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_player_info_effects
                .after(ensure_player_special_model_loaded)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_audio_effects
                .after(apply_script_player_info_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_aircraft_effects
                .after(apply_script_audio_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_pacman_effects
                .after(apply_script_aircraft_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_weapon_area_effects
                .after(apply_script_pacman_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            apply_script_dynamic_entity_effects
                .after(apply_script_weapon_area_effects)
                .in_set(GtaProcessSet::Script)
                .run_if(in_state(GtaAppState::InGame)),
        )
        .add_systems(
            Update,
            process_chase_scene
                .after(process_script_fires)
                .in_set(GtaProcessSet::World)
                .run_if(in_state(GtaAppState::InGame)),
        );
    }
}
