//! re3 reference: references/re-GTA/src/renderer/Clouds.cpp
//! re3 reference: references/re-GTA/src/renderer/Clouds.h
//!
//! Cloud dome / horizon cloud state + render intents.

use bevy::prelude::*;
use re3bevy_core::core::Coordinates::CVector;

pub const CLOUD_TEXTURE_NAMES: [&str; 5] =
    ["cloud1", "cloud2", "cloud3", "cloudhilit", "cloudmasked"];

const LOW_CLOUDS_X: [f32; 12] = [
    1.0, 0.7, 0.0, -0.7, -1.0, -0.7, 0.0, 0.7, 0.8, -0.8, 0.4, -0.4,
];
const LOW_CLOUDS_Y: [f32; 12] = [
    0.0, -0.7, -1.0, -0.7, 0.0, 0.7, 1.0, 0.7, 0.4, 0.4, -0.8, -0.8,
];
const LOW_CLOUDS_Z: [f32; 12] = [0.0, 1.0, 0.5, 0.0, 1.0, 0.3, 0.9, 0.4, 1.3, 1.4, 1.2, 1.7];

const COORS_OFFSET_X: [f32; 37] = [
    0.0, 60.0, 72.0, 48.0, 21.0, 12.0, 9.0, -3.0, -8.4, -18.0, -15.0, -36.0, -40.0, -48.0, -60.0,
    -24.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, -30.0, -20.0,
    10.0, 30.0, 0.0, -100.0, -100.0, -100.0, -100.0, -100.0, -100.0,
];
const COORS_OFFSET_Y: [f32; 37] = [
    100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0,
    100.0, 100.0, 100.0, -30.0, 10.0, -25.0, -5.0, 28.0, -10.0, 10.0, 0.0, 15.0, 40.0, -100.0,
    -100.0, -100.0, -100.0, -100.0, -40.0, -20.0, 0.0, 10.0, 30.0, 35.0,
];
const COORS_OFFSET_Z: [f32; 37] = [
    2.0, 1.0, 0.0, 0.3, 0.7, 1.4, 1.7, 0.24, 0.7, 1.3, 1.6, 1.0, 1.2, 0.3, 0.7, 1.4, 0.0, 0.1, 0.5,
    0.4, 0.55, 0.75, 1.0, 1.4, 1.7, 2.0, 2.0, 2.3, 1.9, 2.4, 2.0, 2.0, 1.5, 1.2, 1.7, 1.5, 2.1,
];

