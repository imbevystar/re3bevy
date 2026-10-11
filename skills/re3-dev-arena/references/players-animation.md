# Primary and co-op actors

- ArenaCoopPlayerTwo owns only co-op fixture input/camera/transition state.
Its construction player is consumed into canonical ped/player components on
the same Entity. ArenaCoopActor queries borrow those owners; animation renders
the same PedAnimation clump. State-only camera/audio-focus readers must not
acquire mutable ped state. Disabling co-op retires the complete actor; later
recreation installs a fresh bundle, not synchronized copies in the fixture.
DeveloperArena retains only the primary player's Entity. The visible ArenaPlayer
Entity owns its consumed canonical ped/player bundle; control, pose, ped audio,
UI and co-op construction resolve that identity through component queries.
A co-op construction template may clone a readonly source once and consume it
immediately; no live CPlayerPed may remain in a Resource. Close retires the
complete scene actor and reopen creates a fresh identity/default transient state.
Primary and coop mutable actor queries must be explicitly disjoint.

## Animation advance and pose presentation

- Arena primary animation advance and pose presentation are distinct stages.
- Advance reads common ped/player association state and writes PedAnimation only;
both player pose paths read animation/association state without mutable gameplay
or unrelated vitality/locomotion joins. Preserve existing update/presentation
ordering. UI animation readouts are projections, never another live clump.
Failed required mesh/association binding remains observable and cannot report
successful pose submission merely because an input mesh was consumed.
ArenaCharacterPoseManifest owns original drawable order/frame indices on the
resident visual Entity. Construct it with the original drawables, discard it
with visual replacement/despawn, and query member Entities directly for pose.
Do not rescan all actors' parts or rebuild sentinel frame lists each pose.
Validate live generation, ChildOf, source index and canonical mesh before
applying a pose. Named original TXD textures must resolve before construction;
native '@' vehicle paint remaps and genuinely untextured materials retain their
authored colour behavior.
