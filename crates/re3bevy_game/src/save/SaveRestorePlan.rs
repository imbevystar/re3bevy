//! re3 reference: references/re-GTA/src/save/GenericGameStorage.cpp
//! re3 reference: references/re-GTA/src/core/Pools.cpp
//!
//! Typed post-load restore plan. This is the hand-off point between save block
//! decoding and the runtime world/pool consumer.

use std::io;

use bevy::prelude::Resource;
use re3bevy_assets::streaming::CdStream::CdStream;
use re3bevy_core::core::{Coordinates::CVector, GameState::ELevelName};

use crate::{
    control::{Pickups::CPickups, Replay::ReplayRuntimeWorldState},
    core::{
        FileLoader::CFileLoader,
        Pools::{
            CPool, CPoolEntry, CPoolEntryKind, CPools, CompatibleObjectSave, CompatiblePedSave,
            CompatiblePoolSave, CompatibleVehicleSave, NUMOBJECTS, NUMPEDS, NUMVEHICLES,
            PoolRuntimeRestorePlan,
        },
        Streaming::CStreaming,
    },
};

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CSaveRestorePlan {
    pub pool_restore: PoolRuntimeRestorePlan,
    pub pickup_objects: Vec<crate::control::Pickups::PickupObjectRestore>,
    pub replay_runtime: Option<ReplayRuntimeWorldState>,
    pub level_to_load: ELevelName,
    pub camera_position: CVector,
    pub post_load_commands: Vec<SavePostLoadCommand>,
    pub load_scene_after_restore: bool,
    pub sort_out_collision_after_load: bool,
    pub remove_unused_big_buildings: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CSaveRuntimeWorldState {
    pub peds: Vec<RuntimePedRestoreState>,
    pub vehicles: Vec<RuntimeVehicleRestoreState>,
    pub objects: Vec<RuntimeObjectRestoreState>,
    pub pickup_objects: Vec<crate::control::Pickups::PickupObjectRestore>,
    pub replay_runtime: Option<ReplayRuntimeWorldState>,
    pub level_to_load: ELevelName,
    pub camera_position: CVector,
    pub post_load_commands: Vec<SavePostLoadCommand>,
    pub load_scene_after_restore: bool,
    pub sort_out_collision_after_load: bool,
    pub remove_unused_big_buildings: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePedRestoreState {
    pub slot: usize,
    pub record: CompatiblePedSave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeVehicleRestoreState {
    pub slot: usize,
    pub record: CompatibleVehicleSave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeObjectRestoreState {
    pub slot: usize,
    pub record: CompatibleObjectSave,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SavePostLoadCommand {
    LoadScene {
        position: CVector,
    },
    RemoveUnusedBigBuildings {
        level: ELevelName,
    },
    RemoveUnusedBuildings {
        level: ELevelName,
    },
    SortOutCollisionAfterLoad,
    RequestBigBuildings {
        level: ELevelName,
    },
    LoadAllRequestedModels {
        priority: bool,
    },
    HaveAllBigBuildingsLoaded {
        level: ELevelName,
    },
    TidyUpMemory {
        flush_streaming: bool,
        clear_collision: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavePostLoadExecution {
    pub results: Vec<SavePostLoadCommandResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavePostLoadCommandResult {
    pub command: SavePostLoadCommand,
    pub status: SavePostLoadCommandStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePostLoadCommandStatus {
    Applied,
    AppliedByLoadScene,
}

impl CSaveRuntimeWorldState {
    pub fn apply_to_pools(&self, pools: &mut CPools) {
        pools.ped_pool = CPool::new(NUMPEDS);
        pools.vehicle_pool = CPool::new(NUMVEHICLES);
        pools.object_pool = CPool::new(NUMOBJECTS);

        for ped in &self.peds {
            pools.ped_pool.new_at(
                ped.record.handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Ped,
                    CompatiblePoolSave::Ped(ped.record.clone()),
                ),
            );
        }
        for vehicle in &self.vehicles {
            pools.vehicle_pool.new_at(
                vehicle.record.handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Vehicle,
                    CompatiblePoolSave::Vehicle(vehicle.record.clone()),
                ),
            );
        }
        for object in &self.objects {
            pools.object_pool.new_at(
                object.record.handle,
                CPoolEntry::with_compatible_save(
                    CPoolEntryKind::Object,
                    CompatiblePoolSave::Object(object.record.clone()),
                ),
            );
        }
    }

    pub fn execute_post_load_commands(
        &self,
        file_loader: &mut CFileLoader,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
    ) -> io::Result<SavePostLoadExecution> {
        let mut results = Vec::with_capacity(self.post_load_commands.len());
        let mut load_scene_applied = false;

        for command in &self.post_load_commands {
            let status = match *command {
                SavePostLoadCommand::LoadScene { position } => {
                    file_loader.load_scene_streaming(
                        streaming,
                        cd_stream,
                        [position.x, position.y, position.z],
                    )?;
                    load_scene_applied = true;
                    SavePostLoadCommandStatus::Applied
                }
                SavePostLoadCommand::RemoveUnusedBigBuildings { level } => {
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        file_loader.remove_unused_big_buildings(streaming, level as i32);
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::RemoveUnusedBuildings { level } => {
                    file_loader.remove_unused_buildings(streaming, level as i32);
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::SortOutCollisionAfterLoad => {
                    file_loader
                        .model_info
                        .remove_col_models_from_other_levels(self.level_to_load as i32);
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::RequestBigBuildings { level } => {
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        file_loader.request_big_buildings(streaming, level as i32);
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::LoadAllRequestedModels { priority } => {
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        file_loader.load_all_requested_models(streaming, cd_stream, priority)?;
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::HaveAllBigBuildingsLoaded { .. } => {
                    streaming.has_loaded_lods = true;
                    if load_scene_applied {
                        SavePostLoadCommandStatus::AppliedByLoadScene
                    } else {
                        SavePostLoadCommandStatus::Applied
                    }
                }
                SavePostLoadCommand::TidyUpMemory { .. } => SavePostLoadCommandStatus::Applied,
            };
            results.push(SavePostLoadCommandResult {
                command: *command,
                status,
            });
        }

        Ok(SavePostLoadExecution { results })
    }
}

impl CSaveRestorePlan {
    pub fn build(pools: &CPools, pickups: Option<&CPickups>) -> Self {
        Self::build_for_level(pools, pickups, ELevelName::Industrial, CVector::ZERO)
    }

    pub fn build_for_level(
        pools: &CPools,
        pickups: Option<&CPickups>,
        level_to_load: ELevelName,
        camera_position: CVector,
    ) -> Self {
        Self {
            pool_restore: pools.build_runtime_restore_plan(),
            pickup_objects: pickups
                .map(CPickups::build_object_restore_plan)
                .unwrap_or_default(),
            replay_runtime: None,
            level_to_load,
            camera_position,
            post_load_commands: build_post_load_commands(level_to_load, camera_position),
            load_scene_after_restore: true,
            sort_out_collision_after_load: true,
            remove_unused_big_buildings: true,
        }
    }

    pub fn with_replay_runtime(mut self, replay_runtime: ReplayRuntimeWorldState) -> Self {
        self.replay_runtime = Some(replay_runtime);
        self
    }

    pub fn apply_to_runtime_state(&self, runtime: &mut CSaveRuntimeWorldState) {
        runtime.peds = self
            .pool_restore
            .peds
            .iter()
            .map(|ped| RuntimePedRestoreState {
                slot: ped.slot,
                record: ped.record.clone(),
            })
            .collect();
        runtime.vehicles = self
            .pool_restore
            .vehicles
            .iter()
            .map(|vehicle| RuntimeVehicleRestoreState {
                slot: vehicle.slot,
                record: vehicle.record.clone(),
            })
            .collect();
        runtime.objects = self
            .pool_restore
            .objects
            .iter()
            .map(|object| RuntimeObjectRestoreState {
                slot: object.slot,
                record: object.record.clone(),
            })
            .collect();
        runtime.pickup_objects = self.pickup_objects.clone();
        runtime.replay_runtime = self.replay_runtime.clone();
        runtime.level_to_load = self.level_to_load;
        runtime.camera_position = self.camera_position;
        runtime.post_load_commands = self.post_load_commands.clone();
        runtime.load_scene_after_restore = self.load_scene_after_restore;
        runtime.sort_out_collision_after_load = self.sort_out_collision_after_load;
        runtime.remove_unused_big_buildings = self.remove_unused_big_buildings;
    }

    pub fn into_runtime_state(&self) -> CSaveRuntimeWorldState {
        let mut runtime = CSaveRuntimeWorldState::default();
        self.apply_to_runtime_state(&mut runtime);
        runtime
    }
}

fn build_post_load_commands(level: ELevelName, position: CVector) -> Vec<SavePostLoadCommand> {
    vec![
        SavePostLoadCommand::LoadScene { position },
        SavePostLoadCommand::RemoveUnusedBigBuildings { level },
        SavePostLoadCommand::RemoveUnusedBuildings { level },
        SavePostLoadCommand::SortOutCollisionAfterLoad,
        SavePostLoadCommand::RequestBigBuildings { level },
        SavePostLoadCommand::LoadAllRequestedModels { priority: false },
        SavePostLoadCommand::HaveAllBigBuildingsLoaded { level },
        SavePostLoadCommand::TidyUpMemory {
            flush_streaming: true,
            clear_collision: false,
        },
    ]
}
