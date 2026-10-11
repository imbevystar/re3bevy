---
name: wesl-rendering
description: re3bevy WESL shaders, bindings, materials, visibility and RenderWare draw behavior.
---

## Rendering contract

- New project shaders use WESL. Register them through `renderer/ShaderAssets.rs` as Bevy shader assets with stable handles and relative virtual module paths; embed project-owned program sources so deployment needs no checkout shader folder.
- GPU code consumes resolved original textures and runtime inputs. For original data, loading or deployment, read [gta-assets-formats](../gta-assets-formats/SKILL.md); it owns provenance and installation rules.
- Trace relevant re3 draw paths/call sites for draw order, visibility, alpha, depth, fog, texture, material and culling decisions. Use Bevy rendering where it preserves those decisions; adapt at the affected boundary where its defaults differ.
- Rust uniform/binding layouts match WESL exactly. Runtime values belong in uniforms, not shader specialization.
- ECS/asset owners supply draw inputs and lifecycle; shaders must not replace game loading or gameplay state.

## Read by affected draw path

- Vehicle lamps/wheels/exhaust: relevant references in [re3-vehicles](../re3-vehicles/SKILL.md).
- Particles or glass: [re3-particles-glass](../re3-particles-glass/SKILL.md).
- Runtime ownership, camera or submission lifecycle: [re3-ecs-runtime](../re3-ecs-runtime/SKILL.md).

Check affected shader modules, bindings and pipeline paths. Verify changed visual
behavior in the production game; a screenshot alone does not establish parity.
