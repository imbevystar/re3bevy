//! re3 reference: references/re-GTA/src/renderer/Shadows.cpp
//! re3 reference: references/re-GTA/src/renderer/Shadows.h
//!
//! `CShadows::Init` state.

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

#[path = "ShadowProjection.rs"]
mod projection;
pub use projection::{ProjectedShadowGeometry, ShadowProjectionDiagnostics, project_car_light, project_additive_shadow};

/// Transient `StoreCarLightShadow` / `StoreShadowToBeRendered` entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarLightShadowIntent {
    pub key: u32,
    pub texture: usize,
    pub position: CVector,
    pub front: [f32; 2],
    pub side: [f32; 2],
    pub color: [u8; 3],
}

#[cfg(test)]
mod car_light_view_tests {
    use super::*;

    #[test]
    fn native_distance_view_angle_and_special_camera_gates() {
        let light = CarLightShadowIntent {
            key: 22,
            texture: 4,
            position: CVector::new(0.0, 20.0, 2.0),
            front: [0.0, 7.0],
            side: [7.0, 0.0],
            color: [45; 3],
        };
        let view = CarLightShadowView {
            position: CVector::ZERO,
            front: [0.0, 1.0],
            special_camera: false,
        };
        assert_eq!(light.for_view(view, 7.0).unwrap().color, [45; 3]);
        let far = CarLightShadowIntent {
            position: CVector::new(0.0, 24.0, 2.0),
            ..light
        };
        assert_eq!(far.for_view(view, 7.0).unwrap().color, [14; 3]);
        let outside = CarLightShadowIntent {
            position: CVector::new(0.0, 27.0, 2.0),
            ..light
        };
        assert!(outside.for_view(view, 7.0).is_none());
        let behind = CarLightShadowIntent {
            position: CVector::new(0.0, -8.0, 2.0),
            ..light
        };
        assert!(behind.for_view(view, 7.0).is_none());
        assert_eq!(
            outside
                .for_view(
                    CarLightShadowView {
                        special_camera: true,
                        ..view
                    },
                    7.0
                )
                .unwrap()
                .color,
            [45; 3]
        );
    }
}

/// `CamFrontXNorm` / `CamFrontYNorm` and the native special-camera gate.
#[derive(Debug, Clone, Copy)]
pub struct CarLightShadowView {
    pub position: CVector,
    pub front: [f32; 2],
    pub special_camera: bool,
}

impl CarLightShadowIntent {
    /// CShadows::StoreCarLightShadow preserves the native 20.25m fade start
    /// and 18m fade origin (including their discontinuity).
    pub fn for_view(mut self, view: CarLightShadowView, max_view_angle: f32) -> Option<Self> {
        let offset = [
            self.position.x - view.position.x,
            self.position.y - view.position.y,
        ];
        let distance_squared = offset[0] * offset[0] + offset[1] * offset[1];
        if !view.special_camera {
            if distance_squared >= 27.0 * 27.0
                || view.front[0] * offset[0] + view.front[1] * offset[1] <= -max_view_angle
            {
                return None;
            }
            let distance = distance_squared.sqrt();
            if distance >= 27.0 * (1.0 - 1.0 / 4.0) {
                let fade = 1.0 - (3.0 / 27.0) * (distance - 27.0 * (1.0 - 1.0 / 3.0));
                self.color = self.color.map(|channel| (f32::from(channel) * fade) as u8);
            }
        }
        Some(self)
    }
}

pub const SHADOW_TEXTURE_NAMES: [&str; 14] = [
    "shad_car",
    "shad_ped",
    "shad_heli",
    "shad_exp",
    "headlight",
    "outline_64",
    "outline2_64",
    "outline3_64",
    "bloodpool_64",
    "reflection01",
    "goal",
    "walk_dont",
    "wincrack_32",
    "lamp_shad_64",
];
pub const MAX_STORED_SHADOWS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptShadow {
    pub texture: u8,
    pub position: CVector,
    pub front: [f32; 2],
    pub side: [f32; 2],
    pub intensity: i16,
    pub color: [u8; 3],
    pub corners: Option<[CVector; 4]>,
}

/// Transient additive fire shadow emitted by `CFire::ProcessFire` when no
/// entity is attached (`SHADOWTYPE_ADDITIVE`, `shad_exp`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticShadowIntent {
    pub key: u32,
    pub texture: u8,
    pub position: CVector,
    pub front: [f32; 2],
    pub side: [f32; 2],
    pub intensity: i16,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, Resource)]
pub struct CShadows {
    pub texture_names: [&'static str; 14],
    pub shadow_index_list: [u16; 24],
    /// Raised by `CShadows::TidyUpShadows`; render extraction clears any
    /// transient projected-shadow entities before resetting it.
    pub tidy_up_requested: bool,
    /// re3's per-frame `aShadows` entries emitted by COMMAND_DRAW_SHADOW.
    pub script_shadows: Vec<ScriptShadow>,
    pub static_shadows: Vec<StaticShadowIntent>,
    pub car_light_shadows: Vec<CarLightShadowIntent>,
    pub pickup_shadows: Vec<StaticShadowIntent>,
    pub car_light_capacity_drops: u64,
}

impl CShadows {
    pub fn init() -> Self {
        Self {
            texture_names: SHADOW_TEXTURE_NAMES,
            shadow_index_list: [
                0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 5, 4, 0, 6, 5, 0, 7, 6, 0, 8, 7, 0, 9, 8,
            ],
            tidy_up_requested: false,
            script_shadows: Vec::new(),
            static_shadows: Vec::new(),
            car_light_shadows: Vec::new(),
            pickup_shadows: Vec::new(),
            car_light_capacity_drops: 0,
        }
    }

    pub fn store_car_light_shadow(
        &mut self,
        light: CarLightShadowIntent,
        view: CarLightShadowView,
        max_view_angle: f32,
    ) {
        let Some(light) = light.for_view(view, max_view_angle) else {
            return;
        };
        if self.script_shadows.len() + self.car_light_shadows.len() >= MAX_STORED_SHADOWS {
            self.car_light_capacity_drops += 1;
            return;
        }
        self.car_light_shadows.push(light);
    }

    #[allow(non_snake_case)]
    pub fn TidyUpShadows(&mut self) {
        self.tidy_up_requested = true;
        self.car_light_shadows.clear();
        self.pickup_shadows.clear();
    }
}
