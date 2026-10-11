//! re3 reference: references/re-GTA/src/renderer/Weather.cpp
//! re3 reference: references/re-GTA/src/renderer/Weather.h
//!
//! `CWeather::Init` and `CWeather::Update` state.

use bevy::{ecs::system::SystemParam, prelude::*};
use re3bevy_core::core::{
    Clock::CClock, Coordinates::CVector, GameState::CGameState, General::CGeneral, Timer::CTimer,
};

use crate::{
    animation::CutsceneMgr::CCutsceneMgr,
    audio::{DMAudio::cDMAudio, soundlist::SOUND_LIGHTNING},
    core::{
        Camera::CCamera, FileLoader::CFileLoader, Frontend::CMenuManager, Pad::CPads, Stats::CStats,
    },
    peds::{
        PlayerPed::{CWorldPlayerState, PlayerPedState},
        VehicleRelations::InVehicle,
    },
    renderer::{
        BevyRender::GtaSceneCamera,
        Particle::{CParticle, ParticleEmitter},
        ParticleMgr::cParticleSystemMgr,
        RenderBuffer::RwIm3DVertexIntent,
    },
    vehicles::{
        HandlingMgr::cHandlingDataMgr,
        Vehicle::{VehicleAppearance, VehicleCore},
    },
};

pub const WEATHER_RANDOM: i16 = -1;
pub const WEATHER_SUNNY: i16 = 0;
pub const WEATHER_CLOUDY: i16 = 1;
pub const WEATHER_RAINY: i16 = 2;
pub const WEATHER_FOGGY: i16 = 3;
pub const WEATHER_TOTAL: i16 = 4;

pub const WEATHER_TYPES_LIST: [i16; 64] = [
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_CLOUDY,
    WEATHER_CLOUDY,
    WEATHER_RAINY,
    WEATHER_RAINY,
    WEATHER_CLOUDY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_CLOUDY,
    WEATHER_FOGGY,
    WEATHER_FOGGY,
    WEATHER_CLOUDY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_CLOUDY,
    WEATHER_CLOUDY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_CLOUDY,
    WEATHER_CLOUDY,
    WEATHER_RAINY,
    WEATHER_RAINY,
    WEATHER_CLOUDY,
    WEATHER_RAINY,
    WEATHER_CLOUDY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_FOGGY,
    WEATHER_FOGGY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_SUNNY,
    WEATHER_RAINY,
    WEATHER_CLOUDY,
];

pub const WINDYNESS: [f32; WEATHER_TOTAL as usize] = [0.0, 0.7, 1.0, 0.5];
pub const MIN_TIME_BETWEEN_LIGHTNING_FLASH_CHANGES: u32 = 50;
pub const RAIN_CHANGE_SPEED: f32 = 0.003;
const SPLASH_CHECK_RADIUS: f32 = 7.0;
const SPLASH_OFFSET_RADIUS: f32 = 2.0;
const NUM_RAIN_STREAKS: usize = 35;
const STREAK_U: f32 = 10.0;
const STREAK_V: f32 = 18.0;
const STREAK_MIN_DISTANCE: f32 = 8.0;
const STREAK_MAX_DISTANCE: f32 = 16.0;
const STREAK_LIFETIME: f32 = 4.0;
const STREAK_INTERPOLATION_TIME: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RainStreak {
    position: CVector,
    direction: CVector,
    timer: u32,
}

