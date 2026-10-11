//! Vehicle body probes at the native CPhysical partial timesteps.
use super::{
    ColModel::{CColModel, CColPoint},
    Collision::{CCollision, partial_physical_matrix},
};
use crate::{
    entities::{
        Entity::eEntityStatus,
        Physical::{CPhysical, CPhysicalCollisionOptions},
    },
    math::{Matrix::CMatrix, Rect::CRect},
};
use re3bevy_core::core::Coordinates::CVector;

pub(crate) struct VehiclePairContact {
    pub matrix: CMatrix,
    pub time_step: f32,
    pub contacts: Vec<CColPoint>,
    pub endpoint_missed: bool,
}

pub(crate) struct VehiclePairProbe {
    pub contact: Option<VehiclePairContact>,
    pub contacts_seen: bool,
}

#[derive(Default, bevy::prelude::Resource)]
pub struct VehiclePairDiagnostics {
    pub partial_contacts: u64,
    pub contacts_missed_by_endpoint: u64,
    pub last_endpoint_miss_pair: Option<(i32, i32)>,
}

fn can_probe_motion(physical: &CPhysical, previous: Option<CMatrix>, responded: bool) -> bool {
    previous.is_some()
        && !responded
        && !physical.entity.flags.has_hit_wall
        && physical.entity.status != eEntityStatus::STATUS_SIMPLE
}

fn bound_at(matrix: &CMatrix, model: &CColModel) -> CRect {
    let sphere = &model.bounding_sphere;
    let center = matrix.transform_point(CVector::new(
        sphere.center.x,
        sphere.center.y,
        sphere.center.z,
    ));
    CRect::new(
        center.x - sphere.radius,
        center.y - sphere.radius,
        center.x + sphere.radius,
        center.y + sphere.radius,
    )
}

/// The index contains only Entity membership. Query the source's covered
/// trajectory, then borrow each counterpart's current pose for narrow phase.
pub(crate) fn candidate_bounds(
    physical: &CPhysical,
    model: &CColModel,
    previous: Option<CMatrix>,
    responded: bool,
    time_step: f32,
) -> CRect {
    let mut bounds = physical.get_bound_rect(model);
    if can_probe_motion(physical, previous, responded) {
        let previous = previous.expect("admitted preintegration pose");
        bounds.ContainRect(bound_at(&previous, model));
        let end = partial_physical_matrix(
            previous,
            physical.move_speed,
            physical.turn_speed,
            time_step,
        );
        bounds.ContainRect(bound_at(&end, model));
    }
    bounds
}

