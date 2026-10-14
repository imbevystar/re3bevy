//! re3 reference: references/re-GTA/src/renderer/Rubbish.cpp
//! re3 reference: references/re-GTA/src/renderer/Rubbish.h
//!
//! Wind-blown street rubbish state and render intents. World collision, cull-zone
//! queries and the re3 RNG are supplied by the ECS owner; this module does not
//! fabricate ground or visibility results.

use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use super::RenderBuffer::RwIm3DVertexIntent;
use crate::{
    core::{Camera::CCamera, FileLoader::CFileLoader},
    renderer::Weather::CWeather,
};

pub const NUM_RUBBISH_SHEETS: usize = 64;

/// ECS adapter for re3 `CRubbish::Update`; world queries remain explicit and
/// use the loaded GTA collision/cull-zone state.
pub(crate) fn update_rubbish(
    timer: Res<CTimer>,
    weather: Res<CWeather>,
    camera: Res<CCamera>,
    file_loader: Res<CFileLoader>,
    mut general: ResMut<CGeneral>,
    rubbish: Option<ResMut<CRubbish>>,
) {
    let Some(mut rubbish) = rubbish else {
        return;
    };
    let camera_position = camera.position;
    let camera_orientation = camera.orientation.x.atan2(camera.orientation.y);
    let mut random = || u32::from(general.get_random_number());
    let mut ground_z = |x: f32, y: f32, z: f32| match file_loader.world.find_ground_z_for_3d_coord(
        x,
        y,
        z,
        &file_loader.model_info,
    ) {
        Some(height) => (height, true),
        None => (0.0, false),
    };
    let attributes_at = |position: CVector| {
        file_loader
            .world
            .cull_zones
            .find_attributes_for_coors([position.x, position.y, position.z])
            .0 as u32
    };
    rubbish.update(
        timer.time_in_milliseconds(),
        timer.frame_counter(),
        timer.time_step(),
        weather.wind,
        camera_position,
        camera_orientation,
        &mut random,
        &mut ground_z,
        &attributes_at,
    );
}

/// re3 calls `CRubbish::StirUp(this)` once from every automobile's
/// `CAutomobile::ProcessControl`, before the status-specific control branch.
/// Keep that ownership in ECS by sampling the live automobile components once
/// per frame instead of making `CRubbish` maintain a parallel vehicle list.
pub(crate) fn stir_rubbish_from_automobiles(
    timer: Res<CTimer>,
    camera: Res<CCamera>,
    file_loader: Res<CFileLoader>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    rubbish: Option<ResMut<CRubbish>>,
) {
    let Some(mut rubbish) = rubbish else {
        return;
    };

    for vehicle in vehicles.iter() {
        let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
            continue;
        };
        let physical = &car.vehicle.core.physical;
        let Some(model_id) = usize::try_from(physical.entity.model_index).ok() else {
            continue;
        };
        let Some(col_model) = file_loader.model_info.col_model(model_id) else {
            continue;
        };

        let snapshot = RubbishVehicleSnapshot {
            position: physical.entity.get_position(),
            move_speed: physical.move_speed,
            forward: physical.entity.get_forward(),
            right: physical.entity.get_right(),
            bound_max_x: col_model.bounding_box.max.x,
            bound_max_y: col_model.bounding_box.max.y,
            random_seed: physical.entity.random_seed as u8,
        };
        let mut ground_z = |x: f32, y: f32, z: f32| {
            file_loader
                .world
                .find_ground_z_for_3d_coord(x, y, z, &file_loader.model_info)
                .unwrap_or(0.0)
        };
        rubbish.stir_up(
            snapshot,
            camera.position,
            timer.frame_counter(),
            timer.time_in_milliseconds(),
            &mut ground_z,
        );
    }
}
pub const RUBBISH_MAX_DIST: f32 = 18.0;
pub const RUBBISH_FADE_DIST: f32 = 16.5;
pub const ATTRZONE_NORAIN: u32 = 8;
pub const RUBBISH_TEXTURE_NAMES: [&str; 4] = [
    "gameleaf01_64",
    "gameleaf02_64",
    "newspaper01_64",
    "newspaper02_64",
];

