//! re3 reference: references/re-GTA/src/animation/CutsceneMgr.cpp
//! re3 reference: references/re-GTA/src/animation/CutsceneMgr.h
//!
//! `CCutsceneMgr::Initialise` state.

use bevy::prelude::*;

use crate::{
    animation::{
        AnimBlendAssocGroup::CAnimBlendAssocGroup,
        AnimBlendAssociation::{ASSOC_HAS_TRANSLATION, ASSOC_RUNNING, CAnimBlendAssociation},
    },
    math::{Quaternion::CQuaternion, Vector::CVector},
    modelinfo::ModelInfo::ModelInfoSlot,
};

pub const CUTSCENEDIRSIZE: usize = 512;
pub const CUTSCENENAMESIZE: usize = 8;
pub const NUMCUTSCENEOBJECTS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutsceneLoadStatus {
    Loading0,
    LoadingAudio,
    Loading2,
    Loading3,
    Loading4,
}

/// Audio decisions emitted by the same cutscene lifecycle points as re3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutsceneAudioAction {
    EnterMusicModeAndPreload { track_id: Option<u8> },
    PlayPreloaded,
    Stop { restore_game_mode: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutsceneAudioTrack {
    pub name: &'static str,
    pub track_id: i32,
}

pub const CUTSCENE_AUDIO_TRACKS: &[CutsceneAudioTrack] = &[
    CutsceneAudioTrack {
        name: "JB",
        track_id: 15,
    },
    CutsceneAudioTrack {
        name: "BET",
        track_id: 16,
    },
    CutsceneAudioTrack {
        name: "L1_LG",
        track_id: 17,
    },
    CutsceneAudioTrack {
        name: "L2_DSB",
        track_id: 18,
    },
    CutsceneAudioTrack {
        name: "L3_DM",
        track_id: 19,
    },
    CutsceneAudioTrack {
        name: "L4_PAP",
        track_id: 20,
    },
    CutsceneAudioTrack {
        name: "L5_TFB",
        track_id: 21,
    },
    CutsceneAudioTrack {
        name: "J0_DM2",
        track_id: 22,
    },
    CutsceneAudioTrack {
        name: "J1_LFL",
        track_id: 23,
    },
    CutsceneAudioTrack {
        name: "J2_KCL",
        track_id: 24,
    },
    CutsceneAudioTrack {
        name: "J3_VH",
        track_id: 25,
    },
    CutsceneAudioTrack {
        name: "J4_ETH",
        track_id: 26,
    },
    CutsceneAudioTrack {
        name: "J5_DST",
        track_id: 27,
    },
    CutsceneAudioTrack {
        name: "J6_TBJ",
        track_id: 28,
    },
    CutsceneAudioTrack {
        name: "T1_TOL",
        track_id: 29,
    },
    CutsceneAudioTrack {
        name: "T2_TPU",
        track_id: 30,
    },
    CutsceneAudioTrack {
        name: "T3_MAS",
        track_id: 31,
    },
    CutsceneAudioTrack {
        name: "T4_TAT",
        track_id: 32,
    },
    CutsceneAudioTrack {
        name: "T5_BF",
        track_id: 33,
    },
    CutsceneAudioTrack {
        name: "S0_MAS",
        track_id: 34,
    },
    CutsceneAudioTrack {
        name: "S1_PF",
        track_id: 35,
    },
    CutsceneAudioTrack {
        name: "S2_CTG",
        track_id: 36,
    },
    CutsceneAudioTrack {
        name: "S3_RTC",
        track_id: 37,
    },
    CutsceneAudioTrack {
        name: "S5_LRQ",
        track_id: 38,
    },
    CutsceneAudioTrack {
        name: "S4_BDBA",
        track_id: 39,
    },
    CutsceneAudioTrack {
        name: "S4_BDBB",
        track_id: 40,
    },
    CutsceneAudioTrack {
        name: "S2_CTG2",
        track_id: 41,
    },
    CutsceneAudioTrack {
        name: "S4_BDBD",
        track_id: 42,
    },
    CutsceneAudioTrack {
        name: "S5_LRQB",
        track_id: 43,
    },
    CutsceneAudioTrack {
        name: "S5_LRQC",
        track_id: 44,
    },
    CutsceneAudioTrack {
        name: "A1_SS0",
        track_id: 45,
    },
    CutsceneAudioTrack {
        name: "A2_PP",
        track_id: 46,
    },
    CutsceneAudioTrack {
        name: "A3_SS",
        track_id: 47,
    },
    CutsceneAudioTrack {
        name: "A4_PDR",
        track_id: 48,
    },
    CutsceneAudioTrack {
        name: "A5_K2FT",
        track_id: 49,
    },
    CutsceneAudioTrack {
        name: "K1_KBO",
        track_id: 50,
    },
    CutsceneAudioTrack {
        name: "K2_GIS",
        track_id: 51,
    },
    CutsceneAudioTrack {
        name: "K3_DS",
        track_id: 52,
    },
    CutsceneAudioTrack {
        name: "K4_SHI",
        track_id: 53,
    },
    CutsceneAudioTrack {
        name: "K5_SD",
        track_id: 54,
    },
    CutsceneAudioTrack {
        name: "R0_PDR2",
        track_id: 55,
    },
    CutsceneAudioTrack {
        name: "R1_SW",
        track_id: 56,
    },
    CutsceneAudioTrack {
        name: "R2_AP",
        track_id: 57,
    },
    CutsceneAudioTrack {
        name: "R3_ED",
        track_id: 58,
    },
    CutsceneAudioTrack {
        name: "R4_GF",
        track_id: 59,
    },
    CutsceneAudioTrack {
        name: "R5_PB",
        track_id: 60,
    },
    CutsceneAudioTrack {
        name: "R6_MM",
        track_id: 61,
    },
    CutsceneAudioTrack {
        name: "D1_STOG",
        track_id: 62,
    },
    CutsceneAudioTrack {
        name: "D2_KK",
        track_id: 63,
    },
    CutsceneAudioTrack {
        name: "D3_ADO",
        track_id: 64,
    },
    CutsceneAudioTrack {
        name: "D5_ES",
        track_id: 65,
    },
    CutsceneAudioTrack {
        name: "D7_MLD",
        track_id: 66,
    },
    CutsceneAudioTrack {
        name: "D4_GTA",
        track_id: 67,
    },
    CutsceneAudioTrack {
        name: "D4_GTA2",
        track_id: 68,
    },
    CutsceneAudioTrack {
        name: "D6_STS",
        track_id: 69,
    },
    CutsceneAudioTrack {
        name: "A6_BAIT",
        track_id: 70,
    },
    CutsceneAudioTrack {
        name: "A7_ETG",
        track_id: 71,
    },
    CutsceneAudioTrack {
        name: "A8_PS",
        track_id: 72,
    },
    CutsceneAudioTrack {
        name: "A9_ASD",
        track_id: 73,
    },
    CutsceneAudioTrack {
        name: "K4_SHI2",
        track_id: 74,
    },
    CutsceneAudioTrack {
        name: "C1_TEX",
        track_id: 75,
    },
    CutsceneAudioTrack {
        name: "EL_PH1",
        track_id: 76,
    },
    CutsceneAudioTrack {
        name: "EL_PH2",
        track_id: 77,
    },
    CutsceneAudioTrack {
        name: "EL_PH3",
        track_id: 78,
    },
    CutsceneAudioTrack {
        name: "EL_PH4",
        track_id: 79,
    },
    CutsceneAudioTrack {
        name: "YD_PH1",
        track_id: 80,
    },
    CutsceneAudioTrack {
        name: "YD_PH2",
        track_id: 81,
    },
    CutsceneAudioTrack {
        name: "YD_PH3",
        track_id: 82,
    },
    CutsceneAudioTrack {
        name: "YD_PH4",
        track_id: 83,
    },
    CutsceneAudioTrack {
        name: "HD_PH1",
        track_id: 84,
    },
    CutsceneAudioTrack {
        name: "HD_PH2",
        track_id: 85,
    },
    CutsceneAudioTrack {
        name: "HD_PH3",
        track_id: 86,
    },
    CutsceneAudioTrack {
        name: "HD_PH4",
        track_id: 87,
    },
    CutsceneAudioTrack {
        name: "HD_PH5",
        track_id: 88,
    },
    CutsceneAudioTrack {
        name: "MT_PH1",
        track_id: 89,
    },
    CutsceneAudioTrack {
        name: "MT_PH2",
        track_id: 90,
    },
    CutsceneAudioTrack {
        name: "MT_PH3",
        track_id: 91,
    },
    CutsceneAudioTrack {
        name: "MT_PH4",
        track_id: 92,
    },
];

/// One IFP bone sample for cutscene mesh skinning/frame override.
#[derive(Debug, Clone, PartialEq)]
pub struct CutsceneBonePose {
    pub name: String,
    pub position: CVector,
    pub rotation: CQuaternion,
    pub has_translation: bool,
}

/// One hierarchy-node pose sampled from a CUTS.IMG `.ANM` stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutsceneHeadNodePose {
    pub node_index: usize,
    pub position: CVector,
    pub rotation: CQuaternion,
}

