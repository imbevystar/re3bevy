//! re3 reference: references/re-GTA/src/vehicles/Plane.cpp
//! re3 reference: references/re-GTA/src/vehicles/Plane.h
//!
//! Plane vehicle state.

use std::io;

use bevy::prelude::{Res, ResMut, Resource};
use rust_gta_3_assets::install::FileMgr::CFileMgr;
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    audio::{DMAudio::cDMAudio, soundlist::SOUND_PLANE_ON_GROUND},
    control::Replay::{CReplay, ReplayMode},
    core::{Camera::CCamera, Game::ScriptEntityWorld, Pools::CPools, Streaming::CStreaming},
    entities::Entity::eEntityStatus,
    modelinfo::{
        ModelIndices::{MI_AIRTRAIN, MI_DEADDODO},
        VehicleModelInfo::eVehicleType,
    },
    vehicles::Vehicle::CVehicle,
};

pub const PLANE_WHEEL_FRONT: usize = 2;
pub const PLANE_WHEEL_READ: usize = 3;
pub const NUM_PLANE_NODES: usize = 4;
pub const CRUISE_SPEED: f32 = 50.0;
pub const TAXI_SPEED: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CesnaMissionStatus {
    None = 0,
    Flying = 1,
    Destroyed = 2,
    Landed = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPlaneNode {
    pub p: CVector,
    pub t: f32,
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPlaneInterpolationLine {
    pub line_type: u8,
    pub time: f32,
    pub position: f32,
    pub speed: f32,
    pub acceleration: f32,
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct PlaneState {
    pub plane_id: i16,
    pub is_far_away: i16,
    pub cur_path_node: i16,
    pub speed: f32,
    pub frame_when_hit: u32,
    pub has_been_hit: bool,
    pub is_drug_run_cesna: bool,
    pub is_drop_off_cesna: bool,
}
use crate::vehicles::Vehicle::{Owned, VehicleCore, VehicleRuntime, VehicleVitality};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio> {
    pub vehicle: VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>,
    pub state: S,
}
pub type CPlane = PlaneRuntimeView<
    Owned<VehicleCore>,
    Owned<VehicleVitality>,
    Owned<PlaneState>,
    crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleDriveState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAppearance>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleEntryState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAudioState>>;
pub type PlaneRef<'a> = PlaneRuntimeView<
    &'a VehicleCore,
    &'a VehicleVitality,
    &'a PlaneState,
    &'a crate::vehicles::Vehicle::VehicleDriveState, &'a crate::vehicles::Vehicle::VehicleAppearance, &'a crate::vehicles::Vehicle::VehicleEntryState, &'a crate::vehicles::Vehicle::VehicleAudioState>;
pub type PlaneMut<'a> = PlaneRuntimeView<
    &'a mut VehicleCore,
    &'a mut VehicleVitality,
    &'a mut PlaneState,
    &'a mut crate::vehicles::Vehicle::VehicleDriveState, &'a mut crate::vehicles::Vehicle::VehicleAppearance, &'a mut crate::vehicles::Vehicle::VehicleEntryState, &'a mut crate::vehicles::Vehicle::VehicleAudioState>;
impl<
    C,
    V,
    S: std::ops::Deref<Target = PlaneState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::Deref for PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    type Target = PlaneState;
    fn deref(&self) -> &PlaneState {
        &self.state
    }
}
impl<
    C,
    V,
    S: std::ops::DerefMut<Target = PlaneState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::DerefMut for PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    fn deref_mut(&mut self) -> &mut PlaneState {
        &mut self.state
    }
}
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = PlaneState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_ref(&self) -> PlaneRef<'_> {
        PlaneRuntimeView {
            vehicle: self.vehicle.as_ref(),
            state: &self.state,
        }
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = PlaneState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_mut(&mut self) -> PlaneMut<'_> {
        PlaneRuntimeView {
            vehicle: self.vehicle.as_mut(),
            state: &mut self.state,
        }
    }
}
impl CPlane {
    pub fn into_parts(
        self,
    ) -> (
        VehicleCore,
        VehicleVitality,
        crate::vehicles::Vehicle::VehicleDriveState,
        crate::vehicles::Vehicle::VehicleAppearance,
        crate::vehicles::Vehicle::VehicleEntryState,
        crate::vehicles::Vehicle::VehicleAudioState,
        PlaneState,
    ) {
        let (core, vitality, drive, appearance, entry, audio) = self.vehicle.into_parts();
        (core, vitality, drive, appearance, entry, audio, self.state.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CPlaneProcessControlResult {
    pub transform_updated: bool,
    pub plane_on_ground_sound: bool,
    pub destroy_when_next_processed: bool,
}

impl CPlane {
    pub fn new(id: i32, created_by: u8, time_ms: u32, general: &mut CGeneral) -> Self {
        let mut vehicle = CVehicle::new(created_by, time_ms, general);
        vehicle.vehicle_type = eVehicleType::VEHICLE_TYPE_PLANE;
        vehicle
            .physical
            .entity
            .set_model_index_no_create(id as usize);
        vehicle.physical.mass = 100000000.0;
        vehicle.physical.turn_mass = 100000000.0;
        vehicle.physical.air_resistance = 0.9994;
        vehicle.physical.elasticity = 0.05;
        vehicle.physical.entity.flags.uses_collision = false;
        vehicle.set_status(eEntityStatus::STATUS_PLANE);
        vehicle.physical.entity.flags.is_big_building = true;
        vehicle.physical.entity.level = 0;

        Self {
            vehicle,
            state: Owned(PlaneState {
                plane_id: 0,
                is_far_away: 0,
                cur_path_node: 0,
                speed: 0.0,
                frame_when_hit: 0,
                has_been_hit: false,
                is_drug_run_cesna: false,
                is_drop_off_cesna: false,
            }),
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = PlaneState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> PlaneRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    /// The path/matrix half of re3 `CPlane::ProcessControl`.
    ///
    /// Explosion particles, audio dispatch and model streaming are adapters of
    /// the returned state. This method owns the original path-node cursor,
    /// orientation, linear speed, far-LOD test and mission-removal transition.
    pub fn process_path_control(
        &mut self,
        runtime: &CPlaneRuntime,
        camera_position: CVector,
        frame_counter: u32,
    ) -> CPlaneProcessControlResult {
        let model_index =
            usize::try_from(self.vehicle.core.physical.entity.model_index).unwrap_or(0);
        if model_index != MI_AIRTRAIN
            && self.state.is_far_away != 0
            && (frame_counter
                .wrapping_add(u32::from(self.vehicle.core.physical.entity.random_seed))
                & 7)
                != 0
        {
            self.vehicle.core.physical.entity.flags.is_in_safe_position = true;
            return CPlaneProcessControlResult::default();
        }

        let mut result = CPlaneProcessControlResult::default();
        let path = if model_index == MI_AIRTRAIN {
            let id = usize::try_from(self.state.plane_id).unwrap_or(0).min(2);
            PlanePathControlInput {
                nodes: &runtime.path_nodes,
                total_length: runtime.total_length[0],
                path_position: runtime.plane_path_position[id],
                path_speed: runtime.plane_path_speed[id],
                rear_offset: 30.0,
                front_offset: 60.0,
                roll_probe_offset: 60.0,
                height_offset: 4.3,
                pitch: airtrain_pitch(
                    runtime.plane_path_position[id],
                    runtime.take_off_point,
                    runtime.landing_point,
                ),
                suppress_roll_on_ground: true,
            }
        } else if self.state.is_drug_run_cesna {
            if runtime.cesna_mission_status == CesnaMissionStatus::Landed {
                self.vehicle.core.physical.entity.flags.remove_from_world = true;
                result.destroy_when_next_processed = true;
            }
            PlanePathControlInput::cesna(
                &runtime.path3_nodes,
                runtime.total_length[2],
                runtime.plane_path3_position,
                runtime.plane_path3_speed,
            )
        } else if self.state.is_drop_off_cesna {
            if runtime.drop_off_mission_status == CesnaMissionStatus::Landed {
                self.vehicle.core.physical.entity.flags.remove_from_world = true;
                result.destroy_when_next_processed = true;
            }
            PlanePathControlInput::cesna(
                &runtime.path4_nodes,
                runtime.total_length[3],
                runtime.plane_path4_position,
                runtime.plane_path4_speed,
            )
        } else {
            let id = usize::try_from(self.state.plane_id).unwrap_or(0).min(2);
            PlanePathControlInput::cesna(
                &runtime.path2_nodes,
                runtime.total_length[1],
                runtime.plane_path2_position[id],
                runtime.plane_path2_speed[id],
            )
        };

        let Some(sample) = sample_plane_path(path, &mut self.state.cur_path_node) else {
            self.vehicle.core.physical.entity.flags.is_in_safe_position = true;
            return result;
        };

        let mut matrix = self.vehicle.core.physical.entity.matrix;
        matrix.set_translate_only(sample.position);
        matrix.right = sample.right;
        matrix.forward = sample.forward;
        matrix.up = sample.up;
        self.vehicle.core.physical.entity.set_matrix(matrix);
        self.vehicle.core.physical.move_speed = scale(sample.forward, path.path_speed / 60.0);
        self.vehicle.core.physical.turn_speed = CVector::ZERO;
        self.state.speed = path.path_speed / 60.0;
        self.state.is_far_away = i16::from(
            magnitude_sqr_2d(sub(sample.front_position, camera_position)) >= 300.0 * 300.0,
        );
        self.vehicle.core.physical.entity.flags.is_in_safe_position = true;

        result.transform_updated = true;
        result.plane_on_ground_sound = model_index == MI_AIRTRAIN
            && runtime.plane_path_position
                [usize::try_from(self.state.plane_id).unwrap_or(0).min(2)]
                >= runtime.landing_point
            && runtime.old_plane_path_position
                [usize::try_from(self.state.plane_id).unwrap_or(0).min(2)]
                < runtime.landing_point;
        result
    }
}

#[derive(Clone, Copy)]
struct PlanePathControlInput<'a> {
    nodes: &'a [CPlaneNode],
    total_length: f32,
    path_position: f32,
    path_speed: f32,
    rear_offset: f32,
    front_offset: f32,
    roll_probe_offset: f32,
    height_offset: f32,
    pitch: f32,
    suppress_roll_on_ground: bool,
}

impl<'a> PlanePathControlInput<'a> {
    fn cesna(
        nodes: &'a [CPlaneNode],
        total_length: f32,
        path_position: f32,
        path_speed: f32,
    ) -> Self {
        Self {
            nodes,
            total_length,
            path_position,
            path_speed,
            rear_offset: 10.0,
            front_offset: 20.0,
            roll_probe_offset: 30.0,
            height_offset: 1.0,
            pitch: 0.0,
            suppress_roll_on_ground: false,
        }
    }
}

#[derive(Clone, Copy)]
struct PlanePathSample {
    position: CVector,
    front_position: CVector,
    right: CVector,
    forward: CVector,
    up: CVector,
}

fn sample_plane_path(
    input: PlanePathControlInput<'_>,
    current_node: &mut i16,
) -> Option<PlanePathSample> {
    if input.nodes.len() < 2 || input.total_length <= 0.0 {
        return None;
    }

    let rear_path_position =
        wrap_path_position(input.path_position - input.rear_offset, input.total_length);
    let (rear_position, rear_node, rear_next) = interpolate_path_from_cursor(
        input.nodes,
        input.total_length,
        rear_path_position,
        usize::try_from(*current_node).unwrap_or(0),
    )?;
    *current_node = rear_node as i16;
    let front_path_position =
        wrap_path_position(rear_path_position + input.front_offset, input.total_length);
    let (front_position, _, _) = interpolate_path_from_cursor(
        input.nodes,
        input.total_length,
        front_path_position,
        rear_node,
    )?;
    let roll_path_position = wrap_path_position(
        front_path_position + input.roll_probe_offset,
        input.total_length,
    );
    let (roll_position, _, _) = interpolate_path_from_cursor(
        input.nodes,
        input.total_length,
        roll_path_position,
        rear_node,
    )?;

    let mut forward = normalise(sub(front_position, rear_position));
    if input.pitch != 0.0 {
        forward.z += 0.4 * input.pitch;
        forward = normalise(forward);
    }
    let forward2 = normalise(sub(roll_position, rear_position));
    let roll = cross(forward, forward2);
    let mut right = cross(forward, CVector::new(0.0, 0.0, 1.0));
    let both_on_ground = input.nodes[rear_node].on_ground && input.nodes[rear_next].on_ground;
    if !input.suppress_roll_on_ground || !both_on_ground {
        right.z += 3.0 * roll.z;
    }
    right = normalise(right);
    let up = cross(right, forward);
    let mut position = scale(add(rear_position, front_position), 0.5);
    position.z += input.height_offset;

    Some(PlanePathSample {
        position,
        front_position,
        right,
        forward,
        up,
    })
}

fn interpolate_path_from_cursor(
    nodes: &[CPlaneNode],
    total_length: f32,
    path_position: f32,
    mut current: usize,
) -> Option<(CVector, usize, usize)> {
    current %= nodes.len();
    for _ in 0..nodes.len() {
        let next = (current + 1) % nodes.len();
        let end = if next == 0 {
            total_length
        } else {
            nodes[next].t
        };
        if path_position >= nodes[current].t && path_position <= end {
            let distance = end - nodes[current].t;
            if distance <= 0.0 {
                return None;
            }
            let fraction = (path_position - nodes[current].t) / distance;
            return Some((
                add(
                    scale(nodes[current].p, 1.0 - fraction),
                    scale(nodes[next].p, fraction),
                ),
                current,
                next,
            ));
        }
        current = next;
    }
    None
}

fn airtrain_pitch(path_position: f32, take_off_point: f32, landing_point: f32) -> f32 {
    let mut pitch = 0.0;
    let take_off_distance = path_position - take_off_point;
    if take_off_distance <= 0.0 && take_off_distance > -70.0 {
        pitch = 1.0 - take_off_distance / -70.0;
    } else if take_off_distance >= 0.0 && take_off_distance < 100.0 {
        pitch = 1.0 - take_off_distance / 100.0;
    }
    let landing_distance = path_position - landing_point;
    if landing_distance <= 0.0 && landing_distance > -200.0 {
        pitch = 1.0 - landing_distance / -200.0;
    } else if landing_distance >= 0.0 && landing_distance < 70.0 {
        pitch = 1.0 - landing_distance / 70.0;
    }
    pitch
}

fn add(left: CVector, right: CVector) -> CVector {
    CVector::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn sub(left: CVector, right: CVector) -> CVector {
    CVector::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale(value: CVector, scalar: f32) -> CVector {
    CVector::new(value.x * scalar, value.y * scalar, value.z * scalar)
}

fn cross(left: CVector, right: CVector) -> CVector {
    CVector::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn normalise(value: CVector) -> CVector {
    let magnitude = value.magnitude();
    if magnitude > 0.0 {
        scale(value, magnitude.recip())
    } else {
        CVector::new(1.0, 0.0, 0.0)
    }
}

fn magnitude_sqr_2d(value: CVector) -> f32 {
    value.x * value.x + value.y * value.y
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CPlaneRuntime {
    pub path_nodes: Vec<CPlaneNode>,
    pub path2_nodes: Vec<CPlaneNode>,
    pub path3_nodes: Vec<CPlaneNode>,
    pub path4_nodes: Vec<CPlaneNode>,
    pub total_length: [f32; 4],
    pub total_duration: [f32; 4],
    pub landing_point: f32,
    pub take_off_point: f32,
    pub interpolation_lines: [CPlaneInterpolationLine; 6],
    pub plane_path_position: [f32; 3],
    pub old_plane_path_position: [f32; 3],
    pub plane_path_speed: [f32; 3],
    pub plane_path2_position: [f32; 3],
    pub plane_path2_speed: [f32; 3],
    pub plane_path3_position: f32,
    pub plane_path3_speed: f32,
    pub plane_path4_position: f32,
    pub plane_path4_speed: f32,
    pub cesna_mission_status: CesnaMissionStatus,
    pub cesna_mission_start_time: u32,
    pub drop_off_mission_status: CesnaMissionStatus,
    pub drop_off_mission_start_time: u32,
}

impl CPlaneRuntime {
    /// Data/curve portion of `CPlane::InitPlanes`. The callback is the exact
    /// building+dummy vertical-line query used to clamp jumbo nodes to ground.
    pub fn initialise<F>(file_mgr: &mut CFileMgr, mut ground_z: F) -> io::Result<Self>
    where
        F: FnMut(CVector, f32) -> Option<f32>,
    {
        let (mut path_nodes, total_length_1) =
            load_path(file_mgr, "data\\paths\\flight.dat", true)?;
        let (path2_nodes, total_length_2) = load_path(file_mgr, "data\\paths\\flight2.dat", true)?;
        let (path3_nodes, total_length_3) = load_path(file_mgr, "data\\paths\\flight3.dat", false)?;
        let (path4_nodes, total_length_4) = load_path(file_mgr, "data\\paths\\flight4.dat", false)?;

        for node in &mut path_nodes {
            if let Some(z) = ground_z(node.p, 1000.0) {
                node.p.z = z;
                node.on_ground = true;
            } else {
                node.on_ground = false;
            }
        }
        let mut landing_point = -1.0;
        let mut take_off_point = -1.0;
        if let Some(last) = path_nodes.last() {
            let mut last_on_ground = last.on_ground;
            for node in &path_nodes {
                if node.on_ground && !last_on_ground {
                    landing_point = node.t;
                } else if !node.on_ground && last_on_ground {
                    take_off_point = node.t;
                }
                last_on_ground = node.on_ground;
            }
        }

        let mut lines = [CPlaneInterpolationLine::default(); 6];
        let mut time = 0.0;
        let mut position = 0.0;
        lines[0] = plane_line(1, time, position, TAXI_SPEED, 0.0);
        let mut distance = (take_off_point - 600.0) - position;
        time += distance / TAXI_SPEED;
        position += distance;
        lines[1] = plane_line(2, time, position, TAXI_SPEED, 618.75 / 600.0);
        time += 600.0 / ((CRUISE_SPEED + TAXI_SPEED) / 2.0);
        position += 600.0;
        lines[2] = plane_line(1, time, position, CRUISE_SPEED, 0.0);
        distance = landing_point - take_off_point;
        time += distance / CRUISE_SPEED;
        position += distance;
        lines[3] = plane_line(2, time, position, CRUISE_SPEED, -618.75 / 600.0);
        time += 600.0 / ((CRUISE_SPEED + TAXI_SPEED) / 2.0);
        position += 600.0;
        lines[4] = plane_line(1, time, position, TAXI_SPEED, 0.0);
        time += (total_length_1 - position) / TAXI_SPEED;
        lines[5] = plane_line(0, time, 0.0, 0.0, 0.0);

        Ok(Self {
            path_nodes,
            path2_nodes,
            path3_nodes,
            path4_nodes,
            total_length: [
                total_length_1,
                total_length_2,
                total_length_3,
                total_length_4,
            ],
            total_duration: [
                time,
                total_length_2 / CRUISE_SPEED,
                total_length_3 / CRUISE_SPEED,
                total_length_4 / CRUISE_SPEED,
            ],
            landing_point,
            take_off_point,
            interpolation_lines: lines,
            plane_path_position: [0.0; 3],
            old_plane_path_position: [0.0; 3],
            plane_path_speed: [0.0; 3],
            plane_path2_position: [0.0; 3],
            plane_path2_speed: [0.0; 3],
            plane_path3_position: 0.0,
            plane_path3_speed: 0.0,
            plane_path4_position: 0.0,
            plane_path4_speed: 0.0,
            cesna_mission_status: CesnaMissionStatus::None,
            cesna_mission_start_time: 0,
            drop_off_mission_status: CesnaMissionStatus::None,
            drop_off_mission_start_time: 0,
        })
    }

    pub fn update_planes(&mut self, time_ms: u32, replay_playback: bool) {
        if replay_playback {
            return;
        }
        let mut phase_time = time_ms;
        for index in 0..3 {
            let t = self.total_duration[0] * (phase_time & 0x7FFFF) as f32 / 0x80000 as f32;
            let mut line = 0usize;
            while line + 1 < self.interpolation_lines.len()
                && t > self.interpolation_lines[line + 1].time
            {
                line += 1;
            }
            line = line.min(4);
            self.old_plane_path_position[index] = self.plane_path_position[index];
            let key = self.interpolation_lines[line];
            let delta_t = t - key.time;
            match key.line_type {
                0 => {
                    self.plane_path_position[index] = key.position;
                    self.plane_path_speed[index] = 0.0;
                }
                1 => {
                    self.plane_path_position[index] = key.position + key.speed * delta_t;
                    self.plane_path_speed[index] =
                        (self.total_duration[0] * 1000.0 / 0x80000 as f32) * key.speed;
                }
                2 => {
                    self.plane_path_position[index] =
                        key.position + (key.speed + key.acceleration * delta_t) * delta_t;
                    self.plane_path_speed[index] =
                        (self.total_duration[0] * 1000.0 / 0x80000 as f32) * key.speed
                            + 2.0 * key.acceleration * delta_t;
                }
                _ => {}
            }
            phase_time = phase_time.wrapping_add(0x80000 / 3);
        }

        let path2_scale = self.total_duration[1] / 0x80000 as f32;
        for index in 0..3 {
            let offset = (0x80000 / 3 * index as u32) as u32;
            self.plane_path2_position[index] =
                CRUISE_SPEED * (time_ms.wrapping_add(offset) & 0x7FFFF) as f32 * path2_scale;
            self.plane_path2_speed[index] = CRUISE_SPEED * path2_scale;
        }
        if self.cesna_mission_status == CesnaMissionStatus::Flying {
            self.plane_path3_speed = CRUISE_SPEED * self.total_duration[2] / 0x20000 as f32;
            let elapsed = time_ms.wrapping_sub(self.cesna_mission_start_time);
            self.plane_path3_position = self.plane_path3_speed * (elapsed & 0x1FFFF) as f32;
            if elapsed >= 128_072 {
                self.cesna_mission_status = CesnaMissionStatus::Landed;
            }
        }
        if self.drop_off_mission_status == CesnaMissionStatus::Flying {
            self.plane_path4_speed = CRUISE_SPEED * self.total_duration[3] / 0x80000 as f32;
            let elapsed = time_ms.wrapping_sub(self.drop_off_mission_start_time);
            self.plane_path4_position = self.plane_path4_speed * (elapsed & 0x7FFFF) as f32;
            if elapsed >= 521_288 {
                self.drop_off_mission_status = CesnaMissionStatus::Landed;
            }
        }
    }

    pub fn create_incoming_cesna(&mut self, time_ms: u32) {
        self.cesna_mission_status = CesnaMissionStatus::Flying;
        self.cesna_mission_start_time = time_ms;
    }

    pub fn create_drop_off_cesna(&mut self, time_ms: u32) {
        self.drop_off_mission_status = CesnaMissionStatus::Flying;
        self.drop_off_mission_start_time = time_ms;
    }

    pub fn has_cesna_landed(&self) -> bool {
        self.cesna_mission_status == CesnaMissionStatus::Landed
    }

    pub fn has_cesna_been_destroyed(&self) -> bool {
        self.cesna_mission_status == CesnaMissionStatus::Destroyed
    }

    pub fn has_drop_off_cesna_been_shot_down(&self) -> bool {
        self.drop_off_mission_status == CesnaMissionStatus::Destroyed
    }

    pub fn drug_plane_coordinates(&self) -> Option<CVector> {
        if matches!(
            self.cesna_mission_status,
            CesnaMissionStatus::None | CesnaMissionStatus::Destroyed
        ) {
            return None;
        }
        mission_plane_position(
            &self.path3_nodes,
            self.total_length[2],
            self.plane_path3_position,
        )
    }

    pub fn drop_off_plane_coordinates(&self) -> Option<CVector> {
        if matches!(
            self.drop_off_mission_status,
            CesnaMissionStatus::None | CesnaMissionStatus::Destroyed
        ) {
            return None;
        }
        mission_plane_position(
            &self.path4_nodes,
            self.total_length[3],
            self.plane_path4_position,
        )
    }
}

fn mission_plane_position(
    nodes: &[CPlaneNode],
    total_length: f32,
    path_position: f32,
) -> Option<CVector> {
    if nodes.len() < 2 || total_length <= 0.0 {
        return None;
    }
    let rear = wrap_path_position(path_position - 10.0, total_length);
    let front = wrap_path_position(rear + 20.0, total_length);
    let rear_position = interpolate_path_position(nodes, total_length, rear)?;
    let front_position = interpolate_path_position(nodes, total_length, front)?;
    Some(CVector::new(
        (rear_position.x + front_position.x) * 0.5,
        (rear_position.y + front_position.y) * 0.5,
        (rear_position.z + front_position.z) * 0.5 + 1.0,
    ))
}

fn wrap_path_position(mut position: f32, total_length: f32) -> f32 {
    while position < 0.0 {
        position += total_length;
    }
    while position > total_length {
        position -= total_length;
    }
    position
}

fn interpolate_path_position(
    nodes: &[CPlaneNode],
    total_length: f32,
    path_position: f32,
) -> Option<CVector> {
    for current in 0..nodes.len() {
        let next = (current + 1) % nodes.len();
        let start = nodes[current].t;
        let end = if next == 0 {
            total_length
        } else {
            nodes[next].t
        };
        if path_position < start || path_position > end {
            continue;
        }
        let distance = end - start;
        if distance <= f32::EPSILON {
            return Some(nodes[current].p);
        }
        let fraction = (path_position - start) / distance;
        return Some(CVector::new(
            nodes[current].p.x + (nodes[next].p.x - nodes[current].p.x) * fraction,
            nodes[current].p.y + (nodes[next].p.y - nodes[current].p.y) * fraction,
            nodes[current].p.z + (nodes[next].p.z - nodes[current].p.z) * fraction,
        ));
    }
    None
}

impl Default for CPlaneInterpolationLine {
    fn default() -> Self {
        plane_line(0, 0.0, 0.0, 0.0, 0.0)
    }
}

fn plane_line(
    line_type: u8,
    time: f32,
    position: f32,
    speed: f32,
    acceleration: f32,
) -> CPlaneInterpolationLine {
    CPlaneInterpolationLine {
        line_type,
        time,
        position,
        speed,
        acceleration,
    }
}

fn load_path(
    file_mgr: &mut CFileMgr,
    filename: &str,
    loop_path: bool,
) -> io::Result<(Vec<CPlaneNode>, f32)> {
    let bytes = file_mgr.load_file(filename, 64 * 1024, "r")?;
    let text = std::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(&bytes)).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{filename} is not text"),
        )
    })?;
    let mut lines = text.lines();
    let num_nodes = lines
        .next()
        .and_then(|line| line.trim().parse::<usize>().ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {filename} node count"),
            )
        })?;
    let mut nodes = Vec::with_capacity(num_nodes);
    for _ in 0..num_nodes {
        let source = lines.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("truncated {filename}"),
            )
        })?;
        let mut values = source.split_whitespace().map(str::parse::<f32>);
        let p = CVector::new(
            values
                .next()
                .and_then(Result::ok)
                .ok_or_else(|| invalid_path_row(filename, source))?,
            values
                .next()
                .and_then(Result::ok)
                .ok_or_else(|| invalid_path_row(filename, source))?,
            values
                .next()
                .and_then(Result::ok)
                .ok_or_else(|| invalid_path_row(filename, source))?,
        );
        nodes.push(CPlaneNode {
            p,
            t: 0.0,
            on_ground: false,
        });
    }
    if nodes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{filename} has no nodes"),
        ));
    }
    let mut total_length = 0.0;
    for index in 0..nodes.len() {
        nodes[index].t = total_length;
        if loop_path || index + 1 < nodes.len() {
            let next = nodes[(index + 1) % nodes.len()].p;
            let dx = next.x - nodes[index].p.x;
            let dy = next.y - nodes[index].p.y;
            total_length += (dx * dx + dy * dy).sqrt();
        }
    }
    Ok((nodes, total_length))
}

