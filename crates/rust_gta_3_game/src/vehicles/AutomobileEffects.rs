//! CAutomobile::PreRender calculations over canonical component inputs.
use super::{
    Automobile::*,
    Vehicle::{VehicleCore, VehicleDriveState, tWheelState},
};
use crate::{
    core::FileLoader::CFileLoader,
    entities::Entity::eEntityStatus,
    modelinfo::ModelInfo::ModelInfoSlot,
    renderer::{Particle::ParticleEmitter, Skidmarks::CSkidmarks},
};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

#[cfg(test)]
mod phase_tests {
    use super::*;

    #[test]
    fn simple_contacts_use_authored_local_xy_and_actor_wheel_height_atomically() {
        use crate::modelinfo::VehicleModelInfo::{AuthoredVehiclePosition, VehicleWheelFrames};
        let mut random = CGeneral::with_seed(7);
        let car = CAutomobile::new(
            crate::modelinfo::ModelIndices::MI_KURUMA as i32,
            2,
            0,
            &mut random,
            None,
            None,
            false,
        );
        let (mut core, _, _, _, _, _, mut state, _) = car.into_parts();
        core.physical
            .entity
            .set_position(CVector::new(10.0, 20.0, 30.0));
        core.physical.entity.set_heading(0.7);
        state.wheel_position = [-0.1, -0.2, -0.3, -0.4];
        let mut frames = VehicleWheelFrames {
            primary: [AuthoredVehiclePosition::Position(CVector::new(1.0, 2.0, 3.0)); 4],
            ..Default::default()
        };
        assert!(reconstruct_simple_wheel_contacts(
            &core, &mut state, &frames
        ));
        for wheel in 0..NUM_CAR_WHEELS {
            let expected = core.physical.entity.transform_point(CVector::new(
                1.0,
                2.0,
                1.5 * state.wheel_position[wheel],
            ));
            assert!((state.wheel_contact_points[wheel] - expected).magnitude_sqr() < 0.000001);
        }
        let saved = state.wheel_contact_points;
        frames.primary[2] = AuthoredVehiclePosition::Invalid;
        assert!(!reconstruct_simple_wheel_contacts(
            &core, &mut state, &frames
        ));
        assert_eq!(state.wheel_contact_points, saved);
    }

    #[test]
    fn middle_wheel_feedback_uses_rear_history_and_runs_outside_primary_status_gate() {
        use crate::renderer::{Particle::CParticle, ParticleMgr::cParticleSystemMgr};
        let mut random = CGeneral::with_seed(7);
        let car = CAutomobile::new(
            crate::modelinfo::ModelIndices::MI_BARRACKS as i32,
            2,
            0,
            &mut random,
            None,
            None,
            false,
        );
        let (mut core, _, _, _, _, _, mut state, _) = car.into_parts();
        core.physical.entity.status = eEntityStatus::STATUS_ABANDONED;
        state.wheel_state[CARWHEEL_REAR_LEFT] = tWheelState::WHEEL_STATE_FIXED;
        state.wheel_state[CARWHEEL_REAR_RIGHT] = tWheelState::WHEEL_STATE_SKIDDING;
        state.wheel_timer = [1.0; NUM_CAR_WHEELS];
        state.wheel_skidmark_bloody[CARWHEEL_REAR_LEFT] = true;
        state.wheel_contact_points[CARWHEEL_REAR_LEFT] = CVector::new(1.0, 2.0, 3.0);
        state.wheel_contact_points[CARWHEEL_REAR_RIGHT] = CVector::new(4.0, 5.0, 6.0);
        let mut particles = CParticle::default();
        let mut skidmarks = CSkidmarks::init();
        let systems = cParticleSystemMgr::default();
        let mut emitter = particles.emitter(&systems, CVector::ZERO);
        wheel_effects(
            &core,
            &mut state,
            &mut emitter,
            Some(&mut skidmarks),
            1,
            0.0,
            0,
            1000,
            &mut random,
            false,
        );
        middle_wheel_effects(
            &core,
            &mut state,
            &mut emitter,
            Some(&mut skidmarks),
            1,
            1000,
            false,
        );
        assert_eq!(particles.add_intents.len(), 2);
        let expected = state.wheel_contact_points[CARWHEEL_REAR_LEFT]
            + core.physical.entity.get_forward() * 2.0
            + CVector::new(0.0, 0.0, 0.25);
        assert_eq!(particles.add_intents[0].position, expected);
        assert!(
            skidmarks
                .skidmarks
                .iter()
                .any(|mark| mark.id == 13 && mark.was_updated && mark.is_bloody)
        );
        assert!(
            skidmarks
                .skidmarks
                .iter()
                .any(|mark| mark.id == 14 && mark.was_updated)
        );
    }

