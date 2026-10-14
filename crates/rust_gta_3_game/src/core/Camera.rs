//! re3 reference: references/re-GTA/src/core/Camera.cpp
//! re3 reference: references/re-GTA/src/core/Camera.h
//!
//! TheCamera resource.

use bevy::prelude::{
    App, IntoScheduleConfigs, PerspectiveProjection, Plugin, Projection, Query,
    Res, ResMut, Resource, Transform, Update, Vec3, With, in_state,
};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};
use bevy::ecs::system::SystemParam;

use crate::{
    animation::CutsceneMgr::CCutsceneMgr,
    control::{
        Garages::CGarages,
        Script::{ScriptGameplayBridge, ScriptHandle},
    },
    core::{
        Cam::{CCam, LOOKING_FORWARD, eCamMode},
        FileLoader::CFileLoader,
        Game::ScriptEntityWorld,
    },
    peds::PlayerPed::{CWorldPlayers, WorldPlayerAccess},
};

pub const MAX_NUM_OF_SPLINETYPES: usize = 4;
pub const MAX_CAMERA_SPLINE_PATH_LENGTH: usize = 800;
pub const CAM_ZOOM_1STPRS: i32 = 0;
pub const CAM_ZOOM_1: i32 = 1;
pub const CAM_ZOOM_2: i32 = 2;
pub const CAM_ZOOM_3: i32 = 3;
pub const CAM_ZOOM_TOPDOWN: i32 = 4;
pub const CAM_ZOOM_CINEMATIC: i32 = 5;
pub const DEFAULT_NEAR: f32 = 0.9;

/// Owns the camera stages of `CGame::Process`; gameplay domains provide the
/// processed pad/player state and the existing shared sets preserve cadence.
pub struct GtaCameraPlugin;

impl Plugin for GtaCameraPlugin {
    fn build(&self, app: &mut App) {
        use rust_gta_3_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet};
        app.add_systems(
            Update,
            reset_camera_motion_blur_for_frame
                .after(crate::core::Pad::update_pads_from_bevy_input)
                .in_set(GtaProcessSet::Pad)
                .run_if(in_state(GtaAppState::InGame)),
        ).add_systems(
            Update,
            follow_script_player_camera
                .after(crate::peds::PlayerPed::update_player_wanted)
                .in_set(GtaProcessSet::Camera)
                .run_if(in_state(GtaAppState::InGame)),
        ).add_systems(
            Update,
            track_camera_speed
                .after(follow_script_player_camera)
                .in_set(GtaProcessSet::Camera)
                .run_if(in_state(GtaAppState::InGame)),
        );
    }
}
pub const MOTION_BLUR_NONE: i32 = 0;
pub const MOTION_BLUR_SNIPER: i32 = 1;
pub const MOTION_BLUR_LIGHT_SCENE: i32 = 2;
pub const MOTION_BLUR_SECURITY_CAM: i32 = 3;
pub const MOTION_BLUR_CUT_SCENE: i32 = 4;
pub const MOTION_BLUR_INTRO: i32 = 5;
pub const MOTION_BLUR_INTRO2: i32 = 6;
pub const MOTION_BLUR_SNIPER_ZOOM: i32 = 7;
pub const MOTION_BLUR_INTRO3: i32 = 8;
pub const MOTION_BLUR_INTRO4: i32 = 9;

