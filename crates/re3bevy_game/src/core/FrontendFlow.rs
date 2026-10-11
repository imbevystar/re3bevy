//! re3 reference: references/re-GTA/src/core/MenuScreens.cpp
//! re3 reference: references/re-GTA/src/core/Frontend.cpp
//!
//! Bevy UI adapter for re3's frontend. The page/action graph comes from
//! `aScreens`; the PC frontend background and logo sprites come from the
//! original `menu.txd` file and are laid out with re3 menu coordinates.

use std::{
    collections::BTreeMap,
    fs,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    input::mouse::MouseWheel,
    picking::{Pickable, hover::Hovered},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::{Pressed, RelativeCursorPosition, widget::NodeImageMode},
    ui_widgets::Button,
    window::{CursorOptions, Monitor, PresentMode, PrimaryMonitor, PrimaryWindow},
    winit::{UpdateMode, WinitSettings},
};
use re3bevy_assets::{
    assets::FrontendAssets::CFrontendAssets, install::OriginalGtaInstall::OriginalGtaInstall,
};
use re3bevy_core::core::GameState::GtaAppState;
use re3bevy_formats::formats::TexRead::{decode_txd_texture, read_txd};

use super::{
    ControllerConfig::{CControllerConfigManager, e_ControllerAction, eControllerType},
    Frontend::{
        CMenuManager, DEFAULT_MAX_FPS, FrontendSkinInfo, MENUSLIDER_BARS,
        PLAYERSETUP_MAX_VISIBLE_ROWS, cycle_max_fps, eMenuAction, eMenuScreen, max_fps_label,
        normalise_max_fps,
    },
};
use crate::text::Text::{CText, unicode_to_ascii};
use crate::{
    audio::{
        DMAudio::cDMAudio,
        audio_enums::MUSICMODE_FRONTEND,
        soundlist::{
            SOUND_FRONTEND_FAIL, SOUND_FRONTEND_MENU_BACK, SOUND_FRONTEND_MENU_NAVIGATION,
            SOUND_FRONTEND_MENU_NEW_PAGE, SOUND_FRONTEND_MENU_SETTING_CHANGE,
            SOUND_FRONTEND_MENU_STARTING,
        },
    },
    core::{
        LocalCoop::{LocalCoopSession, LocalCoopSplitLayout},
        PlayerInfo::CPlayerInfo,
        Stats::CStats,
        Timer::CTimer,
    },
    renderer::{
        Font::{FONT_BANK, FONT_HEADING, FONT_SIZE},
        MBlur::CMBlur,
        PlayerSkin::{CPlayerSkin, DEFAULT_SKIN_NAME},
    },
    save::{
        GenericGameStorage::CGenericGameStorage,
        MemoryCard::{CMemoryCard, MAX_SLOTS, MemoryCardError, SLOTINFO},
        PCSave::C_PcSave,
        SaveRuntime::{PcSaveRequest, PcSaveRequestState},
    },
    text::Messages::CMessages,
};

const DEFAULT_SCREEN_WIDTH: f32 = 640.0;
const DEFAULT_SCREEN_HEIGHT: f32 = 448.0;

const HEADER: Color = Color::srgb(0.0, 0.0, 0.0);
const NORMAL: Color = Color::srgb(235.0 / 255.0, 170.0 / 255.0, 50.0 / 255.0);
const SELECTED: Color = Color::srgb(1.0, 217.0 / 255.0, 106.0 / 255.0);
const DISABLED: Color = Color::srgb(155.0 / 255.0, 117.0 / 255.0, 6.0 / 255.0);
const MENU_TEXT_SHADOW: [u8; 4] = [0, 0, 0, 90];
const MENU_TEXT_SHADOW_OFFSET: f32 = 1.0;
const SELECTION_BAR: Color = Color::srgba(100.0 / 255.0, 200.0 / 255.0, 50.0 / 255.0, 50.0 / 255.0);
const SLIDER_ON: Color = Color::srgb(1.0, 217.0 / 255.0, 106.0 / 255.0);
const SLIDER_OFF: Color = Color::srgb(185.0 / 255.0, 120.0 / 255.0, 0.0);
const SLIDER_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 200.0 / 255.0);
const MENUSLIDER_WIDTH: f32 = 256.0;

#[derive(Debug, Clone, Copy)]
struct MenuEntry {
    action: i32,
    key: &'static str,
    save_slot: i32,
    target: i32,
}

#[derive(Debug, Clone, Copy)]
struct MenuPage {
    title: &'static str,
    previous_when_not_loaded: i32,
    entries: &'static [MenuEntry],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendTransitionInput {
    Up,
    Down,
    Select,
    Back,
    ResetStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendTransitionOutcome {
    None,
    ChangeScreen,
    PopulateSlots,
    CheckSave,
    BlockedEmptySlot,
    BlockedCorruptedSlot,
    NewGame,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendTransitionSnapshot {
    pub curr_screen: i32,
    pub prev_screen: i32,
    pub curr_option: i32,
    pub curr_save_slot: i32,
    pub menu_active: bool,
    pub game_not_loaded: bool,
    pub want_to_restart: bool,
    pub entry_action: i32,
    pub entry_key: &'static str,
    pub entry_target: i32,
    pub entry_save_slot: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootFrontendStateStep {
    pub index: usize,
    pub cpp_state: &'static str,
    pub rust_state: &'static str,
    pub event: &'static str,
    pub action: &'static str,
    pub movie: &'static str,
    pub next_cpp_state: &'static str,
    pub next_rust_state: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootFrontendIntent {
    pub order: usize,
    pub intent: &'static str,
    pub value: &'static str,
    pub numeric: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootFrontendSequenceSnapshot {
    pub movies_enabled: bool,
    pub language: &'static str,
    pub logo_movie: &'static str,
    pub intro_movie: &'static str,
    pub game_initialise_before_frontend: bool,
    pub level_load_deferred_until: &'static str,
    pub dat_file: &'static str,
    pub states: Vec<BootFrontendStateStep>,
    pub frontend_idle_calls: Vec<&'static str>,
    pub load_all_textures_intents: Vec<BootFrontendIntent>,
    pub frontend: FrontendTransitionSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrontendLayoutItem {
    pub order: usize,
    pub kind: &'static str,
    pub texture: &'static str,
    pub text_key: &'static str,
    pub entry_index: i32,
    pub selected: bool,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub x: f32,
    pub y: f32,
    pub style: i16,
    pub scale_x: f32,
    pub scale_y: f32,
    pub align: &'static str,
    pub colour: [u8; 4],
    pub shadow_colour: [u8; 4],
    pub foreground_offset_x: f32,
    pub foreground_offset_y: f32,
}

#[derive(Component)]
pub struct FrontendRoot;

#[derive(Component)]
pub struct FrontendButton {
    entry_index: usize,
}

#[derive(Component)]
pub struct FrontendBitmapText {
    entry_index: usize,
    selected_only: bool,
    text: String,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    shadow: bool,
    value_column: bool,
    logical_left: f32,
    logical_top: f32,
    align: TextAlign,
    /// Last colour the node was rendered with. Together with `text` and the
    /// render scale below it exactly determines the cached image handle and
    /// node rect, so an unchanged triple lets `refresh_text` skip the
    /// handle lookup and component writes (and the UI change-detection
    /// churn they cause) without any staleness risk.
    last_colour: Option<Color>,
    last_render_scale: Option<[u32; 2]>,
}

#[derive(Component)]
pub struct FrontendSelectionBar {
    entry_index: usize,
}

#[derive(Component)]
pub struct FrontendMouseCursor;

#[derive(Component)]
pub struct FrontendSliderBar {
    entry_index: usize,
    bar_index: usize,
}

#[derive(Component)]
pub struct FrontendSliderHitArea {
    entry_index: usize,
    logical_left: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerSetupControl {
    SkinRow(usize),
    UseSkin,
    Back,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
}

#[derive(Component)]
pub(crate) struct FrontendPlayerSetupControl(PlayerSetupControl);

#[derive(Resource, Default)]
pub struct FrontendMenuUi {
    pub root: Option<Entity>,
}

/// Shared settings state for every boot route.
///
/// re3 loads its persisted frontend preferences before gameplay starts; they
/// are global runtime settings rather than state owned by the visible menu.
/// Keeping this in a Resource lets the normal frontend and the direct New Game
/// route share one load/save snapshot instead of each system instance carrying
/// an independent Local copy.
#[derive(Resource, Default)]
pub struct FrontendSettingsSyncState {
    loaded: bool,
    last_payload: String,
}

#[derive(Resource)]
pub struct FrontendTxdTextures {
    pub loaded: bool,
    pub hud_loaded: bool,
    pub images: BTreeMap<String, Handle<Image>>,
    pub(crate) bitmaps: BTreeMap<String, RgbaTexture>,
    text_images: BTreeMap<String, Handle<Image>>,
    // CFont renders its glyph quads directly in physical screen coordinates.
    // Bitmap UI text must therefore be produced at this scale, not rasterized
    // at 640x448 and filtered a second time by an ImageNode stretch.
    text_render_scale_x: f32,
    text_render_scale_y: f32,
}

impl FrontendTxdTextures {
    pub(crate) fn text_scale(&self) -> (f32, f32) {
        (self.text_render_scale_x, self.text_render_scale_y)
    }

    /// Reuse an original frontend TXD image in isolated diagnostic UI without
    /// exposing the texture cache's mutable internals.
    pub fn image_handle(&self, name: &str) -> Option<Handle<Image>> {
        self.images.get(&name.to_ascii_lowercase()).cloned()
    }
}

impl Default for FrontendTxdTextures {
    fn default() -> Self {
        Self {
            loaded: false,
            hud_loaded: false,
            images: BTreeMap::new(),
            bitmaps: BTreeMap::new(),
            text_images: BTreeMap::new(),
            text_render_scale_x: 1.0,
            text_render_scale_y: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RgbaTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const START_MENU_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEN_STA",
        eMenuScreen::MENUPAGE_NEW_GAME,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_OPT",
        eMenuScreen::MENUPAGE_OPTIONS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_QT",
        eMenuScreen::MENUPAGE_EXIT,
    ),
];

const NEW_GAME_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FES_SNG",
        eMenuScreen::MENUPAGE_NEW_GAME_RELOAD,
    ),
    entry(
        eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU,
        "GMLOAD",
        eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT,
    ),
    entry(
        eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU,
        "FES_DGA",
        eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
    // Mod extension is appended after every authored action so every original
    // option index remains unchanged for parity and save-menu navigation.
    entry(
        eMenuAction::MENUACTION_LOCAL_COOP,
        "2 PLAYER GAME",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "2 PLAYER SETTINGS",
        eMenuScreen::MENUPAGE_COOP_SETTINGS,
    ),
];

const NEW_GAME_RELOAD_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FESZ_QR",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_NO",
        eMenuScreen::MENUPAGE_NEW_GAME,
    ),
    entry(
        eMenuAction::MENUACTION_NEWGAME,
        "FEM_YES",
        eMenuScreen::MENUPAGE_NEW_GAME_RELOAD,
    ),
];

const LOAD_SLOT_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FESZ_CA",
        eMenuScreen::MENUPAGE_NEW_GAME,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL0",
        0,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL1",
        1,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL2",
        2,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL3",
        3,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL4",
        4,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL5",
        5,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL6",
        6,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHECKSAVE,
        "FEM_SL7",
        7,
        eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
    ),
];

const DELETE_SLOT_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FESZ_CA",
        eMenuScreen::MENUPAGE_NEW_GAME,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL0",
        0,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL1",
        1,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL2",
        2,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL3",
        3,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL4",
        4,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL5",
        5,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL6",
        6,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL7",
        7,
        eMenuScreen::MENUPAGE_DELETE_SLOT_CONFIRM,
    ),
];

const SAVE_SLOT_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_RESUME_FROM_SAVEZONE,
        "FESZ_CA",
        eMenuScreen::MENUPAGE_NONE,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL1",
        0,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL2",
        1,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL3",
        2,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL4",
        3,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL5",
        4,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL6",
        5,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL7",
        6,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
    slot_entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_SL8",
        7,
        eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM,
    ),
];

const SAVE_OVERWRITE_CONFIRM_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FESZ_QO",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_YES",
        eMenuScreen::MENUPAGE_SAVING_IN_PROGRESS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_NO",
        eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT,
    ),
];

const SAVING_IN_PROGRESS_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_LABEL,
    "FES_WAR",
    eMenuScreen::MENUPAGE_NONE,
)];

const SAVE_SUCCESS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FES_SSC",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_RESUME_FROM_SAVEZONE,
        "FEC_OKK",
        eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT,
    ),
];

const SAVE_FAILED_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FEC_SVU",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEC_OKK",
        eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT,
    ),
];

const LOAD_SLOT_CONFIRM_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FESZ_QL",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_NO",
        eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_YES",
        eMenuScreen::MENUPAGE_LOADING_IN_PROGRESS,
    ),
];

const DELETE_SLOT_CONFIRM_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FESZ_QD",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_NO",
        eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_YES",
        eMenuScreen::MENUPAGE_DELETING,
    ),
];

const LOADING_IN_PROGRESS_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_LABEL,
    "FED_LDW",
    eMenuScreen::MENUPAGE_LOAD_SLOT_CONFIRM,
)];

const DELETING_IN_PROGRESS_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_LABEL,
    "FEDL_WR",
    eMenuScreen::MENUPAGE_NONE,
)];

const DELETING_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_LABEL,
    "FED_DLW",
    eMenuScreen::MENUPAGE_NONE,
)];

const DELETE_SUCCESS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "DEL_FNM",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEC_OKK",
        eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT,
    ),
];

const PLAIN_SCREEN_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_CHANGEMENU,
    "FEDS_TB",
    eMenuScreen::MENUPAGE_NONE,
)];

const SAVE_LOAD_ERROR_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FES_LOE",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const OPTIONS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_CTL",
        eMenuScreen::MENUPAGE_CONTROLLER_PC,
    ),
    entry(
        eMenuAction::MENUACTION_LOADRADIO,
        "FET_AUD",
        eMenuScreen::MENUPAGE_SOUND_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_DIS",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_LAN",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_PLAYERSETUP,
        "FET_PSU",
        eMenuScreen::MENUPAGE_SKIN_SELECT,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

// `aScreens` entries for the PC frontend.  These used to be collapsed into a
// one-row "back" page, which made every real setting unreachable.
const CONTROLLER_PC_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_CTRLMETHOD,
        "FET_CME",
        eMenuScreen::MENUPAGE_CONTROLLER_PC,
    ),
    entry(
        eMenuAction::MENUACTION_KEYBOARDCTRLS,
        "FET_RDK",
        eMenuScreen::MENUPAGE_KEYBOARD_CONTROLS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_AMS",
        eMenuScreen::MENUPAGE_MOUSE_CONTROLS,
    ),
    entry(
        eMenuAction::MENUACTION_RESTOREDEF,
        "FET_DEF",
        eMenuScreen::MENUPAGE_CONTROLLER_PC,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const PLAYER_SETUP_ENTRIES: &[MenuEntry] = &[entry(
    eMenuAction::MENUACTION_CHANGEMENU,
    "FEDS_TB",
    eMenuScreen::MENUPAGE_OPTIONS,
)];

const KEYBOARD_CONTROL_ENTRIES: &[MenuEntry] = &[
    getkey_entry("FIRE WEAPON"),
    getkey_entry("NEXT WEAPON"),
    getkey_entry("PREVIOUS WEAPON"),
    getkey_entry("GO FORWARD"),
    getkey_entry("GO BACK"),
    getkey_entry("GO LEFT"),
    getkey_entry("GO RIGHT"),
    getkey_entry("SNIPER ZOOM IN"),
    getkey_entry("SNIPER ZOOM OUT"),
    getkey_entry("ENTER / EXIT VEHICLE"),
    getkey_entry("CHANGE CAMERA"),
    getkey_entry("JUMP"),
    getkey_entry("SPRINT"),
    getkey_entry("LOOK BEHIND"),
    getkey_entry("LOCK TARGET"),
    getkey_entry("ACCELERATE"),
    getkey_entry("BRAKE / REVERSE"),
    getkey_entry("CHANGE RADIO STATION"),
    getkey_entry("HORN"),
    getkey_entry("TOGGLE SUBMISSIONS"),
    getkey_entry("HANDBRAKE"),
    getkey_entry("VEHICLE LOOK LEFT"),
    getkey_entry("VEHICLE LOOK RIGHT"),
    getkey_entry("PREVIOUS TARGET"),
    getkey_entry("NEXT TARGET"),
    getkey_entry("FIRST PERSON LOOK LEFT"),
    getkey_entry("FIRST PERSON LOOK RIGHT"),
    getkey_entry("FIRST PERSON LOOK UP"),
    getkey_entry("FIRST PERSON LOOK DOWN"),
    getkey_entry("VEHICLE LOOK BEHIND"),
    getkey_entry("TURRET LEFT"),
    getkey_entry("TURRET RIGHT"),
    getkey_entry("TURRET UP"),
    getkey_entry("TURRET DOWN"),
    getkey_entry("CENTER CAMERA"),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_CONTROLLER_PC,
    ),
];

const MOUSE_CONTROL_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_MOUSESENS,
        "FEC_MSH",
        eMenuScreen::MENUPAGE_MOUSE_CONTROLS,
    ),
    entry(
        eMenuAction::MENUACTION_INVVERT,
        "FEC_IVV",
        eMenuScreen::MENUPAGE_MOUSE_CONTROLS,
    ),
    entry(
        eMenuAction::MENUACTION_MOUSESTEER,
        "FET_MST",
        eMenuScreen::MENUPAGE_MOUSE_CONTROLS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_CONTROLLER_PC,
    ),
];

const SOUND_SETTINGS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_MUSICVOLUME,
        "FEA_MUS",
        eMenuScreen::MENUPAGE_SOUND_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_SFXVOLUME,
        "FEA_SFX",
        eMenuScreen::MENUPAGE_SOUND_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_RADIO,
        "FEA_RSS",
        eMenuScreen::MENUPAGE_SOUND_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_RESTOREDEF,
        "FET_DEF",
        eMenuScreen::MENUPAGE_SOUND_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const DISPLAY_SETTINGS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_BRIGHTNESS,
        "FED_BRI",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_DRAWDIST,
        "FEM_LOD",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_FRAMESYNC,
        "FEM_VSC",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_FRAMELIMIT,
        "FEM_FRM",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_TRAILS,
        "FED_TRA",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_SUBTITLES,
        "FED_SUB",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_WIDESCREEN,
        "FED_WIS",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_SCREENRES,
        "FED_RES",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_RESTOREDEF,
        "FET_DEF",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
    // Mod preference is appended so all authored re3 option indices remain stable.
    entry(
        eMenuAction::MENUACTION_LOCAL_COOP_SPLIT,
        "SPLIT SCREEN",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_FREE_CAM,
        "FREE CAM",
        eMenuScreen::MENUPAGE_DISPLAY_SETTINGS,
    ),
];

const LANGUAGE_SETTINGS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LANG_ENG,
        "FEL_ENG",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_LANG_FRE,
        "FEL_FRE",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_LANG_GER,
        "FEL_GER",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_LANG_ITA,
        "FEL_ITA",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_LANG_SPA,
        "FEL_SPA",
        eMenuScreen::MENUPAGE_LANGUAGE_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const PAUSE_MENU_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_RESUME,
        "FEM_RES",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEN_STA",
        eMenuScreen::MENUPAGE_NEW_GAME,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEP_STA",
        eMenuScreen::MENUPAGE_STATS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEP_BRI",
        eMenuScreen::MENUPAGE_BRIEFS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FET_OPT",
        eMenuScreen::MENUPAGE_OPTIONS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "2 PLAYER SETTINGS",
        eMenuScreen::MENUPAGE_COOP_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEM_QT",
        eMenuScreen::MENUPAGE_EXIT,
    ),
];

