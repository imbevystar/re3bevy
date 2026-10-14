//! re3 reference: references/re-GTA/src/rw/TxdStore.cpp
//! re3 reference: references/re-GTA/src/rw/TxdStore.h
//!
//! Texture dictionary slot store. RenderWare texture dictionaries are tracked
//! as load-state markers until the RW texture reader is ported.

use crate::core::Streaming::TXDSTORESIZE;
use rust_gta_3_formats::formats::TexRead::RwTexDictionarySummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxdLoadState {
    Empty,
    Started,
    Loaded,
}

#[derive(Debug, Clone)]
pub struct TxdDef {
    pub load_state: TxdLoadState,
    pub ref_count: i32,
    pub name: String,
    pub texture_summary: Option<RwTexDictionarySummary>,
    /// Monotonically increasing identity for the resident RW dictionary.
    ///
    /// re3 destroys and recreates `RwTexDictionary*` values when streaming a
    /// TXD out and back in. Renderer caches must therefore key residency by
    /// dictionary identity, not by texture count: two different dictionaries
    /// can legitimately contain the same number of textures.
    pub generation: u64,
    /// Monotonic content revision inside one resident dictionary identity.
    ///
    /// re3's two-phase `StartLoadTxd`/`FinishLoadTxd` keeps the same
    /// `RwTexDictionary*`, but `FinishLoadTxd` appends the second half of the
    /// rasters to that dictionary. Renderer caches therefore need a content
    /// revision in addition to pointer/dictionary identity; otherwise a cache
    /// observed between the two phases stays permanently half-populated.
    pub texture_revision: u64,
}

impl TxdDef {
    fn new(name: &str, generation: u64) -> Self {
        Self {
            load_state: TxdLoadState::Empty,
            ref_count: 0,
            name: name.to_owned(),
            texture_summary: None,
            generation,
            texture_revision: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CTxdStore {
    slots: Vec<Option<TxdDef>>,
    stored_txd: Option<usize>,
    generation_counter: u64,
    pub store_revision: u64,
}

impl Default for CTxdStore {
    fn default() -> Self {
        Self::initialise()
    }
}

impl CTxdStore {
    pub fn initialise() -> Self {
        Self {
            slots: vec![None; TXDSTORESIZE],
            stored_txd: None,
            generation_counter: 0,
            store_revision: 0,
        }
    }

    #[inline(always)]
    pub fn store_revision(&self) -> u64 {
        self.store_revision
    }

    #[inline(always)]
    pub fn bump_revision(&mut self) {
        self.store_revision = self.store_revision.wrapping_add(1);
    }

    pub fn shutdown(&mut self) {
        self.slots.fill(None);
        self.stored_txd = None;
        self.bump_revision();
    }

    pub fn game_shutdown(&mut self) {
        for slot in 0..self.slots.len() {
            if self.slots[slot].is_some() && self.get_num_refs(slot) == 0 {
                self.remove_txd_slot(slot);
            }
        }
    }

    pub fn add_txd_slot(&mut self, name: &str) -> usize {
        if let Some(slot) = self.find_txd_slot(name) {
            return slot;
        }

        let slot = self
            .slots
            .iter()
            .position(Option::is_none)
            .expect("CTxdStore pool is full");
        let generation = self.next_generation();
        self.slots[slot] = Some(TxdDef::new(name, generation));
        slot
    }

    pub fn remove_txd_slot(&mut self, slot: usize) {
        self.slots[slot] = None;
        self.bump_revision();
    }

    pub fn find_txd_slot(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.as_ref()
                .is_some_and(|def| def.name.eq_ignore_ascii_case(name))
        })
    }

    pub fn get_txd_name(&self, slot: usize) -> Option<&str> {
        self.slots
            .get(slot)
            .and_then(Option::as_ref)
            .map(|def| def.name.as_str())
    }

    pub fn push_current_txd(&mut self, slot: usize) {
        self.stored_txd = Some(slot);
    }

    pub fn pop_current_txd(&mut self) -> Option<usize> {
        self.stored_txd.take()
    }

    pub fn create(&mut self, slot: usize) {
        let generation = self.next_generation();
        let def = self.slot_mut(slot);
        def.load_state = TxdLoadState::Loaded;
        def.generation = generation;
        def.texture_revision = def.texture_revision.wrapping_add(1);
    }

    pub fn get_num_refs(&self, slot: usize) -> i32 {
        self.slots
            .get(slot)
            .and_then(Option::as_ref)
            .map(|def| def.ref_count)
            .unwrap_or(0)
    }

    pub fn add_ref(&mut self, slot: usize) {
        self.slot_mut(slot).ref_count += 1;
    }

    pub fn remove_ref_without_delete(&mut self, slot: usize) {
        self.slot_mut(slot).ref_count -= 1;
    }

    pub fn remove_ref(&mut self, slot: usize) -> bool {
        let def = self.slot_mut(slot);
        def.ref_count -= 1;
        def.ref_count <= 0
    }

    pub fn mark_loaded(&mut self, slot: usize, summary: Option<RwTexDictionarySummary>) {
        // `StartLoadTxd` allocates the new `RwTexDictionary*` and
        // `FinishLoadTxd` fills that same object. Preserve its identity across
        // the two streaming phases. A direct LoadTxd/replacement, however,
        // creates a new resident dictionary and must advance the generation.
        let generation = if self
            .slot(slot)
            .is_some_and(|def| def.load_state == TxdLoadState::Started)
        {
            self.slot(slot)
                .expect("CTxdStore slot is not allocated")
                .generation
        } else {
            self.next_generation()
        };
        let def = self.slot_mut(slot);
        def.load_state = TxdLoadState::Loaded;
        def.texture_summary = summary;
        def.generation = generation;
        // `FinishLoadTxd` mutates the existing dictionary by adding its
        // remaining native textures. Keep `generation` stable for dictionary
        // identity, while making that renderer-visible content change explicit.
        def.texture_revision = def.texture_revision.wrapping_add(1);
        self.bump_revision();
    }

    pub fn mark_started(&mut self, slot: usize, summary: Option<RwTexDictionarySummary>) {
        let generation = self.next_generation();
        let def = self.slot_mut(slot);
        def.load_state = TxdLoadState::Started;
        def.texture_summary = summary;
        def.generation = generation;
        def.texture_revision = def.texture_revision.wrapping_add(1);
        self.bump_revision();
    }

    pub fn remove_txd(&mut self, slot: usize) {
        let should_bump = self
            .slots
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|def| {
                def.load_state != TxdLoadState::Empty || def.texture_summary.is_some()
            });
        let generation = should_bump.then(|| self.next_generation());
        let Some(def) = self.slots.get_mut(slot).and_then(Option::as_mut) else {
            return;
        };
        def.load_state = TxdLoadState::Empty;
        def.texture_summary = None;
        def.texture_revision = def.texture_revision.wrapping_add(1);
        if let Some(generation) = generation {
            def.generation = generation;
        }
    }

