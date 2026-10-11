//! re3 reference: references/re-GTA/src/rw/TexturePools.cpp
//! re3 reference: references/re-GTA/src/rw/TexturePools.h
//!
//! D3D texture-pool reuse policy represented with texture ids.

use bevy::prelude::Resource;

pub const MAX_TEXTURE_POOLS: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CTexturePool {
    pub format: u32,
    pub size: i32,
    pub levels: u32,
    pub textures_max: i32,
    pub textures_used: i32,
    pub textures_num: i32,
    pub textures: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CPaletteList {
    pub max: usize,
    pub data: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Resource, Default)]
pub struct TexturePools {
    pub pools: Vec<CTexturePool>,
    pub palette_list: CPaletteList,
    pub max_palette_index: i32,
    pub use_palette_index: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureDesc {
    pub texture_id: usize,
    pub width: i32,
    pub height: i32,
    pub levels: u32,
    pub format: u32,
}

impl CTexturePool {
    pub fn create(format: u32, size: i32, mipmap_levels: u32, num_textures: i32) -> Self {
        Self {
            format,
            size,
            levels: mipmap_levels,
            textures_max: num_textures,
            textures_used: 0,
            textures_num: 0,
            textures: Vec::with_capacity(num_textures.max(0) as usize),
        }
    }

    pub fn release(&mut self) {
        self.textures.clear();
        self.textures_num = 0;
        self.textures_used = 0;
    }

    pub fn find_texture(&mut self) -> Option<usize> {
        if self.textures_num == 0 {
            return None;
        }
        self.textures_used -= 1;
        self.textures_num -= 1;
        self.textures.pop()
    }

    pub fn add_texture(&mut self, texture: usize) -> bool {
        self.textures_used += 1;
        if self.textures_num >= self.textures_max {
            return false;
        }
        self.textures.push(texture);
        self.textures_num += 1;
        true
    }

    pub fn resize(&mut self, num_textures: i32) {
        if num_textures == self.textures_max {
            return;
        }
        self.textures_max = num_textures;
        self.textures.truncate(num_textures.max(0) as usize);
        self.textures_num = self.textures.len() as i32;
    }

    pub fn get_size(&self) -> i32 {
        self.size
    }
}

impl CPaletteList {
    pub fn alloc(&mut self, max: usize) {
        self.max = max;
        self.data.clear();
        self.data.reserve(max);
    }

    pub fn free(&mut self) {
        self.data.clear();
        self.max = 0;
    }

    pub fn find(&mut self) -> Option<i32> {
        self.data.pop()
    }

    pub fn add(&mut self, item: i32) {
        if self.data.len() >= self.max {
            self.resize(2 * self.max.max(1));
        }
        self.data.push(item);
    }

    pub fn resize(&mut self, max: usize) {
        self.max = max;
        self.data.truncate(max);
    }
}

impl TexturePools {
    pub fn initialise(&mut self) {
        self.palette_list.alloc(100);
        self.max_palette_index = 0;
        self.use_palette_index = true;
    }

    pub fn shutdown(&mut self) {
        for pool in &mut self.pools {
            pool.release();
        }
        self.pools.clear();
        self.use_palette_index = false;
        self.palette_list.free();
    }

    pub fn create_texture(
        &mut self,
        width: i32,
        height: i32,
        levels: u32,
        format: u32,
    ) -> Option<usize> {
        if width == height {
            for pool in &mut self.pools {
                if width == pool.get_size() && levels == pool.levels && format == pool.format {
                    if let Some(texture) = pool.find_texture() {
                        return Some(texture);
                    }
                }
            }
        }
        None
    }

    pub fn release_texture(&mut self, desc: TextureDesc) -> bool {
        let levels = if desc.levels > 1 { 0 } else { 1 };
        if desc.width == desc.height {
            for pool in &mut self.pools {
                if desc.width == pool.get_size()
                    && desc.format == pool.format
                    && levels == pool.levels
                {
                    if !pool.add_texture(desc.texture_id) {
                        if pool.textures_used > 3 * pool.textures_max / 2 {
                            pool.resize(2 * pool.textures_max.max(1));
                            pool.textures_used -= 1;
                            pool.add_texture(desc.texture_id);
                        } else {
                            return false;
                        }
                    }
                    return true;
                }
            }
        }

        if self.pools.len() < MAX_TEXTURE_POOLS
            && self.use_palette_index
            && levels != 0
            && desc.width == desc.height
            && matches!(desc.width, 64 | 128 | 256)
        {
            let mut pool = CTexturePool::create(desc.format, desc.width, 1, 16);
            pool.add_texture(desc.texture_id);
            self.pools.push(pool);
            return true;
        }

        false
    }

    pub fn find_available_palette_index(&mut self) -> i32 {
        self.palette_list.find().unwrap_or_else(|| {
            let index = self.max_palette_index;
            self.max_palette_index += 1;
            index
        })
    }

    pub fn add_available_palette_index(&mut self, index: i32) {
        if self.use_palette_index {
            self.palette_list.add(index);
        }
    }
}
