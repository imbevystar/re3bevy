//! re3 reference: references/re-GTA/src/renderer/Particle.cpp
//! re3 reference: references/re-GTA/src/renderer/Particle.h
//!
//! `CParticle::Initialise` state.

use bevy::prelude::*;
use rust_gta_3_core::core::Coordinates::CVector;
use rust_gta_3_core::core::General::CGeneral;
use std::collections::HashMap;

use super::ParticleType::tParticleType;
use rust_gta_3_core::core::Timer::CTimer;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleCreationSet { Inputs, AfterScripts, AfterGameLogic, BeforeRender }

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeParticleRenderSet { Consume }

pub struct ParticleCreationInputPlugin;
impl Plugin for ParticleCreationInputPlugin {
    fn build(&self, app: &mut App) {
        use rust_gta_3_core::core::{Schedules::{GtaFrameSet, GtaProcessSet}, GameState::GtaAppState};
        app.init_resource::<ParticleCreationPolicy>()
            .add_systems(Update, sync_particle_residency.before(ParticleCreationSet::Inputs))
            .add_systems(Update, sync_particle_creation_inputs.in_set(ParticleCreationSet::Inputs)
                .after(GtaFrameSet::Timer).before(GtaFrameSet::GameProcess))
            .add_systems(Update, (
                sync_particle_creation_inputs.in_set(ParticleCreationSet::AfterScripts)
                    .after(GtaProcessSet::Script).before(GtaProcessSet::Collision),
                sync_particle_creation_inputs.in_set(ParticleCreationSet::AfterGameLogic)
                    .after(GtaProcessSet::GameLogic).before(GtaProcessSet::Bridge),
                sync_particle_creation_inputs.in_set(ParticleCreationSet::BeforeRender)
                    .after(GtaProcessSet::Audio).before(GtaProcessSet::RenderPrep),
            ).in_set(GtaFrameSet::GameProcess).run_if(in_state(GtaAppState::InGame)));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleSimulationDomain { NativeGame, IsolatedDeveloperScene }

/// Derived membership only; Bevy owns the particle entities and their poses.
pub fn sync_particle_residency(
    particles: Option<ResMut<CParticle>>,
    live: Query<(Entity, &super::BevyRender::GtaParticleBillboard)>,
    added: Query<(Entity, &super::BevyRender::GtaParticleBillboard), Added<super::BevyRender::GtaParticleBillboard>>,
    mut removed: RemovedComponents<super::BevyRender::GtaParticleBillboard>,
) {
    let Some(mut particles) = particles else { removed.read().for_each(drop); return; };
    for entity in removed.read() { particles.retire_live_particle(entity); }
    if particles.is_added() {
        particles.live_particles.clear();
        for (entity, particle) in &live { particles.register_live_particle(entity, particle.tracking_id); }
    } else {
        for (entity, particle) in &added { particles.register_live_particle(entity, particle.tracking_id); }
    }
}

/// Immutable stage inputs projected from CTimer; this never advances time.
#[derive(Debug, Clone, Copy)]
struct ParticleCreationInputs { frame_counter: u32, paused: bool, interval_mask: u32 }

pub fn sync_particle_creation_inputs(
    particles: Option<ResMut<CParticle>>, timer: Option<Res<CTimer>>,
    policy: Res<ParticleCreationPolicy>,
) {
    // Current timer/policy writer phases have explicit native producer barriers.
    // A writer that emits in the same function must update its borrowed creation
    // context immediately after the mutation, before calling AddParticle.
    let Some(mut particles) = particles else { return; };
    if let Some(timer) = timer {
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::NativeGame);
    } else {
        particles.invalidate_creation_inputs();
    }
}

/// Native PC_PARTICLE smoke group shared by creation and screen-size culling.
pub(crate) fn is_pc_smoke_particle(particle_type: tParticleType) -> bool {
    use super::ParticleType::*;
    matches!(particle_type, PARTICLE_ENGINE_SMOKE | PARTICLE_ENGINE_SMOKE2
        | PARTICLE_ENGINE_STEAM | PARTICLE_CARFLAME_SMOKE | PARTICLE_RUBBER_SMOKE
        | PARTICLE_BURNINGRUBBER_SMOKE | PARTICLE_EXHAUST_FUMES | PARTICLE_CARCOLLISION_DUST)
}

/// App-lifetime `nParticleCreationInterval`; actor/config restarts do not own it.
#[derive(Debug, Clone, Copy, Resource)]
pub struct ParticleCreationPolicy {
    pub interval_mask: u32,
}
impl Default for ParticleCreationPolicy {
    fn default() -> Self { Self { interval_mask: 1 } }
}
impl ParticleCreationPolicy {
    /// CParticle::AddParticle rejects pause first, then applies the native bitmask.
    /// Inputs must come from canonical CTimer and the explicit simulation domain.
    pub fn allows(&self, particle_type: tParticleType, frame_counter: u32, paused: bool) -> bool {
        !paused && (!is_pc_smoke_particle(particle_type) || self.interval_mask & frame_counter == 0)
    }
}

pub const MAX_PARTICLES_ON_SCREEN: usize = 1000;

#[cfg(test)]
mod creation_policy_tests {
    use super::*;
    use crate::renderer::ParticleType::*;