impl Default for RainStreak {
    fn default() -> Self {
        Self {
            position: CVector::ZERO,
            direction: CVector::ZERO,
            timer: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WeatherRenderBatch {
    pub(crate) texture_name: &'static str,
    pub(crate) vertices: Vec<RwIm3DVertexIntent>,
    pub(crate) indices: Vec<u16>,
}

#[derive(Debug, Clone, Resource)]
pub struct CWeather {
    pub sound_handle: i32,
    pub weather_type_in_list: i32,
    pub old_weather_type: i16,
    pub new_weather_type: i16,
    pub forced_weather_type: i16,
    pub lightning_flash: bool,
    pub lightning_burst: bool,
    pub lightning_start: u32,
    pub lightning_flash_last_change: u32,
    pub when_to_play_lightning_sound: u32,
    pub lightning_duration: u32,
    pub foggyness: f32,
    pub cloud_coverage: f32,
    pub wind: f32,
    pub rain: f32,
    pub interpolation_value: f32,
    pub wet_roads: f32,
    pub rainbow: f32,
    pub scripts_force_rain: bool,
    pub stored_state_stored: bool,
    pub stored_interpolation_value: f32,
    pub stored_old_weather_type: i16,
    pub stored_new_weather_type: i16,
    pub stored_rain: f32,
    pub(crate) rain_streaks: [RainStreak; NUM_RAIN_STREAKS],
    pub(crate) rain_render_batches: Vec<WeatherRenderBatch>,
    rain_streak_random_tex: f32,
    rain_streak_random_tex_x: f32,
    rain_streak_random_tex_y: f32,
    rain_streak_previous_camera_position: CVector,
    rain_streak_previous_camera_valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherUpdateContext {
    pub minute: u8,
    pub hour: u8,
    pub time_ms: u32,
    pub frame_counter: u32,
    pub time_step: f32,
    pub right_shock_just_down: bool,
    pub cam_no_rain: bool,
    pub player_no_rain: bool,
    pub looking_lrb_first_person: bool,
    pub player_vehicle_has_roof: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherUpdateResult {
    pub lightning_sound: Option<LightningSoundIntent>,
    pub pad_shake: Option<PadShakeIntent>,
    pub rain_intent: Option<RainIntent>,
    pub remove_2d_raindrops: bool,
    pub mm_rain_delta: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightningSoundIntent {
    pub sound_handle: i32,
    pub sound: i16,
    pub duration: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadShakeIntent {
    pub duration: u32,
    pub frequency: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RainIntent {
    pub rain: f32,
    pub current_frame: i32,
    pub num_drops: i32,
    pub num_splash_attempts: i32,
    pub num_splashes: i32,
}

impl CWeather {
    pub fn init() -> Self {
        Self {
            sound_handle: -1,
            weather_type_in_list: 0,
            old_weather_type: WEATHER_CLOUDY,
            new_weather_type: WEATHER_SUNNY,
            forced_weather_type: WEATHER_RANDOM,
            lightning_flash: false,
            lightning_burst: false,
            lightning_start: 0,
            lightning_flash_last_change: 0,
            when_to_play_lightning_sound: 0,
            lightning_duration: 0,
            foggyness: 0.0,
            cloud_coverage: 0.0,
            wind: 0.0,
            rain: 0.0,
            interpolation_value: 0.0,
            wet_roads: 0.0,
            rainbow: 0.0,
            scripts_force_rain: false,
            stored_state_stored: false,
            stored_interpolation_value: 0.0,
            stored_old_weather_type: 0,
            stored_new_weather_type: 0,
            stored_rain: 0.0,
            rain_streaks: [RainStreak::default(); NUM_RAIN_STREAKS],
            rain_render_batches: Vec::new(),
            rain_streak_random_tex: 0.0,
            rain_streak_random_tex_x: 0.0,
            rain_streak_random_tex_y: 0.0,
            rain_streak_previous_camera_position: CVector::ZERO,
            rain_streak_previous_camera_valid: false,
        }
    }

    /// re3 `CWeather::Init` restart-time assignments. The caller recreates
    /// the weather audio entity and supplies its fresh handle.
    pub fn reset_for_restart(&mut self, sound_handle: i32) {
        self.new_weather_type = WEATHER_SUNNY;
        self.scripts_force_rain = false;
        self.old_weather_type = WEATHER_CLOUDY;
        self.stored_state_stored = false;
        self.interpolation_value = 0.0;
        self.when_to_play_lightning_sound = 0;
        self.weather_type_in_list = 0;
        self.forced_weather_type = WEATHER_RANDOM;
        self.sound_handle = sound_handle;
    }

    pub fn update(
        &mut self,
        context: WeatherUpdateContext,
        general: &mut CGeneral,
    ) -> WeatherUpdateResult {
        let mut mm_rain_delta = 0;
        let f_new_interpolation = context.minute as f32 / 60.0;
        if f_new_interpolation < self.interpolation_value {
            self.old_weather_type = self.new_weather_type;
            if self.forced_weather_type >= 0 {
                self.new_weather_type = self.forced_weather_type;
            } else {
                self.weather_type_in_list =
                    (self.weather_type_in_list + 1) % WEATHER_TYPES_LIST.len() as i32;
                self.new_weather_type = WEATHER_TYPES_LIST[self.weather_type_in_list as usize];
                if self.new_weather_type == WEATHER_RAINY {
                    mm_rain_delta = general.get_random_number() & 7;
                }
            }
        }
        self.interpolation_value = f_new_interpolation;

        if context.right_shock_just_down {
            self.new_weather_type = (self.new_weather_type + 1) % WEATHER_TOTAL;
            self.old_weather_type = self.new_weather_type;
        }

        self.update_lightning(context, general);
        let (lightning_sound, pad_shake) = self.consume_lightning_sound(context);

        self.update_wet_roads();
        self.update_rain(context);
        self.update_clouds_fog_rainbow_wind(context.hour);
        let (rain_intent, remove_2d_raindrops) = self.add_rain_intent(context);

        WeatherUpdateResult {
            lightning_sound,
            pad_shake,
            rain_intent,
            remove_2d_raindrops,
            mm_rain_delta,
        }
    }

    pub fn force_weather(&mut self, weather: i16) {
        self.forced_weather_type = weather;
    }

    pub fn force_weather_now(&mut self, weather: i16) {
        self.old_weather_type = weather;
        self.new_weather_type = weather;
        self.forced_weather_type = weather;
    }

    pub fn release_weather(&mut self) {
        self.forced_weather_type = WEATHER_RANDOM;
    }

    pub fn store_weather_state(&mut self) {
        self.stored_state_stored = true;
        self.stored_interpolation_value = self.interpolation_value;
        self.stored_old_weather_type = self.old_weather_type;
        self.stored_new_weather_type = self.new_weather_type;
        self.stored_rain = self.rain;
    }

    pub fn restore_weather_state(&mut self) {
        // Default GTA III PC restores the four saved fields unconditionally.
        // Only the optional FIX_BUGS path clears Stored_StateStored.
        self.interpolation_value = self.stored_interpolation_value;
        self.old_weather_type = self.stored_old_weather_type;
        self.new_weather_type = self.stored_new_weather_type;
        self.rain = self.stored_rain;
    }

    /// re3 `CWeather::RenderRainStreaks`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_rain_streaks(
        &mut self,
        time_ms: u32,
        code_paused: bool,
        timer_paused: bool,
        fog_reduction: i32,
        camera: &CCamera,
        player_speed: CVector,
        cutscene_running: bool,
        general: &mut CGeneral,
    ) {
        self.rain_render_batches.clear();
        if code_paused {
            return;
        }

        let previous_camera_position = if self.rain_streak_previous_camera_valid {
            self.rain_streak_previous_camera_position
        } else {
            camera.position
        };
        self.rain_streak_previous_camera_position = camera.position;
        self.rain_streak_previous_camera_valid = true;

        let base_intensity =
            ((64 - fog_reduction) as f32 / 64.0 * (255.0 * self.rain) as i32 as f32) as i32;
        if base_intensity == 0 {
            return;
        }
        let Some(active) = camera.cams.get(camera.active_cam).copied() else {
            return;
        };

        let mut vertices = Vec::with_capacity(NUM_RAIN_STREAKS * 5);
        let mut indices = Vec::with_capacity(NUM_RAIN_STREAKS * 12);
        let mut random_tex = self.rain_streak_random_tex;
        let mut random_tex_x = self.rain_streak_random_tex_x;
        let mut random_tex_y = self.rain_streak_random_tex_y;

        for streak in &mut self.rain_streaks {
            if streak.timer != 0 {
                let seconds_elapsed = time_ms.wrapping_sub(streak.timer) as f32 / 1024.0;
                if seconds_elapsed > STREAK_LIFETIME {
                    streak.timer = 0;
                    continue;
                }

                let intensity = if seconds_elapsed < STREAK_INTERPOLATION_TIME {
                    (base_intensity as f32 * 0.5 * seconds_elapsed / STREAK_INTERPOLATION_TIME)
                        as i32
                } else if seconds_elapsed > STREAK_LIFETIME - STREAK_INTERPOLATION_TIME {
                    ((STREAK_LIFETIME - seconds_elapsed) * 0.5 * base_intensity as f32
                        / STREAK_INTERPOLATION_TIME) as i32
                } else {
                    (base_intensity as f32 * 0.5) as i32
                };
                let pos = CVector::new(
                    streak.position.x + seconds_elapsed * streak.direction.x,
                    streak.position.y + seconds_elapsed * streak.direction.y,
                    streak.position.z + seconds_elapsed * streak.direction.z,
                );
                let distance = vector_distance(pos, camera.position);
                render_one_rain_streak(
                    &mut vertices,
                    &mut indices,
                    pos,
                    active.up,
                    active.front,
                    intensity,
                    distance,
                    timer_paused,
                    general,
                    &mut random_tex,
                    &mut random_tex_x,
                    &mut random_tex_y,
                );

                // Default re3 consumes two otherwise-unused random numbers in
                // the middle of a streak's lifetime.
                if seconds_elapsed > 1.0 && seconds_elapsed < STREAK_LIFETIME - 1.0 {
                    let _ = general.get_random_number();
                    let _ = general.get_random_number();
                }
            } else if general.get_random_number() & 0xF00 == 0 {
                streak.direction = CVector::new(4.0, 4.0, -4.0);
                streak.position = CVector::new(
                    camera.position.x + 6.0 * active.front.x - 1.8 * streak.direction.x,
                    camera.position.y + 6.0 * active.front.y - 1.8 * streak.direction.y,
                    camera.position.z + 6.0 * active.front.z + 8.0,
                );
                if cutscene_running {
                    streak.position.x += (camera.position.x - previous_camera_position.x) * 20.0;
                    streak.position.y += (camera.position.y - previous_camera_position.y) * 20.0;
                    streak.position.z += (camera.position.z - previous_camera_position.z) * 20.0;
                } else {
                    streak.position.x += 120.0 * player_speed.x;
                    streak.position.y += 120.0 * player_speed.y;
                }
                streak.position.x +=
                    (i32::from(general.get_random_number() & 255) - 128) as f32 * 0.08;
                streak.position.y +=
                    (i32::from(general.get_random_number() & 255) - 128) as f32 * 0.08;
                streak.timer = time_ms;
            }
        }

        self.rain_streak_random_tex = random_tex;
        self.rain_streak_random_tex_x = random_tex_x;
        self.rain_streak_random_tex_y = random_tex_y;
        if !indices.is_empty() {
            self.rain_render_batches.push(WeatherRenderBatch {
                texture_name: "raindrop4",
                vertices,
                indices,
            });
        }
    }

    fn update_lightning(&mut self, context: WeatherUpdateContext, general: &mut CGeneral) {
        if self.new_weather_type != WEATHER_RAINY || self.old_weather_type != WEATHER_RAINY {
            self.lightning_flash = false;
            self.lightning_burst = false;
            return;
        }

        if self.lightning_burst {
            if (general.get_random_number() & 255) >= 32 {
                if context
                    .time_ms
                    .wrapping_sub(self.lightning_flash_last_change)
                    > MIN_TIME_BETWEEN_LIGHTNING_FLASH_CHANGES
                {
                    let old_lightning_flash = self.lightning_flash;
                    self.lightning_flash = general.get_random_true_false();
                    if self.lightning_flash != old_lightning_flash {
                        self.lightning_flash_last_change = context.time_ms;
                    }
                }
            } else {
                self.lightning_burst = false;
                self.lightning_duration = context
                    .frame_counter
                    .wrapping_sub(self.lightning_start)
                    .min(20);
                self.lightning_flash = false;
                self.when_to_play_lightning_sound =
                    context.time_ms + 150 * (20 - self.lightning_duration);
            }
        } else if general.get_random_number() >= 200 {
            self.lightning_flash = false;
        } else {
            self.lightning_burst = true;
            self.lightning_start = context.frame_counter;
            self.lightning_flash_last_change = context.time_ms;
            self.lightning_flash = true;
        }
    }

    fn consume_lightning_sound(
        &mut self,
        context: WeatherUpdateContext,
    ) -> (Option<LightningSoundIntent>, Option<PadShakeIntent>) {
        if self.when_to_play_lightning_sound == 0
            || context.time_ms <= self.when_to_play_lightning_sound
        {
            return (None, None);
        }
        self.when_to_play_lightning_sound = 0;
        (
            Some(LightningSoundIntent {
                sound_handle: self.sound_handle,
                sound: SOUND_LIGHTNING as i16,
                duration: self.lightning_duration,
            }),
            Some(PadShakeIntent {
                duration: 40 * self.lightning_duration + 100,
                frequency: 2 * self.lightning_duration + 80,
            }),
        )
    }

    fn update_wet_roads(&mut self) {
        if self.old_weather_type == WEATHER_RAINY {
            if self.new_weather_type == WEATHER_RAINY {
                self.wet_roads = 1.0;
            } else {
                self.wet_roads = 1.0 - self.interpolation_value;
            }
        } else if self.new_weather_type == WEATHER_RAINY {
            self.wet_roads = self.interpolation_value;
        } else {
            self.wet_roads = 0.0;
        }
    }

    fn update_rain(&mut self, context: WeatherUpdateContext) {
        let new_rain = if self.new_weather_type == WEATHER_RAINY {
            let mut rain = ((context.time_ms as u16 >> 14) as f32) * 0.33;
            if self.old_weather_type != WEATHER_RAINY {
                if self.interpolation_value < 0.4 {
                    rain = 0.5;
                } else {
                    rain = 0.25 + ((context.time_ms as u16 >> 14) as f32) * 0.25;
                }
            }
            rain
        } else {
            0.0
        };

        if self.rain != new_rain {
            if self.rain < new_rain {
                self.rain = new_rain.min(self.rain + RAIN_CHANGE_SPEED * context.time_step);
            } else {
                self.rain = new_rain.max(self.rain - RAIN_CHANGE_SPEED * context.time_step);
            }
        }
    }

    fn update_clouds_fog_rainbow_wind(&mut self, hour: u8) {
        self.cloud_coverage = if self.old_weather_type != WEATHER_SUNNY {
            1.0 - self.interpolation_value
        } else {
            0.0
        };
        if self.new_weather_type != WEATHER_SUNNY {
            self.cloud_coverage += self.interpolation_value;
        }

        self.foggyness = if self.old_weather_type == WEATHER_FOGGY {
            1.0 - self.interpolation_value
        } else {
            0.0
        };
        if self.new_weather_type == WEATHER_FOGGY {
            self.foggyness += self.interpolation_value;
        }

        if self.old_weather_type == WEATHER_RAINY
            && self.new_weather_type == WEATHER_SUNNY
            && self.interpolation_value < 0.5
            && hour > 6
            && hour < 21
        {
            self.rainbow = 1.0 - 4.0 * (self.interpolation_value - 0.25).abs() / 4.0;
        } else {
            self.rainbow = 0.0;
        }

        self.wind = self.interpolation_value * windyness(self.new_weather_type)
            + (1.0 - self.interpolation_value) * windyness(self.old_weather_type);
    }

    fn add_rain_intent(&self, context: WeatherUpdateContext) -> (Option<RainIntent>, bool) {
        if context.cam_no_rain || context.player_no_rain {
            return (None, false);
        }
        if context.looking_lrb_first_person && context.player_vehicle_has_roof {
            return (None, true);
        }
        if self.rain <= 0.1 {
            return (None, false);
        }
        let current_frame = (3.0 * self.rain) as i32 & 3;
        let num_drops = (2.0 * self.rain) as i32 + 2;
        (
            Some(RainIntent {
                rain: self.rain,
                current_frame,
                num_drops,
                num_splash_attempts: (3.0 * self.rain) as i32 + 1,
                num_splashes: (3.0 * self.rain) as i32 + 4,
            }),
            false,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn render_one_rain_streak(
    vertices: &mut Vec<RwIm3DVertexIntent>,
    indices: &mut Vec<u16>,
    pos: CVector,
    up: CVector,
    front: CVector,
    intensity: i32,
    distance: f32,
    timer_paused: bool,
    general: &mut CGeneral,
    random_tex: &mut f32,
    random_tex_x: &mut f32,
    random_tex_y: &mut f32,
) {
    let right = normalised(CVector::new(
        front.y * up.z - front.z * up.y,
        front.z * up.x - front.x * up.z,
        front.x * up.y - front.y * up.x,
    ));
    let base = vertices.len() as u16;
    indices.extend_from_slice(&[
        base,
        base + 2,
        base + 1,
        base,
        base + 3,
        base + 2,
        base + 1,
        base + 2,
        base + 4,
        base + 2,
        base + 3,
        base + 4,
    ]);

    let centre_colour = [
        (200 * intensity / 256).clamp(0, 255) as u8,
        (200 * intensity / 256).clamp(0, 255) as u8,
        intensity.clamp(0, 255) as u8,
        255,
    ];
    let positions = [
        CVector::new(
            pos.x + 11.0 * up.x,
            pos.y + 11.0 * up.y,
            pos.z + 11.0 * up.z,
        ),
        CVector::new(
            pos.x - 9.0 * right.x,
            pos.y - 9.0 * right.y,
            pos.z - 9.0 * right.z,
        ),
        pos,
        CVector::new(
            pos.x + 9.0 * right.x,
            pos.y + 9.0 * right.y,
            pos.z + 9.0 * right.z,
        ),
        CVector::new(
            pos.x - 11.0 * up.x,
            pos.y - 11.0 * up.y,
            pos.z - 11.0 * up.z,
        ),
    ];

    let distance_coefficient = if distance < STREAK_MIN_DISTANCE {
        1.0
    } else if distance > STREAK_MAX_DISTANCE {
        0.5
    } else {
        1.0 - 0.5 * (distance - STREAK_MIN_DISTANCE) / (STREAK_MAX_DISTANCE - STREAK_MIN_DISTANCE)
    };
    let u = STREAK_U * distance_coefficient;
    let v = STREAK_V * distance_coefficient;
    if !timer_paused {
        *random_tex = (i32::from(general.get_random_number() & 255) - 128) as f32 * 0.01;
        *random_tex_x = f32::from(general.get_random_number() & 127) * 0.01;
        *random_tex_y = f32::from(general.get_random_number() & 127) * 0.01;
    }
    let uvs = [
        [
            0.5 * u - *random_tex + *random_tex_x,
            -0.5 * v + *random_tex_y,
        ],
        [*random_tex_x, *random_tex_y],
        [0.5 * u + *random_tex_x, *random_tex_y],
        [u + *random_tex_x, *random_tex_y],
        [
            0.5 * u + *random_tex + *random_tex_x,
            0.5 * v + *random_tex_y,
        ],
    ];
    for index in 0..5 {
        vertices.push(RwIm3DVertexIntent {
            position: positions[index],
            colour: if index == 2 {
                centre_colour
            } else {
                [0, 0, 0, 0]
            },
            uv: uvs[index],
        });
    }
}

fn normalised(value: CVector) -> CVector {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON {
        CVector::ZERO
    } else {
        CVector::new(value.x / length, value.y / length, value.z / length)
    }
}

fn vector_distance(left: CVector, right: CVector) -> f32 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    (x * x + y * y + z * z).sqrt()
}

impl Default for CWeather {
    fn default() -> Self {
        Self::init()
    }
}

#[derive(SystemParam)]
pub(crate) struct WeatherScene<'w, 's> {
    file_loader: Res<'w, CFileLoader>,
    camera_state: Option<Res<'w, CCamera>>,
    player_state: Option<Res<'w, CWorldPlayerState>>,
    player_relations: Query<'w, 's, &'static InVehicle, With<PlayerPedState>>,
    vehicles: Query<
        'w,
        's,
        (&'static VehicleCore, &'static VehicleAppearance),
        With<crate::control::ScriptEntities::ScriptVehicleIdentity>,
    >,
    handling_manager: Option<Res<'w, cHandlingDataMgr>>,
    particles: Option<ResMut<'w, CParticle>>,
    particle_systems: Option<Res<'w, cParticleSystemMgr>>,
    camera_query: Query<'w, 's, (&'static Camera, &'static GtaSceneCamera)>,
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
}

/// Bevy schedule adapter for re3 `CWeather::Update` + the immediately
/// following `CWeather::AddRain` call in `CGame::Process`.
pub(crate) fn update_weather(
    mut weather: ResMut<CWeather>,
    clock: Res<CClock>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut stats: Option<ResMut<CStats>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut pads: ResMut<CPads>,
    menu: Option<Res<CMenuManager>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    game_state: Res<CGameState>,
    mut scene: WeatherScene,
    mut rain_state: Local<WeatherRainState>,
) {
    let pad1 = pads.get_pad(1);
    let looking_lrb_first_person = scene
        .camera_state
        .as_ref()
        .is_some_and(|camera| camera.get_looking_lrb_first_person());
    let player_vehicle_has_roof = scene
        .player_state
        .as_ref()
        .and_then(|state| state.entities.get(state.player_in_focus).copied().flatten())
        .and_then(|player| scene.player_relations.get(player).ok())
        .and_then(|relation| scene.vehicles.get(relation.vehicle).ok())
        .zip(scene.handling_manager.as_deref())
        .is_some_and(|((core, appearance), handling)| core.car_has_roof(appearance, handling));
    let result = weather.update(
        WeatherUpdateContext {
            minute: clock.minutes(),
            hour: clock.hours(),
            time_ms: timer.time_in_milliseconds(),
            frame_counter: timer.frame_counter(),
            time_step: timer.time_step(),
            right_shock_just_down: pad1.new_state.right_shock != 0
                && pad1.old_state.right_shock == 0,
            // Weather consumes the cached cull masks from the previous
            // `CCullZones::Update`; C++ refreshes them later, after Camera,
            // for the next frame (`Game.cpp::Process`, `ZoneCull.cpp:270-283`).
            cam_no_rain: scene.file_loader.world.cull_zones.camera_no_rain(),
            player_no_rain: scene.file_loader.world.cull_zones.player_no_rain(),
            looking_lrb_first_person,
            player_vehicle_has_roof,
        },
        &mut general,
    );

    // re3 `CWeather::AddRain` runs immediately after `Update` and appends
    // screen-space `PARTICLE_RAINDROP_2D` records. Keeping both pieces under
    // Weather ownership makes the RenderPrep boundary explicit.
    if let Some(particles) = scene.particles.as_deref_mut() {
        if result.remove_2d_raindrops {
            particles.remove_p_system(crate::renderer::ParticleType::PARTICLE_RAINDROP_2D);
        }
        if result.rain_intent.is_some()
            && (scene.particle_systems.is_none() || scene.camera_state.is_none())
        {
            warn_once!("weather_particles_waiting_original_creation_context");
        }
        if let (Some(rain), Some(systems), Some(camera)) = (
            result.rain_intent,
            scene.particle_systems.as_deref(),
            scene.camera_state.as_deref(),
        ) {
            let mut emitter = particles.emitter(systems, camera.position);
            let viewport = scene
                .camera_query
                .iter()
                .filter(|(camera, marker)| camera.is_active && marker.local_player == 0)
                .find_map(|(camera, _)| {
                    camera.viewport.as_ref().map(|viewport| {
                        (
                            viewport.physical_size.x as f32,
                            viewport.physical_size.y as f32,
                        )
                    })
                })
                .or_else(|| {
                    scene
                        .camera_query
                        .iter()
                        .filter(|(camera, _)| camera.is_active)
                        .find_map(|(camera, _)| {
                            camera.viewport.as_ref().map(|viewport| {
                                (
                                    viewport.physical_size.x as f32,
                                    viewport.physical_size.y as f32,
                                )
                            })
                        })
                })
                .or_else(|| {
                    scene
                        .windows
                        .iter()
                        .next()
                        .map(|window| (window.width(), window.height()))
                });
            if let Some((screen_width, screen_height)) = viewport {
                emit_weather_rain_particles(
                    &mut emitter,
                    &mut rain_state,
                    rain,
                    screen_width,
                    screen_height,
                    &mut general,
                );
            }
            emit_weather_ground_splashes(
                &mut emitter,
                rain,
                camera,
                &scene.file_loader,
                &mut general,
            );
        }
    }

    if let Some(stats) = stats.as_mut() {
        stats.mm_rain = stats.mm_rain.wrapping_add(i32::from(result.mm_rain_delta));
    }
    if let (Some(sound), Some(dmaudio)) = (result.lightning_sound, dmaudio.as_mut()) {
        dmaudio.PlayOneShot(
            sound.sound_handle,
            sound.sound as u16,
            sound.duration as f32,
        );
    }
    if let Some(shake) = result.pad_shake {
        let vibration_enabled = menu
            .as_ref()
            .is_none_or(|menu| menu.prefs_use_vibration != 0);
        let cutscene_running = cutscene
            .as_ref()
            .is_some_and(|cutscene| cutscene.is_running());
        pads.get_pad_mut(0).start_shake(
            shake.duration,
            shake.frequency,
            vibration_enabled,
            cutscene_running,
            game_state.playing_intro,
        );
    }
}

/// re3 `CWeather::AddRain` world-space splash pass. The reference asks
/// `CWorld::ProcessVerticalLine` for buildings and dummies only; the port's
/// static vertical-line query owns exactly those collision classes.
fn emit_weather_ground_splashes(
    particles: &mut ParticleEmitter<'_>,
    rain: RainIntent,
    camera: &CCamera,
    file_loader: &CFileLoader,
    general: &mut CGeneral,
) {
    use crate::renderer::{
        Particle::ParticleColour,
        ParticleType::{PARTICLE_RAIN_SPLASH, PARTICLE_RAIN_SPLASHUP},
    };

    let Some(active) = camera.cams.get(camera.active_cam) else {
        return;
    };
    let view_window_x = (active.fov.to_radians() * 0.5).tan();
    let view_window_y = view_window_x / camera.aspect_ratio.max(1.0e-4);
    let view_window_magnitude =
        (view_window_x * view_window_x + view_window_y * view_window_y).sqrt();
    if view_window_magnitude <= f32::EPSILON {
        return;
    }

    // The x components of re3's four transformed splash points cancel when
    // averaged. Their 1x/1x/4x/4x scale leaves this exact 2.5x centre.
    let scale = 2.5 / view_window_magnitude;
    let fp = CVector::new(
        camera.position.x + active.up.x * view_window_y * scale + active.front.x * scale,
        camera.position.y + active.up.y * view_window_y * scale + active.front.y * scale,
        camera.position.z + active.up.z * view_window_y * scale + active.front.z * scale,
    );
    let colour = ParticleColour::default();

    for _ in 0..rain.num_splash_attempts.max(0) as usize {
        let np = CVector::new(
            fp.x + general
                .get_random_number_in_range_f32(-SPLASH_CHECK_RADIUS, SPLASH_CHECK_RADIUS),
            fp.y + general
                .get_random_number_in_range_f32(-SPLASH_CHECK_RADIUS, SPLASH_CHECK_RADIUS),
            fp.z,
        );
        let Some(point) = file_loader.world.process_vertical_line_static(
            [np.x, np.y, np.z + 40.0],
            -40.0,
            &file_loader.model_info,
        ) else {
            continue;
        };

        for _ in 0..rain.num_splashes.max(0) as usize {
            let particle_type = if general.get_random_true_false() {
                PARTICLE_RAIN_SPLASH
            } else {
                PARTICLE_RAIN_SPLASHUP
            };
            particles.add_particle_coloured(
                particle_type,
                CVector::new(
                    np.x + general.get_random_number_in_range_f32(
                        -SPLASH_OFFSET_RADIUS,
                        SPLASH_OFFSET_RADIUS,
                    ),
                    np.y + general.get_random_number_in_range_f32(
                        -SPLASH_OFFSET_RADIUS,
                        SPLASH_OFFSET_RADIUS,
                    ),
                    point.point.z + 0.1,
                ),
                CVector::ZERO,
                None,
                0.0,
                colour,
                0,
                0,
                0,
                0,
            );
        }
    }
}

/// Persistent state used by re3's `CWeather::AddRain` static sweep.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WeatherRainState {
    static_rain_angle: i32,
    count: u32,
    add_angle: bool,
}

impl Default for WeatherRainState {
    fn default() -> Self {
        Self {
            static_rain_angle: -45,
            count: 1500,
            add_angle: true,
        }
    }
}

/// C++ `CWeather::AddRain` screen-space drop generation
/// (`renderer/Weather.cpp`) using the production `CParticle` intent queue.
fn emit_weather_rain_particles(
    particles: &mut ParticleEmitter<'_>,
    state: &mut WeatherRainState,
    rain: RainIntent,
    screen_width: f32,
    screen_height: f32,
    general: &mut CGeneral,
) {
    use crate::renderer::{Particle::ParticleColour, ParticleType::PARTICLE_RAINDROP_2D};

    if state.count > 0 {
        state.count -= 1;
    }
    if state.count == 0 {
        state.count = 1;
        if state.add_angle {
            state.static_rain_angle += 12;
            if state.static_rain_angle > 45 {
                state.count = 1500;
                state.add_angle = false;
            }
        } else {
            state.static_rain_angle -= 12;
            if state.static_rain_angle < -45 {
                state.count = 1500;
                state.add_angle = true;
            }
        }
    }

    let angle_degrees = state.static_rain_angle + i32::from(state.static_rain_angle < 0) * 360;
    let rain_angle = (angle_degrees as f32).to_radians();
    let (sin_angle, cos_angle) = rain_angle.sin_cos();
    let base_x = -sin_angle;
    let base_y = cos_angle;
    let colour = ParticleColour::default();
    let num_drops = rain.num_drops.max(0) as usize;
    if num_drops == 0 {
        return;
    }

    let random_dir = |general: &mut CGeneral| {
        CVector::new(
            (general.get_random_number_in_range_f32(-0.5, 0.5) + base_x)
                * general.get_random_number_in_range_f32(10.0, 25.0),
            (general.get_random_number_in_range_f32(-0.5, 0.5) + base_y)
                * general.get_random_number_in_range_f32(10.0, 25.0),
            0.0,
        )
    };

    let mut xpos = CVector::ZERO;
    for _ in 0..(2 * num_drops) {
        let direction = random_dir(general);
        particles.add_particle_coloured(
            PARTICLE_RAINDROP_2D,
            xpos,
            direction,
            None,
            general.get_random_number_in_range_f32(0.5, 0.9),
            colour,
            0,
            angle_degrees + general.get_random_number_in_range_i32(-10, 10),
            rain.current_frame,
            0,
        );
        xpos.x += screen_width / (2.0 * num_drops as f32);
        xpos.x += general.get_random_number_in_range_f32(-25.0, 25.0);
    }

    let mut ypos = CVector::ZERO;
    for _ in 0..num_drops {
        let direction = random_dir(general);
        particles.add_particle_coloured(
            PARTICLE_RAINDROP_2D,
            ypos,
            direction,
            None,
            general.get_random_number_in_range_f32(0.5, 0.9),
            colour,
            0,
            angle_degrees + general.get_random_number_in_range_i32(-10, 10),
            rain.current_frame,
            0,
        );
        ypos.y += screen_width / num_drops as f32;
        ypos.y += general.get_random_number_in_range_f32(-25.0, 25.0);
    }

    let mut ypos2 = CVector::ZERO;
    for _ in 0..num_drops {
        let direction = random_dir(general);
        particles.add_particle_coloured(
            PARTICLE_RAINDROP_2D,
            ypos2,
            direction,
            None,
            general.get_random_number_in_range_f32(0.5, 0.9),
            colour,
            0,
            angle_degrees + general.get_random_number_in_range_i32(-10, 10),
            rain.current_frame,
            0,
        );
        ypos2.y += screen_width / num_drops as f32;
        ypos2.y += general.get_random_number_in_range_f32(-25.0, 25.0);
    }

    for _ in 0..num_drops {
        let position = CVector::new(
            general.get_random_number_in_range_f32(10.0, screen_width - 10.0),
            general.get_random_number_in_range_f32(10.0, screen_height - 10.0),
            0.0,
        );
        particles.add_particle_coloured(
            PARTICLE_RAINDROP_2D,
            position,
            CVector::ZERO,
            None,
            general.get_random_number_in_range_f32(0.5, 0.9),
            colour,
            general.get_random_number_in_range_i32(-10, 10),
            360 - angle_degrees + general.get_random_number_in_range_i32(-30, 30),
            rain.current_frame,
            50,
        );
    }
}

#[cfg(test)]
mod rain_render_tests {
    use super::*;

    #[test]
    fn weather_rain_intent_emits_screen_space_layers() {
        let mut particles = CParticle::initialise();
        let particle_systems = cParticleSystemMgr::default();
        let mut state = WeatherRainState::default();
        let mut general = CGeneral::with_seed(7);
        emit_weather_rain_particles(
            &mut particles.emitter(&particle_systems, CVector::ZERO),
            &mut state,
            RainIntent {
                rain: 0.5,
                current_frame: 1,
                num_drops: 2,
                num_splash_attempts: 2,
                num_splashes: 5,
            },
            1280.0,
            720.0,
            &mut general,
        );

        assert_eq!(particles.add_intents.len(), 10);
        assert!(
            particles
                .add_intents
                .iter()
                .all(|intent| intent.particle_type
                    == crate::renderer::ParticleType::PARTICLE_RAINDROP_2D)
        );
        assert!(
            particles
                .add_intents
                .iter()
                .any(|intent| intent.position.y > 0.0)
        );
        assert_eq!(state.static_rain_angle, -45);
        assert_eq!(state.count, 1499);
    }

    #[test]
    fn rain_streak_render_matches_re3_five_vertex_strip() {
        let mut weather = CWeather::init();
        weather.rain = 1.0;
        weather.rain_streaks[0] = RainStreak {
            position: CVector::new(0.0, 0.0, 10.0),
            direction: CVector::new(4.0, 4.0, -4.0),
            timer: 1,
        };
        let camera = CCamera::default();
        let mut general = CGeneral::with_seed(7);

        weather.render_rain_streaks(
            1025,
            false,
            false,
            0,
            &camera,
            CVector::ZERO,
            false,
            &mut general,
        );

        assert_eq!(weather.rain_render_batches.len(), 1);
        let batch = &weather.rain_render_batches[0];
        assert_eq!(batch.texture_name, "raindrop4");
        assert_eq!(batch.vertices.len(), 5);
        assert_eq!(batch.indices.len(), 12);
        assert_eq!(batch.vertices[2].position, CVector::new(4.0, 4.0, 6.0));
        assert_eq!(batch.vertices[2].colour[3], 255);
    }
}

fn windyness(weather: i16) -> f32 {
    WINDYNESS[weather.clamp(0, WEATHER_TOTAL - 1) as usize]
}
