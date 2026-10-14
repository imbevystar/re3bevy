---
name: re3-dev-arena
description: RustGta3 isolated developer arena and co-op actor fixtures, construction, stage access, presentation and collision.
---

# Arena ownership

The arena uses production component owners and borrowed queries. Load only the
reference for the fixture or stage being changed:

- Spawn DTOs, required original inputs, SURFACE.DAT validation or persistent arena RNG: [references/construction.md](references/construction.md).
- Primary/co-op identity, state, creation/retirement, animation advance or pose submission: [references/players-animation.md](references/players-animation.md).
- Ped audio stages, physical audio identity, binding ownership or retirement: [references/audio.md](references/audio.md).
- Wheel/skid/exhaust PreRender effects, canonical timer and required weather inputs: [references/vehicle-effects.md](references/vehicle-effects.md).
- Integration history, spatial membership, authored bounds or body contacts: [references/collision.md](references/collision.md).

Read the production domain skill only when its owners/interfaces are involved:
[re3-peds](../re3-peds/SKILL.md), [re3-vehicles](../re3-vehicles/SKILL.md),
[re3-audio](../re3-audio/SKILL.md), or
[re3-particles-glass](../re3-particles-glass/SKILL.md).
