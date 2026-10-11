# Resident vehicle descriptors and suspension

## Model-owned positions

- VehicleModelPositions owns immutable local LF/LB/RF/RB wheel frames and the distinct right-middle descriptor. GetWheelPosn uses local translations; lamps/exhaust use parent-transformed positions. Middle frames cannot substitute for rear frames.
- Resolve lamp/exhaust hierarchy once when the model becomes resident. Replacement/unload invalidates these descriptors with the clump. Construction, control and presentation borrow the same cache; no per-actor hierarchy reconstruction or independent model-ID cache.
- Exhaust descriptors distinguish Position/Absent/Invalid. An omitted dummy is expected filtering; invalid hierarchy/coordinates need a named failure. Use the descriptor with the actor's current matrix. Compatibility lamp types/accessors may re-export or project the same owner.

## Deferred suspension

- VehicleConstruction owns SetupSuspensionLines and its diagnostic Resource; the vehicle plugin registers it. Query indexed identity/core readonly and AutomobileState mutable. Unrelated vitality/drive/appearance/entry/audio/history cannot gate initialization.
- VehicleConstructionSet::Suspension runs at RenderPrep after NativeRenderPreparationSet::WorldRequests and before mesh refresh/Audit. It also precedes automobile PreRender particles so first-resident simple contacts cannot read default suspension height.
- Order shared production/arena consumers by the explicit domain set; function-based ordering is ambiguous. Renderer compatibility names re-export the same diagnostic owner. Do not duplicate construction registration, restore renderer-owned state writes or add a second origin calculation.
