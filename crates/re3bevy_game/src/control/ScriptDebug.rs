//! re3 reference: references/re-GTA/src/control/ScriptDebug.cpp
//!
//! SCM command observation and clean PageUp/PageDown mission-entry navigation.

use crate::{
    animation::PedVehicleAnimation::CPedVehicleAnimation,
    control::{
        Script::{CTheScripts, ScriptEntityWorld},
        ScriptTypes::*,
    },
    core::FileLoader::CFileLoader,
    peds::{
        PlayerPed::{CWorldPlayers, MAX_PLAYERS},
        VehicleRelations::complete_player_exit_vehicle,
    },
    text::Messages::CMessages,
};
use bevy::prelude::*;
use re3bevy_formats::formats::Script::SIZE_MAIN_SCRIPT;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptDebugHook {
    BeforeScriptProcessing,
    AfterScriptProcessing,
    BeforeCommand(i32),
    AfterCommand(i32),
}

/// Last native root exit, retained only as a debugger projection. Cutscene-only
/// and multi-part entries can terminate normally without awarding mission passed.
#[derive(Debug, Clone, Copy)]
pub struct LoadedMissionTermination {
    pub mission_index: i32,
    pub script_name: [u8; 8],
    pub instruction_pointer: u32,
    pub deatharrest_executed: bool,
}

/// LAUNCH_MISSION can also mark persistent MAIN service threads as mission
/// scripts. Only the loaded mission and threads in its shared segment retire
/// when that segment is replaced.
pub fn is_loaded_mission_thread(script: &crate::control::Script::CRunningScript) -> bool {
    script.mission_flag || script.ip >= SIZE_MAIN_SCRIPT as u32
}

/// Clean mission-entry navigation requested for the normal-game debugger.
/// Runs the installed mission's bottom-stack cleanup before replacement. Only
/// failure feedback and end-of-mission player relocation are suppressed.
#[derive(Debug, Default, Resource)]
pub struct GtaMissionDebugSwitcher {
    pub navigation: MissionDebugNavigation,
    pending_target: Option<i32>,
    /// The outgoing mission's effects must cross the ECS apply barrier before
    /// its shared SCM bytecode slot can be replaced.
    cleanup_frame_elapsed: bool,
    cleanup_script: Option<usize>,
    retirement_queued: bool,
    outgoing_entities: Vec<CleanupEntity>,
    resources: MissionDebugResources,
    /// Debug entry can cross islands before the story's normal travel. Keep
    /// only transition provenance; CCollision reads the canonical live pose.
    entry_mission: Option<i32>,
    entry_collision_pending: bool,
    pub entry_collision_wait_frames: u64,
    pub entry_collision_loads_completed: u64,
    pub completed_switches: u64,
    pub failed_switches: u64,
    pub streaming_wait_frames: u64,
    pub original_cleanup_wait_frames: u64,
    pub original_cleanups_completed: u64,
    pub cleanup_without_return: u64,
    pub original_cleanups_failed: u64,
    pub suppressed_cleanup_effects: u64,
    pub terminated_loaded_missions: u64,
    pub last_loaded_mission_termination: Option<LoadedMissionTermination>,
}

/// re3.cpp's debug-menu categories: intro, 14 information/activity entries,
/// then phone/contact missions. Launch addresses remain owned by MAIN.SCM.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MissionDebugNavigation {
    #[default]
    StoryAndPhone,
    AllInstalled,
}

impl MissionDebugNavigation {
    fn includes(self, index: i32) -> bool {
        self == Self::AllInstalled || index == 0 || index >= 15
    }

    pub fn entry_count(self, installed_count: i32) -> usize {
        (0..installed_count)
            .filter(|&index| self.includes(index))
            .count()
    }
}

/// Provenance of native handles allocated by mission opcodes. MAIN's persistent
/// resources are excluded; these sets own no copied live gameplay state.
#[derive(Debug, Default)]
struct MissionDebugResources {
    pickups: HashSet<i32>,
    blips: HashSet<i32>,
    sounds: HashSet<i32>,
    clocks: HashSet<u32>,
    counters: HashSet<u32>,
    end_of_game_tune: bool,
}

