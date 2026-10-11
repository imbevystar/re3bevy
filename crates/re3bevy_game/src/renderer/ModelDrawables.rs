//! DFF drawable construction, vehicle frame transforms, and IFP/skin pose conversion.

use super::{
    GtaWorldTextureCache, basis_rotation, entity_matrix_to_bevy_transform, material_base_color,
    rgba_u8_to_f32, texture_handle_for_model_material,
};
use crate::{
    animation::{AnimManager::CAnimManager, RpAnimBlend::animation_frame_names_match},
    core::{FileLoader::CFileLoader, Streaming::ModelInfoType},
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_formats::formats::ClumpRead::RwClumpSummary;

/// Colour remaps for vehicle primary/secondary materials (re3 SetVehicleColour).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VehicleColourRemap {
    pub colour1: Option<[u8; 4]>,
    pub colour2: Option<[u8; 4]>,
    /// re3 colour index into `ms_vehicleColourTable` / `ms_colourTextureTable`.
    pub colour1_index: Option<u8>,
    pub colour2_index: Option<u8>,
    /// Live `CAutomobile::Render` wheel-frame state. Kept with the per-instance
    /// render remap so static world models never acquire fake wheel motion.
    pub wheel_state: Option<VehicleWheelRenderState>,
    /// Per-component `CDamageManager` status for a live automobile. World
    /// entities intentionally leave this absent: they do not own a CAutomobile
    /// damage manager.
    pub damage_state: Option<VehicleDamageRenderState>,
    /// Live re3 `CDoor::{m_fAngle,m_nAxis}` values for bonnet, boot and doors.
    pub door_state: Option<VehicleDoorRenderState>,
    /// `CVehicle::m_aExtras`: the two optional `extra1..extra6` component
    /// indices cloned by `CVehicleModelInfo::CreateInstance` for this vehicle.
    pub optional_components: Option<[i8; 2]>,
    /// `CHeli::Render` replaces top/back rotor frame bases every visible frame.
    pub heli_rotor_rotation: Option<f32>,
}

/// State which can change the resident vehicle atomics or their materials.
/// re3 keeps this separate from the per-frame matrices written by
/// `CAutomobile::Render`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleResidentMeshState {
    pub colour1: Option<[u8; 4]>,
    pub colour2: Option<[u8; 4]>,
    pub colour1_index: Option<u8>,
    pub colour2_index: Option<u8>,
    pub damage: Option<VehicleDamageRenderState>,
    pub wheel_status: Option<[u8; crate::vehicles::Automobile::NUM_CAR_WHEELS]>,
    pub optional_components: Option<[i8; 2]>,
}

impl VehicleColourRemap {
    /// Attach the live component state consumed by re3
    /// `CAutomobile::Render` while preserving the instance's CARCOLS remap.
    ///
    /// Production world rendering and the isolated developer arena both use
    /// this adapter so wheel, door and damage geometry cannot drift into two
    /// different interpretations of the same `CAutomobile`.
    pub fn with_automobile_render_state<AutoCore, AutoVitality, AutoState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
        mut self,
        car: &crate::vehicles::Automobile::AutomobileRuntimeView<AutoCore, AutoVitality, AutoState, Drive, Appearance, Entry, Audio, AutoAudio>,
    ) -> Self
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
        self.wheel_state = Some(VehicleWheelRenderState {
            rotation: car.wheel_rotation,
            position: car.wheel_position,
            suspension_initialized: car.suspension_initialized,
            steer_angle: car.vehicle.drive.steer_angle,
            status: car.damage.wheel_status,
        });
        self.damage_state = Some(VehicleDamageRenderState::from_damage_manager(&car.damage));
        self.door_state = Some(VehicleDoorRenderState::from_doors(&car.doors));
        self.optional_components = Some(car.vehicle.appearance.extras);
        self
    }

    pub fn resident_mesh_state(self) -> VehicleResidentMeshState {
        VehicleResidentMeshState {
            colour1: self.colour1,
            colour2: self.colour2,
            colour1_index: self.colour1_index,
            colour2_index: self.colour2_index,
            damage: self.damage_state,
            wheel_status: self.wheel_state.map(|wheel| wheel.status),
            optional_components: self.optional_components,
        }
    }

    pub fn requires_full_dynamic_refresh(self) -> bool {
        self.damage_state.is_some_and(|damage| {
            damage.door_status.iter().any(|status| *status != 0)
                || damage.panel_status.iter().any(|status| *status != 0)
        }) || self.wheel_state.is_some_and(|wheel| {
            wheel.status.iter().any(|status| {
                *status == crate::vehicles::DamageManager::eWheelStatus::WHEEL_STATUS_MISSING as u8
            })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleDoorRenderState {
    pub angles: [f32; crate::vehicles::Automobile::NUM_CAR_DOORS],
    pub axes: [u8; crate::vehicles::Automobile::NUM_CAR_DOORS],
}

impl VehicleDoorRenderState {
    fn from_doors(
        doors: &[crate::vehicles::Automobile::CDoor; crate::vehicles::Automobile::NUM_CAR_DOORS],
    ) -> Self {
        Self {
            angles: doors.map(|door| door.angle),
            axes: doors.map(|door| door.axis),
        }
    }
}

/// Exact component status surface consumed by
/// `CAutomobile::{SetDoorDamage,SetPanelDamage,SetBumperDamage}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleDamageRenderState {
    pub door_status: [u8; crate::vehicles::DamageManager::NUM_DAMAGE_DOORS],
    pub panel_status: [u8; 7],
}

impl VehicleDamageRenderState {
    fn from_damage_manager(damage: &crate::vehicles::DamageManager::CDamageManager) -> Self {
        let mut panel_status = [0; 7];
        for (index, status) in panel_status.iter_mut().enumerate() {
            *status = damage.get_panel_status(index as u8) as u8;
        }
        Self {
            door_status: damage.door_status,
            panel_status,
        }
    }
}

/// `CAutomobile::Render` visual wheel state (GTA radians / local Z suspension).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleWheelRenderState {
    pub rotation: [f32; crate::vehicles::Automobile::NUM_CAR_WHEELS],
    pub position: [f32; crate::vehicles::Automobile::NUM_CAR_WHEELS],
    pub suspension_initialized: bool,
    pub steer_angle: f32,
    pub status: [u8; crate::vehicles::Automobile::NUM_CAR_WHEELS],
}

/// One textured/coloured submesh from a DFF atomic (per RW material).
#[derive(Debug, Clone)]
pub struct ModelMaterialDrawable {
    pub mesh: Mesh,
    pub base_color: Color,
    pub texture_name: Option<String>,
    pub texture_model_index: usize,
    /// TXD captured with a cutscene object's cloned clump. Special model IDs
    /// are reused after `CreateInstance`, so consulting the live model slot can
    /// bind COLT2 textures onto an already-created COLT1 object.
    pub texture_txd_slot: Option<usize>,
    pub vertex_count: usize,
    pub index_count: usize,
    pub vehicle_colour_slot: Option<u8>,
    /// re3 `RpMaterialGetSurfaceProperties` specular — envmap coefficient when > 0.
    pub specular: f32,
    /// RenderWare `RpMaterialGetSurfaceProperties` ambient/diffuse factors.
    pub surface_ambient: f32,
    pub surface_diffuse: f32,
    /// Whether the source geometry carries `rpGEOMETRYPRELIT` colours.
    pub has_prelit: bool,
    /// Wheel/door/rotor atomic whose local frame can change while the vehicle
    /// stays resident. Static body atomics never need a 30 Hz mesh rewrite.
    pub dynamic_vehicle_component: bool,
    /// Current vehicle-local RW frame baked into `mesh`. This lets resident
    /// vehicle atomics follow later wheel/door/rotor frames with a child
    /// transform instead of rebuilding vertex/index buffers.
    pub vehicle_component_transform: Option<Transform>,
}

/// Public bind snapshot for diagnostics (real install path, not parity lab).
#[derive(Debug, Clone)]
pub struct MeshBindReport {
    pub model_index: usize,
    pub submesh_count: usize,
    pub total_vertices: usize,
    pub total_indices: usize,
    pub textured_submeshes: usize,
    pub materials_with_names: usize,
    pub skip_reason: Option<&'static str>,
}

/// RenderWare frame matrix in GTA coordinates.
///
/// re3/libRW stores frame basis as right/up/at; `CMatrix::Update` maps those
/// exact fields to GTA right/forward/up respectively. Geometry vertices are transformed by
/// the atomic frame before the entity/world transform, matching
/// `RpAtomicSetFrame` + clump render instead of collapsing every atomic at the
/// clump origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct GtaFrameMatrix {
    pub(super) right: CVector,
    pub(super) forward: CVector,
    pub(super) up: CVector,
    pub(super) position: CVector,
}

impl GtaFrameMatrix {
    pub(super) const IDENTITY: Self = Self {
        right: CVector::new(1.0, 0.0, 0.0),
        forward: CVector::new(0.0, 1.0, 0.0),
        up: CVector::new(0.0, 0.0, 1.0),
        position: CVector::ZERO,
    };

    pub(super) fn from_rw_frame(
        frame: &re3bevy_formats::formats::ClumpRead::RwFrameDescriptor,
    ) -> Self {
        Self {
            right: CVector::new(frame.right[0], frame.right[1], frame.right[2]),
            forward: CVector::new(frame.up[0], frame.up[1], frame.up[2]),
            up: CVector::new(frame.at[0], frame.at[1], frame.at[2]),
            position: CVector::new(frame.position[0], frame.position[1], frame.position[2]),
        }
    }

    pub(super) fn multiply3x3(self, vector: CVector) -> CVector {
        CVector::new(
            self.right.x * vector.x + self.forward.x * vector.y + self.up.x * vector.z,
            self.right.y * vector.x + self.forward.y * vector.y + self.up.y * vector.z,
            self.right.z * vector.x + self.forward.z * vector.y + self.up.z * vector.z,
        )
    }

    pub(super) fn transform_point(self, vector: CVector) -> CVector {
        let v = self.multiply3x3(vector);
        CVector::new(
            v.x + self.position.x,
            v.y + self.position.y,
            v.z + self.position.z,
        )
    }

    pub(super) fn compose(self, child: Self) -> Self {
        Self {
            right: self.multiply3x3(child.right),
            forward: self.multiply3x3(child.forward),
            up: self.multiply3x3(child.up),
            position: self.transform_point(child.position),
        }
    }

    pub(super) fn with_uniform_basis_scale(self, scale: f32) -> Self {
        Self {
            right: CVector::new(
                self.right.x * scale,
                self.right.y * scale,
                self.right.z * scale,
            ),
            forward: CVector::new(
                self.forward.x * scale,
                self.forward.y * scale,
                self.forward.z * scale,
            ),
            up: CVector::new(self.up.x * scale, self.up.y * scale, self.up.z * scale),
            position: self.position,
        }
    }

    pub(super) fn to_bevy_transform(self) -> Transform {
        Transform {
            translation: self.position.to_bevy(),
            rotation: basis_rotation(
                self.right.to_bevy(),
                self.up.to_bevy(),
                -self.forward.to_bevy(),
            ),
            scale: Vec3::ONE,
        }
    }
}

pub fn build_model_material_drawables(
    model_index: usize,
    distance: f32,
    damaged: bool,
    file_loader: &CFileLoader,
    vehicle_colours: VehicleColourRemap,
) -> Vec<ModelMaterialDrawable> {
    let mut out = Vec::new();
    if let Some(simple) = file_loader.model_info.simple_model_info(model_index)
        && !simple.atomic_bindings.is_empty()
    {
        if let Some(binding) = simple.atomic_binding_from_distance(
            distance,
            file_loader.renderer.lod_distance_multiplier(),
            damaged,
        ) {
            append_geometry_drawables(
                model_index,
                &binding.geometry,
                vehicle_colours,
                None,
                &mut out,
            );
            return out;
        }
        return out;
    }
    let Some(model) = file_loader.model_info.get_model_info(model_index) else {
        return out;
    };
    let Some(summary) = model.base().clump_summary.as_ref() else {
        return out;
    };
    let mut drew_atomic = false;
    let is_vehicle = model.base().model_type == ModelInfoType::Vehicle;
    let vehicle_type = match model {
        crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) => Some(vehicle.vehicle_type),
        _ => None,
    };
    for atomic in &summary.atomics {
        // re3::CVehicleModelInfo::PreprocessHierarchy replaces every
        // carIds/ADD_WHEEL dummy atomic with the single m_wheelId atomic
        // attached to that frame.  Raw DFF summaries can still contain the
        // authored wheel atomic, so do not draw it here or the separate wheel
        // pass below renders a second (and, after damage, very obvious) tyre.
        if is_vehicle
            && atomic.frame_name.as_deref().is_some_and(|name| {
                vehicle_type.is_some_and(|kind| vehicle_frame_adds_separate_wheel(kind, name))
            })
        {
            continue;
        }
        if !should_render_atomic_for_model(
            is_vehicle,
            atomic.frame_name.as_deref(),
            damaged,
            vehicle_colours.damage_state,
            vehicle_colours.optional_components,
        ) {
            continue;
        }
        let Some(geometry_index) = usize::try_from(atomic.geometry_index).ok() else {
            continue;
        };
        let Some(geometry) = summary.geometries.get(geometry_index) else {
            continue;
        };
        // `CVehicleModelInfo::PreprocessHierarchy` destroys every child frame
        // below a COLLAPSE component and moves its atomics onto the component
        // dummy without baking the discarded child matrix into the geometry.
        // `_ok`/`_dam` frames frequently contain marker transforms which are
        // therefore not part of the rendered LTM in re3.
        let frame =
            vehicle_atomic_render_frame(summary, atomic.frame_index, is_vehicle.then_some(model))
                .map(|frame| {
                    let frame = vehicle_colours
                        .door_state
                        .map(|doors| {
                            automobile_door_frame(atomic.frame_name.as_deref(), frame, doors)
                        })
                        .unwrap_or(frame);
                    vehicle_colours
                        .heli_rotor_rotation
                        .map(|rotation| {
                            heli_rotor_frame(atomic.frame_name.as_deref(), frame, rotation)
                        })
                        .unwrap_or(frame)
                });
        let first_drawable = out.len();
        if let Some(deformed) = deform_skin_geometry(geometry, summary, None, None) {
            let skinned_geometry = deformed;
            append_decoded_geometry_drawables(
                model_index,
                geometry,
                &skinned_geometry,
                vehicle_colours,
                None,
                &mut out,
            );
        } else {
            append_geometry_drawables(model_index, geometry, vehicle_colours, frame, &mut out);
        }
        if is_vehicle && vehicle_atomic_frame_is_dynamic(atomic.frame_name.as_deref()) {
            let component_transform = frame.map(GtaFrameMatrix::to_bevy_transform);
            for drawable in &mut out[first_drawable..] {
                drawable.dynamic_vehicle_component = true;
                drawable.vehicle_component_transform = component_transform;
            }
        }
        drew_atomic = true;
    }
    if is_vehicle {
        append_vehicle_wheel_drawables(
            model_index,
            model,
            summary,
            distance,
            file_loader,
            vehicle_colours,
            &mut out,
        );
    }
    if !drew_atomic {
        for geometry in &summary.geometries {
            append_geometry_drawables(model_index, geometry, vehicle_colours, None, &mut out);
        }
    }
    out
}

