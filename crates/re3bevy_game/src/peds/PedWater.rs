//! CPed::ProcessBuoyancy player boundary, using original water/COL/timecycle.
//! Damage borrows canonical vitality and player protection through InflictDamage.
use super::{Ped::PedWeaponDamageResult, PlayerPed::PlayerPedMut};
use crate::{
    audio::{DMAudio::cDMAudio, soundlist::SOUND_SPLASH},
    core::FileLoader::CFileLoader,
    entities::Physical::GTA_PHYSICS_HZ,
    objects::ParticleObject::{CParticleObjectRuntime, CRGBA, eParticleObjectType},
    renderer::{Particle::ParticleEmitter, Timecycle::CTimeCycle, WaterLevel::CWaterLevel},
    vehicles::Floater::process_buoyancy,
    weapons::WeaponType::eWeaponType,
};
use bevy::log::warn_once;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};

// TODO(parity): CPed::ProcessBuoyancy boat-underfoot vertical probe,
// PlacePedOnDryLand, and shared native delayed-effect timer lifetime.

pub(crate) fn apply_player_drowning_damage(
    player: &mut PlayerPedMut<'_>,
    damage: f32,
    now_ms: u32,
) -> PedWeaponDamageResult {
    let can_be_damaged = player.can_be_damaged;
    player.ped.inflict_weapon_damage(
        player.vitality,
        eWeaponType::WEAPONTYPE_DROWNING,
        damage,
        1.0,
        false,
        can_be_damaged,
        now_ms,
        0,
    )
}

