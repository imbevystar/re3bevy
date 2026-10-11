---
name: re3-port-workflow
description: re3bevy source changes, behavior investigation, bounded cleanup and production verification.
---

## Project direction

- Current deliverable: faithful GTA III behavior rebuilt in Rust/Bevy ECS. C++ is evidence for decisions, timing and side effects, not a class/module blueprint. Preserve required native cadence and ordering without rebuilding global pools or world snapshots.
- Current deployment: executable/dist runs with an original GTA III installation, including when placed in its directory. Keep installation loading separate from game behavior; no mandatory offline asset conversion or checkout dependency.
- Preserve provenance and third-party notices when using source references. Behavior parity is not publication clearance; translated code and tables need their own redistribution basis. Do not remove reference comments to disguise origin or assert clean-room development. Publication scope is in [LICENSE](../../LICENSE) and [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
- Longer-term direction: Vice City, then San Andreas on the shared base, eventually playable in one executable. Keep game-specific rules/data identifiable; extract shared behavior when actual use supports it. Do not invent future-game systems during a GTA III task.
- Later, reuse the import/parsing code in a separate asset-converter executable producing organized Bevy-friendly content. The eventual original game should reuse the systems with new assets, handling and story while retaining the classic GTA feel. This direction does not authorize implementing those stages now or force every asset into one format.

## Work on the requested behavior

- Trace the relevant C++ declarations, implementations and call sites under `references/re-GTA/src`, then the Rust production path. Do not guess constants, coordinate conversions, timing, state transitions or failure behavior.
- Separate GTA decisions from parsing, ECS scheduling, physics and rendering. Extend the existing owner; avoid parallel implementations, placeholder gameplay, no-ops and silent substitutes.
- For bugs, missing behavior and ports, read [behavior-parity.md](references/behavior-parity.md): follow the complete affected path and interfaces, fix related causes, and keep unrelated discoveries outside the change.
- For source cleanup or refactoring, read [code-quality.md](references/code-quality.md). Cleanup is bounded by the task, not permission to rewrite everything encountered.
- Select domain skills through [AGENTS.md](../../AGENTS.md). For runtime state, scheduling or lifecycle changes, also read [re3-ecs-runtime](../re3-ecs-runtime/SKILL.md); for original data/loading/deployment, read [gta-assets-formats](../gta-assets-formats/SKILL.md).

## Verification and completion

- Validate after a coherent implementation; use an interim check when it resolves a blocker or meaningful uncertainty. Do not build, launch or add a probe after every small edit.
- Check affected crates and run relevant existing tests/parity cases. Add lasting regression coverage only for a meaningful behavioral risk, using existing infrastructure. A blanket nextest/workspace run is not mandatory for every change.
- Game behavior changes require production-game observation or runtime evidence for the affected path. Compilation and isolated tests alone do not establish runtime correctness. If runtime verification is blocked, state the blocker and the unverified behavior; do not claim full completion.
- Follow [AGENTS.md](../../AGENTS.md) for shared build cache and temporary outputs. Instruction-only changes need coverage/link checks; no game launch.
- Report the change, relevant evidence and material remaining issues briefly in conversation. Do not turn task findings into new documentation or skill history.
