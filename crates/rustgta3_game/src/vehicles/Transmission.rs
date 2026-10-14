//! re3 reference: references/re-GTA/src/vehicles/Transmission.cpp
//! re3 reference: references/re-GTA/src/vehicles/Transmission.h
//!
//! Vehicle transmission state.

use crate::vehicles::Automobile::{HANDLING_1G_BOOST, HANDLING_2G_BOOST};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct tGear {
    pub max_velocity: f32,
    pub shift_up_velocity: f32,
    pub shift_down_velocity: f32,
}

impl Default for tGear {
    fn default() -> Self {
        Self {
            max_velocity: 0.0,
            shift_up_velocity: 0.0,
            shift_down_velocity: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cTransmission {
    pub gears: [tGear; 6],
    pub drive_type: char,
    pub engine_type: char,
    pub number_of_gears: i8,
    pub flags: u8,
    pub engine_acceleration: f32,
    pub max_velocity: f32,
    pub max_cruise_velocity: f32,
    pub max_reverse_velocity: f32,
    pub cur_velocity: f32,
}

impl Default for cTransmission {
    fn default() -> Self {
        Self {
            gears: [tGear::default(); 6],
            drive_type: '\0',
            engine_type: '\0',
            number_of_gears: 0,
            flags: 0,
            engine_acceleration: 0.0,
            max_velocity: 0.0,
            max_cruise_velocity: 0.0,
            max_reverse_velocity: 0.0,
            cur_velocity: 0.0,
        }
    }
}

impl cTransmission {
    pub fn init_gear_ratios(&mut self) {
        self.gears = [tGear::default(); 6];
        for i in 1..=self.number_of_gears as usize {
            self.gears[i].max_velocity = i as f32 / self.number_of_gears as f32 * self.max_velocity;
            let velocity_diff = self.gears[i].max_velocity - self.gears[i - 1].max_velocity;
            if i >= self.number_of_gears as usize {
                self.gears[i].shift_up_velocity = self.max_velocity;
            } else {
                self.gears[i + 1].shift_down_velocity =
                    velocity_diff * 0.42 + self.gears[i - 1].max_velocity;
                self.gears[i].shift_up_velocity =
                    velocity_diff * 0.6667 + self.gears[i - 1].max_velocity;
            }
        }

        self.gears[0].max_velocity = self.max_reverse_velocity;
        self.gears[0].shift_up_velocity = -0.01;
        self.gears[0].shift_down_velocity = self.max_reverse_velocity;
        self.gears[1].shift_down_velocity = -0.01;
    }

    pub fn calculate_gear_for_simple_car(&mut self, speed: f32, gear: &mut u8) {
        let gear_ratio = self.gears[*gear as usize];
        self.cur_velocity = speed;
        if speed > gear_ratio.shift_up_velocity {
            *gear = gear.saturating_add(1);
        } else if speed < gear_ratio.shift_down_velocity {
            *gear = gear.saturating_sub(1);
        }
    }

    pub fn calculate_drive_acceleration(
        &mut self,
        gas_pedal: f32,
        gear: &mut u8,
        time: &mut f32,
        velocity: f32,
        cheat: bool,
        time_step: f32,
        time_step_seconds: f32,
    ) -> f32 {
        if velocity < self.max_reverse_velocity {
            return 0.0;
        }
        if velocity > self.max_velocity {
            return 0.0;
        }
        self.cur_velocity = velocity;

        debug_assert!(*gear <= self.number_of_gears as u8);
        let gear_index = *gear as usize;
        let gear_ratio = self.gears[gear_index];
        if velocity > gear_ratio.shift_up_velocity {
            if *gear != 0 || gas_pedal > 0.0 {
                *gear = gear.saturating_add(1);
                *time = 0.0;
                return self.calculate_drive_acceleration(
                    gas_pedal,
                    gear,
                    time,
                    velocity,
                    false,
                    time_step,
                    time_step_seconds,
                );
            }
        } else if velocity < gear_ratio.shift_down_velocity
            && *gear != 0
            && (*gear != 1 || gas_pedal < 0.0)
        {
            *gear = gear.saturating_sub(1);
            *time = 0.0;
            return self.calculate_drive_acceleration(
                gas_pedal,
                gear,
                time,
                velocity,
                false,
                time_step,
                time_step_seconds,
            );
        }

        if *time > 0.0 {
            *time -= time_step_seconds;
            return 0.0;
        }

        let (speed_mul, accel_mul) = if *gear < 1 {
            (
                -1.0,
                if u32::from(self.flags) & HANDLING_2G_BOOST != 0 {
                    2.0
                } else {
                    1.0
                },
            )
        } else if self.number_of_gears == 1 {
            (1.0, 1.0)
        } else {
            // re3 performs this division on uint8/int8 operands before
            // assigning the result to float. That makes it 0 for every
            // forward gear except the highest one, where it becomes 1.
            let gear_fraction =
                (i32::from(*gear) - 1) / (i32::from(self.number_of_gears) - 1);
            let f = 1.0 - gear_fraction as f32;
            let speed_mul = 3.0 * f * f + 1.0;
            let accel_mul = if u32::from(self.flags) & HANDLING_2G_BOOST != 0 {
                if *gear == 1 {
                    if u32::from(self.flags) & HANDLING_1G_BOOST != 0 {
                        3.0
                    } else {
                        2.0
                    }
                } else if *gear == 2 {
                    1.3
                } else {
                    1.0
                }
            } else if u32::from(self.flags) & HANDLING_1G_BOOST != 0 && *gear == 1 {
                3.0
            } else {
                1.0
            };
            (speed_mul, accel_mul)
        };

        let cheat_mul = if cheat { 1.2 } else { 1.0 };
        let target_velocity = self.gears[*gear as usize].max_velocity * speed_mul * cheat_mul;
        let accel = (target_velocity - velocity) * (self.engine_acceleration * accel_mul)
            / target_velocity.abs();
        if velocity.abs() < (self.gears[*gear as usize].max_velocity * cheat_mul).abs() {
            gas_pedal * accel * time_step
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn five_speed_transmission() -> cTransmission {
        let mut transmission = cTransmission {
            number_of_gears: 5,
            engine_acceleration: 0.1,
            max_velocity: 1.0,
            max_reverse_velocity: -0.2,
            ..Default::default()
        };
        transmission.init_gear_ratios();
        transmission
    }

    #[test]
    fn intermediate_gears_preserve_re3_integer_speed_multiplier() {
        let mut transmission = five_speed_transmission();
        let mut gear = 2;
        let mut gear_change_time = 0.0;

        let acceleration = transmission.calculate_drive_acceleration(
            1.0,
            &mut gear,
            &mut gear_change_time,
            0.2,
            false,
            1.0,
            1.0 / 30.0,
        );

        // C++ integer division makes speedMul 4.0 in gear 2:
        // target = 0.4 * 4 = 1.6, accel = (1.6 - 0.2) * 0.1 / 1.6.
        assert!((acceleration - 0.0875).abs() < 1.0e-6);
    }

    #[test]
    fn out_of_range_velocity_does_not_overwrite_cur_velocity() {
        let mut transmission = five_speed_transmission();
        transmission.cur_velocity = 0.25;
        let mut gear = 5;
        let mut gear_change_time = 0.0;

        assert_eq!(
            transmission.calculate_drive_acceleration(
                1.0,
                &mut gear,
                &mut gear_change_time,
                1.1,
                false,
                1.0,
                1.0 / 30.0,
            ),
            0.0
        );
        assert_eq!(transmission.cur_velocity, 0.25);
    }
}
