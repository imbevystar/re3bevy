//! re3 reference: references/re-GTA/src/core/MenuScreens.cpp
//!
//! Menu screen table metadata ported from the re3 `aScreens` source comments.

use crate::core::Frontend::{CMenuEntry, CMenuScreen, NUM_MENUROWS};

pub const RE3_MENU_SCREEN_COMMENTS: &[(&str, i32)] = &[
    ("MENUPAGE_NONE", 0),
    ("MENUPAGE_STATS", 1),
    ("MENUPAGE_NEW_GAME", 2),
    ("MENUPAGE_BRIEFS", 3),
    ("MENUPAGE_CONTROLLER_SETTINGS", 4),
    ("MENUPAGE_SOUND_SETTINGS", 5),
    ("MENUPAGE_DISPLAY_SETTINGS", 6),
    ("MENUPAGE_LANGUAGE_SETTINGS", 7),
    ("MENUPAGE_CHOOSE_LOAD_SLOT", 8),
    ("MENUPAGE_CHOOSE_DELETE_SLOT", 9),
    ("MENUPAGE_NEW_GAME_RELOAD", 10),
    ("MENUPAGE_LOAD_SLOT_CONFIRM", 11),
    ("MENUPAGE_DELETE_SLOT_CONFIRM", 12),
    ("MENUPAGE_NO_MEMORY_CARD", 13),
    ("MENUPAGE_LOADING_IN_PROGRESS", 14),
    ("MENUPAGE_DELETING_IN_PROGRESS", 15),
    ("MENUPAGE_PS2_LOAD_FAILED", 16),
    ("MENUPAGE_DELETE_FAILED", 17),
    ("MENUPAGE_DEBUG_MENU", 18),
    ("MENUPAGE_MEMORY_CARD_DEBUG", 19),
    ("MENUPAGE_MEMORY_CARD_TEST", 20),
    ("MENUPAGE_MULTIPLAYER_MAIN", 21),
    ("MENUPAGE_PS2_SAVE_FAILED", 22),
    ("MENUPAGE_PS2_SAVE_FAILED_2", 23),
    ("MENUPAGE_SAVE", 24),
    ("MENUPAGE_NO_MEMORY_CARD_2", 25),
    ("MENUPAGE_CHOOSE_SAVE_SLOT", 26),
    ("MENUPAGE_SAVE_OVERWRITE_CONFIRM", 27),
    ("MENUPAGE_MULTIPLAYER_MAP", 28),
    ("MENUPAGE_MULTIPLAYER_CONNECTION", 29),
    ("MENUPAGE_MULTIPLAYER_FIND_GAME", 30),
    ("MENUPAGE_MULTIPLAYER_MODE", 31),
    ("MENUPAGE_MULTIPLAYER_CREATE", 32),
    ("MENUPAGE_MULTIPLAYER_START", 33),
    ("MENUPAGE_SKIN_SELECT_OLD", 34),
    ("MENUPAGE_CONTROLLER_PC", 35),
    ("MENUPAGE_CONTROLLER_PC_OLD1", 36),
    ("MENUPAGE_CONTROLLER_PC_OLD2", 37),
    ("MENUPAGE_CONTROLLER_PC_OLD3", 38),
    ("MENUPAGE_CONTROLLER_PC_OLD4", 39),
    ("MENUPAGE_CONTROLLER_DEBUG", 40),
    ("MENUPAGE_OPTIONS", 41),
    ("MENUPAGE_EXIT", 42),
    ("MENUPAGE_SAVING_IN_PROGRESS", 43),
    ("MENUPAGE_SAVE_SUCCESSFUL", 44),
    ("MENUPAGE_DELETING", 45),
    ("MENUPAGE_DELETE_SUCCESS", 46),
    ("MENUPAGE_SAVE_FAILED", 47),
    ("MENUPAGE_LOAD_FAILED", 48),
    ("MENUPAGE_LOAD_FAILED_2", 49),
    ("MENUPAGE_FILTER_GAME", 50),
    ("MENUPAGE_START_MENU", 51),
    ("MENUPAGE_PAUSE_MENU", 52),
    ("MENUPAGE_CHOOSE_MODE", 53),
    ("MENUPAGE_SKIN_SELECT", 54),
    ("MENUPAGE_KEYBOARD_CONTROLS", 55),
    ("MENUPAGE_MOUSE_CONTROLS", 56),
    ("MENUPAGE_MISSION_RETRY", 57),
    ("MENUPAGE_UNK", 58),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Re3MenuScreenComment {
    pub page_name: String,
    pub page_id: i32,
}

pub fn re3_menu_screen_comments() -> Vec<Re3MenuScreenComment> {
    RE3_MENU_SCREEN_COMMENTS
        .iter()
        .map(|(page_name, page_id)| Re3MenuScreenComment {
            page_name: (*page_name).to_owned(),
            page_id: *page_id,
        })
        .collect()
}

pub fn declared_menu_page_count() -> usize {
    RE3_MENU_SCREEN_COMMENTS.len()
}

pub fn empty_screen(name: &str, previous_page: [i32; 2], parent_entry: [i32; 2]) -> CMenuScreen {
    CMenuScreen {
        screen_name: key8(name),
        unk: 1,
        previous_page,
        parent_entry,
        entries: std::array::from_fn(|_| CMenuEntry::default()),
    }
}

pub fn menu_entry(action: i32, name: &str, save_slot: i32, target_menu: i32) -> CMenuEntry {
    CMenuEntry {
        action,
        entry_name: key8(name),
        save_slot,
        target_menu,
    }
}

pub fn fill_entries(screen: &mut CMenuScreen, entries: &[CMenuEntry]) {
    for (index, entry) in entries.iter().take(NUM_MENUROWS).enumerate() {
        screen.entries[index] = entry.clone();
    }
}

fn key8(name: &str) -> [u8; 8] {
    let mut key = [0; 8];
    for (index, byte) in name.as_bytes().iter().take(8).enumerate() {
        key[index] = *byte;
    }
    key
}
