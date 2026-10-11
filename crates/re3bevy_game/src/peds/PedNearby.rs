//! CPed::BuildPedLists over canonical live actors and primary sector membership.
use crate::{
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
    core::{EntitySectorIndex::EntitySectorIndex, FileLoader::CFileLoader},
    peds::{
        Ped::CPed,
        PlayerPed::{CWorldPlayerState, PlayerPedState},
        VehicleRelations::InVehicle,
    },
};
use bevy::{ecs::query::QueryData, prelude::*};
use re3bevy_core::core::{Coordinates::CVector, Timer::CTimer};

const NEAR_PED_CAPACITY: usize = 10;
const TEMP_PED_CAPACITY: usize = 49; // gapTempPedList[50] reserves its null terminator.
const PED_LIST_SCAN_HALF_EXTENT: f32 = 20.0;
const PED_LIST_PRUNE_DISTANCE_SQUARED: f32 = 900.0;

#[derive(Debug, Clone, Copy, Resource)]
pub struct PedThreatReactionRange {
    pub multiplier: u16,
}
impl Default for PedThreatReactionRange {
    fn default() -> Self {
        Self { multiplier: 1 }
    }
}

#[derive(Default, Resource)]
pub struct PedNearbyDiagnostics {
    pub missing_original_bounds: u64,
    pub invalid_sector_queries: u64,
    pub rebuilds: u64,
    pub candidate_visits: u64,
    pub pruned_references: u64,
    pub missing_identity_resources: u64,
}

#[derive(Default, Resource)]
pub(crate) struct PedNearbySpatial {
    grid: EntitySectorIndex,
}

fn bound_centre(ped: &CPed, loader: &CFileLoader) -> Option<CVector> {
    let model = usize::try_from(ped.physical.entity.model_index).ok()?;
    let col = loader.model_info.col_model(model)?;
    let centre = col.bounding_sphere.center;
    Some(
        ped.physical
            .entity
            .matrix
            .transform_point(CVector::new(centre.x, centre.y, centre.z)),
    )
}

/// Membership owns identities only. Removal readers run during pause/loading;
/// control and native 16-frame list decisions have their own gameplay guards.
pub(crate) fn sync_near_ped_spatial(
    mut spatial: ResMut<PedNearbySpatial>,
    mut diagnostics: ResMut<PedNearbyDiagnostics>,
    loader: Option<Res<CFileLoader>>,
    changed: Query<
        (Entity, &CPed),
        (
            Or<(With<ScriptPedIdentity>, With<PlayerPedState>)>,
            Or<(
                Changed<CPed>,
                Added<ScriptPedIdentity>,
                Added<PlayerPedState>,
            )>,
        ),
    >,
    mut removed_peds: RemovedComponents<CPed>,
    mut removed_identities: RemovedComponents<ScriptPedIdentity>,
    mut removed_players: RemovedComponents<PlayerPedState>,
) {
    for entity in removed_peds
        .read()
        .chain(removed_identities.read())
        .chain(removed_players.read())
    {
        spatial.grid.remove(entity);
    }
    for (entity, ped) in &changed {
        let centre = loader
            .as_deref()
            .and_then(|loader| bound_centre(ped, loader));
        if !centre.is_some_and(|centre| spatial.grid.update(entity, centre)) {
            spatial.grid.remove(entity);
            diagnostics.missing_original_bounds += 1;
            warn_once!("near_ped_lists_waiting_original_col_bounds");
        }
    }
}

#[derive(QueryData)]
pub(crate) struct NearPedRead {
    ped: &'static CPed,
    identity: Option<&'static ScriptPedIdentity>,
    player: Option<&'static PlayerPedState>,
    vehicle: Option<&'static InVehicle>,
}

fn live_handle(
    entity: Entity,
    actor: &NearPedReadItem<'_, '_>,
    index: &ScriptEntityIndex,
    players: &CWorldPlayerState,
) -> Option<i32> {
    if let Some(player) = actor.player {
        return (players.entities.contains(&Some(entity)) && player.pool_handle != 0)
            .then_some(player.pool_handle);
    }
    let handle = actor.identity?.handle;
    (index.ped(handle) == Some(entity)).then_some(handle)
}

