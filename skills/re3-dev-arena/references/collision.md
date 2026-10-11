# Arena integration history and collision

- The developer arena uses the same VehicleCollisionState owner on the live
vehicle Entity. Both local-player control paths and uncontrolled integration
write it before integration; pair collision reads it. Arena runtime wrappers
must not retain another saved matrix, and presentation/audio/UI need no
history access. Native construction installs the initial pose with the actor.
- Arena vehicle collision borrows live matrices and authored COL sphere centres,
using a separate arena membership owner backed by core EntitySectorIndex.
Store Entity membership, not world pose snapshots. Changed/removal maintenance
runs outside arena/pause/loading guards; collision refreshes both members after
response and considers newly local candidates. Native CPhysical::GetBoundRect
uses the transformed authored COL sphere centre and radius for moving actors;
CEntity::GetBoundRect separately transforms four authored box corners.
Index all covered sector memberships; no actor maximum-
radius cache is needed. Siren behavior keeps point membership through its thin
AutomobileSpatialIndex domain wrapper over the same core grid. Native source/list
ordering remains a distinct parity requirement; do not claim Entity order is it.
Native automobile body contacts register reciprocal physical collision records
before impulse response. Zero-relative-speed contact must not disappear because
ApplyCollision returns false. Disabled collision actors are expected live flag
filters; do not manufacture damage, rollback or shift to produce a contact record.
Publish the native non-simple vehicle_col_processed marker at narrow-phase entry.
CRect has one canonical definition in math/Rect; entities/Entity and sector
helpers may re-export it, never declare a second rectangle with the same fields.

The shared collision-history and spatial lifecycle rules are in [re3-ecs-runtime](../../re3-ecs-runtime/SKILL.md). Bounds, reciprocal body contacts and CRect ownership above also apply when changing the production interface.