    #[test]
    fn borrowed_range_admission_rejects_before_reservation_and_id_allocation() {
        let mut particles = CParticle::initialise();
        let mut systems = super::super::ParticleMgr::cParticleSystemMgr::default();
        systems.particle_mut(PARTICLE_EXPLOSION_MEDIUM).unwrap().create_range = 4.0;
        let origin = CVector::new(10.0, 20.0, 30.0);
        let outside = origin + CVector::new(0.0, 0.0, 3.0);
        {
            let mut emitter = particles.emitter(&systems, origin);
            emitter.add_particle(PARTICLE_EXPLOSION_MEDIUM, outside, CVector::ZERO,
                None, 1.0, 0, 0, 0, 0);
            emitter.add_particle_coloured(PARTICLE_EXPLOSION_MEDIUM, outside,
                CVector::ZERO, None, 1.0, ParticleColour::default(), 0, 0, 0, 0);
            assert_eq!(emitter.add_tracked_particle(PARTICLE_EXPLOSION_MEDIUM,
                outside, CVector::ZERO, None, 1.0, 0, 0, 0, 0), None);
        }
        assert!(particles.add_intents.is_empty());
        assert!(particles.has_creation_capacity());
        let boundary = origin + CVector::new(0.0, 0.0, 2.0);
        assert_eq!(particles.emitter(&systems, origin).add_tracked_particle(
            PARTICLE_EXPLOSION_MEDIUM, boundary, CVector::ZERO, None, 1.0,
            0, 0, 0, 0), Some(1));
        assert!(particles.add_intents[0].range_admitted);
        systems.particle_mut(PARTICLE_EXPLOSION_MEDIUM).unwrap().create_range = 0.0;
        assert_eq!(particles.emitter(&systems, origin).add_tracked_particle(
            PARTICLE_EXPLOSION_MEDIUM, outside, CVector::ZERO, None, 1.0,
            0, 0, 0, 0), Some(2));
    }