const COOP_SETTINGS_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LOCAL_COOP_SIDE,
        "TARAFINI SEC",
        eMenuScreen::MENUPAGE_COOP_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_LOCAL_COOP_SPLIT,
        "SPLIT SCREEN",
        eMenuScreen::MENUPAGE_COOP_SETTINGS,
    ),
    entry(
        eMenuAction::MENUACTION_CHANGEMENU,
        "FEDS_TB",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const EXIT_ENTRIES: &[MenuEntry] = &[
    entry(
        eMenuAction::MENUACTION_LABEL,
        "FEQ_SRE",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_DONTCANCEL,
        "FEM_NO",
        eMenuScreen::MENUPAGE_NONE,
    ),
    entry(
        eMenuAction::MENUACTION_CANCELGAME,
        "FEM_YES",
        eMenuScreen::MENUPAGE_NONE,
    ),
];

const fn entry(action: eMenuAction, key: &'static str, target: eMenuScreen) -> MenuEntry {
    MenuEntry {
        action: action as i32,
        key,
        save_slot: -1,
        target: target as i32,
    }
}

const fn getkey_entry(key: &'static str) -> MenuEntry {
    MenuEntry {
        action: eMenuAction::MENUACTION_GETKEY as i32,
        key,
        save_slot: -1,
        target: eMenuScreen::MENUPAGE_KEYBOARD_CONTROLS as i32,
    }
}

const fn slot_entry(
    action: eMenuAction,
    key: &'static str,
    save_slot: i32,
    target: eMenuScreen,
) -> MenuEntry {
    MenuEntry {
        action: action as i32,
        key,
        save_slot,
        target: target as i32,
    }
}

fn menu_page(screen: i32) -> MenuPage {
    match screen {
        1 => MenuPage {
            title: "FET_STA",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_PAUSE_MENU as i32,
            entries: PLAIN_SCREEN_ENTRIES,
        },
        2 => MenuPage {
            title: "FET_SGA",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NONE as i32,
            entries: NEW_GAME_ENTRIES,
        },
        3 => MenuPage {
            title: "FET_BRE",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_PAUSE_MENU as i32,
            entries: PLAIN_SCREEN_ENTRIES,
        },
        5 => MenuPage {
            title: "FET_AUD",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_OPTIONS as i32,
            entries: SOUND_SETTINGS_ENTRIES,
        },
        6 => MenuPage {
            title: "FET_DIS",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_OPTIONS as i32,
            entries: DISPLAY_SETTINGS_ENTRIES,
        },
        7 => MenuPage {
            title: "FET_LAN",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_OPTIONS as i32,
            entries: LANGUAGE_SETTINGS_ENTRIES,
        },
        35 => MenuPage {
            title: "FET_CTL",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_OPTIONS as i32,
            entries: CONTROLLER_PC_ENTRIES,
        },
        54 => MenuPage {
            title: "FET_PSU",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_OPTIONS as i32,
            entries: PLAYER_SETUP_ENTRIES,
        },
        55 => MenuPage {
            title: "FET_STI",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CONTROLLER_PC as i32,
            entries: KEYBOARD_CONTROL_ENTRIES,
        },
        56 => MenuPage {
            title: "FET_MTI",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CONTROLLER_PC as i32,
            entries: MOUSE_CONTROL_ENTRIES,
        },
        8 => MenuPage {
            title: "FET_LG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NEW_GAME as i32,
            entries: LOAD_SLOT_ENTRIES,
        },
        9 => MenuPage {
            title: "FET_DG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NEW_GAME as i32,
            entries: DELETE_SLOT_ENTRIES,
        },
        10 => MenuPage {
            title: "FET_NG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NEW_GAME as i32,
            entries: NEW_GAME_RELOAD_ENTRIES,
        },
        11 => MenuPage {
            title: "FET_LG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT as i32,
            entries: LOAD_SLOT_CONFIRM_ENTRIES,
        },
        12 => MenuPage {
            title: "FET_DG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32,
            entries: DELETE_SLOT_CONFIRM_ENTRIES,
        },
        14 => MenuPage {
            title: "FET_LG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_DISABLED as i32,
            entries: LOADING_IN_PROGRESS_ENTRIES,
        },
        15 => MenuPage {
            title: "FET_DG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_DISABLED as i32,
            entries: DELETING_IN_PROGRESS_ENTRIES,
        },
        24 | 26 => MenuPage {
            title: "FET_SG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_DISABLED as i32,
            entries: SAVE_SLOT_ENTRIES,
        },
        27 => MenuPage {
            title: "FET_SG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32,
            entries: SAVE_OVERWRITE_CONFIRM_ENTRIES,
        },
        43 => MenuPage {
            title: "",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32,
            entries: SAVING_IN_PROGRESS_ENTRIES,
        },
        44 => MenuPage {
            title: "FET_SG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32,
            entries: SAVE_SUCCESS_ENTRIES,
        },
        45 => MenuPage {
            title: "FET_DG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32,
            entries: DELETING_ENTRIES,
        },
        46 => MenuPage {
            title: "FET_DG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32,
            entries: DELETE_SUCCESS_ENTRIES,
        },
        47 => MenuPage {
            title: "FET_SG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32,
            entries: SAVE_FAILED_ENTRIES,
        },
        16 | 17 | 22 | 23 | 48 | 49 => MenuPage {
            title: "FET_SG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NEW_GAME as i32,
            entries: SAVE_LOAD_ERROR_ENTRIES,
        },
        41 => MenuPage {
            title: "FET_OPT",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NONE as i32,
            entries: OPTIONS_ENTRIES,
        },
        52 => MenuPage {
            title: "FET_PAU",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_DISABLED as i32,
            entries: PAUSE_MENU_ENTRIES,
        },
        60 => MenuPage {
            title: "2 PLAYER SETTINGS",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NEW_GAME as i32,
            entries: COOP_SETTINGS_ENTRIES,
        },
        42 => MenuPage {
            title: "FET_QG",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_NONE as i32,
            entries: EXIT_ENTRIES,
        },
        _ => MenuPage {
            title: "FEM_MM",
            previous_when_not_loaded: eMenuScreen::MENUPAGE_DISABLED as i32,
            entries: START_MENU_ENTRIES,
        },
    }
}

pub fn initialise_frontend_transition_menu(game_not_loaded: bool) -> CMenuManager {
    let mut menu = CMenuManager::initialise();
    menu.menu_active = true;
    menu.game_not_loaded = game_not_loaded;
    change_screen_state(
        &mut menu,
        eMenuScreen::MENUPAGE_START_MENU as i32,
        0,
        false,
        true,
    );
    menu
}

pub fn frontend_transition_snapshot(menu: &CMenuManager) -> FrontendTransitionSnapshot {
    let entry = current_entry(menu);
    FrontendTransitionSnapshot {
        curr_screen: menu.curr_screen,
        prev_screen: menu.prev_screen,
        curr_option: menu.curr_option,
        curr_save_slot: menu.curr_save_slot,
        menu_active: menu.menu_active,
        game_not_loaded: menu.game_not_loaded,
        want_to_restart: menu.want_to_restart,
        entry_action: entry.action,
        entry_key: entry.key,
        entry_target: entry.target,
        entry_save_slot: entry.save_slot,
    }
}

pub fn boot_to_frontend_state_sequence_snapshot(
    french_or_german: bool,
) -> BootFrontendSequenceSnapshot {
    let menu = initialise_frontend_transition_menu(true);
    let intro_movie = if french_or_german {
        "movies\\GTAtitlesGER.mpg"
    } else {
        "movies\\GTAtitles.mpg"
    };

    BootFrontendSequenceSnapshot {
        movies_enabled: true,
        language: if french_or_german {
            "french_or_german"
        } else {
            "american"
        },
        logo_movie: "movies\\Logo.mpg",
        intro_movie,
        game_initialise_before_frontend: false,
        level_load_deferred_until: "GS_INIT_PLAYING_GAME",
        dat_file: "DATA\\GTA3.DAT",
        states: vec![
            BootFrontendStateStep {
                index: 0,
                cpp_state: "GS_START_UP",
                rust_state: "Boot",
                event: "app_start",
                action: "CGame::InitialiseOnceBeforeRW",
                movie: "",
                next_cpp_state: "GS_INIT_LOGO_MPEG",
                next_rust_state: "LogoMpeg",
            },
            BootFrontendStateStep {
                index: 1,
                cpp_state: "GS_INIT_LOGO_MPEG",
                rust_state: "LogoMpeg",
                event: "enter_logo",
                action: "PlayMovieInWindow",
                movie: "movies\\Logo.mpg",
                next_cpp_state: "GS_LOGO_MPEG",
                next_rust_state: "LogoMpeg",
            },
            BootFrontendStateStep {
                index: 2,
                cpp_state: "GS_LOGO_MPEG",
                rust_state: "LogoMpeg",
                event: "logo_finished",
                action: "advance",
                movie: "",
                next_cpp_state: "GS_INIT_INTRO_MPEG",
                next_rust_state: "IntroMpeg",
            },
            BootFrontendStateStep {
                index: 3,
                cpp_state: "GS_INIT_INTRO_MPEG",
                rust_state: "IntroMpeg",
                event: "enter_intro",
                action: "PlayMovieInWindow",
                movie: intro_movie,
                next_cpp_state: "GS_INTRO_MPEG",
                next_rust_state: "IntroMpeg",
            },
            BootFrontendStateStep {
                index: 4,
                cpp_state: "GS_INTRO_MPEG",
                rust_state: "IntroMpeg",
                event: "intro_finished",
                action: "advance",
                movie: "",
                next_cpp_state: "GS_INIT_ONCE",
                next_rust_state: "Boot",
            },
            BootFrontendStateStep {
                index: 5,
                cpp_state: "GS_INIT_ONCE",
                rust_state: "Boot",
                event: "initialise_once_after_rw",
                action: "CGame::InitialiseOnceAfterRW",
                movie: "",
                next_cpp_state: "GS_INIT_FRONTEND",
                next_rust_state: "Frontend",
            },
            BootFrontendStateStep {
                index: 6,
                cpp_state: "GS_INIT_FRONTEND",
                rust_state: "Frontend",
                event: "frontend_startup_request",
                action: "m_bGameNotLoaded=true;m_bStartUpFrontEndRequested=true",
                movie: "",
                next_cpp_state: "GS_FRONTEND",
                next_rust_state: "Frontend",
            },
            BootFrontendStateStep {
                index: 7,
                cpp_state: "GS_FRONTEND",
                rust_state: "Frontend",
                event: "rsFRONTENDIDLE",
                action: "FrontendIdle",
                movie: "",
                next_cpp_state: "GS_FRONTEND",
                next_rust_state: "Frontend",
            },
        ],
        frontend_idle_calls: vec![
            "CTimer::Update",
            "CSprite2d::SetRecipNearClip",
            "CSprite2d::InitPerFrame",
            "CFont::InitPerFrame",
            "CPad::UpdatePads",
            "CMenuManager::Process",
            "CMenuManager::DrawFrontEnd",
            "DoFade",
            "Render2dStuffAfterFade",
            "DoRWStuffEndOfFrame",
        ],
        load_all_textures_intents: vec![
            BootFrontendIntent {
                order: 0,
                intent: "ChangeMusicMode",
                value: "MUSICMODE_FRONTEND",
                numeric: MUSICMODE_FRONTEND,
            },
            BootFrontendIntent {
                order: 1,
                intent: "PlayFrontEndSound",
                value: "SOUND_FRONTEND_MENU_STARTING",
                numeric: SOUND_FRONTEND_MENU_STARTING,
            },
            BootFrontendIntent {
                order: 2,
                intent: "SetCurrentOption",
                value: "0",
                numeric: 0,
            },
            BootFrontendIntent {
                order: 3,
                intent: "LoadTxd",
                value: "MODELS/FRONTEND.TXD",
                numeric: 0,
            },
            BootFrontendIntent {
                order: 4,
                intent: "LoadTxd",
                value: "MODELS/MENU.TXD",
                numeric: 0,
            },
            BootFrontendIntent {
                order: 5,
                intent: "SpritesLoaded",
                value: "true",
                numeric: 1,
            },
        ],
        frontend: frontend_transition_snapshot(&menu),
    }
}

pub fn frontend_main_menu_layout_snapshot(curr_option: i32) -> Vec<FrontendLayoutItem> {
    let screen = eMenuScreen::MENUPAGE_START_MENU as i32;
    let page = menu_page(screen);
    let layout = menu_layout(screen);
    let selected_option = curr_option.clamp(0, page.entries.len().saturating_sub(1) as i32);
    let mut items = Vec::new();
    push_layout_item(
        &mut items,
        FrontendLayoutItem {
            kind: "sprite",
            texture: menu_background_sprite(screen),
            left: 0.0,
            top: 0.0,
            right: DEFAULT_SCREEN_WIDTH,
            bottom: DEFAULT_SCREEN_HEIGHT,
            colour: [255, 255, 255, 255],
            ..layout_item_defaults(0)
        },
    );
    push_layout_item(
        &mut items,
        FrontendLayoutItem {
            kind: "sprite",
            texture: "gtalogo128",
            left: 225.0,
            top: 40.0,
            right: 415.0,
            bottom: 210.0,
            colour: [255, 255, 255, 255],
            ..layout_item_defaults(0)
        },
    );
    push_layout_item(
        &mut items,
        FrontendLayoutItem {
            kind: "page_title",
            text_key: page.title,
            x: DEFAULT_SCREEN_WIDTH - 35.0,
            y: DEFAULT_SCREEN_HEIGHT - 93.0,
            style: FONT_HEADING,
            scale_x: 0.84,
            scale_y: 1.6,
            align: "right",
            colour: [0, 0, 0, 255],
            ..layout_item_defaults(0)
        },
    );

    let mut option_row = 0usize;
    for (entry_index, entry) in page.entries.iter().enumerate() {
        if !is_selectable(*entry) {
            continue;
        }
        let y = layout.entry_y(option_row);
        option_row += 1;
        let selected = entry_index as i32 == selected_option;
        if selected {
            push_layout_item(
                &mut items,
                FrontendLayoutItem {
                    kind: "selection_bar",
                    entry_index: entry_index as i32,
                    selected,
                    left: 10.0,
                    top: y - 2.0,
                    right: DEFAULT_SCREEN_WIDTH - 11.0,
                    bottom: y + layout.usable_line_height(),
                    colour: [100, 200, 50, 50],
                    ..layout_item_defaults(0)
                },
            );
        }
        let colour = if selected {
            [255, 217, 106, 255]
        } else {
            [235, 170, 50, 255]
        };
        push_layout_item(
            &mut items,
            FrontendLayoutItem {
                kind: "entry_text",
                text_key: entry.key,
                entry_index: entry_index as i32,
                selected,
                x: entry_text_left(layout, entry.key),
                y,
                style: FONT_HEADING,
                scale_x: layout.scale_x,
                scale_y: layout.scale_y,
                align: if layout.centre { "center" } else { "left" },
                colour,
                shadow_colour: MENU_TEXT_SHADOW,
                foreground_offset_x: MENU_TEXT_SHADOW_OFFSET,
                foreground_offset_y: MENU_TEXT_SHADOW_OFFSET,
                ..layout_item_defaults(0)
            },
        );
    }
    items
}

fn layout_item_defaults(order: usize) -> FrontendLayoutItem {
    FrontendLayoutItem {
        order,
        kind: "",
        texture: "",
        text_key: "",
        entry_index: -1,
        selected: false,
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        x: 0.0,
        y: 0.0,
        style: -1,
        scale_x: 0.0,
        scale_y: 0.0,
        align: "",
        colour: [0, 0, 0, 0],
        shadow_colour: [0, 0, 0, 0],
        foreground_offset_x: 0.0,
        foreground_offset_y: 0.0,
    }
}

fn push_layout_item(items: &mut Vec<FrontendLayoutItem>, mut item: FrontendLayoutItem) {
    item.order = items.len();
    items.push(item);
}

pub fn apply_frontend_menu_transition(
    menu: &mut CMenuManager,
    input: FrontendTransitionInput,
    slot_states: &[i32],
) -> FrontendTransitionOutcome {
    if input == FrontendTransitionInput::ResetStart {
        *menu = initialise_frontend_transition_menu(menu.game_not_loaded);
        return FrontendTransitionOutcome::ChangeScreen;
    }

    let page = menu_page(menu.curr_screen);
    match input {
        FrontendTransitionInput::Down => {
            let mut next = menu.curr_option + 1;
            if next as usize >= page.entries.len() {
                next = 0;
            }
            menu.curr_option = next;
            FrontendTransitionOutcome::None
        }
        FrontendTransitionInput::Up => {
            let first_is_label = page
                .entries
                .first()
                .is_some_and(|entry| entry.action == eMenuAction::MENUACTION_LABEL as i32);
            if menu.curr_option == i32::from(first_is_label) {
                menu.curr_option = page.entries.len().saturating_sub(1) as i32;
            } else {
                menu.curr_option = (menu.curr_option - 1).max(0);
            }
            FrontendTransitionOutcome::None
        }
        FrontendTransitionInput::Back => go_back_state(menu),
        FrontendTransitionInput::Select => select_current_entry(menu, slot_states),
        FrontendTransitionInput::ResetStart => unreachable!(),
    }
}

fn select_current_entry(menu: &mut CMenuManager, slot_states: &[i32]) -> FrontendTransitionOutcome {
    let entry = current_entry(menu);
    if entry.target == eMenuScreen::MENUPAGE_NEW_GAME_RELOAD as i32 && menu.game_not_loaded {
        return do_settings_before_starting_game(menu);
    }

    match entry.action {
        action
            if action == eMenuAction::MENUACTION_CHANGEMENU as i32
                || action == eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU as i32
                || action == eMenuAction::MENUACTION_LOADRADIO as i32
                || action == eMenuAction::MENUACTION_KEYBOARDCTRLS as i32
                || action == eMenuAction::MENUACTION_PLAYERSETUP as i32 =>
        {
            let mut outcome =
                if entry.action == eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU as i32 {
                    FrontendTransitionOutcome::PopulateSlots
                } else {
                    FrontendTransitionOutcome::ChangeScreen
                };
            if entry.save_slot >= 0 {
                let slot_state = slot_state(slot_states, entry.save_slot);
                menu.curr_save_slot = entry.save_slot;
                if menu.curr_screen == eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32
                    && slot_state == 1
                {
                    return FrontendTransitionOutcome::BlockedEmptySlot;
                }
                if menu.curr_screen == eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT as i32
                    && slot_state != 1
                {
                    return FrontendTransitionOutcome::None;
                }
            }
            if entry.key == "FEDS_TB" {
                outcome = go_back_state(menu);
            } else {
                change_screen_state(menu, entry.target, 0, true, true);
            }
            outcome
        }
        action if action == eMenuAction::MENUACTION_CHECKSAVE as i32 => {
            if entry.save_slot < 0 {
                return FrontendTransitionOutcome::None;
            }
            let slot_state = slot_state(slot_states, entry.save_slot);
            menu.curr_save_slot = entry.save_slot;
            if slot_state == 1 {
                return FrontendTransitionOutcome::BlockedEmptySlot;
            }
            if slot_state == 2 {
                return FrontendTransitionOutcome::BlockedCorruptedSlot;
            }
            change_screen_state(menu, entry.target, 0, true, true);
            FrontendTransitionOutcome::CheckSave
        }
        action if action == eMenuAction::MENUACTION_NEWGAME as i32 => {
            do_settings_before_starting_game(menu)
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP as i32 => {
            do_settings_before_starting_game(menu)
        }
        action if action == eMenuAction::MENUACTION_DONTCANCEL as i32 => go_back_state(menu),
        action if action == eMenuAction::MENUACTION_CANCELGAME as i32 => {
            FrontendTransitionOutcome::Exit
        }
        _ => FrontendTransitionOutcome::None,
    }
}

fn current_entry(menu: &CMenuManager) -> MenuEntry {
    menu_page(menu.curr_screen)
        .entries
        .get(menu.curr_option.max(0) as usize)
        .copied()
        .unwrap_or(MenuEntry {
            action: eMenuAction::MENUACTION_NOTHING as i32,
            key: "",
            save_slot: -1,
            target: eMenuScreen::MENUPAGE_NONE as i32,
        })
}

fn slot_state(slot_states: &[i32], slot: i32) -> i32 {
    usize::try_from(slot)
        .ok()
        .and_then(|index| slot_states.get(index))
        .copied()
        .unwrap_or(1)
}

fn go_back_state(menu: &mut CMenuManager) -> FrontendTransitionOutcome {
    let page = menu_page(menu.curr_screen);
    let old_screen = page.previous_when_not_loaded;
    if old_screen == eMenuScreen::MENUPAGE_DISABLED as i32 {
        return FrontendTransitionOutcome::None;
    }
    let target = if old_screen == eMenuScreen::MENUPAGE_NONE as i32 {
        if menu.game_not_loaded {
            eMenuScreen::MENUPAGE_START_MENU as i32
        } else {
            eMenuScreen::MENUPAGE_PAUSE_MENU as i32
        }
    } else {
        old_screen
    };
    let option = previous_page_option(menu.curr_screen, target);
    change_screen_state(menu, target, option, true, true);
    FrontendTransitionOutcome::ChangeScreen
}

fn previous_page_option(current_screen: i32, previous_screen: i32) -> i32 {
    menu_page(previous_screen)
        .entries
        .iter()
        .position(|entry| entry.target == current_screen)
        .unwrap_or(0) as i32
}

fn do_settings_before_starting_game(menu: &mut CMenuManager) -> FrontendTransitionOutcome {
    menu.want_to_restart = true;
    menu.menu_active = false;
    FrontendTransitionOutcome::NewGame
}

fn change_screen_state(
    menu: &mut CMenuManager,
    screen: i32,
    option: i32,
    update_delay: bool,
    clear_alpha: bool,
) {
    menu.prev_screen = menu.curr_screen;
    menu.curr_screen = screen;
    menu.curr_option = option;
    if update_delay {
        menu.screen_change_delay_timer = 1;
    }
    if clear_alpha {
        menu.menu_fade_alpha = 0;
    }
}

pub fn on_enter_frontend(
    mut commands: Commands,
    mut ui: ResMut<FrontendMenuUi>,
    mut txd: ResMut<FrontendTxdTextures>,
    mut images: ResMut<Assets<Image>>,
    mut menu: ResMut<CMenuManager>,
    text: Option<Res<CText>>,
    install: Option<Res<OriginalGtaInstall>>,
    memory_card: Option<Res<CMemoryCard>>,
) {
    ensure_frontend_textures(&mut txd, &mut images, install.as_deref());
    despawn_frontend(&mut commands, &mut ui);
    menu.menu_active = true;
    menu.game_not_loaded = true;
    change_screen(
        &mut commands,
        &mut ui,
        &mut menu,
        eMenuScreen::MENUPAGE_START_MENU as i32,
        0,
        &text,
        memory_card.as_deref(),
        &mut txd,
        &mut images,
    );
}

pub fn on_enter_pause_frontend(
    mut commands: Commands,
    hud_roots: Query<Entity, With<crate::renderer::HudBevy::GtaHudRoot>>,
    mut ui: ResMut<FrontendMenuUi>,
    mut txd: ResMut<FrontendTxdTextures>,
    mut images: ResMut<Assets<Image>>,
    mut menu: ResMut<CMenuManager>,
    text: Option<Res<CText>>,
    install: Option<Res<OriginalGtaInstall>>,
    mut memory_card: Option<ResMut<CMemoryCard>>,
    mut pc_save: Option<ResMut<C_PcSave>>,
    mut game_storage: Option<ResMut<CGenericGameStorage>>,
) {
    ensure_frontend_textures(&mut txd, &mut images, install.as_deref());
    for entity in &hud_roots {
        commands.entity(entity).despawn();
    }
    despawn_frontend(&mut commands, &mut ui);
    menu.menu_active = true;
    menu.game_not_loaded = false;
    let save_zone = menu.save_menu_active;
    menu.save_menu_active = false;
    if save_zone {
        if let (Some(card), Some(pc_save), Some(storage)) = (
            memory_card.as_deref_mut(),
            pc_save.as_deref_mut(),
            game_storage.as_deref_mut(),
        ) {
            let save_file_name = storage.DefaultPCSaveFileName.clone();
            if let Some(root) = std::path::Path::new(&save_file_name).parent() {
                if let Err(error) = pc_save.PopulateSlotInfo(storage, root) {
                    error!(target: "re3bevy::save", "save-zone slot refresh failed: {error}");
                }
            }
            C_PcSave::SyncMemoryCardSlots(storage, card);
        }
    }
    change_screen(
        &mut commands,
        &mut ui,
        &mut menu,
        if save_zone {
            eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32
        } else {
            eMenuScreen::MENUPAGE_PAUSE_MENU as i32
        },
        0,
        &text,
        memory_card.as_deref().map(|card| &*card),
        &mut txd,
        &mut images,
    );
}

pub fn enter_save_zone_frontend(
    menu: Option<Res<CMenuManager>>,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    if menu.is_some_and(|menu| menu.save_menu_active) {
        next_state.set(GtaAppState::PauseMenu);
    }
}

#[derive(SystemParam)]
pub struct FrontendInput<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    #[cfg(target_os = "android")]
    mobile: Res<'w, super::MobileControls::MobileMenuInput>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    install: Option<Res<'w, OriginalGtaInstall>>,
    player_skin: Option<Res<'w, CPlayerSkin>>,
    audio: Option<ResMut<'w, cDMAudio>>,
    stats: Option<Res<'w, CStats>>,
    player_info: Option<Res<'w, CPlayerInfo>>,
    messages: Option<Res<'w, CMessages>>,
    timer: Option<Res<'w, CTimer>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
}

impl FrontendInput<'_, '_> {
    fn pointer_pressed(&self) -> bool {
        self.mouse.pressed(MouseButton::Left) || self.touches.iter().next().is_some()
    }
    fn pointer_just_pressed(&self) -> bool {
        self.mouse.just_pressed(MouseButton::Left) || self.touches.iter_just_pressed().next().is_some()
    }
}

#[derive(SystemParam)]
pub struct FrontendStorage<'w> {
    memory_card: Option<ResMut<'w, CMemoryCard>>,
    pc_save: Option<ResMut<'w, C_PcSave>>,
    game_storage: Option<ResMut<'w, CGenericGameStorage>>,
    save_request: ResMut<'w, PcSaveRequest>,
    local_coop: ResMut<'w, LocalCoopSession>,
}

