//! re3 reference: references/re-GTA/src/rw/MemoryMgr.cpp
//! re3 reference: references/re-GTA/src/rw/MemoryMgr.h
//!
//! RenderWare memory function accounting.

use bevy::prelude::Resource;

use super::MemoryHeap::CMemoryHeap;

#[derive(Debug, Clone, PartialEq, Eq, Resource, Default)]
pub struct MemoryMgr {
    pub heap: CMemoryHeap,
    pub memory_top: usize,
    next_ptr_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBlock {
    pub ptr_id: usize,
    pub size: usize,
    pub align: Option<usize>,
}

impl MemoryMgr {
    pub fn init_memory_mgr() -> Self {
        Self::default()
    }

    pub fn malloc(&mut self, size: usize) -> MemoryBlock {
        self.next_ptr_id += 1;
        self.heap.register_malloc(self.next_ptr_id, size);
        self.memory_top = self.memory_top.max(self.next_ptr_id + size);
        MemoryBlock {
            ptr_id: self.next_ptr_id,
            size,
            align: None,
        }
    }

    pub fn calloc(&mut self, num: usize, size: usize) -> MemoryBlock {
        self.malloc(num.saturating_mul(size))
    }

    pub fn realloc(&mut self, block: MemoryBlock, size: usize) -> MemoryBlock {
        self.free(block);
        self.malloc(size)
    }

    pub fn free(&mut self, block: MemoryBlock) {
        self.heap.register_free(block.ptr_id);
    }

    pub fn rw_malloc_align(&mut self, size: usize, align: usize) -> MemoryBlock {
        let mut block = self.malloc(size + align);
        block.align = Some(align);
        block
    }

    pub fn rw_free_align(&mut self, block: MemoryBlock) {
        self.free(block);
    }
}
