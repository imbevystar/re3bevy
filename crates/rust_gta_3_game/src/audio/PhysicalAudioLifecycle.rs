//! Derived actor/audio bindings replacing CPhysical pointer lifetime in Bevy.
use crate::{
    audio::{DMAudio::cDMAudio, audio_enums::AUDIOTYPE_PHYSICAL},
    control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
    peds::{
        Ped::CPed,
        PlayerPed::{CWorldPlayerState, PlayerPedState},
    },
    vehicles::Vehicle::VehicleCore,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AudioSlotIdentity {
    id: i32,
    generation: u64,
}

#[derive(Default, Resource)]
pub(crate) struct PhysicalAudioBindings {
    actors: HashMap<Entity, AudioSlotIdentity>,
    slots: HashMap<AudioSlotIdentity, Entity>,
    pub stale_slots: u64,
    pub retired_slots: u64,
    pub preserved_reused_slots: u64,
}

fn slot_identity(audio: &cDMAudio, id: i32) -> Option<AudioSlotIdentity> {
    let slot = usize::try_from(id)
        .ok()
        .and_then(|id| audio.AudioManager.m_asAudioEntities.get(id))?;
    (slot.m_bIsUsed && slot.m_nType == AUDIOTYPE_PHYSICAL).then_some(AudioSlotIdentity {
        id,
        generation: slot.allocation_generation,
    })
}

impl PhysicalAudioBindings {
    pub(crate) fn ensure_actor(
        &mut self,
        entity: Entity,
        native_handle: i32,
        id: &mut i32,
        audio: &mut cDMAudio,
    ) {
        let existing = slot_identity(audio, *id).filter(|_| {
            audio.AudioManager.m_asAudioEntities[*id as usize].m_pEntity == Some(native_handle)
        });
        let owned = existing.is_some_and(|slot| {
            self.actors.get(&entity).is_some_and(|owned| *owned == slot)
                || (!self.actors.contains_key(&entity) && !self.slots.contains_key(&slot))
        });
        if *id >= 0 && !owned {
            self.stale_slots += 1;
            warn_once!("physical_audio_stale_actor_slot");
            *id = -1;
        }
        if *id < 0 {
            *id = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(native_handle));
            if *id < 0 {
                warn_once!("physical_audio_waiting_available_slot");
                return;
            }
            audio.SetEntityStatus(*id, true);
        }
        let Some(slot) = slot_identity(audio, *id) else {
            return;
        };
        if let Some(old) = self.actors.insert(entity, slot)
            && old != slot
        {
            self.slots.remove(&old);
            // A still-owned prior allocation has no actor left to retire it.
            if slot_identity(audio, old.id) == Some(old) {
                audio.DestroyEntity(old.id);
            }
        }
        self.slots.insert(slot, entity);
    }
}

/// Runs outside gameplay/loading/pause guards. Query canonical owners rather
/// than retaining a copy of actor pose or a second physical audio ID.
pub(crate) fn retire_physical_audio(
    mut audio: Option<ResMut<cDMAudio>>,
    mut bindings: ResMut<PhysicalAudioBindings>,
    peds: Query<&CPed>,
    vehicles: Query<&VehicleCore>,
    mut claims: Local<HashSet<AudioSlotIdentity>>,
) {
    let Some(audio) = audio.as_deref_mut() else {
        if !bindings.actors.is_empty() {
            warn_once!("physical_audio_retirement_waiting_manager");
        }
        return;
    };
    claims.clear();
    for id in peds.iter().map(|ped| ped.physical.audio_entity_id).chain(
        vehicles
            .iter()
            .map(|vehicle| vehicle.physical.audio_entity_id),
    ) {
        if let Some(slot) = slot_identity(audio, id) {
            claims.insert(slot);
        }
    }
    let PhysicalAudioBindings {
        actors,
        slots,
        retired_slots,
        preserved_reused_slots,
        ..
    } = &mut *bindings;
    actors.retain(|entity, binding| {
        let live_id = peds
            .get(*entity)
            .ok()
            .map(|ped| ped.physical.audio_entity_id)
            .or_else(|| {
                vehicles
                    .get(*entity)
                    .ok()
                    .map(|vehicle| vehicle.physical.audio_entity_id)
            });
        if live_id == Some(binding.id) && slot_identity(audio, binding.id) == Some(*binding) {
            return true;
        }
        slots.remove(binding);
        if slot_identity(audio, binding.id) == Some(*binding) && !claims.contains(binding) {
            audio.DestroyEntity(binding.id);
            *retired_slots += 1;
        } else {
            *preserved_reused_slots += 1;
        }
        false
    });
}