/// Full HAnim hierarchy pose used by `CCutsceneHead`. RenderWare's ANM stream
/// interleaves multiple node tracks, so retaining only a single root transform
/// loses the facial/head deformation.
#[derive(Debug, Clone, PartialEq)]
pub struct CutsceneHeadAnimationPose {
    pub nodes: Vec<CutsceneHeadNodePose>,
}

/// RenderWare clump captured by `CClumpModelInfo::CreateInstance` when SCM
/// creates a cutscene object. Special slots 185..189 are reused immediately
/// for other DFF names; the live model-info entry therefore cannot identify an
/// already-created object. BET creates two COLT1 instances from slot 188 and
/// assigns distinct COLT1/COLT2 IFP associations to them.
#[derive(Debug, Clone, PartialEq)]
pub struct CutsceneObjectModelSnapshot {
    pub model_name: String,
    pub txd_slot: i16,
    pub clump_summary: Option<rust_gta_3_formats::formats::ClumpRead::RwClumpSummary>,
    pub vehicle_type: Option<i32>,
    pub vehicle_component_rules: Option<u32>,
    pub wheel_id: Option<i32>,
    pub wheel_scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CutsceneObjectSlot {
    pub model_id: i32,
    pub is_head: bool,
    pub model_snapshot: Option<CutsceneObjectModelSnapshot>,
    /// Optional `extra1..extra6` atomics cloned by
    /// `CVehicleModelInfo::CreateInstance` when a vehicle model is used as a
    /// `CCutsceneObject`.
    pub optional_components: [i8; 2],
    /// Whether re3 has `CWorld::Add`'d this object. `CreateCutsceneObject`
    /// only allocates the C++ object; animated objects enter the world in
    /// `SetupCutsceneToStart` and non-animated ones never do. The head is
    /// world-added immediately by `AddCutsceneHead`.
    pub in_world: bool,
    /// Source cutscene object slot passed to `CREATE_CUTSCENE_HEAD`.
    /// re3 retains this as `CCutsceneHead::m_parentObject` so a separate head
    /// follows the source clump's `Shead` frame rather than remaining at its
    /// spawn position.
    pub parent_object_slot: Option<usize>,
    pub position: CVector,
    /// Root orientation from cutscene IFP (re3 RwFrame matrix). xyzw.
    pub rotation: CQuaternion,
    /// Per-bone IFP samples (sequence name / bone name → pose) for multi-atomic clumps.
    pub bone_poses: Vec<CutsceneBonePose>,
    pub animation: Option<CAnimBlendAssociation>,
    animation_root: Option<super::CutsceneRoot::CutsceneAnimationRoot>,
    pub head_animation_name: Option<String>,
    /// Decoded `RpHAnimAnimation` selected by re3 `CCutsceneHead::PlayAnimation`.
    pub head_animation: Option<rust_gta_3_formats::formats::HAnimRead::RwHAnimAnimation>,
    /// `RpHAnimHierarchy::interpolator->currentTime`. RenderWare resets this to
    /// exactly zero after crossing duration and discards the overflow.
    pub head_animation_time: f32,
    pub head_animation_pose: Option<CutsceneHeadAnimationPose>,
}

fn sample_head_animation(
    animation: &rust_gta_3_formats::formats::HAnimRead::RwHAnimAnimation,
    time_seconds: f32,
) -> Option<CutsceneHeadAnimationPose> {
    let nodes = animation
        .sample_nodes(time_seconds)
        .into_iter()
        .map(|pose| CutsceneHeadNodePose {
            node_index: pose.node_index,
            position: CVector::new(
                pose.translation[0],
                pose.translation[1],
                pose.translation[2],
            ),
            rotation: CQuaternion::new(
                pose.rotation[0],
                pose.rotation[1],
                pose.rotation[2],
                pose.rotation[3],
            ),
        })
        .collect::<Vec<_>>();
    (!nodes.is_empty()).then_some(CutsceneHeadAnimationPose { nodes })
}

#[derive(Debug, Clone, Resource)]
pub struct CCutsceneMgr {
    pub num_cutscene_objs: u32,
    pub cutscene_objects: Vec<CutsceneObjectSlot>,
    pub loaded: bool,
    pub running: bool,
    pub anim_loaded: bool,
    pub cutscene_processing: bool,
    pub use_lod_multiplier: bool,
    pub cutscene_name: String,
    pub cutscene_associations: CAnimBlendAssocGroup,
    pub cutscene_offset: CVector,
    pub cutscene_timer: f32,
    pub cutscene_load_status: CutsceneLoadStatus,
    pub directory_capacity: usize,
    pub directory_path: &'static str,
    pub pending_audio_actions: Vec<CutsceneAudioAction>,
}

impl CutsceneObjectSlot {
    pub fn root_motion_sequence_index(&self) -> Option<usize> {
        self.animation_root.as_ref().map(|root| root.sequence_index)
    }