    #[test]
    fn front_skid_dirt_requires_a_skidding_rear_wheel() {
        use crate::renderer::{Particle::CParticle, ParticleMgr::cParticleSystemMgr};
        let mut random = CGeneral::with_seed(7);
        let car = CAutomobile::new(
            crate::modelinfo::ModelIndices::MI_KURUMA as i32,
            2,
            0,
            &mut random,
            None,
            None,
            false,
        );
        let (mut core, _, _, _, _, _, mut state, _) = car.into_parts();
        core.physical.entity.status = eEntityStatus::STATUS_PHYSICS;
        core.physical.move_speed = CVector::new(0.0, 0.1, 0.0);
        state.wheel_state[CARWHEEL_FRONT_LEFT] = tWheelState::WHEEL_STATE_SKIDDING;
        state.wheel_contact_surfaces[CARWHEEL_FRONT_LEFT] =
            crate::core::SurfaceTable::eSurfaceType::SURFACE_GRASS as u8;
        let mut particles = CParticle::default();
        wheel_effects(
            &core,
            &mut state,
            &mut particles.emitter(&cParticleSystemMgr::default(), CVector::ZERO),
            None,
            1,
            0.0,
            0,
            1000,
            &mut random,
            false,
        );
        assert!(particles.add_intents.is_empty());
        state.wheel_state[CARWHEEL_REAR_LEFT] = tWheelState::WHEEL_STATE_SKIDDING;
        wheel_effects(
            &core,
            &mut state,
            &mut particles.emitter(&cParticleSystemMgr::default(), CVector::ZERO),
            None,
            1,
            0.0,
            0,
            1000,
            &mut random,
            false,
        );
        assert_eq!(particles.add_intents.len(), 6);
    }

    #[test]
    fn native_special_models_and_abandoned_wheels_keep_distinct_gates() {
        use crate::modelinfo::ModelIndices::{MI_DODO, MI_KURUMA, MI_RCBANDIT};
        use crate::renderer::{Particle::CParticle, ParticleMgr::cParticleSystemMgr};
        for (model, status, count) in [
            (MI_DODO, eEntityStatus::STATUS_PHYSICS, 0),
            (MI_RCBANDIT, eEntityStatus::STATUS_ABANDONED, 1),
            (MI_KURUMA, eEntityStatus::STATUS_ABANDONED, 0),
            (MI_KURUMA, eEntityStatus::STATUS_PHYSICS, 1),
        ] {
            let mut random = CGeneral::with_seed(7);
            let car = CAutomobile::new(model as i32, 2, 0, &mut random, None, None, false);
            let (mut core, _, _, _, _, _, mut state, _) = car.into_parts();
            core.physical.entity.status = status;
            state.wheel_state[CARWHEEL_FRONT_LEFT] = tWheelState::WHEEL_STATE_FIXED;
            state.wheel_contact_points[CARWHEEL_FRONT_LEFT] = CVector::new(1.0, 2.0, 3.0);
            let mut particles = CParticle::default();
            wheel_effects(
                &core,
                &mut state,
                &mut particles.emitter(&cParticleSystemMgr::default(), CVector::ZERO),
                None,
                1,
                0.0,
                0,
                1000,
                &mut random,
                false,
            );
            assert_eq!(
                particles.add_intents.len(),
                count,
                "model={model} status={status:?}"
            );
            if model == MI_RCBANDIT {
                let request = &particles.add_intents[0];
                assert_eq!(request.size, 0.1);
                assert!((request.position.z - 3.05).abs() < 0.00001);
            }
        }
    }
}