pub(crate) fn allocate_ped_audio_entities(
    mut audio: Option<ResMut<cDMAudio>>,
    mut bindings: ResMut<PhysicalAudioBindings>,
    index: Option<Res<ScriptEntityIndex>>,
    players: Option<Res<CWorldPlayerState>>,
    mut peds: Query<
        (
            Entity,
            &mut CPed,
            Option<&ScriptPedIdentity>,
            Option<&PlayerPedState>,
        ),
        Or<(With<ScriptPedIdentity>, With<PlayerPedState>)>,
    >,
) {
    let Some(audio) = audio.as_deref_mut() else {
        if !peds.is_empty() {
            warn_once!("physical_audio_allocation_waiting_manager");
        }
        return;
    };
    for (entity, mut ped, identity, player) in &mut peds {
        let handle = if let Some(player) = player {
            if !players
                .as_ref()
                .is_some_and(|players| players.entities.contains(&Some(entity)))
            {
                continue;
            }
            player.pool_handle
        } else if let Some(identity) = identity {
            if !index
                .as_ref()
                .is_some_and(|index| index.ped(identity.handle) == Some(entity))
            {
                continue;
            }
            identity.handle
        } else {
            continue;
        };
        bindings.ensure_actor(entity, handle, &mut ped.physical.audio_entity_id, audio);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::PedType::ePedType;
    use rust_gta_3_core::core::General::CGeneral;

    #[test]
    fn paused_player_recreation_keeps_one_canonical_audio_allocation_and_releases_it() {
        let mut app = App::new();
        let mut audio = cDMAudio::default();
        audio.Initialise();
        let baseline = audio.AudioManager.m_nAudioEntitiesCount;
        app.insert_resource(audio)
            .init_resource::<PhysicalAudioBindings>()
            .init_resource::<CWorldPlayerState>()
            .init_resource::<rust_gta_3_core::core::Timer::CTimer>()
            .add_systems(Update, (retire_physical_audio, allocate_ped_audio_entities).chain());
        app.world_mut().resource_mut::<rust_gta_3_core::core::Timer::CTimer>().set_code_pause(true);
        let mut random = CGeneral::with_seed(6);
        for _ in 0..100 {
            let mut player = crate::peds::PlayerPed::CPlayerPed::new(&mut random, None);
            player.state.pool_handle = 1;
            player.state.active_in_world = true;
            let entity = app.world_mut().spawn(player.into_bundle()).id();
            app.world_mut().resource_mut::<CWorldPlayerState>().entities[0] = Some(entity);
            app.update();
            let id = app.world().get::<CPed>(entity).unwrap().physical.audio_entity_id;
            let audio = app.world().resource::<cDMAudio>();
            assert!(id >= 0);
            assert_eq!(audio.AudioManager.m_asAudioEntities[id as usize].m_pEntity, Some(1));
            assert_eq!(audio.AudioManager.m_nAudioEntitiesCount, baseline + 1);
            app.update();
            assert_eq!(app.world().resource::<cDMAudio>().AudioManager.m_nAudioEntitiesCount, baseline + 1);
            app.world_mut().despawn(entity);
            app.world_mut().resource_mut::<CWorldPlayerState>().entities[0] = None;
            app.update();
            assert_eq!(app.world().resource::<cDMAudio>().AudioManager.m_nAudioEntitiesCount, baseline);
        }
        let bindings = app.world().resource::<PhysicalAudioBindings>();
        assert_eq!(bindings.stale_slots, 0);
        assert_eq!(bindings.retired_slots, 100);
        assert!(bindings.actors.is_empty() && bindings.slots.is_empty());
    }

    #[test]
    fn retired_ped_cannot_destroy_reused_vehicle_slot_with_equal_native_handle() {
        let mut app = App::new();
        let mut audio = cDMAudio::default();
        audio.Initialise();
        app.insert_resource(audio)
            .init_resource::<PhysicalAudioBindings>()
            .add_systems(Update, retire_physical_audio);
        let mut general = CGeneral::with_seed(1);
        let ped = app
            .world_mut()
            .spawn(CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general))
            .id();
        let mut id = -1;
        app.world_mut()
            .resource_scope(|world, mut bindings: Mut<PhysicalAudioBindings>| {
                bindings.ensure_actor(ped, 257, &mut id, &mut world.resource_mut::<cDMAudio>());
            });
        app.world_mut()
            .get_mut::<CPed>(ped)
            .unwrap()
            .physical
            .audio_entity_id = id;
        app.world_mut().despawn(ped);
        let mut audio = app.world_mut().resource_mut::<cDMAudio>();
        audio.DestroyEntity(id);
        let reused = audio.CreateEntity(AUDIOTYPE_PHYSICAL, Some(257));
        assert_eq!(reused, id);
        drop(audio);
        app.update();
        assert!(
            app.world()
                .resource::<cDMAudio>()
                .AudioManager
                .m_asAudioEntities[id as usize]
                .m_bIsUsed
        );
        assert_eq!(
            app.world()
                .resource::<PhysicalAudioBindings>()
                .preserved_reused_slots,
            1
        );
    }
    #[test]
    fn actor_retirement_destroys_its_own_slot_without_gameplay_resources() {
        let mut app = App::new();
        let mut audio = cDMAudio::default();
        audio.Initialise();
        app.insert_resource(audio)
            .init_resource::<PhysicalAudioBindings>()
            .add_systems(Update, retire_physical_audio);
        let mut general = CGeneral::with_seed(1);
        let ped = app
            .world_mut()
            .spawn(CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general))
            .id();
        let mut id = -1;
        app.world_mut()
            .resource_scope(|world, mut bindings: Mut<PhysicalAudioBindings>| {
                bindings.ensure_actor(ped, 257, &mut id, &mut world.resource_mut::<cDMAudio>());
            });
        app.world_mut()
            .get_mut::<CPed>(ped)
            .unwrap()
            .physical
            .audio_entity_id = id;
        app.update();
        assert!(
            app.world()
                .resource::<cDMAudio>()
                .AudioManager
                .m_asAudioEntities[id as usize]
                .m_bIsUsed
        );
        app.world_mut().despawn(ped);
        app.update();
        assert!(
            !app.world()
                .resource::<cDMAudio>()
                .AudioManager
                .m_asAudioEntities[id as usize]
                .m_bIsUsed
        );
        assert_eq!(
            app.world()
                .resource::<PhysicalAudioBindings>()
                .retired_slots,
            1
        );
    }
}
