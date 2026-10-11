//! re3 reference: references/re-GTA/src/extras/frontendoption.cpp
//! re3 reference: references/re-GTA/src/extras/frontendoption.h
//!
//! Custom frontend option registry.

use bevy::prelude::Resource;

pub const FEOPTION_ACTION_LEFT: i8 = 0;
pub const FEOPTION_ACTION_RIGHT: i8 = 1;
pub const FEOPTION_ACTION_SELECT: i8 = 2;
pub const FEOPTION_ACTION_FOCUSLOSS: i8 = 3;

pub const FESCREEN_CENTER: i8 = 0;
pub const FESCREEN_LEFT_ALIGN: i8 = 1;
pub const FESCREEN_RIGHT_ALIGN: i8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct CustomFrontendOptions {
    pub last_og_screen: i32,
    pub num_custom_frontend_options: i32,
    pub num_custom_frontend_screens: i32,
    pub option_cursor: i8,
    pub current_menu: i32,
    pub option_overwrite: bool,
    pub options: Vec<FrontendOption>,
    pub screens: Vec<FrontendScreen>,
    pub navigation: Vec<FrontendNavigationIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontendOption {
    BuiltinAction {
        screen: i32,
        index: i8,
        gxt_key: String,
        action: i32,
        target_menu: i32,
        save_slot: i32,
    },
    Select {
        screen: i32,
        index: i8,
        gxt_key: String,
        right_texts: Vec<String>,
        value: Option<i8>,
        only_apply_on_enter: bool,
        save_cat: Option<String>,
        save_key: Option<String>,
        disable_if_game_loaded: bool,
    },
    Dynamic {
        screen: i32,
        index: i8,
        gxt_key: String,
        value: Option<i8>,
        save_cat: Option<String>,
        save_key: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendScreen {
    pub id: u8,
    pub gxt_key: String,
    pub sprite: i32,
    pub prev_page: i32,
    pub column_width: i32,
    pub header_height: i32,
    pub line_height: i32,
    pub font: i8,
    pub alignment: i8,
    pub show_left_right_helper: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendNavigationIntent {
    ChangeScreen {
        screen: i32,
        option: i32,
        fade_in: bool,
    },
    GoBack {
        fade_in: bool,
    },
}

impl CustomFrontendOptions {
    pub fn change_screen(&mut self, screen: i32, option: i32, fade_in: bool) {
        self.navigation
            .push(FrontendNavigationIntent::ChangeScreen {
                screen,
                option,
                fade_in,
            });
    }

    pub fn go_back(&mut self, fade_in: bool) {
        self.navigation
            .push(FrontendNavigationIntent::GoBack { fade_in });
    }

    pub fn frontend_option_set_cursor(&mut self, screen: i32, option: i8, overwrite: bool) {
        self.current_menu = screen;
        self.option_cursor = option;
        self.option_overwrite = overwrite;
    }

    pub fn frontend_option_add_builtin_action(
        &mut self,
        gxt_key: &str,
        action: i32,
        target_menu: i32,
        save_slot: i32,
    ) {
        let index = self.register_new_option();
        self.options.push(FrontendOption::BuiltinAction {
            screen: self.current_menu,
            index,
            gxt_key: gxt_key.chars().take(8).collect(),
            action,
            target_menu,
            save_slot,
        });
    }

    pub fn frontend_screen_add(
        &mut self,
        gxt_key: &str,
        sprite: i32,
        prev_page: i32,
        column_width: i32,
        header_height: i32,
        line_height: i32,
        font: i8,
        alignment: i8,
        show_left_right_helper: bool,
    ) -> u8 {
        self.num_custom_frontend_screens += 1;
        let id = (self.last_og_screen + self.num_custom_frontend_screens) as u8;
        self.screens.push(FrontendScreen {
            id,
            gxt_key: gxt_key.chars().take(8).collect(),
            sprite,
            prev_page,
            column_width,
            header_height,
            line_height,
            font,
            alignment,
            show_left_right_helper,
        });
        id
    }

    fn register_new_option(&mut self) -> i8 {
        self.num_custom_frontend_options += 1;
        let index = self.option_cursor;
        self.option_cursor += 1;
        index
    }
}

impl Default for CustomFrontendOptions {
    fn default() -> Self {
        Self {
            last_og_screen: 0,
            num_custom_frontend_options: 0,
            num_custom_frontend_screens: 0,
            option_cursor: -2,
            current_menu: 0,
            option_overwrite: false,
            options: Vec::new(),
            screens: Vec::new(),
            navigation: Vec::new(),
        }
    }
}