pub(crate) fn update_frontend(
    mut input: FrontendInput,
    mut commands: Commands,
    mut ui: ResMut<FrontendMenuUi>,
    mut menu: ResMut<CMenuManager>,
    mut controller: ResMut<CControllerConfigManager>,
    mut mblur: Option<ResMut<CMBlur>>,
    storage: FrontendStorage,
    mut txd: ResMut<FrontendTxdTextures>,
    mut images: ResMut<Assets<Image>>,
    text: Option<Res<CText>>,
    mut text_query: Query<(&mut FrontendBitmapText, &mut ImageNode, &mut Node)>,
    mut bar_query: Query<(&FrontendSelectionBar, &mut Visibility)>,
    mut slider_query: Query<(&FrontendSliderBar, &mut BackgroundColor)>,
    mut interaction_queries: ParamSet<(
        Query<(&Hovered, Has<Pressed>, &FrontendButton), Or<(Changed<Hovered>, Added<Pressed>)>>,
        Query<(
            &Hovered,
            Has<Pressed>,
            &RelativeCursorPosition,
            &FrontendSliderHitArea,
        )>,
        Query<(&Hovered, Has<Pressed>, &FrontendPlayerSetupControl)>,
    )>,
    mut next_state: ResMut<NextState<GtaAppState>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let FrontendStorage {
        mut memory_card,
        mut pc_save,
        mut game_storage,
        mut save_request,
        mut local_coop,
    } = storage;

    if menu.curr_screen == eMenuScreen::MENUPAGE_SAVING_IN_PROGRESS as i32 {
        let target = match save_request.state {
            PcSaveRequestState::Succeeded => Some(eMenuScreen::MENUPAGE_SAVE_SUCCESSFUL),
            PcSaveRequestState::Failed => Some(eMenuScreen::MENUPAGE_SAVE_FAILED),
            PcSaveRequestState::Idle | PcSaveRequestState::Requested(_) => None,
        };
        if let Some(target) = target {
            save_request.state = PcSaveRequestState::Idle;
            change_screen(
                &mut commands,
                &mut ui,
                &mut menu,
                target as i32,
                if target == eMenuScreen::MENUPAGE_SAVE_SUCCESSFUL {
                    1
                } else {
                    0
                },
                &text,
                memory_card.as_deref().map(|card| &*card),
                &mut txd,
                &mut images,
            );
            return;
        }
    }
    #[cfg(target_os = "android")]
    let mobile_up = input.mobile.up;
    #[cfg(not(target_os = "android"))]
    let mobile_up = false;
    let gamepad_up = mobile_up || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::DPadUp));
    #[cfg(target_os = "android")]
    let mobile_down = input.mobile.down;
    #[cfg(not(target_os = "android"))]
    let mobile_down = false;
    let gamepad_down = mobile_down || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::DPadDown));
    #[cfg(target_os = "android")]
    let mobile_left = input.mobile.left;
    #[cfg(not(target_os = "android"))]
    let mobile_left = false;
    let gamepad_left = mobile_left || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::DPadLeft));
    #[cfg(target_os = "android")]
    let mobile_right = input.mobile.right;
    #[cfg(not(target_os = "android"))]
    let mobile_right = false;
    let gamepad_right = mobile_right || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::DPadRight));
    #[cfg(target_os = "android")]
    let mobile_select = input.mobile.select;
    #[cfg(not(target_os = "android"))]
    let mobile_select = false;
    let gamepad_select = mobile_select || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::South));
    #[cfg(target_os = "android")]
    let mobile_back = input.mobile.back;
    #[cfg(not(target_os = "android"))]
    let mobile_back = false;
    let gamepad_back = mobile_back || input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::East));
    let gamepad_page_up = input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::LeftTrigger2));
    let gamepad_page_down = input
        .gamepads
        .iter()
        .any(|pad| pad.just_pressed(GamepadButton::RightTrigger2));
    if let Some(target) = input
        .player_skin
        .as_ref()
        .and_then(|player_skin| player_skin.preview_target.clone())
    {
        txd.images
            .insert("__player_skin_preview".to_owned(), target);
    }
    if menu.curr_screen == eMenuScreen::MENUPAGE_PAUSE_MENU as i32
        && (input.keys.just_pressed(KeyCode::Escape) || gamepad_back)
    {
        if let Some(audio) = input.audio.as_deref_mut() {
            audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_BACK as u16, 0);
        }
        menu.menu_active = false;
        despawn_frontend(&mut commands, &mut ui);
        next_state.set(GtaAppState::InGame);
        return;
    }
    let page = menu_page(menu.curr_screen);
    let initial_option = menu.curr_option;
    let mut setting_changed = false;
    let mut selected = false;

    if menu.curr_screen == eMenuScreen::MENUPAGE_SKIN_SELECT as i32 {
        let mut redraw = false;
        let mut apply_skin = false;
        let mut go_back_requested = false;
        let mut held_scroll_direction = 0_i8;
        let mut play_navigation = false;
        let mut play_setting_change = false;
        let mut play_exit = false;

        // re3 DrawPlayerSetupScreen enumerates on the first frame whenever
        // m_bSkinsEnumerated is false.  Keep the page self-sufficient even if
        // it was entered by a state transition other than MENUACTION_PLAYERSETUP.
        if !menu.player_setup_skins_enumerated || menu.player_setup_skins.is_empty() {
            enumerate_player_setup_skins(&mut menu, &text, input.install.as_deref());
            menu.first_visible_row_on_list = 0;
            menu.selected_list_row = 0;
            menu.player_setup_focus = 0;
            menu.player_setup_scrollbar_top_margin = 0.0;
            menu.player_setup_repeat_direction = 0;
            menu.player_setup_repeat_at = None;
            sync_player_setup_preview_name(&mut menu);
            redraw = true;
        }

        for (hovered, pressed, control) in interaction_queries.p2().iter() {
            if hovered.get() || pressed {
                menu.show_mouse = true;
                let focus = match control.0 {
                    PlayerSetupControl::SkinRow(_) => 0,
                    PlayerSetupControl::UseSkin => 1,
                    PlayerSetupControl::Back => 2,
                    _ => menu.player_setup_focus,
                };
                if focus != menu.player_setup_focus {
                    menu.player_setup_focus = focus;
                    redraw = true;
                    play_navigation = true;
                }
            }
            if (hovered.get() || pressed)
                && input.pointer_pressed()
            {
                held_scroll_direction = match control.0 {
                    PlayerSetupControl::ScrollUp => -1,
                    PlayerSetupControl::ScrollDown => 1,
                    _ => held_scroll_direction,
                };
            }
            if !pressed || !input.pointer_just_pressed()
            {
                continue;
            }
            match control.0 {
                PlayerSetupControl::SkinRow(row) => {
                    if row == menu.selected_list_row.max(0) as usize {
                        apply_skin = true;
                        play_setting_change = true;
                    } else {
                        menu.selected_list_row = row as i32;
                        menu.player_setup_focus = 0;
                        redraw = true;
                        play_navigation = true;
                    }
                }
                PlayerSetupControl::UseSkin => {
                    apply_skin = true;
                    play_setting_change = true;
                }
                PlayerSetupControl::Back => {
                    go_back_requested = true;
                    play_exit = true;
                }
                PlayerSetupControl::ScrollUp => {
                    player_setup_scroll_by(&mut menu, -1);
                    menu.player_setup_repeat_direction = -1;
                    menu.player_setup_repeat_at = Some(Instant::now());
                    redraw = true;
                }
                PlayerSetupControl::ScrollDown => {
                    player_setup_scroll_by(&mut menu, 1);
                    menu.player_setup_repeat_direction = 1;
                    menu.player_setup_repeat_at = Some(Instant::now());
                    redraw = true;
                }
                PlayerSetupControl::PageUp => {
                    redraw |= player_setup_page_by(&mut menu, -1);
                    play_navigation = true;
                }
                PlayerSetupControl::PageDown => {
                    redraw |= player_setup_page_by(&mut menu, 1);
                    play_navigation = true;
                }
            }
        }

        let keyboard_scroll_direction =
            if input.keys.pressed(KeyCode::ArrowUp) || input.keys.pressed(KeyCode::KeyW) {
                -1
            } else if input.keys.pressed(KeyCode::ArrowDown) || input.keys.pressed(KeyCode::KeyS) {
                1
            } else {
                0
            };
        let repeat_direction = if keyboard_scroll_direction != 0 {
            keyboard_scroll_direction
        } else {
            held_scroll_direction
        };
        if repeat_direction != 0
            && !input.keys.just_pressed(KeyCode::ArrowUp)
            && !input.keys.just_pressed(KeyCode::ArrowDown)
            && !input.keys.just_pressed(KeyCode::KeyW)
            && !input.keys.just_pressed(KeyCode::KeyS)
            && !input.pointer_just_pressed()
            && player_setup_repeat_ready(&mut menu, repeat_direction)
        {
            menu.player_setup_focus = 0;
            redraw |= player_setup_scroll_by(&mut menu, i32::from(repeat_direction));
            play_navigation |= keyboard_scroll_direction != 0;
        } else if repeat_direction == 0 {
            redraw |= menu.player_setup_repeat_direction != 0;
            menu.player_setup_repeat_direction = 0;
            menu.player_setup_repeat_at = None;
        }

        let wheel = input.mouse_wheel.read().map(|event| event.y).sum::<f32>();
        if wheel > 0.0 {
            menu.show_mouse = true;
            redraw |= player_setup_scroll_by(&mut menu, -1);
            play_navigation = true;
        } else if wheel < 0.0 {
            menu.show_mouse = true;
            redraw |= player_setup_scroll_by(&mut menu, 1);
            play_navigation = true;
        }

        if input.keys.just_pressed(KeyCode::Tab) {
            menu.show_mouse = false;
            menu.player_setup_focus = match menu.player_setup_focus {
                0 if menu.skin_name != menu.prefs_skin_file => 1,
                0 => 2,
                1 => 2,
                _ => 0,
            };
            redraw = true;
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::ArrowUp)
            || input.keys.just_pressed(KeyCode::KeyW)
            || gamepad_up
        {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            redraw |= player_setup_scroll_by(&mut menu, -1);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::ArrowDown)
            || input.keys.just_pressed(KeyCode::KeyS)
            || gamepad_down
        {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            redraw |= player_setup_scroll_by(&mut menu, 1);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::PageUp) || gamepad_page_up {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            redraw |= player_setup_page_by(&mut menu, -1);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::PageDown) || gamepad_page_down {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            redraw |= player_setup_page_by(&mut menu, 1);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::Home) {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            redraw |= player_setup_select_row(&mut menu, 0);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::End) {
            menu.show_mouse = false;
            menu.player_setup_focus = 0;
            let last = menu.player_setup_skins.len().saturating_sub(1);
            redraw |= player_setup_select_row(&mut menu, last);
            play_navigation = true;
        }
        if input.keys.just_pressed(KeyCode::Enter)
            || input.keys.just_pressed(KeyCode::Space)
            || gamepad_select
        {
            menu.show_mouse = false;
            match menu.player_setup_focus {
                0 | 1 => apply_skin = true,
                _ => {
                    go_back_requested = true;
                    play_exit = true;
                }
            }
            play_setting_change |= !go_back_requested;
        }
        if input.keys.just_pressed(KeyCode::Escape) || gamepad_back {
            menu.show_mouse = false;
            go_back_requested = true;
            play_exit = true;
        }

        if redraw {
            sync_player_setup_preview_name(&mut menu);
        }
        if apply_skin && !menu.player_setup_skins.is_empty() {
            sync_player_setup_preview_name(&mut menu);
            let skin_name = menu.skin_name.clone();
            menu.prefs_skin_file = skin_name;
            menu.player_setup_focus = 2;
            redraw = true;
        }
        if let Some(audio) = input.audio.as_deref_mut() {
            if play_exit {
                audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_BACK as u16, 0);
            } else if play_setting_change {
                audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_SETTING_CHANGE as u16, 0);
            } else if play_navigation {
                audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_NAVIGATION as u16, 0);
            }
        }
        if go_back_requested {
            go_back(
                &mut commands,
                &mut ui,
                &mut menu,
                &text,
                &mut txd,
                &mut images,
            );
            return;
        }
        if redraw {
            spawn_frontend(
                &mut commands,
                &mut ui,
                &menu,
                &text,
                memory_card.as_deref().map(|card| &*card),
                &mut txd,
                &mut images,
            );
        }
        return;
    }

    // re3's GETKEY path owns the next physical key press.  It must run before
    // normal menu navigation so Enter cannot immediately select a second row.
    if menu.waiting_for_new_key_bind {
        if input.keys.just_pressed(KeyCode::Escape) || gamepad_back {
            menu.waiting_for_new_key_bind = false;
            menu.pending_control_label = None;
        } else if let Some(key) = input
            .keys
            .get_just_pressed()
            .copied()
            .find(|key| *key != KeyCode::Enter && *key != KeyCode::Escape)
        {
            if let Some(label) = menu.pending_control_label.as_deref() {
                if let Some(action) = controller_action_for_label(label) {
                    controller.assign_frontend_keyboard_binding(action, key);
                }
            }
            // Bevy KeyCode is a data-carrying enum; the actual binding owns
            // the physical key and this field remains the re3 menu marker.
            menu.key_pressed_code = 0;
            menu.waiting_for_new_key_bind = false;
            menu.pending_control_label = None;
        } else if let Some(button) = input.mouse.get_just_pressed().copied().next() {
            if let Some(label) = menu.pending_control_label.as_deref()
                && let Some(action) = controller_action_for_label(label)
            {
                controller.assign_frontend_mouse_binding(action, button);
            }
            menu.key_pressed_code = 0;
            menu.waiting_for_new_key_bind = false;
            menu.pending_control_label = None;
        }
        refresh_text(
            page,
            &menu,
            &controller,
            &text,
            memory_card.as_deref().map(|card| &*card),
            &mut text_query,
            &mut bar_query,
            &mut slider_query,
            &mut txd,
            &mut images,
        );
        return;
    }

    if menu.curr_screen == eMenuScreen::MENUPAGE_KEYBOARD_CONTROLS as i32
        && input.keys.just_pressed(KeyCode::Backspace)
    {
        let entry = current_entry(&menu);
        if entry.action == eMenuAction::MENUACTION_GETKEY as i32
            && let Some(action) = controller_action_for_label(entry.key)
        {
            controller.clear_frontend_keyboard_binding(action);
            refresh_text(
                page,
                &menu,
                &controller,
                &text,
                memory_card.as_deref().map(|card| &*card),
                &mut text_query,
                &mut bar_query,
                &mut slider_query,
                &mut txd,
                &mut images,
            );
        }
    }

    if matches!(menu.curr_screen, 1 | 3) {
        const VISIBLE_ROWS: i32 = 15;
        let old_first = menu.first_visible_row_on_list;
        let wheel = input.mouse_wheel.read().map(|event| event.y).sum::<f32>();
        if input.keys.just_pressed(KeyCode::ArrowUp) || gamepad_up || wheel > 0.0 {
            menu.first_visible_row_on_list -= 1;
        }
        if input.keys.just_pressed(KeyCode::ArrowDown) || gamepad_down || wheel < 0.0 {
            menu.first_visible_row_on_list += 1;
        }
        if input.keys.just_pressed(KeyCode::PageUp) || gamepad_page_up {
            menu.first_visible_row_on_list -= VISIBLE_ROWS;
        }
        if input.keys.just_pressed(KeyCode::PageDown) || gamepad_page_down {
            menu.first_visible_row_on_list += VISIBLE_ROWS;
        }
        if input.keys.just_pressed(KeyCode::Home) {
            menu.first_visible_row_on_list = 0;
        }
        if input.keys.just_pressed(KeyCode::End) {
            menu.first_visible_row_on_list = i32::MAX;
        }
        let max_first = (menu.total_list_row - VISIBLE_ROWS).max(0);
        menu.first_visible_row_on_list = menu.first_visible_row_on_list.clamp(0, max_first);
        if old_first != menu.first_visible_row_on_list {
            if let Some(audio) = input.audio.as_deref_mut() {
                audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_NAVIGATION as u16, 0);
            }
            spawn_frontend(
                &mut commands,
                &mut ui,
                &menu,
                &text,
                memory_card.as_deref().map(|card| &*card),
                &mut txd,
                &mut images,
            );
        }
    }

    for (hovered, pressed, button) in interaction_queries.p0().iter() {
        if (hovered.get() || pressed) && is_selectable(page.entries[button.entry_index]) {
            menu.show_mouse = true;
            menu.curr_option = button.entry_index as i32;
            selected |= pressed && input.pointer_just_pressed();
        }
    }

    for (hovered, pressed, cursor, hit_area) in interaction_queries.p1().iter() {
        if cursor.cursor_over()
            && (hovered.get() || pressed)
        {
            menu.show_mouse = true;
            menu.curr_option = hit_area.entry_index as i32;
        }
        if !cursor.cursor_over()
            || !pressed
            || !input.pointer_pressed()
        {
            continue;
        }
        let Some(normalized) = cursor.normalized else {
            continue;
        };
        let logical_mouse_x = hit_area.logical_left
            + (normalized.x + 0.5) * (DEFAULT_SCREEN_WIDTH - hit_area.logical_left);
        let entry = page.entries[hit_area.entry_index];
        let layout = menu_layout(menu.curr_screen);
        if let Some(direction) =
            slider_mouse_direction(&menu, entry.action, layout, logical_mouse_x)
        {
            setting_changed |=
                adjust_current_setting(&mut menu, &mut controller, mblur.as_deref_mut(), direction);
        }
    }

    let menu_wheel = input.mouse_wheel.read().map(|event| event.y).sum::<f32>();
    if menu_wheel > 0.0 {
        menu.show_mouse = true;
        apply_frontend_menu_transition(&mut menu, FrontendTransitionInput::Up, &[]);
    } else if menu_wheel < 0.0 {
        menu.show_mouse = true;
        apply_frontend_menu_transition(&mut menu, FrontendTransitionInput::Down, &[]);
    }

    if input.keys.just_pressed(KeyCode::ArrowDown)
        || input.keys.just_pressed(KeyCode::KeyS)
        || gamepad_down
    {
        menu.show_mouse = false;
        apply_frontend_menu_transition(&mut menu, FrontendTransitionInput::Down, &[]);
    }
    if input.keys.just_pressed(KeyCode::ArrowUp)
        || input.keys.just_pressed(KeyCode::KeyW)
        || gamepad_up
    {
        menu.show_mouse = false;
        apply_frontend_menu_transition(&mut menu, FrontendTransitionInput::Up, &[]);
    }
    if input.keys.just_pressed(KeyCode::ArrowLeft)
        || input.keys.just_pressed(KeyCode::KeyA)
        || gamepad_left
    {
        setting_changed |=
            adjust_current_setting(&mut menu, &mut controller, mblur.as_deref_mut(), -1);
    }
    if input.keys.just_pressed(KeyCode::ArrowRight)
        || input.keys.just_pressed(KeyCode::KeyD)
        || gamepad_right
    {
        setting_changed |=
            adjust_current_setting(&mut menu, &mut controller, mblur.as_deref_mut(), 1);
    }
    if input.keys.just_pressed(KeyCode::Enter)
        || input.keys.just_pressed(KeyCode::Space)
        || gamepad_select
    {
        selected = true;
    }
    if input.keys.just_pressed(KeyCode::Escape) || gamepad_back {
        if let Some(audio) = input.audio.as_deref_mut() {
            audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_BACK as u16, 0);
        }
        if menu.curr_screen == eMenuScreen::MENUPAGE_CHOOSE_SAVE_SLOT as i32 {
            menu.menu_active = false;
            despawn_frontend(&mut commands, &mut ui);
            next_state.set(GtaAppState::InGame);
            return;
        }
        go_back(
            &mut commands,
            &mut ui,
            &mut menu,
            &text,
            &mut txd,
            &mut images,
        );
        return;
    }

    if setting_changed {
        if let Some(audio) = input.audio.as_deref_mut() {
            audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_SETTING_CHANGE as u16, 0);
        }
    } else if menu.curr_option != initial_option {
        if let Some(audio) = input.audio.as_deref_mut() {
            audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_NAVIGATION as u16, 0);
        }
    }

    refresh_text(
        page,
        &menu,
        &controller,
        &text,
        memory_card.as_deref().map(|card| &*card),
        &mut text_query,
        &mut bar_query,
        &mut slider_query,
        &mut txd,
        &mut images,
    );

    if selected {
        if activate_current_setting(&mut menu, &mut controller, mblur.as_deref_mut()) {
            if let Some(audio) = input.audio.as_deref_mut() {
                audio.PlayFrontEndSound(SOUND_FRONTEND_MENU_SETTING_CHANGE as u16, 0);
            }
            refresh_text(
                page,
                &menu,
                &controller,
                &text,
                memory_card.as_deref().map(|card| &*card),
                &mut text_query,
                &mut bar_query,
                &mut slider_query,
                &mut txd,
                &mut images,
            );
            return;
        }
        play_frontend_entry_sound(
            input.audio.as_deref_mut(),
            &menu,
            memory_card.as_deref().map(|card| &*card),
        );
        activate_entry(
            &mut commands,
            &mut ui,
            &mut menu,
            memory_card.as_deref_mut(),
            pc_save.as_deref_mut(),
            game_storage.as_deref_mut(),
            &text,
            &mut next_state,
            &mut app_exit,
            &mut txd,
            &mut images,
            input.install.as_deref(),
            input.stats.as_deref(),
            input.player_info.as_deref(),
            input.messages.as_deref(),
            input.timer.as_deref(),
            &mut save_request,
            &mut local_coop,
        );
    }
}

