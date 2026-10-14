//! CPed::SetDie / FinishDieAnimCB / SetDead on canonical Bevy actor owners.
use super::{
    Ped::{
        CPed, MISSION_CHAR, PedState, blend_ped_movement_animation,
        update_non_player_movement_animation,
    },
    PedAnimation::PedAnimation,
    PedLocomotion::{
        PedLocomotion, PedPhysicalSurfaceMotion, apply_standing_update_position_seconds,
    },
    PedVehicleRetirement::release_entry_reservation,
    PedVitality::PedVitality,
    PlayerPed::PlayerPedState,
    VehicleRelations::InVehicle,
};
use crate::{
    animation::{
        AnimAssocDefinitions::ASSOCGRP_STD,
        AnimBlendAssociation::{
            ASSOC_DELETEFADEDOUT, ASSOC_FADEOUTWHENDONE, ASSOC_RUNNING, AnimBlendCallbackEvent,
        },
        AnimManager::CAnimManager,
        PedVehicleAnimation::{CPedVehicleAnimation, PedVehicleAnimationPhase},
    },
    collision::Collision::{CCollision, resolve_player_ped_static_collision_result},
    control::{
        PickupRuntime::weapon_pickup_model_name,
        Pickups::{CPickups, PICKUP_MONEY, PICKUP_ONCE_TIMEOUT, STREET_AMMO},
        ScriptEntities::{ScriptEntityIndex, ScriptPedIdentity},
    },
    core::{Camera::CCamera, EventList::CEventList, FileLoader::CFileLoader},
    entities::Physical::{CPhysical, GTA_PHYSICS_HZ},
    modelinfo::ModelIndices::{MI_COP, MI_FIREMAN},
    objects::Object::CObject,
    vehicles::{
        Boat::BoatState,
        Vehicle::{VehicleCore, VehicleEntryState},
    },
    weapons::{WeaponInfo::CWeaponInfoStore, WeaponType::eWeaponType},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use rust_gta_3_assets::assets::WorldAssets::CSurfaceTableAsset;
use rust_gta_3_core::core::{
    Coordinates::CVector, GameState::CGameState, General::CGeneral, Timer::CTimer,
};

/// One-shot integration barriers, not another ped state, timer or animation.
#[derive(Debug, Default, Clone, PartialEq, Component)]
pub(crate) struct PedDeathLifecycle {
    beginning_applied: bool,
    entry_idle_pending: bool,
    association_prepared: bool,
    corpse_pickups_created: bool,
}

impl PedDeathLifecycle {
    pub(crate) fn restored(ped: &CPed) -> Self {
        Self {
            beginning_applied: ped.ped_state == PedState::PED_DEAD,
            entry_idle_pending: false,
            association_prepared: ped.ped_state == PedState::PED_DEAD,
            corpse_pickups_created: ped.ped_state == PedState::PED_DEAD,
        }
    }
}

fn prepare_death_association(
    ped: &mut CPed,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    seated: bool,
) -> bool {
    if ped.impact_animation_id.is_some() && animation.movement_anim_clump.frames.is_empty() {
        warn_once!("npc_death_waiting_original_clump_frames");
        return false;
    }
    if let Some(id) = ped.impact_animation_id {
        let Some(index) = blend_ped_movement_animation(
            &mut animation.movement_anim_clump,
            manager,
            ASSOCGRP_STD,
            id,
            ped.impact_animation_blend_delta,
        ) else {
            warn_once!("npc_death_waiting_original_ifp_association: {id}");
            return false;
        };
        let association = &mut animation.movement_anim_clump.associations[index];
        if ped.impact_animation_speed > 0.0 {
            association.speed = ped.impact_animation_speed;
        }
        association.flags &= !ASSOC_FADEOUTWHENDONE;
        ped.flags.is_ped_die_anim_playing = association.is_running();
        if association.is_running() {
            association.set_finish_callback();
        }
    } else {
        ped.flags.is_ped_die_anim_playing = false;
    }
    if !seated {
        for association in &mut animation.movement_anim_clump.associations {
            if !association.is_partial() {
                association.flags &= !ASSOC_RUNNING;
            }
        }
    }
    true
}

fn finish_death_association(ped: &mut CPed, animation: &mut PedAnimation) {
    let Some(id) = ped.impact_animation_id else {
        return;
    };
    if let Some(association) = animation
        .movement_anim_clump
        .associations
        .iter_mut()
        .find(|association| association.anim_id == id)
        && association
            .callback_events
            .contains(&AnimBlendCallbackEvent::Finish)
    {
        association
            .callback_events
            .retain(|event| *event != AnimBlendCallbackEvent::Finish);
        ped.flags.is_ped_die_anim_playing = false;
    }
}

fn queue_death_speech(ped: &mut CPed, camera: &CCamera) {
    // CPed::Say's NPC death branch survives its lower speed thresholds and
    // ignores the repeated-phrase cooldown. It still has the height/1.65 gate.
    if camera.position.z + 3.0 < ped.physical.entity.get_position().z || camera.average_speed > 1.65
    {
        return;
    }
    let sound = crate::audio::soundlist::SOUND_PED_DEATH as i16;
    if sound < ped.queued_sound {
        ped.queued_sound = sound;
    }
}

fn original_ground(loader: &CFileLoader, position: CVector) -> Option<CVector> {
    loader
        .world
        .find_ground_z_for_3d_coord(position.x, position.y, position.z, &loader.model_info)
        .map(|ground| CVector::new(position.x, position.y, ground + 0.5))
}

fn create_dead_weapon_pickups(
    ped: &mut CPed,
    seated: bool,
    loader: &CFileLoader,
    pickups: &mut CPickups,
    now: u32,
) -> bool {
    if seated {
        return true;
    }
    // Resolve every required installed model before creating any pickup. A
    // missing dependency leaves the canonical inventory pending, without dupes.
    let mut models = [None; crate::weapons::WeaponType::WEAPONTYPE_TOTAL_INVENTORY_WEAPONS];
    for (slot, weapon) in ped.weapons.iter().enumerate() {
        if matches!(
            weapon.weapon_type,
            eWeaponType::WEAPONTYPE_UNARMED | eWeaponType::WEAPONTYPE_DETONATOR
        ) || weapon.ammo_total == 0
        {
            continue;
        }
        let Some(name) = weapon_pickup_model_name(weapon.weapon_type) else {
            warn_once!(
                "dead_ped_weapon_missing_native_model_mapping: {:?}",
                weapon.weapon_type
            );
            return false;
        };
        let Some((model, _)) = loader.model_info.get_model_info_by_name(name) else {
            warn_once!("dead_ped_weapon_waiting_installed_ide: {name}");
            return false;
        };
        models[slot] = Some(model);
    }
    let position = ped.physical.entity.get_position();
    for (slot, model) in models.into_iter().enumerate() {
        let Some(model) = model else {
            continue;
        };
        let angle = slot as f32 * 1.75;
        let radial = |angle: f32| {
            CVector::new(
                position.x + 1.5 * angle.sin(),
                position.y + 1.5 * angle.cos(),
                position.z,
            )
        };
        let mut pickup_position = original_ground(loader, radial(angle));
        let line_clear = pickup_position.is_some_and(|pickup| {
            let ped_position = CVector::new(position.x, position.y, position.z + 0.3);
            let delta = pickup - ped_position;
            let distance = delta.magnitude();
            let edge = ped_position + delta * ((distance + 0.3) / distance);
            loader
                .world
                .process_line_of_sight_buildings_hit(
                    [edge.x, edge.y, edge.z],
                    [ped_position.x, ped_position.y, ped_position.z],
                    &loader.model_info,
                    None,
                )
                .is_none()
        });
        // Preserve the native condition, including its apparently inverted LOS
        // decision and lack of a second LOS test (Pickups.cpp:1556).
        if pickup_position.is_none() || line_clear {
            pickup_position = original_ground(loader, radial(angle + 3.14));
        }
        if let Some(pickup_position) = pickup_position {
            let weapon = &ped.weapons[slot];
            let quantity = weapon
                .ammo_total
                .min(i32::from(STREET_AMMO[weapon.weapon_type as usize]));
            if pickups
                .generate_new_one(
                    pickup_position,
                    model as i32,
                    PICKUP_ONCE_TIMEOUT as i32,
                    quantity as i32,
                    now,
                )
                .is_none()
            {
                warn_once!("dead_ped_weapon_pickup_capacity_exhausted");
            }
        }
    }
    ped.clear_weapons();
    true
}

fn create_dead_money(
    ped: &CPed,
    seated: bool,
    nasty_game: bool,
    loader: &CFileLoader,
    pickups: &mut CPickups,
    general: &mut CGeneral,
    now: u32,
) -> bool {
    if !nasty_game
        || seated
        || ped.char_created_by == MISSION_CHAR
        || (MI_COP as i16..=MI_FIREMAN as i16).contains(&ped.physical.entity.model_index)
    {
        return true;
    }
    let Some((model, _)) = loader.model_info.get_model_info_by_name("Money") else {
        warn_once!("dead_ped_money_waiting_installed_ide");
        return false;
    };
    let mut money = general.get_random_number() % 60;
    if money < 10 {
        return true;
    }
    if money == 43 {
        money = 700;
    }
    let count = money / 40 + 1;
    let per_pickup = money / count;
    let position = ped.physical.entity.get_position();
    for _ in 0..count {
        let x = position.x
            + 1.5
                * ((general.get_random_number() % 256) as f32 * std::f32::consts::PI / 128.0).sin();
        let y = position.y
            + 1.5
                * ((general.get_random_number() % 256) as f32 * std::f32::consts::PI / 128.0).cos();
        if let Some(ground) = original_ground(loader, CVector::new(x, y, position.z)) {
            let quantity = per_pickup + (general.get_random_number() & 7);
            if pickups
                .generate_new_one(
                    ground,
                    model as i32,
                    PICKUP_MONEY as i32,
                    quantity as i32,
                    now,
                )
                .is_none()
            {
                warn_once!("dead_ped_money_pickup_capacity_exhausted");
            }
        }
    }
    true
}

fn surface_motion(
    ped: &CPed,
    physical: &CPhysical,
    is_vehicle: bool,
    is_boat: bool,
    step: f32,
) -> PedPhysicalSurfaceMotion {
    let speed_native = if is_boat {
        let offset = ped.physical.entity.get_position()
            - physical.entity.get_position()
            - CVector::new(0.0, 0.0, super::PedPlacement::PED_FEET_OFFSET);
        physical.get_speed_at_point(offset) + offset * (-physical.turn_speed.magnitude_sqr() * step)
    } else {
        physical.get_speed_at_point(ped.offset_from_phys_surface)
    };
    PedPhysicalSurfaceMotion {
        speed_native,
        turn_speed_z_native: physical.turn_speed.z,
        is_vehicle,
        is_boat,
    }
}

#[derive(SystemParam)]
pub(crate) struct DeathServices<'w> {
    index: Res<'w, ScriptEntityIndex>,
    loader: Res<'w, CFileLoader>,
    manager: Res<'w, CAnimManager>,
    collision: Res<'w, CCollision>,
    surfaces: Res<'w, CSurfaceTableAsset>,
    camera: Res<'w, CCamera>,
    weapons: Res<'w, CWeaponInfoStore>,
    timer: Res<'w, CTimer>,
    game: Res<'w, CGameState>,
    events: ResMut<'w, CEventList>,
    pickups: ResMut<'w, CPickups>,
    general: ResMut<'w, CGeneral>,
}

