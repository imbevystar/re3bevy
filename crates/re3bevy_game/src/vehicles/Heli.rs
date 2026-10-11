//! re3 reference: references/re-GTA/src/vehicles/Heli.cpp
//! re3 reference: references/re-GTA/src/vehicles/Heli.h
//!
//! Heli vehicle and `CHeli::InitHelis` state.

use bevy::prelude::*;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    core::{
        FileLoader::CFileLoader,
        Game::ScriptEntityWorld,
        Pools::CPools,
        Streaming::{CStreaming, STREAMFLAGS_DONT_REMOVE},
    },
    entities::Entity::eEntityStatus,
    modelinfo::VehicleModelInfo::eVehicleType,
    peds::PlayerPed::CWorldPlayers,
    renderer::WaterLevel::CWaterLevel,
    vehicles::Vehicle::CVehicle,
    weapons::{
        Explosion::{CExplosionPool, eExplosionType},
        ProjectileInfo::CProjectileInfoPool,
    },
};

pub const NUM_HELIS: usize = 4;
pub const HELI_CHASSIS: usize = 1;
pub const HELI_TOPROTOR: usize = 2;
pub const HELI_BACKROTOR: usize = 3;
pub const HELI_TAIL: usize = 4;
pub const HELI_TOPKNOT: usize = 5;
pub const HELI_SKID_LEFT: usize = 6;
pub const HELI_SKID_RIGHT: usize = 7;
pub const NUM_HELI_NODES: usize = 8;

pub const HELI_RANDOM0: usize = 0;
pub const HELI_RANDOM1: usize = 1;
pub const HELI_SCRIPT: usize = 2;
pub const HELI_CATALINA: usize = 3;

pub const HELI_TYPE_RANDOM: i8 = 0;
pub const HELI_TYPE_SCRIPT: i8 = 1;
pub const HELI_TYPE_CATALINA: i8 = 2;

pub const HELI_STATUS_HOVER: i8 = 0;
pub const HELI_STATUS_CHASE_PLAYER: i8 = 1;
pub const HELI_STATUS_FLY_AWAY: i8 = 2;
pub const HELI_STATUS_SHOT_DOWN: i8 = 3;
pub const HELI_STATUS_HOVER2: i8 = 4;

const CATALINA_TARGETS: [CVector; 7] = [
    CVector::new(-478.0, 227.0, 77.0),
    CVector::new(-677.0, 206.0, 66.0),
    CVector::new(-907.0, 210.0, 60.0),
    CVector::new(-1095.0, 242.0, 53.0),
    CVector::new(-1152.0, 278.0, 51.0),
    CVector::new(-1161.0, 341.0, 46.0),
    CVector::new(-1161.0, 341.0, 30.0),
];
const DAM_PATH: [CVector; 6] = [
    CVector::new(-1191.0, 350.0, 42.0),
    CVector::new(-1176.0, 388.0, 37.0),
    CVector::new(-1128.0, 429.0, 28.0),
    CVector::new(-1072.0, 447.0, 28.0),
    CVector::new(-1007.0, 449.0, 31.0),
    CVector::new(-971.0, 416.0, 33.0),
];
const SHORT_PATH: [CVector; 4] = [
    CVector::new(-974.0, 340.0, 41.0),
    CVector::new(-1036.0, 312.0, 38.0),
    CVector::new(-1112.0, 317.0, 32.0),
    CVector::new(-1173.0, 294.0, 39.0),
];
const LONG_PATH: [CVector; 7] = [
    CVector::new(-934.0, 371.0, 57.0),
    CVector::new(-905.0, 362.0, 90.0),
    CVector::new(-906.0, 488.0, 105.0),
    CVector::new(-1063.0, 548.0, 100.0),
    CVector::new(-1204.0, 451.0, 81.0),
    CVector::new(-1233.0, 346.0, 79.0),
    CVector::new(-1207.0, 308.0, 70.0),
];

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct HeliState {
    pub heli_nodes_loaded: [bool; NUM_HELI_NODES],
    pub heli_status: i8,
    pub search_light_x: f32,
    pub search_light_y: f32,
    pub explosion_timer: u32,
    pub rotation: f32,
    pub angular_speed: f32,
    pub target_z: f32,
    pub search_light_intensity: f32,
    pub heli_id: i8,
    pub heli_type: i8,
    pub path_state: i8,
    pub search_light_history_x: [f32; 6],
    pub search_light_history_y: [f32; 6],
    pub search_light_timer: u32,
    pub shoot_timer: u32,
    pub last_shot_time: u32,
    pub bullet_damage: u32,
    pub rotor_rotation: f32,
    pub heli_dust_z: [f32; 8],
    pub police_shout_timer: u32,
    pub target_offset: f32,
    pub test_right: bool,
}
use crate::vehicles::Vehicle::{Owned, VehicleCore, VehicleRuntime, VehicleVitality};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio> {
    pub vehicle: VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>,
    pub state: S,
}
pub type CHeli = HeliRuntimeView<
    Owned<VehicleCore>,
    Owned<VehicleVitality>,
    Owned<HeliState>,
    crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleDriveState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAppearance>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleEntryState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAudioState>>;