/// Return only the mutable RenderWare component frames for a resident vehicle.
/// The indices address the full [`build_model_material_drawables`] result. No
/// vertex/index/material buffers are allocated here; this is the Bevy ECS
/// equivalent of re3 updating `RwFrameGetMatrix(m_aCarNodes[..])` in
/// `CAutomobile::Render`.
pub fn build_vehicle_component_transforms(
    model_index: usize,
    distance: f32,
    file_loader: &CFileLoader,
    vehicle_colours: VehicleColourRemap,
) -> Option<Vec<(usize, Option<Transform>)>> {
    let model = file_loader.model_info.get_model_info(model_index)?;
    if model.base().model_type != ModelInfoType::Vehicle {
        return None;
    }
    let summary = model.base().clump_summary.as_ref()?;
    let mut out = Vec::new();
    let mut part_index = 0usize;

    let vehicle_type = match model {
        crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) => vehicle.vehicle_type,
        _ => return None,
    };

    for atomic in &summary.atomics {
        // Must mirror build_model_material_drawables exactly: skipped raw
        // ADD_WHEEL atomics consume no drawable indices. Counting them here
        // shifted every later dynamic transform onto the wrong submesh.
        if atomic
            .frame_name
            .as_deref()
            .is_some_and(|name| vehicle_frame_adds_separate_wheel(vehicle_type, name))
        {
            continue;
        }
        if !should_render_atomic_for_model(
            true,
            atomic.frame_name.as_deref(),
            false,
            vehicle_colours.damage_state,
            vehicle_colours.optional_components,
        ) {
            continue;
        }
        let geometry_index = usize::try_from(atomic.geometry_index).ok()?;
        let geometry = summary.geometries.get(geometry_index)?;
        let count = geometry_material_drawable_count(geometry);
        if vehicle_atomic_frame_is_dynamic(atomic.frame_name.as_deref()) {
            let frame = vehicle_atomic_render_frame(summary, atomic.frame_index, Some(model)).map(
                |frame| {
                    let frame = vehicle_colours
                        .door_state
                        .map(|doors| {
                            automobile_door_frame(atomic.frame_name.as_deref(), frame, doors)
                        })
                        .unwrap_or(frame);
                    vehicle_colours
                        .heli_rotor_rotation
                        .map(|rotation| {
                            heli_rotor_frame(atomic.frame_name.as_deref(), frame, rotation)
                        })
                        .unwrap_or(frame)
                        .to_bevy_transform()
                },
            );
            out.extend((0..count).map(|offset| (part_index + offset, frame)));
        }
        part_index += count;
    }

    let crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) = model else {
        return Some(out);
    };
    let Ok(wheel_model_id) = usize::try_from(vehicle.wheel_id) else {
        return Some(out);
    };
    let Some(wheel_model) = file_loader.model_info.simple_model_info(wheel_model_id) else {
        return Some(out);
    };
    let Some(wheel_binding) = wheel_model.atomic_binding_from_distance(
        distance,
        file_loader.renderer.lod_distance_multiplier(),
        false,
    ) else {
        return Some(out);
    };
    let wheel_part_count = geometry_material_drawable_count(&wheel_binding.geometry);
    for (frame_index, frame_desc) in summary.frames.iter().enumerate() {
        let Some(name) = frame_desc.name.as_deref() else {
            continue;
        };
        if !vehicle_frame_adds_separate_wheel(vehicle.vehicle_type, name) {
            continue;
        }
        let wheel_index = automobile_wheel_index(name);
        if let (Some(wheel), Some(state)) = (wheel_index, vehicle_colours.wheel_state)
            && state.status[wheel]
                == crate::vehicles::DamageManager::eWheelStatus::WHEEL_STATUS_MISSING as u8
        {
            continue;
        }
        let frame = atomic_frame_world_matrix(summary, frame_index as i32).map(|frame| {
            if let Some(wheel_state) = vehicle_colours.wheel_state {
                automobile_wheel_frame(
                    model_index,
                    name,
                    frame.position,
                    vehicle.wheel_scale,
                    wheel_state,
                )
            } else {
                frame.with_uniform_basis_scale(vehicle.wheel_scale.max(0.001))
            }
            .to_bevy_transform()
        });
        out.extend((0..wheel_part_count).map(|offset| (part_index + offset, frame)));
        part_index += wheel_part_count;
    }
    Some(out)
}

fn geometry_material_drawable_count(
    geometry_desc: &re3bevy_formats::formats::ClumpRead::RwGeometryDescriptor,
) -> usize {
    let Some(geometry) = geometry_desc.decoded.as_ref() else {
        return 0;
    };
    if geometry.positions.is_empty() || geometry.indices.len() < 3 {
        return 0;
    }
    let triangle_count = geometry.indices.len() / 3;
    if triangle_count == 0 {
        return 0;
    }
    if geometry.triangle_materials.is_empty() {
        return 1;
    }
    let max_material = geometry
        .triangle_materials
        .iter()
        .take(triangle_count)
        .copied()
        .max()
        .unwrap_or(0) as usize;
    let mut used = vec![false; geometry_desc.materials.len().max(max_material + 1)];
    for triangle in 0..triangle_count {
        let material = geometry
            .triangle_materials
            .get(triangle)
            .copied()
            .unwrap_or(0) as usize;
        if material >= used.len() {
            used.resize(material + 1, false);
        }
        used[material] = true;
    }
    used.into_iter().filter(|used| *used).count()
}

/// Build a GTA III ped model at the exact pose held by the ported
/// `RpAnimBlend` clump state. This is the same CPU skin/rigid-frame path used
/// by live world players, exposed for isolated developer tooling.
pub fn build_animated_ped_material_drawables(
    model_index: usize,
    file_loader: &CFileLoader,
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
) -> Vec<ModelMaterialDrawable> {
    let poses = player_anim_clump_bone_poses(clump);
    if poses.is_empty() {
        return Vec::new();
    }
    build_cutscene_drawables(
        model_index,
        file_loader,
        &poses,
        true,
        None,
        false,
        None,
        None,
    )
}

/// Static limb meshes plus their owning RenderWare frame indices for GTA III
/// PC's non-skinned ped clumps.
pub fn build_rigid_ped_material_drawables(
    model_index: usize,
    file_loader: &CFileLoader,
) -> Option<(Vec<ModelMaterialDrawable>, Vec<i32>)> {
    if model_uses_cpu_skinning(model_index, file_loader, None) {
        return None;
    }
    let (drawables, frame_indices) =
        build_rigid_cutscene_drawables(model_index, file_loader, false, None, None);
    (!drawables.is_empty() && drawables.len() == frame_indices.len())
        .then_some((drawables, frame_indices))
}

pub fn build_animated_ped_material_drawables_from_association(
    model_index: usize,
    file_loader: &CFileLoader,
    association: &crate::animation::AnimBlendAssociation::CAnimBlendAssociation,
) -> Vec<ModelMaterialDrawable> {
    let poses = ped_bone_poses_from_association(association);
    build_cutscene_drawables(
        model_index,
        file_loader,
        &poses,
        true,
        None,
        false,
        None,
        None,
    )
}

pub fn rigid_ped_part_transforms_from_clump(
    model_index: usize,
    file_loader: &CFileLoader,
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    frame_indices: &[i32],
) -> Option<Vec<Transform>> {
    let poses = player_anim_clump_bone_poses(clump);
    rigid_cutscene_part_transforms(model_index, file_loader, &poses, true, frame_indices, None)
}

pub fn rigid_ped_part_transforms_from_association(
    model_index: usize,
    file_loader: &CFileLoader,
    association: &crate::animation::AnimBlendAssociation::CAnimBlendAssociation,
    frame_indices: &[i32],
) -> Option<Vec<Transform>> {
    let poses = ped_bone_poses_from_association(association);
    rigid_cutscene_part_transforms(model_index, file_loader, &poses, true, frame_indices, None)
}

/// Diagnose mesh/texture bind for a model (production path helper).
pub fn diagnose_model_mesh_bind(
    model_index: usize,
    distance: f32,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
) -> MeshBindReport {
    diagnose_model_mesh_bind_with_colours(
        model_index,
        distance,
        file_loader,
        texture_cache,
        VehicleColourRemap::default(),
    )
}

/// Same as [`diagnose_model_mesh_bind`] with re3 `SetVehicleColour` remaps applied.
pub fn diagnose_model_mesh_bind_with_colours(
    model_index: usize,
    distance: f32,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
    vehicle_colours: VehicleColourRemap,
) -> MeshBindReport {
    let drawables =
        build_model_material_drawables(model_index, distance, false, file_loader, vehicle_colours);
    if drawables.is_empty() {
        let skip = if file_loader.model_info.get_model_info(model_index).is_none() {
            Some("missing_model")
        } else {
            Some("missing_geometry")
        };
        return MeshBindReport {
            model_index,
            submesh_count: 0,
            total_vertices: 0,
            total_indices: 0,
            textured_submeshes: 0,
            materials_with_names: 0,
            skip_reason: skip,
        };
    }
    let mut textured = 0usize;
    let mut named = 0usize;
    let mut verts = 0usize;
    let mut inds = 0usize;
    for d in &drawables {
        verts += d.vertex_count;
        inds += d.index_count;
        if d.texture_name.is_some() {
            named += 1;
        }
        let has_txd = texture_handle_for_model_material(
            d.texture_model_index,
            d.texture_name.as_deref(),
            file_loader,
            texture_cache,
        )
        .is_some();
        // re3 SetVehicleColour: colour-slot materials without non-@ TXD still get
        // ms_colourTextureTable solid paint (counts as textured for diagnostics).
        let has_colour_paint = d.vehicle_colour_slot.is_some()
            && (vehicle_colours.colour1_index.is_some() || vehicle_colours.colour2_index.is_some());
        let has_specular_white = d.specular > 0.0 && texture_cache.white_texture.is_some();
        if has_txd || has_colour_paint || has_specular_white {
            textured += 1;
        }
    }
    MeshBindReport {
        model_index,
        submesh_count: drawables.len(),
        total_vertices: verts,
        total_indices: inds,
        textured_submeshes: textured,
        materials_with_names: named,
        skip_reason: None,
    }
}

