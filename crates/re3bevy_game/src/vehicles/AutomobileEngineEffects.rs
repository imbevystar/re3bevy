//! CAutomobile::PreRender wheel, engine and exhaust effects over canonical actor owners.
use super::{
    Automobile::{AutomobileDamageOrigin, AutomobileState},
    Vehicle::{VehicleCore, VehicleVitality},
};
use crate::{
    core::FileLoader::CFileLoader, entities::Entity::eEntityStatus,
    modelinfo::ModelInfo::ModelInfoSlot,
};
use bevy::prelude::*;
use re3bevy_core::core::Coordinates::CVector;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutomobilePreRenderSet {
    Particles,
}

pub(crate) fn register_automobile_prerender_particles(
    mut actors: Query<(
        Entity,
        &crate::control::ScriptEntities::ScriptVehicleIdentity,
        &VehicleCore,
        &VehicleVitality,
        &mut AutomobileState,
        &super::Vehicle::VehicleDriveState,
    )>,
    index: Res<crate::control::ScriptEntities::ScriptEntityIndex>,
    loader: Res<CFileLoader>,
    camera: Res<crate::core::Camera::CCamera>,
    players: Res<crate::peds::PlayerPed::CWorldPlayerState>,
    relations: Query<&crate::peds::VehicleRelations::InVehicle>,
    systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    handling: Res<super::HandlingMgr::cHandlingDataMgr>,
    mut random: ResMut<re3bevy_core::core::General::CGeneral>,
    mut skidmarks: ResMut<crate::renderer::Skidmarks::CSkidmarks>,
    weather: Res<crate::renderer::Weather::CWeather>,
    timer: Res<re3bevy_core::core::Timer::CTimer>,
    replay: Option<Res<crate::control::Replay::CReplay>>,
) {
    let replay_playback =
        replay.is_some_and(|replay| replay.mode == crate::control::Replay::ReplayMode::Playback);
    let focused_vehicle = players
        .entities
        .get(players.player_in_focus)
        .copied()
        .flatten()
        .and_then(|player| relations.get(player).ok())
        .map(|relation| relation.vehicle);
    let mut particles = particles.emitter(&systems, camera.position);
    for (entity, identity, core, vitality, mut state, drive) in &mut actors {
        if index.vehicle(identity.handle) != Some(entity) {
            continue;
        }
        if identity.kind != crate::control::ScriptEntities::ScriptVehicleKind::Automobile {
            warn_once!("automobile_engine_effects_invalid_live_kind");
            continue;
        }
        super::AutomobileEffects::prerender_wheel_effects(
            core,
            &mut state,
            &loader,
            &mut particles,
            Some(&mut skidmarks),
            identity.handle as usize,
            weather.wet_roads,
            timer.frame_counter(),
            timer.time_in_milliseconds(),
            &mut random,
            replay_playback,
        );
        emit_engine_particles(
            core,
            vitality,
            &state,
            focused_vehicle == Some(entity),
            &camera,
            &loader,
            &mut particles,
        );
        emit_exhaust_particles(core, drive, &loader, &handling, &mut particles, &mut random);
    }
}

