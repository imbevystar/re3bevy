//! re3 reference: references/re-GTA/src/vehicles/Door.cpp
//! re3 reference: references/re-GTA/src/vehicles/Door.h
//!
//! Vehicle door state and hinge behaviour.

use re3bevy_core::core::Coordinates::CVector;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum eDoorState {
    DOORST_SWINGING = 0,
    DOORST_OPEN = 1,
    DOORST_CLOSED = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CDoor {
    pub min_angle: f32,
    pub max_angle: f32,
    pub dir: u8,
    pub axis: u8,
    pub state: u8,
    pub angle: f32,
    pub previous_angle: f32,
    pub angular_velocity: f32,
    pub speed: CVector,
}

impl Default for CDoor {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0, 0)
    }
}

impl CDoor {
    pub const DOORST_SWINGING: u8 = eDoorState::DOORST_SWINGING as u8;
    pub const DOORST_OPEN: u8 = eDoorState::DOORST_OPEN as u8;
    pub const DOORST_CLOSED: u8 = eDoorState::DOORST_CLOSED as u8;

    pub const fn new(min_angle: f32, max_angle: f32, dir: u8, axis: u8) -> Self {
        Self {
            min_angle,
            max_angle,
            dir,
            axis,
            state: Self::DOORST_SWINGING,
            angle: 0.0,
            previous_angle: 0.0,
            angular_velocity: 0.0,
            speed: CVector::ZERO,
        }
    }

    pub fn init(&mut self, min_angle: f32, max_angle: f32, dir: i8, axis: i8) {
        self.min_angle = min_angle;
        self.max_angle = max_angle;
        self.dir = dir as u8;
        self.axis = axis as u8;
    }

    pub fn open(&mut self, ratio: f32) {
        self.previous_angle = self.angle;
        let open = self.angle_when_open();
        if ratio < 1.0 {
            self.angle = open * ratio;
            if self.angle == 0.0 {
                self.angular_velocity = 0.0;
            }
        } else {
            self.state = Self::DOORST_OPEN;
            self.angle = open;
        }
    }

    pub fn angle_when_closed(&self) -> f32 {
        if self.max_angle.abs() < self.min_angle.abs() {
            self.max_angle
        } else {
            self.min_angle
        }
    }

    pub fn angle_when_open(&self) -> f32 {
        if self.max_angle.abs() < self.min_angle.abs() {
            self.min_angle
        } else {
            self.max_angle
        }
    }

    pub fn angle_open_ratio(&self) -> f32 {
        let open = self.angle_when_open();
        if open == 0.0 { 0.0 } else { self.angle / open }
    }

    pub fn is_fully_open(&self) -> bool {
        self.angle.abs() >= self.angle_when_open().abs() - 0.5
    }

    pub fn is_closed(&self) -> bool {
        self.angle == self.angle_when_closed()
    }

