//! re3 reference: references/re-GTA/src/vehicles/Boat.cpp
//! re3 reference: references/re-GTA/src/vehicles/Boat.h
//!
//! Boat vehicle state.
//! TODO(parity): CBoat::SetModelIndex / SetupModelNodes / GetComponentWorldPosition
//! authored boat node bindings, CBoat::Render / RenderWaterOutPolys draw interface,
//! CBoat::Save / Load compatible-save payload.

use rust_gta_3_core::core::{
    Coordinates::{CVector, CVector2D},
    General::CGeneral,
};

use crate::{
    entities::EntityReference::EntityReference,
    modelinfo::VehicleModelInfo::eVehicleType,
    vehicles::Vehicle::{CVehicle, USERTRACK},
};

pub const BOAT_MOVING: usize = 1;
pub const BOAT_RUDDER: usize = 2;
pub const BOAT_WINDSCREEN: usize = 3;
pub const NUM_BOAT_NODES: usize = 4;
pub const INVALID_ORIENTATION: f32 = -9999.99;
pub const MAX_WAKE_POINTS: usize = 32;
pub const MAX_WAKE_LENGTH: f32 = 50.0;
pub const MIN_WAKE_INTERVAL: f32 = 1.0;
pub const WAKE_LIFETIME: f32 = 400.0;

