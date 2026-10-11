//! re3 reference: references/re-GTA/src/vehicles/Floater.cpp
//! re3 reference: references/re-GTA/src/vehicles/Floater.h
//!
//! Buoyancy state.

use re3bevy_core::core::Coordinates::CVector;

use crate::{
    entities::Physical::CPhysical, math::Matrix::CMatrix, renderer::WaterLevel::CWaterLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum tWaterLevel {
    FLOATER_ABOVE_WATER = 0,
    FLOATER_IN_WATER = 1,
    FLOATER_UNDER_WATER = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cBuoyancy {
    pub position: CVector,
    pub matrix: CMatrix,
    pub field_54: i32,
    pub position_z: CVector,
    pub waterlevel: f32,
    pub water_level_inc: f32,
    pub buoyancy: f32,
    pub dim_max: CVector,
    pub dim_min: CVector,
    pub num_partial_volumes: f32,
    pub field_8c: i32,
    pub field_90: i32,
    pub field_94: i32,
    pub have_volume: bool,
    pub step: CVector,
    pub step_ratio: CVector,
    pub num_steps: f32,
    pub flip_average: bool,
    pub field_b9: i8,
    pub is_boat: bool,
    pub volume_under_water: f32,
    pub impulse_point: CVector,
}

impl Default for cBuoyancy {
    fn default() -> Self {
        Self {
            position: CVector::ZERO,
            matrix: CMatrix::default(),
            field_54: 0,
            position_z: CVector::ZERO,
            waterlevel: 0.0,
            water_level_inc: 0.0,
            buoyancy: 0.0,
            dim_max: CVector::ZERO,
            dim_min: CVector::ZERO,
            num_partial_volumes: 1.0,
            field_8c: 0,
            field_90: 0,
            field_94: 0,
            have_volume: false,
            step: CVector::ZERO,
            step_ratio: CVector::ZERO,
            num_steps: 2.0,
            flip_average: false,
            field_b9: 0,
            is_boat: false,
            volume_under_water: 0.0,
            impulse_point: CVector::ZERO,
        }
    }
}

impl cBuoyancy {
    pub fn reset_partial_volume(&mut self) {
        self.have_volume = false;
        self.num_partial_volumes = 1.0;
        self.volume_under_water = 0.0;
        self.impulse_point = CVector::ZERO;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancyResult {
    pub point: CVector,
    pub impulse: CVector,
    pub water_level: f32,
    pub volume_under_water: f32,
}

/// re3 executable hull-volume coefficients, indexed by x + y * 3.
pub const BOAT_VOLUME_DISTRIBUTION: [f32; 9] = [0.75, 0.9, 0.75, 0.95, 1.0, 0.95, 0.3, 0.7, 0.3];

/// ECS-friendly form of re3 cBuoyancy::ProcessBuoyancy.
///
/// The native singleton only contains scratch state, so the Bevy port keeps
/// the calculation pure and returns the force data to the owning ped/vehicle/
/// object system. This preserves the 3x3 volume sampling and GTA water lookup
/// rules without introducing a mutable global resource.
#[allow(clippy::too_many_arguments)]
pub fn process_buoyancy(
    physical: &CPhysical,
    mut dim_min: CVector,
    mut dim_max: CVector,
    buoyancy: f32,
    water: &CWaterLevel,
    time_ms: u32,
    weather_wind: f32,
    time_step: f32,
    is_boat: bool,
    boat_volume_distribution: Option<&[f32; 9]>,
) -> Option<BuoyancyResult> {
    let position = physical.entity.get_position();
    let surface = water.get_water_level(
        position.x,
        position.y,
        position.z,
        physical.entity.flags.touching_water,
        time_ms,
        weather_wind,
    )?;

    // cBuoyancy::PreCalcSetup trims the collision hull before sampling.
    if is_boat {
        use crate::modelinfo::ModelIndices::{MI_REEFER, MI_SPEEDER};
        match physical.entity.model_index as usize {
            MI_SPEEDER => {
                dim_max.y *= 1.1;
                dim_min.y *= 0.9;
            }
            MI_REEFER => dim_min.y *= 0.9,
            _ => {
                dim_max.y *= 0.9;
                dim_min.y *= 0.9;
            }
        }
    }

    if dim_max.x <= dim_min.x || dim_max.y <= dim_min.y || dim_max.z <= dim_min.z {
        return None;
    }

    let num_steps = 2.0_f32;
    let step = CVector::new(
        (dim_max.x - dim_min.x) / num_steps,
        (dim_max.y - dim_min.y) / num_steps,
        (dim_max.z - dim_min.z) / num_steps,
    );
    if step.x.abs() <= f32::EPSILON || step.y.abs() <= f32::EPSILON || step.z.abs() <= f32::EPSILON
    {
        return None;
    }
    let step_ratio = if step.z > step.x && step.z > step.y {
        CVector::new(step.x / step.z, step.y / step.z, 1.0)
    } else if step.y > step.x && step.y > step.z {
        CVector::new(step.x / step.y, 1.0, step.z / step.y)
    } else {
        CVector::new(1.0, step.y / step.x, step.z / step.x)
    };

    let matrix = physical.entity.matrix;
    let mut have_volume = false;
    let mut volume_under_water = 0.0_f32;
    let mut impulse_point = CVector::ZERO;
    let mut num_partial_volumes = 1.0_f32;

    for ix in 0..=2 {
        let x = dim_min.x + step.x * ix as f32;
        for iy in 0..=2 {
            let y = dim_min.y + step.y * iy as f32;
            let local_sample = CVector::new(x, y, 0.0);
            let transformed = matrix.multiply3x3(local_sample);
            let sample_level = water
                .get_water_level(
                    transformed.x + position.x,
                    transformed.y + position.y,
                    position.z,
                    true,
                    time_ms,
                    weather_wind,
                )
                .map_or(0.0, |sample| sample.level);
            // FindWaterLevel passes the sample's initial z=0 to the lookup;
            // a NO_WATER return leaves that output unchanged. Reusing the
            // centre surface would invent water across shoreline cells.
            let mut local_water_z = sample_level - transformed.z - position.z;
            let water_position = if local_water_z > dim_max.z {
                local_water_z = dim_max.z;
                tWaterLevel::FLOATER_UNDER_WATER
            } else if local_water_z < dim_min.z {
                local_water_z = dim_min.z;
                tWaterLevel::FLOATER_ABOVE_WATER
            } else {
                tWaterLevel::FLOATER_IN_WATER
            };
            if water_position == tWaterLevel::FLOATER_ABOVE_WATER {
                continue;
            }

            let distribution_index = ix + iy * 3;
            let volume_multiplier = if is_boat {
                boat_volume_distribution
                    .map(|distribution| distribution[distribution_index])
                    .unwrap_or(BOAT_VOLUME_DISTRIBUTION[distribution_index])
            } else {
                1.0
            };
            let submerged = (local_water_z - dim_min.z).abs();
            let mut this_volume = submerged - (1.0 - volume_multiplier);
            if this_volume < 0.0 {
                continue;
            }
            if is_boat {
                this_volume *= volume_multiplier;
                if this_volume < 0.5 {
                    this_volume = 2.0 * this_volume * this_volume;
                }
                if this_volume < 1.0 {
                    this_volume *= this_volume;
                }
                this_volume *= this_volume;
            }
            volume_under_water += this_volume;

            let average = CVector::new(
                x * step_ratio.x,
                y * step_ratio.y,
                (local_water_z + dim_min.z) * 0.5 * step_ratio.z,
            );
            let fraction = 1.0 / num_partial_volumes;
            let remaining = 1.0 - fraction;
            impulse_point = CVector::new(
                impulse_point.x * remaining + average.x * this_volume * fraction,
                impulse_point.y * remaining + average.y * this_volume * fraction,
                impulse_point.z * remaining + average.z * this_volume * fraction,
            );
            num_partial_volumes += 1.0;
            have_volume = true;
        }
    }

    // Native ProcessBuoyancy returns true for a boat whenever its water
    // lookup succeeded, even when CalcBuoyancyForce finds no immersed volume.
    if !have_volume && !is_boat {
        return None;
    }
    volume_under_water /= (dim_max.z - dim_min.z) * (num_steps + 1.0).powi(2);
    let point = matrix.multiply3x3(impulse_point);
    let impulse = CVector::new(0.0, 0.0, volume_under_water * buoyancy * time_step.max(0.0));
    Some(BuoyancyResult {
        point,
        impulse,
        water_level: surface.level,
        volume_under_water,
    })
}
