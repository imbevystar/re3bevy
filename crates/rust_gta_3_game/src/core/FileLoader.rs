//! re3 reference: references/re-GTA/src/core/FileLoader.cpp
//! re3 reference: references/re-GTA/src/core/FileLoader.h
//!
//! Streaming-facing FileLoader slice. Full RenderWare object construction is
//! deferred to the RW port; this validates RW chunks and updates the same
//! ModelInfo/TxdStore load markers used by CStreaming.

use std::{
    collections::{BTreeSet, HashSet},
    io,
};

use crate::{
    collision::{
        ColModel::{
            CColBox, CColLine, CColModel, CColSphere, CColTriangle, CColTrianglePlane, CColVector,
            CompressedVector,
        },
        TempColModels::CTempColModels,
    },
    control::PathFind::CPathFind,
    core::Game::GameplayAuditTimer,
    core::Radar::{CRadar, RADAR_STREAM_FLAGS},
    core::Streaming::{
        CStreaming, CStreamingConvertContext, CStreamingModelInfo, CStreamingObjectLoader,
        ChannelState, ConvertResult, ModelInfoType, STREAM_OFFSET_TXD, STREAMFLAGS_DONT_REMOVE,
        STREAMFLAGS_NOFADE, STREAMFLAGS_PRIORITY, StreamLoadState,
    },
    core::World::{CWorld, EEntityType, NUMSECTORS_X, NUMSECTORS_Y},
    modelinfo::ModelIndices::{MI_COP, MI_LOPOLYGUY, MI_MALE01, MI_TAXI_D},
    modelinfo::ModelInfo::{CMloInstance, CModelInfo, SimpleModelFlags},
    objects::ObjectData::CObjectData,
    renderer::Renderer::CRenderer,
    rw::TxdStore::CTxdStore,
};
use bevy::prelude::{Res, ResMut, Resource};
use rust_gta_3_assets::{
    install::{FileMgr::CFileMgr, OriginalGtaInstall::OriginalGtaInstall},
    streaming::CdStream::{CdStream, StreamStatus},
};
use rust_gta_3_core::core::Coordinates::CVector;
use rust_gta_3_core::core::{Clock::CClock, General::CGeneral};

const BIGBUILDINGFLAGS: u8 = STREAMFLAGS_DONT_REMOVE | STREAMFLAGS_PRIORITY;
/// re3 `Streaming.cpp`: `#define STREAM_DIST 80.0f`.
pub(crate) const STREAM_DIST: f32 = 80.0;
const CULLZONE_DAT_MAX_LEN: usize = 4 * 1024 * 1024;
use rust_gta_3_formats::formats::{
    ClumpRead::{
        RW_CHUNK_HEADER_LEN, RW_ID_CLUMP, RwAtomicDescriptor, RwClumpReadState, RwClumpSummary,
        RwGeometryDescriptor, gta_clump_stream_read1, gta_clump_stream_read2, read_chunk_header,
        read_clump,
    },
    ColRead::{GtaColBox, GtaColModel, GtaColSphere, GtaColVector, read_col_file},
    FileLoader::{
        GtaDatCommand, GtaDatPlan, GtaIdeClump, GtaIdeMloHeader, GtaIdeMloInstance, GtaIdeObject,
        GtaIdePed, GtaIdePlan, GtaIdeRecord, GtaIdeTimeObject, GtaIdeVehicle, GtaIplPlan,
        GtaIplRecord,
    },
    TexRead::{
        RwTexDictionaryReadState, RwTexDictionarySummary, gta_tex_dictionary_stream_read1,
        gta_tex_dictionary_stream_read2, read_txd,
    },
};

#[derive(Debug, Clone, Resource)]
pub struct CFileLoader {
    pub model_info: CModelInfo,
    pub txd_store: CTxdStore,
    pub temp_col_models: CTempColModels,
    pub clock: CClock,
    pub renderer: CRenderer,
    pub radar: CRadar,
    pub object_data: CObjectData,
    pub world: CWorld,
    pub path_find: CPathFind,
    /// Generation of re3 `CStreaming::DeleteAllRwObjects` requests.
    ///
    /// The streamed model is shared model data; the actual per-world-entity
    /// RwObject lives in the Bevy render registry. `LOAD_SCENE` bumps this
    /// generation so that registry can drop only those instances without
    /// corrupting `CModelInfo` residency.
    pub world_rw_object_epoch: u64,
    /// Cooperative gameplay-side `CStreaming::LoadScene` barrier.
    ///
    /// The original call blocks inside the SCM opcode. Doing the same in Bevy
    /// also blocks ECS/render extraction and turns CD/TXD/DFF work into a hard
    /// frame hitch. The gameplay path prepares the exact same request set, then
    /// lets the normal streaming service drain it over subsequent frames.
    scene_load_in_progress: bool,
    /// Cooperative gameplay-side `COMMAND_LOAD_ALL_MODELS_NOW` barrier.
    load_all_models_in_progress: bool,
    /// True only when the cooperative script streaming barrier set CTimer's
    /// code-pause bit.
    /// This prevents it from clearing a pause owned by another subsystem.
    script_streaming_owns_code_pause: bool,
    /// Per-entity counterpart of re3 `CStreaming::DeleteFarAwayRwObjects`.
    ///
    /// The authoritative per-entity RwObject lives in Bevy's world-mesh
    /// registry, not in `CWorldEntity`, so streaming records the same sector
    /// wake deletions here and the render bridge consumes them after it knows
    /// which entities are being rendered this frame.
    pending_world_rw_object_deletes: HashSet<usize>,
    /// Deferred half of re3 world-entity streaming deletion: the entity mirror
    /// must be released before its model ref reaches zero. Once the render
    /// bridge has dropped that RwObject, the next streaming pump removes the
    /// shared model.
    pending_streaming_model_removals: BTreeSet<usize>,
    /// Derived equivalent of re3 `CWorld::m_nCurrentScanCode` +
    /// `CEntity::m_scanCode` for the streaming-sector walk. This is only a
    /// visitation cache: authoritative world state remains in `CWorld`.
    streaming_scan_codes: Vec<u32>,
    streaming_scan_code: u32,
    pub loaded_level_plans: Vec<LoadedLevelPlan>,
    pub loaded_ide_plans: Vec<LoadedIdePlan>,
    pub loaded_ipl_plans: Vec<LoadedIplPlan>,
    pub loaded_collision_files: Vec<LoadedCollisionFile>,
    pub loaded_tex_dictionaries: Vec<LoadedTexDictionaryFile>,
    pub current_tex_dictionary_textures: Vec<String>,
    pub loaded_model_files: Vec<LoadedModelFile>,
    pub loaded_hier_files: Vec<LoadedHierFile>,
    pub path_object_model_ids: BTreeSet<usize>,
    objects_loaded_for_ipl: bool,
    clump_read_state: Option<RwClumpReadState>,
    txd_read_state: Option<RwTexDictionaryReadState>,
    /// Lazily resolved `CStreaming::RequestSubway` model ids. Name lookup is
    /// a linear scan, so resolve once per level instead of every frame while
    /// the player is underground.
    subway_model_cache: SubwayModelCache,
}

/// Lazily resolved re3 `CStreaming::RequestSubway` model ids.
#[derive(Debug, Clone, Default)]
struct SubwayModelCache {
    resolved: bool,
    platform_level: i32,
    tunnels: Vec<usize>,
    platforms: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedLevelPlan {
    pub filename: String,
    pub plan: GtaDatPlan,
}

/// Incremental counterpart of re3 `CFileLoader::LoadLevel`.
///
/// The original loader calls `LoadingScreenLoadingFile` before each heavy DAT
/// entry, which pumps/presents the loading screen between files. Executing the
/// entire DEFAULT.DAT + GTA3.DAT plan in one Bevy system instead produced a
/// multi-second frame. Keep the exact command order while allowing the game
/// loading state to yield at DAT command boundaries.
#[derive(Debug, Clone)]
pub struct IncrementalLevelPlanLoad {
    plans: Vec<LoadedLevelPlan>,
    plan_index: usize,
    command_index: usize,
    loose_txds_loaded: bool,
}

impl IncrementalLevelPlanLoad {
    pub fn is_finished(&self) -> bool {
        self.plan_index >= self.plans.len() && self.loose_txds_loaded
    }

