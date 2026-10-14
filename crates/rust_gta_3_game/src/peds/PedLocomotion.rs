//! Canonical authored root motion and heading used by CPed::CalculateNewVelocity.
use super::Ped::{CPed, PedState};
use crate::entities::Physical::GTA_PHYSICS_HZ;
use bevy::prelude::Component;
use rust_gta_3_core::core::Coordinates::{CVector, CVector2D};
use rust_gta_3_core::core::General::CGeneral;

/// CPed::CalculateNewVelocity / UpdatePosition use horizontal metres per GTA
/// step (1/50 second). ProcessEntityCollision clears Z on established static
/// ground; a ground-height correction must never become an upward impulse.
/// Airborne and moving-surface integration retain their own physical velocity.
pub(crate) fn grounded_velocity_from_displacement(delta: CVector, gta_time_step: f32) -> CVector {
    let divisor = if gta_time_step >= 0.01 {
        gta_time_step
    } else {
        100.0
    };
    CVector::new(delta.x / divisor, delta.y / divisor, 0.0)
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct PedLocomotion {
    pub anim_move_delta: CVector2D,
    pub moved: CVector2D,
    pub rotation_cur: f32,
    pub heading_rate: f32,
    pub rotation_dest: f32,
}
impl Default for PedLocomotion {
    fn default() -> Self {
        Self {
            anim_move_delta: CVector2D::new(0.0, 0.0),
            moved: CVector2D::new(0.0, 0.0),
            rotation_cur: 0.0,
            heading_rate: 15.0,
            rotation_dest: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_velocity_retains_native_units_across_frame_rates() {
        for hz in [30.0_f32, 60.0, 120.0] {
            let dt = hz.recip();
            let velocity =
                grounded_velocity_from_displacement(CVector::new(1.5 * dt, 0.0, 0.0), dt * 50.0);
            assert!((velocity.x - 0.03).abs() < 1.0e-6);
        }
        assert_eq!(
            grounded_velocity_from_displacement(CVector::new(1.0, 0.0, 0.0), 0.0).x,
            0.01
        );
    }

    #[test]
    fn ground_height_correction_does_not_create_vertical_physics_velocity() {
        let velocity =
            grounded_velocity_from_displacement(CVector::new(0.025, -0.01, 0.07), 5.0 / 6.0);
        assert!((velocity.x - 0.03).abs() < 1.0e-6);
        assert!((velocity.y + 0.012).abs() < 1.0e-6);
        assert_eq!(velocity.z, 0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PedPhysicalSurfaceMotion {
    pub(crate) speed_native: CVector,
    pub(crate) turn_speed_z_native: f32,
    pub(crate) is_vehicle: bool,
    pub(crate) is_boat: bool,
}

/// Seconds-based equivalent of `CPed::UpdatePosition`'s standing XY update.
/// `animation_world_delta` is the current frame's extracted root displacement
/// after heading rotation. re3 divides that displacement by GTA timestep to
/// form `m_moved`, adds the current physical-surface speed, and only then lets
/// `CPhysical` integrate the resulting move-speed. Returning the displacement
/// here preserves that ordering without storing a second authoritative motion
/// state outside `CPed::physical`.
pub(crate) fn apply_standing_update_position_seconds(
    ped: &mut CPed,
    locomotion: &mut PedLocomotion,
    animation_world_delta: CVector,
    surface: Option<PedPhysicalSurfaceMotion>,
    dt_seconds: f32,
) -> CVector {
    if !ped.flags.is_standing || dt_seconds <= 0.0 {
        return CVector::ZERO;
    }

    let animation_velocity = CVector::new(
        animation_world_delta.x / dt_seconds,
        animation_world_delta.y / dt_seconds,
        0.0,
    );
    let steep_surface = crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8;
    let mut velocity_change;

    if let Some(surface) = surface {
        velocity_change = CVector::new(
            animation_velocity.x - ped.physical.move_speed.x,
            animation_velocity.y - ped.physical.move_speed.y,
            0.0,
        );
        let surface_speed_seconds = CVector::new(
            surface.speed_native.x * GTA_PHYSICS_HZ,
            surface.speed_native.y * GTA_PHYSICS_HZ,
            surface.speed_native.z * GTA_PHYSICS_HZ,
        );
        velocity_change.x += surface_speed_seconds.x;
        velocity_change.y += surface_speed_seconds.y;

        let gta_time_step = dt_seconds * GTA_PHYSICS_HZ;
        let speed_change =
            (velocity_change.x * velocity_change.x + velocity_change.y * velocity_change.y).sqrt();
        let max_change_native = if ped.ped_state == PedState::PED_DIE && surface.is_vehicle {
            Some(0.002 * gta_time_step)
        } else if !(surface.is_vehicle && surface.is_boat) {
            Some(0.01 * gta_time_step)
        } else {
            None
        };
        if let Some(max_change_native) = max_change_native {
            let max_change_seconds = max_change_native * GTA_PHYSICS_HZ;
            if speed_change > max_change_seconds && speed_change > 0.0 {
                let scale = max_change_seconds / speed_change;
                velocity_change.x *= scale;
                velocity_change.y *= scale;
            }
        }

        // Native UpdatePosition sets the matrix heading first, then advances
        // the two logical heading fields by the supporting surface's turn
        // speed. The entity matrix therefore picks up this increment on the
        // next CalculateNewOrientation/UpdatePosition pass, so do not eagerly
        // rewrite it here.
        let surface_turn = surface.turn_speed_z_native * gta_time_step;
        locomotion.rotation_cur =
            CGeneral::limit_radian_angle(locomotion.rotation_cur + surface_turn);
        locomotion.rotation_dest =
            CGeneral::limit_radian_angle(locomotion.rotation_dest + surface_turn);
    } else if ped.physical.surface_touched == steep_surface
        && (ped.physical.damage.normal.x != 0.0 || ped.physical.damage.normal.y != 0.0)
    {
        // Native steep-cliff UpdatePosition first kills the existing horizontal
        // velocity and leaves a tiny downward speed, then adds 0.02 native
        // units away from the cliff to m_moved. This controller stores player
        // velocity in units/second, hence the one 50 Hz conversion here.
        ped.physical.move_speed = CVector::new(0.0, 0.0, -0.001 * GTA_PHYSICS_HZ);
        let normal_length = (ped.physical.damage.normal.x * ped.physical.damage.normal.x
            + ped.physical.damage.normal.y * ped.physical.damage.normal.y)
            .sqrt();
        let reaction = CVector::new(
            ped.physical.damage.normal.x / normal_length,
            ped.physical.damage.normal.y / normal_length,
            0.0,
        );
        velocity_change = CVector::new(
            animation_velocity.x + 0.02 * GTA_PHYSICS_HZ * reaction.x,
            animation_velocity.y + 0.02 * GTA_PHYSICS_HZ * reaction.y,
            0.0,
        );
        let reaction_dot = reaction.x * velocity_change.x + reaction.y * velocity_change.y;
        if reaction_dot < 0.0 {
            velocity_change.x -= reaction_dot * reaction.x;
            velocity_change.y -= reaction_dot * reaction.y;
        }
    } else {
        velocity_change = CVector::new(
            animation_velocity.x - ped.physical.move_speed.x,
            animation_velocity.y - ped.physical.move_speed.y,
            0.0,
        );
    }

    ped.physical.move_speed.x += velocity_change.x;
    ped.physical.move_speed.y += velocity_change.y;
    CVector::new(
        ped.physical.move_speed.x * dt_seconds,
        ped.physical.move_speed.y * dt_seconds,
        0.0,
    )
}