fn append_vehicle_wheel_drawables(
    model_index: usize,
    model: &crate::modelinfo::ModelInfo::ModelInfoSlot,
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    distance: f32,
    file_loader: &CFileLoader,
    vehicle_colours: VehicleColourRemap,
    out: &mut Vec<ModelMaterialDrawable>,
) {
    let crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) = model else {
        return;
    };
    let Ok(wheel_model_id) = usize::try_from(vehicle.wheel_id) else {
        return;
    };
    let Some(wheel_model) = file_loader.model_info.simple_model_info(wheel_model_id) else {
        return;
    };
    let Some(wheel_binding) = wheel_model.atomic_binding_from_distance(
        distance,
        file_loader.renderer.lod_distance_multiplier(),
        false,
    ) else {
        return;
    };
    for (frame_index, frame) in summary.frames.iter().enumerate() {
        let Some(name) = frame.name.as_deref() else {
            continue;
        };
        if !vehicle_frame_adds_separate_wheel(vehicle.vehicle_type, name) {
            continue;
        }
        let wheel_index = automobile_wheel_index(name);

        if let (Some(wheel), Some(state)) = (wheel_index, vehicle_colours.wheel_state)
            && state.status[wheel]
                == crate::vehicles::DamageManager::eWheelStatus::WHEEL_STATUS_MISSING as u8
        {
            // re3 SetWheelDamage clears rpATOMICRENDER on a missing wheel.
            continue;
        }
        let frame = atomic_frame_world_matrix(summary, frame_index as i32).map(|frame| {
            if let Some(wheel_state) = vehicle_colours.wheel_state {
                automobile_wheel_frame(
                    model_index,
                    name,
                    frame.position,
                    vehicle.wheel_scale,
                    wheel_state,
                )
            } else {
                frame.with_uniform_basis_scale(vehicle.wheel_scale.max(0.001))
            }
        });
        let first_drawable = out.len();
        // re3 `FindEditableMaterialList` scans the vehicle clump and detached
        // optional components, but not `m_wheelId`.  A wheel material whose
        // authored colour happens to equal a CARCOLS marker must consequently
        // retain its own colour/texture instead of becoming body paint.
        append_geometry_drawables(
            wheel_model_id,
            &wheel_binding.geometry,
            VehicleColourRemap::default(),
            frame,
            out,
        );
        let component_transform = frame.map(GtaFrameMatrix::to_bevy_transform);
        for drawable in &mut out[first_drawable..] {
            drawable.vehicle_colour_slot = None;
            drawable.dynamic_vehicle_component = true;
            drawable.vehicle_component_transform = component_transform;
        }
    }
}

/// `CVehicleModelInfo::PreprocessHierarchy` applies
/// `VEHICLE_FLAG_ADD_WHEEL` only to the six automobile dummy frames in
/// `carIds`. Plane/bike wheel frames already own their authored atomics and
/// must not receive the separate `m_wheelId` model.
fn vehicle_frame_adds_separate_wheel(vehicle_type: i32, frame_name: &str) -> bool {
    vehicle_type == crate::modelinfo::VehicleModelInfo::eVehicleType::VEHICLE_TYPE_CAR as i32
        && automobile_wheel_index(frame_name).is_some()
}