    pub fn current_file(&self) -> Option<&str> {
        let plan = self.plans.get(self.plan_index)?;
        let command = plan.plan.commands.get(self.command_index)?;
        Some(match command {
            GtaDatCommand::ImagePath(path)
            | GtaDatCommand::TexDictionary(path)
            | GtaDatCommand::ModelFile(path)
            | GtaDatCommand::HierFile(path)
            | GtaDatCommand::Ide(path)
            | GtaDatCommand::Ipl(path)
            | GtaDatCommand::MapZone(path)
            | GtaDatCommand::CdImage(path) => path.as_str(),
            GtaDatCommand::ColFile { path, .. } => path.as_str(),
            GtaDatCommand::Splash => "SPLASH",
            GtaDatCommand::Exit => "EXIT",
            GtaDatCommand::Unknown(line) => line.as_str(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedIdePlan {
    pub filename: String,
    pub plan: GtaIdePlan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedIplPlan {
    pub filename: String,
    pub plan: GtaIplPlan,
    pub map_zones_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCollisionFile {
    pub filename: String,
    pub level: i32,
    pub loaded_models: Vec<String>,
    pub missing_models: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedTexDictionaryFile {
    pub filename: String,
    pub summary: Option<RwTexDictionarySummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedModelFile {
    pub filename: String,
    pub summary: Option<RwClumpSummary>,
    pub bound_atomics: Vec<BoundAtomicModelInfo>,
    pub missing_atomics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedHierFile {
    pub filename: String,
    pub clumps: Vec<RwClumpSummary>,
    pub bound_clumps: Vec<BoundClumpModelInfo>,
    pub missing_clumps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundAtomicModelInfo {
    pub model_id: usize,
    pub model_name: String,
    pub lod_index: usize,
    pub frame_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundClumpModelInfo {
    pub model_id: usize,
    pub model_name: String,
    pub lod_index: usize,
    pub frame_name: String,
}

impl Default for CFileLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl CFileLoader {
    /// CBaseModelInfo::GetColModel also resolves the engine's shared
    /// CTempColModels bindings for flying vehicle components (ModelInfo::Initialise).
    pub fn collision_model(&self, model_index: usize) -> Option<&crate::collision::ColModel::CColModel> {
        let base = self.model_info.get_model_info(model_index)?.base();
        base.col_model.as_ref().or_else(|| {
            base.default_col_model.map(|kind| self.temp_col_models.model_for_kind(kind))
        })
    }

    pub fn new() -> Self {
        let mut txd_store = CTxdStore::initialise();
        let model_info = CModelInfo::initialise(&mut txd_store);
        Self {
            model_info,
            txd_store,
            temp_col_models: CTempColModels::initialise(),
            clock: CClock::default(),
            renderer: CRenderer::default(),
            radar: CRadar::default(),
            object_data: CObjectData::default(),
            world: CWorld::initialise(),
            path_find: CPathFind::init(),
            world_rw_object_epoch: 0,
            scene_load_in_progress: false,
            load_all_models_in_progress: false,
            script_streaming_owns_code_pause: false,
            pending_world_rw_object_deletes: HashSet::new(),
            pending_streaming_model_removals: BTreeSet::new(),
            streaming_scan_codes: Vec::new(),
            streaming_scan_code: 0,
            loaded_level_plans: Vec::new(),
            loaded_ide_plans: Vec::new(),
            loaded_ipl_plans: Vec::new(),
            loaded_collision_files: Vec::new(),
            loaded_tex_dictionaries: Vec::new(),
            current_tex_dictionary_textures: Vec::new(),
            loaded_model_files: Vec::new(),
            loaded_hier_files: Vec::new(),
            path_object_model_ids: BTreeSet::new(),
            objects_loaded_for_ipl: false,
            clump_read_state: None,
            txd_read_state: None,
            subway_model_cache: SubwayModelCache::default(),
        }
    }

    /// Rebuild level-owned loader state for re3's restart path while keeping
    /// DeleteAllRwObjects observable by the persistent Bevy render registry.
    pub fn reset_for_restart(&mut self) {
        let next_rw_object_epoch = self.world_rw_object_epoch.wrapping_add(1);
        *self = Self::new();
        self.world_rw_object_epoch = next_rw_object_epoch;
    }

    pub fn with_stores(model_info: CModelInfo, txd_store: CTxdStore) -> Self {
        Self {
            model_info,
            txd_store,
            temp_col_models: CTempColModels::initialise(),
            clock: CClock::default(),
            renderer: CRenderer::default(),
            radar: CRadar::default(),
            object_data: CObjectData::default(),
            world: CWorld::initialise(),
            path_find: CPathFind::init(),
            world_rw_object_epoch: 0,
            scene_load_in_progress: false,
            load_all_models_in_progress: false,
            script_streaming_owns_code_pause: false,
            pending_world_rw_object_deletes: HashSet::new(),
            pending_streaming_model_removals: BTreeSet::new(),
            streaming_scan_codes: Vec::new(),
            streaming_scan_code: 0,
            loaded_level_plans: Vec::new(),
            loaded_ide_plans: Vec::new(),
            loaded_ipl_plans: Vec::new(),
            loaded_collision_files: Vec::new(),
            loaded_tex_dictionaries: Vec::new(),
            current_tex_dictionary_textures: Vec::new(),
            loaded_model_files: Vec::new(),
            loaded_hier_files: Vec::new(),
            path_object_model_ids: BTreeSet::new(),
            objects_loaded_for_ipl: false,
            clump_read_state: None,
            txd_read_state: None,
            subway_model_cache: SubwayModelCache::default(),
        }
    }

    pub fn load_collision_file(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
        level: i32,
    ) -> io::Result<LoadedCollisionFile> {
        let bytes = file_mgr.load_file(filename, 16 * 1024 * 1024, "rb")?;
        let summary = self.load_collision_file_bytes(filename, level, &bytes)?;
        self.loaded_collision_files.push(summary.clone());
        Ok(summary)
    }

    pub fn load_collision_file_bytes(
        &mut self,
        filename: &str,
        level: i32,
        bytes: &[u8],
    ) -> io::Result<LoadedCollisionFile> {
        let mut loaded_models = Vec::new();
        let mut missing_models = Vec::new();
        let col_file = read_col_file(bytes)?;

        for named_model in col_file.models {
            let col_model = convert_col_model(named_model.model, level);
            if let Some((model_id, _)) = self.model_info.get_model_info_by_name(&named_model.name) {
                if let Some(model) = self.model_info.get_model_info_mut(model_id) {
                    model.base_mut().set_col_model(col_model, true);
                    loaded_models.push(named_model.name);
                }
            } else {
                missing_models.push(named_model.name);
            }
        }

        Ok(LoadedCollisionFile {
            filename: filename.to_owned(),
            level,
            loaded_models,
            missing_models,
        })
    }

    pub fn load_collision_from_dat_file(
        &mut self,
        file_mgr: &mut CFileMgr,
        dat_file: &str,
        curr_level: i32,
    ) -> io::Result<Vec<LoadedCollisionFile>> {
        let bytes = file_mgr.load_file(dat_file, 1024 * 1024, "r")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let plan = GtaDatPlan::parse(&text);
        let mut loaded = Vec::new();
        for command in plan.commands {
            if let GtaDatCommand::ColFile { level, path } = command
                && level == curr_level
            {
                loaded.push(self.load_collision_file(file_mgr, &path, level)?);
            }
        }
        Ok(loaded)
    }

    pub fn load_tex_dictionary_file(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<LoadedTexDictionaryFile> {
        let bytes = file_mgr.load_file(filename, 16 * 1024 * 1024, "rb")?;
        let loaded = LoadedTexDictionaryFile {
            filename: filename.to_owned(),
            summary: read_txd(&bytes),
        };
        if let Some(summary) = &loaded.summary {
            self.add_tex_dictionaries(summary);
        }
        self.loaded_tex_dictionaries.push(loaded.clone());
        Ok(loaded)
    }

    pub fn add_tex_dictionaries(&mut self, src: &RwTexDictionarySummary) {
        for texture in &src.textures {
            if !texture.name.is_empty()
                && !self
                    .current_tex_dictionary_textures
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&texture.name))
            {
                self.current_tex_dictionary_textures
                    .push(texture.name.clone());
            }
        }
    }

    pub fn load_model_file(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<LoadedModelFile> {
        let bytes = file_mgr.load_file(filename, 16 * 1024 * 1024, "rb")?;
        let summary = read_clump(&bytes);
        let (bound_atomics, missing_atomics) = summary
            .as_ref()
            .map(|summary| self.bind_model_file_atomics(summary))
            .unwrap_or_default();
        let loaded = LoadedModelFile {
            filename: filename.to_owned(),
            summary,
            bound_atomics,
            missing_atomics,
        };
        self.loaded_model_files.push(loaded.clone());
        Ok(loaded)
    }

    pub fn load_hier_file(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<LoadedHierFile> {
        let bytes = file_mgr.load_file(filename, 16 * 1024 * 1024, "rb")?;
        let clumps = read_all_clumps(&bytes);
        let (bound_clumps, missing_clumps) = self.bind_hier_file_clumps(&clumps);
        let loaded = LoadedHierFile {
            filename: filename.to_owned(),
            clumps,
            bound_clumps,
            missing_clumps,
        };
        self.loaded_hier_files.push(loaded.clone());
        Ok(loaded)
    }

    pub fn load_object_data(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<CObjectData> {
        let bytes = file_mgr.load_file(filename, 1024 * 1024, "r")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let records = CObjectData::parse_records(&text);
        let mut object_data = CObjectData::default();

        for record in records {
            let object_id = object_data.object_info.len() as i16;
            object_data.object_info.push(record.info);
            if let Some((model_id, _)) = self.model_info.get_model_info_by_name(&record.name) {
                self.model_info.set_object_id(model_id, object_id);
                object_data.bound_models.push((model_id, record.name));
            } else {
                object_data.missing_models.push(record.name);
            }
        }

        self.object_data = object_data.clone();
        Ok(object_data)
    }

    fn bind_model_file_atomics(
        &mut self,
        summary: &RwClumpSummary,
    ) -> (Vec<BoundAtomicModelInfo>, Vec<String>) {
        let mut bound = Vec::new();
        let mut missing = Vec::new();
        for (atomic_index, atomic) in summary.atomics.iter().enumerate() {
            let Some(frame_name) = atomic.frame_name.as_deref() else {
                continue;
            };
            let (model_name, lod_index) = get_name_and_lod(frame_name);
            let Some((model_id, _)) = self.model_info.get_model_info_by_name(&model_name) else {
                missing.push(model_name);
                continue;
            };
            let Some(geometry) = atomic_geometry(summary, atomic_index, atomic) else {
                missing.push(model_name);
                continue;
            };
            let txd_slot = self
                .model_info
                .get_model_info(model_id)
                .map(|model| model.base().txd_slot);
            if self.model_info.set_simple_atomic_binding(
                model_id,
                lod_index,
                frame_name,
                atomic.clone(),
                geometry.clone(),
            ) {
                if let Some(txd_slot) = txd_slot.and_then(|slot| usize::try_from(slot).ok()) {
                    self.txd_store.add_ref(txd_slot);
                }
                bound.push(BoundAtomicModelInfo {
                    model_id,
                    model_name,
                    lod_index,
                    frame_name: frame_name.to_owned(),
                });
            } else {
                missing.push(model_name);
            }
        }
        (bound, missing)
    }

    fn bind_hier_file_clumps(
        &mut self,
        clumps: &[RwClumpSummary],
    ) -> (Vec<BoundClumpModelInfo>, Vec<String>) {
        let mut bound = Vec::new();
        let mut missing = Vec::new();
        for clump in clumps {
            let Some(frame_name) = clump.root_frame_name.as_deref() else {
                continue;
            };
            let (model_name, lod_index) = get_name_and_lod(frame_name);
            let Some((model_id, _)) = self.model_info.get_model_info_by_name(&model_name) else {
                missing.push(model_name);
                continue;
            };
            let is_clump = self
                .model_info
                .get_model_info(model_id)
                .map(|model| model.base().model_type.is_clump())
                .unwrap_or(false);
            if !is_clump {
                missing.push(model_name);
                continue;
            }
            self.model_info
                .set_model_loaded(model_id, true, Some(clump.clone()));
            bound.push(BoundClumpModelInfo {
                model_id,
                model_name,
                lod_index,
                frame_name: frame_name.to_owned(),
            });
        }
        (bound, missing)
    }

    pub fn load_level_plan(
        &mut self,
        install: &OriginalGtaInstall,
        file_mgr: &mut CFileMgr,
        cd_stream: &mut CdStream,
        filename: &str,
    ) -> io::Result<GtaDatPlan> {
        let bytes = file_mgr.load_file(filename, 1024 * 1024, "r")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let plan = GtaDatPlan::parse(&text);

        for command in &plan.commands {
            self.apply_level_plan_command(install, file_mgr, cd_stream, command)?;
        }

        self.loaded_level_plans.push(LoadedLevelPlan {
            filename: filename.to_owned(),
            plan: plan.clone(),
        });
        Ok(plan)
    }

    fn parse_level_plan(&self, file_mgr: &mut CFileMgr, filename: &str) -> io::Result<GtaDatPlan> {
        let bytes = file_mgr.load_file(filename, 1024 * 1024, "r")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        Ok(GtaDatPlan::parse(&text))
    }

    fn apply_level_plan_command(
        &mut self,
        install: &OriginalGtaInstall,
        file_mgr: &mut CFileMgr,
        cd_stream: &mut CdStream,
        command: &GtaDatCommand,
    ) -> io::Result<()> {
        match command {
            GtaDatCommand::ColFile { level, path } => {
                self.load_collision_file(file_mgr, path, *level)?;
            }
            GtaDatCommand::TexDictionary(path) => {
                self.load_tex_dictionary_file(file_mgr, path)?;
            }
            GtaDatCommand::ModelFile(path) => {
                self.load_model_file(file_mgr, path)?;
            }
            GtaDatCommand::HierFile(path) => {
                self.load_hier_file(file_mgr, path)?;
            }
            GtaDatCommand::CdImage(path) => {
                cd_stream.add_image(install.resolve(path))?;
            }
            GtaDatCommand::Ide(path) => {
                self.load_object_types_plan(file_mgr, path)?;
            }
            GtaDatCommand::Ipl(path) => {
                self.prepare_objects_before_first_ipl(file_mgr)?;
                self.load_scene_plan(file_mgr, path)?;
            }
            GtaDatCommand::MapZone(path) => {
                self.load_map_zones_plan(file_mgr, path)?;
            }
            GtaDatCommand::ImagePath(_)
            | GtaDatCommand::Splash
            | GtaDatCommand::Exit
            | GtaDatCommand::Unknown(_) => {}
        }
        Ok(())
    }

    pub fn begin_default_and_dat_plans_incremental(
        &mut self,
        install: &OriginalGtaInstall,
        file_mgr: &mut CFileMgr,
        cd_stream: &mut CdStream,
        dat_file: &str,
    ) -> io::Result<IncrementalLevelPlanLoad> {
        cd_stream.remove_images();
        cd_stream.add_image(install.gta3_img())?;
        let default_filename = "DATA\\DEFAULT.DAT";
        let default_plan = self.parse_level_plan(file_mgr, default_filename)?;
        let dat_plan = self.parse_level_plan(file_mgr, dat_file)?;
        Ok(IncrementalLevelPlanLoad {
            plans: vec![
                LoadedLevelPlan {
                    filename: default_filename.to_owned(),
                    plan: default_plan,
                },
                LoadedLevelPlan {
                    filename: dat_file.to_owned(),
                    plan: dat_plan,
                },
            ],
            plan_index: 0,
            command_index: 0,
            loose_txds_loaded: false,
        })
    }

    /// Execute at most one authored DAT command and yield back to Bevy.
    /// Returns true after both plans and the loose-TXD epilogue are complete.
    pub fn step_default_and_dat_plans_incremental(
        &mut self,
        install: &OriginalGtaInstall,
        file_mgr: &mut CFileMgr,
        cd_stream: &mut CdStream,
        state: &mut IncrementalLevelPlanLoad,
    ) -> io::Result<bool> {
        if state.plan_index < state.plans.len() {
            let plan = &state.plans[state.plan_index];
            if let Some(command) = plan.plan.commands.get(state.command_index).cloned() {
                self.apply_level_plan_command(install, file_mgr, cd_stream, &command)?;
                state.command_index += 1;
                return Ok(false);
            }

            self.loaded_level_plans.push(plan.clone());
            state.plan_index += 1;
            state.command_index = 0;
            return Ok(false);
        }

        if !state.loose_txds_loaded {
            self.ensure_loose_txds_loaded(file_mgr);
            state.loose_txds_loaded = true;
        }
        Ok(true)
    }

    pub fn load_scene_plan(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<GtaIplPlan> {
        self.load_ipl_plan(file_mgr, filename, false)
    }

    pub fn load_map_zones_plan(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<GtaIplPlan> {
        self.load_ipl_plan(file_mgr, filename, true)
    }

    fn load_ipl_plan(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
        map_zones_only: bool,
    ) -> io::Result<GtaIplPlan> {
        let bytes = file_mgr.load_file(filename, 8 * 1024 * 1024, "rb")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let mut plan = GtaIplPlan::parse(&text);
        if map_zones_only {
            plan.records
                .retain(|record| matches!(record, GtaIplRecord::Zone(_)));
        }
        self.world.apply_ipl_plan(
            &plan,
            map_zones_only,
            &self.model_info,
            &self.path_object_model_ids,
        );
        if !map_zones_only {
            self.register_new_world_map_objects_for_path_find();
        }
        self.loaded_ipl_plans.push(LoadedIplPlan {
            filename: filename.to_owned(),
            plan: plan.clone(),
            map_zones_only,
        });
        Ok(plan)
    }

    pub fn load_default_and_dat_plans(
        &mut self,
        install: &OriginalGtaInstall,
        file_mgr: &mut CFileMgr,
        cd_stream: &mut CdStream,
        dat_file: &str,
    ) -> io::Result<(GtaDatPlan, GtaDatPlan)> {
        // This entry point rebuilds the complete level/CD-image plan.  It is
        // also used when the frontend starts a new game after one has already
        // been running.  CdStream outlives CStreaming, so retaining its image
        // list here makes the same GTA3.IMG/GTA3.DIR get registered and parsed
        // again on every restart.  GTA III has 90 legitimate "extra object"
        // DFFs in GTA3.DIR; parsing the directory twice therefore overflows
        // re3's fixed EXTRADIRSIZE (128) even though either directory by itself
        // is valid.  Full level-plan construction owns the image list, so make
        // it idempotent before adding the base image again.
        cd_stream.remove_images();
        cd_stream.add_image(install.gta3_img())?;
        let default_plan =
            self.load_level_plan(install, file_mgr, cd_stream, "DATA\\DEFAULT.DAT")?;
        let dat_plan = self.load_level_plan(install, file_mgr, cd_stream, dat_file)?;
        // Loose TXDs (not always present on CD image) needed for generic world textures.
        self.ensure_loose_txds_loaded(file_mgr);
        Ok((default_plan, dat_plan))
    }

    fn register_new_world_map_objects_for_path_find(&mut self) {
        let already_registered = self.path_find.num_map_objects.max(0) as usize;
        for entity_index in self.world.map_objects.iter().skip(already_registered) {
            if let Some(entity) = self.world.entities.get(*entity_index) {
                let _ = self.path_find.register_map_object(
                    entity.model_index,
                    CVector::new(entity.position[0], entity.position[1], entity.position[2]),
                    entity.rotation_axis,
                    entity.rotation_w,
                );
            }
        }
    }

    pub fn initialise_radar(&mut self) {
        self.radar.initialise(&self.txd_store);
    }

    pub fn load_cullzone_visibility_cache(
        &mut self,
        file_mgr: &mut CFileMgr,
        general: &mut CGeneral,
    ) -> io::Result<bool> {
        match file_mgr.load_file("DATA\\cullzone.dat", CULLZONE_DAT_MAX_LEN, "rb") {
            Ok(bytes) => {
                let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
                self.world.cull_zones.load_visibility_cache(bytes)?;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.world
                    .prepare_cull_zone_visibility_candidates_without_cache(
                        &self.model_info,
                        general,
                    );
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub fn load_scene_streaming(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
        position: [f32; 3],
    ) -> io::Result<()> {
        self.begin_load_scene_streaming(streaming, position);
        let result = self.load_all_requested_models(streaming, cd_stream, false);
        self.scene_load_in_progress = false;
        self.script_streaming_owns_code_pause = false;
        result
    }

    /// re3 `CStreaming::DeleteAllRwObjects` for world instances.
    ///
    /// Streamed model data remains resident; the Bevy world-mesh registry
    /// consumes this generation and releases/recycles the per-entity mirrors.
    pub fn delete_all_world_rw_objects(&mut self) {
        self.world_rw_object_epoch = self.world_rw_object_epoch.wrapping_add(1);
        self.pending_world_rw_object_deletes.clear();
    }

    /// Prepare re3 `CStreaming::LoadScene` without blocking the main thread.
    ///
    /// This is byte-for-byte the state-mutating prefix of the synchronous
    /// routine above. Gameplay SCM uses it as a cooperative barrier, while
    /// loading screens and parity harnesses can keep using
    /// `load_scene_streaming` when blocking is intentional.
    pub fn begin_load_scene_streaming(&mut self, streaming: &mut CStreaming, position: [f32; 3]) {
        let level = self.world.zones.get_level_from_position(position);
        self.renderer.loading_priority = false;
        self.remove_non_persistent_requests(streaming);
        self.world.force_cull_zone_coors(position);
        // re3 `CStreaming::LoadScene` calls `DeleteAllRwObjects` here. Do not
        // clear shared CModelInfo geometry: the Bevy registry consumes this
        // epoch and deletes the per-entity RenderWare mirrors instead.
        self.delete_all_world_rw_objects();
        streaming.old_sector_x = CWorld::get_sector_index_x(position[0]) as i32;
        streaming.old_sector_y = CWorld::get_sector_index_y(position[1]) as i32;
        self.add_models_to_request_list(streaming, position);
        self.stream_radar_sections(streaming, position);
        self.remove_unused_big_buildings(streaming, level);
        self.request_big_buildings(streaming, level);
        self.scene_load_in_progress = true;
        self.load_all_models_in_progress = false;
    }

    #[inline]
    pub fn scene_load_in_progress(&self) -> bool {
        self.scene_load_in_progress
    }

    #[inline]
    pub fn script_streaming_barrier_in_progress(&self) -> bool {
        self.scene_load_in_progress || self.load_all_models_in_progress
    }

    #[inline]
    pub fn script_streaming_owns_code_pause(&self) -> bool {
        self.script_streaming_owns_code_pause
    }

    #[inline]
    pub fn set_script_streaming_owns_code_pause(&mut self, owns_pause: bool) {
        self.script_streaming_owns_code_pause = owns_pause;
    }

    /// Begin the cooperative form of SCM `LOAD_ALL_MODELS_NOW`.
    ///
    /// RequestModel side effects have already populated CStreaming before this
    /// command executes. Do not touch that request set; simply hold the VM at
    /// this command boundary until the normal CD service drains it.
    pub fn begin_load_all_models_streaming(&mut self, streaming: &CStreaming) {
        let pending = streaming.requested_list().next().is_some()
            || streaming
                .channel
                .iter()
                .any(|channel| channel.state != ChannelState::Idle);
        self.load_all_models_in_progress = pending;
    }

    #[inline]
    pub fn abort_script_streaming_barrier(&mut self) {
        self.scene_load_in_progress = false;
        self.load_all_models_in_progress = false;
    }

    /// Service one non-blocking CD slice of a cooperative SCM streaming
    /// barrier (`LOAD_SCENE` or `LOAD_ALL_MODELS_NOW`).
    ///
    /// Scene requests deliberately bypass the ordinary end-of-frame request
    /// cleanup: C++ `LoadScene` keeps this request set until
    /// `LoadAllRequestedModels` has drained it completely.
    pub fn pump_script_streaming_barrier(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
    ) -> io::Result<bool> {
        if !self.script_streaming_barrier_in_progress() {
            return Ok(true);
        }

        let is_complete = |streaming: &CStreaming| {
            streaming.requested_list().next().is_none()
                && streaming
                    .channel
                    .iter()
                    .all(|channel| channel.state == ChannelState::Idle)
        };
        if is_complete(streaming) {
            self.scene_load_in_progress = false;
            self.load_all_models_in_progress = false;
            return Ok(true);
        }

        let _ = self.pump_streaming_channels(streaming, cd_stream)?;
        let complete = is_complete(streaming);
        if complete {
            self.scene_load_in_progress = false;
            self.load_all_models_in_progress = false;
        }
        Ok(complete)
    }

    /// re3 `CStreaming::Update` world-streaming slice: request models around
    /// the camera, delete far-away entity RwObjects and service one CD slice.
    /// Radar streaming belongs to `CRadar::DrawRadarMap` later in the frame;
    /// big-building requests belong to scene/level-transition boundaries.
    pub fn update_streaming_around_position(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
        position: [f32; 3],
    ) -> io::Result<usize> {
        // re3 `CStreaming::Update` does not force cull-zone transitions here;
        // `CCullZones::Update` owns that work on frame slots 0/4 after Camera.
        // `LoadScene` still performs its explicit ForceCullZoneCoors above.
        self.add_models_to_request_list(streaming, position);
        self.delete_far_away_rw_objects(streaming, position);
        let loaded = self.pump_streaming_channels(streaming, cd_stream)?;
        // re3 `CStreaming::Update` drops ordinary one-frame requests after
        // `LoadRequestedModels`. Leaving them queued lets models from old
        // camera positions compete indefinitely with the current road/world
        // request set.
        self.remove_non_persistent_requests(streaming);
        Ok(loaded)
    }

    /// Local co-op extension of the continuous stream pass. Each position
    /// contributes the original re3 request radius; loading is pumped once
    /// after the union is complete.
    pub fn update_streaming_around_positions(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
        positions: &[[f32; 3]],
        player_coors: Option<[f32; 3]>,
    ) -> io::Result<usize> {
        let Some(primary) = positions.first().copied() else {
            return Ok(0);
        };
        // Cull-zone transitions are owned by the normal CCullZones pipeline,
        // not by each CStreaming::Update camera ring.
        for position in positions.iter().copied() {
            self.add_models_to_request_list(streaming, position);
        }
        // re3 `CStreaming::Update` requests subway models instead of the
        // camera ring while the player train is underground.
        if let Some(player_coors) = player_coors {
            self.request_subway_models(streaming, player_coors);
        }
        // Local co-op unions request rings, but re3's RwObject wake follows
        // the primary camera/player sector. Keep one authoritative old-sector
        // cursor rather than deleting relative to several cameras at once.
        self.delete_far_away_rw_objects(streaming, primary);
        let loaded = self.pump_streaming_channels(streaming, cd_stream)?;
        self.remove_non_persistent_requests(streaming);
        Ok(loaded)
    }

    /// re3 `CStreaming::DeleteFarAwayRwObjects`.
    ///
    /// C++ deletes entity RwObjects in the wake of sector movement while
    /// leaving off-screen-but-near instances alive. The Bevy renderer owns the
    /// actual instance, so this method records the same deletion candidates;
    /// the renderer lifecycle retires them before the next ScanWorld pass.
    pub fn delete_far_away_rw_objects(&mut self, streaming: &mut CStreaming, position: [f32; 3]) {
        let posx = CWorld::get_sector_index_x(position[0]) as i32;
        let posy = CWorld::get_sector_index_y(position[1]) as i32;

        // Large jump (teleport/initialization): snap cursor directly to avoid
        // sweeping deletion sweeps across unrelated map sectors.
        if (posx - streaming.old_sector_x).abs() > 10 || (posy - streaming.old_sector_y).abs() > 10
        {
            streaming.old_sector_x = posx;
            streaming.old_sector_y = posy;
            return;
        }

        while posx != streaming.old_sector_x {
            if posx < streaming.old_sector_x {
                for r in 8..=14 {
                    let x = streaming.old_sector_x + r;
                    if x < 0 {
                        continue;
                    }
                    if x >= NUMSECTORS_X as i32 {
                        break;
                    }

                    for i in -r..=r {
                        let y = streaming.old_sector_y + i;
                        if y < 0 {
                            continue;
                        }
                        if y >= NUMSECTORS_Y as i32 {
                            break;
                        }

                        queue_far_rw_object_deletes(
                            &self.world,
                            streaming,
                            &mut self.pending_world_rw_object_deletes,
                            x,
                            y,
                            streaming.old_sector_x,
                            streaming.old_sector_y,
                        );
                    }
                }
                streaming.old_sector_x -= 1;
            } else {
                for r in 8..=14 {
                    let x = streaming.old_sector_x - r;
                    if x < 0 {
                        break;
                    }
                    if x >= NUMSECTORS_X as i32 {
                        continue;
                    }

                    for i in -r..=r {
                        let y = streaming.old_sector_y + i;
                        if y < 0 {
                            continue;
                        }
                        if y >= NUMSECTORS_Y as i32 {
                            break;
                        }

                        queue_far_rw_object_deletes(
                            &self.world,
                            streaming,
                            &mut self.pending_world_rw_object_deletes,
                            x,
                            y,
                            streaming.old_sector_x,
                            streaming.old_sector_y,
                        );
                    }
                }
                streaming.old_sector_x += 1;
            }
        }

        while posy != streaming.old_sector_y {
            if posy < streaming.old_sector_y {
                for r in 8..=14 {
                    let y = streaming.old_sector_y + r;
                    if y < 0 {
                        continue;
                    }
                    if y >= NUMSECTORS_Y as i32 {
                        break;
                    }

                    for i in -r..=r {
                        let x = streaming.old_sector_x + i;
                        if x < 0 {
                            continue;
                        }
                        if x >= NUMSECTORS_X as i32 {
                            break;
                        }

                        queue_far_rw_object_deletes(
                            &self.world,
                            streaming,
                            &mut self.pending_world_rw_object_deletes,
                            x,
                            y,
                            streaming.old_sector_x,
                            streaming.old_sector_y,
                        );
                    }
                }
                streaming.old_sector_y -= 1;
            } else {
                for r in 8..=14 {
                    let y = streaming.old_sector_y - r;
                    if y < 0 {
                        break;
                    }
                    if y >= NUMSECTORS_Y as i32 {
                        continue;
                    }

                    for i in -r..=r {
                        let x = streaming.old_sector_x + i;
                        if x < 0 {
                            continue;
                        }
                        if x >= NUMSECTORS_X as i32 {
                            break;
                        }

                        queue_far_rw_object_deletes(
                            &self.world,
                            streaming,
                            &mut self.pending_world_rw_object_deletes,
                            x,
                            y,
                            streaming.old_sector_x,
                            streaming.old_sector_y,
                        );
                    }
                }
                streaming.old_sector_y += 1;
            }
        }
    }

    pub fn mark_world_rw_object_deleted(&mut self, entity_index: usize) {
        self.pending_world_rw_object_deletes.insert(entity_index);
    }

    pub fn take_pending_world_rw_object_deletes(&mut self) -> HashSet<usize> {
        std::mem::take(&mut self.pending_world_rw_object_deletes)
    }

    /// One non-blocking re3 `CStreaming::LoadRequestedModels` service step.
    ///
    /// Normal gameplay must never wait for the CD worker here. A completed
    /// channel is converted, an idle channel receives its next batch, and the
    /// function returns. Loading screens use the explicitly synchronous
    /// `load_all_requested_models` path instead.
    pub fn pump_streaming_channels(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
    ) -> io::Result<usize> {
        // `DeleteIsland` first releases the entity RwObject and only then
        // removes the shared streamed model. In Bevy those two ownership
        // changes straddle the render bridge, so finish deferred removals once
        // `sync_runtime_entity_model_refs` has observed the despawned mirror.
        self.flush_pending_streaming_model_removals(streaming);
        let requested_before = streaming.num_models_requested;
        let mut current_channel = streaming.current_channel;
        if streaming.channel_error != -1 {
            let error_channel = streaming.channel_error as usize;
            streaming.retry_load_file(error_channel, cd_stream, self)?;
        } else {
            streaming.load_requested_models(cd_stream, self, &mut current_channel)?;
        }
        streaming.current_channel = current_channel;
        Ok(requested_before
            .saturating_sub(streaming.num_models_requested)
            .max(0) as usize)
    }

    /// Request DFF+TXD for a list of model indices (cutscene objects, script props).
    pub fn request_models_for_runtime(
        &mut self,
        streaming: &mut CStreaming,
        model_ids: impl IntoIterator<Item = usize>,
        flags: u8,
    ) {
        for model_id in model_ids {
            if model_id == 0 && self.model_info.get_model_info(0).is_none() {
                continue;
            }
            // A renderer request means this shared model is needed again even
            // if an obsolete world instance is still waiting for its deferred
            // DeleteRwObject bridge. Keep the old entity deletion queued, but
            // do not let its deferred model eviction defeat the new request.
            if !self.pending_streaming_model_removals.is_empty() {
                self.pending_streaming_model_removals.remove(&model_id);
            }
            streaming.request_model_with_adapter(model_id, flags, self);
        }
    }

    /// re3 `CStreaming::RequestSubway` (Streaming.cpp): request the subway
    /// tunnel/platform models with STREAMFLAGS_NOFADE. C++ triggers this when
    /// the player train is underground (`train->GetPosition().z < 0`); trains
    /// have no runtime registry in this port yet, so the caller passes the
    /// focused player position, which is underground exactly when riding the
    /// subway. Without this, subway entities are skipped by
    /// `process_entities_in_sector` like C++ but never requested, leaving
    /// solid-but-invisible tunnels.
    pub fn request_subway_models(&mut self, streaming: &mut CStreaming, player_coors: [f32; 3]) {
        const SUBWAY_TUNNELS: [&str; 18] = [
            "sbwy_tunl_start",
            "sbwy_tunl_bit",
            "sbwy_tunl_bend",
            "sbwy_tunl_cstm6",
            "sbwy_tunl_cstm7",
            "sbwy_tunl_cstm8",
            "sbwy_tunl_cstm10",
            "sbwy_tunl_cstm9",
            "sbwy_tunl_cstm11",
            "sbwy_tunl_cstm1",
            "sbwy_tunl_cstm2",
            "sbwy_tunl_cstm4",
            "sbwy_tunl_cstm3",
            "sbwy_tunl_cstm5",
            "subplatform_n2",
            "suby_tunl_start",
            "sbwy_tunl_start2",
            "indy_tunl_start",
        ];
        if player_coors[2] >= 0.0 {
            return;
        }
        // `CTheZones::curr_level` here is the re3 level number
        // (1 = Industrial, 2 = Commercial, 3 = Suburban).
        let level = self.world.zones.curr_level;
        let (tunnels, platforms) = {
            let cache = &mut self.subway_model_cache;
            if !cache.resolved || cache.platform_level != level {
                cache.tunnels = SUBWAY_TUNNELS
                    .iter()
                    .filter_map(|name| {
                        self.model_info
                            .get_model_info_by_name(name)
                            .map(|(id, _)| id)
                    })
                    .collect();
                let platform_names: &[&str] = match level {
                    1 => &["indsubway03"],
                    2 if player_coors[1] < -700.0 => &["comerside_subway", "subplatform"],
                    2 => &["subplatform_n"],
                    3 => &["Otherside_subway", "subplatform_sub"],
                    _ => &[],
                };
                cache.platforms = platform_names
                    .iter()
                    .filter_map(|name| {
                        self.model_info
                            .get_model_info_by_name(name)
                            .map(|(id, _)| id)
                    })
                    .collect();
                cache.platform_level = level;
                cache.resolved = true;
            }
            (cache.tunnels.clone(), cache.platforms.clone())
        };
        for model_id in tunnels.into_iter().chain(platforms) {
            streaming.request_model_with_adapter(model_id, STREAMFLAGS_NOFADE, self);
        }
    }

    /// re3 `CEntity::CreateRwObject`: once an entity successfully owns a
    /// RenderWare instance, the source model (and its TXD) gains one ref until
    /// `CEntity::DeleteRwObject`. Runtime Bevy mesh owners use this boundary so
    /// `CStreaming::RemoveLoadedVehicle` cannot evict a model that still has a
    /// live vehicle/ped/object instance.
    pub fn add_model_ref(&mut self, model_id: usize) -> bool {
        let model_info = &mut self.model_info;
        let txd_store = &mut self.txd_store;
        let Some(model) = model_info.get_model_info_mut(model_id) else {
            return false;
        };
        model.base_mut().add_ref(txd_store);
        true
    }

    pub fn load_initial_peds(&mut self, streaming: &mut CStreaming) {
        streaming.request_model_with_adapter(MI_COP, STREAMFLAGS_DONT_REMOVE, self);
        streaming.request_model_with_adapter(MI_MALE01, STREAMFLAGS_DONT_REMOVE, self);
        streaming.request_model_with_adapter(MI_TAXI_D, STREAMFLAGS_DONT_REMOVE, self);
    }

    pub fn load_initial_weapons(&mut self, streaming: &mut CStreaming) {
        for model_id in [170, 171, 172, 173, 174, 175, 176, 177, 178, 180, 181, 182] {
            streaming.request_model_with_adapter(model_id, STREAMFLAGS_DONT_REMOVE, self);
        }
    }

    pub fn load_initial_vehicles(&mut self, streaming: &mut CStreaming) {
        streaming.num_vehicles_loaded = 0;
        streaming.last_vehicle_deleted = 0;

        if let Some(id) = self.model_id_by_name("taxi") {
            streaming.request_model_with_adapter(id, STREAMFLAGS_DONT_REMOVE, self);
        }
        if let Some(id) = self.model_id_by_name("police") {
            streaming.request_model_with_adapter(id, STREAMFLAGS_DONT_REMOVE, self);
        }
    }

    pub fn request_initial_big_buildings(&mut self, streaming: &mut CStreaming) {
        self.request_big_buildings(streaming, 0);
    }

    pub fn stream_radar_sections(&mut self, streaming: &mut CStreaming, position: [f32; 3]) {
        let plan = self.radar.stream_radar_sections_for_position(position);
        for txd_id in plan.request_txd_ids {
            streaming.request_txd(txd_id, RADAR_STREAM_FLAGS);
        }
        for txd_id in plan.remove_txd_ids {
            streaming.remove_model_with_adapter(txd_id + STREAM_OFFSET_TXD, Some(self));
        }
    }

    /// Local co-op extension of re3 `CRadar::StreamRadarSections`: request the
    /// union of every active viewport's 3x3 radar window. Normal gameplay calls
    /// this from the render/HUD side, after `CStreaming::Update`, like re3.
    pub fn stream_radar_sections_for_positions(
        &mut self,
        streaming: &mut CStreaming,
        positions: &[[f32; 3]],
    ) {
        let plan = self.radar.stream_radar_sections_for_positions(positions);
        for txd_id in plan.request_txd_ids {
            streaming.request_txd(txd_id, RADAR_STREAM_FLAGS);
        }
        for txd_id in plan.remove_txd_ids {
            streaming.remove_model_with_adapter(txd_id + STREAM_OFFSET_TXD, Some(self));
        }
    }

    pub fn add_models_to_request_list(&mut self, streaming: &mut CStreaming, position: [f32; 3]) {
        let xmin = position[0] - STREAM_DIST;
        let ymin = position[1] - STREAM_DIST;
        let xmax = position[0] + STREAM_DIST;
        let ymax = position[1] + STREAM_DIST;
        let ixmin = CWorld::get_sector_index_x(xmin);
        let ixmax = CWorld::get_sector_index_x(xmax);
        let iymin = CWorld::get_sector_index_y(ymin);
        let iymax = CWorld::get_sector_index_y(ymax);
        let center_x = CWorld::get_sector_index_x(position[0]) as i32;
        let center_y = CWorld::get_sector_index_y(position[1]) as i32;
        // Each sector stores overlapping entities in both its primary and
        // overlap lists. Mirror C++'s generation-stamped entity scan code:
        // direct array lookup is O(1), allocation-free after warm-up, and has
        // better locality than hashing thousands of dense world indices.
        let scan_code = self.begin_streaming_scan();

        for iy in iymin..=iymax {
            let dy = iy as i32 - center_y;
            for ix in ixmin..=ixmax {
                if self.renderer.loading_priority && streaming.num_models_requested > 5 {
                    return;
                }

                let dx = ix as i32 - center_x;
                let d = dx * dx + dy * dy;
                if d <= 1 {
                    self.process_entities_in_sector(streaming, ix, iy, position, None, scan_code);
                } else if d <= 4 * 4 {
                    self.process_entities_in_sector(
                        streaming,
                        ix,
                        iy,
                        position,
                        Some(([xmin, ymin], [xmax, ymax])),
                        scan_code,
                    );
                }
            }
        }
    }

    fn begin_streaming_scan(&mut self) -> u32 {
        if self.streaming_scan_codes.len() < self.world.entities.len() {
            self.streaming_scan_codes
                .resize(self.world.entities.len(), 0);
        }
        self.streaming_scan_code = self.streaming_scan_code.wrapping_add(1);
        if self.streaming_scan_code == 0 {
            self.streaming_scan_codes.fill(0);
            self.streaming_scan_code = 1;
        }
        self.streaming_scan_code
    }

    pub fn request_big_buildings(&mut self, streaming: &mut CStreaming, level: i32) {
        let level = level.clamp(0, 3) as usize;
        // re3 `RequestBigBuildings` does not walk CWorld's big-building
        // CPtrList. It scans the building pool from the highest slot down.
        // Those orders differ because CPtrList uses head insertion.
        let entity_indices = self.world.building_pool.clone();
        for entity_index in entity_indices.into_iter().rev() {
            let Some(entity) = self.world.entities.get(entity_index) else {
                continue;
            };
            if entity.is_big_building && entity.level == level as i32 {
                let model_index = entity.model_index;
                if !self.pending_world_rw_object_deletes.is_empty() {
                    self.pending_world_rw_object_deletes.remove(&entity_index);
                }
                if !self.pending_streaming_model_removals.is_empty() {
                    self.pending_streaming_model_removals.remove(&model_index);
                }
                streaming.request_model_with_adapter(model_index, BIGBUILDINGFLAGS, self);
            }
        }
        self.request_islands(streaming, level as i32);
        streaming.has_loaded_lods = false;
    }

    /// re3 `CStreaming::RequestIslands`: request the two connector LODs needed
    /// while the active island is resident. `HaveAllBigBuildingsLoaded` uses
    /// the same models as its transition gate.
    pub fn request_islands(&mut self, streaming: &mut CStreaming, level: i32) {
        let required_names: &[&str] = match level {
            1 => &["IslandLODcomIND", "IslandLODsubIND"],
            2 => &["IslandLODInd", "IslandLODsubCOM"],
            3 => &["IslandLODInd", "IslandLODcomSUB"],
            _ => &[],
        };
        let model_ids = required_names
            .iter()
            .filter_map(|name| {
                self.model_info
                    .get_model_info_by_name(name)
                    .map(|(model_id, _)| model_id)
            })
            .collect::<Vec<_>>();
        for model_id in model_ids {
            self.cancel_pending_island_removal(model_id);
            streaming.request_model_with_adapter(model_id, BIGBUILDINGFLAGS, self);
        }
    }

    /// re3 `CStreaming::HaveAllBigBuildingsLoaded`: island connector LODs
    /// gate removal of the previous island's big buildings.
    pub fn have_all_big_buildings_loaded(
        &mut self,
        streaming: &mut CStreaming,
        level: i32,
    ) -> bool {
        if streaming.has_loaded_lods {
            return true;
        }
        let required_names: &[&str] = match level {
            1 => &["IslandLODcomIND", "IslandLODsubIND"],
            2 => &["IslandLODInd", "IslandLODsubCOM"],
            3 => &["IslandLODInd", "IslandLODcomSUB"],
            _ => &[],
        };
        for name in required_names {
            let Some((model_id, _)) = self.model_info.get_model_info_by_name(name) else {
                return false;
            };
            if streaming.info_for_model.get(model_id).is_none_or(|info| {
                info.load_state != crate::core::Streaming::StreamLoadState::Loaded
            }) {
                return false;
            }
        }
        for entity_index in self.world.building_pool.iter().rev().copied() {
            let Some(entity) = self.world.entities.get(entity_index) else {
                continue;
            };
            if entity.is_big_building
                && entity.level == level
                && streaming
                    .info_for_model
                    .get(entity.model_index)
                    .is_none_or(|info| {
                        info.load_state != crate::core::Streaming::StreamLoadState::Loaded
                    })
            {
                return false;
            }
        }
        self.remove_unused_big_buildings(streaming, level);
        streaming.has_loaded_lods = true;
        true
    }

    pub fn remove_unused_big_buildings(&mut self, streaming: &mut CStreaming, level: i32) {
        // re3 only evicts the three island lists here. LEVEL_GENERIC big
        // buildings are always scanned separately and must survive an island
        // transition/SCM LOAD_SCENE.
        for other_level in 1..=3 {
            if other_level != level {
                self.remove_big_buildings(streaming, other_level);
            }
        }
        self.remove_islands_not_used(streaming, level);
    }

    /// re3 `CStreaming::RemoveIslandsNotUsed` / `DeleteIsland`.
    ///
    /// The original releases the island entity's RwObject before removing the
    /// shared model. The Bevy world-mesh registry owns that RwObject, so queue
    /// the entity deletion for the render bridge and defer model eviction until
    /// its mirrored model ref has actually reached zero.
    fn remove_islands_not_used(&mut self, streaming: &mut CStreaming, level: i32) {
        let unused_names: &[&str] = match level {
            1 => &["IslandLODInd", "IslandLODcomSUB", "IslandLODsubCOM"],
            2 => &["IslandLODcomIND", "IslandLODcomSUB", "IslandLODsubIND"],
            3 => &["IslandLODsubIND", "IslandLODsubCOM", "IslandLODcomIND"],
            _ => &[
                "IslandLODInd",
                "IslandLODcomIND",
                "IslandLODcomSUB",
                "IslandLODsubIND",
                "IslandLODsubCOM",
            ],
        };
        let model_ids = unused_names
            .iter()
            .filter_map(|name| {
                self.model_info
                    .get_model_info_by_name(name)
                    .map(|(model_id, _)| model_id)
            })
            .collect::<Vec<_>>();

        for model_id in model_ids {
            if let Some(entity_index) = self.island_entity_for_model(model_id) {
                self.queue_world_entity_streaming_removal(entity_index, model_id);
            } else {
                self.pending_streaming_model_removals.insert(model_id);
            }
        }
        self.flush_pending_streaming_model_removals(streaming);
    }

    fn queue_world_entity_streaming_removal(&mut self, entity_index: usize, model_id: usize) {
        self.pending_world_rw_object_deletes.insert(entity_index);
        self.pending_streaming_model_removals.insert(model_id);
    }

    fn island_entity_for_model(&self, model_id: usize) -> Option<usize> {
        self.world
            .building_pool
            .iter()
            .copied()
            .find(|&entity_index| {
                self.world
                    .entities
                    .get(entity_index)
                    .is_some_and(|entity| entity.model_index == model_id)
            })
    }

    fn cancel_pending_island_removal(&mut self, model_id: usize) {
        self.pending_streaming_model_removals.remove(&model_id);
        if let Some(entity_index) = self.island_entity_for_model(model_id) {
            self.pending_world_rw_object_deletes.remove(&entity_index);
        }
    }

    pub(crate) fn flush_pending_streaming_model_removals(&mut self, streaming: &mut CStreaming) {
        if self.pending_streaming_model_removals.is_empty() {
            return;
        }
        let ready = self
            .pending_streaming_model_removals
            .iter()
            .copied()
            .filter(|&model_id| self.model_num_refs(model_id) == 0)
            .collect::<Vec<_>>();
        for model_id in ready {
            self.pending_streaming_model_removals.remove(&model_id);
            streaming.remove_model_with_adapter(model_id, Some(self));
        }
    }

    pub fn remove_unused_buildings(&mut self, streaming: &mut CStreaming, level: i32) {
        for other_level in 1..=3 {
            if other_level != level {
                self.remove_buildings(streaming, other_level);
            }
        }
    }

    pub fn remove_buildings(&mut self, streaming: &mut CStreaming, level: i32) {
        // re3 walks BuildingPool, TreadablePool, ObjectPool and DummyPool and
        // calls `DeleteRwObject` before testing the shared model refcount. IPL
        // map objects/dummies live in `CWorldEntity` here; runtime mission
        // ObjectPool ownership remains outside this static-world bridge.
        let mut entity_indices = Vec::new();
        entity_indices.extend(self.world.building_pool.iter().rev().copied());
        entity_indices.extend(self.world.treadable_pool.iter().rev().copied());
        entity_indices.extend(
            self.world
                .entities
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, entity)| {
                    entity.level == level
                        && matches!(entity.entity_type, EEntityType::Dummy | EEntityType::Object)
                })
                .map(|(entity_index, _)| entity_index),
        );

        let mut seen = BTreeSet::new();
        for entity_index in entity_indices {
            if !seen.insert(entity_index) {
                continue;
            }
            let Some(entity) = self.world.entities.get(entity_index) else {
                continue;
            };
            if entity.level != level {
                continue;
            }
            let model_index = entity.model_index;
            self.queue_world_entity_streaming_removal(entity_index, model_index);
        }
        self.flush_pending_streaming_model_removals(streaming);
    }

    pub fn remove_big_buildings(&mut self, streaming: &mut CStreaming, level: i32) {
        let level = level.clamp(0, 3) as usize;
        // Same pool traversal as re3 `RemoveBigBuildings`.
        let entity_indices = self.world.building_pool.clone();
        for entity_index in entity_indices.into_iter().rev() {
            let Some(entity) = self.world.entities.get(entity_index) else {
                continue;
            };
            if entity.is_big_building && entity.level == level as i32 {
                let model_index = entity.model_index;
                self.queue_world_entity_streaming_removal(entity_index, model_index);
            }
        }
        self.flush_pending_streaming_model_removals(streaming);
    }

    pub fn load_all_requested_models(
        &mut self,
        streaming: &mut CStreaming,
        cd_stream: &mut CdStream,
        priority: bool,
    ) -> io::Result<()> {
        const MAX_INITIAL_LOAD_PASSES: usize = 20_000;
        let mut passes = 0usize;
        let mut failed_attempts = std::collections::BTreeMap::<usize, u8>::new();
        while streaming.requested_list().next().is_some()
            || streaming.channel[0].state == ChannelState::Started
        {
            passes += 1;
            if passes > MAX_INITIAL_LOAD_PASSES {
                let requested = streaming.requested_list().collect::<Vec<_>>();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "initial streaming request list did not drain (requested={requested:?}, channel_state={:?}, channel_streams={:?})",
                        streaming.channel[0].state, streaming.channel[0].stream_ids,
                    ),
                ));
            }

            // A split vehicle/TXD conversion owns the loader's parser state and
            // must finish before another object can be dispatched on this channel.
            if streaming.channel[0].state == ChannelState::Started {
                let stream_id = streaming.channel[0].stream_ids[0] as usize;
                streaming.process_loading_channel(0, cd_stream, self);
                if streaming.info_for_model[stream_id].load_state == StreamLoadState::InQueue {
                    let attempts = failed_attempts.entry(stream_id).or_default();
                    *attempts = attempts.saturating_add(1);
                    if *attempts >= 3 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "stream {stream_id} failed conversion after {attempts} attempts (name={:?}, type={:?}, cd_pos_size={:?})",
                                self.model_name(stream_id),
                                self.model_type(stream_id),
                                streaming.info_for_model[stream_id].get_cd_posn_and_size(),
                            ),
                        ));
                    }
                } else {
                    failed_attempts.remove(&stream_id);
                }
                continue;
            }

            let Some(stream_id) = streaming.get_next_file_on_cd(0, priority, self) else {
                let requested = streaming.requested_list().collect::<Vec<_>>();
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!(
                        "streaming requests are blocked by unresolved dependencies: {requested:?}"
                    ),
                ));
            };
            let Some((posn, size)) = streaming.info_for_model[stream_id].get_cd_posn_and_size()
            else {
                // re3 LoadAllRequestedModels marks empty files Loaded. This is
                // normally handled while GetNextFileOnCd scans the request
                // list, but keep the synchronous fallback identical.
                streaming.mark_empty_file_loaded(stream_id);
                continue;
            };
            if size == 0 {
                streaming.remove_model_with_adapter(stream_id, Some(self));
                continue;
            }

            cd_stream.read_async(0, posn, size)?;
            while matches!(
                cd_stream.get_status(0),
                StreamStatus::Reading | StreamStatus::Waiting
            ) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            streaming.prepare_immediate_load(stream_id);
            streaming.channel[0].state = ChannelState::Reading;
            streaming.channel[0].stream_ids = [stream_id as i32, -1, -1, -1];
            streaming.channel[0].offsets = [0, -1, -1, -1];
            streaming.channel[0].size = size as i32;
            streaming.channel[0].position = posn as i32;
            streaming.process_loading_channel(0, cd_stream, self);
            if streaming.channel[0].state == ChannelState::Started {
                continue;
            }
            if streaming.info_for_model[stream_id].load_state == StreamLoadState::InQueue {
                let attempts = failed_attempts.entry(stream_id).or_default();
                *attempts = attempts.saturating_add(1);
                if *attempts >= 3 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "stream {stream_id} failed conversion after {attempts} attempts (name={:?}, type={:?}, cd_pos_size={:?}, bytes={})",
                            self.model_name(stream_id),
                            self.model_type(stream_id),
                            streaming.info_for_model[stream_id].get_cd_posn_and_size(),
                            size as usize
                                * rust_gta_3_assets::streaming::CdStream::CDSTREAM_SECTOR_SIZE,
                        ),
                    ));
                }
            }

            if stream_id < STREAM_OFFSET_TXD {
                self.model_info.set_simple_alpha(stream_id, 255);
            }
        }
        streaming.loading_big_model = false;
        streaming.channel[1].stream_ids = [-1; 4];
        streaming.channel[1].offsets = [-1; 4];
        streaming.channel[1].state = ChannelState::Idle;
        Ok(())
    }

    fn remove_non_persistent_requests(&mut self, streaming: &mut CStreaming) {
        // re3 `CStreaming::Update` performs this sweep immediately after
        // `LoadRequestedModels`, even when one/both CD channels are still
        // READING.  Files already dispatched to a channel have been removed
        // from the requested list, so only the unserved one-frame requests
        // remain here.  Keeping those requests while a channel is busy makes
        // stale/far world models accumulate and compete with the new camera
        // ring on following frames; in practice that can leave a nearby
        // detailed model waiting behind its already-resident big-building LOD.
        let requested: Vec<usize> = streaming.requested_list().collect();
        for stream_id in requested.into_iter().rev() {
            let flags = streaming.info_for_model[stream_id].flags;
            if flags
                & (crate::core::Streaming::STREAMFLAGS_KEEP_IN_MEMORY
                    | crate::core::Streaming::STREAMFLAGS_PRIORITY)
                != 0
            {
                continue;
            }
            streaming.remove_model_with_adapter(stream_id, Some(self));
        }
    }

    fn process_entities_in_sector(
        &mut self,
        streaming: &mut CStreaming,
        ix: usize,
        iy: usize,
        position: [f32; 3],
        bounds: Option<([f32; 2], [f32; 2])>,
        scan_code: u32,
    ) {
        let sector = &self.world.sectors[iy * crate::core::World::NUMSECTORS_X + ix];
        let buildings = sector.buildings.as_slice();
        let buildings_overlap = sector.buildings_overlap.as_slice();
        let objects = sector.objects.as_slice();
        let dummies = sector.dummies.as_slice();

        for slice in [buildings, buildings_overlap, objects, dummies] {
            for &entity_index in slice {
                let Some(code) = self.streaming_scan_codes.get_mut(entity_index) else {
                    continue;
                };
                if *code == scan_code {
                    continue;
                }
                *code = scan_code;
                let Some(entity) = self.world.entities.get(entity_index) else {
                    continue;
                };
                // re3 `ProcessEntitiesInSectorList`: `bStreamingDontDelete`
                // entities (notably big buildings) are handled by the dedicated
                // big-building path; subway entities are handled separately too.
                if entity.streaming_dont_delete || entity.is_subway {
                    continue;
                }
                if !self.is_streamable_at_current_time(entity.model_index) {
                    continue;
                }
                if !self.renderer.is_entity_cull_zone_visible(entity) {
                    continue;
                }

                if let Some((min, max)) = bounds {
                    let Some(model) = self.model_info.simple_model_info(entity.model_index) else {
                        // Still request clump/vehicle-type map props without simple LOD table.
                        let dx = position[0] - entity.position[0];
                        let dy = position[1] - entity.position[1];
                        if dx * dx + dy * dy > STREAM_DIST * STREAM_DIST {
                            continue;
                        }
                        let model_index = entity.model_index;
                        if !self.pending_world_rw_object_deletes.is_empty() {
                            self.pending_world_rw_object_deletes.remove(&entity_index);
                        }
                        if !self.pending_streaming_model_removals.is_empty() {
                            self.pending_streaming_model_removals.remove(&model_index);
                        }
                        streaming.request_model_with_adapter(model_index, 0, self);
                        continue;
                    };
                    // re3 `ProcessEntitiesInSectorList`: use the authored largest
                    // LOD distance directly, capped only by STREAM_DIST.
                    let lod_dist = model
                        .largest_lod_distance(self.renderer.lod_distance_multiplier(), false)
                        .min(STREAM_DIST);
                    let dx = position[0] - entity.position[0];
                    let dy = position[1] - entity.position[1];
                    if !(min[0] < entity.position[0]
                        && entity.position[0] < max[0]
                        && min[1] < entity.position[1]
                        && entity.position[1] < max[1]
                        && dx * dx + dy * dy < lod_dist * lod_dist)
                    {
                        continue;
                    }
                }

                let model_index = entity.model_index;
                if !self.pending_world_rw_object_deletes.is_empty() {
                    self.pending_world_rw_object_deletes.remove(&entity_index);
                }
                if !self.pending_streaming_model_removals.is_empty() {
                    self.pending_streaming_model_removals.remove(&model_index);
                }
                streaming.request_model_with_adapter(model_index, 0, self);
            }
        }
    }

    fn is_streamable_at_current_time(&self, model_index: usize) -> bool {
        let Some(model) = self.model_info.get_model_info(model_index) else {
            return false;
        };
        if model.base().model_type != ModelInfoType::Time {
            return true;
        }
        let Some(simple) = self.model_info.simple_model_info(model_index) else {
            return false;
        };
        match (simple.time_on, simple.time_off) {
            (Some(time_on), Some(time_off)) => self
                .clock
                .get_is_time_in_range(time_on as u8, time_off as u8),
            _ => true,
        }
    }

    pub fn load_object_types_plan(
        &mut self,
        file_mgr: &mut CFileMgr,
        filename: &str,
    ) -> io::Result<GtaIdePlan> {
        let bytes = file_mgr.load_file(filename, 4 * 1024 * 1024, "rb")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let plan = GtaIdePlan::parse(&text);
        if let Some(row) = plan.invalid_vehicle_records.first() {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                format!("invalid IDE vehicle record in {filename}: {row}")));
        }

        self.path_find.apply_ide_plan(&plan);
        self.register_path_object_model_ids(&plan);
        for record in &plan.records {
            self.apply_ide_record(record);
        }

        self.loaded_ide_plans.push(LoadedIdePlan {
            filename: filename.to_owned(),
            plan: plan.clone(),
        });
        Ok(plan)
    }

    fn apply_ide_record(&mut self, record: &GtaIdeRecord) {
        match record {
            GtaIdeRecord::Object(object) => self.apply_ide_object(object),
            GtaIdeRecord::TimeObject(object) => self.apply_ide_time_object(object),
            GtaIdeRecord::MloHeader(header) => self.apply_ide_mlo_header(header),
            GtaIdeRecord::MloInstance(instance) => self.apply_ide_mlo_instance(instance),
            GtaIdeRecord::Clump(clump) => self.apply_ide_clump(clump),
            GtaIdeRecord::Vehicle(vehicle) => self.apply_ide_vehicle(vehicle),
            GtaIdeRecord::Ped(ped) => self.apply_ide_ped(ped),
            GtaIdeRecord::TwoDfx(two_dfx) => {
                self.model_info.add_2d_effect(two_dfx.clone());
            }
            _ => {}
        }
    }

    fn register_path_object_model_ids(&mut self, plan: &GtaIdePlan) {
        let mut current_path_id = None;
        let mut path_node_index = 0usize;

        for record in &plan.records {
            match record {
                GtaIdeRecord::PathHeader(header) => {
                    current_path_id = Some(header.id);
                    path_node_index = 0;
                }
                GtaIdeRecord::PathNode(node) => {
                    if let Some(id) = current_path_id {
                        if path_node_index == 0 && node.node_type != 0 {
                            self.path_object_model_ids.insert(id);
                        }
                        path_node_index += 1;
                        if path_node_index == 12 {
                            current_path_id = None;
                            path_node_index = 0;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn prepare_objects_before_first_ipl(&mut self, file_mgr: &mut CFileMgr) -> io::Result<()> {
        if self.objects_loaded_for_ipl {
            return Ok(());
        }
        // DEFAULT.DAT/GTA3.DAT author all IDEs before the first IPL, matching
        // re3 LoadLevel. These are global model-table fixups, so running them
        // after every individual IDE needlessly rescans the complete table
        // many times during New Game. Finalising once here preserves the same
        // pre-IPL state while keeping loading work proportional to the data.
        self.model_info.link_time_models();
        self.model_info.setup_big_buildings();
        self.model_info.construct_mlo_clumps();
        self.load_object_data(file_mgr, "DATA\\OBJECT.DAT")?;
        self.objects_loaded_for_ipl = true;
        Ok(())
    }

    fn apply_ide_mlo_header(&mut self, header: &GtaIdeMloHeader) {
        self.model_info
            .add_mlo_header(header.id, &header.name, header.draw_dist);
    }

    fn apply_ide_mlo_instance(&mut self, instance: &GtaIdeMloInstance) {
        self.model_info.add_mlo_instance(
            instance.mlo_id,
            CMloInstance {
                model_index: instance.model_index,
                name: instance.name.clone(),
                position: instance.position,
                scale: instance.scale,
                rotation_axis: instance.rotation_axis,
                rotation_w: instance.angle,
            },
        );
    }

    fn apply_ide_object(&mut self, object: &GtaIdeObject) {
        let model = self.model_info.add_simple_model(object.id);
        model.base.set_model_name(&object.model);
        model
            .base
            .set_tex_dictionary(&mut self.txd_store, &object.txd);
        model.num_atomics = object.num_objs;
        model.lod_distances = object.distances;
        model.first_damaged = object.first_damaged;
        model.flags = SimpleModelFlags::from_re3_flags(object.flags);
    }

    fn apply_ide_time_object(&mut self, object: &GtaIdeTimeObject) {
        let model = self.model_info.add_time_model(object.object.id);
        model.base.set_model_name(&object.object.model);
        model
            .base
            .set_tex_dictionary(&mut self.txd_store, &object.object.txd);
        model.num_atomics = object.object.num_objs;
        model.lod_distances = object.object.distances;
        model.first_damaged = object.object.first_damaged;
        model.flags = SimpleModelFlags::from_re3_flags(object.object.flags);
        model.time_on = Some(object.time_on);
        model.time_off = Some(object.time_off);
    }

    fn apply_ide_clump(&mut self, clump: &GtaIdeClump) {
        let model = self.model_info.add_clump_model(clump.id);
        model.base.set_model_name(&clump.model);
        model
            .base
            .set_tex_dictionary(&mut self.txd_store, &clump.txd);
    }

    fn apply_ide_vehicle(&mut self, vehicle: &GtaIdeVehicle) {
        let model = self.model_info.add_vehicle_model(vehicle.id);
        model.clump.base.set_model_name(&vehicle.model);
        model
            .clump
            .base
            .set_tex_dictionary(&mut self.txd_store, &vehicle.txd);
        model.vehicle_type = vehicle_type_id(&vehicle.vehicle_type);
        model.handling_id = vehicle.handling_id.clone();
        model.game_name = vehicle.game_name.clone();
        model.vehicle_class = vehicle.vehicle_class.clone();
        model.frequency = vehicle.frequency;
        model.level = vehicle.level;
        model.component_rules = vehicle.component_rules;
        if vehicle.vehicle_type == "car" {
            model.wheel_id = vehicle.misc;
            model.wheel_scale = vehicle.wheel_scale;
        } else if vehicle.vehicle_type == "plane" {
            model.plane_lod_id = vehicle.misc;
            model.wheel_scale = 1.0;
        } else if vehicle.vehicle_type == "bike" {
            model.bike_steer_angle = vehicle.misc as f32;
            model.wheel_scale = vehicle.wheel_scale;
        }
    }

    fn apply_ide_ped(&mut self, ped: &GtaIdePed) {
        // re3 `LoadPedModel`: every ped shares `ms_colModelPed1` (three
        // 0.35 spheres). Rust owns per-model volumes, so clone the temp
        // registry entry; its LEVEL_GENERIC level survives level switches.
        // Without this, `col_model(ped)` is None and every ped narrow phase
        // early-outs, leaving the character without collision.
        let ped_col = self.temp_col_models.col_model_ped1.clone();
        let model = self.model_info.add_ped_model(ped.id);
        model.clump.base.set_model_name(&ped.model);
        model
            .clump
            .base
            .set_tex_dictionary(&mut self.txd_store, &ped.txd);
        model.clump.base.set_col_model(ped_col, false);
        model.ped_type = ped.ped_type.clone();
        model.ped_stats = ped.ped_stats.clone();
        model.anim_group = ped.anim_group.clone();
        model.cars_can_drive = ped.cars_can_drive;
        // re3 pins MI_LOPOLYGUY to the same temp col on every PEDS line.
        if let Some(lopoly) = self.model_info.get_model_info_mut(MI_LOPOLYGUY) {
            lopoly
                .base_mut()
                .set_col_model(self.temp_col_models.col_model_ped1.clone(), false);
        }
    }

    pub fn load_clump_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool {
        let Some(summary) = read_clump(bytes) else {
            return false;
        };
        self.model_info
            .set_model_loaded(stream_id, true, Some(summary));
        true
    }

    pub fn start_load_clump_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool {
        let Some(state) = gta_clump_stream_read1(bytes) else {
            return false;
        };
        self.clump_read_state = Some(state);
        self.model_info.set_model_loaded(stream_id, false, None);
        true
    }

    pub fn finish_load_clump_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool {
        let Some(state) = self.clump_read_state.take() else {
            return false;
        };
        let Some(summary) = gta_clump_stream_read2(bytes, &state) else {
            return false;
        };
        self.model_info
            .set_model_loaded(stream_id, true, Some(summary));
        true
    }

    pub fn load_atomic_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool {
        let Some(summary) = read_clump(bytes) else {
            return false;
        };
        let txd_slot = self
            .model_info
            .get_model_info(stream_id)
            .map(|model| model.base().txd_slot);
        let mut bound_any = false;
        for (atomic_index, atomic) in summary.atomics.iter().enumerate() {
            let Some(geometry) = atomic_geometry(&summary, atomic_index, atomic) else {
                continue;
            };
            let frame_name = atomic.frame_name.as_deref().unwrap_or_default();
            let (_, lod_index) = get_name_and_lod(frame_name);
            if self.model_info.set_simple_atomic_binding(
                stream_id,
                lod_index,
                frame_name,
                atomic.clone(),
                geometry.clone(),
            ) {
                if let Some(txd_slot) = txd_slot.and_then(|slot| usize::try_from(slot).ok()) {
                    self.txd_store.add_ref(txd_slot);
                }
                bound_any = true;
            }
        }
        // A valid RW clump is a successful streaming conversion even when it
        // intentionally contains no drawable atomic. GTA3.IMG contains empty
        // LOD holders such as LODcommyland.dff; re3 marks those loaded instead
        // of re-requesting them forever. Keep the summary only when there was
        // no per-LOD binding so the empty/unsupported clump remains observable.
        if !bound_any {
            self.model_info
                .set_model_loaded(stream_id, true, Some(summary));
        }
        true
    }

    pub fn load_txd(&mut self, slot: usize, bytes: &[u8]) -> bool {
        let Some(summary) = read_txd(bytes) else {
            return false;
        };
        self.txd_store.mark_loaded(slot, Some(summary));
        true
    }

    /// Load loose TXD files that are not CD-streamed (re3 `MODELS/GENERIC.TXD` etc.).
    /// Many simple world models reference `generic` (slot 0) for grass/road textures.
    pub fn ensure_loose_txds_loaded(&mut self, file_mgr: &mut CFileMgr) {
        self.ensure_loose_txds_loaded_with_streaming(file_mgr, None);
    }

    /// Same as [`Self::ensure_loose_txds_loaded`] and marks `CStreaming` TXD slots Loaded
    /// so model DFF requests are not blocked waiting for a non-CD TXD stream id.
    pub fn ensure_loose_txds_loaded_with_streaming(
        &mut self,
        file_mgr: &mut CFileMgr,
        mut streaming: Option<&mut CStreaming>,
    ) {
        // CFileLoader::LoadLevel does not bind DEFAULT.DAT TEXDICTION files as
        // independent slots: MISC.TXD and GENERIC.TXD are merged into the
        // current `generic` dictionary. This includes the shared vehicle wheel
        // rasters; loading only GENERIC.TXD produced white wheels.
        if let Some(generic_slot) = self.txd_store.find_txd_slot("generic") {
            let mut merged: Option<RwTexDictionarySummary> = None;
            for loaded in &self.loaded_tex_dictionaries {
                let path = loaded.filename.replace('/', "\\").to_ascii_lowercase();
                if !(path.ends_with("models\\misc.txd") || path.ends_with("models\\generic.txd")) {
                    continue;
                }
                let Some(summary) = loaded.summary.as_ref() else {
                    continue;
                };
                let dst = merged.get_or_insert_with(|| RwTexDictionarySummary {
                    num_textures: 0,
                    first_phase_textures: 0,
                    texture_native_chunks: 0,
                    textures: Vec::new(),
                    bevy_images: Vec::new(),
                });
                dst.num_textures += summary.num_textures;
                dst.first_phase_textures += summary.first_phase_textures;
                dst.texture_native_chunks += summary.texture_native_chunks;
                dst.textures.extend(summary.textures.iter().cloned());
                dst.bevy_images.extend(summary.bevy_images.iter().cloned());
            }
            if let Some(merged) = merged {
                self.txd_store.mark_loaded(generic_slot, Some(merged));
            }
        }

        const LOOSE: &[(&str, &str)] = &[
            ("generic", "MODELS\\GENERIC.TXD"),
            ("particle", "MODELS\\PARTICLE.TXD"),
            ("particle", "MODELS\\particle.txd"),
            ("generic", "MODELS\\generic.txd"),
        ];
        for (name, path) in LOOSE {
            let Some(slot) = self.txd_store.find_txd_slot(name) else {
                continue;
            };
            let _ = self.try_load_txd_slot_from_disk(file_mgr, slot, Some(path));
            self.mark_streaming_txd_loaded(streaming.as_deref_mut(), slot);
        }
    }

    /// Load a TXD slot from `MODELS/<name>.txd` when IMG has no entry (common for generic).
    pub fn try_load_txd_slot_from_disk(
        &mut self,
        file_mgr: &mut CFileMgr,
        slot: usize,
        explicit_path: Option<&str>,
    ) -> bool {
        if self.txd_store.is_txd_loaded(slot) {
            return true;
        }
        let name = self
            .txd_store
            .slot(slot)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        if name.is_empty() {
            return false;
        }
        let candidates: Vec<String> = if let Some(path) = explicit_path {
            vec![path.to_owned()]
        } else {
            vec![
                format!("MODELS\\{name}.txd"),
                format!("MODELS\\{name}.TXD"),
                format!("MODELS\\TXD\\{name}.txd"),
            ]
        };
        for path in candidates {
            if let Ok(bytes) = file_mgr.load_file(&path, 8 * 1024 * 1024, "rb") {
                if self.load_txd(slot, &bytes) {
                    bevy::log::debug!("loaded TXD slot={slot} name={name} from {path}");
                    return true;
                }
            }
        }
        false
    }

    fn mark_streaming_txd_loaded(&mut self, streaming: Option<&mut CStreaming>, slot: usize) {
        if !self.txd_store.is_txd_loaded(slot) {
            return;
        }
        let Some(streaming) = streaming else {
            return;
        };
        streaming.mark_streaming_txd_loaded(slot);
    }

    pub fn start_load_txd(&mut self, slot: usize, bytes: &[u8]) -> bool {
        let Some(state) = gta_tex_dictionary_stream_read1(bytes) else {
            return false;
        };
        self.txd_store
            .mark_started(slot, Some(state.summary.clone()));
        self.txd_read_state = Some(state);
        true
    }

    pub fn finish_load_txd(&mut self, slot: usize, bytes: &[u8]) -> bool {
        let Some(state) = self.txd_read_state.take() else {
            return false;
        };
        let Some(summary) = gta_tex_dictionary_stream_read2(bytes, &state) else {
            return false;
        };
        self.txd_store.mark_loaded(slot, Some(summary));
        true
    }
}

impl CStreamingModelInfo for CFileLoader {
    fn model_name(&self, model_id: usize) -> Option<String> {
        self.model_info
            .get_model_info(model_id)
            .map(|model| model.base().name.clone())
    }

    fn model_id_by_name(&mut self, name: &str) -> Option<usize> {
        self.model_info
            .get_model_info_by_name(name)
            .map(|(id, _)| id)
    }

    fn set_model_name(&mut self, model_id: usize, name: &str) {
        self.model_info.set_model_name(model_id, name);
    }

    fn txd_id_by_name_or_add(&mut self, name: &str) -> Option<usize> {
        Some(self.txd_store.add_txd_slot(name))
    }

    fn txd_id_by_name(&self, name: &str) -> Option<usize> {
        self.txd_store.find_txd_slot(name)
    }

    fn txd_slot_for_model(&self, model_id: usize) -> Option<usize> {
        self.model_info.get_model_info(model_id).and_then(|model| {
            (model.base().txd_slot >= 0).then_some(model.base().txd_slot as usize)
        })
    }

    fn clear_txd_slot_for_model(&mut self, model_id: usize) {
        self.model_info.clear_tex_dictionary(model_id);
    }

    fn set_txd_slot_for_model(&mut self, model_id: usize, name: &str) {
        self.model_info
            .set_tex_dictionary(model_id, &mut self.txd_store, name);
    }

    fn model_type(&self, model_id: usize) -> Option<ModelInfoType> {
        self.model_info
            .get_model_info(model_id)
            .map(|model| model.base().model_type)
    }

    fn model_num_refs(&self, model_id: usize) -> u16 {
        self.model_info
            .get_model_info(model_id)
            .map(|model| model.base().ref_count)
            .unwrap_or(0)
    }

    fn remove_model_ref(&mut self, model_id: usize) -> Option<usize> {
        let model = self.model_info.get_model_info_mut(model_id)?;
        model.base_mut().remove_ref(&mut self.txd_store)
    }

    fn txd_num_refs(&self, txd_id: usize) -> u16 {
        self.txd_store.get_num_refs(txd_id).max(0) as u16
    }

    fn is_txd_loaded(&self, txd_id: usize) -> bool {
        self.txd_store.is_txd_loaded(txd_id)
    }

    fn add_txd_ref(&mut self, txd_id: usize) {
        self.txd_store.add_ref(txd_id);
    }

    fn remove_txd_ref_without_delete(&mut self, txd_id: usize) {
        self.txd_store.remove_ref_without_delete(txd_id);
    }

    fn delete_model_rw_object(&mut self, model_id: usize) {
        if std::env::var_os("RUST_GTA_3_STREAM_DIAG").is_some_and(|value| value != "0") {
            let name = self
                .model_info
                .get_model_info(model_id)
                .map(|model| model.base().name.clone())
                .unwrap_or_default();
            bevy::log::info!(target: "rust_gta_3::streaming", model_id, name,
                "deleting streamed model render object");
        }
        if let Some((txd_slot, binding_count)) =
            self.model_info.clear_simple_atomic_bindings(model_id)
        {
            if let Some(txd_slot) = txd_slot {
                for _ in 0..binding_count {
                    self.txd_store.remove_ref(txd_slot);
                }
            }
            return;
        }
        if let Some(model) = self.model_info.get_model_info_mut(model_id) {
            model.delete_rw_object();
        }
    }

    fn remove_txd(&mut self, txd_id: usize) {
        self.txd_store.remove_txd(txd_id);
    }
}

fn queue_far_rw_object_deletes(
    world: &CWorld,
    streaming: &CStreaming,
    pending: &mut HashSet<usize>,
    sector_x: i32,
    sector_y: i32,
    old_sector_x: i32,
    old_sector_y: i32,
) {
    if sector_x < 0
        || sector_x >= NUMSECTORS_X as i32
        || sector_y < 0
        || sector_y >= NUMSECTORS_Y as i32
    {
        return;
    }
    let is_dont_remove = |entity: &crate::core::World::CWorldEntity| -> bool {
        if entity.streaming_dont_delete {
            return true;
        }
        if entity.model_index < streaming.info_for_model.len()
            && streaming.info_for_model[entity.model_index].flags
                & crate::core::Streaming::STREAMFLAGS_DONT_REMOVE
                != 0
        {
            return true;
        }
        false
    };

    let sector = &world.sectors[sector_y as usize * NUMSECTORS_X + sector_x as usize];
    for &entity_index in sector
        .buildings
        .iter()
        .chain(&sector.objects)
        .chain(&sector.dummies)
    {
        if world
            .entities
            .get(entity_index)
            .is_some_and(|entity| !is_dont_remove(entity))
        {
            pending.insert(entity_index);
        }
    }

    // re3's far-wake pass checks only the building overlap list and deletes an
    // overlap RwObject once the entity's own centre sector is at least two
    // sectors away from the old streaming cursor in either axis.
    for &entity_index in &sector.buildings_overlap {
        let Some(entity) = world.entities.get(entity_index) else {
            continue;
        };
        if is_dont_remove(entity) {
            continue;
        }
        let entity_sector_x = CWorld::get_sector_index_x(entity.position[0]) as i32;
        let entity_sector_y = CWorld::get_sector_index_y(entity.position[1]) as i32;
        if (entity_sector_x - old_sector_x).abs() >= 2
            || (entity_sector_y - old_sector_y).abs() >= 2
        {
            pending.insert(entity_index);
        }
    }
}

fn atomic_geometry<'a>(
    summary: &'a RwClumpSummary,
    atomic_index: usize,
    atomic: &RwAtomicDescriptor,
) -> Option<&'a RwGeometryDescriptor> {
    let geometry_index = if summary.num_geometries > 0 {
        usize::try_from(atomic.geometry_index).ok()?
    } else {
        atomic_index
    };
    summary.geometries.get(geometry_index)
}

impl CStreamingObjectLoader for CFileLoader {
    fn convert_buffer_to_object(
        &mut self,
        stream_id: usize,
        bytes: &[u8],
        context: CStreamingConvertContext,
    ) -> ConvertResult {
        let _audit_timer = GameplayAuditTimer::start(if stream_id >= STREAM_OFFSET_TXD {
            "streaming_convert_txd"
        } else {
            match self.model_type(stream_id) {
                Some(ModelInfoType::Simple | ModelInfoType::Time) => "streaming_convert_atomic",
                Some(ModelInfoType::Vehicle) => "streaming_convert_vehicle",
                Some(_) => "streaming_convert_clump",
                None => "streaming_convert_unknown",
            }
        });
        if stream_id < STREAM_OFFSET_TXD {
            match self.model_type(stream_id) {
                Some(ModelInfoType::Simple | ModelInfoType::Time) => {
                    if self.load_atomic_file(stream_id, bytes) {
                        // re3 `ConvertBufferToObject`: normal streamed simple
                        // models enter at alpha 0 and fade in. NOFADE requests
                        // start opaque; big buildings keep their existing alpha.
                        let alpha =
                            self.model_info
                                .simple_model_info(stream_id)
                                .and_then(|simple| {
                                    (!simple.is_big_building).then_some(
                                        if context.stream_flags & STREAMFLAGS_NOFADE != 0 {
                                            255
                                        } else {
                                            0
                                        },
                                    )
                                });
                        if let Some(alpha) = alpha {
                            self.model_info.set_simple_alpha(stream_id, alpha);
                        }
                        ConvertResult::Loaded
                    } else {
                        ConvertResult::Failed
                    }
                }
                Some(ModelInfoType::Vehicle) => {
                    if self.start_load_clump_file(stream_id, bytes) {
                        ConvertResult::Started
                    } else {
                        ConvertResult::Failed
                    }
                }
                Some(_) => {
                    if self.load_clump_file(stream_id, bytes) {
                        ConvertResult::Loaded
                    } else {
                        ConvertResult::Failed
                    }
                }
                None => ConvertResult::Failed,
            }
        } else {
            let txd_id = stream_id - STREAM_OFFSET_TXD;
            // Prefer single-pass TXD load when the whole dictionary is in `bytes`.
            // Two-phase Start without Finish left TXDs in Started and blocked all DFFs.
            if self.load_txd(txd_id, bytes) {
                ConvertResult::Loaded
            } else if context.loading_big_model || context.cd_size_sectors > 200 {
                if self.start_load_txd(txd_id, bytes) {
                    ConvertResult::Started
                } else {
                    ConvertResult::Failed
                }
            } else {
                ConvertResult::Failed
            }
        }
    }

    fn finish_loading_large_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool {
        if stream_id < STREAM_OFFSET_TXD {
            self.finish_load_clump_file(stream_id, bytes)
        } else {
            self.finish_load_txd(stream_id - STREAM_OFFSET_TXD, bytes)
        }
    }
}

fn vehicle_type_id(vehicle_type: &str) -> i32 {
    match vehicle_type {
        "car" => 0,
        "boat" => 1,
        "train" => 2,
        "heli" => 3,
        "plane" => 4,
        "bike" => 5,
        _ => -1,
    }
}

fn convert_col_model(model: GtaColModel, level: i32) -> CColModel {
    let vertices = model
        .vertices
        .into_iter()
        .map(|vertex| CompressedVector {
            x: vertex.x,
            y: vertex.y,
            z: vertex.z,
        })
        .collect::<Vec<_>>();
    let triangles = model
        .triangles
        .into_iter()
        .map(|triangle| {
            CColTriangle::new(
                triangle.a,
                triangle.b,
                triangle.c,
                triangle.surface,
                triangle.piece,
            )
        })
        .collect::<Vec<_>>();
    let triangle_planes = triangles
        .iter()
        .filter_map(|triangle| CColTrianglePlane::from_triangle(&vertices, triangle))
        .collect::<Vec<_>>();
    let triangle_bounds = triangles
        .iter()
        .filter_map(|triangle| {
            let a = vertices.get(triangle.a as usize)?.get();
            let b = vertices.get(triangle.b as usize)?.get();
            let c = vertices.get(triangle.c as usize)?.get();
            Some((
                CColVector::new(
                    a.x.min(b.x).min(c.x),
                    a.y.min(b.y).min(c.y),
                    a.z.min(b.z).min(c.z),
                ),
                CColVector::new(
                    a.x.max(b.x).max(c.x),
                    a.y.max(b.y).max(c.y),
                    a.z.max(b.z).max(c.z),
                ),
            ))
        })
        .collect::<Vec<_>>();
    CColModel {
        bounding_sphere: convert_sphere(model.bounding_sphere),
        bounding_box: convert_box(model.bounding_box),
        level,
        owns_collision_volumes: true,
        spheres: model.spheres.into_iter().map(convert_sphere).collect(),
        lines: model
            .lines
            .into_iter()
            .map(|line| CColLine::new(convert_vec(line.p0), convert_vec(line.p1)))
            .collect(),
        boxes: model.boxes.into_iter().map(convert_box).collect(),
        vertices,
        triangles,
        triangle_planes,
        triangle_spatial_index: crate::collision::ColModel::TriangleSpatialIndex::from_bounds(&triangle_bounds),
        triangle_bounds,
    }
}

fn convert_vec(vec: GtaColVector) -> CColVector {
    CColVector::new(vec.x, vec.y, vec.z)
}

fn convert_sphere(sphere: GtaColSphere) -> CColSphere {
    CColSphere::new(
        sphere.radius,
        convert_vec(sphere.center),
        sphere.surface,
        sphere.piece,
    )
}

fn convert_box(box_: GtaColBox) -> CColBox {
    CColBox::new(
        convert_vec(box_.min),
        convert_vec(box_.max),
        box_.surface,
        box_.piece,
    )
}

fn read_all_clumps(bytes: &[u8]) -> Vec<RwClumpSummary> {
    let mut cursor = 0;
    let mut clumps = Vec::new();
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() {
        let Some(header) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if header.id == RW_ID_CLUMP {
            if let Some(summary) = read_clump(&bytes[cursor..header.data_end]) {
                clumps.push(summary);
            }
        }
        cursor = header.data_end;
    }
    clumps
}

fn get_name_and_lod(node_name: &str) -> (String, usize) {
    let underscore = node_name
        .as_bytes()
        .windows(2)
        .enumerate()
        .filter_map(|(index, bytes)| {
            (bytes[0] == b'_' && (bytes[1] == b'l' || bytes[1] == b'L')).then_some(index)
        })
        .last();

    if let Some(index) = underscore {
        let lod = node_name
            .get(index + 2..)
            .unwrap_or_default()
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '-')
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(0);
        (node_name[..index].to_owned(), lod)
    } else {
        (node_name.chars().take(24).collect(), 0)
    }
}

#[cfg(test)]
mod streaming_request_cleanup_tests {
    use super::CFileLoader;
    use crate::core::Streaming::{
        CStreaming, CStreamingModelInfo, STREAMFLAGS_KEEP_IN_MEMORY, STREAMFLAGS_PRIORITY,
        StreamLoadState,
    };

    #[test]
    fn flying_components_borrow_the_shared_native_collision_models() {
        use crate::modelinfo::ModelIndices::{MI_CAR_BUMPER, MI_CAR_DOOR, MI_CAR_WHEEL};
        let loader = CFileLoader::default();
        for (id, model) in [
            (MI_CAR_BUMPER, &loader.temp_col_models.col_model_bumper1),
            (MI_CAR_DOOR, &loader.temp_col_models.col_model_door1),
            (MI_CAR_WHEEL, &loader.temp_col_models.col_model_wheel1),
        ] {
            assert!(std::ptr::eq(loader.collision_model(id).unwrap(), model));
        }
        assert!(loader.collision_model(usize::MAX).is_none());
    }

    #[test]
    fn resurrection_scene_barrier_drains_inflight_reads_and_split_conversions() {
        use crate::core::Streaming::{ChannelState, RequestOutcome};
        use rust_gta_3_assets::streaming::CdStream::{CDSTREAM_SECTOR_SIZE, CdStream, StreamStatus};
        use rust_gta_3_formats::formats::ClumpRead::{RW_ID_CLUMP, RW_ID_FRAMELIST, RW_ID_STRUCT};

        fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&id.to_le_bytes());
            bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(payload);
            bytes
        }

        // A valid empty clump exercises the production DFF parser and, for
        // vehicles, both split conversion phases without original game data.
        let mut payload = chunk(RW_ID_STRUCT, &0i32.to_le_bytes());
        payload.extend(chunk(
            RW_ID_FRAMELIST,
            &chunk(RW_ID_STRUCT, &0i32.to_le_bytes()),
        ));
        let mut bytes = chunk(RW_ID_CLUMP, &payload);
        bytes.resize(CDSTREAM_SECTOR_SIZE, 0);
        let temp_dir = std::env::temp_dir().join(format!(
            "rust-gta-3-restart-streaming-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&temp_dir).unwrap();
        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(self.0.join("test.img"));
                let _ = std::fs::remove_dir(&self.0);
            }
        }
        let fixture = Fixture(temp_dir);
        let image = fixture.0.join("test.img");
        std::fs::write(&image, bytes).unwrap();

        for channel in 0..2 {
            for split in [false, true] {
                let mut loader = CFileLoader::default();
                let mut streaming = CStreaming::default();
                let model = 724;
                if split {
                    loader.model_info.add_vehicle_model(model);
                } else {
                    loader.model_info.add_simple_model(model);
                }
                streaming.info_for_model[model].set_cd_posn_and_size(0, 1);
                streaming.image_offsets[0] = 0;
                streaming.image_size = 1;
                streaming.streaming_buffer_size = 1;
                streaming.current_channel = channel;
                let mut cd = CdStream::init(2);
                cd.add_image(&image).unwrap();
                streaming.request_model(model, 0);
                streaming
                    .request_model_stream(channel, &mut cd, &mut loader)
                    .unwrap();
                assert_eq!(cd.sync(channel), StreamStatus::None);
                if split {
                    streaming.process_loading_channel(channel, &cd, &mut loader);
                    assert!(streaming.channel[channel].completed_data.is_some());
                }
                let expected_channel = if split {
                    ChannelState::Started
                } else {
                    ChannelState::Reading
                };
                let expected_model = if split {
                    StreamLoadState::Started
                } else {
                    StreamLoadState::Reading
                };
                assert_eq!(streaming.channel[channel].state, expected_channel);
                assert_eq!(streaming.info_for_model[model].load_state, expected_model);

                // Production resurrection flushes queued requests immediately
                // before beginning the cooperative LoadScene barrier.
                let stale = 725;
                streaming.request_model(stale, STREAMFLAGS_PRIORITY);
                streaming.flush_request_list();
                assert_eq!(
                    streaming.info_for_model[stale].load_state,
                    StreamLoadState::NotLoaded
                );
                assert_eq!(streaming.num_models_requested, 0);
                assert_eq!(streaming.num_priority_requests, 0);
                assert_eq!(streaming.channel[channel].state, expected_channel);
                assert_eq!(streaming.channel[channel].stream_ids[0], model as i32);
                assert_eq!(streaming.info_for_model[model].load_state, expected_model);
                assert_eq!(streaming.request_model(model, 0), RequestOutcome::Updated);

                loader.begin_load_scene_streaming(&mut streaming, [0.0; 3]);
                for _ in 0..8 {
                    if loader
                        .pump_script_streaming_barrier(&mut streaming, &mut cd)
                        .unwrap()
                    {
                        break;
                    }
                }
                assert!(!loader.scene_load_in_progress());
                assert_eq!(
                    streaming.info_for_model[model].load_state,
                    StreamLoadState::Loaded
                );
                assert!(
                    loader
                        .model_info
                        .get_model_info(model)
                        .unwrap()
                        .base()
                        .rw_object_loaded
                );
                assert!(
                    streaming
                        .channel
                        .iter()
                        .all(|c| c.state == ChannelState::Idle)
                );
                assert!(streaming.channel[channel].completed_data.is_none());
                assert_eq!(
                    streaming.request_model(model, 0),
                    RequestOutcome::AlreadyLoaded
                );
            }
        }
    }

    #[test]
    fn restart_reset_rebuilds_world_and_advances_rw_object_epoch() {
        let mut file_loader = CFileLoader::default();
        file_loader.world.process_cutscene_only = true;
        file_loader.world_rw_object_epoch = 17;

        file_loader.reset_for_restart();

        assert!(!file_loader.world.process_cutscene_only);
        assert_eq!(file_loader.world_rw_object_epoch, 18);
    }

    #[test]
    fn cooperative_scene_load_prepares_without_synchronously_draining_requests() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let persistent = 123usize;
        streaming.request_model(persistent, STREAMFLAGS_KEEP_IN_MEMORY);
        let epoch_before = file_loader.world_rw_object_epoch;

        file_loader.begin_load_scene_streaming(&mut streaming, [0.0, 0.0, 0.0]);

        assert!(file_loader.scene_load_in_progress());
        assert_eq!(
            file_loader.world_rw_object_epoch,
            epoch_before.wrapping_add(1),
            "DeleteAllRwObjects generation must advance exactly when the scene barrier begins"
        );
        assert_eq!(
            streaming.info_for_model[persistent].load_state,
            StreamLoadState::InQueue,
            "cooperative begin must not perform LoadAllRequestedModels on the caller's frame"
        );
    }

    #[test]
    fn cooperative_load_all_keeps_existing_requests_behind_the_script_barrier() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let requested = 321usize;
        streaming.request_model(requested, STREAMFLAGS_KEEP_IN_MEMORY);

        file_loader.begin_load_all_models_streaming(&streaming);

        assert!(file_loader.script_streaming_barrier_in_progress());
        assert_eq!(
            streaming.info_for_model[requested].load_state,
            StreamLoadState::InQueue,
            "LOAD_ALL_MODELS_NOW must not synchronously consume the request on the SCM frame"
        );
    }

    #[test]
    fn update_cleanup_drops_only_one_frame_streaming_requests() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let ordinary = 100;
        let keep_in_memory = 101;
        let priority = 102;

        streaming.request_model(ordinary, 0);
        streaming.request_model(keep_in_memory, STREAMFLAGS_KEEP_IN_MEMORY);
        streaming.request_model(priority, STREAMFLAGS_PRIORITY);

        file_loader.remove_non_persistent_requests(&mut streaming);

        assert_eq!(
            streaming.info_for_model[ordinary].load_state,
            StreamLoadState::NotLoaded,
            "re3 removes ordinary CStreaming::Update requests after LoadRequestedModels"
        );
        assert_eq!(
            streaming.info_for_model[keep_in_memory].load_state,
            StreamLoadState::InQueue,
            "KEEP_IN_MEMORY requests must survive the end-of-frame cleanup"
        );
        assert_eq!(
            streaming.info_for_model[priority].load_state,
            StreamLoadState::InQueue,
            "PRIORITY requests must survive the end-of-frame cleanup"
        );
    }