fn invalid_path_row(filename: &str, row: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {filename} row: {row}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_plane_position_matches_rear_front_midpoint() {
        let nodes = [
            CPlaneNode {
                p: CVector::new(0.0, 0.0, 10.0),
                t: 0.0,
                on_ground: false,
            },
            CPlaneNode {
                p: CVector::new(100.0, 0.0, 10.0),
                t: 100.0,
                on_ground: false,
            },
            CPlaneNode {
                p: CVector::new(200.0, 0.0, 10.0),
                t: 200.0,
                on_ground: false,
            },
        ];
        assert_eq!(
            mission_plane_position(&nodes, 200.0, 50.0),
            Some(CVector::new(50.0, 0.0, 11.0))
        );
    }
}

// === FAZ8c-plane: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn update_plane_runtime(
    mut planes: Option<ResMut<CPlaneRuntime>>,
    mut script_entities: ScriptEntityWorld,
    mut pools: Option<ResMut<CPools>>,
    camera: Option<Res<CCamera>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    timer: Res<CTimer>,
    replay: Option<Res<CReplay>>,
) {
    if let Some(planes) = planes.as_mut() {
        planes.update_planes(
            timer.time_in_milliseconds(),
            replay
                .as_ref()
                .is_some_and(|replay| replay.mode == ReplayMode::Playback),
        );

        let stale_handles = script_entities
            .vehicles
            .iter()
            .filter_map(|(&handle, vehicle)| match vehicle {
                crate::control::ScriptEntities::ScriptVehicleRef::Plane { plane, .. }
                    if plane.vehicle.core.physical.entity.flags.remove_from_world =>
                {
                    Some(handle)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for handle in stale_handles {
            if let Some(pools) = pools.as_mut() {
                script_entities.remove_car(pools, handle);
            } else {
                script_entities.despawn_vehicle(handle);
            }
        }

        let Some(camera) = camera else {
            return;
        };
        for vehicle in script_entities.vehicles.values_mut() {
            let crate::control::ScriptEntities::ScriptVehicleMut::Plane { mut plane, .. } = vehicle
            else {
                continue;
            };
            let result = plane.process_path_control(planes, camera.position, timer.frame_counter());
            if result.plane_on_ground_sound
                && plane.vehicle.core.physical.audio_entity_id >= 0
                && let Some(dmaudio) = dmaudio.as_mut()
            {
                dmaudio.PlayOneShot(
                    plane.vehicle.core.physical.audio_entity_id,
                    SOUND_PLANE_ON_GROUND as u16,
                    0.0,
                );
            }
        }
    }
}

pub(crate) fn initialise_plane_entities(
    planes: Option<Res<CPlaneRuntime>>,
    mut script_entities: ScriptEntityWorld,
    mut pools: Option<ResMut<CPools>>,
    mut streaming: ResMut<CStreaming>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
) {
    if planes.is_none()
        || script_entities.vehicles.values().any(|vehicle| {
            matches!(
                vehicle,
                crate::control::ScriptEntities::ScriptVehicleRef::Plane { .. }
            )
        })
    {
        return;
    }
    let Some(pools) = pools.as_mut() else {
        return;
    };

    streaming.request_model(MI_AIRTRAIN, 0);
    streaming.request_model(MI_DEADDODO, 0);
    for plane_id in 0..3_i16 {
        let _ = script_entities.insert_plane(
            pools,
            MI_AIRTRAIN,
            plane_id,
            timer.time_in_milliseconds(),
            &mut general,
            false,
            false,
        );
    }
    for plane_id in 0..3_i16 {
        let _ = script_entities.insert_plane(
            pools,
            MI_DEADDODO,
            plane_id,
            timer.time_in_milliseconds(),
            &mut general,
            false,
            false,
        );
    }
}
