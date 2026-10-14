//! re3 reference: references/re-GTA/src/audio/AudioCollision.cpp
//! re3 reference: references/re-GTA/src/audio/AudioCollision.h

use rust_gta_3_core::core::Coordinates::CVector;

use crate::core::SurfaceTable::eSurfaceType;

use super::AudioSamples::*;

pub const NUMAUDIOCOLLISIONS: usize = 10;
pub const COLLISION_MAX_DIST: f32 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cAudioCollision {
    pub m_pEntity1: Option<i32>,
    pub m_pEntity2: Option<i32>,
    pub m_bSurface1: u8,
    pub m_bSurface2: u8,
    pub m_fIntensity1: f32,
    pub m_fIntensity2: f32,
    pub m_vecPosition: CVector,
    pub m_fDistance: f32,
    pub m_nBaseVolume: i32,
}

impl Default for cAudioCollision {
    fn default() -> Self {
        let mut collision = Self {
            m_pEntity1: None,
            m_pEntity2: None,
            m_bSurface1: 0,
            m_bSurface2: 0,
            m_fIntensity1: 0.0,
            m_fIntensity2: 0.0,
            m_vecPosition: CVector::ZERO,
            m_fDistance: 0.0,
            m_nBaseVolume: 0,
        };
        collision.Reset();
        collision
    }
}

impl cAudioCollision {
    pub fn Reset(&mut self) {
        self.m_pEntity1 = None;
        self.m_pEntity2 = None;
        self.m_bSurface1 = 0;
        self.m_bSurface2 = 0;
        self.m_fIntensity1 = 0.0;
        self.m_fIntensity2 = 0.0;
        self.m_vecPosition = CVector::ZERO;
        self.m_fDistance = 0.0;
        self.m_nBaseVolume = 0;
    }
}

#[derive(Debug, Clone)]
pub struct cAudioCollisionManager {
    pub m_asCollisions1: [cAudioCollision; NUMAUDIOCOLLISIONS],
    pub m_asCollisions2: [cAudioCollision; NUMAUDIOCOLLISIONS],
    pub m_bIndicesTable: [usize; NUMAUDIOCOLLISIONS],
    pub m_bCollisionsInQueue: usize,
    pub m_sQueue: cAudioCollision,
}

impl Default for cAudioCollisionManager {
    fn default() -> Self {
        Self {
            m_asCollisions1: [cAudioCollision::default(); NUMAUDIOCOLLISIONS],
            m_asCollisions2: [cAudioCollision::default(); NUMAUDIOCOLLISIONS],
            m_bIndicesTable: [NUMAUDIOCOLLISIONS; NUMAUDIOCOLLISIONS],
            m_bCollisionsInQueue: 0,
            m_sQueue: cAudioCollision::default(),
        }
    }
}

impl cAudioCollisionManager {
    pub fn AddCollisionToRequestedQueue(&mut self) {
        let collisions_index = if self.m_bCollisionsInQueue < NUMAUDIOCOLLISIONS {
            let index = self.m_bCollisionsInQueue;
            self.m_bCollisionsInQueue += 1;
            index
        } else {
            let index = self.m_bIndicesTable[NUMAUDIOCOLLISIONS - 1];
            if self.m_sQueue.m_fDistance >= self.m_asCollisions1[index].m_fDistance {
                return;
            }
            index
        };

        self.m_asCollisions1[collisions_index] = self.m_sQueue;

        let mut i = 0usize;
        if collisions_index != 0 {
            while i < collisions_index {
                let ordered = self.m_bIndicesTable[i];
                if ordered >= NUMAUDIOCOLLISIONS
                    || self.m_asCollisions1[ordered].m_fDistance
                        > self.m_asCollisions1[collisions_index].m_fDistance
                {
                    break;
                }
                i += 1;
            }
            if i < collisions_index {
                for j in (i + 1..NUMAUDIOCOLLISIONS).rev() {
                    self.m_bIndicesTable[j] = self.m_bIndicesTable[j - 1];
                }
            }
        }
        self.m_bIndicesTable[i] = collisions_index;
    }

    pub fn reset_requested_queue(&mut self) {
        self.m_bIndicesTable = [NUMAUDIOCOLLISIONS; NUMAUDIOCOLLISIONS];
        self.m_bCollisionsInQueue = 0;
    }
}

pub const G_ONE_SHOT_COL: [i32; 33] = [
    SFX_COL_TARMAC_1,
    SFX_COL_TARMAC_1,
    SFX_COL_GRASS_1,
    SFX_COL_GRAVEL_1,
    SFX_COL_MUD_1,
    SFX_COL_TARMAC_1,
    SFX_COL_CAR_1,
    SFX_COL_GRASS_1,
    SFX_COL_SCAFFOLD_POLE_1,
    SFX_COL_GARAGE_DOOR_1,
    SFX_COL_CAR_PANEL_1,
    SFX_COL_THICK_METAL_PLATE_1,
    SFX_COL_SCAFFOLD_POLE_1,
    SFX_COL_LAMP_POST_1,
    SFX_COL_HYDRANT_1,
    SFX_COL_HYDRANT_1,
    SFX_COL_METAL_CHAIN_FENCE_1,
    SFX_COL_PED_1,
    SFX_COL_SAND_1,
    SFX_SPLASH_1,
    SFX_COL_WOOD_CRATES_1,
    SFX_COL_WOOD_BENCH_1,
    SFX_COL_WOOD_SOLID_1,
    SFX_COL_GRASS_1,
    SFX_COL_GRASS_1,
    SFX_COL_VEG_1,
    SFX_COL_TARMAC_1,
    SFX_COL_CONTAINER_1,
    SFX_COL_NEWS_VENDOR_1,
    SFX_TYRE_BUMP,
    SFX_COL_CARDBOARD_1,
    SFX_COL_TARMAC_1,
    SFX_COL_GATE,
];