/// Native PreRender over resident original frames and the actor's live wheel history.
pub fn prerender_wheel_effects(
    core: &VehicleCore,
    state: &mut AutomobileState,
    loader: &CFileLoader,
    particles: &mut ParticleEmitter<'_>,
    mut skidmarks: Option<&mut CSkidmarks>,
    vehicle_id: usize,
    wet_roads: f32,
    frame_counter: u32,
    time_ms: u32,
    general: &mut CGeneral,
    replay_playback: bool,
) {
    use crate::modelinfo::{
        ModelIndices::{MI_DODO, MI_RCBANDIT},
        VehicleModelInfo::AuthoredVehiclePosition,
    };
    let Some(ModelInfoSlot::Vehicle(model)) = usize::try_from(core.physical.entity.model_index)
        .ok()
        .and_then(|id| loader.model_info.get_model_info(id))
    else {
        bevy::log::warn_once!("automobile_wheel_prerender_required_model_missing");
        return;
    };
    let Some(descriptors) = model.authored_positions() else {
        bevy::log::warn_once!("automobile_wheel_prerender_waiting_resident_frames");
        return;
    };
    let special = matches!(
        core.physical.entity.model_index as usize,
        MI_DODO | MI_RCBANDIT
    );
    if !special
        && core.physical.entity.status == eEntityStatus::STATUS_SIMPLE
        && (model.wheel_id == -1
            || !reconstruct_simple_wheel_contacts(core, state, &descriptors.wheels))
    {
        bevy::log::warn_once!(
            "automobile_simple_contacts_required_wheel_frames_invalid_or_missing"
        );
        return;
    }
    wheel_effects(
        core,
        state,
        particles,
        skidmarks.as_deref_mut(),
        vehicle_id,
        wet_roads,
        frame_counter,
        time_ms,
        general,
        replay_playback,
    );
    if model.wheel_id != -1 {
        match descriptors.wheels.right_middle {
            AuthoredVehiclePosition::Position(_) => middle_wheel_effects(
                core,
                state,
                particles,
                skidmarks,
                vehicle_id,
                time_ms,
                replay_playback,
            ),
            AuthoredVehiclePosition::Invalid => {
                bevy::log::warn_once!("automobile_middle_wheel_frame_invalid")
            }
            AuthoredVehiclePosition::Absent => {}
        }
    }
}

pub fn reconstruct_simple_wheel_contacts(
    core: &VehicleCore,
    state: &mut AutomobileState,
    frames: &crate::modelinfo::VehicleModelInfo::VehicleWheelFrames,
) -> bool {
    let Some(mut positions) = frames.local_positions() else {
        return false;
    };
    for (wheel, position) in positions.iter_mut().enumerate() {
        position.z = 1.5 * state.wheel_position[wheel];
        *position = core.physical.entity.transform_point(*position);
        if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
            return false;
        }
    }
    state.wheel_contact_points = positions;
    state.wheel_contact_surfaces =
        [crate::core::SurfaceTable::eSurfaceType::SURFACE_DEFAULT as u8; NUM_CAR_WHEELS];
    true
}

