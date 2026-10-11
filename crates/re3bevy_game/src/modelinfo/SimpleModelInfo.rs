//! re3 reference: references/re-GTA/src/modelinfo/SimpleModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/SimpleModelInfo.h
//!
//! C++ file-name facade for `CSimpleModelInfo`.

pub use super::ModelInfo::{CSimpleModelInfo, SimpleAtomicBinding, SimpleModelFlags};

impl CSimpleModelInfo {
    pub fn Init(&mut self) {
        self.lod_distances = [0.0; 3];
        self.num_atomics = 0;
        self.first_damaged = 0;
        self.alpha = 0;
        self.flags = SimpleModelFlags::default();
        self.atomic_bindings.clear();
    }

    pub fn IncreaseAlpha(&mut self) {
        self.alpha = self.alpha.saturating_add(0x10);
    }

    pub fn SetNumAtomics(&mut self, n: u8) {
        self.num_atomics = n.min(3);
    }

    pub fn SetLodDistances(&mut self, distances: [f32; 3]) {
        self.lod_distances = distances;
    }

    pub fn GetLodDistance(&self, i: usize) -> Option<f32> {
        self.lod_distances.get(i).copied()
    }

    pub fn GetNearDistance(&self) -> f32 {
        self.lod_distances[2]
    }

    pub fn GetLargestLodDistance(&self) -> f32 {
        self.largest_lod_distance(1.0, false)
    }

    pub fn AtomicIndexFromDistance(&self, dist: f32) -> Option<usize> {
        self.atomic_index_from_distance(dist, 1.0, false)
    }

    pub fn largest_lod_distance(&self, lod_multiplier: f32, damaged: bool) -> f32 {
        let num_atomics = self.num_atomics as usize;
        if num_atomics == 0 {
            return 0.0;
        }
        let index = if self.first_damaged == 0 || damaged {
            num_atomics - 1
        } else {
            self.first_damaged.saturating_sub(1) as usize
        };
        self.lod_distances.get(index).copied().unwrap_or(0.0) * lod_multiplier
    }

    pub fn atomic_index_from_distance(
        &self,
        dist: f32,
        lod_multiplier: f32,
        damaged: bool,
    ) -> Option<usize> {
        let first = if damaged {
            self.first_damaged as usize
        } else {
            0
        };
        (first..self.num_atomics as usize)
            .find(|index| dist < self.lod_distances[*index] * lod_multiplier)
    }

    pub fn atomic_binding_from_distance(
        &self,
        dist: f32,
        lod_multiplier: f32,
        damaged: bool,
    ) -> Option<&SimpleAtomicBinding> {
        if self.atomic_bindings.is_empty() {
            return None;
        }
        // Past largest draw distance — re3 would not render.
        let far = self.largest_lod_distance(lod_multiplier, damaged);
        if far > 0.0 && dist >= far {
            return None;
        }
        if let Some(index) = self.atomic_index_from_distance(dist, lod_multiplier, damaged) {
            // re3 `CSimpleModelInfo::GetAtomicFromDistance` returns the exact
            // `m_atomics[index]` slot selected by the IDE distance table. A
            // missing slot is `nil`; substituting a neighbouring LOD can put a
            // coarse island/road atomic at the camera and create giant slabs.
            return self
                .atomic_bindings
                .iter()
                .find(|binding| binding.lod_index == index);
        }
        None
    }

    pub fn SetRelatedModel(&mut self, id: usize) {
        self.related_model_id = Some(id);
    }

    pub fn GetRelatedModel(&self) -> Option<usize> {
        self.related_model_id
    }
}
