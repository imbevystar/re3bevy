//! Immutable access to materialized world players for presentation systems.
//! re3 references: CWorld::Players, FindPlayerPed, FindPlayerVehicle.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::{
    PlayerPed::{CWorldPlayerState, MAX_PLAYERS, PlayerPedMut, PlayerPedRef, PlayerPedState},
    VehicleRelations::InVehicle,
};

/// Borrows the canonical slot index and live components without requesting
/// player mutation, commands or a local pending-constructor store. Presentation
/// runs after player spawn commands have materialized the entity components.
#[derive(SystemParam)]
pub struct CWorldPlayersRead<'w, 's> {
    state: Option<Res<'w, CWorldPlayerState>>,
    players: Query<
        'w,
        's,
        (
            &'static super::Ped::CPed,
            &'static super::PedVitality::PedVitality,
            &'static super::PedLocomotion::PedLocomotion,
            &'static super::PedAnimation::PedAnimation,
            &'static PlayerPedState,
            Option<&'static InVehicle>,
        ),
        With<PlayerPedState>,
    >,
}

impl CWorldPlayersRead<'_, '_> {
    pub fn player_entity(&self, index: i32) -> Option<Entity> {
        let index = usize::try_from(index.max(0)).ok()?;
        if index >= MAX_PLAYERS {
            return None;
        }
        self.state.as_ref()?.entities[index]
    }

    pub fn player(&self, index: i32) -> Option<PlayerPedRef<'_>> {
        self.players.get(self.player_entity(index)?).ok().map(
            |(ped, vitality, locomotion, animation, state, _)| PlayerPedRef {
                ped,
                vitality,
                locomotion,
                animation,
                state,
            },
        )
    }

    pub fn focused(&self) -> Option<PlayerPedRef<'_>> {
        self.player(self.player_in_focus() as i32)
    }
    pub fn iter(&self) -> impl Iterator<Item = PlayerPedRef<'_>> + '_ {
        (0..MAX_PLAYERS).filter_map(|index| self.player(index as i32))
    }
    pub fn iter_slots(&self) -> impl Iterator<Item = (usize, Option<PlayerPedRef<'_>>)> + '_ {
        (0..MAX_PLAYERS).map(|index| (index, self.player(index as i32)))
    }

    pub fn relation(&self, index: i32) -> Option<InVehicle> {
        self.players
            .get(self.player_entity(index)?)
            .ok()?
            .5
            .copied()
    }

    pub fn player_in_focus(&self) -> usize {
        self.state.as_ref().map_or(0, |state| state.player_in_focus)
    }
}

/// Combat only joins materialized players. The slot/global index is read-only;
/// this access has no spawning, retirement, seat mutation or pending DTO store.
#[derive(SystemParam)]
pub(crate) struct CWorldPlayersCombat<'w, 's> {
    state: Option<Res<'w, CWorldPlayerState>>,
    players: Query<
        'w,
        's,
        (
            Entity,
            &'static mut super::Ped::CPed,
            &'static mut super::PedVitality::PedVitality,
            &'static mut super::PedLocomotion::PedLocomotion,
            &'static mut super::PedAnimation::PedAnimation,
            &'static mut PlayerPedState,
            Option<&'static InVehicle>,
        ),
        With<PlayerPedState>,
    >,
}

impl CWorldPlayersCombat<'_, '_> {
    pub(crate) const fn slot_count(&self) -> usize {
        MAX_PLAYERS
    }

    pub(crate) fn entity(&self, index: i32) -> Option<Entity> {
        let index = usize::try_from(index.max(0)).ok()?;
        self.state.as_ref()?.entities.get(index).copied().flatten()
    }

