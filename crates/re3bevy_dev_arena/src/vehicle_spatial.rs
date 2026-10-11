//! Arena membership reuses the production automobile sector grid.
use crate::{ArenaOriginalRuntime, ArenaVehicleSettings};
use bevy::prelude::*;
use re3bevy_core::core::Coordinates::CVector;
use re3bevy_game::{
    core::{FileLoader::CFileLoader, EntitySectorIndex::{EntitySectorIndex, SectorRect}},
    vehicles::{Automobile::AutomobileState, Vehicle::VehicleCore},
};

#[derive(Clone, Copy)]
pub(crate) struct VehicleSphere {
    pub center: CVector,
    pub radius: f32,
    pub model: usize,
}

pub(crate) fn live_vehicle_sphere(
    core: &VehicleCore,
    loader: &CFileLoader,
) -> Option<VehicleSphere> {
    let model = usize::try_from(core.physical.entity.model_index).ok()?;
    let col = loader.model_info.col_model(model)?;
    let center = col.bounding_sphere.center;
    let center = core
        .physical
        .entity
        .matrix
        .transform_point(CVector::new(center.x, center.y, center.z));
    let radius = col.bounding_sphere.radius;
    if !radius.is_finite()
        || radius < 0.0
        || !center.x.is_finite()
        || !center.y.is_finite()
        || !center.z.is_finite()
    {
        return None;
    }
    Some(VehicleSphere {
        center,
        radius,
        model,
    })
}

pub(crate) fn live_vehicle_rect(core: &VehicleCore, loader: &CFileLoader) -> Option<SectorRect> {
    let model = usize::try_from(core.physical.entity.model_index).ok()?;
    let col = loader.model_info.col_model(model)?;
    // CPhysical::Add dispatches to the sphere-based physical override.
    live_vehicle_sphere(core, loader)?;
    Some(core.physical.get_bound_rect(col))
}

#[derive(Default, Resource)]
pub(crate) struct ArenaVehicleSpatial {
    pub grid: EntitySectorIndex,
    pub missing_bounds: u64,
    pub candidate_visits: u64,
}
impl ArenaVehicleSpatial {
    pub fn refresh(&mut self, entity: Entity, core: &VehicleCore, loader: &CFileLoader) {
        if !live_vehicle_rect(core, loader).is_some_and(|rect| self.grid.update_rect(entity, rect)) {
            self.grid.remove(entity);
            self.missing_bounds = self.missing_bounds.saturating_add(1);
            warn_once!("arena_collision_waiting_for_authored_bounds");
        }
    }
}

/// Removal messages are drained outside arena/pause/loading guards. Pose changes
/// are indexed after control and before the collision stage in the same frame.
pub(crate) fn sync_arena_vehicle_spatial(
    mut spatial: ResMut<ArenaVehicleSpatial>,
    original: Res<ArenaOriginalRuntime>,
    changed: Query<
        (Entity, &VehicleCore),
        (
            With<ArenaVehicleSettings>,
            With<AutomobileState>,
            Or<(
                Changed<VehicleCore>,
                Added<ArenaVehicleSettings>,
                Added<AutomobileState>,
            )>,
        ),
    >,
    mut removed_core: RemovedComponents<VehicleCore>,
    mut removed_fixture: RemovedComponents<ArenaVehicleSettings>,
    mut removed_kind: RemovedComponents<AutomobileState>,
) {
    for entity in removed_core
        .read()
        .chain(removed_fixture.read())
        .chain(removed_kind.read())
    {
        spatial.grid.remove(entity);
    }
    let Some(data) = original.data.as_ref() else {
        if !changed.is_empty() {
            warn_once!("arena_collision_membership_missing_original_data");
        }
        return;
    };
    for (entity, core) in &changed {
        spatial.refresh(entity, core, &data.file_loader);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CollisionPair {
    pub first: Entity,
    pub second: Entity,
}
impl CollisionPair {
    pub fn new(a: Entity, b: Entity) -> Self {
        if a.to_bits() <= b.to_bits() {
            Self {
                first: a,
                second: b,
            }
        } else {
            Self {
                first: b,
                second: a,
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct ArenaCollisionScratch {
    pub sources: Vec<Entity>,
    pub candidates: Vec<Entity>,
    pub refresh: Vec<Entity>,
    pub visited: std::collections::HashSet<CollisionPair>,
    pub queued: std::collections::HashSet<Entity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distant_actor_membership_does_not_produce_global_pair_candidates() {
        let mut world = World::new();
        let mut grid = EntitySectorIndex::default();
        let mut actors = Vec::new();
        for y in 0..16 {
            for x in 0..16 {
                let entity = world.spawn_empty().id();
                let position =
                    CVector::new(-960.0 + x as f32 * 120.0, -960.0 + y as f32 * 120.0, 0.0);
                grid.update(entity, position);
                actors.push((entity, position));
            }
        }
        let mut candidates = Vec::new();
        for (entity, position) in actors {
            grid.candidates(position, 10.0, &mut candidates);
            assert_eq!(
                candidates,
                vec![entity],
                "sparse actors must query local members only"
            );
        }
    }

    #[test]
    fn removal_messages_drain_when_arena_is_inactive_and_original_data_unavailable() {
        let mut app = App::new();
        app.init_resource::<ArenaVehicleSpatial>()
            .init_resource::<ArenaOriginalRuntime>()
            .init_resource::<crate::DeveloperArena>()
            .add_systems(Update, sync_arena_vehicle_spatial);
        let mut random = re3bevy_core::core::General::CGeneral::default();
        let car = re3bevy_game::vehicles::Automobile::CAutomobile::new(
            111,
            0,
            0,
            &mut random,
            None,
            None,
            false,
        );
        let actor = app
            .world_mut()
            .spawn(
                crate::ArenaAutomobileSpawn {
                    car,
                    settings: ArenaVehicleSettings {
                        handling: Default::default(),
                        wheel_scale: 1.0,
                        wheel_friction: 1.0,
                        surface_adhesion: 1.0,
                    },
                }
                .into_bundle(Default::default()),
            )
            .id();
        app.world_mut()
            .resource_mut::<ArenaVehicleSpatial>()
            .grid
            .update(actor, CVector::ZERO);
        app.update();
        app.world_mut()
            .entity_mut(actor)
            .remove::<ArenaVehicleSettings>();
        app.update();
        assert!(
            !app.world()
                .resource::<ArenaVehicleSpatial>()
                .grid
                .entities()
                .any(|entity| entity == actor)
        );
        app.world_mut()
            .resource_mut::<ArenaVehicleSpatial>()
            .grid
            .update(actor, CVector::ZERO);
        app.world_mut().despawn(actor);
        app.update();
        assert!(
            !app.world()
                .resource::<ArenaVehicleSpatial>()
                .grid
                .entities()
                .any(|entity| entity == actor)
        );
    }
}
