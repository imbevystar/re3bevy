//! re3 reference: references/re-GTA/src/modelinfo/MloModelInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/MloModelInfo.h
//!
//! MLO model metadata and constructed-clump records.

pub use super::ModelInfo::{
    CMloInstance, CMloModelInfo, ConstructedMloAtomic, ConstructedMloClump,
};

impl CMloModelInfo {
    pub fn GetDrawDist(&self) -> f32 {
        self.draw_dist
    }

    pub fn SetDrawDist(&mut self, draw_dist: f32) {
        self.draw_dist = draw_dist;
    }

    pub fn GetInstanceRange(&self) -> (usize, usize) {
        (self.first_instance, self.last_instance)
    }

    pub fn SetInstanceRange(&mut self, first: usize, last: usize) {
        self.first_instance = first;
        self.last_instance = last;
    }

    pub fn ConstructClump(&mut self, atomics: Vec<ConstructedMloAtomic>) {
        self.constructed_clump = Some(ConstructedMloClump {
            atomic_instances: atomics,
        });
    }
}