pub(crate) fn apply_garage_camera_override(
    camera: &mut CCamera,
    garages: &CGarages,
    target: CVector,
    file_loader: &CFileLoader,
) -> bool {
    if camera.script_mode != CameraScriptMode::FollowPlayer {
        return false;
    }
    camera.just_came_out_of_garage = false;
    let Some(garage_index) = camera.garage_we_are_in else {
        if camera.player_is_in_garage {
            camera.just_came_out_of_garage = true;
        }
        camera.player_is_in_garage = false;
        camera.garage_fixed_cam_position_set = false;
        return false;
    };
    if !garages.cam_should_be_outside || !garages.is_point_in_camera_zone(target) {
        if camera.player_is_in_garage {
            camera.just_came_out_of_garage = true;
        }
        camera.player_is_in_garage = false;
        camera.garage_fixed_cam_position_set = false;
        return false;
    }
    let Some(garage) = garages.garages.get(garage_index) else {
        return false;
    };
    let door = if garage.door1_world_index.is_some() {
        CVector::new(garage.door1_x, garage.door1_y, 0.0)
    } else if garage.door2_world_index.is_some() {
        CVector::new(garage.door2_x, garage.door2_y, 0.0)
    } else {
        CVector::new(target.x, target.y, 0.0)
    };
    let center = CVector::new(
        (garage.x1 + garage.x2) * 0.5,
        (garage.y1 + garage.y2) * 0.5,
        0.0,
    );
    let offset = CVector::new(door.x - center.x, door.y - center.y, 0.0);
    let length = (offset.x * offset.x + offset.y * offset.y).sqrt();
    let direction = if length > f32::EPSILON {
        CVector::new(offset.x / length, offset.y / length, 0.0)
    } else {
        CVector::new(0.0, -1.0, 0.0)
    };
    let ground = file_loader
        .world
        .find_ground_z_for_3d_coord(target.x, target.y, target.z, &file_loader.model_info)
        .unwrap_or(target.z - 0.2);
    let source = CVector::new(
        door.x + 13.0 * direction.x,
        door.y + 13.0 * direction.y,
        ground + 3.1,
    );
    let front = CVector::new(
        target.x - source.x,
        target.y - source.y,
        target.z - source.z,
    );
    let front_length = (front.x * front.x + front.y * front.y + front.z * front.z).sqrt();
    camera.position = source;
    camera.look_at = target;
    if front_length > f32::EPSILON {
        camera.orientation = CVector::new(
            front.x / front_length,
            front.y / front_length,
            front.z / front_length,
        );
    }
    if let Some(cam) = camera.cams.get_mut(camera.active_cam) {
        cam.mode = crate::core::Cam::eCamMode::MODE_FIXED;
        cam.source = source;
        cam.front = camera.orientation;
        cam.up = CVector::new(0.0, 0.0, 1.0);
        cam.fov = 70.0;
    }
    camera.garage_fixed_cam_position_set = true;
    camera.player_is_in_garage = true;
    true
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSplineSample {
    pub source: CVector,
    pub target: CVector,
    pub up: CVector,
    pub fov: f32,
    pub position_along_spline: f32,
    pub finish_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraScriptMode {
    /// Follow player / default gameplay camera (after Restore).
    FollowPlayer,
    /// Fixed position + look-at from script (TakeControlNoEntity / fixed mode).
    Fixed,
    /// Script owns the camera and tracks a CPools entity handle.
    ScriptTarget,
    /// re3 `CCam::MODE_FLYBY`, driven by cutscene path splines.
    FlyBy,
    /// re3 `CCam::MODE_BEHINDCAR` while CRemote owns a script vehicle.
    FollowRemoteVehicle,
}

/// SCM pool handles are scoped to their actor kind. Equal numeric ped/vehicle
/// handles identify different Bevy entities, including during deferred spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraScriptTarget {
    Char(ScriptHandle),
    Vehicle(ScriptHandle),
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CCamera {
    pub cams: Vec<CCam>,
    pub active_cam: usize,
    /// Resolved through the canonical ECS index after SCM spawn barriers.
    pub script_target: Option<CameraScriptTarget>,
    /// Exact CCam mode requested by `CCamera::TakeControl`.
    pub script_target_mode: i32,
    /// INTERPOLATION/JUMP_CUT requested by the SCM command.
    pub script_switch_style: i32,
    pub position: CVector,
    /// CCamera::Process's four-frame native speech-gating history.
    pub average_speed: f32,
    previous_speed_position: Option<CVector>,
    speed_so_far: f32,
    speed_frames: u8,
    pub orientation: CVector,
    /// Look-at point for fixed / point-at-point modes.
    pub look_at: CVector,
    pub script_mode: CameraScriptMode,
    pub fade_colour: [u8; 3],
    /// re3 `m_FadeTargetIsSplashScreen`. Black SCM fade colour is special:
    /// `DoFade` composites the currently loaded splash instead of a flat black
    /// rectangle while the fade value is non-zero.
    pub fade_target_is_splash_screen: bool,
    /// Remaining fade time in seconds (re3 Fade duration).
    pub fade_time_remaining: f32,
    /// Total duration captured by `Fade`; `ProcessFade` derives intensity from elapsed time.
    pub fade_duration: f32,
    /// re3 `m_fFLOATingFade` / `CDraw::FadeValue`, in the 0..255 range.
    pub fade_value: f32,
    /// re3 direction: 0 out, 1 in (see CCamera::Fade).
    pub fade_direction: i32,
    pub fading: bool,
    pub path_splines: [[f32; MAX_CAMERA_SPLINE_PATH_LENGTH]; MAX_NUM_OF_SPLINETYPES],
    pub starting_spline: bool,
    pub position_along_spline: f32,
    pub cutscene_finished: bool,
    pub cutscene_offset: CVector,
    pub wide_screen_on: bool,
    pub flyby_elapsed_ms: f32,
    pub flyby_finish_time_ms: u32,
    pub flyby_fov: f32,
    pub flyby_up: CVector,
    pub cam_directly_behind: bool,
    pub cam_directly_in_front: bool,
    pub ped_orient_for_behind_or_in_front: f32,
    /// Frame-local re3 `pToGarageWeAreIn`, represented by the stable garage
    /// array index instead of a raw `CGarage*`.
    pub garage_we_are_in: Option<usize>,
    /// re3 `pToGarageWeAreInForHackAvoidFirstPerson`.
    pub garage_for_first_person_hack: Option<usize>,
    pub garage_fixed_cam_position_set: bool,
    pub player_is_in_garage: bool,
    pub just_came_out_of_garage: bool,
    /// re3 `CCam::Beta`/`Alpha` for MODE_FOLLOWPED mouse camera.
    pub follow_beta: f32,
    pub follow_alpha: f32,
    pub follow_initialised: bool,
    /// re3 FREE_CAM option (CCamera::bFreeCam). Default is off: normal
    /// mouse-follow rotates the on-foot player to the camera heading.
    pub free_cam: bool,
    /// re3 `m_bCameraJustRestored`: the first gameplay frame after a
    /// cutscene/restore must rebuild the active follow camera from the player
    /// instead of leaving the fly-by source in place.
    pub camera_just_restored: bool,
    pub car_zoom_indicator: i32,
    pub ped_zoom_indicator: i32,
    pub car_zoom_value_smooth: f32,
    pub use_transition_beta: bool,
    pub transition_beta: f32,
    pub transition_state: i32,
    pub blur_red: i32,
    pub blur_green: i32,
    pub blur_blue: i32,
    pub motion_blur: i32,
    /// re3 `m_imotionBlurAddAlpha`.
    pub motion_blur_add_alpha: i32,
    pub blur_type: i32,
    /// re3 `m_fCamShakeForce`.
    pub cam_shake_force: f32,
    /// re3 `m_uiCamShakeStart`.
    pub cam_shake_start_ms: u32,
    /// re3 `TheCamera.LODDistMultiplier`, after renderer LOD scale.
    pub lod_dist_multiplier: f32,
    /// re3 `TheCamera.GenerationDistMultiplier`, before renderer LOD scale.
    pub generation_dist_multiplier: f32,
    /// re3 `m_bIgnoreFadingStuffForMusic`.
    pub ignore_fading_stuff_for_music: bool,
    pub near_clip_script: f32,
    /// Live RenderWare raster aspect used to build frustum normals.
    pub aspect_ratio: f32,
    pub far_clip: f32,
}

impl Default for CCamera {
    fn default() -> Self {
        Self {
            cams: vec![CCam::default()],
            active_cam: 0,
            script_target: None,
            script_target_mode: 0,
            script_switch_style: 0,
            position: CVector::ZERO,
            average_speed: 0.0,
            previous_speed_position: None,
            speed_so_far: 0.0,
            speed_frames: 0,
            orientation: CVector::new(0.0, 1.0, 0.0),
            look_at: CVector::ZERO,
            script_mode: CameraScriptMode::FollowPlayer,
            fade_colour: [0, 0, 0],
            fade_target_is_splash_screen: false,
            fade_time_remaining: 0.0,
            fade_duration: 0.0,
            fade_value: 0.0,
            fade_direction: 0,
            fading: false,
            path_splines: [[0.0; MAX_CAMERA_SPLINE_PATH_LENGTH]; MAX_NUM_OF_SPLINETYPES],
            starting_spline: false,
            position_along_spline: 0.0,
            cutscene_finished: false,
            cutscene_offset: CVector::ZERO,
            wide_screen_on: false,
            flyby_elapsed_ms: 0.0,
            flyby_finish_time_ms: 0,
            flyby_fov: 70.0,
            flyby_up: CVector::new(0.0, 0.0, 1.0),
            cam_directly_behind: false,
            cam_directly_in_front: false,
            ped_orient_for_behind_or_in_front: 0.0,
            garage_we_are_in: None,
            garage_for_first_person_hack: None,
            garage_fixed_cam_position_set: false,
            player_is_in_garage: false,
            just_came_out_of_garage: false,
            follow_beta: 0.0,
            follow_alpha: 15.0_f32.to_radians(),
            follow_initialised: false,
            free_cam: false,
            camera_just_restored: false,
            car_zoom_indicator: CAM_ZOOM_2,
            ped_zoom_indicator: CAM_ZOOM_2,
            car_zoom_value_smooth: 0.0,
            use_transition_beta: false,
            transition_beta: 0.0,
            transition_state: 0,
            blur_red: 255,
            blur_green: 255,
            blur_blue: 255,
            motion_blur: 0,
            motion_blur_add_alpha: 0,
            blur_type: 0,
            cam_shake_force: 0.0,
            cam_shake_start_ms: 0,
            lod_dist_multiplier: 1.0,
            generation_dist_multiplier: 1.0,
            ignore_fading_stuff_for_music: false,
            near_clip_script: DEFAULT_NEAR,
            aspect_ratio: 4.0 / 3.0,
            far_clip: 2000.0,
        }
    }
}

impl CCamera {
    /// ClearPlayerWeaponMode in the current Bevy camera projection: the native
    /// player weapon request is represented by the active weapon camera mode.
    pub(crate) fn clear_player_weapon_mode(&mut self) {
        if self.weapon_mode_active() {
            self.cams[self.active_cam].mode = crate::core::Cam::eCamMode::MODE_FOLLOWPED;
            self.cams[self.active_cam].reset_statics = true;
            self.follow_initialised = false;
            self.near_clip_script = DEFAULT_NEAR;
            self.set_motion_blur(0, 0, 0, 0, MOTION_BLUR_NONE);
        }
    }

    pub(crate) fn weapon_mode_active(&self) -> bool {
        self.cams.get(self.active_cam).is_some_and(|cam| matches!(cam.mode,
            crate::core::Cam::eCamMode::MODE_SNIPER | crate::core::Cam::eCamMode::MODE_ROCKETLAUNCHER
            | crate::core::Cam::eCamMode::MODE_M16_1STPERSON))
    }

    /// CCam::Process_Sniper / Process_Rocket / Process_M16_1stPerson.
    pub(crate) fn process_weapon_camera(&mut self, head: CVector, heading: f32,
        pad: &crate::core::Pad::CPad, mouse: bevy::prelude::Vec2, sensitivity: f32,
        invert: bool, time_step: f32, mut blocked: impl FnMut(CVector, CVector) -> bool) {
        let cam = &mut self.cams[self.active_cam];
        if cam.reset_statics {
            cam.beta = heading + std::f32::consts::FRAC_PI_2;
            cam.alpha = 0.0;
            cam.initial_player_orientation = cam.beta;
            cam.fov = 70.0;
            cam.weapon_target_fov = cam.fov;
            cam.weapon_fov_speed = 0.0;
            cam.collision_check_frame = 0;
            cam.last_obscured = false;
            cam.reset_statics = false;
        }
        cam.source = CVector::new(head.x - 0.19 * cam.initial_player_orientation.cos(),
            head.y - 0.19 * cam.initial_player_orientation.sin(), head.z + 0.1);
        let fov_scale = cam.fov / 80.0;
        if mouse != bevy::prelude::Vec2::ZERO {
            cam.beta += -3.0 * mouse.x * 0.0025 * sensitivity * fov_scale;
            cam.alpha += 4.0 * mouse.y * 0.003 * sensitivity * fov_scale * if invert { -1.0 } else { 1.0 };
        } else {
            let x = -(pad.sniper_mode_look_left_right() as f32);
            let y = pad.sniper_mode_look_up_down() as f32;
            cam.beta += (x / 100.0).powi(2) * x.signum() * 0.8 / 14.0 * fov_scale * time_step;
            cam.alpha += (y / 150.0).powi(2) * y.signum() / 14.0 * fov_scale * time_step;
        }
        cam.beta = wrap_pi(cam.beta);
        cam.alpha = cam.alpha.clamp(-89.5_f32.to_radians(), 60_f32.to_radians());
        if cam.mode == crate::core::Cam::eCamMode::MODE_SNIPER {
            if (pad.mouse_wheel_up || pad.mouse_wheel_down) && (pad.sniper_zoom_in() || pad.sniper_zoom_out()) {
                cam.weapon_target_fov = cam.fov + if pad.sniper_zoom_out() { 10.0 } else { -10.0 };
                well_buffer(cam.weapon_target_fov, &mut cam.fov, &mut cam.weapon_fov_speed,
                    0.5, 0.25, false, time_step);
            } else if pad.sniper_zoom_in() || pad.sniper_zoom_out() {
                let factor = (255.0 * time_step + 10000.0) / 10000.0;
                cam.fov = if pad.sniper_zoom_out() { cam.fov * factor } else { cam.fov / factor };
                cam.weapon_target_fov = cam.fov;
                cam.weapon_fov_speed = 0.0;
            } else {
                well_buffer(cam.weapon_target_fov, &mut cam.fov, &mut cam.weapon_fov_speed,
                    0.5, 0.25, false, time_step);
            }
            cam.fov = cam.fov.clamp(15.0, 70.0);
        }
        let front_at = |alpha: f32, beta: f32| CVector::new(alpha.cos()*beta.cos(), alpha.cos()*beta.sin(), alpha.sin());
        cam.front = front_at(cam.alpha, cam.beta);
        cam.up = camera_up(cam.front);
        cam.collision_check_frame = (cam.collision_check_frame + 1) % 12;
        if cam.collision_check_frame == 1 {
            let source = vector_add(cam.source, vector_scale(cam.front, 0.4));
            cam.last_obscured = [(cam.alpha, cam.beta),
                (cam.alpha - 20_f32.to_radians(), cam.beta + 35_f32.to_radians()),
                (cam.alpha - 20_f32.to_radians(), cam.beta - 35_f32.to_radians())].into_iter()
                .any(|(alpha, beta)| blocked(source, vector_add(cam.source, vector_scale(front_at(alpha, beta), 3.0))));
        }
        self.near_clip_script = if cam.last_obscured { 0.4 } else { DEFAULT_NEAR };
        self.position = cam.source;
        self.orientation = cam.front;
        self.look_at = vector_add(cam.source, vector_scale(cam.front, 3.0));
        if cam.mode == crate::core::Cam::eCamMode::MODE_SNIPER {
            self.set_motion_blur(180, 255, 180, 120, MOTION_BLUR_SNIPER);
        }
    }

    #[allow(non_snake_case)]
    pub fn Init(&mut self) {
        *self = Self::default();
    }

    /// re3 `CCamera::Process`: user-selected zoom state, independent of rendering.
    pub fn process_view_selection(
        &mut self,
        pad: &crate::core::Pad::CPad,
        in_vehicle: bool,
        standard_controls: bool,
    ) {
        if self.wide_screen_on || self.script_mode != CameraScriptMode::FollowPlayer {
            return;
        }
        let up = pad.cycle_camera_mode_up_just_down();
        let down = pad.cycle_camera_mode_down_just_down();
        if in_vehicle {
            self.car_zoom_indicator += i32::from(down) - i32::from(up);
            if self.car_zoom_indicator < CAM_ZOOM_1STPRS {
                self.car_zoom_indicator = CAM_ZOOM_CINEMATIC;
            }
            if self.car_zoom_indicator > CAM_ZOOM_CINEMATIC {
                self.car_zoom_indicator = CAM_ZOOM_1STPRS;
            }
        } else {
            if up || down {
                if standard_controls {
                    self.ped_zoom_indicator = if self.ped_zoom_indicator == CAM_ZOOM_TOPDOWN {
                        CAM_ZOOM_1
                    } else {
                        CAM_ZOOM_TOPDOWN
                    };
                } else {
                    self.ped_zoom_indicator += i32::from(down) - i32::from(up);
                }
            }
            if self.ped_zoom_indicator < CAM_ZOOM_1 {
                self.ped_zoom_indicator = CAM_ZOOM_TOPDOWN;
            }
            if self.ped_zoom_indicator > CAM_ZOOM_TOPDOWN {
                self.ped_zoom_indicator = CAM_ZOOM_1;
            }
        }
        // TODO(parity): CCam::Process_1stPerson, Process_TopDown,
        // Process_TopDownPed and CCamera::Process cinematic mode dispatch.
        if up || down {
            bevy::log::info!(target: "rust_gta_3::camera", car_zoom = self.car_zoom_indicator, ped_zoom = self.ped_zoom_indicator, "camera view selection changed; special-mode dispatch remains incomplete");
        }
    }

    /// re3 `CCamera::GetLookingLRBFirstPerson`.
    pub fn get_looking_lrb_first_person(&self) -> bool {
        self.cams.get(self.active_cam).is_some_and(|cam| {
            cam.mode == eCamMode::MODE_1STPERSON && cam.direction_was_looking != LOOKING_FORWARD
        })
    }

    /// re3 CCamera::GetLookingForwardFirstPerson.
    pub fn get_looking_forward_first_person(&self) -> bool {
        self.cams.get(self.active_cam).is_some_and(|cam|
            cam.mode == eCamMode::MODE_1STPERSON && cam.direction_was_looking == LOOKING_FORWARD)
    }

    /// re3 `TheCamera.SetCamPositionForFixedMode`
    #[allow(non_snake_case)]
    pub fn SetCamPositionForFixedMode(&mut self, position: CVector, orientation: CVector) {
        self.position = position;
        self.orientation = orientation;
        self.script_mode = CameraScriptMode::Fixed;
    }

    /// re3 `TheCamera.TakeControlNoEntity` simplified: look at world point.
    pub fn take_control_no_entity(&mut self, look_at: CVector) {
        self.look_at = look_at;
        self.script_target = None;
        self.script_mode = CameraScriptMode::Fixed;
    }

    /// re3 `CCamera::TakeControl(CEntity*, mode, switch, CAMCONTROL_SCRIPT)`.
    pub fn take_control_script_entity(
        &mut self,
        target: CameraScriptTarget,
        position: CVector,
        mode: i32,
        switch_style: i32,
    ) {
        self.script_target = Some(target);
        self.script_target_mode = mode;
        self.script_switch_style = switch_style;
        self.look_at = position;
        self.script_mode = CameraScriptMode::ScriptTarget;
    }

    /// re3 `TheCamera.Restore`.  The gameplay camera uses the same safe
    /// follow-state reset as the jump-cut path.
    pub fn restore(&mut self) {
        self.restore_with_jump_cut();
    }

    /// re3 `CCamera::RestoreWithJumpCut`, used by
    /// `CCutsceneMgr::DeleteCutsceneData`.  The old port only changed the
    /// script enum, so the RenderWare source/target from the fly-by could
    /// survive until the next valid follow update and present as a locked
    /// camera after the intro.
    pub fn restore_with_jump_cut(&mut self) {
        self.script_mode = CameraScriptMode::FollowPlayer;
        self.script_target = None;
        self.script_target_mode = 0;
        self.script_switch_style = 0;
        self.position_along_spline = 0.0;
        self.starting_spline = false;
        self.wide_screen_on = false;
        self.cutscene_finished = false;
        self.cam_directly_behind = false;
        self.cam_directly_in_front = false;
        self.use_transition_beta = false;
        self.transition_state = 0;
        self.follow_initialised = false;
        self.camera_just_restored = true;
        if let Some(cam) = self.cams.get_mut(self.active_cam) {
            cam.mode = crate::core::Cam::eCamMode::MODE_FOLLOWPED;
            cam.reset_statics = true;
        }
    }

    /// re3 `TheCamera.SetFadeColour`
    pub fn set_fade_colour(&mut self, r: i32, g: i32, b: i32) {
        self.fade_colour = [
            r.clamp(0, 255) as u8,
            g.clamp(0, 255) as u8,
            b.clamp(0, 255) as u8,
        ];
        self.fade_target_is_splash_screen = self.fade_colour == [0, 0, 0];
    }

    /// re3 `TheCamera.Fade(timeSeconds, direction)`
    pub fn fade(&mut self, time_seconds: f32, direction: i32) {
        let duration = time_seconds.max(0.0);
        self.fade_duration = duration;
        self.fade_time_remaining = duration;
        self.fade_direction = direction;
        // C++ always raises m_bFading here. A zero-duration request is only
        // cleared by the following `ProcessFade`, which SCM can observe via
        // GET_FADING_STATUS in the meantime.
        self.fading = true;
        self.fade_value = match (direction, duration > 0.0) {
            // FADE_IN starts fully covered; FADE_OUT starts uncovered.
            (1, true) => 255.0,
            (0, true) => 0.0,
            // CCamera::ProcessFade resolves a zero-duration fade immediately.
            (1, false) => 0.0,
            _ => 255.0,
        };
    }

    pub fn get_fading(&self) -> bool {
        self.fading
    }

    /// PC `CCam::Process_FollowPedWithMouse` camera placement. Input updates
    /// Alpha/Beta separately; this performs the reference target translation,
    /// distance curve, source placement and CColPoint source clipping.
    pub fn process_follow_ped_with_mouse<F>(
        &mut self,
        camera_target: CVector,
        target_orientation: f32,
        mut static_hit: F,
    ) where
        F: FnMut(CVector, CVector) -> Option<CVector>,
    {
        const BASE_DIST: f32 = 1.7;
        const ANGLE_DIST: f32 = 2.0;
        const FALLOFF: f32 = 3.0;
        const TRANSLATE_CAM_UP: f32 = 0.8;

        if !self.follow_initialised {
            let forward_x = -target_orientation.sin();
            let forward_y = target_orientation.cos();
            self.follow_beta = forward_y.atan2(forward_x);
            self.follow_alpha = -0.22; // re3 fDefaultAlphaOrient
            self.follow_initialised = true;
        }
        if self.cam_directly_behind {
            self.follow_beta = self.ped_orient_for_behind_or_in_front;
        }
        if self.cam_directly_in_front {
            self.follow_beta = self.ped_orient_for_behind_or_in_front + std::f32::consts::PI;
        }
        self.follow_beta = wrap_pi(self.follow_beta);
        self.follow_alpha = self
            .follow_alpha
            .clamp(-89.5_f32.to_radians(), 45.0_f32.to_radians());

        let target = CVector::new(
            camera_target.x,
            camera_target.y,
            camera_target.z + TRANSLATE_CAM_UP,
        );
        let cam_dist = if self.follow_alpha > BASE_DIST {
            BASE_DIST
                + (self.follow_alpha * FALLOFF)
                    .min(std::f32::consts::FRAC_PI_2)
                    .cos()
                    * ANGLE_DIST
        } else {
            BASE_DIST + self.follow_alpha.cos() * ANGLE_DIST
        };
        let front = CVector::new(
            self.follow_alpha.cos() * self.follow_beta.cos(),
            self.follow_alpha.cos() * self.follow_beta.sin(),
            self.follow_alpha.sin(),
        );
        let desired_source = CVector::new(
            target.x - front.x * cam_dist,
            target.y - front.y * cam_dist,
            target.z - front.z * cam_dist,
        );
        let clipped_source = static_hit(target, desired_source);
        let source = clipped_source.unwrap_or(desired_source);

        self.position = source;
        self.look_at = target;
        // re3 keeps the analytically computed Front vector even when LOS clips
        // Source; GetVectorsReadyForRW only normalises Front itself. Deriving it
        // again from (Target-Source) changes aim under collision and also loses
        // precision at large world coordinates.
        self.orientation = front;
        if let Some(cam) = self.cams.get_mut(self.active_cam) {
            cam.mode = crate::core::Cam::eCamMode::MODE_FOLLOWPED;
            cam.source = source;
            cam.front = self.orientation;
            cam.up = CVector::new(0.0, 0.0, 1.0);
            cam.fov = 70.0;
        }
        self.cam_directly_behind = false;
        self.cam_directly_in_front = false;
        self.camera_just_restored = false;
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 `CCam::LookBehind` ped branch, called after `MODE_FOLLOWPED`
    /// processing when `CPad::GetLookBehindForPed` is held.
    pub fn apply_ped_look_behind<F>(&mut self, camera_target: CVector, mut static_hit: F)
    where
        F: FnMut(CVector, CVector) -> Option<CVector>,
    {
        let Some(active) = self.cams.get(self.active_cam).copied() else {
            return;
        };
        if active.mode != eCamMode::MODE_FOLLOWPED {
            return;
        }
        let mut cam = active;
        let angle = CGeneral::get_atan_of_xy(
            cam.source.x - camera_target.x,
            cam.source.y - camera_target.y,
        ) + std::f32::consts::PI;
        cam.source = CVector::new(
            4.5 * angle.cos() + camera_target.x,
            4.5 * angle.sin() + camera_target.y,
            camera_target.z + 1.15,
        );
        let test_target = CVector::new(camera_target.x, camera_target.y, cam.source.z);
        if let Some(hit) = static_hit(test_target, cam.source) {
            cam.source.x = hit.x;
            cam.source.y = hit.y;
        }
        cam.front = normalize(vector_sub(camera_target, cam.source));
        cam.up = camera_up(cam.front);
        cam.direction_was_looking = crate::core::Cam::LOOKING_BEHIND;
        self.position = cam.source;
        self.orientation = cam.front;
        self.look_at = camera_target;
        self.cams[self.active_cam] = cam;
    }

    /// re3 `CCam::Process_Cam_On_A_String` and its collision helpers.
    ///
    /// World queries are adapters so the camera algorithm remains the C++
    /// state machine while ECS owns static buildings and runtime vehicles.
    #[allow(clippy::too_many_arguments)]
    pub fn process_cam_on_a_string<FGround, FRoof, FStaticHit, FVehicleHit>(
        &mut self,
        vehicle_position: CVector,
        vehicle_forward: CVector,
        vehicle_move_speed: CVector,
        dimensions: CVector,
        time_step: f32,
        look_direction: i32,
        mut ground_z: FGround,
        mut roof_z: FRoof,
        mut static_hit: FStaticHit,
        mut vehicle_hit_max_z: FVehicleHit,
    ) where
        FGround: FnMut(f32, f32, f32) -> Option<f32>,
        FRoof: FnMut(f32, f32, f32) -> Option<f32>,
        FStaticHit: FnMut(CVector, CVector) -> Option<CVector>,
        FVehicleHit: FnMut(CVector, CVector) -> Option<f32>,
    {
        let desired_zoom = match self.car_zoom_indicator {
            CAM_ZOOM_1 => {
                if self.free_cam {
                    -1.0
                } else {
                    0.05
                }
            }
            CAM_ZOOM_2 => {
                if self.free_cam {
                    2.0
                } else {
                    1.9
                }
            }
            CAM_ZOOM_3 => {
                if self.free_cam {
                    6.0
                } else {
                    3.9
                }
            }
            _ => self.car_zoom_value_smooth,
        };
        let step = 0.12 * time_step;
        self.car_zoom_value_smooth +=
            (desired_zoom - self.car_zoom_value_smooth).clamp(-step, step);
        let Some(active) = self.cams.get(self.active_cam).copied() else {
            return;
        };
        let mut cam = active;
        if cam.mode != crate::core::Cam::eCamMode::MODE_CAM_ON_A_STRING {
            cam.mode = crate::core::Cam::eCamMode::MODE_CAM_ON_A_STRING;
            cam.source = self.position;
            cam.reset_statics = true;
        }

        cam.collision_check_frame += 1;
        if cam.collision_check_frame > cam.collision_check_period {
            cam.collision_check_frame = 1;
        }
        cam.collision_checks_on = cam.collision_check_frame == 1;
        cam.fov = 70.0;
        if cam.reset_statics {
            cam.alpha_speed = 0.0;
            cam.last_target_alpha_with_collision_on = 0.0;
            cam.last_top_alpha_speed = 0.0;
            cam.last_alpha_speed_step = 0.0;
            cam.previous_near_check_near_clip_small = false;
        }

        let target_orientation = if vehicle_forward.x == 0.0 && vehicle_forward.y == 0.0 {
            0.0
        } else {
            vehicle_forward.y.atan2(vehicle_forward.x)
        };
        let mut target = vehicle_position;
        let base_dist = (dimensions.x * dimensions.x + dimensions.y * dimensions.y).sqrt();
        target.z += dimensions.z - 0.1;
        cam.beta = (target.y - cam.source.y).atan2(target.x - cam.source.x);
        cam.alpha = wrap_pi(cam.alpha);
        cam.beta = wrap_pi(cam.beta);
        cam.distance_before_changes = magnitude_2d(vector_sub(cam.source, target));

        cam_on_a_string_unobscured(
            &mut cam,
            target,
            base_dist,
            vehicle_forward,
            self.car_zoom_value_smooth,
        );
        work_out_cam_height(
            &mut cam,
            target,
            target_orientation,
            dimensions.z,
            vehicle_forward,
            self.car_zoom_indicator,
            self.car_zoom_value_smooth,
            time_step,
            &mut ground_z,
            &mut roof_z,
        );
        rot_cam_if_in_front_car(
            &mut cam,
            target,
            target_orientation,
            vehicle_forward,
            vehicle_move_speed,
            time_step,
            look_direction != crate::core::Cam::LOOKING_FORWARD,
            self.transition_state,
            self.use_transition_beta,
            self.transition_beta,
            &mut self.cam_directly_behind,
            &mut self.cam_directly_in_front,
        );
        fix_cam_if_obscured(
            &mut cam,
            target,
            vehicle_position,
            dimensions.z,
            target_orientation,
            time_step,
            &mut static_hit,
        );
        fix_cam_when_obscured_by_vehicle(&mut cam, target, time_step, &mut vehicle_hit_max_z);

        cam.front = normalize(vector_sub(target, cam.source));
        cam.up = camera_up(cam.front);
        apply_vehicle_look_direction(
            &mut cam,
            vehicle_position,
            vehicle_forward,
            look_direction,
            &mut static_hit,
        );
        cam.reset_statics = false;
        self.position = cam.source;
        self.look_at = target;
        self.orientation = cam.front;
        self.cams[self.active_cam] = cam;
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 `CCamera::Orientation = Atan2(GetForward().x, GetForward().y)`.
    /// re3 CCam::Process_1stPerson vehicle branch plus the following
    /// LookBehind/LookLeft/LookRight dispatch from CCam::Process.
    pub fn process_vehicle_first_person(
        &mut self,
        seat_source: CVector,
        vehicle_forward: CVector,
        vehicle_up: CVector,
        upside_down: bool,
        is_boat: bool,
        look_direction: i32,
    ) {
        use crate::core::Cam::{LOOKING_BEHIND, LOOKING_LEFT, LOOKING_RIGHT};

        let Some(active) = self.cams.get(self.active_cam).copied() else {
            return;
        };
        let mut cam = active;
        if cam.mode != eCamMode::MODE_1STPERSON {
            cam.mode = eCamMode::MODE_1STPERSON;
            cam.reset_statics = true;
            cam.first_person_world_fixer = 0.0;
        }
        cam.fov = 60.0;
        cam.source = seat_source;
        if upside_down {
            if cam.first_person_world_fixer < 0.5 {
                cam.first_person_world_fixer += 0.03;
            } else {
                cam.first_person_world_fixer = 0.5;
            }
        } else if cam.first_person_world_fixer < 0.0 {
            cam.first_person_world_fixer += 0.03;
        } else {
            cam.first_person_world_fixer = 0.0;
        }
        cam.source.z += cam.first_person_world_fixer;

        let forward = normalize(vehicle_forward);
        let up = normalize(vehicle_up);
        let right = normalize(cross(forward, up));
        cam.front = forward;
        cam.up = normalize(cross(right, forward));

        match look_direction {
            LOOKING_BEHIND => {
                if is_boat {
                    cam.source.z -= 0.5;
                }
                cam.source = vector_add(cam.source, vector_scale(forward, 0.25));
                cam.front = vector_scale(forward, -1.0);
            }
            LOOKING_LEFT => {
                if is_boat {
                    cam.source.z -= 0.5;
                }
                cam.front = normalize(vector_scale(cross(forward, up), -1.0));
                cam.up = camera_up(cam.front);
            }
            LOOKING_RIGHT => {
                if is_boat {
                    cam.source.z -= 0.5;
                }
                cam.front = normalize(cross(forward, up));
                cam.up = camera_up(cam.front);
            }
            _ => {}
        }
        cam.direction_was_looking = look_direction;
        cam.reset_statics = false;
        self.position = cam.source;
        self.orientation = cam.front;
        self.look_at = vector_add(cam.source, cam.front);
        self.cams[self.active_cam] = cam;
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 CCam::Process_TopDown for a vehicle target.
    pub fn process_vehicle_top_down<F>(
        &mut self,
        vehicle_position: CVector,
        vehicle_forward: CVector,
        vehicle_move_speed: CVector,
        time_step: f32,
        mut building_hit: F,
    ) where
        F: FnMut(CVector, CVector) -> Option<CVector>,
    {
        let Some(active) = self.cams.get(self.active_cam).copied() else {
            return;
        };
        let mut cam = active;
        if cam.mode != eCamMode::MODE_TOPDOWN {
            cam.mode = eCamMode::MODE_TOPDOWN;
            cam.reset_statics = true;
        }

        let forward = normalize(CVector::new(vehicle_forward.x, vehicle_forward.y, 0.0));
        let fwd_speed_x = vehicle_move_speed.x * forward.x;
        let fwd_speed_y = vehicle_move_speed.y * forward.y;
        let projected = (fwd_speed_x * fwd_speed_x + fwd_speed_y * fwd_speed_y).sqrt();
        let target_speed_var = if fwd_speed_x + fwd_speed_y > 0.0 {
            (projected / 0.9).min(1.0)
        } else {
            -(projected / 1.8).min(0.5)
        };
        cam.speed_var = 0.895 * cam.speed_var + 0.105 * target_speed_var;
        if cam.reset_statics {
            cam.topdown_height_buffer = 0.0;
            cam.topdown_height_speed = 0.0;
        }

        let target_front = vector_add(
            vehicle_position,
            CVector::new(
                18.0 * forward.x * cam.speed_var,
                18.0 * forward.y * cam.speed_var,
                0.0,
            ),
        );
        const MIX: f32 = 0.4096;
        let target = CVector::new(
            MIX * vehicle_position.x + (1.0 - MIX) * target_front.x,
            MIX * vehicle_position.y + (1.0 - MIX) * target_front.y,
            vehicle_position.z,
        );
        cam.source = CVector::new(
            target.x,
            target.y,
            target.z + (40.0 * cam.speed_var + 30.0) * 0.8,
        );

        let mut height_target = 0.0;
        let test_source = cam.source;
        let test_target = CVector::new(test_source.x, test_source.y, target.z);
        if let Some(hit) = building_hit(test_target, test_source) {
            if cam.source.z < hit.z + 3.0 {
                height_target = hit.z + 3.0 - cam.source.z;
            }
        } else {
            let above = CVector::new(test_source.x, test_source.y, test_source.z + 10.0);
            if let Some(hit) = building_hit(above, test_source)
                && cam.source.z < hit.z + 3.0
            {
                height_target = hit.z + 3.0 - cam.source.z;
            }
        }
        well_buffer(
            height_target,
            &mut cam.topdown_height_buffer,
            &mut cam.topdown_height_speed,
            0.2,
            0.02,
            false,
            time_step,
        );
        cam.source.z += cam.topdown_height_buffer;
        cam.front = normalize(CVector::new(-0.01, -0.01, -1.0));
        cam.up = CVector::new(0.0, 1.0, 0.0);
        cam.fov = 70.0;
        cam.direction_was_looking = LOOKING_FORWARD;
        cam.reset_statics = false;
        self.position = cam.source;
        self.orientation = cam.front;
        self.look_at = vector_add(cam.source, vector_scale(cam.front, 30.0));
        self.cams[self.active_cam] = cam;
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 CCam::Process_TopDownPed.
    pub fn process_ped_top_down<F>(
        &mut self,
        camera_target: CVector,
        player_move_speed: CVector,
        lock_target_position: Option<CVector>,
        time_step: f32,
        mut building_hit: F,
    ) where
        F: FnMut(CVector, CVector) -> Option<CVector>,
    {
        let Some(active) = self.cams.get(self.active_cam).copied() else {
            return;
        };
        let mut cam = active;
        if cam.mode != eCamMode::MODE_TOP_DOWN_PED {
            cam.mode = eCamMode::MODE_TOP_DOWN_PED;
            cam.reset_statics = true;
        }
        if cam.reset_statics {
            cam.topdown_ped_sample_count = 0;
            cam.topdown_ped_average_speed = 0.0;
            cam.topdown_ped_speed_sum = 0.0;
            cam.topdown_ped_height_buffer = 0.0;
            cam.topdown_ped_height_speed = 0.0;
            cam.topdown_ped_south_adjuster = 0.0;
            cam.topdown_ped_south_speed = 0.0;
            cam.topdown_ped_north_adjuster = 0.0;
            cam.topdown_ped_north_speed = 0.0;
            cam.topdown_ped_west_adjuster = 0.0;
            cam.topdown_ped_west_speed = 0.0;
            cam.topdown_ped_east_adjuster = 0.0;
            cam.topdown_ped_east_speed = 0.0;
        }

        cam.topdown_ped_sample_count = cam.topdown_ped_sample_count.saturating_add(1);
        cam.topdown_ped_speed_sum += player_move_speed.magnitude();
        if cam.topdown_ped_sample_count == 5 {
            cam.topdown_ped_average_speed =
                0.4 * cam.topdown_ped_average_speed + 0.6 * (cam.topdown_ped_speed_sum / 5.0);
            cam.topdown_ped_sample_count = 0;
            cam.topdown_ped_speed_sum = 0.0;
        }

        let mut height_target: f32 = if cam.topdown_ped_average_speed > 0.145 {
            7.0
        } else if cam.topdown_ped_average_speed > 0.04 {
            4.5
        } else if cam.topdown_ped_average_speed > 0.01 {
            2.5
        } else {
            0.0
        };
        if let Some(lock_target) = lock_target_position {
            let distance = magnitude_2d(vector_sub(lock_target, camera_target));
            if distance > 6.0 {
                height_target = height_target.max(distance / 22.0 * 37.0);
            }
        }

        cam.source = vector_add(camera_target, CVector::new(0.0, -1.0, 9.0));
        let test_source = cam.source;
        let test_target = CVector::new(test_source.x, test_source.y, camera_target.z);
        if let Some(hit) = building_hit(test_target, test_source) {
            if camera_target.z + 9.0 + height_target < hit.z + 3.0 {
                height_target = hit.z + 3.0 - (camera_target.z + 9.0);
            }
        } else {
            let raised_source = CVector::new(
                test_source.x,
                test_source.y,
                test_source.z + height_target,
            );
            let raised_target = CVector::new(
                raised_source.x,
                raised_source.y,
                raised_source.z + 10.0,
            );
            if let Some(hit) = building_hit(raised_target, raised_source)
                && camera_target.z + 9.0 + height_target < hit.z + 3.0
            {
                height_target = hit.z + 3.0 - (camera_target.z + 9.0);
            }
        }
        well_buffer(
            height_target,
            &mut cam.topdown_ped_height_buffer,
            &mut cam.topdown_ped_height_speed,
            0.3,
            0.03,
            false,
            time_step,
        );
        cam.source.z += cam.topdown_ped_height_buffer;

        avoid_walls_top_down_ped(
            camera_target,
            CVector::new(0.0, -3.0, 3.0),
            &mut cam.topdown_ped_south_adjuster,
            &mut cam.topdown_ped_south_speed,
            time_step,
            &mut building_hit,
        );
        cam.source.y += cam.topdown_ped_south_adjuster;
        avoid_walls_top_down_ped(
            camera_target,
            CVector::new(0.0, 3.0, 3.0),
            &mut cam.topdown_ped_north_adjuster,
            &mut cam.topdown_ped_north_speed,
            time_step,
            &mut building_hit,
        );
        cam.source.y -= cam.topdown_ped_north_adjuster;
        avoid_walls_top_down_ped(
            camera_target,
            CVector::new(3.0, 0.0, 3.0),
            &mut cam.topdown_ped_west_adjuster,
            &mut cam.topdown_ped_west_speed,
            time_step,
            &mut building_hit,
        );
        cam.source.x -= cam.topdown_ped_west_adjuster;
        avoid_walls_top_down_ped(
            camera_target,
            CVector::new(-3.0, 0.0, 3.0),
            &mut cam.topdown_ped_east_adjuster,
            &mut cam.topdown_ped_east_speed,
            time_step,
            &mut building_hit,
        );
        cam.source.x += cam.topdown_ped_east_adjuster;

        let target = CVector::new(
            camera_target.x + cam.topdown_ped_east_adjuster - cam.topdown_ped_west_adjuster,
            cam.source.y + 1.0 + cam.topdown_ped_south_adjuster,
            camera_target.z,
        );
        cam.front = normalize(vector_sub(target, cam.source));
        if cam.front.x == 0.0 && cam.front.y == 0.0 {
            cam.front.y = 0.0001;
        }
        cam.up = normalize(cross(cam.front, CVector::new(-1.0, 0.0, 0.0)));
        cam.fov = 70.0;
        cam.direction_was_looking = LOOKING_FORWARD;
        cam.reset_statics = false;
        self.position = cam.source;
        self.orientation = cam.front;
        self.look_at = target;
        self.cams[self.active_cam] = cam;
        self.script_mode = CameraScriptMode::FollowPlayer;
    }
    pub fn gameplay_orientation(&self) -> f32 {
        self.orientation.x.atan2(self.orientation.y)
    }

    /// re3 CVector::Heading() for the active follow camera Front vector.
    /// This is intentionally not TheCamera.Orientation: the two use
    /// opposite X signs in the original game.
    pub fn follow_ped_heading(&self) -> f32 {
        (-self.orientation.x).atan2(self.orientation.y)
    }

    /// re3 `CCamera::SetMotionBlur`.
    pub fn set_motion_blur(&mut self, r: i32, g: i32, b: i32, a: i32, blur_type: i32) {
        self.blur_red = r;
        self.blur_green = g;
        self.blur_blue = b;
        self.motion_blur = a;
        self.blur_type = blur_type;
    }

    /// re3 CGame::Process frame prologue. The transient ordinary/sniper/
    /// timecycle blur state is cleared before cutscene and SCM processing;
    /// persistent special blur modes are deliberately left alone.
    pub fn reset_motion_blur_for_frame(&mut self) {
        self.motion_blur_add_alpha = 0;
        if matches!(
            self.blur_type,
            MOTION_BLUR_NONE | MOTION_BLUR_SNIPER | MOTION_BLUR_LIGHT_SCENE
        ) {
            self.set_motion_blur(0, 0, 0, 0, MOTION_BLUR_NONE);
        }
    }

    /// re3 `CCamera::CamShake`.
    pub fn cam_shake(&mut self, strength: f32, point: CVector, now_ms: u32) {
        let delta = vector_sub(self.position, point);
        let distance = delta.magnitude().clamp(0.0, 100.0);
        let multiplier = 1.0 - distance / 100.0;
        let current_force = multiplier
            * (self.cam_shake_force - now_ms.wrapping_sub(self.cam_shake_start_ms) as f32 / 1000.0);
        let strength = multiplier * strength;
        if current_force.clamp(0.0, 2.0) < strength {
            self.cam_shake_force = strength;
            self.cam_shake_start_ms = now_ms;
        }
    }

    /// PC helper `CamShakeNoPos`.
    pub fn cam_shake_no_position(&mut self, strength: f32, now_ms: u32) {
        let current_force =
            self.cam_shake_force - now_ms.wrapping_sub(self.cam_shake_start_ms) as f32 / 1000.0;
        if current_force.clamp(0.0, 2.0) < strength {
            self.cam_shake_force = strength;
            self.cam_shake_start_ms = now_ms;
        }
    }

    /// re3 `CCamera::Process` shake offset, applied after the camera basis has
    /// been built so it changes translation without re-aiming the camera.
    pub fn process_shake_offset(&mut self, now_ms: u32, random: u16) -> CVector {
        let strength = (self.cam_shake_force
            - 0.28 * now_ms.wrapping_sub(self.cam_shake_start_ms) as f32 / 1000.0)
            .clamp(0.0, 2.0);
        let offset = strength * 0.1;
        if offset > 0.0 && self.blur_type != 1 {
            self.motion_blur_add_alpha = ((strength * 255.0) as i32 + 25).min(150);
        }
        CVector::new(
            offset * (i32::from(random & 0xF) - 7) as f32,
            offset * (i32::from((random & 0xF0) >> 4) - 7) as f32,
            offset * (i32::from((random & 0xF00) >> 8) - 7) as f32,
        )
    }

    /// re3 `CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString`.
    pub fn set_camera_directly_behind_for_follow_ped(&mut self, player_heading_rad: f32) {
        self.cam_directly_behind = true;
        self.cam_directly_in_front = false;
        let forward_x = -player_heading_rad.sin();
        let forward_y = player_heading_rad.cos();
        self.ped_orient_for_behind_or_in_front = forward_y.atan2(forward_x);
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 `SetCameraDirectlyInFrontForFollowPed_CamOnAString`.
    pub fn set_camera_directly_in_front_for_follow_ped(&mut self, player_heading_rad: f32) {
        self.cam_directly_in_front = true;
        self.cam_directly_behind = false;
        let forward_x = -player_heading_rad.sin();
        let forward_y = player_heading_rad.cos();
        self.ped_orient_for_behind_or_in_front = forward_y.atan2(forward_x);
        self.script_mode = CameraScriptMode::FollowPlayer;
    }

    /// re3 `CDraw::ConvertFOV` / `CDraw::SetFOV` for the build carried by the
    /// reference tree (`ASPECT_RATIO_SCALE` enabled in `core/config.h`).
    /// Cutscenes deliberately keep the authored horizontal FOV unscaled.
    pub fn scaled_horizontal_fov(
        horizontal_fov: f32,
        aspect_ratio: f32,
        cutscene_running: bool,
    ) -> f32 {
        if cutscene_running {
            return horizontal_fov;
        }

        const DEFAULT_ASPECT_RATIO: f32 = 4.0 / 3.0;
        let horizontal = horizontal_fov.to_radians();
        let vertical = 2.0 * ((horizontal * 0.5).tan() / DEFAULT_ASPECT_RATIO).atan();
        (2.0 * ((vertical * 0.5).tan() * aspect_ratio).atan()).to_degrees()
    }

    /// Horizontal FOV used by the actual live viewport and world culling.
    ///
    /// re3's HOR+ conversion is correct for wide viewports: vertical framing
    /// stays at the authored 4:3 value while extra width reveals more world.
    /// For portrait/tall arbitrary window sizes, however, applying the same
    /// conversion in reverse would make the horizontal FOV smaller than the
    /// authored value. Bevy can represent the larger vertical view directly,
    /// so preserve at least the authored horizontal coverage there. This
    /// keeps the 4:3 view as the minimum visible area on every aspect ratio.
    pub fn viewport_horizontal_fov(
        horizontal_fov: f32,
        aspect_ratio: f32,
        _cutscene_running: bool,
    ) -> f32 {
        // `scaled_horizontal_fov` retains re3's original cutscene exception
        // for code that needs exact CDraw parity.  The live Bevy viewport must
        // not inherit that exception, though: a 16:9 (or wider) raster really
        // exposes world to the left/right of the authored 4:3 frame during a
        // fly-by as well.  Using the legacy unscaled cutscene FOV here made
        // projection, ScanWorld and frustum culling fall back to 4:3 coverage
        // precisely while player control was locked for the opening scene.
        let scaled = Self::scaled_horizontal_fov(horizontal_fov, aspect_ratio.max(1.0e-4), false);
        if aspect_ratio < 4.0 / 3.0 {
            scaled.max(horizontal_fov)
        } else {
            scaled
        }
    }

    pub fn viewport_vertical_fov(
        horizontal_fov: f32,
        aspect_ratio: f32,
        cutscene_running: bool,
    ) -> f32 {
        let aspect_ratio = aspect_ratio.max(1.0e-4);
        let horizontal =
            Self::viewport_horizontal_fov(horizontal_fov, aspect_ratio, cutscene_running)
                .to_radians();
        2.0 * ((horizontal * 0.5).tan() / aspect_ratio).atan()
    }

    fn view_basis(&self) -> CameraViewBasis {
        let cam = self.cams.get(self.active_cam);
        let mut front = cam.map_or(self.orientation, |cam| cam.front);
        let mut up = cam.map_or(CVector::new(0.0, 0.0, 1.0), |cam| cam.up);
        // Fixed/point-at-point presentation uses the materialized camera pose,
        // while CCam can still contain the previous gameplay target's basis.
        // Match follow_script_player_camera's actual look-at transform.
        if matches!(self.script_mode, CameraScriptMode::Fixed | CameraScriptMode::ScriptTarget) {
            front = self.look_at - self.position;
            up = CVector::new(0.0, 0.0, 1.0);
        }
        normalize_camera_vector(&mut front, CVector::new(0.0, 1.0, 0.0));
        normalize_camera_vector(&mut up, CVector::new(0.0, 0.0, 1.0));
        let mut right = CVector::new(
            front.y * up.z - front.z * up.y,
            front.z * up.x - front.x * up.z,
            front.x * up.y - front.y * up.x,
        );
        normalize_camera_vector(&mut right, CVector::new(1.0, 0.0, 0.0));
        // Re-orthogonalise up exactly as the camera matrix construction does.
        up = CVector::new(
            right.y * front.z - right.z * front.y,
            right.z * front.x - right.x * front.z,
            right.x * front.y - right.y * front.x,
        );

        CameraViewBasis { right, front, up }
    }

    fn camera_source(&self) -> CVector {
        if matches!(self.script_mode, CameraScriptMode::Fixed | CameraScriptMode::ScriptTarget) {
            return self.position;
        }
        self.cams
            .get(self.active_cam)
            .map_or(self.position, |cam| cam.source)
    }

    pub(crate) fn camera_space_point_from_source(
        &self,
        point: CVector,
        source: CVector,
    ) -> CVector {
        let CameraViewBasis { right, front, up } = self.view_basis();

        let delta = CVector::new(point.x - source.x, point.y - source.y, point.z - source.z);
        CVector::new(
            dot_camera_vector(delta, right),
            dot_camera_vector(delta, front),
            dot_camera_vector(delta, up),
        )
    }

    /// re3 `CCamera::IsSphereVisible` with the exact horizontal FOV already
    /// selected by `CDraw::SetFOV`.
    ///
    /// `CalculateDerivedValues` divides both vertical plane coefficients by
    /// aspect ratio without normalising them afterwards. Keeping that unusual
    /// scale is important because the sphere radius on the right hand side is
    /// not divided; replacing it with a conventional vertical-FOV plane culls
    /// nearby roads/buildings too aggressively at the top/bottom of the view.
    pub fn is_sphere_visible_with_horizontal_fov(
        &self,
        center: CVector,
        radius: f32,
        horizontal_fov: f32,
    ) -> bool {
        self.is_sphere_visible_from_source_with_horizontal_fov(
            center,
            radius,
            self.camera_source(),
            horizontal_fov,
        )
    }
}

fn apply_vehicle_look_direction<F>(
    cam: &mut CCam,
    vehicle_position: CVector,
    vehicle_forward: CVector,
    look_direction: i32,
    static_hit: &mut F,
) where
    F: FnMut(CVector, CVector) -> Option<CVector>,
{
    use crate::core::Cam::{LOOKING_BEHIND, LOOKING_FORWARD, LOOKING_LEFT, LOOKING_RIGHT};

    if look_direction == LOOKING_FORWARD {
        cam.direction_was_looking = LOOKING_FORWARD;
        return;
    }
    let forward = normalize(vehicle_forward);
    if forward.magnitude() <= f32::EPSILON {
        cam.direction_was_looking = LOOKING_FORWARD;
        return;
    }
    let target_orientation = forward.y.atan2(forward.x);
    let distance = cam.ca_max_distance;
    let previous_direction = cam.direction_was_looking;
    let source_angle = match look_direction {
        LOOKING_BEHIND => {
            let delta_beta = wrap_pi(target_orientation - cam.beta);
            if previous_direction != LOOKING_BEHIND {
                cam.look_behind_cam_was_in_front = delta_beta.abs() >= std::f32::consts::FRAC_PI_2;
            }
            target_orientation
                + if cam.look_behind_cam_was_in_front {
                    std::f32::consts::PI
                } else {
                    0.0
                }
        }
        LOOKING_LEFT => target_orientation - std::f32::consts::FRAC_PI_2,
        LOOKING_RIGHT => target_orientation + std::f32::consts::FRAC_PI_2,
        _ => {
            cam.direction_was_looking = LOOKING_FORWARD;
            return;
        }
    };

    cam.source.x = distance * source_angle.cos() + vehicle_position.x;
    cam.source.y = distance * source_angle.sin() + vehicle_position.y;
    cam.source.z -= 1.0;
    if let Some(hit) = static_hit(vehicle_position, cam.source) {
        cam.source = hit;
    }
    cam.source.z += 1.0;
    cam.front = vector_sub(vehicle_position, cam.source);
    if matches!(look_direction, LOOKING_LEFT | LOOKING_RIGHT) {
        cam.front.z += 1.1;
    }
    cam.front = normalize(cam.front);
    cam.up = camera_up(cam.front);
    cam.direction_was_looking = look_direction;
}

/// Precomputed camera basis and frustum plane normals.
///
/// In re3 C++ (`CCamera::CalculateDerivedValues`), the camera matrix and
/// `m_vecFrustumNormals` are precomputed once per frame. This struct caches
/// the orthogonal basis and trig/aspect factors so visibility checks against
/// thousands of world entities do not repeat vector normalizations, cross
/// products, or trigonometric calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFrustum {
    pub source: CVector,
    pub right: CVector,
    pub front: CVector,
    pub up: CVector,
    pub near_clip: f32,
    pub far_clip: f32,
    pub cos_h: f32,
    pub sin_h: f32,
    pub cos_v: f32,
    pub sin_v: f32,
}

impl CameraFrustum {
    #[inline]
    pub fn camera_space_point(&self, point: CVector) -> CVector {
        let delta = CVector::new(
            point.x - self.source.x,
            point.y - self.source.y,
            point.z - self.source.z,
        );
        CVector::new(
            dot_camera_vector(delta, self.right),
            dot_camera_vector(delta, self.front),
            dot_camera_vector(delta, self.up),
        )
    }

    #[inline]
    pub fn is_sphere_visible(&self, center: CVector, radius: f32) -> bool {
        let c = self.camera_space_point(center);
        if c.y + radius < self.near_clip || c.y - radius > self.far_clip {
            return false;
        }

        if c.x * self.cos_h - c.y * self.sin_h > radius
            || -c.x * self.cos_h - c.y * self.sin_h > radius
        {
            return false;
        }

        if -c.y * self.sin_v - c.z * self.cos_v > radius
            || -c.y * self.sin_v + c.z * self.cos_v > radius
        {
            return false;
        }
        true
    }

    #[inline]
    pub fn is_point_visible(&self, point: CVector) -> bool {
        let c = self.camera_space_point(point);
        if c.y < self.near_clip || c.y > self.far_clip {
            return false;
        }

        if c.x * self.cos_h - c.y * self.sin_h > 0.0 || -c.x * self.cos_h - c.y * self.sin_h > 0.0 {
            return false;
        }

        -c.y * self.sin_v - c.z * self.cos_v <= 0.0 && -c.y * self.sin_v + c.z * self.cos_v <= 0.0
    }

    #[inline]
    pub fn is_box_visible(&self, corners: &[CVector; 8]) -> bool {
        let mut outside = [0_u8; 4];
        for point in corners {
            let c = self.camera_space_point(*point);
            if c.y < self.near_clip {
                outside[0] += 1;
            }
            if c.y > self.far_clip {
                outside[1] += 1;
            }
            if c.x * self.cos_h - c.y * self.sin_h > 0.0 {
                outside[2] += 1;
            }
            if -c.x * self.cos_h - c.y * self.sin_h > 0.0 {
                outside[3] += 1;
            }
        }
        outside.into_iter().all(|count| count != 8)
    }
}

impl CCamera {
    /// Precomputes the camera frustum basis and plane factors for a given
    /// camera source position and horizontal field of view.
    pub fn frustum_for_source_and_fov(
        &self,
        camera_source: CVector,
        horizontal_fov: f32,
    ) -> CameraFrustum {
        let CameraViewBasis { right, front, up } = self.view_basis();

        let half_fov = 0.5 * horizontal_fov.to_radians();
        let (sin_h, cos_h) = half_fov.sin_cos();

        let aspect = self.aspect_ratio;
        let (sin_v, cos_v) = if aspect >= 4.0 / 3.0 {
            (sin_h / aspect, cos_h / aspect)
        } else {
            let v_half = ((half_fov.tan() / aspect.max(1e-4)).atan()).max(half_fov);
            v_half.sin_cos()
        };

        CameraFrustum {
            source: camera_source,
            right,
            front,
            up,
            near_clip: self.near_clip_script,
            far_clip: self.far_clip,
            cos_h,
            sin_h,
            cos_v,
            sin_v,
        }
    }

    /// Same re3 sphere test with the already-shaken RenderWare camera source.
    /// `CCamera::Process` applies shake to `GetMatrix().GetPosition()` before
    /// `CalculateDerivedValues`, so the renderer must not fall back to the
    /// unshaken `CCam::Source` for this production path.
    pub fn is_sphere_visible_from_source_with_horizontal_fov(
        &self,
        center: CVector,
        radius: f32,
        camera_source: CVector,
        horizontal_fov: f32,
    ) -> bool {
        self.frustum_for_source_and_fov(camera_source, horizontal_fov)
            .is_sphere_visible(center, radius)
    }

    /// re3 `CCamera::IsPointVisible`, sharing the same derived frustum planes
    /// as `IsSphereVisible`.
    pub fn is_point_visible_with_horizontal_fov(
        &self,
        point: CVector,
        horizontal_fov: f32,
    ) -> bool {
        self.is_point_visible_from_source_with_horizontal_fov(
            point,
            self.camera_source(),
            horizontal_fov,
        )
    }

    pub fn is_point_visible_from_source_with_horizontal_fov(
        &self,
        point: CVector,
        camera_source: CVector,
        horizontal_fov: f32,
    ) -> bool {
        self.frustum_for_source_and_fov(camera_source, horizontal_fov)
            .is_point_visible(point)
    }

    /// re3 `CCamera::IsBoxVisible`. The original code intentionally does not
    /// test the top/bottom planes here; only near/far and the two horizontal
    /// planes can reject all eight corners.
    pub fn is_box_visible_with_horizontal_fov(
        &self,
        corners: &[CVector; 8],
        horizontal_fov: f32,
    ) -> bool {
        self.is_box_visible_from_source_with_horizontal_fov(
            corners,
            self.camera_source(),
            horizontal_fov,
        )
    }

    pub fn is_box_visible_from_source_with_horizontal_fov(
        &self,
        corners: &[CVector; 8],
        camera_source: CVector,
        horizontal_fov: f32,
    ) -> bool {
        self.frustum_for_source_and_fov(camera_source, horizontal_fov)
            .is_box_visible(corners)
    }

    /// Data-only convenience path. Production rendering supplies the
    /// `CDraw`-scaled FOV explicitly through
    /// `is_sphere_visible_with_horizontal_fov` because cutscene state belongs
    /// to `CCutsceneMgr`, not this resource.
    pub fn is_sphere_visible(&self, center: CVector, radius: f32) -> bool {
        let horizontal_fov = self.cams.get(self.active_cam).map_or(70.0, |cam| cam.fov);
        self.is_sphere_visible_with_horizontal_fov(center, radius, horizontal_fov)
    }

    pub fn update_fade(&mut self, dt_seconds: f32) {
        if !self.fading {
            return;
        }
        self.fade_time_remaining -= dt_seconds;
        let remaining = self.fade_time_remaining.max(0.0);
        let progress = if self.fade_duration > 0.0 {
            (1.0 - remaining / self.fade_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.fade_value = if self.fade_direction == 1 {
            255.0 * (1.0 - progress)
        } else {
            255.0 * progress
        };
        if remaining == 0.0 {
            self.fade_time_remaining = 0.0;
            self.fading = false;
        }
    }

    /// re3 `CCamera::LoadPathSplines`.
    pub fn load_path_splines(&mut self, bytes: &[u8]) -> Result<usize, String> {
        self.path_splines = [[0.0; MAX_CAMERA_SPLINE_PATH_LENGTH]; MAX_NUM_OF_SPLINETYPES];
        self.starting_spline = false;

        let mut spline_index = 0usize;
        let mut value_index = 0usize;
        let mut token = String::with_capacity(32);
        for byte in bytes {
            match *byte {
                0 => break,
                b'+' | b'-' | b'.' | b'0'..=b'9' | b'e' | b'E' => {
                    if token.len() >= 31 {
                        return Err("cutscene camera token exceeds 31 bytes".to_owned());
                    }
                    token.push(*byte as char);
                }
                b',' | b';' => {
                    if spline_index < MAX_NUM_OF_SPLINETYPES
                        && value_index < MAX_CAMERA_SPLINE_PATH_LENGTH
                    {
                        self.path_splines[spline_index][value_index] =
                            token.parse::<f32>().unwrap_or(0.0);
                    }
                    token.clear();
                    if *byte == b';' {
                        spline_index += 1;
                        value_index = 0;
                    } else {
                        value_index += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(spline_index.min(MAX_NUM_OF_SPLINETYPES))
    }

    /// re3 `CCamera::Process` LOD-distance block for the reference build,
    /// where `FIX_BUGS` is enabled in `core/config.h`.
    /// `fov` is RenderWare's horizontal FOV and `aspect_ratio` is the live
    /// camera raster aspect.
    pub fn update_lod_dist_multiplier(
        &mut self,
        fov: f32,
        _aspect_ratio: f32,
        cutscene_running: bool,
        cutscene_uses_lod_multiplier: bool,
        renderer_lod_scale: f32,
    ) {
        if !cutscene_running || cutscene_uses_lod_multiplier {
            self.lod_dist_multiplier = 70.0 / fov;
        } else {
            self.lod_dist_multiplier = 1.0;
        }
        self.generation_dist_multiplier = self.lod_dist_multiplier;
        self.lod_dist_multiplier *= renderer_lod_scale;
    }

    /// Pure form used by render-list construction when camera state is read
    /// through an immutable Bevy resource borrow.
    pub fn calculate_lod_dist_multiplier(
        fov: f32,
        _aspect_ratio: f32,
        cutscene_running: bool,
        cutscene_uses_lod_multiplier: bool,
        renderer_lod_scale: f32,
    ) -> f32 {
        let camera_multiplier = if !cutscene_running || cutscene_uses_lod_multiplier {
            70.0 / fov
        } else {
            1.0
        };
        camera_multiplier * renderer_lod_scale
    }

    /// re3 `CCamera::TakeControlWithSpline`.
    pub fn take_control_with_spline(&mut self) {
        self.script_mode = CameraScriptMode::FlyBy;
        if let Some(cam) = self.cams.get_mut(self.active_cam) {
            cam.mode = crate::core::Cam::eCamMode::MODE_FLYBY;
        }
        self.cutscene_finished = false;
        self.position_along_spline = 0.0;
        self.starting_spline = false;
        self.flyby_elapsed_ms = 0.0;
        self.flyby_finish_time_ms = 0;
    }

    pub fn set_cam_cutscene_offset(&mut self, offset: CVector) {
        self.cutscene_offset = offset;
    }

    pub fn finish_cutscene(&mut self) {
        self.position_along_spline = 1.0;
        self.cutscene_finished = true;
    }

    /// re3 `CCam::Process_FlyBy`.
    pub fn update_cutscene_spline(
        &mut self,
        time_step_non_clipped_ms: f32,
    ) -> Result<CameraSplineSample, String> {
        if self.cutscene_finished {
            return self.sample_cutscene_spline(self.flyby_elapsed_ms);
        }
        if self.starting_spline {
            self.flyby_elapsed_ms += time_step_non_clipped_ms;
        } else {
            self.flyby_elapsed_ms = 0.0;
            self.starting_spline = true;
        }
        let sample = self.sample_cutscene_spline(self.flyby_elapsed_ms)?;
        self.position = sample.source;
        self.look_at = sample.target;
        self.orientation = normalize(vector_sub(sample.target, sample.source));
        self.flyby_up = sample.up;
        self.flyby_fov = sample.fov;
        // re3 `CCam::Process_FlyBy` writes the active CCam Source/Front/Up/FOV,
        // and `CCamera::CalculateDerivedValues` immediately derives the render
        // frustum from that same camera matrix. Keep the active CCam in sync
        // with the spline sample so world visibility is tested in the actual
        // cutscene view instead of the stale pre-cutscene follow camera.
        if let Some(cam) = self.cams.get_mut(self.active_cam) {
            cam.source = sample.source;
            cam.front = self.orientation;
            cam.up = sample.up;
            cam.fov = sample.fov;
        }
        self.flyby_finish_time_ms = sample.finish_time_ms;
        self.position_along_spline = sample.position_along_spline;
        Ok(sample)
    }

    pub fn sample_cutscene_spline(&self, elapsed_ms: f32) -> Result<CameraSplineSample, String> {
        let fov_path = &self.path_splines[0];
        let up_path = &self.path_splines[1];
        let source_path = &self.path_splines[2];
        let target_path = &self.path_splines[3];
        let source_frames = spline_frame_count(source_path)?;
        let finish_index = 10usize
            .checked_mul(source_frames.saturating_sub(1))
            .and_then(|index| index.checked_add(1))
            .ok_or("cutscene source spline finish index overflow")?;
        let finish_seconds = *source_path
            .get(finish_index)
            .ok_or("cutscene source spline finish index out of range")?;
        let finish_time_ms = seconds_to_milliseconds(finish_seconds);
        let time_ms = elapsed_ms.max(0.0) as u32;
        let at_end = time_ms >= finish_time_ms;

        let source_marker = spline_marker(source_path, time_ms, 10, at_end)?;
        let target_marker = spline_marker(target_path, time_ms, 10, at_end)?;
        let up_marker = spline_marker(up_path, time_ms, 4, at_end)?;
        let fov_marker = spline_marker(fov_path, time_ms, 4, at_end)?;
        let source = vector_add(
            sample_vector_spline(source_path, time_ms, source_marker)?,
            self.cutscene_offset,
        );
        let target = vector_add(
            sample_vector_spline(target_path, time_ms, target_marker)?,
            self.cutscene_offset,
        );
        let up_angle = sample_float_spline(up_path, time_ms, up_marker)?.to_radians()
            + std::f32::consts::FRAC_PI_2;
        let preliminary_up = CVector::new(up_angle.cos(), 0.0, up_angle.sin());
        let front = normalize(vector_sub(target, source));
        let left = cross(preliminary_up, front);
        let up = normalize(cross(front, left));
        let fov = sample_float_spline(fov_path, time_ms, fov_marker)?;
        let position_along_spline = if at_end || finish_time_ms == 0 {
            1.0
        } else {
            time_ms as f32 / finish_time_ms as f32
        };
        Ok(CameraSplineSample {
            source,
            target,
            up,
            fov,
            position_along_spline,
            finish_time_ms,
        })
    }
}

fn cam_on_a_string_unobscured(
    cam: &mut CCam,
    target: CVector,
    base_dist: f32,
    vehicle_forward: CVector,
    car_zoom_value_smooth: f32,
) {
    cam.ca_max_distance = base_dist + 0.1 + car_zoom_value_smooth;
    cam.ca_min_distance = (base_dist * 0.6).min(3.5);
    let mut dist = vector_sub(cam.source, target);
    if cam.reset_statics {
        cam.source = vector_add(target, vector_scale(dist, cam.ca_max_distance + 1.0));
    }
    dist = vector_sub(cam.source, target);
    let mut length = magnitude_2d(dist);
    if length < 0.001 {
        let mut forward = vehicle_forward;
        forward.z = 0.0;
        forward = normalize(forward);
        cam.source = vector_sub(target, vector_scale(forward, cam.ca_max_distance));
        dist = vector_sub(cam.source, target);
        length = magnitude_2d(dist);
    }
    if length > cam.ca_max_distance {
        cam.source.x = target.x + dist.x / length * cam.ca_max_distance;
        cam.source.y = target.y + dist.y / length * cam.ca_max_distance;
    } else if length < cam.ca_min_distance && length > f32::EPSILON {
        cam.source.x = target.x + dist.x / length * cam.ca_min_distance;
        cam.source.y = target.y + dist.y / length * cam.ca_min_distance;
    }
}

#[allow(clippy::too_many_arguments)]
fn work_out_cam_height<FGround, FRoof>(
    cam: &mut CCam,
    target: CVector,
    target_orientation: f32,
    target_height: f32,
    vehicle_forward: CVector,
    car_zoom_indicator: i32,
    car_zoom_value_smooth: f32,
    time_step: f32,
    ground_z: &mut FGround,
    roof_z: &mut FRoof,
) where
    FGround: FnMut(f32, f32, f32) -> Option<f32>,
    FRoof: FnMut(f32, f32, f32) -> Option<f32>,
{
    let zoom = car_zoom_value_smooth.max(0.1);
    let alpha_offset = match car_zoom_indicator {
        CAM_ZOOM_1 => zoom.atan2(23.0),
        CAM_ZOOM_2 => zoom.atan2(10.8),
        CAM_ZOOM_3 => zoom.atan2(7.0),
        _ => 0.0,
    };
    let length = magnitude_2d(vector_sub(cam.source, target));
    if cam.collision_checks_on {
        let mut car_alpha = vehicle_forward.z.atan2(magnitude_2d(vehicle_forward));
        car_alpha = wrap_pi(car_alpha);
        cam.beta = wrap_pi(cam.beta);
        let behind_car_ness = wrap_pi(cam.beta - target_orientation).cos();
        car_alpha = -car_alpha * behind_car_ness;
        if car_alpha < -0.01 {
            car_alpha = -0.01;
        }

        let mut delta_alpha = wrap_pi(car_alpha - cam.alpha);
        let angle_limit = 1.8_f32.to_radians();
        if delta_alpha > angle_limit {
            delta_alpha -= angle_limit;
        } else if delta_alpha < -angle_limit {
            delta_alpha += angle_limit;
        } else {
            delta_alpha = 0.0;
        }

        let mut target_alpha = 0.0;
        let car_bottom = target.z - target_height / 2.0;
        let car_roof = roof_z(target.x, target.y, car_bottom);
        let forward = normalize(vehicle_forward);
        let side_angle = forward.y.atan2(forward.x) + std::f32::consts::FRAC_PI_2;
        let side_x = 2.5 * side_angle.cos();
        let side_y = 2.5 * side_angle.sin();
        let roof_side_1 = roof_z(target.x + side_x, target.y + side_y, car_bottom);
        let roof_side_2 = roof_z(target.x - side_x, target.y - side_y, car_bottom);

        let predicted_cam_z =
            target.z + length * (cam.alpha + alpha_offset).sin() + cam.close_in_car_height_offset;
        let cam_ground = ground_z(cam.source.x, cam.source.y, predicted_cam_z);
        let (cam_roof, cam_target_z) = if let Some(cam_ground) = cam_ground {
            (
                roof_z(cam.source.x, cam.source.y, cam_ground + target_height),
                cam_ground + target_height * 1.5 + 0.1,
            )
        } else {
            (None, target.z)
        };

        let mut cam_clear = true;
        if let Some(car_roof) = car_roof
            && cam_roof.is_none()
            && (roof_side_1.is_some() || roof_side_2.is_some())
        {
            target_alpha = (car_roof - cam_target_z - 1.5).atan2(cam.ca_max_distance);
            cam_clear = false;
        }
        if let Some(cam_roof) = cam_roof {
            let roof = car_roof.map_or(cam_roof, |car_roof| cam_roof.min(car_roof));
            target_alpha = (roof - cam_target_z - 1.5).atan2(cam.ca_max_distance);
            cam_clear = false;
        }
        target_alpha = wrap_pi(target_alpha).max((-7.0_f32).to_radians());
        if target_alpha > alpha_offset {
            cam_clear = true;
        }
        cam.previous_near_check_near_clip_small = false;
        let mut top_alpha_speed = 0.15;
        let mut alpha_speed_step = 0.015;
        if !cam_clear {
            cam.previous_near_check_near_clip_small = true;
            delta_alpha = wrap_pi(target_alpha - (cam.alpha + alpha_offset));
            top_alpha_speed = 0.3;
            alpha_speed_step = 0.03;
        }

        let cam_z = target.z
            + length * (cam.alpha + delta_alpha + alpha_offset).sin()
            + cam.close_in_car_height_offset;
        if let Some(cam_ground_2) = ground_z(cam.source.x, cam.source.y, cam_z) {
            if cam_clear && cam_z - cam_ground_2 < 1.5 {
                cam.previous_near_check_near_clip_small = true;
                let dz = cam_ground_2 + 1.5 - target.z;
                let angle = if length == 0.0 || dz == 0.0 {
                    cam.alpha
                } else {
                    dz.atan2(length)
                };
                delta_alpha = wrap_pi(angle) - cam.alpha;
            }
        } else if cam_clear
            && let Some(mut cam_roof_2) = roof_z(cam.source.x, cam.source.y, cam_z)
            && cam_z - cam_roof_2 < 1.5
        {
            cam.previous_near_check_near_clip_small = true;
            cam_roof_2 = cam_roof_2.min(target.z + 3.5);
            let dz = cam_roof_2 + 1.5 - target.z;
            let angle = if length == 0.0 || dz == 0.0 {
                cam.alpha
            } else {
                dz.atan2(length)
            };
            delta_alpha = wrap_pi(angle) - cam.alpha;
        }
        cam.last_target_alpha_with_collision_on = delta_alpha + cam.alpha;
        cam.last_top_alpha_speed = top_alpha_speed;
        cam.last_alpha_speed_step = alpha_speed_step;
    }

    well_buffer(
        cam.last_target_alpha_with_collision_on,
        &mut cam.alpha,
        &mut cam.alpha_speed,
        cam.last_top_alpha_speed,
        cam.last_alpha_speed_step,
        true,
        time_step,
    );
    cam.source.z =
        target.z + (cam.alpha + alpha_offset).sin() * length + cam.close_in_car_height_offset;
}

#[allow(clippy::too_many_arguments)]
fn rot_cam_if_in_front_car(
    cam: &mut CCam,
    target: CVector,
    target_orientation: f32,
    vehicle_forward: CVector,
    vehicle_move_speed: CVector,
    time_step: f32,
    _looking_away: bool,
    transition_state: i32,
    use_transition_beta: bool,
    transition_beta: f32,
    cam_directly_behind: &mut bool,
    cam_directly_in_front: &mut bool,
) -> bool {
    let moving_forward = dot(vehicle_forward, vehicle_move_speed) > 0.02;
    let dist = magnitude_2d(vector_sub(cam.source, target));
    let mut delta_beta = wrap_pi(target_orientation - cam.beta);
    if delta_beta.abs() > 20.0_f32.to_radians() && moving_forward && transition_state == 0 {
        cam.fixing_beta = true;
    }
    if !cam.fixing_beta && !use_transition_beta && !*cam_directly_behind && !*cam_directly_in_front
    {
        return false;
    }
    well_buffer(
        target_orientation,
        &mut cam.beta,
        &mut cam.beta_speed,
        0.15,
        0.007,
        true,
        time_step,
    );
    if *cam_directly_behind {
        cam.beta = target_orientation;
    }
    if *cam_directly_in_front {
        cam.beta = target_orientation + std::f32::consts::PI;
    }
    if use_transition_beta {
        cam.beta = transition_beta;
    }
    cam.source.x = target.x - cam.beta.cos() * dist;
    cam.source.y = target.y - cam.beta.sin() * dist;
    delta_beta = wrap_pi(target_orientation - cam.beta);
    if delta_beta.abs() < 2.0_f32.to_radians() {
        cam.fixing_beta = false;
    }
    *cam_directly_behind = false;
    *cam_directly_in_front = false;
    true
}

fn fix_cam_if_obscured<F>(
    cam: &mut CCam,
    target_coors: CVector,
    entity_position: CVector,
    target_height: f32,
    target_orientation: f32,
    time_step: f32,
    static_hit: &mut F,
) -> bool
where
    F: FnMut(CVector, CVector) -> Option<CVector>,
{
    let mut target = target_coors;
    target.z += target_height / 2.0;
    let mut temp_source = cam.source;
    let mut obscured_1 = false;
    let mut obscured_2 = false;
    let mut fix_1 = false;
    let mut dist_1 = 0.0;
    let mut dist_2 = 0.0;
    let mut col_point = CVector::ZERO;
    if cam.collision_checks_on || cam.last_obscured {
        if let Some(hit) = static_hit(target, temp_source) {
            obscured_1 = true;
            col_point = hit;
            dist_1 = magnitude_2d(vector_sub(target, hit));
            fix_1 = true;
            if let Some(hit) = static_hit(entity_position, temp_source) {
                col_point = hit;
            } else {
                obscured_1 = false;
            }
        } else if cam.fixing_beta {
            let distance = vector_sub(temp_source, target).magnitude();
            temp_source.x = target.x - distance * target_orientation.cos();
            temp_source.y = target.y - distance * target_orientation.sin();
            if let Some(hit) = static_hit(target, temp_source) {
                obscured_2 = true;
                col_point = hit;
                dist_2 = magnitude_2d(vector_sub(target, hit));
                if let Some(hit) = static_hit(entity_position, temp_source) {
                    col_point = hit;
                } else {
                    obscured_2 = false;
                }
            }
        }
        cam.last_obscured = obscured_1 || obscured_2;
    }
    if !cam.last_obscured {
        return false;
    }
    if fix_1 {
        cam.source.x = target.x - cam.beta.cos() * dist_1;
        cam.source.y = target.y - cam.beta.sin() * dist_1;
    } else {
        well_buffer(
            dist_2,
            &mut cam.distance_before_changes,
            &mut cam.distance_speed,
            0.2,
            0.025,
            false,
            time_step,
        );
        cam.source.x = target.x - cam.beta.cos() * cam.distance_before_changes;
        cam.source.y = target.y - cam.beta.sin() * cam.distance_before_changes;
    }
    if cam.reset_statics {
        cam.distance_before_changes = magnitude_2d(vector_sub(cam.source, target));
        cam.distance_speed = 0.0;
        cam.source.x = col_point.x;
        cam.source.y = col_point.y;
    }
    true
}

fn fix_cam_when_obscured_by_vehicle<F>(
    cam: &mut CCam,
    target: CVector,
    time_step: f32,
    vehicle_hit_max_z: &mut F,
) where
    F: FnMut(CVector, CVector) -> Option<f32>,
{
    let mut height_target = 0.0;
    if let Some(vehicle_max_z) = vehicle_hit_max_z(target, cam.source) {
        height_target = vehicle_max_z + 1.0 + target.z - cam.source.z;
        if height_target < 0.0 {
            height_target = 0.0;
        }
    }
    well_buffer(
        height_target,
        &mut cam.height_fixer_cars_obscuring,
        &mut cam.height_fixer_cars_obscuring_speed,
        0.2,
        0.025,
        false,
        time_step,
    );
    cam.source.z += cam.height_fixer_cars_obscuring;
}

fn avoid_walls_top_down_ped<F>(
    target_coors: CVector,
    offset: CVector,
    adjuster: &mut f32,
    adjuster_speed: &mut f32,
    time_step: f32,
    building_hit: &mut F,
) where
    F: FnMut(CVector, CVector) -> Option<CVector>,
{
    let test_point2 = vector_add(target_coors, offset);
    let test_point1 = CVector::new(target_coors.x, target_coors.y, test_point2.z);
    let mut target = 0.0;
    let mut max_speed = 0.13;
    let mut acceleration = 0.015;
    if let Some(hit) = building_hit(test_point1, test_point2) {
        let mut dy = test_point1.y - hit.y;
        if dy > 1.0 {
            dy = 1.0;
        }
        let speed_mult = 1.0 - (dy / 1.0).abs();
        target = 2.5;
        max_speed += speed_mult * 0.3;
        acceleration += speed_mult * 0.03;
    }
    well_buffer(
        target,
        adjuster,
        adjuster_speed,
        max_speed,
        acceleration,
        false,
        time_step,
    );
}
fn well_buffer(
    target: f32,
    current: &mut f32,
    speed: &mut f32,
    max_speed: f32,
    acceleration: f32,
    is_angle: bool,
    time_step: f32,
) {
    let mut delta = target - *current;
    if is_angle {
        delta = wrap_pi(delta);
    }
    let target_speed = delta * max_speed;
    *speed += acceleration * (target_speed - *speed) * time_step;
    if target_speed < 0.0 && *speed < target_speed || target_speed > 0.0 && *speed > target_speed {
        *speed = target_speed;
    }
    *current += *speed * time_step.min(10.0);
}

fn magnitude_2d(vector: CVector) -> f32 {
    (vector.x * vector.x + vector.y * vector.y).sqrt()
}

fn vector_scale(vector: CVector, scale: f32) -> CVector {
    CVector::new(vector.x * scale, vector.y * scale, vector.z * scale)
}

fn dot(left: CVector, right: CVector) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn camera_up(front: CVector) -> CVector {
    let right = normalize(cross(front, CVector::new(0.0, 0.0, 1.0)));
    normalize(cross(right, front))
}

fn spline_frame_count(path: &[f32; MAX_CAMERA_SPLINE_PATH_LENGTH]) -> Result<usize, String> {
    let count = path[0] as usize;
    if count < 2 {
        Err(format!("cutscene spline has {count} frames"))
    } else {
        Ok(count)
    }
}

fn spline_marker(
    path: &[f32; MAX_CAMERA_SPLINE_PATH_LENGTH],
    time_ms: u32,
    stride: usize,
    at_end: bool,
) -> Result<usize, String> {
    let frame_count = spline_frame_count(path)?;
    if at_end {
        return Ok((frame_count - 1) * stride + 1);
    }
    let mut marker = stride + 1;
    while marker < path.len()
        && time_ms >= ((path[marker] - path[1]) * 1000.0).max(0.0).trunc() as u32
    {
        marker += stride;
    }
    if marker >= path.len() {
        return Err("cutscene spline marker out of range".to_owned());
    }
    while marker >= stride && seconds_to_milliseconds(path[marker] - path[marker - stride]) <= 75 {
        marker += stride;
        if (marker - 1) / stride > frame_count {
            marker = (frame_count - 1) * stride + 1;
            break;
        }
    }
    Ok(marker)
}

fn sample_float_spline(
    path: &[f32; MAX_CAMERA_SPLINE_PATH_LENGTH],
    time_ms: u32,
    marker: usize,
) -> Result<f32, String> {
    let start_time = seconds_to_milliseconds(*path.get(marker - 4).ok_or("float spline start")?);
    let end_time = seconds_to_milliseconds(*path.get(marker).ok_or("float spline end")?);
    let a = interpolation_factor(time_ms, start_time, end_time);
    let b = 1.0 - a;
    Ok(b * b * b * path[marker - 3]
        + 3.0 * a * b * b * path[marker - 1]
        + 3.0 * a * a * b * path[marker + 2]
        + a * a * a * path[marker + 1])
}

fn sample_vector_spline(
    path: &[f32; MAX_CAMERA_SPLINE_PATH_LENGTH],
    time_ms: u32,
    marker: usize,
) -> Result<CVector, String> {
    let start_time = seconds_to_milliseconds(*path.get(marker - 10).ok_or("vector spline start")?);
    let end_time = seconds_to_milliseconds(*path.get(marker).ok_or("vector spline end")?);
    let a = interpolation_factor(time_ms, start_time, end_time);
    let b = 1.0 - a;
    let bezier = |start: f32, control_a: f32, control_b: f32, end: f32| {
        b * b * b * start
            + 3.0 * a * b * b * control_a
            + 3.0 * a * a * b * control_b
            + a * a * a * end
    };
    Ok(CVector::new(
        bezier(
            path[marker - 9],
            path[marker - 3],
            path[marker + 4],
            path[marker + 1],
        ),
        bezier(
            path[marker - 8],
            path[marker - 2],
            path[marker + 5],
            path[marker + 2],
        ),
        bezier(
            path[marker - 7],
            path[marker - 1],
            path[marker + 6],
            path[marker + 3],
        ),
    ))
}

fn seconds_to_milliseconds(seconds: f32) -> u32 {
    (seconds * 1000.0).max(0.0).trunc() as u32
}

fn interpolation_factor(time_ms: u32, start_ms: u32, end_ms: u32) -> f32 {
    if end_ms == start_ms {
        return 0.0;
    }
    ((time_ms as f32 - start_ms as f32) / (end_ms as f32 - start_ms as f32)).clamp(0.0, 1.0)
}

fn wrap_pi(mut angle: f32) -> f32 {
    while angle >= std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }
    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }
    angle
}

fn vector_add(left: CVector, right: CVector) -> CVector {
    CVector::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn vector_sub(left: CVector, right: CVector) -> CVector {
    CVector::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn cross(left: CVector, right: CVector) -> CVector {
    CVector::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn normalize(vector: CVector) -> CVector {
    let magnitude = vector.magnitude();
    if magnitude <= f32::EPSILON {
        CVector::ZERO
    } else {
        CVector::new(
            vector.x / magnitude,
            vector.y / magnitude,
            vector.z / magnitude,
        )
    }
}

fn dot_camera_vector(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn normalize_camera_vector(vector: &mut CVector, fallback: CVector) {
    let length_squared = dot_camera_vector(*vector, *vector);
    if length_squared <= f32::EPSILON {
        *vector = fallback;
        return;
    }
    let inverse_length = length_squared.sqrt().recip();
    vector.x *= inverse_length;
    vector.y *= inverse_length;
    vector.z *= inverse_length;
}

pub(crate) fn reset_camera_motion_blur_for_frame(mut camera: Option<ResMut<CCamera>>) {
    if let Some(camera) = camera.as_deref_mut() {
        if camera.camera_just_restored {
            camera.previous_speed_position = None;
        }
        camera.reset_motion_blur_for_frame();
    }
}

fn track_camera_speed(mut camera: Option<ResMut<CCamera>>) {
    let Some(camera) = camera.as_deref_mut() else {
        bevy::log::warn_once!("camera_speed_waiting_canonical_camera");
        return;
    };
    let previous = camera.previous_speed_position.unwrap_or(camera.position);
    camera.speed_so_far += (camera.position - previous).magnitude();
    camera.speed_frames += 1;
    if camera.speed_frames == 4 {
        camera.average_speed = camera.speed_so_far / 4.0;
        camera.speed_so_far = 0.0;
        camera.speed_frames = 0;
    }
    camera.previous_speed_position = Some(camera.position);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_speed_uses_native_four_frame_distance_average() {
        let mut app = App::new();
        app.init_resource::<CCamera>().add_systems(Update, track_camera_speed);
        for x in [0.0, 2.0, 4.0, 6.0] {
            app.world_mut().resource_mut::<CCamera>().position.x = x;
            app.update();
        }
        assert_eq!(app.world().resource::<CCamera>().average_speed, 1.5);
        for x in [8.0, 10.0, 12.0, 14.0] {
            app.world_mut().resource_mut::<CCamera>().position.x = x;
            app.update();
        }
        assert_eq!(app.world().resource::<CCamera>().average_speed, 2.0);
    }

    #[test]
    fn follow_ped_heading_matches_re3_vector_heading_and_free_cam_defaults_off() {
        let mut camera = CCamera::default();
        assert!(!camera.free_cam);

        camera.orientation = CVector::new(-1.0, 0.0, 0.0);
        assert!((camera.follow_ped_heading() - std::f32::consts::FRAC_PI_2).abs() < 1.0e-6);

        camera.orientation = CVector::new(1.0, 0.0, 0.0);
        assert!((camera.follow_ped_heading() + std::f32::consts::FRAC_PI_2).abs() < 1.0e-6);
    }

    fn float_spline(start: f32, control_a: f32, control_b: f32, end: f32) -> [f32; 800] {
        let mut path = [0.0; MAX_CAMERA_SPLINE_PATH_LENGTH];
        path[0] = 2.0;
        path[1] = 0.0;
        path[2] = start;
        path[4] = control_a;
        path[5] = 1.0;
        path[6] = end;
        path[7] = control_b;
        path
    }

    fn vector_spline(
        start: CVector,
        control_a: CVector,
        control_b: CVector,
        end: CVector,
    ) -> [f32; 800] {
        let mut path = [0.0; MAX_CAMERA_SPLINE_PATH_LENGTH];
        path[0] = 2.0;
        path[1] = 0.0;
        path[2..5].copy_from_slice(&[start.x, start.y, start.z]);
        path[8..11].copy_from_slice(&[control_a.x, control_a.y, control_a.z]);
        path[11] = 1.0;
        path[12..15].copy_from_slice(&[end.x, end.y, end.z]);
        path[15..18].copy_from_slice(&[control_b.x, control_b.y, control_b.z]);
        path
    }

    #[test]
    fn scaled_horizontal_fov_matches_re3_aspect_ratio_scale_branch() {
        let four_three = CCamera::scaled_horizontal_fov(70.0, 4.0 / 3.0, false);
        let sixteen_nine = CCamera::scaled_horizontal_fov(70.0, 16.0 / 9.0, false);
        let cutscene = CCamera::scaled_horizontal_fov(70.0, 16.0 / 9.0, true);

        assert!((four_three - 70.0).abs() < 1.0e-5);
        assert!((sixteen_nine - 86.06708).abs() < 1.0e-4);
        assert_eq!(cutscene, 70.0);
    }

    #[test]
    fn visibility_uses_re3_derived_planes_instead_of_a_normalised_vertical_fov() {
        let mut camera = CCamera::default();
        camera.aspect_ratio = 16.0 / 9.0;
        camera.cams[0].source = CVector::ZERO;
        camera.cams[0].front = CVector::new(0.0, 1.0, 0.0);
        camera.cams[0].up = CVector::new(0.0, 0.0, 1.0);
        let fov = CCamera::viewport_horizontal_fov(70.0, camera.aspect_ratio, false);

        // This point is above the actual raster's vertical projection, but re3
        // keeps it inside the culling frustum because CalculateDerivedValues
        // divides both vertical plane coefficients by aspect without
        // re-normalising them.
        assert!(camera.is_point_visible_with_horizontal_fov(CVector::new(0.0, 10.0, 8.0), fov,));
        assert!(!camera.is_point_visible_with_horizontal_fov(CVector::new(0.0, 10.0, 10.0), fov,));
    }

    #[test]
    fn narrow_aspect_preserves_vertical_frustum_visibility() {
        let mut camera = CCamera::default();
        // Tall/portrait aspect ratio (e.g. user resized window to narrow tall strip).
        camera.aspect_ratio = 0.25;
        camera.cams[0].source = CVector::ZERO;
        camera.cams[0].front = CVector::new(0.0, 1.0, 0.0);
        camera.cams[0].up = CVector::new(0.0, 0.0, 1.0);
        let fov = CCamera::viewport_horizontal_fov(70.0, camera.aspect_ratio, false);

        // Point is well within the vertical FOV (approx 21.8 deg, while v_half is ~27.7 deg).
        // Without the aspect < 4/3 fix, this point and sphere were falsely culled because
        // dividing by 0.25 narrowed the culling plane angle to the Hor+ horizontal FOV (7.5 deg)
        // and shrunk the radius tolerance by 4x.
        assert!(camera.is_point_visible_with_horizontal_fov(CVector::new(0.0, 10.0, 4.0), fov,));
        assert!(camera.is_sphere_visible_from_source_with_horizontal_fov(
            CVector::new(0.0, 10.0, 4.0),
            1.0,
            CVector::ZERO,
            fov,
        ));
        // The same tall viewport must keep the authored horizontal coverage;
        // an object 30 degrees off-centre is still on-screen and must not be culled.
        let side_point = CVector::new(5.0, 10.0, 0.0);
        let frustum = camera.frustum_for_source_and_fov(CVector::ZERO, fov);
        assert!(
            frustum.is_point_visible(side_point),
            "fov={fov} camera_space={:?} frustum={frustum:?}",
            frustum.camera_space_point(side_point)
        );
        // A point clearly outside the vertical FOV (>70 deg) must still be culled.
        assert!(!camera.is_point_visible_with_horizontal_fov(CVector::new(0.0, 10.0, 30.0), fov,));
    }

    #[test]
    fn complex_box_visibility_preserves_re3_vertical_plane_omission() {
        let mut camera = CCamera::default();
        camera.cams[0].source = CVector::ZERO;
        camera.cams[0].front = CVector::new(0.0, 1.0, 0.0);
        camera.cams[0].up = CVector::new(0.0, 0.0, 1.0);
        let corners = [
            CVector::new(-1.0, 10.0, 99.0),
            CVector::new(-1.0, 10.0, 101.0),
            CVector::new(1.0, 10.0, 99.0),
            CVector::new(1.0, 10.0, 101.0),
            CVector::new(-1.0, 12.0, 99.0),
            CVector::new(-1.0, 12.0, 101.0),
            CVector::new(1.0, 12.0, 99.0),
            CVector::new(1.0, 12.0, 101.0),
        ];

        assert!(camera.is_box_visible_with_horizontal_fov(&corners, 70.0));
    }

    #[test]
    fn production_visibility_uses_the_shaken_render_camera_source() {
        let mut camera = CCamera::default();
        camera.cams[0].source = CVector::ZERO;
        camera.cams[0].front = CVector::new(0.0, 1.0, 0.0);
        camera.cams[0].up = CVector::new(0.0, 0.0, 1.0);
        let point = CVector::new(0.0, 10.0, 0.0);

        assert!(camera.is_sphere_visible_with_horizontal_fov(point, 0.0, 70.0));
        assert!(!camera.is_sphere_visible_from_source_with_horizontal_fov(
            point,
            0.0,
            CVector::new(20.0, 0.0, 0.0),
            70.0,
        ));
    }

    #[test]
    fn lod_multiplier_uses_reference_fix_bugs_branch() {
        let lod = CCamera::calculate_lod_dist_multiplier(70.0, 16.0 / 9.0, false, false, 1.2);
        assert!((lod - 1.2).abs() < 1.0e-6);
    }

    #[test]
    fn flyby_uses_re3_bezier_timing_and_reaches_the_authored_end_frame() {
        let mut camera = CCamera::default();
        camera.path_splines[0] = float_spline(70.0, 70.0, 70.0, 70.0);
        camera.path_splines[1] = float_spline(0.0, 0.0, 0.0, 0.0);
        camera.path_splines[2] = vector_spline(
            CVector::ZERO,
            CVector::new(10.0 / 3.0, 0.0, 0.0),
            CVector::new(20.0 / 3.0, 0.0, 0.0),
            CVector::new(10.0, 0.0, 0.0),
        );
        camera.path_splines[3] = vector_spline(
            CVector::new(0.0, 10.0, 0.0),
            CVector::new(10.0 / 3.0, 10.0, 0.0),
            CVector::new(20.0 / 3.0, 10.0, 0.0),
            CVector::new(10.0, 10.0, 0.0),
        );
        camera.take_control_with_spline();

        let start = camera.update_cutscene_spline(500.0).unwrap();
        assert_eq!(start.position_along_spline, 0.0);
        assert!(start.source.magnitude() < 1.0e-6);

        let middle = camera.update_cutscene_spline(500.0).unwrap();
        assert!((middle.position_along_spline - 0.5).abs() < 1.0e-6);
        assert!((middle.source.x - 5.0).abs() < 1.0e-5);
        assert!((middle.target.x - 5.0).abs() < 1.0e-5);
        assert!((middle.target.y - 10.0).abs() < 1.0e-5);
        assert!((middle.fov - 70.0).abs() < 1.0e-5);
        assert!((camera.cams[0].source.x - middle.source.x).abs() < 1.0e-5);
        assert!((camera.cams[0].front.y - 1.0).abs() < 1.0e-5);
        assert_eq!(camera.cams[0].up, middle.up);
        assert!((camera.cams[0].fov - middle.fov).abs() < 1.0e-5);

        let end = camera.update_cutscene_spline(500.0).unwrap();
        assert_eq!(end.finish_time_ms, 1_000);
        assert_eq!(end.position_along_spline, 1.0);
        assert!((end.source.x - 10.0).abs() < 1.0e-5);
        assert!((end.target.x - 10.0).abs() < 1.0e-5);
    }

    #[test]
    fn flyby_skips_re3_sub_75ms_keyframe_intervals() {
        let mut float_path = [0.0; MAX_CAMERA_SPLINE_PATH_LENGTH];
        float_path[0] = 3.0;
        float_path[1] = 0.0;
        float_path[5] = 0.05;
        float_path[9] = 1.0;
        assert_eq!(spline_marker(&float_path, 0, 4, false).unwrap(), 9);

        let mut vector_path = [0.0; MAX_CAMERA_SPLINE_PATH_LENGTH];
        vector_path[0] = 3.0;
        vector_path[1] = 0.0;
        vector_path[11] = 0.05;
        vector_path[21] = 1.0;
        assert_eq!(spline_marker(&vector_path, 0, 10, false).unwrap(), 21);
    }

    #[test]
    fn camera_shake_preserves_distance_decay_and_process_nibbles() {
        let mut camera = CCamera::default();
        camera.position = CVector::ZERO;
        camera.cam_shake(1.0, CVector::new(50.0, 0.0, 0.0), 1_000);
        assert!((camera.cam_shake_force - 0.5).abs() < 1.0e-6);
        assert_eq!(camera.cam_shake_start_ms, 1_000);

        camera.cam_shake_no_position(0.6, 1_000);
        let offset = camera.process_shake_offset(1_000, 0x0321);
        assert!((offset.x - -0.36).abs() < 1.0e-6);
        assert!((offset.y - -0.30).abs() < 1.0e-6);
        assert!((offset.z - -0.24).abs() < 1.0e-6);
        assert_eq!(camera.motion_blur_add_alpha, 150);

        camera.cam_shake(2.0, CVector::new(100.0, 0.0, 0.0), 2_000);
        assert!((camera.cam_shake_force - 0.6).abs() < 1.0e-6);
    }

    #[test]
    fn restore_with_jump_cut_invalidates_flyby_follow_state() {
        let mut camera = CCamera::default();
        camera.script_mode = CameraScriptMode::FlyBy;
        camera.follow_initialised = true;
        camera.position_along_spline = 0.75;
        camera.cutscene_finished = true;
        camera.cams[0].mode = crate::core::Cam::eCamMode::MODE_FIXED;

        camera.restore_with_jump_cut();

        assert_eq!(camera.script_mode, CameraScriptMode::FollowPlayer);
        assert!(!camera.follow_initialised);
        assert!(camera.camera_just_restored);
        assert_eq!(camera.position_along_spline, 0.0);
        assert!(!camera.cutscene_finished);
        assert_eq!(
            camera.cams[0].mode,
            crate::core::Cam::eCamMode::MODE_FOLLOWPED
        );
        assert!(camera.cams[0].reset_statics);
    }

    #[test]
    fn spline_control_sets_the_native_flyby_camera_mode() {
        let mut camera = CCamera::default();
        camera.take_control_with_spline();

        assert_eq!(camera.script_mode, CameraScriptMode::FlyBy);
        assert_eq!(
            camera.cams[camera.active_cam].mode,
            crate::core::Cam::eCamMode::MODE_FLYBY
        );
    }

    #[test]
    fn follow_render_transform_uses_active_cam_basis_after_source_clipping() {
        let mut camera = CCamera::default();
        camera.position = CVector::new(10.0, 20.0, 4.0);
        // Deliberately make Target-Source disagree with the analytical CCam
        // front, exactly like follow-camera collision can in re3.
        camera.look_at = CVector::new(20.0, 20.0, 14.0);
        camera.cams[0].front = CVector::new(0.0, 1.0, 0.0);
        camera.cams[0].up = CVector::new(0.0, 0.0, 1.0);

        let transform = follow_player_render_transform(&camera);
        let expected = camera.cams[0].front.to_bevy().normalize();
        let actual = *transform.forward();

        assert!(
            actual.dot(expected) > 0.999_999,
            "render camera must use the same CCam Front basis consumed by CRenderer"
        );
        let target_direction = (camera.look_at.to_bevy() - camera.position.to_bevy()).normalize();
        assert!(
            actual.dot(target_direction) < 0.9,
            "test must keep Target-Source distinct from the active CCam Front"
        );
    }

    #[test]
    fn script_camera_projection_sync_uses_live_raster_aspect() {
        let mut camera = CCamera::default();
        camera.script_mode = CameraScriptMode::Fixed;
        camera.cams[camera.active_cam].fov = 70.0;
        let mut perspective = PerspectiveProjection {
            aspect_ratio: 0.25,
            ..Default::default()
        };

        sync_camera_perspective(Some(&mut camera), &mut perspective, false);

        assert!((camera.aspect_ratio - 0.25).abs() < 1.0e-6);
        assert_eq!(camera.far_clip, 2000.0);
        let expected_vertical = CCamera::viewport_vertical_fov(70.0, 0.25, false);
        assert!((perspective.fov - expected_vertical).abs() < 1.0e-6);
        assert!((CCamera::viewport_horizontal_fov(70.0, 0.25, false) - 70.0).abs() < 1.0e-6);

        // Wide gameplay stays HOR+: the vertical framing is identical to 4:3
        // instead of zooming the character/world as the viewport gets wider.
        perspective.aspect_ratio = 32.0 / 9.0;
        sync_camera_perspective(Some(&mut camera), &mut perspective, false);
        let baseline_vertical = CCamera::viewport_vertical_fov(70.0, 4.0 / 3.0, false);
        assert!((perspective.fov - baseline_vertical).abs() < 1.0e-6);

        camera.script_mode = CameraScriptMode::FlyBy;
        camera.flyby_fov = 55.0;
        perspective.aspect_ratio = 3.0;
        sync_camera_perspective(Some(&mut camera), &mut perspective, true);
        let expected_flyby = CCamera::viewport_vertical_fov(55.0, 4.0 / 3.0, false);
        assert!((camera.aspect_ratio - 3.0).abs() < 1.0e-6);
        assert!((perspective.fov - expected_flyby).abs() < 1.0e-6);
        assert!(
            CCamera::viewport_horizontal_fov(55.0, 3.0, true) > 55.0,
            "wide cutscene viewports must expose and cull the extra side coverage, not the authored 4:3 width"
        );
    }
}


/// Place the active gameplay camera through re3 `Process_FollowPedWithMouse`.
pub fn snap_follow_camera_resource(
    camera: &mut CCamera,
    player_position: rust_gta_3_core::core::Coordinates::CVector,
    heading_degrees: f32,
) {
    camera.process_follow_ped_with_mouse(
        player_position,
        heading_degrees.to_radians(),
        |_target, _source| None,
    );
}





pub(crate) fn focused_player_vehicle_handle(
    players: &CWorldPlayers,
    entities: &ScriptEntityWorld,
) -> Option<i32> {
    players
        .vehicle_relation(players.player_in_focus() as i32)
        .and_then(|relation| entities.vehicle_handle_for_entity(relation.vehicle))
}

/// Presentation only needs the materialized target's canonical pose. It must
/// not borrow SCM mutation buffers or build a full gameplay snapshot.
#[derive(SystemParam)]
pub(crate) struct CameraTargetActors<'w, 's> {
    index: Res<'w, crate::control::ScriptEntities::ScriptEntityIndex>,
    vehicles: crate::control::ScriptEntities::VehicleCoreRead<'w, 's>,
    peds: Query<'w, 's, (
        &'static crate::control::ScriptEntities::ScriptPedIdentity,
        &'static crate::peds::Ped::CPed,
    )>,
}

impl CameraTargetActors<'_, '_> {
    fn position(&self, target: CameraScriptTarget) -> Option<CVector> {
        let handle = match target {
            CameraScriptTarget::Vehicle(ScriptHandle(handle)) => {
                return self.vehicles.get(handle).map(|core| core.physical.entity.get_position());
            }
            CameraScriptTarget::Char(ScriptHandle(handle)) => handle,
        };
        let entity = self.index.ped(handle)?;
        let (identity, ped) = self.peds.get(entity).ok()?;
        (identity.handle == handle).then(|| ped.physical.entity.get_position())
    }
}

#[cfg(test)]
mod camera_target_access_tests {
    use super::*;
    use bevy::{ecs::system::SystemState, prelude::*};
    use crate::{
        control::ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
        peds::{Ped::CPed, PedType::ePedType, PedVitality::PedVitality},
    };

    #[test]
    fn camera_reads_canonical_pose_and_rejects_recycled_handles_with_narrow_access() {
        let mut world = World::new();
        world.init_resource::<ScriptEntityIndex>();
        let mut general = CGeneral::with_seed(19);
        let old_handle = 0x201;
        let new_handle = 0x202;
        let actor = world.spawn((
            ScriptPedIdentity { handle: old_handle, model_id: 0 },
            CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
            PedVitality::default(),
        )).id();
        world.resource_mut::<ScriptEntityIndex>().peds.insert(old_handle, actor);
        let (mut core, vitality, drive, appearance, entry, audio) = crate::vehicles::Vehicle::CVehicle::new(0, 0, &mut general).into_parts();
        let vehicle_position = CVector::new(100.0, 200.0, 300.0);
        core.physical.entity.set_position(vehicle_position);
        let vehicle = world.spawn((
            crate::control::ScriptEntities::ScriptVehicleIdentity {
                handle: old_handle,
                kind: crate::control::ScriptEntities::ScriptVehicleKind::Automobile,
            }, core, vitality, drive, appearance, entry, audio,
        )).id();
        world.resource_mut::<ScriptEntityIndex>().vehicles.insert(old_handle, vehicle);
        // A camera pose read must coexist with an unrelated vitality write.
        // The former all-actor mutable facade could not join this query.
        let mut state = SystemState::<(CameraTargetActors, Query<&mut PedVitality>)>::new(&mut world);
        let expected = CVector::new(7.0, 8.0, 9.0);
        world.get_mut::<CPed>(actor).unwrap().physical.entity.set_position(expected);
        {
            let (targets, mut vitality) = state.get_mut(&mut world).unwrap();
            vitality.get_mut(actor).unwrap().health = 42.0;
            assert_eq!(targets.position(CameraScriptTarget::Char(ScriptHandle(old_handle))), Some(expected));
            assert_eq!(targets.position(CameraScriptTarget::Vehicle(ScriptHandle(old_handle))), Some(vehicle_position));
            assert_eq!(targets.position(CameraScriptTarget::Char(ScriptHandle(new_handle))), None);
        }
        world.despawn(actor);
        let replacement = world.spawn((
            ScriptPedIdentity { handle: new_handle, model_id: 0 },
            CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
            PedVitality::default(),
        )).id();
        {
            let mut index = world.resource_mut::<ScriptEntityIndex>();
            index.peds.remove(&old_handle);
            index.peds.insert(new_handle, replacement);
        }
        let (targets, _) = state.get_mut(&mut world).unwrap();
        assert_eq!(targets.position(CameraScriptTarget::Char(ScriptHandle(old_handle))), None);
        assert_eq!(targets.position(CameraScriptTarget::Vehicle(ScriptHandle(old_handle))), Some(vehicle_position));
        assert_eq!(targets.position(CameraScriptTarget::Char(ScriptHandle(new_handle))), Some(CVector::ZERO));
        // Even a stale index alias cannot bypass the component generation.
        world.resource_mut::<ScriptEntityIndex>().peds.insert(old_handle, replacement);
        let (targets, _) = state.get_mut(&mut world).unwrap();
        assert_eq!(targets.position(CameraScriptTarget::Char(ScriptHandle(old_handle))), None);
    }
}


fn sync_camera_perspective(
    camera: Option<&mut CCamera>,
    perspective: &mut PerspectiveProjection,
    cutscene_running: bool,
) {
    let (near, horizontal_fov) = if let Some(camera) = camera {
        camera.aspect_ratio = perspective.aspect_ratio;
        camera.far_clip = 2000.0;
        let horizontal_fov = if camera.script_mode == CameraScriptMode::FlyBy {
            camera.flyby_fov
        } else {
            camera
                .cams
                .get(camera.active_cam)
                .map_or(70.0, |cam| cam.fov)
        };
        (camera.near_clip_script.max(0.01), horizontal_fov)
    } else {
        (crate::core::Camera::DEFAULT_NEAR, 70.0)
    };
    perspective.near = near;
    perspective.far = 2000.0;
    perspective.fov =
        CCamera::viewport_vertical_fov(horizontal_fov, perspective.aspect_ratio, cutscene_running);
}

pub(crate) fn follow_script_player_camera(
    bridge: Res<ScriptGameplayBridge>,
    script_entities: Option<CameraTargetActors<'_, '_>>,
    world_players: crate::peds::PlayerRead::CWorldPlayersRead,
    mut the_camera: Option<ResMut<CCamera>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut camera_query: Query<
        (
            &crate::renderer::BevyRender::GtaSceneCamera,
            &mut Transform,
            Option<&mut Projection>,
        ),
        With<crate::renderer::BevyRender::GtaSceneCamera>,
    >,
) {
    let Some((_, mut transform, mut projection)) = camera_query
        .iter_mut()
        .find(|(camera, _, _)| camera.local_player == 0)
    else {
        return;
    };
    let cutscene_running = cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.is_running());

    // Keep the CCamera visibility frustum and Bevy raster projection on the
    // same aspect/FOV before any script-camera early return below.
    if let Some(Projection::Perspective(perspective)) = projection.as_deref_mut() {
        sync_camera_perspective(the_camera.as_deref_mut(), perspective, cutscene_running);
    }

    if let Some(camera) = the_camera.as_mut()
        && camera.script_mode == CameraScriptMode::ScriptTarget
        && let Some(target) = camera.script_target
    {
        let target = script_entities
            .as_ref()
            .and_then(|entities| entities.position(target))
            .or_else(|| {
                let CameraScriptTarget::Char(ScriptHandle(handle)) = target else { return None; };
                world_players
                    .iter()
                    .find(|player| player.pool_handle == handle)
                    .map(|player| player.position())
            });
        if let Some(target) = target {
            camera.look_at = target;
        } else {
            bevy::log::warn_once!(
                target: "rust_gta_3::camera_target",
                ?target,
                "required camera target is not materialized or its handle is stale"
            );
        }
    }

    // re3 CCamera::Process consumes one CGeneral random number every frame and
    // applies shake after building the camera basis.
    let shake_offset = the_camera
        .as_mut()
        .map(|camera| {
            camera.process_shake_offset(timer.time_in_milliseconds(), general.get_random_number())
        })
        .unwrap_or(CVector::ZERO)
        .to_bevy();

    // Prefer script fixed camera (re3 SetCamPositionForFixedMode / PointCameraAtPoint).
    if let Some(the_camera) = the_camera.as_ref() {
        if the_camera.script_mode == CameraScriptMode::FlyBy {
            let eye = the_camera.position.to_bevy();
            let look = the_camera.look_at.to_bevy();
            let up = if the_camera.flyby_up.magnitude() > 1e-4 {
                the_camera.flyby_up.to_bevy().normalize()
            } else {
                Vec3::Y
            };
            if (eye - look).length_squared() > 1e-4 {
                *transform = Transform::from_translation(eye).looking_at(look, up);
            } else {
                *transform = Transform::from_translation(eye);
            }
            transform.translation += shake_offset;
            return;
        }
        if matches!(
            the_camera.script_mode,
            CameraScriptMode::Fixed | CameraScriptMode::ScriptTarget
        ) {
            let eye = the_camera.position.to_bevy();
            let look = if the_camera.look_at != rust_gta_3_core::core::Coordinates::CVector::ZERO {
                the_camera.look_at.to_bevy()
            } else {
                eye + the_camera.orientation.to_bevy().normalize_or_zero()
            };
            *transform = Transform::from_translation(eye).looking_at(look, Vec3::Y);
            transform.translation += shake_offset;
            return;
        }
    }

    if !bridge.player_spawned {
        return;
    }
    // `update_player_on_foot_control` has already run the port of
    // `CCam::Process_FollowPedWithMouse`, including re3's 1.7/2.0 distance
    // curve and world line-of-sight clipping. This system is only the RW →
    // Bevy boundary; deriving a second 4.7-unit camera here discarded that
    // result and produced a visibly different gameplay camera.
    if let Some(the_camera) = the_camera
        .as_ref()
        .filter(|camera| camera.script_mode == CameraScriptMode::FollowPlayer)
    {
        *transform = follow_player_render_transform(the_camera);
        transform.translation += shake_offset;
    }
}

/// RenderWare boundary for the normal follow camera.
///
/// C++ copies the active `CCam::Front/Up` basis into the RW camera matrix. It
/// does not recompute Front from `(Target - Source)` after collision has moved
/// Source, so the renderer and visibility scanner must consume the same basis.
fn follow_player_render_transform(camera: &CCamera) -> Transform {
    let eye = camera.position.to_bevy();
    if let Some(cam) = camera.cams.get(camera.active_cam) {
        let front = cam.front.to_bevy().normalize_or_zero();
        let up = cam.up.to_bevy().normalize_or_zero();
        if front.length_squared() > f32::EPSILON && up.length_squared() > f32::EPSILON {
            return Transform::from_translation(eye).looking_to(front, up);
        }
    }
    Transform::from_translation(eye).looking_at(camera.look_at.to_bevy(), Vec3::Y)
}

/// Canonical orthonormal basis for camera projection and frustum queries.
struct CameraViewBasis {
    right: CVector,
    front: CVector,
    up: CVector,
}
