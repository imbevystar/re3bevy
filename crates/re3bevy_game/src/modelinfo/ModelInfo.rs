//! re3 reference: references/re-GTA/src/modelinfo/ModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/ModelInfo.h
//!
//! Model info pointer table and typed stores. RenderWare object pointers are
//! represented by loaded markers until the RW object graph is ported.

use crate::collision::ColModel::{CColModel, LEVEL_GENERIC};
use crate::core::Streaming::{MODELINFOSIZE, ModelInfoType};
use crate::modelinfo::ModelIndices::{
    MI_BODYPARTA, MI_BODYPARTB, MI_CAR_BONNET, MI_CAR_BOOT, MI_CAR_BUMPER, MI_CAR_DOOR,
    MI_CAR_PANEL, MI_CAR_WHEEL,
};
use crate::rw::TxdStore::CTxdStore;
use re3bevy_formats::formats::ClumpRead::{
    RwAtomicDescriptor, RwClumpSummary, RwGeometryDescriptor,
};

pub const LOD_DISTANCE: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultColModelKind {
    Door,
    Bumper,
    Panel,
    Bonnet,
    Boot,
    Wheel,
    BodyPartA,
    BodyPartB,
}

#[derive(Debug, Clone)]
pub struct CBaseModelInfo {
    pub name: String,
    pub object_id: i16,
    pub ref_count: u16,
    pub txd_slot: i16,
    pub model_type: ModelInfoType,
    pub num_2d_effects: u8,
    pub owns_col_model: bool,
    pub default_col_model: Option<DefaultColModelKind>,
    pub col_model: Option<CColModel>,
    pub rw_object_loaded: bool,
    pub clump_summary: Option<RwClumpSummary>,
    /// Program metadata identifying replacement/retirement of this resident object.
    rw_object_revision: u64,
}

impl CBaseModelInfo {
    pub fn new(model_type: ModelInfoType) -> Self {
        Self {
            name: String::new(),
            object_id: -1,
            ref_count: 0,
            txd_slot: -1,
            model_type,
            num_2d_effects: 0,
            owns_col_model: false,
            default_col_model: None,
            col_model: None,
            rw_object_loaded: false,
            clump_summary: None,
            rw_object_revision: 0,
        }
    }

    pub fn set_model_name(&mut self, name: &str) {
        self.name = name.chars().take(24).collect();
    }

    pub fn set_tex_dictionary(&mut self, txd_store: &mut CTxdStore, name: &str) {
        let slot = txd_store.add_txd_slot(name);
        self.txd_slot = slot as i16;
    }

    pub fn add_ref(&mut self, txd_store: &mut CTxdStore) {
        self.ref_count += 1;
        if self.txd_slot >= 0 {
            txd_store.add_ref(self.txd_slot as usize);
        }
    }

    pub fn remove_ref(&mut self, txd_store: &mut CTxdStore) -> Option<usize> {
        self.ref_count = self.ref_count.saturating_sub(1);
        if self.txd_slot >= 0 && txd_store.remove_ref(self.txd_slot as usize) {
            Some(self.txd_slot as usize)
        } else {
            None
        }
    }

    pub fn delete_rw_object(&mut self) {
        if self.rw_object_loaded || self.clump_summary.is_some() {
            self.advance_rw_object_revision();
        }
        self.rw_object_loaded = false;
        self.clump_summary = None;
    }

    pub fn rw_object_revision(&self) -> u64 {
        self.rw_object_revision
    }

    pub(crate) fn advance_rw_object_revision(&mut self) {
        self.rw_object_revision = self.rw_object_revision.wrapping_add(1);
    }

    pub fn set_col_model(&mut self, col_model: CColModel, owns_col_model: bool) {
        self.col_model = Some(col_model);
        self.owns_col_model = owns_col_model;
    }

    pub fn is_simple(&self) -> bool {
        self.model_type.is_simple()
    }
}