const RUBBISH_INDICES: [u16; 6] = [0, 1, 2, 1, 3, 2];
const TWO_PI_RE3: f32 = 6.28;

// The first 17 entries are XY displacement and the second 17 are height.
const ANIMATIONS: [[f32; 34]; 3] = [
    [0.0; 34],
    [
        0.0, 0.05, 0.12, 0.25, 0.42, 0.57, 0.68, 0.8, 0.86, 0.9, 0.93, 0.95, 0.96, 0.97, 0.98,
        0.99, 1.0, 0.15, 0.35, 0.6, 0.9, 1.2, 1.25, 1.3, 1.2, 1.1, 0.95, 0.8, 0.6, 0.45, 0.3, 0.2,
        0.1, 0.0,
    ],
    [
        0.0, 0.05, 0.12, 0.25, 0.42, 0.57, 0.68, 0.8, 0.95, 1.1, 1.15, 1.18, 1.15, 1.1, 1.05, 1.03,
        1.0, 0.15, 0.35, 0.6, 0.9, 1.2, 1.25, 1.3, 1.2, 1.1, 0.95, 0.8, 0.6, 0.45, 0.3, 0.2, 0.1,
        0.0,
    ],
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct COneSheetState {
    pub base_pos: CVector,
    pub animated_pos: CVector,
    pub target_z: f32,
    /// 0: empty, 1: static, 2: moving.
    pub state: u8,
    pub animation_type: u8,
    pub move_start: u32,
    pub move_duration: u32,
    pub anim_height: f32,
    pub x_dist: f32,
    pub y_dist: f32,
    pub angle: f32,
    pub is_visible: bool,
    pub target_is_visible: bool,
}

impl Default for COneSheetState {
    fn default() -> Self {
        Self {
            base_pos: CVector::ZERO,
            animated_pos: CVector::ZERO,
            target_z: 0.0,
            state: 0,
            animation_type: 0,
            move_start: 0,
            move_duration: 0,
            anim_height: 0.0,
            x_dist: 0.0,
            y_dist: 0.0,
            angle: 0.0,
            is_visible: false,
            target_is_visible: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RubbishVehicleSnapshot {
    pub position: CVector,
    pub move_speed: CVector,
    pub forward: CVector,
    pub right: CVector,
    /// Positive maximum X/Y of the vehicle collision bounding box.
    pub bound_max_x: f32,
    pub bound_max_y: f32,
    pub random_seed: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RubbishRenderBatch {
    pub texture_name: &'static str,
    pub vertices: Vec<RwIm3DVertexIntent>,
    pub indices: Vec<u16>,
    pub z_write: bool,
    pub vertex_alpha: bool,
    pub fog: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CRubbish {
    pub rubbish_invisible: bool,
    pub rubbish_visibility: i32,
    pub sheets: [COneSheetState; NUM_RUBBISH_SHEETS],
    /// Intrusive C++ lists represented by indices, head first.
    pub empty_list: Vec<usize>,
    pub statics_list: Vec<usize>,
    pub movers_list: Vec<usize>,
    pub render_batches: Vec<RubbishRenderBatch>,
}

impl CRubbish {
    pub fn init() -> Self {
        Self {
            rubbish_invisible: false,
            rubbish_visibility: 255,
            sheets: [COneSheetState::default(); NUM_RUBBISH_SHEETS],
            empty_list: (0..NUM_RUBBISH_SHEETS).collect(),
            statics_list: Vec::new(),
            movers_list: Vec::new(),
            render_batches: Vec::new(),
        }
    }

    pub fn shutdown(&mut self) {
        // re3 destroys the four particle-TXD textures. Texture ownership lives in
        // the Bevy TXD store, so only frame-local draw references are released here.
        self.render_batches.clear();
    }

    pub fn set_visibility(&mut self, visible: bool) {
        self.rubbish_invisible = !visible;
    }

    /// C++ `CRubbish::Update`. `ground_z` returns (height, foundGround), and
    /// `attributes_at` is `CCullZones::FindAttributesForCoors`.
    pub fn update(
        &mut self,
        time_ms: u32,
        frame_counter: u32,
        time_step: f32,
        wind: f32,
        camera_position: CVector,
        camera_orientation: f32,
        random: &mut impl FnMut() -> u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> (f32, bool),
        attributes_at: &impl Fn(CVector) -> u32,
    ) {
        if self.rubbish_invisible {
            self.rubbish_visibility = (self.rubbish_visibility - 5).max(0);
        } else {
            self.rubbish_visibility = (self.rubbish_visibility + 5).min(255);
        }

        // Spawn exactly one sheet from the head of the empty list.
        if let Some(&index) = self.empty_list.first() {
            let spawn_dist = (random() & 0xFF) as f32 / 256.0 + RUBBISH_MAX_DIST;
            let r = random() as u8;
            let spawn_angle = if r & 1 != 0 {
                (random() & 0xFF) as f32 / 256.0 * TWO_PI_RE3
            } else {
                (r as f32 - 128.0) / 160.0 + camera_orientation
            };
            let x = camera_position.x + spawn_dist * spawn_angle.sin();
            let y = camera_position.y + spawn_dist * spawn_angle.cos();
            let (z, found_ground) = ground_z(x, y, camera_position.z);
            if found_ground {
                let sheet = &mut self.sheets[index];
                sheet.base_pos = CVector::new(x, y, z + 0.1);
                sheet.angle = (random() & 0xFF) as f32 / 256.0 * TWO_PI_RE3;
                sheet.state = 1;
                sheet.is_visible = attributes_at(sheet.base_pos) & ATTRZONE_NORAIN == 0;
                self.move_to_list_head(index, RubbishList::Statics);
            }
        }

        // Iterate a snapshot because completed movers change lists during traversal.
        for index in self.movers_list.clone() {
            let current_time = time_ms.wrapping_sub(self.sheets[index].move_start);
            let duration = self.sheets[index].move_duration;
            if current_time < duration {
                let step = (16 * current_time / duration) as usize;
                let step_time = duration / 16;
                let s = (current_time - step_time * step as u32) as f32 / step_time as f32;
                let t = current_time as f32 / duration as f32;
                let anim = ANIMATIONS[self.sheets[index].animation_type as usize];
                let fxy = anim[step] * (1.0 - s) + anim[step + 1] * s;
                let fz = anim[step + 17] * (1.0 - s) + anim[step + 18] * s;
                let sheet = &mut self.sheets[index];
                sheet.animated_pos.x = sheet.base_pos.x + fxy * sheet.x_dist;
                sheet.animated_pos.y = sheet.base_pos.y + fxy * sheet.y_dist;
                sheet.animated_pos.z =
                    (1.0 - t) * sheet.base_pos.z + t * sheet.target_z + fz * sheet.anim_height;
                sheet.angle += time_step * 0.04;
                if sheet.angle > TWO_PI_RE3 {
                    sheet.angle -= TWO_PI_RE3;
                }
            } else {
                let sheet = &mut self.sheets[index];
                sheet.base_pos.x += sheet.x_dist;
                sheet.base_pos.y += sheet.y_dist;
                sheet.base_pos.z = sheet.target_z;
                sheet.state = 1;
                sheet.is_visible = sheet.target_is_visible;
                self.move_to_list_head(index, RubbishList::Statics);
            }
        }

        let freq = if wind < 0.1 {
            31
        } else if wind < 0.4 {
            7
        } else if wind < 0.7 {
            1
        } else {
            0
        };
        if frame_counter & freq == 0 {
            let index = random() as usize % NUM_RUBBISH_SHEETS;
            if self.sheets[index].state == 1 {
                let sheet = &mut self.sheets[index];
                sheet.move_start = time_ms;
                sheet.move_duration = (wind * 1500.0 + 1000.0) as u32;
                sheet.anim_height = 0.2;
                sheet.x_dist = 3.0 * wind;
                sheet.y_dist = 3.0 * wind;
                let tx = sheet.base_pos.x + sheet.x_dist;
                let ty = sheet.base_pos.y + sheet.y_dist;
                let tz = sheet.base_pos.z + 3.0;
                let (target_z, found_ground) = ground_z(tx, ty, tz);
                sheet.target_z = target_z + 0.1;
                sheet.target_is_visible =
                    attributes_at(CVector::new(tx, ty, sheet.target_z)) & ATTRZONE_NORAIN == 0;
                if found_ground {
                    sheet.state = 2;
                    sheet.animation_type = 1;
                    self.move_to_list_head(index, RubbishList::Movers);
                }
            }
        }

        let first = (frame_counter as usize % (NUM_RUBBISH_SHEETS / 4)) * 4;
        for index in first..first + 4 {
            let sheet = self.sheets[index];
            let dx = sheet.base_pos.x - camera_position.x;
            let dy = sheet.base_pos.y - camera_position.y;
            if sheet.state == 1 && dx * dx + dy * dy > (RUBBISH_MAX_DIST + 1.0).powi(2) {
                self.sheets[index].state = 0;
                self.move_to_list_head(index, RubbishList::Empty);
            }
        }
    }

    /// C++ `CRubbish::StirUp`; ground collision is queried only after a sheet is selected.
    pub fn stir_up(
        &mut self,
        vehicle: RubbishVehicleSnapshot,
        camera_position: CVector,
        frame_counter: u32,
        time_ms: u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> f32,
    ) {
        if (frame_counter ^ u32::from(vehicle.random_seed & 3)) == 0 {
            return;
        }
        if (vehicle.position.x - camera_position.x).abs() >= 20.0
            || (vehicle.position.y - camera_position.y).abs() >= 20.0
            || (vehicle.move_speed.x.abs() <= 0.05 && vehicle.move_speed.y.abs() <= 0.05)
        {
            return;
        }
        let speed = magnitude_2d(vehicle.move_speed);
        if speed <= 0.05 {
            return;
        }
        let moving_forward = dot_2d(vehicle.move_speed, vehicle.forward) > 0.0;
        for index in self.statics_list.clone() {
            let sheet = self.sheets[index];
            let car_to_sheet = CVector::new(
                sheet.base_pos.x - vehicle.position.x,
                sheet.base_pos.y - vehicle.position.y,
                0.0,
            );
            let dist_fwd = dot_2d(car_to_sheet, vehicle.forward);
            let behind = (moving_forward
                && dist_fwd < -0.5 * vehicle.bound_max_y
                && dist_fwd > -1.5 * vehicle.bound_max_y)
                || (!moving_forward
                    && dist_fwd > 0.5 * vehicle.bound_max_y
                    && dist_fwd < 1.5 * vehicle.bound_max_y);
            if !behind {
                continue;
            }
            let dist_side = dot_2d(car_to_sheet, vehicle.right).abs();
            if dist_side >= 1.5 * vehicle.bound_max_x {
                continue;
            }
            let speed_to_check = if dist_side < vehicle.bound_max_x {
                speed
            } else {
                speed * 0.5
            };
            if speed_to_check <= 0.05 {
                continue;
            }
            let sheet = &mut self.sheets[index];
            sheet.state = 2;
            sheet.animation_type = if speed_to_check > 0.15 { 2 } else { 1 };
            sheet.move_duration = 2000;
            sheet.x_dist = vehicle.move_speed.x;
            sheet.y_dist = vehicle.move_speed.y;
            let dist = (sheet.x_dist * sheet.x_dist + sheet.y_dist * sheet.y_dist).sqrt();
            sheet.x_dist *= 25.0 * speed / dist;
            sheet.y_dist *= 25.0 * speed / dist;
            sheet.anim_height = 3.0 * speed;
            sheet.move_start = time_ms;
            let tx = sheet.base_pos.x + sheet.x_dist;
            let ty = sheet.base_pos.y + sheet.y_dist;
            sheet.target_z = ground_z(tx, ty, sheet.base_pos.z + 3.0) + 0.1;
            self.move_to_list_head(index, RubbishList::Movers);
        }
    }

    /// Builds the same four texture-grouped quads as `CRubbish::Render`.
    pub fn render(&mut self, time_ms: u32, camera_position: CVector) {
        self.render_batches.clear();
        for (texture_type, texture_name) in RUBBISH_TEXTURE_NAMES.iter().enumerate() {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let start = texture_type * NUM_RUBBISH_SHEETS / 4;
            let end = (texture_type + 1) * NUM_RUBBISH_SHEETS / 4;
            for index in start..end {
                let sheet = self.sheets[index];
                if sheet.state == 0 {
                    continue;
                }
                let (pos, mut alpha) = if sheet.state == 1 {
                    (sheet.base_pos, if sheet.is_visible { 128.0 } else { 0.0 })
                } else {
                    let mut alpha = 128.0;
                    if !sheet.is_visible || !sheet.target_is_visible {
                        let t = time_ms.wrapping_sub(sheet.move_start) as f32
                            / sheet.move_duration as f32;
                        let f1 = if sheet.is_visible { 1.0 - t } else { 0.0 };
                        let f2 = if sheet.target_is_visible { t } else { 0.0 };
                        alpha = 128.0 * (f1 + f2);
                    }
                    (sheet.animated_pos, alpha)
                };
                let dx = pos.x - camera_position.x;
                let dy = pos.y - camera_position.y;
                let cam_dist = (dx * dx + dy * dy).sqrt();
                if cam_dist >= RUBBISH_MAX_DIST {
                    continue;
                }
                if cam_dist >= RUBBISH_FADE_DIST {
                    alpha -= alpha * (cam_dist - RUBBISH_FADE_DIST)
                        / (RUBBISH_MAX_DIST - RUBBISH_FADE_DIST);
                }
                let alpha = ((self.rubbish_visibility as f32 * alpha) / 256.0) as u8;
                let vx = sheet.angle.sin() * 0.4;
                let vy = sheet.angle.cos() * 0.4;
                let base = vertices.len() as u16;
                let colour = [255, 255, 255, alpha];
                vertices.extend([
                    vertex(pos.x + vx, pos.y + vy, pos.z, colour, [0.0, 0.0]),
                    vertex(pos.x - vy, pos.y + vx, pos.z, colour, [1.0, 0.0]),
                    vertex(pos.x + vy, pos.y - vx, pos.z, colour, [0.0, 1.0]),
                    vertex(pos.x - vx, pos.y - vy, pos.z, colour, [1.0, 1.0]),
                ]);
                indices.extend(RUBBISH_INDICES.map(|i| i + base));
            }
            if !indices.is_empty() {
                self.render_batches.push(RubbishRenderBatch {
                    texture_name: *texture_name,
                    vertices,
                    indices,
                    z_write: false,
                    vertex_alpha: true,
                    fog: true,
                });
            }
        }
    }

    fn move_to_list_head(&mut self, index: usize, destination: RubbishList) {
        self.empty_list.retain(|&item| item != index);
        self.statics_list.retain(|&item| item != index);
        self.movers_list.retain(|&item| item != index);
        match destination {
            RubbishList::Empty => self.empty_list.insert(0, index),
            RubbishList::Statics => self.statics_list.insert(0, index),
            RubbishList::Movers => self.movers_list.insert(0, index),
        }
    }
}

impl Default for CRubbish {
    fn default() -> Self {
        Self::init()
    }
}

#[derive(Debug, Clone, Copy)]
enum RubbishList {
    Empty,
    Statics,
    Movers,
}

fn dot_2d(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y
}

fn magnitude_2d(value: CVector) -> f32 {
    (value.x * value.x + value.y * value.y).sqrt()
}

fn vertex(x: f32, y: f32, z: f32, colour: [u8; 4], uv: [f32; 2]) -> RwIm3DVertexIntent {
    RwIm3DVertexIntent {
        position: CVector::new(x, y, z),
        colour,
        uv,
    }
}