impl GtaMissionDebugSwitcher {
    pub(crate) fn observe_entry_teleport(
        &mut self,
        mission: bool,
        loaded_mission: Option<i32>,
        effects: &[ScriptCommandEffect],
    ) {
        if self.entry_mission != loaded_mission {
            self.entry_mission = None;
            self.entry_collision_pending = false;
        }
        if mission
            && self.entry_mission.is_some()
            && !self.is_transitioning()
            && effects.iter().any(|effect| {
                matches!(
                    effect,
                    ScriptCommandEffect::SetPlayerCoordinates { .. }
                        | ScriptCommandEffect::WarpPlayerFromCarToCoord { .. }
                )
            })
        {
            self.entry_collision_pending = true;
        }
    }

    pub(crate) fn needs_entry_collision(&self) -> bool {
        self.entry_collision_pending
    }

    pub(crate) fn finish_entry_collision(&mut self) {
        if self.entry_collision_pending {
            self.entry_collision_pending = false;
            self.entry_collision_loads_completed += 1;
        }
    }

    pub(crate) fn observe_termination(
        &mut self,
        mission_index: Option<i32>,
        script: &crate::control::Script::CRunningScript,
        outcome: &ScriptCommandOutcome,
    ) {
        if !outcome.terminated || !script.mission_flag {
            return;
        }
        let Some(mission_index) = mission_index else {
            return;
        };
        self.terminated_loaded_missions += 1;
        self.last_loaded_mission_termination = Some(LoadedMissionTermination {
            mission_index,
            script_name: script.script_name,
            instruction_pointer: outcome.ip_before,
            deatharrest_executed: script.deatharrest_executed,
        });
    }

    pub(crate) fn is_transitioning(&self) -> bool {
        self.pending_target.is_some()
    }

    pub(crate) fn is_waiting_for_effect_barrier(&self) -> bool {
        self.is_transitioning() && self.cleanup_script.is_none()
    }

    pub(crate) fn cleanup_script(&self) -> Option<usize> {
        self.cleanup_script
    }

    /// Apply the user's clean-entry policy at the typed effect boundary. The
    /// opcode still advances and all original world-restoration effects remain.
    pub(crate) fn filter_cleanup_effects(
        &mut self,
        effects: &mut Vec<ScriptCommandEffect>,
        start: usize,
    ) {
        if self.cleanup_script.is_none() {
            return;
        }
        let mut write = start;
        for read in start..effects.len() {
            let effect = effects[read];
            if is_debug_cleanup_feedback_or_relocation(&effect) {
                self.suppressed_cleanup_effects += 1;
            } else {
                effects[write] = effect;
                write += 1;
            }
        }
        effects.truncate(write);
    }

    pub(crate) fn observe_effects(&mut self, mission: bool, effects: &[ScriptCommandEffect]) {
        for effect in effects {
            // A cleanup handler can release newly created actors before its
            // final TERMINATE. Retain their native generation-bearing handles
            // for debug retirement even after CMissionCleanup drains its list.
            if mission && self.is_transitioning() {
                let registration = match *effect {
                    ScriptCommandEffect::MarkCarAsNoLongerNeeded { handle } => {
                        Some((CLEANUP_CAR, handle))
                    }
                    ScriptCommandEffect::MarkCharAsNoLongerNeeded { handle } => {
                        Some((CLEANUP_CHAR, handle))
                    }
                    ScriptCommandEffect::MarkObjectAsNoLongerNeeded { handle } => {
                        Some((CLEANUP_OBJECT, handle))
                    }
                    _ => None,
                };
                if let Some((entity_type, id)) = registration {
                    self.outgoing_entities
                        .push(CleanupEntity { entity_type, id });
                }
            }
            match *effect {
                // Native completion decides which contact markers and other
                // authored resources persist. Debug abandonment must no longer
                // claim ownership of a normally completed mission's results.
                ScriptCommandEffect::MissionCleanupProcessed if !self.is_transitioning() => {
                    self.resources = MissionDebugResources::default();
                }
                ScriptCommandEffect::LoadEndOfGameTune if mission => {
                    self.resources.end_of_game_tune = true;
                }
                ScriptCommandEffect::StopEndOfGameTune => {
                    self.resources.end_of_game_tune = false;
                }
                ScriptCommandEffect::CreatePickup { handle, .. }
                | ScriptCommandEffect::CreateFloatingPackage { handle, .. }
                    if mission =>
                {
                    self.resources.pickups.insert(handle);
                }
                ScriptCommandEffect::RemovePickup { handle } => {
                    self.resources.pickups.remove(&handle);
                }
                ScriptCommandEffect::CreateCoordinateBlip { handle, .. }
                | ScriptCommandEffect::CreateEntityBlip { handle, .. }
                | ScriptCommandEffect::CreatePickupBlip { handle, .. }
                | ScriptCommandEffect::CreateSpritePickupBlip { handle, .. }
                    if mission =>
                {
                    self.resources.blips.insert(handle);
                }
                ScriptCommandEffect::RemoveBlip { handle } => {
                    self.resources.blips.remove(&handle);
                }
                ScriptCommandEffect::AddContinuousSound { handle, .. } if mission => {
                    self.resources.sounds.insert(handle);
                }
                ScriptCommandEffect::RemoveSound { handle } => {
                    self.resources.sounds.remove(&handle);
                }
                ScriptCommandEffect::AddOnscreenClock {
                    variable_offset, ..
                } if mission => {
                    self.resources.clocks.insert(variable_offset);
                }
                ScriptCommandEffect::ClearOnscreenClock { variable_offset } => {
                    self.resources.clocks.remove(&variable_offset);
                }
                ScriptCommandEffect::AddOnscreenCounter {
                    variable_offset, ..
                } if mission => {
                    self.resources.counters.insert(variable_offset);
                }
                ScriptCommandEffect::ClearOnscreenCounter { variable_offset } => {
                    self.resources.counters.remove(&variable_offset);
                }
                _ => {}
            }
        }
    }