    pub fn process<VehCore, VehVitality, Drive: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleDriveState>, Appearance: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAppearance>, Entry: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleEntryState>, Audio: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleAudioState>>(&mut self, vehicle: &crate::vehicles::Vehicle::VehicleRuntime<VehCore, VehVitality, Drive, Appearance, Entry, Audio>)
where
    VehCore: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleCore>,
    VehVitality: std::ops::Deref<Target = crate::vehicles::Vehicle::VehicleVitality>,
{
        let speed = vehicle
            .physical
            .get_speed_at_point(CVector::new(1.0, 0.0, 0.0));
        let speed_diff = CVector::new(
            speed.x - self.speed.x,
            speed.y - self.speed.y,
            speed.z - self.speed.z,
        );
        let local = vehicle.physical.entity.matrix.multiply3x3(speed_diff);
        let speed_difference = match self.axis {
            0 if self.dir != 0 => local.y + local.z,
            0 => -(local.y + local.z),
            2 if self.dir != 0 => -(local.y + local.x),
            2 => local.y - local.x,
            _ => 0.0,
        }
        .clamp(-0.2, 0.2);

        if speed_difference.abs() > 0.002 {
            self.angular_velocity += speed_difference;
        }
        self.angular_velocity = (self.angular_velocity * 0.945).clamp(-0.3, 0.3);
        self.angle += self.angular_velocity;
        self.state = Self::DOORST_SWINGING;
        if self.angle > self.max_angle {
            self.angle = self.max_angle;
            self.angular_velocity *= -0.8;
            self.state = Self::DOORST_OPEN;
        }
        if self.angle < self.min_angle {
            self.angle = self.min_angle;
            self.angular_velocity *= -0.8;
            self.state = Self::DOORST_CLOSED;
        }
        self.speed = speed;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CTrainDoor {
    pub closed_posn: f32,
    pub open_posn: f32,
    pub dirn: i8,
    pub door_state: eDoorState,
    pub axis: i8,
    pub posn: f32,
    pub prev_posn: f32,
    pub field_14: i32,
}

impl Default for CTrainDoor {
    fn default() -> Self {
        Self {
            closed_posn: 0.0,
            open_posn: 0.0,
            dirn: 0,
            door_state: eDoorState::DOORST_SWINGING,
            axis: 0,
            posn: 0.0,
            prev_posn: 0.0,
            field_14: 0,
        }
    }
}

impl CTrainDoor {
    pub fn init(&mut self, open: f32, closed: f32, dir: i8, axis: i8) {
        self.open_posn = open;
        self.closed_posn = closed;
        self.dirn = dir;
        self.axis = axis;
    }

    pub fn open(&mut self, ratio: f32) {
        self.prev_posn = self.posn;
        let open = self.translation_when_open();
        if ratio < 1.0 {
            self.posn = open * ratio;
        } else {
            self.door_state = eDoorState::DOORST_OPEN;
            self.posn = open;
        }
    }

    pub fn translation_when_closed(&self) -> f32 {
        if self.closed_posn.abs() < self.open_posn.abs() {
            self.closed_posn
        } else {
            self.open_posn
        }
    }

    pub fn translation_when_open(&self) -> f32 {
        if self.closed_posn.abs() < self.open_posn.abs() {
            self.open_posn
        } else {
            self.closed_posn
        }
    }

    pub fn is_fully_open(&self) -> bool {
        self.posn.abs() >= self.translation_when_open().abs() - 0.5
    }

    pub fn is_closed(&self) -> bool {
        self.posn == self.translation_when_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automobile_door_opening_matches_re3() {
        let mut door = CDoor::new(-1.2, 0.0, 0, 2);
        assert_eq!(door.state, CDoor::DOORST_SWINGING);
        assert_eq!(door.angle_when_closed(), 0.0);
        assert_eq!(door.angle_when_open(), -1.2);

        door.angular_velocity = 0.2;
        door.open(0.5);
        assert_eq!(door.previous_angle, 0.0);
        assert_eq!(door.angle, -0.6);
        assert_eq!(door.angular_velocity, 0.2);
        assert!(!door.is_closed());
        assert!(!door.is_fully_open());

        door.open(1.0);
        assert_eq!(door.previous_angle, -0.6);
        assert_eq!(door.angle, -1.2);
        assert_eq!(door.state, CDoor::DOORST_OPEN);
        assert!(door.is_fully_open());
    }

    #[test]
    fn zero_open_ratio_stops_automobile_hinge() {
        let mut door = CDoor::new(0.0, 1.0, 1, 2);
        door.angular_velocity = 0.25;
        door.open(0.0);
        assert_eq!(door.angle, 0.0);
        assert_eq!(door.angular_velocity, 0.0);
        assert_eq!(door.angle_open_ratio(), 0.0);
    }

    #[test]
    fn train_door_translation_matches_re3() {
        let mut door = CTrainDoor::default();
        door.init(-0.8, 0.0, 0, 0);
        door.door_state = eDoorState::DOORST_CLOSED;

        assert_eq!(door.translation_when_closed(), 0.0);
        assert_eq!(door.translation_when_open(), -0.8);
        assert!(door.is_closed());

        door.open(0.25);
        assert_eq!(door.prev_posn, 0.0);
        assert_eq!(door.posn, -0.2);
        assert_eq!(door.door_state, eDoorState::DOORST_CLOSED);

        door.open(1.0);
        assert_eq!(door.prev_posn, -0.2);
        assert_eq!(door.posn, -0.8);
        assert_eq!(door.door_state, eDoorState::DOORST_OPEN);
        assert!(door.is_fully_open());
    }
}