/// Records the actual native Bevy window render target for Display Settings.
pub fn sync_frontend_window_settings(
    windows: Query<&Window, With<PrimaryWindow>>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    mut menu: Option<ResMut<CMenuManager>>,
    mut txd: ResMut<FrontendTxdTextures>,
    mut last_key: Local<Option<(u32, u32, usize, u64)>>,
) {
    let (Ok(window), Some(menu)) = (windows.single(), menu.as_deref_mut()) else {
        return;
    };
    let width = window.resolution.physical_width();
    let height = window.resolution.physical_height();
    // Rebuilding, sorting and deduping the mode list every frame is pure
    // overhead when nothing changed. The key covers the window size plus an
    // allocation-free checksum of the monitor mode list, so a hit provably
    // reproduces the same `video_modes` below and the whole scan is skipped.
    // (Scalars are only written on a miss, but the key guarantees they are
    // already equal on a hit.)
    let mut mode_count = 0_usize;
    let mut mode_sum = 0_u64;
    for monitor in &monitors {
        for mode in &monitor.video_modes {
            mode_count += 1;
            // Position-weighted so both the multiset and the order feed the
            // key; only a byte-identical mode list can reproduce it.
            mode_sum = mode_sum.wrapping_add((mode_count as u64).wrapping_mul(
                mode.physical_size.x as u64 * 1_000_000 + mode.physical_size.y as u64,
            ));
        }
    }
    let key = (width, height, mode_count, mode_sum);
    if *last_key == Some(key) {
        return;
    }
    *last_key = Some(key);
    menu.window_width = width;
    menu.window_height = height;
    let mut video_modes = monitors
        .iter()
        .flat_map(|monitor| monitor.video_modes.iter())
        .map(|mode| [mode.physical_size.x, mode.physical_size.y])
        .collect::<Vec<_>>();
    // During the direct New Game route this system can run before winit has
    // populated PrimaryMonitor. re3 has already loaded the saved video mode at
    // this point, so preserve that requested size as a one-frame fallback and
    // let the later monitor-backed refresh validate/rebuild the mode list.
    // Without this, re3bevynewgame silently stays at the 1280x720 bootstrap
    // size while the normal frontend path reaches the saved resolution.
    if video_modes.is_empty()
        && menu.requested_video_size[0] > 0
        && menu.requested_video_size[1] > 0
    {
        video_modes.push(menu.requested_video_size);
    }
    // re3's platform list exposes modes, not refresh-rate duplicates.
    video_modes.sort_unstable();
    video_modes.dedup();
    if !video_modes.contains(&[width, height]) {
        video_modes.push([width, height]);
        video_modes.sort_unstable();
    }
    if video_modes != menu.video_modes {
        menu.video_modes = video_modes;
        let current = menu
            .video_modes
            .iter()
            .position(|mode| *mode == [width, height])
            .unwrap_or(0);
        let preferred = menu
            .video_modes
            .iter()
            .position(|mode| *mode == menu.requested_video_size)
            .unwrap_or(current);
        menu.display_video_mode = preferred;
        menu.prefs_video_mode = preferred;
        if preferred != current {
            menu.apply_video_mode_requested = true;
        }
    }
    let scale_x = width as f32 / DEFAULT_SCREEN_WIDTH;
    let scale_y = height as f32 / DEFAULT_SCREEN_HEIGHT;
    if (txd.text_render_scale_x - scale_x).abs() > f32::EPSILON
        || (txd.text_render_scale_y - scale_y).abs() > f32::EPSILON
    {
        txd.text_render_scale_x = scale_x;
        txd.text_render_scale_y = scale_y;
        // Handles in existing ImageNodes are replaced by refresh_text below.
        // Keeping a logical-scale cache would make Bevy filter the glyph image
        // a second time, unlike CFont's direct sprite-bank path.
        txd.text_images.clear();
    }
}

/// re3 `LoadSettings`/`SaveSettings`, stored beside the PC save slots. The
/// payload is port-owned because Bevy `KeyCode` values are platform-neutral
/// and cannot use RenderWare's Win32 key integers.
pub fn sync_frontend_settings(
    install: Option<Res<OriginalGtaInstall>>,
    mut menu: Option<ResMut<CMenuManager>>,
    mut controller: Option<ResMut<CControllerConfigManager>>,
    mut state: ResMut<FrontendSettingsSyncState>,
) {
    let (Some(install), Some(menu), Some(controller)) =
        (install, menu.as_deref_mut(), controller.as_deref_mut())
    else {
        return;
    };
    let path = install
        .root()
        .parent()
        .map(|path| path.join("re3bevy-userfiles").join("gta3.set"))
        .unwrap_or_else(|| install.root().join("re3bevy-userfiles").join("gta3.set"));
    if !state.loaded {
        if let Ok(payload) = fs::read_to_string(&path) {
            load_frontend_settings_payload(&payload, menu, controller);
        }
        state.last_payload = frontend_settings_payload(menu, controller);
        state.loaded = true;
        return;
    }
    let payload = frontend_settings_payload(menu, controller);
    if payload == state.last_payload {
        return;
    }
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        warn!("CMenuManager::SaveSettings directory failed: {error}");
        return;
    }
    match fs::write(&path, &payload) {
        Ok(()) => state.last_payload = payload,
        Err(error) => warn!("CMenuManager::SaveSettings failed: {error}"),
    }
}

fn frontend_settings_payload(menu: &CMenuManager, controller: &CControllerConfigManager) -> String {
    let resolution = menu
        .video_modes
        .get(menu.prefs_video_mode)
        .copied()
        .unwrap_or(menu.requested_video_size);
    let mut out = format!(
        "RE3BEVY_SETTINGS=1\nmusic={}\nsfx={}\nradio={}\nbrightness={}\nlod={}\nsubtitles={}\nwidescreen={}\nsplit_screen={}\ncoop_swap_sides={}\nvsync={}\nmax_fps={}\ntrails={}\nlanguage={}\ncontrol_method={}\nmouse_sensitivity={}\nmouse_invert={}\nmouse_steering={}\nfree_cam={}\nskin={}\nresolution={}x{}\n",
        menu.prefs_music_volume,
        menu.prefs_sfx_volume,
        menu.prefs_radio_station,
        menu.prefs_brightness,
        menu.prefs_lod,
        menu.prefs_show_subtitles,
        menu.prefs_use_wide_screen,
        if menu.prefs_local_coop_split == 0 {
            "top_bottom"
        } else {
            "left_right"
        },
        menu.prefs_local_coop_swap_sides,
        menu.prefs_vsync_disp,
        menu.prefs_max_fps,
        u8::from(menu.prefs_trails),
        menu.prefs_language,
        menu.control_method,
        menu.mouse_sensitivity,
        u8::from(menu.mouse_invert),
        u8::from(!menu.disable_mouse_steering),
        u8::from(menu.prefs_free_cam),
        menu.prefs_skin_file,
        resolution[0],
        resolution[1],
    );
    for ((action, kind), binding) in &controller.bindings {
        let kind = match kind {
            eControllerType::KEYBOARD => "K",
            eControllerType::OPTIONAL_EXTRA => "O",
            eControllerType::MOUSE => "M",
            eControllerType::JOYSTICK => "J",
        };
        out.push_str(&format!(
            "bind={},{kind},{:?}\n",
            *action as u16, binding.key
        ));
    }
    for (action, button) in &controller.mouse_bindings {
        out.push_str(&format!("mbind={},{button:?}\n", *action as u16));
    }
    out
}

fn load_frontend_settings_payload(
    payload: &str,
    menu: &mut CMenuManager,
    controller: &mut CControllerConfigManager,
) {
    if !payload.lines().any(|line| line == "RE3BEVY_SETTINGS=1") {
        return;
    }
    let mut bindings = Vec::new();
    let mut mouse_bindings = Vec::new();
    for line in payload.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "music" => menu.prefs_music_volume = parse_i32(value, 0, 127, 102),
            "sfx" => menu.prefs_sfx_volume = parse_i32(value, 0, 127, 102),
            "radio" => menu.prefs_radio_station = parse_i32(value, 0, 9, 0) as i8,
            "brightness" => menu.prefs_brightness = parse_i32(value, 0, 511, 256),
            "lod" => menu.prefs_lod = value.parse::<f32>().unwrap_or(1.2).clamp(0.8, 1.8),
            "subtitles" => menu.prefs_show_subtitles = parse_i32(value, 0, 1, 1) as i8,
            "widescreen" => menu.prefs_use_wide_screen = parse_i32(value, 0, 1, 0) as i8,
            "split_screen" => {
                menu.prefs_local_coop_split = i8::from(value.eq_ignore_ascii_case("left_right"));
            }
            "coop_swap_sides" => {
                menu.prefs_local_coop_swap_sides = parse_i32(value, 0, 1, 0) as i8;
            }
            "vsync" => {
                menu.prefs_vsync_disp = parse_i32(value, 0, 1, 1) as i8;
                menu.prefs_vsync = menu.prefs_vsync_disp;
            }
            "max_fps" => {
                menu.prefs_max_fps = normalise_max_fps(value.parse().unwrap_or(DEFAULT_MAX_FPS))
            }
            // Legacy bool toggle from before the multi-step cap (`0` = off,
            // `1` = 30 Hz). Kept so older `gta3.set` files keep working.
            "frame_limiter" => {
                menu.prefs_max_fps = match value.trim() {
                    "0" => 0,
                    _ => 30,
                };
            }
            "trails" => menu.prefs_trails = value == "1",
            "language" => menu.prefs_language = parse_i32(value, 0, 4, 0),
            "control_method" => menu.control_method = parse_i32(value, 0, 1, 0),
            "mouse_sensitivity" => menu.mouse_sensitivity = parse_i32(value, 0, 200, 100),
            "mouse_invert" => menu.mouse_invert = value == "1",
            "mouse_steering" => menu.disable_mouse_steering = value != "1",
            "free_cam" => menu.prefs_free_cam = value == "1",
            "skin" => menu.prefs_skin_file = value.to_owned(),
            "resolution" => {
                if let Some((width, height)) = value.split_once('x')
                    && let (Ok(width), Ok(height)) = (width.parse(), height.parse())
                {
                    menu.requested_video_size = [width, height];
                }
            }
            "bind" => {
                let mut parts = value.split(',');
                let (Some(action), Some(kind), Some(key)) =
                    (parts.next(), parts.next(), parts.next())
                else {
                    continue;
                };
                let Some(action) = action
                    .parse::<u16>()
                    .ok()
                    .and_then(controller_action_from_id)
                else {
                    continue;
                };
                let kind = match kind {
                    "K" => eControllerType::KEYBOARD,
                    "O" => eControllerType::OPTIONAL_EXTRA,
                    _ => continue,
                };
                if let Some(key) = parse_key_code(key) {
                    bindings.push((action, kind, key));
                }
            }
            "mbind" => {
                let Some((action, button)) = value.split_once(',') else {
                    continue;
                };
                let Some(action) = action
                    .parse::<u16>()
                    .ok()
                    .and_then(controller_action_from_id)
                else {
                    continue;
                };
                let button = match button {
                    "Left" => MouseButton::Left,
                    "Right" => MouseButton::Right,
                    "Middle" => MouseButton::Middle,
                    "Back" => MouseButton::Back,
                    "Forward" => MouseButton::Forward,
                    _ => continue,
                };
                mouse_bindings.push((action, button));
            }
            _ => {}
        }
    }
    if !bindings.is_empty() {
        controller.MakeControllerActionsBlank();
        for (action, kind, key) in bindings {
            controller.SetControllerKeyAssociatedWithAction(action, key, kind);
        }
        for (action, button) in mouse_bindings {
            controller.mouse_bindings.insert(action, button);
        }
    }
    menu.skin_name.clone_from(&menu.prefs_skin_file);
    menu.front_end_reload_obr_txt_gxt = 1;
}

fn parse_i32(value: &str, min: i32, max: i32, default: i32) -> i32 {
    value.parse::<i32>().unwrap_or(default).clamp(min, max)
}

fn controller_action_from_id(id: u16) -> Option<e_ControllerAction> {
    e_ControllerAction::KEYBOARD_MENU_ACTIONS
        .into_iter()
        .find(|action| *action as u16 == id)
}

fn parse_key_code(value: &str) -> Option<KeyCode> {
    Some(match value {
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Enter" => KeyCode::Enter,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "Space" => KeyCode::Space,
        "Insert" => KeyCode::Insert,
        "Delete" => KeyCode::Delete,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "CapsLock" => KeyCode::CapsLock,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Backslash" => KeyCode::Backslash,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        value if value.starts_with("Key") && value.len() == 4 => match &value[3..] {
            "A" => KeyCode::KeyA,
            "B" => KeyCode::KeyB,
            "C" => KeyCode::KeyC,
            "D" => KeyCode::KeyD,
            "E" => KeyCode::KeyE,
            "F" => KeyCode::KeyF,
            "G" => KeyCode::KeyG,
            "H" => KeyCode::KeyH,
            "I" => KeyCode::KeyI,
            "J" => KeyCode::KeyJ,
            "K" => KeyCode::KeyK,
            "L" => KeyCode::KeyL,
            "M" => KeyCode::KeyM,
            "N" => KeyCode::KeyN,
            "O" => KeyCode::KeyO,
            "P" => KeyCode::KeyP,
            "Q" => KeyCode::KeyQ,
            "R" => KeyCode::KeyR,
            "S" => KeyCode::KeyS,
            "T" => KeyCode::KeyT,
            "U" => KeyCode::KeyU,
            "V" => KeyCode::KeyV,
            "W" => KeyCode::KeyW,
            "X" => KeyCode::KeyX,
            "Y" => KeyCode::KeyY,
            "Z" => KeyCode::KeyZ,
            _ => return None,
        },
        _ => return None,
    })
}

