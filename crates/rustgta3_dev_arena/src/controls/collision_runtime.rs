//! Live arena collision over canonical actors and derived sector membership.
use rustgta3_game::entities::EntityReference::EntityReference;
use super::*;
struct CollisionDamageContact {
    impulse: f32,
    piece: u8,
    normal: CVector,
}
/// Arena-side `CWorld::ProcessCollisionSectorList` for automobile pairs. The
/// narrow phase uses both authored vehicle COL models and feeds the ordinary
/// re3 reciprocal impulse, friction, shift and component-damage paths.
pub(crate) fn resolve_arena_vehicle_collisions(
    time: Res<Time>,
    original: Res<ArenaOriginalRuntime>,
    timer: Option<Res<CTimer>>,
    mut random: ResMut<crate::ArenaRandomState>,
    mut spatial: ResMut<crate::vehicle_spatial::ArenaVehicleSpatial>,
    mut scratch: Local<crate::vehicle_spatial::ArenaCollisionScratch>,
    mut vehicles: Query<(
        Entity,
        &mut Transform,
        &mut ArenaVehicle,
        ArenaAutomobileRuntime,
        &VehicleCollisionState,
    )>,
) {
    let Some(data) = original.data.as_ref() else {
        if !vehicles.is_empty() {
            warn_once!("arena_collision_missing_original_data");
        }
        return;
    };
    let time_step = (time.delta_secs().clamp(0.0, 0.05) * 50.0).max(0.0);
    let time_ms = timer.as_deref().map_or(0, CTimer::time_in_milliseconds);
    scratch.sources.clear();
    scratch.sources.extend(spatial.grid.entities());
    // TODO(parity): CWorld::Process moving-list/sector insertion ordering.
    scratch.sources.sort_by_key(|entity| entity.to_bits());
    scratch.visited.clear();
    let crate::vehicle_spatial::ArenaCollisionScratch {
        sources,
        candidates,
        refresh,
        visited,
        queued,
    } = &mut *scratch;
    for &entity_a in sources.iter() {
        let Ok((_, _, _, source, _)) = vehicles.get(entity_a) else {
            continue;
        };
        let Some(source_rect) = crate::vehicle_spatial::live_vehicle_rect(
            source.into_view().car.vehicle.core,
            &data.file_loader,
        ) else {
            warn_once!("arena_collision_waiting_for_authored_bounds");
            continue;
        };
        if !spatial.grid.candidates_rect(source_rect, candidates) {
            warn_once!("arena_collision_invalid_bound_rect");
            continue;
        }
        queued.clear();
        queued.extend(candidates.iter().copied());
        let mut cursor = 0;
        while cursor < candidates.len() {
            let entity_b = candidates[cursor];
            cursor += 1;
            if entity_a == entity_b
                || !visited.insert(crate::vehicle_spatial::CollisionPair::new(
                    entity_a, entity_b,
                ))
            {
                continue;
            }
            spatial.candidate_visits = spatial.candidate_visits.saturating_add(1);
            let (Ok((_, _, _, a, _)), Ok((_, _, _, b, _))) =
                (vehicles.get(entity_a), vehicles.get(entity_b))
            else {
                warn_once!("arena_collision_candidate_retired");
                continue;
            };
            let a = a.into_view();
            let b = b.into_view();
            if !a.car.vehicle.core.physical.entity.flags.uses_collision
                || !b.car.vehicle.core.physical.entity.flags.uses_collision {
                continue;
            }
            let (Some(sphere_a), Some(sphere_b)) = (
                crate::vehicle_spatial::live_vehicle_sphere(a.car.vehicle.core, &data.file_loader),
                crate::vehicle_spatial::live_vehicle_sphere(b.car.vehicle.core, &data.file_loader),
            ) else {
                warn_once!("arena_collision_waiting_for_authored_bounds");
                continue;
            };
            let matrix_a = a.car.vehicle.core.physical.entity.matrix;
            let matrix_b = b.car.vehicle.core.physical.entity.matrix;
            let model_a_id = sphere_a.model;
            let model_b_id = sphere_b.model;
            let delta = CVector::new(
                sphere_a.center.x - sphere_b.center.x,
                sphere_a.center.y - sphere_b.center.y,
                sphere_a.center.z - sphere_b.center.z,
            );
            if delta.magnitude_sqr() > (sphere_a.radius + sphere_b.radius).powi(2) {
                continue;
            }
            let (Some(model_a), Some(model_b)) = (
                data.file_loader.model_info.col_model(model_a_id),
                data.file_loader.model_info.col_model(model_b_id),
            ) else {
                warn_once!("arena_collision_waiting_for_authored_col");
                continue;
            };
            let Ok(
                [
                    (_, mut transform_a, mut arena_a, runtime_a, history_a),
                    (_, mut transform_b, mut arena_b, runtime_b, history_b),
                ],
            ) = vehicles.get_many_mut([entity_a, entity_b])
            else {
                warn_once!("arena_collision_pair_lifecycle_changed");
                continue;
            };
            let mut runtime_a = runtime_a.into_view();
            let mut runtime_b = runtime_b.into_view();
            if runtime_a.car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
                runtime_a.car.vehicle.core.flags.vehicle_col_processed = true;
            }
            if runtime_b.car.vehicle.core.physical.entity.status != eEntityStatus::STATUS_SIMPLE {
                runtime_b.car.vehicle.core.flags.vehicle_col_processed = true;
            }
            // TODO(parity): CAutomobile::ProcessEntityCollision wheel-only
            // contact/ground references and special COL selection.
            let contacts =
                CCollision::process_col_models_spheres(&matrix_a, model_a, &matrix_b, model_b);
            if contacts.is_empty() { continue; }
            // CAutomobile::ProcessEntityCollision records body contact before
            // ApplyCollision; resting contacts do not require an impulse.
            runtime_a.car.vehicle.core.physical.add_collision_record(EntityReference::Entity(entity_b), time_ms);
            runtime_b.car.vehicle.core.physical.add_collision_record(EntityReference::Entity(entity_a), time_ms);

            let mut strongest_a: Option<CollisionDamageContact> = None;
            let mut strongest_b: Option<CollisionDamageContact> = None;
            let mut response_count = 0_u32;
            for point in &contacts {
                let response = runtime_a.car.vehicle.physical.apply_collision_dynamic(
                    &mut runtime_b.car.vehicle.physical,
                    point,
                    CPhysicalCollisionOptions {
                        time_step,
                        ..Default::default()
                    },
                );
                if !response.applied {
                    continue;
                }
                response_count += 1;
                let normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                if strongest_a
                    .as_ref()
                    .is_none_or(|contact| response.impulse_a > contact.impulse)
                {
                    strongest_a = Some(CollisionDamageContact {
                        impulse: response.impulse_a,
                        piece: point.piece_a,
                        normal,
                    });
                }
                if strongest_b
                    .as_ref()
                    .is_none_or(|contact| response.impulse_b > contact.impulse)
                {
                    strongest_b = Some(CollisionDamageContact {
                        impulse: response.impulse_b,
                        piece: point.piece_b,
                        normal,
                    });
                }
                let adhesion = runtime_a.surface_adhesion.min(runtime_b.surface_adhesion)
                    / contacts.len() as f32;
                if runtime_a
                    .car
                    .vehicle
                    .physical
                    .apply_collision_friction_between(
                        &mut runtime_b.car.vehicle.physical,
                        adhesion,
                        point,
                        time_step,
                    )
                    .applied
                {
                    runtime_a.car.vehicle.physical.entity.flags.has_contacted = true;
                    runtime_b.car.vehicle.physical.entity.flags.has_contacted = true;
                }
            }

            if response_count == 0 {
                continue;
            }
            // CPhysical::ProcessCollision rolls the integrated matrices back
            // after an impulse response. ProcessShift belongs to CWorld's
            // later stuck-body pass, not every ordinary vehicle contact.
            let previous_a = history_a.previous_matrix;
            let previous_b = history_b.previous_matrix;
            runtime_a.car.vehicle.physical.entity.set_matrix(previous_a);
            runtime_b.car.vehicle.physical.entity.set_matrix(previous_b);
            // CWorld::ProcessShift first replays the rejected integration step,
            // then separates each physical sequentially. Rolling both cars
            // back without this pass leaves them suspended at the impact pose.
            runtime_a.car.vehicle.physical.apply_move_speed(time_step);
            runtime_a.car.vehicle.physical.apply_turn_speed(time_step);
            runtime_a
                .car
                .vehicle
                .physical
                .entity
                .matrix
                .reorthogonalise();
            runtime_b.car.vehicle.physical.apply_move_speed(time_step);
            runtime_b.car.vehicle.physical.apply_turn_speed(time_step);
            runtime_b
                .car
                .vehicle
                .physical
                .entity
                .matrix
                .reorthogonalise();
            let before_shift_a = runtime_a.car.vehicle.physical.entity.matrix;
            let contacts_a = CCollision::process_col_models_spheres(
                &before_shift_a,
                model_a,
                &runtime_b.car.vehicle.physical.entity.matrix,
                model_b,
            );
            arena_vehicle_pair_shift(
                &mut runtime_a.car.vehicle.core.physical,
                &runtime_b.car.vehicle.core.physical,
                &contacts_a,
                false,
            );
            let still_a = CCollision::process_col_models_spheres(
                &runtime_a.car.vehicle.physical.entity.matrix,
                model_a,
                &runtime_b.car.vehicle.physical.entity.matrix,
                model_b,
            );
            if !still_a.is_empty() {
                runtime_a
                    .car
                    .vehicle
                    .physical
                    .entity
                    .set_matrix(before_shift_a);
                arena_vehicle_pair_shift(
                    &mut runtime_a.car.vehicle.core.physical,
                    &runtime_b.car.vehicle.core.physical,
                    &still_a,
                    true,
                );
            }
            let before_shift_b = runtime_b.car.vehicle.physical.entity.matrix;
            let contacts_b = CCollision::process_col_models_spheres(
                &before_shift_b,
                model_b,
                &runtime_a.car.vehicle.physical.entity.matrix,
                model_a,
            );
            arena_vehicle_pair_shift(
                &mut runtime_b.car.vehicle.core.physical,
                &runtime_a.car.vehicle.core.physical,
                &contacts_b,
                false,
            );
            let still_b = CCollision::process_col_models_spheres(
                &runtime_b.car.vehicle.physical.entity.matrix,
                model_b,
                &runtime_a.car.vehicle.physical.entity.matrix,
                model_a,
            );
            if !still_b.is_empty() {
                runtime_b
                    .car
                    .vehicle
                    .physical
                    .entity
                    .set_matrix(before_shift_b);
                arena_vehicle_pair_shift(
                    &mut runtime_b.car.vehicle.core.physical,
                    &runtime_a.car.vehicle.core.physical,
                    &still_b,
                    true,
                );
            }
            if let Some(CollisionDamageContact {
                impulse,
                piece,
                normal,
            }) = strongest_a
            {
                runtime_a.car.vehicle.physical.set_damaged_piece_record(
                    piece as u16,
                    impulse,
                    Some(EntityReference::Entity(entity_b)),
                    normal,
                );
                if let Some(piece) = eCarPiece::from_piece_id(piece) {
                    let multiplier = runtime_a.handling.collision_damage_multiplier;
                    let damage_nodes = rustgta3_game::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(
                        &data.file_loader, runtime_a.car.vehicle.core.physical.entity.model_index);
                    runtime_a.car.apply_collision_piece_damage(
                        impulse,
                        piece,
                        multiplier,
                        &mut random.0,
                        damage_nodes,
                    );
                }
            }
            if let Some(CollisionDamageContact {
                impulse,
                piece,
                normal,
            }) = strongest_b
            {
                runtime_b.car.vehicle.physical.set_damaged_piece_record(
                    piece as u16,
                    impulse,
                    Some(EntityReference::Entity(entity_a)),
                    normal,
                );
                if let Some(piece) = eCarPiece::from_piece_id(piece) {
                    let multiplier = runtime_b.handling.collision_damage_multiplier;
                    let damage_nodes = rustgta3_game::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(
                        &data.file_loader, runtime_b.car.vehicle.core.physical.entity.model_index);
                    runtime_b.car.apply_collision_piece_damage(
                        impulse,
                        piece,
                        multiplier,
                        &mut random.0,
                        damage_nodes,
                    );
                }
            }
            let telemetry_a = automobile_telemetry(&runtime_a.car.as_ref());
            let telemetry_b = automobile_telemetry(&runtime_b.car.as_ref());
            *transform_a = telemetry_a.transform;
            *transform_b = telemetry_b.transform;
            arena_a.speed = telemetry_a.speed_mps;
            arena_b.speed = telemetry_b.speed_mps;
            spatial.refresh(entity_a, runtime_a.car.vehicle.core, &data.file_loader);
            spatial.refresh(entity_b, runtime_b.car.vehicle.core, &data.file_loader);
            if let Some(rect) = crate::vehicle_spatial::live_vehicle_rect(
                runtime_a.car.vehicle.core,
                &data.file_loader,
            ) {
                if !spatial.grid.candidates_rect(rect, refresh) {
                    warn_once!("arena_collision_invalid_bound_rect");
                    continue;
                }
                for &entity in refresh.iter() {
                    if queued.insert(entity) {
                        candidates.push(entity);
                    }
                }
            }
        }
    }
}