struct DeathActorInputs<'a> {
    loader: &'a CFileLoader,
    manager: &'a CAnimManager,
    weapons: &'a CWeaponInfoStore,
    camera: &'a CCamera,
}

/// Shared SetDie integration, borrowed from canonical NPC and player owners.
/// It neither advances animation time nor integrates another physical frame.
fn begin_actor_death(
    entity: Entity,
    ped: &mut CPed,
    animation: &mut PedAnimation,
    lifecycle: &mut PedDeathLifecycle,
    seated: bool,
    transition: Option<&mut CPedVehicleAnimation>,
    commands: &mut Commands,
    inputs: &DeathActorInputs<'_>,
    mut release_reservation: impl FnMut(Entity, usize, bool),
) -> bool {
    if !lifecycle.beginning_applied {
        if let Some(transition) = transition {
            if !seated
                && matches!(
                    transition.phase,
                    PedVehicleAnimationPhase::Align
                        | PedVehicleAnimationPhase::LockedDoor
                        | PedVehicleAnimationPhase::OpenDoor
                        | PedVehicleAnimationPhase::PullOut
                        | PedVehicleAnimationPhase::GetIn
                        | PedVehicleAnimationPhase::CloseEnter
                        | PedVehicleAnimationPhase::Shuffle
                )
            {
                lifecycle.entry_idle_pending = true;
                release_reservation(
                    transition.vehicle,
                    transition.door,
                    transition.enter_as_driver,
                );
                if let Some(info) = inputs.weapons.GetWeaponInfo(ped.current_weapon) {
                    ped.weapon_model_id = info.model_id;
                } else {
                    warn_once!("npc_death_entry_abort_waiting_original_weapon_info");
                }
                ped.physical.entity.flags.uses_collision = true;
                transition.association.blend_delta = -4.0;
                transition.association.flags |= ASSOC_DELETEFADEDOUT;
                transition.association.flags &= !ASSOC_RUNNING;
            } else {
                transition.association.blend_delta = -1000.0;
            }
            if ped.ped_state != PedState::PED_DEAD {
                animation
                    .movement_anim_clump
                    .associations
                    .insert(0, std::mem::take(&mut transition.association));
            }
            commands.entity(entity).remove::<CPedVehicleAnimation>();
        }
        if ped.ped_state == PedState::PED_DIE {
            queue_death_speech(ped, &inputs.camera);
        }
        lifecycle.beginning_applied = true;
    }
    if !lifecycle.association_prepared {
        if ped.impact_animation_id.is_some()
            && !animation.ensure_original_frames(
                ped.physical.entity.model_index.max(0) as usize,
                &inputs.loader,
            )
        {
            return false;
        }
        if lifecycle.entry_idle_pending {
            for association in &mut animation.movement_anim_clump.associations {
                if !association.is_partial() {
                    association.flags |= ASSOC_RUNNING;
                }
            }
            let idle = crate::animation::AnimationId::ANIM_STD_IDLE;
            if !animation
                .movement_anim_clump
                .associations
                .iter()
                .any(|association| association.anim_id == idle)
                && blend_ped_movement_animation(
                    &mut animation.movement_anim_clump,
                    &inputs.manager,
                    ped.anim_group as usize,
                    idle,
                    100.0,
                )
                .is_none()
            {
                warn_once!("npc_death_entry_abort_waiting_original_idle_association");
                return false;
            }
            lifecycle.entry_idle_pending = false;
        }
        if ped.ped_state == PedState::PED_DIE
            && !prepare_death_association(ped, animation, &inputs.manager, seated)
        {
            return false;
        }
        lifecycle.association_prepared = true;
    }
    true
}

