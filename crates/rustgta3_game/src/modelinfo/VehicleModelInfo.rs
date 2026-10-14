//! re3 reference: references/re-GTA/src/modelinfo/VehicleModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/VehicleModelInfo.h
//!
//! `CVehicleModelInfo::LoadVehicleColours` data path.

use std::io;

use bevy::prelude::Resource;
use rustgta3_assets::install::FileMgr::CFileMgr;
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};
use rustgta3_formats::formats::ClumpRead::{RwClumpSummary, RwFrameDescriptor};

use crate::modelinfo::ModelInfo::{CModelInfo, ModelInfoSlot, NUM_VEHICLE_COLOURS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eVehicleType {
    VEHICLE_TYPE_CAR = 0,
    VEHICLE_TYPE_BOAT = 1,
    VEHICLE_TYPE_TRAIN = 2,
    VEHICLE_TYPE_HELI = 3,
    VEHICLE_TYPE_PLANE = 4,
    VEHICLE_TYPE_BIKE = 5,
}

pub(crate) fn script_vehicle_base_z(model_info: Option<&ModelInfoSlot>) -> f32 {
    model_info
        .and_then(|slot| slot.base().col_model.as_ref())
        .map(|col| -col.bounding_box.min.z)
        .unwrap_or(0.0)
}

/// Port of `CVehicleModelInfo::ChooseComponent`/`ChooseSecondComponent`.
/// `PreprocessHierarchy` detaches `extra1..extra6` from the shared clump and
/// `CVehicle::SetModelIndex` records only the two selected component indices on
/// the instance.
pub(crate) fn choose_vehicle_optional_components(
    model_info: Option<&ModelInfoSlot>,
    general: &mut CGeneral,
    rainy: bool,
) -> [i8; 2] {
    let Some(ModelInfoSlot::Vehicle(model)) = model_info else {
        return [-1; 2];
    };
    choose_vehicle_optional_components_from_clump(
        model.clump.base.clump_summary.as_ref(),
        model.component_rules,
        general,
        rainy,
    )
}

pub(crate) fn choose_vehicle_optional_components_from_clump(
    summary: Option<&RwClumpSummary>,
    rules: u32,
    general: &mut CGeneral,
    rainy: bool,
) -> [i8; 2] {
    let Some(summary) = summary else {
        return [-1; 2];
    };
    let num_components = (0..6)
        .filter(|index| {
            let expected = format!("extra{}", index + 1);
            summary.frames.iter().any(|frame| {
                frame
                    .name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&expected))
            })
        })
        .count();
    if num_components == 0 {
        return [-1; 2];
    }

    let first_rule = ((rules >> 12) & 0xF) as i32;
    let second_rule = ((rules >> 28) & 0xF) as i32;
    let first_comps = rules & 0xFFF;
    let second_comps = (rules >> 16) & 0xFFF;
    let valid_rule = |rule: i32| rule != 2 || rainy;

    let rule_component = |rule: i32, comps: u32, general: &mut CGeneral| -> i8 {
        if rule == 0 || !valid_rule(rule) {
            return -1;
        }
        let mut remaining = comps;
        let mut count = 0_i32;
        while remaining != 0 {
            if remaining & 0xF != 0xF {
                count += 1;
            }
            remaining >>= 4;
        }
        if count == 0 {
            return -1;
        }
        let selected = general.get_random_number_in_range_i32(0, count) as u32;
        ((comps >> (selected * 4)) & 0xF) as i8
    };

    let components_not_used_by_rules = || {
        let mut used = [false; 6];
        for (rule, comps) in [(first_rule, first_comps), (second_rule, second_comps)] {
            if rule != 0 && valid_rule(rule) {
                for slot in 0..3 {
                    let component = ((comps >> (slot * 4)) & 0xF) as usize;
                    if component != 0xF && component < used.len() {
                        used[component] = true;
                    }
                }
            }
        }
        (0..num_components)
            .filter(|component| !used[*component])
            .map(|component| component as i8)
            .collect::<Vec<_>>()
    };

    let mut selected = [-1_i8; 2];
    selected[0] = rule_component(first_rule, first_comps, general);
    if selected[0] < 0 && general.get_random_number_in_range_i32(0, 3) < 2 {
        let available = components_not_used_by_rules();
        if !available.is_empty() {
            let index = general.get_random_number_in_range_i32(0, available.len() as i32) as usize;
            selected[0] = available[index];
        }
    }
    selected[1] = rule_component(second_rule, second_comps, general);
    if selected[1] < 0
        && valid_rule(first_rule)
        && first_rule != 0
        && general.get_random_number_in_range_i32(0, 3) < 2
    {
        let available = components_not_used_by_rules();
        if !available.is_empty() {
            let index = general.get_random_number_in_range_i32(0, available.len() as i32) as usize;
            selected[1] = available[index];
        }
    }
    selected
}

