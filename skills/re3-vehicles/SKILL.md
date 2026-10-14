---
name: re3-vehicles
description: rust-gta-3 vehicle ownership, entry/control, suspension construction and native presentation.
---

## Component owners

- Construction values are consumed into identity/core/vitality/drive/appearance/entry/audio/concrete/collision-history components. VehicleRef/VehicleMut and concrete views borrow them; an owned construction enum is not a live actor Component.
- VehicleDriveState owns steering/throttle/brake, gear/input and engine/handbrake/comedy flags. Control/SCM/render/audio/saves borrow it; no copies in core/common flags. Pose-only joins need no input access.
- VehicleAppearance owns colours, selected DFF extras and light/fade/damaged-render flags. Lighting writes it using core/drive/damage inputs; render/audio/saves read the same state.
- VehicleEntryState owns lock/capacity and transient entry/exit/carjack reservations. InVehicle owns occupants and seat uniqueness; no synchronized driver/passenger list or reservation copy in core.
- Presentation/save projections read immutable state. Audio reads drive/vitality and writes persistent audio/concrete audio state. Consuming a DTO does not justify leaving unrelated mutable state coupled.

## Identity boundary

Gameplay predicates use original model IDs; audio predicates use indices relative
to MI_LANDSTAL. Preserve the conversion and distinct native behavior rather than
sharing untyped integers.

## Load only affected details

- Suspension initialization, DFF wheel/lamp/exhaust descriptors, residency/replacement or construction order: [model-construction.md](references/model-construction.md).
- Lighting, wheel/skid/engine/exhaust, PreRender or shallow-water emission: [presentation.md](references/presentation.md). Add model-construction only if descriptors/residency are involved.
- Audio or HydraulicControl shared motion: [re3-audio](../re3-audio/SKILL.md).
- Particle emission: [re3-particles-glass](../re3-particles-glass/SKILL.md).
- Arena fixtures: relevant references in [re3-dev-arena](../re3-dev-arena/SKILL.md).
- Collision history/spatial lifecycle: [re3-ecs-runtime](../re3-ecs-runtime/SKILL.md). Authored bounds/body contacts, including production: [collision.md](../re3-dev-arena/references/collision.md).
