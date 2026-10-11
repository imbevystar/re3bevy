//! re3 reference: references/re-GTA/src/core/Frontend.cpp
//! re3 reference: references/re-GTA/src/core/Frontend.h
//!
//! CMenuManager state and menu constants. Rendering is owned by the Bevy
//! frontend flow, while state names, action ids and default values remain
//! aligned with re3.

use std::time::Instant;

use bevy::prelude::*;

pub const NUM_MENUROWS: usize = 18;
pub const MENUSLIDER_BARS: i32 = 16;
pub const PLAYERSETUP_MAX_VISIBLE_ROWS: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendSkinInfo {
    pub skin_id: usize,
    pub original_name: String,
    pub displayed_name: String,
    pub date: String,
}
pub const CONTROL_STANDARD: i32 = 0;

/// Display Settings FPS options for `MENUACTION_FRAMELIMIT`. `0` means
/// uncapped. re3's `m_PrefsFrameLimiter` was a plain on/off toggle targeting
/// `RsGlobal.maxFPS` (30 Hz); the port exposes the cap directly instead.
pub const FPS_OPTIONS: [i32; 6] = [30, 60, 120, 240, 360, 0];
pub const DEFAULT_MAX_FPS: i32 = 0;

/// Normalises a stored cap onto [`FPS_OPTIONS`]; unknown values fall back to
/// [`DEFAULT_MAX_FPS`] instead of silently disabling the limiter.
pub fn normalise_max_fps(value: i32) -> i32 {
    if FPS_OPTIONS.contains(&value) {
        value
    } else {
        DEFAULT_MAX_FPS
    }
}

/// Steps through [`FPS_OPTIONS`] with wraparound. `direction >= 0` moves
/// forward, anything else moves backward.
pub fn cycle_max_fps(current: i32, direction: i32) -> i32 {
    let current = normalise_max_fps(current);
    let pos = FPS_OPTIONS
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let len = FPS_OPTIONS.len() as i32;
    let next = if direction >= 0 {
        (pos as i32 + 1).rem_euclid(len)
    } else {
        (pos as i32 - 1).rem_euclid(len)
    };
    FPS_OPTIONS[next as usize]
}

