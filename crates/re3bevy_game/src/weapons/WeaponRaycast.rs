//! Shared weapon line queries over borrowed live actors and original COL data.

use re3bevy_core::core::Coordinates::CVector;

use crate::{
    collision::ColModel::{CColLine, CColPoint, CColVector},
    control::ScriptEntities::ScriptEntityWorld,
    core::FileLoader::CFileLoader,
    entities::Entity::CEntity,
    math::Matrix::CMatrix,
    modelinfo::ModelIndices::MI_PLAYER,
    peds::{Ped::CPed, PedAnimation::PedAnimation, PlayerPed::PlayerPedRef},
};

/// Ray queries need only poses, collision flags and ped hit animations. The
/// access implementation decides how canonical identities resolve; the query
/// never requests actor mutation, constructor buffers or copied world state.
pub(crate) trait WeaponRaycastActors {
    fn ray_peds(&self) -> impl Iterator<Item = (i32, &CPed, &PedAnimation)>;
    fn ray_vehicles(&self) -> impl Iterator<Item = (i32, &CEntity)>;
    fn ray_objects(&self) -> impl Iterator<Item = (i32, &CEntity)>;
}

/// Compatibility callers borrow their existing views; combat systems use the
/// narrower WeaponActorAccess implementation of the same query contract.
impl WeaponRaycastActors for ScriptEntityWorld<'_, '_> {
    fn ray_peds(&self) -> impl Iterator<Item = (i32, &CPed, &PedAnimation)> {
        self.peds
            .iter()
            .map(|(&handle, actor)| (handle, actor.ped, actor.animation))
    }

    fn ray_vehicles(&self) -> impl Iterator<Item = (i32, &CEntity)> {
        self.vehicles.iter().map(|(&handle, vehicle)| {
            let core = vehicle.base().core;
            (handle, &core.physical.entity)
        })
    }

    fn ray_objects(&self) -> impl Iterator<Item = (i32, &CEntity)> {
        self.objects
            .iter()
            .map(|actor| (actor.handle, &actor.object.physical.entity))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WeaponRayHit {
    Player {
        distance: f32,
        point: CVector,
    },
    Ped {
        handle: i32,
        distance: f32,
        point: CVector,
    },
    Vehicle {
        handle: i32,
        distance: f32,
        point: CVector,
        normal: CVector,
    },
    Object {
        handle: i32,
        distance: f32,
        point: CVector,
        normal: CVector,
    },
    Static {
        entity_index: usize,
        distance: f32,
        point: CVector,
        normal: CVector,
    },
}

impl WeaponRayHit {
    fn distance(self) -> f32 {
        match self {
            Self::Player { distance, .. }
            | Self::Ped { distance, .. }
            | Self::Vehicle { distance, .. }
            | Self::Object { distance, .. }
            | Self::Static { distance, .. } => distance,
        }
    }
}

pub(crate) fn weapon_col_model_hit(
    source: CVector,
    target: CVector,
    matrix: &CMatrix,
    model_index: usize,
    uses_collision: bool,
    file_loader: &CFileLoader,
) -> Option<(f32, CVector, CVector)> {
    if !uses_collision {
        return None;
    }
    let col_model = file_loader.model_info.col_model(model_index)?;
    weapon_collision_model_hit(source, target, matrix, col_model)
}

fn weapon_collision_model_hit(
    source: CVector,
    target: CVector,
    matrix: &CMatrix,
    col_model: &crate::collision::ColModel::CColModel,
) -> Option<(f32, CVector, CVector)> {
    let local_source = matrix.multiply_inverse_point(source);
    let local_target = matrix.multiply_inverse_point(target);
    let line = CColLine::new(
        CColVector::new(local_source.x, local_source.y, local_source.z),
        CColVector::new(local_target.x, local_target.y, local_target.z),
    );
    let mut col_point = CColPoint::new();
    let mut fraction = 1.0;
    if !col_model.process_line_of_sight_primitives(&line, &mut col_point, &mut fraction) {
        return None;
    }
    let point = matrix.transform_point(CVector::new(
        col_point.point.x,
        col_point.point.y,
        col_point.point.z,
    ));
    let local_normal = CVector::new(col_point.normal.x, col_point.normal.y, col_point.normal.z);
    let world_normal = matrix.multiply3x3(local_normal);
    let normal_length = world_normal.magnitude();
    let normal = if normal_length > 1.0e-6 {
        CVector::new(
            world_normal.x / normal_length,
            world_normal.y / normal_length,
            world_normal.z / normal_length,
        )
    } else {
        CVector::ZERO
    };
    let delta = CVector::new(point.x - source.x, point.y - source.y, point.z - source.z);
    Some((delta.magnitude(), point, normal))
}

fn weapon_ped_model_hit(
    source: CVector,
    target: CVector,
    ped: &CPed,
    animation: &crate::peds::PedAnimation::PedAnimation,
    model_index: usize,
    file_loader: &CFileLoader,
) -> Option<(f32, CVector, CVector)> {
    if !ped.physical.entity.flags.uses_collision {
        return None;
    }
    if ped.use_ground_col_model() {
        return weapon_collision_model_hit(
            source,
            target,
            &ped.physical.entity.matrix,
            &file_loader.temp_col_models.col_model_ped_ground_hit,
        );
    }
    let model = crate::renderer::BevyRender::ped_weapon_hit_model(
        model_index,
        file_loader,
        &animation.movement_anim_clump,
    );
    let Some(model) = model else {
        bevy::log::warn_once!("weapon_ray_waiting_original_ped_hit_model: model={model_index}");
        return None;
    };
    weapon_collision_model_hit(source, target, &ped.physical.entity.matrix, &model)
}

pub(crate) fn closest_weapon_ray_hit(
    source: CVector,
    direction: CVector,
    max_range: f32,
    player_target: Option<PlayerPedRef<'_>>,
    excluded_ped_handle: Option<i32>,
    excluded_vehicle_handle: Option<i32>,
    actors: &impl WeaponRaycastActors,
    file_loader: &CFileLoader,
) -> Option<WeaponRayHit> {
    let target = CVector::new(
        source.x + direction.x * max_range,
        source.y + direction.y * max_range,
        source.z + direction.z * max_range,
    );
    let mut best = file_loader
        .world
        .process_line_of_sight_static_contact(
            [source.x, source.y, source.z],
            [target.x, target.y, target.z],
            &file_loader.model_info,
            None,
            true,
        )
        .map(|contact| {
            let point = CVector::new(contact.point[0], contact.point[1], contact.point[2]);
            let dx = point.x - source.x;
            let dy = point.y - source.y;
            let dz = point.z - source.z;
            WeaponRayHit::Static {
                entity_index: contact.entity_index,
                distance: (dx * dx + dy * dy + dz * dz).sqrt(),
                point,
                normal: CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]),
            }
        });

