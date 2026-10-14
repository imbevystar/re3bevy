//! Model-dependent automobile construction over resident original input owners.
use crate::{
    control::ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity},
    core::FileLoader::CFileLoader,
    vehicles::HandlingMgr::cHandlingDataMgr,
};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum VehicleConstructionSet {
    Suspension,
}

/// Deferred CAutomobile::SetupSuspensionLines dependencies and successful initializations.
#[derive(Debug, Default, Resource)]
pub struct GtaSuspensionDiagnostics {
    pub missing_file_loader_frames: u64,
    pub missing_handling_manager_frames: u64,
    pub waiting_model_frames: u64,
    pub waiting_clump_frames: u64,
    pub missing_handling_identity_frames: u64,
    pub missing_handling_data_frames: u64,
    pub invalid_wheel_frame_frames: u64,
    pub initialized_vehicles: u64,
    pub invalidated_bindings: u64,
    pub missing_entity_index_frames: u64,
}

/// Complete the model-dependent half of `CAutomobile` construction after
/// streaming has made the vehicle DFF resident.  re3 calls
/// `SetupSuspensionLines()` from the constructor; the ECS constructor cannot
/// see the asynchronously loaded clump, so this RenderPrep boundary performs
/// the same one-time initialization before wheel meshes are built.
pub(crate) fn initialise_automobile_suspension(
    file_loader: Option<Res<CFileLoader>>,
    handling_manager: Option<Res<cHandlingDataMgr>>,
    index: Res<ScriptEntityIndex>,
    mut vehicles: Query<(
        Entity,
        &ScriptVehicleIdentity,
        &crate::vehicles::Vehicle::VehicleCore,
        &mut crate::vehicles::Automobile::AutomobileState,
    )>,
    mut diagnostics: ResMut<GtaSuspensionDiagnostics>,
) {
    let Some(file_loader) = file_loader else {
        diagnostics.missing_file_loader_frames += 1;
        return;
    };
    let Some(handling_manager) = handling_manager else {
        diagnostics.missing_handling_manager_frames += 1;
        return;
    };
    for (entity, identity, core, mut state) in &mut vehicles {
        if index.vehicle(identity.handle) != Some(entity) {
            continue;
        }
        let Ok(model_index) = usize::try_from(core.physical.entity.model_index) else {
            state.invalidate_suspension_source();
            diagnostics.waiting_model_frames += 1;
            continue;
        };
        let Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(model)) =
            file_loader.model_info.get_model_info(model_index)
        else {
            state.invalidate_suspension_source();
            diagnostics.waiting_model_frames += 1;
            continue;
        };
        let Some(descriptors) = model.authored_positions() else {
            state.invalidate_suspension_source();
            diagnostics.waiting_clump_frames += 1;
            continue;
        };
        let Some(handling_id) = core.handling_id.and_then(|id| usize::try_from(id).ok()) else {
            state.invalidate_suspension_source();
            diagnostics.missing_handling_identity_frames += 1;
            continue;
        };
        let Some(handling) = handling_manager.get_handling_data(handling_id) else {
            state.invalidate_suspension_source();
            diagnostics.missing_handling_data_frames += 1;
            continue;
        };
        let Some(positions) = descriptors.wheels.local_positions() else {
            state.invalidate_suspension_source();
            diagnostics.invalid_wheel_frame_frames += 1;
            continue;
        };
        let source = super::Automobile::AutomobileSuspensionSource {
            model_index: core.physical.entity.model_index,
            revision: model.clump.base.rw_object_revision(),
            handling_id,
        };
        if state.suspension_initialized && state.suspension_source == Some(source) {
            continue;
        }
        state.setup_suspension_lines(positions, model.wheel_scale, handling);
        state.suspension_source = Some(source);
        diagnostics.initialized_vehicles += 1;
    }
}

/// Descriptor retirement is lifecycle maintenance and continues while gameplay is paused/loading.
pub(crate) fn invalidate_automobile_suspension_sources(
    loader: Option<Res<CFileLoader>>,
    index: Option<Res<ScriptEntityIndex>>,
    mut vehicles: Query<(
        Entity,
        &ScriptVehicleIdentity,
        &crate::vehicles::Vehicle::VehicleCore,
        &mut crate::vehicles::Automobile::AutomobileState,
    )>,
    mut diagnostics: ResMut<GtaSuspensionDiagnostics>,
) {
    let Some(index) = index else {
        if !vehicles.is_empty() {
            diagnostics.missing_entity_index_frames += 1;
        }
        return;
    };
    for (entity, identity, core, mut state) in &mut vehicles {
        if index.vehicle(identity.handle) != Some(entity) {
            continue;
        }
        let Some(source) = state.suspension_source else {
            continue;
        };
        let current_revision = loader
            .as_deref()
            .and_then(|loader| {
                usize::try_from(core.physical.entity.model_index)
                    .ok()
                    .and_then(|id| loader.model_info.get_model_info(id))
            })
            .and_then(|slot| match slot {
                crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(model)
                    if model.authored_positions().is_some() =>
                {
                    Some(model.clump.base.rw_object_revision())
                }
                _ => None,
            });
        if source.model_index != core.physical.entity.model_index
            || current_revision != Some(source.revision)
            || core.handling_id.and_then(|id| usize::try_from(id).ok()) != Some(source.handling_id)
        {
            state.invalidate_suspension_source();
            diagnostics.invalidated_bindings += 1;
        }
    }
}
