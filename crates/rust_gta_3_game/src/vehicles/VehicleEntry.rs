//! Lock/capacity and transient entry/exit reservations from CVehicle.
//! Occupied seats remain solely on the InVehicle relationship.
use super::Vehicle::{MAX_VEHICLE_PASSENGERS, eCarLock};

#[derive(Debug, Clone, PartialEq, bevy::prelude::Component)]
pub struct VehicleEntryState {
    pub num_getting_in: i8,
    pub getting_in_flags: i8,
    pub getting_out_flags: i8,
    pub num_max_passengers: u8,
    pub door_lock: eCarLock,
    pub is_being_car_jacked: bool,
}

impl Default for VehicleEntryState {
    fn default() -> Self {
        Self {
            num_getting_in: 0,
            getting_in_flags: 0,
            getting_out_flags: 0,
            num_max_passengers: MAX_VEHICLE_PASSENGERS as u8,
            door_lock: eCarLock::CARLOCK_UNLOCKED,
            is_being_car_jacked: false,
        }
    }
}