fn contains_ascii_ignore_case(value: &str, needle: &str) -> bool {
    value
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn automobile_door_index(frame_name: &str) -> Option<usize> {
    if contains_ascii_ignore_case(frame_name, "bonnet") {
        Some(0)
    } else if contains_ascii_ignore_case(frame_name, "boot") {
        Some(1)
    } else if contains_ascii_ignore_case(frame_name, "door_lf") {
        Some(2)
    } else if contains_ascii_ignore_case(frame_name, "door_rf") {
        Some(3)
    } else if contains_ascii_ignore_case(frame_name, "door_lr") {
        Some(4)
    } else if contains_ascii_ignore_case(frame_name, "door_rr") {
        Some(5)
    } else {
        None
    }
}

fn vehicle_atomic_frame_is_dynamic(frame_name: Option<&str>) -> bool {
    let Some(name) = frame_name else {
        return false;
    };
    automobile_door_index(name).is_some()
        || contains_ascii_ignore_case(name, "toprotor")
        || contains_ascii_ignore_case(name, "backrotor")
        || contains_ascii_ignore_case(name, "windscreen")
}

/// re3 `CAutomobile::OpenDoor`: `CMatrix::SetRotate` replaces the component
/// frame basis using its current hinge angle, then restores its authored pivot.
fn automobile_door_frame(
    frame_name: Option<&str>,
    original: GtaFrameMatrix,
    doors: VehicleDoorRenderState,
) -> GtaFrameMatrix {
    let Some(index) = frame_name.and_then(automobile_door_index) else {
        return original;
    };
    let Some(&angle) = doors.angles.get(index) else {
        return original;
    };
    let axis = doors.axes.get(index).copied().unwrap_or(0);
    let (x, y, z) = match axis {
        0 => (angle, 0.0, 0.0),
        1 => (0.0, angle, 0.0),
        2 => (0.0, 0.0, angle),
        _ => return original,
    };
    gta_rotation_matrix(x, y, z, original.position, 1.0)
}

fn heli_rotor_frame(
    frame_name: Option<&str>,
    original: GtaFrameMatrix,
    rotation: f32,
) -> GtaFrameMatrix {
    let Some(name) = frame_name else {
        return original;
    };
    if contains_ascii_ignore_case(name, "toprotor") {
        gta_rotation_matrix(0.0, 0.0, rotation, original.position, 1.0)
    } else if contains_ascii_ignore_case(name, "backrotor") {
        gta_rotation_matrix(rotation, 0.0, 0.0, original.position, 1.0)
    } else {
        original
    }
}

fn automobile_wheel_index(frame_name: &str) -> Option<usize> {
    use crate::vehicles::Automobile::{
        CARWHEEL_FRONT_LEFT, CARWHEEL_FRONT_RIGHT, CARWHEEL_REAR_LEFT, CARWHEEL_REAR_RIGHT,
    };
    if frame_name.eq_ignore_ascii_case("wheel_lf_dummy") {
        Some(CARWHEEL_FRONT_LEFT)
    } else if frame_name.eq_ignore_ascii_case("wheel_lm_dummy")
        || frame_name.eq_ignore_ascii_case("wheel_lb_dummy")
    {
        Some(CARWHEEL_REAR_LEFT)
    } else if frame_name.eq_ignore_ascii_case("wheel_rf_dummy") {
        Some(CARWHEEL_FRONT_RIGHT)
    } else if frame_name.eq_ignore_ascii_case("wheel_rm_dummy")
        || frame_name.eq_ignore_ascii_case("wheel_rb_dummy")
    {
        Some(CARWHEEL_REAR_RIGHT)
    } else {
        None
    }
}

/// Exact orientation branches used by `CAutomobile::Render`: right wheels use
/// `SetRotate(rot, 0, steer)`, left wheels negate roll and start at PI, burst
/// tyres add the sine wobble, and Rhino front wheels never steer.
/// `m_aWheelPosition` replaces only local Z.
fn automobile_wheel_frame(
    model_index: usize,
    frame_name: &str,
    original_position: CVector,
    wheel_scale: f32,
    state: VehicleWheelRenderState,
) -> GtaFrameMatrix {
    let Some(index) = automobile_wheel_index(frame_name) else {
        return GtaFrameMatrix::IDENTITY;
    };
    let is_front = matches!(
        index,
        crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT
            | crate::vehicles::Automobile::CARWHEEL_FRONT_RIGHT
    );
    let is_left = matches!(
        index,
        crate::vehicles::Automobile::CARWHEEL_FRONT_LEFT
            | crate::vehicles::Automobile::CARWHEEL_REAR_LEFT
    );
    let roll = if is_left {
        -state.rotation[index]
    } else {
        state.rotation[index]
    };
    let burst = state.status[index]
        == crate::vehicles::DamageManager::eWheelStatus::WHEEL_STATUS_BURST as u8;
    let mut yaw = if is_left { std::f32::consts::PI } else { 0.0 };
    if is_front && model_index != crate::modelinfo::ModelIndices::MI_RHINO {
        yaw += state.steer_angle;
    }
    if burst && model_index != crate::modelinfo::ModelIndices::MI_RHINO {
        // CAutomobile::Render adds the burst-tyre wobble after applying the
        // left-wheel roll sign. Dodo uses the same front-right branch.
        yaw += 0.3 * roll.sin();
    }
    // `CAutomobile::SetupSuspensionLines` initializes these values from the
    // authored wheel dummy before the first Render call.  Until that setup is
    // available on a partially constructed/script vehicle, Rust's zeroed
    // array must not drag every wheel to the vehicle origin.
    let suspension_z = if state.suspension_initialized {
        state.position[index]
    } else {
        original_position.z
    };
    gta_rotation_matrix(
        roll,
        0.0,
        yaw,
        CVector::new(original_position.x, original_position.y, suspension_z),
        wheel_scale.max(0.001),
    )
}

#[cfg(test)]
mod vehicle_wheel_render_tests {
    use super::*;

    #[test]
    fn separate_wheel_model_matches_re3_carids_add_wheel_flags() {
        use crate::modelinfo::VehicleModelInfo::eVehicleType;

        for name in [
            "wheel_rf_dummy",
            "wheel_rm_dummy",
            "wheel_rb_dummy",
            "wheel_lf_dummy",
            "wheel_lm_dummy",
            "wheel_lb_dummy",
        ] {
            assert!(vehicle_frame_adds_separate_wheel(
                eVehicleType::VEHICLE_TYPE_CAR as i32,
                name
            ));
        }
        assert!(!vehicle_frame_adds_separate_wheel(
            eVehicleType::VEHICLE_TYPE_PLANE as i32,
            "wheel_front_dummy"
        ));
        assert!(!vehicle_frame_adds_separate_wheel(
            eVehicleType::VEHICLE_TYPE_PLANE as i32,
            "wheel_lf_dummy"
        ));
        assert!(!vehicle_frame_adds_separate_wheel(
            eVehicleType::VEHICLE_TYPE_BIKE as i32,
            "wheel_front"
        ));
        assert!(!vehicle_frame_adds_separate_wheel(
            eVehicleType::VEHICLE_TYPE_CAR as i32,
            "wheel_front_dummy"
        ));
    }

    #[test]
    fn zeroed_runtime_suspension_keeps_authored_wheel_height() {
        let original = CVector::new(1.2, 2.4, -0.45);
        let frame = automobile_wheel_frame(
            crate::modelinfo::ModelIndices::MI_KURUMA,
            "wheel_lf_dummy",
            original,
            0.8,
            VehicleWheelRenderState {
                rotation: [0.0; 4],
                position: [0.0; 4],
                suspension_initialized: false,
                steer_angle: 0.0,
                status: [0; 4],
            },
        );
        assert_eq!(frame.position, original);
    }

    #[test]
    fn initialized_runtime_suspension_replaces_only_wheel_z() {
        let original = CVector::new(1.2, 2.4, -0.45);
        let frame = automobile_wheel_frame(
            crate::modelinfo::ModelIndices::MI_KURUMA,
            "wheel_lf_dummy",
            original,
            0.8,
            VehicleWheelRenderState {
                rotation: [0.0; 4],
                position: [-0.2, -0.3, -0.4, -0.5],
                suspension_initialized: true,
                steer_angle: 0.0,
                status: [0; 4],
            },
        );
        assert_eq!(frame.position, CVector::new(1.2, 2.4, -0.2));
    }

    #[test]
    fn burst_wheel_and_rhino_front_branch_match_automobile_render() {
        use crate::vehicles::{Automobile::CARWHEEL_FRONT_RIGHT, DamageManager::eWheelStatus};

        let mut state = VehicleWheelRenderState {
            rotation: [0.0; 4],
            position: [0.0; 4],
            suspension_initialized: true,
            steer_angle: 0.35,
            status: [eWheelStatus::WHEEL_STATUS_OK as u8; 4],
        };
        state.rotation[CARWHEEL_FRONT_RIGHT] = 0.8;
        state.status[CARWHEEL_FRONT_RIGHT] = eWheelStatus::WHEEL_STATUS_BURST as u8;
        let normal = automobile_wheel_frame(
            crate::modelinfo::ModelIndices::MI_KURUMA,
            "wheel_rf_dummy",
            CVector::ZERO,
            1.0,
            state,
        );
        let expected_yaw = state.steer_angle + 0.3 * state.rotation[CARWHEEL_FRONT_RIGHT].sin();
        let expected = gta_rotation_matrix(
            state.rotation[CARWHEEL_FRONT_RIGHT],
            0.0,
            expected_yaw,
            CVector::ZERO,
            1.0,
        );
        assert_eq!(normal, expected);

        let rhino = automobile_wheel_frame(
            crate::modelinfo::ModelIndices::MI_RHINO,
            "wheel_rf_dummy",
            CVector::ZERO,
            1.0,
            state,
        );
        let expected_rhino = gta_rotation_matrix(
            state.rotation[CARWHEEL_FRONT_RIGHT],
            0.0,
            0.0,
            CVector::ZERO,
            1.0,
        );
        assert_eq!(rhino, expected_rhino);
    }
}

#[cfg(test)]
mod cutscene_frame_parity_tests {
    use super::*;
    use crate::{
        animation::{
            AnimBlendAssociation::{
                ASSOC_HAS_TRANSLATION, ASSOC_HAS_X_TRANSLATION, ASSOC_PARTIAL,
                CAnimBlendAssociation,
            },
            AnimBlendClumpData::CAnimBlendClumpData,
            AnimBlendHierarchy::CAnimBlendHierarchy,
            AnimBlendSequence::{AnimKeyFrame, CAnimBlendSequence, KeyFrameTrans},
            CutsceneMgr::CutsceneBonePose,
        },
        math::{Quaternion::CQuaternion, Vector::CVector as AnimVector},
    };

    fn cpp_quaternion_matrix(q: CQuaternion) -> GtaFrameMatrix {
        let x2 = q.x + q.x;
        let y2 = q.y + q.y;
        let z2 = q.z + q.z;
        let x_2x = q.x * x2;
        let x_2y = q.x * y2;
        let x_2z = q.x * z2;
        let y_2y = q.y * y2;
        let y_2z = q.y * z2;
        let z_2z = q.z * z2;
        let w_2x = q.w * x2;
        let w_2y = q.w * y2;
        let w_2z = q.w * z2;
        GtaFrameMatrix {
            right: CVector::new(1.0 - (y_2y + z_2z), x_2y + w_2z, x_2z - w_2y),
            forward: CVector::new(x_2y - w_2z, 1.0 - (x_2x + z_2z), y_2z + w_2x),
            up: CVector::new(x_2z + w_2y, y_2z - w_2x, 1.0 - (x_2x + y_2y)),
            position: CVector::ZERO,
        }
    }

    fn assert_vector_close(actual: CVector, expected: CVector) {
        assert!((actual.x - expected.x).abs() < 1.0e-6);
        assert!((actual.y - expected.y).abs() < 1.0e-6);
        assert!((actual.z - expected.z).abs() < 1.0e-6);
    }

    #[test]
    fn cutscene_quaternions_match_cpp_cquaternion_get_for_xyz_rotations() {
        let half = std::f32::consts::FRAC_1_SQRT_2;
        for rotation in [
            CQuaternion::IDENTITY,
            CQuaternion::new(half, 0.0, 0.0, half),
            CQuaternion::new(0.0, half, 0.0, half),
            CQuaternion::new(0.0, 0.0, half, half),
        ] {
            let pose = CutsceneBonePose {
                name: "root".to_owned(),
                position: AnimVector::ZERO,
                rotation,
                has_translation: false,
            };
            let actual = gta_frame_from_cutscene_pose(&pose);
            let expected = cpp_quaternion_matrix(rotation);
            assert_vector_close(actual.right, expected.right);
            assert_vector_close(actual.forward, expected.forward);
            assert_vector_close(actual.up, expected.up);
        }
    }

    #[test]
    fn extracted_root_translation_keeps_the_re3_root_rotation() {
        let half = std::f32::consts::FRAC_1_SQRT_2;
        let rotation = CQuaternion::new(0.0, 0.0, half, half);
        let pose = CutsceneBonePose {
            name: "root".to_owned(),
            position: AnimVector::new(12.0, -4.0, 3.0),
            rotation,
            has_translation: true,
        };

        let root = gta_frame_from_pose_override(&pose, 0, true);
        let expected = cpp_quaternion_matrix(rotation);

        assert_eq!(root.position, CVector::ZERO);
        assert_vector_close(root.right, expected.right);
        assert_vector_close(root.forward, expected.forward);
        assert_vector_close(root.up, expected.up);
        assert_ne!(root, GtaFrameMatrix::IDENTITY);
    }

    #[test]
    fn vehicle_partial_root_translation_stays_on_the_clump() {
        // JACKEDCAR/vehicle enter-exit associations have translated root
        // sequences but do not carry ASSOC_HAS_TRANSLATION. re3 therefore
        // leaves their authored translation on the clump instead of moving it
        // onto CEntity via velocity extraction.
        let pose = CutsceneBonePose {
            name: "root".to_owned(),
            position: AnimVector::new(0.25, -0.5, -1.125),
            rotation: CQuaternion::new(0.0, 0.0, 0.0, 1.0),
            has_translation: true,
        };

        let root = gta_frame_from_pose_override(&pose, 0, false);

        assert_eq!(root.position, CVector::new(0.25, -0.5, -1.125));
    }

    #[test]
    fn only_has_translation_associations_extract_root_motion() {
        let mut sequence = CAnimBlendSequence::new();
        sequence.set_name("root");
        sequence.set_num_frames(2, true);
        sequence.set_key_frame(
            0,
            AnimKeyFrame::Trans(KeyFrameTrans {
                translation: AnimVector::new(1.0, 2.0, 3.0),
                delta_time: 1.0,
                ..Default::default()
            }),
        );
        sequence.set_key_frame(
            1,
            AnimKeyFrame::Trans(KeyFrameTrans {
                translation: AnimVector::new(3.0, 4.0, 5.0),
                delta_time: 1.0,
                ..Default::default()
            }),
        );
        let hierarchy = CAnimBlendHierarchy {
            sequences: vec![sequence],
            total_length: 2.0,
            ..Default::default()
        };
        let mut movement = CAnimBlendAssociation::new();
        movement.flags = ASSOC_HAS_TRANSLATION;

        let retained = ped_bone_poses_from_hierarchy(&hierarchy, 0.5, false, false);
        assert_eq!(retained[0].position, AnimVector::new(2.0, 3.0, 4.0));
        let extracted = ped_bone_poses_from_hierarchy(&hierarchy, 0.5, true, false);
        assert_eq!(extracted[0].position, AnimVector::new(2.0, 0.0, 4.0));
        let extracted_xy = ped_bone_poses_from_hierarchy(&hierarchy, 0.5, true, true);
        assert_eq!(extracted_xy[0].position, AnimVector::new(0.0, 0.0, 4.0));
        // Keep the association import in this parity test tied to the same
        // flag that the renderer now receives from the real template.
        assert!(movement.has_translation());
        movement.flags |= ASSOC_HAS_X_TRANSLATION;
        assert!(movement.has_x_translation());
    }

    #[test]
    fn partial_animation_reduces_base_weight_like_frame_update_callback() {
        fn association(translation_x: f32, flags: i32, blend: f32) -> CAnimBlendAssociation {
            let mut sequence = CAnimBlendSequence::new();
            sequence.set_name("root");
            sequence.set_num_frames(1, true);
            sequence.set_key_frame(
                0,
                AnimKeyFrame::Trans(KeyFrameTrans {
                    translation: AnimVector::new(translation_x, 0.0, 0.0),
                    ..Default::default()
                }),
            );
            let mut association = CAnimBlendAssociation::new();
            association.hierarchy = CAnimBlendHierarchy {
                sequences: vec![sequence],
                total_length: 1.0,
                ..Default::default()
            };
            association.flags = flags;
            association.blend_amount = blend;
            association
        }

        let clump = CAnimBlendClumpData {
            associations: vec![
                association(10.0, 0, 1.0),
                association(20.0, ASSOC_PARTIAL, 0.25),
            ],
            ..Default::default()
        };
        let poses = player_anim_clump_bone_poses(&clump);
        let root = poses.iter().find(|pose| pose.name == "root").unwrap();
        // C++: base 10*(1-.25) + partial 20*.25 = 12.5.
        assert!((root.position.x - 12.5).abs() < 1.0e-6);
    }

    #[test]
    fn separate_jump_partial_blends_over_player_movement_clump() {
        fn association(translation_x: f32, flags: i32, blend: f32) -> CAnimBlendAssociation {
            let mut sequence = CAnimBlendSequence::new();
            sequence.set_name("root");
            sequence.set_num_frames(1, true);
            sequence.set_key_frame(
                0,
                AnimKeyFrame::Trans(KeyFrameTrans {
                    translation: AnimVector::new(translation_x, 0.0, 0.0),
                    ..Default::default()
                }),
            );
            let mut association = CAnimBlendAssociation::new();
            association.hierarchy = CAnimBlendHierarchy {
                sequences: vec![sequence],
                total_length: 1.0,
                ..Default::default()
            };
            association.flags = flags;
            association.blend_amount = blend;
            association
        }

        let clump = CAnimBlendClumpData {
            associations: vec![association(10.0, 0, 1.0)],
            ..Default::default()
        };
        let landing = association(20.0, ASSOC_PARTIAL, 0.25);
        let poses = player_anim_clump_bone_poses_with_overlay(&clump, Some(&landing));
        let root = poses.iter().find(|pose| pose.name == "root").unwrap();
        assert!((root.position.x - 12.5).abs() < 1.0e-6);
    }

    #[test]
    fn mouse_diagonal_leg_pose_uses_renderware_postconcat_order() {
        let base = Quat::from_rotation_y(0.35);
        let mut poses = vec![CutsceneBonePose {
            name: "upperlegl".to_owned(),
            position: AnimVector::ZERO,
            rotation: CQuaternion::new(base.x, base.y, base.z, base.w),
            has_translation: false,
        }];
        let clump = CAnimBlendClumpData::new();
        let yaw = -45.0_f32.to_radians();

        apply_player_mouse_diagonal_leg_pose(&mut poses, &clump, Some(yaw));

        let pose = &poses[0];
        let actual = Quat::from_xyzw(
            pose.rotation.x,
            pose.rotation.y,
            pose.rotation.z,
            pose.rotation.w,
        )
        .normalize();
        // re3 calls RtQuatRotate(Z, pitch, POSTCONCAT), then
        // RtQuatRotate(X, yaw, POSTCONCAT). librw implements POSTCONCAT as
        // rotation * current quaternion.
        let expected = (Quat::from_rotation_x(yaw) * Quat::from_rotation_z(0.1) * base).normalize();
        assert!(actual.dot(expected).abs() > 1.0 - 1.0e-6);
    }
}

/// `CMatrix::SetRotate` followed by `Scale` and `Translate` (GTA axes).
fn gta_rotation_matrix(
    x_angle: f32,
    y_angle: f32,
    z_angle: f32,
    position: CVector,
    scale: f32,
) -> GtaFrameMatrix {
    let (sin_x, cos_x) = x_angle.sin_cos();
    let (sin_y, cos_y) = y_angle.sin_cos();
    let (sin_z, cos_z) = z_angle.sin_cos();
    GtaFrameMatrix {
        right: CVector::new(
            (cos_z * cos_y - (sin_z * sin_x) * sin_y) * scale,
            ((cos_z * sin_x) * sin_y + sin_z * cos_y) * scale,
            -cos_x * sin_y * scale,
        ),
        forward: CVector::new(-sin_z * cos_x * scale, cos_z * cos_x * scale, sin_x * scale),
        up: CVector::new(
            ((sin_z * sin_x) * cos_y + cos_z * sin_y) * scale,
            (sin_z * sin_y - (cos_z * sin_x) * cos_y) * scale,
            cos_x * cos_y * scale,
        ),
        position,
    }
}

fn should_render_atomic_for_model(
    is_vehicle: bool,
    frame_name: Option<&str>,
    damaged: bool,
    damage_state: Option<VehicleDamageRenderState>,
    optional_components: Option<[i8; 2]>,
) -> bool {
    if !is_vehicle {
        return true;
    }
    let Some(name) = frame_name else {
        return true;
    };
    if let Some(component) = vehicle_optional_component_index(name) {
        // re3 removes every optional component from the model template in
        // `PreprocessHierarchy`; an instance receives only the one or two
        // atomics recorded in `CVehicle::m_aExtras`.
        return optional_components
            .is_some_and(|selected| selected.into_iter().any(|value| value == component));
    }
    // re3 `CVehicleModelInfo::SetAtomicRendererCB` removes normal car/boat
    // `_lo` atomics and hides `_dam` atomics until a component swap occurs.
    if contains_ascii_ignore_case(name, "_lo") || contains_ascii_ignore_case(name, "_vlo") {
        return false;
    }
    if let Some(state) = damage_state
        && let Some(status) = vehicle_component_damage_status(name, state)
    {
        // `SetDoorDamage` / `SetPanelDamage` select the damaged atomic only
        // for SMASHED(1), hide both for MISSING(3), and otherwise retain the
        // normal atomic (SWINGING only disables angle culling in re3).
        if status >= 3 {
            return false;
        }
        // Panel/bumper status 2 follows a prior SMASHED1 transition; re3's
        // SetPanelDamage deliberately leaves that selected damaged atomic in
        // place. Door status 2 is SWINGING and keeps its normal atomic.
        let damaged_variant = status == 1
            || (status == 2
                && (contains_ascii_ignore_case(name, "wing_")
                    || contains_ascii_ignore_case(name, "bump_")
                    || contains_ascii_ignore_case(name, "windscreen")));
        if contains_ascii_ignore_case(name, "_dam") {
            return damaged_variant;
        }
        if contains_ascii_ignore_case(name, "_ok") {
            return !damaged_variant;
        }
    }
    if !damaged && contains_ascii_ignore_case(name, "_dam") {
        return false;
    }
    if damaged && contains_ascii_ignore_case(name, "_ok") {
        return false;
    }
    true
}

fn vehicle_optional_component_index(name: &str) -> Option<i8> {
    (0..6).find_map(|index| {
        name.eq_ignore_ascii_case(&format!("extra{}", index + 1))
            .then_some(index as i8)
    })
}

/// Map the re3 `carIds` component names to CDamageManager slots. Only names
/// that actually carry `_ok`/`_dam` atomics participate; unrelated model
/// names must never be hidden by a coincidental substring.
fn vehicle_component_damage_status(
    frame_name: &str,
    state: VehicleDamageRenderState,
) -> Option<u8> {
    if !(contains_ascii_ignore_case(frame_name, "_ok")
        || contains_ascii_ignore_case(frame_name, "_dam"))
    {
        return None;
    }
    let door = if contains_ascii_ignore_case(frame_name, "bonnet") {
        Some(0)
    } else if contains_ascii_ignore_case(frame_name, "boot") {
        Some(1)
    } else if contains_ascii_ignore_case(frame_name, "door_lf") {
        Some(2)
    } else if contains_ascii_ignore_case(frame_name, "door_rf") {
        Some(3)
    } else if contains_ascii_ignore_case(frame_name, "door_lr") {
        Some(4)
    } else if contains_ascii_ignore_case(frame_name, "door_rr") {
        Some(5)
    } else {
        None
    };
    if let Some(door) = door {
        return state.door_status.get(door).copied();
    }
    let panel = if contains_ascii_ignore_case(frame_name, "wing_lf") {
        Some(0)
    } else if contains_ascii_ignore_case(frame_name, "wing_rf") {
        Some(1)
    } else if contains_ascii_ignore_case(frame_name, "wing_lr") {
        Some(2)
    } else if contains_ascii_ignore_case(frame_name, "wing_rr") {
        Some(3)
    } else if contains_ascii_ignore_case(frame_name, "windscreen") {
        Some(4)
    } else if contains_ascii_ignore_case(frame_name, "bump_front") {
        Some(5)
    } else if contains_ascii_ignore_case(frame_name, "bump_rear") {
        Some(6)
    } else {
        None
    };
    panel.and_then(|panel| state.panel_status.get(panel).copied())
}

pub(super) fn atomic_frame_world_matrix(
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    frame_index: i32,
) -> Option<GtaFrameMatrix> {
    atomic_frame_world_matrix_with_poses(summary, frame_index, None)
}

/// Return the frame on which re3 ultimately renders a vehicle atomic after
/// `CVehicleModelInfo::PreprocessHierarchy` has applied `VEHICLE_FLAG_COLLAPSE`.
///
/// RenderWare's `MoveObjectsCB` merely reassigns the atomic to the component
/// dummy and then destroys all descendant frames.  It deliberately does not
/// multiply those descendant matrices into the geometry.  Keeping their LTM,
/// as the generic DFF path must do, makes `_dam` meshes explode away from the
/// body as soon as collision damage selects them.
fn vehicle_atomic_render_frame(
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    frame_index: i32,
    model: Option<&crate::modelinfo::ModelInfo::ModelInfoSlot>,
) -> Option<GtaFrameMatrix> {
    let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle)) = model else {
        return atomic_frame_world_matrix(summary, frame_index);
    };
    let mut current = usize::try_from(frame_index).ok()?;
    let mut remaining = summary.frames.len().saturating_add(1);
    while remaining > 0 {
        remaining -= 1;
        let frame = summary.frames.get(current)?;
        if frame
            .name
            .as_deref()
            .is_some_and(|name| vehicle_frame_is_collapsed_component(vehicle.vehicle_type, name))
        {
            return atomic_frame_world_matrix(summary, current as i32);
        }
        let Ok(parent) = usize::try_from(frame.parent_index) else {
            break;
        };
        current = parent;
    }
    atomic_frame_world_matrix(summary, frame_index)
}