    fn queue_resource_cleanup(&mut self, effects: &mut Vec<ScriptCommandEffect>) {
        if std::mem::take(&mut self.resources.end_of_game_tune) {
            effects.push(ScriptCommandEffect::StopEndOfGameTune);
        }
        effects.extend(
            self.resources
                .pickups
                .drain()
                .map(|handle| ScriptCommandEffect::RemovePickup { handle }),
        );
        effects.extend(
            self.resources
                .blips
                .drain()
                .map(|handle| ScriptCommandEffect::RemoveBlip { handle }),
        );
        effects.extend(
            self.resources
                .sounds
                .drain()
                .map(|handle| ScriptCommandEffect::RemoveSound { handle }),
        );
        effects.extend(
            self.resources
                .clocks
                .drain()
                .map(|variable_offset| ScriptCommandEffect::ClearOnscreenClock { variable_offset }),
        );
        effects.extend(
            self.resources.counters.drain().map(|variable_offset| {
                ScriptCommandEffect::ClearOnscreenCounter { variable_offset }
            }),
        );
    }
}

fn is_debug_cleanup_feedback_or_relocation(effect: &ScriptCommandEffect) -> bool {
    matches!(
        effect,
        ScriptCommandEffect::PrintBig { .. }
            | ScriptCommandEffect::Print { .. }
            | ScriptCommandEffect::PrintNow { .. }
            | ScriptCommandEffect::PrintSoon { .. }
            | ScriptCommandEffect::PrintBigWithNumbers { .. }
            | ScriptCommandEffect::PrintWithNumbers { .. }
            | ScriptCommandEffect::PrintStringNow { .. }
            | ScriptCommandEffect::PrintHelp { .. }
            | ScriptCommandEffect::PlayerFailedCriticalMission
            | ScriptCommandEffect::SetPlayerCoordinates { .. }
            | ScriptCommandEffect::SetPlayerHeading { .. }
            | ScriptCommandEffect::WarpPlayerFromCarToCoord { .. }
            | ScriptCommandEffect::WarpPlayerIntoCar { .. }
    )
}

/// re3 SwitchToMission unwinds to the bottom GOSUB return and latches death/
/// arrest. The installed SCM then executes its own failure and cleanup blocks.
fn begin_original_mission_cleanup(scripts: &mut CTheScripts) -> Option<usize> {
    let root = scripts.active_scripts.iter().copied().find(|&index| {
        let script = &scripts.scripts[index];
        script.mission_flag
            && script.deatharrest_enabled
            && script.stack_pointer > 0
            && script.stack[0] >= SIZE_MAIN_SCRIPT as u32
            && (script.stack[0] as usize) < scripts.script_space.len()
    });
    let active = scripts.active_scripts.clone();
    for index in active {
        if Some(index) != root && is_loaded_mission_thread(&scripts.scripts[index]) {
            scripts.remove_script_from_active(index);
        }
    }
    if let Some(index) = root {
        let script = &mut scripts.scripts[index];
        script.stack_pointer = 0;
        script.set_ip(script.stack[0]);
        script.wake_time = 0;
        script.deatharrest_executed = true;
        script.skip_message_wait_this_frame = false;
    }
    root
}

