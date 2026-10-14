//! re3 reference: references/re-GTA/src/core/Cam.h
//!
//! Single camera mode state.

use rustgta3_core::core::Coordinates::CVector;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum eCamMode {
    #[default]
    MODE_NONE,
    MODE_FOLLOWPED,
    MODE_FIXED,
    MODE_1STPERSON,
    MODE_FLYBY,
    MODE_CAM_ON_A_STRING,
    MODE_BEHINDCAR,
    MODE_SNIPER,
    MODE_ROCKETLAUNCHER,
    MODE_TOPDOWN,
    MODE_GTACLASSIC,
    MODE_TOP_DOWN_PED,
    MODE_M16_1STPERSON,
}

pub const LOOKING_BEHIND: i32 = 0;
pub const LOOKING_LEFT: i32 = 1;
pub const LOOKING_RIGHT: i32 = 2;
pub const LOOKING_FORWARD: i32 = 3;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CCam {
    pub mode: eCamMode,
    pub source: CVector,
    pub front: CVector,
    pub up: CVector,
    pub fov: f32,
    pub alpha: f32,
    pub beta: f32,
    pub alpha_speed: f32,
    pub beta_speed: f32,
    pub distance_before_changes: f32,
    pub distance_speed: f32,
    pub ca_min_distance: f32,
    pub ca_max_distance: f32,
    pub reset_statics: bool,
    pub collision_checks_on: bool,
    pub collision_check_frame: i32,
    pub collision_check_period: i32,
    pub fixing_beta: bool,
    pub direction_was_looking: i32,
    /// re3 `LookBehindCamWasInFront`; sampled when look-behind begins so the
    /// camera keeps the authored side of the vehicle while the button is held.
    pub look_behind_cam_was_in_front: bool,
    pub close_in_car_height_offset: f32,
    pub last_target_alpha_with_collision_on: f32,
    pub last_top_alpha_speed: f32,
    pub last_alpha_speed_step: f32,
    pub previous_near_check_near_clip_small: bool,
    pub last_obscured: bool,
    pub height_fixer_cars_obscuring: f32,
    pub height_fixer_cars_obscuring_speed: f32,
    /// re3 `CCam::SpeedVar`, smoothed from target vehicle forward speed.
    pub speed_var: f32,
    /// re3 `m_fInitialPlayerOrientation`, used by first-person camera modes.
    pub initial_player_orientation: f32,
    /// `Process_TopDown` height buffer (the C++ implementation stores this as
    /// a function-static; keeping it on the active camera preserves lifecycle
    /// without introducing process-global state).
    pub topdown_height_buffer: f32,
    pub topdown_height_speed: f32,
    /// `Process_TopDownPed` persistent speed/height and wall-avoidance state.
    pub topdown_ped_sample_count: u8,
    pub topdown_ped_average_speed: f32,
    pub topdown_ped_speed_sum: f32,
    pub topdown_ped_height_buffer: f32,
    pub topdown_ped_height_speed: f32,
    pub topdown_ped_south_adjuster: f32,
    pub topdown_ped_south_speed: f32,
    pub topdown_ped_north_adjuster: f32,
    pub topdown_ped_north_speed: f32,
    pub topdown_ped_west_adjuster: f32,
    pub topdown_ped_west_speed: f32,
    pub topdown_ped_east_adjuster: f32,
    pub topdown_ped_east_speed: f32,
    /// re3 `Process_1stPerson` upside-down vehicle source correction.
    pub first_person_world_fixer: f32,
    pub weapon_target_fov: f32,
    pub weapon_fov_speed: f32,
}

impl Default for CCam {
    fn default() -> Self {
        Self {
            mode: eCamMode::MODE_NONE,
            source: CVector::ZERO,
            front: CVector::new(0.0, 1.0, 0.0),
            up: CVector::new(0.0, 0.0, 1.0),
            fov: 70.0,
            alpha: 0.0,
            beta: 0.0,
            alpha_speed: 0.0,
            beta_speed: 0.0,
            distance_before_changes: 30.0,
            distance_speed: 0.0,
            ca_min_distance: 0.0,
            ca_max_distance: 0.0,
            reset_statics: true,
            collision_checks_on: false,
            collision_check_frame: 0,
            collision_check_period: 9,
            fixing_beta: false,
            direction_was_looking: LOOKING_FORWARD,
            look_behind_cam_was_in_front: false,
            close_in_car_height_offset: 0.0,
            last_target_alpha_with_collision_on: 0.0,
            last_top_alpha_speed: 0.0,
            last_alpha_speed_step: 0.0,
            previous_near_check_near_clip_small: false,
            last_obscured: false,
            height_fixer_cars_obscuring: 0.0,
            height_fixer_cars_obscuring_speed: 0.0,
            speed_var: 0.0,
            initial_player_orientation: 0.0,
            topdown_height_buffer: 0.0,
            topdown_height_speed: 0.0,
            topdown_ped_sample_count: 0,
            topdown_ped_average_speed: 0.0,
            topdown_ped_speed_sum: 0.0,
            topdown_ped_height_buffer: 0.0,
            topdown_ped_height_speed: 0.0,
            topdown_ped_south_adjuster: 0.0,
            topdown_ped_south_speed: 0.0,
            topdown_ped_north_adjuster: 0.0,
            topdown_ped_north_speed: 0.0,
            topdown_ped_west_adjuster: 0.0,
            topdown_ped_west_speed: 0.0,
            topdown_ped_east_adjuster: 0.0,
            topdown_ped_east_speed: 0.0,
            first_person_world_fixer: 0.0,
            weapon_target_fov: 70.0,
            weapon_fov_speed: 0.0,
        }
    }
}