pub fn GetCollisionRatio(a: f32, b: f32, c: f32, d: f32) -> f32 {
    if a <= b { 0.0 } else { (a.min(c) - b) / d }
}

pub fn GetCollisionOneShotRatio(surface: u32, intensity: f32) -> f32 {
    let s = |surface: eSurfaceType| surface as u32;
    match surface {
        x if x == s(eSurfaceType::SURFACE_DEFAULT)
            || x == s(eSurfaceType::SURFACE_TARMAC)
            || x == s(eSurfaceType::SURFACE_PAVEMENT)
            || x == s(eSurfaceType::SURFACE_STEEP_CLIFF)
            || x == s(eSurfaceType::SURFACE_TRANSPARENT_STONE) =>
        {
            GetCollisionRatio(intensity, 10.0, 60.0, 50.0)
        }
        x if x == s(eSurfaceType::SURFACE_GRASS)
            || x == s(eSurfaceType::SURFACE_CARDBOARDBOX)
            || x == s(eSurfaceType::SURFACE_GRAVEL)
            || x == s(eSurfaceType::SURFACE_MUD_DRY) =>
        {
            GetCollisionRatio(intensity, 0.0, 2.0, 2.0)
        }
        x if x == s(eSurfaceType::SURFACE_CAR) => GetCollisionRatio(intensity, 6.0, 50.0, 44.0),
        x if x == s(eSurfaceType::SURFACE_GLASS)
            || x == s(eSurfaceType::SURFACE_METAL_CHAIN_FENCE) =>
        {
            GetCollisionRatio(intensity, 0.1, 10.0, 9.9)
        }
        x if x == s(eSurfaceType::SURFACE_TRANSPARENT_CLOTH)
            || x == s(eSurfaceType::SURFACE_THICK_METAL_PLATE) =>
        {
            GetCollisionRatio(intensity, 30.0, 130.0, 100.0)
        }
        x if x == s(eSurfaceType::SURFACE_GARAGE_DOOR) => {
            GetCollisionRatio(intensity, 20.0, 100.0, 80.0)
        }
        x if x == s(eSurfaceType::SURFACE_CAR_PANEL) => GetCollisionRatio(intensity, 0.0, 4.0, 4.0),
        x if x == s(eSurfaceType::SURFACE_SCAFFOLD_POLE)
            || x == s(eSurfaceType::SURFACE_METAL_GATE)
            || x == s(eSurfaceType::SURFACE_LAMP_POST) =>
        {
            GetCollisionRatio(intensity, 1.0, 10.0, 9.0)
        }
        x if x == s(eSurfaceType::SURFACE_FIRE_HYDRANT) => {
            GetCollisionRatio(intensity, 1.0, 15.0, 14.0)
        }
        x if x == s(eSurfaceType::SURFACE_GIRDER) => GetCollisionRatio(intensity, 8.0, 50.0, 42.0),
        x if x == s(eSurfaceType::SURFACE_PED) => GetCollisionRatio(intensity, 0.0, 20.0, 20.0),
        x if x == s(eSurfaceType::SURFACE_SAND)
            || x == s(eSurfaceType::SURFACE_WATER)
            || x == s(eSurfaceType::SURFACE_RUBBER)
            || x == s(eSurfaceType::SURFACE_WHEELBASE) =>
        {
            GetCollisionRatio(intensity, 0.0, 10.0, 10.0)
        }
        x if x == s(eSurfaceType::SURFACE_WOOD_CRATES) => {
            GetCollisionRatio(intensity, 1.0, 4.0, 3.0)
        }
        x if x == s(eSurfaceType::SURFACE_WOOD_BENCH) => {
            GetCollisionRatio(intensity, 0.1, 5.0, 4.9)
        }
        x if x == s(eSurfaceType::SURFACE_WOOD_SOLID) => {
            GetCollisionRatio(intensity, 0.1, 40.0, 39.9)
        }
        x if x == s(eSurfaceType::SURFACE_PLASTIC) => GetCollisionRatio(intensity, 0.1, 4.0, 3.9),
        x if x == s(eSurfaceType::SURFACE_HEDGE) => GetCollisionRatio(intensity, 0.0, 0.5, 0.5),
        x if x == s(eSurfaceType::SURFACE_CONTAINER) => {
            GetCollisionRatio(intensity, 4.0, 40.0, 36.0)
        }
        x if x == s(eSurfaceType::SURFACE_NEWS_VENDOR) => {
            GetCollisionRatio(intensity, 0.0, 5.0, 5.0)
        }
        _ => 0.0,
    }
}

pub fn GetCollisionLoopingRatio(_a: u32, _b: u32, c: f32) -> f32 {
    GetCollisionRatio(c, 0.0, 0.02, 0.02)
}