#[derive(Debug, Clone, Copy)]
struct VehicleFrameMatrix {
    right: CVector,
    forward: CVector,
    up: CVector,
    position: CVector,
}

impl VehicleFrameMatrix {
    fn from_rw_frame(frame: &RwFrameDescriptor) -> Self {
        Self {
            right: CVector::new(frame.right[0], frame.right[1], frame.right[2]),
            forward: CVector::new(frame.up[0], frame.up[1], frame.up[2]),
            up: CVector::new(frame.at[0], frame.at[1], frame.at[2]),
            position: CVector::new(frame.position[0], frame.position[1], frame.position[2]),
        }
    }

    fn multiply3x3(self, vector: CVector) -> CVector {
        CVector::new(
            self.right.x * vector.x + self.forward.x * vector.y + self.up.x * vector.z,
            self.right.y * vector.x + self.forward.y * vector.y + self.up.y * vector.z,
            self.right.z * vector.x + self.forward.z * vector.y + self.up.z * vector.z,
        )
    }

    fn transform_point(self, vector: CVector) -> CVector {
        let transformed = self.multiply3x3(vector);
        CVector::new(
            transformed.x + self.position.x,
            transformed.y + self.position.y,
            transformed.z + self.position.z,
        )
    }

    fn compose(self, child: Self) -> Self {
        Self {
            right: self.multiply3x3(child.right),
            forward: self.multiply3x3(child.forward),
            up: self.multiply3x3(child.up),
            position: self.transform_point(child.position),
        }
    }
}

/// Model-space position recorded by re3
/// `CVehicleModelInfo::PreprocessHierarchy` for a `VEHICLE_FLAG_POS` dummy.
/// The descriptor translation is parent-relative; re3 walks the complete
/// frame chain before destroying the dummy and storing `m_positions[]`.
pub fn vehicle_dummy_model_position(summary: &RwClumpSummary, name: &str) -> Option<CVector> {
    fn compose_frame(
        summary: &RwClumpSummary,
        index: usize,
        remaining: usize,
    ) -> Option<VehicleFrameMatrix> {
        if remaining == 0 {
            return None;
        }
        let frame = summary.frames.get(index)?;
        let local = VehicleFrameMatrix::from_rw_frame(frame);
        if frame.parent_index < 0 {
            return Some(local);
        }
        let parent = usize::try_from(frame.parent_index).ok()?;
        compose_frame(summary, parent, remaining - 1).map(|matrix| matrix.compose(local))
    }

    let (index, _) = summary.frames.iter().enumerate().find(|(_, frame)| {
        frame
            .name
            .as_deref()
            .is_some_and(|frame_name| frame_name.eq_ignore_ascii_case(name))
    })?;
    compose_frame(summary, index, summary.frames.len().saturating_add(1))
        .map(|matrix| matrix.position)
}

/// An omitted authored dummy is distinct from an invalid required parent chain.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AuthoredVehiclePosition {
    #[default]
    Absent,
    Position(CVector),
    Invalid,
}

impl AuthoredVehiclePosition {
    pub fn position(self) -> Option<CVector> {
        match self { Self::Position(position) => Some(position), Self::Absent | Self::Invalid => None }
    }
}

/// `CVehicleModelInfo::GetWheelPosn` uses these four frame IDs in LF/LB/RF/RB
/// order. Middle wheels are distinct nodes, never rear-wheel substitutes.
const PRIMARY_WHEEL_FRAME_NAMES: [&str; 4] = [
    "wheel_lf_dummy", "wheel_lb_dummy", "wheel_rf_dummy", "wheel_rb_dummy",
];

