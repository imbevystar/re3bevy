---
name: re3-fire
description: rust-gta-3 native fire manager slots, SCM attachments, flamethrower damage and actor fire lifecycle/origins.
---

# Fire ownership and lifecycle

- `CFireManager` retains the 40 native SCM slots as a Resource. Fire control and
  flamethrower volumes borrow CPed/PedVitality and readonly player protection;
  vehicle fire control reads VehicleCore and writes VehicleVitality. SCM engine
  damage also borrows AutomobileState. Ordinary materialized actor fires attach
  to Entity; SCM handles resolve through the canonical kind/generation index.
  GtaFirePlugin owns registration. SCM attachments follow dynamic actor creation
  in the Script set so deferred construction completes before owner queries.
  Weak source/target retirement runs without gameplay/pause guards and preserves
  last fire position and SCM reservations. Resurrection also clears canonical
  fire links to the living player/previous vehicle; extinguishing the player's
  fire preserves a script fire's reservation. Use resident authored headlights
  for automobile flame origins; invalid/missing data cannot become a fake origin.

When changing automobile origins, read the model residency and presentation references selected by [re3-vehicles](../re3-vehicles/SKILL.md). Fire damage and PreRender engine steam/smoke are distinct phases.