/// Right-column text for the frame limiter row. Numeric caps render plainly
/// (like the resolution row); uncapped renders as `UNLIMITED`.
pub fn max_fps_label(value: i32) -> String {
    match normalise_max_fps(value) {
        0 => "UNLIMITED".to_owned(),
        fps => fps.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum eMenuScreen {
    MENUPAGE_DISABLED = -1,
    MENUPAGE_NONE = 0,
    MENUPAGE_STATS = 1,
    MENUPAGE_NEW_GAME = 2,
    MENUPAGE_BRIEFS = 3,
    MENUPAGE_CONTROLLER_SETTINGS = 4,
    MENUPAGE_SOUND_SETTINGS = 5,
    MENUPAGE_DISPLAY_SETTINGS = 6,
    MENUPAGE_LANGUAGE_SETTINGS = 7,
    MENUPAGE_CHOOSE_LOAD_SLOT = 8,
    MENUPAGE_CHOOSE_DELETE_SLOT = 9,
    MENUPAGE_NEW_GAME_RELOAD = 10,
    MENUPAGE_LOAD_SLOT_CONFIRM = 11,
    MENUPAGE_DELETE_SLOT_CONFIRM = 12,
    MENUPAGE_LOADING_IN_PROGRESS = 14,
    MENUPAGE_DELETING_IN_PROGRESS = 15,
    MENUPAGE_PS2_LOAD_FAILED = 16,
    MENUPAGE_DELETE_FAILED = 17,
    MENUPAGE_PS2_SAVE_FAILED = 22,
    MENUPAGE_PS2_SAVE_FAILED_2 = 23,
    MENUPAGE_SAVE = 24,
    MENUPAGE_CHOOSE_SAVE_SLOT = 26,
    MENUPAGE_SAVE_OVERWRITE_CONFIRM = 27,
    MENUPAGE_CONTROLLER_PC = 35,
    MENUPAGE_OPTIONS = 41,
    MENUPAGE_SAVING_IN_PROGRESS = 43,
    MENUPAGE_SAVE_SUCCESSFUL = 44,
    MENUPAGE_DELETING = 45,
    MENUPAGE_DELETE_SUCCESS = 46,
    MENUPAGE_SAVE_FAILED = 47,
    MENUPAGE_LOAD_FAILED = 48,
    MENUPAGE_LOAD_FAILED_2 = 49,
    MENUPAGE_EXIT = 42,
    MENUPAGE_START_MENU = 51,
    MENUPAGE_PAUSE_MENU = 52,
    MENUPAGE_SKIN_SELECT = 54,
    MENUPAGE_KEYBOARD_CONTROLS = 55,
    MENUPAGE_MOUSE_CONTROLS = 56,
    MENUPAGE_COOP_SETTINGS = 60,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum eMenuAction {
    MENUACTION_NOTHING = 0,
    MENUACTION_LABEL = 1,
    MENUACTION_CHANGEMENU = 2,
    MENUACTION_CTRLVIBRATION = 3,
    MENUACTION_CTRLCONFIG = 4,
    MENUACTION_CTRLDISPLAY = 5,
    MENUACTION_FRAMESYNC = 6,
    MENUACTION_FRAMELIMIT = 7,
    MENUACTION_TRAILS = 8,
    MENUACTION_SUBTITLES = 9,
    MENUACTION_WIDESCREEN = 10,
    MENUACTION_BRIGHTNESS = 11,
    MENUACTION_DRAWDIST = 12,
    MENUACTION_MUSICVOLUME = 13,
    MENUACTION_SFXVOLUME = 14,
    MENUACTION_RADIO = 16,
    MENUACTION_LANG_ENG = 17,
    MENUACTION_LANG_FRE = 18,
    MENUACTION_LANG_GER = 19,
    MENUACTION_LANG_ITA = 20,
    MENUACTION_LANG_SPA = 21,
    MENUACTION_POPULATESLOTS_CHANGEMENU = 22,
    MENUACTION_CHECKSAVE = 23,
    MENUACTION_NEWGAME = 25,
    MENUACTION_RESUME_FROM_SAVEZONE = 49,
    MENUACTION_LOADRADIO = 101,
    MENUACTION_PLAYERSETUP = 97,
    MENUACTION_DONTCANCEL = 93,
    MENUACTION_CANCELGAME = 82,
    MENUACTION_RESUME = 92,
    MENUACTION_KEYBOARDCTRLS = 72,
    MENUACTION_GETKEY = 78,
    MENUACTION_SHOWHEADBOB = 79,
    MENUACTION_INVVERT = 81,
    MENUACTION_MOUSESENS = 84,
    MENUACTION_SCREENRES = 94,
    MENUACTION_AUDIOHW = 95,
    MENUACTION_SPEAKERCONF = 96,
    MENUACTION_RESTOREDEF = 98,
    MENUACTION_CTRLMETHOD = 99,
    MENUACTION_MOUSESTEER = 102,
    /// re3bevy mod action; kept outside the original re3 action table.
    MENUACTION_LOCAL_COOP = 103,
    /// re3bevy mod preference; kept outside the original re3 action table.
    MENUACTION_LOCAL_COOP_SPLIT = 104,
    /// re3bevy mod side selection action.
    MENUACTION_LOCAL_COOP_SIDE = 105,
    /// re3 FREE_CAM build option exposed by the custom PC display menu.
    MENUACTION_FREE_CAM = 106,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LANGUAGE {
    LANGUAGE_AMERICAN = 0,
    LANGUAGE_FRENCH = 1,
    LANGUAGE_GERMAN = 2,
    LANGUAGE_ITALIAN = 3,
    LANGUAGE_SPANISH = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CMenuEntry {
    pub action: i32,
    pub entry_name: [u8; 8],
    pub save_slot: i32,
    pub target_menu: i32,
}

impl Default for CMenuEntry {
    fn default() -> Self {
        Self {
            action: eMenuAction::MENUACTION_NOTHING as i32,
            entry_name: [0; 8],
            save_slot: 0,
            target_menu: eMenuScreen::MENUPAGE_NONE as i32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CMenuScreen {
    pub screen_name: [u8; 8],
    pub unk: i32,
    pub previous_page: [i32; 2],
    pub parent_entry: [i32; 2],
    pub entries: [CMenuEntry; NUM_MENUROWS],
}

impl Default for CMenuScreen {
    fn default() -> Self {
        Self {
            screen_name: [0; 8],
            unk: 0,
            previous_page: [eMenuScreen::MENUPAGE_NONE as i32; 2],
            parent_entry: [0; 2],
            entries: std::array::from_fn(|_| CMenuEntry::default()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FrontendIntent {
    MessageScreen(String),
    HelperText(i32),
    Slider { x: f32, y: f32, progress: f32 },
    RequestStartUp,
    RequestShutDown,
}

#[derive(Debug, Clone, Resource)]
pub struct CMenuManager {
    /// Live primary-window dimensions. Display Settings reports this native
    /// render target rather than staging a fake lower-resolution framebuffer.
    pub window_width: u32,
    pub window_height: u32,
    /// Platform video modes exposed by Bevy's primary monitor, deduplicated by size.
    pub video_modes: Vec<[u32; 2]>,
    pub display_video_mode: usize,
    pub prefs_video_mode: usize,
    pub apply_video_mode_requested: bool,
    pub requested_video_size: [u32; 2],
    pub prefs_audio_3d_provider_index: i8,
    pub key_change_not_processed: bool,
    pub skin_name: String,
    pub helper_text_msg_id: i32,
    pub language_loaded: bool,
    pub menu_active: bool,
    /// re3 `FrontEndMenuManager.m_bSaveMenuActive` (Frontend.h). Set by SCM
    /// COMMAND_ACTIVATE_SAVE_MENU (Script4.cpp:1944) and cleared by the save
    /// screen flow (Frontend.cpp:5736).
    pub save_menu_active: bool,
    pub menu_state_changed: bool,
    pub waiting_for_new_key_bind: bool,
    /// Current re3 `m_CurrCntrlAction` equivalent while the frontend captures a key.
    pub pending_control_label: Option<String>,
    pub want_to_restart: bool,
    pub first_time: bool,
    pub game_not_loaded: bool,
    pub mouse_pos_x: i32,
    pub mouse_pos_y: i32,
    pub show_mouse: bool,
    pub first_visible_row_on_list: i32,
    pub total_list_row: i32,
    pub selected_list_row: i32,
    /// Runtime rows for re3's native Statistics and Briefs list screens.
    pub plain_screen_rows: Vec<(String, String)>,
    pub curr_screen: i32,
    pub curr_option: i32,
    pub prev_screen: i32,
    pub curr_save_slot: i32,
    pub screen_change_delay_timer: i32,
    pub menu_fade_alpha: i32,
    pub hover_option: i32,
    pub prefs_use_vibration: i8,
    pub display_controller_on_foot: i8,
    pub prefs_use_wide_screen: i8,
    pub prefs_local_coop_split: i8,
    pub prefs_local_coop_swap_sides: i8,
    pub prefs_radio_station: i8,
    pub prefs_vsync: i8,
    pub prefs_vsync_disp: i8,
    /// Selected frame cap from [`FPS_OPTIONS`]; `0` means uncapped.
    pub prefs_max_fps: i32,
    pub prefs_show_subtitles: i8,
    pub prefs_speakers: i8,
    pub control_method: i32,
    pub prefs_dma: i8,
    pub prefs_language: i32,
    pub prefs_brightness: i32,
    pub prefs_lod: f32,
    pub front_end_reload_obr_txt_gxt: i8,
    pub prefs_music_volume: i32,
    pub prefs_sfx_volume: i32,
    pub prefs_skin_file: String,
    pub player_setup_skins: Vec<FrontendSkinInfo>,
    pub player_setup_skins_enumerated: bool,
    /// re3 `m_nCurrExLayer`: 0=list, 1=Use Skin, 2=Back.
    pub player_setup_focus: u8,
    pub player_setup_scrollbar_top_margin: f32,
    pub player_setup_repeat_direction: i8,
    pub player_setup_repeat_at: Option<Instant>,
    pub key_pressed_code: i32,
    pub start_up_front_end_requested: bool,
    pub shut_down_front_end_requested: bool,
    pub prefs_allow_nasty_game: bool,
    pub prefs_stereo_mono: u8,
    pub selected_map: i32,
    pub selected_game_type: i32,
    pub prefs_player_red: u8,
    pub prefs_player_green: u8,
    pub prefs_player_blue: u8,
    /// PC menu state owned by re3 systems but consumed through Bevy input.
    pub mouse_sensitivity: i32,
    pub mouse_invert: bool,
    pub disable_mouse_steering: bool,
    pub prefs_free_cam: bool,
    pub prefs_trails: bool,
    pub bottom_bar_active: bool,
    pub render_game_in_menu: bool,
    pub draw_intents: Vec<FrontendIntent>,
}

impl CMenuManager {
    pub fn initialise() -> Self {
        Self {
            window_width: 0,
            window_height: 0,
            video_modes: Vec::new(),
            display_video_mode: 0,
            prefs_video_mode: 0,
            apply_video_mode_requested: false,
            requested_video_size: [0, 0],
            prefs_audio_3d_provider_index: -1,
            key_change_not_processed: false,
            skin_name: "$$\"\"".to_owned(),
            helper_text_msg_id: 0,
            language_loaded: false,
            menu_active: false,
            save_menu_active: false,
            menu_state_changed: false,
            waiting_for_new_key_bind: false,
            pending_control_label: None,
            want_to_restart: false,
            first_time: true,
            game_not_loaded: true,
            mouse_pos_x: 0,
            mouse_pos_y: 0,
            show_mouse: false,
            first_visible_row_on_list: 0,
            total_list_row: 0,
            selected_list_row: 0,
            plain_screen_rows: Vec::new(),
            curr_screen: eMenuScreen::MENUPAGE_NONE as i32,
            curr_option: 0,
            prev_screen: eMenuScreen::MENUPAGE_NONE as i32,
            curr_save_slot: 0,
            screen_change_delay_timer: 0,
            menu_fade_alpha: 0,
            hover_option: -1,
            prefs_use_vibration: 1,
            display_controller_on_foot: 0,
            prefs_use_wide_screen: 0,
            prefs_local_coop_split: 0,
            prefs_local_coop_swap_sides: 0,
            prefs_radio_station: 0,
            // CMenuManager static defaults in Frontend.cpp are both true.
            // Allow opting into VSync via RE3_VSYNC=1 or the Display Settings menu.
            // By default, do not force a 60 Hz monitor sync lock.
            prefs_vsync: if cfg!(target_os = "android")
                || std::env::var("RE3_VSYNC").as_deref() == Ok("1")
            {
                1
            } else {
                0
            },
            prefs_vsync_disp: if cfg!(target_os = "android")
                || std::env::var("RE3_VSYNC").as_deref() == Ok("1")
            {
                1
            } else {
                0
            },
            prefs_max_fps: DEFAULT_MAX_FPS,
            prefs_show_subtitles: 1,
            prefs_speakers: 0,
            control_method: CONTROL_STANDARD,
            prefs_dma: 1,
            prefs_language: LANGUAGE::LANGUAGE_AMERICAN as i32,
            prefs_brightness: 256,
            prefs_lod: 1.2,
            front_end_reload_obr_txt_gxt: 0,
            prefs_music_volume: 102,
            prefs_sfx_volume: 102,
            prefs_skin_file: "$$\"\"".to_owned(),
            player_setup_skins: Vec::new(),
            player_setup_skins_enumerated: false,
            player_setup_focus: 0,
            player_setup_scrollbar_top_margin: 0.0,
            player_setup_repeat_direction: 0,
            player_setup_repeat_at: None,
            key_pressed_code: -1,
            start_up_front_end_requested: false,
            shut_down_front_end_requested: false,
            prefs_allow_nasty_game: true,
            prefs_stereo_mono: 0,
            selected_map: 0,
            selected_game_type: 0,
            prefs_player_red: 255,
            prefs_player_green: 255,
            prefs_player_blue: 255,
            mouse_sensitivity: 100,
            mouse_invert: true,
            disable_mouse_steering: true,
            prefs_free_cam: false,
            prefs_trails: true,
            bottom_bar_active: false,
            render_game_in_menu: false,
            draw_intents: Vec::new(),
        }
    }

    pub fn get_is_menu_active(&self) -> bool {
        self.menu_active
    }

    pub fn process(&mut self) {
        self.draw_intents.clear();
        if self.start_up_front_end_requested {
            self.menu_active = true;
            self.start_up_front_end_requested = false;
        }
        if self.shut_down_front_end_requested {
            self.menu_active = false;
            self.shut_down_front_end_requested = false;
        }
        self.screen_change_delay_timer = self.screen_change_delay_timer.saturating_sub(1);
    }

    pub fn display_slider(
        &mut self,
        x: f32,
        y: f32,
        _most_left_bar_size: f32,
        _most_right_bar_size: f32,
        _rect_size: f32,
        progress: f32,
    ) -> i32 {
        self.draw_intents
            .push(FrontendIntent::Slider { x, y, progress });
        (progress.clamp(0.0, 1.0) * MENUSLIDER_BARS as f32) as i32
    }

    pub fn set_helper_text(&mut self, text: i32) {
        self.helper_text_msg_id = text;
        self.draw_intents.push(FrontendIntent::HelperText(text));
    }

    pub fn reset_helper_text(&mut self) {
        self.helper_text_msg_id = 0;
    }

    pub fn request_front_end_start_up(&mut self) {
        self.start_up_front_end_requested = true;
        self.draw_intents.push(FrontendIntent::RequestStartUp);
    }

    pub fn request_front_end_shut_down(&mut self) {
        self.shut_down_front_end_requested = true;
        self.draw_intents.push(FrontendIntent::RequestShutDown);
    }

    pub fn switch_menu_on_and_off(&mut self) {
        self.menu_active = !self.menu_active;
        self.menu_state_changed = true;
        if self.menu_active && self.curr_screen == eMenuScreen::MENUPAGE_NONE as i32 {
            self.curr_screen = eMenuScreen::MENUPAGE_PAUSE_MENU as i32;
        }
    }

    pub fn check_hover(&self, x1: i32, x2: i32, y1: i32, y2: i32) -> bool {
        self.mouse_pos_x >= x1
            && self.mouse_pos_x <= x2
            && self.mouse_pos_y >= y1
            && self.mouse_pos_y <= y2
    }

    pub fn scroll_up_list_by_one(&mut self) {
        self.first_visible_row_on_list = self.first_visible_row_on_list.saturating_sub(1);
    }

    pub fn scroll_down_list_by_one(&mut self) {
        self.first_visible_row_on_list += 1;
    }

    pub fn page_up_list(&mut self) {
        self.first_visible_row_on_list = self
            .first_visible_row_on_list
            .saturating_sub(NUM_MENUROWS as i32);
    }

    pub fn page_down_list(&mut self) {
        self.first_visible_row_on_list += NUM_MENUROWS as i32;
    }

    pub fn message_screen(&mut self, text: &str) {
        self.draw_intents
            .push(FrontendIntent::MessageScreen(text.to_owned()));
    }
}

pub struct GtaFrontendPlugin;

impl Plugin for GtaFrontendPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            process_frontend.in_set(re3bevy_core::core::Schedules::GtaProcessSet::Frontend),
        );
    }
}

fn process_frontend(frontend: Option<ResMut<CMenuManager>>) {
    if let Some(mut frontend) = frontend {
        frontend.process();
    }
}
