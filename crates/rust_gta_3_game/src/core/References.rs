//! re3 reference: references/re-GTA/src/core/References.cpp
//! re3 reference: references/re-GTA/src/core/References.h
//!
//! Compatibility calculation model for the native 800-node reference pool.
//! It does not own live actor links; typed Bevy relationships own those links.
//! TODO(parity): CEntity::RegisterReference / ResolveReferences and
//! CReferences::RemoveReferencesToPlayer remaining actor interfaces must use
//! canonical typed Entity identities rather than unqualified numeric handles.

use std::collections::HashMap;

use bevy::prelude::Resource;

pub const NUMREFERENCES: usize = 800;
pub type ReferenceEntityHandle = u32;
pub type ReferenceRelationKey = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CReference {
    pub next: Option<usize>,
    pub relation: Option<ReferenceRelationKey>,
}

#[derive(Debug, Clone, Resource)]
pub struct CReferences {
    pub a_refs: [CReference; NUMREFERENCES],
    pub empty_list_head: Option<usize>,
    /// Compatibility relation values, not synchronized copies of actor fields.
    pub relation_values: HashMap<ReferenceRelationKey, Option<ReferenceEntityHandle>>,
    /// `CEntity::m_pFirstReference`, indexed by the referenced pool handle.
    pub entity_first_reference: HashMap<ReferenceEntityHandle, usize>,
}

impl CReferences {
    pub fn init() -> Self {
        let mut a_refs = [CReference::default(); NUMREFERENCES];
        for (index, slot) in a_refs.iter_mut().enumerate().take(NUMREFERENCES - 1) {
            slot.next = Some(index + 1);
        }
        Self {
            a_refs,
            empty_list_head: Some(0),
            relation_values: HashMap::new(),
            entity_first_reference: HashMap::new(),
        }
    }

    /// Assigns a compatibility relation and calculates native registration.
    /// Buildings must be filtered by the caller, just as the C++ method exits
    /// before touching the global pool for building entities.
    pub fn register_reference(
        &mut self,
        target: ReferenceEntityHandle,
        relation: ReferenceRelationKey,
        target_is_building: bool,
    ) -> bool {
        self.relation_values.insert(relation, Some(target));
        if target_is_building {
            return false;
        }

        let mut cursor = self.entity_first_reference.get(&target).copied();
        while let Some(index) = cursor {
            if self.a_refs[index].relation == Some(relation) {
                return false;
            }
            cursor = self.a_refs[index].next;
        }

        let Some(index) = self.empty_list_head else {
            return false;
        };
        self.empty_list_head = self.a_refs[index].next;
        self.a_refs[index].relation = Some(relation);
        self.a_refs[index].next = self.entity_first_reference.get(&target).copied();
        self.entity_first_reference.insert(target, index);
        true
    }

    pub fn set_relation_value(
        &mut self,
        relation: ReferenceRelationKey,
        value: Option<ReferenceEntityHandle>,
    ) {
        self.relation_values.insert(relation, value);
    }

    pub fn relation_value(&self, relation: ReferenceRelationKey) -> Option<ReferenceEntityHandle> {
        self.relation_values.get(&relation).copied().flatten()
    }

    /// `CEntity::ResolveReferences`: null matching relation values, then splice
    /// the entity's complete reference list back onto `pEmptyList`.
    pub fn resolve_references(&mut self, target: ReferenceEntityHandle) {
        let Some(head) = self.entity_first_reference.remove(&target) else {
            return;
        };
        let mut cursor = Some(head);
        let mut tail = head;
        while let Some(index) = cursor {
            if let Some(relation) = self.a_refs[index].relation
                && self.relation_value(relation) == Some(target)
            {
                self.relation_values.insert(relation, None);
            }
            tail = index;
            cursor = self.a_refs[index].next;
        }
        self.a_refs[tail].next = self.empty_list_head;
        self.empty_list_head = Some(head);
        self.clear_relations_in_free_chain(head, tail);
    }

    /// `CEntity::PruneReferences`: retain nodes whose relation still points at
    /// this entity and individually return stale nodes to the global free list.
    pub fn prune_references(&mut self, target: ReferenceEntityHandle) {
        let mut cursor = self.entity_first_reference.get(&target).copied();
        let mut retained_head: Option<usize> = None;
        let mut retained_tail: Option<usize> = None;
        while let Some(index) = cursor {
            let next = self.a_refs[index].next;
            let keep = self.a_refs[index]
                .relation
                .and_then(|relation| self.relation_value(relation))
                == Some(target);
            if keep {
                if retained_head.is_none() {
                    retained_head = Some(index);
                }
                if let Some(tail) = retained_tail {
                    self.a_refs[tail].next = Some(index);
                }
                retained_tail = Some(index);
            } else {
                self.a_refs[index].relation = None;
                self.a_refs[index].next = self.empty_list_head;
                self.empty_list_head = Some(index);
            }
            cursor = next;
        }
        if let Some(tail) = retained_tail {
            self.a_refs[tail].next = None;
            self.entity_first_reference
                .insert(target, retained_head.unwrap());
        } else {
            self.entity_first_reference.remove(&target);
        }
    }

    pub fn prune_all_references_in_world(
        &mut self,
        ped_handles: impl IntoIterator<Item = ReferenceEntityHandle>,
        vehicle_handles: impl IntoIterator<Item = ReferenceEntityHandle>,
        object_handles: impl IntoIterator<Item = ReferenceEntityHandle>,
    ) {
        for handle in ped_handles {
            self.prune_references(handle);
        }
        for handle in vehicle_handles {
            self.prune_references(handle);
        }
        for handle in object_handles {
            self.prune_references(handle);
        }
    }

    fn clear_relations_in_free_chain(&mut self, head: usize, tail: usize) {
        let mut cursor = Some(head);
        while let Some(index) = cursor {
            self.a_refs[index].relation = None;
            if index == tail {
                break;
            }
            cursor = self.a_refs[index].next;
        }
    }
}

impl Default for CReferences {
    fn default() -> Self {
        Self::init()
    }
}