fn arena_vehicle_pair_shift(
    vehicle: &mut rustgta3_game::entities::Physical::CPhysical,
    other: &rustgta3_game::entities::Physical::CPhysical,
    contacts: &[rustgta3_game::collision::ColPoint::CColPoint],
    second_shift: bool,
) {
    if contacts.is_empty() {
        return;
    }
    let scale = if second_shift { 1.5 } else { 1.2 } / contacts.len() as f32;
    let mut shift = CVector::ZERO;
    for point in contacts {
        shift.x += point.normal.x * point.depth * scale;
        shift.y += point.normal.y * point.depth * scale;
        shift.z += point.normal.z * point.depth * scale;
    }
    if let Some(deepest) = contacts.iter().max_by(|a, b| a.depth.total_cmp(&b.depth)) {
        let a = vehicle.entity.get_position();
        let b = other.entity.get_position();
        let mut dir = CVector::new(a.x - b.x, a.y - b.y, a.z - b.z);
        let len = dir.magnitude();
        if len > 0.0 {
            dir.x /= len;
            dir.y /= len;
            dir.z /= len;
            let fz = vehicle.entity.get_forward().z;
            let rz = vehicle.entity.get_right().z;
            if dir.z < 0.0 && dir.z < fz && dir.z < rz {
                dir.z = 0.0_f32.min(fz.min(rz));
            }
            let amount = deepest.depth * 0.5;
            shift.x += dir.x * amount;
            shift.y += dir.y * amount;
            shift.z += dir.z * amount;
        }
    }
    vehicle.apply_process_shift(shift);
    vehicle.entity.flags.is_stuck = false;
    vehicle.entity.flags.is_in_safe_position = true;
}