/// One cloud billboard to draw this frame (re3 `CClouds::Render` slice).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudRenderIntent {
    pub position: CVector,
    pub width: f32,
    pub height: f32,
    pub texture_index: u8,
    pub colour: [u8; 4],
    pub rotation_degrees: f32,
    pub additive: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CClouds {
    pub texture_names: [&'static str; 5],
    pub cloud_rotation: f32,
    pub individual_rotation: u32,
    pub camera_roll: f32,
    pub horizon_z: f32,
    pub render_intents: Vec<CloudRenderIntent>,
}

impl CClouds {
    pub fn init() -> Self {
        Self {
            texture_names: CLOUD_TEXTURE_NAMES,
            cloud_rotation: 0.0,
            individual_rotation: 0,
            camera_roll: 0.0,
            horizon_z: 0.0,
            render_intents: Vec::new(),
        }
    }

    /// re3 `CClouds::Update` + visible low/fluffy-cloud portion of
    /// `CClouds::Render`.
    pub fn update_and_build_intents(
        &mut self,
        camera_pos: CVector,
        camera_orientation: f32,
        time_step: f32,
        time_step_fix: f32,
        wind: f32,
        foggyness: f32,
        cloud_coverage: f32,
        low_clouds: [i32; 3],
        fluffy_top: [i32; 3],
        fluffy_bottom: [i32; 3],
    ) {
        self.render_intents.clear();
        let rotation_delta =
            wind * (camera_orientation - 0.85).sin() * 0.0025 * time_step_fix.max(0.0);
        self.cloud_rotation =
            (self.cloud_rotation + rotation_delta).rem_euclid(std::f32::consts::TAU);
        let individual_delta =
            ((wind * time_step.max(0.0) + 0.3 * time_step_fix.max(0.0)) * 60.0) as u32;
        self.individual_rotation = self.individual_rotation.wrapping_add(individual_delta);

        let coverage = foggyness.max(cloud_coverage).clamp(0.0, 1.0);
        let low_intensity = 1.0 - coverage;
        let low_colour = [
            scale_channel(low_clouds[0], low_intensity),
            scale_channel(low_clouds[1], low_intensity),
            scale_channel(low_clouds[2], low_intensity),
            255,
        ];

        // Clouds.cpp: cloudtype outer loop preserves the RenderWare texture
        // binding/order cloud1 -> cloud2 -> cloud3.
        for cloud_type in 0..3 {
            for i in (cloud_type..12).step_by(3) {
                self.render_intents.push(CloudRenderIntent {
                    position: CVector::new(
                        camera_pos.x + 800.0 * LOW_CLOUDS_X[i],
                        camera_pos.y + 800.0 * LOW_CLOUDS_Y[i],
                        40.0 + 60.0 * LOW_CLOUDS_Z[i],
                    ),
                    width: 320.0,
                    height: 40.0,
                    texture_index: cloud_type as u8,
                    colour: low_colour,
                    rotation_degrees: 0.0,
                    additive: true,
                });
            }
        }

        let fluffy_alpha = (160.0 * (1.0 - foggyness.clamp(0.0, 1.0))) as u8;
        if fluffy_alpha == 0 {
            return;
        }
        let fluffy_colour = [
            average_channel(fluffy_top[0], fluffy_bottom[0]),
            average_channel(fluffy_top[1], fluffy_bottom[1]),
            average_channel(fluffy_top[2], fluffy_bottom[2]),
            fluffy_alpha,
        ];
        let rot_sin = self.cloud_rotation.sin();
        let rot_cos = self.cloud_rotation.cos();
        let individual_rotation =
            (self.individual_rotation as u16 as f32 / 65_336.0 * 360.0).rem_euclid(360.0);
        for i in 0..37 {
            let x = 2.0 * COORS_OFFSET_X[i];
            let y = 2.0 * COORS_OFFSET_Y[i];
            self.render_intents.push(CloudRenderIntent {
                position: CVector::new(
                    x * rot_cos + y * rot_sin + camera_pos.x,
                    x * rot_sin - y * rot_cos + camera_pos.y,
                    40.0 * COORS_OFFSET_Z[i] + 40.0,
                ),
                width: 55.0,
                height: 55.0,
                texture_index: 4,
                colour: fluffy_colour,
                rotation_degrees: individual_rotation,
                additive: false,
            });
        }
    }

    pub fn drain_render_intents(&mut self) -> Vec<CloudRenderIntent> {
        std::mem::take(&mut self.render_intents)
    }
}

fn scale_channel(channel: i32, multiplier: f32) -> u8 {
    (channel.clamp(0, 255) as f32 * multiplier).clamp(0.0, 255.0) as u8
}

fn average_channel(top: i32, bottom: i32) -> u8 {
    ((top.clamp(0, 255) + bottom.clamp(0, 255)) / 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_intents_use_re3_low_and_fluffy_layouts_and_timecycle_colours() {
        let mut clouds = CClouds::init();
        clouds.update_and_build_intents(
            CVector::new(10.0, 20.0, 500.0),
            0.85,
            1.0,
            1.0,
            0.0,
            0.0,
            0.0,
            [80, 120, 160],
            [200, 180, 160],
            [100, 80, 60],
        );

        assert_eq!(clouds.render_intents.len(), 12 + 37);
        let first = clouds.render_intents[0];
        assert_eq!(first.position, CVector::new(810.0, 20.0, 40.0));
        assert_eq!(first.colour, [80, 120, 160, 255]);
        assert_eq!((first.width, first.height), (320.0, 40.0));
        assert!(first.additive);

        let first_fluffy = clouds.render_intents[12];
        assert_eq!(first_fluffy.texture_index, 4);
        assert_eq!(first_fluffy.colour, [150, 130, 110, 160]);
        assert_eq!((first_fluffy.width, first_fluffy.height), (55.0, 55.0));
        assert!(!first_fluffy.additive);
        // re3 cloud Z is world-space, not camera-relative.
        assert_eq!(first_fluffy.position.z, 120.0);
    }
}
