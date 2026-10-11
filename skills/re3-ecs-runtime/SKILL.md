---
name: re3-ecs-runtime
description: re3bevy runtime state ownership, actor lifecycle, SCM identity, scheduling, camera and spatial indexes.
---

## State ownership

- Bevy owns live actors and relationships as entities/components. Parsed immutable tables and genuinely global managers may be Resources; parsers, pure calculations and shaders remain ordinary code.
- Split components by independent access/lifetime, not one field per component. One owner holds each gameplay value. For example, ped health belongs to PedVitality and vehicle input to VehicleDriveState; HUD, SCM, audio and saves borrow those values.
- Construction DTOs such as CPlayerPed/ScriptPedEntity consume values into bundles. Live query views borrow components; no complete C++ inheritance tree in a Component, synchronized actor copies or end-of-system copy-back.
- Request only required access. Read-only counters, camera and render/save projections must not acquire an all-actor mutable facade.
- One canonical definition owns time/coordinate types; compatibility paths may re-export it. Convert GTA coordinates at explicit boundaries, never introduce another clock/vector owner.
- Use existing ECS/sector/spatial queries in dense scenes. Avoid per-frame whole-world snapshots and flat all-pairs scans.

## Identity and scheduling

- SCM handles resolve through the canonical kind/generation-to-Entity index, not actor scans. Equal numeric ped/vehicle handles are distinct; carry pool kind or Entity across camera and other interfaces.
- Preserve same-pass opcode mutation visibility, deferred spawn barriers, native side-effect order and each behavior's cadence. Do not replace all processing with a blanket fixed-step loop.
- Domain plugins register their systems; the lifecycle plugin composes them. Use shared process sets and explicit cross-domain dependencies.
- Camera presentation reads materialized target poses and players after gameplay barriers. The camera domain owns Pad/Camera stages; no SCM actor snapshot or mutable gameplay store for position reads.

## Derived data and lifecycle

- VehicleCollisionState/ObjectCollisionState capture pose immediately before integration. Collision borrows that transient history; passenger processing must not overwrite the driver's saved pose.
- Spatial indexes store Entity membership and borrow live state. Maintain membership from ECS changes/removals. Removal maintenance continues through loading/restart/pause so buffered removals cannot expire behind gameplay guards; response cadence is separate.
- Parsing, material resolution, drawable construction, previews, diagnostics and mesh submission have distinct owners. Relocating code alone does not correct lifetime or ownership.
- Required dependency/lifecycle failures need a named counter or rate-limited event; expected filtering needs no warning.

Read the affected domain skill from [AGENTS.md](../../AGENTS.md) when changing its
owners/interfaces. For authored bounds, sector coverage, reciprocal body contacts
or CRect, read [collision.md](../re3-dev-arena/references/collision.md); those
shared rules also apply to production.