    #[test]
    fn live_plus_pending_capacity_rejects_without_consuming_ids_and_retires_members() {
        let mut world = World::new();
        let mut particles = CParticle::initialise();
        let members: Vec<_> = (0..MAX_PARTICLES_ON_SCREEN).map(|_| world.spawn_empty().id()).collect();
        for entity in &members { particles.register_live_particle(*entity, None); }
        particles.add_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0);
        assert!(particles.add_intents.is_empty());
        assert_eq!(particles.add_tracked_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), None);
        particles.reload_config_for_restart();
        assert_eq!(particles.live_particle_count(), MAX_PARTICLES_ON_SCREEN,
            "config reset must not fabricate retirement of live ECS entities");
        particles.retire_live_particle(members[0]);
        assert_eq!(particles.add_tracked_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), Some(1));
        assert_eq!(particles.add_tracked_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), None);
        let (requests, _) = particles.drain_frame_intents();
        let new_entity = world.spawn_empty().id();
        particles.register_live_particle(new_entity, requests[0].tracking_id);
        particles.set_tracked_particle_position(1, CVector::ZERO);
        assert_eq!(particles.live_particle_count(), MAX_PARTICLES_ON_SCREEN);
        particles.retire_live_particle(new_entity);
        assert!(particles.tracked_particle_position(1).is_none());
        assert_eq!(particles.add_tracked_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), Some(2));
    }

    #[test]
    fn every_native_pc_smoke_type_uses_the_same_bitmask_and_pause_gate() {
        let policy = ParticleCreationPolicy::default();
        for kind in [PARTICLE_ENGINE_SMOKE, PARTICLE_ENGINE_SMOKE2, PARTICLE_ENGINE_STEAM,
            PARTICLE_CARFLAME_SMOKE, PARTICLE_RUBBER_SMOKE, PARTICLE_BURNINGRUBBER_SMOKE,
            PARTICLE_EXHAUST_FUMES, PARTICLE_CARCOLLISION_DUST] {
            assert!(policy.allows(kind, 0, false));
            assert!(!policy.allows(kind, 1, false));
            assert!(policy.allows(kind, 2, false));
            assert!(!policy.allows(kind, 2, true));
        }
        assert!(policy.allows(PARTICLE_PED_SPLASH, 1, false));
        assert!(!policy.allows(PARTICLE_PED_SPLASH, 0, true));
    }

    #[test]
    fn interval_is_a_mask_not_a_modulo_and_particle_restart_does_not_reset_it() {
        let mut world = World::new();
        world.init_resource::<ParticleCreationPolicy>();
        world.resource_mut::<ParticleCreationPolicy>().interval_mask = 2;
        let policy = *world.resource::<ParticleCreationPolicy>();
        for frame in [0, 1, 4, 5] { assert!(policy.allows(PARTICLE_EXHAUST_FUMES, frame, false)); }
        for frame in [2, 3, 6, 7] { assert!(!policy.allows(PARTICLE_EXHAUST_FUMES, frame, false)); }
        world.insert_resource(CParticle::initialise());
        world.resource_mut::<CParticle>().reload_config_for_restart();
        world.init_resource::<ParticleCreationPolicy>();
        assert_eq!(world.resource::<ParticleCreationPolicy>().interval_mask, 2);
        let unmasked = ParticleCreationPolicy { interval_mask: 0 };
        assert!(unmasked.allows(PARTICLE_EXHAUST_FUMES, 7, false));
        assert!(!unmasked.allows(PARTICLE_EXHAUST_FUMES, 7, true));
    }

    #[test]
    fn creation_entry_points_reject_before_tracking_ids_and_preserve_native_pause() {
        let mut particles = CParticle::initialise();
        let mut timer = CTimer::initialise();
        let policy = ParticleCreationPolicy::default();
        timer.set_frame_counter(1);
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::NativeGame);
        particles.add_particle(PARTICLE_EXHAUST_FUMES, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0);
        particles.add_particle_coloured(PARTICLE_EXHAUST_FUMES, CVector::ZERO, CVector::ZERO, None, 1.0,
            ParticleColour::default(), 0, 0, 0, 0);
        assert_eq!(particles.add_tracked_particle(PARTICLE_EXHAUST_FUMES, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), None);
        assert!(particles.add_intents.is_empty());
        timer.set_frame_counter(2);
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::NativeGame);
        assert_eq!(particles.add_tracked_particle(PARTICLE_EXHAUST_FUMES, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), Some(1));
        particles.drain_frame_intents();
        timer.set_code_pause(true);
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::NativeGame);
        assert!(!particles.can_create(PARTICLE_PED_SPLASH));
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::IsolatedDeveloperScene);
        assert!(particles.can_create(PARTICLE_EXHAUST_FUMES));
        timer.start_user_pause();
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::IsolatedDeveloperScene);
        assert!(!particles.can_create(PARTICLE_PED_SPLASH));
        particles.invalidate_creation_inputs();
        assert_eq!(particles.add_tracked_particle(PARTICLE_PED_SPLASH, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), None);
        timer.end_user_pause();
        particles.set_creation_inputs(&timer, &policy, ParticleSimulationDomain::IsolatedDeveloperScene);
        assert_eq!(particles.add_tracked_particle(PARTICLE_EXHAUST_FUMES, CVector::ZERO, CVector::ZERO, None, 1.0, 0, 0, 0, 0), Some(2));
    }

    #[test]
    fn native_phase_barriers_observe_same_update_pause_resume_and_policy_changes() {
        use rust_gta_3_core::core::{Schedules::{GtaFrameSet, GtaProcessSet}, GameState::GtaAppState};
        #[derive(Default, Resource)]
        struct Observed { collision_rejected: bool, bridge_id: Option<u64>, render_rejected: bool }
        fn emit(particles: &mut CParticle, kind: tParticleType) -> Option<u64> {
            particles.add_tracked_particle(kind, CVector::ZERO,
                CVector::ZERO, None, 1.0, 0, 0, 0, 0)
        }
        let mut app = App::new();
        let mut timer = CTimer::initialise();
        timer.set_frame_counter(3);
        app.insert_resource(timer).insert_resource(CParticle::initialise())
            .insert_resource(State::new(GtaAppState::InGame)).init_resource::<Observed>()
            .add_plugins(ParticleCreationInputPlugin)
            .configure_sets(Update, (GtaFrameSet::Timer, GtaFrameSet::GameProcess).chain())
            .configure_sets(Update, (GtaProcessSet::Script, GtaProcessSet::Collision,
                GtaProcessSet::GameLogic, GtaProcessSet::Bridge, GtaProcessSet::Audio,
                GtaProcessSet::RenderPrep).chain().in_set(GtaFrameSet::GameProcess))
            .add_systems(Update, (|mut timer: ResMut<CTimer>| timer.set_code_pause(true))
                .in_set(GtaProcessSet::Script))
            .add_systems(Update, (|mut particles: ResMut<CParticle>, mut result: ResMut<Observed>| {
                result.collision_rejected = emit(&mut particles, PARTICLE_PED_SPLASH).is_none();
            }).in_set(GtaProcessSet::Collision))
            .add_systems(Update, (|mut timer: ResMut<CTimer>, mut policy: ResMut<ParticleCreationPolicy>| {
                timer.set_code_pause(false);
                policy.interval_mask = 0;
            }).in_set(GtaProcessSet::GameLogic))
            .add_systems(Update, (|mut particles: ResMut<CParticle>, mut result: ResMut<Observed>| {
                result.bridge_id = emit(&mut particles, PARTICLE_EXHAUST_FUMES);
            }).in_set(GtaProcessSet::Bridge))
            .add_systems(Update, (|mut timer: ResMut<CTimer>| timer.start_user_pause())
                .in_set(GtaProcessSet::Audio))
            .add_systems(Update, (|mut particles: ResMut<CParticle>, mut result: ResMut<Observed>| {
                result.render_rejected = emit(&mut particles, PARTICLE_PED_SPLASH).is_none();
            }).in_set(GtaProcessSet::RenderPrep));
        app.update();
        let result = app.world().resource::<Observed>();
        assert!(result.collision_rejected && result.render_rejected);
        assert_eq!(result.bridge_id, Some(1));
        assert_eq!(app.world().resource::<CParticle>().add_intents.len(), 1);
    }
}
pub const RAND_TABLE_SIZE: usize = 20;
pub const SIN_COS_TABLE_SIZE: usize = 1024;

