//! CBoat::ProcessControl PC particle and landing-audio branch.
use crate::{
    audio::{DMAudio::cDMAudio, soundlist::SOUND_CAR_SPLASH},
    core::{Cam::eCamMode, Camera::CCamera},
    entities::Entity::eEntityStatus,
    renderer::{
        Particle::{ParticleEmitter, ParticleColour},
        ParticleType::*,
        Timecycle::CTimeCycle,
        WaterLevel::CWaterLevel,
    },
    vehicles::Boat::BoatWaterEffects,
};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

pub struct BoatEffectContext<'a> {
    pub particles: ParticleEmitter<'a>,
    pub general: &'a mut CGeneral,
    pub audio: &'a mut cDMAudio,
    pub water: &'a CWaterLevel,
    pub colours: BoatWaterColours,
    pub camera: &'a CCamera,
    pub frame: u32,
    pub time_ms: u32,
    pub wind: f32,
}

#[derive(Clone, Copy)]
pub struct BoatWaterColours {
    pub splash: ParticleColour,
    pub jet: ParticleColour,
}

pub fn boat_water_colours(timecycle: &CTimeCycle, general: &mut CGeneral) -> BoatWaterColours {
    let channels = |scale: f32| {
        let current = &timecycle.current;
        let channel = |index: usize| {
            ((current.ambient[index] + 0.5 * current.directional[index]) * scale).clamp(0.0, 255.0)
                as u8
        };
        [channel(0), channel(1), channel(2)]
    };
    let splash_rgb = channels(114.75);
    let jet_rgb = channels(242.25);
    let splash = ParticleColour {
        red: splash_rgb[0],
        green: splash_rgb[1],
        blue: splash_rgb[2],
        alpha: general.get_random_number_in_range_i32(128, 150) as u8,
    };
    let jet = ParticleColour {
        red: jet_rgb[0],
        green: jet_rgb[1],
        blue: jet_rgb[2],
        alpha: general.get_random_number_in_range_i32(96, 128) as u8,
    };
    general.get_random_number();
    BoatWaterColours { splash, jet }
}

pub fn emit_boat_water_effects<BoatCore, BoatVitality, BoatState, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(
    boat: &crate::vehicles::Boat::BoatRuntimeView<BoatCore, BoatVitality, BoatState, Drive, Appearance, Entry, Audio>,
    effects: BoatWaterEffects,
    bounds_min: CVector,
    bounds_max: CVector,
    context: &mut BoatEffectContext<'_>,
)
where
    BoatCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    BoatVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
    BoatState: std::ops::Deref<Target = crate::vehicles::Boat::BoatState>,
{
    let scaled = |v: CVector, scale: f32| CVector::new(v.x * scale, v.y * scale, v.z * scale);
    let add = |a: CVector, b: CVector| CVector::new(a.x + b.x, a.y + b.y, a.z + b.z);
    let splash_colour = context.colours.splash;
    let jet_colour = context.colours.jet;
    let matrix = boat.vehicle.core.physical.entity.matrix;
    if let Some(propeller) = effects.propeller {
        let mut direction = scaled(propeller.force, -0.04);
        if boat.vehicle.drive.gas_pedal > 0.0 {
            if boat.vehicle.core.physical.entity.status == eEntityStatus::STATUS_PLAYER {
                let camera_hack = context
                    .camera
                    .cams
                    .get(context.camera.active_cam)
                    .is_some_and(|cam| cam.mode == eCamMode::MODE_TOPDOWN);
                // TODO(parity): CBoat::ProcessControl CAMCONTROL_OBBE camera interface.
                let stern = matrix.multiply3x3(CVector::new(0.0, bounds_min.y, 0.0));
                let mut jet_position = add(matrix.position, stern);
                jet_position.z = if camera_hack { 1.0 } else { 0.0 };
                let mut wake_position = add(matrix.position, stern);
                wake_position.z -= 0.65;
                context.particles.add_particle_coloured(
                    PARTICLE_BOAT_THRUSTJET,
                    jet_position,
                    direction,
                    None,
                    0.0,
                    jet_colour,
                    0,
                    0,
                    0,
                    0,
                );
                context.particles.add_particle_coloured(
                    PARTICLE_CAR_SPLASH,
                    jet_position,
                    scaled(direction, 0.25),
                    None,
                    1.0,
                    splash_colour,
                    context.general.get_random_number_in_range_i32(0, 30),
                    context.general.get_random_number_in_range_i32(0, 90),
                    3,
                    0,
                );
                if !camera_hack {
                    context.particles.add_particle_coloured(
                        PARTICLE_BOAT_WAKE,
                        wake_position,
                        scaled(direction, 0.75),
                        None,
                        0.0,
                        jet_colour,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            } else if (context.frame + boat.vehicle.core.physical.entity.random_seed as u32) & 1 != 0 {
                direction.z = 0.018;
                direction.x *= 0.01;
                direction.y *= 0.01;
                let position = add(propeller.world_position, CVector::new(0.0, 0.0, 1.5));
                context.particles.add_particle_coloured(
                    PARTICLE_BOAT_SPLASH,
                    position,
                    direction,
                    None,
                    1.5,
                    jet_colour,
                    0,
                    0,
                    0,
                    0,
                );
                context.particles.add_particle_coloured(
                    PARTICLE_CAR_SPLASH,
                    position,
                    scaled(direction, 0.1),
                    None,
                    0.5,
                    splash_colour,
                    context.general.get_random_number_in_range_i32(0, 30),
                    context.general.get_random_number_in_range_i32(0, 90),
                    3,
                    0,
                );
            }
        }
    }
    if effects.landing_splash {
        context.audio.PlayOneShot(
            boat.vehicle.core.physical.audio_entity_id,
            SOUND_CAR_SPLASH as u16,
            boat.delta_volume_under_water as f32,
        );
    }
    if boat.delta_volume_under_water > 75 {
        let speed = boat.vehicle.core.physical.move_speed;
        let magnitude = speed.magnitude();
        let splash_size = magnitude.min(0.75);
        let boat_size = boat.delta_volume_under_water as f32 * 0.005 * 0.2;
        for side in [1.0, -1.0] {
            let mut direction = add(
                scaled(speed, -0.5),
                scaled(matrix.right, side * 0.5 * magnitude),
            );
            direction.z += 0.1 * magnitude;
            let mut position = add(
                add(
                    add(matrix.position, boat.buoyance_point),
                    scaled(matrix.forward, 0.9 * bounds_max.y),
                ),
                scaled(matrix.right, side * 0.5),
            );
            if let Some(surface) = context.water.get_water_level(
                position.x,
                position.y,
                position.z,
                true,
                context.time_ms,
                context.wind,
            ) {
                position.z = surface.level;
            }
            context.particles.add_particle_coloured(
                PARTICLE_CAR_SPLASH,
                position,
                scaled(direction, 0.75),
                None,
                splash_size,
                splash_colour,
                context.general.get_random_number_in_range_i32(0, 30),
                context.general.get_random_number_in_range_i32(0, 90),
                1,
                0,
            );
            context.particles.add_particle_coloured(
                PARTICLE_BOAT_SPLASH,
                position,
                direction,
                None,
                boat_size,
                jet_colour,
                0,
                0,
                0,
                0,
            );
        }
    }
}
