//! Indexed SCM identity, including generation-bearing pool slot lookups.
//! re3 references: CPool::GetAt, GetIndex and GetJustIndex.

use crate::core::Pools::CPoolHandle;
use bevy::prelude::Entity;
use std::{
    collections::{HashMap, hash_map::Drain},
    ops::Deref,
};

/// Read-only map compatibility preserves existing iteration at explicit
/// lifecycle boundaries. All writes update secondary indexes together.
#[derive(Debug, Clone, Default)]
pub struct ScriptHandleIndex {
    handles: HashMap<i32, Entity>,
    entity_handles: HashMap<Entity, i32>,
    slot_handles: HashMap<usize, i32>,
}

impl Deref for ScriptHandleIndex {
    type Target = HashMap<i32, Entity>;
    fn deref(&self) -> &Self::Target {
        &self.handles
    }
}

impl ScriptHandleIndex {
    pub fn insert(&mut self, handle: i32, entity: Entity) -> Option<Entity> {
        let old_entity = self.handles.insert(handle, entity);
        if let Some(old_entity) = old_entity {
            if self.entity_handles.get(&old_entity) == Some(&handle) {
                self.entity_handles.remove(&old_entity);
            }
        }
        self.entity_handles.insert(entity, handle);
        self.slot_handles
            .insert(CPoolHandle(handle).index(), handle);
        old_entity
    }

    pub fn remove(&mut self, handle: &i32) -> Option<Entity> {
        let entity = self.handles.remove(handle)?;
        if self.entity_handles.get(&entity) == Some(handle) {
            self.entity_handles.remove(&entity);
        }
        let slot = CPoolHandle(*handle).index();
        if self.slot_handles.get(&slot) == Some(handle) {
            self.slot_handles.remove(&slot);
        }
        Some(entity)
    }

    pub fn clear(&mut self) {
        self.handles.clear();
        self.entity_handles.clear();
        self.slot_handles.clear();
    }

    pub fn drain(&mut self) -> Drain<'_, i32, Entity> {
        self.entity_handles.clear();
        self.slot_handles.clear();
        self.handles.drain()
    }

    pub fn handle_for_entity(&self, entity: Entity) -> Option<i32> {
        self.entity_handles.get(&entity).copied()
    }

    pub fn handle_for_slot(&self, slot: usize) -> Option<i32> {
        self.slot_handles.get(&slot).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;

    #[test]
    fn generations_replacement_removal_and_drain_preserve_lookup_ownership() {
        let mut world = World::new();
        let old_entity = world.spawn_empty().id();
        let new_entity = world.spawn_empty().id();
        let mut index = ScriptHandleIndex::default();
        index.insert(0x101, old_entity);
        index.insert(0x102, new_entity);
        assert_eq!(index.handle_for_slot(1), Some(0x102));
        index.remove(&0x101);
        assert_eq!(index.handle_for_slot(1), Some(0x102));
        assert_eq!(index.handle_for_entity(old_entity), None);
        index.insert(0x102, old_entity);
        assert_eq!(index.handle_for_entity(new_entity), None);
        assert_eq!(index.handle_for_entity(old_entity), Some(0x102));
        assert_eq!(index.drain().count(), 1);
        assert_eq!(index.handle_for_slot(1), None);
        assert_eq!(index.handle_for_entity(old_entity), None);
        index.insert(0x201, new_entity);
        index.clear();
        assert!(index.is_empty());
        assert_eq!(index.handle_for_entity(new_entity), None);
    }
}