/// Applies menu-owned values to the Bevy-facing owners.  This is deliberately
/// a separate system: menu state remains a re3 resource while wgpu/window,
/// audio and renderer adapters own the actual side effects.
pub fn apply_frontend_preferences(
    mut menu: Option<ResMut<CMenuManager>>,
    mut audio: Option<ResMut<crate::audio::DMAudio::cDMAudio>>,
    mut file_loader: Option<ResMut<crate::core::FileLoader::CFileLoader>>,
    mut camera: Option<ResMut<crate::core::Camera::CCamera>>,
    mut mblur: Option<ResMut<CMBlur>>,
    mut local_coop: Option<ResMut<LocalCoopSession>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut frontend_radio: Local<Option<i8>>,
    mut frontend_radio_playing: Local<bool>,
) {
    let Some(menu) = menu.as_deref_mut() else {
        return;
    };
    if let Some(audio) = audio.as_deref_mut() {
        audio.SetMusicMasterVolume(menu.prefs_music_volume.clamp(0, 128) as u8);
        audio.SetEffectsMasterVolume(menu.prefs_sfx_volume.clamp(0, 128) as u8);
        let wants_frontend_radio =
            menu.menu_active && menu.curr_screen == eMenuScreen::MENUPAGE_SOUND_SETTINGS as i32;
        if wants_frontend_radio
            && (!*frontend_radio_playing || *frontend_radio != Some(menu.prefs_radio_station))
        {
            audio.PlayFrontEndTrack(menu.prefs_radio_station.max(0) as u8, true);
            *frontend_radio = Some(menu.prefs_radio_station);
            *frontend_radio_playing = true;
        } else if !wants_frontend_radio && *frontend_radio_playing {
            audio.StopFrontEndTrack();
            *frontend_radio_playing = false;
        }
    }
    if let Some(file_loader) = file_loader.as_deref_mut() {
        // re3 CRenderer::ms_lodDistScale receives the Display LOD preference.
        file_loader.renderer.lod_dist_scale_tenths = (menu.prefs_lod * 10.0).round() as i32;
    }
    if let Some(camera) = camera.as_deref_mut() {
        camera.wide_screen_on = menu.prefs_use_wide_screen != 0;
    }
    if let Some(mblur) = mblur.as_deref_mut() {
        mblur.blur_on = menu.prefs_trails;
        if !mblur.blur_on {
            mblur.motion_blur_close();
        }
    }
    if let Some(local_coop) = local_coop.as_deref_mut() {
        local_coop.split_layout = if menu.prefs_local_coop_split == 0 {
            LocalCoopSplitLayout::TopBottom
        } else {
            LocalCoopSplitLayout::LeftRight
        };
        local_coop.swap_sides = menu.prefs_local_coop_swap_sides != 0;
    }

    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if menu.apply_video_mode_requested {
        if let Some([width, height]) = menu.video_modes.get(menu.prefs_video_mode).copied() {
            window.resolution.set_physical_resolution(width, height);
        }
        menu.apply_video_mode_requested = false;
    }
    window.present_mode = if menu.prefs_vsync != 0 {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
}

/// Frame cap selected in Display Settings (`30/60/120/240/360/UNLIMITED`).
/// `0` (unlimited) skips throttling; anything else targets that rate.
/// re3 reference: `m_PrefsFrameLimiter` gate in `win.cpp`/`glfw.cpp` against
/// `RsGlobal.maxFPS` (30 Hz in `skeleton.cpp:410`).
pub fn apply_frame_limit(
    menu: Option<Res<CMenuManager>>,
    mut winit_settings: Option<ResMut<WinitSettings>>,
    mut last_frame: Local<Option<Instant>>,
) {
    let frame_rate = menu
        .map(|menu| normalise_max_fps(menu.prefs_max_fps))
        .unwrap_or(0);
    if let Some(winit_settings) = winit_settings.as_deref_mut() {
        // Keep Winit itself continuous. Its Windows WaitUntil path can
        // periodically oversleep a 16.7 ms deadline to ~30 ms. re3 gates the
        // call into rsIDLE instead, so pace immediately before the new Bevy
        // frame starts and let the previous render submission run meanwhile.
        if winit_settings.focused_mode != UpdateMode::Continuous {
            winit_settings.focused_mode = UpdateMode::Continuous;
        }
    }

    let now = Instant::now();
    if frame_rate <= 0 {
        *last_frame = Some(now);
        return;
    }

    let target = Duration::from_secs_f64(1.0 / f64::from(frame_rate));
    if let Some(previous) = *last_frame {
        let elapsed = previous.elapsed();
        if elapsed < target {
            let remaining = target - elapsed;
            // Windows sleeps are intentionally given a small safety margin.
            // The short tail is completed with spin_loop so scheduler jitter
            // cannot turn a 60 Hz cap into a missed 30 Hz-looking frame.
            const SPIN_TAIL: Duration = Duration::from_millis(1);
            if remaining > SPIN_TAIL {
                std::thread::sleep(remaining - SPIN_TAIL);
            }
            while previous.elapsed() < target {
                std::hint::spin_loop();
            }
        }
    }
    *last_frame = Some(Instant::now());
}

/// re3's `InitialiseChangedLanguageSettings`: load the selected original GXT
/// archive, then let frontend text refresh on the next frame.  A read/parse
/// failure restores the active language instead of leaving preferences and
/// text resources out of sync.
pub fn reload_frontend_language(
    mut menu: Option<ResMut<CMenuManager>>,
    mut frontend_assets: Option<ResMut<CFrontendAssets>>,
    mut text: Option<ResMut<CText>>,
    mut before_rw: Option<ResMut<crate::core::Game::CGameBeforeRw>>,
) {
    let (Some(menu), Some(frontend_assets), Some(text), Some(before_rw)) = (
        menu.as_deref_mut(),
        frontend_assets.as_deref_mut(),
        text.as_deref_mut(),
        before_rw.as_deref_mut(),
    ) else {
        return;
    };
    if menu.front_end_reload_obr_txt_gxt == 0 {
        return;
    }

    match CFrontendAssets::load_language_text(&mut before_rw.file_mgr, menu.prefs_language) {
        Ok(assets) => {
            *text = CText::from_frontend_assets(&assets);
            *frontend_assets = assets;
            menu.language_loaded = true;
            menu.front_end_reload_obr_txt_gxt = 0;
        }
        Err(error) => {
            error!(
                "CMenuManager::InitialiseChangedLanguageSettings failed for language {}: {error}",
                menu.prefs_language
            );
            menu.prefs_language = language_for_text_file(&frontend_assets.language_file);
            menu.language_loaded = false;
            menu.front_end_reload_obr_txt_gxt = 0;
        }
    }
}

fn language_for_text_file(file: &str) -> i32 {
    (0..=4)
        .find(|language| CFrontendAssets::language_file(*language) == Some(file))
        .unwrap_or(0)
}

pub fn sync_frontend_custom_cursor(
    mut window_query: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor_query: Query<(&mut Node, &mut Visibility), With<FrontendMouseCursor>>,
) {
    if cfg!(target_os = "android") {
        for (_, mut visibility) in &mut cursor_query {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Ok((window, mut cursor_options)) = window_query.single_mut() else {
        return;
    };
    cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
    cursor_options.visible = false;
    let Some(position) = window.cursor_position() else {
        for (_, mut visibility) in &mut cursor_query {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let scale = window.height() / DEFAULT_SCREEN_HEIGHT;
    for (mut node, mut visibility) in &mut cursor_query {
        node.left = Val::Px(position.x);
        node.top = Val::Px(position.y);
        node.width = Val::Px(75.0 * scale);
        node.height = Val::Px(75.0 * scale);
        *visibility = Visibility::Visible;
    }
}

pub fn on_exit_frontend(
    mut commands: Commands,
    mut ui: ResMut<FrontendMenuUi>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    despawn_frontend(&mut commands, &mut ui);
    if !cfg!(target_os = "android") && let Ok(mut cursor) = cursor_options.single_mut() {
        cursor.visible = true;
    }
}

fn adjust_current_setting(
    menu: &mut CMenuManager,
    _controller: &mut CControllerConfigManager,
    _mblur: Option<&mut CMBlur>,
    direction: i32,
) -> bool {
    let entry = current_entry(menu);
    match entry.action {
        action if action == eMenuAction::MENUACTION_MOUSESENS as i32 => {
            menu.mouse_sensitivity = (menu.mouse_sensitivity + direction * 8).clamp(0, 200);
        }
        action if action == eMenuAction::MENUACTION_MUSICVOLUME as i32 => {
            menu.prefs_music_volume = (menu.prefs_music_volume + direction * 8).clamp(0, 127);
        }
        action if action == eMenuAction::MENUACTION_SFXVOLUME as i32 => {
            menu.prefs_sfx_volume = (menu.prefs_sfx_volume + direction * 8).clamp(0, 127);
        }
        action if action == eMenuAction::MENUACTION_BRIGHTNESS as i32 => {
            menu.prefs_brightness = (menu.prefs_brightness + direction * 32).clamp(0, 511);
        }
        action if action == eMenuAction::MENUACTION_DRAWDIST as i32 => {
            menu.prefs_lod = (menu.prefs_lod + direction as f32 * 0.1).clamp(0.8, 1.8);
        }
        action if action == eMenuAction::MENUACTION_RADIO as i32 => {
            menu.prefs_radio_station = (menu.prefs_radio_station + direction as i8).rem_euclid(10);
        }
        action if action == eMenuAction::MENUACTION_CTRLMETHOD as i32 => {
            menu.control_method = if menu.control_method == 0 { 1 } else { 0 };
        }
        action if action == eMenuAction::MENUACTION_FRAMELIMIT as i32 => {
            menu.prefs_max_fps = cycle_max_fps(menu.prefs_max_fps, direction);
        }
        action if action == eMenuAction::MENUACTION_SCREENRES as i32 => {
            if !menu.game_not_loaded || menu.video_modes.is_empty() {
                return false;
            }
            menu.display_video_mode = (menu.display_video_mode as i32 + direction)
                .rem_euclid(menu.video_modes.len() as i32)
                as usize;
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SPLIT as i32 => {
            menu.prefs_local_coop_split ^= 1;
        }
        action if action == eMenuAction::MENUACTION_FREE_CAM as i32 => {
            menu.prefs_free_cam = !menu.prefs_free_cam;
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SIDE as i32 => {
            menu.prefs_local_coop_swap_sides ^= 1;
        }
        _ => return false,
    }
    true
}

fn activate_current_setting(
    menu: &mut CMenuManager,
    controller: &mut CControllerConfigManager,
    mut mblur: Option<&mut CMBlur>,
) -> bool {
    let entry = current_entry(menu);
    match entry.action {
        action if action == eMenuAction::MENUACTION_GETKEY as i32 => {
            menu.waiting_for_new_key_bind = true;
            menu.pending_control_label = Some(entry.key.to_owned());
        }
        action if action == eMenuAction::MENUACTION_CTRLMETHOD as i32 => {
            menu.control_method = if menu.control_method == 0 { 1 } else { 0 };
        }
        action if action == eMenuAction::MENUACTION_INVVERT as i32 => {
            menu.mouse_invert = !menu.mouse_invert;
        }
        action if action == eMenuAction::MENUACTION_MOUSESTEER as i32 => {
            menu.disable_mouse_steering = !menu.disable_mouse_steering;
        }
        action if action == eMenuAction::MENUACTION_FREE_CAM as i32 => {
            menu.prefs_free_cam = !menu.prefs_free_cam;
        }
        action if action == eMenuAction::MENUACTION_FRAMESYNC as i32 => {
            menu.prefs_vsync_disp ^= 1;
            menu.prefs_vsync = menu.prefs_vsync_disp;
        }
        action if action == eMenuAction::MENUACTION_FRAMELIMIT as i32 => {
            menu.prefs_max_fps = cycle_max_fps(menu.prefs_max_fps, 1);
        }
        action if action == eMenuAction::MENUACTION_TRAILS as i32 => {
            menu.prefs_trails = !menu.prefs_trails;
            if let Some(blur) = mblur.as_deref_mut() {
                blur.blur_on = menu.prefs_trails;
                if !blur.blur_on {
                    blur.motion_blur_close();
                }
            }
        }
        action if action == eMenuAction::MENUACTION_SUBTITLES as i32 => {
            menu.prefs_show_subtitles ^= 1;
        }
        action if action == eMenuAction::MENUACTION_WIDESCREEN as i32 => {
            menu.prefs_use_wide_screen ^= 1;
        }
        action if action == eMenuAction::MENUACTION_SCREENRES as i32 => {
            if !menu.game_not_loaded || menu.video_modes.is_empty() {
                return false;
            }
            menu.prefs_video_mode = menu.display_video_mode;
            menu.apply_video_mode_requested = true;
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SPLIT as i32 => {
            menu.prefs_local_coop_split ^= 1;
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SIDE as i32 => {
            menu.prefs_local_coop_swap_sides ^= 1;
        }
        action if action == eMenuAction::MENUACTION_LANG_ENG as i32 => select_language(menu, 0),
        action if action == eMenuAction::MENUACTION_LANG_FRE as i32 => select_language(menu, 1),
        action if action == eMenuAction::MENUACTION_LANG_GER as i32 => select_language(menu, 2),
        action if action == eMenuAction::MENUACTION_LANG_ITA as i32 => select_language(menu, 3),
        action if action == eMenuAction::MENUACTION_LANG_SPA as i32 => select_language(menu, 4),
        action if action == eMenuAction::MENUACTION_RESTOREDEF as i32 => {
            restore_frontend_defaults(menu, controller);
        }
        // Sliders are selectable in re3, but their value changes only through
        // left/right (or their mouse hit areas), so Enter intentionally does
        // not mutate them.
        action
            if action == eMenuAction::MENUACTION_MOUSESENS as i32
                || action == eMenuAction::MENUACTION_MUSICVOLUME as i32
                || action == eMenuAction::MENUACTION_SFXVOLUME as i32
                || action == eMenuAction::MENUACTION_BRIGHTNESS as i32
                || action == eMenuAction::MENUACTION_DRAWDIST as i32
                || action == eMenuAction::MENUACTION_RADIO as i32 => {}
        _ => return false,
    }
    true
}

fn select_language(menu: &mut CMenuManager, language: i32) {
    if menu.prefs_language != language {
        menu.prefs_language = language;
        // Matches m_bFrontEnd_ReloadObrTxtGxt before InitialiseChangedLanguageSettings.
        menu.front_end_reload_obr_txt_gxt = 1;
    }
}

fn restore_frontend_defaults(menu: &mut CMenuManager, controller: &mut CControllerConfigManager) {
    match menu.curr_screen {
        screen if screen == eMenuScreen::MENUPAGE_CONTROLLER_PC as i32 => {
            menu.control_method = 0;
            menu.mouse_sensitivity = 100;
            menu.mouse_invert = true;
            menu.disable_mouse_steering = true;
            controller.InitDefaultControlConfiguration();
        }
        screen if screen == eMenuScreen::MENUPAGE_SOUND_SETTINGS as i32 => {
            menu.prefs_music_volume = 102;
            menu.prefs_sfx_volume = 102;
            menu.prefs_radio_station = 0;
        }
        screen if screen == eMenuScreen::MENUPAGE_DISPLAY_SETTINGS as i32 => {
            menu.prefs_brightness = 256;
            menu.prefs_lod = 1.2;
            menu.prefs_vsync = 1;
            menu.prefs_vsync_disp = 1;
            menu.prefs_max_fps = DEFAULT_MAX_FPS;
            menu.prefs_show_subtitles = 1;
            menu.prefs_use_wide_screen = 0;
            menu.prefs_trails = true;
            menu.prefs_local_coop_split = 0;
            menu.prefs_free_cam = false;
        }
        _ => {}
    }
}

fn controller_action_for_label(label: &str) -> Option<e_ControllerAction> {
    use e_ControllerAction::*;
    Some(match label {
        "FIRE WEAPON" => PED_FIREWEAPON,
        "NEXT WEAPON" => PED_CYCLE_WEAPON_RIGHT,
        "PREVIOUS WEAPON" => PED_CYCLE_WEAPON_LEFT,
        "GO FORWARD" => GO_FORWARD,
        "GO BACK" => GO_BACK,
        "GO LEFT" => GO_LEFT,
        "GO RIGHT" => GO_RIGHT,
        "SNIPER ZOOM IN" => PED_SNIPER_ZOOM_IN,
        "SNIPER ZOOM OUT" => PED_SNIPER_ZOOM_OUT,
        "ENTER / EXIT VEHICLE" => VEHICLE_ENTER_EXIT,
        "CHANGE CAMERA" => CAMERA_CHANGE_VIEW_ALL_SITUATIONS,
        "JUMP" => PED_JUMPING,
        "SPRINT" => PED_SPRINT,
        "LOOK BEHIND" => PED_LOOKBEHIND,
        "LOCK TARGET" => PED_LOCK_TARGET,
        "ACCELERATE" => VEHICLE_ACCELERATE,
        "BRAKE / REVERSE" => VEHICLE_BRAKE,
        "CHANGE RADIO STATION" => VEHICLE_CHANGE_RADIO_STATION,
        "HORN" => VEHICLE_HORN,
        "TOGGLE SUBMISSIONS" => TOGGLE_SUBMISSIONS,
        "HANDBRAKE" => VEHICLE_HANDBRAKE,
        "VEHICLE LOOK LEFT" => VEHICLE_LOOKLEFT,
        "VEHICLE LOOK RIGHT" => VEHICLE_LOOKRIGHT,
        "PREVIOUS TARGET" => PED_CYCLE_TARGET_LEFT,
        "NEXT TARGET" => PED_CYCLE_TARGET_RIGHT,
        "FIRST PERSON LOOK LEFT" => PED_1RST_PERSON_LOOK_LEFT,
        "FIRST PERSON LOOK RIGHT" => PED_1RST_PERSON_LOOK_RIGHT,
        "FIRST PERSON LOOK UP" => PED_1RST_PERSON_LOOK_UP,
        "FIRST PERSON LOOK DOWN" => PED_1RST_PERSON_LOOK_DOWN,
        "VEHICLE LOOK BEHIND" => VEHICLE_LOOKBEHIND,
        "TURRET LEFT" => VEHICLE_TURRETLEFT,
        "TURRET RIGHT" => VEHICLE_TURRETRIGHT,
        "TURRET UP" => VEHICLE_TURRETUP,
        "TURRET DOWN" => VEHICLE_TURRETDOWN,
        "CENTER CAMERA" => PED_CENTER_CAMERA_BEHIND_PLAYER,
        _ => return None,
    })
}

fn prepare_plain_screen_rows(
    menu: &mut CMenuManager,
    screen: i32,
    text: &Option<Res<CText>>,
    stats: Option<&CStats>,
    player_info: Option<&CPlayerInfo>,
    messages: Option<&CMessages>,
    timer: Option<&CTimer>,
) {
    menu.plain_screen_rows.clear();
    menu.first_visible_row_on_list = 0;
    if screen == eMenuScreen::MENUPAGE_STATS as i32 {
        let Some(stats) = stats else {
            return;
        };
        let total_progress = if menu.prefs_allow_nasty_game {
            stats.total_progress_in_game
        } else {
            stats.total_progress_in_game.saturating_sub(1)
        };
        let percent = if total_progress == 0 {
            0
        } else {
            (stats.progress_made * 100 / total_progress).clamp(0, 100)
        };
        let mut row = |key: &str, value: String| {
            menu.plain_screen_rows.push((gxt(text, key), value));
        };
        let out_of = gxt(text, "FEST_OO");
        row("PL_STAT", String::new());
        row("PER_COM", percent.to_string());
        row("NMISON", stats.missions_given.to_string());
        row(
            "FEST_MP",
            format!(
                "{} {} {}",
                stats.missions_passed, out_of, stats.total_number_missions
            ),
        );
        if menu.prefs_allow_nasty_game {
            row(
                "FEST_RP",
                format!(
                    "{} {} {}",
                    stats.number_kill_frenzies_passed, out_of, stats.total_number_kill_frenzies
                ),
            );
        }
        if let Some(info) = player_info {
            row(
                "PERPIC",
                format!(
                    "{} {} {}",
                    info.m_nCollectedPackages, out_of, info.m_nTotalPackages
                ),
            );
        }
        row(
            "NOUNIF",
            format!(
                "{} {} {}",
                stats.number_of_unique_jumps_found, out_of, stats.total_number_of_unique_jumps
            ),
        );
        row("DAYSPS", stats.days_passed.to_string());
        if menu.prefs_allow_nasty_game {
            row("PE_WAST", stats.people_killed_by_player.to_string());
            row("PE_WSOT", stats.people_killed_by_others.to_string());
        }
        row("CAR_EXP", stats.cars_exploded.to_string());
        row("TM_BUST", stats.times_arrested.to_string());
        row("TM_DED", stats.times_died.to_string());
        // ePedType is laid out exactly as re3: gangs 1..9 are 7..15 and
        // PEDTYPE_CRIMINAL is 18.
        let gang_kills = stats.peds_killed_of_this_type[7..=15].iter().sum::<i32>();
        row("GNG_WST", gang_kills.to_string());
        row("DED_CRI", stats.peds_killed_of_this_type[18].to_string());
        row("HEL_DST", stats.helis_destroyed.to_string());
        row("KGS_EXP", stats.kgs_of_explosives_used.to_string());
        let accuracy = if stats.instant_hits_fired_by_player == 0 {
            0
        } else {
            stats.instant_hits_hit_by_player * 100 / stats.instant_hits_fired_by_player
        };
        row("ACCURA", format!("{accuracy}%"));
        for (key, value) in [
            ("ELBURRO", stats.el_burro_time),
            ("FEST_R1", stats.record_4x4_one),
            ("FEST_R2", stats.record_4x4_two),
            ("FEST_R3", stats.record_4x4_three),
            ("FEST_RM", stats.record_4x4_mayhem),
            ("FEST_LF", stats.longest_flight_in_dodo),
            ("FEST_BD", stats.time_taken_defuse_mission),
        ] {
            if value > 0 {
                row(key, value.to_string());
            }
        }
        row("CAR_CRU", stats.cars_crushed.to_string());
        if stats.highest_scores[0] > 0 {
            row("FEST_BB", String::new());
            row("FEST_H0", stats.highest_scores[0].to_string());
        }
        if stats.highest_scores[1..=4].iter().sum::<i32>() > 0 {
            row("FEST_GC", String::new());
        }
        for (key, score) in ["FEST_H1", "FEST_H2", "FEST_H3", "FEST_H4"]
            .into_iter()
            .zip(stats.highest_scores[1..=4].iter().copied())
        {
            if score > 0 {
                row(key, score.to_string());
            }
        }
        if menu.prefs_language == 0 {
            row(
                "FEST_DF",
                format!("{:.2}", stats.distance_travelled_on_foot / 1670.0),
            );
            row(
                "FEST_DC",
                format!("{:.2}", stats.distance_travelled_in_vehicle / 1670.0),
            );
            row("MMRAIN", stats.mm_rain.to_string());
            row(
                "MXCARD",
                format!("{:.2}", stats.maximum_jump_distance * 3.33),
            );
            row("MXCARJ", format!("{:.2}", stats.maximum_jump_height * 3.33));
        } else {
            row(
                "FESTDFM",
                format!("{:.2}", stats.distance_travelled_on_foot),
            );
            row(
                "FESTDCM",
                format!("{:.2}", stats.distance_travelled_in_vehicle),
            );
            row("MMRAIN", stats.mm_rain.to_string());
            row("MXCARDM", format!("{:.2}", stats.maximum_jump_distance));
            row("MXCARJM", format!("{:.2}", stats.maximum_jump_height));
        }
        row("MXFLIP", stats.maximum_jump_flips.to_string());
        row("MXJUMP", stats.maximum_jump_spins.to_string());
        let stunt_key = match stats.best_stunt_jump {
            1 => "INSTUN",
            2 => "PRINST",
            3 => "DBINST",
            4 => "DBPINS",
            5 => "TRINST",
            6 => "PRTRST",
            7 => "QUINST",
            8 => "PQUINS",
            _ => "NOSTUC",
        };
        row("BSTSTU", gxt(text, stunt_key));
        row("PASDRO", stats.passengers_dropped_off_with_taxi.to_string());
        row("MONTAX", stats.money_made_with_taxi.to_string());
        row("FEST_LS", stats.lives_saved_with_ambulance.to_string());
        row("FEST_HA", stats.highest_level_ambulance_mission.to_string());
        row("FEST_CC", stats.criminals_caught.to_string());
        row("FEST_FE", stats.fires_extinguished.to_string());
        row(
            "DAYPLC",
            timer
                .map(|timer| timer.time_in_milliseconds().saturating_add(100))
                .unwrap_or(100)
                .to_string(),
        );
    } else if screen == eMenuScreen::MENUPAGE_BRIEFS as i32 {
        let Some(messages) = messages else {
            return;
        };
        for brief in messages
            .previous_briefs
            .iter()
            .filter(|brief| brief.text.is_some())
        {
            let mut expanded =
                CMessages::insert_number_in_string(brief.text.as_deref(), brief.numbers);
            CMessages::insert_string_in_string(&mut expanded, brief.string.as_deref());
            menu.plain_screen_rows
                .push((unicode_to_ascii(&expanded), String::new()));
        }
    }
    menu.total_list_row = menu.plain_screen_rows.len() as i32;
}

fn activate_entry(
    commands: &mut Commands,
    ui: &mut ResMut<FrontendMenuUi>,
    menu: &mut CMenuManager,
    mut memory_card: Option<&mut CMemoryCard>,
    mut pc_save: Option<&mut C_PcSave>,
    mut game_storage: Option<&mut CGenericGameStorage>,
    text: &Option<Res<CText>>,
    next_state: &mut ResMut<NextState<GtaAppState>>,
    app_exit: &mut MessageWriter<AppExit>,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    install: Option<&OriginalGtaInstall>,
    stats: Option<&CStats>,
    player_info: Option<&CPlayerInfo>,
    messages: Option<&CMessages>,
    timer: Option<&CTimer>,
    save_request: &mut PcSaveRequest,
    local_coop: &mut LocalCoopSession,
) {
    let page = menu_page(menu.curr_screen);
    let Some(entry) = page.entries.get(menu.curr_option as usize).copied() else {
        return;
    };

    if entry.target == eMenuScreen::MENUPAGE_NEW_GAME_RELOAD as i32 && menu.game_not_loaded {
        local_coop.enabled = false;
        menu.want_to_restart = true;
        menu.menu_active = false;
        despawn_frontend(commands, ui);
        next_state.set(GtaAppState::LoadingGame);
        return;
    }

    match entry.action {
        action if action == eMenuAction::MENUACTION_LOCAL_COOP as i32 => {
            local_coop.enabled = true;
            local_coop.player_two_spawned = false;
            menu.want_to_restart = true;
            menu.menu_active = false;
            despawn_frontend(commands, ui);
            next_state.set(GtaAppState::LoadingGame);
        }
        action
            if action == eMenuAction::MENUACTION_RESUME as i32
                || action == eMenuAction::MENUACTION_RESUME_FROM_SAVEZONE as i32 =>
        {
            menu.menu_active = false;
            despawn_frontend(commands, ui);
            next_state.set(GtaAppState::InGame);
        }
        action if action == eMenuAction::MENUACTION_NEWGAME as i32 => {
            local_coop.enabled = false;
            menu.want_to_restart = true;
            menu.menu_active = false;
            despawn_frontend(commands, ui);
            next_state.set(GtaAppState::LoadingGame);
        }
        action if action == eMenuAction::MENUACTION_CANCELGAME as i32 => {
            app_exit.write(AppExit::Success);
        }
        action if action == eMenuAction::MENUACTION_DONTCANCEL as i32 => {
            go_back(commands, ui, menu, text, txd, images);
        }
        action if action == eMenuAction::MENUACTION_CHECKSAVE as i32 => {
            if let Some(slot) = select_save_slot(menu, memory_card.as_deref_mut(), entry) {
                menu.curr_save_slot = slot as i32;
                change_screen(
                    commands,
                    ui,
                    menu,
                    entry.target,
                    0,
                    text,
                    memory_card.as_deref(),
                    txd,
                    images,
                );
            }
        }
        action
            if action == eMenuAction::MENUACTION_CHANGEMENU as i32
                || action == eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU as i32
                || action == eMenuAction::MENUACTION_LOADRADIO as i32
                || action == eMenuAction::MENUACTION_KEYBOARDCTRLS as i32
                || action == eMenuAction::MENUACTION_PLAYERSETUP as i32 =>
        {
            if matches!(entry.target, 1 | 3) {
                prepare_plain_screen_rows(
                    menu,
                    entry.target,
                    text,
                    stats,
                    player_info,
                    messages,
                    timer,
                );
            }
            if entry.action == eMenuAction::MENUACTION_PLAYERSETUP as i32 {
                enumerate_player_setup_skins(menu, text, install);
                menu.first_visible_row_on_list = 0;
                menu.selected_list_row = 0;
                menu.player_setup_focus = 0;
                menu.player_setup_scrollbar_top_margin = 0.0;
                menu.player_setup_repeat_direction = 0;
                menu.player_setup_repeat_at = None;
                sync_player_setup_preview_name(menu);
            }
            if entry.action == eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU as i32 {
                if let Some(card) = memory_card.as_deref_mut() {
                    populate_slot_entries_from_memory_card(card);
                }
            }
            if entry.save_slot >= 0
                && select_save_slot(menu, memory_card.as_deref_mut(), entry).is_none()
            {
                return;
            }
            if entry.target == eMenuScreen::MENUPAGE_LOADING_IN_PROGRESS as i32 {
                if let Some(card) = memory_card.as_deref_mut() {
                    card.LoadSlotToBuffer(menu.curr_save_slot as usize);
                    if is_memory_card_error(card) {
                        change_screen(
                            commands,
                            ui,
                            menu,
                            eMenuScreen::MENUPAGE_LOAD_FAILED as i32,
                            0,
                            text,
                            Some(&*card),
                            txd,
                            images,
                        );
                        return;
                    }
                }
                menu.menu_active = false;
                despawn_frontend(commands, ui);
                next_state.set(GtaAppState::LoadingGame);
                return;
            }
            if entry.target == eMenuScreen::MENUPAGE_DELETING as i32 {
                if let (Some(card), Some(pc_save), Some(storage)) = (
                    memory_card.as_deref_mut(),
                    pc_save.as_deref_mut(),
                    game_storage.as_deref_mut(),
                ) {
                    if let Err(error) = pc_save.DeleteSlotWithMemoryCard(
                        storage,
                        card,
                        menu.curr_save_slot as usize,
                    ) {
                        error!("C_PcSave delete failed: {error}");
                    }
                    if is_memory_card_error(card) {
                        change_screen(
                            commands,
                            ui,
                            menu,
                            eMenuScreen::MENUPAGE_DELETE_FAILED as i32,
                            0,
                            text,
                            Some(&*card),
                            txd,
                            images,
                        );
                        return;
                    }
                    change_screen(
                        commands,
                        ui,
                        menu,
                        eMenuScreen::MENUPAGE_DELETE_SUCCESS as i32,
                        1,
                        text,
                        Some(&*card),
                        txd,
                        images,
                    );
                    return;
                }
            }
            if entry.target == eMenuScreen::MENUPAGE_SAVING_IN_PROGRESS as i32 {
                let Ok(slot) = usize::try_from(menu.curr_save_slot) else {
                    error!(target: "re3bevy::save", "save requested without a selected slot");
                    return;
                };
                save_request.state = PcSaveRequestState::Requested(slot);
            }
            if entry.key == "FEDS_TB" || entry.target == eMenuScreen::MENUPAGE_NONE as i32 {
                go_back(commands, ui, menu, text, txd, images);
            } else {
                change_screen(
                    commands,
                    ui,
                    menu,
                    entry.target,
                    0,
                    text,
                    memory_card.as_deref(),
                    txd,
                    images,
                );
            }
        }
        _ => {}
    }
}

fn play_frontend_entry_sound(
    audio: Option<&mut cDMAudio>,
    menu: &CMenuManager,
    memory_card: Option<&CMemoryCard>,
) {
    let Some(audio) = audio else {
        return;
    };
    let entry = current_entry(menu);
    let is_back = entry.key == "FEDS_TB" || entry.key == "FESZ_CA";
    let invalid_slot = usize::try_from(entry.save_slot)
        .ok()
        .and_then(|slot| memory_card.map(|card| card.GetInfoOnSpecificSlot(slot)))
        .is_some_and(|slot| {
            (menu.curr_screen == eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT as i32
                && slot != SLOTINFO::SLOT_PRESENT)
                || (menu.curr_screen == eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32
                    && slot == SLOTINFO::SLOT_NOTPRESENT)
        });
    let sound = if invalid_slot {
        SOUND_FRONTEND_FAIL
    } else if is_back {
        SOUND_FRONTEND_MENU_BACK
    } else if entry.target == eMenuScreen::MENUPAGE_NEW_GAME_RELOAD as i32 && menu.game_not_loaded {
        SOUND_FRONTEND_MENU_STARTING
    } else if matches!(
        entry.action,
        action if action == eMenuAction::MENUACTION_CHANGEMENU as i32
            || action == eMenuAction::MENUACTION_POPULATESLOTS_CHANGEMENU as i32
            || action == eMenuAction::MENUACTION_LOADRADIO as i32
            || action == eMenuAction::MENUACTION_KEYBOARDCTRLS as i32
            || action == eMenuAction::MENUACTION_PLAYERSETUP as i32
    ) {
        SOUND_FRONTEND_MENU_NEW_PAGE
    } else {
        SOUND_FRONTEND_MENU_SETTING_CHANGE
    };
    audio.PlayFrontEndSound(sound as u16, 0);
}

fn is_memory_card_error(card: &CMemoryCard) -> bool {
    !matches!(
        card.GetError(),
        MemoryCardError::ERR_NONE | MemoryCardError::NO_ERR_SUCCESS
    )
}

fn select_save_slot(
    menu: &mut CMenuManager,
    memory_card: Option<&mut CMemoryCard>,
    entry: MenuEntry,
) -> Option<usize> {
    let slot = usize::try_from(entry.save_slot).ok()?;
    if slot >= MAX_SLOTS {
        return None;
    }
    let slot_state = memory_card
        .as_deref()
        .map(|card| card.GetInfoOnSpecificSlot(slot))
        .unwrap_or(SLOTINFO::SLOT_NOTPRESENT);
    match menu.curr_screen {
        screen if screen == eMenuScreen::MENUPAGE_CHOOSE_LOAD_SLOT as i32 => {
            if slot_state == SLOTINFO::SLOT_NOTPRESENT {
                return None;
            }
        }
        screen if screen == eMenuScreen::MENUPAGE_CHOOSE_DELETE_SLOT as i32 => {
            if slot_state == SLOTINFO::SLOT_NOTPRESENT {
                return None;
            }
        }
        _ => {}
    }
    menu.curr_save_slot = slot as i32;
    Some(slot)
}

fn populate_slot_entries_from_memory_card(card: &mut CMemoryCard) {
    for slot in 0..MAX_SLOTS {
        if card
            .GetNameOfSavedGame(slot)
            .is_some_and(|name| !name.is_empty())
        {
            card.Slots[slot] = SLOTINFO::SLOT_PRESENT;
        }
    }
}

fn enumerate_player_setup_skins(
    menu: &mut CMenuManager,
    text: &Option<Res<CText>>,
    install: Option<&OriginalGtaInstall>,
) {
    menu.player_setup_skins.clear();
    let default_display_name = gxt(text, "FET_DSN");
    let default_display_name = default_display_name
        .strip_suffix(".bmp")
        .or_else(|| default_display_name.strip_suffix(".BMP"))
        .unwrap_or(&default_display_name)
        .to_owned();
    menu.player_setup_skins.push(FrontendSkinInfo {
        skin_id: 0,
        original_name: DEFAULT_SKIN_NAME.to_owned(),
        displayed_name: default_display_name,
        date: String::new(),
    });

    if let Some(install) = install {
        let skins_dir = install.resolve_existing_case_path("skins");
        if let Ok(entries) = fs::read_dir(skins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file()
                    || !path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("bmp"))
                {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if stem == DEFAULT_SKIN_NAME {
                    continue;
                }
                let skin_id = menu.player_setup_skins.len();
                let date = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .map(player_setup_short_date)
                    .unwrap_or_default();
                menu.player_setup_skins.push(FrontendSkinInfo {
                    skin_id,
                    original_name: stem.chars().take(32).collect(),
                    displayed_name: player_setup_display_name(stem),
                    date,
                });
            }
        }
    }
    menu.player_setup_skins_enumerated = true;
    menu.total_list_row = menu.player_setup_skins.len() as i32;
}

fn player_setup_display_name(filename_stem: &str) -> String {
    let replaced: String = filename_stem
        .chars()
        .take(39)
        .map(|character| match character {
            '_' | '@' => ' ',
            '{' => '(',
            '}' => ')',
            '£' => '$',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    let mut capitalise = true;
    replaced
        .chars()
        .map(|character| {
            let output = if capitalise {
                character.to_ascii_uppercase()
            } else {
                character
            };
            capitalise = character == ' ';
            output
        })
        .collect()
}

fn player_setup_short_date(time: SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    let (year, month, day) = civil_date_from_unix_days((duration.as_secs() / 86_400) as i64);
    format!("{day:02}/{month:02}/{year:04}")
}

fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn sync_player_setup_preview_name(menu: &mut CMenuManager) {
    if let Some(skin) = menu
        .player_setup_skins
        .get(menu.selected_list_row.max(0) as usize)
    {
        menu.skin_name.clone_from(&skin.original_name);
    }
}

fn update_player_setup_scrollbar(menu: &mut CMenuManager) {
    menu.total_list_row = menu.player_setup_skins.len() as i32;
    menu.player_setup_scrollbar_top_margin = if menu.total_list_row > 0 {
        263.0 / menu.total_list_row as f32 * menu.first_visible_row_on_list as f32
    } else {
        0.0
    };
}

fn player_setup_repeat_ready(menu: &mut CMenuManager, direction: i8) -> bool {
    let now = Instant::now();
    if menu.player_setup_repeat_direction != direction {
        menu.player_setup_repeat_direction = direction;
        menu.player_setup_repeat_at = Some(now);
        return false;
    }
    let ready = menu
        .player_setup_repeat_at
        .is_some_and(|last| now.duration_since(last) >= Duration::from_millis(200));
    if ready {
        menu.player_setup_repeat_at = Some(now);
    }
    ready
}

fn player_setup_scroll_by(menu: &mut CMenuManager, direction: i32) -> bool {
    let total = menu.player_setup_skins.len() as i32;
    if total == 0 {
        return false;
    }
    let old_selected = menu.selected_list_row;
    let old_first = menu.first_visible_row_on_list;
    let visible = PLAYERSETUP_MAX_VISIBLE_ROWS as i32;
    if direction < 0 {
        if menu.selected_list_row == menu.first_visible_row_on_list
            && menu.first_visible_row_on_list > 0
        {
            menu.selected_list_row -= 1;
            menu.first_visible_row_on_list -= 1;
        } else if menu.selected_list_row > 0 {
            menu.selected_list_row -= 1;
        }
    } else if menu.selected_list_row == menu.first_visible_row_on_list + visible - 1
        && menu.first_visible_row_on_list < total - visible
    {
        menu.selected_list_row += 1;
        menu.first_visible_row_on_list += 1;
    } else if menu.selected_list_row < total - 1 {
        menu.selected_list_row += 1;
    }
    update_player_setup_scrollbar(menu);
    old_selected != menu.selected_list_row || old_first != menu.first_visible_row_on_list
}

fn player_setup_page_by(menu: &mut CMenuManager, direction: i32) -> bool {
    let total = menu.player_setup_skins.len() as i32;
    let visible = PLAYERSETUP_MAX_VISIBLE_ROWS as i32;
    if total <= visible {
        return false;
    }
    let old_selected = menu.selected_list_row;
    let old_first = menu.first_visible_row_on_list;
    if direction < 0 {
        if menu.first_visible_row_on_list > 0 {
            menu.first_visible_row_on_list = (menu.first_visible_row_on_list - visible).max(0);
            menu.selected_list_row = menu
                .selected_list_row
                .min(menu.first_visible_row_on_list + visible - 1);
        } else {
            menu.selected_list_row = 0;
        }
    } else if menu.first_visible_row_on_list < total - visible {
        menu.first_visible_row_on_list =
            (menu.first_visible_row_on_list + visible).min(total - visible);
        menu.selected_list_row = menu.selected_list_row.max(menu.first_visible_row_on_list);
    } else {
        menu.first_visible_row_on_list = total - visible;
        menu.selected_list_row = total - 1;
    }
    update_player_setup_scrollbar(menu);
    old_selected != menu.selected_list_row || old_first != menu.first_visible_row_on_list
}

fn player_setup_select_row(menu: &mut CMenuManager, row: usize) -> bool {
    if menu.player_setup_skins.is_empty() {
        return false;
    }
    let row = row.min(menu.player_setup_skins.len() - 1) as i32;
    let old_selected = menu.selected_list_row;
    let old_first = menu.first_visible_row_on_list;
    menu.selected_list_row = row;
    let visible = PLAYERSETUP_MAX_VISIBLE_ROWS as i32;
    if row < menu.first_visible_row_on_list {
        menu.first_visible_row_on_list = row;
    } else if row >= menu.first_visible_row_on_list + visible {
        menu.first_visible_row_on_list = row - visible + 1;
    }
    update_player_setup_scrollbar(menu);
    old_selected != menu.selected_list_row || old_first != menu.first_visible_row_on_list
}

fn change_screen(
    commands: &mut Commands,
    ui: &mut ResMut<FrontendMenuUi>,
    menu: &mut CMenuManager,
    screen: i32,
    option: i32,
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
) {
    let screen = if screen == eMenuScreen::MENUPAGE_NONE as i32 {
        if menu.game_not_loaded {
            eMenuScreen::MENUPAGE_START_MENU as i32
        } else {
            eMenuScreen::MENUPAGE_PAUSE_MENU as i32
        }
    } else {
        screen
    };
    change_screen_state(menu, screen, option, true, true);
    spawn_frontend(commands, ui, menu, text, memory_card, txd, images);
}

fn go_back(
    commands: &mut Commands,
    ui: &mut ResMut<FrontendMenuUi>,
    menu: &mut CMenuManager,
    text: &Option<Res<CText>>,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
) {
    if menu.curr_screen == eMenuScreen::MENUPAGE_SKIN_SELECT as i32 {
        menu.skin_name.clone_from(&menu.prefs_skin_file);
        menu.total_list_row = 0;
        menu.first_visible_row_on_list = 0;
        menu.selected_list_row = 0;
        menu.player_setup_scrollbar_top_margin = 0.0;
        menu.player_setup_repeat_direction = 0;
        menu.player_setup_repeat_at = None;
    }
    go_back_state(menu);
    spawn_frontend(commands, ui, menu, text, None, txd, images);
}

fn spawn_frontend(
    commands: &mut Commands,
    ui: &mut ResMut<FrontendMenuUi>,
    menu: &CMenuManager,
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
) {
    if let Some(root) = ui.root.take() {
        commands.entity(root).despawn();
    }

    let page = menu_page(menu.curr_screen);
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            FrontendRoot,
            Name::new("FrontEndMenuManager"),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        spawn_background(parent, txd, menu_background_sprite(menu.curr_screen));
        if let Some(cursor) = txd.images.get("mouse") {
            parent.spawn((
                FrontendMouseCursor,
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: pct(75.0, DEFAULT_SCREEN_WIDTH),
                    height: pct(75.0, DEFAULT_SCREEN_HEIGHT),
                    ..default()
                },
                ImageNode {
                    image: cursor.clone(),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                ZIndex(1000),
                Name::new("FrontEndMenuManager Mouse"),
            ));
        }
        if menu.curr_screen == eMenuScreen::MENUPAGE_SKIN_SELECT as i32 {
            spawn_player_setup_page(parent, txd, images, menu, text);
            return;
        }
        spawn_menu_logo(parent, txd, menu.curr_screen);

        spawn_bitmap_text(
            parent,
            txd,
            images,
            BitmapTextSpawn {
                entry_index: usize::MAX,
                text: gxt(text, page.title).to_uppercase(),
                colour: HEADER,
                style: FONT_HEADING,
                scale_x: 0.84,
                scale_y: 1.6,
                left: 35.0,
                top: DEFAULT_SCREEN_HEIGHT - 93.0,
                align: TextAlign::Right,
                track_selection: false,
                shadow: false,
                value_column: false,
                name: "FrontendPageTitle",
            },
        );

        if matches!(menu.curr_screen, 1 | 3) {
            spawn_plain_screen_rows(parent, txd, images, menu);
        }

        let layout = menu_layout(menu.curr_screen);
        let mut option_row = 0usize;
        for (index, entry) in page.entries.iter().enumerate() {
            let selected = index == menu.curr_option as usize;
            let colour = entry_colour(*entry, selected);
            let text_value = entry_text(menu, text, memory_card, *entry);
            if is_selectable(*entry) {
                let y = layout.entry_y(option_row);
                option_row += 1;
                parent.spawn((
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        left: pct(10.0, DEFAULT_SCREEN_WIDTH),
                        right: pct(11.0, DEFAULT_SCREEN_WIDTH),
                        top: pct(y - 2.0, DEFAULT_SCREEN_HEIGHT),
                        height: pct(layout.usable_line_height() + 2.0, DEFAULT_SCREEN_HEIGHT),
                        ..default()
                    },
                    BackgroundColor(SELECTION_BAR),
                    if selected {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    FrontendSelectionBar { entry_index: index },
                    Name::new(format!("FrontendSelectionBar{index}")),
                ));
                parent.spawn((
                    Button,
                    Hovered::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: if layout.centre {
                            Val::Px(0.0)
                        } else {
                            pct(layout.column_width, DEFAULT_SCREEN_WIDTH)
                        },
                        top: pct(y, DEFAULT_SCREEN_HEIGHT),
                        width: if layout.centre {
                            Val::Percent(100.0)
                        } else {
                            pct(
                                DEFAULT_SCREEN_WIDTH - layout.column_width - 40.0,
                                DEFAULT_SCREEN_WIDTH,
                            )
                        },
                        height: pct(layout.line_height, DEFAULT_SCREEN_HEIGHT),
                        justify_content: if layout.centre {
                            JustifyContent::Center
                        } else {
                            JustifyContent::FlexStart
                        },
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    FrontendButton { entry_index: index },
                    Name::new(format!("FrontendEntry{index}")),
                ));
                spawn_entry_text(parent, txd, images, layout, index, y, text_value, colour);
                if let Some(progress) = slider_progress(menu, entry.action) {
                    spawn_slider(parent, layout, index, entry.action, y, progress);
                } else {
                    spawn_entry_value(
                        parent,
                        txd,
                        images,
                        layout,
                        index,
                        y,
                        entry_value(menu, None, text, *entry),
                    );
                }
            } else if entry.action == eMenuAction::MENUACTION_LABEL as i32 {
                spawn_label_text(parent, txd, images, index, text_value);
            } else {
                let y = layout.entry_y(option_row);
                option_row += 1;
                spawn_entry_text(parent, txd, images, layout, index, y, text_value, colour);
                if let Some(progress) = slider_progress(menu, entry.action) {
                    spawn_slider(parent, layout, index, entry.action, y, progress);
                } else {
                    spawn_entry_value(
                        parent,
                        txd,
                        images,
                        layout,
                        index,
                        y,
                        entry_value(menu, None, text, *entry),
                    );
                }
            }
        }
    });
    ui.root = Some(root);
}

fn spawn_plain_screen_rows(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    menu: &CMenuManager,
) {
    const VISIBLE_ROWS: usize = 15;
    let first = menu.first_visible_row_on_list.max(0) as usize;
    for (visible, (left, right)) in menu
        .plain_screen_rows
        .iter()
        .skip(first)
        .take(VISIBLE_ROWS)
        .enumerate()
    {
        let y = 42.0 + visible as f32 * 18.0;
        spawn_bitmap_text(
            parent,
            txd,
            images,
            BitmapTextSpawn {
                entry_index: usize::MAX - 100 - visible,
                text: left.clone(),
                colour: NORMAL,
                style: FONT_BANK,
                scale_x: 0.45,
                scale_y: 0.7,
                left: 42.0,
                top: y,
                align: TextAlign::Left,
                track_selection: false,
                shadow: true,
                value_column: false,
                name: "FrontendPlainRow",
            },
        );
        if !right.is_empty() {
            spawn_bitmap_text(
                parent,
                txd,
                images,
                BitmapTextSpawn {
                    entry_index: usize::MAX - 200 - visible,
                    text: right.clone(),
                    colour: NORMAL,
                    style: FONT_BANK,
                    scale_x: 0.45,
                    scale_y: 0.7,
                    left: 42.0,
                    top: y,
                    align: TextAlign::Right,
                    track_selection: false,
                    shadow: true,
                    value_column: true,
                    name: "FrontendPlainValue",
                },
            );
        }
    }
}

fn spawn_player_setup_page(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    menu: &CMenuManager,
    text: &Option<Res<CText>>,
) {
    if let Some(preview) = txd.images.get("__player_skin_preview") {
        parent.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode {
                image: preview.clone(),
                image_mode: NodeImageMode::Stretch,
                color: Color::WHITE,
                ..default()
            },
            Name::new("CPlayerSkin Frontend Preview"),
        ));
    }

    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: usize::MAX,
            text: gxt(text, "FET_PS").to_uppercase(),
            colour: HEADER,
            style: FONT_HEADING,
            scale_x: 0.84,
            scale_y: 1.6,
            left: 35.0,
            top: DEFAULT_SCREEN_HEIGHT - 93.0,
            align: TextAlign::Right,
            track_selection: false,
            shadow: false,
            value_column: false,
            name: "FrontendPlayerSetupTitle",
        },
    );

    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(200.0, DEFAULT_SCREEN_WIDTH),
            top: pct(28.0, DEFAULT_SCREEN_HEIGHT),
            width: pct(404.0, DEFAULT_SCREEN_WIDTH),
            height: pct(295.0, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(Color::srgba(
            200.0 / 255.0,
            200.0 / 255.0,
            50.0 / 255.0,
            50.0 / 255.0,
        )),
        Name::new("FrontendPlayerSetupListBackground"),
    ));

    let header_colour = if menu.player_setup_focus == 0 {
        SELECTED
    } else {
        NORMAL
    };
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: usize::MAX - 1,
            text: gxt(text, "FES_SKN"),
            colour: header_colour,
            style: FONT_HEADING,
            scale_x: if matches!(menu.prefs_language, 1 | 4) {
                0.6
            } else {
                0.9
            },
            scale_y: 0.9,
            left: 220.0,
            top: 28.0,
            align: TextAlign::Left,
            track_selection: false,
            shadow: false,
            value_column: false,
            name: "FrontendPlayerSetupSkinHeader",
        },
    );
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: usize::MAX - 2,
            text: gxt(text, "FES_DAT"),
            colour: header_colour,
            style: FONT_HEADING,
            scale_x: 0.9,
            scale_y: 0.9,
            left: 56.0,
            top: 28.0,
            align: TextAlign::Right,
            track_selection: false,
            shadow: false,
            value_column: false,
            name: "FrontendPlayerSetupDateHeader",
        },
    );

    let first = menu.first_visible_row_on_list.max(0) as usize;
    let end = (first + PLAYERSETUP_MAX_VISIBLE_ROWS).min(menu.player_setup_skins.len());
    for row in first..end {
        let skin = &menu.player_setup_skins[row];
        let y = 46.0 + (row - first) as f32 * 9.0;
        let colour = if row == menu.selected_list_row.max(0) as usize {
            Color::WHITE
        } else if skin.original_name == menu.prefs_skin_file {
            Color::srgb(1.0, 1.0, 155.0 / 255.0)
        } else {
            Color::srgb(155.0 / 255.0, 155.0 / 255.0, 155.0 / 255.0)
        };
        spawn_bitmap_text(
            parent,
            txd,
            images,
            BitmapTextSpawn {
                entry_index: usize::MAX - 3 - row,
                text: skin.displayed_name.clone(),
                colour,
                style: FONT_BANK,
                scale_x: 0.4,
                scale_y: 0.6,
                left: 220.0,
                top: y,
                align: TextAlign::Left,
                track_selection: false,
                shadow: false,
                value_column: false,
                name: "FrontendPlayerSetupSkinRow",
            },
        );
        if row != 0 {
            spawn_bitmap_text(
                parent,
                txd,
                images,
                BitmapTextSpawn {
                    entry_index: usize::MAX - 100 - row,
                    text: skin.date.clone(),
                    colour,
                    style: FONT_BANK,
                    scale_x: 0.4,
                    scale_y: 0.6,
                    left: 56.0,
                    top: y,
                    align: TextAlign::Right,
                    track_selection: false,
                    shadow: false,
                    value_column: false,
                    name: "FrontendPlayerSetupDateRow",
                },
            );
        }
        spawn_player_setup_control(
            parent,
            PlayerSetupControl::SkinRow(row),
            200.0,
            47.0 + (row - first) as f32 * 9.0,
            404.0,
            9.0,
        );
    }

    spawn_player_setup_scrollbar(parent, txd, menu);

    let use_enabled = menu.skin_name != menu.prefs_skin_file;
    let use_colour = if !use_enabled {
        DISABLED
    } else if menu.player_setup_focus == 1 {
        SELECTED
    } else {
        NORMAL
    };
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: usize::MAX - 500,
            text: gxt(text, "FES_SET"),
            colour: use_colour,
            style: FONT_HEADING,
            scale_x: 0.45,
            scale_y: 0.7,
            left: 200.0,
            top: 328.0,
            align: TextAlign::Left,
            track_selection: false,
            shadow: true,
            value_column: false,
            name: "FrontendPlayerSetupUseSkin",
        },
    );
    if use_enabled {
        spawn_player_setup_control(
            parent,
            PlayerSetupControl::UseSkin,
            200.0,
            325.0,
            150.0,
            26.0,
        );
        let apply_scale_x = match menu.prefs_language {
            1 => 1.1,
            2 => 0.85,
            3 | 4 => 1.4,
            _ => 1.9,
        };
        spawn_bitmap_text(
            parent,
            txd,
            images,
            BitmapTextSpawn {
                entry_index: usize::MAX - 501,
                text: gxt(text, "FET_APL"),
                colour: Color::srgba(1.0, 217.0 / 255.0, 106.0 / 255.0, 120.0 / 255.0),
                style: FONT_HEADING,
                scale_x: apply_scale_x,
                scale_y: 1.9,
                left: 20.0,
                top: 220.0,
                align: TextAlign::Left,
                track_selection: false,
                shadow: false,
                value_column: false,
                name: "FrontendPlayerSetupApply",
            },
        );
    }

    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: usize::MAX - 502,
            text: gxt(text, "FEDS_TB"),
            colour: if menu.player_setup_focus == 2 {
                SELECTED
            } else {
                NORMAL
            },
            style: FONT_HEADING,
            scale_x: 0.45,
            scale_y: 0.7,
            left: 33.0,
            top: 328.0,
            align: TextAlign::Right,
            track_selection: false,
            shadow: true,
            value_column: false,
            name: "FrontendPlayerSetupBack",
        },
    );
    spawn_player_setup_control(parent, PlayerSetupControl::Back, 500.0, 325.0, 107.0, 26.0);
}