/// PageUp = next mission, PageDown = previous mission. Outgoing VM identity and
/// registrations retire before the shared bytecode is replaced, across an ECS
/// apply barrier. Neither the old mission nor MAIN executes during that barrier.
pub(crate) fn prepare_debug_mission_switch(
    keys: Res<ButtonInput<KeyCode>>,
    mut scripts: ResMut<CTheScripts>,
    mut messages: Option<ResMut<CMessages>>,
    mut switcher: ResMut<GtaMissionDebugSwitcher>,
    mut players: CWorldPlayers,
    mut entities: ScriptEntityWorld,
    file_loader: Res<CFileLoader>,
    mut commands: Commands,
) {
    if let Some(target) = switcher.pending_target {
        if let Some(root) = switcher.cleanup_script {
            if scripts.last_vm_error.is_some() {
                switcher.original_cleanups_failed += 1;
                switcher.failed_switches += 1;
                error!(
                    "mission_debug_original_cleanup_failed: {:?}",
                    scripts.last_vm_error
                );
                scripts.remove_script_from_active(root);
            } else if scripts.active_scripts.contains(&root) {
                switcher.original_cleanup_wait_frames += 1;
                return;
            } else {
                switcher.original_cleanups_completed += 1;
            }
            switcher.cleanup_script = None;
        }
        if !switcher.retirement_queued {
            finalize_debug_mission_retirement(
                &mut scripts,
                &mut switcher,
                &mut players,
                &mut entities,
                &file_loader,
                &mut commands,
            );
            return;
        }
        // Install at the start of the next SCM slice, before MAIN.SCM can
        // launch another mission into the shared bytecode slot. The outgoing
        // mission's deferred effects were applied in the previous frame.
        if switcher.cleanup_frame_elapsed {
            if file_loader.script_streaming_barrier_in_progress() {
                switcher.streaming_wait_frames += 1;
                return;
            }
            if let Some(messages) = messages.as_deref_mut() {
                messages.clear_messages();
            }
            switcher.pending_target = None;
            switcher.cleanup_frame_elapsed = false;
            switcher.retirement_queued = false;
            switcher.last_loaded_mission_termination = None;
            match scripts.load_and_launch_mission_internal(target) {
                Ok(_) => {
                    switcher.entry_mission = Some(target);
                    switcher.entry_collision_pending = false;
                    switcher.completed_switches += 1;
                    info!("GTA mission debug switch launched mission {target}");
                }
                Err(error) => {
                    switcher.failed_switches += 1;
                    error!("GTA mission debug switch failed for mission {target}: {error:?}");
                }
            }
        }
        return;
    }

    let direction = match (
        keys.just_pressed(KeyCode::PageUp),
        keys.just_pressed(KeyCode::PageDown),
    ) {
        (true, false) => 1,
        (false, true) => -1,
        _ => return,
    };

    let mission_count = i32::from(scripts.number_of_mission_scripts);
    if mission_count <= 0 {
        warn_once!("mission_debug_waiting_original_mission_table");
        return;
    }

    let current = scripts.last_loaded_mission_index;
    let target = mission_switch_target(current, mission_count, direction, switcher.navigation);

    switcher.outgoing_entities = scripts
        .mission_cleanup
        .entities
        .iter()
        .filter(|entry| entry.entity_type != CLEANUP_UNUSED)
        .cloned()
        .collect();
    switcher.cleanup_script = begin_original_mission_cleanup(&mut scripts);
    switcher.pending_target = Some(target);
    switcher.cleanup_frame_elapsed = false;
    switcher.retirement_queued = false;
    scripts.fail_current_mission = 0;
    if scripts.on_a_mission_flag != 0 {
        let on_a_mission_flag = scripts.on_a_mission_flag as usize;
        if let Err(error) = scripts.write_i32_at_script_space(on_a_mission_flag, 0) {
            switcher.failed_switches += 1;
            error!("mission_debug_on_mission_flag_invalid: {error:?}");
        }
    }
    // Interrupt the outgoing scene before running its original cleanup, while
    // keeping actors alive for that handler's native handle queries.
    scripts.pending_effects.extend([
        ScriptCommandEffect::ClearCutscene,
        ScriptCommandEffect::StopChaseScene,
        ScriptCommandEffect::SetIntroIsPlaying { playing: false },
        ScriptCommandEffect::SetWorldProcessingEnabled(true),
        ScriptCommandEffect::ClearPrints,
        ScriptCommandEffect::ClearHelp,
    ]);
    if switcher.cleanup_script.is_none() {
        switcher.cleanup_without_return += 1;
        debug!("mission_debug_no_active_cleanup_return: mission={current:?}");
        finalize_debug_mission_retirement(
            &mut scripts,
            &mut switcher,
            &mut players,
            &mut entities,
            &file_loader,
            &mut commands,
        );
    }
    info!(
        "GTA mission debug switch: {:?} -> {} ({})",
        current,
        target,
        if direction > 0 { "PageUp" } else { "PageDown" }
    );
}

