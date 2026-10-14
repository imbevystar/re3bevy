//! re3 reference: references/re-GTA/src/objects/Object.cpp
//! re3 reference: references/re-GTA/src/objects/Object.h
//!
//! Object state and damage transitions.

use bevy::prelude::{Commands, Component, Query, Res, ResMut, Resource};
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    audio::AudioScriptObject::cAudioScriptObject,
    audio::{
        DMAudio::cDMAudio,
        soundlist::{
            SCRIPT_SOUND_BOX_DESTROYED_1, SCRIPT_SOUND_BOX_DESTROYED_2,
            SCRIPT_SOUND_METAL_COLLISION, SCRIPT_SOUND_TIRE_COLLISION,
        },
    },
    core::{
        Camera::CCamera,
        FileLoader::CFileLoader,
        Game::{
            ScriptEntityWorld, ScriptObjectEntity, remove_script_object,
            spawn_script_object,
        },
        Pools::{CPoolEntry, CPoolEntryKind, CPools},
    },
    entities::{Entity::eEntityType, Physical::CPhysical},
    math::Matrix::CMatrix,
    objects::ObjectData::CObjectInfo,
    renderer::{WaterLevel::CWaterLevel, Weather::CWeather},
    renderer::Particle::{CParticle, ParticleColour, ParticleEmitter},
    renderer::ParticleMgr::cParticleSystemMgr,
    vehicles::Automobile::{
        CarDebrisComponent, flying_component_model, flying_component_speed,
        vehicle_component_world_matrix,
    },
};