fn resolve_near_ped(
    handle: i32,
    index: &ScriptEntityIndex,
    players: &CWorldPlayerState,
    actors: &Query<NearPedRead>,
) -> Option<Entity> {
    if let Some(entity) = index.ped(handle) {
        return actors
            .get(entity)
            .ok()
            .filter(|actor| live_handle(entity, actor, index, players) == Some(handle))
            .map(|_| entity);
    }
    players.entities.iter().flatten().copied().find(|&entity| {
        actors
            .get(entity)
            .ok()
            .is_some_and(|actor| live_handle(entity, &actor, index, players) == Some(handle))
    })
}

#[derive(Clone, Copy)]
pub(crate) struct NearPedCandidate {
    handle: i32,
    distance: f32,
}

/// CPed::SortPeds' pivot/exchange decisions, including equal-distance exchanges.
fn sort_candidates(candidates: &mut [NearPedCandidate]) {
    if candidates.len() <= 1 {
        return;
    }
    let middle = candidates[(candidates.len() - 1) / 2].distance;
    let mut right = 0_isize;
    let mut left = candidates.len() as isize - 1;
    while right <= left {
        while candidates[right as usize].distance < middle {
            right += 1;
        }
        while candidates[left as usize].distance > middle {
            left -= 1;
        }
        if right <= left {
            candidates.swap(right as usize, left as usize);
            right += 1;
            left -= 1;
        }
    }
    if left >= 0 {
        sort_candidates(&mut candidates[..=left as usize]);
    }
    if (right as usize) < candidates.len() {
        sort_candidates(&mut candidates[right as usize..]);
    }
}

pub(crate) fn process_near_ped_lists(
    identities: Query<Entity, Or<(With<ScriptPedIdentity>, With<PlayerPedState>)>>,
    mut actors: ParamSet<(Query<NearPedRead>, Query<&mut CPed>)>,
    spatial: Res<PedNearbySpatial>,
    index: Res<ScriptEntityIndex>,
    players: Res<CWorldPlayerState>,
    loader: Res<CFileLoader>,
    timer: Res<CTimer>,
    threat_range: Res<PedThreatReactionRange>,
    mut diagnostics: ResMut<PedNearbyDiagnostics>,
    mut candidate_entities: Local<Vec<Entity>>,
    mut candidates: Local<Vec<NearPedCandidate>>,
) {
    let maximum_distance = f32::from(threat_range.multiplier) * 30.0;
    for entity in &identities {
        let mut handles = [None; NEAR_PED_CAPACITY];
        let count;
        {
            let read = actors.p0();
            let Ok(actor) = read.get(entity) else {
                continue;
            };
            if live_handle(entity, &actor, &index, &players).is_none() {
                continue;
            }
            let position = actor.ped.physical.entity.get_position();
            let rebuild = timer
                .frame_counter()
                .wrapping_add(u32::from(actor.ped.physical.entity.random_seed))
                % 16
                == 0;
            if rebuild {
                let Some(centre) = bound_centre(actor.ped, &loader) else {
                    diagnostics.missing_original_bounds += 1;
                    warn_once!("near_ped_lists_waiting_original_col_bounds");
                    continue;
                };
                if !spatial.grid.candidates(
                    centre,
                    PED_LIST_SCAN_HALF_EXTENT,
                    &mut candidate_entities,
                ) {
                    diagnostics.invalid_sector_queries += 1;
                    warn_once!("near_ped_lists_invalid_sector_query");
                    continue;
                }
                diagnostics.rebuilds += 1;
                candidates.clear();
                for &other_entity in candidate_entities.iter() {
                    if other_entity == entity {
                        continue;
                    }
                    let Ok(other) = read.get(other_entity) else {
                        continue;
                    };
                    let Some(handle) = live_handle(other_entity, &other, &index, &players) else {
                        continue;
                    };
                    if other.vehicle.is_some() {
                        continue;
                    }
                    diagnostics.candidate_visits += 1;
                    let delta = other.ped.physical.entity.get_position() - position;
                    if (delta.x * delta.x + delta.y * delta.y).sqrt() >= maximum_distance {
                        continue;
                    }
                    if candidates.len() == TEMP_PED_CAPACITY {
                        sort_candidates(&mut candidates);
                        candidates.truncate(NEAR_PED_CAPACITY);
                    }
                    candidates.push(NearPedCandidate {
                        handle,
                        distance: delta.magnitude(),
                    });
                }
                sort_candidates(&mut candidates);
                count = candidates.len().min(NEAR_PED_CAPACITY);
                for (slot, candidate) in candidates.iter().take(count).enumerate() {
                    handles[slot] = Some(candidate.handle);
                }
            } else {
                let mut retained = 0;
                for handle in actor.ped.near_peds.iter().flatten().copied() {
                    let nearby = resolve_near_ped(handle, &index, &players, &read)
                        .and_then(|entity| read.get(entity).ok())
                        .is_some_and(|other| {
                            let delta = other.ped.physical.entity.get_position() - position;
                            delta.x * delta.x + delta.y * delta.y <= PED_LIST_PRUNE_DISTANCE_SQUARED
                        });
                    if nearby {
                        handles[retained] = Some(handle);
                        retained += 1;
                    } else {
                        diagnostics.pruned_references += 1;
                    }
                }
                count = retained;
            }
        }
        if let Ok(mut ped) = actors.p1().get_mut(entity) {
            ped.near_peds = handles;
            ped.num_near_peds = count as u8;
        }
    }
}

