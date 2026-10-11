//! re3 reference: references/re-GTA/src/audio/PolRadio.cpp
//! re3 reference: references/re-GTA/src/audio/PolRadio.h

use re3bevy_core::core::Coordinates::CVector;

use super::AudioSamples::*;

pub type eCrimeType = i32;
pub const CRIME_NONE: eCrimeType = 0;
pub const CRIME_POSSESSION_GUN: eCrimeType = 1;
pub const CRIME_HIT_PED: eCrimeType = 2;
pub const CRIME_HIT_COP: eCrimeType = 3;
pub const CRIME_SHOOT_PED: eCrimeType = 4;
pub const CRIME_SHOOT_COP: eCrimeType = 5;
pub const CRIME_STEAL_CAR: eCrimeType = 6;
pub const CRIME_RUN_REDLIGHT: eCrimeType = 7;
pub const CRIME_RECKLESS_DRIVING: eCrimeType = 8;
pub const CRIME_SPEEDING: eCrimeType = 9;
pub const CRIME_RUNOVER_PED: eCrimeType = 10;
pub const CRIME_RUNOVER_COP: eCrimeType = 11;
pub const CRIME_SHOOT_HELI: eCrimeType = 12;
pub const CRIME_PED_BURNED: eCrimeType = 13;
pub const CRIME_COP_BURNED: eCrimeType = 14;
pub const CRIME_VEHICLE_BURNED: eCrimeType = 15;
pub const CRIME_DESTROYED_CESSNA: eCrimeType = 16;
pub const NUM_CRIME_TYPES: usize = 17;

pub const POLICE_RADIO_QUEUE_MAX_SAMPLES: usize = 60;
pub const NUMAUDIOZONES: usize = 35;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cAMCrime {
    pub r#type: eCrimeType,
    pub position: CVector,
    pub timer: u16,
}