pub const PARTICLE_TEXTURE_NAMES: &[&str] = &[
    "smoke1",
    "smoke2",
    "smoke3",
    "smoke4",
    "smoke5",
    "smokeII_1",
    "smokeII_2",
    "smokeII_3",
    "smokeII_4",
    "smokeII_5",
    "rubber1",
    "rubber2",
    "rubber3",
    "rubber4",
    "rubber5",
    "splash1",
    "splash2",
    "splash3",
    "splash4",
    "splash5",
    "waterspray1",
    "waterspray2",
    "waterspray3",
    "explo01",
    "explo02",
    "explo03",
    "explo04",
    "explo05",
    "explo06",
    "gunflash1",
    "gunflash2",
    "gunflash3",
    "gunflash4",
    "raindrop1",
    "raindrop2",
    "raindrop3",
    "raindrop4",
    "splash_up1",
    "splash_up2",
    "birdf_01",
    "birdf_02",
    "birdf_03",
    "birdf_04",
    "cardebris_01",
    "cardebris_02",
    "cardebris_03",
    "cardebris_04",
    "carsplash_01",
    "carsplash_02",
    "carsplash_03",
    "carsplash_04",
    "flame1",
    "flame5",
    "rainsmall",
    "blood",
    "gameleaf01_64",
    "cloud3",
    "cloudmasked",
    "bloodsplat2",
    "gunge",
    "collisionsmoke",
    "bullethitsmoke",
    "gunshell",
];