    fn refresh_animation_root(&mut self) {
        self.animation_root = self.animation.as_ref().and_then(|animation|
            self.model_snapshot.as_ref()?.clump_summary.as_ref().and_then(|summary|
                super::CutsceneRoot::CutsceneAnimationRoot::resolve(&summary.frames, &animation.hierarchy)));
    }
}

impl CCutsceneMgr {
    pub fn initialise() -> Self {
        Self {
            num_cutscene_objs: 0,
            cutscene_objects: Vec::with_capacity(NUMCUTSCENEOBJECTS),
            loaded: false,
            running: false,
            anim_loaded: false,
            cutscene_processing: false,
            use_lod_multiplier: false,
            cutscene_name: String::new(),
            cutscene_associations: CAnimBlendAssocGroup::default(),
            cutscene_offset: CVector::ZERO,
            cutscene_timer: 0.0,
            cutscene_load_status: CutsceneLoadStatus::Loading0,
            directory_capacity: CUTSCENEDIRSIZE,
            directory_path: "ANIM\\CUTS.DIR",
            pending_audio_actions: Vec::new(),
        }
    }

    pub fn start_cutscene_processing(&mut self) {
        self.cutscene_processing = true;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn has_loaded(&self) -> bool {
        self.loaded
    }

    pub fn is_cutscene_processing(&self) -> bool {
        self.cutscene_processing
    }

    pub fn use_lod_multiplier(&self) -> bool {
        self.use_lod_multiplier
    }

    pub fn get_cutscene_object(&self, id: usize) -> Option<&CutsceneObjectSlot> {
        self.cutscene_objects.get(id)
    }

    pub fn get_cutscene_time_in_milliseconds(&self) -> i32 {
        (1000.0 * self.cutscene_timer) as i32
    }

    pub fn get_cutscene_name(&self) -> &str {
        &self.cutscene_name
    }

    pub fn set_cutscene_offset(&mut self, vec: CVector) {
        self.cutscene_offset = vec;
    }

    pub fn load_cutscene_data(&mut self, cutscene_name: &str, anim_loaded: bool) {
        self.cutscene_processing = true;
        self.use_lod_multiplier = cutscene_name.eq_ignore_ascii_case("jb");
        self.cutscene_name = cutscene_name.chars().take(CUTSCENENAMESIZE).collect();
        self.cutscene_timer = 0.0;
        self.loaded = true;
        self.cutscene_offset = CVector::ZERO;
        self.anim_loaded = anim_loaded;
        if !anim_loaded {
            self.cutscene_associations.destroy_associations();
        }
        self.queue_load_audio(cutscene_name);
    }

    /// re3 `CCutsceneMgr::LoadCutsceneData` with IFP hierarchies from CUTS.IMG.
    pub fn load_cutscene_data_with_hierarchies(
        &mut self,
        cutscene_name: &str,
        hierarchies: &[crate::animation::AnimBlendHierarchy::CAnimBlendHierarchy],
    ) {
        self.cutscene_processing = true;
        self.use_lod_multiplier = cutscene_name.eq_ignore_ascii_case("jb");
        self.cutscene_name = cutscene_name.chars().take(CUTSCENENAMESIZE).collect();
        self.cutscene_timer = 0.0;
        self.loaded = true;
        self.cutscene_offset = CVector::ZERO;
        self.cutscene_associations.destroy_associations();
        if hierarchies.is_empty() {
            self.anim_loaded = false;
        } else {
            self.cutscene_associations
                .create_associations_from_hierarchies(hierarchies);
            self.anim_loaded = true;
        }
        self.queue_load_audio(cutscene_name);
    }

    pub fn setup_cutscene_to_start(&mut self) {
        self.cutscene_offset.z += 1.0;
        for object in self.cutscene_objects.iter_mut().rev() {
            if let Some(mut animation) = object.animation.take() {
                // re3 SetupCutsceneToStart: position = offset + first root
                // keyframe, then CWorld::Add, then SetRun.
                animation.set_current_time(0.0);
                animation.set_run();
                object.in_world = Self::sample_object_from_animation(object, &animation, self.cutscene_offset, 0.0);
                object.animation = Some(animation);
            } else {
                // re3: objects without an association are SetPosition'd but
                // never CWorld::Add'd, so they never appear in the cutscene.
                // The head is the exception: AddCutsceneHead already
                // CWorld::Add'd it, so it stays in the world.
                object.position = self.cutscene_offset;
                object.rotation = CQuaternion::IDENTITY;
                if !object.is_head {
                    object.in_world = false;
                }
            }
        }
        self.running = true;
        self.cutscene_timer = 0.0;
    }

    /// Sample all IFP sequences at cutscene time (seconds).
    /// Placement uses sequence zero; movement extracts the actual DFF-bound
    /// root sequence as in CAnimBlendAssociation::Init and FrameUpdate.
    fn sample_object_from_animation(
        object: &mut CutsceneObjectSlot,
        animation: &CAnimBlendAssociation,
        offset: CVector,
        time_seconds: f32,
    ) -> bool {
        object.bone_poses.clear();
        let t = time_seconds.max(0.0);
        if object.animation_root.is_none() {
            object.animation_root = object.model_snapshot.as_ref().and_then(|snapshot|
                snapshot.clump_summary.as_ref().and_then(|summary|
                    super::CutsceneRoot::CutsceneAnimationRoot::resolve(&summary.frames, &animation.hierarchy)));
        }
        let Some(root) = &object.animation_root else {
            bevy::log::warn_once!("cutscene_waiting_original_root_binding: {}", animation.hierarchy.name);
            return false;
        };
        let Some(position) = root.position(&animation.hierarchy, offset, t) else {
            bevy::log::warn_once!("cutscene_original_root_keyframe_missing: {}", animation.hierarchy.name);
            return false;
        };
        object.position = position;
        for (sequence_index, sequence) in animation.hierarchy.sequences.iter().enumerate() {
            let Some(frame) = sequence
                .get_key_frame_at_time(t)
                .or_else(|| sequence.get_key_frame(0))
            else {
                continue;
            };
            let name = if !sequence.name.is_empty() {
                sequence.name.clone()
            } else if let Some(bone_name) =
                crate::animation::Bones::convert_bone_tag_2_bone_name(i32::from(sequence.bone_tag))
            {
                bone_name.to_owned()
            } else {
                continue;
            };
            let has_translation = sequence.has_translation();
            let position = if has_translation {
                frame.translation()
            } else {
                CVector::ZERO
            };
            let rotation = frame.rotation();
            let mut pose_position = position;
            // RpAnimBlend extracts only root translation through velocity3d and
            // CCutsceneObject::ApplyMoveSpeed. Root rotation remains on the
            // animated clump frame; applying it to the object too rotated every
            // bone twice and produced the characteristic actors-lying-down bug.
            if sequence_index == root.sequence_index && has_translation {
                pose_position = CVector::ZERO;
            }
            object.bone_poses.push(CutsceneBonePose {
                name,
                position: pose_position,
                rotation,
                has_translation,
            });
        }
        true
    }

    pub fn set_cutscene_anim(&mut self, anim_name: &str, object_id: usize) {
        let Some(object) = self.cutscene_objects.get_mut(object_id) else {
            bevy::log::warn_once!("cutscene_animation_target_missing: {object_id}");
            return;
        };
        if let Some(mut animation) = self.cutscene_associations.copy_animation_by_name(anim_name) {
            // re3 SetCutsceneAnim: SetCurrentTime(0), ASSOC_HAS_TRANSLATION,
            // and explicitly clear ASSOC_RUNNING — the animation only starts
            // when SetupCutsceneToStart calls SetRun.
            animation.set_current_time(0.0);
            animation.flags |= ASSOC_HAS_TRANSLATION;
            animation.flags &= !ASSOC_RUNNING;
            object.animation = Some(animation);
            object.refresh_animation_root();
        } else {
            bevy::log::warn_once!("cutscene_waiting_original_association: {anim_name}");
        }
    }

    pub fn set_head_anim(&mut self, anim_name: &str, object_id: usize) {
        let full_name = format!("{}_{}", self.cutscene_name, anim_name);
        let Some(object) = self.cutscene_objects.get_mut(object_id) else {
            return;
        };
        object.head_animation_name = Some(full_name);
        object.head_animation = None;
        object.head_animation_time = 0.0;
        object.head_animation_pose = None;
    }

    /// Attach the decoded `.ANM` payload selected by `SET_CUTSCENE_HEAD_ANIM`.
    pub fn set_head_animation_data(
        &mut self,
        object_id: usize,
        animation: rust_gta_3_formats::formats::HAnimRead::RwHAnimAnimation,
    ) {
        let Some(object) = self.cutscene_objects.get_mut(object_id) else {
            return;
        };
        object.head_animation_time = 0.0;
        object.head_animation_pose = sample_head_animation(&animation, 0.0);
        object.head_animation = Some(animation);
    }

    pub fn add_cutscene_head(
        &mut self,
        model_id: i32,
        source_object_slot: usize,
        source_position: CVector,
    ) -> Option<usize> {
        // re3 AddCutsceneHead: CWorld::Add(pHead) immediately after creation.
        let slot = self.push_cutscene_object(model_id, true, true, source_position)?;
        if let Some(head) = self.cutscene_objects.get_mut(slot) {
            head.parent_object_slot = Some(source_object_slot);
        }
        Some(slot)
    }

    pub fn create_cutscene_object(&mut self, model_id: i32) -> Option<usize> {
        // re3 CreateCutsceneObject allocates the CCutsceneObject but leaves it
        // out of the world until SetupCutsceneToStart (animated objects only).
        self.push_cutscene_object(model_id, false, false, CVector::ZERO)
    }

    pub fn set_cutscene_optional_components(&mut self, object_id: usize, components: [i8; 2]) {
        if let Some(object) = self.cutscene_objects.get_mut(object_id) {
            object.optional_components = components;
        }
    }

    pub fn set_cutscene_model_snapshot(
        &mut self,
        object_id: usize,
        snapshot: CutsceneObjectModelSnapshot,
    ) {
        if let Some(object) = self.cutscene_objects.get_mut(object_id) {
            object.model_snapshot = Some(snapshot);
            object.refresh_animation_root();
        }
    }

    pub fn delete_cutscene_data(&mut self) {
        if !self.loaded {
            return;
        }
        let cutscene_name = self.cutscene_name.clone();
        self.cutscene_processing = false;
        self.use_lod_multiplier = false;
        self.cutscene_objects.clear();
        self.num_cutscene_objs = 0;
        self.anim_loaded = false;
        self.running = false;
        self.loaded = false;
        self.cutscene_associations.destroy_associations();
        if !cutscene_name.eq_ignore_ascii_case("end") {
            self.pending_audio_actions.push(CutsceneAudioAction::Stop {
                restore_game_mode: !cutscene_name.eq_ignore_ascii_case("bet"),
            });
        }
    }

    /// Abort a failed original-asset load without manufacturing an empty
    /// cutscene. re3 assumes CUTS.IMG is valid; this only keeps the Rust runtime
    /// from carrying a previous cutscene's objects into the failed request.
    pub fn abort_failed_load(&mut self) {
        self.cutscene_processing = false;
        self.use_lod_multiplier = false;
        self.cutscene_objects.clear();
        self.num_cutscene_objs = 0;
        self.loaded = false;
        self.running = false;
        self.anim_loaded = false;
        self.cutscene_name.clear();
        self.cutscene_timer = 0.0;
        self.cutscene_load_status = CutsceneLoadStatus::Loading0;
        self.cutscene_associations.destroy_associations();
        self.pending_audio_actions.clear();
    }

    /// re3 `CCutsceneMgr::FinishCutscene` — snap timer to camera finish time.
    /// Does not delete data; scripts still see HAS_CUTSCENE_FINISHED then CLEAR.
    pub fn finish_cutscene(&mut self, camera_finish_time_ms: f32) {
        self.cutscene_timer = camera_finish_time_ms * 0.001;
    }

    /// re3 `CCutsceneMgr::Update` loading state machine + timer advance.
    /// Returns true when skip requested while flyby is active (caller finishes camera).
    pub fn update(&mut self, time_step_non_clipped_seconds: f32, skip_requested: bool) -> bool {
        match self.cutscene_load_status {
            CutsceneLoadStatus::LoadingAudio => {
                self.setup_cutscene_to_start();
                if !self.cutscene_name.eq_ignore_ascii_case("end") {
                    self.pending_audio_actions
                        .push(CutsceneAudioAction::PlayPreloaded);
                }
                self.cutscene_load_status = CutsceneLoadStatus::Loading2;
            }
            CutsceneLoadStatus::Loading2 => {
                self.cutscene_load_status = CutsceneLoadStatus::Loading3;
            }
            CutsceneLoadStatus::Loading3 => {
                self.cutscene_load_status = CutsceneLoadStatus::Loading4;
            }
            CutsceneLoadStatus::Loading4 => {
                self.cutscene_load_status = CutsceneLoadStatus::Loading0;
            }
            CutsceneLoadStatus::Loading0 => {}
        }

        // re3 `CCutsceneHead::ProcessControl` runs for any head that is in the
        // world. `AddCutsceneHead` world-adds the head at CREATE_CUTSCENE_HEAD,
        // before the cutscene starts running, so its ANM playhead advances
        // immediately (while the parent actors are still out of the world and
        // their Shead frames sit at the authored DFF pose).
        for object in self.cutscene_objects.iter_mut() {
            if !object.in_world {
                continue;
            }
            if let Some(animation) = object.head_animation.as_ref() {
                // librw `AnimInterpolator::addTime`: non-positive deltas do
                // nothing; crossing (not equalling) duration calls
                // `setCurrentAnim`, resetting to zero without carrying excess.
                if time_step_non_clipped_seconds > 0.0 {
                    object.head_animation_time += time_step_non_clipped_seconds;
                    if object.head_animation_time > animation.summary.duration_seconds {
                        object.head_animation_time = 0.0;
                    }
                }
                object.head_animation_pose =
                    sample_head_animation(animation, object.head_animation_time);
            }
        }

        if !self.running {
            return false;
        }

        self.cutscene_timer += time_step_non_clipped_seconds;
        // Drive cutscene object roots from IFP (re3 anim hierarchy update → frame).
        let offset = self.cutscene_offset;
        for object in self.cutscene_objects.iter_mut() {
            if let Some(mut animation) = object.animation.take() {
                if animation.is_running() {
                    animation.update_time(time_step_non_clipped_seconds, 1.0);
                }
                let sample_time = animation.current_time;
                object.in_world = Self::sample_object_from_animation(object, &animation, offset, sample_time);
                object.animation = Some(animation);
            }
        }
        // re3: skip FinishCutscene when pad cross/space/start during flyby
        // after load status returns to Loading0 (not during "end" cutscene).
        if !self.cutscene_name.eq_ignore_ascii_case("end")
            && self.cutscene_load_status == CutsceneLoadStatus::Loading0
            && skip_requested
        {
            return true;
        }
        false
    }

    /// re3 `CCutsceneMgr::HasCutsceneFinished`.
    pub fn has_cutscene_finished(&self, camera_position_along_spline: f32) -> bool {
        (camera_position_along_spline - 1.0).abs() < 1e-4
    }

    pub fn find_cutscene_audio_track_id(cutscene_name: &str) -> i32 {
        CUTSCENE_AUDIO_TRACKS
            .iter()
            .find(|track| track.name.eq_ignore_ascii_case(cutscene_name))
            .map(|track| track.track_id)
            .unwrap_or(-1)
    }

    pub fn drain_audio_actions(&mut self) -> Vec<CutsceneAudioAction> {
        std::mem::take(&mut self.pending_audio_actions)
    }

    fn queue_load_audio(&mut self, cutscene_name: &str) {
        if cutscene_name.eq_ignore_ascii_case("end") {
            return;
        }
        let track_id = Self::find_cutscene_audio_track_id(cutscene_name)
            .try_into()
            .ok();
        self.pending_audio_actions
            .push(CutsceneAudioAction::EnterMusicModeAndPreload { track_id });
    }

    fn push_cutscene_object(
        &mut self,
        model_id: i32,
        is_head: bool,
        in_world: bool,
        position: CVector,
    ) -> Option<usize> {
        if self.cutscene_objects.len() >= NUMCUTSCENEOBJECTS {
            return None;
        }
        let id = self.cutscene_objects.len();
        self.cutscene_objects.push(CutsceneObjectSlot {
            model_id,
            is_head,
            model_snapshot: None,
            optional_components: [-1; 2],
            in_world,
            parent_object_slot: None,
            position,
            rotation: CQuaternion::IDENTITY,
            bone_poses: Vec::new(),
            animation: None,
            animation_root: None,
            head_animation_name: None,
            head_animation: None,
            head_animation_time: 0.0,
            head_animation_pose: None,
        });
        self.num_cutscene_objs = self.cutscene_objects.len() as u32;
        Some(id)
    }
}

// === FAZ7b: moved from core/Game.rs (verbatim, parity-preserving) ===
// Cutscene manager runtime: tick, load/finish helpers and pad handoff.
// Tests in core/Game.rs consume the finish helpers through re-exports.

#[allow(unused_imports)]
use crate::animation::AnimManager::CAnimManager;
#[allow(unused_imports)]
use crate::core::Camera::{CCamera, CameraScriptMode};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Game::ScriptGameplayBridge;
#[allow(unused_imports)]
use crate::core::Pad::{CPad, CPads};
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use rust_gta_3_assets::assets::AnimationAssets::CAnimationAssets;
#[allow(unused_imports)]
use rust_gta_3_assets::install::FileMgr::CFileMgr;
#[allow(unused_imports)]
use rust_gta_3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
#[allow(unused_imports)]
use rust_gta_3_core::core::{Clock::CClock, GameState::CGameState, Timer::CTimer};
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

pub(crate) struct LoadedCutsceneRuntimeData {
    pub(crate) hierarchies: Vec<crate::animation::AnimBlendHierarchy::CAnimBlendHierarchy>,
    pub(crate) camera_entry: Option<rust_gta_3_assets::assets::AnimationAssets::CutsceneArchiveEntry>,
}

/// Load `{name}.IFP` and `{name}.DAT` through re3's CUTS.DIR/CUTS.IMG path.
/// re3 `CCutsceneHead::PlayAnimation`: find `{cutscene}_{anim}.ANM` in
/// CUTS.DIR/CUTS.IMG and decode its `rwID_HANIMANIMATION` stream.
pub(crate) fn load_cutscene_head_animation(
    head_animation_name: &str,
) -> Result<rust_gta_3_formats::formats::HAnimRead::RwHAnimAnimation, String> {
    let install = OriginalGtaInstall::from_config_file().map_err(|error| error.to_string())?;
    let mut file_mgr = CFileMgr::initialise(install.root());
    let filename = format!("{head_animation_name}.ANM");
    let (_, entry) = CAnimationAssets::load_cutscene_archive_file(&mut file_mgr, &filename)
        .map_err(|error| error.to_string())?;
    let entry = entry.ok_or_else(|| format!("CUTS.IMG does not contain {filename}"))?;
    rust_gta_3_formats::formats::HAnimRead::decode_hanim_animation(&entry.bytes)
}

pub(crate) fn load_cutscene_runtime_data(
    cutscene_name: &str,
) -> Result<LoadedCutsceneRuntimeData, String> {
    let install = OriginalGtaInstall::from_config_file().map_err(|error| error.to_string())?;
    let mut file_mgr = CFileMgr::initialise(install.root());
    let loaded = CAnimationAssets::load_cutscene(&mut file_mgr, cutscene_name)
        .map_err(|error| error.to_string())?;
    let mut hierarchies = Vec::new();
    if let Some(ifp) = loaded.animation_file.as_ref() {
        for block in &ifp.blocks {
            for animation in &block.animations {
                hierarchies.push(crate::animation::AnimManager::hierarchy_from_ifp(animation));
            }
        }
    }
    Ok(LoadedCutsceneRuntimeData {
        hierarchies,
        camera_entry: loaded.camera_entry,
    })
}

/// re3 `DeleteCutsceneData` player, pad and camera restore side effects.
pub fn restore_after_cutscene_clear(
    bridge: &mut ScriptGameplayBridge,
    _game_state: Option<&mut CGameState>,
    cutscene: &mut CCutsceneMgr,
    mut camera: Option<&mut CCamera>,
) {
    cutscene.delete_cutscene_data();
    bridge.cutscene_started = false;
    bridge.cutscene_requested = false;
    bridge.widescreen = false;
    if bridge.player_spawned {
        bridge.player_visible = true;
        bridge.player_control_enabled = true;
    }
    if let Some(camera) = camera.as_mut() {
        // re3 DeleteCutsceneData uses RestoreWithJumpCut, not the normal
        // interpolation restore. The following SCM opcodes may immediately
        // load another cutscene and take spline control again.
        camera.restore_with_jump_cut();
        camera.wide_screen_on = false;
    }
    if let Some(camera) = camera.as_mut() {
        camera.cutscene_finished = false;
    }
}

/// re3 `FinishCutscene` side effects: snap mgr timer + camera spline end + show player.
/// Control stays off until CLEAR/delete (re3 pad re-enable is DeleteCutsceneData).
pub fn apply_cutscene_finish(
    cutscene: &mut CCutsceneMgr,
    camera: &mut CCamera,
    bridge: &mut ScriptGameplayBridge,
) {
    let finish_ms = if camera.flyby_finish_time_ms > 0 {
        camera.flyby_finish_time_ms as f32
    } else {
        (cutscene.cutscene_timer * 1000.0).max(0.0)
    };
    cutscene.finish_cutscene(finish_ms);
    camera.finish_cutscene();
    if bridge.player_spawned {
        bridge.player_visible = true;
    }
}

pub(crate) fn update_cutscene_mgr(
    mut cutscene: Option<ResMut<CCutsceneMgr>>,
    mut camera: Option<ResMut<CCamera>>,
    mut bridge: Option<ResMut<ScriptGameplayBridge>>,
    file_loader: Option<Res<CFileLoader>>,
    pads: Option<Res<CPads>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    timer: Res<CTimer>,
) {
    let Some(cutscene) = cutscene.as_mut() else {
        return;
    };
    if let Some(loader) = file_loader.as_deref() {
        if cutscene.loaded || cutscene.running {
            for (slot, object) in cutscene.cutscene_objects.iter_mut().enumerate() {
                if object
                    .model_snapshot
                    .as_ref()
                    .and_then(|s| s.clump_summary.as_ref())
                    .is_none()
                    && object.model_id >= 0
                {
                    if let Some(snapshot) =
                        cutscene_model_snapshot(loader, object.model_id as usize)
                    {
                        let same_model_identity =
                            object.model_snapshot.as_ref().is_none_or(|old| {
                                old.model_name.eq_ignore_ascii_case(&snapshot.model_name)
                                    && old.txd_slot == snapshot.txd_slot
                            });
                        if same_model_identity && snapshot.clump_summary.is_some() {
                            object.model_snapshot = Some(snapshot);
                            let _ = slot;
                        }
                    }
                }
            }
        }
    }
    // re3 skip: Cross / Start / space / Enter / LMB — map from pad state.
    let skip_requested = pads.as_ref().is_some_and(|pads| {
        let p = pads.get_pad(0);
        p.cross_just_down()
            || (bridge.as_deref().is_some_and(|b| b.playing_intro) && p.start_just_down())
    }) || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Space)
        || mouse.just_pressed(MouseButton::Left);