impl Default for cAMCrime {
    fn default() -> Self {
        Self {
            r#type: CRIME_NONE,
            position: CVector::ZERO,
            timer: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct cPoliceRadioQueue {
    pub m_aSamples: [u32; POLICE_RADIO_QUEUE_MAX_SAMPLES],
    pub m_nSamplesInQueue: u8,
    pub m_nAddOffset: u8,
    pub m_nRemoveOffset: u8,
}

impl Default for cPoliceRadioQueue {
    fn default() -> Self {
        let mut queue = Self {
            m_aSamples: [TOTAL_AUDIO_SAMPLES as u32; POLICE_RADIO_QUEUE_MAX_SAMPLES],
            m_nSamplesInQueue: 0,
            m_nAddOffset: 0,
            m_nRemoveOffset: 0,
        };
        queue.Reset();
        queue
    }
}

impl cPoliceRadioQueue {
    pub fn Reset(&mut self) {
        self.m_nAddOffset = 0;
        self.m_nRemoveOffset = 0;
        self.m_nSamplesInQueue = 0;
    }

    pub fn Add(&mut self, sample: u32) -> bool {
        if self.m_nSamplesInQueue as usize != POLICE_RADIO_QUEUE_MAX_SAMPLES {
            self.m_aSamples[self.m_nAddOffset as usize] = sample;
            self.m_nSamplesInQueue += 1;
            self.m_nAddOffset = (self.m_nAddOffset + 1) % POLICE_RADIO_QUEUE_MAX_SAMPLES as u8;
            true
        } else {
            false
        }
    }

    pub fn Remove(&mut self) -> u32 {
        if self.m_nSamplesInQueue != 0 {
            let sample = self.m_aSamples[self.m_nRemoveOffset as usize];
            self.m_nSamplesInQueue -= 1;
            self.m_nRemoveOffset =
                (self.m_nRemoveOffset + 1) % POLICE_RADIO_QUEUE_MAX_SAMPLES as u8;
            sample
        } else {
            TOTAL_AUDIO_SAMPLES as u32
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct tPoliceRadioZone {
    pub m_aName: &'static str,
    pub m_nSampleIndex: u32,
    pub field_12: i32,
}

impl Default for tPoliceRadioZone {
    fn default() -> Self {
        Self {
            m_aName: "",
            m_nSampleIndex: TOTAL_AUDIO_SAMPLES as u32,
            field_12: 0,
        }
    }
}

pub const ZONE_SFX: [tPoliceRadioZone; NUMAUDIOZONES] = [
    tPoliceRadioZone {
        m_aName: "HOSPI_2",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "CONSTRU",
        m_nSampleIndex: SFX_POLICE_RADIO_FORT_STAUNTON as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "STADIUM",
        m_nSampleIndex: SFX_POLICE_RADIO_ASPATRIA as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "YAKUSA",
        m_nSampleIndex: SFX_POLICE_RADIO_TORRINGTON as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SHOPING",
        m_nSampleIndex: SFX_POLICE_RADIO_BEDFORD_POINT as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "COM_EAS",
        m_nSampleIndex: SFX_POLICE_RADIO_NEWPORT as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PARK",
        m_nSampleIndex: SFX_POLICE_RADIO_BELLEVILLE_PARK as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "UNIVERS",
        m_nSampleIndex: SFX_POLICE_RADIO_LIBERTY_CAMPUS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "BIG_DAM",
        m_nSampleIndex: SFX_POLICE_RADIO_COCHRANE_DAM as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SUB_IND",
        m_nSampleIndex: SFX_POLICE_RADIO_PIKE_CREEK as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SWANKS",
        m_nSampleIndex: SFX_POLICE_RADIO_CEDAR_GROVE as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PROJECT",
        m_nSampleIndex: SFX_POLICE_RADIO_WICHITA_GARDENS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "AIRPORT",
        m_nSampleIndex: SFX_POLICE_RADIO_FRANCIS_INTERNATIONAL_AIRPORT as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PORT_W",
        m_nSampleIndex: SFX_POLICE_RADIO_CALLAHAN_POINT as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PORT_S",
        m_nSampleIndex: SFX_POLICE_RADIO_ATLANTIC_QUAYS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PORT_E",
        m_nSampleIndex: SFX_POLICE_RADIO_PORTLAND_HARBOUR as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "PORT_I",
        m_nSampleIndex: SFX_POLICE_RADIO_TRENTON as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "CHINA",
        m_nSampleIndex: SFX_POLICE_RADIO_CHINATOWN as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "REDLIGH",
        m_nSampleIndex: SFX_POLICE_RADIO_RED_LIGHT_DISTRICT as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "TOWERS",
        m_nSampleIndex: SFX_POLICE_RADIO_HEPBURN_HEIGHTS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "LITTLEI",
        m_nSampleIndex: SFX_POLICE_RADIO_SAINT_MARKS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "HARWOOD",
        m_nSampleIndex: SFX_POLICE_RADIO_HARWOOD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "EASTBAY",
        m_nSampleIndex: SFX_POLICE_RADIO_PORTLAND_BEACH as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "S_VIEW",
        m_nSampleIndex: SFX_POLICE_RADIO_PORTLAND_STRAIGHTS as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "CITYZON",
        m_nSampleIndex: SFX_POLICE_RADIO_LIBERTY_CITY as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "IND_ZON",
        m_nSampleIndex: SFX_POLICE_RADIO_PORTLAND as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "COM_ZON",
        m_nSampleIndex: SFX_POLICE_RADIO_STAUNTON_ISLAND as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SUB_ZON",
        m_nSampleIndex: SFX_POLICE_RADIO_SHORESIDE_VALE as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SUB_ZO2",
        m_nSampleIndex: SFX_POLICE_RADIO_SHORESIDE_VALE as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "SUB_ZO3",
        m_nSampleIndex: SFX_POLICE_RADIO_SHORESIDE_VALE as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "A",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "A",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "A",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "A",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
    tPoliceRadioZone {
        m_aName: "A",
        m_nSampleIndex: SFX_POLICE_RADIO_ROCKFORD as u32,
        field_12: 0,
    },
];
