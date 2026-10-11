//! Ambient CPed::WanderPath/Flee control over the canonical ECS owners.

use bevy::{ecs::system::SystemParam, prelude::*};
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    animation::AnimManager::CAnimManager,
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity, ScriptVehicleIdentity,
    },
    core::{FileLoader::CFileLoader, Game::resolve_ped_static_motion},
    entities::Entity::CEntity,
    objects::Object::CObject,
    peds::{
        Ped::{
            CPed, PedState, RANDOM_CHAR, apply_script_ped_wander_path, eMoveState, eObjective,
            update_non_player_movement_animation,
        },
        PedAnimation::{PedAnimation, PedAnimationSound},
        PedLocomotion::PedLocomotion,
        PedVitality::PedVitality,
        PlayerPed::{PlayerPedState, player_dynamic_vertical_hit_entity},
        VehicleRelations::InVehicle,
    },
    vehicles::Vehicle::VehicleCore,
};

/// Read only the owners needed for ground and fleeing queries. In particular,
/// unrelated vehicle drive/damage/audio owners and object state are never
/// acquired mutably, and no per-frame copy of the dynamic world is retained.
#[derive(SystemParam)]
pub(crate) struct PedWanderWorld<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    vehicles: Query<'w, 's, (Entity, &'static ScriptVehicleIdentity, &'static VehicleCore)>,
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static CObject)>,
}

impl PedWanderWorld<'_, '_> {
    fn vehicle_position(&self, slot: usize) -> Option<CVector> {
        let handle = self.index.vehicle_handle_for_slot(slot)?;
        let entity = self.index.vehicle(handle)?;
        let (_, identity, core) = self.vehicles.get(entity).ok()?;
        (identity.handle == handle).then_some(core.physical.entity.get_position())
    }

    fn vehicle_entities(&self) -> impl Iterator<Item = (Entity, &CEntity)> + '_ {
        self.vehicles
            .iter()
            .filter_map(move |(entity, identity, core)| {
                (self.index.vehicle(identity.handle) == Some(entity)
                    && core.physical.entity.flags.uses_collision)
                    .then_some((entity, &core.physical.entity))
            })
    }

    fn ground_entities(&self) -> impl Iterator<Item = (Entity, &CEntity)> + '_ {
        self.vehicle_entities()
            .chain(
                self.objects
                    .iter()
                    .filter_map(move |(entity, identity, object)| {
                        (self.index.object(identity.handle) == Some(entity)
                            && object.physical.entity.flags.uses_collision)
                            .then_some((entity, &object.physical.entity))
                    }),
            )
    }

    /// Existing lightweight authored vehicle bounds check for ambient fleeing.
    /// The full native sphere/contact response remains in the collision domain.
    fn vehicle_position_blocked(&self, position: CVector, loader: &CFileLoader) -> bool {
        self.vehicle_entities().any(|(_, vehicle)| {
            let Some(col) = loader
                .model_info
                .col_model(vehicle.model_index.max(0) as usize)
            else {
                return false;
            };
            let local = vehicle.matrix.multiply_inverse_point(position);
            let feet_z = local.z - crate::peds::PedPlacement::PED_FEET_OFFSET;
            let min = &col.bounding_box.min;
            let max = &col.bounding_box.max;
            if feet_z > max.z + 0.1 {
                return false;
            }
            local.x >= min.x - 0.35
                && local.x <= max.x + 0.35
                && local.y >= min.y - 0.35
                && local.y <= max.y + 0.35
                && feet_z >= min.z - 0.05
                && feet_z <= max.z + 0.1
        })
    }
}