    pub fn is_txd_loaded(&self, slot: usize) -> bool {
        self.slots
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|def| def.load_state == TxdLoadState::Loaded)
    }

    pub fn slot(&self, slot: usize) -> Option<&TxdDef> {
        self.slots.get(slot).and_then(Option::as_ref)
    }

    fn slot_mut(&mut self, slot: usize) -> &mut TxdDef {
        self.slots[slot]
            .as_mut()
            .expect("CTxdStore slot is not allocated")
    }

    fn next_generation(&mut self) -> u64 {
        self.bump_revision();
        self.generation_counter = self.generation_counter.wrapping_add(1);
        if self.generation_counter == 0 {
            self.generation_counter = 1;
        }
        self.generation_counter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txd_generation_changes_across_resident_dictionary_replacements() {
        let mut store = CTxdStore::initialise();
        let slot = store.add_txd_slot("roads");
        let allocated = store.slot(slot).expect("allocated slot").generation;

        store.mark_started(slot, None);
        let started_def = store.slot(slot).expect("started slot");
        let started = started_def.generation;
        let started_revision = started_def.texture_revision;
        assert!(started > allocated);

        store.mark_loaded(slot, None);
        let loaded_def = store.slot(slot).expect("loaded slot");
        let loaded = loaded_def.generation;
        assert_eq!(loaded, started);
        assert!(loaded_def.texture_revision > started_revision);

        store.remove_txd(slot);
        let removed = store.slot(slot).expect("removed slot marker").generation;
        assert!(removed > loaded);

        store.mark_loaded(slot, None);
        let reloaded = store.slot(slot).expect("reloaded slot").generation;
        assert!(reloaded > removed);

        store.remove_txd_slot(slot);
        let reused_slot = store.add_txd_slot("roads2");
        assert_eq!(reused_slot, slot);
        assert!(
            store.slot(reused_slot).expect("reused slot").generation > reloaded,
            "slot reuse must not recycle an old renderer-visible TXD identity"
        );
    }
}
