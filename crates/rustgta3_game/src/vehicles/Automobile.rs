//! re3 reference: references/re-GTA/src/vehicles/Automobile.cpp
//! re3 reference: references/re-GTA/src/vehicles/Automobile.h
//!
//! Automobile state.

use crate::entities::EntityReference::EntityReference;
use rustgta3_core::core::{Coordinates::CVector, General::CGeneral};
use rustgta3_formats::formats::ClumpRead::RwClumpSummary;


pub use crate::vehicles::Door::CDoor;

use crate::{
    collision::Collision::CCollision,
    core::FileLoader::CFileLoader,
    entities::Entity::eEntityStatus,
    math::Matrix::CMatrix,
    modelinfo::ModelIndices::{
        MI_CAR_BONNET, MI_CAR_BOOT, MI_CAR_BUMPER, MI_CAR_DOOR, MI_CAR_PANEL, MI_CAR_WHEEL, MI_DODO,
    },
    modelinfo::{ModelInfo::ModelInfoSlot, VehicleModelInfo::eVehicleType},
    objects::ParticleObject::CParticleObjectRuntime,
    renderer::{Particle::{CParticle, ParticleEmitter}, Skidmarks::CSkidmarks, Timecycle::CTimeCycle},
    vehicles::{
        DamageManager::{CDamageManager, eDoorStatus, eWheelStatus},
        HandlingMgr::tHandlingData,
        Vehicle::{
            CVehicle, CVehicleProcessWheelInput, CVehicleProcessWheelResult, CVehicleWheelHandling,
            tWheelState,
        },
    },
};

/// re3 `CAutomobile::PreRender` wheel smoke, dirt, water spray and skidmark
/// registration. The ECS schedule supplies frame/time state, while the
/// automobile owns the authored wheel-state behavior.
pub fn process_automobile_wheel_effects<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    particles: &mut ParticleEmitter<'_>,
    skidmarks: Option<&mut CSkidmarks>,
    vehicle_id: usize,
    wet_roads: f32,
    frame_counter: u32,
    time_ms: u32,
    general: &mut CGeneral,
    replay_playback: bool,
) where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    super::AutomobileEffects::wheel_effects(&car.vehicle.core, &mut car.state, particles, skidmarks, vehicle_id, wet_roads, frame_counter, time_ms, general, replay_playback)
}

/// Frame-local render/audio inputs; actor state remains in its canonical owners.
#[derive(Default)]
pub struct AutomobileBuoyancyEffects<'a, 'data> {
    pub particles: Option<&'a mut crate::renderer::Particle::ParticleEmitter<'data>>,
    pub audio: Option<&'a mut crate::audio::DMAudio::cDMAudio>,
    pub frame_counter: u32,
}

pub fn process_automobile_buoyancy<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    dim_min: CVector,
    dim_max: CVector,
    water: &crate::renderer::WaterLevel::CWaterLevel,
    time_ms: u32,
    weather_wind: f32,
    time_step: f32,
    particle_objects: &mut CParticleObjectRuntime,
    general: &mut CGeneral,
    timecycle: Option<&CTimeCycle>,
    mut effects: AutomobileBuoyancyEffects<'_, '_>,
) where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    use crate::{entities::Physical::GRAVITY, vehicles::Floater::process_buoyancy};

    car.vehicle.core.physical.flags.is_in_water = false;
    let Some(result) = process_buoyancy(
        &car.vehicle.core.physical,
        dim_min,
        dim_max,
        car.vehicle.core.physical.buoyancy,
        water,
        time_ms,
        weather_wind,
        time_step,
        false,
        None,
    ) else {
        car.vehicle.core.physical.entity.flags.touching_water = false;
        for wheel in 0..NUM_CAR_WHEELS {
            let Some(speed) = super::AutomobileEffects::wheel_surface_water_effects(
                &car.vehicle.core, &car.state, wheel, effects.particles.as_deref_mut(), general)
            else { continue; };
            if effects.frame_counter & 0xF == 0 {
                match effects.audio.as_deref_mut() {
                    Some(audio) if car.vehicle.core.physical.audio_entity_id >= 0 => audio.PlayOneShot(
                        car.vehicle.core.physical.audio_entity_id,
                        crate::audio::soundlist::SOUND_CAR_SPLASH as u16, 2000.0 * speed),
                    Some(_) => bevy::log::warn_once!("automobile_water_splash_waiting_audio_identity"),
                    None => bevy::log::warn_once!("automobile_water_splash_waiting_audio_manager"),
                }
            }
        }
        return;
    };

    car.vehicle.core.physical.entity.flags.touching_water = true;
    car.vehicle.core.physical.apply_move_force(result.impulse);
    car.vehicle
        .core
        .physical
        .apply_turn_force(result.impulse, result.point);
    // re3 snapshots initialSpeed after ApplyMoveForce/ApplyTurnForce and
    // before water resistance, so the splash trigger includes buoyancy's
    // upward impulse exactly as the original does.
    let initial_speed = car.vehicle.core.physical.move_speed;

    let impulse_ratio = result.impulse.z
        / (GRAVITY * car.vehicle.core.physical.mass.max(f32::EPSILON) * time_step.max(0.01));
    let water_resistance = (1.0 - 0.05 * impulse_ratio)
        .max(0.0)
        .powf(time_step.max(0.0));
    car.vehicle.core.physical.move_speed.x *= water_resistance;
    car.vehicle.core.physical.move_speed.y *= water_resistance;
    car.vehicle.core.physical.move_speed.z *= water_resistance;
    car.vehicle.core.physical.turn_speed.x *= water_resistance;
    car.vehicle.core.physical.turn_speed.y *= water_resistance;
    car.vehicle.core.physical.turn_speed.z *= water_resistance;

    if impulse_ratio > 0.5 {
        car.vehicle.core.physical.flags.is_in_water = true;
        if car.vehicle.core.physical.move_speed.z < -0.1 {
            car.vehicle.core.physical.move_speed.z = -0.1;
        }
    }

    if initial_speed.z < -0.3 && result.impulse.z > 0.3 {
        if let Some(timecycle) = timecycle {
            super::AutomobileEffects::deep_water_splash(&car.vehicle.core,
                timecycle, particle_objects, general, time_ms);
        } else {
            bevy::log::warn_once!("automobile_deep_water_splash_waiting_timecycle");
        }
        if car.vehicle.core.physical.move_speed.z < -0.2 {
            car.vehicle.core.physical.move_speed.z = -0.2;
        }
    }
}

pub fn emit_wheel_dirt_and_water<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    wheel: usize,
    below_effect_speed: bool,
    wet_roads: f32,
    frame_counter: u32,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
) -> bool
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    super::AutomobileEffects::wheel_dirt_and_water(&car.vehicle.core, &car.state, wheel, below_effect_speed, wet_roads, frame_counter, particles, general)
}

pub const NUM_CAR_NODES: usize = 20;
pub const CAR_BONNET_NODE: usize = 17;
pub const CAR_BOOT_NODE: usize = 18;
pub const NUM_CAR_DOORS: usize = 6;
pub const NUM_CAR_WHEELS: usize = 4;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eCarPiece {
    CAR_PIECE_BONNET = 1,
    CAR_PIECE_BOOT = 2,
    CAR_PIECE_BUMP_FRONT = 3,
    CAR_PIECE_BUMP_REAR = 4,
    CAR_PIECE_DOOR_LF = 5,
    CAR_PIECE_DOOR_RF = 6,
    CAR_PIECE_DOOR_LR = 7,
    CAR_PIECE_DOOR_RR = 8,
    CAR_PIECE_WING_LF = 9,
    CAR_PIECE_WING_RF = 10,
    CAR_PIECE_WING_LR = 11,
    CAR_PIECE_WING_RR = 12,
    CAR_PIECE_WHEEL_LF = 13,
    CAR_PIECE_WHEEL_RF = 14,
    CAR_PIECE_WHEEL_LR = 15,
    CAR_PIECE_WHEEL_RR = 16,
    CAR_PIECE_WINDSCREEN = 17,
}

impl eCarPiece {
    pub const fn from_piece_id(piece: u8) -> Option<Self> {
        match piece {
            1 => Some(Self::CAR_PIECE_BONNET),
            2 => Some(Self::CAR_PIECE_BOOT),
            3 => Some(Self::CAR_PIECE_BUMP_FRONT),
            4 => Some(Self::CAR_PIECE_BUMP_REAR),
            5 => Some(Self::CAR_PIECE_DOOR_LF),
            6 => Some(Self::CAR_PIECE_DOOR_RF),
            7 => Some(Self::CAR_PIECE_DOOR_LR),
            8 => Some(Self::CAR_PIECE_DOOR_RR),
            9 => Some(Self::CAR_PIECE_WING_LF),
            10 => Some(Self::CAR_PIECE_WING_RF),
            11 => Some(Self::CAR_PIECE_WING_LR),
            12 => Some(Self::CAR_PIECE_WING_RR),
            13 => Some(Self::CAR_PIECE_WHEEL_LF),
            14 => Some(Self::CAR_PIECE_WHEEL_RF),
            15 => Some(Self::CAR_PIECE_WHEEL_LR),
            16 => Some(Self::CAR_PIECE_WHEEL_RR),
            17 => Some(Self::CAR_PIECE_WINDSCREEN),
            _ => None,
        }
    }
}

pub const CARWHEEL_FRONT_LEFT: usize = 0;
pub const CARWHEEL_REAR_LEFT: usize = 1;
pub const CARWHEEL_FRONT_RIGHT: usize = 2;
pub const CARWHEEL_REAR_RIGHT: usize = 3;

/// re3 `CAutomobile::SpawnFlyingComponent` source node identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarDebrisComponent {
    Bonnet,
    Boot,
    DoorLeftFront,
    DoorRightFront,
    DoorLeftRear,
    DoorRightRear,
    WingLeftFront,
    WingRightFront,
    WingLeftRear,
    WingRightRear,
    Windscreen,
    BumperFront,
    BumperRear,
    WheelLeftFront,
    WheelRightFront,
    WheelLeftRear,
    WheelRightRear,
}

pub(crate) fn door_to_debris_component(door: usize) -> Option<CarDebrisComponent> {
    Some(match door {
        2 => CarDebrisComponent::DoorLeftFront,
        3 => CarDebrisComponent::DoorRightFront,
        4 => CarDebrisComponent::DoorLeftRear,
        5 => CarDebrisComponent::DoorRightRear,
        _ => return None,
    })
}

pub(crate) fn flying_component_model(component: CarDebrisComponent) -> usize {
    match component {
        CarDebrisComponent::Windscreen | CarDebrisComponent::Bonnet => MI_CAR_BONNET,
        CarDebrisComponent::Boot => MI_CAR_BOOT,
        CarDebrisComponent::DoorLeftFront
        | CarDebrisComponent::DoorRightFront
        | CarDebrisComponent::DoorLeftRear
        | CarDebrisComponent::DoorRightRear => MI_CAR_DOOR,
        CarDebrisComponent::BumperFront | CarDebrisComponent::BumperRear => MI_CAR_BUMPER,
        CarDebrisComponent::WheelLeftFront
        | CarDebrisComponent::WheelRightFront
        | CarDebrisComponent::WheelLeftRear
        | CarDebrisComponent::WheelRightRear => MI_CAR_WHEEL,
        CarDebrisComponent::WingLeftFront
        | CarDebrisComponent::WingRightFront
        | CarDebrisComponent::WingLeftRear
        | CarDebrisComponent::WingRightRear => MI_CAR_PANEL,
    }
}

pub(crate) fn flying_component_speed(
    component: CarDebrisComponent,
    speed: CVector,
    vehicle_up: CVector,
) -> CVector {
    let mut result = CVector::new(speed.x * 0.75, speed.y * 0.75, speed.z);
    if result.z > 0.0 {
        result.z *= 1.5;
    } else if vehicle_up.z > 0.0
        && matches!(
            component,
            CarDebrisComponent::Bonnet | CarDebrisComponent::Boot | CarDebrisComponent::Windscreen
        )
    {
        result.z = result.z * -1.5 + 0.04;
    } else {
        result.z *= 0.25;
    }
    result
}

pub(crate) fn original_model_id<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
) -> usize
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    car.vehicle.core.physical.entity.model_index.max(0) as usize
}

pub(crate) fn automobile_door_missing<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    door: usize,
) -> bool
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    car.damage.get_door_status(door as u8) == eDoorStatus::DOOR_STATUS_MISSING as u32
}

/// Script-created automobile base offset after `CAutomobile` has run
/// `SetupSuspensionLines`, matching re3's effective COL minimum.
pub(crate) fn script_automobile_base_z<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    model_info: Option<&ModelInfoSlot>,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
) -> f32
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let authored_min = model_info
        .and_then(|slot| slot.base().col_model.as_ref())
        .map(|col| col.bounding_box.min.z)
        .unwrap_or(0.0);
    let effective_min = if car.suspension_initialized {
        authored_min.min(car.suspension_line_end[0].z)
    } else {
        authored_min
    };
    -effective_min
}

/// World transform of the authored RenderWare component frame used by
/// `CAutomobile::SpawnFlyingComponent` and ped vehicle-door adapters.
pub(crate) fn vehicle_component_world_matrix(
    file_loader: &CFileLoader,
    model_index: usize,
    vehicle_matrix: CMatrix,
    component: CarDebrisComponent,
) -> Option<CMatrix> {
    let summary = file_loader
        .model_info
        .get_model_info(model_index)?
        .base()
        .clump_summary
        .as_ref()?;
    let names: &[&str] = match component {
        CarDebrisComponent::Bonnet => &["bonnet_dummy", "bonnet"],
        CarDebrisComponent::Boot => &["boot_dummy", "boot"],
        CarDebrisComponent::DoorLeftFront => &["door_lf_dummy", "door_lf"],
        CarDebrisComponent::DoorRightFront => &["door_rf_dummy", "door_rf"],
        CarDebrisComponent::DoorLeftRear => &["door_lr_dummy", "door_lr"],
        CarDebrisComponent::DoorRightRear => &["door_rr_dummy", "door_rr"],
        CarDebrisComponent::WingLeftFront => &["wing_lf_dummy", "wing_lf"],
        CarDebrisComponent::WingRightFront => &["wing_rf_dummy", "wing_rf"],
        CarDebrisComponent::WingLeftRear => &["wing_lr_dummy", "wing_lr"],
        CarDebrisComponent::WingRightRear => &["wing_rr_dummy", "wing_rr"],
        CarDebrisComponent::Windscreen => &["windscreen_dummy", "windscreen"],
        CarDebrisComponent::WheelLeftFront => &["wheel_lf_dummy", "wheel_lf"],
        CarDebrisComponent::WheelRightFront => &["wheel_rf_dummy", "wheel_rf"],
        CarDebrisComponent::WheelLeftRear => {
            &["wheel_lm_dummy", "wheel_lb_dummy", "wheel_lm", "wheel_lb"]
        }
        CarDebrisComponent::WheelRightRear => {
            &["wheel_rm_dummy", "wheel_rb_dummy", "wheel_rm", "wheel_rb"]
        }
        CarDebrisComponent::BumperFront => &["bump_front_dummy", "bump_front"],
        CarDebrisComponent::BumperRear => &["bump_rear_dummy", "bump_rear"],
    };
    let frame_index = summary.frames.iter().position(|frame| {
        frame.name.as_deref().is_some_and(|name| {
            names
                .iter()
                .any(|candidate| name.eq_ignore_ascii_case(candidate))
        })
    })?;
    let local = clump_frame_matrix(summary, frame_index)?;
    Some(compose_game_matrix(vehicle_matrix, local))
}

/// Resolve a clump frame to the accumulated GTA matrix produced after
/// walking every RenderWare parent frame.
pub(crate) fn clump_frame_matrix(summary: &RwClumpSummary, frame_index: usize) -> Option<CMatrix> {
    let mut chain = Vec::new();
    let mut current = frame_index;
    loop {
        let frame = summary.frames.get(current)?;
        chain.push(current);
        if frame.parent_index < 0 {
            break;
        }
        current = usize::try_from(frame.parent_index).ok()?;
        if chain.len() > summary.frames.len() {
            return None;
        }
    }
    let mut matrix = CMatrix::unity();
    for index in chain.into_iter().rev() {
        let frame = summary.frames.get(index)?;
        let local = CMatrix {
            right: CVector::new(frame.right[0], frame.right[1], frame.right[2]),
            // RenderWare stores the basis as right/up/at. re3's CMatrix
            // update maps those fields to GTA right/forward/up.
            forward: CVector::new(frame.up[0], frame.up[1], frame.up[2]),
            up: CVector::new(frame.at[0], frame.at[1], frame.at[2]),
            position: CVector::new(frame.position[0], frame.position[1], frame.position[2]),
            has_rw_matrix: false,
        };
        matrix = compose_game_matrix(matrix, local);
    }
    Some(matrix)
}

fn compose_game_matrix(parent: CMatrix, child: CMatrix) -> CMatrix {
    CMatrix {
        right: parent.multiply3x3(child.right),
        forward: parent.multiply3x3(child.forward),
        up: parent.multiply3x3(child.up),
        position: parent.transform_point(child.position),
        has_rw_matrix: false,
    }
}

/// Resolve the authored headlights dummy used by
/// `CAutomobile::AddDamagedVehicleParticles` before asking the automobile for
/// its final engine-damage particle position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomobileDamageOrigin { Engine, Fire { looking_forward_first_person: bool } }

/// Read the original descriptor that shares residency with its vehicle clump.
pub(crate) fn automobile_damage_particle_position<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    file_loader: &CFileLoader,
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    origin: AutomobileDamageOrigin,
) -> Option<CVector>
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    super::AutomobileEngineEffects::resident_origin(file_loader, &car.vehicle.core, &car.vehicle.vitality, &car.state, origin)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eBombType {
    CARBOMB_NONE = 0,
    CARBOMB_TIMED = 1,
    CARBOMB_ONIGNITION = 2,
    CARBOMB_REMOTE = 3,
    CARBOMB_TIMEDACTIVE = 4,
    CARBOMB_ONIGNITIONACTIVE = 5,
}

