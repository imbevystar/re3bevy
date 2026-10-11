//! Named construction failures persist independently of spawned actor lifetime.
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ArenaVehicleInput {
    ModelLoader, ModelInfo, HandlingData, RandomState, SurfaceData,
    HandlingIdentity, HandlingRecord, ModelClump, ModelCollision, Suspension,
}

impl ArenaVehicleInput {
    pub fn counter_name(self) -> &'static str {
        match self {
            Self::ModelLoader => "arena_vehicle_missing_model_loader",
            Self::ModelInfo => "arena_vehicle_missing_model_info",
            Self::HandlingData => "arena_vehicle_invalid_or_missing_handling_data",
            Self::RandomState => "arena_vehicle_missing_random_state",
            Self::SurfaceData => "arena_vehicle_invalid_or_missing_surface_data",
            Self::HandlingIdentity => "arena_vehicle_unknown_handling_identity",
            Self::HandlingRecord => "arena_vehicle_missing_handling_record",
            Self::ModelClump => "arena_vehicle_missing_model_clump",
            Self::ModelCollision => "arena_vehicle_missing_model_collision",
            Self::Suspension => "arena_vehicle_missing_authored_suspension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArenaVehicleBuildError {
    pub input: ArenaVehicleInput,
    pub detail: String,
}

impl ArenaVehicleBuildError {
    pub fn new(input: ArenaVehicleInput, detail: impl Into<String>) -> Self {
        Self { input, detail: detail.into() }
    }
}

impl std::fmt::Display for ArenaVehicleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.input.counter_name(), self.detail)
    }
}

#[derive(Default, Resource)]
pub(crate) struct ArenaVehicleSpawnDiagnostics {
    counts: BTreeMap<ArenaVehicleInput, u64>,
}

impl ArenaVehicleSpawnDiagnostics {
    pub fn record(&mut self, model_index: usize, error: &ArenaVehicleBuildError) {
        let count = self.counts.entry(error.input).or_default();
        *count = count.saturating_add(1);
        if count.is_power_of_two() {
            warn!(model_index, counter = error.input.counter_name(), count = *count,
                detail = %error.detail, "arena_vehicle_construction_failed");
        }
    }
}

pub(crate) enum ArenaVehicleRuntimeOutcome {
    Ready,
    UnsupportedKind,
    Failed(ArenaVehicleBuildError),
}

pub(crate) struct ArenaVehicleSpawnResult {
    pub entity: Option<Entity>,
    pub visual_loaded: bool,
    pub runtime: ArenaVehicleRuntimeOutcome,
}

impl ArenaVehicleSpawnResult {
    pub fn runtime_ready(&self) -> bool { matches!(self.runtime, ArenaVehicleRuntimeOutcome::Ready) }
}