/// CPed::~CPed's reciprocal cache compaction. Explicit SCM retirement and
/// player destruction use this same helper.
pub(crate) fn remove_near_ped_reference(ped: &mut CPed, removed_handle: i32) {
    let count = usize::from(ped.num_near_peds).min(NEAR_PED_CAPACITY);
    let mut write = 0;
    for read in 0..count {
        let Some(handle) = ped.near_peds[read] else {
            continue;
        };
        if handle == removed_handle {
            continue;
        }
        ped.near_peds[write] = Some(handle);
        write += 1;
    }
    ped.near_peds[write..].fill(None);
    ped.num_near_peds = write as u8;
}

/// Arbitrary Bevy retirement invalidates cached handles during pause/loading.
pub(crate) fn retire_near_ped_references(
    mut removed: RemovedComponents<CPed>,
    mut removed_identities: RemovedComponents<ScriptPedIdentity>,
    mut removed_players: RemovedComponents<PlayerPedState>,
    index: Option<Res<ScriptEntityIndex>>,
    players: Option<Res<CWorldPlayerState>>,
    mut diagnostics: ResMut<PedNearbyDiagnostics>,
    mut actors: ParamSet<(Query<NearPedRead>, Query<&mut CPed>)>,
    identities: Query<Entity, Or<(With<ScriptPedIdentity>, With<PlayerPedState>)>>,
) {
    if removed
        .read()
        .chain(removed_identities.read())
        .chain(removed_players.read())
        .count()
        == 0
    {
        return;
    }
    let (Some(index), Some(players)) = (index.as_deref(), players.as_deref()) else {
        if !identities.is_empty() {
            diagnostics.missing_identity_resources += 1;
            warn_once!("near_ped_retirement_waiting_canonical_identity");
        }
        return;
    };
    for entity in &identities {
        let mut invalid = Vec::new();
        {
            let read = actors.p0();
            let Ok(actor) = read.get(entity) else {
                continue;
            };
            for handle in actor.ped.near_peds.iter().flatten().copied() {
                if resolve_near_ped(handle, index, players, &read).is_none() {
                    invalid.push(handle);
                }
            }
        }
        if !invalid.is_empty()
            && let Ok(mut ped) = actors.p1().get_mut(entity)
        {
            for handle in invalid {
                remove_near_ped_reference(&mut ped, handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::PedType::ePedType;
    use re3bevy_core::core::General::CGeneral;

    fn spawn_ped(app: &mut App, handle: i32, position: CVector) -> Entity {
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        ped.physical.entity.model_index = 7;
        ped.physical.entity.random_seed = 0;
        ped.physical.entity.set_position(position);
        let entity = app
            .world_mut()
            .spawn((
                ped,
                ScriptPedIdentity {
                    handle,
                    model_id: 7,
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .insert(handle, entity);
        entity
    }

    #[test]
    fn live_sector_lists_use_native_capacity_occupancy_phase_pruning_and_paused_retirement() {
        let mut app = App::new();
        let mut loader = CFileLoader::new();
        let native_col = loader.temp_col_models.col_model_ped1.clone();
        loader
            .model_info
            .add_ped_model(7)
            .clump
            .base
            .set_col_model(native_col, false);
        app.insert_resource(loader)
            .init_resource::<ScriptEntityIndex>()
            .init_resource::<CWorldPlayerState>()
            .init_resource::<CTimer>()
            .init_resource::<PedThreatReactionRange>()
            .init_resource::<PedNearbySpatial>()
            .init_resource::<PedNearbyDiagnostics>()
            .add_systems(
                Update,
                (
                    retire_near_ped_references,
                    sync_near_ped_spatial,
                    process_near_ped_lists
                        .run_if(|loader: Res<CFileLoader>| !loader.world.process_cutscene_only),
                )
                    .chain(),
            );
        let observer = spawn_ped(&mut app, 257, CVector::ZERO);
        let mut closest = None;
        for slot in (1..=65).rev() {
            let entity = spawn_ped(
                &mut app,
                257 + slot * 256,
                CVector::new(slot as f32 * 0.1, 0.0, 0.0),
            );
            if slot == 1 {
                closest = Some(entity);
            }
        }
        let vehicle = app.world_mut().spawn_empty().id();
        let occupant = spawn_ped(&mut app, 100001, CVector::new(0.01, 0.0, 0.0));
        app.world_mut()
            .entity_mut(occupant)
            .insert(InVehicle::driver(vehicle));
        app.update();
        let ped = app.world().get::<CPed>(observer).unwrap();
        assert_eq!(ped.num_near_peds, 10);
        let expected = std::array::from_fn(|slot| Some(257 + (slot as i32 + 1) * 256));
        assert_eq!(
            ped.near_peds, expected,
            "bounded dense-scene truncation retains native nearest ten"
        );
        let closest = closest.unwrap();
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_frame_counter(1);
        app.world_mut()
            .get_mut::<CPed>(closest)
            .unwrap()
            .physical
            .entity
            .set_position(CVector::new(30.0, 0.0, 500.0));
        app.update();
        assert_eq!(
            app.world().get::<CPed>(observer).unwrap().num_near_peds,
            10,
            "native between-phase pruning uses XY and retains equality at 30m"
        );
        app.world_mut()
            .get_mut::<CPed>(closest)
            .unwrap()
            .physical
            .entity
            .set_position(CVector::new(30.001, 0.0, 0.0));
        app.update();
        assert_eq!(app.world().get::<CPed>(observer).unwrap().num_near_peds, 9);
        let second = app
            .world()
            .resource::<ScriptEntityIndex>()
            .ped(257 + 2 * 256)
            .unwrap();
        app.world_mut()
            .resource_mut::<CFileLoader>()
            .world
            .process_cutscene_only = true;
        app.world_mut().despawn(second);
        for _ in 0..8 {
            app.update();
        }
        assert!(
            !app.world()
                .resource::<PedNearbySpatial>()
                .grid
                .contains(&second)
        );
        assert!(
            !app.world()
                .get::<CPed>(observer)
                .unwrap()
                .near_peds
                .contains(&Some(257 + 2 * 256)),
            "arbitrary despawn retires references while gameplay is suspended"
        );
        assert_eq!(
            app.world()
                .resource::<PedNearbyDiagnostics>()
                .missing_original_bounds,
            0
        );
    }

    #[test]
    fn equal_distance_sort_uses_native_exchange_order() {
        let mut candidates: Vec<_> = (0..5)
            .map(|handle| NearPedCandidate {
                handle,
                distance: 1.0,
            })
            .collect();
        sort_candidates(&mut candidates);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.handle)
                .collect::<Vec<_>>(),
            [3, 4, 2, 0, 1]
        );
    }
}
