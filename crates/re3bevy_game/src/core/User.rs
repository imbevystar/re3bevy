//! re3 reference: references/re-GTA/src/core/User.cpp
//! re3 reference: references/re-GTA/src/core/User.h
//!
//! `CUserDisplay` place-name/current-vehicle state.

use bevy::prelude::*;

#[derive(Debug, Clone, Resource)]
pub struct CUserDisplay {
    /// Index into the loaded world zone table, mirroring `CPlaceName::m_pZone`.
    pub place_name_zone: Option<usize>,
    /// Default-zone fallback, mirroring `CPlaceName::m_pZone2`.
    pub place_name_zone2: Option<usize>,
    pub place_name_additional_timer: u16,
    pub pager_empty: bool,
    /// ECS vehicle entity currently occupied by the focused player.
    pub current_vehicle: Option<Entity>,
}

impl CUserDisplay {
    pub fn init() -> Self {
        Self {
            place_name_zone: None,
            place_name_zone2: None,
            place_name_additional_timer: 0,
            pager_empty: true,
            current_vehicle: None,
        }
    }

    /// Mirrors `CPlaceName::Process` and returns the zone whose translated
    /// name should be passed to `CHud::SetZoneName` this frame.
    pub fn process_place_name(
        &mut self,
        navig_zone: Option<usize>,
        default_zone: Option<usize>,
    ) -> Option<usize> {
        if navig_zone.is_none() {
            self.place_name_zone = None;
        }
        if default_zone.is_none() {
            self.place_name_zone2 = None;
        }

        if navig_zone == self.place_name_zone {
            if default_zone == self.place_name_zone2 || self.place_name_zone.is_some() {
                if navig_zone.is_some() || default_zone.is_some() {
                    if self.place_name_additional_timer != 0 {
                        self.place_name_additional_timer -= 1;
                    }
                } else {
                    self.place_name_additional_timer = 0;
                    self.place_name_zone = None;
                    self.place_name_zone2 = None;
                }
            } else {
                self.place_name_zone2 = default_zone;
                self.place_name_additional_timer = 250;
            }
        } else {
            self.place_name_zone = navig_zone;
            self.place_name_additional_timer = 250;
        }

        self.place_name_zone.or(self.place_name_zone2)
    }

    /// Mirrors `CCurrentVehicle::Process`'s pointer bookkeeping.
    pub fn process_current_vehicle(&mut self, current_vehicle: Option<Entity>) {
        self.current_vehicle = current_vehicle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_name_prefers_navig_zone_and_counts_down_like_re3() {
        let mut display = CUserDisplay::init();
        assert_eq!(display.process_place_name(Some(7), Some(3)), Some(7));
        assert_eq!(display.place_name_additional_timer, 250);

        assert_eq!(display.process_place_name(Some(7), Some(3)), Some(7));
        assert_eq!(display.place_name_additional_timer, 249);
    }

    #[test]
    fn default_zone_change_is_tracked_when_no_navig_zone_exists() {
        let mut display = CUserDisplay::init();
        assert_eq!(display.process_place_name(None, Some(2)), Some(2));
        assert_eq!(display.place_name_zone2, Some(2));
        assert_eq!(display.place_name_additional_timer, 250);

        assert_eq!(display.process_place_name(None, Some(4)), Some(4));
        assert_eq!(display.place_name_zone2, Some(4));
        assert_eq!(display.place_name_additional_timer, 250);
    }
}
