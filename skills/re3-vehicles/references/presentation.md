# Lighting and native automobile effects

- Vehicle lighting reads canonical damage state and authored DFF lamp positions;
  it must not acquire unrelated actor vitality or collision history. Validate
  identity through the canonical index before processing a live ECS entity that
  may already be retired. Missing weather, handling or authored lamp inputs must
  remain observable rather than becoming fabricated clear weather or geometry.
  Resolve invariant DFF descriptors when the parsed model becomes resident;
  Damage origins read that same resident headlight provenance. An absent dummy
  preserves CVehicleModelInfo's native zero-initialized position; invalid present
  data or missing resident/model state requires a named failure/wait, not zero.
  Engine and fire origins retain their native bonnet/BFINJECT/forward-first-
  person/firetruck distinctions. Missing presentation inputs cannot suppress
  independent health/fire timer, explosion/reward or audio processing.
  Damaged-engine steam/smoke belongs to native automobile PreRender, not the World
  fire/damage lifecycle. Vehicle registration emits wheel, engine then exhaust per actor
  after BeforeRender inputs and before NativeParticleRenderSet::Consume, using
  readonly core/vitality/drive owners and typed focused InVehicle. Only wheel/skid
  history requires mutable concrete state. Skidmark ageing stays in the earlier
  Skidmarks manager phase; actor registration belongs to automobile PreRender.
  Shallow SURFACE_WATER contacts belong to ProcessBuoyancy's no-body-buoyancy
  branch, unlike PreRender wet-road spray. Player and NPC buoyancy callers borrow
  the current camera/config there; do not merge the two effect phases.
- Simple PreRender contacts use original local XY and 1.5 times canonical actor
wheel height. Middle feedback uses rear contact plus forward * 2, the same
rear blood/mud history and native slots 5/6, outside the primary status gate.
Its gate respects native ADD_WHEEL retirement when wheel_id is -1. Missing or
invalid required frames must remain observable and cannot count as successful
suspension initialization. Do not reintroduce per-actor hierarchy scans/cache.
- Production exhaust presentation queries identity/core/drive readonly and
validates the live handle/Entity through ScriptEntityIndex. It must not request
ScriptEntityWorld or mutable vehicle/concrete owners to emit particles.
