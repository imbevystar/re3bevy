//! re3 reference: references/re-GTA/src/entities/Physical.cpp
//! re3 reference: references/re-GTA/src/entities/Physical.h
//!
//! Physical entity state before full collision response and vehicle/ped control.

use crate::entities::EntityReference::EntityReference;
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

use crate::{
    collision::ColModel::{CColPoint, LEVEL_GENERIC},
    control::PathFind::{CPathFind, PATH_CAR, PATH_PED},
    core::SurfaceTable::{CSurfaceTable, eAdhesiveGroup, eSurfaceType},
    entities::Entity::{CEntity, eEntityStatus, eEntityType},
};

pub const PHYSICAL_MAX_COLLISIONRECORDS: usize = 6;
pub const GRAVITY: f32 = 0.008;
/// re3 `CTimer::GetTimeStep()` advances at 50 game-step units per real second.
pub const GTA_PHYSICS_HZ: f32 = 50.0;
/// Native gravity converted for controllers that keep velocity in world-units/second.
pub const GRAVITY_WORLD_UNITS_PER_SECOND_SQ: f32 = GRAVITY * GTA_PHYSICS_HZ * GTA_PHYSICS_HZ;
pub const SURFACE_DEFAULT: u8 = 0;

pub(crate) fn physical_collision_records(
    physical: &CPhysical,
) -> [Option<EntityReference>; PHYSICAL_MAX_COLLISIONRECORDS] {
    let mut records = [None; PHYSICAL_MAX_COLLISIONRECORDS];
    for (destination, source) in records.iter_mut().zip(&physical.collision_records) {
        *destination = Some(*source);
    }
    records
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CPhysicalFlags {
    pub is_heavy: bool,
    pub affected_by_gravity: bool,
    pub infinite_mass: bool,
    pub is_in_water: bool,
    pub flag_a10: bool,
    pub flag_a20: bool,
    pub hit_by_train: bool,
    pub skip_line_col: bool,
}

impl Default for CPhysicalFlags {
    fn default() -> Self {
        Self {
            is_heavy: false,
            affected_by_gravity: true,
            infinite_mass: false,
            is_in_water: false,
            flag_a10: false,
            flag_a20: false,
            hit_by_train: false,
            skip_line_col: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CDamagePieceRecord {
    pub piece_type: i16,
    pub impulse: f32,
    pub entity: Option<EntityReference>,
    pub normal: CVector,
}

impl Default for CDamagePieceRecord {
    fn default() -> Self {
        Self {
            piece_type: 0,
            impulse: 0.0,
            entity: None,
            normal: CVector::ZERO,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CPhysical {
    pub entity: CEntity,
    pub audio_entity_id: i32,
    pub phys_unused1: f32,
    /// re3 `m_treadable[PATH_*]`, represented by the corresponding
    /// `CPathFind::map_objects` slot.
    pub treadable: [Option<usize>; 2],
    pub last_time_collided: u32,
    pub move_speed: CVector,
    pub turn_speed: CVector,
    pub move_friction: CVector,
    pub turn_friction: CVector,
    pub move_speed_avg: CVector,
    pub turn_speed_avg: CVector,
    pub mass: f32,
    pub turn_mass: f32,
    pub force_multiplier: f32,
    pub air_resistance: f32,
    pub elasticity: f32,
    pub buoyancy: f32,
    pub centre_of_mass: CVector,
    pub moving_list_node: Option<usize>,
    pub phys_unused2: i8,
    pub static_frames: u8,
    pub collision_records: Vec<EntityReference>,
    pub is_vehicle_being_shifted: bool,
    pub distance_travelled: f32,
    pub damage: CDamagePieceRecord,
    pub flags: CPhysicalFlags,
    pub surface_touched: u8,
    pub zone_level: i8,
}

impl CPhysical {
    /// CPhysical::GetBoundRect overrides CEntity's box calculation for moving actors.
    pub fn get_bound_rect(&self, col: &crate::collision::ColModel::CColModel) -> crate::math::Rect::CRect {
        let sphere = &col.bounding_sphere;
        let center = self.entity.matrix.transform_point(CVector::new(
            sphere.center.x, sphere.center.y, sphere.center.z,
        ));
        let radius = sphere.radius;
        crate::math::Rect::CRect::new(
            center.x - radius, center.y - radius, center.x + radius, center.y + radius,
        )
    }

    pub fn new(general: &mut CGeneral) -> Self {
        let mut entity = CEntity::new(general);
        entity.flags.uses_collision = true;

        Self {
            entity,
            audio_entity_id: -5,
            phys_unused1: 100.0,
            treadable: [None; 2],
            last_time_collided: 0,
            move_speed: CVector::ZERO,
            turn_speed: CVector::ZERO,
            move_friction: CVector::ZERO,
            turn_friction: CVector::ZERO,
            move_speed_avg: CVector::ZERO,
            turn_speed_avg: CVector::ZERO,
            mass: 0.0,
            turn_mass: 0.0,
            force_multiplier: 1.0,
            air_resistance: 0.0,
            elasticity: 0.0,
            buoyancy: 0.0,
            centre_of_mass: CVector::ZERO,
            moving_list_node: None,
            phys_unused2: 0,
            static_frames: 0,
            collision_records: Vec::with_capacity(PHYSICAL_MAX_COLLISIONRECORDS),
            is_vehicle_being_shifted: false,
            distance_travelled: 0.0,
            damage: CDamagePieceRecord::default(),
            flags: CPhysicalFlags::default(),
            surface_touched: SURFACE_DEFAULT,
            zone_level: LEVEL_GENERIC as i8,
        }
    }

    pub fn set_entity_type(&mut self, entity_type: eEntityType) {
        self.entity.set_type(entity_type);
    }

    pub fn set_move_speed(&mut self, x: f32, y: f32, z: f32) {
        self.move_speed = CVector::new(x, y, z);
    }

    pub fn set_turn_speed(&mut self, x: f32, y: f32, z: f32) {
        self.turn_speed = CVector::new(x, y, z);
    }

    pub fn set_center_of_mass(&mut self, x: f32, y: f32, z: f32) {
        self.centre_of_mass = CVector::new(x, y, z);
    }

    pub fn set_damaged_piece_record(
        &mut self,
        piece: u16,
        impulse: f32,
        entity: Option<EntityReference>,
        dir: CVector,
    ) {
        self.damage.piece_type = piece as i16;
        self.damage.impulse = impulse;
        self.damage.entity = entity;
        self.damage.normal = dir;
    }

    /// ProcessCollisionSectorList keeps the strongest impulse across all
    /// candidates and retries. SetDamagedPieceRecord itself remains a setter.
    pub(crate) fn record_collision_damage(
        &mut self, piece: u16, impulse: f32, entity: Option<EntityReference>, dir: CVector,
    ) {
        if impulse > self.damage.impulse {
            self.set_damaged_piece_record(piece, impulse, entity, dir);
        }
    }

    /// Physical.cpp NUMSTEPS vehicle branch, shared by static and body probes.
    /// The existing 16-step corrupt-velocity guard is retained explicitly.
    pub(crate) fn vehicle_collision_steps(&self, time_step: f32) -> usize {
        let distance = self.move_speed.magnitude() * time_step.max(0.0);
        let steps = if distance < 0.4 { 1 }
            else if self.entity.status == crate::entities::Entity::eEntityStatus::STATUS_PLAYER {
                (distance / 0.2).ceil() as usize
            } else {
                (distance / if distance * distance > 0.32 { 0.3 } else { 0.4 }).ceil() as usize
            };
        steps.clamp(1, 16)
    }

    pub fn add_collision_record(&mut self, entity: EntityReference, time_ms: u32) {
        self.entity.flags.has_collided = true;
        if self.entity.flags.use_collision_records {
            if self.collision_records.contains(&entity) {
                return;
            }
            if self.collision_records.len() < PHYSICAL_MAX_COLLISIONRECORDS {
                self.collision_records.push(entity);
            }
            self.last_time_collided = time_ms;
        }
    }

    pub fn get_has_collided_with(&self, entity: EntityReference) -> bool {
        self.entity.flags.use_collision_records && self.collision_records.contains(&entity)
    }

    pub fn remove_refs_to_entity(&mut self, entity: EntityReference) {
        self.collision_records.retain(|record| *record != entity);
        if self.damage.entity == Some(entity) {
            self.damage.entity = None;
        }
    }

    pub fn set_treadable_for_path_type(&mut self, path_type: u8, map_object: Option<usize>) {
        match path_type {
            PATH_CAR => self.treadable[PATH_CAR as usize] = map_object,
            PATH_PED => self.treadable[PATH_PED as usize] = map_object,
            _ => {}
        }
    }

    /// re3 `CPhysical::AddCollisionRecord_Treadable`.
    ///
    /// Static path objects are the Rust equivalent of `CTreadable`.  Keep
    /// the path-map slot here so `DoPathSearch` can begin on the exact road
    /// object the vehicle is touching instead of guessing between stacked or
    /// parallel road pieces.
    pub fn add_collision_record_treadable(
        &mut self,
        map_object_index: usize,
        paths: &CPathFind,
    ) {
        let Some(map_object) = paths
            .map_objects
            .get(map_object_index)
            .and_then(|value| value.as_ref())
        else {
            return;
        };

        for path_type in [PATH_PED, PATH_CAR] {
            if map_object.node_indices[path_type as usize][..4]
                .iter()
                .any(|node| *node >= 0)
            {
                self.set_treadable_for_path_type(path_type, Some(map_object_index));
            }
        }
    }

    pub fn get_speed_at_point(&self, point: CVector) -> CVector {
        add_vec3(
            add_vec3(self.move_speed, self.move_friction),
            cross_vec3(add_vec3(self.turn_speed, self.turn_friction), point),
        )
    }

    pub fn get_mass(&self, pos: CVector, dir: CVector) -> f32 {
        1.0 / (cross_vec3(pos, dir).magnitude_sqr() / self.turn_mass + 1.0 / self.mass)
    }

    pub fn get_mass_tweak(&self, pos: CVector, dir: CVector, tweak: f32) -> f32 {
        1.0 / (cross_vec3(pos, dir).magnitude_sqr() / (self.turn_mass * tweak)
            + 1.0 / (self.mass * tweak))
    }

    pub fn add_to_moving_list(&mut self, moving_list_node: usize) {
        self.moving_list_node = Some(moving_list_node);
    }

    pub fn remove_from_moving_list(&mut self) {
        self.moving_list_node = None;
    }

    pub fn place_physical_relative_to_other_physical(
        other: &CPhysical,
        phys: &mut CPhysical,
        local_pos: CVector,
        time_step: f32,
    ) {
        let step = 0.9 * time_step;
        let world_pos = other.entity.transform_point(local_pos);
        let position = add_vec3(world_pos, scale_vec3(other.move_speed, step));
        phys.entity.set_matrix(other.entity.matrix);
        phys.entity.set_position(position);
        phys.move_speed = other.move_speed;
    }

    pub fn process_control_pre_physics(&mut self, time_step: f32) -> CPhysicalControlResult {
        if !self.entity.is_ped() {
            self.flags.is_in_water = false;
        }
        self.entity.flags.has_contacted = false;
        self.entity.flags.is_in_safe_position = false;
        self.entity.flags.was_postponed = false;
        self.entity.flags.has_hit_wall = false;

        if self.entity.status == crate::entities::Entity::eEntityStatus::STATUS_SIMPLE {
            return CPhysicalControlResult::SimpleStatus;
        }

        self.collision_records.clear();
        self.entity.flags.has_collided = false;
        self.damage.piece_type = 0;
        self.damage.impulse = 0.0;
        self.damage.entity = None;

        if !self.entity.flags.is_stuck
            && (self.entity.is_object() || (self.entity.is_ped() && !self.entity.flags.ped_physics))
        {
            self.move_speed_avg = scale_vec3(add_vec3(self.move_speed_avg, self.move_speed), 0.5);
            self.turn_speed_avg = scale_vec3(add_vec3(self.turn_speed_avg, self.turn_speed), 0.5);
            let step = time_step * 0.003;
            if self.move_speed_avg.magnitude_sqr() < step * step
                && self.turn_speed_avg.magnitude_sqr() < step * step
            {
                self.static_frames = self.static_frames.saturating_add(1);
                if self.static_frames > 10 {
                    self.static_frames = 10;
                    self.entity.set_is_static(true);
                    self.move_speed = CVector::ZERO;
                    self.turn_speed = CVector::ZERO;
                    self.move_friction = CVector::ZERO;
                    self.turn_friction = CVector::ZERO;
                    return CPhysicalControlResult::BecameStatic;
                }
            } else {
                self.static_frames = 0;
            }
        }

        self.apply_gravity(time_step);
        self.apply_friction();
        self.apply_air_resistance(time_step);
        CPhysicalControlResult::AppliedForces
    }

    pub fn apply_move_speed(&mut self, time_step: f32) {
        self.entity
            .translate(scale_vec3(self.move_speed, time_step));
    }

    /// Apply the positional part of re3 `CPhysical::ProcessShift` after a
    /// collision response has rejected the integrated matrix.  The native
    /// code translates the entity out of the deepest contact, then marks the
    /// resulting transform as safe for the next world pass.
    pub fn apply_process_shift(&mut self, shift: CVector) -> bool {
        if shift.magnitude_sqr() <= f32::EPSILON {
            return false;
        }
        self.entity.translate(shift);
        true
    }

    pub fn apply_turn_speed(&mut self, time_step: f32) {
        let turn_vec = scale_vec3(self.turn_speed, time_step);
        let right = self.entity.matrix.right;
        let forward = self.entity.matrix.forward;
        let up = self.entity.matrix.up;
        self.entity.matrix.right = add_vec3(right, cross_vec3(turn_vec, right));
        self.entity.matrix.forward = add_vec3(forward, cross_vec3(turn_vec, forward));
        self.entity.matrix.up = add_vec3(up, cross_vec3(turn_vec, up));
    }

    pub fn apply_move_force(&mut self, impulse: CVector) {
        if self.mass != 0.0 {
            self.move_speed = add_vec3(self.move_speed, scale_vec3(impulse, 1.0 / self.mass));
        }
    }

    pub fn apply_turn_force(&mut self, impulse: CVector, point: CVector) {
        if self.turn_mass != 0.0 {
            let com = self.entity.transform_vector(self.centre_of_mass);
            let turn_impulse = cross_vec3(sub_vec3(point, com), impulse);
            self.turn_speed = add_vec3(
                self.turn_speed,
                scale_vec3(turn_impulse, 1.0 / self.turn_mass),
            );
        }
    }

    pub fn apply_friction_move_force(&mut self, impulse: CVector) {
        if self.mass != 0.0 {
            self.move_friction = add_vec3(self.move_friction, scale_vec3(impulse, 1.0 / self.mass));
        }
    }

    pub fn apply_friction_turn_force(&mut self, impulse: CVector, point: CVector) {
        if self.turn_mass != 0.0 {
            let com = self.entity.transform_vector(self.centre_of_mass);
            let turn_impulse = cross_vec3(sub_vec3(point, com), impulse);
            self.turn_friction = add_vec3(
                self.turn_friction,
                scale_vec3(turn_impulse, 1.0 / self.turn_mass),
            );
        }
    }

    pub fn apply_spring_collision(
        &mut self,
        spring_const: f32,
        spring_dir: CVector,
        point: CVector,
        spring_ratio: f32,
        bias: f32,
        time_step: f32,
    ) -> bool {
        let compression = 1.0 - spring_ratio;
        if compression > 0.0 {
            let step = time_step.min(3.0);
            let impulse = -GRAVITY * self.mass * step * spring_const * compression * bias * 2.0;
            let spring_impulse = scale_vec3(spring_dir, impulse);
            self.apply_move_force(spring_impulse);
            self.apply_turn_force(spring_impulse, point);
        }
        true
    }

    pub fn apply_spring_dampening(
        &mut self,
        damping: f32,
        spring_dir: CVector,
        point: CVector,
        speed: CVector,
        time_step: f32,
    ) -> bool {
        let speed_a = dot_vec3(speed, spring_dir);
        let speed_b = dot_vec3(self.get_speed_at_point(point), spring_dir);
        if speed_b == 0.0 {
            return true;
        }

        let step = time_step.min(3.0);
        let mut impulse = -damping * (speed_a + speed_b) / 2.0 * self.mass * step * 0.53;
        let a = (self.turn_mass / ((point.magnitude_sqr() + 1.0) * 2.0 * self.mass)).min(1.0);
        let b = (impulse / (speed_b * self.mass)).abs();
        if a < b {
            impulse *= a / b;
        }

        let spring_impulse = scale_vec3(spring_dir, impulse);
        self.apply_move_force(spring_impulse);
        self.apply_turn_force(spring_impulse, point);
        true
    }

    pub fn apply_gravity(&mut self, time_step: f32) {
        if self.flags.affected_by_gravity {
            self.move_speed.z -= GRAVITY * time_step;
        }
    }

    pub fn apply_friction(&mut self) {
        self.move_speed = add_vec3(self.move_speed, self.move_friction);
        self.turn_speed = add_vec3(self.turn_speed, self.turn_friction);
        self.move_friction = CVector::ZERO;
        self.turn_friction = CVector::ZERO;
    }

    pub fn apply_air_resistance(&mut self, time_step: f32) {
        if self.air_resistance > 0.1 {
            let factor = self.air_resistance.powf(time_step);
            self.move_speed = scale_vec3(self.move_speed, factor);
            self.turn_speed = scale_vec3(self.turn_speed, factor);
        } else {
            let factor = (1.0
                / (self.air_resistance * 0.5 * self.move_speed.magnitude_sqr() + 1.0).abs())
            .powf(time_step);
            self.move_speed = scale_vec3(self.move_speed, factor);
            self.turn_speed = scale_vec3(self.turn_speed, 0.99);
        }
    }

    pub fn unset_is_in_safe_position(&mut self, time_step: f32) {
        self.move_speed = scale_vec3(self.move_speed, -1.0);
        self.turn_speed = scale_vec3(self.turn_speed, -1.0);
        self.apply_turn_speed(time_step);
        self.apply_move_speed(time_step);
        self.move_speed = scale_vec3(self.move_speed, -1.0);
        self.turn_speed = scale_vec3(self.turn_speed, -1.0);
        self.entity.flags.is_in_safe_position = false;
    }

    pub fn apply_collision_dynamic(
        &mut self,
        other: &mut CPhysical,
        colpoint: &CColPoint,
        options: CPhysicalCollisionOptions,
    ) -> CPhysicalCollisionResult {
        let normal = col_vec_to_vec3(colpoint.normal);
        let point = col_vec_to_vec3(colpoint.point);
        let mut impulse_a = 0.0;
        let mut impulse_b = 0.0;

        let mut mass_factor_a = if other.entity.flags.ped_physics {
            10.0
        } else if self.flags.is_heavy {
            2.0
        } else {
            1.0
        };
        if mass_factor_a == 0.0 {
            mass_factor_a = 1.0;
        }

        let mut mass_factor_b = if self.entity.flags.ped_physics {
            if options.player_ped_against_abandoned_or_wrecked_vehicle {
                2200.0 / other.mass
            } else {
                10.0
            }
        } else if other.flags.is_heavy {
            2.0
        } else {
            1.0
        };
        if mass_factor_b == 0.0 {
            mass_factor_b = 1.0;
        }

        let applied = if self.entity.flags.ped_physics && other.entity.flags.ped_physics {
            let speed_a = dot_vec3(self.move_speed, normal);
            let speed_sum = dot_vec3(other.move_speed, normal).max(0.0);
            if speed_a < speed_sum {
                let e_a = if self.entity.flags.has_hit_wall {
                    speed_sum
                } else {
                    speed_sum - (speed_a - speed_sum) * (self.elasticity + other.elasticity) / 2.0
                };
                impulse_a = (e_a - speed_a) * self.mass * mass_factor_a;
                if !self.flags.infinite_mass {
                    self.apply_move_force(scale_vec3(normal, impulse_a / mass_factor_a));
                }
                true
            } else {
                false
            }
        } else if self.entity.flags.ped_physics {
            let pointpos_b = sub_vec3(point, other.entity.get_position());
            let speed_a = dot_vec3(self.move_speed, normal);
            let speed_b = dot_vec3(other.get_speed_at_point(pointpos_b), normal);
            let mass_a = self.mass * mass_factor_a;
            let mass_b = other.get_mass_tweak(pointpos_b, normal, mass_factor_b);
            let speed_sum = (mass_b * speed_b + mass_a * speed_a) / (mass_a + mass_b);
            if speed_a < speed_sum {
                let e_a = collision_rebound_speed(
                    speed_a,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    self.entity.flags.has_hit_wall,
                );
                let e_b = collision_rebound_speed(
                    speed_b,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    other.entity.flags.has_hit_wall,
                );
                impulse_a = (e_a - speed_a) * mass_a;
                impulse_b = -(e_b - speed_b) * mass_b;
                let mut force_a = scale_vec3(normal, impulse_a / mass_factor_a);
                let force_b = scale_vec3(normal, -impulse_b / mass_factor_b);
                if !self.flags.infinite_mass {
                    if force_a.z < 0.0 {
                        force_a.z = 0.0;
                    }
                    if options.b_is_current_physical_surface_for_a {
                        force_a.x *= 2.0;
                        force_a.y *= 2.0;
                    }
                    self.apply_move_force(force_a);
                }
                if !other.flags.infinite_mass && !options.b_is_current_physical_surface_for_a {
                    other.apply_move_force(force_b);
                    other.apply_turn_force(force_b, pointpos_b);
                }
                true
            } else {
                false
            }
        } else if other.entity.flags.ped_physics {
            let pointpos_a = sub_vec3(point, self.entity.get_position());
            let speed_a = dot_vec3(self.get_speed_at_point(pointpos_a), normal);
            let speed_b = dot_vec3(other.move_speed, normal);
            let mass_a = self.get_mass_tweak(pointpos_a, normal, mass_factor_a);
            let mass_b = other.mass * mass_factor_b;
            let speed_sum = (mass_b * speed_b + mass_a * speed_a) / (mass_a + mass_b);
            if speed_a < speed_sum {
                let e_a = collision_rebound_speed(
                    speed_a,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    self.entity.flags.has_hit_wall,
                );
                let e_b = collision_rebound_speed(
                    speed_b,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    other.entity.flags.has_hit_wall,
                );
                impulse_a = (e_a - speed_a) * mass_a;
                impulse_b = -(e_b - speed_b) * mass_b;
                let mut force_a = scale_vec3(normal, impulse_a / mass_factor_a);
                let mut force_b = scale_vec3(normal, -impulse_b / mass_factor_b);
                if !self.flags.infinite_mass && !options.a_is_current_physical_surface_for_b {
                    if force_a.z < 0.0 {
                        force_a.z = 0.0;
                    }
                    self.apply_move_force(force_a);
                    self.apply_turn_force(force_a, pointpos_a);
                }
                if !other.flags.infinite_mass {
                    if force_b.z < 0.0 {
                        force_b.z = 0.0;
                        if speed_a.abs() < 0.01 {
                            force_b = scale_vec3(force_b, 0.5);
                        }
                    }
                    if options.a_is_current_physical_surface_for_b {
                        force_b.x *= 2.0;
                        force_b.y *= 2.0;
                    }
                    other.apply_move_force(force_b);
                }
                true
            } else {
                false
            }
        } else {
            let mut pointpos_a = sub_vec3(point, self.entity.get_position());
            let mut pointpos_b = sub_vec3(point, other.entity.get_position());
            let speed_a = dot_vec3(self.get_speed_at_point(pointpos_a), normal);
            let speed_b = dot_vec3(other.get_speed_at_point(pointpos_b), normal);
            let mass_a = self.get_mass_tweak(pointpos_a, normal, mass_factor_a);
            let mass_b = other.get_mass_tweak(pointpos_b, normal, mass_factor_b);
            let speed_sum = (mass_b * speed_b + mass_a * speed_a) / (mass_a + mass_b);
            if speed_a < speed_sum {
                let e_a = collision_rebound_speed(
                    speed_a,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    self.entity.flags.has_hit_wall,
                );
                let e_b = collision_rebound_speed(
                    speed_b,
                    speed_sum,
                    self.elasticity,
                    other.elasticity,
                    other.entity.flags.has_hit_wall,
                );
                impulse_a = (e_a - speed_a) * mass_a;
                impulse_b = -(e_b - speed_b) * mass_b;
                let mut force_a = scale_vec3(normal, impulse_a / mass_factor_a);
                let mut force_b = scale_vec3(normal, -impulse_b / mass_factor_b);

                if self.entity.is_vehicle() && !self.entity.flags.has_hit_wall {
                    force_a.x *= 1.4;
                    force_a.y *= 1.4;
                    if normal.z < 0.7 {
                        force_a.z *= 0.3;
                    }
                    if self.entity.status == eEntityStatus::STATUS_PLAYER {
                        pointpos_a = scale_vec3(pointpos_a, 0.8);
                    }
                    if options.no_more_collision_torque {
                        self.apply_friction_move_force(scale_vec3(force_a, -0.3));
                        self.apply_friction_turn_force(scale_vec3(force_a, -0.3), pointpos_a);
                    }
                }

                if other.entity.is_vehicle() && !other.entity.flags.has_hit_wall {
                    force_b.x *= 1.4;
                    force_b.y *= 1.4;
                    if -normal.z < 0.7 {
                        force_b.z *= 0.3;
                    }
                    if other.entity.status == eEntityStatus::STATUS_PLAYER {
                        pointpos_b = scale_vec3(pointpos_b, 0.8);
                    }
                    if options.no_more_collision_torque {
                        other.apply_friction_move_force(scale_vec3(force_b, -0.3));
                        other.apply_friction_turn_force(scale_vec3(force_b, -0.3), pointpos_b);
                    }
                }

                if !self.flags.infinite_mass {
                    self.apply_move_force(force_a);
                    self.apply_turn_force(force_a, pointpos_a);
                }
                if !other.flags.infinite_mass {
                    if other.entity.flags.is_in_safe_position {
                        other.unset_is_in_safe_position(options.time_step);
                    }
                    other.apply_move_force(force_b);
                    other.apply_turn_force(force_b, pointpos_b);
                }
                true
            } else {
                false
            }
        };

        CPhysicalCollisionResult {
            applied,
            impulse_a,
            impulse_b,
        }
    }

    pub fn apply_collision_alt(
        &mut self,
        colpoint: &CColPoint,
        other_is_building_or_infinite_mass: bool,
        time_step: f32,
    ) -> CPhysicalCollisionAltResult {
        let normal = col_vec_to_vec3(colpoint.normal);
        let point = col_vec_to_vec3(colpoint.point);
        if self.entity.flags.ped_physics {
            let normal_speed = dot_vec3(self.move_speed, normal);
            if normal_speed < 0.0 {
                let impulse = -normal_speed * self.mass;
                self.apply_move_force(scale_vec3(normal, impulse));
                return CPhysicalCollisionAltResult {
                    applied: true,
                    impulse,
                    move_speed: self.move_speed,
                    turn_speed: self.turn_speed,
                };
            }
            return CPhysicalCollisionAltResult::not_applied(self.move_speed, self.turn_speed);
        }

        let pointpos = sub_vec3(point, self.entity.get_position());
        let speed = self.get_speed_at_point(pointpos);
        let normal_speed = dot_vec3(speed, normal);
        if normal_speed < 0.0 {
            let min_speed = 1.3 * GRAVITY * time_step;
            let e = if (self.entity.is_object()
                || (self.entity.is_vehicle() && self.entity.get_up().z < -0.3))
                && !self.entity.flags.has_contacted
                && self.move_speed.x.abs() < min_speed
                && self.move_speed.y.abs() < min_speed
                && self.move_speed.z.abs() < min_speed * 2.0
            {
                -1.0
            } else {
                -(self.elasticity + 1.0)
            };
            let impulse = normal_speed * e * self.get_mass(pointpos, normal);
            let vector_impulse = scale_vec3(normal, impulse);
            let mut move_speed = self.move_speed;
            let mut turn_speed = self.turn_speed;
            if self.entity.is_vehicle()
                && (!self.entity.flags.has_hit_wall
                    || !(self.move_speed.magnitude_sqr() > 0.1
                        || !other_is_building_or_infinite_mass))
            {
                move_speed = add_vec3(move_speed, scale_vec3(vector_impulse, 1.2 / self.mass));
            } else {
                move_speed = add_vec3(move_speed, scale_vec3(vector_impulse, 1.0 / self.mass));
            }

            let com = self.entity.transform_vector(self.centre_of_mass);
            let turn_impulse = cross_vec3(sub_vec3(pointpos, com), vector_impulse);
            turn_speed = add_vec3(turn_speed, scale_vec3(turn_impulse, 1.0 / self.turn_mass));
            return CPhysicalCollisionAltResult {
                applied: true,
                impulse,
                move_speed,
                turn_speed,
            };
        }

        CPhysicalCollisionAltResult::not_applied(self.move_speed, self.turn_speed)
    }

    pub fn apply_collision_with_static_object(
        &mut self,
        colpoint: &CColPoint,
        options: CPhysicalStaticObjectCollisionOptions,
    ) -> CPhysicalStaticObjectCollisionResult {
        let normal = col_vec_to_vec3(colpoint.normal);
        let point = col_vec_to_vec3(colpoint.point);

        if self.entity.flags.ped_physics {
            let speed_a = dot_vec3(self.move_speed, normal);
            if speed_a >= 0.0 {
                return CPhysicalStaticObjectCollisionResult::not_applied();
            }

            let impulse = -speed_a * self.mass;
            let mut result = CPhysicalStaticObjectCollisionResult {
                applied: true,
                impulse,
                should_damage_object: false,
                damage_impulse: 0.0,
                should_uproot_object: false,
                should_mark_object_damaged: false,
                should_hard_glass_collision: false,
                should_soft_glass_collision: false,
                object_collision_disabled_by_damage: false,
            };

            if impulse > options.object_uproot_limit {
                if options.object_is_glass {
                    result.should_hard_glass_collision = true;
                } else if !options.object_infinite_mass {
                    result.should_uproot_object = true;
                }
            } else {
                if options.object_is_glass {
                    result.should_soft_glass_collision = true;
                }
                if !self.flags.infinite_mass {
                    self.apply_move_force(scale_vec3(normal, (1.0 + self.elasticity) * impulse));
                }
                return result;
            }

            if options.object_infinite_mass {
                if !self.flags.infinite_mass {
                    self.apply_move_force(scale_vec3(normal, (1.0 + self.elasticity) * impulse));
                }
            }
            return result;
        }

        let pointpos_a = sub_vec3(point, self.entity.get_position());
        let speed_a = dot_vec3(self.get_speed_at_point(pointpos_a), normal);
        if speed_a >= 0.0 {
            return CPhysicalStaticObjectCollisionResult::not_applied();
        }

        let elasticity = if self.entity.flags.has_hit_wall {
            -1.0
        } else {
            -(1.0 + self.elasticity)
        };
        let impulse = elasticity * speed_a * self.get_mass(pointpos_a, normal);
        let mut result = CPhysicalStaticObjectCollisionResult {
            applied: true,
            impulse,
            should_damage_object: false,
            damage_impulse: 0.0,
            should_uproot_object: false,
            should_mark_object_damaged: false,
            should_hard_glass_collision: false,
            should_soft_glass_collision: false,
            object_collision_disabled_by_damage: false,
        };

        if options.object_collision_damage_effect != 0 && impulse > 20.0 {
            result.should_damage_object = true;
            result.damage_impulse = impulse;
            if !options.object_uses_collision_after_damage {
                result.object_collision_disabled_by_damage = true;
                if !self.flags.infinite_mass {
                    let force = scale_vec3(normal, 0.2 * impulse);
                    self.apply_move_force(force);
                    self.apply_turn_force(force, pointpos_a);
                }
                return result;
            }
        }

        if (impulse > options.object_uproot_limit || self.entity.flags.is_stuck)
            && !options.object_infinite_mass
        {
            if options.object_is_glass {
                result.should_hard_glass_collision = true;
            } else {
                result.should_uproot_object = true;
            }
            result.should_mark_object_damaged = true;
        } else {
            if options.object_is_glass {
                result.should_soft_glass_collision = true;
            }
            let mut force = scale_vec3(normal, impulse);
            if self.entity.is_vehicle() && normal.z < 0.7 {
                force.z *= 0.3;
            }
            if !self.flags.infinite_mass {
                self.apply_move_force(force);
                if !self.entity.is_vehicle() || !options.no_more_collision_torque {
                    self.apply_turn_force(force, pointpos_a);
                }
            }
        }

        result
    }

    pub fn apply_collision_friction_between(
        &mut self,
        other: &mut CPhysical,
        adhesive_limit: f32,
        colpoint: &CColPoint,
        time_step: f32,
    ) -> CPhysicalFrictionResult {
        let normal = col_vec_to_vec3(colpoint.normal);
        let point = col_vec_to_vec3(colpoint.point);

        if self.entity.flags.ped_physics && other.entity.flags.ped_physics {
            let normal_speed_a = dot_vec3(self.move_speed, normal);
            let normal_speed_b = dot_vec3(other.move_speed, normal);
            let other_speed_a = sub_vec3(self.move_speed, scale_vec3(normal, normal_speed_a));
            let other_speed_b = sub_vec3(other.move_speed, scale_vec3(normal, normal_speed_b));
            let friction = friction_pair_impulses(
                other_speed_a,
                other_speed_b,
                self.mass,
                other.mass,
                adhesive_limit,
                time_step,
            );
            if friction.applied {
                self.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_a));
                other.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_b));
            }
            return friction;
        }

        if self.entity.flags.ped_physics {
            if other.entity.is_vehicle() {
                return CPhysicalFrictionResult::not_applied();
            }
            let pointpos_b = sub_vec3(point, other.entity.get_position());
            let speed_b = other.get_speed_at_point(pointpos_b);
            let normal_speed_a = dot_vec3(self.move_speed, normal);
            let normal_speed_b = dot_vec3(speed_b, normal);
            let other_speed_a = sub_vec3(self.move_speed, scale_vec3(normal, normal_speed_a));
            let other_speed_b = sub_vec3(speed_b, scale_vec3(normal, normal_speed_b));
            let direction = normalise_vec3(other_speed_a);
            let mass_b = other.get_mass(pointpos_b, direction);
            let friction = friction_pair_impulses(
                other_speed_a,
                other_speed_b,
                self.mass,
                mass_b,
                adhesive_limit,
                time_step,
            );
            if friction.applied {
                self.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_a));
                other.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_b));
                other.apply_friction_turn_force(
                    scale_vec3(friction.direction, friction.impulse_b),
                    pointpos_b,
                );
            }
            return friction;
        }

        if other.entity.flags.ped_physics {
            if self.entity.is_vehicle() {
                return CPhysicalFrictionResult::not_applied();
            }
            let pointpos_a = sub_vec3(point, self.entity.get_position());
            let speed_a = self.get_speed_at_point(pointpos_a);
            let normal_speed_a = dot_vec3(speed_a, normal);
            let normal_speed_b = dot_vec3(other.move_speed, normal);
            let other_speed_a = sub_vec3(speed_a, scale_vec3(normal, normal_speed_a));
            let other_speed_b = sub_vec3(other.move_speed, scale_vec3(normal, normal_speed_b));
            let direction = normalise_vec3(other_speed_a);
            let mass_a = self.get_mass(pointpos_a, direction);
            let friction = friction_pair_impulses(
                other_speed_a,
                other_speed_b,
                mass_a,
                other.mass,
                adhesive_limit,
                time_step,
            );
            if friction.applied {
                self.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_a));
                self.apply_friction_turn_force(
                    scale_vec3(friction.direction, friction.impulse_a),
                    pointpos_a,
                );
                other.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_b));
            }
            return friction;
        }

        let pointpos_a = sub_vec3(point, self.entity.get_position());
        let pointpos_b = sub_vec3(point, other.entity.get_position());
        let speed_a = self.get_speed_at_point(pointpos_a);
        let speed_b = other.get_speed_at_point(pointpos_b);
        let normal_speed_a = dot_vec3(speed_a, normal);
        let normal_speed_b = dot_vec3(speed_b, normal);
        let other_speed_a = sub_vec3(speed_a, scale_vec3(normal, normal_speed_a));
        let other_speed_b = sub_vec3(speed_b, scale_vec3(normal, normal_speed_b));
        let direction = normalise_vec3(other_speed_a);
        let mass_a = self.get_mass(pointpos_a, direction);
        let mass_b = other.get_mass(pointpos_b, direction);
        let friction = friction_pair_impulses(
            other_speed_a,
            other_speed_b,
            mass_a,
            mass_b,
            adhesive_limit,
            time_step,
        );
        if friction.applied {
            self.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_a));
            self.apply_friction_turn_force(
                scale_vec3(friction.direction, friction.impulse_a),
                pointpos_a,
            );
            other.apply_friction_move_force(scale_vec3(friction.direction, friction.impulse_b));
            other.apply_friction_turn_force(
                scale_vec3(friction.direction, friction.impulse_b),
                pointpos_b,
            );
        }
        friction
    }

    pub fn apply_collision_friction_single(
        &mut self,
        adhesive_limit: f32,
        colpoint: &CColPoint,
        time_step: f32,
    ) -> CPhysicalFrictionResult {
        let normal = col_vec_to_vec3(colpoint.normal);
        let point = col_vec_to_vec3(colpoint.point);

        if self.entity.flags.ped_physics {
            let normal_speed = dot_vec3(self.move_speed, normal);
            let other_speed = sub_vec3(self.move_speed, scale_vec3(normal, normal_speed));
            let other_speed_mag = other_speed.magnitude();
            if other_speed_mag > 0.0 {
                let friction_dir = normalise_vec3(other_speed);
                let mut impulse = -other_speed_mag;
                let impulse_limit = adhesive_limit * time_step / self.mass;
                if impulse < -impulse_limit {
                    impulse = -impulse_limit;
                }
                let vector_impulse = scale_vec3(friction_dir, impulse);
                self.move_friction = add_vec3(
                    self.move_friction,
                    CVector::new(vector_impulse.x, vector_impulse.y, 0.0),
                );
                return CPhysicalFrictionResult {
                    applied: true,
                    impulse_a: impulse,
                    impulse_b: 0.0,
                    direction: friction_dir,
                    would_emit_sparks: false,
                };
            }
            return CPhysicalFrictionResult::not_applied();
        }

        let pointpos = sub_vec3(point, self.entity.get_position());
        let speed = self.get_speed_at_point(pointpos);
        let normal_speed = dot_vec3(speed, normal);
        let other_speed = sub_vec3(speed, scale_vec3(normal, normal_speed));
        let other_speed_mag = other_speed.magnitude();
        if other_speed_mag > 0.0 {
            let friction_dir = normalise_vec3(other_speed);
            let mut impulse = -other_speed_mag * self.mass;
            let impulse_limit = adhesive_limit * time_step * 1.5;
            if impulse < -impulse_limit {
                impulse = -impulse_limit;
            }
            let friction_impulse = scale_vec3(friction_dir, impulse);
            self.apply_friction_move_force(friction_impulse);
            self.apply_friction_turn_force(friction_impulse, pointpos);

            let would_emit_sparks = other_speed_mag > 0.1
                && colpoint.surface_b != eSurfaceType::SURFACE_GRASS as u8
                && colpoint.surface_b != eSurfaceType::SURFACE_MUD_DRY as u8
                && CSurfaceTable::get_adhesion_group(colpoint.surface_a)
                    == eAdhesiveGroup::ADHESIVE_HARD;

            return CPhysicalFrictionResult {
                applied: true,
                impulse_a: impulse,
                impulse_b: 0.0,
                direction: friction_dir,
                would_emit_sparks,
            };
        }

        CPhysicalFrictionResult::not_applied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CPhysicalControlResult {
    SimpleStatus,
    BecameStatic,
    AppliedForces,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalCollisionOptions {
    pub no_more_collision_torque: bool,
    pub player_ped_against_abandoned_or_wrecked_vehicle: bool,
    pub a_is_current_physical_surface_for_b: bool,
    pub b_is_current_physical_surface_for_a: bool,
    pub time_step: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalStaticObjectCollisionOptions {
    pub object_uproot_limit: f32,
    pub object_collision_damage_effect: u8,
    pub object_uses_collision_after_damage: bool,
    pub object_infinite_mass: bool,
    pub object_is_glass: bool,
    pub no_more_collision_torque: bool,
}

impl Default for CPhysicalStaticObjectCollisionOptions {
    fn default() -> Self {
        Self {
            object_uproot_limit: 0.0,
            object_collision_damage_effect: 0,
            object_uses_collision_after_damage: true,
            object_infinite_mass: false,
            object_is_glass: false,
            no_more_collision_torque: false,
        }
    }
}

impl Default for CPhysicalCollisionOptions {
    fn default() -> Self {
        Self {
            no_more_collision_torque: false,
            player_ped_against_abandoned_or_wrecked_vehicle: false,
            a_is_current_physical_surface_for_b: false,
            b_is_current_physical_surface_for_a: false,
            time_step: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalCollisionResult {
    pub applied: bool,
    pub impulse_a: f32,
    pub impulse_b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalCollisionAltResult {
    pub applied: bool,
    pub impulse: f32,
    pub move_speed: CVector,
    pub turn_speed: CVector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalFrictionResult {
    pub applied: bool,
    pub impulse_a: f32,
    pub impulse_b: f32,
    pub direction: CVector,
    pub would_emit_sparks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhysicalStaticObjectCollisionResult {
    pub applied: bool,
    pub impulse: f32,
    pub should_damage_object: bool,
    pub damage_impulse: f32,
    pub should_uproot_object: bool,
    pub should_mark_object_damaged: bool,
    pub should_hard_glass_collision: bool,
    pub should_soft_glass_collision: bool,
    pub object_collision_disabled_by_damage: bool,
}

impl CPhysicalStaticObjectCollisionResult {
    fn not_applied() -> Self {
        Self {
            applied: false,
            impulse: 0.0,
            should_damage_object: false,
            damage_impulse: 0.0,
            should_uproot_object: false,
            should_mark_object_damaged: false,
            should_hard_glass_collision: false,
            should_soft_glass_collision: false,
            object_collision_disabled_by_damage: false,
        }
    }
}

impl CPhysicalFrictionResult {
    fn not_applied() -> Self {
        Self {
            applied: false,
            impulse_a: 0.0,
            impulse_b: 0.0,
            direction: CVector::ZERO,
            would_emit_sparks: false,
        }
    }
}

impl CPhysicalCollisionAltResult {
    fn not_applied(move_speed: CVector, turn_speed: CVector) -> Self {
        Self {
            applied: false,
            impulse: 0.0,
            move_speed,
            turn_speed,
        }
    }
}

fn add_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale_vec3(a: CVector, scale: f32) -> CVector {
    CVector::new(a.x * scale, a.y * scale, a.z * scale)
}

fn dot_vec3(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn collision_rebound_speed(
    speed: f32,
    speed_sum: f32,
    elasticity_a: f32,
    elasticity_b: f32,
    has_hit_wall: bool,
) -> f32 {
    if has_hit_wall {
        speed_sum
    } else {
        speed_sum - (speed - speed_sum) * (elasticity_a + elasticity_b) / 2.0
    }
}

fn friction_pair_impulses(
    other_speed_a: CVector,
    other_speed_b: CVector,
    mass_a: f32,
    mass_b: f32,
    adhesive_limit: f32,
    time_step: f32,
) -> CPhysicalFrictionResult {
    let speed_a = other_speed_a.magnitude();
    let speed_b = other_speed_b.magnitude();
    if speed_a <= 0.0 {
        return CPhysicalFrictionResult::not_applied();
    }

    let friction_dir = normalise_vec3(other_speed_a);
    let speed_sum = (mass_b * speed_b + mass_a * speed_a) / (mass_b + mass_a);
    if speed_a > speed_sum {
        let mut impulse_a = (speed_sum - speed_a) * mass_a;
        let mut impulse_b = (speed_sum - speed_b) * mass_b;
        let impulse_limit = adhesive_limit * time_step;
        if impulse_a < -impulse_limit {
            impulse_a = -impulse_limit;
        }
        if impulse_b > impulse_limit {
            impulse_b = impulse_limit;
        }
        return CPhysicalFrictionResult {
            applied: true,
            impulse_a,
            impulse_b,
            direction: friction_dir,
            would_emit_sparks: false,
        };
    }

    CPhysicalFrictionResult::not_applied()
}

fn normalise_vec3(value: CVector) -> CVector {
    let magnitude = value.magnitude();
    if magnitude > 0.0 {
        scale_vec3(value, 1.0 / magnitude)
    } else {
        CVector::new(1.0, 0.0, 0.0)
    }
}

fn col_vec_to_vec3(value: crate::collision::ColModel::CColVector) -> CVector {
    CVector::new(value.x, value.y, value.z)
}

fn cross_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::ColModel::CColVector;

    fn ped_vehicle_collision(special_player_push: bool) -> (CPhysicalCollisionResult, CPhysical) {
        let mut general = CGeneral::default();
        let mut ped = CPhysical::new(&mut general);
        ped.entity.set_type(eEntityType::ENTITY_TYPE_PED);
        ped.entity.flags.ped_physics = true;
        ped.mass = 70.0;
        ped.turn_mass = 100.0;
        ped.elasticity = 0.0;
        ped.move_speed = CVector::new(-1.0, 0.0, 0.0);

        let mut vehicle = CPhysical::new(&mut general);
        vehicle.entity.set_type(eEntityType::ENTITY_TYPE_VEHICLE);
        vehicle.mass = 1000.0;
        vehicle.turn_mass = 1500.0;
        vehicle.elasticity = 0.0;

        let mut point = CColPoint::new();
        point.normal = CColVector::new(1.0, 0.0, 0.0);
        point.point = CColVector::new(0.0, 0.0, 0.0);

        let response = ped.apply_collision_dynamic(
            &mut vehicle,
            &point,
            CPhysicalCollisionOptions {
                player_ped_against_abandoned_or_wrecked_vehicle: special_player_push,
                ..Default::default()
            },
        );
        (response, vehicle)
    }

    #[test]
    fn player_ped_as_a_uses_abandoned_vehicle_mass_tweak() {
        let (special, special_vehicle) = ped_vehicle_collision(true);
        let (ordinary, ordinary_vehicle) = ped_vehicle_collision(false);

        assert!(special.applied);
        assert!(ordinary.applied);
        assert!(special.impulse_a > 0.0);
        assert!(ordinary.impulse_a > 0.0);
        assert!(
            special_vehicle.move_speed.x.abs() > ordinary_vehicle.move_speed.x.abs() * 3.0,
            "player-as-A abandoned/wrecked branch must make the vehicle materially easier to push"
        );
    }

    #[test]
    fn damaged_piece_record_matches_re3_setter_semantics() {
        let mut general = CGeneral::default();
        let mut physical = CPhysical::new(&mut general);

        physical.set_damaged_piece_record(4, 50.0, Some(EntityReference::World(10)), CVector::new(1.0, 0.0, 0.0));
        physical.set_damaged_piece_record(7, 5.0, Some(EntityReference::World(20)), CVector::new(0.0, 1.0, 0.0));

        assert_eq!(physical.damage.piece_type, 7);
        assert_eq!(physical.damage.impulse, 5.0);
        assert_eq!(physical.damage.entity, Some(EntityReference::World(20)));
        assert_eq!(physical.damage.normal, CVector::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn collision_candidates_preserve_the_strongest_record_and_first_equal_impulse() {
        let mut physical = CPhysical::new(&mut CGeneral::default());
        physical.record_collision_damage(4, 50.0, Some(EntityReference::World(10)), CVector::new(1.0, 0.0, 0.0));
        physical.record_collision_damage(7, 5.0, Some(EntityReference::Vehicle(257)), CVector::new(0.0, 1.0, 0.0));
        physical.record_collision_damage(8, 50.0, Some(EntityReference::Ped(257)), CVector::new(0.0, 0.0, 1.0));
        assert_eq!(physical.damage.piece_type, 4);
        assert_eq!(physical.damage.entity, Some(EntityReference::World(10)));
        physical.record_collision_damage(2, 60.0, Some(EntityReference::Vehicle(257)), CVector::new(0.0, 1.0, 0.0));
        assert_eq!(physical.damage.impulse, 60.0);
        assert_eq!(physical.damage.entity, Some(EntityReference::Vehicle(257)));
    }
}