/// Resident local frame translations, as read by GetWheelPosn/PreRender.
/// These differ from the fully parent-transformed VEHICLE_FLAG_POS dummies.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VehicleWheelFrames {
    pub primary: [AuthoredVehiclePosition; PRIMARY_WHEEL_FRAME_NAMES.len()],
    pub right_middle: AuthoredVehiclePosition,
}

impl VehicleWheelFrames {
    pub fn from_clump(summary: &RwClumpSummary) -> Self {
        let resolve = |name: &str| {
            let Some(frame) = summary.frames.iter().find(|frame|
                frame.name.as_deref().is_some_and(|frame_name| frame_name.eq_ignore_ascii_case(name)))
            else { return AuthoredVehiclePosition::Absent; };
            let finite = frame.position.iter().chain(&frame.right).chain(&frame.up)
                .chain(&frame.at).all(|value| value.is_finite());
            let valid_chain = vehicle_dummy_model_position(summary, name)
                .is_some_and(|position| position.x.is_finite() && position.y.is_finite() && position.z.is_finite());
            if !finite || !valid_chain { return AuthoredVehiclePosition::Invalid; }
            AuthoredVehiclePosition::Position(CVector::new(frame.position[0], frame.position[1], frame.position[2]))
        };
        Self {
            primary: PRIMARY_WHEEL_FRAME_NAMES.map(resolve),
            right_middle: resolve("wheel_rm_dummy"),
        }
    }

    pub fn local_positions(&self) -> Option<[CVector; 4]> {
        let [lf, lb, rf, rb] = self.primary;
        Some([lf.position()?, lb.position()?, rf.position()?, rb.position()?])
    }
}

/// Authored position descriptors resolved with the resident vehicle clump.
/// `CVehicleModelInfo::PreprocessHierarchy` stores these model-space positions;
/// `CAutomobile::PreRender` only transforms them by the actor's current matrix.
#[derive(Debug, Clone, Copy, Default)]
pub struct VehicleModelPositions {
    pub headlights: AuthoredVehiclePosition,
    pub taillights: Option<CVector>,
    pub exhaust: AuthoredVehiclePosition,
    pub wheels: VehicleWheelFrames,
    pub damage_nodes: VehicleDamageNodes,
}

/// Original retained bonnet/boot nodes used by SetupModelNodes/VehicleDamage.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VehicleDamageNodes {
    pub bonnet: AuthoredVehiclePosition,
    pub boot: AuthoredVehiclePosition,
}

pub fn resident_vehicle_damage_nodes(
    loader: &crate::core::FileLoader::CFileLoader,
    model_index: i16,
) -> Option<&VehicleDamageNodes> {
    let Some(ModelInfoSlot::Vehicle(model)) = usize::try_from(model_index).ok()
        .and_then(|id| loader.model_info.get_model_info(id)) else {
        bevy::log::warn_once!("automobile_damage_nodes_required_model_missing");
        return None;
    };
    let Some(descriptors) = model.authored_positions() else {
        bevy::log::warn_once!("automobile_damage_nodes_waiting_resident_clump");
        return None;
    };
    Some(&descriptors.damage_nodes)
}

pub use VehicleModelPositions as VehicleLampPositions;

