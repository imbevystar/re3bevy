//! re3 reference: references/re-GTA/src/control/NameGrid.cpp
//! re3 reference: references/re-GTA/src/control/NameGrid.h
//!
//! Area-name grid queried by scripts and HUD.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CNameGridCell {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CNameGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Option<CNameGridCell>>,
}

impl CNameGrid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![None; width.saturating_mul(height)],
        }
    }

    #[allow(non_snake_case)]
    pub fn SetName(&mut self, x: usize, y: usize, name: impl Into<String>) {
        if let Some(cell) = self
            .cells
            .get_mut(y.saturating_mul(self.width).saturating_add(x))
        {
            *cell = Some(CNameGridCell { name: name.into() });
        }
    }

    #[allow(non_snake_case)]
    pub fn GetName(&self, x: usize, y: usize) -> Option<&str> {
        self.cells
            .get(y.saturating_mul(self.width).saturating_add(x))
            .and_then(|cell| cell.as_ref())
            .map(|cell| cell.name.as_str())
    }
}