    #[test]
    fn remove_unused_big_buildings_evicts_only_unused_island_connector_lods() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let islands = [
            (5000, "IslandLODInd"),
            (5001, "IslandLODcomIND"),
            (5002, "IslandLODcomSUB"),
            (5003, "IslandLODsubIND"),
            (5004, "IslandLODsubCOM"),
        ];
        for (model_id, name) in islands {
            file_loader.model_info.add_simple_model(model_id);
            file_loader.model_info.set_model_name(model_id, name);
            streaming.info_for_model[model_id].load_state = StreamLoadState::Loaded;
        }

        file_loader.remove_unused_big_buildings(&mut streaming, 1);

        for model_id in [5000, 5002, 5004] {
            assert_eq!(
                streaming.info_for_model[model_id].load_state,
                StreamLoadState::NotLoaded,
                "industrial level must drop the connector LODs unused by re3 RemoveIslandsNotUsed"
            );
        }
        for model_id in [5001, 5003] {
            assert_eq!(
                streaming.info_for_model[model_id].load_state,
                StreamLoadState::Loaded,
                "industrial level must retain the two connector LODs requested by re3 RequestIslands"
            );
        }
    }

    #[test]
    fn cleanup_drops_unserved_ordinary_requests_while_channels_are_busy() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let ordinary = 100;
        streaming.channel[0].state = crate::core::Streaming::ChannelState::Reading;
        streaming.channel[1].state = crate::core::Streaming::ChannelState::Reading;
        streaming.request_model(ordinary, 0);