#[derive(Debug, Clone)]
pub struct CSimpleModelInfo {
    pub base: CBaseModelInfo,
    pub lod_distances: [f32; 3],
    pub num_atomics: u8,
    pub first_damaged: u8,
    pub flags: SimpleModelFlags,
    pub time_on: Option<i32>,
    pub time_off: Option<i32>,
    pub alpha: u8,
    pub is_big_building: bool,
    /// `CSimpleModelInfo::m_atomics[2]` when it is repurposed as the related
    /// non-LOD/LOD model by `SetupBigBuilding`.
    pub related_model_id: Option<usize>,
    /// `CTimeModelInfo::m_otherTimeModelID`. This is a distinct C++ member;
    /// sharing `related_model_id` made time-of-day pairs overwrite LOD links.
    pub other_time_model_id: Option<usize>,
    pub atomic_bindings: Vec<SimpleAtomicBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleAtomicBinding {
    pub lod_index: usize,
    pub frame_name: String,
    pub atomic: RwAtomicDescriptor,
    pub geometry: RwGeometryDescriptor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SimpleModelFlags {
    pub normal_cull: bool,
    pub no_fade: bool,
    pub draw_last: bool,
    pub additive: bool,
    pub is_subway: bool,
    pub ignore_light: bool,
    pub no_zwrite: bool,
}

impl CSimpleModelInfo {
    fn new(model_type: ModelInfoType) -> Self {
        Self {
            base: CBaseModelInfo::new(model_type),
            lod_distances: [0.0; 3],
            num_atomics: 0,
            first_damaged: 0,
            flags: SimpleModelFlags::default(),
            time_on: None,
            time_off: None,
            alpha: 0,
            is_big_building: false,
            related_model_id: None,
            other_time_model_id: None,
            atomic_bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CClumpModelInfo {
    pub base: CBaseModelInfo,
    pub mlo: Option<CMloModelInfo>,
}

impl CClumpModelInfo {
    fn new(model_type: ModelInfoType) -> Self {
        Self {
            base: CBaseModelInfo::new(model_type),
            mlo: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMloModelInfo {
    pub draw_dist: f32,
    pub first_instance: usize,
    pub last_instance: usize,
    pub constructed_clump: Option<ConstructedMloClump>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructedMloClump {
    pub atomic_instances: Vec<ConstructedMloAtomic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructedMloAtomic {
    pub model_index: usize,
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_axis: [f32; 3],
    pub rotation_w: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CMloInstance {
    pub model_index: usize,
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_axis: [f32; 3],
    pub rotation_w: f32,
}

#[derive(Debug, Clone)]
pub struct CVehicleModelInfo {
    pub clump: CClumpModelInfo,
    pub(super) authored_positions: super::VehicleModelInfo::VehicleModelPositions,
    pub vehicle_type: i32,
    pub wheel_id: i32,
    pub plane_lod_id: i32,
    pub wheel_scale: f32,
    pub handling_id: String,
    pub game_name: String,
    pub vehicle_class: String,
    pub level: i32,
    pub component_rules: u32,
    pub frequency: u32,
    pub bike_steer_angle: f32,
    pub num_colours: usize,
    pub colours1: [u8; NUM_VEHICLE_COLOURS],
    pub colours2: [u8; NUM_VEHICLE_COLOURS],
    /// re3 `CVehicleModelInfo::m_lastColorVariation`.
    pub last_colour_variation: usize,
}

pub const NUM_VEHICLE_COLOURS: usize = 8;

impl CVehicleModelInfo {
    pub fn lamp_positions(&self) -> Option<&super::VehicleModelInfo::VehicleLampPositions> {
        self.authored_positions()
    }

    pub fn authored_positions(&self) -> Option<&super::VehicleModelInfo::VehicleModelPositions> {
        (self.clump.base.rw_object_loaded && self.clump.base.clump_summary.is_some())
            .then_some(&self.authored_positions)
    }

    fn new() -> Self {
        Self {
            clump: CClumpModelInfo::new(ModelInfoType::Vehicle),
            authored_positions: Default::default(),
            vehicle_type: -1,
            wheel_id: -1,
            plane_lod_id: -1,
            wheel_scale: 1.0,
            handling_id: String::new(),
            game_name: String::new(),
            vehicle_class: String::new(),
            level: 0,
            component_rules: 0,
            frequency: 0,
            bike_steer_angle: 999.99,
            num_colours: 0,
            colours1: [0; NUM_VEHICLE_COLOURS],
            colours2: [0; NUM_VEHICLE_COLOURS],
            last_colour_variation: 0,
        }
    }

    pub fn choose_vehicle_colour(&mut self, current: Option<(u8, u8)>) -> (u8, u8) {
        if self.num_colours == 0 {
            return (0, 0);
        }
        self.last_colour_variation = (self.last_colour_variation + 1) % self.num_colours;
        let mut colour = (
            self.colours1[self.last_colour_variation],
            self.colours2[self.last_colour_variation],
        );
        if self.num_colours > 1 && current == Some(colour) {
            self.last_colour_variation = (self.last_colour_variation + 1) % self.num_colours;
            colour = (
                self.colours1[self.last_colour_variation],
                self.colours2[self.last_colour_variation],
            );
        }
        colour
    }
}

#[derive(Debug, Clone)]
pub struct CPedModelInfo {
    pub clump: CClumpModelInfo,
    pub ped_type: String,
    pub ped_stats: String,
    pub anim_group: String,
    pub cars_can_drive: u32,
    pub(crate) hit_spheres: Vec<super::PedModelInfo::PedHitSphere>,
}

impl CPedModelInfo {
    fn new() -> Self {
        Self {
            clump: CClumpModelInfo::new(ModelInfoType::Ped),
            ped_type: String::new(),
            ped_stats: String::new(),
            anim_group: String::new(),
            cars_can_drive: 0,
            hit_spheres: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModelInfoSlot {
    Simple(CSimpleModelInfo),
    Mlo(CClumpModelInfo),
    Time(CSimpleModelInfo),
    Clump(CClumpModelInfo),
    Ped(CPedModelInfo),
    Vehicle(CVehicleModelInfo),
    XtraComps(CClumpModelInfo),
}

impl ModelInfoSlot {
    pub fn delete_rw_object(&mut self) {
        if let Self::Vehicle(model) = self {
            model.authored_positions = Default::default();
        }
        if let Self::Ped(model) = self {
            model.hit_spheres.clear();
        }
        self.base_mut().delete_rw_object();
    }

    pub fn base(&self) -> &CBaseModelInfo {
        match self {
            Self::Simple(model) | Self::Time(model) => &model.base,
            Self::Mlo(model) | Self::Clump(model) | Self::XtraComps(model) => &model.base,
            Self::Ped(model) => &model.clump.base,
            Self::Vehicle(model) => &model.clump.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut CBaseModelInfo {
        match self {
            Self::Simple(model) | Self::Time(model) => &mut model.base,
            Self::Mlo(model) | Self::Clump(model) | Self::XtraComps(model) => &mut model.base,
            Self::Ped(model) => &mut model.clump.base,
            Self::Vehicle(model) => &mut model.clump.base,
        }
    }
}

use re3bevy_formats::formats::FileLoader::GtaTwoDfx;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CModelInfo {
    model_info_ptrs: Vec<Option<ModelInfoSlot>>,
    mlo_instances: Vec<CMloInstance>,
    pub two_d_effects: HashMap<usize, Vec<GtaTwoDfx>>,
}

impl CModelInfo {
    pub fn initialise_empty() -> Self {
        Self {
            model_info_ptrs: vec![None; MODELINFOSIZE],
            mlo_instances: Vec::new(),
            two_d_effects: HashMap::new(),
        }
    }

    pub fn add_2d_effect(&mut self, effect: GtaTwoDfx) {
        let id = effect.id;
        if let Some(slot) = self.get_model_info_mut(id) {
            slot.base_mut().num_2d_effects = slot.base().num_2d_effects.saturating_add(1);
        }
        self.two_d_effects.entry(id).or_default().push(effect);
    }

    pub fn get_2d_effects(&self, id: usize) -> Option<&[GtaTwoDfx]> {
        self.two_d_effects.get(&id).map(|v| v.as_slice())
    }

    pub fn initialise(txd_store: &mut CTxdStore) -> Self {
        let mut model_info = Self::initialise_empty();
        model_info.add_default_simple_model(MI_CAR_DOOR, DefaultColModelKind::Door, txd_store);
        model_info.add_default_simple_model(MI_CAR_BUMPER, DefaultColModelKind::Bumper, txd_store);
        model_info.add_default_simple_model(MI_CAR_PANEL, DefaultColModelKind::Panel, txd_store);
        model_info.add_default_simple_model(MI_CAR_BONNET, DefaultColModelKind::Bonnet, txd_store);
        model_info.add_default_simple_model(MI_CAR_BOOT, DefaultColModelKind::Boot, txd_store);
        model_info.add_default_simple_model(MI_CAR_WHEEL, DefaultColModelKind::Wheel, txd_store);
        model_info.add_default_simple_model(
            MI_BODYPARTA,
            DefaultColModelKind::BodyPartA,
            txd_store,
        );
        model_info.add_default_simple_model(
            MI_BODYPARTB,
            DefaultColModelKind::BodyPartB,
            txd_store,
        );
        model_info
    }

    pub fn shutdown(&mut self) {
        self.model_info_ptrs.fill(None);
    }

    pub fn add_simple_model(&mut self, id: usize) -> &mut CSimpleModelInfo {
        self.model_info_ptrs[id] = Some(ModelInfoSlot::Simple(CSimpleModelInfo::new(
            ModelInfoType::Simple,
        )));
        match self.model_info_ptrs[id].as_mut().unwrap() {
            ModelInfoSlot::Simple(model) => model,
            _ => unreachable!(),
        }
    }

    pub fn add_mlo_model(&mut self, id: usize) -> &mut CClumpModelInfo {
        self.model_info_ptrs[id] =
            Some(ModelInfoSlot::Mlo(CClumpModelInfo::new(ModelInfoType::Mlo)));
        self.clump_like_mut(id)
    }

    pub fn add_mlo_header(&mut self, id: usize, name: &str, draw_dist: f32) {
        let first_instance = self.mlo_instances.len();
        let model = self.add_mlo_model(id);
        model.base.set_model_name(name);
        model.base.txd_slot = -1;
        model.mlo = Some(CMloModelInfo {
            draw_dist,
            first_instance,
            last_instance: first_instance,
            constructed_clump: None,
        });
    }

    pub fn add_mlo_instance(&mut self, mlo_id: usize, instance: CMloInstance) {
        let index = self.mlo_instances.len();
        self.mlo_instances.push(instance);
        if let Some(ModelInfoSlot::Mlo(model)) = self.get_model_info_mut(mlo_id)
            && let Some(mlo) = &mut model.mlo
        {
            mlo.last_instance = index + 1;
        }
    }

    pub fn construct_mlo_clumps(&mut self) {
        let mlo_ids: Vec<usize> = self
            .model_info_ptrs
            .iter()
            .enumerate()
            .filter_map(|(id, model)| matches!(model, Some(ModelInfoSlot::Mlo(_))).then_some(id))
            .collect();

        for id in mlo_ids {
            let Some(ModelInfoSlot::Mlo(model)) = self.get_model_info(id) else {
                continue;
            };
            let Some(mlo) = &model.mlo else {
                continue;
            };
            let first = mlo.first_instance;
            let last = mlo.last_instance;
            let atomic_instances = self.mlo_instances[first..last]
                .iter()
                .filter(|instance| {
                    self.simple_model_info(instance.model_index)
                        .is_some_and(|simple| !simple.atomic_bindings.is_empty())
                })
                .map(|instance| ConstructedMloAtomic {
                    model_index: instance.model_index,
                    name: instance.name.clone(),
                    position: instance.position,
                    scale: instance.scale,
                    rotation_axis: instance.rotation_axis,
                    rotation_w: instance.rotation_w,
                })
                .collect::<Vec<_>>();

            if let Some(ModelInfoSlot::Mlo(model)) = self.get_model_info_mut(id)
                && let Some(mlo) = &mut model.mlo
            {
                mlo.constructed_clump = (!atomic_instances.is_empty())
                    .then_some(ConstructedMloClump { atomic_instances });
                model.base.rw_object_loaded = mlo.constructed_clump.is_some();
            }
        }
    }

    pub fn add_time_model(&mut self, id: usize) -> &mut CSimpleModelInfo {
        self.model_info_ptrs[id] = Some(ModelInfoSlot::Time(CSimpleModelInfo::new(
            ModelInfoType::Time,
        )));
        match self.model_info_ptrs[id].as_mut().unwrap() {
            ModelInfoSlot::Time(model) => model,
            _ => unreachable!(),
        }
    }

    pub fn add_clump_model(&mut self, id: usize) -> &mut CClumpModelInfo {
        self.model_info_ptrs[id] = Some(ModelInfoSlot::Clump(CClumpModelInfo::new(
            ModelInfoType::Clump,
        )));
        self.clump_like_mut(id)
    }

    pub fn add_ped_model(&mut self, id: usize) -> &mut CPedModelInfo {
        self.model_info_ptrs[id] = Some(ModelInfoSlot::Ped(CPedModelInfo::new()));
        match self.model_info_ptrs[id].as_mut().unwrap() {
            ModelInfoSlot::Ped(model) => model,
            _ => unreachable!(),
        }
    }

    pub fn add_vehicle_model(&mut self, id: usize) -> &mut CVehicleModelInfo {
        self.model_info_ptrs[id] = Some(ModelInfoSlot::Vehicle(CVehicleModelInfo::new()));
        match self.model_info_ptrs[id].as_mut().unwrap() {
            ModelInfoSlot::Vehicle(model) => model,
            _ => unreachable!(),
        }
    }

    pub fn get_model_info(&self, id: usize) -> Option<&ModelInfoSlot> {
        self.model_info_ptrs.get(id).and_then(Option::as_ref)
    }

    pub fn get_model_info_mut(&mut self, id: usize) -> Option<&mut ModelInfoSlot> {
        self.model_info_ptrs.get_mut(id).and_then(Option::as_mut)
    }

    pub fn get_model_info_by_name(&self, name: &str) -> Option<(usize, &ModelInfoSlot)> {
        self.model_info_ptrs
            .iter()
            .enumerate()
            .find_map(|(id, model)| {
                let model = model.as_ref()?;
                model
                    .base()
                    .name
                    .eq_ignore_ascii_case(name)
                    .then_some((id, model))
            })
    }

    pub fn simple_model_info_mut(&mut self, id: usize) -> Option<&mut CSimpleModelInfo> {
        match self.get_model_info_mut(id)? {
            ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model) => Some(model),
            _ => None,
        }
    }

    pub fn simple_model_info(&self, id: usize) -> Option<&CSimpleModelInfo> {
        match self.get_model_info(id)? {
            ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model) => Some(model),
            _ => None,
        }
    }

    /// re3 `CBaseModelInfo::GetRwObject` / `CSimpleModelInfo::GetRwObject`.
    /// Simple/time models expose only `m_atomics[0]` as their model-level RW
    /// object; a resident higher LOD atomic must not make that test succeed.
    pub fn rw_object_loaded(&self, id: usize) -> bool {
        match self.get_model_info(id) {
            Some(ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model)) => model
                .atomic_bindings
                .iter()
                .any(|binding| binding.lod_index == 0),
            Some(model) => model.base().rw_object_loaded,
            None => false,
        }
    }

    pub fn simple_related_model_id(&self, id: usize) -> Option<usize> {
        self.simple_model_info(id)?.related_model_id
    }

    pub fn col_model(&self, id: usize) -> Option<&CColModel> {
        self.get_model_info(id)?.base().col_model.as_ref()
    }

    pub fn remove_col_models_from_other_levels(&mut self, level: i32) -> usize {
        let mut removed = 0;
        for model in self.model_info_ptrs.iter_mut().flatten() {
            let Some(col_model) = model.base_mut().col_model.as_mut() else {
                continue;
            };
            if col_model.level != LEVEL_GENERIC && col_model.level != level {
                col_model.remove_collision_volumes();
                removed += 1;
            }
        }
        removed
    }

    pub fn set_model_loaded(&mut self, id: usize, loaded: bool, summary: Option<RwClumpSummary>) {
        if let Some(model) = self.get_model_info_mut(id) {
            if loaded || model.base().rw_object_loaded || model.base().clump_summary.is_some() {
                model.base_mut().advance_rw_object_revision();
            }
            if let ModelInfoSlot::Ped(ped) = model {
                ped.hit_spheres = if loaded {
                    summary.as_ref().map(CPedModelInfo::resolve_hit_spheres).unwrap_or_default()
                } else { Vec::new() };
            }
            if let ModelInfoSlot::Vehicle(vehicle) = model {
                vehicle.authored_positions = if loaded {
                    summary.as_ref()
                        .map(super::VehicleModelInfo::VehicleModelPositions::from_clump)
                        .unwrap_or_default()
                } else {
                    Default::default()
                };
            }
            model.base_mut().rw_object_loaded = loaded;
            model.base_mut().clump_summary = summary;
        }
    }

    pub fn set_simple_alpha(&mut self, id: usize, alpha: u8) {
        if let Some(ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model)) =
            self.get_model_info_mut(id)
        {
            model.alpha = alpha;
        }
    }

    pub fn set_simple_atomic_binding(
        &mut self,
        id: usize,
        lod_index: usize,
        frame_name: &str,
        atomic: RwAtomicDescriptor,
        geometry: RwGeometryDescriptor,
    ) -> bool {
        let Some(ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model)) =
            self.get_model_info_mut(id)
        else {
            return false;
        };
        model
            .atomic_bindings
            .retain(|binding| binding.lod_index != lod_index);
        model.atomic_bindings.push(SimpleAtomicBinding {
            lod_index,
            frame_name: frame_name.to_owned(),
            atomic,
            geometry,
        });
        model
            .atomic_bindings
            .sort_unstable_by_key(|binding| binding.lod_index);
        // Any loaded atomic LOD counts as geometry present (L1-only DFFs still draw).
        model.base.rw_object_loaded = !model.atomic_bindings.is_empty();
        true
    }

    pub fn clear_simple_atomic_bindings(&mut self, id: usize) -> Option<(Option<usize>, usize)> {
        let model = match self.get_model_info_mut(id)? {
            ModelInfoSlot::Simple(model) | ModelInfoSlot::Time(model) => model,
            _ => return None,
        };
        let binding_count = model.atomic_bindings.len();
        let txd_slot = usize::try_from(model.base.txd_slot).ok();
        model.atomic_bindings.clear();
        model.base.rw_object_loaded = false;
        model.base.clump_summary = None;
        Some((txd_slot, binding_count))
    }

    pub fn set_model_name(&mut self, id: usize, name: &str) {
        if let Some(model) = self.get_model_info_mut(id) {
            model.base_mut().set_model_name(name);
        }
    }

    pub fn clear_tex_dictionary(&mut self, id: usize) {
        if let Some(model) = self.get_model_info_mut(id) {
            model.base_mut().txd_slot = -1;
        }
    }

    pub fn set_tex_dictionary(&mut self, id: usize, txd_store: &mut CTxdStore, name: &str) {
        if let Some(model) = self.get_model_info_mut(id) {
            model.base_mut().set_tex_dictionary(txd_store, name);
        }
    }

    pub fn set_object_id(&mut self, id: usize, object_id: i16) {
        if let Some(model) = self.get_model_info_mut(id) {
            model.base_mut().object_id = object_id;
        }
    }

    pub fn setup_big_buildings(&mut self) {
        let related_models = self.find_related_simple_models();
        let largest_lod_distances = self.simple_largest_lod_distances();
        for (id, model) in self.model_info_ptrs.iter_mut().enumerate() {
            let Some(ModelInfoSlot::Simple(simple) | ModelInfoSlot::Time(simple)) = model else {
                continue;
            };
            if simple.lod_distances[0] > LOD_DISTANCE && simple.related_model_id.is_none() {
                simple.is_big_building = true;
                simple.related_model_id = related_models[id];
                simple.lod_distances[2] = if let Some(related_id) = simple.related_model_id {
                    largest_lod_distances
                        .get(related_id)
                        .and_then(|distance| *distance)
                        .unwrap_or(0.0)
                } else if simple.base.name.len() >= 3
                    && simple.base.name[..3].eq_ignore_ascii_case("LOD")
                {
                    100.0
                } else {
                    0.0
                };
            }
        }
    }

    /// re3 `CTimeModelInfo::FindOtherTimeModel` plus the reciprocal
    /// `other->SetOtherTimeModel(id)` performed by `LoadTimeObject`.
    pub fn link_time_models(&mut self) {
        let time_names = self
            .model_info_ptrs
            .iter()
            .enumerate()
            .filter_map(|(id, model)| match model {
                Some(ModelInfoSlot::Time(time)) => Some((time.base.name.to_ascii_lowercase(), id)),
                _ => None,
            })
            .collect::<std::collections::BTreeMap<_, _>>();

        let counterparts = self
            .model_info_ptrs
            .iter()
            .map(|model| {
                let Some(ModelInfoSlot::Time(time)) = model else {
                    return None;
                };
                let mut counterpart = time.base.name.to_ascii_lowercase();
                if let Some(offset) = counterpart.find("_nt") {
                    counterpart.replace_range(offset..offset + 3, "_dy");
                } else if let Some(offset) = counterpart.find("_dy") {
                    counterpart.replace_range(offset..offset + 3, "_nt");
                } else {
                    return None;
                }
                time_names.get(&counterpart).copied()
            })
            .collect::<Vec<_>>();

        for (id, counterpart) in counterparts.into_iter().enumerate() {
            let Some(counterpart) = counterpart.filter(|other| *other != id) else {
                continue;
            };
            if let Some(ModelInfoSlot::Time(time)) = self.model_info_ptrs[id].as_mut() {
                time.other_time_model_id = Some(counterpart);
            }
        }
    }

    fn find_related_simple_models(&self) -> Vec<Option<usize>> {
        self.model_info_ptrs
            .iter()
            .enumerate()
            .map(|(id, model)| {
                let Some(ModelInfoSlot::Simple(simple) | ModelInfoSlot::Time(simple)) = model
                else {
                    return None;
                };
                let related_suffix = simple.base.name.get(3..)?;
                self.model_info_ptrs
                    .iter()
                    .enumerate()
                    .find_map(|(other_id, other_model)| {
                        if other_id == id {
                            return None;
                        }
                        let other = match other_model {
                            Some(ModelInfoSlot::Simple(other) | ModelInfoSlot::Time(other)) => {
                                other
                            }
                            _ => return None,
                        };
                        other
                            .base
                            .name
                            .get(3..)
                            .is_some_and(|suffix| suffix == related_suffix)
                            .then_some(other_id)
                    })
            })
            .collect()
    }

    fn simple_largest_lod_distances(&self) -> Vec<Option<f32>> {
        self.model_info_ptrs
            .iter()
            .map(|model| match model {
                Some(ModelInfoSlot::Simple(simple) | ModelInfoSlot::Time(simple)) => {
                    Some(simple.largest_lod_distance(1.0, false))
                }
                _ => None,
            })
            .collect()
    }

    fn add_default_simple_model(
        &mut self,
        id: usize,
        col_model: DefaultColModelKind,
        txd_store: &mut CTxdStore,
    ) {
        let model = self.add_simple_model(id);
        model.base.default_col_model = Some(col_model);
        model.base.set_tex_dictionary(txd_store, "generic");
        model.num_atomics = 1;
        model.lod_distances[0] = 80.0;
    }

    fn clump_like_mut(&mut self, id: usize) -> &mut CClumpModelInfo {
        match self.model_info_ptrs[id].as_mut().unwrap() {
            ModelInfoSlot::Mlo(model)
            | ModelInfoSlot::Clump(model)
            | ModelInfoSlot::XtraComps(model) => model,
            _ => unreachable!(),
        }
    }
}

impl SimpleModelFlags {
    pub fn from_re3_flags(flags: u32) -> Self {
        Self {
            normal_cull: flags & 1 != 0,
            no_fade: flags & 2 != 0,
            draw_last: flags & (4 | 8) != 0,
            additive: flags & 8 != 0,
            is_subway: flags & 0x10 != 0,
            ignore_light: flags & 0x20 != 0,
            no_zwrite: flags & 0x40 != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use re3bevy_formats::formats::ClumpRead::{RwAtomicDescriptor, RwGeometryDescriptor};

    fn add_empty_atomic(model_info: &mut CModelInfo, model_id: usize, lod_index: usize) {
        assert!(model_info.set_simple_atomic_binding(
            model_id,
            lod_index,
            "test",
            RwAtomicDescriptor {
                frame_index: 0,
                geometry_index: 0,
                flags: 0,
                frame_name: None,
            },
            RwGeometryDescriptor {
                flags: 0,
                raw_flags: 0,
                num_triangles: 0,
                num_vertices: 0,
                num_morph_targets: 1,
                num_materials: 0,
                num_tex_coord_sets: 0,
                chunk_version: 0,
                chunk_length: 0,
                decoded: None,
                materials: Vec::new(),
                skin: None,
            },
        ));
    }

    #[test]
    fn simple_model_rw_object_is_atomic_zero_only() {
        let mut model_info = CModelInfo::initialise_empty();
        let model_id = 5;
        let simple = model_info.add_simple_model(model_id);
        simple.num_atomics = 2;

        add_empty_atomic(&mut model_info, model_id, 1);
        assert!(
            model_info
                .simple_model_info(model_id)
                .unwrap()
                .base
                .rw_object_loaded
        );
        assert!(!model_info.rw_object_loaded(model_id));

        add_empty_atomic(&mut model_info, model_id, 0);
        assert!(model_info.rw_object_loaded(model_id));
    }

    #[test]
    fn setup_big_building_uses_related_normal_lod_limit() {
        let mut model_info = CModelInfo::initialise_empty();
        let lod_id = 10;
        let detail_id = 11;

        {
            let lod = model_info.add_simple_model(lod_id);
            lod.base.set_model_name("LODfoo");
            lod.lod_distances[0] = 400.0;
            lod.num_atomics = 1;
        }
        {
            let detail = model_info.add_simple_model(detail_id);
            detail.base.set_model_name("comfoo");
            detail.lod_distances = [80.0, 250.0, 900.0];
            detail.num_atomics = 3;
            detail.first_damaged = 2;
        }

        model_info.setup_big_buildings();

        let lod = model_info
            .simple_model_info(lod_id)
            .expect("LOD model should exist");
        assert_eq!(lod.related_model_id, Some(detail_id));
        assert_eq!(lod.lod_distances[2], 250.0);
    }

    #[test]
    fn related_model_suffix_match_is_case_sensitive_like_re3_faststrcmp() {
        let mut model_info = CModelInfo::initialise_empty();
        let lod_id = 20;
        let detail_id = 21;

        {
            let lod = model_info.add_simple_model(lod_id);
            lod.base.set_model_name("LODfoo");
            lod.lod_distances[0] = 400.0;
            lod.num_atomics = 1;
        }
        {
            let detail = model_info.add_simple_model(detail_id);
            detail.base.set_model_name("comFOO");
            detail.lod_distances[0] = 120.0;
            detail.num_atomics = 1;
        }

        model_info.setup_big_buildings();

        let lod = model_info
            .simple_model_info(lod_id)
            .expect("LOD model should exist");
        assert_eq!(lod.related_model_id, None);
        assert_eq!(lod.lod_distances[2], 100.0);
    }
}