pub(crate) fn script_bomb_type(value: i32) -> eBombType {
    match value {
        1 => eBombType::CARBOMB_TIMED,
        2 => eBombType::CARBOMB_ONIGNITION,
        3 => eBombType::CARBOMB_REMOTE,
        4 => eBombType::CARBOMB_TIMEDACTIVE,
        5 => eBombType::CARBOMB_ONIGNITIONACTIVE,
        _ => eBombType::CARBOMB_NONE,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAutomobileHandlingSnapshot {
    pub mass: f32,
    pub turn_mass: f32,
    pub centre_of_mass: CVector,
    pub dimension: CVector,
    pub buoyancy: f32,
    pub flags: u32,
    pub handling_id: i32,
    pub monetary_value: i32,
}

/// Normalized pad values consumed by re3
/// `CAutomobile::ProcessControlInputs`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CAutomobileControlInput {
    /// re3 `GetSteeringLeftRight`, in the original -128..128 range.
    pub steering_left_right: f32,
    /// re3 `GetMouseX` raw horizontal motion; used when PC mouse steering is enabled.
    pub mouse_x: f32,
    pub mouse_steering: bool,
    /// re3 `GetAccelerate`, in the original 0..255 range.
    pub accelerate: f32,
    /// re3 `GetBrake`, in the original 0..255 range.
    pub brake: f32,
    pub handbrake: bool,
    pub exit_vehicle: bool,
    /// re3 `CPad::ArePlayerControlsDisabled`, applied after pedal selection.
    pub controls_disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAutomobileWheelProcessInput {
    pub contact_speeds: [CVector; NUM_CAR_WHEELS],
    pub contact_points: [CVector; NUM_CAR_WHEELS],
    pub adhesive_limits: [f32; NUM_CAR_WHEELS],
    pub acceleration: f32,
    pub brake: f32,
    pub traction: f32,
    pub traction_bias_front: f32,
    pub traction_bias_rear: f32,
    pub brake_bias_front: f32,
    pub brake_bias_rear: f32,
    pub front_wheel_drive: bool,
    pub rear_wheel_drive: bool,
    pub wheel_state_scratch: [tWheelState; NUM_CAR_WHEELS],
    pub already_skidding: bool,
    pub time_step: f32,
    pub handling: CVehicleWheelHandling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAutomobileWheelProcessResult {
    pub wheel_state_scratch: [tWheelState; NUM_CAR_WHEELS],
    pub already_skidding: bool,
    pub wheel_results: [Option<CVehicleProcessWheelResult>; NUM_CAR_WHEELS],
}

/// Inputs owned by the world/contact adapter around re3
/// `CAutomobile::ProcessControl`. Contact points are vectors from the vehicle
/// origin in world axes, exactly like the C++ `contactPoints[]` array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAutomobileDriveStepInput {
    pub control: CAutomobileControlInput,
    pub local_contact_points: [CVector; NUM_CAR_WHEELS],
    pub contact_points: [CVector; NUM_CAR_WHEELS],
    /// Rescaled re3 suspension ratios: 0 is fully compressed, 1 extended.
    pub spring_ratios: [f32; NUM_CAR_WHEELS],
    /// `Multiply3x3(GetMatrix(), line.p1-line.p0)`, normalised in world axes.
    pub spring_directions: [CVector; NUM_CAR_WHEELS],
    pub grounded: [bool; NUM_CAR_WHEELS],
    pub collision_entities: [Option<EntityReference>; NUM_CAR_WHEELS],
    pub collision_time_ms: u32,
    pub adhesive_limits: [f32; NUM_CAR_WHEELS],
    pub wheel_scale: f32,
    pub time_step: f32,
    pub time_step_seconds: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAutomobileDriveStepResult {
    pub forward_speed: f32,
    pub acceleration: f32,
    pub wheels_on_ground: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CAutomobileDamageStepResult {
    pub on_fire: bool,
    pub exploded: bool,
}

pub const HANDLING_1G_BOOST: u32 = 0x1;
pub const HANDLING_2G_BOOST: u32 = 0x2;
pub const HANDLING_REV_BONNET: u32 = 0x4;
pub const HANDLING_HANGING_BOOT: u32 = 0x8;
pub const HANDLING_NO_DOORS: u32 = 0x10;
pub const HANDLING_IS_VAN: u32 = 0x20;
pub const HANDLING_IS_BUS: u32 = 0x40;
pub const HANDLING_IS_LOW: u32 = 0x80;
pub const HANDLING_DBL_EXHAUST: u32 = 0x100;
pub const HANDLING_TAILGATE_BOOT: u32 = 0x200;
pub const HANDLING_NOSWING_BOOT: u32 = 0x400;
pub const HANDLING_NONPLAYER_STABILISER: u32 = 0x800;
pub const HANDLING_NEUTRALHANDLING: u32 = 0x1000;
pub const HANDLING_HAS_NO_ROOF: u32 = 0x2000;
pub const HANDLING_IS_BIG: u32 = 0x4000;
pub const HANDLING_HALOGEN_LIGHTS: u32 = 0x8000;

/// Transient preintegration pose borrowed by the separate collision stage.
/// Mirrors `CPhysical::ProcessCollision` savedMatrix; not part of save data.
#[derive(Debug, Clone, Copy, Default, bevy::prelude::Component)]
pub struct VehicleCollisionState {
    pub previous_matrix: CMatrix,
}

/// Actor-owned smoothing and previous forward speed used by native vehicle
/// audio decisions and HydraulicControl. Function-static player history has a
/// separate Resource lifetime.
#[derive(Debug, Clone, Copy, Default, PartialEq, bevy::prelude::Component)]
pub struct AutomobileAudioMotion {
    pub gas_pedal_audio: f32,
    pub velocity_change_for_audio: f32,
}

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
#[require(AutomobileAudioMotion)]
pub struct AutomobileState {
    pub damage: CDamageManager,
    pub doors: [CDoor; NUM_CAR_DOORS],
    pub wheel_col_points: [Option<usize>; NUM_CAR_WHEELS],
    pub suspension_spring_ratio: [f32; NUM_CAR_WHEELS],
    pub suspension_spring_ratio_prev: [f32; NUM_CAR_WHEELS],
    pub wheel_timer: [f32; NUM_CAR_WHEELS],
    pub auto_unused1: f32,
    pub wheel_skidmark_muddy: [bool; NUM_CAR_WHEELS],
    pub wheel_skidmark_bloody: [bool; NUM_CAR_WHEELS],
    pub wheel_contact_points: [CVector; NUM_CAR_WHEELS],
    pub wheel_contact_surfaces: [u8; NUM_CAR_WHEELS],
    pub wheel_rotation: [f32; NUM_CAR_WHEELS],
    pub wheel_position: [f32; NUM_CAR_WHEELS],
    pub wheel_speed: [f32; NUM_CAR_WHEELS],
    pub auto_unused2: u8,
    pub bomb_type: eBombType,
    pub taxi_light: bool,
    pub driver_last_frame: bool,
    pub fixed_colour: bool,
    pub big_wheels: bool,
    pub water_tight: bool,
    pub not_damaged_upside_down: bool,
    pub more_resistant_to_damage: bool,
    pub handling_no_doors: bool,
    /// `pHandling->nMonetaryValue`, retained for the crusher reward formula.
    pub handling_monetary_value: i32,
    pub bomb_rigger: Option<usize>,
    pub auto_unk1: i16,
    pub hydraulic_state: u16,
    pub bus_door_timer_end: u32,
    pub bus_door_timer_start: u32,
    pub suspension_spring_length: [f32; NUM_CAR_WHEELS],
    pub suspension_line_length: [f32; NUM_CAR_WHEELS],
    /// Model-space suspension line endpoints created by re3
    /// `SetupSuspensionLines`.  COL files may contain stale/placeholder lines;
    /// the runtime uses these authored DFF wheel positions once initialized.
    pub suspension_line_start: [CVector; NUM_CAR_WHEELS],
    pub suspension_line_end: [CVector; NUM_CAR_WHEELS],
    pub suspension_initialized: bool,
    /// Derived source identity for these parameters, never another model/pose copy.
    pub(crate) suspension_source: Option<AutomobileSuspensionSource>,
    pub height_above_road: f32,
    pub traction: f32,
    pub random_values: [f32; 6],
    pub fire_blow_up_timer: f32,
    pub ground_physical: [Option<usize>; NUM_CAR_WHEELS],
    pub ground_offset: [CVector; NUM_CAR_WHEELS],
    pub set_on_fire_entity: Option<EntityReference>,
    pub weapon_door_timer_left: f32,
    pub weapon_door_timer_right: f32,
    pub car_gun_lr: f32,
    pub car_gun_ud: f32,
    pub propeller_rotation: f32,
    pub wheels_on_ground: u8,
    pub drive_wheels_on_ground: u8,
    pub drive_wheels_on_ground_prev: u8,
    pub wheel_state: [tWheelState; NUM_CAR_WHEELS],
    /// Previous CDamageManager values observed by the ECS debris bridge.
    /// Initial missing doors (for HANDLING_NO_DOORS) are seeded here and never
    /// fabricate flying debris; only a live transition to MISSING emits one.
    pub observed_door_status: [u8; NUM_CAR_DOORS],
    pub observed_panel_status: [u8; 7],
    pub observed_wheel_status: [u8; NUM_CAR_WHEELS],
}
use crate::vehicles::Vehicle::{Owned, VehicleCore, VehicleRuntime, VehicleVitality};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio> {
    pub vehicle: VehicleRuntime<C, V, Drive, Appearance, Entry, Audio>,
    pub state: S,
    pub audio_motion: AutoAudio,
}
pub type CAutomobile = AutomobileRuntimeView<
    Owned<VehicleCore>,
    Owned<VehicleVitality>,
    Owned<AutomobileState>,
    crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleDriveState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAppearance>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleEntryState>, crate::vehicles::Vehicle::Owned<crate::vehicles::Vehicle::VehicleAudioState>, Owned<AutomobileAudioMotion>>;
pub type AutomobileRef<'a> = AutomobileRuntimeView<
    &'a VehicleCore,
    &'a VehicleVitality,
    &'a AutomobileState,
    &'a crate::vehicles::Vehicle::VehicleDriveState, &'a crate::vehicles::Vehicle::VehicleAppearance, &'a crate::vehicles::Vehicle::VehicleEntryState, &'a crate::vehicles::Vehicle::VehicleAudioState, &'a AutomobileAudioMotion>;
pub type AutomobileMut<'a> = AutomobileRuntimeView<
    &'a mut VehicleCore,
    &'a mut VehicleVitality,
    &'a mut AutomobileState,
    &'a mut crate::vehicles::Vehicle::VehicleDriveState, &'a mut crate::vehicles::Vehicle::VehicleAppearance, &'a mut crate::vehicles::Vehicle::VehicleEntryState, &'a mut crate::vehicles::Vehicle::VehicleAudioState, &'a mut AutomobileAudioMotion>;
impl<
    C,
    V,
    S: std::ops::Deref<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::Deref for AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    type Target = AutomobileState;
    fn deref(&self) -> &AutomobileState {
        &self.state
    }
}
impl<
    C,
    V,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> std::ops::DerefMut for AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn deref_mut(&mut self) -> &mut AutomobileState {
        &mut self.state
    }
}
impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    pub fn as_ref(&self) -> AutomobileRef<'_> {
        AutomobileRuntimeView {
            vehicle: self.vehicle.as_ref(),
            state: &self.state,
            audio_motion: &self.audio_motion,
        }
    }
}
impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    pub fn as_mut(&mut self) -> AutomobileMut<'_> {
        AutomobileRuntimeView {
            vehicle: self.vehicle.as_mut(),
            state: &mut self.state,
            audio_motion: &mut self.audio_motion,
        }
    }
}
impl CAutomobile {
    pub fn into_parts(
        self,
    ) -> (
        VehicleCore,
        VehicleVitality,
        crate::vehicles::Vehicle::VehicleDriveState,
        crate::vehicles::Vehicle::VehicleAppearance,
        crate::vehicles::Vehicle::VehicleEntryState,
        crate::vehicles::Vehicle::VehicleAudioState,
        AutomobileState,
        AutomobileAudioMotion,
    ) {
        let (core, vitality, drive, appearance, entry, audio) = self.vehicle.into_parts();
        (core, vitality, drive, appearance, entry, audio, self.state.0, self.audio_motion.0)
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `AddDamagedVehicleParticles` engine origin. `headlights` is the
    /// authored `CAR_POS_HEADLIGHTS` dummy extracted from the original DFF.
    pub fn damage_particle_position(&self, headlights: CVector, origin: AutomobileDamageOrigin) -> CVector {
        super::AutomobileEngineEffects::origin_position(&self.vehicle.core, &self.vehicle.vitality, &self.state, headlights, origin)
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// The non-rendering tail of re3 `VehicleDamage` +
    /// `AddDamagedVehicleParticles`.  ECS adapters own particles, occupants,
    /// explosion propagation and audio, while this method owns the exact car
    /// state transition and five-second timer.
    pub fn process_damage_step(
        &mut self,
        _time_step: f32,
        time_step_ms: f32,
        now_ms: u32,
    ) -> CAutomobileDamageStepResult {
        if self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_WRECKED {
            return CAutomobileDamageStepResult::default();
        }
        if self.state.damage.get_engine_status() > 225 && self.vehicle.vitality.health > 250.0 {
            // re3 intentionally applies this once per ProcessControl call; it
            // is not multiplied by CTimer::GetTimeStep().
            self.vehicle.vitality.health -= 2.0;
        }
        if self.vehicle.vitality.health >= 250.0 {
            self.state.fire_blow_up_timer = 0.0;
            return CAutomobileDamageStepResult::default();
        }
        if self.state.damage.get_engine_status() < 225 {
            self.state.damage.set_engine_status(225);
            self.state.fire_blow_up_timer = 0.0;
        }
        self.state.fire_blow_up_timer += time_step_ms.max(0.0);
        if self.state.fire_blow_up_timer <= 5_000.0 {
            return CAutomobileDamageStepResult {
                on_fire: true,
                exploded: false,
            };
        }

        self.apply_blow_up_state(now_ms);
        CAutomobileDamageStepResult {
            on_fire: true,
            exploded: true,
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CVehicle::ProcessDelayedExplosion`.
    ///
    /// Returns true on the frame the timer expires and `BlowUpCar` would run.
    pub fn process_delayed_explosion(&mut self, time_step: f32, now_ms: u32) -> bool {
        if self.vehicle.vitality.bomb_timer == 0
            || self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_WRECKED
        {
            return false;
        }

        let tick = (time_step / 60.0 * 1_000.0).max(0.0) as i32;
        if tick > i32::from(self.vehicle.vitality.bomb_timer) {
            self.vehicle.vitality.bomb_timer = 0;
        } else {
            self.vehicle.vitality.bomb_timer =
                (i32::from(self.vehicle.vitality.bomb_timer) - tick) as i16;
        }
        if self.vehicle.vitality.bomb_timer != 0 {
            return false;
        }

        self.state.set_on_fire_entity = self.vehicle.vitality.blow_up_entity;
        self.apply_blow_up_state(now_ms);
        true
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn apply_blow_up_state(&mut self, now_ms: u32) {
        self.vehicle.core.physical.move_speed.z += 0.13;
        self.vehicle.set_status(eEntityStatus::STATUS_WRECKED);
        self.vehicle.vitality.time_of_death = now_ms;
        self.state.damage.fuck_car_completely();
        self.vehicle.vitality.health = 0.0;
        self.vehicle.vitality.bomb_timer = 0;
        self.state.bomb_type = eBombType::CARBOMB_NONE;
        self.vehicle.drive.engine_on = false;
        self.vehicle.appearance.lights_on = false;
        self.vehicle.audio.siren_or_alarm = false;
        self.state.taxi_light = false;
        self.vehicle.core.flags.is_law_enforcer = false;
        self.vehicle.appearance.is_damaged = true;
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Port of `CAutomobile::ResetSuspension`, used by the re3 teleport
    /// boundary after the vehicle matrix and velocities have been reset.
    pub fn reset_suspension(&mut self) {
        self.state.suspension_spring_ratio = [1.0; NUM_CAR_WHEELS];
        self.state.wheel_timer = [0.0; NUM_CAR_WHEELS];
        self.state.wheel_rotation = [0.0; NUM_CAR_WHEELS];
        self.state.wheel_state = [tWheelState::WHEEL_STATE_NORMAL; NUM_CAR_WHEELS];
    }
}

impl CAutomobile {
    /// Return the four authored RenderWare wheel-frame positions in the exact
    /// order used by re3 `CVehicleModelInfo::GetWheelPosn`:
    /// LF, LB, RF, RB. Delegates to the canonical parsed-frame resolver.
    pub fn authored_wheel_positions(summary: &RwClumpSummary) -> Option<[CVector; NUM_CAR_WHEELS]> {
        crate::modelinfo::VehicleModelInfo::VehicleWheelFrames::from_clump(summary).local_positions()
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Port of re3 `CAutomobile::SetupSuspensionLines`.
    ///
    /// RenderWare wheel dummies are the authoritative model-space source.  A
    /// zeroed `CAutomobile` is therefore never allowed to pull wheels to the
    /// vehicle origin; this method computes the spring/line lengths and the
    /// compressed rest height exactly once per loaded model instance.
    pub fn setup_suspension_lines(
        &mut self,
        wheel_positions: [CVector; NUM_CAR_WHEELS],
        wheel_scale: f32,
        handling: &tHandlingData,
    ) {
        self.state.setup_suspension_lines(wheel_positions, wheel_scale, handling);
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Convenience boundary used by streaming/render code once a vehicle DFF
    /// has become resident.
    pub fn setup_suspension_from_clump(
        &mut self,
        summary: &RwClumpSummary,
        wheel_scale: f32,
        handling: &tHandlingData,
    ) -> bool {
        let Some(positions) = CAutomobile::authored_wheel_positions(summary) else {
            return false;
        };
        self.setup_suspension_lines(positions, wheel_scale, handling);
        true
    }

    /// Native suspension construction borrows descriptors owned by the resident model.
    pub fn setup_suspension_from_model(
        &mut self,
        model: &crate::modelinfo::ModelInfo::CVehicleModelInfo,
        handling: &tHandlingData,
    ) -> bool {
        self.state.setup_suspension_from_model(
            self.vehicle.core.physical.entity.model_index, model, handling)
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CAutomobile::GetHeightAboveRoad` (Automobile.cpp:4049-4052).
    pub fn height_above_road(&self) -> f32 {
        self.state.height_above_road
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CAutomobile::PlaceOnRoadProperly` (Automobile.cpp:3180-3222).
    ///
    /// Probes front and rear ground elevations using vertical lines along the
    /// vehicle's forward axis, orienting Right/Forward/Up matrix basis to
    /// match the terrain/road slope and elevating the center of mass by
    /// `height_above_road`.
    pub fn place_on_road_properly<F>(
        &mut self,
        bbox_min: CVector,
        bbox_max: CVector,
        mut vertical_line_probe: F,
    ) where
        F: FnMut(CVector, f32) -> Option<(f32, usize)>,
    {
        let len_fwd = bbox_max.y;
        let len_back = -bbox_min.y;
        let len = (len_fwd + len_back).max(0.1);

        let pos = self.vehicle.core.physical.entity.get_position();
        let fwd = self.vehicle.core.physical.entity.get_forward();

        let front = CVector::new(
            pos.x + fwd.x * len_fwd,
            pos.y + fwd.y * len_fwd,
            pos.z + 5.0,
        );
        let front_z = if let Some((z, entity)) = vertical_line_probe(front, pos.z - 5.0) {
            self.vehicle.core.current_ground_entity = Some(entity);
            z
        } else {
            pos.z
        };

        let rear = CVector::new(
            pos.x - fwd.x * len_back,
            pos.y - fwd.y * len_back,
            pos.z + 5.0,
        );
        let rear_z = if let Some((z, entity)) = vertical_line_probe(rear, pos.z - 5.0) {
            self.vehicle.core.current_ground_entity = Some(entity);
            z
        } else {
            pos.z
        };

        let angle = ((front_z - rear_z) / len).atan();
        let c = angle.cos();
        let s = angle.sin();

        let right = CVector::new((front.y - rear.y) / len, -(front.x - rear.x) / len, 0.0);
        let forward = CVector::new(-c * right.y, c * right.x, s);
        let up = right.cross(forward);

        let position = CVector::new(
            (front.x + rear.x) / 2.0,
            (front.y + rear.y) / 2.0,
            (front_z + rear_z) / 2.0 + self.state.height_above_road,
        );

        let matrix = &mut self.vehicle.core.physical.entity.matrix;
        matrix.right = right;
        matrix.forward = forward;
        matrix.up = up;
        matrix.position = position;
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CAutomobile::ProcessControl` player horn and short-tap siren logic.
    pub fn process_player_horn(&mut self, pad: &crate::core::Pad::CPad, strong_grip: bool) {
        use crate::modelinfo::ModelIndices::*;
        let model = self.vehicle.core.physical.entity.model_index.max(0) as usize;
        let [
            horn_now,
            horn_previous,
            horn_two_samples_ago,
            _,
            horn_four_samples_ago,
        ] = pad.horn_history;
        if model == MI_MRWHOOP {
            if horn_now && !horn_previous {
                self.vehicle.audio.siren_or_alarm = !self.vehicle.audio.siren_or_alarm;
            }
        } else if matches!(
            model,
            MI_FIRETRUCK | MI_AMBULAN | MI_FBICAR | MI_POLICE | MI_ENFORCER | MI_PREDATOR
        ) {
            self.vehicle.audio.car_horn_timer =
                u8::from(horn_now && horn_previous && horn_two_samples_ago);
            if !horn_now && horn_previous && !horn_four_samples_ago {
                self.vehicle.audio.siren_or_alarm = !self.vehicle.audio.siren_or_alarm;
            }
        } else if model != MI_YARDIE && !strong_grip {
            self.vehicle.audio.car_horn_timer = u8::from(pad.get_horn());
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Direct state port of re3 `CAutomobile::ProcessControlInputs`.
    ///
    /// Deterministic pad input branch used by gameplay and the developer arena.
    pub fn process_control_inputs(
        &mut self,
        input: CAutomobileControlInput,
        handling: &tHandlingData,
        time_step: f32,
        general: &mut CGeneral,
    ) {
        let forward = self.vehicle.core.physical.entity.get_forward();
        let velocity = self.vehicle.core.physical.move_speed;
        let speed = velocity.x * forward.x + velocity.y * forward.y + velocity.z * forward.z;

        self.vehicle.drive.is_handbrake_on = input.exit_vehicle || input.handbrake;

        // re3 Automobile.cpp ProcessControlInputs: mouse motion takes priority.
        // When the mouse was the last steering device, idle frames hold the
        // current wheel angle until keyboard/pad steering resumes.
        if input.mouse_steering && input.mouse_x != 0.0 {
            self.vehicle.drive.steer_input -= 0.0035 * input.mouse_x;
            self.vehicle.drive.mouse_steering_active = true;
            if self.vehicle.drive.steer_input.abs() < 0.35 {
                self.vehicle.drive.steer_input *= 0.975_f32.powf(time_step);
            }
        } else if !input.mouse_steering
            || input.steering_left_right != 0.0
            || !self.vehicle.drive.mouse_steering_active
        {
            let target_steer = -input.steering_left_right / 128.0;
            self.vehicle.drive.steer_input +=
                (target_steer - self.vehicle.drive.steer_input) * 0.2 * time_step.max(0.0);
            self.vehicle.drive.mouse_steering_active = false;
        }
        self.vehicle.drive.steer_input = self.vehicle.drive.steer_input.clamp(-1.0, 1.0);

        let mut acceleration = (input.accelerate - input.brake) / 255.0;
        if self.vehicle.core.physical.entity.model_index == MI_DODO as i16 && acceleration < 0.0 {
            acceleration *= 0.3;
        }
        if speed.abs() < 0.01 {
            self.vehicle.drive.gas_pedal = acceleration;
            self.vehicle.drive.brake_pedal = 0.0;
        } else if speed * acceleration < 0.0 {
            self.vehicle.drive.gas_pedal = 0.0;
            self.vehicle.drive.brake_pedal = acceleration.abs();
        } else {
            self.vehicle.drive.gas_pedal = acceleration;
            self.vehicle.drive.brake_pedal = 0.0;
        }

        let steer_curve =
            self.vehicle.drive.steer_input.signum() * self.vehicle.drive.steer_input.powi(2);
        self.vehicle.drive.steer_angle = handling.steering_lock.to_radians() * steer_curve;
        self.vehicle.drive.apply_comedy_controls(general);

        if input.controls_disabled {
            self.vehicle.drive.brake_pedal = 1.0;
            self.vehicle.drive.is_handbrake_on = true;
            self.vehicle.drive.gas_pedal = 0.0;
            let speed = self.vehicle.core.physical.move_speed.magnitude();
            if speed > 0.28 {
                let scale = 0.28 / speed;
                self.vehicle.core.physical.move_speed = CVector::new(
                    self.vehicle.core.physical.move_speed.x * scale,
                    self.vehicle.core.physical.move_speed.y * scale,
                    self.vehicle.core.physical.move_speed.z * scale,
                );
            }
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Player-controlled automobile portion of re3
    /// `CAutomobile::ProcessControl` shared by gameplay and the developer
    /// arena. World collision owns the four contact points; this method owns
    /// pad smoothing, transmission, wheel forces and physical integration.
    pub fn process_player_drive_step(
        &mut self,
        input: CAutomobileDriveStepInput,
        handling: &mut tHandlingData,
        wheel_friction: f32,
        general: &mut CGeneral,
        damage_nodes: Option<&crate::modelinfo::VehicleModelInfo::VehicleDamageNodes>,
    ) -> CAutomobileDriveStepResult {
        let time_step = input.time_step.max(0.0);
        let dt = input.time_step_seconds.max(0.0);
        // re3 advances the physical matrix with the velocity accumulated by
        // the previous frame before CPhysical::ProcessControl and this
        // frame's suspension/tyre forces.  Contacts supplied to this function
        // are probed against that same predicted pose (see
        // `automobile_road_contacts`).  Advancing after tyre forces instead
        // made steering use newly-generated angular velocity immediately,
        // while its suspension rays still belonged to the old pose; at speed
        // that could drop all wheel contacts for a frame and make the engine
        // audio repeatedly fall back to its low-gear/rev path.
        let previous_matrix = self.vehicle.core.physical.entity.matrix;
        let integrated_matrix = integrate_physical_matrix(
            previous_matrix,
            self.vehicle.core.physical.move_speed,
            self.vehicle.core.physical.turn_speed,
            time_step,
        );
        self.vehicle
            .core
            .physical
            .entity
            .set_matrix(integrated_matrix);
        let damage_impulse = self.vehicle.core.physical.damage.impulse;
        let damaged_piece = self.vehicle.core.physical.damage.piece_type;
        if damage_impulse > 0.0
            && let Some(piece) = eCarPiece::from_piece_id(damaged_piece.max(0) as u8)
        {
            self.apply_collision_piece_damage_internal(
                damage_impulse,
                piece,
                handling.collision_damage_multiplier,
                1.0,
                general,
                damage_nodes,
            );
        }
        if self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PLAYER {
            self.process_control_inputs(input.control, handling, time_step, general);
        } else if self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PLAYER_DISABLED
        {
            // re3 CAutomobile::ProcessControl STATUS_PLAYER_DISABLED:
            // m_fBrakePedal = 1.0f, bIsHandbrakeOn = true, m_fSteerAngle = 0.0f, m_fGasPedal = 0.0f
            self.vehicle.drive.is_handbrake_on = true;
            self.vehicle.drive.gas_pedal = 0.0;
            self.vehicle.drive.brake_pedal = 1.0;
            self.vehicle.drive.steer_angle = 0.0;
            self.vehicle.drive.steer_input = 0.0;
        } else {
            // CAutomobile::ProcessControl only calls ProcessControlInputs in
            // STATUS_PLAYER. ABANDONED/WRECKED/PHYSICS use their already
            // resolved pedals. Feeding an abandoned car's holding brake into
            // player controls converted it to reverse gas at zero speed.
            self.vehicle.drive.is_handbrake_on = input.control.handbrake;
            self.vehicle.drive.gas_pedal = (input.control.accelerate / 255.0).clamp(-1.0, 1.0);
            self.vehicle.drive.brake_pedal = (input.control.brake / 255.0).clamp(0.0, 1.0);
        }

        // CAutomobile::ProcessControl calls CPhysical::ProcessControl before
        // sampling forward speed for transmission and tyre work. Gravity,
        // accumulated friction and air resistance therefore belong before
        // CalculateDriveAcceleration, not after it.
        let _ = self
            .vehicle
            .core
            .physical
            .process_control_pre_physics(time_step);
        // re3's CAutomobile::ProcessEntityCollision adds a collision record
        // when a suspension line hits the road. Script opcode
        // IS_CAR_IN_AIR_PROPER relies on those records, so rebuild them after
        // CPhysical::ProcessControl clears the previous frame.
        for entity in input.collision_entities.into_iter().flatten() {
            self.vehicle
                .core
                .physical
                .add_collision_record(entity, input.collision_time_ms);
        }

        let front_wheel_drive = handling.transmission.drive_type != 'R';
        let rear_wheel_drive = handling.transmission.drive_type != 'F';
        let mut grounded = input.grounded;
        self.state.suspension_spring_ratio_prev = self.state.suspension_spring_ratio;
        self.state.suspension_spring_ratio = input.spring_ratios.map(|ratio| ratio.clamp(0.0, 1.0));
        let forward = self.vehicle.core.physical.entity.get_forward();
        let velocity = self.vehicle.core.physical.move_speed;
        let speed_abs = velocity.dot(forward).abs();
        for wheel in 0..NUM_CAR_WHEELS {
            match self.wheel_status(wheel) {
                eWheelStatus::WHEEL_STATUS_MISSING => {
                    grounded[wheel] = false;
                    self.state.suspension_spring_ratio[wheel] = 1.0;
                }
                eWheelStatus::WHEEL_STATUS_BURST => {
                    let upper = (40.0 * speed_abs) as i32 + 98;
                    if general.get_random_number_in_range_i32(0, upper.max(1)) < 100 {
                        let spring_length =
                            self.state.suspension_spring_length[wheel].max(f32::EPSILON);
                        self.state.suspension_spring_ratio[wheel] =
                            (self.state.suspension_spring_ratio[wheel]
                                + 0.3 * (self.state.suspension_line_length[wheel] - spring_length)
                                    / spring_length)
                                .min(1.0);
                    }
                }
                eWheelStatus::WHEEL_STATUS_OK => {}
            }
        }
        self.state.drive_wheels_on_ground_prev = self.state.drive_wheels_on_ground;
        for (wheel, is_grounded) in grounded.into_iter().enumerate() {
            self.state.wheel_timer[wheel] =
                if is_grounded && self.state.suspension_spring_ratio[wheel] < 1.0 {
                    4.0
                } else {
                    (self.state.wheel_timer[wheel] - time_step).max(0.0)
                };
        }
        self.state.wheels_on_ground = self
            .wheel_timer
            .iter()
            .filter(|timer| **timer > 0.0)
            .count() as u8;
        self.state.drive_wheels_on_ground = self
            .wheel_timer
            .iter()
            .enumerate()
            .filter(|(wheel, timer)| {
                **timer > 0.0
                    && (if matches!(*wheel, CARWHEEL_FRONT_LEFT | CARWHEEL_FRONT_RIGHT) {
                        front_wheel_drive
                    } else {
                        rear_wheel_drive
                    })
            })
            .count() as u8;

        for wheel in 0..NUM_CAR_WHEELS {
            if !grounded[wheel] || self.state.suspension_spring_ratio[wheel] >= 1.0 {
                continue;
            }
            let rear = matches!(wheel, CARWHEEL_REAR_LEFT | CARWHEEL_REAR_RIGHT);
            let bias = if rear {
                1.0 - handling.suspension_bias
            } else {
                handling.suspension_bias
            };
            self.vehicle.core.physical.apply_spring_collision(
                handling.suspension_force_level,
                input.spring_directions[wheel],
                input.contact_points[wheel],
                self.state.suspension_spring_ratio[wheel],
                bias,
                time_step,
            );
        }

        let spring_contact_speeds: [CVector; NUM_CAR_WHEELS] = std::array::from_fn(|wheel| {
            self.vehicle
                .core
                .physical
                .get_speed_at_point(input.contact_points[wheel])
        });
        for wheel in 0..NUM_CAR_WHEELS {
            if !grounded[wheel] || self.state.suspension_spring_ratio[wheel] >= 1.0 {
                continue;
            }
            self.vehicle.core.physical.apply_spring_dampening(
                handling.suspension_damping_level,
                input.spring_directions[wheel],
                input.contact_points[wheel],
                spring_contact_speeds[wheel],
                time_step,
            );
        }
        // re3 samples contact speed again after spring damping before tyre
        // longitudinal/lateral forces are calculated.
        let contact_speeds = std::array::from_fn(|wheel| {
            self.vehicle
                .core
                .physical
                .get_speed_at_point(input.contact_points[wheel])
        });
        let forward = self.vehicle.core.physical.entity.get_forward();
        let velocity = self.vehicle.core.physical.move_speed;
        let forward_speed = velocity.dot(forward);
        let acceleration = {
            let drive = &mut *self.vehicle.drive;
            handling.transmission.calculate_drive_acceleration(
                drive.gas_pedal,
                &mut drive.current_gear,
                &mut drive.change_gear_time,
                forward_speed,
                false,
                time_step,
                dt,
            ) / self.vehicle.core.physical.force_multiplier
        };
        let neutral = handling.flags & HANDLING_NEUTRALHANDLING != 0;
        let brake = self.vehicle.drive.brake_pedal * handling.brake_deceleration * time_step;
        let brake_bias_front = if neutral {
            1.0
        } else {
            2.0 * handling.brake_bias
        };
        let brake_bias_rear = if neutral {
            1.0
        } else {
            2.0 * (1.0 - handling.brake_bias)
        };
        let traction_bias_front = if neutral {
            1.0
        } else {
            2.0 * handling.traction_bias
        };
        let traction_bias_rear = if neutral {
            1.0
        } else {
            2.0 - traction_bias_front
        };
        // CAutomobile::ProcessControl only carries m_fTraction into PHYSICS
        // status. A player car starts from the stock 0.004 coefficient. Both
        // acceleration and traction are divided by m_fForceMultiplier in re3.
        let status_traction =
            if self.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PHYSICS {
                self.state.traction
            } else {
                1.0
            };
        let traction = 0.004 * status_traction * handling.traction_multiplier
            / 4.0
            / self.vehicle.core.physical.force_multiplier;
        self.process_wheels(
            CAutomobileWheelProcessInput {
                contact_speeds,
                contact_points: input.contact_points,
                adhesive_limits: input.adhesive_limits,
                acceleration,
                brake,
                traction,
                traction_bias_front,
                traction_bias_rear,
                brake_bias_front,
                brake_bias_rear,
                front_wheel_drive,
                rear_wheel_drive,
                wheel_state_scratch: self.state.wheel_state,
                already_skidding: false,
                time_step,
                handling: CVehicleWheelHandling {
                    mass: handling.mass,
                    traction_loss: handling.traction_loss,
                    wheel_friction,
                },
            },
            general,
        );

        if self.vehicle.core.physical.entity.status != eEntityStatus::STATUS_PLAYER {
            self.vehicle.audio.reduce_horn_counter();
        }

        // CAutomobile::ProcessControl smooths the visible wheel frame toward
        // the authored suspension-line position every physics frame. Keeping
        // the setup-time value here made the body react to springs while the
        // wheels appeared rigidly attached to it.
        for wheel in 0..NUM_CAR_WHEELS {
            let mut wheel_position = self.state.suspension_line_start[wheel].z;
            if self.state.suspension_spring_ratio[wheel] > 0.0 {
                wheel_position -= self.state.suspension_spring_ratio[wheel]
                    * self.state.suspension_spring_length[wheel];
            }
            self.state.wheel_position[wheel] +=
                (wheel_position - self.state.wheel_position[wheel]) * 0.75;
        }

        self.update_wheel_rotation_for_render(
            input.local_contact_points,
            input.wheel_scale,
            time_step,
        );

        CAutomobileDriveStepResult {
            forward_speed,
            acceleration,
            wheels_on_ground: self.state.wheels_on_ground,
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// State-changing core of re3 `CAutomobile::VehicleDamage`. Particle,
    /// audio, money and pad-shake intents are emitted by ECS adapters.
    pub fn apply_collision_piece_damage(
        &mut self,
        impulse: f32,
        piece: eCarPiece,
        collision_damage_multiplier: f32,
        general: &mut CGeneral,
        damage_nodes: Option<&crate::modelinfo::VehicleModelInfo::VehicleDamageNodes>,
    ) -> bool {
        self.apply_collision_piece_damage_internal(
            impulse,
            piece,
            collision_damage_multiplier,
            0.2,
            general,
            damage_nodes,
        )
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn apply_collision_piece_damage_internal(
        &mut self,
        impulse: f32,
        piece: eCarPiece,
        collision_damage_multiplier: f32,
        damage_multiplier: f32,
        general: &mut CGeneral,
        damage_nodes: Option<&crate::modelinfo::VehicleModelInfo::VehicleDamageNodes>,
    ) -> bool {
        super::AutomobileDamage::apply_collision_piece_damage(&self.vehicle.core,
            &mut self.vehicle.vitality, &mut self.vehicle.appearance, &self.vehicle.entry,
            &mut self.state, impulse, piece, collision_damage_multiplier, damage_multiplier,
            general, damage_nodes)
    }
}

impl CAutomobile {
    pub fn new(
        id: i32,
        created_by: u8,
        time_ms: u32,
        general: &mut CGeneral,
        handling: Option<CAutomobileHandlingSnapshot>,
        colours: Option<(u8, u8)>,
        all_taxi_lights: bool,
    ) -> Self {
        let mut vehicle = CVehicle::new(created_by, time_ms, general);
        vehicle.vehicle_type = eVehicleType::VEHICLE_TYPE_CAR;
        vehicle
            .physical
            .entity
            .set_model_index_no_create(id as usize);

        let mut damage = CDamageManager::new();
        let handling_flags = handling.map(|handling| handling.flags).unwrap_or_default();
        let handling_monetary_value = handling
            .map(|handling| handling.monetary_value)
            .unwrap_or_default();

        vehicle.flags.is_van = handling_flags & HANDLING_IS_VAN != 0;
        vehicle.flags.is_big = handling_flags & HANDLING_IS_BIG != 0;
        vehicle.flags.is_bus = handling_flags & HANDLING_IS_BUS != 0;
        vehicle.flags.low_vehicle = handling_flags & HANDLING_IS_LOW != 0;

        let doors = default_doors(vehicle.flags.is_bus, vehicle.flags.is_van, handling_flags);
        if handling_flags & HANDLING_NO_DOORS != 0 {
            damage.set_door_status(2, eDoorStatus::DOOR_STATUS_MISSING as u32);
            damage.set_door_status(3, eDoorStatus::DOOR_STATUS_MISSING as u32);
            damage.set_door_status(4, eDoorStatus::DOOR_STATUS_MISSING as u32);
            damage.set_door_status(5, eDoorStatus::DOOR_STATUS_MISSING as u32);
        }

        if let Some((colour1, colour2)) = colours {
            vehicle.appearance.current_colour1 = colour1;
            vehicle.appearance.current_colour2 = colour2;
        }

        if let Some(handling) = handling {
            vehicle.handling_id = Some(handling.handling_id);
            vehicle.physical.mass = handling.mass;
            vehicle.physical.turn_mass = handling.turn_mass;
            vehicle.physical.centre_of_mass = handling.centre_of_mass;
            vehicle.physical.air_resistance =
                handling.dimension.x * handling.dimension.z / handling.mass;
            vehicle.physical.buoyancy = handling.buoyancy;
        }
        vehicle.physical.elasticity = 0.05;

        let mut random_values = [0.0; 6];
        for value in &mut random_values {
            *value = general.get_random_number_in_range_f32(-0.15, 0.15);
        }

        vehicle.set_status(eEntityStatus::STATUS_SIMPLE);
        vehicle.physical.entity.flags.use_collision_records = true;

        Self {
            vehicle,
            audio_motion: Owned(AutomobileAudioMotion::default()),
            state: Owned(AutomobileState {
                damage,
                doors,
                wheel_col_points: [None; NUM_CAR_WHEELS],
                suspension_spring_ratio: [1.0; NUM_CAR_WHEELS],
                suspension_spring_ratio_prev: [1.0; NUM_CAR_WHEELS],
                wheel_timer: [0.0; NUM_CAR_WHEELS],
                auto_unused1: 20.0,
                wheel_skidmark_muddy: [false; NUM_CAR_WHEELS],
                wheel_skidmark_bloody: [false; NUM_CAR_WHEELS],
                wheel_contact_points: [CVector::ZERO; NUM_CAR_WHEELS],
                wheel_contact_surfaces: [0; NUM_CAR_WHEELS],
                wheel_rotation: [0.0; NUM_CAR_WHEELS],
                wheel_position: [0.0; NUM_CAR_WHEELS],
                wheel_speed: [0.0; NUM_CAR_WHEELS],
                auto_unused2: 0,
                bomb_type: eBombType::CARBOMB_NONE,
                taxi_light: all_taxi_lights,
                driver_last_frame: false,
                fixed_colour: false,
                big_wheels: false,
                water_tight: false,
                not_damaged_upside_down: false,
                more_resistant_to_damage: false,
                handling_no_doors: handling_flags & HANDLING_NO_DOORS != 0,
                handling_monetary_value,
                bomb_rigger: None,
                auto_unk1: 0,
                hydraulic_state: 0,
                bus_door_timer_end: 0,
                bus_door_timer_start: 0,
                suspension_spring_length: [0.0; NUM_CAR_WHEELS],
                suspension_line_length: [0.0; NUM_CAR_WHEELS],
                suspension_line_start: [CVector::ZERO; NUM_CAR_WHEELS],
                suspension_line_end: [CVector::ZERO; NUM_CAR_WHEELS],
                suspension_initialized: false,
                suspension_source: None,
                height_above_road: 0.0,
                traction: 1.0,
                random_values,
                fire_blow_up_timer: 0.0,
                ground_physical: [None; NUM_CAR_WHEELS],
                ground_offset: [CVector::ZERO; NUM_CAR_WHEELS],
                set_on_fire_entity: None,
                weapon_door_timer_left: 0.0,
                weapon_door_timer_right: 0.0,
                car_gun_lr: 0.0,
                car_gun_ud: 0.0,
                propeller_rotation: 0.0,
                wheels_on_ground: 0,
                drive_wheels_on_ground: 0,
                drive_wheels_on_ground_prev: 0,
                wheel_state: [tWheelState::WHEEL_STATE_NORMAL; NUM_CAR_WHEELS],
                observed_door_status: damage.door_status,
                observed_panel_status: [0; 7],
                observed_wheel_status: damage.wheel_status,
            }),
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CAutomobile::Fix`. Renderer component visibility is derived from
    /// these damage arrays, so resetting them also restores intact meshes.
    pub fn fix(&mut self) {
        self.state.damage.reset_damage_status();
        if self.state.handling_no_doors {
            for door in 2..NUM_CAR_DOORS {
                self.state
                    .damage
                    .set_door_status(door as u8, eDoorStatus::DOOR_STATUS_MISSING as u32);
            }
        }
        self.vehicle.appearance.is_damaged = false;
        self.state.fire_blow_up_timer = 0.0;
        let state = &mut *self.state;
        for (index, door) in state.doors.iter_mut().enumerate() {
            door.state = CDoor::DOORST_CLOSED;
            door.angle = door.angle_when_closed();
            door.previous_angle = door.angle;
            door.angular_velocity = 0.0;
            door.speed = CVector::ZERO;
            state.observed_door_status[index] = state.damage.get_door_status(index as u8) as u8;
        }
        state.observed_wheel_status = state.damage.wheel_status;
        for panel in 0..state.observed_panel_status.len() {
            state.observed_panel_status[panel] = state.damage.get_panel_status(panel as u8) as u8;
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// State half of re3 `CAutomobile::OpenDoor`. Rendering consumes the same
    /// CDoor angle through the live vehicle remap; culling/audio are handled by
    /// their dedicated renderer/audio paths.
    pub fn open_door(&mut self, door: usize, open_ratio: f32) -> bool {
        let Some(door_state) = self.state.doors.get_mut(door) else {
            return false;
        };
        let was_closed = door_state.is_closed();
        door_state.open(open_ratio);
        if was_closed && door_state.angle != door_state.angle_when_closed() {
            self.state
                .damage
                .set_door_status(door as u8, eDoorStatus::DOOR_STATUS_SWINGING as u32);
        } else if !was_closed && open_ratio == 0.0 {
            // `OpenDoor`: a closed swinging door becomes intact again.
            if self.state.damage.get_door_status(door as u8)
                == eDoorStatus::DOOR_STATUS_SWINGING as u32
            {
                self.state
                    .damage
                    .set_door_status(door as u8, eDoorStatus::DOOR_STATUS_OK as u32);
            }
        }
        true
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// re3 `CAutomobile::ProcessOpenDoor`, expressed by `eDoors` index rather
    /// than an RW frame pointer. Ped enter/exit animation time feeds this API.
    /// Return every component that transitioned to MISSING since the previous
    /// process step. The caller owns object-pool allocation, matching re3
    /// where `SpawnFlyingComponent` belongs to CAutomobile/world ownership.
    pub fn take_missing_component_transitions(&mut self) -> Vec<CarDebrisComponent> {
        let mut transitions = Vec::new();
        for door in 0..NUM_CAR_DOORS {
            let status = self.state.damage.get_door_status(door as u8) as u8;
            if self.state.observed_door_status[door] != eDoorStatus::DOOR_STATUS_MISSING as u8
                && status == eDoorStatus::DOOR_STATUS_MISSING as u8
            {
                transitions.push(match door {
                    0 => CarDebrisComponent::Bonnet,
                    1 => CarDebrisComponent::Boot,
                    2 => CarDebrisComponent::DoorLeftFront,
                    3 => CarDebrisComponent::DoorRightFront,
                    4 => CarDebrisComponent::DoorLeftRear,
                    _ => CarDebrisComponent::DoorRightRear,
                });
            }
            self.state.observed_door_status[door] = status;
        }
        for wheel in 0..NUM_CAR_WHEELS {
            let status = self.state.damage.get_wheel_status(wheel as u8) as u8;
            if self.state.observed_wheel_status[wheel] != eWheelStatus::WHEEL_STATUS_MISSING as u8
                && status == eWheelStatus::WHEEL_STATUS_MISSING as u8
            {
                transitions.push(match wheel {
                    CARWHEEL_FRONT_LEFT => CarDebrisComponent::WheelLeftFront,
                    CARWHEEL_FRONT_RIGHT => CarDebrisComponent::WheelRightFront,
                    CARWHEEL_REAR_LEFT => CarDebrisComponent::WheelLeftRear,
                    _ => CarDebrisComponent::WheelRightRear,
                });
            }
            self.state.observed_wheel_status[wheel] = status;
        }
        for panel in 0..7 {
            let status = self.state.damage.get_panel_status(panel as u8) as u8;
            if self.state.observed_panel_status[panel] != eDoorStatus::DOOR_STATUS_MISSING as u8
                && status == eDoorStatus::DOOR_STATUS_MISSING as u8
            {
                transitions.push(match panel {
                    0 => CarDebrisComponent::WingLeftFront,
                    1 => CarDebrisComponent::WingRightFront,
                    2 => CarDebrisComponent::WingLeftRear,
                    3 => CarDebrisComponent::WingRightRear,
                    4 => CarDebrisComponent::Windscreen,
                    5 => CarDebrisComponent::BumperFront,
                    _ => CarDebrisComponent::BumperRear,
                });
            }
            self.state.observed_panel_status[panel] = status;
        }
        transitions
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    pub fn process_open_door(&mut self, door: usize, animation_id: i32, time: f32) -> bool {
        use crate::animation::AnimationId::*;
        if door >= NUM_CAR_DOORS
            || self.state.damage.get_door_status(door as u8)
                == eDoorStatus::DOOR_STATUS_MISSING as u32
        {
            return false;
        }
        let ratio = match animation_id {
            ANIM_STD_QUICKJACK
            | ANIM_STD_CAR_OPEN_DOOR_LHS
            | ANIM_STD_CAR_OPEN_DOOR_RHS
            | ANIM_STD_COACH_OPEN_LHS
            | ANIM_STD_COACH_OPEN_RHS => door_open_ratio(time, 0.66, 0.8),
            ANIM_STD_CAR_CLOSE_DOOR_LHS
            | ANIM_STD_CAR_CLOSE_DOOR_LO_LHS
            | ANIM_STD_CAR_CLOSE_DOOR_RHS
            | ANIM_STD_CAR_CLOSE_DOOR_LO_RHS => door_close_ratio(time, 0.2, 0.63),
            ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS | ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS => {
                door_open_close_ratio(time, 0.1, 0.6, 0.95)
            }
            ANIM_STD_GETOUT_LHS
            | ANIM_STD_GETOUT_LO_LHS
            | ANIM_STD_GETOUT_RHS
            | ANIM_STD_GETOUT_LO_RHS => door_open_ratio(time, 0.06, 0.43),
            ANIM_STD_CAR_CLOSE_LHS | ANIM_STD_CAR_CLOSE_RHS => door_close_ratio(time, 0.1, 0.23),
            ANIM_STD_CAR_PULL_OUT_PED_RHS | ANIM_STD_CAR_PULL_OUT_PED_LO_RHS => Some(1.0),
            ANIM_STD_COACH_GET_OUT_LHS => door_open_ratio(time, 0.0, 0.3),
            ANIM_STD_VAN_OPEN_DOOR_REAR_LHS | ANIM_STD_VAN_OPEN_DOOR_REAR_RHS => {
                door_open_ratio(time, 0.37, 0.55)
            }
            ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS | ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS => {
                door_close_ratio(time, 0.5, 0.8)
            }
            ANIM_STD_VAN_GET_OUT_REAR_LHS | ANIM_STD_VAN_GET_OUT_REAR_RHS => {
                door_open_ratio(time, 0.5, 0.6)
            }
            // re3 ANIM_STD uses its `time` operand as the open ratio.
            x if x == ANIM_STD_NUM as i32 => Some(time),
            _ => None,
        };
        let Some(ratio) = ratio else {
            return false;
        };
        let current = self.state.doors[door].angle_open_ratio();
        // C++ ProcessDoorCloseAnimation/ProcessDoorOpenCloseAnimation only
        // writes while the authored hinge is moving toward the requested
        // ratio. CAR_CLOSE_LHS/RHS and the rolling-close animations are close
        // phases, so their authored ratio only moves the hinge toward shut.
        let closing = matches!(
            animation_id,
            ANIM_STD_CAR_CLOSE_DOOR_LHS
                | ANIM_STD_CAR_CLOSE_DOOR_LO_LHS
                | ANIM_STD_CAR_CLOSE_DOOR_RHS
                | ANIM_STD_CAR_CLOSE_DOOR_LO_RHS
                | ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS
                | ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS
                | ANIM_STD_CAR_CLOSE_LHS
                | ANIM_STD_CAR_CLOSE_RHS
                | ANIM_STD_VAN_CLOSE_DOOR_REAR_LHS
                | ANIM_STD_VAN_CLOSE_DOOR_REAR_RHS
        );
        let should_apply = if closing {
            current > ratio || ratio == 0.0
        } else {
            current < ratio || ratio == 0.0
        };
        if should_apply {
            self.open_door(door, ratio);
        }
        true
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    pub fn process_door(&mut self, door: usize) -> bool {
        let Some(door_state) = self.state.doors.get_mut(door) else {
            return false;
        };
        door_state.process(&self.vehicle);
        true
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    pub fn process_wheels(
        &mut self,
        input: CAutomobileWheelProcessInput,
        general: &mut CGeneral,
    ) -> CAutomobileWheelProcessResult {
        let mut wheel_state = input.wheel_state_scratch;
        let mut already_skidding = input.already_skidding;
        let mut wheel_results = [None; NUM_CAR_WHEELS];

        if self.state.wheel_timer[CARWHEEL_FRONT_LEFT] > 0.0
            || self.state.wheel_timer[CARWHEEL_FRONT_RIGHT] > 0.0
        {
            let (sin_steer, cos_steer) = self.vehicle.drive.steer_angle.sin_cos();
            let wheel_fwd = self
                .vehicle
                .core
                .physical
                .entity
                .transform_vector(CVector::new(-sin_steer, cos_steer, 0.0));
            let wheel_right = self
                .vehicle
                .core
                .physical
                .entity
                .transform_vector(CVector::new(cos_steer, sin_steer, 0.0));

            self.process_ground_wheel(
                CARWHEEL_FRONT_LEFT,
                wheel_fwd,
                wheel_right,
                if input.front_wheel_drive {
                    input.acceleration
                } else {
                    0.0
                },
                input.brake * input.brake_bias_front,
                input.adhesive_limits[CARWHEEL_FRONT_LEFT]
                    * input.traction
                    * input.traction_bias_front,
                input,
                &mut wheel_state,
                &mut already_skidding,
                &mut wheel_results,
                general,
            );
            self.process_ground_wheel(
                CARWHEEL_FRONT_RIGHT,
                wheel_fwd,
                wheel_right,
                if input.front_wheel_drive {
                    input.acceleration
                } else {
                    0.0
                },
                input.brake * input.brake_bias_front,
                input.adhesive_limits[CARWHEEL_FRONT_RIGHT]
                    * input.traction
                    * input.traction_bias_front,
                input,
                &mut wheel_state,
                &mut already_skidding,
                &mut wheel_results,
                general,
            );
        }

        self.process_off_ground_wheel(
            CARWHEEL_FRONT_LEFT,
            input.front_wheel_drive,
            input.acceleration,
        );
        self.process_off_ground_wheel(
            CARWHEEL_FRONT_RIGHT,
            input.front_wheel_drive,
            input.acceleration,
        );

        if self.state.wheel_timer[CARWHEEL_REAR_LEFT] > 0.0
            || self.state.wheel_timer[CARWHEEL_REAR_RIGHT] > 0.0
        {
            let wheel_fwd = self.vehicle.core.physical.entity.get_forward();
            let wheel_right = self.vehicle.core.physical.entity.get_right();
            let brake = if self.vehicle.drive.is_handbrake_on {
                // re3 FIX_BUGS: rear handbrake impulse is frame-rate
                // independent (`GetTimeStepFix = timeStep / (50/30)`).
                20000.0 * input.time_step.max(0.0) / (50.0 / 30.0)
            } else {
                input.brake
            };

            self.process_ground_wheel(
                CARWHEEL_REAR_LEFT,
                wheel_fwd,
                wheel_right,
                if input.rear_wheel_drive {
                    input.acceleration
                } else {
                    0.0
                },
                brake * input.brake_bias_rear,
                input.adhesive_limits[CARWHEEL_REAR_LEFT]
                    * input.traction
                    * input.traction_bias_rear,
                input,
                &mut wheel_state,
                &mut already_skidding,
                &mut wheel_results,
                general,
            );
            self.process_ground_wheel(
                CARWHEEL_REAR_RIGHT,
                wheel_fwd,
                wheel_right,
                if input.rear_wheel_drive {
                    input.acceleration
                } else {
                    0.0
                },
                brake * input.brake_bias_rear,
                input.adhesive_limits[CARWHEEL_REAR_RIGHT]
                    * input.traction
                    * input.traction_bias_rear,
                input,
                &mut wheel_state,
                &mut already_skidding,
                &mut wheel_results,
                general,
            );
        }

        self.process_off_ground_wheel(
            CARWHEEL_REAR_LEFT,
            input.rear_wheel_drive,
            input.acceleration,
        );
        self.process_off_ground_wheel(
            CARWHEEL_REAR_RIGHT,
            input.rear_wheel_drive,
            input.acceleration,
        );

        self.state.wheel_state = wheel_state;

        CAutomobileWheelProcessResult {
            wheel_state_scratch: wheel_state,
            already_skidding,
            wheel_results,
        }
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn process_ground_wheel(
        &mut self,
        wheel: usize,
        wheel_fwd: CVector,
        wheel_right: CVector,
        thrust: f32,
        brake: f32,
        mut adhesion: f32,
        input: CAutomobileWheelProcessInput,
        wheel_state: &mut [tWheelState; NUM_CAR_WHEELS],
        already_skidding: &mut bool,
        wheel_results: &mut [Option<CVehicleProcessWheelResult>; NUM_CAR_WHEELS],
        general: &mut CGeneral,
    ) {
        if self.state.wheel_timer[wheel] <= 0.0 {
            return;
        }

        wheel_state[wheel] = self.state.wheel_state[wheel];
        let wheel_status = self.wheel_status(wheel);
        if wheel_status == eWheelStatus::WHEEL_STATUS_MISSING {
            return;
        }
        if wheel_status == eWheelStatus::WHEEL_STATUS_BURST {
            adhesion *= self.state.damage.wheel_damage_effect;
        }

        let result = self.vehicle.process_wheel(
            CVehicleProcessWheelInput {
                wheel_fwd,
                wheel_right,
                wheel_contact_speed: input.contact_speeds[wheel],
                wheel_contact_point: input.contact_points[wheel],
                wheels_on_ground: i32::from(self.state.wheels_on_ground),
                thrust,
                brake,
                adhesion,
                wheel_id: wheel as i8,
                wheel_speed: self.state.wheel_speed[wheel],
                wheel_state: wheel_state[wheel],
                wheel_status,
                already_skidding: *already_skidding,
                time_step: input.time_step,
                handling: input.handling,
            },
            general,
        );
        // `CVehicle::process_wheel`, like the C++ method, already applies its
        // move and turn impulses. Applying the returned diagnostic force a
        // second time doubled every driven wheel's acceleration and was most
        // obvious on RHINO and other heavy handling records.
        wheel_state[wheel] = result.wheel_state;
        self.state.wheel_speed[wheel] = result.wheel_speed;
        *already_skidding = result.already_skidding;
        wheel_results[wheel] = Some(result);
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn process_off_ground_wheel(&mut self, wheel: usize, is_drive_wheel: bool, acceleration: f32) {
        if self.state.wheel_timer[wheel] > 0.0 {
            return;
        }

        if is_drive_wheel && acceleration != 0.0 {
            if acceleration > 0.0 {
                if self.state.wheel_speed[wheel] < 2.0 {
                    self.state.wheel_speed[wheel] -= 0.2;
                }
            } else if self.state.wheel_speed[wheel] > -2.0 {
                self.state.wheel_speed[wheel] += 0.1;
            }
        } else {
            self.state.wheel_speed[wheel] *= 0.95;
        }
        self.state.wheel_rotation[wheel] += self.state.wheel_speed[wheel];
    }
}

impl<
    C: std::ops::DerefMut<Target = VehicleCore>,
    V: std::ops::DerefMut<Target = VehicleVitality>,
    S: std::ops::DerefMut<Target = AutomobileState>,
    AutoAudio: std::ops::DerefMut<Target = AutomobileAudioMotion>,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    /// Render-phase wheel rotation from re3 `CAutomobile::Render`.
    ///
    /// Grounded wheels are deliberately advanced here rather than in
    /// `ProcessWheel`: the C++ renderer samples the final physical speed at
    /// each contact point, applies the front steering direction and stores the
    /// resulting angle back on the automobile.
    pub fn update_wheel_rotation_for_render(
        &mut self,
        local_contact_points: [CVector; NUM_CAR_WHEELS],
        wheel_scale: f32,
        time_step: f32,
    ) {
        let steer = self.vehicle.drive.steer_angle;
        let front_wheel_fwd = self
            .vehicle
            .core
            .physical
            .entity
            .transform_vector(CVector::new(-steer.sin(), steer.cos(), 0.0));
        let rear_wheel_fwd = self.vehicle.core.physical.entity.get_forward();
        let radius = (0.5 * wheel_scale).max(f32::EPSILON);
        for wheel in 0..NUM_CAR_WHEELS {
            if self.state.wheel_timer[wheel] <= 0.0 {
                continue;
            }
            let contact_point = self
                .vehicle
                .core
                .physical
                .entity
                .transform_vector(local_contact_points[wheel]);
            let contact_speed = self.vehicle.core.physical.get_speed_at_point(contact_point);
            let wheel_fwd = if wheel == CARWHEEL_FRONT_LEFT || wheel == CARWHEEL_FRONT_RIGHT {
                front_wheel_fwd
            } else {
                rear_wheel_fwd
            };
            self.state.wheel_speed[wheel] = CVehicle::process_wheel_rotation(
                self.state.wheel_state[wheel],
                wheel_fwd,
                contact_speed,
                radius,
                time_step,
            );
            self.state.wheel_rotation[wheel] += self.state.wheel_speed[wheel];
        }
    }
}

impl<
    C: std::ops::Deref<Target = VehicleCore>,
    V: std::ops::Deref<Target = VehicleVitality>,
    S: std::ops::Deref<Target = AutomobileState>,
    AutoAudio: std::ops::Deref<Target = AutomobileAudioMotion>,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>> AutomobileRuntimeView<C, V, S, Drive, Appearance, Entry, Audio, AutoAudio>
{
    fn wheel_status(&self, wheel: usize) -> eWheelStatus {
        match self.state.damage.get_wheel_status(wheel as u8) {
            1 => eWheelStatus::WHEEL_STATUS_BURST,
            2 => eWheelStatus::WHEEL_STATUS_MISSING,
            _ => eWheelStatus::WHEEL_STATUS_OK,
        }
    }
}

/// `ProcessDoorOpenAnimation`: no action before start, linear open during the
/// interval, then fully open after it.
fn door_open_ratio(time: f32, start: f32, end: f32) -> Option<f32> {
    if time > start && time < end {
        Some((time - start) / (end - start))
    } else if time > end {
        Some(1.0)
    } else {
        None
    }
}

/// `ProcessDoorCloseAnimation`: no action before start, linear close during
/// the interval, then fully closed after it.
fn door_close_ratio(time: f32, start: f32, end: f32) -> Option<f32> {
    if time > start && time < end {
        Some(1.0 - (time - start) / (end - start))
    } else if time > end {
        Some(0.0)
    } else {
        None
    }
}

fn door_open_close_ratio(time: f32, start: f32, mid: f32, end: f32) -> Option<f32> {
    if time > start && time < mid {
        Some((time - start) / (mid - start))
    } else if time > mid && time < end {
        Some(1.0 - (time - mid) / (end - mid))
    } else if time > end {
        Some(0.0)
    } else {
        None
    }
}

fn default_doors(is_bus: bool, is_van: bool, flags: u32) -> [CDoor; NUM_CAR_DOORS] {
    const PI: f32 = std::f32::consts::PI;
    const HALFPI: f32 = std::f32::consts::FRAC_PI_2;

    let mut doors = [CDoor::new(0.0, 0.0, 0, 0); NUM_CAR_DOORS];
    if is_bus {
        doors[2] = CDoor::new(-HALFPI, 0.0, 0, 2);
        doors[3] = CDoor::new(0.0, HALFPI, 1, 2);
    } else {
        doors[2] = CDoor::new(-PI * 0.4, 0.0, 0, 2);
        doors[3] = CDoor::new(0.0, PI * 0.4, 1, 2);
    }

    if is_van {
        doors[4] = CDoor::new(-HALFPI, 0.0, 1, 2);
        doors[5] = CDoor::new(0.0, HALFPI, 0, 2);
    } else {
        doors[4] = CDoor::new(-PI * 0.4, 0.0, 0, 2);
        doors[5] = CDoor::new(0.0, PI * 0.4, 1, 2);
    }

    doors[0] = if flags & HANDLING_REV_BONNET != 0 {
        CDoor::new(-PI * 0.3, 0.0, 1, 0)
    } else {
        CDoor::new(0.0, PI * 0.3, 1, 0)
    };

    doors[1] = if flags & HANDLING_HANGING_BOOT != 0 {
        CDoor::new(-PI * 0.4, 0.0, 0, 0)
    } else if flags & HANDLING_TAILGATE_BOOT != 0 {
        CDoor::new(0.0, HALFPI, 1, 0)
    } else {
        CDoor::new(-PI * 0.3, 0.0, 1, 0)
    };

    doors
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustgta3_formats::formats::ClumpRead::RwFrameDescriptor;

    #[test]
    fn damage_node_followup_preserves_optional_provenance_dodo_and_health() {
        use crate::modelinfo::{ModelIndices::{MI_KURUMA, MI_DODO},
            VehicleModelInfo::{AuthoredVehiclePosition, VehicleDamageNodes}};
        use crate::vehicles::DamageManager::{ePanels, eDoorStatus};
        for (model, node, supplied, expected) in [
            (MI_KURUMA, AuthoredVehiclePosition::Position(CVector::ZERO), true, 1),
            (MI_KURUMA, AuthoredVehiclePosition::Absent, true, 0),
            (MI_KURUMA, AuthoredVehiclePosition::Invalid, true, 0),
            (MI_KURUMA, AuthoredVehiclePosition::Position(CVector::ZERO), false, 0),
            (MI_DODO, AuthoredVehiclePosition::Position(CVector::ZERO), true, 0),
        ] {
            let mut random = CGeneral::with_seed(7);
            let mut car = CAutomobile::new(model as i32, 2, 0, &mut random, None, None, false);
            let nodes = VehicleDamageNodes { bonnet: node, boot: node };
            car.damage.set_panel_status(ePanels::VEHBUMPER_FRONT as u8, 2);
            let health = car.vehicle.vitality.health;
            car.apply_collision_piece_damage(1000.0, eCarPiece::CAR_PIECE_BUMP_FRONT,
                1.0, &mut random, supplied.then_some(&nodes));
            assert_eq!(car.damage.get_door_status(0), expected);
            assert_eq!(car.damage.get_panel_status(ePanels::VEHBUMPER_FRONT as u8), 3);
            assert!(car.vehicle.vitality.health < health, "node failure cannot suppress common health damage");
            if expected == 1 { assert!(car.damage.get_engine_status() > 0); }
            if model == MI_KURUMA && supplied && matches!(node, AuthoredVehiclePosition::Position(_)) {
                car.damage.set_panel_status(ePanels::VEHBUMPER_REAR as u8, 2);
                car.apply_collision_piece_damage(1000.0, eCarPiece::CAR_PIECE_BUMP_REAR,
                    1.0, &mut random, Some(&nodes));
                assert_eq!(car.damage.get_door_status(1), eDoorStatus::DOOR_STATUS_SMASHED as u32);
            }
        }
    }

    #[test]
    fn shallow_water_contact_emits_re3_splash_and_white_spray() {
        use crate::{
            core::SurfaceTable::eSurfaceType::SURFACE_WATER,
            renderer::ParticleType::{PARTICLE_PED_SPLASH, PARTICLE_RUBBER_SMOKE},
        };

        let mut general = CGeneral::with_seed(7);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.suspension_spring_ratio[CARWHEEL_FRONT_LEFT] = 0.5;
        car.wheel_contact_surfaces[CARWHEEL_FRONT_LEFT] = SURFACE_WATER as u8;
        car.wheel_contact_points[CARWHEEL_FRONT_LEFT] = CVector::new(1.0, 2.0, 0.0);
        car.vehicle.core.physical.move_speed = CVector::new(0.1, 0.0, 0.0);
        let mut particles = CParticle::default();

        process_automobile_buoyancy(&mut car, CVector::new(-1.0, -2.0, -0.5),
            CVector::new(1.0, 2.0, 1.0), &crate::renderer::WaterLevel::CWaterLevel::default(),
            1000, 0.0, 1.0, &mut CParticleObjectRuntime::initialise(), &mut general, None,
            AutomobileBuoyancyEffects {
                particles: Some(&mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO)),
                ..Default::default()
            });

        assert_eq!(particles.add_intents.len(), 2);
        assert_eq!(particles.add_intents[0].particle_type, PARTICLE_PED_SPLASH);
        assert_eq!(
            particles.add_intents[1].particle_type,
            PARTICLE_RUBBER_SMOKE
        );
        assert_eq!(particles.add_intents[0].colour.red, 155);
        assert_eq!(particles.add_intents[0].colour.blue, 185);
        assert_eq!(particles.add_intents[0].colour.alpha, 196);
        assert_eq!(particles.add_intents[1].colour.red, 255);
        assert_eq!(particles.add_intents[1].colour.green, 255);
        assert_eq!(particles.add_intents[1].colour.blue, 255);
    }

    #[test]
    fn skidmark_registration_requires_grounded_wheel_timer_like_re3() {
        let mut general = CGeneral::with_seed(11);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.wheel_state[CARWHEEL_FRONT_LEFT] = tWheelState::WHEEL_STATE_SPINNING;
        car.wheel_contact_points[CARWHEEL_FRONT_LEFT] = CVector::new(3.0, 4.0, 0.0);
        let mut particles = CParticle::default();
        let mut skidmarks = CSkidmarks::init();

        car.wheel_timer[CARWHEEL_FRONT_LEFT] = 0.0;
        process_automobile_wheel_effects(
            &mut car,
            &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
            Some(&mut skidmarks),
            42,
            0.0,
            0,
            1_000,
            &mut general,
            false,
        );
        assert!(skidmarks.skidmarks.iter().all(|mark| mark.state == 0));

        car.wheel_timer[CARWHEEL_FRONT_LEFT] = 1.0;
        process_automobile_wheel_effects(
            &mut car,
            &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
            Some(&mut skidmarks),
            42,
            0.0,
            0,
            1_016,
            &mut general,
            false,
        );
        assert!(skidmarks.skidmarks.iter().any(|mark| mark.state != 0));
    }

    #[test]
    fn wheel_registration_needs_only_index_core_and_wheel_state_owners() {
        let mut random = CGeneral::with_seed(7);
        let car = CAutomobile::new(90, 2, 0, &mut random, None, None, false);
        let (mut core, _vitality, _drive, _appearance, _entry, _audio, mut state, _motion) = car.into_parts();
        core.physical.entity.status = crate::entities::Entity::eEntityStatus::STATUS_PHYSICS;
        core.physical.move_speed = CVector::new(0.1, 0.0, 0.0);
        state.suspension_spring_ratio[CARWHEEL_FRONT_LEFT] = 0.5;
        state.wheel_state[CARWHEEL_FRONT_LEFT] = tWheelState::WHEEL_STATE_FIXED;
        state.wheel_contact_points[CARWHEEL_FRONT_LEFT] = CVector::ZERO;
        let mut app = App::new();
        let entity = app.world_mut().spawn((crate::control::ScriptEntities::ScriptVehicleIdentity {
            handle: 42, kind: crate::control::ScriptEntities::ScriptVehicleKind::Automobile,
        }, core, state)).id();
        let mut index = crate::control::ScriptEntities::ScriptEntityIndex::default();
        index.vehicles.insert(42, entity);
        app.insert_resource(index).insert_resource(random).insert_resource(CParticle::default())
            .insert_resource(CSkidmarks::init()).insert_resource(CTimer::initialise())
            .insert_resource(CWeather::default()).insert_resource(crate::core::Camera::CCamera::default())
            .insert_resource(crate::renderer::ParticleMgr::cParticleSystemMgr::default())
            .add_systems(Update, register_automobile_skidmarks);
        app.update();
        assert!(app.world().get::<crate::vehicles::Vehicle::VehicleVitality>(entity).is_none());
        assert_eq!(app.world().resource::<CParticle>().add_intents.len(), 1);
        app.world_mut().resource_mut::<crate::control::ScriptEntities::ScriptEntityIndex>().vehicles.remove(&42);
        app.world_mut().resource_mut::<CParticle>().add_intents.clear();
        app.update();
        assert!(app.world().resource::<CParticle>().add_intents.is_empty(), "retired index identity cannot emit");
    }

    #[test]
    fn engine_and_fire_origins_preserve_native_model_and_view_distinctions() {
        let mut random = CGeneral::with_seed(7);
        let mut car = CAutomobile::new(crate::modelinfo::ModelIndices::MI_BFINJECT as i32,
            2, 0, &mut random, None, None, false);
        let headlights = CVector::new(5.0, 6.0, 7.0);
        assert!((car.damage_particle_position(headlights, AutomobileDamageOrigin::Engine)
            - CVector::new(0.3, -1.5, 0.05)).magnitude() < 0.0001);
        car.vehicle.vitality.health = 100.0;
        car.vehicle.core.physical.entity.status = eEntityStatus::STATUS_PLAYER;
        let fire = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: false });
        let forward = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: true });
        assert!((forward - fire - CVector::new(0.0, 1.2, -0.8)).magnitude() < 0.0001);
        car.vehicle.core.physical.entity.model_index = crate::modelinfo::ModelIndices::MI_FIRETRUCK as i16;
        let fire = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: false });
        let forward = car.damage_particle_position(headlights, AutomobileDamageOrigin::Fire { looking_forward_first_person: true });
        assert!((forward - fire - CVector::new(0.0, 3.0, -0.2)).magnitude() < 0.0001);
    }

    #[test]
    fn rw_frame_basis_uses_the_same_right_up_at_mapping_as_cpp_cmatrix_update() {
        let summary = RwClumpSummary {
            num_atomics: 0,
            num_lights: 0,
            num_cameras: 0,
            num_frames: 1,
            num_geometries: 0,
            first_phase_geometries: 0,
            atomic_chunks: 0,
            root_frame_name: Some("wheel_lf_dummy".to_owned()),
            frame_names: vec![Some("wheel_lf_dummy".to_owned())],
            frames: vec![RwFrameDescriptor {
                right: [1.0, 0.0, 0.0],
                up: [0.0, 0.0, 1.0],
                at: [0.0, -1.0, 0.0],
                position: [2.0, 3.0, 4.0],
                parent_index: -1,
                matrix_flags: 0,
                name: Some("wheel_lf_dummy".to_owned()),
                hanim_id: None,
            }],
            hanim_nodes: Vec::new(),
            hanim_hierarchy_flags: 0,
            hanim_root_frame_index: None,
            hanim_node_ids: Vec::new(),
            geometries: Vec::new(),
            atomics: Vec::new(),
            bevy_meshes: Vec::new(),
        };

        let matrix = clump_frame_matrix(&summary, 0).expect("single root frame");
        assert_eq!(matrix.right, CVector::new(1.0, 0.0, 0.0));
        assert_eq!(matrix.forward, CVector::new(0.0, 0.0, 1.0));
        assert_eq!(matrix.up, CVector::new(0.0, -1.0, 0.0));
        assert_eq!(matrix.position, CVector::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn fix_resets_damage_visibility_and_hinge_state() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.vehicle.appearance.is_damaged = true;
        car.fire_blow_up_timer = 123.0;
        car.damage
            .set_door_status(2, eDoorStatus::DOOR_STATUS_MISSING as u32);
        car.damage
            .set_wheel_status(0, eWheelStatus::WHEEL_STATUS_BURST as u32);
        car.doors[2].open(1.0);

        car.fix();

        assert!(!car.vehicle.appearance.is_damaged);
        assert_eq!(car.fire_blow_up_timer, 0.0);
        assert_eq!(
            car.damage.get_door_status(2),
            eDoorStatus::DOOR_STATUS_OK as u32
        );
        assert_eq!(
            car.damage.get_wheel_status(0),
            eWheelStatus::WHEEL_STATUS_OK as u32
        );
        assert!(car.doors[2].is_closed());
    }

    #[test]
    fn rolling_close_animation_reaches_closed_hinge() {
        let mut general = CGeneral::with_seed(2);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.open_door(2, 1.0);
        assert_eq!(
            car.damage.get_door_status(2),
            eDoorStatus::DOOR_STATUS_SWINGING as u32
        );

        car.process_open_door(
            2,
            crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS,
            0.35,
        );
        assert!(car.doors[2].angle_open_ratio() < 1.0);
        assert!(car.doors[2].angle_open_ratio() > 0.0);

        car.process_open_door(
            2,
            crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS,
            1.0,
        );
        assert!(car.doors[2].is_closed());
    }

    #[test]
    fn mouse_steering_holds_on_idle_and_returns_to_keyboard_control() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        let handling = tHandlingData { steering_lock: 30.0, ..Default::default() };
        car.process_control_inputs(
            CAutomobileControlInput {
                mouse_x: 40.0, mouse_steering: true, ..Default::default()
            }, &handling, 1.0, &mut general,
        );
        let mouse_angle = -0.0035 * 40.0 * 0.975;
        assert!((car.vehicle.drive.steer_input - mouse_angle).abs() < 1.0e-6);
        assert!(car.vehicle.drive.mouse_steering_active);

        car.process_control_inputs(
            CAutomobileControlInput { mouse_steering: true, ..Default::default() },
            &handling, 1.0, &mut general,
        );
        assert!((car.vehicle.drive.steer_input - mouse_angle).abs() < 1.0e-6);

        car.process_control_inputs(
            CAutomobileControlInput {
                steering_left_right: -128.0, mouse_steering: true, ..Default::default()
            }, &handling, 1.0, &mut general,
        );
        let keyboard_angle = mouse_angle + (1.0 - mouse_angle) * 0.2;
        assert!((car.vehicle.drive.steer_input - keyboard_angle).abs() < 1.0e-6);
        assert!(!car.vehicle.drive.mouse_steering_active);

        car.process_control_inputs(
            CAutomobileControlInput { mouse_steering: true, ..Default::default() },
            &handling, 1.0, &mut general,
        );
        assert!((car.vehicle.drive.steer_input - keyboard_angle * 0.8).abs() < 1.0e-6);
    }

    #[test]
    fn process_control_inputs_matches_re3_steering_and_brake_selection() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        let handling = tHandlingData {
            steering_lock: 30.0,
            suspension_force_level: 1.0,
            suspension_damping_level: 0.25,
            suspension_bias: 0.5,
            ..Default::default()
        };

        car.process_control_inputs(
            CAutomobileControlInput {
                steering_left_right: -128.0,
                accelerate: 255.0,
                ..Default::default()
            },
            &handling,
            1.0,
            &mut general,
        );
        assert!((car.vehicle.drive.steer_input - 0.2).abs() < 1.0e-6);
        assert!((car.vehicle.drive.steer_angle - 30.0_f32.to_radians() * 0.04).abs() < 1.0e-6);
        assert_eq!(car.vehicle.drive.gas_pedal, 1.0);
        assert_eq!(car.vehicle.drive.brake_pedal, 0.0);

        car.vehicle.core.physical.move_speed = CVector::new(0.0, 0.2, 0.0);
        car.process_control_inputs(
            CAutomobileControlInput {
                brake: 255.0,
                handbrake: true,
                ..Default::default()
            },
            &handling,
            1.0,
            &mut general,
        );
        assert_eq!(car.vehicle.drive.gas_pedal, 0.0);
        assert_eq!(car.vehicle.drive.brake_pedal, 1.0);
        assert!(car.vehicle.drive.is_handbrake_on);

        car.vehicle.core.physical.move_speed = CVector::new(0.0, 0.4, 0.0);
        car.process_control_inputs(
            CAutomobileControlInput {
                brake: 255.0,
                controls_disabled: true,
                ..Default::default()
            },
            &handling,
            1.0,
            &mut general,
        );
        assert_eq!(car.vehicle.drive.gas_pedal, 0.0);
        assert_eq!(car.vehicle.drive.brake_pedal, 1.0);
        assert!(car.vehicle.drive.is_handbrake_on);
        assert!((car.vehicle.core.physical.move_speed.magnitude() - 0.28).abs() < 1.0e-6);
    }

    #[test]
    fn grounded_player_drive_step_moves_and_steers_the_real_automobile_state() {
        let mut general = CGeneral::with_seed(7);
        let mut car = CAutomobile::new(
            90,
            2,
            0,
            &mut general,
            Some(CAutomobileHandlingSnapshot {
                mass: 1_200.0,
                turn_mass: 2_400.0,
                centre_of_mass: CVector::new(0.0, 0.0, -0.2),
                dimension: CVector::new(1.8, 4.0, 1.4),
                buoyancy: 0.0,
                flags: 0,
                handling_id: 0,
                monetary_value: 0,
            }),
            None,
            false,
        );
        // re3 CAutomobile::ProcessControl only reads player pad input for a
        // STATUS_PLAYER vehicle; abandoned vehicles retain resolved pedals.
        car.vehicle.set_status(eEntityStatus::STATUS_PLAYER);
        let mut handling = tHandlingData {
            identifier: 0,
            mass: 1_200.0,
            inv_mass: 1.0 / 1_200.0,
            turn_mass: 2_400.0,
            dimension: CVector::new(1.8, 4.0, 1.4),
            centre_of_mass: CVector::new(0.0, 0.0, -0.2),
            traction_multiplier: 1.0,
            traction_loss: 0.8,
            traction_bias: 0.5,
            brake_deceleration: 0.004,
            brake_bias: 0.5,
            steering_lock: 30.0,
            ..Default::default()
        };
        handling.transmission.drive_type = 'F';
        handling.transmission.number_of_gears = 5;
        handling.transmission.engine_acceleration = 0.004;
        handling.transmission.max_velocity = 0.8;
        handling.transmission.max_reverse_velocity = -0.2;
        handling.transmission.init_gear_ratios();
        let contacts = [
            CVector::new(-0.7, 1.3, -0.5),
            CVector::new(-0.7, -1.3, -0.5),
            CVector::new(0.7, 1.3, -0.5),
            CVector::new(0.7, -1.3, -0.5),
        ];
        let initial = car.vehicle.core.physical.entity.get_position();
        for _ in 0..30 {
            car.process_player_drive_step(
                CAutomobileDriveStepInput {
                    control: CAutomobileControlInput {
                        steering_left_right: 128.0,
                        accelerate: 255.0,
                        ..Default::default()
                    },
                    local_contact_points: contacts,
                    contact_points: contacts,
                    spring_ratios: [0.875; NUM_CAR_WHEELS],
                    spring_directions: [CVector::new(0.0, 0.0, -1.0); NUM_CAR_WHEELS],
                    grounded: [true; NUM_CAR_WHEELS],
                    collision_entities: [None; NUM_CAR_WHEELS],
                    collision_time_ms: 0,
                    adhesive_limits: [1.0; NUM_CAR_WHEELS],
                    wheel_scale: 1.0,
                    time_step: 1.0,
                    time_step_seconds: 0.02,
                },
                &mut handling,
                0.9,
                &mut general,
            None,
            );
        }
        let position = car.vehicle.core.physical.entity.get_position();
        assert!(
            position.y > initial.y,
            "accelerate must move along GTA forward"
        );
        assert!(
            car.vehicle.drive.steer_angle < 0.0,
            "positive pad X steers right"
        );
        assert!(
            car.wheel_rotation
                .iter()
                .any(|rotation| rotation.abs() > 0.0),
            "grounded wheel render rotation must follow physical speed"
        );
        assert_eq!(car.wheels_on_ground, NUM_CAR_WHEELS as u8);
        assert!(
            car.vehicle.core.physical.entity.get_up().z > 0.98,
            "suspension support must keep a level car from rolling like a ball"
        );

        // CPhysical::ProcessControl clears last frame's collision records. re3
        // then repopulates them from suspension-line contacts before SCM asks
        // IS_CAR_IN_AIR_PROPER. A grounded car must therefore keep a road
        // collision record instead of looking airborne to the script.
        car.vehicle.core.physical.collision_records.push(EntityReference::World(99));
        car.process_player_drive_step(
            CAutomobileDriveStepInput {
                control: CAutomobileControlInput::default(),
                local_contact_points: contacts,
                contact_points: contacts,
                spring_ratios: [0.875; NUM_CAR_WHEELS],
                spring_directions: [CVector::new(0.0, 0.0, -1.0); NUM_CAR_WHEELS],
                grounded: [true; NUM_CAR_WHEELS],
                collision_entities: [Some(EntityReference::World(7)), Some(EntityReference::World(7)), None, None],
                collision_time_ms: 1_234,
                adhesive_limits: [1.0; NUM_CAR_WHEELS],
                wheel_scale: 1.0,
                time_step: 1.0,
                time_step_seconds: 0.02,
            },
            &mut handling,
            0.9,
            &mut general,
        None,
        );
        assert_eq!(car.vehicle.core.physical.collision_records, vec![EntityReference::World(7)]);
        assert_eq!(car.vehicle.core.physical.last_time_collided, 1_234);
    }

    #[test]
    fn grounded_wheel_results_apply_the_same_move_force_as_re3_process_wheel() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(
            90,
            2,
            0,
            &mut general,
            Some(CAutomobileHandlingSnapshot {
                mass: 1_000.0,
                turn_mass: 2_000.0,
                centre_of_mass: CVector::ZERO,
                dimension: CVector::new(2.0, 4.0, 1.5),
                buoyancy: 0.0,
                flags: 0,
                handling_id: 0,
                monetary_value: 0,
            }),
            None,
            false,
        );
        car.wheels_on_ground = 4;
        car.wheel_timer = [4.0; NUM_CAR_WHEELS];
        let contact_points = [
            CVector::new(-1.0, 1.0, -0.5),
            CVector::new(-1.0, -1.0, -0.5),
            CVector::new(1.0, 1.0, -0.5),
            CVector::new(1.0, -1.0, -0.5),
        ];

        car.process_wheels(
            CAutomobileWheelProcessInput {
                contact_speeds: [CVector::ZERO; NUM_CAR_WHEELS],
                contact_points,
                adhesive_limits: [1.0; NUM_CAR_WHEELS],
                acceleration: 0.01,
                brake: 0.0,
                traction: 1.0,
                traction_bias_front: 1.0,
                traction_bias_rear: 1.0,
                brake_bias_front: 1.0,
                brake_bias_rear: 1.0,
                front_wheel_drive: true,
                rear_wheel_drive: true,
                wheel_state_scratch: [tWheelState::WHEEL_STATE_NORMAL; NUM_CAR_WHEELS],
                already_skidding: false,
                time_step: 1.0,
                handling: CVehicleWheelHandling {
                    mass: 1_000.0,
                    traction_loss: 1.0,
                    wheel_friction: 0.9,
                },
            },
            &mut general,
        );

        assert!(
            car.vehicle.core.physical.move_speed.y > 0.0,
            "four driven ground wheels must apply their forward impulse"
        );
    }

    #[test]
    fn grounded_wheel_render_rotation_matches_re3_process_wheel_rotation() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        car.wheel_timer = [4.0; NUM_CAR_WHEELS];
        car.vehicle.core.physical.move_speed = CVector::new(0.0, 1.0, 0.0);
        car.wheel_state[CARWHEEL_REAR_RIGHT] = tWheelState::WHEEL_STATE_FIXED;

        car.update_wheel_rotation_for_render([CVector::ZERO; NUM_CAR_WHEELS], 1.0, 1.0);

        assert_eq!(car.wheel_rotation[CARWHEEL_FRONT_LEFT], -2.0);
        assert_eq!(car.wheel_rotation[CARWHEEL_REAR_LEFT], -2.0);
        assert_eq!(car.wheel_rotation[CARWHEEL_FRONT_RIGHT], -2.0);
        assert_eq!(car.wheel_rotation[CARWHEEL_REAR_RIGHT], 0.0);
    }

    #[test]
    fn setup_suspension_lines_matches_re3_limits_and_compressed_rest_height() {
        let mut general = CGeneral::with_seed(1);
        let mut car = CAutomobile::new(90, 2, 0, &mut general, None, None, false);
        let handling = tHandlingData {
            suspension_force_level: 1.0,
            suspension_upper_limit: 0.25,
            suspension_lower_limit: -0.15,
            ..Default::default()
        };
        let wheels = [
            CVector::new(-0.8, 1.2, -0.45),
            CVector::new(-0.8, -1.2, -0.45),
            CVector::new(0.8, 1.2, -0.45),
            CVector::new(0.8, -1.2, -0.45),
        ];
        car.setup_suspension_lines(wheels, 0.8, &handling);

        assert!(car.suspension_initialized);
        assert!((car.suspension_line_start[0].z + 0.2).abs() < 1.0e-6);
        assert!((car.suspension_line_end[0].z + 1.0).abs() < 1.0e-6);
        assert_eq!(car.suspension_line_start[0].x, -0.8);
        assert_eq!(car.suspension_line_start[0].y, 1.2);
        assert!((car.suspension_spring_length[0] - 0.4).abs() < 1.0e-6);
        assert!((car.suspension_line_length[0] - 0.8).abs() < 1.0e-6);
        // re3: height = -(p0.z + (p1.z-p0.z)*(1-1/(8*force))).
        let expected_height = -(-0.2 + (-0.8) * (1.0 - 1.0 / 8.0));
        assert!((car.height_above_road - expected_height).abs() < 1.0e-6);
        assert!((car.wheel_position[0] - (0.4 - expected_height)).abs() < 1.0e-6);
        assert_eq!(car.wheel_position, [car.wheel_position[0]; NUM_CAR_WHEELS]);
    }
}

// === FAZ2: moved from core/Game.rs (verbatim, parity-preserving) ===
// Vehicle dynamics: road contacts, static collision steps, passive physics,
// damage lifecycle, skidmark/exhaust registration. Schedule stays in core/Game.rs.

#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::collision::ColModel::{CColPoint, CColVector};
#[allow(unused_imports)]
use crate::control::Replay::{CReplay, ReplayMode};
#[allow(unused_imports)]
use crate::core::EventList::{CEventList, EEventEntity, EEventType};
#[allow(unused_imports)]
use crate::core::Game::{
    GameplayAuditTimer, ScriptEntityWorld, ScriptVehicleEntity, integrate_physical_matrix,
};
#[allow(unused_imports)]
use crate::core::Pools::CPoolHandle;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_RCBANDIT;
#[allow(unused_imports)]
use crate::peds::PlayerPed::CWorldPlayers;
#[allow(unused_imports)]
use crate::renderer::Weather::CWeather;
#[allow(unused_imports)]
use crate::vehicles::DamageManager::eEngineStatus;
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::weapons::Explosion::{CExplosionPool, eExplosionType};
#[allow(unused_imports)]
use bevy::prelude::*;
#[allow(unused_imports)]
use rustgta3_assets::assets::WorldAssets::CSurfaceTableAsset;
#[allow(unused_imports)]
use rustgta3_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

#[cfg(test)]
fn register_automobile_skidmarks(
    mut vehicles: Query<(Entity, &crate::control::ScriptEntities::ScriptVehicleIdentity, &VehicleCore, &mut AutomobileState)>,
    index: Res<crate::control::ScriptEntities::ScriptEntityIndex>,
    mut skidmarks: ResMut<CSkidmarks>,
    mut particles: ResMut<CParticle>,
    weather: Res<CWeather>,
    mut general: ResMut<CGeneral>,
    timer: Res<CTimer>,
    replay: Option<Res<CReplay>>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    camera: Res<crate::core::Camera::CCamera>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let replay_playback = replay.is_some_and(|replay| replay.mode == ReplayMode::Playback);
    for (entity, identity, core, mut state) in &mut vehicles {
        if index.vehicle(identity.handle) != Some(entity) { continue; }
        if identity.kind != crate::control::ScriptEntities::ScriptVehicleKind::Automobile {
            bevy::log::warn_once!("automobile_wheel_effects_invalid_live_kind");
            continue;
        }
        super::AutomobileEffects::wheel_effects(
            core,
            &mut state,
            &mut particles,
            Some(&mut skidmarks),
            identity.handle as usize,
            weather.wet_roads,
            timer.frame_counter(),
            timer.time_in_milliseconds(),
            &mut general,
            replay_playback,
        );
    }
}

/// re3 `CAutomobile::PreRender` exhaust path (`vehicles/Automobile.cpp:1444-1483`).
///
/// Exhaust positions are authored by `CVehicleModelInfo::PreprocessHierarchy`
/// from the model's `exhaust` frame. Resident parsed descriptors retain this
/// model-space position; this stage applies the live vehicle matrix. It stays
/// in RenderPrep, after vehicle
/// control/physics and immediately before the renderer drains particle intents,
/// matching the C++ PreRender ordering without introducing a second particle
/// simulation path.

pub fn process_single_automobile_exhaust_effects<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    handling_flags: u32,
    file_loader: &CFileLoader,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
) where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    super::AutomobileEffects::exhaust_effects(&car.vehicle.core, &car.vehicle.drive, handling_flags, file_loader, particles, general)
}

#[derive(Debug, Clone, Copy)]
pub struct AutomobileRoadContacts {
    pub local_points: [CVector; NUM_CAR_WHEELS],
    pub relative_world_points: [CVector; NUM_CAR_WHEELS],
    pub spring_ratios: [f32; NUM_CAR_WHEELS],
    pub spring_directions: [CVector; NUM_CAR_WHEELS],
    pub grounded: [bool; NUM_CAR_WHEELS],
    pub collision_entities: [Option<EntityReference>; NUM_CAR_WHEELS],
    pub ground_entities: [Option<usize>; NUM_CAR_WHEELS],
    pub surfaces: [u8; NUM_CAR_WHEELS],
    pub wheel_scale: f32,
}

/// Build re3's four `contactPoints[]` from the automobile COL suspension
/// lines. These are original GTA model-space lines, not guessed render bounds;
/// the fallback exists only for a malformed/unloaded COL and uses its actual
/// bounding box when available.
pub fn automobile_road_contacts<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &CFileLoader,
    time_step: f32,
) -> AutomobileRoadContacts
where
    AutoCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::Deref<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
    let model_info = file_loader.model_info.get_model_info(model_id);
    let col_model = model_info.and_then(|model| model.base().col_model.as_ref());
    let wheel_scale = match model_info {
        Some(ModelInfoSlot::Vehicle(model)) => model.wheel_scale.max(0.001),
        _ => 1.0,
    };
    let local_starts = if car.suspension_initialized {
        car.suspension_line_start
    } else if let Some(col) = col_model
        && col.lines.len() >= NUM_CAR_WHEELS
    {
        std::array::from_fn(|wheel| {
            let point = col.lines[wheel].p0;
            CVector::new(point.x, point.y, point.z)
        })
    } else {
        let top = car.vehicle.core.physical.centre_of_mass.z.max(0.25);
        [CVector::new(0.0, 0.0, top); NUM_CAR_WHEELS]
    };
    let local_points = if car.suspension_initialized {
        std::array::from_fn(|wheel| {
            let point = car.suspension_line_end[wheel];
            CVector::new(point.x, point.y, point.z)
        })
    } else if let Some(col) = col_model
        && col.lines.len() >= NUM_CAR_WHEELS
    {
        std::array::from_fn(|wheel| {
            let point = col.lines[wheel].p1;
            CVector::new(point.x, point.y, point.z)
        })
    } else {
        let (min, max) = col_model.map_or(
            (CVector::new(-1.0, -2.0, -0.5), CVector::new(1.0, 2.0, 0.8)),
            |col| {
                (
                    CVector::new(
                        col.bounding_box.min.x,
                        col.bounding_box.min.y,
                        col.bounding_box.min.z,
                    ),
                    CVector::new(
                        col.bounding_box.max.x,
                        col.bounding_box.max.y,
                        col.bounding_box.max.z,
                    ),
                )
            },
        );
        let x = 0.8 * min.x.abs().max(max.x.abs()).max(0.5);
        let front = max.y * 0.72;
        let rear = min.y * 0.72;
        [
            CVector::new(-x, front, min.z),
            CVector::new(-x, rear, min.z),
            CVector::new(x, front, min.z),
            CVector::new(x, rear, min.z),
        ]
    };

    // CAutomobile::ProcessControl applies the previous frame's move/turn
    // speed before collision populates the suspension line ratios.  Probe
    // against that pose even though the live matrix is committed inside
    // `process_player_drive_step`, so suspension and tyre forces observe the
    // same transform as re3.
    let contact_matrix = integrate_physical_matrix(
        car.vehicle.core.physical.entity.matrix,
        car.vehicle.core.physical.move_speed,
        car.vehicle.core.physical.turn_speed,
        time_step.max(0.0),
    );
    let position = contact_matrix.position;
    let relative_starts: [CVector; NUM_CAR_WHEELS] =
        std::array::from_fn(|wheel| contact_matrix.multiply3x3(local_starts[wheel]));
    let relative_ends =
        std::array::from_fn(|wheel| contact_matrix.multiply3x3(local_points[wheel]));
    let mut grounded = [false; NUM_CAR_WHEELS];
    let mut collision_entities = [None; NUM_CAR_WHEELS];
    let mut ground_entities = [None; NUM_CAR_WHEELS];
    let mut spring_ratios = [1.0; NUM_CAR_WHEELS];
    let mut spring_directions = [CVector::new(0.0, 0.0, -1.0); NUM_CAR_WHEELS];
    let mut relative_world_points = relative_ends;
    let mut surfaces =
        [crate::core::SurfaceTable::eSurfaceType::SURFACE_TARMAC as u8; NUM_CAR_WHEELS];
    for wheel in 0..NUM_CAR_WHEELS {
        let start = relative_starts[wheel];
        let end = relative_ends[wheel];
        let line = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
        let line_length = line.magnitude().max(1.0e-5);
        spring_directions[wheel] = CVector::new(
            line.x / line_length,
            line.y / line_length,
            line.z / line_length,
        );
        let world_start = [
            position.x + start.x,
            position.y + start.y,
            position.z + start.z,
        ];
        let world_end = [position.x + end.x, position.y + end.y, position.z + end.z];
        let Some(contact) = file_loader.world.process_line_of_sight_static_contact(
            world_start,
            world_end,
            &file_loader.model_info,
            None,
            true,
        ) else {
            continue;
        };
        let line_len_sq = line.magnitude_sqr();
        if line_len_sq <= 1.0e-10 {
            continue;
        }
        let hit_delta = CVector::new(
            contact.point[0] - world_start[0],
            contact.point[1] - world_start[1],
            contact.point[2] - world_start[2],
        );
        let raw_ratio =
            (hit_delta.x * line.x + hit_delta.y * line.y + hit_delta.z * line.z) / line_len_sq;
        if !(-0.001..=1.001).contains(&raw_ratio) {
            continue;
        }
        let raw_ratio = raw_ratio.clamp(0.0, 1.0);
        grounded[wheel] = true;
        collision_entities[wheel] = Some(EntityReference::World(contact.entity_index));
        if file_loader
            .world
            .entities
            .get(contact.entity_index)
            .is_some_and(|entity| entity.entity_type == crate::core::World::EEntityType::Building)
        {
            ground_entities[wheel] = Some(contact.entity_index);
        }
        surfaces[wheel] = contact.surface;
        relative_world_points[wheel] = CVector::new(
            contact.point[0] - position.x,
            contact.point[1] - position.y,
            contact.point[2] - position.z,
        );
        let wheel_fraction = if car.suspension_initialized {
            (1.0 - car.suspension_spring_length[wheel]
                / car.suspension_line_length[wheel].max(1.0e-5))
            .clamp(0.0, 0.999)
        } else {
            0.0
        };
        spring_ratios[wheel] =
            ((raw_ratio - wheel_fraction) / (1.0 - wheel_fraction)).clamp(0.0, 1.0);
    }
    AutomobileRoadContacts {
        local_points,
        relative_world_points,
        spring_ratios,
        spring_directions,
        grounded,
        collision_entities,
        ground_entities,
        surfaces,
        wheel_scale,
    }
}

pub fn store_automobile_wheel_contacts<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    contacts: &AutomobileRoadContacts,
    file_loader: Option<&CFileLoader>,
) where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    use crate::core::SurfaceTable::eSurfaceType::{SURFACE_GRASS, SURFACE_MUD_DRY, SURFACE_SAND};

    let position = car.vehicle.core.physical.entity.get_position();
    for wheel in 0..NUM_CAR_WHEELS {
        let relative = contacts.relative_world_points[wheel];
        car.wheel_contact_points[wheel] = CVector::new(
            position.x + relative.x,
            position.y + relative.y,
            position.z + relative.z,
        );
        car.wheel_contact_surfaces[wheel] = contacts.surfaces[wheel];
        car.wheel_skidmark_muddy[wheel] = matches!(
            contacts.surfaces[wheel],
            surface
                if surface == SURFACE_GRASS as u8
                    || surface == SURFACE_MUD_DRY as u8
                    || surface == SURFACE_SAND as u8
        );
        if let Some(entity_index) = contacts.ground_entities[wheel]
            && let Some(file_loader) = file_loader
        {
            // re3 CAutomobile::ProcessEntityCollision updates
            // CVehicle::m_pCurGroundEntity whenever a suspension line hits a
            // building. It intentionally leaves the previous pointer alone on
            // frames without a building contact.
            car.vehicle.core.current_ground_entity = Some(entity_index);
            if let Some(map_object_index) = file_loader
                .world
                .entities
                .get(entity_index)
                .and_then(|entity| entity.map_object_slot)
            {
                // The same native ProcessEntityCollision call ends in
                // CPhysical::AddCollisionRecord(ent), which in turn refreshes
                // m_treadable. Preserve that road-object identity as well as
                // m_pCurGroundEntity.
                car.vehicle
                    .core
                    .physical
                    .add_collision_record_treadable(map_object_index, &file_loader.path_find);
            }
        }
    }
}

pub fn automobile_static_collision_step<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    saved_matrix: CMatrix,
    vehicle_handle: Option<i32>,
    file_loader: &CFileLoader,
    audio_camera_position: CVector,
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    time_ms: Option<u32>,
    _collision_damage_multiplier: f32,
    _general: &mut CGeneral,
    mut dmaudio: Option<&mut cDMAudio>,
) where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    crate::profile_scope!("automobile_static_collision_step");
    let _audit_timer = GameplayAuditTimer::start("automobile_static_collision_step");
    // re3 `CPhysical::ProcessCollision` dispatches STATUS_SIMPLE cars to
    // `CheckCollision_SimpleCar`, which only scans vehicles/objects.  Their
    // position has already been authored by `CCarCtrl::UpdateCarOnRails`; the
    // ordinary static-building branch below saves/restores a collision matrix
    // and would therefore rewind every rail step to the previous frame.
    if car.vehicle.core.physical.entity.status == eEntityStatus::STATUS_SIMPLE {
        return;
    }
    let mut integrated_matrix = car.vehicle.core.physical.entity.matrix;
    let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
    let mut strongest: Option<(f32, crate::core::World::StaticModelContact)> = None;
    let mut responded = false;
    let mut safe = false;

    // Native CWorld gives an unsafe physical one ProcessCollision pass, four
    // retries, then one final stuck pass. Every pass starts from the matrix
    // saved before integration; only velocity/torque changes survive a rejected
    // collision. The previous port used the first colliding sample as that
    // origin, then added a whole frame of movement on top of it, which could
    // bury or suspend a car at walls and ground edges.
    for attempt in 0..=5 {
        let steps = car.vehicle.core.physical.vehicle_collision_steps(time_step);
        let mut rejecting = None;
        for step in 1..=steps {
            let collision_time_step = if step == steps { time_step }
                else { time_step * step as f32 / steps as f32 };
            if step == steps {
                car.vehicle.core.physical.entity.set_matrix(integrated_matrix);
            } else {
                // Native restores savedMatrix after every successful partial
                // check. ApplyTurnSpeed intentionally leaves axes denormalised
                // here; only the final full-frame pose is reorthogonalised.
                let physical = &mut car.vehicle.core.physical;
                physical.entity.set_matrix(saved_matrix);
                physical.apply_move_speed(collision_time_step);
                physical.apply_turn_speed(collision_time_step);
            }
            let contacts = automobile_static_contacts_for_matrix(file_loader, model_id,
                &car.vehicle.core.physical.entity.matrix);
            if !contacts.is_empty() {
                car.vehicle.core.physical.entity.flags.has_hit_wall = true;
                // ProcessCollisionSectorList averages only B's manifold and
                // returns after its first response. Unrelated floors/walls
                // cannot dilute its impulse or adhesive limit.
                for manifold in contacts.chunk_by(|left, right|
                    left.entity_index == right.entity_index)
                {
                    let (response_count, pass_strongest) = apply_automobile_static_contacts(
                        car, manifold, surface_table, collision_time_step, attempt == 0,
                        time_ms, file_loader, audio_camera_position, vehicle_handle, &mut dmaudio,
                    );
                    if response_count != 0 {
                        rejecting = Some(pass_strongest);
                        break;
                    }
                }
                if rejecting.is_some() { break; }
            }
            if step != steps {
                car.vehicle.core.physical.entity.set_matrix(saved_matrix);
                car.suspension_spring_ratio = [1.0; NUM_CAR_WHEELS];
            }
        }
        let Some(pass_strongest) = rejecting else {
            car.vehicle.core.physical.entity.set_matrix(integrated_matrix);
            safe = true;
            break;
        };
        responded = true;
        car.vehicle.core.physical.distance_travelled = 0.0;
        if pass_strongest
            .is_some_and(|candidate| strongest.is_none_or(|current| candidate.0 > current.0))
        {
            strongest = pass_strongest;
        }
        car.vehicle.core.physical.static_frames = 0;
        car.vehicle.core.flags.vehicle_col_processed = true;

        if attempt == 5 {
            break;
        }
        car.vehicle.core.physical.entity.set_matrix(saved_matrix);
        integrated_matrix = integrate_physical_matrix(
            saved_matrix,
            car.vehicle.core.physical.move_speed,
            car.vehicle.core.physical.turn_speed,
            time_step.max(0.0),
        );
    }

    if responded && !safe {
        // Separate CWorld ProcessShift passes. Each starts by applying this
        // frame's movement from the saved matrix, then tries the 1.2/0.2 and
        // 1.5/0.4 separation weights respectively.
        for second_shift in [false, true] {
            let physical = &mut car.vehicle.core.physical;
            let integrated = integrate_physical_matrix(
                saved_matrix,
                physical.move_speed,
                physical.turn_speed,
                time_step.max(0.0),
            );
            physical.entity.set_matrix(integrated);
            if automobile_static_contacts_for_matrix(
                file_loader,
                model_id,
                &car.vehicle.core.physical.entity.matrix,
            )
            .is_empty()
                || automobile_static_process_shift(car, file_loader, second_shift)
            {
                safe = true;
                break;
            }
            car.vehicle.core.physical.entity.set_matrix(saved_matrix);
        }
    }

    if safe {
        let final_position = car.vehicle.core.physical.entity.get_position();
        car.vehicle.core.physical.distance_travelled = CVector::new(
            final_position.x - saved_matrix.position.x,
            final_position.y - saved_matrix.position.y,
            final_position.z - saved_matrix.position.z,
        )
        .magnitude();
    } else {
        car.vehicle.core.physical.entity.set_matrix(saved_matrix);
        car.vehicle.core.physical.distance_travelled = 0.0;
        // This is the *outer* CWorld::Process final-stuck fallback, after the
        // initial collision pass, four retries, the stuck pass and both shift
        // passes have all failed.  re3 damps the player vehicle's remaining
        // velocity and advances once from the restored matrix.  Omitting this
        // leaves the player car snapping back to exactly the same penetrating
        // pose on successive frames, which is especially visible on the roof
        // and against walls.
        if car.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PLAYER {
            car.vehicle.core.physical.move_speed = CVector::new(
                car.vehicle.core.physical.move_speed.x * 0.3,
                car.vehicle.core.physical.move_speed.y * 0.3,
                car.vehicle.core.physical.move_speed.z * 0.3,
            );
            car.vehicle
                .core
                .physical
                .apply_move_speed(time_step.max(0.0));
            car.vehicle
                .core
                .physical
                .apply_turn_speed(time_step.max(0.0));
            car.vehicle.core.physical.entity.matrix.reorthogonalise();
        }
    }
    car.vehicle.core.physical.entity.flags.is_in_safe_position = safe;
    car.vehicle.core.physical.entity.flags.is_stuck = !safe;

    let Some((impulse, contact)) = strongest else {
        return;
    };
    car.vehicle.core.physical.record_collision_damage(
        contact.piece_a as u16,
        impulse,
        Some(EntityReference::World(contact.entity_index)),
        CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]),
    );
}

#[cfg(test)]
mod static_collision_regressions {
    use super::*;
    use crate::collision::ColModel::{CColBox, CColModel, CColSphere, CColVector};
    use rustgta3_formats::formats::FileLoader::GtaIplInstance;

    #[test]
    fn unrelated_nonresponding_floor_does_not_dilute_a_wall_manifold() {
        let mut loader = CFileLoader::default();
        let surfaces = CSurfaceTableAsset {
            filename: "unit_collision_surfaces".into(),
            data: rustgta3_formats::formats::DataRead::SurfaceDat::parse(
                "Rubber 1\nHard 1 1\nRoad 1 1 1\nLoose 1 1 1 1\nWet 1 1 1 1 1"
            ).expect("explicit unit adhesion table"),
        };
        loader.model_info.add_vehicle_model(200).clump.base.set_col_model(
            CColModel::new(CColSphere::new(0.4, CColVector::new(0.0, 0.0, 0.0), 10, 0),
                CColBox::new(CColVector::new(-0.4, -0.4, -0.4),
                    CColVector::new(0.4, 0.4, 0.4), 10, 0),
                vec![CColSphere::new(0.4, CColVector::new(0.0, 0.0, 0.0), 10, 0)]), true);
        for (id, min, max) in [
            (1000, [-2.0, -2.0, -0.3], [2.0, 2.0, -0.2]),
            (1001, [0.3, -2.0, -2.0], [0.5, 2.0, 2.0]),
        ] {
            let bounds = CColBox::new(CColVector::new(min[0], min[1], min[2]),
                CColVector::new(max[0], max[1], max[2]), 0, 0);
            let mut col = CColModel::new(CColSphere::new(4.0,
                CColVector::new(0.0, 0.0, 0.0), 0, 0), bounds.clone(), Vec::new());
            col.boxes.push(bounds);
            loader.model_info.add_simple_model(id).base.set_col_model(col, true);
            loader.world.load_object_instance(&GtaIplInstance {
                id, name: "unit_collision_surface".into(), position: [0.0; 3],
                scale: [1.0; 3], rotation_axis: [0.0; 3], angle: 1.0,
            }, &loader.model_info, &Default::default()).unwrap();
        }
        let mut tangential_friction = Vec::new();
        for include_floor in [false, true] {
            loader.world.entities[0].uses_collision = include_floor;
            let mut general = CGeneral::with_seed(7);
            let mut car = CAutomobile::new(200, 1, 0, &mut general, None, None, false);
            car.vehicle.core.physical.mass = 1000.0;
            car.vehicle.core.physical.turn_mass = 1500.0;
            car.vehicle.core.physical.entity.status = eEntityStatus::STATUS_PHYSICS;
            // Native upright cars deliberately disable this body-friction
            // branch. A roof/side contact exercises its actual adhesion path.
            car.vehicle.core.physical.entity.matrix.forward.y = -1.0;
            car.vehicle.core.physical.entity.matrix.up.z = -1.0;
            car.vehicle.core.physical.move_speed = CVector::new(1.0, 0.5, 0.0);
            let mut saved = car.vehicle.core.physical.entity.matrix;
            saved.position = CVector::new(-1.0, -0.5, 0.0);
            let contacts = automobile_static_contacts_for_matrix(&loader, 200,
                &car.vehicle.core.physical.entity.matrix);
            assert_eq!(contacts.len(), if include_floor { 2 } else { 1 },
                "fixture must add exactly one independent nonresponding manifold");
            automobile_static_collision_step(&mut car, saved, Some(257), &loader,
                CVector::ZERO, &surfaces, 1.0, Some(100), 1.0, &mut general, None);
            assert!((car.vehicle.core.physical.move_speed.x + 0.05).abs() < 0.0001);
            assert!((car.vehicle.core.physical.damage.impulse - 1050.0).abs() < 0.01);
            tangential_friction.push(car.vehicle.core.physical.move_friction.y);
        }
        assert!(tangential_friction[0] < 0.0, "wall must establish actual tangential friction");
        assert!((tangential_friction[0] - tangential_friction[1]).abs() < 1.0e-7,
            "unrelated floor diluted native wall adhesion: {:?}", tangential_friction);

    }

    #[test]
    fn leaving_an_early_contact_does_not_skip_the_later_wall() {
        let mut loader = CFileLoader::default();
        let surfaces = CSurfaceTableAsset {
            filename: "unit_collision_surfaces".into(),
            data: rustgta3_formats::formats::DataRead::SurfaceDat::parse(
                "Rubber 1\nHard 1 1\nRoad 1 1 1\nLoose 1 1 1 1\nWet 1 1 1 1 1"
            ).expect("explicit unit adhesion table"),
        };
        let car_bounds = CColBox::new(CColVector::new(-0.4, -0.4, -0.4),
            CColVector::new(0.4, 0.4, 0.4), 10, 0);
        loader.model_info.add_vehicle_model(200).clump.base.set_col_model(
            CColModel::new(CColSphere::new(0.4, CColVector::new(0.0, 0.0, 0.0), 10, 0),
                car_bounds, vec![CColSphere::new(0.4, CColVector::new(0.0, 0.0, 0.0), 10, 0)]), true);
        let mut walls = Vec::new();
        for (id, x) in [(1000, -0.5), (1001, 1.0)] {
            let bounds = CColBox::new(CColVector::new(-0.4, -1.0, -1.0),
                CColVector::new(0.4, 1.0, 1.0), 0, 0);
            let mut col = CColModel::new(
                CColSphere::new(2.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
                bounds.clone(), Vec::new());
            col.boxes.push(bounds);
            loader.model_info.add_simple_model(id).base.set_col_model(col, true);
            walls.push(loader.world.load_object_instance(&GtaIplInstance {
                id, name: "unit_collision_wall".into(),
                position: [x, 0.0, 0.0], scale: [1.0; 3],
                rotation_axis: [0.0; 3], angle: 1.0,
            }, &loader.model_info, &Default::default()).unwrap());
        }
        let mut general = CGeneral::with_seed(7);
        let mut car = CAutomobile::new(200, 1, 0, &mut general, None, None, false);
        // Explicit collision fixture inertia; production constructors require
        // installed handling data and never substitute these unit values.
        car.vehicle.core.physical.mass = 1000.0;
        car.vehicle.core.physical.turn_mass = 1500.0;
        car.vehicle.core.physical.entity.status = eEntityStatus::STATUS_PLAYER;
        car.vehicle.core.physical.move_speed = CVector::new(1.0, 0.0, 0.0);
        car.vehicle.core.physical.entity.flags.use_collision_records = true;
        let saved = car.vehicle.core.physical.entity.matrix;
        let mut first_probe = saved;
        first_probe.position.x = 0.2;
        let early = automobile_static_contacts_for_matrix(&loader, 200, &first_probe);
        assert!(!early.is_empty(), "fixture must start by leaving an actual COL overlap");
        car.vehicle.core.physical.entity.set_matrix(first_probe);
        let (responding, _) = apply_automobile_static_contacts(&mut car, &early,
            &surfaces, 0.2, true, Some(100), &loader,
            CVector::ZERO, Some(257), &mut None);
        assert_eq!(responding, 0, "early manifold must be moving away");
        let integrated = integrate_physical_matrix(saved,
            car.vehicle.core.physical.move_speed, CVector::ZERO, 1.0);
        car.vehicle.core.physical.entity.set_matrix(integrated);
        automobile_static_collision_step(&mut car, saved, Some(257), &loader,
            CVector::ZERO, &surfaces, 1.0, Some(100), 1.0,
            &mut general, None);
        assert!(car.vehicle.core.physical.collision_records
            .contains(&EntityReference::World(walls[1])),
            "later closing wall was skipped after the early nonresponding contact");
        assert!(car.vehicle.core.physical.damage.impulse > 0.0);
        assert!(car.vehicle.core.physical.move_speed.x < 1.0);
        assert!(car.vehicle.core.physical.entity.matrix.position.x.is_finite());
    }
}

pub(crate) fn automobile_static_contacts_for_matrix(
    file_loader: &CFileLoader,
    model_id: usize,
    matrix: &CMatrix,
) -> Vec<crate::core::World::StaticModelContact> {
    let Some(col) = file_loader.collision_model(model_id) else { return Vec::new(); };
    file_loader.world.col_model_contacts_static_world(
        [matrix.position.x, matrix.position.y, matrix.position.z],
        [
            [matrix.right.x, matrix.right.y, matrix.right.z],
            [matrix.forward.x, matrix.forward.y, matrix.forward.z],
            [matrix.up.x, matrix.up.y, matrix.up.z],
        ],
        col,
        &file_loader.model_info,
    )
}

pub(crate) fn apply_automobile_static_contacts<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    contacts: &[crate::core::World::StaticModelContact],
    surface_table: &CSurfaceTableAsset,
    time_step: f32,
    allow_collision_torque: bool,
    time_ms: Option<u32>,
    file_loader: &CFileLoader,
    audio_camera_position: CVector,
    vehicle_handle: Option<i32>,
    dmaudio: &mut Option<&mut cDMAudio>,
) -> (u32, Option<(f32, crate::core::World::StaticModelContact)>)
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    // Static buildings use re3's ApplyCollisionAlt branch.  Keep one result
    // per authored vehicle sphere, then average the velocity/torque deltas as
    // CPhysical::ProcessCollision does for the whole manifold.
    let contact_count = contacts.len() as f32;
    let apply_body_friction = !car.vehicle.core.physical.entity.flags.has_contacted;
    let base_move_speed = car.vehicle.core.physical.move_speed;
    let base_turn_speed = car.vehicle.core.physical.turn_speed;
    let mut move_response = CVector::ZERO;
    let mut turn_response = CVector::ZERO;
    let mut response_count = 0_u32;
    let mut strongest: Option<(f32, crate::core::World::StaticModelContact)> = None;

    for contact in contacts {
        let mut point = CColPoint::new();
        point.point = CColVector::new(contact.point[0], contact.point[1], contact.point[2]);
        point.normal = CColVector::new(contact.normal[0], contact.normal[1], contact.normal[2]);
        point.set_surfaces(
            contact.surface_a,
            contact.piece_a,
            contact.surface_b,
            contact.piece_b,
        );
        point.depth = contact.depth;

        let response =
            car.vehicle
                .core
                .physical
                .apply_collision_alt(&point, true, time_step.max(0.0));
        if response.applied {
            response_count += 1;
            move_response = CVector::new(
                move_response.x + response.move_speed.x - base_move_speed.x,
                move_response.y + response.move_speed.y - base_move_speed.y,
                move_response.z + response.move_speed.z - base_move_speed.z,
            );
            turn_response = CVector::new(
                turn_response.x + response.turn_speed.x - base_turn_speed.x,
                turn_response.y + response.turn_speed.y - base_turn_speed.y,
                turn_response.z + response.turn_speed.z - base_turn_speed.z,
            );
            if strongest.is_none_or(|(impulse, _)| response.impulse > impulse) {
                strongest = Some((response.impulse, *contact));
            }
            if apply_body_friction {
                let mut adhesion = crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                    &surface_table.data,
                    point.surface_a,
                    point.surface_b,
                ) / contact_count;
                if car.vehicle.core.physical.entity.model_index == MI_RCBANDIT as i16 {
                    adhesion *= 0.2;
                } else if car.vehicle.core.physical.entity.status == eEntityStatus::STATUS_WRECKED {
                    adhesion *= 3.0;
                } else if car.vehicle.core.physical.entity.get_up().z > 0.3 {
                    adhesion = 0.0;
                } else {
                    adhesion *= (0.03 * response.impulse + 1.0).min(5.0);
                }
                if car
                    .vehicle
                    .core
                    .physical
                    .apply_collision_friction_single(adhesion, &point, time_step.max(0.0))
                    .applied
                {
                    car.vehicle.core.physical.entity.flags.has_contacted = true;
                }
            }
        }
        if let Some(map_object_index) = file_loader
            .world
            .entities
            .get(contact.entity_index)
            .and_then(|entity| entity.map_object_slot)
        {
            car.vehicle
                .core
                .physical
                .add_collision_record_treadable(map_object_index, &file_loader.path_find);
        }
        if let Some(time_ms) = time_ms {
            car.vehicle
                .core
                .physical
                .add_collision_record(EntityReference::World(contact.entity_index), time_ms);
        }

        if let Some(dmaudio) = dmaudio.as_deref_mut()
            && response.applied
        {
            // re3 reports the body's squared linear/angular speed as the
            // looping-collision intensity. Using linear magnitude here made
            // resting roof/side contacts far too loud and kept retriggering
            // scrape audio while the car was merely rocking in place.
            let collision_velocity = car
                .vehicle
                .core
                .physical
                .move_speed
                .magnitude_sqr()
                .max(car.vehicle.core.physical.turn_speed.magnitude_sqr());
            let mut collision_impulse = response.impulse;
            // Native ProcessCollisionSectorList deliberately quiets a car
            // resting upside down so repeated roof contacts do not sound like
            // full-strength crashes every frame.
            if car.vehicle.core.physical.entity.get_up().z < -0.6
                && car.vehicle.core.physical.move_speed.x.abs() < 0.05
                && car.vehicle.core.physical.move_speed.y.abs() < 0.05
            {
                collision_impulse *= 0.1;
            }
            dmaudio.ReportCollision(
                vehicle_handle,
                Some(contact.entity_index as i32),
                point.surface_a,
                point.surface_b,
                collision_impulse,
                collision_velocity,
                car.vehicle.core.physical.entity.get_position(),
                CVector::new(contact.point[0], contact.point[1], contact.point[2]),
                false,
                true,
                audio_camera_position,
            );
        }
    }

    if response_count != 0 {
        let divisor = response_count as f32;
        car.vehicle.core.physical.move_speed = CVector::new(
            base_move_speed.x + move_response.x / divisor,
            base_move_speed.y + move_response.y / divisor,
            base_move_speed.z + move_response.z / divisor,
        );
        car.vehicle.core.physical.turn_speed = CVector::new(
            base_turn_speed.x + turn_response.x / divisor,
            base_turn_speed.y + turn_response.y / divisor,
            base_turn_speed.z + turn_response.z / divisor,
        );

        // Initial CWorld collision pass only: re3 counteracts part of the
        // player car's collision response through the friction accumulator.
        // Later retry/shift passes run with bNoMoreCollisionTorque and skip
        // this. Omitting it leaves a side/roof contact with too much residual
        // angular energy, which shows up as visible rocking and repeated
        // scrape reports on the following frames.
        if allow_collision_torque
            && car.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PLAYER
            && car.vehicle.core.physical.move_speed.x.abs() > 0.2
            && car.vehicle.core.physical.move_speed.y.abs() > 0.2
        {
            let contact_divisor = contacts.len().max(1) as f32;
            car.vehicle.core.physical.move_friction.x += move_response.x * -0.3 / contact_divisor;
            car.vehicle.core.physical.move_friction.y += move_response.y * -0.3 / contact_divisor;
            car.vehicle.core.physical.turn_friction = CVector::new(
                car.vehicle.core.physical.turn_friction.x
                    + turn_response.x * -0.3 / contact_divisor,
                car.vehicle.core.physical.turn_friction.y
                    + turn_response.y * -0.3 / contact_divisor,
                car.vehicle.core.physical.turn_friction.z
                    + turn_response.z * -0.3 / contact_divisor,
            );
        }
    }
    (response_count, strongest)
}

pub(crate) fn automobile_static_process_shift<
    AutoCore,
    AutoVitality,
    AutoState,
    Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>, AutoAudio: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileAudioMotion>>(
    car: &mut crate::vehicles::Automobile::AutomobileRuntimeView<
        AutoCore,
        AutoVitality,
        AutoState,
        Drive, Appearance, Entry, Audio, AutoAudio>,
    file_loader: &CFileLoader,
    second_shift: bool,
) -> bool
where
    AutoCore: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleCore>,
    AutoVitality: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleVitality>,
    AutoState: std::ops::DerefMut<Target = crate::vehicles::Automobile::AutomobileState>,
{
    let entity = &car.vehicle.core.physical.entity;
    let contacts = file_loader.world.vehicle_contacts_static_world(
        [
            entity.get_position().x,
            entity.get_position().y,
            entity.get_position().z,
        ],
        [
            [
                entity.get_right().x,
                entity.get_right().y,
                entity.get_right().z,
            ],
            [
                entity.get_forward().x,
                entity.get_forward().y,
                entity.get_forward().z,
            ],
            [entity.get_up().x, entity.get_up().y, entity.get_up().z],
        ],
        entity.model_index.max(0) as usize,
        &file_loader.model_info,
    );
    let Some(deepest) = contacts
        .iter()
        .max_by(|left, right| left.depth.total_cmp(&right.depth))
    else {
        return false;
    };
    let count = contacts.len() as f32;
    let manifold_scale = if second_shift { 1.5 } else { 1.2 } / count;
    let deepest_scale = if second_shift { 0.4 } else { 0.2 };
    let mut shift = CVector::ZERO;
    for contact in &contacts {
        shift.x += contact.normal[0] * contact.depth * manifold_scale;
        shift.y += contact.normal[1] * contact.depth * manifold_scale;
        shift.z += contact.normal[2] * contact.depth * manifold_scale;
    }
    shift.x += deepest.normal[0] * deepest.depth * deepest_scale;
    shift.y += deepest.normal[1] * deepest.depth * deepest_scale;
    shift.z += deepest.normal[2] * deepest.depth * deepest_scale;
    let original = car.vehicle.core.physical.entity.matrix;
    if !car.vehicle.core.physical.apply_process_shift(shift) {
        return false;
    }
    let remaining_contacts = automobile_static_contacts_for_matrix(
        file_loader,
        original_model_id(car),
        &car.vehicle.core.physical.entity.matrix,
    );
    // Native ProcessShift does not require the shifted manifold to disappear.
    // It runs ProcessCollisionSectorList and accepts the pose when the
    // remaining contacts are separating/tangential and therefore produce no
    // collision impulse. Requiring zero sphere overlap makes a car on its
    // side/roof repeatedly over-shift and snap back.
    let requires_response = remaining_contacts.iter().any(|contact| {
        let point = CVector::new(contact.point[0], contact.point[1], contact.point[2]);
        let normal = CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]);
        let point_offset = CVector::new(
            point.x - car.vehicle.core.physical.entity.get_position().x,
            point.y - car.vehicle.core.physical.entity.get_position().y,
            point.z - car.vehicle.core.physical.entity.get_position().z,
        );
        car.vehicle
            .core
            .physical
            .get_speed_at_point(point_offset)
            .dot(normal)
            < 0.0
    });
    if !requires_response {
        car.vehicle.core.physical.entity.flags.is_in_safe_position = true;
        car.vehicle.core.physical.entity.flags.is_stuck = false;
        true
    } else {
        car.vehicle.core.physical.entity.set_matrix(original);
        car.vehicle.core.physical.entity.flags.is_in_safe_position = false;
        false
    }
}

/// re3 `CAutomobile::AddDamagedVehicleParticles` / `BlowUpCar` world adapter.
/// Collision has already updated health and component state when this runs.
pub(crate) fn process_automobile_damage_lifecycle(
    mut script_entities: ScriptEntityWorld,
    mut world_players: CWorldPlayers,
    timer: Res<CTimer>,
    file_loader: Res<CFileLoader>,
    mut general: ResMut<CGeneral>,
    mut particles: ResMut<CParticle>,
    mut explosions: ResMut<CExplosionPool>,
    mut events: ResMut<CEventList>,
    mut camera: ResMut<crate::core::Camera::CCamera>,
    mut dmaudio: ResMut<cDMAudio>,
    handling: Res<crate::vehicles::HandlingMgr::cHandlingDataMgr>,
    mut player_info: ResMut<crate::core::PlayerInfo::CPlayerInfo>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let now_ms = timer.time_in_milliseconds();
    let mut exploded = Vec::new();
    let focused_player = world_players
        .focused()
        .map(|player| EntityReference::Ped(player.pool_handle));
    let focused_vehicle = world_players
        .relation(world_players.player_in_focus() as i32)
        .and_then(|relation| script_entities.vehicle_handle_for_entity(relation.vehicle));
    for (handle, vehicle) in script_entities.vehicles.iter_mut() {
        if let crate::control::ScriptEntities::ScriptVehicleMut::Boat { mut boat, .. } = vehicle {
            if let Some(position) = boat.explosion_pending.take() {
                camera.cam_shake(0.7, position, now_ms);
                exploded.push((handle, position, boat.set_on_fire_entity));
            }
            continue;
        }
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle
        else {
            continue;
        };
        let previous_bomb_timer = car.vehicle.vitality.bomb_timer;
        let timed_bomb_active = car.bomb_type == eBombType::CARBOMB_TIMEDACTIVE;
        let delayed_explosion = car.process_delayed_explosion(timer.time_step(), now_ms);
        if delayed_explosion
            && focused_player.is_some()
            && car.vehicle.vitality.blow_up_entity == focused_player
            && focused_vehicle != Some(handle)
        {
            if let Some(data) = car
                .vehicle
                .core
                .handling_id
                .and_then(|id| usize::try_from(id).ok())
                .and_then(|id| handling.get_handling_data(id))
            {
                player_info.award_money_for_explosion(data.monetary_value, now_ms, &mut general);
            } else {
                bevy::log::warn_once!(target: "rustgta3::vehicles", "bomb explosion reward missing original handling data");
            }
        }
        if timed_bomb_active
            && (car.vehicle.vitality.bomb_timer as u16 & 0xFE00)
                != (previous_bomb_timer as u16 & 0xFE00)
        {
            dmaudio.PlayOneShot(
                car.vehicle.core.physical.audio_entity_id,
                crate::audio::soundlist::SOUND_CAR_BOMB_TICK as u16,
                0.0,
            );
        }
        let result = if delayed_explosion {
            CAutomobileDamageStepResult {
                on_fire: true,
                exploded: true,
            }
        } else {
            car.process_damage_step(
                timer.time_step(),
                timer.time_step_in_milliseconds() as f32,
                now_ms,
            )
        };
        if result.on_fire {
            if let Some(position) = automobile_damage_particle_position(&file_loader, &car,
                AutomobileDamageOrigin::Fire { looking_forward_first_person:
                    focused_vehicle == Some(handle) && camera.get_looking_forward_first_person() }) {
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_CARFLAME,
                position,
                CVector::new(
                    0.0,
                    0.0,
                    general.get_random_number_in_range_f32(0.01125, 0.09),
                ),
                Some(handle),
                0.9,
                0,
                0,
                0,
                0,
            );
            let smoke_position = CVector::new(
                position.x + general.get_random_number_in_range_f32(-0.5625, 0.5625),
                position.y + general.get_random_number_in_range_f32(-0.5625, 0.5625),
                position.z + general.get_random_number_in_range_f32(0.5625, 2.25),
            );
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_CARFLAME_SMOKE,
                smoke_position,
                CVector::ZERO,
                Some(handle),
                0.5,
                0,
                0,
                0,
                0,
            );
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_ENGINE_SMOKE2,
                position,
                CVector::ZERO,
                Some(handle),
                0.5,
                0,
                0,
                0,
                0,
            );
        }
            }
        if result.exploded {
            let base = &car.vehicle;
            exploded.push((
                handle,
                base.physical.entity.get_position(),
                car.set_on_fire_entity,
            ));
        }
    }

    for (handle, position, culprit) in exploded {
        let Some(vehicle_entity) = script_entities.vehicle_entity(handle) else {
            continue;
        };
        let is_boat = matches!(
            script_entities
                .vehicles
                .get(&script_entities.index, &handle),
            Some(crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. })
        );
        let occupant_handles = script_entities
            .script_occupant_handles(vehicle_entity)
            .filter(|(_, seat)| {
                !is_boat || *seat == crate::peds::VehicleRelations::VehicleSeat::Driver
            })
            .map(|(ped_handle, _)| ped_handle)
            .collect::<Vec<_>>();
        let occupant_players = (0..world_players.slot_count())
            .filter(|index| {
                world_players
                    .relation(*index as i32)
                    .is_some_and(|relation| relation.vehicle == vehicle_entity)
            })
            .collect::<Vec<_>>();
        for ped_handle in occupant_handles {
            if let Some(entity) = script_entities
                .peds
                .get_mut(&script_entities.index, &ped_handle)
            {
                entity
                    .ped
                    .set_dead(entity.vitality, ped_handle, now_ms, &mut events);
            }
        }
        for player_index in occupant_players {
            if let Some(player) = world_players.player_mut(player_index as i32) {
                player
                    .ped
                    .set_dead(player.vitality, player.pool_handle, now_ms, &mut events);
            }
        }
        explosions.AddExplosion(
            Some(EntityReference::Vehicle(handle)),
            culprit,
            eExplosionType::EXPLOSION_CAR,
            position,
            0,
            now_ms,
        );
        // CEvent's culprit is a CPed. Resolve through the already borrowed
        // canonical owners, retaining pool kind and generation throughout.
        let criminal = culprit.and_then(|reference| match reference {
            EntityReference::Ped(handle) => {
                let player = (0..world_players.slot_count()).any(|slot| {
                    world_players.player(slot as i32)
                        .is_some_and(|player| player.pool_handle == handle)
                });
                (player || script_entities.peds.contains_key(&script_entities.index, &handle))
                    .then_some(handle)
            }
            EntityReference::Entity(entity) => {
                (0..world_players.slot_count()).find_map(|slot| {
                    (world_players.player_entity(slot as i32) == Some(entity))
                        .then(|| world_players.player(slot as i32).map(|player| player.pool_handle))
                        .flatten()
                }).or_else(|| {
                    let handle = script_entities.index.ped_handle_for_entity(entity)?;
                    script_entities.peds.contains_key(&script_entities.index, &handle)
                        .then_some(handle)
                })
            }
            EntityReference::Vehicle(_) | EntityReference::Object(_) | EntityReference::World(_) => None,
        });
        let _ = events.register_entity_event(
            EEventType::Explosion,
            EEventEntity::Vehicle,
            handle,
            criminal,
            position,
            1_500,
            now_ms,
        );
    }
}

/// Cumulative vehicle-frame observations of required ProcessControl dependencies.
#[derive(Debug, Default, bevy::prelude::Resource)]
pub struct AutomobilePhysicsDiagnostics {
    pub waiting_for_collision_frames: u64,
    pub missing_handling_identity_frames: u64,
    pub missing_handling_data_frames: u64,
    pub processed_vehicle_frames: u64,
}

pub(crate) fn process_passive_automobile_physics(
    mut vehicles: Query<crate::control::ScriptEntities::VehicleComponents>,
    mut histories: Query<&mut VehicleCollisionState>,
    mut diagnostics: ResMut<AutomobilePhysicsDiagnostics>,
    file_loader: Res<CFileLoader>,
    collision: Res<CCollision>,
    mut handling_manager: ResMut<cHandlingDataMgr>,
    surface_table: Res<CSurfaceTableAsset>,
    timer: Res<CTimer>,
    water: Option<Res<crate::renderer::WaterLevel::CWaterLevel>>,
    weather: Res<crate::renderer::Weather::CWeather>,
    timecycle: Option<Res<CTimeCycle>>,
    mut particle_objects: ResMut<CParticleObjectRuntime>,
    mut general: ResMut<CGeneral>,
    particle_inputs: (ResMut<CParticle>, Res<crate::renderer::ParticleMgr::cParticleSystemMgr>, Res<crate::core::Camera::CCamera>),
    mut audio: Option<ResMut<crate::audio::DMAudio::cDMAudio>>,
) {
    let (mut particles, particle_systems, particle_camera) = particle_inputs;
    crate::profile_scope!("process_passive_automobile_physics");
    let _audit_timer = GameplayAuditTimer::start("process_passive_automobile_physics");
    let time_step = timer.time_step().max(0.0);
    let dt = timer.time_step_in_seconds().clamp(0.0, 0.05);
    let wheel_friction = handling_manager.wheel_friction;

    for item in vehicles.iter_mut() {
        let entity = item.0;
        let Some(mut vehicle) = crate::control::ScriptEntities::vehicle_mut(item) else {
            continue;
        };
        let Ok(mut collision_state) = histories.get_mut(entity) else {
            bevy::log::warn_once!("vehicle collision history missing");
            continue;
        };
        let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } = &mut vehicle
        else {
            continue;
        };
        let status = car.vehicle.core.physical.entity.status;
        if !matches!(
            status,
            eEntityStatus::STATUS_PHYSICS
                | eEntityStatus::STATUS_ABANDONED
                | eEntityStatus::STATUS_WRECKED
                | eEntityStatus::STATUS_PLAYER_DISABLED
        ) {
            continue;
        }
        // Script-created cars carry the island level from their spawn point,
        // just like re3's m_nZoneLevel.  Do not integrate gravity/suspension
        // while that island's collision is not resident; otherwise a mission
        // car teleported or left far from the player falls through unloaded
        // roads/bridges and returns as a "ghost" when streaming catches up.
        // LEVEL_GENERIC ambient traffic keeps processing normally.
        let zone_level = car.vehicle.core.physical.zone_level;
        if zone_level > crate::collision::ColModel::LEVEL_GENERIC as i8
            && zone_level != collision.collision_in_memory as i8
        {
            diagnostics.waiting_for_collision_frames += 1;
            continue;
        }
        // re3 `CAutomobile::ProcessDamage` guards the flipped-over damage with
        // `this != FindPlayerVehicle()`: the car the player is in never bleeds
        // health just for being upside down, so the player can rock it back
        // over instead of losing the mission car to a 5 s fire timer.
        let player_vehicle = matches!(
            status,
            eEntityStatus::STATUS_PLAYER | eEntityStatus::STATUS_PLAYER_DISABLED
        );
        if car.vehicle.core.physical.entity.get_up().z < 0.0
            && !player_vehicle
            && !car.not_damaged_upside_down
            && !car.vehicle.core.physical.flags.is_in_water
        {
            car.vehicle.vitality.health -= 4.0 * time_step;
        }
        let handling_id = car.vehicle.core.handling_id.or_else(|| {
            let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
            match file_loader.model_info.get_model_info(model_id) {
                Some(ModelInfoSlot::Vehicle(model)) => {
                    cHandlingDataMgr::get_handling_id(&model.handling_id)
                        .and_then(|id| i32::try_from(id).ok())
                }
                _ => None,
            }
        });
        let Some(handling_id) = handling_id else {
            diagnostics.missing_handling_identity_frames += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_physics", "CAutomobile::ProcessControl: required handling identity unavailable");
            continue;
        };
        car.vehicle.core.handling_id = Some(handling_id);
        let Some(handling) = usize::try_from(handling_id)
            .ok()
            .and_then(|id| handling_manager.get_handling_data_mut(id))
        else {
            diagnostics.missing_handling_data_frames += 1;
            bevy::log::warn_once!(target: "rustgta3::vehicle_physics", handling_id, "CAutomobile::ProcessControl: required handling data unavailable");
            continue;
        };
        diagnostics.processed_vehicle_frames += 1;

        if let Some(water) = water.as_deref() {
            let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
            if let Some(col) = file_loader.model_info.col_model(model_id) {
                process_automobile_buoyancy(
                    car,
                    CVector::new(
                        col.bounding_box.min.x,
                        col.bounding_box.min.y,
                        col.bounding_box.min.z,
                    ),
                    CVector::new(
                        col.bounding_box.max.x,
                        col.bounding_box.max.y,
                        col.bounding_box.max.z,
                    ),
                    water,
                    timer.time_in_milliseconds(),
                    weather.wind,
                    time_step,
                    &mut particle_objects,
                    &mut general,
                    timecycle.as_deref(),
                    AutomobileBuoyancyEffects {
                        particles: Some(&mut particles.emitter(&particle_systems, particle_camera.position)),
                        audio: audio.as_deref_mut(),
                        frame_counter: timer.frame_counter(),
                    },
                );
            }
        }

        {
            let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
            if let Some(ModelInfoSlot::Vehicle(model)) =
                file_loader.model_info.get_model_info(model_id)

            {
                car.setup_suspension_from_model(model, handling);
            }
        }

        let contacts = automobile_road_contacts(car, &file_loader, time_step);
        store_automobile_wheel_contacts(car, &contacts, Some(&file_loader));
        let adhesive_limits = contacts.surfaces.map(|surface| {
            crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                &surface_table.data,
                crate::core::SurfaceTable::eSurfaceType::SURFACE_WHEELBASE as u8,
                surface,
            )
        });
        let all_grounded = contacts.grounded.iter().all(|&g| g);

        // CAutomobile's abandoned/wrecked rest detector is deliberately
        // vehicle-specific (CPhysical only sleeps objects and non-ped peds).
        // It removes sub-millimetre spring jitter without freezing a car that
        // has received a real collision impulse or is still airborne.
        if matches!(
            status,
            eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED
        ) {
            let physical = &mut car.vehicle.core.physical;
            physical.move_speed_avg = CVector::new(
                (physical.move_speed_avg.x + physical.move_speed.x) * 0.5,
                (physical.move_speed_avg.y + physical.move_speed.y) * 0.5,
                (physical.move_speed_avg.z + physical.move_speed.z) * 0.5,
            );
            physical.turn_speed_avg = CVector::new(
                (physical.turn_speed_avg.x + physical.turn_speed.x) * 0.5,
                (physical.turn_speed_avg.y + physical.turn_speed.y) * 0.5,
                (physical.turn_speed_avg.z + physical.turn_speed.z) * 0.5,
            );
            let (move_limit, turn_limit, distance_limit) =
                if status == eEntityStatus::STATUS_WRECKED {
                    (0.006, 0.0015, 0.015)
                } else {
                    (0.003, 0.0009, 0.005)
                };
            let resting = all_grounded
                && physical.move_speed_avg.magnitude_sqr() <= (move_limit * time_step).powi(2)
                && physical.turn_speed_avg.magnitude_sqr() <= (turn_limit * time_step).powi(2)
                && physical.distance_travelled < distance_limit;
            if resting {
                physical.static_frames = physical.static_frames.saturating_add(1);
                let position = physical.entity.get_position();
                if physical.static_frames > 10
                    && !crate::control::CarRails::map_could_move_in_this_area(
                        position.x, position.y,
                    )
                {
                    physical.static_frames = 10;
                    physical.move_speed = CVector::ZERO;
                    physical.turn_speed = CVector::ZERO;
                    physical.move_friction = CVector::ZERO;
                    physical.turn_friction = CVector::ZERO;
                    physical.entity.flags.has_contacted = false;
                    physical.entity.flags.is_in_safe_position = false;
                    physical.entity.flags.was_postponed = false;
                    physical.entity.flags.has_hit_wall = false;
                    physical.entity.flags.has_collided = false;
                    physical.collision_records.clear();
                    physical.damage.piece_type = 0;
                    physical.damage.impulse = 0.0;
                    physical.damage.entity = None;
                    car.vehicle.core.flags.vehicle_col_processed = false;
                    collision_state.previous_matrix = physical.entity.matrix;
                    continue;
                }
            } else {
                physical.static_frames = 0;
            }
        }
        let (brake, handbrake) = match status {
            eEntityStatus::STATUS_ABANDONED => (0.2 * 255.0, false),
            eEntityStatus::STATUS_WRECKED => (0.05 * 255.0, true),
            eEntityStatus::STATUS_PLAYER_DISABLED => (255.0, true),
            _ => (
                car.vehicle.drive.brake_pedal.clamp(0.0, 1.0) * 255.0,
                car.vehicle.drive.is_handbrake_on,
            ),
        };
        let steering_left_right = if status == eEntityStatus::STATUS_PLAYER_DISABLED {
            0.0
        } else {
            -car.vehicle.drive.steer_input.clamp(-1.0, 1.0) * 128.0
        };
        let accelerate = if status == eEntityStatus::STATUS_PLAYER_DISABLED {
            0.0
        } else {
            car.vehicle.drive.gas_pedal.clamp(-1.0, 1.0) * 255.0
        };
        collision_state.previous_matrix = car.vehicle.core.physical.entity.matrix;
        let damage_nodes = crate::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(&file_loader, car.vehicle.core.physical.entity.model_index);
        car.process_player_drive_step(
            CAutomobileDriveStepInput {
                control: CAutomobileControlInput {
                    steering_left_right,
                    mouse_x: 0.0,
                    mouse_steering: false,
                    accelerate,
                    brake,
                    handbrake,
                    exit_vehicle: false,
                    controls_disabled: false,
                },
                local_contact_points: contacts.local_points,
                contact_points: contacts.relative_world_points,
                spring_ratios: contacts.spring_ratios,
                spring_directions: contacts.spring_directions,
                grounded: contacts.grounded,
                collision_entities: contacts.collision_entities,
                collision_time_ms: timer.time_in_milliseconds(),
                adhesive_limits,
                wheel_scale: contacts.wheel_scale,
                time_step,
                time_step_seconds: dt,
            },
            handling,
            wheel_friction,
            &mut general,
        damage_nodes,
        );
        let final_position = car.vehicle.core.physical.entity.get_position();
        let saved_position = collision_state.previous_matrix.position;
        car.vehicle.core.physical.distance_travelled = CVector::new(
            final_position.x - saved_position.x,
            final_position.y - saved_position.y,
            final_position.z - saved_position.z,
        )
        .magnitude();
    }
}

#[cfg(test)]
mod player_horn_tests {
    use super::*;
    use crate::core::Pad::{CControllerState, CPad};
    use crate::modelinfo::ModelIndices::{MI_POLICE, MI_TAXI};

    #[test]
    fn ordinary_horn_is_held_and_siren_distinguishes_tap_from_long_press() {
        let mut car = CAutomobile::new(
            MI_TAXI as i32,
            crate::vehicles::Vehicle::RANDOM_VEHICLE,
            0,
            &mut CGeneral::default(),
            None,
            None,
            false,
        );
        let mut pad = CPad::default();
        let held = CControllerState {
            left_shock: 255,
            ..Default::default()
        };
        pad.Update(held);
        car.process_player_horn(&pad, false);
        assert_eq!(car.vehicle.audio.car_horn_timer, 1);
        pad.Update(CControllerState::default());
        car.process_player_horn(&pad, false);
        assert_eq!(car.vehicle.audio.car_horn_timer, 0);
        car.vehicle.core.physical.entity.model_index = MI_POLICE as i16;
        pad.Update(held);
        car.process_player_horn(&pad, false);
        pad.Update(CControllerState::default());
        car.process_player_horn(&pad, false);
        assert!(car.vehicle.audio.siren_or_alarm);
        for _ in 0..5 {
            pad.Update(held);
            car.process_player_horn(&pad, false);
        }
        assert_eq!(car.vehicle.audio.car_horn_timer, 1);
        pad.Update(CControllerState::default());
        car.process_player_horn(&pad, false);
        assert!(
            car.vehicle.audio.siren_or_alarm,
            "holding horn must not toggle siren on release"
        );
    }
}

impl AutomobileState {
    /// CAutomobile::SetupSuspensionLines writes only canonical suspension history.
    pub fn setup_suspension_lines(&mut self, wheel_positions: [CVector; NUM_CAR_WHEELS], wheel_scale: f32, handling: &tHandlingData) {
        self.suspension_source = None;
        let radius = (wheel_scale * 0.5).max(f32::EPSILON);
        let force_level = handling.suspension_force_level.max(f32::EPSILON);
        for (index, authored) in wheel_positions.into_iter().enumerate() {
            self.suspension_line_start[index] = CVector::new(
                authored.x,
                authored.y,
                authored.z + handling.suspension_upper_limit,
            );
            self.suspension_line_end[index] = CVector::new(
                authored.x,
                authored.y,
                authored.z + handling.suspension_lower_limit - radius,
            );
            self.suspension_spring_length[index] =
                handling.suspension_upper_limit - handling.suspension_lower_limit;
            self.suspension_line_length[index] =
                self.suspension_line_start[index].z - self.suspension_line_end[index].z;
        }
        self.height_above_road = -(self.suspension_line_start[0].z
            + (self.suspension_line_end[0].z - self.suspension_line_start[0].z)
                * (1.0 - 1.0 / (8.0 * force_level)));
        self.wheel_position = [radius - self.height_above_road; NUM_CAR_WHEELS];
        self.suspension_initialized = true;
    }

    pub fn setup_suspension_from_model(&mut self, model_index: i16,
        model: &crate::modelinfo::ModelInfo::CVehicleModelInfo, handling: &tHandlingData) -> bool {
        let Some(descriptors) = model.authored_positions() else {
            self.invalidate_suspension_source();
            bevy::log::warn_once!("automobile_suspension_waiting_resident_wheel_frames");
            return false;
        };
        let source = AutomobileSuspensionSource {
            model_index, revision: model.clump.base.rw_object_revision(),
            handling_id: handling.identifier,
        };
        if self.suspension_initialized && self.suspension_source == Some(source) {
            return true;
        }
        let Some(positions) = descriptors.wheels.local_positions() else {
            self.invalidate_suspension_source();
            bevy::log::warn_once!("automobile_suspension_required_wheel_frames_invalid_or_missing");
            return false;
        };
        self.setup_suspension_lines(positions, model.wheel_scale, handling);
        self.suspension_source = Some(source);
        true
    }

    pub fn invalidate_suspension_source(&mut self) {
        self.suspension_source = None;
        self.suspension_initialized = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutomobileSuspensionSource {
    pub model_index: i16,
    pub revision: u64,
    pub handling_id: usize,
}