    let was_running = cutscene.running;
    let camera_offset_before_setup = cutscene.cutscene_offset;
    let want_finish_skip =
        cutscene.update(timer.time_step_non_clipped_in_seconds(), skip_requested);
    if !was_running && cutscene.running {
        if let Some(camera) = camera.as_mut() {
            camera.set_cam_cutscene_offset(rust_gta_3_core::core::Coordinates::CVector::new(
                camera_offset_before_setup.x,
                camera_offset_before_setup.y,
                camera_offset_before_setup.z,
            ));
            camera.take_control_with_spline();
            camera.wide_screen_on = true;
        }
    }
    if cutscene.running {
        if let Some(camera) = camera.as_mut() {
            if camera.script_mode == CameraScriptMode::FlyBy {
                let spline_failed = camera
                    .update_cutscene_spline(timer.time_step_non_clipped_in_milliseconds())
                    .inspect_err(|error| {
                        error!(
                            "CCam::Process_FlyBy failed; finishing cutscene to avoid a permanent fly-by lock: {error}"
                        );
                    })
                    .is_err();
                let at_end =
                    camera.cutscene_finished || (camera.position_along_spline - 1.0).abs() < 1e-4;
                // Safety net: a mission cutscene must never leave the screen
                // black because the spline stopped advancing. re3 ends the
                // cutscene at the authored camera finish time, so once the
                // timer passes that (with a small settle margin) finish it even
                // if `position_along_spline` never snapped to 1.
                let time_exceeded = camera.flyby_finish_time_ms > 0
                    && cutscene.cutscene_timer * 1000.0
                        >= camera.flyby_finish_time_ms as f32 + 100.0;
                if want_finish_skip || at_end || spline_failed || time_exceeded {
                    if let Some(bridge) = bridge.as_mut() {
                        apply_cutscene_finish(cutscene, camera, bridge);
                    } else {
                        let finish_ms = camera.flyby_finish_time_ms as f32;
                        cutscene.finish_cutscene(finish_ms);
                        camera.finish_cutscene();
                    }
                }
            } else if want_finish_skip {
                if let Some(bridge) = bridge.as_mut() {
                    apply_cutscene_finish(cutscene, camera, bridge);
                }
            }
        }
    }
}

/// Keep the CPad ownership mask in lockstep with the re3 cutscene lifecycle.
///
/// The original engine disables the `PLAYERCONTROL_CUTSCENE` owner in
/// `CCutsceneMgr::LoadCutsceneData` and releases that exact owner from
/// `DeleteCutsceneData`.  The bridge boolean is not sufficient: input is
/// consumed by `CPad` before the world step, so a stale mask can leave the
/// player apparently frozen after the intro even though the script says
/// control is enabled.
pub(crate) fn sync_cutscene_pad_controls(
    cutscene: Option<Res<CCutsceneMgr>>,
    _bridge: Res<ScriptGameplayBridge>,
    mut pads: ResMut<CPads>,
) {
    let cutscene_active = cutscene.as_ref().is_some_and(|cutscene| {
        cutscene.cutscene_processing || cutscene.loaded || cutscene.running
    });
    if cutscene_active {
        pads.get_pad_mut(0)
            .set_disable_player_controls(crate::core::Pad::PLAYERCONTROL_CUTSCENE);
    } else {
        pads.get_pad_mut(0)
            .set_enable_player_controls(crate::core::Pad::PLAYERCONTROL_CUTSCENE);
    }
}

// === FAZ10e-cutscene: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn cutscene_model_snapshot(
    file_loader: &CFileLoader,
    model_id: usize,
) -> Option<crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot> {
    let model = file_loader.model_info.get_model_info(model_id)?;
    let base = model.base();
    let vehicle = match model {
        ModelInfoSlot::Vehicle(vehicle) => Some(vehicle),
        _ => None,
    };
    Some(crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot {
        model_name: base.name.clone(),
        txd_slot: base.txd_slot,
        clump_summary: base.clump_summary.clone(),
        vehicle_type: vehicle.map(|vehicle| vehicle.vehicle_type),
        vehicle_component_rules: vehicle.map(|vehicle| vehicle.component_rules),
        wheel_id: vehicle.map(|vehicle| vehicle.wheel_id),
        wheel_scale: vehicle.map(|vehicle| vehicle.wheel_scale),
    })
}

// FAZ8c-pad: moved to core/Pad.rs verbatim (2 items): fn update_gameplay_mouse_capture, fn update_pads_from_bevy_input

// FAZ9d: moved to collision/Collision.rs verbatim (7 items): fn object_static_collision_step, fn object_static_contacts_for_matrix, fn integrate_physical_matrix, fn first_object_static_collision, fn object_collision_substeps, fn apply_object_static_contacts, fn object_static_process_shift