fn land_new_ambient_ped_on_ground(
    ped: &mut CPed,
    animation: &mut PedAnimation,
    manager: &CAnimManager,
    file_loader: &CFileLoader,
    world: &PedWanderWorld,
) -> Option<u16> {
    if ped.flags.is_standing
        || ped.objective != eObjective::OBJECTIVE_NONE
        || !matches!(
            ped.ped_state,
            PedState::PED_IDLE | PedState::PED_WANDER_PATH
        )
    {
        return None;
    }

    let mut position = ped.physical.entity.get_position();
    let feet_offset = crate::peds::PedPlacement::PED_FEET_OFFSET;
    let line_start_z = position.z + 0.5;
    let line_end_z = position.z - feet_offset;
    let mut ground_z = file_loader
        .world
        .process_vertical_line_buildings_only_with_entity(
            [position.x, position.y, line_start_z],
            line_end_z,
            &file_loader.model_info,
        )
        .map(|(hit, _)| hit.point.z);
    for (entity, physical) in world.ground_entities() {
        let hit = player_dynamic_vertical_hit_entity(
            position.x,
            position.y,
            line_start_z,
            line_end_z,
            entity,
            &physical.matrix,
            physical.model_index.max(0) as usize,
            true,
            file_loader,
        );
        if let Some(hit) = hit
            && ground_z.is_none_or(|current| hit.z > current)
        {
            ground_z = Some(hit.z);
        }
    }
    let Some(ground_z) = ground_z else {
        warn_once!("ambient_ped_landing_waiting_original_ground");
        return None;
    };

    // CPed::ProcessControl (Ped.cpp:2616-2633): the short landing probe
    // precedes walk control and raises the root to FEET_OFFSET above the hit.
    position.z = ground_z + feet_offset;
    ped.physical.entity.set_position(position);
    ped.flags.is_in_the_air = false;
    ped.flags.is_standing = true;
    let Some(sound) = animation.start_landing(manager) else {
        warn_once!("ambient_ped_landing_missing_original_association");
        return None;
    };
    ped.flags.is_landing = true;
    Some(sound)
}