pub(crate) fn process_script_ped_deaths(
    mut commands: Commands,
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &mut PedVitality,
            &mut PedLocomotion,
            &mut PedAnimation,
            &mut PedDeathLifecycle,
            Has<InVehicle>,
            Option<&mut CPedVehicleAnimation>,
        ),
        Without<PlayerPedState>,
    >,
    mut vehicles: ParamSet<(
        Query<(&mut VehicleEntryState, &mut VehicleCore)>,
        Query<(&VehicleCore, Has<BoatState>)>,
    )>,
    objects: Query<&CObject>,
    mut services: DeathServices,
) {
    let now = services.timer.time_in_milliseconds();
    let step = services.timer.time_step();
    let dt = services.timer.time_step_in_seconds();
    for (
        entity,
        identity,
        mut ped,
        mut vitality,
        mut locomotion,
        mut animation,
        mut lifecycle,
        seated,
        transition,
    ) in &mut peds
    {
        if services.index.ped(identity.handle) != Some(entity) {
            continue;
        }
        if !ped.dying_or_dead() {
            if lifecycle.beginning_applied
                || lifecycle.association_prepared
                || lifecycle.corpse_pickups_created
            {
                *lifecycle = PedDeathLifecycle::default();
            }
            continue;
        }
        let newly_started = !lifecycle.association_prepared;
        let inputs = DeathActorInputs {
            loader: &services.loader,
            manager: &services.manager,
            weapons: &services.weapons,
            camera: &services.camera,
        };
        if !begin_actor_death(
            entity,
            &mut ped,
            &mut animation,
            &mut lifecycle,
            seated,
            transition.map(|transition| transition.into_inner()),
            &mut commands,
            &inputs,
            |vehicle, door, enter_as_driver| {
                if let Ok((mut entry, mut core)) = vehicles.p0().get_mut(vehicle) {
                    release_entry_reservation(&mut entry, &mut core, door, enter_as_driver);
                }
            },
        ) {
            continue;
        }
        if ped.ped_state == PedState::PED_DIE
            && !newly_started
            && !ped.flags.is_ped_die_anim_playing
            && ((ped.flags.is_standing && ped.current_phys_surface.is_none())
                || ped.physical.flags.is_in_water
                || !ped.physical.entity.flags.uses_collision)
        {
            ped.set_dead(&mut vitality, identity.handle, now, &mut services.events);
            commands.entity(entity).remove::<CPedVehicleAnimation>();
        }
        if ped.ped_state == PedState::PED_DEAD {
            if !lifecycle.corpse_pickups_created {
                if !create_dead_weapon_pickups(
                    &mut ped,
                    seated,
                    &services.loader,
                    &mut services.pickups,
                    now,
                ) {
                    continue;
                }
                if !create_dead_money(
                    &ped,
                    seated,
                    services.game.nasty_game,
                    &services.loader,
                    &mut services.pickups,
                    &mut services.general,
                    now,
                ) {
                    continue;
                }
                lifecycle.corpse_pickups_created = true;
            }
            continue;
        }
        if !seated && !ped.physical.entity.flags.remove_from_world {
            let zone = ped.physical.zone_level;
            if zone > crate::collision::ColModel::LEVEL_GENERIC as i8
                && zone != services.collision.collision_in_memory as i8
            {
                warn_once!("npc_death_waiting_original_collision_level");
                continue;
            }
            ped.physical.process_control_pre_physics(step);
            update_non_player_movement_animation(
                &mut ped,
                &mut locomotion,
                &vitality,
                &mut animation,
                &services.manager,
                &services.loader,
                dt,
                false,
                None,
            );
            ped.calculate_new_velocity(&mut locomotion, &vitality, step, false);
            if ped.flags.is_standing {
                ped.physical.entity.set_heading(locomotion.rotation_cur);
            }
            let surface = ped.current_phys_surface.and_then(|entity| {
                if let Ok((core, boat)) = vehicles.p1().get(entity) {
                    Some(surface_motion(&ped, &core.physical, true, boat, step))
                } else {
                    objects
                        .get(entity)
                        .ok()
                        .map(|object| surface_motion(&ped, &object.physical, false, false, step))
                }
            });
            if surface.is_some_and(|surface| surface.is_boat) {
                ped.physical.move_speed.z = surface.unwrap().speed_native.z;
            }
            let old_position = ped.physical.entity.get_position();
            // Existing static response/standing calculations use seconds units.
            // Convert the borrowed canonical velocity only across that boundary.
            ped.physical.move_speed = ped.physical.move_speed * GTA_PHYSICS_HZ;
            let moved = CVector::new(locomotion.moved.x * step, locomotion.moved.y * step, 0.0);
            apply_standing_update_position_seconds(&mut ped, &mut locomotion, moved, surface, dt);
            let attempted = old_position + ped.physical.move_speed * dt;
            super::PlayerPed::begin_player_collision_frame(&mut ped);
            let response = resolve_player_ped_static_collision_result(
                old_position,
                attempted,
                &mut ped,
                &services.loader,
                &services.surfaces,
                step,
                now,
            );
            ped.physical.move_speed = ped.physical.move_speed * GTA_PHYSICS_HZ.recip();
            ped.physical.entity.set_position(response.position);
            ped.current_phys_surface = None;
        } else {
            update_non_player_movement_animation(
                &mut ped,
                &mut locomotion,
                &vitality,
                &mut animation,
                &services.manager,
                &services.loader,
                dt,
                false,
                None,
            );
        }
        finish_death_association(&mut ped, &mut animation);
    }
}