pub type HeliRef<'a> = HeliRuntimeView<
    &'a VehicleCore,
    &'a VehicleVitality,
    &'a HeliState,
    &'a crate::vehicles::Vehicle::VehicleDriveState, &'a crate::vehicles::Vehicle::VehicleAppearance, &'a crate::vehicles::Vehicle::VehicleEntryState, &'a crate::vehicles::Vehicle::VehicleAudioState>;
pub type HeliMut<'a> = HeliRuntimeView<
    &'a mut VehicleCore,
    &'a mut VehicleVitality,
    &'a mut HeliState,
    &'a mut crate::vehicles::Vehicle::VehicleDriveState, &'a mut crate::vehicles::Vehicle::VehicleAppearance, &'a mut crate::vehicles::Vehicle::VehicleEntryState, &'a mut crate::vehicles::Vehicle::VehicleAudioState>;
impl<
    C,
    V,
    S: std::ops::Deref<Target = HeliState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::Deref for HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    type Target = HeliState;
    fn deref(&self) -> &HeliState {
        &self.state
    }
}
impl<
    C,
    V,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::DerefMut for HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    fn deref_mut(&mut self) -> &mut HeliState {
        &mut self.state
    }
}
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = HeliState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_ref(&self) -> HeliRef<'_> {
        HeliRuntimeView {
            vehicle: self.vehicle.as_ref(),
            state: &self.state,
        }
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_mut(&mut self) -> HeliMut<'_> {
        HeliRuntimeView {
            vehicle: self.vehicle.as_mut(),
            state: &mut self.state,
        }
    }
}
impl CHeli {
    pub fn into_parts(
        self,
    ) -> (
        VehicleCore,
        VehicleVitality,
        crate::vehicles::Vehicle::VehicleDriveState,
        crate::vehicles::Vehicle::VehicleAppearance,
        crate::vehicles::Vehicle::VehicleEntryState,
        crate::vehicles::Vehicle::VehicleAudioState,
        HeliState,
    ) {
        let (core, vitality, drive, appearance, entry, audio) = self.vehicle.into_parts();
        (core, vitality, drive, appearance, entry, audio, self.state.0)
    }
}

