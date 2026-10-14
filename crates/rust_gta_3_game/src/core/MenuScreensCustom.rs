//! re3 reference: references/re-GTA/src/core/MenuScreensCustom.cpp
//!
//! Custom frontend option screen metadata ported from re3 preprocessor macros.

pub const CUSTOM_FRONTEND_OPTION_MACROS: &[&str] = &[
    "DENSITY_SLIDERS",
    "VIDEOMODE_SELECTOR",
    "MULTISAMPLING_SELECTOR",
    "POSTFX_SELECTORS",
    "CUTSCENE_BORDERS_TOGGLE",
    "FREE_CAM_TOGGLE",
    "DUALPASS_SELECTOR",
    "ISLAND_LOADING_SELECTOR",
    "INVERT_PAD_SELECTOR",
    "SELECT_CONTROLLER_TYPE",
];

#[derive(Debug, Clone, PartialEq)]
pub struct CCustomScreenLayout {
    pub sprite: i32,
    pub column_width: i32,
    pub header_height: i32,
    pub line_height: i32,
    pub font: i8,
    pub alignment: i8,
    pub show_left_right_helper: bool,
    pub font_scale_x: f32,
    pub font_scale_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomFrontendOptionKind {
    Select {
        save_category: Option<String>,
        save_key: Option<String>,
        num_right_texts: i8,
        only_apply_on_enter: bool,
        disable_if_game_loaded: bool,
    },
    Slider {
        save_category: Option<String>,
        save_key: Option<String>,
        min: f32,
        max: f32,
    },
    Dynamic {
        save_category: Option<String>,
        save_key: Option<String>,
    },
}

pub fn custom_frontend_option_macro_names() -> Vec<String> {
    CUSTOM_FRONTEND_OPTION_MACROS
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}
