//! re3 reference: references/re-GTA/src/renderer/TexList.cpp
//! re3 reference: references/re-GTA/src/renderer/TexList.h
//!
//! Global texture-list slot bookkeeping.

use bevy::prelude::Resource;

pub const MAX_TEXUSED: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CTexList {
    pub tex_used: [bool; MAX_TEXUSED],
    pub slots: Vec<Option<String>>,
    pub current_dir: String,
}

impl CTexList {
    pub fn initialise(&mut self) {}

    pub fn shutdown(&mut self) {}

    pub fn set_texture(&mut self, slot: i32, name: &str) -> Option<String> {
        let slot = usize::try_from(slot).ok()?;
        if slot >= MAX_TEXUSED {
            return None;
        }
        if self.slots.len() < MAX_TEXUSED {
            self.slots.resize(MAX_TEXUSED, None);
        }
        self.tex_used[slot] = true;
        self.slots[slot] = Some(name.to_owned());
        self.slots[slot].clone()
    }

    pub fn get_first_free_texture(&self) -> i32 {
        self.tex_used
            .iter()
            .position(|used| !*used)
            .map(|slot| slot as i32)
            .unwrap_or(-1)
    }

    pub fn load_file_name_texture(&mut self, name: &str) -> Option<String> {
        self.set_texture(self.get_first_free_texture(), name)
    }

    pub fn load_global_texture_list(&mut self) {
        self.current_dir = "TEXTURES".to_owned();
    }
}

impl Default for CTexList {
    fn default() -> Self {
        Self {
            tex_used: [false; MAX_TEXUSED],
            slots: vec![None; MAX_TEXUSED],
            current_dir: String::new(),
        }
    }
}