impl VehicleModelPositions {
    pub(super) fn from_clump(summary: &RwClumpSummary) -> Self {
        let resolve = |name| {
            vehicle_dummy_model_position(summary, name).filter(|position| {
                position.x.is_finite() && position.y.is_finite() && position.z.is_finite()
            })
        };
        Self {
            wheels: VehicleWheelFrames::from_clump(summary),
            damage_nodes: VehicleDamageNodes {
                bonnet: if summary.frames.iter().any(|frame| frame.name.as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case("bonnet_dummy"))) {
                    resolve("bonnet_dummy").map_or(AuthoredVehiclePosition::Invalid, AuthoredVehiclePosition::Position)
                } else { AuthoredVehiclePosition::Absent },
                boot: if summary.frames.iter().any(|frame| frame.name.as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case("boot_dummy"))) {
                    resolve("boot_dummy").map_or(AuthoredVehiclePosition::Invalid, AuthoredVehiclePosition::Position)
                } else { AuthoredVehiclePosition::Absent },
            },
            headlights: if summary.frames.iter().any(|frame| frame.name.as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("headlights"))) {
                resolve("headlights").map_or(AuthoredVehiclePosition::Invalid, AuthoredVehiclePosition::Position)
            } else { AuthoredVehiclePosition::Absent },
            taillights: resolve("taillights"),
            exhaust: if summary.frames.iter().any(|frame| frame.name.as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("exhaust"))) {
                resolve("exhaust").map_or(AuthoredVehiclePosition::Invalid, AuthoredVehiclePosition::Position)
            } else {
                AuthoredVehiclePosition::Absent
            },
        }
    }
}

#[cfg(test)]
mod lamp_residency_tests {
    use super::*;

    #[test]
    fn damage_node_descriptors_follow_original_resident_lifetime_and_provenance() {
        let mut authored = clump();
        authored.frames.push(frame("bonnet_dummy", [1.0, 2.0, 3.0], 0));
        authored.frames.push(frame("boot_dummy", [4.0, 5.0, 6.0], 0));
        let mut models = CModelInfo::initialise_empty();
        models.add_vehicle_model(120);
        models.set_model_loaded(120, true, Some(authored.clone()));
        assert_eq!(lamps(&models, 120).damage_nodes.bonnet.position(), Some(CVector::new(8.0, 21.0, 33.0)));
        authored.frames[3].position[0] = 9.0;
        models.set_model_loaded(120, true, Some(authored.clone()));
        assert_eq!(lamps(&models, 120).damage_nodes.bonnet.position(), Some(CVector::new(8.0, 29.0, 33.0)));
        models.set_model_loaded(120, false, None);
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info(120) else { panic!("vehicle") };
        assert!(model.authored_positions().is_none());
        models.set_model_loaded(120, true, Some(authored.clone()));
        assert!(lamps(&models, 120).damage_nodes.boot.position().is_some());
        authored.frames[3].parent_index = 3;
        assert_eq!(VehicleModelPositions::from_clump(&authored).damage_nodes.bonnet, AuthoredVehiclePosition::Invalid);
        authored.frames[3].parent_index = 0;
        authored.frames[3].position[0] = f32::NAN;
        assert_eq!(VehicleModelPositions::from_clump(&authored).damage_nodes.bonnet, AuthoredVehiclePosition::Invalid);
        authored.frames.pop();
        assert_eq!(VehicleModelPositions::from_clump(&authored).damage_nodes.boot, AuthoredVehiclePosition::Absent);
    }

    #[test]
    fn wheel_frames_keep_native_local_ids_and_resident_lifetime() {
        let mut authored = clump();
        for name in PRIMARY_WHEEL_FRAME_NAMES {
            authored.frames.push(frame(name, [1.0, 2.0, 3.0], 0));
        }
        authored.frames.push(frame("wheel_rm_dummy", [4.0, 5.0, 6.0], 0));
        let mut models = CModelInfo::initialise_empty();
        models.add_vehicle_model(120);
        models.set_model_loaded(120, true, Some(authored.clone()));
        let wheels = lamps(&models, 120).wheels;
        assert_eq!(wheels.local_positions(), Some([CVector::new(1.0, 2.0, 3.0); 4]),
            "GetWheelPosn reads local translations, not transformed dummy positions");
        assert_eq!(wheels.right_middle.position(), Some(CVector::new(4.0, 5.0, 6.0)));
        authored.frames[4].position[0] = 7.0;
        models.set_model_loaded(120, true, Some(authored.clone()));
        assert_eq!(lamps(&models, 120).wheels.primary[1].position().unwrap().x, 7.0);
        models.set_model_loaded(120, false, None);
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info(120) else { panic!("vehicle") };
        assert!(model.authored_positions().is_none());
        models.set_model_loaded(120, true, Some(authored.clone()));
        authored.frames[4].name = Some("wheel_lm_dummy".to_owned());
        assert_eq!(VehicleWheelFrames::from_clump(&authored).primary[1], AuthoredVehiclePosition::Absent,
            "middle-left cannot replace missing rear-left");
        authored.frames[4].name = Some("wheel_lb_dummy".to_owned());
        authored.frames[4].parent_index = 4;
        assert_eq!(VehicleWheelFrames::from_clump(&authored).primary[1], AuthoredVehiclePosition::Invalid);
        authored.frames[4].parent_index = 0;
        authored.frames[4].position[0] = f32::NAN;
        assert_eq!(VehicleWheelFrames::from_clump(&authored).primary[1], AuthoredVehiclePosition::Invalid);
    }

