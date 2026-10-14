//! re3 reference: references/re-GTA/src/skel/skeleton.cpp
//! re3 reference: references/re-GTA/src/skel/skeleton.h
//!
//! RenderWare skeleton app/event state represented for Bevy.

use bevy::prelude::Resource;

#[allow(non_upper_case_globals)]
pub const rsRESOURCESDEFAULTARENASIZE: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RsInputDeviceType {
    rsKEYBOARD = 0,
    rsMOUSE = 1,
    rsPAD = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RsEventStatus {
    rsEVENTERROR = 0,
    rsEVENTPROCESSED = 1,
    rsEVENTNOTPROCESSED = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RsEvent {
    rsCAMERASIZE = 0,
    rsCOMMANDLINE = 1,
    rsFILELOAD = 2,
    rsINITDEBUG = 3,
    rsINPUTDEVICEATTACH = 4,
    rsLEFTBUTTONDOWN = 5,
    rsLEFTBUTTONUP = 6,
    rsMOUSEMOVE = 7,
    rsMOUSEWHEELMOVE = 8,
    rsPLUGINATTACH = 9,
    rsREGISTERIMAGELOADER = 10,
    rsRIGHTBUTTONDOWN = 11,
    rsRIGHTBUTTONUP = 12,
    rsRWINITIALIZE = 21,
    rsRWTERMINATE = 22,
    rsSELECTDEVICE = 23,
    rsINITIALIZE = 24,
    rsTERMINATE = 25,
    rsIDLE = 26,
    rsFRONTENDIDLE = 27,
    rsKEYDOWN = 28,
    rsKEYUP = 29,
    rsQUITAPP = 30,
    rsPADBUTTONDOWN = 31,
    rsPADBUTTONUP = 32,
    rsPADANALOGUELEFT = 33,
    rsPADANALOGUELEFTRESET = 34,
    rsPADANALOGUERIGHT = 35,
    rsPADANALOGUERIGHTRESET = 36,
    rsPREINITCOMMANDLINE = 37,
    rsACTIVATE = 38,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum RsKeyCodes {
    rsESC = 1000,
    rsF1 = 1001,
    rsF2 = 1002,
    rsF3 = 1003,
    rsF4 = 1004,
    rsF5 = 1005,
    rsF6 = 1006,
    rsF7 = 1007,
    rsF8 = 1008,
    rsF9 = 1009,
    rsF10 = 1010,
    rsF11 = 1011,
    rsF12 = 1012,
    rsUP = 1019,
    rsDOWN = 1020,
    rsLEFT = 1021,
    rsRIGHT = 1022,
    rsBACKSP = 1042,
    rsTAB = 1043,
    rsENTER = 1045,
    rsSHIFT = 1048,
    rsLCTRL = 1049,
    rsRCTRL = 1050,
    rsLALT = 1051,
    rsRALT = 1052,
    rsNULL = 1056,
    rsMOUSELEFTBUTTON = 1,
    rsMOUSMIDDLEBUTTON = 2,
    rsMOUSERIGHTBUTTON = 3,
    rsMOUSEWHEELUPBUTTON = 4,
    rsMOUSEWHEELDOWNBUTTON = 5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsInputDevice {
    pub input_device_type: RsInputDeviceType,
    pub used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub struct RsGlobalType {
    pub app_name: String,
    pub width: i32,
    pub height: i32,
    pub maximum_width: i32,
    pub maximum_height: i32,
    pub max_fps: i32,
    pub quit: bool,
    pub keyboard: RsInputDevice,
    pub mouse: RsInputDevice,
    pub pad: RsInputDevice,
}

impl Default for RsGlobalType {
    fn default() -> Self {
        Self {
            app_name: "rust-gta-3".to_owned(),
            width: 640,
            height: 480,
            maximum_width: 640,
            maximum_height: 480,
            max_fps: 30,
            quit: false,
            keyboard: RsInputDevice {
                input_device_type: RsInputDeviceType::rsKEYBOARD,
                used: false,
            },
            mouse: RsInputDevice {
                input_device_type: RsInputDeviceType::rsMOUSE,
                used: false,
            },
            pad: RsInputDevice {
                input_device_type: RsInputDeviceType::rsPAD,
                used: false,
            },
        }
    }
}

pub fn RsInitialize(global: &mut RsGlobalType) -> bool {
    *global = RsGlobalType::default();
    true
}

pub fn RsInputDeviceAttach(global: &mut RsGlobalType, input_device: RsInputDeviceType) -> bool {
    match input_device {
        RsInputDeviceType::rsKEYBOARD => global.keyboard.used = true,
        RsInputDeviceType::rsMOUSE => global.mouse.used = true,
        RsInputDeviceType::rsPAD => global.pad.used = true,
    }
    true
}

pub fn RsEventHandler(_event: RsEvent) -> RsEventStatus {
    RsEventStatus::rsEVENTNOTPROCESSED
}
