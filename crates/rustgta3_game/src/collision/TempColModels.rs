//! re3 reference: references/re-GTA/src/collision/TempColModels.cpp
//! re3 reference: references/re-GTA/src/collision/TempColModels.h
//!
//! Temporary collision models initialised by CTempColModels::Initialise.

use crate::collision::ColModel::{
    CColBox, CColModel, CColSphere, CColVector, SURFACE_CAR_PANEL, SURFACE_PED, SURFACE_WHEELBASE,
};
pub const PED_BODY_RADIUS: f32 = 0.35;
pub const PED_BODY_MIN_Z: f32 = -1.0;
pub const PED_BODY_MAX_Z: f32 = 0.9;

#[derive(Debug, Clone)]
pub struct CTempColModels {
    pub col_model_bumper1: CColModel,
    pub col_model_wheel1: CColModel,
    pub col_model_panel1: CColModel,
    pub col_model_body_part2: CColModel,
    pub col_model_body_part1: CColModel,
    pub col_model_boot1: CColModel,
    pub col_model_door1: CColModel,
    pub col_model_bonnet1: CColModel,
    /// re3 `CTempColModels::ms_colModelPed1` — the three-sphere ped body used
    /// for ped-vs-vehicle/world collision regardless of the loaded ped DFF.
    pub col_model_ped1: CColModel,
    /// re3 ms_colModelPedGroundHit — four fore/aft spheres used while a ped
    /// is fallen/dying/dead so uneven terrain contacts follow the lying body.
    pub col_model_ped_ground_hit: CColModel,
}

impl CTempColModels {
    pub fn initialise() -> Self {
        Self {
            col_model_bumper1: bumper1(),
            col_model_wheel1: wheel1(),
            col_model_panel1: panel1(),
            col_model_body_part2: body_part2(),
            col_model_body_part1: body_part1(),
            col_model_boot1: boot1(),
            col_model_door1: door1(),
            col_model_bonnet1: bonnet1(),
            col_model_ped1: ped1(),
            col_model_ped_ground_hit: ped_ground_hit(),
        }
    }

    pub fn model_for_kind(
        &self,
        kind: crate::modelinfo::ModelInfo::DefaultColModelKind,
    ) -> &CColModel {
        match kind {
            crate::modelinfo::ModelInfo::DefaultColModelKind::Door => &self.col_model_door1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::Bumper => &self.col_model_bumper1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::Panel => &self.col_model_panel1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::Bonnet => &self.col_model_bonnet1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::Boot => &self.col_model_boot1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::Wheel => &self.col_model_wheel1,
            crate::modelinfo::ModelInfo::DefaultColModelKind::BodyPartA => {
                &self.col_model_body_part1
            }
            crate::modelinfo::ModelInfo::DefaultColModelKind::BodyPartB => {
                &self.col_model_body_part2
            }
        }
    }
}

fn v(x: f32, y: f32, z: f32) -> CColVector {
    CColVector::new(x, y, z)
}

fn sphere(radius: f32, center: CColVector, surface: u8) -> CColSphere {
    CColSphere::new(radius, center, surface, 0)
}

fn bounds(
    radius: f32,
    center: CColVector,
    min: CColVector,
    max: CColVector,
) -> (CColSphere, CColBox) {
    (
        CColSphere::new(radius, center, 0, 0),
        CColBox::new(min, max, 0, 0),
    )
}

fn door1() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        1.5,
        v(0.0, -0.6, 0.0),
        v(-0.3, 0.0, -0.6),
        v(0.3, -1.2, 0.6),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.15, v(0.0, -0.25, -0.35), SURFACE_CAR_PANEL),
            sphere(0.15, v(0.0, -0.95, -0.35), SURFACE_CAR_PANEL),
            sphere(0.25, v(0.0, -0.6, 0.25), SURFACE_CAR_PANEL),
        ],
    )
}

fn bumper1() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        2.2,
        v(0.0, -0.6, 0.0),
        v(-1.2, -0.3, -0.2),
        v(1.2, 0.3, 0.2),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.15, v(0.85, -0.05, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(0.4, 0.05, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(-0.4, 0.05, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(-0.85, -0.05, 0.0), SURFACE_CAR_PANEL),
        ],
    )
}

fn panel1() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        1.4,
        v(0.0, 0.0, 0.0),
        v(-0.3, -0.6, -0.15),
        v(0.3, 0.6, 0.15),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.15, v(0.15, 0.45, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(0.15, -0.45, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(-0.15, -0.45, 0.0), SURFACE_CAR_PANEL),
            sphere(0.15, v(-0.15, 0.45, 0.0), SURFACE_CAR_PANEL),
        ],
    )
}

