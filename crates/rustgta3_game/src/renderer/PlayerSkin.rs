//! re3 reference: references/re-GTA/src/renderer/PlayerSkin.cpp
//! re3 reference: references/re-GTA/src/renderer/PlayerSkin.h
//!
//! Player skin TXD/front-end edit state.

use bevy::prelude::{Entity, Handle, Image, Resource};
use rustgta3_core::core::Coordinates::CVector;
use rustgta3_formats::formats::ClumpRead::RwClumpSummary;

pub const DEFAULT_SKIN_NAME: &str = "$$\"\"";

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPlayerSkin {
    pub txd_slot: Option<i32>,
    pub player_clump_loaded: bool,
    pub old_fov: f32,
    pub frontend_rotation: f32,
    pub last_flash: u32,
    pub skin_texture_requests: Vec<SkinTextureRequest>,
    pub preview_target: Option<Handle<Image>>,
    pub preview_camera: Option<Entity>,
    pub preview_root: Option<Entity>,
    pub preview_skin_name: String,
    pub frontend_clump: Option<RwClumpSummary>,
    pub frontend_clump_load_attempted: bool,
    pub gameplay_skin_name: String,
    pub gameplay_texture: Option<Handle<Image>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinTextureRequest {
    pub texture_name: String,
    pub fallback_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontendSkinEditRender {
    pub position: CVector,
    pub axis1_rotation_deg: f32,
    pub axis2_rotation_deg: f32,
    pub ambient_colour: [f32; 4],
}

impl CPlayerSkin {
    pub fn initialise(&mut self, txd_slot: i32) {
        self.txd_slot = Some(txd_slot);
    }

    pub fn shutdown(&mut self) {
        self.txd_slot = None;
        self.player_clump_loaded = false;
        self.preview_target = None;
        self.preview_camera = None;
        self.preview_root = None;
        self.preview_skin_name.clear();
        self.frontend_clump = None;
        self.frontend_clump_load_attempted = false;
        self.gameplay_skin_name.clear();
        self.gameplay_texture = None;
    }

    pub fn get_skin_texture(&mut self, tex_name: &str) -> SkinTextureRequest {
        let fallback_path = if tex_name == DEFAULT_SKIN_NAME {
            "models\\generic\\player.bmp".to_owned()
        } else {
            format!("skins\\{tex_name}.bmp")
        };
        let request = SkinTextureRequest {
            texture_name: tex_name.to_owned(),
            fallback_path,
        };
        self.skin_texture_requests.push(request.clone());
        request
    }

    pub fn begin_frontend_skin_edit(&mut self, current_fov: f32) -> f32 {
        self.player_clump_loaded = true;
        self.old_fov = current_fov;
        30.0
    }

    pub fn end_frontend_skin_edit(&mut self) -> f32 {
        self.player_clump_loaded = false;
        self.old_fov
    }

    pub fn render_frontend_skin_edit(&mut self, time_ms_pause_mode: u32) -> FrontendSkinEditRender {
        if time_ms_pause_mode.wrapping_sub(self.last_flash) > 7 {
            self.frontend_rotation += 2.0;
            if self.frontend_rotation > 360.0 {
                self.frontend_rotation -= 360.0;
            }
            self.last_flash = time_ms_pause_mode;
        }

        FrontendSkinEditRender {
            position: CVector::new(1.35, 0.35, 7.725),
            axis1_rotation_deg: -90.0,
            axis2_rotation_deg: self.frontend_rotation,
            ambient_colour: [0.65, 0.65, 0.65, 1.0],
        }
    }
}

impl Default for CPlayerSkin {
    fn default() -> Self {
        Self {
            txd_slot: None,
            player_clump_loaded: false,
            old_fov: 0.0,
            frontend_rotation: 0.0,
            last_flash: 0,
            skin_texture_requests: Vec::new(),
            preview_target: None,
            preview_camera: None,
            preview_root: None,
            preview_skin_name: String::new(),
            frontend_clump: None,
            frontend_clump_load_attempted: false,
            gameplay_skin_name: String::new(),
            gameplay_texture: None,
        }
    }
}