impl CHeli {
    pub fn new(id: i32, created_by: u8, time_ms: u32, general: &mut CGeneral) -> Self {
        let mut vehicle = CVehicle::new(created_by, time_ms, general);
        vehicle.vehicle_type = eVehicleType::VEHICLE_TYPE_HELI;
        vehicle
            .physical
            .entity
            .set_model_index_no_create(id as usize);
        vehicle.physical.mass = 100000000.0;
        vehicle.physical.turn_mass = 100000000.0;
        vehicle.physical.air_resistance = 0.9994;
        vehicle.physical.elasticity = 0.05;
        vehicle.set_status(eEntityStatus::STATUS_HELI);

        Self {
            vehicle,
            state: Owned(HeliState {
                heli_nodes_loaded: [false; NUM_HELI_NODES],
                heli_status: HELI_STATUS_HOVER,
                search_light_x: 0.0,
                search_light_y: 0.0,
                explosion_timer: 0,
                rotation: 0.0,
                angular_speed: 0.0,
                target_z: 0.0,
                search_light_intensity: 0.0,
                heli_id: 0,
                heli_type: HELI_TYPE_RANDOM,
                path_state: 0,
                search_light_history_x: [0.0; 6],
                search_light_history_y: [0.0; 6],
                search_light_timer: time_ms,
                shoot_timer: 0,
                last_shot_time: 0,
                bullet_damage: 0,
                rotor_rotation: 0.0,
                heli_dust_z: [-50.0; 8],
                police_shout_timer: time_ms,
                target_offset: 0.0,
                test_right: true,
            }),
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    /// Catalina branch of re3 `CHeli::ProcessControl`.
    ///
    /// The mission helicopter never uses the police searchlight/shooting branch,
    /// but it does use the common hover/chase, velocity, altitude and matrix
    /// integration below it. `path_point` mirrors the original file-static
    /// `PathPoint`, which is shared by Catalina path states 9, 10 and 11.
    pub fn process_catalina_control(
        &mut self,
        now_ms: u32,
        previous_ms: u32,
        time_step: f32,
        path_point: &mut usize,
    ) -> Option<CVector> {
        if self.state.heli_type != HELI_TYPE_CATALINA
            || self.state.heli_status == HELI_STATUS_SHOT_DOWN
        {
            return None;
        }

        let mut position = self.vehicle.core.physical.entity.get_position();
        let mut target = position;
        match self.state.path_state {
            0..=5 => {
                target = CATALINA_TARGETS[self.state.path_state as usize];
                if distance(target, position) < 9.0 {
                    self.state.path_state += 1;
                }
            }
            6 => {
                target = CATALINA_TARGETS[6];
                if position.z <= 31.55 {
                    self.state.path_state = 7;
                    position.z = 31.55;
                    self.vehicle.core.physical.move_speed = CVector::ZERO;
                }
            }
            7 => {
                position.z = 31.55;
                target = position;
            }
            8 => {
                target = CVector::new(position.x, position.y, 74.0);
                if position.z >= 40.0 {
                    *path_point = 2;
                    self.state.path_state = 9;
                }
            }
            9 => {
                *path_point = (*path_point).min(DAM_PATH.len() - 1);
                target = DAM_PATH[*path_point];
                if distance(target, position) < 9.0 {
                    *path_point += 1;
                    if *path_point >= DAM_PATH.len() {
                        self.state.path_state = 10;
                        *path_point = 0;
                    }
                }
            }
            10 => {
                *path_point = (*path_point).min(SHORT_PATH.len() - 1);
                target = SHORT_PATH[*path_point];
                if distance(target, position) < 9.0 {
                    *path_point += 1;
                    // re3 deliberately changes back before ShortPath[3].
                    if *path_point >= 3 {
                        self.state.path_state = 9;
                        *path_point = 1;
                    }
                }
            }
            11 => {
                *path_point = (*path_point).min(LONG_PATH.len() - 1);
                target = LONG_PATH[*path_point];
                if distance(target, position) < 9.0 {
                    *path_point += 1;
                    if *path_point >= LONG_PATH.len() {
                        self.state.path_state = 9;
                        *path_point = 0;
                    }
                }
            }
            12 => {
                target = CVector::new(position.x, position.y, 200.0);
            }
            _ => {}
        }

        let mut target_dx = target.x - position.x;
        let mut target_dy = target.y - position.y;
        self.state.target_z = target.z;
        if self.state.path_state == 6 {
            position.x = position.x * 0.99 + target.x * 0.01;
            position.y = position.y * 0.99 + target.y * 0.01;
        }

        let target_distance = (target_dx * target_dx + target_dy * target_dy).sqrt();
        match self.state.heli_status {
            HELI_STATUS_HOVER | HELI_STATUS_HOVER2 => {
                if target_distance > 8.0 {
                    self.state.heli_status = HELI_STATUS_CHASE_PLAYER;
                }
            }
            HELI_STATUS_CHASE_PLAYER => {
                if target_distance < 1.0 || target_distance < 4.0 {
                    self.state.heli_status = HELI_STATUS_HOVER;
                }
            }
            _ => {}
        }

        let mut speed = if target_distance > 100.0 {
            1.0
        } else if target_distance > 75.0 {
            0.7
        } else {
            0.4
        };
        if matches!(
            self.state.heli_status,
            HELI_STATUS_HOVER | HELI_STATUS_HOVER2 | HELI_STATUS_SHOT_DOWN
        ) {
            speed = 0.0;
        }
        if target_distance != 0.0 {
            target_dx /= target_distance;
            target_dy /= target_distance;
        } else {
            target_dx = 1.0;
        }
        let target_speed_x = target_dx * speed;
        let target_speed_y = target_dy * speed;

        if matches!(self.state.path_state, 9..=11) {
            let damping = 0.997_f32.powf(time_step);
            self.vehicle.core.physical.move_speed.x *= damping;
            self.vehicle.core.physical.move_speed.y *= damping;
        }
        approach_xy_speed(
            &mut self.vehicle.core.physical.move_speed,
            target_speed_x,
            target_speed_y,
            time_step * 0.002,
        );
        position.x += self.vehicle.core.physical.move_speed.x * time_step;
        position.y += self.vehicle.core.physical.move_speed.y * time_step;

        if self.state.heli_status == HELI_STATUS_FLY_AWAY {
            self.state.target_z = 1000.0;
        }
        if now_ms.wrapping_add(800_u32.wrapping_mul(self.state.heli_id.max(0) as u32)) & 0x800 != 0
        {
            self.state.target_z += 2.0;
        }
        self.state.target_z += f32::from(self.state.heli_id) * 5.0;
        let target_speed_z = (self.state.target_z - position.z) * 0.01;
        let speed_diff_z = target_speed_z - self.vehicle.core.physical.move_speed.z;
        let mut speed_inc_z = time_step * 0.001;
        if self.state.heli_status == HELI_STATUS_FLY_AWAY {
            speed_inc_z *= 1.5;
        }
        if speed_diff_z.abs() < speed_inc_z {
            self.vehicle.core.physical.move_speed.z = target_speed_z;
        } else if speed_diff_z < 0.0 {
            self.vehicle.core.physical.move_speed.z -= speed_inc_z;
        } else {
            self.vehicle.core.physical.move_speed.z += speed_inc_z * 1.5;
        }
        position.z += self.vehicle.core.physical.move_speed.z * time_step;

        self.state.angular_speed *= 0.995_f32.powf(time_step);
        let target_angular_speed = if target_distance < 8.0 {
            0.0
        } else {
            let mut rotation_diff = gta_atan_of_xy(target_dx, target_dy) - self.state.rotation;
            while rotation_diff < -3.14 {
                rotation_diff += 6.28;
            }
            while rotation_diff > 3.14 {
                rotation_diff -= 6.28;
            }
            if rotation_diff.abs() > 0.4 {
                if rotation_diff < 0.0 { -0.2 } else { 0.2 }
            } else {
                0.0
            }
        };
        let angular_diff = target_angular_speed - self.state.angular_speed;
        let angular_inc = time_step * 0.0001;
        if angular_diff.abs() < angular_inc {
            self.state.angular_speed = target_angular_speed;
        } else if angular_diff < 0.0 {
            self.state.angular_speed -= angular_inc;
        } else {
            self.state.angular_speed += angular_inc;
        }
        self.state.rotation += self.state.angular_speed * time_step;

        let up = normalise(CVector::new(
            3.0 * self.vehicle.core.physical.move_speed.x,
            3.0 * self.vehicle.core.physical.move_speed.y,
            1.0,
        ));
        let temporary_forward =
            CVector::new(-self.state.rotation.cos(), -self.state.rotation.sin(), 0.0);
        let right = cross(up, temporary_forward);
        let forward = cross(up, right);
        let entity = &mut self.vehicle.core.physical.entity;
        entity.set_position(position);
        entity.matrix.right = right;
        entity.matrix.forward = forward;
        entity.matrix.up = up;
        self.state.search_light_intensity = 0.0;
        self.vehicle.core.physical.entity.flags.is_in_safe_position = true;

        let crossed_bomb_interval = (now_ms >> 9) != (previous_ms >> 9);
        if self.state.path_state > 8 && crossed_bomb_interval {
            let bomb = CVector::new(
                position.x - 60.0 * self.vehicle.core.physical.move_speed.x,
                position.y - 60.0 * self.vehicle.core.physical.move_speed.y,
                position.z - 60.0 * self.vehicle.core.physical.move_speed.z,
            );
            return Some(bomb);
        }
        None
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn hit_by_rocket(&mut self, now_ms: u32, clockwise: bool) {
        if self.vehicle.core.physical.entity.flags.explosion_proof {
            return;
        }
        self.state.angular_speed = if clockwise { 0.05 } else { -0.05 };
        self.state.heli_status = HELI_STATUS_SHOT_DOWN;
        self.state.explosion_timer = now_ms.wrapping_add(10_000);
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    /// Shot-down branch of `CHeli::ProcessControl`. Collision probes are
    /// supplied by the world adapter so this vehicle module keeps no Bevy
    /// world dependency.
    pub fn process_shot_down_control(
        &mut self,
        now_ms: u32,
        previous_ms: u32,
        frame_counter: u32,
        time_step: f32,
        player_position: CVector,
        ground_z: f32,
        right_clear: bool,
        left_clear: bool,
        forward_or_back_blocked: bool,
    ) {
        if self.state.heli_status != HELI_STATUS_SHOT_DOWN {
            return;
        }
        let mut position = self.vehicle.core.physical.entity.get_position();
        let mut dx = player_position.x - position.x;
        let mut dy = player_position.y - position.y;
        self.state.target_z = ground_z.max(player_position.z) + 8.0 + self.state.target_offset;
        let target_distance = (dx * dx + dy * dy).sqrt();
        if target_distance != 0.0 {
            dx /= target_distance;
            dy /= target_distance;
        } else {
            dx = 1.0;
        }

        let force =
            (frame_counter + u32::from(self.vehicle.core.physical.entity.random_seed)) & 8 != 0;
        let mut target_speed_x = 0.0;
        let mut target_speed_y = 0.0;
        if self.state.test_right {
            if force || right_clear {
                self.state.target_offset -= time_step * 0.05;
                target_speed_x += dx * 0.15;
                target_speed_y -= dy * 0.15;
            } else {
                self.state.test_right = false;
                self.state.target_offset -= 5.0;
            }
        } else if force || left_clear {
            self.state.target_offset -= time_step * 0.05;
            target_speed_x -= dx * 0.15;
            target_speed_y += dy * 0.15;
        } else {
            self.state.test_right = true;
            self.state.target_offset -= 5.0;
        }
        self.state.target_offset = self.state.target_offset.min(30.0);
        if force && forward_or_back_blocked {
            self.state.explosion_timer = previous_ms;
        }

        approach_xy_speed(
            &mut self.vehicle.core.physical.move_speed,
            target_speed_x,
            target_speed_y,
            time_step * 0.002,
        );
        position.x += self.vehicle.core.physical.move_speed.x * time_step;
        position.y += self.vehicle.core.physical.move_speed.y * time_step;

        if now_ms.wrapping_add(800_u32.wrapping_mul(self.state.heli_id.max(0) as u32)) & 0x800 != 0
        {
            self.state.target_z += 2.0;
        }
        self.state.target_z += f32::from(self.state.heli_id) * 5.0;
        let target_speed_z = (self.state.target_z - position.z) * 0.01;
        let speed_diff_z = target_speed_z - self.vehicle.core.physical.move_speed.z;
        let speed_inc_z = time_step * 0.001;
        if speed_diff_z.abs() < speed_inc_z {
            self.vehicle.core.physical.move_speed.z = target_speed_z;
        } else if speed_diff_z < 0.0 {
            self.vehicle.core.physical.move_speed.z -= speed_inc_z;
        } else {
            self.vehicle.core.physical.move_speed.z += speed_inc_z * 1.5;
        }
        position.z += self.vehicle.core.physical.move_speed.z * time_step;

        self.state.angular_speed *= 0.995_f32.powf(time_step);
        let target_angular_speed = if target_distance < 8.0 {
            0.0
        } else {
            let mut rotation_diff = gta_atan_of_xy(dx, dy) - self.state.rotation;
            while rotation_diff < -3.14 {
                rotation_diff += 6.28;
            }
            while rotation_diff > 3.14 {
                rotation_diff -= 6.28;
            }
            if rotation_diff.abs() > 0.4 {
                if rotation_diff < 0.0 { -0.2 } else { 0.2 }
            } else {
                0.0
            }
        };
        let angular_difference = target_angular_speed - self.state.angular_speed;
        let angular_increment = time_step * 0.0001;
        if angular_difference.abs() < angular_increment {
            self.state.angular_speed = target_angular_speed;
        } else if angular_difference < 0.0 {
            self.state.angular_speed -= angular_increment;
        } else {
            self.state.angular_speed += angular_increment;
        }
        self.state.rotation += self.state.angular_speed * time_step;

        let up = normalise(CVector::new(
            3.0 * self.vehicle.core.physical.move_speed.x,
            3.0 * self.vehicle.core.physical.move_speed.y,
            1.0,
        ));
        let temporary_forward =
            CVector::new(-self.state.rotation.cos(), -self.state.rotation.sin(), 0.0);
        let right = cross(up, temporary_forward);
        let forward = cross(up, right);
        let entity = &mut self.vehicle.core.physical.entity;
        entity.set_position(position);
        entity.matrix.right = right;
        entity.matrix.forward = forward;
        entity.matrix.up = up;
        entity.flags.is_in_safe_position = true;
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = HeliState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> HeliRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn hit_by_bullet(&mut self, damage: u32, now_ms: u32, clockwise: bool) {
        if self.vehicle.core.physical.entity.flags.bullet_proof {
            return;
        }
        self.state.bullet_damage = self.state.bullet_damage.saturating_add(damage);
        let threshold = if self.state.heli_type == HELI_TYPE_CATALINA {
            400
        } else {
            700
        };
        if self.state.bullet_damage > threshold {
            self.state.angular_speed = if clockwise { 0.05 } else { -0.05 };
            self.state.heli_status = HELI_STATUS_SHOT_DOWN;
            self.state.explosion_timer = now_ms.wrapping_add(10_000);
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CHeliRuntime {
    pub num_random_helis: i32,
    pub test_for_new_random_helis_timer: u32,
    pub num_script_helis: i32,
    pub catalina_heli_on: bool,
    pub catalina_has_been_shot_down: bool,
    pub script_heli_on: bool,
    pub heli_present: [bool; NUM_HELIS],
    pub heli_handles: [Option<i32>; NUM_HELIS],
    pub path_point: usize,
    pub escape_col_model_set_to_ped1: bool,
    pub chopper_col_model_set_to_ped1: bool,
}

impl CHeliRuntime {
    pub fn init_helis() -> Self {
        Self {
            num_random_helis: 0,
            test_for_new_random_helis_timer: 0,
            num_script_helis: 0,
            catalina_heli_on: false,
            catalina_has_been_shot_down: false,
            script_heli_on: false,
            heli_present: [false; NUM_HELIS],
            heli_handles: [None; NUM_HELIS],
            path_point: 0,
            escape_col_model_set_to_ped1: true,
            chopper_col_model_set_to_ped1: true,
        }
    }

    pub fn start_catalina_fly_by(&mut self) {
        self.catalina_heli_on = true;
        self.catalina_has_been_shot_down = false;
    }

    pub fn catalina_handle(&self) -> Option<i32> {
        self.heli_handles[HELI_CATALINA]
    }

    pub fn clear_catalina(&mut self) -> Option<i32> {
        self.catalina_heli_on = false;
        self.heli_present[HELI_CATALINA] = false;
        self.heli_handles[HELI_CATALINA].take()
    }
}

fn distance(a: CVector, b: CVector) -> f32 {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z).magnitude()
}

fn approach_xy_speed(speed: &mut CVector, target_x: f32, target_y: f32, increment: f32) {
    let dx = target_x - speed.x;
    let dy = target_y - speed.y;
    let difference = (dx * dx + dy * dy).sqrt();
    if difference == 0.0 {
        return;
    }
    if difference < increment {
        speed.x = target_x;
        speed.y = target_y;
    } else {
        speed.x += dx / difference * increment;
        speed.y += dy / difference * increment;
    }
}

fn gta_atan_of_xy(x: f32, y: f32) -> f32 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let angle = y.atan2(x);
    if angle < 0.0 {
        angle + std::f32::consts::TAU
    } else {
        angle
    }
}

fn normalise(value: CVector) -> CVector {
    let magnitude = value.magnitude();
    if magnitude == 0.0 {
        CVector::ZERO
    } else {
        CVector::new(
            value.x / magnitude,
            value.y / magnitude,
            value.z / magnitude,
        )
    }
}

fn cross(a: CVector, b: CVector) -> CVector {
    CVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalina_landing_takeoff_and_shared_path_point_match_reference_states() {
        let mut general = CGeneral::new();
        let mut heli = CHeli::new(147, 4, 0, &mut general);
        heli.heli_type = HELI_TYPE_CATALINA;
        heli.path_state = 6;
        heli.vehicle
            .core
            .physical
            .entity
            .set_position(CVector::new(-1161.0, 341.0, 31.0));
        let mut path_point = 0;
        heli.process_catalina_control(0, 0, 1.0, &mut path_point);
        assert_eq!(heli.path_state, 7);
        // The snap is followed by the common z-speed integrator in the same
        // ProcessControl call (target.z is still CatalinaTargetZ[6] == 30).
        assert!((heli.vehicle.core.physical.entity.get_position().z - 31.549).abs() < 0.0001);

        heli.path_state = 8;
        heli.vehicle
            .core
            .physical
            .entity
            .set_position(CVector::new(-1161.0, 341.0, 40.0));
        heli.process_catalina_control(16, 0, 1.0, &mut path_point);
        assert_eq!(heli.path_state, 9);
        assert_eq!(path_point, 2);
    }

    #[test]
    fn catalina_bullet_threshold_is_strictly_greater_than_four_hundred() {
        let mut general = CGeneral::new();
        let mut heli = CHeli::new(147, 4, 0, &mut general);
        heli.heli_type = HELI_TYPE_CATALINA;
        heli.hit_by_bullet(400, 1000, true);
        assert_ne!(heli.heli_status, HELI_STATUS_SHOT_DOWN);
        heli.hit_by_bullet(1, 1000, true);
        assert_eq!(heli.heli_status, HELI_STATUS_SHOT_DOWN);
        assert_eq!(heli.explosion_timer, 11_000);
    }
}

// === FAZ8c-heli: moved from core/Game.rs (verbatim, parity-preserving) ===

/// re3 `CHeli::UpdateHelis` Catalina mission branch. The helicopter lives in
/// the same generation-checked vehicle pool used by SCM-created vehicles, so
/// `GRAB_CATALINA_HELI` returns a handle accepted by later vehicle opcodes and
/// the renderer sees the live `MI_ESCAPE` transform.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_heli_runtime(
    mut helis: Option<ResMut<CHeliRuntime>>,
    mut script_entities: ScriptEntityWorld,
    mut pools: Option<ResMut<CPools>>,
    mut streaming: Option<ResMut<CStreaming>>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
    world_players: CWorldPlayers,
    file_loader: Res<CFileLoader>,
    water: Option<Res<CWaterLevel>>,
    mut projectiles: Option<ResMut<CProjectileInfoPool>>,
    mut explosions: Option<ResMut<CExplosionPool>>,
) {
    use crate::{modelinfo::ModelIndices::MI_ESCAPE, weapons::WeaponType::eWeaponType};

    let Some(helis) = helis.as_mut() else {
        return;
    };
    if helis.catalina_heli_on && helis.catalina_handle().is_none() {
        let loaded = streaming
            .as_ref()
            .is_some_and(|streaming| streaming.has_model_loaded(MI_ESCAPE));
        if loaded {
            if let Some(pools) = pools.as_mut()
                && let Some(handle) = script_entities.insert_catalina_heli(
                    pools,
                    timer.time_in_milliseconds(),
                    &mut general,
                )
            {
                helis.heli_handles[HELI_CATALINA] = Some(handle);
                helis.heli_present[HELI_CATALINA] = true;
            }
        } else if let Some(streaming) = streaming.as_mut() {
            streaming.request_model(MI_ESCAPE, STREAMFLAGS_DONT_REMOVE);
        }
    }

    let Some(handle) = helis.catalina_handle() else {
        return;
    };
    if !script_entities
        .vehicles
        .contains_key(&script_entities.index, &handle)
    {
        helis.heli_handles[HELI_CATALINA] = None;
        helis.heli_present[HELI_CATALINA] = false;
        return;
    }

    let now_ms = timer.time_in_milliseconds();
    let previous_ms = timer.previous_time_in_milliseconds();
    let player_position = world_players
        .player(world_players.player_in_focus() as i32)
        .map(|player| player.ped.physical.entity.get_position())
        .unwrap_or(CVector::ZERO);

    let mut remove_after_update = false;
    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Heli { mut heli, .. }) =
        script_entities
            .vehicles
            .get_mut(&script_entities.index, &handle)
    {
        if !helis.catalina_heli_on && heli.heli_status != HELI_STATUS_SHOT_DOWN {
            heli.heli_status = HELI_STATUS_FLY_AWAY;
        }

        if heli.heli_status != HELI_STATUS_SHOT_DOWN {
            if let Some(projectiles) = projectiles.as_mut() {
                let heli_position = heli.vehicle.core.physical.entity.get_position();
                let hit_slots: Vec<usize> = projectiles
                    .info
                    .iter()
                    .enumerate()
                    .filter_map(|(index, projectile)| {
                        (projectile.in_use
                            && projectile.weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
                            && CVector::new(
                                projectile.pos.x - heli_position.x,
                                projectile.pos.y - heli_position.y,
                                projectile.pos.z - heli_position.z,
                            )
                            .magnitude_sqr()
                                < 64.0)
                            .then_some(index)
                    })
                    .collect();
                if let Some(index) = hit_slots.first().copied() {
                    heli.hit_by_rocket(now_ms, general.get_random_true_false());
                    projectiles.RemoveProjectile(index);
                }
            }
        }

        if heli.heli_status != HELI_STATUS_SHOT_DOWN {
            if let Some(mut bomb_position) = heli.process_catalina_control(
                now_ms,
                previous_ms,
                timer.time_step(),
                &mut helis.path_point,
            ) {
                let dx = player_position.x - bomb_position.x;
                let dy = player_position.y - bomb_position.y;
                if dx * dx + dy * dy < 35.0 * 35.0 {
                    let ground_z = file_loader
                        .world
                        .find_ground_z_for_3d_coord(
                            bomb_position.x,
                            bomb_position.y,
                            bomb_position.z,
                            &file_loader.model_info,
                        )
                        .unwrap_or(0.0);
                    let water_z = water
                        .as_ref()
                        .and_then(|water| {
                            water
                                .get_water_level_no_waves(
                                    bomb_position.x,
                                    bomb_position.y,
                                    bomb_position.z,
                                )
                                .map(|sample| sample.level)
                        })
                        .unwrap_or(0.0);
                    if ground_z > water_z {
                        bomb_position.z = ground_z + 2.0;
                        if let Some(explosions) = explosions.as_mut() {
                            explosions.AddExplosion(
                                None,
                                Some(crate::entities::EntityReference::EntityReference::Vehicle(handle)),
                                eExplosionType::EXPLOSION_HELI_BOMB,
                                bomb_position,
                                0,
                                now_ms,
                            );
                        }
                    }
                }
            }
        } else {
            let position = heli.vehicle.core.physical.entity.get_position();
            let right = heli.vehicle.core.physical.entity.get_right();
            let forward = heli.vehicle.core.physical.entity.get_forward();
            let sphere_blocked = |point: CVector| {
                file_loader.world.test_sphere_against_static_world(
                    [point.x, point.y, point.z],
                    2.0,
                    &file_loader.model_info,
                )
            };
            let right_clear = !sphere_blocked(CVector::new(
                position.x + 4.0 * right.x,
                position.y + 4.0 * right.y,
                position.z + 4.0 * right.z,
            ));
            let left_clear = !sphere_blocked(CVector::new(
                position.x - 4.0 * right.x,
                position.y - 4.0 * right.y,
                position.z - 4.0 * right.z,
            ));
            let forward_or_back_blocked = sphere_blocked(CVector::new(
                position.x + 1.5 * forward.x,
                position.y + 1.5 * forward.y,
                position.z + 1.5 * forward.z,
            )) || sphere_blocked(CVector::new(
                position.x - 1.5 * forward.x,
                position.y - 1.5 * forward.y,
                position.z - 1.5 * forward.z,
            ));
            let ground_z = file_loader
                .world
                .find_ground_z_for_3d_coord(position.x, position.y, 1000.0, &file_loader.model_info)
                .unwrap_or(0.0);
            heli.process_shot_down_control(
                now_ms,
                previous_ms,
                timer.frame_counter(),
                timer.time_step(),
                player_position,
                ground_z,
                right_clear,
                left_clear,
                forward_or_back_blocked,
            );
        }

        if heli.heli_status == HELI_STATUS_SHOT_DOWN && now_ms > heli.explosion_timer {
            if let Some(explosions) = explosions.as_mut() {
                explosions.AddExplosion(
                    None,
                    None,
                    eExplosionType::EXPLOSION_HELI,
                    heli.vehicle.core.physical.entity.get_position(),
                    0,
                    now_ms,
                );
            }
            helis.catalina_has_been_shot_down = true;
            remove_after_update = true;
        } else if heli.heli_status == HELI_STATUS_SHOT_DOWN
            && now_ms.wrapping_add(7_000) > heli.explosion_timer
        {
            if previous_ms.wrapping_add(7_000) < heli.explosion_timer {
                heli.angular_speed *= -2.5;
                heli.vehicle.core.physical.entity.flags.render_scorched = true;
                let position = heli.vehicle.core.physical.entity.get_position();
                let forward = heli.vehicle.core.physical.entity.get_forward();
                if let Some(explosions) = explosions.as_mut() {
                    explosions.AddExplosion(
                        None,
                        None,
                        eExplosionType::EXPLOSION_HELI,
                        CVector::new(
                            position.x - 2.5 * forward.x,
                            position.y - 2.5 * forward.y,
                            position.z - 2.5 * forward.z,
                        ),
                        0,
                        now_ms,
                    );
                }
            } else {
                heli.angular_speed *= 1.03;
            }
        }

        if heli.heli_status == HELI_STATUS_FLY_AWAY
            && heli.vehicle.core.physical.entity.get_position().z > 150.0
        {
            remove_after_update = true;
        }
        heli.rotor_rotation += 3.14 / 6.5;
        if heli.rotor_rotation > 6.28 {
            heli.rotor_rotation -= 6.28;
        }
    }

    if remove_after_update {
        if let Some(pools) = pools.as_mut() {
            script_entities.remove_car(pools, handle);
        } else {
            script_entities.despawn_vehicle(handle);
        }
        helis.heli_handles[HELI_CATALINA] = None;
        helis.heli_present[HELI_CATALINA] = false;
    }
}