#[derive(SystemParam)]
pub(crate) struct PlayerDeathServices<'w> {
    slots: Res<'w, super::PlayerPed::CWorldPlayerState>,
    loader: Res<'w, CFileLoader>,
    manager: Res<'w, CAnimManager>,
    weapons: Res<'w, CWeaponInfoStore>,
    camera: ResMut<'w, CCamera>,
    crosshair: ResMut<'w, crate::weapons::WeaponEffects::CWeaponEffects>,
    timer: Res<'w, CTimer>,
    events: ResMut<'w, CEventList>,
}

fn player_death_deadline_passed(now: u32, death_time: u32) -> bool {
    now > death_time.wrapping_add(4_000)
}

/// CPed control's real finish gate plus CPlayerPed's native four-second gate.
/// Physical movement remains in the existing P1/P2 controller; this stage only
/// services their canonical death clump and one-shot integration lifetime.
pub(crate) fn process_player_death_associations(
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut CPed,
            &mut PedVitality,
            &mut PedLocomotion,
            &mut PedAnimation,
            &mut PlayerPedState,
            &mut PedDeathLifecycle,
            Has<InVehicle>,
            Option<&mut CPedVehicleAnimation>,
        ),
        With<PlayerPedState>,
    >,
    mut vehicles: Query<(&mut VehicleEntryState, &mut VehicleCore)>,
    mut services: PlayerDeathServices,
) {
    let now = services.timer.time_in_milliseconds();
    for (
        entity,
        mut ped,
        mut vitality,
        mut locomotion,
        mut animation,
        mut state,
        mut lifecycle,
        seated,
        transition,
    ) in &mut players
    {
        if !services.slots.entities.contains(&Some(entity)) || !state.active_in_world {
            continue;
        }
        if !ped.dying_or_dead() {
            if lifecycle.beginning_applied || lifecycle.association_prepared {
                *lifecycle = PedDeathLifecycle::default();
            }
            continue;
        }
        let newly_started = !lifecycle.association_prepared;
        let inputs = DeathActorInputs {
            loader: &services.loader,
            manager: &services.manager,
            weapons: &services.weapons,
            camera: &services.camera,
        };
        if !begin_actor_death(
            entity,
            &mut ped,
            &mut animation,
            &mut lifecycle,
            seated,
            transition.map(|transition| transition.into_inner()),
            &mut commands,
            &inputs,
            |vehicle, door, enter_as_driver| {
                if let Ok((mut entry, mut core)) = vehicles.get_mut(vehicle) {
                    release_entry_reservation(&mut entry, &mut core, door, enter_as_driver);
                }
            },
        ) {
            continue;
        }
        // ClearWeaponTarget owns these player fields even while seated/dead.
        state.weapon_lock_on_target = None;
        state.weapon_lock_on_object = None;
        state.has_lock_on_target = false;
        state.have_target_selected = false;
        ped.point_gun_at = None;
        ped.flags.is_pointing_gun_at = false;
        if ped.ped_type == super::PedType::ePedType::PEDTYPE_PLAYER1 {
            services.crosshair.clear_crosshair();
            if services.camera.weapon_mode_active() {
                ped.physical.entity.flags.is_visible = true;
            }
            services.camera.clear_player_weapon_mode();
        }
        if !newly_started {
            update_non_player_movement_animation(
                &mut ped,
                &mut locomotion,
                &vitality,
                &mut animation,
                &services.manager,
                &services.loader,
                services.timer.time_step_in_seconds(),
                false,
                None,
            );
            finish_death_association(&mut ped, &mut animation);
        }
        // World still advances the clump after the forced player deadline;
        // DEAD control itself leaves the physical pose unchanged.
        if ped.ped_state == PedState::PED_DEAD {
            continue;
        }
        let completed_on_surface = !ped.flags.is_ped_die_anim_playing
            && !seated
            && ((ped.flags.is_standing && ped.current_phys_surface.is_none())
                || ped.physical.flags.is_in_water
                || !ped.physical.entity.flags.uses_collision);
        if (!newly_started && completed_on_surface)
            || player_death_deadline_passed(now, vitality.bloody_footprint_count_or_death_time)
        {
            ped.set_dead(&mut vitality, state.pool_handle, now, &mut services.events);
            commands.entity(entity).remove::<CPedVehicleAnimation>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        animation::{
            AnimBlendAssocGroup::CAnimBlendAssocGroup,
            AnimBlendAssociation::{ASSOC_PARTIAL, CAnimBlendAssociation},
            AnimationId::ANIM_STD_KO_FRONT,
        },
        peds::{Ped::eMoveState, PedType::ePedType},
        weapons::Weapon::CWeapon,
    };

    fn player_death_app(seated: bool, clip_length: f32) -> (App, Entity) {
        let mut app = App::new();
        let mut manager = CAnimManager::initialise();
        let mut template = CAnimBlendAssociation::new();
        template.anim_id = ANIM_STD_KO_FRONT;
        template.hierarchy.total_length = clip_length;
        template.flags = ASSOC_PARTIAL;
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); ASSOCGRP_STD + 1];
        manager.anim_assoc_groups[ASSOCGRP_STD]
            .assoc_list
            .push(template);
        let mut timer = CTimer::initialise();
        timer.set_time_in_milliseconds(1000);
        timer.set_time_step(5.0);
        app.insert_resource(manager)
            .insert_resource(timer)
            .init_resource::<CFileLoader>()
            .init_resource::<CWeaponInfoStore>()
            .init_resource::<CCamera>()
            .init_resource::<CEventList>()
            .insert_resource(crate::weapons::WeaponEffects::CWeaponEffects::init())
            .init_resource::<super::super::PlayerPed::CWorldPlayerState>()
            .add_systems(Update, process_player_death_associations);
        let mut player =
            super::super::PlayerPed::CPlayerPed::new(&mut CGeneral::with_seed(7), None);
        player.state.active_in_world = true;
        player.state.pool_handle = 1;
        player.ped.physical.entity.model_index = 0;
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.flags.is_standing = !seated;
        // Explicit unit frame/timeline; real-game probes require installed DFF/IFP.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "unit_player_death".into();
        player.ped.set_die(
            &mut player.vitality,
            Some(ANIM_STD_KO_FRONT),
            4.0,
            0.5,
            1000,
        );
        let entity = app.world_mut().spawn(player.into_bundle()).id();
        app.world_mut()
            .resource_mut::<super::super::PlayerPed::CWorldPlayerState>()
            .entities[0] = Some(entity);
        if seated {
            let vehicle = app.world_mut().spawn_empty().id();
            app.world_mut()
                .entity_mut(entity)
                .insert(InVehicle::driver(vehicle));
        }
        (app, entity)
    }

    #[test]
    fn canonical_player_death_consumes_a_real_slowed_finish_instead_of_wall_time() {
        let (mut app, entity) = player_death_app(false, 0.5);
        app.update();
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_time_in_milliseconds(4999);
        app.update();
        assert_eq!(
            app.world().get::<CPed>(entity).unwrap().ped_state,
            PedState::PED_DIE
        );
        assert!(
            app.world()
                .get::<CPed>(entity)
                .unwrap()
                .flags
                .is_ped_die_anim_playing
        );
        app.world_mut().resource_mut::<CTimer>().set_time_step(50.5);
        app.update();
        let ped = app.world().get::<CPed>(entity).unwrap();
        assert_eq!(ped.ped_state, PedState::PED_DEAD);
        assert!(!ped.flags.is_ped_die_anim_playing);
        assert_eq!(
            app.world()
                .get::<PedAnimation>(entity)
                .unwrap()
                .movement_anim_clump
                .associations[0]
                .current_time,
            0.5
        );
        assert!(!ped.physical.entity.flags.uses_collision);
    }

    #[test]
    fn seated_player_retains_native_strict_four_second_deadline_without_a_fake_finish() {
        let (mut app, entity) = player_death_app(true, 10.0);
        app.update();
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_time_in_milliseconds(5000);
        app.update();
        assert_eq!(
            app.world().get::<CPed>(entity).unwrap().ped_state,
            PedState::PED_DIE
        );
        app.world_mut()
            .resource_mut::<CTimer>()
            .set_time_in_milliseconds(5001);
        app.update();
        assert_eq!(
            app.world().get::<CPed>(entity).unwrap().ped_state,
            PedState::PED_DEAD
        );
        let clump = &app
            .world()
            .get::<PedAnimation>(entity)
            .unwrap()
            .movement_anim_clump;
        assert!(clump.associations[0].current_time < 10.0);
        assert!(
            !clump.associations[0]
                .callback_events
                .contains(&AnimBlendCallbackEvent::Finish)
        );
        assert!(
            app.world().get::<InVehicle>(entity).is_some(),
            "SetDead retains the native seat link"
        );
    }

    #[test]
    fn resurrection_cancels_old_clump_requests_and_death_presentation_metadata() {
        let mut player =
            super::super::PlayerPed::CPlayerPed::new(&mut CGeneral::with_seed(7), None);
        player
            .ped
            .set_fall(&player.vitality, 1500, ANIM_STD_KO_FRONT, false, 20, 11);
        player
            .ped
            .set_die(&mut player.vitality, Some(ANIM_STD_KO_FRONT), 4.0, 0.5, 21);
        player.locomotion.anim_move_delta =
            rust_gta_3_core::core::Coordinates::CVector2D::new(0.3, 0.4);
        player.animation.movement_anim_clump.velocity2d = Some((0.3, 0.4));
        crate::control::GameLogic::CGameLogic::restore_player_stuff_during_resurrection(
            &mut player.as_mut(),
            CVector::new(1.0, 2.0, 3.0),
            90.0,
        );
        assert!(player.ped.fall_animation_request.is_none());
        assert!(player.ped.impact_animation_id.is_none());
        assert_eq!(
            player.animation.movement_anim_clump.velocity2d,
            Some((0.0, 0.0))
        );
        assert_eq!(player.vitality.health, 100.0);
        assert_eq!(player.ped.ped_state, PedState::PED_IDLE);
        assert_eq!(player.position(), CVector::new(1.0, 2.0, 4.0));
    }

    #[test]
    fn set_die_preserves_stored_state_halves_fallen_blend_and_supports_no_clip() {
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        let mut vitality = PedVitality::default();
        ped.ped_state = PedState::PED_FALL;
        ped.last_ped_state = PedState::PED_WANDER_PATH;
        ped.seek_target = Some(12);
        ped.set_die(&mut vitality, None, 4.0, 0.0, 17);
        assert_eq!(ped.last_ped_state, PedState::PED_WANDER_PATH);
        assert_eq!(ped.seek_target, Some(12)); // ClearAll's native uncontrollable filter.
        assert_eq!(ped.impact_animation_blend_delta, 2.0);
        assert!(!ped.flags.is_ped_die_anim_playing);
        assert_eq!(vitality.bloody_footprint_count_or_death_time, 17);
        assert_eq!(vitality.health, 0.0);
        ped.set_die(&mut vitality, Some(ANIM_STD_KO_FRONT), 8.0, 3.0, 99);
        assert_eq!(ped.impact_animation_id, None);
        assert_eq!(vitality.bloody_footprint_count_or_death_time, 17);
    }

    #[test]
    fn controllable_death_clears_seek_and_preserves_native_look_restoration() {
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        let mut vitality = PedVitality::default();
        ped.ped_state = PedState::PED_IDLE;
        ped.move_state = eMoveState::PEDMOVE_WALK;
        ped.seek_target = Some(12);
        ped.flags.is_looking = true;
        ped.flags.is_aiming_gun = true;
        ped.flags.shake_fist = true;
        ped.set_die(&mut vitality, Some(ANIM_STD_KO_FRONT), 4.0, 0.0, 17);
        assert_eq!(ped.seek_target, None);
        assert_eq!(ped.move_state, eMoveState::PEDMOVE_NONE);
        assert!(ped.flags.is_restoring_look && ped.flags.is_restoring_gun);
        assert!(!ped.flags.is_looking && !ped.flags.is_aiming_gun && !ped.flags.shake_fist);
        assert_eq!(ped.look_timer, 4017);
    }

    #[test]
    fn death_callback_retires_the_flag_and_retains_the_authored_final_pose() {
        let mut manager = CAnimManager::initialise();
        let mut template = CAnimBlendAssociation::new();
        template.anim_id = ANIM_STD_KO_FRONT;
        template.hierarchy.total_length = 1.0; // Synthetic timeline for the callback contract only.
        template.flags = ASSOC_PARTIAL | ASSOC_FADEOUTWHENDONE;
        template.speed = 0.75;
        manager.anim_assoc_groups.push(CAnimBlendAssocGroup {
            assoc_list: vec![template],
        });
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        ped.set_die(
            &mut PedVitality::default(),
            Some(ANIM_STD_KO_FRONT),
            4.0,
            0.0,
            0,
        );
        let mut animation = PedAnimation::default();
        animation.movement_anim_clump.set_number_of_frames(1);
        assert!(prepare_death_association(
            &mut ped,
            &mut animation,
            &manager,
            false
        ));
        assert_eq!(animation.movement_anim_clump.associations[0].speed, 0.75);
        assert_eq!(
            animation.movement_anim_clump.associations[0].flags & ASSOC_FADEOUTWHENDONE,
            0
        );
        animation.movement_anim_clump.associations[0].update_time(2.0, 1.0);
        finish_death_association(&mut ped, &mut animation);
        assert!(!ped.flags.is_ped_die_anim_playing);
        assert_eq!(
            animation.movement_anim_clump.associations[0].current_time,
            1.0
        );
        assert!(!animation.movement_anim_clump.associations[0].deleted);
    }

    #[test]
    fn missing_installed_weapon_model_preserves_pending_inventory_and_creates_no_pickup() {
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut CGeneral::with_seed(1));
        ped.weapons[2] = CWeapon::initialise(eWeaponType::WEAPONTYPE_COLT45, 60);
        let mut pickups = CPickups::init();
        assert!(!create_dead_weapon_pickups(
            &mut ped,
            false,
            &CFileLoader::new(),
            &mut pickups,
            100
        ));
        assert_eq!(ped.weapons[2].ammo_total, 60);
        assert!(pickups.pickups.iter().all(|pickup| pickup.pickup_type == 0));
        assert!(create_dead_weapon_pickups(
            &mut ped,
            true,
            &CFileLoader::new(),
            &mut pickups,
            100
        ));
        assert_eq!(ped.weapons[2].ammo_total, 60); // In-vehicle deaths do not clear inventory.
    }
}