fn middle_wheel_effects(
    core: &VehicleCore,
    state: &mut AutomobileState,
    particles: &mut ParticleEmitter<'_>,
    mut skidmarks: Option<&mut CSkidmarks>,
    vehicle_id: usize,
    time_ms: u32,
    replay_playback: bool,
) {
    struct MiddleWheel {
        rear: usize,
        skidmark_slot: usize,
    }
    const MIDDLE_WHEELS: [MiddleWheel; 2] = [
        MiddleWheel {
            rear: CARWHEEL_REAR_LEFT,
            skidmark_slot: 5,
        },
        MiddleWheel {
            rear: CARWHEEL_REAR_RIGHT,
            skidmark_slot: 6,
        },
    ];
    let forward = core.physical.entity.get_forward();
    let offset = forward * 2.0;
    for wheel in MIDDLE_WHEELS {
        if !matches!(
            state.wheel_state[wheel.rear],
            tWheelState::WHEEL_STATE_SPINNING
                | tWheelState::WHEEL_STATE_SKIDDING
                | tWheelState::WHEEL_STATE_FIXED
        ) {
            continue;
        }
        let point = state.wheel_contact_points[wheel.rear] + offset;
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_RUBBER_SMOKE,
            point + CVector::new(0.0, 0.0, 0.25),
            CVector::ZERO,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
        if state.wheel_timer[wheel.rear] > 0.0
            && let Some(skidmarks) = skidmarks.as_deref_mut()
        {
            skidmarks.register_one(
                ((vehicle_id as u32 as usize) << 3) | wheel.skidmark_slot,
                point,
                forward.x,
                forward.y,
                &mut state.wheel_skidmark_muddy[wheel.rear],
                &mut state.wheel_skidmark_bloody[wheel.rear],
                time_ms,
                replay_playback,
            );
        }
    }
}

