---
name: re3-audio
description: RustGta3 actor audio owners, one-shot sequences, focused-player engine channels and stereo playback.
---

## Actor and shared state

- VehicleAudioState owns radio/alarm/horn/siren/rain counters. Playback reads core/drive/vitality/appearance/entry and writes audio/concrete state; lighting, siren response and saves use the same owner. Preserve native alarm save format.
- Physical audio identity remains on CPhysical. Allocate it in the audio phase before playback; sound processing need not hold mutable physical pose.
- AutomobileAudioMotion owns per-car gas smoothing/previous forward speed. Audio reads wheels/doors/suspension/damage and writes that owner; HydraulicControl shares its velocity history. Construction consumes it into the bundle; compatible saves preserve transient/default lifetime.
- Boat/heli/plane audio reads physical state; shared rain counters remain in VehicleAudioState. Automobile audio-control residency uses original model COL level, not actor zone membership; missing required COL needs a named wait.

## One-shot processing

- VehicleOneShotAudioState holds native wave/gun/wheel/crunch sequences once per app. Actor creation/retirement and audio-manager restart do not reset them.
- Every supported vehicle kind drains events before its rain tail. Special vehicle speech uses PedTalk and actual LOS/time; native static sequence state belongs to shared Resources, not per-car copies.

## Focused-player engine

- PlayerEngineAudioState owns the focused-player channel and ProcessPlayersVehicleEngine/ProcessCesna statics. NPC engine-off cannot reset it.
- A focused Entity change stops the old channel even if its slot is reused. Player entry resets engine history without resetting Cesna nAccel.
- Dedicated acceleration starts bypass generic priority/release queues as native SampleManager does. Pending starts are frame commands cleared on stop/pause/termination/retirement.
- Channel-used status comes from the live Bevy SFX voice/sink. Do not estimate playback duration or synchronize another active/frequency/timer copy. Required missing backend assets need named counters or rate-limited events.

## Read by changed path

- Jumbo/plane endpoints, SDT loop metadata, stereo routing or voice lifetime: [jumbo-stereo.md](references/jumbo-stereo.md).
- Arena ped stages/bindings: [arena audio](../re3-dev-arena/references/audio.md).
- Vehicle predicate IDs: [re3-vehicles](../re3-vehicles/SKILL.md).
- Original samples/metadata: [gta-assets-formats](../gta-assets-formats/SKILL.md).

Verify actual playback and changed pause/retirement behavior; request queues alone
do not establish working audio.