    fn frame(name: &str, position: [f32; 3], parent_index: i32) -> RwFrameDescriptor {
        RwFrameDescriptor {
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            at: [0.0, 0.0, 1.0],
            position,
            parent_index,
            matrix_flags: 0,
            name: Some(name.to_owned()),
            hanim_id: None,
        }
    }

    fn clump() -> RwClumpSummary {
        let mut root = frame("chassis", [10.0, 20.0, 30.0], -1);
        root.right = [0.0, 1.0, 0.0];
        root.up = [-1.0, 0.0, 0.0];
        RwClumpSummary {
            num_atomics: 0,
            num_lights: 0,
            num_cameras: 0,
            num_frames: 3,
            num_geometries: 0,
            first_phase_geometries: 0,
            atomic_chunks: 0,
            root_frame_name: Some("chassis".to_owned()),
            frame_names: Vec::new(),
            frames: vec![
                root,
                frame("headlights", [2.0, 3.0, 4.0], 0),
                frame("taillights", [-2.0, -3.0, 1.0], 0),
            ],
            hanim_nodes: Vec::new(),
            hanim_hierarchy_flags: 0,
            hanim_root_frame_index: None,
            hanim_node_ids: Vec::new(),
            geometries: Vec::new(),
            atomics: Vec::new(),
            bevy_meshes: Vec::new(),
        }
    }

    fn lamps(models: &CModelInfo, model_id: usize) -> &VehicleLampPositions {
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info(model_id) else {
            panic!("vehicle model missing")
        };
        &model.authored_positions
    }

    #[test]
    fn exhaust_descriptor_follows_replacement_unload_and_compatibility_retirement() {
        let mut models = CModelInfo::initialise_empty();
        models.add_vehicle_model(120);
        let mut authored = clump();
        authored.frames.push(frame("exhaust", [2.0, 3.0, 4.0], 0));
        authored.num_frames = authored.frames.len() as i32;
        models.set_model_loaded(120, true, Some(authored.clone()));
        assert_eq!(lamps(&models, 120).exhaust,
            AuthoredVehiclePosition::Position(CVector::new(7.0, 22.0, 34.0)));
        let mut replacement = authored.clone();
        replacement.frames[3].position = [5.0, -2.0, 1.0];
        models.set_model_loaded(120, true, Some(replacement));
        assert_eq!(lamps(&models, 120).exhaust,
            AuthoredVehiclePosition::Position(CVector::new(12.0, 25.0, 31.0)));
        models.set_model_loaded(120, false, None);
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info(120) else { panic!("vehicle model") };
        assert!(model.authored_positions().is_none());
        models.set_model_loaded(120, true, Some(authored));
        assert_eq!(lamps(&models, 120).exhaust,
            AuthoredVehiclePosition::Position(CVector::new(7.0, 22.0, 34.0)));
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info_mut(120) else { panic!("vehicle model") };
        model.clump.DeleteRwObject();
        assert!(model.authored_positions().is_none());
        models.set_model_loaded(120, true, Some(clump()));
        assert_eq!(lamps(&models, 120).exhaust, AuthoredVehiclePosition::Absent);
    }

    #[test]
    fn absent_exhaust_is_distinct_from_invalid_authored_parent_chain_or_numbers() {
        let mut authored = clump();
        assert_eq!(VehicleModelPositions::from_clump(&authored).exhaust, AuthoredVehiclePosition::Absent);
        authored.frames.push(frame("exhaust", [1.0, 2.0, 3.0], 3));
        assert_eq!(VehicleModelPositions::from_clump(&authored).exhaust, AuthoredVehiclePosition::Invalid);
        authored.frames[3].parent_index = 0;
        authored.frames[3].position[0] = f32::NAN;
        assert_eq!(VehicleModelPositions::from_clump(&authored).exhaust, AuthoredVehiclePosition::Invalid);
    }

