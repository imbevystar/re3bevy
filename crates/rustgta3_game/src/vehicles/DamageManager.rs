//! re3 reference: references/re-GTA/src/vehicles/DamageManager.cpp
//! re3 reference: references/re-GTA/src/vehicles/DamageManager.h
//!
//! Damage status enums shared by vehicles.

use rustgta3_core::core::General::CGeneral;

pub const NUM_DAMAGE_WHEELS: usize = 4;
pub const NUM_DAMAGE_DOORS: usize = 6;

const G_A_COMPONENT_DAMAGE: [f32; 7] = [2.5, 1.25, 3.2, 1.4, 2.5, 2.8, 0.5];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eEngineStatus {
    ENGINE_STATUS_STEAM1 = 100,
    ENGINE_STATUS_STEAM2 = 150,
    ENGINE_STATUS_SMOKE = 200,
    ENGINE_STATUS_ON_FIRE = 225,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eDoorStatus {
    DOOR_STATUS_OK = 0,
    DOOR_STATUS_SMASHED = 1,
    DOOR_STATUS_SWINGING = 2,
    DOOR_STATUS_MISSING = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ePanelStatus {
    PANEL_STATUS_OK = 0,
    PANEL_STATUS_SMASHED1 = 1,
    PANEL_STATUS_SMASHED2 = 2,
    PANEL_STATUS_MISSING = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum eWheelStatus {
    WHEEL_STATUS_OK = 0,
    WHEEL_STATUS_BURST = 1,
    WHEEL_STATUS_MISSING = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eLightStatus {
    LIGHT_STATUS_OK = 0,
    LIGHT_STATUS_BROKEN = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum tComponent {
    COMPONENT_DEFAULT = 0,
    COMPONENT_WHEEL_FRONT_LEFT = 1,
    COMPONENT_WHEEL_FRONT_RIGHT = 2,
    COMPONENT_WHEEL_REAR_LEFT = 3,
    COMPONENT_WHEEL_REAR_RIGHT = 4,
    COMPONENT_DOOR_BONNET = 5,
    COMPONENT_DOOR_BOOT = 6,
    COMPONENT_DOOR_FRONT_LEFT = 7,
    COMPONENT_DOOR_FRONT_RIGHT = 8,
    COMPONENT_DOOR_REAR_LEFT = 9,
    COMPONENT_DOOR_REAR_RIGHT = 10,
    COMPONENT_PANEL_FRONT_LEFT = 11,
    COMPONENT_PANEL_FRONT_RIGHT = 12,
    COMPONENT_PANEL_REAR_LEFT = 13,
    COMPONENT_PANEL_REAR_RIGHT = 14,
    COMPONENT_PANEL_WINDSCREEN = 15,
    COMPONENT_BUMPER_FRONT = 16,
    COMPONENT_BUMPER_REAR = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum tComponentGroup {
    COMPGROUP_BUMPER = 0,
    COMPGROUP_WHEEL = 1,
    COMPGROUP_DOOR = 2,
    COMPGROUP_BONNET = 3,
    COMPGROUP_BOOT = 4,
    COMPGROUP_PANEL = 5,
    COMPGROUP_DEFAULT = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eDoors {
    DOOR_BONNET = 0,
    DOOR_BOOT = 1,
    DOOR_FRONT_LEFT = 2,
    DOOR_FRONT_RIGHT = 3,
    DOOR_REAR_LEFT = 4,
    DOOR_REAR_RIGHT = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ePanels {
    VEHPANEL_FRONT_LEFT = 0,
    VEHPANEL_FRONT_RIGHT = 1,
    VEHPANEL_REAR_LEFT = 2,
    VEHPANEL_REAR_RIGHT = 3,
    VEHPANEL_WINDSCREEN = 4,
    VEHBUMPER_FRONT = 5,
    VEHBUMPER_REAR = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eLights {
    VEHLIGHT_FRONT_LEFT = 0,
    VEHLIGHT_FRONT_RIGHT = 1,
    VEHLIGHT_REAR_LEFT = 2,
    VEHLIGHT_REAR_RIGHT = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CDamageManager {
    pub wheel_damage_effect: f32,
    pub engine_status: u8,
    pub wheel_status: [u8; NUM_DAMAGE_WHEELS],
    pub door_status: [u8; NUM_DAMAGE_DOORS],
    pub light_status: u32,
    pub panel_status: u32,
    pub field_18: u8,
}

impl Default for CDamageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CDamageManager {
    pub fn new() -> Self {
        let mut damage = Self {
            wheel_damage_effect: 0.0,
            engine_status: 0,
            wheel_status: [0; NUM_DAMAGE_WHEELS],
            door_status: [0; NUM_DAMAGE_DOORS],
            light_status: 0,
            panel_status: 0,
            field_18: 0,
        };
        damage.reset_damage_status();
        damage.wheel_damage_effect = 0.75;
        damage.field_18 = 1;
        damage
    }

    pub fn reset_damage_status(&mut self) {
        self.wheel_damage_effect = 0.0;
        self.engine_status = 0;
        self.wheel_status = [0; NUM_DAMAGE_WHEELS];
        self.door_status = [0; NUM_DAMAGE_DOORS];
        self.light_status = 0;
        self.panel_status = 0;
        self.field_18 = 0;
    }

    pub fn fuck_car_completely(&mut self) {
        self.wheel_status[0] = eWheelStatus::WHEEL_STATUS_MISSING as u8;
        for door in 0..self.door_status.len() {
            self.door_status[door] = eDoorStatus::DOOR_STATUS_MISSING as u8;
        }
        for _ in 0..3 {
            self.progress_panel_damage(ePanels::VEHBUMPER_FRONT as u8);
            self.progress_panel_damage(ePanels::VEHBUMPER_REAR as u8);
        }
        self.light_status = 0;
        self.panel_status = 0;
        self.set_engine_status(250);
    }

    pub fn apply_damage(
        &mut self,
        component: tComponent,
        damage: f32,
        _unused: f32,
        general: &mut CGeneral,
    ) -> bool {
        let Some((group, sub_comp)) = Self::get_component_group(component) else {
            return false;
        };
        let damage = damage * G_A_COMPONENT_DAMAGE[group as usize];
        if damage <= 150.0 {
            return false;
        }

        match group {
            tComponentGroup::COMPGROUP_WHEEL => {
                self.progress_wheel_damage(sub_comp);
            }
            tComponentGroup::COMPGROUP_DOOR | tComponentGroup::COMPGROUP_BOOT => {
                self.progress_door_damage(sub_comp);
            }
            tComponentGroup::COMPGROUP_BONNET => {
                if damage > 220.0 {
                    self.progress_engine_damage(general);
                }
                self.progress_door_damage(sub_comp);
            }
            tComponentGroup::COMPGROUP_PANEL => {
                self.set_light_status(sub_comp, eLightStatus::LIGHT_STATUS_BROKEN as u32);
                if damage > 220.0
                    && matches!(
                        component,
                        tComponent::COMPONENT_PANEL_FRONT_LEFT
                            | tComponent::COMPONENT_PANEL_FRONT_RIGHT
                            | tComponent::COMPONENT_PANEL_WINDSCREEN
                    )
                {
                    self.progress_engine_damage(general);
                }
                self.progress_panel_damage(sub_comp);
            }
            tComponentGroup::COMPGROUP_BUMPER => {
                self.progress_panel_damage(sub_comp);
            }
            _ => {}
        }
        true
    }

    pub fn get_component_group(component: tComponent) -> Option<(tComponentGroup, u8)> {
        let component = component as u8;
        if component >= tComponent::COMPONENT_PANEL_FRONT_LEFT as u8 {
            let group = if component >= tComponent::COMPONENT_BUMPER_FRONT as u8 {
                tComponentGroup::COMPGROUP_BUMPER
            } else {
                tComponentGroup::COMPGROUP_PANEL
            };
            Some((
                group,
                component - tComponent::COMPONENT_PANEL_FRONT_LEFT as u8,
            ))
        } else if component >= tComponent::COMPONENT_DOOR_BONNET as u8 {
            let group = if component == tComponent::COMPONENT_DOOR_BONNET as u8 {
                tComponentGroup::COMPGROUP_BONNET
            } else if component == tComponent::COMPONENT_DOOR_BOOT as u8 {
                tComponentGroup::COMPGROUP_BOOT
            } else {
                tComponentGroup::COMPGROUP_DOOR
            };
            Some((group, component - tComponent::COMPONENT_DOOR_BONNET as u8))
        } else if component >= tComponent::COMPONENT_WHEEL_FRONT_LEFT as u8 {
            Some((
                tComponentGroup::COMPGROUP_WHEEL,
                component - tComponent::COMPONENT_WHEEL_FRONT_LEFT as u8,
            ))
        } else {
            Some((tComponentGroup::COMPGROUP_DEFAULT, 0))
        }
    }

    pub fn set_door_status(&mut self, door: u8, status: u32) {
        self.door_status[door as usize] = status as u8;
    }

    pub fn get_door_status(&self, door: u8) -> u32 {
        self.door_status[door as usize] as u32
    }

    pub fn progress_door_damage(&mut self, door: u8) -> bool {
        let status = self.get_door_status(door);
        if status == ePanelStatus::PANEL_STATUS_MISSING as u32 {
            return false;
        }
        self.set_door_status(door, status + 1);
        true
    }

    pub fn set_panel_status(&mut self, panel: u8, status: u32) {
        self.panel_status = dpb(status, u32::from(panel) * 4, 4, self.panel_status);
    }

    pub fn get_panel_status(&self, panel: u8) -> u32 {
        Self::get_panel_status_from_bits(self.panel_status, panel)
    }

    pub fn get_panel_status_from_bits(panel_status: u32, panel: u8) -> u32 {
        ldb(u32::from(panel) * 4, 4, panel_status)
    }

    pub fn progress_panel_damage(&mut self, panel: u8) -> bool {
        let status = self.get_panel_status(panel);
        if status == eDoorStatus::DOOR_STATUS_MISSING as u32 {
            return false;
        }
        self.set_panel_status(panel, status + 1);
        true
    }

    pub fn set_light_status(&mut self, light: u8, status: u32) {
        self.light_status = dpb(status, u32::from(light) * 2, 2, self.light_status);
    }

    pub fn get_light_status(&self, light: u8) -> u32 {
        ldb(u32::from(light) * 2, 2, self.light_status)
    }

    pub fn set_wheel_status(&mut self, wheel: u8, status: u32) {
        self.wheel_status[wheel as usize] = status as u8;
    }

    pub fn get_wheel_status(&self, wheel: u8) -> u32 {
        self.wheel_status[wheel as usize] as u32
    }

    pub fn progress_wheel_damage(&mut self, wheel: u8) -> bool {
        let status = self.get_wheel_status(wheel);
        if status == eWheelStatus::WHEEL_STATUS_MISSING as u32 {
            return false;
        }
        self.set_wheel_status(wheel, status + 1);
        true
    }

    pub fn set_engine_status(&mut self, status: u32) {
        self.engine_status = status.min(250) as u8;
    }

    pub fn get_engine_status(&self) -> u32 {
        self.engine_status as u32
    }

    pub fn progress_engine_damage(&mut self, general: &mut CGeneral) -> bool {
        let status = self.get_engine_status();
        let mut new_status = status + 32 + u32::from(general.get_random_number() & 0x1F);
        if status < eEngineStatus::ENGINE_STATUS_ON_FIRE as u32
            && new_status > eEngineStatus::ENGINE_STATUS_ON_FIRE as u32 - 1
        {
            new_status = eEngineStatus::ENGINE_STATUS_ON_FIRE as u32 - 1;
        }
        self.set_engine_status(new_status);
        true
    }
}

fn ldb(shift: u32, width: u32, value: u32) -> u32 {
    (value >> shift) & ((1 << width) - 1)
}

fn dpb(value: u32, shift: u32, width: u32, base: u32) -> u32 {
    let mask = ((1 << width) - 1) << shift;
    (base & !mask) | ((value << shift) & mask)
}