#[derive(Debug, Clone, Resource)]
pub struct CParticle {
    creation_inputs: Option<ParticleCreationInputs>,
    pub config_reloaded: bool,
    pub particle_objects_initialised: bool,
    live_particles: HashMap<Entity, Option<u64>>,
    pub rand_table: [f32; RAND_TABLE_SIZE],
    pub sin_table_len: usize,
    pub cos_table_len: usize,
    pub texture_names: &'static [&'static str],
    pub renderware_textures_loaded: bool,
    pub add_intents: Vec<ParticleAddIntent>,
    pub remove_system_intents: Vec<tParticleType>,
    tracked_positions: HashMap<u64, CVector>,
    next_tracking_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleColour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleAddIntent {
    pub particle_type: tParticleType,
    pub position: CVector,
    pub direction: CVector,
    pub entity: Option<i32>,
    pub size: f32,
    pub colour: ParticleColour,
    pub rotation_speed: i32,
    pub rotation: i32,
    pub current_frame: i32,
    pub lifespan: i32,
    pub tracking_id: Option<u64>,
    /// Native range admission already ran against the emitting domain's camera.
    pub range_admitted: bool,
}

/// Request-time access to the single parsed particle table and camera input.
/// This view owns no live state, configuration copy or camera cache.
pub struct ParticleEmitter<'a> {
    particles: &'a mut CParticle,
    systems: Option<&'a super::ParticleMgr::cParticleSystemMgr>,
    camera_position: Option<CVector>,
}

impl std::ops::Deref for ParticleEmitter<'_> {
    type Target = CParticle;
    fn deref(&self) -> &Self::Target { self.particles }
}

