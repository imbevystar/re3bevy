//! re3 reference: references/re-GTA/src/renderer/WaterCannon.cpp
//! re3 reference: references/re-GTA/src/renderer/WaterCannon.h

use bevy::prelude::*;
use rustgta3_core::core::{
    Coordinates::CVector, General::CGeneral, Timer::CTimer,
};

use crate::{
    animation::{
        AnimationId::ANIM_STD_HIGHIMPACT_FRONT,
    },
    core::{
        Fire::CFireManager,
        Game::ScriptEntityWorld,
        Pad::CPads,
    },
    entities::Entity::eEntityStatus,
    modelinfo::ModelIndices::MI_FIRETRUCK,
    peds::{Ped::CPed, PlayerPed::CWorldPlayers},
};

use super::RenderBuffer::RwIm3DVertexIntent;

pub const NUM_WATERCANNONS: usize = 3;
pub const NUM_SEGMENTPOINTS: usize = 16;
pub const WATERCANNON_GRAVITY: f32 = 0.009;
pub const WATERCANNON_LIFETIME: u32 = 150;
const WATER_CANNON_INDICES: [u16; 12] = [0, 1, 2, 1, 3, 2, 0, 2, 1, 1, 2, 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CWaterCannon {
    pub id: u32,
    pub cur: i16,
    pub time_created: u32,
    pub positions: [CVector; NUM_SEGMENTPOINTS],
    pub velocities: [CVector; NUM_SEGMENTPOINTS],
    pub used: [bool; NUM_SEGMENTPOINTS],
}

impl CWaterCannon {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            cur: 0,
            time_created: 0,
            positions: [CVector::ZERO; NUM_SEGMENTPOINTS],
            velocities: [CVector::ZERO; NUM_SEGMENTPOINTS],
            used: [false; NUM_SEGMENTPOINTS],
        }
    }

    pub fn init(&mut self, time_ms: u32) {
        self.id = 0;
        self.cur = 0;
        self.time_created = time_ms;
        self.used.fill(false);
    }

    pub fn update_new_input(&mut self, position: CVector, direction: CVector) {
        let current = self.cur as usize;
        self.positions[current] = position;
        self.velocities[current] = direction;
        self.used[current] = true;
    }

    fn update_once_per_frame(
        &mut self,
        index: usize,
        time_ms: u32,
        frame_counter: u32,
        time_step: f32,
        random: &mut impl FnMut() -> u32,
        extinguish_points: &mut Vec<WaterCannonExtinguishIntent>,
        request_ped_push: &mut bool,
    ) {
        if time_ms > self.time_created.wrapping_add(WATERCANNON_LIFETIME) {
            self.cur = (self.cur + 1) % NUM_SEGMENTPOINTS as i16;
            self.used[self.cur as usize] = false;
        }
        for segment in 0..NUM_SEGMENTPOINTS {
            if self.used[segment] {
                self.velocities[segment].z -= WATERCANNON_GRAVITY * time_step;
                self.positions[segment] = add(
                    self.positions[segment],
                    scale(self.velocities[segment], time_step),
                );
            }
        }
        let extinguishing_point = random() as usize & (NUM_SEGMENTPOINTS - 1);
        if self.used[extinguishing_point] {
            extinguish_points.push(WaterCannonExtinguishIntent {
                position: self.positions[extinguishing_point],
                radius: 3.0,
            });
        }
        *request_ped_push = ((index as u32 + frame_counter) & 3) == 0;
        if !self.used.iter().any(|used| *used) {
            self.id = 0;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterCannonExtinguishIntent {
    pub position: CVector,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterCannonPedSnapshot {
    pub handle: i32,
    pub position: CVector,
    pub move_speed: CVector,
    /// Result of `CPed::GetLocalDirection(CVector2D(1,0))`.
    pub local_direction: i32,
    pub has_fire: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterCannonPedPushIntent {
    pub handle: i32,
    pub standing: bool,
    pub vertical_force: f32,
    pub move_speed: CVector,
    pub fall_duration_ms: u32,
    /// Offset from `ANIM_STD_HIGHIMPACT_FRONT`.
    pub fall_animation_direction: i32,
    pub extinguish_fire: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaterCannonRenderBatch {
    pub vertices: Vec<RwIm3DVertexIntent>,
    pub indices: Vec<u16>,
    pub z_write: bool,
    pub fog: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CWaterCannons {
    pub cannons: [CWaterCannon; NUM_WATERCANNONS],
    pub extinguish_intents: Vec<WaterCannonExtinguishIntent>,
    pub ped_push_intents: Vec<WaterCannonPedPushIntent>,
    pub render_batches: Vec<WaterCannonRenderBatch>,
}

impl CWaterCannons {
    pub fn init_at(time_ms: u32) -> Self {
        let mut cannons = [CWaterCannon::empty(); NUM_WATERCANNONS];
        for cannon in &mut cannons {
            cannon.init(time_ms);
        }
        Self {
            cannons,
            extinguish_intents: Vec::new(),
            ped_push_intents: Vec::new(),
            render_batches: Vec::new(),
        }
    }

    pub fn init() -> Self {
        Self::init_at(0)
    }

    pub fn update_one(
        &mut self,
        id: u32,
        position: CVector,
        direction: CVector,
        time_ms: u32,
    ) -> bool {
        if let Some(cannon) = self.cannons.iter_mut().find(|cannon| cannon.id == id) {
            cannon.update_new_input(position, direction);
            return true;
        }
        let Some(cannon) = self.cannons.iter_mut().find(|cannon| cannon.id == 0) else {
            return false;
        };
        cannon.init(time_ms);
        cannon.id = id;
        cannon.update_new_input(position, direction);
        true
    }

    pub fn update(
        &mut self,
        time_ms: u32,
        frame_counter: u32,
        time_step: f32,
        random: &mut impl FnMut() -> u32,
        peds: &[WaterCannonPedSnapshot],
    ) {
        self.extinguish_intents.clear();
        self.ped_push_intents.clear();
        for index in 0..NUM_WATERCANNONS {
            if self.cannons[index].id == 0 {
                continue;
            }
            let mut request_ped_push = false;
            self.cannons[index].update_once_per_frame(
                index,
                time_ms,
                frame_counter,
                time_step,
                random,
                &mut self.extinguish_intents,
                &mut request_ped_push,
            );
            if request_ped_push {
                push_peds_for_cannon(
                    &self.cannons[index],
                    peds,
                    time_step,
                    &mut self.ped_push_intents,
                );
            }
        }
    }

    pub fn render(&mut self, camera_forward: CVector, random: &mut impl FnMut() -> u32) {
        self.render_batches.clear();
        for cannon in self.cannons.iter().filter(|cannon| cannon.id != 0) {
            let v = (random() & 255) as f32 / 256.0;
            let mut point_a = cannon.cur as usize % NUM_SEGMENTPOINTS;
            let mut point_b = (point_a + NUM_SEGMENTPOINTS - 1) % NUM_SEGMENTPOINTS;
            let mut normal = CVector::ZERO;
            let mut normal_initialised = false;
            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            for i in 0..NUM_SEGMENTPOINTS - 1 {
                if cannon.used[point_a] && cannon.used[point_b] {
                    if !normal_initialised {
                        let cp = cross(
                            sub(cannon.positions[point_b], cannon.positions[point_a]),
                            camera_forward,
                        );
                        normal = scale(cp, 0.05 / magnitude(cp));
                        normal_initialised = true;
                    }
                    let distance = (i * i * i) as f32 / 300.0 + 1.0;
                    let brightness = i as f32 / NUM_SEGMENTPOINTS as f32;
                    let colour_value = ((1.0 - brightness * brightness) * 255.0) as u8;
                    let colour = [colour_value; 4];
                    let offset = scale(normal, distance);
                    let base = vertices.len() as u16;
                    vertices.extend([
                        vertex(sub(cannon.positions[point_a], offset), colour, [0.0, v]),
                        vertex(add(cannon.positions[point_a], offset), colour, [1.0, v]),
                        vertex(sub(cannon.positions[point_b], offset), colour, [0.0, v]),
                        vertex(add(cannon.positions[point_b], offset), colour, [1.0, v]),
                    ]);
                    indices.extend(WATER_CANNON_INDICES.map(|value| value + base));
                }
                point_a = point_b;
                point_b = (point_b + NUM_SEGMENTPOINTS - 1) % NUM_SEGMENTPOINTS;
            }
            if !indices.is_empty() {
                self.render_batches.push(WaterCannonRenderBatch {
                    vertices,
                    indices,
                    z_write: false,
                    fog: true,
                });
            }
        }
    }

    pub fn drain_extinguish_intents(&mut self) -> Vec<WaterCannonExtinguishIntent> {
        std::mem::take(&mut self.extinguish_intents)
    }

    pub fn drain_ped_push_intents(&mut self) -> Vec<WaterCannonPedPushIntent> {
        std::mem::take(&mut self.ped_push_intents)
    }
}

impl Default for CWaterCannons {
    fn default() -> Self {
        Self::init()
    }
}

/// re3 `CWaterCannons::Update`: advance the three cannon streams, extinguish
/// fires and apply the authored water impact to peds. This runs in the
/// WaterCannons process set, before `CWorld::Process`, matching Game.cpp.
pub(crate) fn update_water_cannons_runtime(
    mut cannons: ResMut<CWaterCannons>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut fire_manager: ResMut<CFireManager>,

    mut script_entities: ScriptEntityWorld,
    mut world_players: CWorldPlayers,
) {
    let mut peds = Vec::with_capacity(script_entities.peds.len() + 2);
    for (handle, entity) in script_entities.peds.iter() {
        let ped = &entity.ped;
        peds.push(WaterCannonPedSnapshot {
            handle: *handle,
            position: ped.physical.entity.get_position(),
            move_speed: ped.physical.move_speed,
            local_direction: i32::from(ped.get_local_direction(entity.locomotion.rotation_cur, CVector::new(1.0, 0.0, 0.0))),
            has_fire: entity.vitality.fire.is_some(),
        });
    }
    for player in world_players.iter() {
        peds.push(WaterCannonPedSnapshot {
            handle: player.pool_handle,
            position: player.ped.physical.entity.get_position(),
            move_speed: player.ped.physical.move_speed,
            local_direction: i32::from(
                player
                    .ped
                    .get_local_direction(player.locomotion.rotation_cur, CVector::new(1.0, 0.0, 0.0)),
            ),
            has_fire: player.vitality.fire.is_some(),
        });
    }

    cannons.update(
        timer.time_in_milliseconds(),
        timer.frame_counter(),
        timer.time_step(),
        &mut || u32::from(general.get_random_number()),
        &peds,
    );

    for intent in cannons.drain_extinguish_intents() {
        fire_manager.ExtinguishPoint(intent.position, intent.radius);
    }

    let now_ms = timer.time_in_milliseconds();
    let frame_counter = timer.frame_counter();
    for intent in cannons.drain_ped_push_intents() {
        if let Some(entity) = script_entities.peds.get_mut(&script_entities.index, &intent.handle) {
            apply_ped_push(
                &mut *entity.ped,
                entity.vitality,
                intent,
                &mut fire_manager,

                now_ms,
                frame_counter,
            );
            continue;
        }
        if let Some(player) = world_players
            .iter_mut()
            .find(|player| player.pool_handle == intent.handle)
        {
            apply_ped_push(
                player.ped,
                player.vitality,
                intent,
                &mut fire_manager,

                now_ms,
                frame_counter,
            );
        }
    }
}

/// re3 `CAutomobile::FireTruckControl`. Keeping the special vehicle control
/// here avoids making the automobile core depend on the renderer-owned cannon
/// pool while retaining the original CWorld-process timing.
pub(crate) fn process_firetruck_water_cannon_control(
    mut cannons: ResMut<CWaterCannons>,
    pads: Res<CPads>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    fire_manager: Res<CFireManager>,
    mut script_entities: ScriptEntityWorld,
) {
    let time_step = timer.time_step();
    let now_ms = timer.time_in_milliseconds();
    let pad = pads.get_pad(0);

    for (handle, vehicle) in script_entities.vehicles.iter_mut() {
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle else {
            continue;
        };
        if i32::from(car.vehicle.core.physical.entity.model_index) != MI_FIRETRUCK as i32 {
            continue;
        }

        let status = car.vehicle.core.physical.entity.status;
        let local_cannon_position;
        if status == eEntityStatus::STATUS_PLAYER {
            // FIX_BUGS behavior from re3: the cannon uses the dedicated held
            // car-gun input instead of the generic weapon mapping.
            if !pad.get_car_gun_fired() {
                continue;
            }
            car.car_gun_lr +=
                f32::from(pad.get_car_gun_left_right()) * 0.00025 * time_step;
            car.car_gun_ud += f32::from(pad.get_car_gun_up_down()) * 0.0001 * time_step;
            car.car_gun_ud = car.car_gun_ud.clamp(0.05, 0.3);
            local_cannon_position = CVector::new(0.0, 1.5, 1.9);
        } else if status == eEntityStatus::STATUS_PHYSICS {
            let car_position = car.vehicle.core.physical.entity.get_position();
            let Some(fire_slot) =
                fire_manager.FindFurthestFire_NeverMindFireMen(car_position, 10.0, 35.0)
            else {
                continue;
            };
            let fire_position = fire_manager.fires[fire_slot].position;
            let target_angle =
                (fire_position.x - car_position.x).atan2(fire_position.y - car_position.y);
            let forward = car.vehicle.core.physical.entity.get_forward();
            let forward_angle = forward.x.atan2(forward.y);
            let target_cannon_angle = forward_angle - target_angle;
            let angle_delta = time_step * 0.01;
            let mut cannon_delta = target_cannon_angle - car.car_gun_lr;
            while cannon_delta < std::f32::consts::PI {
                cannon_delta += std::f32::consts::TAU;
            }
            while cannon_delta > std::f32::consts::PI {
                cannon_delta -= std::f32::consts::TAU;
            }
            if cannon_delta.abs() < angle_delta {
                car.car_gun_lr = target_cannon_angle;
            } else if cannon_delta > 0.0 {
                car.car_gun_lr += angle_delta;
            } else {
                car.car_gun_lr -= angle_delta;
            }

            let up_down =
                ((now_ms & 0xFFF) as f32 / 0x1000 as f32 * std::f32::consts::TAU).sin();
            car.car_gun_ud = 0.2 + 0.2 * up_down;
            if ((now_ms >> 10) & 3) == 0 {
                continue;
            }
            local_cannon_position = CVector::new(0.0, 0.0, 2.2);
        } else {
            continue;
        }

        let matrix = car.vehicle.core.physical.entity.matrix;
        let cannon_position = matrix.transform_point(local_cannon_position);
        let mut cannon_direction = matrix.multiply3x3(CVector::new(
            car.car_gun_lr.sin() * car.car_gun_ud.cos(),
            car.car_gun_lr.cos() * car.car_gun_ud.cos(),
            car.car_gun_ud.sin(),
        ));
        cannon_direction.z += f32::from(general.get_random_number() & 0xF) / 1_000.0;

        // re3 uses the non-null automobile pointer as the stream id. Script
        // handles are stable for the same pooled vehicle; reserve zero for the
        // cannon pool's "free" sentinel.
        let cannon_id = (handle as u32).wrapping_add(1);
        let _ = cannons.update_one(
            cannon_id,
            cannon_position,
            cannon_direction,
            now_ms,
        );
    }
}

fn apply_ped_push(
    ped: &mut CPed,
    vitality: &mut crate::peds::PedVitality::PedVitality,
    intent: WaterCannonPedPushIntent,
    fire_manager: &mut CFireManager,

    now_ms: u32,
    frame_counter: u32,
) {
    ped.flags.is_standing = intent.standing;
    ped.physical
        .apply_move_force(CVector::new(0.0, 0.0, intent.vertical_force));
    ped.physical.move_speed.x = intent.move_speed.x;
    ped.physical.move_speed.y = intent.move_speed.y;

    let direction = intent.fall_animation_direction.rem_euclid(4) as usize;
    let _ = ped.set_fall(vitality,
        intent.fall_duration_ms as i32,
        ANIM_STD_HIGHIMPACT_FRONT + direction as i32,
        false,
        now_ms,
        frame_counter,);

    if intent.extinguish_fire
        && let Some(slot) = vitality.fire.take()
    {
        let _ = fire_manager.extinguish_slot(slot);
    }
}

fn push_peds_for_cannon(
    cannon: &CWaterCannon,
    peds: &[WaterCannonPedSnapshot],
    time_step: f32,
    out: &mut Vec<WaterCannonPedPushIntent>,
) {
    let mut min = CVector::new(10_000.0, 10_000.0, 10_000.0);
    let mut max = CVector::new(-10_000.0, -10_000.0, -10_000.0);
    for index in 0..NUM_SEGMENTPOINTS {
        if cannon.used[index] {
            let position = cannon.positions[index];
            min.x = min.x.min(position.x);
            min.y = min.y.min(position.y);
            min.z = min.z.min(position.z);
            max.x = max.x.max(position.x);
            max.y = max.y.max(position.y);
            max.z = max.z.max(position.z);
        }
    }
    for ped in peds.iter().rev() {
        if ped.position.x <= min.x
            || ped.position.x >= max.x
            || ped.position.y <= min.y
            || ped.position.y >= max.y
            || ped.position.z <= min.z
            || ped.position.z >= max.z
        {
            continue;
        }
        for segment in 0..NUM_SEGMENTPOINTS {
            if cannon.used[segment]
                && magnitude_sqr(sub(cannon.positions[segment], ped.position)) < 5.0
            {
                out.push(WaterCannonPedPushIntent {
                    handle: ped.handle,
                    standing: false,
                    vertical_force: 2.0 * time_step,
                    move_speed: CVector::new(
                        (0.6 * cannon.velocities[segment].x + ped.move_speed.x) * 0.5,
                        (0.6 * cannon.velocities[segment].y + ped.move_speed.y) * 0.5,
                        ped.move_speed.z,
                    ),
                    fall_duration_ms: 2_000,
                    fall_animation_direction: ped.local_direction,
                    extinguish_fire: ped.has_fire,
                });
                break;
            }
        }
    }
}

fn vertex(position: CVector, colour: [u8; 4], uv: [f32; 2]) -> RwIm3DVertexIntent {
    RwIm3DVertexIntent {
        position,
        colour,
        uv,
    }
}
fn add(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(v: CVector, s: f32) -> CVector {
    CVector::new(v.x * s, v.y * s, v.z * s)
}
fn cross(a: CVector, b: CVector) -> CVector {
    CVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn magnitude_sqr(v: CVector) -> f32 {
    v.x * v.x + v.y * v.y + v.z * v.z
}
fn magnitude(v: CVector) -> f32 {
    magnitude_sqr(v).sqrt()
}