pub(crate) fn emit_engine_particles(
    core: &VehicleCore,
    vitality: &VehicleVitality,
    state: &AutomobileState,
    focused: bool,
    camera: &crate::core::Camera::CCamera,
    loader: &CFileLoader,
    particles: &mut crate::renderer::Particle::ParticleEmitter<'_>,
) {
    use super::DamageManager::eEngineStatus;
    use crate::renderer::ParticleType::*;
    if focused && camera.get_looking_forward_first_person() {
        return;
    }
    let status = state.damage.get_engine_status();
    if status < eEngineStatus::ENGINE_STATUS_STEAM1 as u32
        || core
            .physical
            .move_speed
            .dot(core.physical.entity.get_forward())
            * 180.0
            >= 90.0
    {
        return;
    }
    let Some(position) = resident_origin(
        loader,
        core,
        vitality,
        state,
        AutomobileDamageOrigin::Engine,
    ) else {
        return;
    };
    let mut direction = core.physical.move_speed * 0.5;
    if status < eEngineStatus::ENGINE_STATUS_STEAM2 as u32 {
        direction.z += 0.05;
        particles.add_particle(
            PARTICLE_ENGINE_STEAM,
            position,
            direction,
            None,
            0.1,
            0,
            0,
            0,
            0,
        );
    } else if status < eEngineStatus::ENGINE_STATUS_SMOKE as u32 {
        particles.add_particle(
            PARTICLE_ENGINE_STEAM,
            position,
            direction,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    } else if status < eEngineStatus::ENGINE_STATUS_ON_FIRE as u32 {
        particles.add_particle(
            PARTICLE_ENGINE_STEAM,
            position,
            direction,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
        particles.add_particle(
            PARTICLE_ENGINE_SMOKE,
            position,
            direction * 0.3,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    } else if vitality.health > 250.0 {
        particles.add_particle(
            PARTICLE_ENGINE_SMOKE2,
            position,
            direction * 0.2,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    }
}

pub(crate) fn resident_origin(
    loader: &CFileLoader,
    core: &VehicleCore,
    vitality: &VehicleVitality,
    state: &AutomobileState,
    origin: AutomobileDamageOrigin,
) -> Option<CVector> {
    let Ok(model_index) = usize::try_from(core.physical.entity.model_index) else {
        bevy::log::warn_once!("automobile_damage_invalid_original_model_identity");
        return None;
    };
    let Some(ModelInfoSlot::Vehicle(model)) = loader.model_info.get_model_info(model_index) else {
        bevy::log::warn_once!("automobile_damage_missing_original_model");
        return None;
    };
    let Some(positions) = model.authored_positions() else {
        bevy::log::warn_once!("automobile_damage_waiting_for_resident_positions");
        return None;
    };
    use crate::modelinfo::VehicleModelInfo::AuthoredVehiclePosition;
    let headlights = match positions.headlights {
        AuthoredVehiclePosition::Position(position) => position,
        // CVehicleModelInfo initializes every m_positions entry to zero.
        AuthoredVehiclePosition::Absent => CVector::ZERO,
        AuthoredVehiclePosition::Invalid => {
            bevy::log::warn_once!("automobile_damage_invalid_authored_headlights");
            return None;
        }
    };
    Some(origin_position(core, vitality, state, headlights, origin))
}

pub(crate) fn origin_position(
    core: &VehicleCore,
    vitality: &VehicleVitality,
    state: &AutomobileState,
    headlights: CVector,
    origin: AutomobileDamageOrigin,
) -> CVector {
    let bonnet_status = state.damage.get_door_status(0);
    let engine_offset = if matches!(bonnet_status, 0 | 1) {
        CVector::new(-0.1, -0.1, 0.0)
    } else {
        CVector::new(-0.5, -0.3, 0.0)
    };
    let mut local = headlights + engine_offset;
    if origin == AutomobileDamageOrigin::Engine
        && core.physical.entity.model_index as usize == crate::modelinfo::ModelIndices::MI_BFINJECT
    {
        local = CVector::new(0.3, -1.5, -0.1);
    }
    if matches!(
        origin,
        AutomobileDamageOrigin::Fire {
            looking_forward_first_person: true
        }
    ) && vitality.health < 250.0
        && core.physical.entity.status != eEntityStatus::STATUS_WRECKED
    {
        local += if core.physical.entity.model_index as usize
            == crate::modelinfo::ModelIndices::MI_FIRETRUCK
        {
            CVector::new(0.0, 3.0, -0.2)
        } else {
            CVector::new(0.0, 1.2, -0.8)
        };
    }
    let mut position = core.physical.entity.transform_point(local);
    position.z += 0.15;
    position
}

fn emit_exhaust_particles(
    core: &VehicleCore,
    drive: &super::Vehicle::VehicleDriveState,
    loader: &CFileLoader,
    handling: &super::HandlingMgr::cHandlingDataMgr,
    particles: &mut crate::renderer::Particle::ParticleEmitter<'_>,
    random: &mut re3bevy_core::core::General::CGeneral,
) {
    use super::HandlingMgr::cHandlingDataMgr;
    let Ok(model_id) = usize::try_from(core.physical.entity.model_index) else {
        bevy::log::warn_once!("automobile_exhaust_invalid_original_model_identity");
        return;
    };
    let Some(ModelInfoSlot::Vehicle(model)) = loader.model_info.get_model_info(model_id) else {
        bevy::log::warn_once!("automobile_exhaust_missing_original_model");
        return;
    };
    let handling_id = core.handling_id.or_else(|| {
        cHandlingDataMgr::get_handling_id(&model.handling_id).and_then(|id| i32::try_from(id).ok())
    });
    let Some(handling_id) = handling_id else {
        bevy::log::warn_once!("automobile_exhaust_missing_original_handling_identity");
        return;
    };
    let Some(handling) = usize::try_from(handling_id)
        .ok()
        .and_then(|id| handling.get_handling_data(id))
    else {
        bevy::log::warn_once!("automobile_exhaust_missing_original_handling");
        return;
    };
    super::AutomobileEffects::exhaust_effects(
        core,
        drive,
        handling.flags,
        loader,
        particles,
        random,
    );
}