    #[test]
    fn headlight_provenance_distinguishes_native_absence_from_invalid_data() {
        let mut authored = clump();
        assert!(matches!(VehicleModelPositions::from_clump(&authored).headlights, AuthoredVehiclePosition::Position(_)));
        authored.frames[0].position[0] = f32::NAN;
        assert_eq!(VehicleModelPositions::from_clump(&authored).headlights, AuthoredVehiclePosition::Invalid);
        for frame in &mut authored.frames {
            if frame.name.as_deref() == Some("headlights") { frame.name = Some("unrelated".to_owned()); }
        }
        assert_eq!(VehicleModelPositions::from_clump(&authored).headlights, AuthoredVehiclePosition::Absent);
    }

    #[test]
    fn authored_lamps_follow_rotated_parent_replacement_unload_and_reload() {
        let mut models = CModelInfo::initialise_empty();
        let model_id = 120;
        models.add_vehicle_model(model_id);
        assert!(lamps(&models, model_id).headlights.position().is_none());
        models.set_model_loaded(model_id, true, Some(clump()));
        assert_eq!(
            lamps(&models, model_id).headlights.position(),
            Some(CVector::new(7.0, 22.0, 34.0))
        );
        assert_eq!(
            lamps(&models, model_id).taillights,
            Some(CVector::new(13.0, 18.0, 31.0))
        );
        let mut replacement = clump();
        replacement.frames[0].position[0] = 100.0;
        replacement.frames.pop();
        models.set_model_loaded(model_id, true, Some(replacement));
        assert_eq!(
            lamps(&models, model_id).headlights.position(),
            Some(CVector::new(97.0, 22.0, 34.0))
        );
        assert!(lamps(&models, model_id).taillights.is_none());
        models
            .get_model_info_mut(model_id)
            .unwrap()
            .delete_rw_object();
        assert!(lamps(&models, model_id).headlights.position().is_none());
        assert!(
            models
                .get_model_info(model_id)
                .unwrap()
                .base()
                .clump_summary
                .is_none()
        );
        models.set_model_loaded(model_id, true, Some(clump()));
        assert_eq!(
            lamps(&models, model_id).taillights,
            Some(CVector::new(13.0, 18.0, 31.0))
        );
        models.set_model_loaded(model_id, false, None);
        assert!(lamps(&models, model_id).headlights.position().is_none());
        models.shutdown();
        assert!(models.get_model_info(model_id).is_none());
    }

    #[test]
    fn malformed_authored_lamps_remain_missing_without_fabricated_positions() {
        let mut invalid = clump();
        invalid.frames[0].position[0] = f32::NAN;
        assert!(
            VehicleLampPositions::from_clump(&invalid)
                .headlights.position()
                .is_none()
        );
        invalid = clump();
        invalid.frames[1].parent_index = 99;
        assert!(
            VehicleLampPositions::from_clump(&invalid)
                .headlights.position()
                .is_none()
        );
        assert!(
            VehicleLampPositions::from_clump(&invalid)
                .taillights
                .is_some()
        );
        invalid.frames[0].parent_index = 1;
        invalid.frames[1].parent_index = 0;
        assert!(
            VehicleLampPositions::from_clump(&invalid)
                .headlights.position()
                .is_none()
        );
        invalid = clump();
        invalid.frames[0].right[0] = f32::MAX;
        assert!(
            VehicleLampPositions::from_clump(&invalid)
                .headlights.position()
                .is_none()
        );
    }

