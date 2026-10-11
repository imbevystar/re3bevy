---
name: re3-peds
description: re3bevy ped/player component ownership, animation, vitality, locomotion and obstacle relationships.
---

## Shared ped/player state

- PedAnimation owns the locomotion clump. Gameplay root extraction, attack associations and rendered poses borrow it; construction consumes animation into the bundle. No second clump in CPed/player state.
- PedVitality owns health, armour, fire and bleeding. Player/NPC damage, SCM, HUD and saves borrow it; no synchronized fields in player/common state.
- PedLocomotion owns root displacement and heading. Player, NPC, SCM, entry/exit and pose calculations borrow it.

## Locomotion and obstacle changes

- Walk-around reads the canonical destination heading. Preserve vector Heading() for obstacle classification versus GetRadianAngleBetweenPoints for CPed::Seek; substituting them changes axis-aligned tie decisions.
- PedWalkAroundObstacle is the canonical Bevy relationship. Reverse membership is derived; retiring a target clears the reference without despawning the ped. Do not restore a copied obstacle Entity in CPed.
- Relationship cleanup runs through pause/loading. CReferences::RemoveReferencesToPlayer also clears living targets; resurrection clears canonical relationships on the player and previous vehicle, not just compatibility tables.
- Native m_collidingEntityWhileFleeing shares this obstacle identity; m_pCollidingEntity is a distinct field. New flee readers borrow the shared relationship.
- PedObstacleAvoidance owns collision timer and walk-around type for actor lifetime. Corner/evasion/recovery writers query it; removing an obstacle preserves this history. New bundles initialize default transient history.

## Affected interfaces

Read [re3-fire](../re3-fire/SKILL.md) for fire control,
[re3-audio](../re3-audio/SKILL.md) for audio, or
[re3-dev-arena](../re3-dev-arena/SKILL.md) for arena fixtures only when involved.
For changed state ownership, include lifecycle, SCM, HUD/save and presentation
consumers that use that state; do not verify only the writer.