/// Advance random on-foot actors using their authored paths and animations.
/// Other actor poses are read directly from their current ECS components.
pub(crate) fn process_ambient_ped_wander(
    world: PedWanderWorld,
    mut peds: Query<
        (
            Entity,
            &ScriptPedIdentity,
            &mut CPed,
            &PedVitality,
            &mut PedLocomotion,
            &mut PedAnimation,
            Has<InVehicle>,
        ),
        Without<PlayerPedState>,
    >,
    file_loader: Res<CFileLoader>,
    anim_manager: Res<CAnimManager>,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    mut commands: Commands,
) {
    let dt = timer.time_step_in_seconds().clamp(0.0, 0.05);
    if dt <= 0.0 {
        return;
    }
    let now_ms = timer.time_in_milliseconds();
    for (ped_entity, identity, mut ped, vitality, mut locomotion, mut animation, seated) in
        &mut peds
    {
        if world.index.ped(identity.handle) != Some(ped_entity)
            || ped.char_created_by != RANDOM_CHAR
            || seated
            || vitality.health <= 0.0
        {
            continue;
        }
        let ped = &mut *ped;
        let locomotion = &mut *locomotion;
        let animation = &mut *animation;
        // Objective/vehicle callbacks may defer SetWanderPath until control is
        // restored. Consume that hand-off here; otherwise a released mission
        // actor can become RANDOM_CHAR while remaining frozen in PED_IDLE.
        if ped.flags.start_wander_path_on_foot && ped.is_ped_in_control(vitality) {
            let path = i32::from(ped.path_dir);
            apply_script_ped_wander_path(ped, vitality, path, &file_loader, &mut general, now_ms);
        }

        if let Some(sound) =
            land_new_ambient_ped_on_ground(ped, animation, &anim_manager, &file_loader, &world)
        {
            commands.entity(ped_entity).insert(PedAnimationSound(sound));
        }
        if ped.flags.is_landing {
            update_non_player_movement_animation(
                ped,
                locomotion,
                vitality,
                animation,
                &anim_manager,
                &file_loader,
                dt,
                false,
                None,
            );
            continue;
        }

        // `CPed::Flee` keeps a RANDOM_CHAR on a node seek until the timer has
        // elapsed and it is at least 30 m from the threat
        // (`peds/Ped.cpp:5628-5770`).  The direct target is only used when the
        // path graph cannot provide a next node; it is still swept against
        // authored COLs so a fleeing ped cannot tunnel through a car.
        if matches!(
            ped.ped_state,
            PedState::PED_FLEE_ENTITY | PedState::PED_FLEE_POS | PedState::PED_ON_FIRE
        ) {
            let source = ped
                .flee_from
                .and_then(|slot| world.vehicle_position(slot))
                .or_else(|| {
                    matches!(
                        ped.ped_state,
                        PedState::PED_FLEE_POS | PedState::PED_ON_FIRE
                    )
                    .then_some(CVector::new(
                        ped.flee_from_pos_x,
                        ped.flee_from_pos_y,
                        ped.physical.entity.get_position().z,
                    ))
                });
            let position = ped.physical.entity.get_position();
            if ped.flee_timer != 0 && now_ms > ped.flee_timer {
                let far_enough = source.is_none_or(|source| {
                    let dx = position.x - source.x;
                    let dy = position.y - source.y;
                    dx * dx + dy * dy >= 30.0 * 30.0
                });
                if far_enough && vitality.fire.is_none() {
                    ped.flee_from = None;
                    ped.flee_timer = 0;
                    ped.flags.use_ped_node_seek = false;
                    ped.next_path_node = None;
                    ped.ped_state = if ped.last_ped_state == PedState::PED_WANDER_PATH {
                        PedState::PED_WANDER_PATH
                    } else {
                        PedState::PED_IDLE
                    };
                    ped.move_state = if ped.ped_state == PedState::PED_WANDER_PATH {
                        eMoveState::PEDMOVE_WALK
                    } else {
                        eMoveState::PEDMOVE_STILL
                    };
                    ped.physical.move_speed = CVector::ZERO;
                    continue;
                }
                ped.flee_timer = now_ms.saturating_add(5_000);
            }

            let Some(source) = source else {
                ped.ped_state = PedState::PED_WANDER_PATH;
                ped.move_state = eMoveState::PEDMOVE_WALK;
                ped.flee_from = None;
                ped.flee_timer = 0;
                continue;
            };
            let mut away = CVector::new(position.x - source.x, position.y - source.y, 0.0);
            let away_len = (away.x * away.x + away.y * away.y).sqrt();
            if away_len <= f32::EPSILON {
                away = CVector::new(
                    -locomotion.rotation_cur.sin(),
                    locomotion.rotation_cur.cos(),
                    0.0,
                );
            } else {
                away.x /= away_len;
                away.y /= away_len;
            }
            let target = CVector::new(
                position.x + away.x * 4.0,
                position.y + away.y * 4.0,
                position.z,
            );
            let distance = (target.x - position.x).hypot(target.y - position.y);
            locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
                source.x, source.y, position.x, position.y,
            );
            let turn =
                CGeneral::limit_radian_angle(locomotion.rotation_dest - locomotion.rotation_cur);
            let max_turn = locomotion.heading_rate.to_radians() * dt * 50.0;
            locomotion.rotation_cur = CGeneral::limit_radian_angle(
                locomotion.rotation_cur + turn.clamp(-max_turn, max_turn),
            );
            ped.physical.entity.set_heading(locomotion.rotation_cur);
            ped.move_state = if ped.ped_state == PedState::PED_ON_FIRE {
                eMoveState::PEDMOVE_SPRINT
            } else {
                eMoveState::PEDMOVE_RUN
            };
            let local_delta = update_non_player_movement_animation(
                ped,
                locomotion,
                vitality,
                animation,
                &anim_manager,
                &file_loader,
                dt,
                false,
                None,
            );
            let (sin, cos) = locomotion.rotation_cur.sin_cos();
            let mut step_x = local_delta.x * cos - sin * local_delta.y;
            let mut step_y = local_delta.y * cos + local_delta.x * sin;
            let root_distance = step_x.hypot(step_y);
            if root_distance > distance && root_distance > f32::EPSILON {
                let scale = distance / root_distance;
                step_x *= scale;
                step_y *= scale;
            }
            let mut next_position =
                CVector::new(position.x + step_x, position.y + step_y, position.z);
            if let Some(ground) = file_loader.world.find_ground_z_for_3d_coord(
                next_position.x,
                next_position.y,
                next_position.z + 1.0,
                &file_loader.model_info,
            ) {
                next_position.z = ground + crate::peds::PedPlacement::PED_FEET_OFFSET;
            }
            let resolved = resolve_ped_static_motion(position, next_position, &file_loader);
            let resolved = if world.vehicle_position_blocked(resolved, &file_loader) {
                position
            } else {
                resolved
            };
            ped.physical.entity.flags.has_hit_wall = resolved != next_position;
            ped.physical.move_speed =
                crate::peds::PedLocomotion::grounded_velocity_from_displacement(
                    resolved - position,
                    timer.time_step(),
                );
            ped.physical.entity.set_position(resolved);
            ped.flags.is_in_the_air = false;
            ped.flags.is_standing = true;
            continue;
        }

        if matches!(
            ped.ped_state,
            PedState::PED_STEP_AWAY | PedState::PED_DIVE_AWAY
        ) {
            let delta = update_non_player_movement_animation(
                ped,
                locomotion,
                vitality,
                animation,
                &anim_manager,
                &file_loader,
                dt,
                false,
                None,
            );
            let position = ped.physical.entity.get_position();
            let (sin, cos) = locomotion.rotation_cur.sin_cos();
            let target = position
                + CVector::new(
                    delta.x * cos - sin * delta.y,
                    delta.y * cos + sin * delta.x,
                    0.0,
                );
            let resolved = resolve_ped_static_motion(position, target, &file_loader);
            ped.physical.move_speed =
                crate::peds::PedLocomotion::grounded_velocity_from_displacement(
                    resolved - position,
                    timer.time_step(),
                );
            ped.physical.entity.set_position(resolved);
            super::PedEvasion::service_evasion_finish(
                ped,
                vitality,
                animation,
                &file_loader,
                &mut general,
                now_ms,
            );
            continue;
        }
        if ped.char_created_by != RANDOM_CHAR
            || seated
            || vitality.health <= 0.0
            || ped.objective != eObjective::OBJECTIVE_NONE
            || ped.ped_state != PedState::PED_WANDER_PATH
        {
            continue;
        }

        let position = ped.physical.entity.get_position();
        let next_node = ped.next_path_node.or_else(|| {
            let requested_dir = (ped.path_dir as i32).rem_euclid(8) as u8;
            let (node, next_dir) = file_loader.path_find.find_next_node_wandering(
                crate::control::PathFind::PATH_PED,
                position,
                ped.last_path_node,
                requested_dir,
                |from, to| {
                    file_loader
                        .world
                        .process_line_of_sight_buildings_hit(
                            [from.x, from.y, from.z],
                            [to.x, to.y, to.z],
                            &file_loader.model_info,
                            None,
                        )
                        .is_none()
                },
            )?;
            ped.path_dir = next_dir as i16;
            Some(node)
        });
        let Some(next_node) = next_node else {
            bevy::log::warn_once!(
                target: "re3bevy::ped_ai",
                "ambient ped wander could not resolve a PATH_PED node"
            );
            ped.ped_state = PedState::PED_IDLE;
            ped.move_state = eMoveState::PEDMOVE_STILL;
            ped.physical.move_speed = CVector::ZERO;
            continue;
        };
        ped.next_path_node = Some(next_node);
        let Some(target_node) = file_loader.path_find.path_nodes.get(next_node).copied() else {
            bevy::log::warn_once!(
                target: "re3bevy::ped_ai",
                "ambient ped wander selected an out-of-range PATH_PED node"
            );
            ped.next_path_node = None;
            continue;
        };
        let dx = target_node.pos.x - position.x;
        let dy = target_node.pos.y - position.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= 0.75 {
            ped.last_path_node = Some(next_node);
            ped.next_path_node = None;
            continue;
        }

        let heading = CGeneral::get_radian_angle_between_points(
            target_node.pos.x,
            target_node.pos.y,
            position.x,
            position.y,
        );
        let turn = CGeneral::limit_radian_angle(heading - locomotion.rotation_cur);
        let max_turn = locomotion.heading_rate.to_radians() * dt * 50.0;
        locomotion.rotation_cur =
            CGeneral::limit_radian_angle(locomotion.rotation_cur + turn.clamp(-max_turn, max_turn));
        locomotion.rotation_dest = heading;
        ped.physical.entity.set_heading(locomotion.rotation_cur);

        ped.move_state = eMoveState::PEDMOVE_WALK;
        let local_delta = update_non_player_movement_animation(
            ped,
            locomotion,
            vitality,
            animation,
            &anim_manager,
            &file_loader,
            dt,
            false,
            None,
        );
        let (sin, cos) = locomotion.rotation_cur.sin_cos();
        let mut step_x = local_delta.x * cos - sin * local_delta.y;
        let mut step_y = local_delta.y * cos + local_delta.x * sin;
        let root_distance = step_x.hypot(step_y);
        if root_distance > distance && root_distance > f32::EPSILON {
            let scale = distance / root_distance;
            step_x *= scale;
            step_y *= scale;
        }
        let mut next_position = CVector::new(position.x + step_x, position.y + step_y, position.z);
        if let Some(ground) = file_loader.world.find_ground_z_for_3d_coord(
            next_position.x,
            next_position.y,
            next_position.z + 1.0,
            &file_loader.model_info,
        ) {
            next_position.z = ground + crate::peds::PedPlacement::PED_FEET_OFFSET;
        }
        let resolved = resolve_ped_static_motion(position, next_position, &file_loader);
        ped.physical.entity.flags.has_hit_wall = resolved != next_position;
        ped.physical.move_speed = crate::peds::PedLocomotion::grounded_velocity_from_displacement(
            resolved - position,
            timer.time_step(),
        );
        ped.physical.entity.set_position(resolved);
        ped.flags.is_in_the_air = false;
        ped.flags.is_standing = true;
    }
}