/// Check only A's native partial poses against B's live pose. B gets its own
/// turn as a source if A found no contact; do not invent a simultaneous sweep.
/// Static rejection and prior pair response already consumed A's original
/// motion in this split pipeline, so those paths use their current live pose.
pub(crate) fn first_contact(
    source: &CPhysical,
    source_model: &CColModel,
    previous: Option<CMatrix>,
    responded: bool,
    other: &CPhysical,
    other_model: &CColModel,
    time_step: f32,
) -> VehiclePairProbe {
    let mut contacts_seen = false;
    if can_probe_motion(source, previous, responded) {
        let previous = previous.expect("admitted preintegration pose");
        let steps = source.vehicle_collision_steps(time_step);
        for step in 1..steps {
            let partial_step = time_step * step as f32 / steps as f32;
            let matrix = partial_physical_matrix(
                previous,
                source.move_speed,
                source.turn_speed,
                partial_step,
            );
            let contacts = CCollision::process_col_models_spheres(
                &matrix,
                source_model,
                &other.entity.matrix,
                other_model,
            );
            if !contacts.is_empty() {
                contacts_seen = true;
                // CheckCollision rejects a partial pose only when the
                // manifold produces a response. Separating contacts still
                // publish history, but must not stop the remaining probes.
                let mut source_probe = source.clone();
                source_probe.entity.set_matrix(matrix);
                let mut other_probe = other.clone();
                if !contacts.iter().any(|point| {
                    source_probe
                        .apply_collision_dynamic(
                            &mut other_probe,
                            point,
                            CPhysicalCollisionOptions {
                                time_step: partial_step,
                                ..Default::default()
                            },
                        )
                        .applied
                }) {
                    continue;
                }
                let endpoint_missed = CCollision::process_col_models_spheres(
                    &source.entity.matrix,
                    source_model,
                    &other.entity.matrix,
                    other_model,
                )
                .is_empty();
                return VehiclePairProbe {
                    contacts_seen,
                    contact: Some(VehiclePairContact {
                        matrix,
                        time_step: partial_step,
                        contacts,
                        endpoint_missed,
                    }),
                };
            }
        }
    }
    let matrix = source.entity.matrix;
    let contacts = CCollision::process_col_models_spheres(
        &matrix,
        source_model,
        &other.entity.matrix,
        other_model,
    );
    contacts_seen |= !contacts.is_empty();
    VehiclePairProbe {
        contacts_seen,
        contact: (!contacts.is_empty()).then_some(VehiclePairContact {
            matrix,
            time_step,
            contacts,
            endpoint_missed: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::ColModel::{CColBox, CColSphere, CColVector};
    use re3bevy_core::core::General::CGeneral;

    fn fixture() -> (CPhysical, CPhysical, CColModel, CMatrix) {
        let mut general = CGeneral::with_seed(7);
        let mut source = CPhysical::new(&mut general);
        source.set_entity_type(crate::entities::Entity::eEntityType::ENTITY_TYPE_VEHICLE);
        source.mass = 1000.0;
        source.turn_mass = 1500.0;
        source.entity.status = eEntityStatus::STATUS_PLAYER;
        source.move_speed = CVector::new(2.0, 0.0, 0.0);
        source.entity.matrix.position.x = 1.0;
        let mut previous = source.entity.matrix;
        previous.position.x = -1.0;
        let mut other = CPhysical::new(&mut general);
        other.set_entity_type(crate::entities::Entity::eEntityType::ENTITY_TYPE_VEHICLE);
        other.mass = source.mass;
        other.turn_mass = source.turn_mass;
        let model = CColModel::new(
            CColSphere::new(0.2, CColVector::new(0.0, 0.0, 0.0), 10, 0),
            CColBox::new(
                CColVector::new(-0.2, -0.2, -0.2),
                CColVector::new(0.2, 0.2, 0.2),
                10,
                0,
            ),
            vec![CColSphere::new(0.2, CColVector::new(0.0, 0.0, 0.0), 10, 0)],
        );
        (source, other, model, previous)
    }

    #[test]
    fn crossing_without_endpoint_overlap_produces_an_actual_partial_col_contact() {
        let (source, other, model, previous) = fixture();
        assert!(
            CCollision::process_col_models_spheres(
                &source.entity.matrix,
                &model,
                &other.entity.matrix,
                &model
            )
            .is_empty()
        );
        let contact = first_contact(&source, &model, Some(previous), false, &other, &model, 1.0)
            .contact
            .expect("intermediate native COL contact");
        assert!(contact.time_step < 1.0 && contact.matrix.position.x < 0.0);
        assert!(!contact.contacts.is_empty());
        let bounds = candidate_bounds(&source, &model, Some(previous), false, 1.0);
        assert!(bounds.left <= -1.2 && bounds.right >= 1.2);
    }

    #[test]
    fn rejected_or_already_responded_motion_does_not_resweep_the_old_path() {
        let (mut source, other, model, previous) = fixture();
        assert!(
            first_contact(&source, &model, Some(previous), true, &other, &model, 1.0)
                .contact
                .is_none()
        );
        source.entity.flags.has_hit_wall = true;
        assert!(
            first_contact(&source, &model, Some(previous), false, &other, &model, 1.0)
                .contact
                .is_none()
        );
        source.entity.flags.has_hit_wall = false;
        source.entity.status = eEntityStatus::STATUS_SIMPLE;
        assert!(
            first_contact(&source, &model, Some(previous), false, &other, &model, 1.0)
                .contact
                .is_none()
        );
    }

    #[test]
    fn separating_partial_contact_keeps_history_without_rejecting_the_end_pose() {
        let (mut source, other, model, mut previous) = fixture();
        previous.position.x = 0.0;
        source.entity.matrix.position.x = 2.0;
        let probe = first_contact(&source, &model, Some(previous), false, &other, &model, 1.0);
        assert!(probe.contacts_seen, "native partial contact history");
        assert!(
            probe.contact.is_none(),
            "separating manifold must not reject integration"
        );
    }
}