    pub(crate) fn player(&self, index: i32) -> Option<PlayerPedRef<'_>> {
        let (_, ped, vitality, locomotion, animation, state, _) =
            self.players.get(self.entity(index)?).ok()?;
        Some(PlayerPedRef {
            ped,
            vitality,
            locomotion,
            animation,
            state,
        })
    }

    pub(crate) fn player_mut(&mut self, index: i32) -> Option<PlayerPedMut<'_>> {
        let entity = self.entity(index)?;
        let (_, ped, vitality, locomotion, animation, state, _) =
            self.players.get_mut(entity).ok()?;
        Some(PlayerPedMut {
            ped: ped.into_inner(),
            vitality: vitality.into_inner(),
            locomotion: locomotion.into_inner(),
            animation: animation.into_inner(),
            state: state.into_inner(),
        })
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = PlayerPedRef<'_>> {
        (0..MAX_PLAYERS).filter_map(move |index| self.player(index as i32))
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = PlayerPedMut<'_>> {
        let indexed = self
            .state
            .as_ref()
            .map_or([None; MAX_PLAYERS], |state| state.entities);
        self.players.iter_mut().filter_map(
            move |(entity, ped, vitality, locomotion, animation, state, _)| {
                indexed.contains(&Some(entity)).then(|| PlayerPedMut {
                    ped: ped.into_inner(),
                    vitality: vitality.into_inner(),
                    locomotion: locomotion.into_inner(),
                    animation: animation.into_inner(),
                    state: state.into_inner(),
                })
            },
        )
    }

    pub(crate) fn focused(&self) -> Option<PlayerPedRef<'_>> {
        self.player(self.state.as_ref()?.player_in_focus as i32)
    }

    pub(crate) fn relation(&self, index: i32) -> Option<InVehicle> {
        self.players.get(self.entity(index)?).ok()?.6.copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::PlayerPed::{CPlayerPed, CWorldPlayers, WorldPlayerAccess};
    use bevy::ecs::system::SystemState;
    use rustgta3_core::core::General::CGeneral;

    #[test]
    fn presentation_observes_replacement_only_after_materialization() {
        let mut world = World::new();
        world.init_resource::<CWorldPlayerState>();
        let mut general = CGeneral::with_seed(1);
        let mut writer = SystemState::<CWorldPlayers>::new(&mut world);
        let mut reader = SystemState::<CWorldPlayersRead>::new(&mut world);
        {
            let mut players = writer.get_mut(&mut world).unwrap();
            let mut player = CPlayerPed::new(&mut general, None);
            player.vitality.health = 73.0;
            players.replace_player(0, player);
            players.flush_pending();
        }
        assert!(reader.get(&world).unwrap().player(0).is_none());
        writer.apply(&mut world);
        let old_entity = world.resource::<CWorldPlayerState>().entities[0].unwrap();
        assert_eq!(
            reader
                .get(&world)
                .unwrap()
                .player(0)
                .unwrap()
                .vitality
                .health,
            73.0
        );
        {
            let mut players = writer.get_mut(&mut world).unwrap();
            let mut replacement = CPlayerPed::new(&mut general, None);
            replacement.vitality.health = 41.0;
            players.replace_player(0, replacement);
            players.flush_pending();
        }
        // The slot already points to its new identity, so presentation must
        // never expose the old player while its deferred despawn is pending.
        assert!(world.get_entity(old_entity).is_ok());
        assert!(reader.get(&world).unwrap().player(0).is_none());
        writer.apply(&mut world);
        assert!(world.get_entity(old_entity).is_err());
        assert_eq!(
            reader
                .get(&world)
                .unwrap()
                .player(0)
                .unwrap()
                .vitality
                .health,
            41.0
        );
    }
    #[test]
    fn player_components_share_mutation_and_pending_pair_access() {
        use crate::control::ScriptEntities::ScriptPedStore;
        use crate::peds::PedAnimation::PedAnimation;
        let mut world = World::new();
        world.init_resource::<CWorldPlayerState>();
        let mut general = CGeneral::with_seed(13);
        let mut writer = SystemState::<CWorldPlayers>::new(&mut world);
        {
            let mut players = writer.get_mut(&mut world).unwrap();
            players.replace_player(0, CPlayerPed::new(&mut general, None));
            players.replace_player(1, CPlayerPed::new(&mut general, None));
            let (p0, p1) = players.get_pair_mut(0, 1);
            p0.unwrap().vitality.health = 64.0;
            p1.unwrap().animation.movement_anim_clump.velocity2d = Some((4.0, 5.0));
            assert_eq!(players.player(0).unwrap().vitality.health, 64.0);
            players.flush_pending();
        }
        writer.apply(&mut world);
        let entities = world.resource::<CWorldPlayerState>().entities;
        let e0 = entities[0].unwrap();
        let e1 = entities[1].unwrap();
        assert_eq!(
            world
                .get::<crate::peds::PedVitality::PedVitality>(e0)
                .unwrap()
                .health,
            64.0
        );
        assert_eq!(
            world
                .get::<PedAnimation>(e1)
                .unwrap()
                .movement_anim_clump
                .velocity2d,
            Some((4.0, 5.0))
        );
        world
            .get_mut::<crate::peds::PedVitality::PedVitality>(e0)
            .unwrap()
            .health = 22.0;
        world.get_mut::<PlayerPedState>(e0).unwrap().current_stamina = 85.0;
        {
            let mut players = writer.get_mut(&mut world).unwrap();
            assert_eq!(players.player(0).unwrap().vitality.health, 22.0);
            assert_eq!(players.player(0).unwrap().state.current_stamina, 85.0);
            players.replace_player(1, CPlayerPed::new(&mut general, None));
            let (p0, p1) = players.get_pair_mut(0, 1);
            p0.unwrap().vitality.health = 17.0;
            p1.unwrap().state.current_stamina = 42.0;
            players.flush_pending();
        }
        writer.apply(&mut world);
        assert!(world.get_entity(e1).is_err());
        assert_eq!(
            world
                .get::<crate::peds::PedVitality::PedVitality>(e0)
                .unwrap()
                .health,
            17.0
        );
        // Both access owners can be joined in one system without overlapping
        // mutable common-ped/animation queries.
        let mut joined = SystemState::<(CWorldPlayers, ScriptPedStore)>::new(&mut world);
        let (players, _) = joined.get_mut(&mut world).unwrap();
        assert_eq!(players.player(1).unwrap().state.current_stamina, 42.0);
    }
}