#[derive(Debug, Clone, Copy)]
pub struct BoatPropellerEffect {
    pub world_position: CVector,
    pub force: CVector,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BoatWaterEffects {
    pub propeller: Option<BoatPropellerEffect>,
    pub landing_splash: bool,
}

struct BoatEngineSmoke {
    local_position: CVector,
    lateral_velocity: f32,
    vertical_velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CBoatHandlingSnapshot {
    pub mass: f32,
    pub turn_mass: f32,
    pub centre_of_mass: CVector,
    pub dimension: CVector,
    pub buoyancy: f32,
    pub handling_id: i32,
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct BoatState {
    pub thrust_z: f32,
    pub thrust_y: f32,
    pub move_res: CVector,
    pub turn_res: CVector,
    pub moving_rotation: f32,
    pub boat_unused1: i32,
    pub boat_nodes_loaded: [bool; NUM_BOAT_NODES],
    pub boat_in_water: bool,
    pub propeller_in_water: bool,
    pub is_anchored: bool,
    pub orientation: f32,
    pub boat_unused2: i32,
    pub damage: f32,
    pub set_on_fire_entity: Option<EntityReference>,
    pub boat_unused3: bool,
    pub accelerate: f32,
    pub brake: f32,
    pub steering_left_right: f32,
    pub pad_id: u8,
    pub boat_unused4: i32,
    pub volume_under_water: f32,
    pub buoyance_point: CVector,
    pub prev_volume_under_water: f32,
    pub delta_volume_under_water: i16,
    pub num_wake_points: u16,
    pub wake_points: [CVector2D; MAX_WAKE_POINTS],
    pub wake_point_life_time: [f32; MAX_WAKE_POINTS],
    pub(crate) explosion_pending: Option<CVector>,
}
use crate::vehicles::Vehicle::{Owned, VehicleCore, VehicleRuntime, VehicleVitality};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio> {
    pub vehicle: VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>,
    pub state: S,
}
pub type CBoat = BoatRuntimeView<
    Owned<VehicleCore>,
    Owned<VehicleVitality>,
    Owned<BoatState>,
    crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleDriveState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAppearance>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleEntryState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAudioState>>;
pub type BoatRef<'a> = BoatRuntimeView<
    &'a VehicleCore,
    &'a VehicleVitality,
    &'a BoatState,
    &'a crate::vehicles::Vehicle::VehicleDriveState, &'a crate::vehicles::Vehicle::VehicleAppearance, &'a crate::vehicles::Vehicle::VehicleEntryState, &'a crate::vehicles::Vehicle::VehicleAudioState>;
pub type BoatMut<'a> = BoatRuntimeView<
    &'a mut VehicleCore,
    &'a mut VehicleVitality,
    &'a mut BoatState,
    &'a mut crate::vehicles::Vehicle::VehicleDriveState, &'a mut crate::vehicles::Vehicle::VehicleAppearance, &'a mut crate::vehicles::Vehicle::VehicleEntryState, &'a mut crate::vehicles::Vehicle::VehicleAudioState>;
impl<
    C,
    V,
    S: std::ops::Deref<Target = BoatState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::Deref for BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    type Target = BoatState;
    fn deref(&self) -> &BoatState {
        &self.state
    }
}
impl<
    C,
    V,
    S: std::ops::DerefMut<Target = BoatState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::DerefMut for BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    fn deref_mut(&mut self) -> &mut BoatState {
        &mut self.state
    }
}
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = BoatState>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_ref(&self) -> BoatRef<'_> {
        BoatRuntimeView {
            vehicle: self.vehicle.as_ref(),
            state: &self.state,
        }
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = BoatState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn as_mut(&mut self) -> BoatMut<'_> {
        BoatRuntimeView {
            vehicle: self.vehicle.as_mut(),
            state: &mut self.state,
        }
    }
}
impl CBoat {
    pub fn into_parts(
        self,
    ) -> (
        VehicleCore,
        VehicleVitality,
        crate::vehicles::Vehicle::VehicleDriveState,
        crate::vehicles::Vehicle::VehicleAppearance,
        crate::vehicles::Vehicle::VehicleEntryState,
        crate::vehicles::Vehicle::VehicleAudioState,
        BoatState,
    ) {
        let (core, vitality, drive, appearance, entry, audio) = self.vehicle.into_parts();
        (core, vitality, drive, appearance, entry, audio, self.state.0)
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = BoatState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    /// CBoat::BlowUpCar state transition. The ECS damage lifecycle owns
    /// occupant death, explosion/audio and detached authored geometry.
    pub fn blow_up_state(&mut self, now_ms: u32) -> bool {
        if !self.vehicle.vitality.can_be_damaged {
            return false;
        }
        self.vehicle.core.physical.move_speed.z += 0.13;
        self.vehicle.core.physical.entity.status =
            crate::entities::Entity::eEntityStatus::STATUS_WRECKED;
        self.vehicle.core.physical.entity.flags.render_scorched = true;
        self.vehicle.vitality.health = 0.0;
        self.vehicle.vitality.bomb_timer = 0;
        self.vehicle.vitality.time_of_death = now_ms;
        self.vehicle.drive.engine_on = false;
        self.vehicle.appearance.lights_on = false;
        self.vehicle.core.flags.is_law_enforcer = false;
        self.state.explosion_pending = Some(self.vehicle.core.physical.entity.get_position());
        // TODO(parity): CBoat::BlowUpCar detached BOAT_MOVING authored atomic.
        true
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = BoatState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    pub fn process_delayed_explosion(&mut self, time_step: f32, now_ms: u32) -> bool {
        if self.vehicle.vitality.bomb_timer == 0 {
            return false;
        }
        let tick = (time_step / 60.0 * 1000.0) as i16;
        if tick > self.vehicle.vitality.bomb_timer {
            self.vehicle.vitality.bomb_timer = 0;
        } else {
            self.vehicle.vitality.bomb_timer -= tick;
        }
        if self.vehicle.vitality.bomb_timer != 0 {
            return false;
        }
        self.state.set_on_fire_entity = self.vehicle.vitality.blow_up_entity;
        // TODO(parity): CVehicle::ProcessDelayedExplosion / CPlayerInfo::AwardMoneyForExplosion.
        self.blow_up_state(now_ms)
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = BoatState>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
{
    /// CBoat::ProcessControl damaged-engine presentation and fire timer.
    pub fn process_damage_particles(
        &mut self,
        camera_position: CVector,
        time_step_ms: f32,
        now_ms: u32,
        frame_counter: u32,
        general: &mut CGeneral,
        particles: &mut crate::renderer::Particle::ParticleEmitter<'_>,
    ) -> bool {
        use crate::{
            entities::Entity::eEntityStatus,
            modelinfo::ModelIndices::{MI_REEFER, MI_SPEEDER},
            renderer::ParticleType::*,
        };
        let position = self.vehicle.core.physical.entity.get_position();
        if self.vehicle.vitality.health > 600.0
            || self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_WRECKED
            || (position.x - camera_position.x).abs() >= 200.0
            || (position.y - camera_position.y).abs() >= 200.0
        {
            return false;
        }
        let speed = self.vehicle.core.physical.move_speed;
        let right = self.vehicle.core.physical.entity.matrix.right;
        let smoke = match self.vehicle.core.physical.entity.model_index as usize {
            MI_SPEEDER => BoatEngineSmoke {
                local_position: CVector::new(0.4, -2.4, 0.8),
                lateral_velocity: 0.05,
                vertical_velocity: 0.2 * speed.z,
            },
            MI_REEFER => BoatEngineSmoke {
                local_position: CVector::new(2.0, -1.0, 0.5),
                lateral_velocity: 0.07,
                vertical_velocity: 0.0,
            },
            _ => BoatEngineSmoke {
                local_position: CVector::new(-1.5, -0.5, 1.2),
                lateral_velocity: -0.08,
                vertical_velocity: 0.0,
            },
        };
        let smoke_position = self
            .vehicle
            .core
            .physical
            .entity
            .matrix
            .transform_point(smoke.local_position);
        let direction = CVector::new(
            0.8 * speed.x + smoke.lateral_velocity * right.x,
            0.8 * speed.y + smoke.lateral_velocity * right.y,
            0.8 * speed.z + smoke.lateral_velocity * right.z + smoke.vertical_velocity,
        );
        if self.vehicle.vitality.health < 150.0 {
            particles.add_particle(
                PARTICLE_CARFLAME,
                smoke_position,
                CVector::new(
                    0.0,
                    0.0,
                    general.get_random_number_in_range_f32(2.25 / 200.0, 0.09),
                ),
                None,
                0.9,
                0,
                0,
                0,
                0,
            );
            let smoke = CVector::new(
                smoke_position.x + general.get_random_number_in_range_f32(-2.25 / 4.0, 2.25 / 4.0),
                smoke_position.y + general.get_random_number_in_range_f32(-2.25 / 4.0, 2.25 / 4.0),
                smoke_position.z + general.get_random_number_in_range_f32(2.25 / 4.0, 2.25),
            );
            particles.add_particle(
                PARTICLE_ENGINE_SMOKE2,
                smoke,
                CVector::ZERO,
                None,
                0.0,
                0,
                0,
                0,
                0,
            );
            self.state.damage += time_step_ms;
            if self.state.damage > 5000.0 {
                return self.blow_up_state(now_ms);
            }
        }
        if speed.magnitude_sqr() < 0.25 {
            if (frame_counter + self.vehicle.core.physical.entity.random_seed as u32) & 1 != 0 {
                particles.add_particle(
                    PARTICLE_ENGINE_STEAM,
                    smoke_position,
                    direction,
                    None,
                    0.0,
                    0,
                    0,
                    0,
                    0,
                );
            }
            if self.vehicle.vitality.health <= 350.0 {
                particles.add_particle(
                    PARTICLE_ENGINE_SMOKE,
                    smoke_position,
                    CVector::new(direction.x * 1.25, direction.y * 1.25, direction.z * 1.25),
                    None,
                    0.0,
                    0,
                    0,
                    0,
                    0,
                );
            }
        }
        false
    }
}

impl CBoat {
    pub fn new(
        id: i32,
        owner: u8,
        time_ms: u32,
        general: &mut CGeneral,
        handling: Option<CBoatHandlingSnapshot>,
        colours: Option<(u8, u8)>,
    ) -> Self {
        let mut vehicle = CVehicle::new(owner, time_ms, general);
        vehicle.vehicle_type = eVehicleType::VEHICLE_TYPE_BOAT;
        vehicle
            .physical
            .entity
            .set_model_index_no_create(id as usize);

        if let Some(handling) = handling {
            vehicle.handling_id = Some(handling.handling_id);
            vehicle.physical.mass = handling.mass;
            vehicle.physical.turn_mass = handling.turn_mass / 2.0;
            vehicle.physical.centre_of_mass = handling.centre_of_mass;
            vehicle.physical.air_resistance =
                handling.dimension.x * handling.dimension.z / handling.mass;
            vehicle.physical.buoyancy = handling.buoyancy;
        }
        if let Some((colour1, colour2)) = colours {
            vehicle.appearance.current_colour1 = colour1;
            vehicle.appearance.current_colour2 = colour2;
        }
        vehicle.physical.elasticity = 0.1;
        vehicle.drive.steer_angle = 0.0;
        vehicle.drive.gas_pedal = 0.0;
        vehicle.drive.brake_pedal = 0.0;
        vehicle.ammo_in_clip = 20;
        vehicle.audio.radio_station %= USERTRACK;
        vehicle.physical.flags.is_in_water = true;
        vehicle.physical.entity.flags.touching_water = true;

        Self {
            vehicle,
            state: Owned(BoatState {
                thrust_z: 0.25,
                thrust_y: 0.35,
                move_res: CVector::new(0.7, 0.998, 0.999),
                turn_res: CVector::new(0.85, 0.96, 0.96),
                moving_rotation: 0.0,
                boat_unused1: 0,
                boat_nodes_loaded: [false; NUM_BOAT_NODES],
                boat_in_water: true,
                propeller_in_water: true,
                is_anchored: true,
                orientation: INVALID_ORIENTATION,
                boat_unused2: 0,
                damage: 0.0,
                set_on_fire_entity: None,
                boat_unused3: false,
                accelerate: 0.0,
                brake: 0.0,
                steering_left_right: 0.0,
                pad_id: 0,
                boat_unused4: 0,
                volume_under_water: 7.0,
                buoyance_point: CVector::ZERO,
                prev_volume_under_water: 7.0,
                delta_volume_under_water: 0,
                num_wake_points: 0,
                wake_points: [CVector2D::new(0.0, 0.0); MAX_WAKE_POINTS],
                wake_point_life_time: [0.0; MAX_WAKE_POINTS],
                explosion_pending: None,
            }),
        }
    }
}