    if let Some(other) = player_target {
        let model_index = if other.ped.physical.entity.model_index <= 0 {
            MI_PLAYER
        } else {
            other.ped.physical.entity.model_index as usize
        };
        if let Some((distance, point, _)) = weapon_ped_model_hit(
            source,
            target,
            other.ped,
            other.animation,
            model_index,
            file_loader,
        ) && best.is_none_or(|hit| distance < hit.distance())
        {
            best = Some(WeaponRayHit::Player { distance, point });
        }
    }

    for (handle, ped, animation) in actors.ray_peds() {
        if Some(handle) == excluded_ped_handle {
            continue;
        }
        let model_index = ped.physical.entity.model_index.max(0) as usize;
        if let Some((distance, point, _)) =
            weapon_ped_model_hit(source, target, ped, animation, model_index, file_loader)
            && best.is_none_or(|hit| distance < hit.distance())
        {
            best = Some(WeaponRayHit::Ped {
                handle,
                distance,
                point,
            });
        }
    }

    for (handle, entity) in actors.ray_vehicles() {
        if Some(handle) == excluded_vehicle_handle {
            continue;
        }
        let model_index = entity.model_index.max(0) as usize;
        if let Some((distance, point, normal)) = weapon_col_model_hit(
            source,
            target,
            &entity.matrix,
            model_index,
            entity.flags.uses_collision,
            file_loader,
        ) && best.is_none_or(|hit| distance < hit.distance())
        {
            best = Some(WeaponRayHit::Vehicle {
                handle,
                distance,
                point,
                normal,
            });
        }
    }
    for (handle, entity) in actors.ray_objects() {
        let model_index = entity.model_index.max(0) as usize;
        if let Some((distance, point, normal)) = weapon_col_model_hit(
            source,
            target,
            &entity.matrix,
            model_index,
            entity.flags.uses_collision,
            file_loader,
        ) && best.is_none_or(|hit| distance < hit.distance())
        {
            best = Some(WeaponRayHit::Object {
                handle,
                distance,
                point,
                normal,
            });
        }
    }
    best
}