fn spawn_player_setup_scrollbar(
    parent: &mut ChildSpawnerCommands,
    txd: &FrontendTxdTextures,
    menu: &CMenuManager,
) {
    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(606.0, DEFAULT_SCREEN_WIDTH),
            top: pct(28.0, DEFAULT_SCREEN_HEIGHT),
            width: pct(16.0, DEFAULT_SCREEN_WIDTH),
            height: pct(295.0, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(Color::srgba(
            100.0 / 255.0,
            100.0 / 255.0,
            66.0 / 255.0,
            205.0 / 255.0,
        )),
        Name::new("FrontendPlayerSetupScrollbarBackground"),
    ));

    let total = menu.player_setup_skins.len().max(1);
    let thumb_height = if total <= PLAYERSETUP_MAX_VISIBLE_ROWS {
        258.0
    } else {
        (263.0 * PLAYERSETUP_MAX_VISIBLE_ROWS as f32 / total as f32 - 8.0).max(12.0)
    };
    let thumb_top = 47.0 + menu.player_setup_scrollbar_top_margin;
    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(608.0, DEFAULT_SCREEN_WIDTH),
            top: pct(thumb_top + 1.0, DEFAULT_SCREEN_HEIGHT),
            width: pct(11.0, DEFAULT_SCREEN_WIDTH),
            height: pct(thumb_height, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(Color::srgb(50.0 / 255.0, 50.0 / 255.0, 50.0 / 255.0)),
    ));
    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(608.0, DEFAULT_SCREEN_WIDTH),
            top: pct(thumb_top, DEFAULT_SCREEN_HEIGHT),
            width: pct(12.0, DEFAULT_SCREEN_WIDTH),
            height: pct(thumb_height, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(NORMAL),
        Name::new("FrontendPlayerSetupScrollbarThumb"),
    ));

    spawn_player_setup_image_control(
        parent,
        txd,
        if menu.player_setup_repeat_direction < 0 {
            "upon"
        } else {
            "upoff"
        },
        PlayerSetupControl::ScrollUp,
        606.0,
        28.0,
        16.0,
        17.0,
    );
    spawn_player_setup_image_control(
        parent,
        txd,
        if menu.player_setup_repeat_direction > 0 {
            "downon"
        } else {
            "downoff"
        },
        PlayerSetupControl::ScrollDown,
        606.0,
        306.0,
        16.0,
        17.0,
    );
    if thumb_top > 47.0 {
        spawn_player_setup_control(
            parent,
            PlayerSetupControl::PageUp,
            606.0,
            45.0,
            16.0,
            thumb_top - 45.0,
        );
    }
    let thumb_bottom = thumb_top + thumb_height;
    if thumb_bottom < 306.0 {
        spawn_player_setup_control(
            parent,
            PlayerSetupControl::PageDown,
            606.0,
            thumb_bottom,
            16.0,
            306.0 - thumb_bottom,
        );
    }
}

