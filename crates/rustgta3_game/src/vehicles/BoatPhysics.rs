//! CBoat::ProcessControl native pad, wake, force and anchor calculations.
use super::Boat::{
    BoatPropellerEffect, BoatWaterEffects, INVALID_ORIENTATION, MAX_WAKE_POINTS, MIN_WAKE_INTERVAL,
    WAKE_LIFETIME,
};
use rustgta3_core::core::Coordinates::{CVector, CVector2D};

impl<C, V, S, Drive: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::DerefMut<Target = crate::vehicles::Vehicle::VehicleAudioState>>
    super::Boat::BoatRuntimeView<C, V, S, Drive, Appearance, Entry, Audio>
where
    C: std::ops::DerefMut<Target = super::Vehicle::VehicleCore>,
    V: std::ops::DerefMut<Target = super::Vehicle::VehicleVitality>,
    S: std::ops::DerefMut<Target = super::Boat::BoatState>,
{
    /// re3 CBoat::ProcessControlInputs, serviced at the native control cadence.
    pub fn process_control_inputs(
        &mut self,
        pad: &crate::core::Pad::CPad,
        pad_id: u8,
        steering_lock: f32,
    ) {
        self.state.pad_id = pad_id.min(3);
        self.state.brake = (self.state.brake
            + (pad.get_brake() as f32 / 255.0 - self.state.brake) * 0.1)
            .clamp(0.0, 1.0);
        if self.state.brake < 0.05 {
            self.state.brake = 0.0;
            self.state.accelerate = (self.state.accelerate
                + (pad.get_accelerate() as f32 / 255.0 - self.state.accelerate) * 0.1)
                .clamp(0.0, 1.0);
        } else {
            self.state.accelerate = -self.state.brake * 0.2;
        }
        self.state.steering_left_right = (self.state.steering_left_right
            + (-pad.get_steering_left_right() as f32 / 128.0 - self.state.steering_left_right)
                * 0.2)
            .clamp(-1.0, 1.0);
        self.vehicle.drive.steer_angle = steering_lock.to_radians()
            * self.state.steering_left_right
            * self.state.steering_left_right.abs();
        self.vehicle.drive.gas_pedal = self.state.accelerate;
    }

    /// re3 CBoat::PruneWakeTrail; lifetimes use native time-step units.
    pub fn prune_wake_trail(&mut self, time_step: f32) {
        let mut count = 0;
        for life in &mut self.state.wake_point_life_time {
            if *life <= 0.0 {
                break;
            }
            if *life <= time_step {
                *life = 0.0;
                break;
            }
            *life -= time_step;
            count += 1;
        }
        self.state.num_wake_points = count;
    }

    /// re3 CBoat::AddWakePoint, retaining the original less-than comparison.
    pub fn add_wake_point(&mut self, point: CVector) {
        if self.state.wake_point_life_time[0] > 0.0 {
            let position = self.vehicle.core.physical.entity.get_position();
            let dx = position.x - self.state.wake_points[0].x;
            let dy = position.y - self.state.wake_points[0].y;
            if dx * dx + dy * dy >= MIN_WAKE_INTERVAL * MIN_WAKE_INTERVAL {
                return;
            }
            let last = (self.state.num_wake_points as usize).min(MAX_WAKE_POINTS - 1);
            for index in (1..=last).rev() {
                self.state.wake_points[index] = self.state.wake_points[index - 1];
                self.state.wake_point_life_time[index] = self.state.wake_point_life_time[index - 1];
            }
            self.state.num_wake_points =
                (self.state.num_wake_points + 1).min(MAX_WAKE_POINTS as u16);
        } else {
            self.state.num_wake_points = 1;
        }
        self.state.wake_points[0] = CVector2D::new(point.x, point.y);
        self.state.wake_point_life_time[0] = WAKE_LIFETIME;
    }

    /// re3 CBoat::ApplyWaterResistance: local-axis drag and induced torque.
    pub fn apply_water_resistance(&mut self, time_step: f32) {
        let physical = &mut self.vehicle.core.physical;
        let matrix = physical.entity.matrix;
        let speed = physical.move_speed;
        let dot = |a: CVector, b: CVector| a.x * b.x + a.y * b.y + a.z * b.z;
        let forward_speed = dot(speed, matrix.forward);
        let resistance = 0.001 * self.state.volume_under_water.powi(2) * physical.mass;
        let divisor = ((forward_speed.powi(2) + 0.05) * resistance + 1.0).abs();
        let damping = CVector::new(
            (self.state.move_res.x / divisor).powf(0.5 * time_step),
            (self.state.move_res.y / divisor).powf(0.5 * time_step),
            (self.state.move_res.z / divisor).powf(0.5 * time_step),
        );
        let local = CVector::new(
            dot(speed, matrix.right) * damping.x,
            dot(speed, matrix.forward) * damping.y,
            dot(speed, matrix.up) * damping.z,
        );
        let force = (damping.y - 1.0) * local.y * physical.mass;
        physical.move_speed = matrix.multiply3x3(local);
        physical.apply_turn_force(
            CVector::new(
                matrix.forward.x * force,
                matrix.forward.y * force,
                matrix.forward.z * force,
            ),
            CVector::new(-matrix.up.x, -matrix.up.y, -matrix.up.z),
        );
        physical.move_speed.z *= if physical.move_speed.z > 0.0 {
            damping.z
        } else {
            (1.0 - damping.z) * 0.5 + damping.z
        };
    }

    /// re3 CBoat::ProcessControl anchor constraint, applied after forces.
    pub fn apply_anchor(&mut self) {
        if !self.state.is_anchored {
            return;
        }
        self.vehicle.core.physical.move_speed.x = 0.0;
        self.vehicle.core.physical.move_speed.y = 0.0;
        let matrix = &mut self.vehicle.core.physical.entity.matrix;
        let heading = (-matrix.forward.x).atan2(matrix.forward.y);
        if self.state.orientation == INVALID_ORIENTATION {
            self.state.orientation = heading;
        } else {
            let position = matrix.position;
            matrix.rotate_z(self.state.orientation - heading);
            matrix.position = position;
        }
    }

    /// re3 CBoat::ProcessControl buoyancy, hydroplaning and splash impulses.
    /// Propeller input, collision response and presentation are separate
    /// interfaces and must be serviced by the owning ECS control system.
    pub fn apply_buoyancy_response(
        &mut self,
        result: crate::vehicles::Floater::BuoyancyResult,
        handling: &crate::vehicles::HandlingMgr::tHandlingData,
        time_step: f32,
        on_land: bool,
        water: &crate::renderer::WaterLevel::CWaterLevel,
        time_ms: u32,
        wind: f32,
    ) -> BoatWaterEffects {
        let gas_pedal = self.vehicle.drive.gas_pedal;
        let steer_angle = self.vehicle.drive.steer_angle;
        use crate::entities::Physical::GRAVITY;
        let mut effects = BoatWaterEffects::default();
        let scaled = |v: CVector, scale: f32| CVector::new(v.x * scale, v.y * scale, v.z * scale);
        let dot = |a: CVector, b: CVector| a.x * b.x + a.y * b.y + a.z * b.z;
        let physical = &mut self.vehicle.core.physical;
        let matrix = physical.entity.matrix;
        self.state.boat_in_water = 0.1 * physical.mass * GRAVITY * time_step < result.impulse.z;
        physical.flags.is_in_water = self.state.boat_in_water;
        self.state.volume_under_water = result.volume_under_water;
        self.state.buoyance_point = result.point;
        physical.apply_move_force(result.impulse);
        if !on_land {
            physical.apply_turn_force(result.impulse, result.point);
        }
        if !on_land && self.state.boat_in_water && matrix.up.z > 0.0 {
            let speed_squared = dot(physical.move_speed, physical.move_speed);
            let impulse = if gas_pedal > 0.05 {
                speed_squared
                    * handling.suspension_force_level
                    * result.impulse.z
                    * time_step
                    * 0.5
                    * gas_pedal
            } else {
                0.0
            };
            let impulse = impulse
                .min(GRAVITY * handling.suspension_damping_level * physical.mass * time_step);
            let force = scaled(matrix.up, impulse);
            physical.apply_move_force(force);
            physical.apply_turn_force(
                force,
                CVector::new(
                    result.point.x - handling.suspension_bias * matrix.forward.x,
                    result.point.y - handling.suspension_bias * matrix.forward.y,
                    result.point.z - handling.suspension_bias * matrix.forward.z,
                ),
            );
        }
        if gas_pedal.abs() > 0.05
            || (physical.move_speed.x.powi(2) + physical.move_speed.y.powi(2)).sqrt() > 0.01
        {
            let speed = physical.move_speed;
            let position = matrix.position;
            if self.state.boat_in_water {
                self.add_wake_point(position);
            }
            let physical = &mut self.vehicle.core.physical;
            let forward_factor = if physical.entity.model_index as usize
                == crate::modelinfo::ModelIndices::MI_GHOST
            {
                0.3
            } else {
                1.0
            };
            let steer_factor = (1.0 - dot(speed, matrix.forward) * forward_factor).max(0.0);
            let propeller = matrix.multiply3x3(CVector::new(
                0.0,
                -handling.dimension.y * self.state.thrust_y,
                -handling.dimension.z * self.state.thrust_z,
            ));
            let world_propeller = CVector::new(
                position.x + propeller.x,
                position.y + propeller.y,
                position.z + propeller.z,
            );
            self.state.propeller_in_water = false;
            if let Some(surface) = water.get_water_level(
                world_propeller.x,
                world_propeller.y,
                world_propeller.z,
                true,
                time_ms,
                wind,
            ) && world_propeller.z - 0.5 < surface.level
            {
                self.state.propeller_in_water = true;
                let depth = surface.level - (world_propeller.z - 0.5);
                let depth = if depth > 1.0 { 1.0 } else { depth * depth };
                let angle = -steer_angle * steer_factor;
                let steer_sin = angle.sin();
                if gas_pedal.abs() > 0.05 {
                    let force_direction = matrix.multiply3x3(CVector::new(
                        -steer_sin,
                        angle.cos(),
                        -steer_angle.abs(),
                    ));
                    let mut force = scaled(
                        force_direction,
                        depth
                            * gas_pedal
                            * 40.0
                            * handling.transmission.engine_acceleration
                            * handling.mass,
                    );
                    if force.z > 0.2 {
                        force.z = (1.2 - force.z).powi(2) + 0.2;
                    }
                    if on_land {
                        if gas_pedal < 0.0 {
                            force.x *= 5.0;
                            force.y *= 5.0;
                        }
                        force.z = force.z.max(0.0);
                        physical.apply_move_force(scaled(force, time_step));
                    } else {
                        physical.apply_move_force(scaled(force, time_step));
                        physical.apply_turn_force(
                            scaled(force, time_step),
                            CVector::new(
                                propeller.x - handling.traction_bias * matrix.up.x,
                                propeller.y - handling.traction_bias * matrix.up.y,
                                propeller.z - handling.traction_bias * matrix.up.z,
                            ),
                        );
                        let right_force = dot(matrix.right, force);
                        physical.apply_turn_force(
                            scaled(matrix.right, -right_force * time_step),
                            matrix.up,
                        );
                    }
                    effects.propeller = Some(BoatPropellerEffect {
                        world_position: world_propeller,
                        force,
                    });
                } else if !on_land {
                    let force = (50.0 * dot(physical.move_speed, matrix.forward)).min(10.0);
                    let force = scaled(
                        matrix.multiply3x3(CVector::new(-steer_sin * force, 0.0, 0.0)),
                        depth * time_step * 0.5,
                    );
                    physical.apply_move_force(force);
                    physical.apply_turn_force(force, propeller);
                }
            }
        }
        let physical = &mut self.vehicle.core.physical;
        physical.move_speed.x = physical
            .move_speed
            .x
            .min(-(matrix.position.x - 1900.0) * 0.01)
            .max(-(matrix.position.x + 1515.0) * 0.01);
        physical.move_speed.y = physical
            .move_speed
            .y
            .min(-(matrix.position.y - 600.0) * 0.01)
            .max(-(matrix.position.y + 1900.0) * 0.01);
        // Finish borrowing the physical state before the resistance method.
        if !on_land && self.state.boat_in_water {
            self.apply_water_resistance(time_step);
        }
        let physical = &mut self.vehicle.core.physical;
        let turn = physical.turn_speed;
        let mut local = CVector::new(
            dot(turn, matrix.right),
            dot(turn, matrix.forward),
            dot(turn, matrix.up),
        );
        let damping_x = self.state.turn_res.x.powf(time_step);
        let damping_y = self.state.turn_res.y.powf(time_step);
        let damping_z = self.state.turn_res.z.powf(time_step);
        let pitch_damping = damping_x / (1000.0 * local.x.powi(2) + 1.0);
        local.y *= damping_y;
        local.z *= damping_z;
        let force_up = (pitch_damping - 1.0) * local.x * physical.turn_mass;
        physical.turn_speed = matrix.multiply3x3(local);
        let centre = matrix.multiply3x3(physical.centre_of_mass);
        physical.apply_turn_force(
            CVector::new(0.0, 0.0, force_up),
            CVector::new(
                centre.x + matrix.forward.x,
                centre.y + matrix.forward.y,
                centre.z + matrix.forward.z,
            ),
        );
        self.state.delta_volume_under_water =
            ((self.state.volume_under_water - self.state.prev_volume_under_water) * 10000.0) as i16;
        if !on_land
            && self.state.boat_in_water
            && matrix.up.z > 0.0
            && self.state.delta_volume_under_water > 200
        {
            effects.landing_splash = true;
            let delta = self.state.delta_volume_under_water as f32;
            let speed_squared = dot(physical.move_speed, physical.move_speed);
            let speed_up = (speed_squared * delta * 0.0004)
                .min(handling.brake_deceleration - physical.move_speed.z)
                .max(0.0);
            let speed_forward =
                dot(physical.move_speed, matrix.forward) * -delta * 0.01 * handling.traction_loss;
            let impulse = CVector::new(
                matrix.forward.x * speed_forward * physical.mass,
                matrix.forward.y * speed_forward * physical.mass,
                (matrix.forward.z * speed_forward + speed_up) * physical.mass,
            );
            physical.apply_move_force(impulse);
            physical.apply_turn_force(impulse, result.point);
        }
        self.state.prev_volume_under_water = self.state.volume_under_water;
        effects
    }
}
