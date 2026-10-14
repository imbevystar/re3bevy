//! Arena actor views borrow the production player component owners.
use bevy::ecs::query::QueryData;
use rust_gta_3_game::peds::{
    Ped::CPed, PedAnimation::PedAnimation, PedLocomotion::PedLocomotion,
    PedVitality::PedVitality, PlayerPed::{PlayerPedMut, PlayerPedRef, PlayerPedState},
};
use crate::ArenaCoopPlayerTwo;

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct ArenaPrimaryActor {
    ped: &'static mut CPed,
    vitality: &'static mut PedVitality,
    locomotion: &'static mut PedLocomotion,
    animation: &'static mut PedAnimation,
    player: &'static mut PlayerPedState,
}
impl<'w, 's> ArenaPrimaryActorItem<'w, 's> {
    fn into_view(self) -> PlayerPedMut<'w> {
        PlayerPedMut { ped: self.ped.into_inner(), vitality: self.vitality.into_inner(),
            locomotion: self.locomotion.into_inner(), animation: self.animation.into_inner(),
            state: self.player.into_inner() }
    }
}
impl<'w, 's> ArenaPrimaryActorReadOnlyItem<'w, 's> {
    fn into_view(self) -> PlayerPedRef<'w> {
        PlayerPedRef { ped: self.ped, vitality: self.vitality, locomotion: self.locomotion,
            animation: self.animation, state: self.player }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArenaPrimaryPlayers<'w, 's> {
    actors: bevy::prelude::Query<'w, 's, ArenaPrimaryActor,
        (bevy::prelude::With<crate::ArenaPlayer>, bevy::prelude::Without<crate::ArenaCoopPlayerTwo>)>,
}
impl ArenaPrimaryPlayers<'_, '_> {
    pub fn get_mut(&mut self, entity: Option<bevy::prelude::Entity>) -> Option<PlayerPedMut<'_>> {
        let entity = entity?;
        match self.actors.get_mut(entity) {
            Ok(actor) => Some(actor.into_view()),
            Err(_) => { bevy::log::warn_once!("arena_primary_missing_canonical_actor"); None }
        }
    }
    pub fn get(&self, entity: Option<bevy::prelude::Entity>) -> Option<PlayerPedRef<'_>> {
        let entity = entity?;
        match self.actors.get(entity) {
            Ok(actor) => Some(actor.into_view()),
            Err(_) => { bevy::log::warn_once!("arena_primary_missing_canonical_actor"); None }
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArenaPrimaryPlayerReader<'w, 's> {
    actors: bevy::prelude::Query<'w, 's, ArenaPrimaryActorReadOnly,
        (bevy::prelude::With<crate::ArenaPlayer>, bevy::prelude::Without<crate::ArenaCoopPlayerTwo>)>,
}
impl ArenaPrimaryPlayerReader<'_, '_> {
    pub fn get(&self, entity: Option<bevy::prelude::Entity>) -> Option<PlayerPedRef<'_>> {
        let entity = entity?;
        match self.actors.get(entity) {
            Ok(actor) => Some(actor.into_view()),
            Err(_) => { bevy::log::warn_once!("arena_primary_missing_canonical_actor"); None }
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub(crate) struct ArenaCoopActor {
    ped: &'static mut CPed,
    vitality: &'static mut PedVitality,
    locomotion: &'static mut PedLocomotion,
    animation: &'static mut PedAnimation,
    player: &'static mut PlayerPedState,
    fixture: &'static mut ArenaCoopPlayerTwo,
}

pub(crate) struct ArenaCoopView<Runtime, Fixture> {
    pub runtime: Runtime,
    pub fixture: Fixture,
}
pub(crate) type ArenaCoopMut<'a> = ArenaCoopView<PlayerPedMut<'a>, &'a mut ArenaCoopPlayerTwo>;

impl<R, F: std::ops::Deref<Target = ArenaCoopPlayerTwo>> std::ops::Deref for ArenaCoopView<R, F> {
    type Target = ArenaCoopPlayerTwo;
    fn deref(&self) -> &Self::Target { &self.fixture }
}
impl<R, F: std::ops::DerefMut<Target = ArenaCoopPlayerTwo>> std::ops::DerefMut for ArenaCoopView<R, F> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.fixture }
}

impl<'w, 's> ArenaCoopActorItem<'w, 's> {
    pub fn into_view(self) -> ArenaCoopMut<'w> {
        ArenaCoopView {
            runtime: PlayerPedMut {
                ped: self.ped.into_inner(), vitality: self.vitality.into_inner(),
                locomotion: self.locomotion.into_inner(), animation: self.animation.into_inner(),
                state: self.player.into_inner(),
            },
            fixture: self.fixture.into_inner(),
        }
    }
}