fn spawn_player_setup_image_control(
    parent: &mut ChildSpawnerCommands,
    txd: &FrontendTxdTextures,
    texture: &str,
    control: PlayerSetupControl,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) {
    if let Some(image) = txd.images.get(texture) {
        parent.spawn((
            Button,
            Hovered::default(),
            Node {
                position_type: PositionType::Absolute,
                left: pct(left, DEFAULT_SCREEN_WIDTH),
                top: pct(top, DEFAULT_SCREEN_HEIGHT),
                width: pct(width, DEFAULT_SCREEN_WIDTH),
                height: pct(height, DEFAULT_SCREEN_HEIGHT),
                ..default()
            },
            ImageNode {
                image: image.clone(),
                image_mode: NodeImageMode::Stretch,
                color: Color::WHITE,
                ..default()
            },
            FrontendPlayerSetupControl(control),
        ));
    } else {
        spawn_player_setup_control(parent, control, left, top, width, height);
    }
}

fn spawn_player_setup_control(
    parent: &mut ChildSpawnerCommands,
    control: PlayerSetupControl,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) {
    parent.spawn((
        Button,
        Hovered::default(),
        Node {
            position_type: PositionType::Absolute,
            left: pct(left, DEFAULT_SCREEN_WIDTH),
            top: pct(top, DEFAULT_SCREEN_HEIGHT),
            width: pct(width, DEFAULT_SCREEN_WIDTH),
            height: pct(height, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(Color::NONE),
        FrontendPlayerSetupControl(control),
    ));
}

fn spawn_label_text(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    index: usize,
    text_value: String,
) {
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: index,
            text: text_value,
            colour: NORMAL,
            style: FONT_BANK,
            scale_x: 0.45,
            scale_y: 0.7,
            left: 40.0,
            top: 60.0,
            align: TextAlign::Left,
            track_selection: false,
            shadow: false,
            value_column: false,
            name: "FrontendLabel",
        },
    );
}

fn spawn_entry_value(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    layout: MenuLayout,
    index: usize,
    y: f32,
    value: String,
) {
    if value.is_empty() {
        return;
    }
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: index,
            text: value,
            colour: NORMAL,
            style: FONT_HEADING,
            scale_x: layout.scale_x,
            scale_y: layout.scale_y,
            left: layout.column_width,
            top: y,
            align: TextAlign::Right,
            track_selection: true,
            shadow: true,
            value_column: true,
            name: "FrontendEntryValue",
        },
    );
}

fn spawn_slider(
    parent: &mut ChildSpawnerCommands,
    layout: MenuLayout,
    entry_index: usize,
    action: i32,
    entry_y: f32,
    progress: f32,
) {
    let smallest_bar_height = layout.line_height * 0.1;
    let largest_bar_height = layout.usable_line_height();
    let slider_left = DEFAULT_SCREEN_WIDTH - (MENUSLIDER_WIDTH + layout.column_width);
    let slider_top = entry_y - 2.0;

    for bar_index in 0..MENUSLIDER_BARS as usize {
        let bar_x = slider_left + bar_index as f32 * MENUSLIDER_WIDTH / MENUSLIDER_BARS as f32;
        let bar_height = ((MENUSLIDER_BARS as usize - bar_index) as f32 * smallest_bar_height
            + bar_index as f32 * largest_bar_height)
            / MENUSLIDER_BARS as f32;
        let bar_top = slider_top + largest_bar_height - bar_height;

        parent.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: pct(bar_x + 2.0, DEFAULT_SCREEN_WIDTH),
                top: pct(bar_top + 2.0, DEFAULT_SCREEN_HEIGHT),
                width: pct(10.0, DEFAULT_SCREEN_WIDTH),
                height: pct(bar_height, DEFAULT_SCREEN_HEIGHT),
                ..default()
            },
            BackgroundColor(SLIDER_SHADOW),
            Name::new(format!("FrontendSliderShadow{entry_index}_{bar_index}")),
        ));
        parent.spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: pct(bar_x, DEFAULT_SCREEN_WIDTH),
                top: pct(bar_top, DEFAULT_SCREEN_HEIGHT),
                width: pct(10.0, DEFAULT_SCREEN_WIDTH),
                height: pct(bar_height, DEFAULT_SCREEN_HEIGHT),
                ..default()
            },
            BackgroundColor(slider_bar_colour(progress, bar_index)),
            FrontendSliderBar {
                entry_index,
                bar_index,
            },
            Name::new(format!("FrontendSliderBar{entry_index}_{bar_index}")),
        ));
    }

    let logical_left = slider_hover_start(action);
    parent.spawn((
        Button,
        Hovered::default(),
        Node {
            position_type: PositionType::Absolute,
            left: pct(logical_left, DEFAULT_SCREEN_WIDTH),
            top: pct(entry_y, DEFAULT_SCREEN_HEIGHT),
            width: pct(DEFAULT_SCREEN_WIDTH - logical_left, DEFAULT_SCREEN_WIDTH),
            height: pct(28.0, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        BackgroundColor(Color::NONE),
        RelativeCursorPosition::default(),
        FrontendSliderHitArea {
            entry_index,
            logical_left,
        },
        Name::new(format!("FrontendSliderHitArea{entry_index}")),
    ));
}

fn slider_bar_colour(progress: f32, bar_index: usize) -> Color {
    if slider_bar_active(progress, bar_index) {
        SLIDER_ON
    } else {
        SLIDER_OFF
    }
}

fn slider_bar_active(progress: f32, bar_index: usize) -> bool {
    let midpoint = bar_index as f32 / MENUSLIDER_BARS as f32 + 1.0 / (MENUSLIDER_BARS as f32 * 2.0);
    midpoint < progress
}

fn slider_hover_start(action: i32) -> f32 {
    if action == eMenuAction::MENUACTION_MOUSESENS as i32 {
        200.0
    } else {
        170.0
    }
}

fn slider_mouse_direction(
    menu: &CMenuManager,
    action: i32,
    layout: MenuLayout,
    logical_mouse_x: f32,
) -> Option<i32> {
    let progress = slider_progress(menu, action)?;
    let slider_left = DEFAULT_SCREEN_WIDTH - (MENUSLIDER_WIDTH + layout.column_width);
    if logical_mouse_x < slider_left {
        return None;
    }

    let last_active_bar_x = (0..MENUSLIDER_BARS as usize)
        .filter(|bar_index| slider_bar_active(progress, *bar_index))
        .next_back()
        .map(|bar_index| slider_left + bar_index as f32 * MENUSLIDER_WIDTH / MENUSLIDER_BARS as f32)
        .unwrap_or(0.0);

    if logical_mouse_x < last_active_bar_x - 10.0 {
        Some(-1)
    } else if logical_mouse_x > last_active_bar_x + 10.0 {
        Some(1)
    } else {
        None
    }
}

fn spawn_entry_text(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    layout: MenuLayout,
    index: usize,
    y: f32,
    text_value: String,
    colour: Color,
) {
    let left = entry_text_left(layout, &text_value);
    let align = if layout.centre {
        TextAlign::Center
    } else {
        TextAlign::Left
    };
    spawn_bitmap_text(
        parent,
        txd,
        images,
        BitmapTextSpawn {
            entry_index: index,
            text: text_value,
            colour,
            style: FONT_HEADING,
            scale_x: layout.scale_x,
            scale_y: layout.scale_y,
            left,
            top: y,
            align,
            track_selection: true,
            shadow: true,
            value_column: false,
            name: "FrontendEntryText",
        },
    );
}

#[derive(Debug, Clone, Copy)]
enum TextAlign {
    Left,
    Center,
    Right,
}

struct BitmapTextSpawn {
    entry_index: usize,
    text: String,
    colour: Color,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    left: f32,
    top: f32,
    align: TextAlign,
    track_selection: bool,
    shadow: bool,
    value_column: bool,
    name: &'static str,
}

fn spawn_bitmap_text(
    parent: &mut ChildSpawnerCommands,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    spec: BitmapTextSpawn,
) {
    let Some(rendered) = bitmap_text_handle(
        txd,
        images,
        &spec.text,
        spec.style,
        spec.scale_x,
        spec.scale_y,
        spec.colour,
        spec.shadow,
    ) else {
        return;
    };

    let logical_bitmap_width = rendered.bitmap_width / txd.text_render_scale_x;
    let logical_bitmap_height = rendered.bitmap_height / txd.text_render_scale_y;
    let logical_advance_width = rendered.advance_width / txd.text_render_scale_x;
    let left = logical_text_left(spec.left, spec.align, logical_advance_width);

    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(left.max(0.0), DEFAULT_SCREEN_WIDTH),
            top: pct(spec.top, DEFAULT_SCREEN_HEIGHT),
            width: pct(logical_bitmap_width, DEFAULT_SCREEN_WIDTH),
            height: pct(logical_bitmap_height, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        ImageNode {
            image: rendered.handle,
            image_mode: NodeImageMode::Stretch,
            color: Color::WHITE,
            ..default()
        },
        FrontendBitmapText {
            entry_index: spec.entry_index,
            selected_only: spec.track_selection,
            text: spec.text,
            style: spec.style,
            scale_x: spec.scale_x,
            scale_y: spec.scale_y,
            shadow: spec.shadow,
            value_column: spec.value_column,
            logical_left: spec.left,
            logical_top: spec.top,
            align: spec.align,
            last_colour: None,
            last_render_scale: None,
        },
        Name::new(spec.name),
    ));
}

fn logical_text_left(left: f32, align: TextAlign, logical_width: f32) -> f32 {
    match align {
        TextAlign::Left => left,
        TextAlign::Center => left - logical_width * 0.5,
        TextAlign::Right => DEFAULT_SCREEN_WIDTH - left - logical_width,
    }
}

fn spawn_background(parent: &mut ChildSpawnerCommands, txd: &FrontendTxdTextures, name: &str) {
    let Some(image) = txd.images.get(name) else {
        return;
    };
    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        ImageNode {
            image: image.clone(),
            image_mode: NodeImageMode::Stretch,
            color: Color::WHITE,
            ..default()
        },
        Name::new("FrontendBackground"),
    ));
}

fn spawn_menu_logo(parent: &mut ChildSpawnerCommands, txd: &FrontendTxdTextures, screen: i32) {
    if screen == eMenuScreen::MENUPAGE_START_MENU as i32 {
        spawn_abs_image_px(parent, txd, "gtalogo128", 225.0, 40.0, 190.0, 170.0);
    }
}

fn spawn_abs_image_px(
    parent: &mut ChildSpawnerCommands,
    txd: &FrontendTxdTextures,
    name: &str,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) {
    let Some(image) = txd.images.get(name) else {
        return;
    };
    parent.spawn((
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: pct(left, DEFAULT_SCREEN_WIDTH),
            top: pct(top, DEFAULT_SCREEN_HEIGHT),
            width: pct(width, DEFAULT_SCREEN_WIDTH),
            height: pct(height, DEFAULT_SCREEN_HEIGHT),
            ..default()
        },
        ImageNode {
            image: image.clone(),
            image_mode: NodeImageMode::Stretch,
            color: Color::WHITE,
            ..default()
        },
    ));
}

fn despawn_frontend(commands: &mut Commands, ui: &mut FrontendMenuUi) {
    if let Some(root) = ui.root.take() {
        commands.entity(root).despawn();
    }
}

fn refresh_text(
    page: MenuPage,
    menu: &CMenuManager,
    controller: &CControllerConfigManager,
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    text_query: &mut Query<(&mut FrontendBitmapText, &mut ImageNode, &mut Node)>,
    bar_query: &mut Query<(&FrontendSelectionBar, &mut Visibility)>,
    slider_query: &mut Query<(&FrontendSliderBar, &mut BackgroundColor)>,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
) {
    for (mut marker, mut image_node, mut node) in text_query.iter_mut() {
        let (colour, value) = if marker.entry_index == usize::MAX {
            // The page title has no menu row, but it must be redrawn when the
            // selected GXT archive changes.
            (HEADER, gxt(text, page.title).to_uppercase())
        } else if let Some(entry) = page.entries.get(marker.entry_index).copied() {
            let selected = marker.entry_index == menu.curr_option as usize;
            let colour = if marker.selected_only {
                entry_colour(entry, selected)
            } else {
                NORMAL
            };
            let value = if marker.value_column {
                entry_value(menu, Some(&controller), text, entry)
            } else {
                entry_text(menu, text, memory_card, entry)
            };
            (colour, value)
        } else {
            continue;
        };
        // (text, colour, render scale) exactly determines the cached handle
        // and node rect below, so an unchanged triple skips the lookup and
        // all component writes. Anything new recomputes, so output can never
        // go stale — worst case is one redundant refresh.
        let render_scale = [
            txd.text_render_scale_x.to_bits(),
            txd.text_render_scale_y.to_bits(),
        ];
        if value == marker.text
            && marker.last_colour == Some(colour)
            && marker.last_render_scale == Some(render_scale)
        {
            continue;
        }
        marker.text = value;
        marker.last_colour = Some(colour);
        marker.last_render_scale = Some(render_scale);
        if let Some(rendered) = bitmap_text_handle(
            txd,
            images,
            &marker.text,
            marker.style,
            marker.scale_x,
            marker.scale_y,
            colour,
            marker.shadow,
        ) {
            let logical_bitmap_width = rendered.bitmap_width / txd.text_render_scale_x;
            let logical_bitmap_height = rendered.bitmap_height / txd.text_render_scale_y;
            let logical_advance_width = rendered.advance_width / txd.text_render_scale_x;
            node.left = pct(
                logical_text_left(marker.logical_left, marker.align, logical_advance_width)
                    .max(0.0),
                DEFAULT_SCREEN_WIDTH,
            );
            node.top = pct(marker.logical_top, DEFAULT_SCREEN_HEIGHT);
            node.width = pct(logical_bitmap_width, DEFAULT_SCREEN_WIDTH);
            node.height = pct(logical_bitmap_height, DEFAULT_SCREEN_HEIGHT);
            image_node.image = rendered.handle;
        }
    }
    for (marker, mut visibility) in bar_query.iter_mut() {
        let visible = if marker.entry_index == menu.curr_option as usize {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != visible {
            *visibility = visible;
        }
    }
    for (marker, mut background) in slider_query.iter_mut() {
        let Some(entry) = page.entries.get(marker.entry_index) else {
            continue;
        };
        let Some(progress) = slider_progress(menu, entry.action) else {
            continue;
        };
        let colour = slider_bar_colour(progress, marker.bar_index);
        if background.0 != colour {
            background.0 = colour;
        }
    }
}

fn entry_text(
    menu: &CMenuManager,
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    entry: MenuEntry,
) -> String {
    if menu.curr_screen == eMenuScreen::MENUPAGE_SAVE_OVERWRITE_CONFIRM as i32
        && entry.action == eMenuAction::MENUACTION_LABEL as i32
        && usize::try_from(menu.curr_save_slot)
            .ok()
            .and_then(|slot| memory_card.map(|card| card.GetInfoOnSpecificSlot(slot)))
            .is_some_and(|slot| slot == SLOTINFO::SLOT_NOTPRESENT)
    {
        return gxt(text, "FESZ_QZ").to_uppercase();
    }
    if let Some(error_text) = memory_card_error_text(text, memory_card, entry) {
        return error_text;
    }
    if let Some(slot_text) = save_slot_text(text, memory_card, entry) {
        return slot_text;
    }
    if entry.action == eMenuAction::MENUACTION_GETKEY as i32 {
        return gxt(text, controller_action_gxt(entry.key)).to_uppercase();
    }
    gxt(text, entry.key).to_uppercase()
}

fn controller_action_gxt(label: &str) -> &str {
    match label {
        "FIRE WEAPON" => "FEC_FIR",
        "NEXT WEAPON" => "FEC_NWE",
        "PREVIOUS WEAPON" => "FEC_PWE",
        "GO FORWARD" | "ACCELERATE" => "FEC_FOR",
        "GO BACK" | "BRAKE / REVERSE" => "FEC_BAC",
        "GO LEFT" => "FEC_LEF",
        "GO RIGHT" => "FEC_RIG",
        "SNIPER ZOOM IN" => "FEC_ZIN",
        "SNIPER ZOOM OUT" => "FEC_ZOT",
        "ENTER / EXIT VEHICLE" => "FEC_EEX",
        "CHANGE RADIO STATION" => "FEC_RAD",
        "HORN" => "FEC_HRN",
        "TOGGLE SUBMISSIONS" => "FEC_SUB",
        "CHANGE CAMERA" => "FEC_CMR",
        "JUMP" => "FEC_JMP",
        "SPRINT" => "FEC_SPN",
        "HANDBRAKE" => "FEC_HND",
        "LOCK TARGET" => "FEC_TAR",
        "TURRET LEFT" => "FEC_TFL",
        "TURRET RIGHT" => "FEC_TFR",
        "TURRET UP" => "FEC_TFU",
        "TURRET DOWN" => "FEC_TFD",
        "LOOK BEHIND" | "VEHICLE LOOK BEHIND" => "FEC_LBA",
        "VEHICLE LOOK LEFT" | "FIRST PERSON LOOK LEFT" => "FEC_LOL",
        "VEHICLE LOOK RIGHT" | "FIRST PERSON LOOK RIGHT" => "FEC_LOR",
        "FIRST PERSON LOOK UP" => "FEC_LUD",
        "FIRST PERSON LOOK DOWN" => "FEC_LDU",
        "NEXT TARGET" => "FEC_NTR",
        "PREVIOUS TARGET" => "FEC_PTT",
        "CENTER CAMERA" => "FEC_CEN",
        _ => label,
    }
}

fn memory_card_error_text(
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    entry: MenuEntry,
) -> Option<String> {
    if entry.action != eMenuAction::MENUACTION_LABEL as i32 {
        return None;
    }
    let card = memory_card?;
    let key = card.GetErrorMessage();
    if key.is_empty() {
        return None;
    }
    Some(gxt(text, key).to_uppercase())
}

fn save_slot_text(
    text: &Option<Res<CText>>,
    memory_card: Option<&CMemoryCard>,
    entry: MenuEntry,
) -> Option<String> {
    let slot = usize::try_from(entry.save_slot).ok()?;
    if slot >= MAX_SLOTS {
        return None;
    }

    let Some(card) = memory_card else {
        return Some(gxt(text, entry.key).to_uppercase());
    };

    match card.GetInfoOnSpecificSlot(slot) {
        SLOTINFO::SLOT_PRESENT => {
            let name = card
                .GetNameOfSavedGame(slot)
                .filter(|name| !name.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| gxt(text, entry.key));
            let date = card
                .GetDateAndTimeOfSavedGame(slot)
                .filter(|date| !date.trim().is_empty())
                .unwrap_or("");
            if date.is_empty() {
                Some(name.to_uppercase())
            } else {
                Some(format!("{}   {}", name.to_uppercase(), date.to_uppercase()))
            }
        }
        SLOTINFO::SLOT_CORRUPTED => Some(gxt(text, "FESZ_CR").to_uppercase()),
        SLOTINFO::SLOT_NOTPRESENT => Some(gxt(text, entry.key).to_uppercase()),
    }
}

fn entry_value(
    menu: &CMenuManager,
    controller: Option<&CControllerConfigManager>,
    text: &Option<Res<CText>>,
    entry: MenuEntry,
) -> String {
    let on_off = |value: bool| gxt(text, if value { "FEM_ON" } else { "FEM_OFF" });
    match entry.action {
        action if action == eMenuAction::MENUACTION_CTRLMETHOD as i32 => {
            match menu.control_method {
                0 => gxt(text, "FET_SCN"),
                _ => gxt(text, "FET_CCN"),
            }
        }
        action if action == eMenuAction::MENUACTION_GETKEY as i32 => controller
            .and_then(|controller| {
                controller_action_for_label(entry.key).map(|action| controller.binding_text(action))
            })
            .unwrap_or_else(|| default_binding_for_label(entry.key).to_owned()),
        action if action == eMenuAction::MENUACTION_INVVERT as i32 => on_off(menu.mouse_invert),
        action if action == eMenuAction::MENUACTION_MOUSESTEER as i32 => {
            on_off(!menu.disable_mouse_steering)
        }
        action if action == eMenuAction::MENUACTION_RADIO as i32 => {
            gxt(text, &format!("FEA_FM{}", menu.prefs_radio_station.max(0)))
        }
        action if action == eMenuAction::MENUACTION_FRAMESYNC as i32 => {
            on_off(menu.prefs_vsync_disp != 0)
        }
        action if action == eMenuAction::MENUACTION_FRAMELIMIT as i32 => {
            max_fps_label(menu.prefs_max_fps)
        }
        action if action == eMenuAction::MENUACTION_TRAILS as i32 => on_off(menu.prefs_trails),
        action if action == eMenuAction::MENUACTION_SUBTITLES as i32 => {
            on_off(menu.prefs_show_subtitles != 0)
        }
        action if action == eMenuAction::MENUACTION_WIDESCREEN as i32 => {
            on_off(menu.prefs_use_wide_screen != 0)
        }
        action if action == eMenuAction::MENUACTION_FREE_CAM as i32 => on_off(menu.prefs_free_cam),
        action if action == eMenuAction::MENUACTION_SCREENRES as i32 => {
            let [width, height] = menu
                .video_modes
                .get(menu.display_video_mode)
                .copied()
                .unwrap_or([menu.window_width, menu.window_height]);
            format!("{width} X {height}")
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SPLIT as i32 => {
            if menu.prefs_local_coop_split == 0 {
                "TOP - BOTTOM".to_owned()
            } else {
                "LEFT - RIGHT".to_owned()
            }
        }
        action if action == eMenuAction::MENUACTION_LOCAL_COOP_SIDE as i32 => {
            if menu.prefs_local_coop_swap_sides == 0 {
                "P1: SOL - P2: SAG".to_owned()
            } else {
                "P1: SAG - P2: SOL".to_owned()
            }
        }
        action if action == eMenuAction::MENUACTION_LANG_ENG as i32 => language_selected(menu, 0),
        action if action == eMenuAction::MENUACTION_LANG_FRE as i32 => language_selected(menu, 1),
        action if action == eMenuAction::MENUACTION_LANG_GER as i32 => language_selected(menu, 2),
        action if action == eMenuAction::MENUACTION_LANG_ITA as i32 => language_selected(menu, 3),
        action if action == eMenuAction::MENUACTION_LANG_SPA as i32 => language_selected(menu, 4),
        _ => String::new(),
    }
}

fn slider_progress(menu: &CMenuManager, action: i32) -> Option<f32> {
    match action {
        action if action == eMenuAction::MENUACTION_MOUSESENS as i32 => {
            Some(menu.mouse_sensitivity as f32 / 200.0)
        }
        action if action == eMenuAction::MENUACTION_MUSICVOLUME as i32 => {
            Some(menu.prefs_music_volume as f32 / 128.0)
        }
        action if action == eMenuAction::MENUACTION_SFXVOLUME as i32 => {
            Some(menu.prefs_sfx_volume as f32 / 128.0)
        }
        action if action == eMenuAction::MENUACTION_BRIGHTNESS as i32 => {
            Some(menu.prefs_brightness as f32 / 512.0)
        }
        action if action == eMenuAction::MENUACTION_DRAWDIST as i32 => Some(menu.prefs_lod - 0.8),
        _ => None,
    }
}

fn language_selected(menu: &CMenuManager, language: i32) -> String {
    if menu.prefs_language == language {
        "<".to_owned()
    } else {
        String::new()
    }
}

fn default_binding_for_label(label: &str) -> &'static str {
    match label {
        "FIRE WEAPON" => "INS OR CTRL",
        "NEXT WEAPON" => "NUM ENTER",
        "PREVIOUS WEAPON" => "DEL",
        "GO FORWARD" | "ACCELERATE" => "UP OR W",
        "GO BACK" | "BRAKE / REVERSE" => "DOWN OR S",
        "GO LEFT" => "LEFT OR A",
        "GO RIGHT" => "RIGHT OR D",
        "SNIPER ZOOM IN" => "PGUP OR Z",
        "SNIPER ZOOM OUT" => "PGDN OR X",
        "ENTER / EXIT VEHICLE" => "ENTER OR F",
        "CHANGE CAMERA" => "HOME OR C",
        "JUMP" => "RCTRL OR SPACE",
        "SPRINT" => "RSHIFT OR LSHIFT",
        "LOOK BEHIND" => "END OR CAPSLOCK",
        "LOCK TARGET" => "DEL",
        "CHANGE RADIO STATION" => "INS OR R",
        "HORN" => "RSHIFT OR LSHIFT",
        "TOGGLE SUBMISSIONS" => "+ OR CAPSLOCK",
        "HANDBRAKE" => "RCTRL OR SPACE",
        "VEHICLE LOOK LEFT" => "END OR Q",
        "VEHICLE LOOK RIGHT" => "PGDN OR E",
        "PREVIOUS TARGET" => "[",
        "NEXT TARGET" => "]",
        _ => "",
    }
}

fn entry_colour(entry: MenuEntry, selected: bool) -> Color {
    if !is_selectable(entry) {
        DISABLED
    } else if selected {
        SELECTED
    } else {
        NORMAL
    }
}

fn is_selectable(entry: MenuEntry) -> bool {
    entry.action != eMenuAction::MENUACTION_NOTHING as i32
        && entry.action != eMenuAction::MENUACTION_LABEL as i32
}

fn menu_background_sprite(screen: i32) -> &'static str {
    match screen {
        2 | 8 | 9 | 10 | 11 | 12 | 42 => "singleplayer24",
        41 | 54 => "playersetup24",
        4 | 5 | 6 | 7 => "mainmenu24",
        _ => "mainmenu24",
    }
}