impl ParticleEmitter<'_> {
    fn admits(&self, kind: tParticleType, position: CVector) -> bool {
        if !self.particles.can_create(kind) || !self.particles.has_creation_capacity() {
            return false;
        }
        let Some(system) = self.systems.and_then(|systems| systems.particle(kind)) else {
            warn_once!("particle_creation_missing_original_system kind={}", kind);
            return false;
        };
        // CParticle::AddParticle: zero disables the gate; equality is admitted.
        if system.create_range == 0.0 {
            return true;
        }
        let Some(camera_position) = self.camera_position else {
            warn_once!("particle_creation_waiting_original_camera kind={}", kind);
            return false;
        };
        !(system.create_range < (camera_position - position).magnitude_sqr())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_particle(&mut self, kind: tParticleType, position: CVector,
        direction: CVector, entity: Option<i32>, size: f32, rotation_speed: i32,
        rotation: i32, current_frame: i32, lifespan: i32) {
        self.add_particle_coloured(kind, position, direction, entity, size,
            ParticleColour::default(), rotation_speed, rotation, current_frame, lifespan);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_particle_coloured(&mut self, kind: tParticleType, position: CVector,
        direction: CVector, entity: Option<i32>, size: f32, colour: ParticleColour,
        rotation_speed: i32, rotation: i32, current_frame: i32, lifespan: i32) {
        if self.admits(kind, position) {
            self.particles.add_particle_coloured(kind, position, direction, entity,
                size, colour, rotation_speed, rotation, current_frame, lifespan);
            self.particles.add_intents.last_mut().expect("admitted request").range_admitted = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_tracked_particle(&mut self, kind: tParticleType, position: CVector,
        direction: CVector, entity: Option<i32>, size: f32, rotation_speed: i32,
        rotation: i32, current_frame: i32, lifespan: i32) -> Option<u64> {
        if !self.admits(kind, position) { return None; }
        let id = self.particles.add_tracked_particle(kind, position, direction, entity,
            size, rotation_speed, rotation, current_frame, lifespan);
        if id.is_some() {
            self.particles.add_intents.last_mut().expect("admitted tracked request").range_admitted = true;
        }
        id
    }
    /// re3 `CParticle::AddJetExplosion`.
    pub fn add_jet_explosion(
        &mut self,
        position: CVector,
        mut power: f32,
        size: f32,
        general: &mut CGeneral,
    ) {
        use super::ParticleType::PARTICLE_EXPLOSION_MFAST;

        power = power.max(1.0);
        let step = CVector::new(
            general.get_random_number_in_range_f32(-0.4, 0.4) * 2.0,
            general.get_random_number_in_range_f32(-0.4, 0.4) * 2.0,
            general.get_random_number_in_range_f32(0.1, 0.3) * 2.0,
        );
        let colour = ParticleColour {
            red: 240,
            green: 240,
            blue: 240,
            alpha: 255,
        };
        let mut step_pos = position;
        for _ in 0..(power * 4.0) as i32 {
            self.add_particle_coloured(
                PARTICLE_EXPLOSION_MFAST,
                step_pos,
                CVector::new(
                    general.get_random_number_in_range_f32(-0.02, 0.02),
                    general.get_random_number_in_range_f32(-0.02, 0.02),
                    general.get_random_number_in_range_f32(-0.02, 0.0),
                ),
                None,
                size,
                colour,
                0,
                0,
                0,
                0,
            );
            for _ in 0..2 {
                self.add_particle_coloured(
                    PARTICLE_EXPLOSION_MFAST,
                    step_pos,
                    CVector::new(
                        general.get_random_number_in_range_f32(-0.04, 0.04),
                        general.get_random_number_in_range_f32(-0.04, 0.04),
                        general.get_random_number_in_range_f32(0.0, 0.07),
                    ),
                    None,
                    size,
                    colour,
                    0,
                    0,
                    0,
                    0,
                );
            }
            step_pos = CVector::new(
                step_pos.x + step.x,
                step_pos.y + step.y,
                step_pos.z + step.z,
            );
        }
    }
}

impl Default for ParticleColour {
    fn default() -> Self {
        // re3 CParticle::AddParticle overload creates CRGBA(0,0,0,0);
        // alpha zero tells the full overload to use PARTICLE.CFG's authored
        // render colour. White/opaque here bypassed every configured colour.
        Self {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        }
    }
}

impl Default for CParticle {
    fn default() -> Self {
        Self::initialise()
    }
}

impl CParticle {
    pub fn emitter<'a>(&'a mut self, systems: &'a super::ParticleMgr::cParticleSystemMgr,
        camera_position: CVector) -> ParticleEmitter<'a> {
        self.emitter_with_context(Some(systems), Some(camera_position))
    }

    /// Control/damage must still run when presentation inputs are unavailable.
    /// Missing configuration rejects emission; missing camera rejects only
    /// particles with an authored range gate. No camera or table is fabricated.
    pub fn emitter_with_context<'a>(
        &'a mut self,
        systems: Option<&'a super::ParticleMgr::cParticleSystemMgr>,
        camera_position: Option<CVector>,
    ) -> ParticleEmitter<'a> {
        ParticleEmitter { particles: self, systems, camera_position }
    }

    pub fn initialise() -> Self {
        let mut rand_table = [0.0; RAND_TABLE_SIZE];
        let mut rand_val = -1.0;
        for value in &mut rand_table {
            *value = rand_val;
            rand_val += 0.1;
        }

        Self {
            // Native initial CTimer state, before the first frame projection.
            creation_inputs: Some(ParticleCreationInputs { frame_counter: 0, paused: false, interval_mask: 1 }),
            config_reloaded: true,
            particle_objects_initialised: true,
            live_particles: HashMap::new(),
            rand_table,
            sin_table_len: SIN_COS_TABLE_SIZE,
            cos_table_len: SIN_COS_TABLE_SIZE,
            texture_names: PARTICLE_TEXTURE_NAMES,
            renderware_textures_loaded: false,
            add_intents: Vec::new(),
            remove_system_intents: Vec::new(),
            tracked_positions: HashMap::new(),
            next_tracking_id: 1,
        }
    }

    /// Discard pending frame requests at the restart boundary. Live Bevy
    /// particles retain their identities until the lifecycle systems retire them.
    pub fn reload_config_for_restart(&mut self) {
        self.config_reloaded = true;
        self.add_intents.clear();
        self.remove_system_intents.clear();
        // Live ECS membership and logical IDs retire with their entities.
        // Resetting them here would invalidate still-live tracked particles.
    }

    pub fn set_creation_inputs(&mut self, timer: &CTimer, policy: &ParticleCreationPolicy, domain: ParticleSimulationDomain) {
        let paused = match domain {
            ParticleSimulationDomain::NativeGame => timer.get_is_paused(),
            ParticleSimulationDomain::IsolatedDeveloperScene => timer.get_is_user_paused(),
        };
        self.creation_inputs = Some(ParticleCreationInputs {
            frame_counter: timer.frame_counter(), paused, interval_mask: policy.interval_mask,
        });
    }

    pub fn invalidate_creation_inputs(&mut self) {
        self.creation_inputs = None;
        warn_once!("particle_creation_missing_canonical_timer");
    }

    pub fn can_create(&self, kind: tParticleType) -> bool {
        let Some(inputs) = self.creation_inputs else {
            warn_once!("particle_creation_waiting_for_canonical_inputs");
            return false;
        };
        ParticleCreationPolicy { interval_mask: inputs.interval_mask }
            .allows(kind, inputs.frame_counter, inputs.paused)
    }

    pub fn live_particle_count(&self) -> usize { self.live_particles.len() }
    pub fn has_creation_capacity(&self) -> bool {
        self.live_particles.len() + self.add_intents.len() < MAX_PARTICLES_ON_SCREEN
    }

    pub fn register_live_particle(&mut self, entity: Entity, tracking_id: Option<u64>) {
        self.live_particles.insert(entity, tracking_id);
        if let Some(id) = tracking_id {
            self.next_tracking_id = self.next_tracking_id.max(id.saturating_add(1));
        }
    }

    pub fn retire_live_particle(&mut self, entity: Entity) {
        if let Some(Some(id)) = self.live_particles.remove(&entity) {
            self.remove_tracked_particle(id);
        }
    }

    pub fn add_particle(
        &mut self,
        particle_type: tParticleType,
        position: CVector,
        direction: CVector,
        entity: Option<i32>,
        size: f32,
        rotation_speed: i32,
        rotation: i32,
        current_frame: i32,
        lifespan: i32,
    ) {
        self.add_particle_coloured(
            particle_type,
            position,
            direction,
            entity,
            size,
            ParticleColour::default(),
            rotation_speed,
            rotation,
            current_frame,
            lifespan,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_particle_coloured(
        &mut self,
        particle_type: tParticleType,
        position: CVector,
        direction: CVector,
        entity: Option<i32>,
        size: f32,
        colour: ParticleColour,
        rotation_speed: i32,
        rotation: i32,
        current_frame: i32,
        lifespan: i32,
    ) {
        if !self.can_create(particle_type) { return; }
        if !self.has_creation_capacity() {
            return;
        }
        self.add_intents.push(ParticleAddIntent {
            particle_type,
            position,
            direction,
            entity,
            size,
            colour,
            rotation_speed,
            rotation,
            current_frame,
            lifespan,
            tracking_id: None,
            range_admitted: false,
        });
    }

    /// Queue a particle whose live ECS position must remain observable by a
    /// gameplay system. This replaces re3's raw particle pointer linkage with
    /// a stable logical id while keeping the particle itself owned by Bevy ECS.
    #[allow(clippy::too_many_arguments)]
    pub fn add_tracked_particle(
        &mut self,
        particle_type: tParticleType,
        position: CVector,
        direction: CVector,
        entity: Option<i32>,
        size: f32,
        rotation_speed: i32,
        rotation: i32,
        current_frame: i32,
        lifespan: i32,
    ) -> Option<u64> {
        if !self.can_create(particle_type) { return None; }
        if !self.has_creation_capacity() {
            return None;
        }
        // TODO(parity): CParticle::AddParticle rejects m_fCreateRange before
        // returning a particle pointer. Migrate remaining raw request callers
        // to ParticleEmitter, which borrows those inputs before reaching this ID.
        let tracking_id = self.next_tracking_id;
        self.next_tracking_id = self.next_tracking_id.wrapping_add(1).max(1);
        self.add_intents.push(ParticleAddIntent {
            particle_type,
            position,
            direction,
            entity,
            size,
            colour: ParticleColour::default(),
            rotation_speed,
            rotation,
            current_frame,
            lifespan,
            tracking_id: Some(tracking_id),
            range_admitted: false,
        });
        Some(tracking_id)
    }

    pub fn tracked_particle_position(&self, tracking_id: u64) -> Option<CVector> {
        self.tracked_positions.get(&tracking_id).copied()
    }

    pub(crate) fn set_tracked_particle_position(&mut self, tracking_id: u64, position: CVector) {
        self.tracked_positions.insert(tracking_id, position);
    }

    pub(crate) fn remove_tracked_particle(&mut self, tracking_id: u64) {
        self.tracked_positions.remove(&tracking_id);
    }

    pub fn remove_p_system(&mut self, particle_type: tParticleType) {
        self.remove_system_intents.push(particle_type);
        self.add_intents
            .retain(|intent| intent.particle_type != particle_type);
    }

    pub fn drain_frame_intents(&mut self) -> (Vec<ParticleAddIntent>, Vec<tParticleType>) {
        (
            std::mem::take(&mut self.add_intents),
            std::mem::take(&mut self.remove_system_intents),
        )
    }
}