fn finalize_debug_mission_retirement(
    scripts: &mut CTheScripts,
    switcher: &mut GtaMissionDebugSwitcher,
    players: &mut CWorldPlayers,
    entities: &mut ScriptEntityWorld,
    file_loader: &CFileLoader,
    commands: &mut Commands,
) {
    // Cleanup may have started another thread in the outgoing shared segment.
    for index in scripts.active_scripts.clone() {
        if is_loaded_mission_thread(&scripts.scripts[index]) {
            scripts.remove_script_from_active(index);
        }
    }
    // A seated player's SET_PLAYER_COORDINATES teleports its car in native SCM.
    // Complete the canonical exit first so the new mission can place the ped.
    for slot in 0..MAX_PLAYERS {
        let slot = slot as i32;
        if let Some(relation) = players.relation(slot) {
            if let Some(handle) = entities.vehicle_handle_for_entity(relation.vehicle) {
                complete_player_exit_vehicle(players, entities, file_loader, slot, handle, 0);
            } else {
                players.clear_relation(slot);
                warn_once!("mission_debug_player_vehicle_identity_missing");
            }
        }
        if let Some(entity) = players.player_entity(slot) {
            commands.entity(entity).remove::<CPedVehicleAnimation>();
        }
    }
    // Debug abandonment needs immediate retirement, while ordinary native
    // mission completion releases actors to population. Drain only the exact
    // cleanup registrations, never persistent MAIN objects or the entire world.
    switcher
        .outgoing_entities
        .extend(scripts.mission_cleanup.take_for_process());
    let mut retired = HashSet::new();
    for entry in switcher.outgoing_entities.drain(..) {
        if !retired.insert((entry.entity_type, entry.id)) {
            continue;
        }
        let effect = match entry.entity_type {
            CLEANUP_CAR if entities.vehicle_entity(entry.id).is_some() => {
                ScriptCommandEffect::DeleteCar { handle: entry.id }
            }
            CLEANUP_CHAR if entities.ped_entity(entry.id).is_some() => {
                ScriptCommandEffect::DeleteChar { handle: entry.id }
            }
            CLEANUP_OBJECT if entities.object_get(entry.id).is_some() => {
                ScriptCommandEffect::DeleteObject { handle: entry.id }
            }
            _ => continue,
        };
        scripts.pending_effects.push(effect);
    }
    switcher.queue_resource_cleanup(&mut scripts.pending_effects);
    scripts.pending_effects.extend([
        ScriptCommandEffect::ClearCutscene,
        ScriptCommandEffect::StopChaseScene,
        ScriptCommandEffect::SetIntroIsPlaying { playing: false },
        ScriptCommandEffect::SetWorldProcessingEnabled(true),
        ScriptCommandEffect::SetStreamingEnabled(true),
        ScriptCommandEffect::SetPlayerControl {
            player_index: players.player_in_focus() as i32,
            enabled: true,
        },
        ScriptCommandEffect::SetPlayerVisible {
            player_index: players.player_in_focus() as i32,
            visible: true,
        },
        ScriptCommandEffect::EnablePlayerControlCamera,
        ScriptCommandEffect::ClearPrints,
        ScriptCommandEffect::ClearHelp,
    ]);
    scripts.countdown_to_make_player_unsafe = 0;
    scripts.fail_current_mission = 0;
    scripts.queue_mission_cleanup_effects();
    scripts.already_running_a_mission_script = false;
    if scripts.on_a_mission_flag != 0 {
        let on_a_mission_flag = scripts.on_a_mission_flag as usize;
        if let Err(error) = scripts.write_i32_at_script_space(on_a_mission_flag, 0) {
            switcher.failed_switches += 1;
            error!("mission_debug_on_mission_flag_invalid: {error:?}");
        }
    }

    switcher.retirement_queued = true;
}