#[derive(Debug, Clone, Copy)]
struct MenuLayout {
    column_width: f32,
    header_height: f32,
    line_height: f32,
    centre: bool,
    scale_x: f32,
    scale_y: f32,
}

impl MenuLayout {
    fn entry_y(self, index: usize) -> f32 {
        self.header_height + 74.0 + self.line_height * index as f32
    }

    fn usable_line_height(self) -> f32 {
        self.line_height * 0.9
    }
}

fn menu_layout(screen: i32) -> MenuLayout {
    match screen {
        1 | 3 => MenuLayout {
            column_width: 320.0,
            header_height: 260.0,
            line_height: 24.0,
            centre: true,
            scale_x: 0.75,
            scale_y: 0.9,
        },
        35 | 56 => MenuLayout {
            column_width: 40.0,
            header_height: 0.0,
            line_height: 20.0,
            centre: false,
            scale_x: 0.55,
            scale_y: 0.8,
        },
        55 => MenuLayout {
            // DrawControllerSetupScreen uses 10.7 logical pixels for the
            // standard PC layout, so all 25 command rows fit on one page.
            column_width: 18.0,
            header_height: 0.0,
            line_height: 7.8,
            centre: false,
            scale_x: 0.34,
            scale_y: 0.5,
        },
        4 | 5 | 6 => MenuLayout {
            column_width: 50.0,
            header_height: 0.0,
            line_height: 20.0,
            centre: false,
            scale_x: 0.55,
            scale_y: 0.8,
        },
        60 => MenuLayout {
            column_width: 50.0,
            header_height: 40.0,
            line_height: 24.0,
            centre: false,
            scale_x: 0.55,
            scale_y: 0.8,
        },
        10 | 42 => MenuLayout {
            column_width: 320.0,
            header_height: 60.0,
            line_height: 24.0,
            centre: true,
            scale_x: 0.75,
            scale_y: 0.9,
        },
        51 => MenuLayout {
            column_width: 320.0,
            header_height: 140.0,
            line_height: 24.0,
            centre: true,
            scale_x: 0.75,
            scale_y: 0.9,
        },
        _ => MenuLayout {
            column_width: 320.0,
            header_height: 40.0,
            line_height: 24.0,
            centre: true,
            scale_x: 0.75,
            scale_y: 0.9,
        },
    }
}

fn entry_text_left(layout: MenuLayout, _text: &str) -> f32 {
    if !layout.centre {
        return layout.column_width;
    }
    DEFAULT_SCREEN_WIDTH * 0.5
}

fn pct(value: f32, base: f32) -> Val {
    Val::Percent(value / base * 100.0)
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedBitmapText {
    pub handle: Handle<Image>,
    pub bitmap_width: f32,
    pub bitmap_height: f32,
    pub advance_width: f32,
}

pub(crate) fn bitmap_text_advance_width(
    txd: &FrontendTxdTextures,
    text: &str,
    style: i16,
    scale_x: f32,
) -> f32 {
    let render_scale_x = txd.text_render_scale_x.max(f32::EPSILON);
    bitmap_text_metrics(
        text,
        style,
        scale_x * render_scale_x,
        txd.text_render_scale_y,
        Vec2::ZERO,
    )
    .advance_width
        / render_scale_x
}

pub(crate) fn bitmap_text_handle(
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    text: &str,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    colour: Color,
    shadow: bool,
) -> Option<RenderedBitmapText> {
    let [r, g, b, a] = colour.to_srgba().to_u8_array();
    let scale_x = scale_x * txd.text_render_scale_x;
    let scale_y = scale_y * txd.text_render_scale_y;
    // Frontend.cpp offsets textLayer 1 by MENU_X/Y(1.0), so the separation
    // grows with the physical render target just like the glyph quads do.
    let layer_offset = if shadow {
        Vec2::new(
            MENU_TEXT_SHADOW_OFFSET * txd.text_render_scale_x,
            MENU_TEXT_SHADOW_OFFSET * txd.text_render_scale_y,
        )
    } else {
        Vec2::ZERO
    };
    let metrics = bitmap_text_metrics(text, style, scale_x, scale_y, layer_offset);
    let cache_key = format!(
        "{style}:{scale_x:.3}:{scale_y:.3}:{:.3}:{:.3}:{r}:{g}:{b}:{a}:{shadow}:{text}",
        layer_offset.x, layer_offset.y
    );
    if let Some(handle) = txd.text_images.get(&cache_key) {
        return Some(RenderedBitmapText {
            handle: handle.clone(),
            bitmap_width: metrics.bitmap_width,
            bitmap_height: metrics.bitmap_height,
            advance_width: metrics.advance_width,
        });
    }

    let atlas_name = if style == FONT_BANK { "font2" } else { "font1" };
    let atlas = txd.bitmaps.get(atlas_name)?;
    let rgba = render_bitmap_text_rgba(
        text,
        style,
        scale_x,
        scale_y,
        [r, g, b, a],
        shadow,
        layer_offset,
        metrics,
        atlas,
    );
    let mut image = Image::new(
        Extent3d {
            width: metrics.bitmap_width as u32,
            height: metrics.bitmap_height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // The atlas has already been sampled with rwFILTERLINEAR above.  The
    // resulting screen-space bitmap must not be blurred by a second UI sample.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::default()
    });
    let handle = images.add(image);
    txd.text_images.insert(cache_key, handle.clone());
    Some(RenderedBitmapText {
        handle,
        bitmap_width: metrics.bitmap_width,
        bitmap_height: metrics.bitmap_height,
        advance_width: metrics.advance_width,
    })
}

#[derive(Debug, Clone, Copy)]
struct BitmapTextMetrics {
    advance_width: f32,
    bitmap_width: f32,
    bitmap_height: f32,
}

fn bitmap_text_metrics(
    text: &str,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    layer_offset: Vec2,
) -> BitmapTextMetrics {
    let style = style.clamp(0, 2) as usize;
    let mut line_advance = 0.0_f32;
    let mut max_advance = 0.0_f32;
    let mut glyph_right = 0.0_f32;
    let mut line_glyph_right = 0.0_f32;
    let mut line_count = 1_u32;
    for ch in text.chars() {
        if ch == '\n' {
            max_advance = max_advance.max(line_advance);
            glyph_right = glyph_right.max(line_glyph_right);
            line_advance = 0.0;
            line_glyph_right = 0.0;
            line_count += 1;
            continue;
        }
        if ch == '\r' {
            continue;
        }
        let glyph = glyph_index(ch);
        if glyph != 0 {
            line_glyph_right = line_glyph_right.max(line_advance + 32.0 * scale_x);
        }
        line_advance += FONT_SIZE[style][glyph.min(FONT_SIZE[style].len() - 1)] as f32 * scale_x;
    }
    max_advance = max_advance.max(line_advance);
    glyph_right = glyph_right.max(line_glyph_right);
    let line_height = 40.0 * scale_y * 0.5;
    BitmapTextMetrics {
        advance_width: max_advance,
        // CFont aligns with GetStringWidth, but PrintChar always emits a
        // 32*scaleX quad.  Preserve that final-glyph overhang without moving
        // centred/right-justified text away from the C++ anchor.
        bitmap_width: (max_advance.max(glyph_right) + layer_offset.x)
            .max(1.0)
            .ceil(),
        bitmap_height: (line_height * line_count as f32 + layer_offset.y)
            .max(1.0)
            .ceil(),
    }
}

fn render_bitmap_text_rgba(
    text: &str,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    colour: [u8; 4],
    shadow: bool,
    layer_offset: Vec2,
    metrics: BitmapTextMetrics,
    atlas: &RgbaTexture,
) -> Vec<u8> {
    let out_w = metrics.bitmap_width as u32;
    let out_h = metrics.bitmap_height as u32;
    let mut out = vec![0_u8; out_w as usize * out_h as usize * 4];

    // Frontend.cpp's textLayer loop prints the complete black string first,
    // then the complete coloured string.  Character-by-character shadow/
    // foreground pairs change blending wherever 32-pixel glyph quads overlap.
    if shadow {
        rasterize_text_layer(
            &mut out,
            out_w,
            out_h,
            text,
            style,
            scale_x,
            scale_y,
            Vec2::ZERO,
            MENU_TEXT_SHADOW,
            atlas,
        );
    }
    rasterize_text_layer(
        &mut out,
        out_w,
        out_h,
        text,
        style,
        scale_x,
        scale_y,
        layer_offset,
        colour,
        atlas,
    );
    compensate_rw_gamma_space_alpha(&mut out);
    out
}

/// RenderWare's D3D8-era SRCALPHA/INVSRCALPHA path blends the encoded colour
/// values written to the framebuffer. Bevy UI blends on an sRGB target in
/// linear light, so feeding the same numeric alpha makes the 90-alpha menu
/// shadow and the atlas edge coverage visibly weaker. Convert the finished
/// CFont coverage into the equivalent linear-light opacity before ImageNode
/// performs the final composite. The 2.4 exponent is the sRGB transfer curve
/// used by Bevy's target; fully transparent/opaque texels remain unchanged.
fn compensate_rw_gamma_space_alpha(rgba: &mut [u8]) {
    // Input coverage has exactly 256 values. Preserve the same rounding while
    // avoiding a transcendental calculation for every screen-space texel.
    static OPACITY: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();
    let opacity = OPACITY.get_or_init(|| std::array::from_fn(|coverage| {
        let alpha = coverage as f32 / 255.0;
        ((1.0 - (1.0 - alpha).powf(2.4)) * 255.0).round() as u8
    }));
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = opacity[usize::from(pixel[3])];
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_text_layer(
    out: &mut [u8],
    out_w: u32,
    out_h: u32,
    text: &str,
    style: i16,
    scale_x: f32,
    scale_y: f32,
    layer_offset: Vec2,
    colour: [u8; 4],
    atlas: &RgbaTexture,
) {
    let style = style.clamp(0, 2) as usize;
    let mut cursor_x = 0.0_f32;
    let mut cursor_y = 0.0_f32;
    let line_height = 40.0 * scale_y * 0.5;
    for ch in text.chars() {
        if ch == '\n' {
            cursor_x = 0.0;
            cursor_y += line_height;
            continue;
        }
        if ch == '\r' {
            continue;
        }
        let glyph = glyph_index(ch);
        rasterize_glyph(
            out,
            out_w,
            out_h,
            cursor_x + layer_offset.x,
            cursor_y + layer_offset.y,
            glyph,
            scale_x,
            scale_y,
            colour,
            atlas,
        );
        cursor_x += FONT_SIZE[style][glyph.min(FONT_SIZE[style].len() - 1)] as f32 * scale_x;
    }
}

fn glyph_index(ch: char) -> usize {
    let value = ch as u32;
    value.saturating_sub(' ' as u32).min(192) as usize
}

#[allow(clippy::too_many_arguments)]
fn rasterize_glyph(
    out: &mut [u8],
    out_w: u32,
    out_h: u32,
    origin_x: f32,
    origin_y: f32,
    glyph: usize,
    scale_x: f32,
    scale_y: f32,
    colour: [u8; 4],
    atlas: &RgbaTexture,
) {
    if glyph == 0 {
        return;
    }
    let quad_w = (32.0 * scale_x).max(f32::EPSILON);
    let quad_h = (40.0 * scale_y * 0.5).max(f32::EPSILON);
    let end_x = origin_x + quad_w;
    let end_y = origin_y + quad_h;
    let start_x = origin_x.floor().max(0.0) as u32;
    let start_y = origin_y.floor().max(0.0) as u32;
    let end_x_px = end_x.ceil().min(out_w as f32) as u32;
    let end_y_px = end_y.ceil().min(out_h as f32) as u32;

    let xoff = (glyph % 16) as f32;
    let yoff = (glyph / 16) as f32;
    let u0 = xoff / 16.0;
    let v0 = yoff / 12.8;
    let u1 = (xoff + 1.0) / 16.0 - 0.001;
    let v1 = (yoff + 1.0) / 12.8 - 0.002;

    for out_y in start_y..end_y_px {
        let pixel_y = out_y as f32 + 0.5;
        if pixel_y < origin_y || pixel_y >= end_y {
            continue;
        }
        let v = v0 + (v1 - v0) * ((pixel_y - origin_y) / quad_h);
        for out_x in start_x..end_x_px {
            let pixel_x = out_x as f32 + 0.5;
            if pixel_x < origin_x || pixel_x >= end_x {
                continue;
            }
            let u = u0 + (u1 - u0) * ((pixel_x - origin_x) / quad_w);
            let texel = texture_rgba_linear(atlas, u, v);
            if texel[3] == 0 {
                continue;
            }
            let modulated = [
                ((u16::from(colour[0]) * u16::from(texel[0])) / 255) as u8,
                ((u16::from(colour[1]) * u16::from(texel[1])) / 255) as u8,
                ((u16::from(colour[2]) * u16::from(texel[2])) / 255) as u8,
                colour[3],
            ];
            let dst = (out_y as usize * out_w as usize + out_x as usize) * 4;
            alpha_blend_pixel(out, dst, modulated, texel[3]);
        }
    }
}

fn alpha_blend_pixel(out: &mut [u8], dst: usize, colour: [u8; 4], glyph_alpha: u8) {
    let src_a = (u16::from(glyph_alpha) * u16::from(colour[3])) / 255;
    if src_a == 0 {
        return;
    }

    // ImageNode expects straight-alpha RGBA. The old rasterizer generated a
    // premultiplied RGB value and ImageNode multiplied it a second time, which
    // made every antialiased glyph look translucent. Sprite2d composites the
    // font mask once, so compose this temporary bitmap in straight alpha too.
    // RGB premultiplication can reach 255*255*255, beyond u16.
    let dst_a = u32::from(out[dst + 3]);
    let src_a = u32::from(src_a);
    let inv_src_a = 255 - src_a;
    let out_a = src_a + (dst_a * inv_src_a) / 255;
    for (channel, source) in colour[..3].iter().enumerate() {
        let dst_premultiplied = u32::from(out[dst + channel]) * dst_a;
        let src_premultiplied = u32::from(*source) * src_a;
        out[dst + channel] = if out_a == 0 {
            0
        } else {
            ((src_premultiplied + (dst_premultiplied * inv_src_a) / 255) / out_a) as u8
        };
    }
    out[dst + 3] = out_a as u8;
}

fn texture_rgba_linear(source: &RgbaTexture, u: f32, v: f32) -> [u8; 4] {
    // D3D/RW normalized UVs address texel centres at (n + 0.5) / size.
    let x = u * source.width as f32 - 0.5;
    let y = v * source.height as f32 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let fraction_x = x - x0;
    let fraction_y = y - y0;
    let sample = |sample_x: f32, sample_y: f32, channel: usize| {
        let sample_x = sample_x.clamp(0.0, source.width.saturating_sub(1) as f32) as usize;
        let sample_y = sample_y.clamp(0.0, source.height.saturating_sub(1) as f32) as usize;
        source.rgba[(sample_y * source.width as usize + sample_x) * 4 + channel] as f32
    };
    let mut rgba = [0_u8; 4];
    for (channel, value) in rgba.iter_mut().enumerate() {
        let top = sample(x0, y0, channel) * (1.0 - fraction_x)
            + sample(x0 + 1.0, y0, channel) * fraction_x;
        let bottom = sample(x0, y0 + 1.0, channel) * (1.0 - fraction_x)
            + sample(x0 + 1.0, y0 + 1.0, channel) * fraction_x;
        *value = (top * (1.0 - fraction_y) + bottom * fraction_y).round() as u8;
    }
    rgba
}

fn gxt(text: &Option<Res<CText>>, key: &str) -> String {
    if let Some(text) = text {
        let value = unicode_to_ascii(&text.get(key));
        if !value.is_empty() {
            return value;
        }
    }
    key.to_owned()
}

fn ensure_frontend_textures(
    cache: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    install: Option<&OriginalGtaInstall>,
) {
    if cache.loaded {
        return;
    }
    let Some(install) = install else {
        error!("Frontend TXD load skipped: original install missing");
        cache.loaded = true;
        return;
    };
    for txd_name in ["menu.txd", "frontend.txd", "fonts.txd"] {
        let path = install.root().join("models").join(txd_name);
        let Ok(bytes) = fs::read(&path) else {
            error!("Frontend TXD load failed: {}", path.display());
            continue;
        };
        let Some(summary) = read_txd(&bytes) else {
            error!("Frontend TXD parse failed: {}", path.display());
            continue;
        };
        for texture in summary.textures {
            if let Some(decoded) = decode_txd_texture(&texture) {
                let key = decoded.name.to_ascii_lowercase();
                let bitmap = RgbaTexture {
                    width: decoded.width,
                    height: decoded.height,
                    rgba: decoded.rgba,
                };
                cache
                    .images
                    .insert(key.clone(), images.add(bitmap.to_image()));
                cache.bitmaps.insert(key, bitmap);
            }
        }
    }
    cache.loaded = true;
}

/// Load the original in-game HUD dictionary and the 8×8 radar tile dictionaries.
/// This is deliberately separate from the frontend load: re3 loads HUD.TXD during
/// `CHud::Initialise`, while radar TXDs remain normal streamed GTA assets.
pub(crate) fn ensure_hud_textures(
    cache: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    install: Option<&OriginalGtaInstall>,
) {
    if cache.hud_loaded {
        return;
    }
    let Some(install) = install else {
        return;
    };
    // HUD text uses the same font1/font2 RenderWare atlases as the frontend.
    // Loading fonts here is required for direct New Game boot as well: the
    // normal frontend happens to preload fonts.txd, but gameplay must not
    // depend on having visited a menu first. radar00..radar63 remain streamed
    // TXD entries inside GTA3.IMG and are synchronized from CTxdStore.
    for txd_name in ["hud.txd", "fonts.txd"] {
        let path = install.root().join("models").join(&txd_name);
        let Ok(bytes) = fs::read(&path) else {
            warn!("HUD TXD load failed: {}", path.display());
            continue;
        };
        let Some(summary) = read_txd(&bytes) else {
            warn!("HUD TXD parse failed: {}", path.display());
            continue;
        };
        for texture in summary.textures {
            if let Some(decoded) = decode_txd_texture(&texture) {
                let key = decoded.name.to_ascii_lowercase();
                let bitmap = RgbaTexture {
                    width: decoded.width,
                    height: decoded.height,
                    rgba: decoded.rgba,
                };
                cache
                    .images
                    .insert(key.clone(), images.add(bitmap.to_image()));
                cache.bitmaps.insert(key, bitmap);
            }
        }
    }
    cache.hud_loaded = true;
}

impl RgbaTexture {
    fn to_image(&self) -> Image {
        Image::new(
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            self.rgba.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }
}
