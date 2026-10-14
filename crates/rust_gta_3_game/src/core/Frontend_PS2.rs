//! re3 reference: references/re-GTA/src/core/Frontend_PS2.cpp
//! re3 reference: references/re-GTA/src/core/Frontend_PS2.h
//!
//! PS2 frontend page ids kept for layout parity checks.

use bevy::prelude::Resource;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum eFrontendPage {
    #[default]
    MENUPAGE_NONE,
    MENUPAGE_STATS,
    MENUPAGE_NEW_GAME,
    MENUPAGE_BRIEFS,
    MENUPAGE_CONTROLLER_PC,
    MENUPAGE_SOUND_SETTINGS,
    MENUPAGE_DISPLAY_SETTINGS,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Resource)]
pub struct CMenuManagerPS2 {
    pub current_page: eFrontendPage,
    pub previous_page: eFrontendPage,
}

impl CMenuManagerPS2 {
    #[allow(non_snake_case)]
    pub fn SwitchMenuOnAndOff(&mut self) {
        self.previous_page = self.current_page;
        self.current_page = if self.current_page == eFrontendPage::MENUPAGE_NONE {
            eFrontendPage::MENUPAGE_STATS
        } else {
            eFrontendPage::MENUPAGE_NONE
        };
    }
}
