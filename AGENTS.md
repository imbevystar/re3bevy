# re3bevy

Reproduce GTA III behavior in idiomatic Rust/Bevy ECS. Reference C++ defines
behavior; Bevy owns live state and lifecycle. Do not translate C++ architecture
or maintain a second gameplay world.

## Always apply

- Preserve existing work. No destructive Git commands. `references/re-GTA/` is read-only and must not be embedded in the binary.
- Change Cargo manifests, lockfile or build/profile settings only when explicitly requested.
- No new docs, reports, task histories, permanent probes, scratch scripts or standalone test files unless requested; do not recreate `docs/`. Keep README to essential introduction/setup/use and instructions to maintained rules. Report findings in conversation.
- Temporary logs/captures/scratch belong in a task-specific OS temp directory; remove them after inspection. Share default `target/` (`debug`, `release`, `runtime`, `profiling`; tests share it). No `CARGO_TARGET_DIR`/`--target-dir` overrides unless isolation is requested.
- Preserve required configuration, test inputs, dependency metadata and third-party licenses; file extensions alone do not justify deletion.
- Publication rights are separate from behavior fidelity. Preserve source attribution; do not relabel reference-derived code as original or clean-room, or assume rewriting in Rust supplies redistribution rights. Follow [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); do not claim the complete project is cleared while provenance is unresolved.
- Keep original game data, reference source trees, local configuration, private asset-containing APKs and generated outputs out of public distributions. Android builds default to importing user-provided data; bundling it requires the explicit private-build option. Run `python scripts/check-source-distribution.py` before preparing source for publication; this checks files, not legal rights.

## Read only what the task needs

Read repository `SKILL.md` files directly. For source changes, start with
[re3-port-workflow](skills/re3-port-workflow/SKILL.md), which owns project direction,
scope and verification. Instruction-only edits need coverage/link checks, not a game run.
Add affected domains below; follow reference links only when their stated trigger
applies. Load another domain when its owner/interface becomes relevant, not all skills up front.

| Task | Skill |
| --- | --- |
| Runtime ownership, lifecycle, scheduling, SCM identity, camera, spatial indexes; weapon/projectile actor interfaces | [re3-ecs-runtime](skills/re3-ecs-runtime/SKILL.md) |
| Original install, parsers, assets, streaming, deployment | [gta-assets-formats](skills/gta-assets-formats/SKILL.md) |
| Shaders, materials, visibility, render order | [wesl-rendering](skills/wesl-rendering/SKILL.md) |
| Ped/player behavior, animation, locomotion, damage, relationships | [re3-peds](skills/re3-peds/SKILL.md) |
| Vehicles, construction, entry, control, suspension, effects | [re3-vehicles](skills/re3-vehicles/SKILL.md) |
| Actor audio, playback and audio state | [re3-audio](skills/re3-audio/SKILL.md) |
| Developer arena and co-op fixtures | [re3-dev-arena](skills/re3-dev-arena/SKILL.md) |
| Particle producers/admission/cadence, glass | [re3-particles-glass](skills/re3-particles-glass/SKILL.md) |
| Fire, flamethrower, fire attachment/retirement | [re3-fire](skills/re3-fire/SKILL.md) |