    #[test]
    fn compatibility_clump_deletion_cannot_expose_stale_lamp_positions() {
        let mut models = CModelInfo::initialise_empty();
        models.add_vehicle_model(120);
        models.set_model_loaded(120, true, Some(clump()));
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info_mut(120) else {
            panic!("vehicle model missing")
        };
        assert!(model.lamp_positions().is_some());
        model.clump.DeleteRwObject();
        assert!(model.lamp_positions().is_none());
        models.set_model_loaded(120, true, Some(clump()));
        let Some(ModelInfoSlot::Vehicle(model)) = models.get_model_info(120) else {
            panic!("vehicle model missing")
        };
        assert_eq!(model.lamp_positions().unwrap().headlights.position(), Some(CVector::new(7.0, 22.0, 34.0)));
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CVehicleColours {
    pub vehicle_colour_table: Vec<RwRGBA>,
    pub vehicle_variations: Vec<VehicleColourVariation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RwRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleColourVariation {
    pub model_name: String,
    pub colours1: Vec<u8>,
    pub colours2: Vec<u8>,
}

impl CVehicleColours {
    pub const FILENAME: &'static str = "DATA\\CARCOLS.DAT";

    pub fn load(file_mgr: &mut CFileMgr, model_info: &mut CModelInfo) -> io::Result<Self> {
        let bytes = file_mgr.load_file(Self::FILENAME, 128 * 1024, "r")?;
        let text = String::from_utf8_lossy(bytes.strip_suffix(&[0]).unwrap_or(&bytes));
        let colours = parse_carcols(&text);
        colours.apply_to_model_info(model_info);
        Ok(colours)
    }

    pub fn apply_to_model_info(&self, model_info: &mut CModelInfo) {
        for variation in &self.vehicle_variations {
            let Some((id, _)) = model_info.get_model_info_by_name(&variation.model_name) else {
                continue;
            };
            let Some(ModelInfoSlot::Vehicle(vehicle)) = model_info.get_model_info_mut(id) else {
                continue;
            };

            vehicle.num_colours = variation.colours1.len().min(NUM_VEHICLE_COLOURS);
            for i in 0..vehicle.num_colours {
                vehicle.colours1[i] = variation.colours1[i];
                vehicle.colours2[i] = variation.colours2[i];
            }
        }
    }
}

fn parse_carcols(text: &str) -> CVehicleColours {
    let mut section = CarcolsSection::None;
    let mut vehicle_colour_table = Vec::with_capacity(256);
    let mut vehicle_variations = Vec::new();

    for raw in text.lines() {
        let line = normalise_carcols_line(raw);
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("end") {
            section = CarcolsSection::None;
            continue;
        }

        if section == CarcolsSection::None {
            if line.starts_with("col") {
                section = CarcolsSection::Colours;
            } else if line.starts_with("car") {
                section = CarcolsSection::Cars;
            }
            continue;
        }

        match section {
            CarcolsSection::Colours => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    continue;
                }
                let Some(red) = parse_u8(parts[0]) else {
                    continue;
                };
                let Some(green) = parse_u8(parts[1]) else {
                    continue;
                };
                let Some(blue) = parse_u8(parts[2]) else {
                    continue;
                };
                vehicle_colour_table.push(RwRGBA {
                    red,
                    green,
                    blue,
                    alpha: 0xFF,
                });
            }
            CarcolsSection::Cars => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let Some(model_name) = parts.first() else {
                    continue;
                };

                let mut colours = parts
                    .iter()
                    .skip(1)
                    .filter_map(|part| parse_u8(part))
                    .take(NUM_VEHICLE_COLOURS * 2);

                let mut colours1 = Vec::new();
                let mut colours2 = Vec::new();
                while let Some(colour1) = colours.next() {
                    let Some(colour2) = colours.next() else {
                        break;
                    };
                    colours1.push(colour1);
                    colours2.push(colour2);
                }

                vehicle_variations.push(VehicleColourVariation {
                    model_name: (*model_name).to_owned(),
                    colours1,
                    colours2,
                });
            }
            CarcolsSection::None => {}
        }
    }

    CVehicleColours {
        vehicle_colour_table,
        vehicle_variations,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarcolsSection {
    None,
    Colours,
    Cars,
}

fn normalise_carcols_line(raw: &str) -> String {
    raw.chars()
        .map(|ch| if ch == ',' || ch == '\r' { ' ' } else { ch })
        .collect()
}

fn parse_u8(text: &str) -> Option<u8> {
    let value = text.parse::<u16>().ok()?;
    u8::try_from(value).ok()
}