pub const UNKNOWN_OBJECT: i8 = 0;
pub const GAME_OBJECT: i8 = 1;
pub const MISSION_OBJECT: i8 = 2;
pub const TEMP_OBJECT: i8 = 3;
pub const CUTSCENE_OBJECT: i8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CollisionSpecialResponseCase {
    COLLRESPONSE_NONE = 0,
    COLLRESPONSE_LAMPOST = 1,
    COLLRESPONSE_SMALLBOX = 2,
    COLLRESPONSE_BIGBOX = 3,
    COLLRESPONSE_FENCEPART = 4,
    COLLRESPONSE_UNKNOWN5 = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CollisionDamageEffect {
    DAMAGE_EFFECT_NONE = 0,
    DAMAGE_EFFECT_CHANGE_MODEL = 1,
    DAMAGE_EFFECT_SPLIT_MODEL = 2,
    DAMAGE_EFFECT_SMASH_COMPLETELY = 3,
    DAMAGE_EFFECT_CHANGE_THEN_SMASH = 4,
    DAMAGE_EFFECT_SMASH_CARDBOARD_COMPLETELY = 50,
    DAMAGE_EFFECT_SMASH_WOODENBOX_COMPLETELY = 60,
    DAMAGE_EFFECT_SMASH_TRAFFICCONE_COMPLETELY = 70,
    DAMAGE_EFFECT_SMASH_BARPOST_COMPLETELY = 80,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct CObjectGlobalState {
    pub no_temp_objects: i16,
    pub body_cast_health: i16,
    pub frame_gen: i8,
}

impl Default for CObjectGlobalState {
    fn default() -> Self {
        Self {
            no_temp_objects: 0,
            body_cast_health: 1000,
            frame_gen: 0,
        }
    }
}

/// Native nNoTempObjects follows the canonical object lifetime. Changed and
/// removed inputs are consumed even while normal actor processing is disabled.
pub(crate) fn sync_temporary_object_count(
    mut globals: ResMut<CObjectGlobalState>,
    objects: Query<&CObject>,
    changed: Query<(), bevy::prelude::Changed<CObject>>,
    mut removed: bevy::prelude::RemovedComponents<CObject>,
) {
    let retired = removed.read().count() != 0;
    if !retired && changed.is_empty() { return; }
    globals.no_temp_objects = objects.iter().filter(|object| object.object_created_by == TEMP_OBJECT).count() as i16;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CObjectDamageContext {
    pub body_cast_model_index: Option<i16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CObjectDamageResult {
    pub applied: bool,
    pub render_damaged: bool,
    pub smashed: bool,
    pub split_model: bool,
    pub debris_particles: u8,
    pub debris_direction_z: f32,
    pub script_sound: Option<CObjectDamageSound>,
    pub debris: Vec<CObjectDebris>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CObjectDebris {
    pub direction: CVector,
    pub size: f32,
    pub rotation_speed: i32,
    pub current_frame: i32,
    pub colour: CObjectDebrisColour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CObjectDebrisColour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl CObjectDamageResult {
    fn not_applied() -> Self {
        Self {
            applied: false,
            render_damaged: false,
            smashed: false,
            split_model: false,
            debris_particles: 0,
            debris_direction_z: 0.0,
            script_sound: None,
            debris: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CObjectDamageSound {
    BoxDestroyed2,
    BoxDestroyed1,
    TireCollision,
    MetalCollision,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CObjectFlags {
    pub is_pickup: bool,
    pub pickup_obj_with_message: bool,
    pub out_of_stock: bool,
    pub glass_cracked: bool,
    pub glass_broken: bool,
    pub has_been_damaged: bool,
    pub use_vehicle_colours: bool,
    pub camera_to_avoid_this_object: bool,
}

impl Default for CObjectFlags {
    fn default() -> Self {
        Self {
            is_pickup: false,
            pickup_obj_with_message: false,
            out_of_stock: false,
            glass_cracked: false,
            glass_broken: false,
            has_been_damaged: false,
            use_vehicle_colours: false,
            camera_to_avoid_this_object: false,
        }
    }
}

/// Pre-integration collision pose retained across the control/collision stages.
/// Mirrors CPhysical::ProcessCollision's savedMatrix; damage state does not own it.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct ObjectCollisionState {
    pub previous_matrix: CMatrix,
}

#[derive(Debug, Clone, PartialEq, Component)]
#[require(ObjectCollisionState)]
pub struct CObject {
    pub physical: CPhysical,
    pub object_matrix: CMatrix,
    pub uproot_limit: f32,
    pub object_created_by: i8,
    pub flags: CObjectFlags,
    pub bonus_value: i8,
    pub collision_damage_multiplier: f32,
    pub collision_damage_effect: u8,
    pub special_collision_response_cases: u8,
    pub obj_unused1: u32,
    pub end_of_life_time: u32,
    pub ref_model_index: i16,
    pub current_surface: Option<usize>,
    pub body_cast_health_init_pending: bool,
    pub colliding_entity: Option<usize>,
    pub colour1: i8,
    pub colour2: i8,
}

impl CObject {
    pub fn new(general: &mut CGeneral) -> Self {
        let mut physical = CPhysical::new(general);
        physical.entity.set_type(eEntityType::ENTITY_TYPE_OBJECT);

        Self {
            physical,
            object_matrix: CMatrix::default(),
            uproot_limit: 0.0,
            object_created_by: UNKNOWN_OBJECT,
            flags: CObjectFlags::default(),
            bonus_value: 0,
            collision_damage_multiplier: 0.0,
            collision_damage_effect: CollisionDamageEffect::DAMAGE_EFFECT_NONE as u8,
            special_collision_response_cases: CollisionSpecialResponseCase::COLLRESPONSE_NONE as u8,
            obj_unused1: 0,
            end_of_life_time: 0,
            ref_model_index: -1,
            current_surface: None,
            body_cast_health_init_pending: false,
            colliding_entity: None,
            colour1: 0,
            colour2: 0,
        }
    }

    pub fn new_with_model(id: i32, create_rw: bool, general: &mut CGeneral) -> Self {
        let mut object = Self::new(general);
        if create_rw {
            object
                .physical
                .entity
                .set_model_index_loaded_marker(id as usize);
        } else {
            object
                .physical
                .entity
                .set_model_index_no_create(id as usize);
        }
        object.init(None);
        object
    }

    /// Completes the world/model-dependent part of `CObject::Init` once the
    /// authored world and model tables are available. Call before the caller
    /// moves the newly-created object from its constructor position.
    pub fn initialize_world_state(&mut self, file_loader: &CFileLoader) {
        let model_index = self.physical.entity.model_index.max(0) as usize;
        if let Some(model) = file_loader.model_info.get_model_info(model_index) {
            let object_id = model.base().GetObjectID();
            if let Ok(object_id) = usize::try_from(object_id) {
                if let Some(info) = file_loader.object_data.object_info.get(object_id) {
                    self.apply_object_data(info);
                } else {
                    bevy::log::warn_once!(target: "rustgta3::objects", object_id = object_id, "object_init_missing_object_dat_record");
                }
            }
        } else {
            bevy::log::warn_once!(target: "rustgta3::objects", model_index = model_index, "object_init_missing_model_metadata");
        }
        let position = self.physical.entity.get_position();
        self.current_surface = file_loader
            .world
            .process_vertical_line_buildings_only_with_entity(
                [position.x, position.y, position.z],
                position.z - 10.0,
                &file_loader.model_info,
            )
            .map(|(_, entity_index)| entity_index);

        let model_name = file_loader
            .model_info
            .get_model_info(model_index)
            .map(|model| model.base().name.as_str());
        match model_name {
            Some(name) if name.eq_ignore_ascii_case("bodycast") => {
                self.body_cast_health_init_pending = true;
            }
            Some(name) if name.eq_ignore_ascii_case("bouy") => {
                self.physical.entity.flags.touching_water = true;
            }
            _ => {}
        }
    }

    /// Mirrors `CObjectData::SetObjectData` from re3.  Object.DAT is loaded
    /// once by `CFileLoader`; constructors apply the selected record to the
    /// live physical body before the object enters the world broad phase.
    pub fn apply_object_data(&mut self, info: &CObjectInfo) {
        self.physical.mass = info.mass;
        self.physical.turn_mass = info.turn_mass;
        self.physical.air_resistance = info.air_resistance;
        self.physical.elasticity = info.elasticity;
        self.physical.buoyancy = info.buoyancy;
        self.uproot_limit = info.uproot_limit;
        self.collision_damage_multiplier = info.collision_damage_multiplier;
        self.collision_damage_effect = info.collision_damage_effect;
        self.special_collision_response_cases = info.special_collision_response_cases;
        self.flags.camera_to_avoid_this_object = info.camera_to_avoid_this_object;
        if self.physical.mass >= 99_998.0 {
            self.physical.flags.infinite_mass = true;
            self.physical.flags.affected_by_gravity = false;
            self.physical.entity.flags.explosion_proof = true;
        }
    }

    /// Combined ECS world step corresponding to `CObject::ProcessControl`
    /// followed by the movement portion of `CPhysical::ProcessCollision`.
    /// Collision resolution itself remains in the central world pass.
    pub fn process_world_control_step(&mut self, time_step: f32, collision: &mut ObjectCollisionState) -> bool {
        if self.physical.entity.flags.is_static {
            return false;
        }
        collision.previous_matrix = self.physical.entity.matrix;
        let control = self.physical.process_control_pre_physics(time_step);
        if matches!(
            control,
            crate::entities::Physical::CPhysicalControlResult::SimpleStatus
                | crate::entities::Physical::CPhysicalControlResult::BecameStatic
        ) || self.physical.entity.flags.is_static
        {
            self.object_matrix = self.physical.entity.matrix;
            return false;
        }
        let before = self.physical.entity.get_position();
        self.physical.apply_turn_speed(time_step);
        self.physical.entity.matrix.reorthogonalise();
        self.physical.apply_move_speed(time_step);
        let after = self.physical.entity.get_position();
        let dx = after.x - before.x;
        let dy = after.y - before.y;
        let dz = after.z - before.z;
        self.physical.distance_travelled = (dx * dx + dy * dy + dz * dz).sqrt();
        self.object_matrix = self.physical.entity.matrix;
        true
    }

    /// Mirrors the moving-object guard in
    /// `CPhysical::ProcessCollisionSectorList`: an uprooted object keeps its
    /// `m_pCollidingEntity` while the pair's bounds overlap and skips another
    /// response from that same entity. Separation releases the lock.
    pub fn update_vehicle_collision_lock(
        &mut self,
        vehicle_slot: usize,
        bounding_spheres_touching: bool,
    ) -> bool {
        if self.colliding_entity != Some(vehicle_slot) {
            return false;
        }
        if bounding_spheres_touching {
            true
        } else {
            self.colliding_entity = None;
            false
        }
    }

    pub fn init(&mut self, current_surface: Option<usize>) {
        self.physical
            .entity
            .set_type(eEntityType::ENTITY_TYPE_OBJECT);
        self.end_of_life_time = 0;
        self.object_created_by = GAME_OBJECT;
        self.physical.entity.set_is_static(true);
        self.flags.is_pickup = false;
        self.flags.pickup_obj_with_message = false;
        self.flags.out_of_stock = false;
        self.flags.glass_cracked = false;
        self.flags.glass_broken = false;
        self.flags.has_been_damaged = false;
        self.flags.use_vehicle_colours = false;
        self.ref_model_index = -1;
        self.body_cast_health_init_pending = false;
        self.colour1 = 0;
        self.colour2 = 0;
        self.bonus_value = 0;
        self.colliding_entity = None;
        self.current_surface = current_surface;
    }

    pub fn can_be_deleted(&self) -> bool {
        match self.object_created_by {
            GAME_OBJECT => true,
            MISSION_OBJECT => false,
            TEMP_OBJECT => true,
            CUTSCENE_OBJECT => false,
            _ => true,
        }
    }

    pub fn teleport(&mut self, position: CVector) {
        self.physical.entity.set_position(position);
    }

    pub fn object_damage(
        &mut self,
        mut amount: f32,
        globals: &mut CObjectGlobalState,
        context: CObjectDamageContext,
        general: &mut CGeneral,
    ) -> CObjectDamageResult {
        if self.collision_damage_effect == 0 || !self.physical.entity.flags.uses_collision {
            return CObjectDamageResult::not_applied();
        }

        let mut body_cast_damage_effect = false;
        if Some(self.physical.entity.model_index) == context.body_cast_model_index {
            if amount > 50.0 {
                globals.body_cast_health =
                    (globals.body_cast_health as f32 - 0.5 * amount).max(0.0) as i16;
            }
            if globals.body_cast_health < 200 {
                body_cast_damage_effect = true;
            }
            amount = 0.0;
        }

        if amount * self.collision_damage_multiplier <= 150.0 && !body_cast_damage_effect {
            return CObjectDamageResult::not_applied();
        }

        let direction_z = 0.0002 * amount;
        let effect = self.collision_damage_effect;
        let mut result = CObjectDamageResult {
            applied: true,
            render_damaged: false,
            smashed: false,
            split_model: false,
            debris_particles: 0,
            debris_direction_z: direction_z,
            script_sound: None,
            debris: Vec::new(),
        };

        match effect {
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_CHANGE_MODEL as u8 => {
                self.physical.entity.flags.render_damaged = true;
                result.render_damaged = true;
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SPLIT_MODEL as u8 => {
                result.split_model = true;
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SMASH_COMPLETELY as u8 => {
                self.smash_completely();
                result.smashed = true;
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_CHANGE_THEN_SMASH as u8 => {
                if !self.physical.entity.flags.render_damaged {
                    self.physical.entity.flags.render_damaged = true;
                    result.render_damaged = true;
                } else {
                    self.smash_completely();
                    result.smashed = true;
                }
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SMASH_CARDBOARD_COMPLETELY as u8 => {
                self.smash_completely();
                result.smashed = true;
                result.debris_particles = 25;
                result.script_sound = Some(CObjectDamageSound::BoxDestroyed2);
                result.debris = make_debris(
                    25,
                    CObjectDebrisColour {
                        red: 96,
                        green: 48,
                        blue: 0,
                        alpha: 255,
                    },
                    DebrisColourMode::ScaleRedGreen(0.01, 1.0),
                    direction_z,
                    globals,
                    general,
                );
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SMASH_WOODENBOX_COMPLETELY as u8 => {
                self.smash_completely();
                result.smashed = true;
                result.debris_particles = 45;
                result.script_sound = Some(CObjectDamageSound::BoxDestroyed1);
                result.debris = make_debris(
                    45,
                    CObjectDebrisColour {
                        red: 128,
                        green: 128,
                        blue: 128,
                        alpha: 255,
                    },
                    DebrisColourMode::ScaleAll(0.5, 1.0),
                    direction_z,
                    globals,
                    general,
                );
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SMASH_TRAFFICCONE_COMPLETELY as u8 => {
                self.smash_completely();
                result.smashed = true;
                result.debris_particles = 10;
                result.script_sound = Some(CObjectDamageSound::TireCollision);
                result.debris = make_debris(
                    10,
                    CObjectDebrisColour {
                        red: 200,
                        green: 0,
                        blue: 0,
                        alpha: 255,
                    },
                    DebrisColourMode::Alternating(CObjectDebrisColour {
                        red: 200,
                        green: 200,
                        blue: 200,
                        alpha: 255,
                    }),
                    direction_z,
                    globals,
                    general,
                );
            }
            x if x == CollisionDamageEffect::DAMAGE_EFFECT_SMASH_BARPOST_COMPLETELY as u8 => {
                self.smash_completely();
                result.smashed = true;
                result.debris_particles = 32;
                result.script_sound = Some(CObjectDamageSound::MetalCollision);
                result.debris = make_debris(
                    32,
                    CObjectDebrisColour {
                        red: 200,
                        green: 0,
                        blue: 0,
                        alpha: 255,
                    },
                    DebrisColourMode::Alternating(CObjectDebrisColour {
                        red: 200,
                        green: 200,
                        blue: 200,
                        alpha: 255,
                    }),
                    direction_z,
                    globals,
                    general,
                );
            }
            _ => {
                result.applied = false;
            }
        }

        result
    }

    fn smash_completely(&mut self) {
        self.physical.entity.flags.is_visible = false;
        self.physical.entity.flags.uses_collision = false;
        self.physical.entity.set_is_static(true);
        self.physical.entity.flags.explosion_proof = true;
        self.physical.set_move_speed(0.0, 0.0, 0.0);
        self.physical.set_turn_speed(0.0, 0.0, 0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DebrisColourMode {
    ScaleRedGreen(f32, f32),
    ScaleAll(f32, f32),
    Alternating(CObjectDebrisColour),
}

fn make_debris(
    count: u8,
    base: CObjectDebrisColour,
    colour_mode: DebrisColourMode,
    direction_z: f32,
    globals: &mut CObjectGlobalState,
    general: &mut CGeneral,
) -> Vec<CObjectDebris> {
    let mut debris = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let direction = CVector::new(
            general.get_random_number_in_range_f32(-0.35, 0.35),
            general.get_random_number_in_range_f32(-0.35, 0.35),
            general.get_random_number_in_range_f32(0.1, 0.25) + direction_z,
        );
        globals.frame_gen = globals.frame_gen.wrapping_add(1);
        let current_frame = i32::from(globals.frame_gen) & 3;
        let colour = match colour_mode {
            DebrisColourMode::ScaleRedGreen(low, high) => {
                let factor = general.get_random_number_in_range_f32(low, high);
                CObjectDebrisColour {
                    red: (f32::from(base.red) * factor) as u8,
                    green: (f32::from(base.green) * factor) as u8,
                    ..base
                }
            }
            DebrisColourMode::ScaleAll(low, high) => {
                let factor = general.get_random_number_in_range_f32(low, high);
                CObjectDebrisColour {
                    red: (f32::from(base.red) * factor) as u8,
                    green: (f32::from(base.green) * factor) as u8,
                    blue: (f32::from(base.blue) * factor) as u8,
                    alpha: base.alpha,
                }
            }
            DebrisColourMode::Alternating(other) => {
                if globals.frame_gen & 1 != 0 {
                    base
                } else {
                    other
                }
            }
        };
        debris.push(CObjectDebris {
            direction,
            size: general.get_random_number_in_range_f32(0.02, 0.20),
            rotation_speed: general.get_random_number_in_range_i32(-40, 40),
            current_frame,
            colour,
        });
    }
    debris
}

// === FAZ9a: moved from core/Game.rs (verbatim, parity-preserving) ===

/// Moving-list control pass for all live CObjects. This is deliberately
/// separate from debris spawning: mission objects uprooted by a vehicle use
/// the same OBJECT.DAT gravity, friction and air-resistance path as temporary
/// automobile components.
pub(crate) fn process_script_object_control(
    mut object_entities: Query<(&mut CObject, &mut ObjectCollisionState)>,
    timer: Res<CTimer>,
    mut object_globals: ResMut<CObjectGlobalState>,
    mut general: ResMut<CGeneral>,
    mut particles: Option<ResMut<CParticle>>,
    particle_systems: Option<Res<cParticleSystemMgr>>,
    camera: Option<Res<CCamera>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    file_loader: Option<Res<CFileLoader>>,
    water: Option<Res<CWaterLevel>>,
    weather: Option<Res<CWeather>>,
) {
    let time_step = timer.time_step().max(0.0);
    let mut particle_emitter = particles
        .as_deref_mut()
        .zip(particle_systems.as_deref())
        .zip(camera.as_deref())
        .map(|((particles, systems), camera)| particles.emitter(systems, camera.position));
    let model_ids = file_loader.as_deref().map(|loader| {
        (
            loader.model_info.get_model_info_by_name("bodycast").map(|(id, _)| id as i16),
            loader.model_info.get_model_info_by_name("barrel4").map(|(id, _)| id as i32),
            loader.model_info.get_model_info_by_name("petrolpump").map(|(id, _)| id as i32),
        )
    });
    let water_context = water.as_deref().zip(file_loader.as_deref()).zip(weather.as_deref());
    for (mut entity, mut collision) in object_entities.iter_mut() {
        // CObject::ProcessControl calls ObjectDamage with the impulse saved
        // by the preceding CWorld collision pass before CPhysical clears it.
        let object = &mut *entity;
        if object.body_cast_health_init_pending {
            object_globals.body_cast_health = 1000;
            object.body_cast_health_init_pending = false;
        }
        if object.collision_damage_effect != 0 && object.physical.damage.impulse > 0.0 {
            let impulse = object.physical.damage.impulse;
            let damage = object.object_damage(
                impulse,
                &mut object_globals,
                CObjectDamageContext {
                    body_cast_model_index: model_ids.and_then(|ids| ids.0),
                },
                &mut general,
            );
            emit_object_damage_effects(
                &damage,
                object.physical.entity.get_position(),
                particle_emitter.as_mut(),
                dmaudio.as_deref_mut(),
            );
        }
        object.process_world_control_step(time_step, &mut collision);

        if let Some(((water, loader), weather)) = water_context {
            if !object.physical.entity.flags.is_static {
                let model_id = object.physical.entity.model_index.max(0) as usize;
                if let Some(col) = loader.collision_model(model_id) {
                    let min = CVector::new(col.bounding_box.min.x, col.bounding_box.min.y, col.bounding_box.min.z);
                    let max = CVector::new(col.bounding_box.max.x, col.bounding_box.max.y, col.bounding_box.max.z);
                    if let Some(result) = crate::vehicles::Floater::process_buoyancy(
                        &object.physical,
                        min,
                        max,
                        object.physical.buoyancy,
                        water,
                        timer.time_in_milliseconds(),
                        weather.wind,
                        time_step,
                        false,
                        None,
                    ) {
                        object.physical.flags.is_in_water = true;
                        object.physical.entity.set_is_static(false);
                        object.physical.entity.flags.touching_water = true;
                        object.physical.apply_move_force(result.impulse);
                        object.physical.apply_turn_force(result.impulse, result.point);
                        let damping = 0.97_f32.powf(time_step);
                        object.physical.move_speed.x *= damping;
                        object.physical.move_speed.y *= damping;
                        object.physical.move_speed.z *= damping;
                        object.physical.turn_speed.x *= damping;
                        object.physical.turn_speed.y *= damping;
                        object.physical.turn_speed.z *= damping;
                    }
                } else {
                    bevy::log::warn_once!(target: "rustgta3::objects", model_id = model_id, "object_buoyancy_missing_authored_col");
                }
            }
        } else if !object.physical.entity.flags.is_static {
            bevy::log::warn_once!(target: "rustgta3::objects", "object_buoyancy_missing_runtime_water_data");
        }

        if object.flags.has_been_damaged
            && object.physical.entity.flags.is_visible
            && model_ids.is_some_and(|(_, barrel, pump)| {
                [barrel, pump].into_iter().flatten().any(|id| id == i32::from(object.physical.entity.model_index))
            })
            && general.get_random_number() & 0x1f == 10
        {
            object.physical.entity.flags.explosion_proof = true;
            object.physical.entity.flags.is_visible = false;
            object.physical.entity.flags.uses_collision = false;
            object.physical.flags.affected_by_gravity = false;
            object.physical.set_move_speed(0.0, 0.0, 0.0);
        }
    }
}

pub(crate) fn emit_object_damage_effects(
    result: &CObjectDamageResult,
    position: CVector,
    particles: Option<&mut ParticleEmitter<'_>>,
    mut dmaudio: Option<&mut cDMAudio>,
) {
    if let Some(particles) = particles {
        for debris in &result.debris {
            particles.add_particle_coloured(
                crate::renderer::ParticleType::PARTICLE_CAR_DEBRIS,
                position,
                debris.direction,
                None,
                debris.size,
                ParticleColour {
                    red: debris.colour.red,
                    green: debris.colour.green,
                    blue: debris.colour.blue,
                    alpha: debris.colour.alpha,
                },
                debris.rotation_speed,
                0,
                debris.current_frame,
                0,
            );
        }
    } else if !result.debris.is_empty() {
        bevy::log::warn_once!("object_damage_particles_waiting_original_creation_context");
    }
    if let (Some(dmaudio), Some(sound)) = (dmaudio.as_deref_mut(), result.script_sound) {
        let sound_id = match sound {
            CObjectDamageSound::BoxDestroyed1 => SCRIPT_SOUND_BOX_DESTROYED_1,
            CObjectDamageSound::BoxDestroyed2 => SCRIPT_SOUND_BOX_DESTROYED_2,
            CObjectDamageSound::TireCollision => SCRIPT_SOUND_TIRE_COLLISION,
            CObjectDamageSound::MetalCollision => SCRIPT_SOUND_METAL_COLLISION,
        };
        let mut object = cAudioScriptObject::new(sound_id as i16, position);
        dmaudio.CreateOneShotScriptObject(&mut object);
    }
}

/// ECS ownership equivalent of re3 `CAutomobile::SpawnFlyingComponent`.
/// The generic CAR_* model is intentional: re3 clones the source atomic only
/// for RW ownership, while the CObject model index selects the corresponding
/// loose component physics/model metadata.
pub(crate) fn process_script_vehicle_debris(
    mut script_entities: ScriptEntityWorld,
    mut commands: Commands,
    mut pools: ResMut<CPools>,
    file_loader: Option<Res<CFileLoader>>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
) {
    // CObject::ProcessControl removes temporary flying pieces at this deadline.
    let expired: Vec<i32> = script_entities
        .objects
        .values()
        .filter_map(|entity| {
            (entity.object.object_created_by == TEMP_OBJECT
                && entity.object.end_of_life_time != 0
                && timer.time_in_milliseconds() >= entity.object.end_of_life_time)
                .then_some(entity.handle)
        })
        .collect();
    for handle in expired {
        let _ = remove_script_object(
            &mut commands,
            &mut script_entities.index,
            Some(&mut pools),
            handle,
        );
    }

    let Some(file_loader) = file_loader.as_deref() else {
        return;
    };
    let mut intents = Vec::new();
    for vehicle in script_entities.vehicles.values_mut() {
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle else {
            continue;
        };
        let model_index = car.vehicle.core.physical.entity.model_index.max(0) as usize;
        let vehicle_matrix = car.vehicle.core.physical.entity.matrix;
        let move_speed = car.vehicle.core.physical.move_speed;
        let turn_speed = car.vehicle.core.physical.turn_speed;
        let colour1 = car.vehicle.appearance.current_colour1;
        let colour2 = car.vehicle.appearance.current_colour2;
        for component in car.take_missing_component_transitions() {
            let Some(matrix) =
                vehicle_component_world_matrix(file_loader, model_index, vehicle_matrix, component)
            else {
                // No fabricated centre-of-car debris when the original DFF has
                // no corresponding component frame.
                continue;
            };
            intents.push((
                component,
                model_index,
                matrix,
                vehicle_matrix.position,
                vehicle_matrix.up,
                move_speed,
                turn_speed,
                colour1,
                colour2,
            ));
        }
    }

    let mut temp_count = script_entities
        .objects
        .values()
        .filter(|entity| entity.object.object_created_by == TEMP_OBJECT)
        .count();
    for (
        component,
        source_model_index,
        matrix,
        vehicle_position,
        vehicle_up,
        move_speed,
        turn_speed,
        colour1,
        colour2,
    ) in intents
    {
        if temp_count >= crate::core::config::NUMTEMPOBJECTS {
            break;
        }
        let Some(handle) = pools
            .object_pool
            .new_entry(CPoolEntry::new(CPoolEntryKind::Object))
            .map(|handle| handle.0)
        else {
            break;
        };
        let mut object = CObject::new_with_model(
            flying_component_model(component) as i32,
            false,
            &mut general,
        );
        object.object_created_by = TEMP_OBJECT;
        // CObject::RefModelInfo is resource ownership in re3. Model-info is
        // resource-owned in Bevy, while this metadata preserves the exact
        // source-model relationship for object lifetime/save paths.
        object.ref_model_index = i16::try_from(source_model_index).unwrap_or(-1);
        object.flags.use_vehicle_colours = true;
        object.colour1 = colour1 as i8;
        object.colour2 = colour2 as i8;
        object.physical.entity.set_is_static(false);
        object.physical.entity.matrix = matrix;
        object.physical.entity.set_position(matrix.position);
        object.object_matrix = matrix;
        object.physical.mass = 10.0;
        object.physical.turn_mass = 25.0;
        object.physical.air_resistance = 0.97;
        object.physical.elasticity = 0.1;
        object.physical.buoyancy = object.physical.mass * crate::entities::Physical::GRAVITY / 0.75;
        object.end_of_life_time = timer.time_in_milliseconds()
            + if temp_count + 1 > 20 {
                4_000
            } else if temp_count + 1 > 10 {
                10_000
            } else {
                20_000
            };
        object.physical.move_speed = flying_component_speed(component, move_speed, matrix.up);
        object.physical.turn_speed =
            CVector::new(turn_speed.x * 2.0, turn_speed.y * 2.0, turn_speed.z * 2.0);
        if matches!(
            component,
            CarDebrisComponent::WheelLeftFront
                | CarDebrisComponent::WheelRightFront
                | CarDebrisComponent::WheelLeftRear
                | CarDebrisComponent::WheelRightRear
        ) {
            // re3 wheel branch in SpawnFlyingComponent.
            object.physical.turn_mass = 5.0;
            object.physical.turn_speed.x = 0.5;
            object.physical.air_resistance = 0.99;
        }
        let mut impulse = CVector::new(
            vehicle_position.x - matrix.position.x,
            vehicle_position.y - matrix.position.y,
            vehicle_position.z - matrix.position.z,
        );
        // The source component matrix supplies the actual direction. When the
        // vehicle and pivot coincide, re3's Normalize yields no artificial push.
        if matches!(
            component,
            CarDebrisComponent::Bonnet | CarDebrisComponent::Boot | CarDebrisComponent::Windscreen
        ) {
            impulse = CVector::new(
                impulse.x + vehicle_up.x,
                impulse.y + vehicle_up.y,
                impulse.z + vehicle_up.z,
            );
            if vehicle_up.z > 0.0 {
                let speed_2d = (move_speed.x * move_speed.x + move_speed.y * move_speed.y).sqrt();
                object.physical.entity.matrix.position = CVector::new(
                    object.physical.entity.matrix.position.x + vehicle_up.x * speed_2d,
                    object.physical.entity.matrix.position.y + vehicle_up.y * speed_2d,
                    object.physical.entity.matrix.position.z + vehicle_up.z * speed_2d,
                );
                object
                    .physical
                    .entity
                    .set_position(object.physical.entity.matrix.position);
                object.object_matrix = object.physical.entity.matrix;
            }
        }
        let magnitude = impulse.magnitude();
        if magnitude > 0.0 {
            impulse = CVector::new(
                impulse.x / magnitude,
                impulse.y / magnitude,
                impulse.z / magnitude,
            );
            object.physical.apply_move_force(impulse);
        }
        spawn_script_object(
            &mut commands,
            &mut script_entities.index,
            ScriptObjectEntity { handle, object },
        );
        temp_count += 1;
    }
}


#[cfg(test)]
mod ownership_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn control_preserves_preintegration_pose_in_independent_collision_component() {
        let mut world = bevy::prelude::World::new();
        let mut general = CGeneral::with_seed(7);
        let mut object = CObject::new(&mut general);
        object.physical.entity.flags.is_static = false;
        object.physical.entity.status = crate::entities::Entity::eEntityStatus::STATUS_PHYSICS;
        object.physical.flags.affected_by_gravity = false;
        object.physical.move_speed = CVector::new(1.0, 0.0, 0.0);
        object.physical.entity.set_position(CVector::new(12.0, 8.0, 4.0));
        let initial = object.physical.entity.matrix;
        let entity = world.spawn(ScriptObjectEntity { handle: 0x502, object }.into_bundle()).id();
        world.insert_resource(general);
        world.init_resource::<CTimer>();
        world.init_resource::<CObjectGlobalState>();
        world.run_system_once(process_script_object_control).unwrap();
        assert_eq!(world.get::<ObjectCollisionState>(entity).unwrap().previous_matrix, initial);
        let integrated = world.get::<CObject>(entity).unwrap().physical.entity.matrix;
        assert!(integrated.position.x > initial.position.x);
        world.run_system_once(process_script_object_control).unwrap();
        assert_eq!(world.get::<ObjectCollisionState>(entity).unwrap().previous_matrix, integrated);
    }
}
