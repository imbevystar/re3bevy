//! Native walk-around obstacle references with Bevy-owned lifetime.
//! CEntity::RegisterReference / ResolveReferences (core/References.cpp).
use bevy::prelude::*;

/// Shared native collision/walk-around hysteresis, owned for the ped lifetime.
/// ResolveReferences clears identity only; it must preserve this history.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PedObstacleAvoidance {
    pub colliding_thing_timer: u32,
    pub walk_around_type: i32,
}

/// Wander-only progress across frames. A blocked PATH_PED link must not keep
/// an ambient actor walking into the same static collision indefinitely.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PedWanderProgress {
    pub blocked_seconds: f32,
    pub blocked_node: Option<usize>,
    pub retry_at_ms: u32,
}

/// The absence of this component is native's null obstacle reference.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[relationship(relationship_target = PedObstacleFollowers)]
pub struct PedWalkAroundObstacle(Entity);

impl PedWalkAroundObstacle {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    pub fn entity(&self) -> Entity {
        self.0
    }
}

/// Derived membership, maintained exclusively by Bevy relationship hooks.
/// Target retirement removes references; it does not retire the pedestrians.
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = PedWalkAroundObstacle)]
pub struct PedObstacleFollowers(Vec<Entity>);

/// CReferences::RemoveReferencesToPlayer may clear references to a live actor.
/// Removing reverse membership runs the same source-removal hooks as retirement.
pub fn clear_obstacle_references(commands: &mut Commands, target: Entity) {
    commands.entity(target).remove::<PedObstacleFollowers>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_history_access_and_reference_retirement_preserve_ped_and_history() {
        use crate::peds::{Ped::CPed, PedType::ePedType};
        use bevy::ecs::system::SystemState;
        use rustgta3_core::core::General::CGeneral;
        let mut world = World::new();
        let obstacle = world.spawn_empty().id();
        let mut general = CGeneral::with_seed(1);
        let original = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        let ped = world
            .spawn((original.clone(), PedWalkAroundObstacle::new(obstacle)))
            .id();
        assert_eq!(
            *world.get::<PedObstacleAvoidance>(ped).unwrap(),
            PedObstacleAvoidance::default()
        );
        let mut state = SystemState::<Query<(&CPed, &mut PedObstacleAvoidance)>>::new(&mut world);
        {
            let mut query = state.get_mut(&mut world).unwrap();
            let (readonly_ped, mut avoidance) = query.get_mut(ped).unwrap();
            assert_eq!(readonly_ped, &original);
            avoidance.colliding_thing_timer = 4_000;
            avoidance.walk_around_type = 2;
        }
        world.despawn(obstacle);
        world.flush();
        assert_eq!(world.get::<CPed>(ped).unwrap(), &original);
        assert_eq!(
            *world.get::<PedObstacleAvoidance>(ped).unwrap(),
            PedObstacleAvoidance {
                colliding_thing_timer: 4_000,
                walk_around_type: 2,
            }
        );
        assert!(world.get::<PedWalkAroundObstacle>(ped).is_none());
        world.despawn(ped);
        let replacement = world
            .spawn(CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general))
            .id();
        assert_eq!(
            *world.get::<PedObstacleAvoidance>(replacement).unwrap(),
            PedObstacleAvoidance::default()
        );
    }

    #[test]
    fn explicit_clear_preserves_live_target_and_all_source_identities() {
        let mut world = World::new();
        let target = world.spawn_empty().id();
        let ped = world.spawn(PedWalkAroundObstacle::new(target)).id();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        clear_obstacle_references(&mut Commands::new(&mut queue, &world), target);
        queue.apply(&mut world);
        world.flush();
        assert!(world.get_entity(target).is_ok());
        assert!(world.get_entity(ped).is_ok());
        assert!(world.get::<PedWalkAroundObstacle>(ped).is_none());
        world
            .entity_mut(ped)
            .insert(PedWalkAroundObstacle::new(target));
        assert_eq!(
            world.get::<PedWalkAroundObstacle>(ped).unwrap().entity(),
            target
        );
    }

    #[test]
    fn replacement_and_source_retirement_maintain_exact_reverse_membership() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let ped = world.spawn(PedWalkAroundObstacle::new(first)).id();
        assert_eq!(
            world.get::<PedObstacleFollowers>(first).unwrap().0,
            vec![ped]
        );
        world
            .entity_mut(ped)
            .insert(PedWalkAroundObstacle::new(second));
        assert!(
            world
                .get::<PedObstacleFollowers>(first)
                .is_none_or(|members| members.0.is_empty())
        );
        assert_eq!(
            world.get::<PedObstacleFollowers>(second).unwrap().0,
            vec![ped]
        );
        world.despawn(first);
        assert_eq!(
            world.get::<PedWalkAroundObstacle>(ped).unwrap().entity(),
            second
        );
        world.despawn(ped);
        assert!(
            world
                .get::<PedObstacleFollowers>(second)
                .is_none_or(|members| members.0.is_empty())
        );
    }

    #[test]
    fn target_retirement_clears_all_references_without_retiring_pedestrians() {
        let mut world = World::new();
        let obstacle = world.spawn_empty().id();
        let peds: Vec<_> = (0..3)
            .map(|_| world.spawn(PedWalkAroundObstacle::new(obstacle)).id())
            .collect();
        world.despawn(obstacle);
        world.flush();
        for ped in peds {
            assert!(world.get_entity(ped).is_ok());
            assert!(world.get::<PedWalkAroundObstacle>(ped).is_none());
        }
        let replacement = world.spawn_empty().id();
        assert_ne!(replacement, obstacle);
    }
}
