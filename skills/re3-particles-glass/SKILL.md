---
name: re3-particles-glass
description: RustGta3 particle cadence, producer stages, admission/capacity and falling-glass lifetime/submission.
---

## Creation policy and stages

- ParticleCreationPolicy owns the native PC interval for app lifetime. The eight smoke types share Particle.rs's predicate with sprite screen-size culling; preserve the bitmask, not modulo or an exhaust-only half rate.
- Canonical CTimer supplies inputs. Pause/mask admission precedes capacity/ID allocation. Isolated simulation needs an explicit boundary; do not suppress production code-pause semantics.
- ParticleCreationInputPlugin registers refreshes after Script/GameLogic/Audio and before downstream producers, inside GameProcess so suspended production cannot overwrite arena inputs.
- A same-function timer/policy writer updates its borrowed creation context before emission; frame-start capture alone is insufficient.

## Admission and capacity

- Capacity counts derived live Entity membership plus pending requests. Record allocated Entities before deferred materialization; unguarded Added/Removed maintenance releases membership. Do not mirror C++ active-slot booleans or reset IDs/quota while actors remain.
- ParticleEmitter borrows original configuration and the emitting domain's canonical camera. Native AddParticle range admission precedes pending request/logical ID reservation. Zero authored range disables the gate; equality is admitted.
- Do not copy configuration into CParticle or add a persistent camera owner. Already admitted requests cannot be reclassified against a later render camera.

## Automobile and arena producers

- Wheel/exhaust helpers use the borrowed emitter. Wheel registration reads canonical identity/index and VehicleCore, writing only AutomobileState; no ScriptEntityWorld or unrelated vitality/drive/audio/history access.
- Arena effects use the active primary root-camera Transform at the existing stage, not a frozen production camera or any-camera admission.
- For PreRender order and wheel/lamp/exhaust data, read the affected references in [re3-vehicles](../re3-vehicles/SKILL.md).
- For falling panes, fragment sequences, CGlass reset or submission, read [glass.md](references/glass.md). Particle-only work does not need it.