fn bonnet1() -> CColModel {
    let (bounding_sphere, bounding_box) =
        bounds(1.7, v(0.0, 0.5, 0.0), v(-0.7, -0.2, -0.3), v(0.7, 1.2, 0.3));
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.2, v(-0.4, 0.1, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(-0.4, 0.9, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(0.4, 0.1, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(0.4, 0.9, 0.0), SURFACE_CAR_PANEL),
        ],
    )
}

fn boot1() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        1.4,
        v(0.0, -0.4, 0.0),
        v(-0.7, -0.9, -0.3),
        v(0.7, 0.2, 0.3),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.2, v(-0.4, -0.1, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(-0.4, -0.6, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(0.4, -0.1, 0.0), SURFACE_CAR_PANEL),
            sphere(0.2, v(0.4, -0.6, 0.0), SURFACE_CAR_PANEL),
        ],
    )
}

fn wheel1() -> CColModel {
    let (bounding_sphere, bounding_box) =
        bounds(1.4, v(0.0, 0.0, 0.0), v(-0.7, -0.4, -0.4), v(0.7, 0.4, 0.4));
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.35, v(-0.3, 0.0, 0.0), SURFACE_WHEELBASE),
            sphere(0.35, v(0.3, 0.0, 0.0), SURFACE_WHEELBASE),
        ],
    )
}

fn body_part1() -> CColModel {
    let (bounding_sphere, bounding_box) =
        bounds(0.7, v(0.4, 0.0, 0.0), v(-0.3, -0.3, -0.3), v(1.1, 0.3, 0.3));
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.2, v(0.0, 0.0, 0.0), SURFACE_PED),
            sphere(0.2, v(0.8, 0.0, 0.0), SURFACE_PED),
        ],
    )
}

fn body_part2() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        0.5,
        v(0.25, 0.0, 0.0),
        v(-0.2, -0.2, -0.2),
        v(0.7, 0.2, 0.2),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(0.15, v(0.0, 0.0, 0.0), SURFACE_PED),
            sphere(0.15, v(0.5, 0.0, 0.0), SURFACE_PED),
        ],
    )
}

/// re3 TempColModels.cpp:58-78 — `s_aPedSpheres` + bounding volumes.
fn ped1() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        1.25,
        v(0.0, 0.0, 0.0),
        v(-PED_BODY_RADIUS, -PED_BODY_RADIUS, PED_BODY_MIN_Z),
        v(PED_BODY_RADIUS, PED_BODY_RADIUS, PED_BODY_MAX_Z),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            sphere(PED_BODY_RADIUS, v(0.0, 0.0, -0.25), SURFACE_PED),
            sphere(PED_BODY_RADIUS, v(0.0, 0.0, 0.15), SURFACE_PED),
            sphere(PED_BODY_RADIUS, v(0.0, 0.0, 0.55), SURFACE_PED),
        ],
    )
}

/// re3 TempColModels.cpp s_aPedGSpheres / ms_colModelPedGroundHit.
fn ped_ground_hit() -> CColModel {
    let (bounding_sphere, bounding_box) = bounds(
        2.0,
        v(0.0, 0.0, 0.0),
        v(-0.4, -1.0, -1.25),
        v(0.4, 1.2, -0.5),
    );
    CColModel::new(
        bounding_sphere,
        bounding_box,
        vec![
            CColSphere::new(0.35, v(0.0, -0.4, -0.9), SURFACE_PED, 4),
            CColSphere::new(0.35, v(0.0, -0.1, -0.9), SURFACE_PED, 1),
            CColSphere::new(0.35, v(0.0, 0.25, -0.9), SURFACE_PED, 0),
            CColSphere::new(0.30, v(0.0, 0.65, -0.9), SURFACE_PED, 6),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ped_ground_hit_matches_re3_four_sphere_layout() {
        let col = ped_ground_hit();
        assert_eq!(col.spheres.len(), 4);
        let expected = [
            (0.35, -0.4, 4),
            (0.35, -0.1, 1),
            (0.35, 0.25, 0),
            (0.30, 0.65, 6),
        ];
        for (sphere, (radius, y, piece)) in col.spheres.iter().zip(expected) {
            assert!((sphere.radius - radius).abs() < 1.0e-6);
            assert!(sphere.center.x.abs() < 1.0e-6);
            assert!((sphere.center.y - y).abs() < 1.0e-6);
            assert!((sphere.center.z + 0.9).abs() < 1.0e-6);
            assert_eq!(sphere.piece, piece);
            assert_eq!(sphere.surface, SURFACE_PED);
        }
        assert!((col.bounding_sphere.radius - 2.0).abs() < 1.0e-6);
        assert!((col.bounding_box.min.y + 1.0).abs() < 1.0e-6);
        assert!((col.bounding_box.max.y - 1.2).abs() < 1.0e-6);
    }
}