        file_loader.remove_non_persistent_requests(&mut streaming);

        assert_eq!(
            streaming.info_for_model[ordinary].load_state,
            StreamLoadState::NotLoaded,
            "re3 drops unserved ordinary requests after LoadRequestedModels even while channels read"
        );
        assert_eq!(
            streaming.num_models_requested, 0,
            "stale requests must not keep the loading-priority throttle latched across frames"
        );
    }

    #[test]
    fn cleanup_drops_ordinary_model_and_txd_requests_together() {
        let mut file_loader = CFileLoader::default();
        let mut streaming = CStreaming::default();
        let model_id = 200;
        file_loader.model_info.add_simple_model(model_id);
        file_loader
            .model_info
            .set_tex_dictionary(model_id, &mut file_loader.txd_store, "test_txd");
        let txd_id = file_loader.txd_slot_for_model(model_id).unwrap();
        let txd_stream = txd_id + crate::core::Streaming::STREAM_OFFSET_TXD;

        streaming.request_model(model_id, 0);
        streaming.request_txd(txd_id, 0);

        file_loader.remove_non_persistent_requests(&mut streaming);

        assert_eq!(
            streaming.info_for_model[model_id].load_state,
            StreamLoadState::NotLoaded,
            "re3's end-of-Update sweep drops an unserved ordinary DFF request"
        );
        assert_eq!(
            streaming.info_for_model[txd_stream].load_state,
            StreamLoadState::NotLoaded,
            "re3's end-of-Update sweep drops the matching unserved ordinary TXD request"
        );
    }

    #[test]
    fn queue_far_rw_object_deletes_respects_streaming_dont_remove_flag() {
        use crate::core::World::CWorld;
        let mut world = CWorld::default();
        let mut streaming = CStreaming::default();
        let mut pending = std::collections::HashSet::new();

        let model_dont_remove = 300;
        streaming.info_for_model[model_dont_remove].flags |=
            crate::core::Streaming::STREAMFLAGS_DONT_REMOVE;

        let entity = crate::core::World::CWorldEntity {
            entity_type: crate::core::World::EEntityType::Building,
            model_index: model_dont_remove,
            name: String::new(),
            position: [100.0, 100.0, 10.0],
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 1,
            is_static: true,
            is_visible: true,
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        };
        world.entities.push(entity);
        let entity_idx = 0;

        let sx = CWorld::get_sector_index_x(100.0) as i32;
        let sy = CWorld::get_sector_index_y(100.0) as i32;
        world.sectors[sy as usize * crate::core::World::NUMSECTORS_X + sx as usize]
            .buildings
            .push(entity_idx);

        super::queue_far_rw_object_deletes(
            &world,
            &streaming,
            &mut pending,
            sx,
            sy,
            sx + 5,
            sy + 5,
        );

        assert!(
            !pending.contains(&entity_idx),
            "entities with STREAMFLAGS_DONT_REMOVE must not be queued for wake deletion"
        );
    }
}

// === FAZ10e-fileloader: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn sync_file_loader_clock(clock: Res<CClock>, mut file_loader: ResMut<CFileLoader>) {
    file_loader.clock = clock.clone();
}