/// The exact `VEHICLE_FLAG_COLLAPSE` entries in re3's `ms_vehicleDescs`.
fn vehicle_frame_is_collapsed_component(vehicle_type: i32, name: &str) -> bool {
    use crate::modelinfo::VehicleModelInfo::eVehicleType;

    let names: &[&str] = if vehicle_type == eVehicleType::VEHICLE_TYPE_CAR as i32 {
        &[
            "bump_front_dummy",
            "bonnet_dummy",
            "wing_rf_dummy",
            "wing_rr_dummy",
            "door_rf_dummy",
            "door_rr_dummy",
            "wing_lf_dummy",
            "wing_lr_dummy",
            "door_lf_dummy",
            "door_lr_dummy",
            "boot_dummy",
            "bump_rear_dummy",
            "windscreen_dummy",
        ]
    } else if vehicle_type == eVehicleType::VEHICLE_TYPE_BOAT as i32 {
        &["boat_moving_hi", "boat_rudder_hi", "windscreen"]
    } else if vehicle_type == eVehicleType::VEHICLE_TYPE_TRAIN as i32 {
        &["door_lhs_dummy", "door_rhs_dummy"]
    } else if vehicle_type == eVehicleType::VEHICLE_TYPE_HELI as i32 {
        &["chassis_dummy"]
    } else {
        &[]
    };
    names
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

/// Compose atomic LTM; optional IFP bone poses replace matching frame locals (cutscene).
fn atomic_frame_world_matrix_with_poses(
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    frame_index: i32,
    pose_overrides: Option<&[Option<GtaFrameMatrix>]>,
) -> Option<GtaFrameMatrix> {
    fn compose_frame(
        summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
        index: usize,
        pose_overrides: Option<&[Option<GtaFrameMatrix>]>,
        remaining: usize,
    ) -> Option<GtaFrameMatrix> {
        if remaining == 0 {
            return None;
        }
        let frame = summary.frames.get(index)?;
        let authored = GtaFrameMatrix::from_rw_frame(frame);
        let local = pose_overrides
            .and_then(|poses| poses.get(index).copied().flatten())
            .map(|mut pose| {
                // re3 FrameUpdateCallBackNonSkinned writes animation rotation,
                // then adds AnimBlendFrameData::resetPos to animation translation.
                pose.position = CVector::new(
                    authored.position.x + pose.position.x,
                    authored.position.y + pose.position.y,
                    authored.position.z + pose.position.z,
                );
                pose
            })
            .unwrap_or(authored);
        if frame.parent_index < 0 {
            return Some(local);
        }
        let parent = usize::try_from(frame.parent_index).ok()?;
        compose_frame(summary, parent, pose_overrides, remaining - 1)
            .map(|matrix| matrix.compose(local))
    }

    let frame_index = usize::try_from(frame_index).ok()?;
    compose_frame(
        summary,
        frame_index,
        pose_overrides,
        summary.frames.len().saturating_add(1),
    )
}


fn gta_frame_from_cutscene_pose(
    pose: &crate::animation::CutsceneMgr::CutsceneBonePose,
) -> GtaFrameMatrix {
    // IFP and RenderWare both store this quaternion in GTA/RW axes.  Keep it
    // there while composing the frame hierarchy; convert to Bevy only once,
    // when vertices/entity transforms cross the renderer boundary.
    let mut q = Quat::from_xyzw(
        pose.rotation.x,
        pose.rotation.y,
        pose.rotation.z,
        pose.rotation.w,
    );
    if q.length_squared() > 1e-8 {
        q = q.normalize();
    } else {
        q = Quat::IDENTITY;
    }
    let mat = Mat3::from_quat(q);
    let right = mat.col(0);
    let forward = mat.col(1);
    let up = mat.col(2);
    GtaFrameMatrix {
        right: CVector::new(right.x, right.y, right.z),
        forward: CVector::new(forward.x, forward.y, forward.z),
        up: CVector::new(up.x, up.y, up.z),
        position: if pose.has_translation {
            CVector::new(pose.position.x, pose.position.y, pose.position.z)
        } else {
            CVector::ZERO
        },
    }
}

fn gta_frame_from_pose_override(
    pose: &crate::animation::CutsceneMgr::CutsceneBonePose,
    pose_index: usize,
    root_translation_on_entity: bool,
) -> GtaFrameMatrix {
    let mut frame = gta_frame_from_cutscene_pose(pose);
    if root_translation_on_entity && pose_index == 0 && pose.has_translation {
        // re3 extracts only the movement association's root translation to
        // CEntity while retaining its quaternion on the clump root.
        frame.position = CVector::ZERO;
    }
    frame
}

/// Build re3's clump-frame lookup once per pose refresh. The C++ renderer
/// stores `AnimBlendFrameData` in frame order; an index-aligned table avoids
/// cloning bone names and walking a temporary tree for every atomic/bone.
fn frame_pose_overrides(
    summary: &RwClumpSummary,
    bone_poses: &[crate::animation::CutsceneMgr::CutsceneBonePose],
    root_translation_on_entity: bool,
) -> Vec<Option<GtaFrameMatrix>> {
    let mut overrides = vec![None; summary.frames.len()];
    for (frame_index, frame) in summary.frames.iter().enumerate() {
        let Some(frame_name) = frame.name.as_deref() else {
            continue;
        };
        let Some((pose_index, pose)) = bone_poses
            .iter()
            .enumerate()
            .find(|(_, pose)| animation_frame_names_match(&pose.name, frame_name))
        else {
            continue;
        };
        overrides[frame_index] = Some(gta_frame_from_pose_override(
            pose,
            pose_index,
            root_translation_on_entity,
        ));
    }

    overrides
}

/// Cutscene clump drawables with IFP bone poses applied to matching frames.
fn cutscene_slot_root_transform(
    object: &crate::animation::CutsceneMgr::CutsceneObjectSlot,
) -> Transform {
    let translation =
        CVector::new(object.position.x, object.position.y, object.position.z).to_bevy();
    // GTA IFP quat (x,y,z,w) → Bevy's X/Y-up/Z-back convention.
    let r = object.rotation;
    let raw_rotation = Quat::from_xyzw(r.x, r.z, -r.y, r.w);
    let rotation = if raw_rotation.length_squared() > 1e-8 {
        raw_rotation.normalize()
    } else {
        Quat::IDENTITY
    };
    Transform::from_translation(translation).with_rotation(rotation)
}

pub(super) fn player_anim_clump_bone_poses(
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    player_anim_clump_bone_poses_with_overlay(clump, None)
}

/// re3 `CPed::CalculateNewVelocity` applies a small post-animation rotation to
/// both upper legs while mouse-controlled movement stays inside the forward
/// +/-50 degree cone. This is what makes W+A/W+D read as diagonal locomotion
/// even though ProcessAnimGroups intentionally keeps the normal player RUN
/// hierarchy for a 45 degree input.
pub(super) fn apply_player_mouse_diagonal_leg_pose(
    poses: &mut [crate::animation::CutsceneMgr::CutsceneBonePose],
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    local_walk_angle: Option<f32>,
) {
    use crate::animation::AnimationId::{ANIM_STD_FIGHT_IDLE, ANIM_STD_IDLE};

    let idle_blocks_leg_turn = clump
        .associations
        .iter()
        .find(|association| association.anim_id == ANIM_STD_IDLE)
        .is_some_and(|association| association.blend_amount >= 0.5);
    let fight_idle_present = clump
        .associations
        .iter()
        .any(|association| association.anim_id == ANIM_STD_FIGHT_IDLE);
    if idle_blocks_leg_turn || fight_idle_present {
        return;
    }

    let Some(yaw) = local_walk_angle else {
        return;
    };

    let fifty_degrees = 50.0_f32.to_radians();
    if yaw <= -fifty_degrees || yaw >= fifty_degrees {
        return;
    }

    // PED_SKIN branch in re3: pitch=0.1 around Z, then yaw around X, both
    // `rwCOMBINEPOSTCONCAT`. librw's quaternion post-concat is `rot * q`,
    // not `q * rot`, so preserve that order exactly here.
    let pitch = Quat::from_rotation_z(0.1);
    let yaw_rotation = Quat::from_rotation_x(yaw);
    for pose in poses.iter_mut().filter(|pose| {
        animation_frame_names_match(&pose.name, "Supperlegl")
            || animation_frame_names_match(&pose.name, "Supperlegr")
    }) {
        let mut rotation = Quat::from_xyzw(
            pose.rotation.x,
            pose.rotation.y,
            pose.rotation.z,
            pose.rotation.w,
        );
        if rotation.length_squared() <= 1.0e-8 {
            rotation = Quat::IDENTITY;
        } else {
            rotation = rotation.normalize();
        }
        rotation = (pitch * rotation).normalize();
        rotation = (yaw_rotation * rotation).normalize();
        pose.rotation = crate::math::Quaternion::CQuaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        );
    }
}

pub(super) fn player_anim_clump_bone_poses_with_overlay(
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    overlay: Option<&crate::animation::AnimBlendAssociation::CAnimBlendAssociation>,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    use crate::math::{Quaternion::CQuaternion, Vector::CVector as AnimVector};
    let reduce_base_for_partials = clump
        .associations
        .iter()
        .chain(overlay.iter().copied())
        .any(|association| association.blend_amount > 0.0 && !association.is_movement());
    // RpAnimBlend sets `foobar` when any active association is not a movement
    // association. In that case the base/full-body weight is reduced by the
    // partial blend amount on each frame that the partial animation owns.
    let estimated_bones = clump
        .associations
        .iter()
        .chain(overlay.iter().copied())
        .filter(|association| association.blend_amount > 0.0)
        .map(|association| association.hierarchy.sequences.len())
        .max()
        .unwrap_or(0);
    let mut accumulated =
        Vec::<(String, AnimVector, CQuaternion, f32, bool)>::with_capacity(estimated_bones);
    for association in clump
        .associations
        .iter()
        .chain(overlay.iter().copied())
        .filter(|association| association.blend_amount > 0.0)
    {
        let extracted_root_sequence = if association.has_translation() {
            association
                .hierarchy
                .sequences
                .iter()
                .position(|sequence| sequence.has_translation())
        } else {
            None
        };
        for (sequence_index, sequence) in association.hierarchy.sequences.iter().enumerate() {
            let Some(mut pose) = sample_anim_sequence_pose(sequence, association.current_time)
            else {
                continue;
            };
            if extracted_root_sequence == Some(sequence_index) {
                // FrameUpdateCallBackWithVelocityExtractionNonSkinned extracts
                // locomotion from Y, and X only for ASSOC_HAS_X_TRANSLATION.
                // It deliberately keeps authored Z on the root frame. Landing
                // and FALL_COLLAPSE use that Z to lower the body onto the
                // ground; clearing all three axes made the ped lie in mid-air.
                pose.position.y = 0.0;
                if association.has_x_translation() {
                    pose.position.x = 0.0;
                }
            }
            let partial_blend = if reduce_base_for_partials {
                clump
                    .associations
                    .iter()
                    .chain(overlay.iter().copied())
                    .filter(|other| {
                        other.blend_amount > 0.0
                            && other.is_partial()
                            && other.hierarchy.sequences.iter().any(|other_sequence| {
                                (other_sequence
                                    .get_key_frame_at_time(other.current_time)
                                    .or_else(|| other_sequence.get_key_frame(0)))
                                .is_some()
                                    && animation_sequence_name(other_sequence).is_some_and(|name| {
                                        animation_frame_names_match(name, &pose.name)
                                    })
                            })
                    })
                    .map(|other| other.blend_amount)
                    .sum::<f32>()
            } else {
                0.0
            };
            let weight = association.get_blend_amount(1.0 - partial_blend);
            if weight <= 0.0 {
                continue;
            }
            let entry =
                if let Some(index) = accumulated.iter().position(|(name, ..)| name == &pose.name) {
                    &mut accumulated[index]
                } else {
                    accumulated.push((pose.name, AnimVector::ZERO, CQuaternion::ZERO, 0.0, false));
                    accumulated.last_mut().expect("pose was just appended")
                };
            entry.1 += pose.position * weight;
            let weighted_rotation = pose.rotation * weight;
            if entry.3 > 0.0 && entry.2.dot(weighted_rotation) < 0.0 {
                entry.2 -= weighted_rotation;
            } else {
                entry.2 += weighted_rotation;
            }
            entry.3 += weight;
            entry.4 |= pose.has_translation;
        }
    }
    // The previous BTreeMap yielded lexical order. Keep that observable order
    // while replacing its per-node allocation and logarithmic insertion cost.
    accumulated.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    accumulated
        .into_iter()
        .map(|(name, position, mut rotation, weight, has_translation)| {
            if weight > 0.0 {
                rotation.normalise();
            }
            crate::animation::CutsceneMgr::CutsceneBonePose {
                name,
                position,
                rotation,
                has_translation,
            }
        })
        .collect()
}