pub(crate) fn process_player_water(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    water: &CWaterLevel,
    file_loader: &CFileLoader,
    mut particles: Option<&mut ParticleEmitter<'_>>,
    particle_objects: &mut CParticleObjectRuntime,
    general: &mut CGeneral,
    timecycle: Option<&CTimeCycle>,
    mut audio: Option<&mut cDMAudio>,
    time_ms: u32,
    weather_wind: f32,
    time_step: f32,
) {
    use crate::entities::Physical::GRAVITY;
    use crate::renderer::{Particle::ParticleColour, ParticleType::PARTICLE_RAIN_SPLASH_BIGGROW};

    let splash_colour = timecycle.map(|timecycle| {
        let current = &timecycle.current;
        CRGBA::new(
            ((0.5 * current.directional[0] + current.ambient[0]) * 127.5).clamp(0.0, 255.0) as u8,
            ((0.5 * current.directional[2] + current.ambient[2]) * 127.5).clamp(0.0, 255.0) as u8,
            ((0.5 * current.directional[1] + current.ambient[1]) * 127.5).clamp(0.0, 255.0) as u8,
            general.get_random_number_in_range_f32(48.0, 96.0) as u8,
        )
    });
    if splash_colour.is_none() {
        warn_once!("player_buoyancy_effects_waiting_original_timecycle");
    }
    let particle_colour = splash_colour.map(|colour| ParticleColour {
        red: colour.red,
        green: colour.green,
        blue: colour.blue,
        alpha: colour.alpha,
    });

    if player.state.water_circles_at != 0 && time_ms >= player.state.water_circles_at {
        let mut pos = player.position();
        if let Some(sample) =
            water.get_water_level(pos.x, pos.y, pos.z, false, time_ms, weather_wind)
        {
            pos.z = sample.level;
        }
        if pos.z != 0.0 {
            player.state.water_circles_at = 0;
            for _ in 0..4 {
                pos.x += general.get_random_number_in_range_f32(-0.75, 0.75);
                pos.y += general.get_random_number_in_range_f32(-0.75, 0.75);
                if let (Some(particles), Some(colour)) = (particles.as_mut(), particle_colour) {
                    particles.add_particle_coloured(
                        PARTICLE_RAIN_SPLASH_BIGGROW,
                        pos,
                        CVector::ZERO,
                        None,
                        0.0,
                        colour,
                        0,
                        0,
                        0,
                        0,
                    );
                }
            }
        }
    }

    if player.state.water_raindrops_at != 0 && time_ms >= player.state.water_raindrops_at {
        let mut pos = player.position();
        if let Some(sample) =
            water.get_water_level(pos.x, pos.y, pos.z, false, time_ms, weather_wind)
        {
            pos.z = sample.level;
        }
        if pos.z >= 0.0 {
            pos.z += 0.25;
            player.state.water_raindrops_at = 0;
            particle_objects.add_object(
                eParticleObjectType::POBJECT_SPLASHES_AROUND as i32,
                pos,
                CVector::ZERO,
                4.5,
                1_500,
                CRGBA::new(0, 0, 0, 0),
                true,
                time_ms,
            );
        }
    }

    player.ped.physical.flags.is_in_water = false;
    let Ok(model_id) = usize::try_from(player.ped.physical.entity.model_index) else {
        warn_once!("player_buoyancy_invalid_original_model");
        return;
    };
    let Some(col) = file_loader.model_info.col_model(model_id) else {
        warn_once!("player_buoyancy_waiting_original_ped_col: {model_id}");
        player.ped.physical.entity.flags.touching_water = false;
        return;
    };
    let min = CVector::new(
        col.bounding_box.min.x,
        col.bounding_box.min.y,
        col.bounding_box.min.z,
    );
    let max = CVector::new(
        col.bounding_box.max.x,
        col.bounding_box.max.y,
        col.bounding_box.max.z,
    );
    let buoyancy =
        GRAVITY * player.ped.physical.mass * if player.ped.dying_or_dead() { 1.5 } else { 1.3 };
    let Some(result) = process_buoyancy(
        &player.ped.physical,
        min,
        max,
        buoyancy,
        water,
        time_ms,
        weather_wind,
        time_step,
        false,
        None,
    ) else {
        player.ped.physical.entity.flags.touching_water = false;
        return;
    };

    player.ped.physical.entity.flags.touching_water = true;
    player.ped.physical.flags.is_in_water = true;
    player.ped.physical.move_speed.z +=
        result.impulse.z / player.ped.physical.mass.max(f32::EPSILON) * GTA_PHYSICS_HZ;
    if player.ped.dying_or_dead() {
        return;
    }

    let native_ratio = result.impulse.z
        / (GRAVITY * player.ped.physical.mass.max(f32::EPSILON) * time_step.max(0.01));
    if native_ratio > 0.75 || result.water_level > player.position().z {
        let resistance = 0.9_f32.powf(time_step);
        player.ped.physical.move_speed.x *= resistance;
        player.ped.physical.move_speed.y *= resistance;
        player.ped.physical.move_speed.z *= resistance;
        player.ped.flags.is_standing = false;
        apply_player_drowning_damage(player, 3.0 * time_step, time_ms);
    }
    if native_ratio > 0.25 {
        let resistance = 0.9_f32.powf(time_step);
        player.ped.physical.move_speed.x *= resistance;
        player.ped.physical.move_speed.y *= resistance;
        if player.ped.physical.move_speed.z < -0.1 * GTA_PHYSICS_HZ {
            player.ped.physical.move_speed.z = -0.01 * GTA_PHYSICS_HZ;
            let mut splash = player.position();
            splash.x += player.ped.physical.move_speed.x * (2.2 / GTA_PHYSICS_HZ);
            splash.y += player.ped.physical.move_speed.y * (2.2 / GTA_PHYSICS_HZ);
            if let Some(sample) =
                water.get_water_level(splash.x, splash.y, splash.z, false, time_ms, weather_wind)
            {
                splash.z = sample.level;
            } else {
                splash.z = result.water_level;
            }
            if let Some(splash_colour) = splash_colour {
                particle_objects.add_object(
                    eParticleObjectType::POBJECT_PED_WATER_SPLASH as i32,
                    splash,
                    CVector::new(0.0, 0.0, 0.1),
                    0.0,
                    200,
                    splash_colour,
                    true,
                    time_ms,
                );
            }
            player.state.water_raindrops_at = time_ms.saturating_add(80);
            player.state.water_circles_at = time_ms.saturating_add(100);
            if let Some(audio) = audio.as_deref_mut()
                && player.ped.physical.audio_entity_id >= 0
            {
                audio.PlayOneShot(
                    player.ped.physical.audio_entity_id,
                    SOUND_SPLASH as u16,
                    0.0,
                );
            }
        } else if player.ped.physical.move_speed.z < -0.04 * GTA_PHYSICS_HZ {
            player.ped.physical.move_speed.z = -0.02 * GTA_PHYSICS_HZ;
        }
    }
}
