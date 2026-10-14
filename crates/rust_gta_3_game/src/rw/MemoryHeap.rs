//! re3 reference: references/re-GTA/src/rw/MemoryHeap.cpp
//! re3 reference: references/re-GTA/src/rw/MemoryHeap.h
//!
//! Custom heap accounting model for re3 memory ids.

use bevy::prelude::Resource;

pub const MEMID_FREE: usize = 0;
pub const MEMID_GAME: usize = 1;
pub const MEMID_WORLD: usize = 2;
pub const MEMID_ANIMATION: usize = 3;
pub const MEMID_POOLS: usize = 4;
pub const MEMID_DEF_MODELS: usize = 5;
pub const MEMID_STREAM: usize = 6;
pub const MEMID_STREAM_MODELS: usize = 7;
pub const MEMID_STREAM_TEXUTRES: usize = 8;
pub const MEMID_TEXTURES: usize = 9;
pub const MEMID_COLLISION: usize = 10;
pub const MEMID_RENDERLIST: usize = 11;
pub const MEMID_GAME_PROCESS: usize = 12;
pub const MEMID_SCRIPT: usize = 13;
pub const MEMID_CARS: usize = 14;
pub const MEMID_RENDER: usize = 15;
pub const MEMID_FRONTEND: usize = 17;
pub const NUM_MEMIDS: usize = 18;
pub const NUM_FIXED_MEMBLOCKS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CMemoryHeap {
    pub total: usize,
    pub total_mem_used: usize,
    pub current_mem_id: usize,
    pub mem_used: [usize; NUM_MEMIDS],
    pub total_blocks_used: usize,
    pub blocks_used: [usize; NUM_MEMIDS],
    pub id_stack: Vec<usize>,
    pub fixed_sizes: [usize; NUM_FIXED_MEMBLOCKS],
    pub allocations: Vec<HeapAllocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeapAllocation {
    pub ptr_id: usize,
    pub size: usize,
    pub mem_id: usize,
}

impl CMemoryHeap {
    pub fn init(total: usize) -> Self {
        Self {
            total,
            total_mem_used: 0,
            current_mem_id: MEMID_GAME,
            mem_used: [0; NUM_MEMIDS],
            total_blocks_used: 0,
            blocks_used: [0; NUM_MEMIDS],
            id_stack: Vec::with_capacity(16),
            fixed_sizes: [0x10, 0x20, 0xE0, 0x60, 0x1C0, 0x50],
            allocations: Vec::new(),
        }
    }

    pub fn register_malloc(&mut self, ptr_id: usize, size: usize) {
        let size = align_16(size);
        let mem_id = self.current_mem_id.min(NUM_MEMIDS - 1);
        self.allocations.push(HeapAllocation {
            ptr_id,
            size,
            mem_id,
        });
        if mem_id != MEMID_FREE {
            self.total_mem_used += size;
            self.mem_used[mem_id] += size;
            self.blocks_used[mem_id] += 1;
            self.total_blocks_used += 1;
        }
    }

    pub fn register_free(&mut self, ptr_id: usize) {
        let Some(index) = self
            .allocations
            .iter()
            .position(|allocation| allocation.ptr_id == ptr_id)
        else {
            return;
        };
        let allocation = self.allocations.remove(index);
        if allocation.mem_id != MEMID_FREE {
            self.total_mem_used = self.total_mem_used.saturating_sub(allocation.size);
            self.mem_used[allocation.mem_id] =
                self.mem_used[allocation.mem_id].saturating_sub(allocation.size);
            self.blocks_used[allocation.mem_id] =
                self.blocks_used[allocation.mem_id].saturating_sub(1);
            self.total_blocks_used = self.total_blocks_used.saturating_sub(1);
        }
    }

    pub fn push_mem_id(&mut self, id: usize) {
        self.id_stack.push(self.current_mem_id);
        self.current_mem_id = id.min(NUM_MEMIDS - 1);
    }

    pub fn pop_mem_id(&mut self) {
        if let Some(id) = self.id_stack.pop() {
            self.current_mem_id = id;
        }
    }

    pub fn get_memory_used(&self, id: usize) -> usize {
        self.mem_used.get(id).copied().unwrap_or(0)
    }

    pub fn get_blocks_used(&self, id: usize) -> usize {
        self.blocks_used.get(id).copied().unwrap_or(0)
    }

    pub fn get_largest_free_block(&self) -> usize {
        self.total.saturating_sub(self.total_mem_used)
    }
}

impl Default for CMemoryHeap {
    fn default() -> Self {
        Self::init(128 * 1024 * 1024)
    }
}

fn align_16(size: usize) -> usize {
    if size & 0xF == 0 {
        size
    } else {
        (size & !0xF) + 0x10
    }
}