/// Primary wheel feedback, after native simple contact reconstruction.
pub fn wheel_effects(
    core: &VehicleCore,
    state: &mut AutomobileState,
    particles: &mut ParticleEmitter<'_>,
    mut skidmarks: Option<&mut CSkidmarks>,
    vehicle_id: usize,
    wet_roads: f32,
    frame_counter: u32,
    time_ms: u32,
    general: &mut CGeneral,
    replay_playback: bool,
) {
    use crate::modelinfo::ModelIndices::{MI_DODO, MI_RCBANDIT};
    if core.physical.entity.model_index as usize == MI_DODO {
        return;
    }
    if core.physical.entity.model_index as usize == MI_RCBANDIT {
        for wheel in 0..NUM_CAR_WHEELS {
            if matches!(
                state.wheel_state[wheel],
                tWheelState::WHEEL_STATE_SPINNING
                    | tWheelState::WHEEL_STATE_SKIDDING
                    | tWheelState::WHEEL_STATE_FIXED
            ) {
                particles.add_particle(
                    crate::renderer::ParticleType::PARTICLE_RUBBER_SMOKE,
                    state.wheel_contact_points[wheel] + CVector::new(0.0, 0.0, 0.05),
                    CVector::ZERO,
                    None,
                    0.1,
                    0,
                    0,
                    0,
                    0,
                );
            }
        }
        return;
    }
    if !matches!(
        core.physical.entity.status,
        eEntityStatus::STATUS_SIMPLE
            | eEntityStatus::STATUS_PHYSICS
            | eEntityStatus::STATUS_PLAYER
            | eEntityStatus::STATUS_PLAYER_PLAYBACKFROMBUFFER
    ) {
        return;
    }
    let rear_skidding = [CARWHEEL_REAR_LEFT, CARWHEEL_REAR_RIGHT]
        .into_iter()
        .any(|wheel| state.wheel_state[wheel] == tWheelState::WHEEL_STATE_SKIDDING);
    let forward = core.physical.entity.get_forward();
    let fwd_speed = core.physical.move_speed.dot(forward) * 180.0;
    let draw_particles = fwd_speed.abs() < 90.0;
    for wheel in 0..NUM_CAR_WHEELS {
        let register = match state.wheel_state[wheel] {
            tWheelState::WHEEL_STATE_SPINNING | tWheelState::WHEEL_STATE_FIXED => true,
            tWheelState::WHEEL_STATE_SKIDDING => {
                matches!(wheel, CARWHEEL_REAR_LEFT | CARWHEEL_REAR_RIGHT) || rear_skidding
            }
            tWheelState::WHEEL_STATE_NORMAL => state.wheel_skidmark_bloody[wheel],
        };
        let feedback = match state.wheel_state[wheel] {
            tWheelState::WHEEL_STATE_SPINNING | tWheelState::WHEEL_STATE_FIXED => true,
            tWheelState::WHEEL_STATE_SKIDDING => {
                matches!(wheel, CARWHEEL_REAR_LEFT | CARWHEEL_REAR_RIGHT) || rear_skidding
            }
            tWheelState::WHEEL_STATE_NORMAL => false,
        };
        if feedback {
            let permits_burning = wheel_dirt_and_water(
                core,
                state,
                wheel,
                draw_particles,
                wet_roads,
                frame_counter,
                particles,
                general,
            );
            let point = state.wheel_contact_points[wheel];
            let smoke_position = CVector::new(point.x, point.y, point.z + 0.25);
            if state.wheel_state[wheel] == tWheelState::WHEEL_STATE_SPINNING && permits_burning {
                for direction in [CVector::ZERO, CVector::new(0.0, 0.0, 0.05)] {
                    particles.add_particle(
                        crate::renderer::ParticleType::PARTICLE_BURNINGRUBBER_SMOKE,
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
            }
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_RUBBER_SMOKE,
                smoke_position,
                CVector::ZERO,
                None,
                0.0,
                0,
                0,
                0,
                0,
            );
        } else if state.wheel_state[wheel] == tWheelState::WHEEL_STATE_NORMAL
            && fwd_speed.abs() > 5.0
        {
            let _ = wheel_dirt_and_water(
                core,
                state,
                wheel,
                draw_particles,
                wet_roads,
                frame_counter,
                particles,
                general,
            );
        }
        if !register || state.wheel_timer[wheel] <= 0.0 {
            continue;
        }
        if let Some(skidmarks) = skidmarks.as_deref_mut() {
            skidmarks.register_one(
                ((vehicle_id as u32 as usize) << 3) | wheel,
                state.wheel_contact_points[wheel],
                forward.x,
                forward.y,
                &mut state.wheel_skidmark_muddy[wheel],
                &mut state.wheel_skidmark_bloody[wheel],
                time_ms,
                replay_playback,
            );
        }
    }
}

pub(super) fn wheel_surface_water_effects(
    core: &VehicleCore,
    state: &AutomobileState,
    wheel: usize,
    particles: Option<&mut ParticleEmitter<'_>>,
    general: &mut CGeneral,
) -> Option<f32> {
    use crate::{
        core::SurfaceTable::eSurfaceType::SURFACE_WATER,
        renderer::{Particle::ParticleColour, ParticleType::*},
    };

    if state.suspension_spring_ratio[wheel] >= 1.0
        || state.wheel_contact_surfaces[wheel] != SURFACE_WATER as u8
    {
        return None;
    }

    let position = core.physical.entity.get_position();
    let up = core.physical.entity.get_up();
    let point = state.wheel_contact_points[wheel];
    let relative_point = CVector::new(
        point.x + 0.3 * up.x - position.x,
        point.y + 0.3 * up.y - position.y,
        point.z + 0.3 * up.z - position.z,
    );
    let mut speed = core.physical.get_speed_at_point(relative_point);
    speed.z = 0.0;
    let speed_sqr = speed.magnitude_sqr();
    if speed_sqr <= 0.05_f32.powi(2) {
        return None;
    }

    if !speed_sqr.is_finite() {
        bevy::log::warn_once!("automobile_water_contact_speed_invalid");
        return None;
    }

    let speed_magnitude = speed_sqr.sqrt();
    let Some(particles) = particles else {
        bevy::log::warn_once!("automobile_buoyancy_particle_inputs_missing");
        return Some(speed_magnitude);
    };
    let size = ((if speed_magnitude < 0.15 { 0.25 } else { 0.75 }) * speed_magnitude).min(0.6);
    let vehicle_right = core.physical.entity.get_right();
    let right = CVector::new(
        0.2 * speed_magnitude * vehicle_right.x + 0.2 * speed.x,
        0.2 * speed_magnitude * vehicle_right.y + 0.2 * speed.y,
        0.2 * speed_magnitude * vehicle_right.z + 0.2 * speed.z,
    );
    let world_position = CVector::new(
        relative_point.x + position.x,
        relative_point.y + position.y,
        relative_point.z + position.z,
    );

    particles.add_particle_coloured(
        PARTICLE_PED_SPLASH,
        world_position,
        CVector::new(-0.5 * right.x, -0.5 * right.y, -0.5 * right.z),
        None,
        size,
        ParticleColour {
            red: 155,
            green: 155,
            blue: 185,
            alpha: 196,
        },
        general.get_random_number_in_range_f32(0.0, 10.0) as i32,
        general.get_random_number_in_range_f32(0.0, 90.0) as i32,
        1,
        0,
    );
    particles.add_particle_coloured(
        PARTICLE_RUBBER_SMOKE,
        world_position,
        CVector::new(-0.6 * right.x, -0.6 * right.y, -0.6 * right.z),
        None,
        size,
        ParticleColour {
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
        },
        0,
        0,
        0,
        0,
    );
    Some(speed_magnitude)
}

/// CAutomobile::ProcessBuoyancy uses current native timecycle colours and the shared RNG stream.
pub(super) fn deep_water_splash(
    core: &VehicleCore,
    timecycle: &crate::renderer::Timecycle::CTimeCycle,
    objects: &mut crate::objects::ParticleObject::CParticleObjectRuntime,
    general: &mut CGeneral,
    time_ms: u32,
) {
    use crate::objects::ParticleObject::{CRGBA, eParticleObjectType};
    let channel = |index: usize| {
        ((0.5 * timecycle.current.directional[index] + timecycle.current.ambient[index])
            * 0.45
            * 255.0)
            .clamp(0.0, 255.0) as u8
    };
    let colour = CRGBA::new(
        channel(0),
        channel(1),
        channel(2),
        (general.get_random_number_in_range_i32(0, 32) + 128) as u8,
    );
    let target = CVector::new(0.0, 0.0, general.get_random_number_in_range_f32(0.15, 0.3));
    objects.add_object(
        eParticleObjectType::POBJECT_CAR_WATER_SPLASH as i32,
        core.physical.entity.get_position(),
        target,
        0.0,
        75,
        colour,
        true,
        time_ms,
    );
}

pub fn wheel_dirt_and_water(
    core: &VehicleCore,
    state: &AutomobileState,
    wheel: usize,
    below_effect_speed: bool,
    wet_roads: f32,
    frame_counter: u32,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
) -> bool {
    use crate::{
        core::SurfaceTable::eSurfaceType::{SURFACE_GRASS, SURFACE_GRAVEL, SURFACE_MUD_DRY},
        renderer::{Particle::ParticleColour, ParticleType::*},
    };

    if !below_effect_speed {
        return false;
    }
    let point = state.wheel_contact_points[wheel];
    let surface = state.wheel_contact_surfaces[wheel];
    let move_speed = core.physical.move_speed;
    struct WheelDirt {
        colour: ParticleColour,
        max_size: f32,
        permits_burning: bool,
    }
    let dirt = if surface == SURFACE_GRASS as u8 {
        Some(WheelDirt {
            colour: ParticleColour {
                red: 8,
                green: 24,
                blue: 8,
                alpha: 255,
            },
            max_size: 0.1,
            permits_burning: false,
        })
    } else if surface == SURFACE_GRAVEL as u8 {
        Some(WheelDirt {
            colour: ParticleColour {
                red: 64,
                green: 64,
                blue: 64,
                alpha: 255,
            },
            max_size: 0.06,
            permits_burning: true,
        })
    } else if surface == SURFACE_MUD_DRY as u8 {
        Some(WheelDirt {
            colour: ParticleColour {
                red: 64,
                green: 32,
                blue: 16,
                alpha: 255,
            },
            max_size: 0.06,
            permits_burning: false,
        })
    } else {
        None
    };
    if let Some(dirt) = dirt {
        for _ in 0..4 {
            particles.add_particle_coloured(
                PARTICLE_WHEEL_DIRT,
                point,
                CVector::new(
                    -0.05 * move_speed.x,
                    -0.05 * move_speed.y,
                    general.get_random_number_in_range_f32(0.03, 0.06),
                ),
                None,
                general.get_random_number_in_range_f32(0.02, dirt.max_size),
                dirt.colour,
                0,
                0,
                0,
                0,
            );
        }
        return dirt.permits_burning;
    }
    if wet_roads > 0.01 && frame_counter & 1 != 0 {
        particles.add_particle_coloured(
            PARTICLE_WATERSPRAY,
            CVector::new(point.x, point.y, point.z + 0.5),
            CVector::new(0.0, 0.0, 1.0),
            None,
            general.get_random_number_in_range_f32(0.1, 0.5),
            ParticleColour {
                red: 48,
                green: 48,
                blue: 64,
                alpha: 0,
            },
            0,
            0,
            0,
            0,
        );
        return false;
    }
    true
}

pub fn exhaust_effects(
    core: &VehicleCore,
    drive: &VehicleDriveState,
    handling_flags: u32,
    file_loader: &CFileLoader,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
) {
    if !drive.engine_on {
        return;
    }

    // C++ uses DotProduct(...) * 180 and intentionally does not take the
    // absolute value here; reverse motion therefore still emits smoke.
    let fwd_speed = core
        .physical
        .move_speed
        .dot(core.physical.entity.get_forward())
        * 180.0;
    if fwd_speed >= 90.0 {
        return;
    }

    let Ok(model_id) = usize::try_from(core.physical.entity.model_index) else {
        bevy::log::warn_once!("automobile_exhaust_invalid_original_model_identity");
        return;
    };
    let Some(ModelInfoSlot::Vehicle(model)) = file_loader.model_info.get_model_info(model_id)
    else {
        bevy::log::warn_once!("automobile_exhaust_missing_original_model");
        return;
    };
    let Some(positions) = model.authored_positions() else {
        bevy::log::warn_once!("automobile_exhaust_waiting_for_original_clump");
        return;
    };
    let exhaust_position = match positions.exhaust {
        crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Absent => return,
        crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Position(position) => position,
        crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition::Invalid => {
            bevy::log::warn_once!("automobile_exhaust_invalid_original_frame_hierarchy");
            return;
        }
    };
    if exhaust_position == CVector::ZERO {
        return;
    }

    let position_one = core.physical.entity.transform_point(exhaust_position);
    let position_two = core.physical.entity.transform_point(CVector::new(
        -exhaust_position.x,
        exhaust_position.y,
        exhaust_position.z,
    ));

    let direction = if fwd_speed < 10.0 {
        let steer_forward = core.physical.entity.transform_vector(CVector::new(
            -drive.steer_angle.sin(),
            drive.steer_angle.cos(),
            0.0,
        ));
        let random = general.get_random_number_in_range_f32(-0.06, -0.03);
        CVector::new(steer_forward.x * random, steer_forward.y * random, 0.0)
    } else {
        let speed = core.physical.move_speed;
        CVector::new(speed.x, speed.y, 0.0)
    };

    // The C++ loop is `for(i = 0; i <= 4.0f*m_fGasPedal; ++i)`; truncating
    // toward zero preserves its integer conversion for reverse gas too.
    let emission_count = (4.0 * drive.gas_pedal).trunc() as i32;
    if emission_count < 0 {
        return;
    }
    for _ in 0..=emission_count {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_EXHAUST_FUMES,
            position_one,
            direction,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
        if handling_flags & HANDLING_DBL_EXHAUST != 0 {
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_EXHAUST_FUMES,
                position_two,
                direction,
                None,
                0.0,
                0,
                0,
                0,
                0,
            );
        }
    }
}