/// Keep one complete domain-effect/application frame between retirement and
/// replacement of the shared 32 KiB mission segment.
pub(crate) fn finish_debug_mission_switch(mut switcher: ResMut<GtaMissionDebugSwitcher>) {
    if !switcher.retirement_queued || switcher.cleanup_frame_elapsed {
        return;
    }

    switcher.cleanup_frame_elapsed = true;
}

fn mission_switch_target(
    current: Option<i32>,
    mission_count: i32,
    direction: i32,
    navigation: MissionDebugNavigation,
) -> i32 {
    debug_assert!(mission_count > 0);
    let mut target = match current {
        Some(current) => (current + direction).rem_euclid(mission_count),
        None if direction > 0 => 0,
        None => mission_count - 1,
    };
    while !navigation.includes(target) {
        target = (target + direction).rem_euclid(mission_count);
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_entry_collision_follows_only_current_loaded_mission_teleports() {
        let mut debug = GtaMissionDebugSwitcher {
            entry_mission: Some(44),
            ..Default::default()
        };
        let teleport = ScriptCommandEffect::SetPlayerCoordinates {
            index: 0,
            position: Default::default(),
            needs_ground: false,
        };
        debug.observe_entry_teleport(false, Some(44), &[teleport]);
        assert!(
            !debug.needs_entry_collision(),
            "MAIN service changes do not request entry residency"
        );
        debug.observe_entry_teleport(true, Some(44), &[teleport]);
        assert!(debug.needs_entry_collision());
        debug.finish_entry_collision();
        assert!(!debug.needs_entry_collision());
        assert_eq!(debug.entry_collision_loads_completed, 1);
        debug.observe_entry_teleport(true, Some(44), &[teleport]);
        debug.observe_entry_teleport(true, Some(45), &[teleport]);
        assert!(
            !debug.needs_entry_collision(),
            "ordinary later missions relinquish debug provenance"
        );
        assert!(debug.entry_mission.is_none());
        debug.entry_mission = Some(46);
        debug.pending_target = Some(47);
        debug.observe_entry_teleport(true, Some(46), &[teleport]);
        assert!(
            !debug.needs_entry_collision(),
            "outgoing cleanup cannot request incoming residency"
        );
    }

    #[test]
    fn abandoning_a_mission_stops_only_its_owned_final_tune() {
        let mut debug = GtaMissionDebugSwitcher::default();
        debug.observe_effects(false, &[ScriptCommandEffect::LoadEndOfGameTune]);
        let mut effects = Vec::new();
        debug.queue_resource_cleanup(&mut effects);
        assert!(effects.is_empty());
        debug.observe_effects(true, &[ScriptCommandEffect::LoadEndOfGameTune]);
        debug.queue_resource_cleanup(&mut effects);
        assert!(matches!(
            effects.as_slice(),
            [ScriptCommandEffect::StopEndOfGameTune]
        ));
        debug.queue_resource_cleanup(&mut effects);
        assert_eq!(effects.len(), 1);
        debug.observe_effects(
            true,
            &[
                ScriptCommandEffect::LoadEndOfGameTune,
                ScriptCommandEffect::StopEndOfGameTune,
            ],
        );
        effects.clear();
        debug.queue_resource_cleanup(&mut effects);
        assert!(effects.is_empty());
    }

    #[test]
    fn original_cleanup_unwinds_root_and_holds_main_without_advancing_its_timers() {
        use re3bevy_formats::formats::Script::ScmFile;
        let mut scripts = CTheScripts::from_scm(ScmFile::parse_main_scm(&[]), Vec::new());
        let main = scripts.start_new_script(100).unwrap();
        scripts.scripts[main].is_mission_script = true;
        let root = scripts
            .start_new_script(SIZE_MAIN_SCRIPT as u32 + 100)
            .unwrap();
        let child = scripts
            .start_new_script(SIZE_MAIN_SCRIPT as u32 + 200)
            .unwrap();
        scripts.scripts[root].mission_flag = true;
        scripts.scripts[root].stack_pointer = 2;
        scripts.scripts[root].stack[0] = SIZE_MAIN_SCRIPT as u32 + 17;
        scripts.scripts[root].stack[1] = SIZE_MAIN_SCRIPT as u32 + 70;
        scripts.scripts[root].wake_time = u32::MAX;
        let main_before = scripts.scripts[main].clone();
        assert_eq!(begin_original_mission_cleanup(&mut scripts), Some(root));
        assert!(!scripts.active_scripts.contains(&child));
        assert!(scripts.active_scripts.contains(&main));
        let root_script = &scripts.scripts[root];
        assert_eq!(root_script.ip, SIZE_MAIN_SCRIPT as u32 + 17);
        assert_eq!(root_script.stack_pointer, 0);
        assert!(root_script.deatharrest_executed);
        assert_eq!(root_script.wake_time, 0);
        scripts
            .process_active_scripts_with_selected_executor(
                100,
                16.0,
                false,
                Some(root),
                |scripts, index| {
                    assert_eq!(index, root);
                    let ip = scripts.scripts[index].ip;
                    Ok(ScriptCommandOutcome {
                        command: super::super::ScriptCommands::COMMAND_WAIT,
                        yielded: true,
                        terminated: false,
                        ip_before: ip,
                        ip_after: ip,
                        not_flag: false,
                        script_index: index,
                    })
                },
            )
            .unwrap();
        assert_eq!(scripts.scripts[main].ip, main_before.ip);
        assert_eq!(
            scripts.scripts[main].local_variables,
            main_before.local_variables
        );
        assert_eq!(scripts.scripts[main].wake_time, main_before.wake_time);
    }

    #[test]
    fn debug_cleanup_preserves_world_restoration_and_suppresses_only_end_feedback() {
        let mut switcher = GtaMissionDebugSwitcher::default();
        switcher.cleanup_script = Some(1);
        let prefix = ScriptCommandEffect::PrintHelp {
            key: *b"KEEP\0\0\0\0",
        };
        let road = ScriptCommandEffect::SwitchRoadsInArea {
            min: Default::default(),
            max: Default::default(),
            pedestrian: false,
            disable: false,
        };
        let zone = ScriptCommandEffect::SetZonePedInfo {
            label: *b"PORT_E\0\0",
            day: true,
            ped_density: 15,
            gang_density: [0; 9],
            cop_density: 1,
        };
        let mut effects = vec![
            prefix,
            ScriptCommandEffect::PrintBig {
                key: *b"M_FAIL\0\0",
                time_ms: 5000,
                style: 1,
                queue: false,
            },
            road,
            zone,
            ScriptCommandEffect::SetPlayerCoordinates {
                index: 0,
                position: Default::default(),
                needs_ground: false,
            },
            ScriptCommandEffect::MissionCleanupProcessed,
        ];
        switcher.filter_cleanup_effects(&mut effects, 1);
        assert_eq!(
            effects,
            vec![
                prefix,
                road,
                zone,
                ScriptCommandEffect::MissionCleanupProcessed
            ]
        );
        assert_eq!(switcher.suppressed_cleanup_effects, 2);
    }

    #[test]
    fn normal_completion_relinquishes_contact_marker_ownership_but_debug_cleanup_retains_it() {
        let mut switcher = GtaMissionDebugSwitcher::default();
        switcher.resources.blips.insert(42);
        switcher.observe_effects(true, &[ScriptCommandEffect::MissionCleanupProcessed]);
        assert!(switcher.resources.blips.is_empty());
        switcher.pending_target = Some(20);
        switcher.resources.blips.insert(43);
        switcher.observe_effects(true, &[ScriptCommandEffect::MissionCleanupProcessed]);
        assert!(switcher.resources.blips.contains(&43));
    }

    #[test]
    fn normal_loaded_entry_exit_is_distinct_from_persistent_main_service_exit() {
        let mut debug = GtaMissionDebugSwitcher::default();
        let mut script = crate::control::Script::CRunningScript::new();
        script.is_mission_script = true;
        let outcome = ScriptCommandOutcome {
            command: crate::control::ScriptCommands::COMMAND_TERMINATE_THIS_SCRIPT,
            yielded: true,
            terminated: true,
            ip_before: SIZE_MAIN_SCRIPT as u32 + 44,
            ip_after: SIZE_MAIN_SCRIPT as u32 + 46,
            not_flag: false,
            script_index: 1,
        };
        debug.observe_termination(Some(38), &script, &outcome);
        assert_eq!(debug.terminated_loaded_missions, 0);
        assert!(debug.last_loaded_mission_termination.is_none());

        script.mission_flag = true;
        script.script_name = *b"frank3\0\0";
        debug.observe_termination(Some(38), &script, &outcome);
        let termination = debug.last_loaded_mission_termination.unwrap();
        assert_eq!(debug.terminated_loaded_missions, 1);
        assert_eq!(termination.mission_index, 38);
        assert_eq!(
            termination.instruction_pointer,
            SIZE_MAIN_SCRIPT as u32 + 44
        );
        assert!(!termination.deatharrest_executed);
    }

    #[test]
    fn mission_switch_target_wraps_both_directions() {
        let all = MissionDebugNavigation::AllInstalled;
        assert_eq!(mission_switch_target(Some(4), 6, 1, all), 5);
        assert_eq!(mission_switch_target(Some(5), 6, 1, all), 0);
        assert_eq!(mission_switch_target(Some(1), 6, -1, all), 0);
        assert_eq!(mission_switch_target(Some(0), 6, -1, all), 5);
        assert_eq!(mission_switch_target(None, 6, 1, all), 0);
        assert_eq!(mission_switch_target(None, 6, -1, all), 5);
    }

    #[test]
    fn story_navigation_includes_intro_and_phone_missions_without_activity_prerequisites() {
        let story = MissionDebugNavigation::StoryAndPhone;
        assert_eq!(story.entry_count(80), 66);
        assert_eq!(mission_switch_target(Some(79), 80, 1, story), 0);
        assert_eq!(mission_switch_target(Some(0), 80, 1, story), 15);
        assert_eq!(mission_switch_target(Some(15), 80, -1, story), 0);
        assert_eq!(mission_switch_target(Some(0), 80, -1, story), 79);
        assert_eq!(mission_switch_target(Some(19), 80, 1, story), 20);
        assert_eq!(mission_switch_target(Some(6), 80, -1, story), 0);
        assert_eq!(mission_switch_target(None, 80, 1, story), 0);
        assert_eq!(story.entry_count(6), 1);
        assert_eq!(mission_switch_target(Some(0), 6, 1, story), 0);
    }

    #[test]
    fn debug_abandonment_retires_only_mission_auxiliary_resources_once() {
        let mut switcher = GtaMissionDebugSwitcher::default();
        switcher.observe_effects(
            false,
            &[ScriptCommandEffect::AddContinuousSound {
                handle: 1,
                position: Default::default(),
                sound_id: 1,
            }],
        );
        switcher.observe_effects(
            true,
            &[
                ScriptCommandEffect::AddContinuousSound {
                    handle: 2,
                    position: Default::default(),
                    sound_id: 1,
                },
                ScriptCommandEffect::AddOnscreenClock {
                    variable_offset: 100,
                    key: None,
                },
            ],
        );
        let mut effects = Vec::new();
        switcher.queue_resource_cleanup(&mut effects);
        assert_eq!(effects.len(), 2);
        assert!(effects.contains(&ScriptCommandEffect::RemoveSound { handle: 2 }));
        assert!(effects.contains(&ScriptCommandEffect::ClearOnscreenClock {
            variable_offset: 100
        }));
        switcher.queue_resource_cleanup(&mut effects);
        assert_eq!(
            effects.len(),
            2,
            "abandoned resources cannot be requeued forever"
        );
    }

    #[test]
    fn main_service_mission_threads_survive_shared_segment_replacement() {
        let mut service = crate::control::Script::CRunningScript::new();
        service.is_mission_script = true;
        service.ip = 500;
        assert!(!is_loaded_mission_thread(&service));
        service.mission_flag = true;
        assert!(is_loaded_mission_thread(&service));
        service.mission_flag = false;
        service.ip = SIZE_MAIN_SCRIPT as u32 + 100;
        assert!(is_loaded_mission_thread(&service));
    }
}