fn animation_sequence_name(
    sequence: &crate::animation::AnimBlendSequence::CAnimBlendSequence,
) -> Option<&str> {
    if sequence.name.is_empty() {
        crate::animation::Bones::convert_bone_tag_2_bone_name(i32::from(sequence.bone_tag))
    } else {
        Some(sequence.name.as_str())
    }
}

fn sample_anim_sequence_pose(
    sequence: &crate::animation::AnimBlendSequence::CAnimBlendSequence,
    sample_time: f32,
) -> Option<crate::animation::CutsceneMgr::CutsceneBonePose> {
    let frame = sequence
        .get_key_frame_at_time(sample_time)
        .or_else(|| sequence.get_key_frame(0))?;
    Some(crate::animation::CutsceneMgr::CutsceneBonePose {
        name: animation_sequence_name(sequence)?.to_owned(),
        position: frame.translation(),
        rotation: frame.rotation(),
        has_translation: sequence.has_translation(),
    })
}

/// Build the non-skinned GTA III PC ped limb-frame pose from the real PED.IFP
/// association template. This is the render-side equivalent of re3
/// `RpAnimBlendClumpInitNotSkinned` + `FrameUpdateCallBackNonSkinned`.
pub(super) fn ped_ifp_bone_poses(
    manager: &CAnimManager,
    group_id: usize,
    move_state: crate::peds::Ped::eMoveState,
    time_seconds: f32,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    use crate::animation::AnimationId::{
        ANIM_STD_IDLE, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK,
    };
    let animation_id = match move_state {
        crate::peds::Ped::eMoveState::PEDMOVE_SPRINT => ANIM_STD_RUNFAST,
        crate::peds::Ped::eMoveState::PEDMOVE_RUN => ANIM_STD_RUN,
        crate::peds::Ped::eMoveState::PEDMOVE_WALK => ANIM_STD_WALK,
        _ => ANIM_STD_IDLE,
    };
    ped_ifp_bone_poses_for_animation(manager, group_id, animation_id, time_seconds)
}

fn ped_ifp_bone_poses_for_animation(
    manager: &CAnimManager,
    group_id: usize,
    animation_id: crate::animation::AnimationId::AnimationId,
    time_seconds: f32,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    let Some(association) = manager.get_anim_association(group_id, animation_id as usize) else {
        return Vec::new();
    };
    let hierarchy = &association.hierarchy;
    let sample_time = if hierarchy.total_length > 0.0 {
        time_seconds.rem_euclid(hierarchy.total_length)
    } else {
        0.0
    };
    ped_bone_poses_from_hierarchy(
        hierarchy,
        sample_time,
        association.has_translation(),
        association.has_x_translation(),
    )
}

pub(super) fn ped_ifp_bone_poses_for_one_shot(
    manager: &CAnimManager,
    group_id: usize,
    animation_id: crate::animation::AnimationId::AnimationId,
    time_seconds: f32,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    let Some(association) = manager.get_anim_association(group_id, animation_id as usize) else {
        return Vec::new();
    };
    let hierarchy = &association.hierarchy;
    let sample_time = time_seconds.clamp(0.0, hierarchy.total_length.max(0.0));
    ped_bone_poses_from_hierarchy(
        hierarchy,
        sample_time,
        association.has_translation(),
        association.has_x_translation(),
    )
}

pub(super) fn ped_frame_transform(
    model_index: usize,
    poses: &[crate::animation::CutsceneMgr::CutsceneBonePose],
    frame_name: &str,
    file_loader: &CFileLoader,
) -> Option<Transform> {
    let summary = file_loader
        .model_info
        .get_model_info(model_index)?
        .base()
        .clump_summary
        .as_ref()?;
    let frame_index = summary.frames.iter().position(|frame| {
        frame
            .name
            .as_deref()
            .is_some_and(|name| animation_frame_names_match(frame_name, name))
            || (frame_name.eq_ignore_ascii_case("Rhand")
                && frame.hanim_id == Some(crate::animation::Bones::BONE_RHAND as i32))
    })?;
    // Player poses arrive after association-aware root extraction; keep the
    // remaining authored Z so weapons follow crouch/landing/collapse height.
    let pose_overrides = frame_pose_overrides(summary, poses, false);
    atomic_frame_world_matrix_with_poses(summary, frame_index as i32, Some(&pose_overrides))
        .map(GtaFrameMatrix::to_bevy_transform)
}

/// Resolve a point authored relative to a live ped frame into GTA world space.
///
/// This is the ECS equivalent of re3 `CPed::TransformToNode`: gameplay and
/// rendering sample the same blended clump pose, then the ped entity matrix
/// supplies the final world transform.
pub(crate) fn ped_frame_world_point_from_clump(
    model_index: usize,
    file_loader: &CFileLoader,
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    entity_matrix: &crate::math::Matrix::CMatrix,
    frame_name: &str,
    local_point: CVector,
) -> Option<CVector> {
    let poses = player_anim_clump_bone_poses(clump);
    let frame = ped_frame_transform(model_index, &poses, frame_name, file_loader)?;
    let root = entity_matrix_to_bevy_transform(entity_matrix);
    Some(CVector::from_bevy(
        root.mul_transform(frame)
            .transform_point(local_point.to_bevy()),
    ))
}

/// CPedModelInfo::AnimatePedColModel samples all hit nodes from the same pose
/// used to draw the actor. The invariant DFF indices live with the resident model.
pub(crate) fn ped_weapon_hit_model(
    model_index: usize,
    file_loader: &CFileLoader,
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
) -> Option<crate::collision::ColModel::CColModel> {
    use crate::{collision::ColModel::{CColBox, CColModel, CColSphere, CColVector, SURFACE_PED}, modelinfo::ModelInfo::ModelInfoSlot};
    let Some(ModelInfoSlot::Ped(model)) = file_loader.model_info.get_model_info(model_index) else { return None; };
    let summary = model.clump.base.clump_summary.as_ref()?;
    if model.hit_spheres.len() != 8 {
        bevy::log::warn_once!("ped_weapon_waiting_original_hit_model: model={model_index}");
        return None;
    }
    let poses = player_anim_clump_bone_poses(clump);
    let overrides = frame_pose_overrides(summary, &poses, false);
    let mut spheres = Vec::with_capacity(model.hit_spheres.len());
    for node in &model.hit_spheres {
        let frame = atomic_frame_world_matrix_with_poses(summary, node.frame_index as i32, Some(&overrides))?;
        spheres.push(CColSphere::new(node.radius, CColVector::new(
            frame.position.x + node.offset_x, frame.position.y, frame.position.z + node.offset_z,
        ), SURFACE_PED, node.piece as u8));
    }
    Some(CColModel::new(CColSphere::new(2.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
        CColBox::new(CColVector::new(-0.5, -0.5, -1.2), CColVector::new(0.5, 0.5, 1.2), 0, 0), spheres))
}

/// Resolve an authored ped frame without an animation override. Aiming queries
/// use this stable PED_MID point rather than a locomotion pose offset.
pub(crate) fn ped_frame_world_point_static(
    model_index: usize,
    file_loader: &CFileLoader,
    entity_matrix: &crate::math::Matrix::CMatrix,
    frame_name: &str,
    local_point: CVector,
) -> Option<CVector> {
    let frame = ped_frame_transform(model_index, &[], frame_name, file_loader)?;
    let root = entity_matrix_to_bevy_transform(entity_matrix);
    Some(CVector::from_bevy(
        root.mul_transform(frame)
            .transform_point(local_point.to_bevy()),
    ))
}

/// Render the real current time of a persistent `m_pVehicleAnim` association.
pub(super) fn ped_bone_poses_from_association(
    association: &crate::animation::AnimBlendAssociation::CAnimBlendAssociation,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    ped_bone_poses_from_hierarchy(
        &association.hierarchy,
        association.current_time,
        association.has_translation(),
        association.has_x_translation(),
    )
}

fn ped_bone_poses_from_hierarchy(
    hierarchy: &crate::animation::AnimBlendHierarchy::CAnimBlendHierarchy,
    sample_time: f32,
    extract_root_translation: bool,
    extract_root_x_translation: bool,
) -> Vec<crate::animation::CutsceneMgr::CutsceneBonePose> {
    let mut poses = hierarchy
        .sequences
        .iter()
        .filter_map(|sequence| sample_anim_sequence_pose(sequence, sample_time))
        .collect::<Vec<_>>();

    // re3's 2D velocity-extraction callback removes root Y for
    // ASSOC_HAS_TRANSLATION, and root X only for ASSOC_HAS_X_TRANSLATION. Z is
    // never extracted here: landing/collapse animations need the authored Z
    // offset to visually reach the physical ground. Vehicle enter/exit clips
    // lack ASSOC_HAS_TRANSLATION and therefore retain their full root motion.
    if extract_root_translation
        && let Some(root) = poses.iter_mut().find(|pose| pose.has_translation)
    {
        root.position.y = 0.0;
        if extract_root_x_translation {
            root.position.x = 0.0;
        }
    }
    poses
}

/// re3 `CCutsceneHead::ProcessControl` / `Render` placement.
/// A separate head follows the source `CCutsceneObject`'s `Shead` frame and
/// receives the same local +90° GTA-Y rotation before rendering its own clump.
pub(super) fn cutscene_object_render_transform(
    object: &crate::animation::CutsceneMgr::CutsceneObjectSlot,
    objects: &[crate::animation::CutsceneMgr::CutsceneObjectSlot],
    file_loader: &CFileLoader,
) -> Transform {
    if !object.is_head {
        return cutscene_slot_root_transform(object);
    }
    let Some(parent_slot) = object.parent_object_slot else {
        return cutscene_slot_root_transform(object);
    };
    let Some(parent) = objects.get(parent_slot) else {
        return cutscene_slot_root_transform(object);
    };
    let summary = parent
        .model_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| {
            file_loader
                .model_info
                .get_model_info(parent.model_id.max(0) as usize)
                .and_then(|model| model.base().clump_summary.as_ref())
        });
    let Some(summary) = summary else {
        return cutscene_slot_root_transform(parent);
    };
    let Some(shead_index) = summary.frames.iter().position(|frame| {
        frame
            .name
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case("shead"))
            || frame.hanim_id == Some(crate::animation::Bones::BONE_HEAD)
    }) else {
        return cutscene_slot_root_transform(parent);
    };
    let pose_overrides = frame_pose_overrides(summary, &parent.bone_poses, true);
    let mut local =
        atomic_frame_world_matrix_with_poses(summary, shead_index as i32, Some(&pose_overrides))
            .unwrap_or(GtaFrameMatrix::IDENTITY);
    // The ANM animates the separate head clump's skin hierarchy. It does not
    // alter the object's world matrix; `ProcessControl` derives that matrix
    // solely from the parent actor's `Shead` frame.
    // `CMatrix::SetRotateY(PI/2)` in CutsceneHead.cpp, expressed in GTA axes.
    local = local.compose(GtaFrameMatrix {
        right: CVector::new(0.0, 0.0, -1.0),
        forward: CVector::new(0.0, 1.0, 0.0),
        up: CVector::new(1.0, 0.0, 0.0),
        position: CVector::ZERO,
    });
    cutscene_slot_root_transform(parent).mul_transform(local.to_bevy_transform())
}

pub(super) fn model_uses_cpu_skinning(
    model_index: usize,
    file_loader: &CFileLoader,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) -> bool {
    model_snapshot
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| {
            file_loader
                .model_info
                .get_model_info(model_index)
                .and_then(|model| model.base().clump_summary.as_ref())
        })
        .is_some_and(|summary| {
            summary.atomics.iter().any(|atomic| {
                usize::try_from(atomic.geometry_index)
                    .ok()
                    .and_then(|index| summary.geometries.get(index))
                    .is_some_and(|geometry| geometry.skin.is_some())
            })
        })
}

fn apply_cutscene_texture_slot(
    drawables: &mut [ModelMaterialDrawable],
    model_index: usize,
    file_loader: &CFileLoader,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) {
    let slot = model_snapshot
        .and_then(|snapshot| usize::try_from(snapshot.txd_slot).ok())
        .or_else(|| {
            file_loader
                .model_info
                .get_model_info(model_index)
                .and_then(|model| usize::try_from(model.base().txd_slot).ok())
        });
    for drawable in drawables {
        drawable.texture_txd_slot = slot;
    }
}

/// Build GTA III PC's non-skinned ped/cutscene clumps once. RenderWare keeps
/// each limb mesh static and changes its owning RwFrame matrix every animation
/// step (`FrameUpdateCallBackNonSkinned`); baking those matrices into vertices
/// every frame is both slower and behaviorally different.
pub(super) fn build_rigid_cutscene_drawables(
    model_index: usize,
    file_loader: &CFileLoader,
    hide_original_head: bool,
    optional_components: Option<[i8; 2]>,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) -> (Vec<ModelMaterialDrawable>, Vec<i32>) {
    let mut out = Vec::new();
    let mut frame_indices = Vec::new();
    let Some(summary) = model_snapshot
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| {
            file_loader
                .model_info
                .get_model_info(model_index)
                .and_then(|model| model.base().clump_summary.as_ref())
        })
    else {
        if let Some(simple) = file_loader.model_info.simple_model_info(model_index) {
            for binding in &simple.atomic_bindings {
                let first = out.len();
                append_geometry_drawables(
                    model_index,
                    &binding.geometry,
                    VehicleColourRemap::default(),
                    None,
                    &mut out,
                );
                apply_cutscene_texture_slot(
                    &mut out[first..],
                    model_index,
                    file_loader,
                    model_snapshot,
                );
                frame_indices.extend(std::iter::repeat_n(-1, out.len().saturating_sub(first)));
            }
        }
        return (out, frame_indices);
    };
    let vehicle_type = model_snapshot
        .and_then(|snapshot| snapshot.vehicle_type)
        .or_else(
            || match file_loader.model_info.get_model_info(model_index) {
                Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle)) => {
                    Some(vehicle.vehicle_type)
                }
                _ => None,
            },
        );
    for atomic in &summary.atomics {
        if !should_render_atomic_for_model(
            vehicle_type.is_some(),
            atomic.frame_name.as_deref(),
            false,
            None,
            optional_components,
        ) {
            continue;
        }
        if atomic.frame_name.as_deref().is_some_and(|name| {
            vehicle_type.is_some_and(|kind| vehicle_frame_adds_separate_wheel(kind, name))
        }) {
            continue;
        }
        if hide_original_head
            && atomic_descends_from_named_frame(summary, atomic.frame_index, "shead")
        {
            continue;
        }
        let Some(geometry) = usize::try_from(atomic.geometry_index)
            .ok()
            .and_then(|index| summary.geometries.get(index))
        else {
            continue;
        };
        let first = out.len();
        append_geometry_drawables(
            model_index,
            geometry,
            VehicleColourRemap::default(),
            None,
            &mut out,
        );
        apply_cutscene_texture_slot(&mut out[first..], model_index, file_loader, model_snapshot);
        frame_indices.extend(std::iter::repeat_n(
            atomic.frame_index,
            out.len().saturating_sub(first),
        ));
    }
    // `CVehicleModelInfo::PreprocessHierarchy` creates wheel atomics from the
    // separate wheel model and attaches one to every `wheel_*_dummy` frame.
    // Raw DFF summaries predate that runtime mutation, so the generic rigid
    // cutscene path above contains the body but no wheels.
    let wheel = model_snapshot
        .and_then(|snapshot| Some((snapshot.wheel_id?, snapshot.wheel_scale?)))
        .or_else(
            || match file_loader.model_info.get_model_info(model_index) {
                Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle)) => {
                    Some((vehicle.wheel_id, vehicle.wheel_scale))
                }
                _ => None,
            },
        );
    if let Some((wheel_id, wheel_scale)) = wheel
        && let Ok(wheel_model_id) = usize::try_from(wheel_id)
        && let Some(wheel_model) = file_loader.model_info.simple_model_info(wheel_model_id)
        && let Some(wheel_binding) = wheel_model.atomic_binding_from_distance(
            0.0,
            file_loader.renderer.lod_distance_multiplier(),
            false,
        )
    {
        for (frame_index, frame) in summary.frames.iter().enumerate() {
            let is_wheel_dummy = frame.name.as_deref().is_some_and(|name| {
                vehicle_type.is_some_and(|kind| vehicle_frame_adds_separate_wheel(kind, name))
            });
            if !is_wheel_dummy {
                continue;
            }
            let first = out.len();
            append_geometry_drawables(
                wheel_model_id,
                &wheel_binding.geometry,
                VehicleColourRemap::default(),
                Some(GtaFrameMatrix::IDENTITY.with_uniform_basis_scale(wheel_scale.max(0.001))),
                &mut out,
            );
            apply_cutscene_texture_slot(
                &mut out[first..],
                model_index,
                file_loader,
                model_snapshot,
            );
            frame_indices.extend(std::iter::repeat_n(
                frame_index as i32,
                out.len().saturating_sub(first),
            ));
        }
    }
    if out.is_empty() {
        for geometry in &summary.geometries {
            let first = out.len();
            append_geometry_drawables(
                model_index,
                geometry,
                VehicleColourRemap::default(),
                None,
                &mut out,
            );
            apply_cutscene_texture_slot(
                &mut out[first..],
                model_index,
                file_loader,
                model_snapshot,
            );
            frame_indices.extend(std::iter::repeat_n(-1, out.len().saturating_sub(first)));
        }
    }
    (out, frame_indices)
}

pub(super) fn rigid_cutscene_part_transforms(
    model_index: usize,
    file_loader: &CFileLoader,
    bone_poses: &[crate::animation::CutsceneMgr::CutsceneBonePose],
    root_translation_on_entity: bool,
    frame_indices: &[i32],
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) -> Option<Vec<Transform>> {
    let summary = model_snapshot
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| {
            file_loader
                .model_info
                .get_model_info(model_index)
                .and_then(|model| model.base().clump_summary.as_ref())
        })?;
    let pose_overrides = frame_pose_overrides(summary, bone_poses, root_translation_on_entity);
    let poses = Some(pose_overrides.as_slice());
    // One RW atomic commonly produces several material drawables with the same
    // frame index. Re3 computes that frame LTM once; avoid walking the same
    // parent hierarchy once per material on every 30 Hz animation refresh.
    let mut frame_cache = vec![None::<Transform>; summary.frames.len()];
    Some(
        frame_indices
            .iter()
            .map(|frame_index| {
                let Ok(frame_index) = usize::try_from(*frame_index) else {
                    return Transform::IDENTITY;
                };
                let Some(slot) = frame_cache.get_mut(frame_index) else {
                    return Transform::IDENTITY;
                };
                *slot.get_or_insert_with(|| {
                    atomic_frame_world_matrix_with_poses(summary, frame_index as i32, poses)
                        .unwrap_or(GtaFrameMatrix::IDENTITY)
                        .to_bevy_transform()
                })
            })
            .collect(),
    )
}

pub(super) fn build_cutscene_drawables(
    model_index: usize,
    file_loader: &CFileLoader,
    bone_poses: &[crate::animation::CutsceneMgr::CutsceneBonePose],
    root_translation_on_entity: bool,
    head_animation: Option<&crate::animation::CutsceneMgr::CutsceneHeadAnimationPose>,
    hide_original_head: bool,
    optional_components: Option<[i8; 2]>,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) -> Vec<ModelMaterialDrawable> {
    let mut out = Vec::new();
    let live_model = file_loader.model_info.get_model_info(model_index);
    let summary = model_snapshot
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| live_model.and_then(|model| model.base().clump_summary.as_ref()));
    let Some(summary) = summary else {
        // Fall back to simple/atomic path without poses.
        return build_model_material_drawables(
            model_index,
            0.0,
            false,
            file_loader,
            VehicleColourRemap::default(),
        );
    };
    // Root is already on the Bevy entity transform — override only child bones.
    let pose_overrides = frame_pose_overrides(summary, bone_poses, root_translation_on_entity);
    let poses = Some(pose_overrides.as_slice());
    let is_vehicle = matches!(
        live_model,
        Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(_))
    ) || model_snapshot.is_some_and(|snapshot| snapshot.vehicle_type.is_some());
    let mut drew = false;
    for atomic in &summary.atomics {
        if !should_render_atomic_for_model(
            is_vehicle,
            atomic.frame_name.as_deref(),
            false,
            None,
            optional_components,
        ) {
            continue;
        }
        // CCutsceneHead constructor clears rpATOMICRENDER on the parent's
        // `Shead` atomic before rendering the separately animated head clump.
        if hide_original_head
            && atomic_descends_from_named_frame(summary, atomic.frame_index, "shead")
        {
            continue;
        }
        let Some(geometry_index) = usize::try_from(atomic.geometry_index).ok() else {
            continue;
        };
        let Some(geometry) = summary.geometries.get(geometry_index) else {
            continue;
        };
        let frame = atomic_frame_world_matrix_with_poses(summary, atomic.frame_index, poses);
        // Most GTA III PC peds are frame-separated atomics, but cutscene and
        // platform DFFs may carry a standard RW Skin PLG. In that case the
        // vertex positions come from the real bone palette, not the atomic
        // frame alone (`RpAnimBlendClumpInitSkinned`).
        let first = out.len();
        if let Some(deformed) = deform_skin_geometry(geometry, summary, poses, head_animation) {
            let skinned_geometry = deformed;
            append_decoded_geometry_drawables(
                model_index,
                geometry,
                &skinned_geometry,
                VehicleColourRemap::default(),
                None,
                &mut out,
            );
        } else {
            append_geometry_drawables(
                model_index,
                geometry,
                VehicleColourRemap::default(),
                frame,
                &mut out,
            );
        }
        apply_cutscene_texture_slot(&mut out[first..], model_index, file_loader, model_snapshot);
        drew = true;
    }
    if !drew {
        for geometry in &summary.geometries {
            let first = out.len();
            append_geometry_drawables(
                model_index,
                geometry,
                VehicleColourRemap::default(),
                None,
                &mut out,
            );
            apply_cutscene_texture_slot(
                &mut out[first..],
                model_index,
                file_loader,
                model_snapshot,
            );
        }
    }
    out
}

fn atomic_descends_from_named_frame(
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    frame_index: i32,
    frame_name: &str,
) -> bool {
    let mut current = frame_index;
    for _ in 0..summary.frames.len() {
        let Ok(index) = usize::try_from(current) else {
            return false;
        };
        let Some(frame) = summary.frames.get(index) else {
            return false;
        };
        if frame
            .name
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case(frame_name))
        {
            return true;
        }
        if frame.parent_index < 0 {
            return false;
        }
        current = frame.parent_index;
    }
    false
}

/// CPU equivalent of re3's skin palette upload for the immediate-mesh
/// renderer. `Skin::inverseMatrices` maps bind-pose skin space into a bone;
/// the animated frame matrix maps that bone back into clump space. This keeps
/// the normal DFF path usable on every Bevy backend without embedding GTA
/// assets or adding a second shader language.
/// RenderWare `HAnimHierarchy::updateMatrices`: walk local ANM transforms in
/// interpolator order and obey the serialized POP/PUSH parent flags.
fn build_hanim_hierarchy_matrices(
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    animation: &crate::animation::CutsceneMgr::CutsceneHeadAnimationPose,
) -> Vec<GtaFrameMatrix> {
    const RP_HANIM_POP_PARENT_MATRIX: i32 = 1;
    const RP_HANIM_PUSH_PARENT_MATRIX: i32 = 2;

    let count = summary.hanim_nodes.len().max(
        animation
            .nodes
            .iter()
            .map(|pose| pose.node_index + 1)
            .max()
            .unwrap_or(0),
    );
    let mut matrices = vec![GtaFrameMatrix::IDENTITY; count];
    let local_for_node = |node_index: usize| {
        animation
            .nodes
            .iter()
            .find(|pose| pose.node_index == node_index)
            .map(|pose| {
                gta_frame_from_cutscene_pose(&crate::animation::CutsceneMgr::CutsceneBonePose {
                    name: String::new(),
                    position: pose.position,
                    rotation: pose.rotation,
                    has_translation: true,
                })
            })
            .unwrap_or(GtaFrameMatrix::IDENTITY)
    };
    // librw `HAnimHierarchy::updateMatrices`: unless LOCALSPACEMATRICES is
    // set, the parent of the hierarchy-owning frame supplies `rootMat`.
    const RP_HANIM_LOCAL_SPACE_MATRICES: i32 = 0x4000;
    let root_matrix = if summary.hanim_hierarchy_flags & RP_HANIM_LOCAL_SPACE_MATRICES == 0 {
        summary
            .hanim_root_frame_index
            .and_then(|root| summary.frames.get(root))
            .and_then(|root| (root.parent_index >= 0).then_some(root.parent_index))
            .and_then(|parent| atomic_frame_world_matrix(summary, parent))
            .unwrap_or(GtaFrameMatrix::IDENTITY)
    } else {
        GtaFrameMatrix::IDENTITY
    };
    // The parent pointer is pushed before node zero, then every node applies
    // PUSH before replacing the parent and POP afterwards.
    let mut parent = root_matrix;
    let mut stack = vec![parent];
    for node_index in 0..count {
        let current = parent.compose(local_for_node(node_index));
        matrices[node_index] = current;
        let flags = summary
            .hanim_nodes
            .get(node_index)
            .map(|node| node.flags)
            .unwrap_or(0);
        if flags & RP_HANIM_PUSH_PARENT_MATRIX != 0 {
            stack.push(parent);
        }
        parent = current;
        if flags & RP_HANIM_POP_PARENT_MATRIX != 0 {
            parent = stack.pop().unwrap_or(GtaFrameMatrix::IDENTITY);
        }
    }
    matrices
}

pub(super) fn deform_skin_geometry(
    geometry_desc: &re3bevy_formats::formats::ClumpRead::RwGeometryDescriptor,
    summary: &re3bevy_formats::formats::ClumpRead::RwClumpSummary,
    poses: Option<&[Option<GtaFrameMatrix>]>,
    head_animation: Option<&crate::animation::CutsceneMgr::CutsceneHeadAnimationPose>,
) -> Option<re3bevy_formats::formats::ClumpRead::RwDecodedGeometry> {
    let source = geometry_desc.decoded.as_ref()?;
    let skin = geometry_desc.skin.as_ref()?;
    if skin.vertex_bone_indices.len() != source.positions.len()
        || skin.vertex_bone_weights.len() != source.positions.len()
        || skin.skin_to_bone_matrices.len() < skin.num_bones as usize
    {
        return None;
    }

    let hanim_matrices =
        head_animation.map(|animation| build_hanim_hierarchy_matrices(summary, animation));
    let palette: Vec<GtaFrameMatrix> = skin
        .skin_to_bone_matrices
        .iter()
        .enumerate()
        .map(|(bone, inverse_bind)| {
            // librw `HAnimHierarchy::attachByIndex` resolves a palette index
            // through the hierarchy node ID, then finds the frame carrying
            // that ID. Legacy clumps without HAnim metadata retain the direct
            // frame-index fallback.
            let frame_index = summary
                .hanim_node_ids
                .get(bone)
                .and_then(|node_id| {
                    summary
                        .frames
                        .iter()
                        .position(|frame| frame.hanim_id == Some(*node_id))
                })
                .unwrap_or(bone);
            let animated_bone = hanim_matrices
                .as_ref()
                .and_then(|matrices| matrices.get(bone))
                .copied()
                .or_else(|| {
                    atomic_frame_world_matrix_with_poses(summary, frame_index as i32, poses)
                })
                .unwrap_or(GtaFrameMatrix::IDENTITY);
            animated_bone.compose(gta_frame_from_skin_matrix(*inverse_bind))
        })
        .collect();

    let mut deformed = source.clone();
    for vertex in 0..source.positions.len() {
        let source_position = CVector::new(
            source.positions[vertex][0],
            source.positions[vertex][1],
            source.positions[vertex][2],
        );
        let source_normal = source
            .normals
            .get(vertex)
            .map(|normal| CVector::new(normal[0], normal[1], normal[2]));
        let mut position = CVector::ZERO;
        let mut normal = CVector::ZERO;
        let mut total_weight = 0.0;
        for influence in 0..4 {
            let weight = skin.vertex_bone_weights[vertex][influence];
            if weight <= 0.0 {
                continue;
            }
            let bone = skin.vertex_bone_indices[vertex][influence] as usize;
            let Some(matrix) = palette.get(bone).copied() else {
                continue;
            };
            let transformed = matrix.transform_point(source_position);
            position = CVector::new(
                position.x + transformed.x * weight,
                position.y + transformed.y * weight,
                position.z + transformed.z * weight,
            );
            if let Some(source_normal) = source_normal {
                let transformed = matrix.multiply3x3(source_normal);
                normal = CVector::new(
                    normal.x + transformed.x * weight,
                    normal.y + transformed.y * weight,
                    normal.z + transformed.z * weight,
                );
            }
            total_weight += weight;
        }
        // A malformed zero-weight vertex is left in its source pose, matching
        // the practical RW fallback rather than collapsing it to the origin.
        if total_weight > 0.0 {
            deformed.positions[vertex] = [position.x, position.y, position.z];
            if vertex < deformed.normals.len() {
                let inverse_weight = total_weight.recip();
                deformed.normals[vertex] = [
                    normal.x * inverse_weight,
                    normal.y * inverse_weight,
                    normal.z * inverse_weight,
                ];
            }
        }
    }

    Some(deformed)
}

fn gta_frame_from_skin_matrix(matrix: [[f32; 4]; 4]) -> GtaFrameMatrix {
    GtaFrameMatrix {
        right: CVector::new(matrix[0][0], matrix[0][1], matrix[0][2]),
        forward: CVector::new(matrix[1][0], matrix[1][1], matrix[1][2]),
        up: CVector::new(matrix[2][0], matrix[2][1], matrix[2][2]),
        position: CVector::new(matrix[3][0], matrix[3][1], matrix[3][2]),
    }
}

pub(super) fn append_geometry_drawables(
    texture_model_index: usize,
    geometry_desc: &re3bevy_formats::formats::ClumpRead::RwGeometryDescriptor,
    vehicle_colours: VehicleColourRemap,
    frame: Option<GtaFrameMatrix>,
    out: &mut Vec<ModelMaterialDrawable>,
) {
    let Some(geometry) = geometry_desc.decoded.as_ref() else {
        return;
    };
    append_decoded_geometry_drawables(
        texture_model_index,
        geometry_desc,
        geometry,
        vehicle_colours,
        frame,
        out,
    );
}

pub(super) fn append_decoded_geometry_drawables(
    texture_model_index: usize,
    geometry_desc: &re3bevy_formats::formats::ClumpRead::RwGeometryDescriptor,
    geometry: &re3bevy_formats::formats::ClumpRead::RwDecodedGeometry,
    vehicle_colours: VehicleColourRemap,
    frame: Option<GtaFrameMatrix>,
    out: &mut Vec<ModelMaterialDrawable>,
) {
    if geometry.positions.is_empty() || geometry.indices.len() < 3 {
        return;
    }
    let materials = &geometry_desc.materials;
    // Group triangles by material index (re3 one RpMaterial draw per group).
    // This function is also used by the 30 Hz CPU skinning path.  The old
    // BTreeMap-per-triangle implementation made cutscene peds pay O(T log M)
    // plus a tree allocation on every refresh.  RW material indices are
    // compact, so a linearly indexed vector is equivalent and keeps the
    // material order deterministic.
    let tri_count = geometry.indices.len() / 3;
    let max_material = geometry
        .triangle_materials
        .iter()
        .copied()
        .max()
        .unwrap_or(0) as usize;
    let material_count = materials.len().max(max_material.saturating_add(1));
    let mut by_material = (0..material_count)
        .map(|_| Vec::<u32>::new())
        .collect::<Vec<_>>();
    for tri in 0..tri_count {
        let mat = geometry.triangle_materials.get(tri).copied().unwrap_or(0) as usize;
        if mat >= by_material.len() {
            by_material.resize_with(mat + 1, Vec::new);
        }
        let i0 = geometry.indices[tri * 3];
        let i1 = geometry.indices[tri * 3 + 1];
        let i2 = geometry.indices[tri * 3 + 2];
        // Match RenderWare's stream decoder: malformed triangles are dropped
        // before they can produce a screen-filling face in the renderer.
        if [i0, i1, i2]
            .iter()
            .any(|index| (*index as usize) >= geometry.positions.len())
        {
            continue;
        }
        by_material[mat].extend_from_slice(&[i0, i1, i2]);
    }
    if tri_count == 0 || geometry.triangle_materials.is_empty() {
        // No triangle_materials — single mesh with all indices.
        by_material[0] = geometry.indices.clone();
    }
    for (mat_index, tri_indices) in by_material.into_iter().enumerate() {
        if tri_indices.is_empty() {
            continue;
        }
        let material = materials.get(mat_index as usize);
        let Some(mesh) = build_submesh_from_geometry(geometry, &tri_indices, frame) else {
            continue;
        };
        let mut unique = vec![false; geometry.positions.len()];
        for &i in &tri_indices {
            if let Some(entry) = unique.get_mut(i as usize) {
                *entry = true;
            }
        }
        out.push(ModelMaterialDrawable {
            mesh,
            base_color: material_base_color(material, vehicle_colours),
            texture_name: material.and_then(|m| m.texture_name.clone()),
            texture_model_index,
            texture_txd_slot: None,
            vertex_count: unique.into_iter().filter(|used| *used).count(),
            index_count: tri_indices.len(),
            vehicle_colour_slot: material.and_then(|m| m.vehicle_colour_slot()),
            specular: material.map(|m| m.specular).unwrap_or(0.0),
            surface_ambient: material.map(|m| m.ambient).unwrap_or(1.0),
            surface_diffuse: material.map(|m| m.diffuse).unwrap_or(1.0),
            has_prelit: geometry_desc.flags
                & re3bevy_formats::formats::ClumpRead::RW_GEOMETRY_PRELIT
                != 0,
            dynamic_vehicle_component: false,
            vehicle_component_transform: None,
        });
    }
}

fn build_submesh_from_geometry(
    geometry: &re3bevy_formats::formats::ClumpRead::RwDecodedGeometry,
    triangle_indices: &[u32],
    frame: Option<GtaFrameMatrix>,
) -> Option<Mesh> {
    if triangle_indices.len() < 3 {
        return None;
    }
    // Keep index topology intact when malformed source indices are present;
    // dropping individual corners would otherwise re-group the remaining
    // corners into unrelated triangles.
    let valid_indices = triangle_indices
        .chunks_exact(3)
        .filter(|triangle| {
            triangle.iter().all(|index| {
                let Some(position) = geometry.positions.get(*index as usize) else {
                    return false;
                };
                if !position
                    .iter()
                    .all(|value| value.is_finite() && value.abs() <= 100_000.0)
                {
                    return false;
                }
                let local = CVector::new(position[0], position[1], position[2]);
                let transformed = frame
                    .map(|frame| frame.transform_point(local))
                    .unwrap_or(local);
                transformed.x.is_finite()
                    && transformed.y.is_finite()
                    && transformed.z.is_finite()
                    && transformed.x.abs() <= 100_000.0
                    && transformed.y.abs() <= 100_000.0
                    && transformed.z.abs() <= 100_000.0
            })
        })
        .flat_map(|triangle| triangle.iter().copied())
        .collect::<Vec<_>>();
    if valid_indices.len() < 3 {
        return None;
    }
    // Compact used vertices.
    let vertex_count = geometry.positions.len();
    // Vertex indices in RW geometry are dense.  A sentinel array avoids a
    // logarithmic tree lookup for every triangle corner (this is hot for
    // animated cutscenes, where the mesh is rebuilt at GTA's 30 Hz tick).
    let mut remap = vec![u32::MAX; vertex_count];
    let mut positions = Vec::with_capacity(vertex_count.min(triangle_indices.len()));
    let mut normals = Vec::with_capacity(positions.capacity());
    let mut uvs = Vec::with_capacity(positions.capacity());
    let mut colours = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(valid_indices.len());
    for &src in &valid_indices {
        if src as usize >= vertex_count {
            continue;
        }
        let src_index = src as usize;
        let dst = if remap[src_index] != u32::MAX {
            remap[src_index]
        } else {
            let new_index = positions.len() as u32;
            let position = geometry.positions[src_index];
            let local_pos = CVector::new(position[0], position[1], position[2]);
            let gta_pos = frame
                .map(|frame| frame.transform_point(local_pos))
                .unwrap_or(local_pos);
            positions.push(gta_pos.to_bevy().to_array());
            if geometry.normals.len() == vertex_count {
                let n = geometry.normals[src_index];
                let local_normal = CVector::new(n[0], n[1], n[2]);
                let gta_normal = frame
                    .map(|frame| frame.multiply3x3(local_normal))
                    .unwrap_or(local_normal);
                normals.push(gta_normal.to_bevy().normalize_or_zero().to_array());
            } else {
                normals.push([0.0, 1.0, 0.0]);
            }
            if let Some(first_uv_set) = geometry
                .tex_coords
                .first()
                .filter(|set| set.len() == vertex_count)
            {
                let uv = first_uv_set[src_index];
                // GTA III PC DFFs are authored for D3D (V increases downward in
                // texture space, V=0 at top). Bevy/wgpu Image sampling uses the same
                // top-left origin for CPU-uploaded rasters — do **not** invert V
                // (OpenGL-style flip would mirror every world texture).
                uvs.push([uv[0], uv[1]]);
            } else {
                uvs.push([0.0, 0.0]);
            }
            if geometry.prelit_colors.len() == vertex_count {
                colours.push(rgba_u8_to_f32(geometry.prelit_colors[src_index]));
            } else {
                colours.push([1.0, 1.0, 1.0, 1.0]);
            }
            remap[src_index] = new_index;
            new_index
        };
        indices.push(dst);
    }
    build_mesh_from_world_buffers(positions, normals, uvs, colours, indices)
}

fn build_mesh_from_world_buffers(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
) -> Option<Mesh> {
    if positions.is_empty() || indices.len() < 3 {
        return None;
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours),
    )
}
