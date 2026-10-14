//! re3 reference: references/re-GTA/src/renderer/PointLights.cpp
//! re3 reference: references/re-GTA/src/renderer/PointLights.h

use bevy::prelude::Resource;
use rust_gta_3_core::core::Coordinates::CVector;

pub const NUMPOINTLIGHTS: usize = 32;
pub const MAX_DIST: f32 = 22.0;

pub const LIGHT_POINT: u8 = 0;
pub const LIGHT_DIRECTIONAL: u8 = 1;
pub const LIGHT_DARKEN: u8 = 2;
pub const LIGHT_FOGONLY_ALWAYS: u8 = 3;
pub const LIGHT_FOGONLY: u8 = 4;

pub const FOG_NONE: u8 = 0;
pub const FOG_NORMAL: u8 = 1;
pub const FOG_ALWAYS: u8 = 2;

const FOG_AREA_LENGTH: f32 = 12.0;
const FOG_AREA_WIDTH: f32 = 5.0;
const FOG_AREA_RADIUS: f32 = 9.0;
const FOG_SIZES: [f32; 8] = [1.3, 2.0, 1.7, 2.0, 1.4, 2.1, 1.5, 2.3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CRegisteredPointLight {
    pub coors: CVector,
    pub dir: CVector,
    pub radius: f32,
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub light_type: u8,
    pub fog_type: u8,
    pub cast_extra_shadows: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtraDirectionalLightIntent {
    pub direction: CVector,
    pub colour: [f32; 3],
}

/// One `CPointLights::RenderFogEffect` sprite before camera projection. re3
/// draws these with the particle TXD's `pointlight` raster and ONE/ONE blend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLightFogSpriteIntent {
    pub position: CVector,
    /// World-space half size. `CSprite::CalcScreenCoors` projects one GTA unit
    /// before RenderOneXLUSprite multiplies by this authored size.
    pub size: f32,
    pub rotation: f32,
    /// Pre-vertex RGB and intensity are kept separately because re3 multiplies
    /// them again in `RwIm2DVertexSetIntRGBA`.
    pub light_colour: [f32; 3],
    pub intensity: f32,
}

#[derive(Debug, Clone, Resource)]
pub struct CPointLights {
    pub lights: Vec<CRegisteredPointLight>,
    pub affecting_object: Vec<ExtraDirectionalLightIntent>,
}

impl Default for CPointLights {
    fn default() -> Self {
        Self {
            lights: Vec::with_capacity(NUMPOINTLIGHTS),
            affecting_object: Vec::new(),
        }
    }
}

impl CPointLights {
    pub fn init_per_frame(&mut self) {
        self.lights.clear();
        self.affecting_object.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_light(
        &mut self,
        light_type: u8,
        coors: CVector,
        dir: CVector,
        radius: f32,
        red: f32,
        green: f32,
        blue: f32,
        fog_type: u8,
        cast_extra_shadows: bool,
        camera_position: CVector,
    ) {
        if self.lights.len() >= NUMPOINTLIGHTS {
            return;
        }
        let dist = sub(coors, camera_position);
        if dist.x.abs() >= MAX_DIST || dist.y.abs() >= MAX_DIST {
            return;
        }
        let distance = dist.magnitude();
        if distance >= MAX_DIST {
            return;
        }
        let fade = if distance < MAX_DIST * 0.75 {
            1.0
        } else {
            1.0 - (distance / MAX_DIST - 0.75) * 4.0
        };
        self.lights.push(CRegisteredPointLight {
            coors,
            dir,
            radius,
            red: red * fade,
            green: green * fade,
            blue: blue * fade,
            light_type,
            fog_type,
            cast_extra_shadows,
        });
    }

    pub fn generate_lights_affecting_object(&mut self, obj_coors: CVector) -> f32 {
        self.affecting_object.clear();
        let mut ret = 1.0;
        for light in &self.lights {
            if light.light_type == LIGHT_FOGONLY || light.light_type == LIGHT_FOGONLY_ALWAYS {
                continue;
            }
            let dist = sub(light.coors, obj_coors);
            let radius = light.radius;
            if dist.x.abs() >= radius || dist.y.abs() >= radius || dist.z.abs() >= radius {
                continue;
            }
            let distance = dist.magnitude();
            if distance >= radius {
                continue;
            }
            let dist_norm = distance / radius;
            if light.light_type == LIGHT_DARKEN {
                ret *= dist_norm;
                continue;
            }
            let mut intensity = if dist_norm < 0.5 {
                1.0
            } else {
                1.0 - (dist_norm - 0.5) / 0.5
            };
            if distance != 0.0 {
                let dir = scale(dist, 1.0 / distance);
                if light.light_type == LIGHT_DIRECTIONAL {
                    let dot = -dot(dir, light.dir);
                    intensity *= ((dot - 0.5) * 2.0).max(0.0);
                }
                if intensity > 0.0 {
                    self.affecting_object.push(ExtraDirectionalLightIntent {
                        direction: dir,
                        colour: [
                            light.red * intensity,
                            light.green * intensity,
                            light.blue * intensity,
                        ],
                    });
                }
            }
        }
        ret
    }

    pub fn remove_lights_affecting_object(&mut self) {
        self.affecting_object.clear();
    }

    /// re3 `CPointLights::RenderFogEffect` expressed as billboard intents.
    /// Static-world vertical probes stay with the world owner through the
    /// callback; this resource only owns the original grid/intensity rules.
    pub fn render_fog_effect_into(
        &self,
        camera_position: CVector,
        fogginess: f32,
        time_ms: u32,
        mut ground_z: impl FnMut(CVector, f32) -> Option<f32>,
        out: &mut Vec<PointLightFogSpriteIntent>,
    ) {
        out.clear();
        for light in &self.lights {
            if light.fog_type != FOG_NORMAL && light.fog_type != FOG_ALWAYS {
                continue;
            }
            let fog = if light.fog_type == FOG_NORMAL {
                fogginess
            } else {
                1.0
            };
            if fog == 0.0 {
                continue;
            }

            if light.light_type == LIGHT_DIRECTIONAL {
                self.directional_fog_sprites(light, fog, time_ms, &mut ground_z, out);
            } else if matches!(
                light.light_type,
                LIGHT_POINT | LIGHT_FOGONLY | LIGHT_FOGONLY_ALWAYS
            ) {
                self.point_fog_sprites(
                    light,
                    camera_position,
                    fog,
                    time_ms,
                    &mut ground_z,
                    out,
                );
            }
        }
    }

    fn directional_fog_sprites(
        &self,
        light: &CRegisteredPointLight,
        fogginess: f32,
        time_ms: u32,
        ground_z: &mut impl FnMut(CVector, f32) -> Option<f32>,
        out: &mut Vec<PointLightFogSpriteIntent>,
    ) {
        let coors2_x = light.coors.x + FOG_AREA_LENGTH * light.dir.x;
        let coors2_y = light.coors.y + FOG_AREA_LENGTH * light.dir.y;
        let xmin = coors2_x.min(light.coors.x) - FOG_AREA_WIDTH;
        let xmax = coors2_x.max(light.coors.x) + FOG_AREA_WIDTH;
        let ymin = coors2_y.min(light.coors.y) - FOG_AREA_WIDTH;
        let ymax = coors2_y.max(light.coors.y) + FOG_AREA_WIDTH;
        let rotation = (time_ms & 0x1fff) as f32 * 2.0 * 3.14 / 0x2000 as f32;

        let mut xi = aligned_grid_start(xmin, 4);
        let x_end = xmax as i16 as i32 + 4;
        while xi <= x_end {
            let mut yi = aligned_grid_start(ymin, 4);
            let y_end = ymax as i16 as i32 + 4;
            while yi <= y_end {
                let r = ((xi ^ yi) >> 2) & 0x0f;
                if r & 1 != 0 {
                    let dx = xi as f32 - light.coors.x;
                    let dy = yi as f32 - light.coors.y;
                    let dot2 = dx * light.dir.x + dy * light.dir.y;
                    let dist2_sq = dx * dx + dy * dy;
                    let line2_sq = dist2_sq - dot2 * dot2;
                    if dot2 > 0.0
                        && dot2 < FOG_AREA_LENGTH
                        && line2_sq < FOG_AREA_WIDTH * FOG_AREA_WIDTH
                    {
                        let top = CVector::new(xi as f32, yi as f32, light.coors.z + 10.0);
                        if let Some(z) = ground_z(top, light.coors.z - 10.0) {
                            let fog_pos = CVector::new(xi as f32, yi as f32, z + 1.3);
                            let dx = fog_pos.x - light.coors.x;
                            let dy = fog_pos.y - light.coors.y;
                            let dz = fog_pos.z - light.coors.z;
                            let dot = dx * light.dir.x + dy * light.dir.y + dz * light.dir.z;
                            let dist_sq = dx * dx + dy * dy + dz * dz;
                            let line_sq = (dist_sq - dot * dot).max(0.0);
                            if dot > 0.0
                                && dot < FOG_AREA_LENGTH
                                && line_sq < FOG_AREA_WIDTH * FOG_AREA_WIDTH
                                && dist_sq > f32::EPSILON
                            {
                                let mut intensity = 158.0 * fogginess;
                                intensity *= dot / dist_sq.sqrt();
                                intensity *= 1.0 - (dot / FOG_AREA_LENGTH).powi(2);
                                intensity *= 1.0 - (line_sq.sqrt() / FOG_AREA_WIDTH).powi(2);
                                if intensity > 0.0 {
                                    out.push(PointLightFogSpriteIntent {
                                        position: fog_pos,
                                        size: FOG_SIZES[(r >> 1) as usize],
                                        rotation,
                                        light_colour: [light.red, light.green, light.blue],
                                        intensity,
                                    });
                                }
                            }
                        }
                    }
                }
                yi += 4;
            }
            xi += 4;
        }
    }

    fn point_fog_sprites(
        &self,
        light: &CRegisteredPointLight,
        camera_position: CVector,
        fogginess: f32,
        time_ms: u32,
        ground_z: &mut impl FnMut(CVector, f32) -> Option<f32>,
        out: &mut Vec<PointLightFogSpriteIntent>,
    ) {
        let Some(z) = ground_z(light.coors, light.coors.z - 20.0) else {
            return;
        };
        let xmin = light.coors.x - FOG_AREA_RADIUS;
        let xmax = light.coors.x + FOG_AREA_RADIUS;
        let ymin = light.coors.y - FOG_AREA_RADIUS;
        let ymax = light.coors.y + FOG_AREA_RADIUS;
        let rotation = (time_ms & 0x3fff) as f32 * 2.0 * 3.14 / 0x4000 as f32;

        let mut xi = aligned_grid_start(xmin, 2);
        let x_end = xmax as i16 as i32 + 2;
        while xi <= x_end {
            let mut yi = aligned_grid_start(ymin, 2);
            let y_end = ymax as i16 as i32 + 2;
            while yi <= y_end {
                let r = ((xi ^ yi) >> 1) & 0x0f;
                if r & 1 != 0 {
                    let light_dx = xi as f32 - light.coors.x;
                    let light_dy = yi as f32 - light.coors.y;
                    let light_dist = (light_dx * light_dx + light_dy * light_dy).sqrt();
                    if light_dist < FOG_AREA_RADIUS {
                        let cam_dx = xi as f32 - camera_position.x;
                        let cam_dy = yi as f32 - camera_position.y;
                        let cam_dist = (cam_dx * cam_dx + cam_dy * cam_dy).sqrt();
                        if cam_dist < MAX_DIST {
                            let camera_fade = if cam_dist < MAX_DIST * 0.5 {
                                1.0
                            } else {
                                1.0 - (cam_dist - MAX_DIST * 0.5) / (MAX_DIST * 0.5)
                            };
                            let intensity = camera_fade
                                * 132.0
                                * fogginess
                                * (1.0 - (light_dist / FOG_AREA_RADIUS).powi(2));
                            if intensity > 0.0 {
                                out.push(PointLightFogSpriteIntent {
                                    position: CVector::new(xi as f32, yi as f32, z + 1.6),
                                    size: FOG_SIZES[(r >> 1) as usize],
                                    rotation,
                                    light_colour: [light.red, light.green, light.blue],
                                    intensity,
                                });
                            }
                        }
                    }
                }
                yi += 2;
            }
            xi += 2;
        }
    }
}

fn aligned_grid_start(value: f32, step: i32) -> i32 {
    let value = value as i16 as i32;
    value - value % step
}

fn sub(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(a: CVector, scalar: f32) -> CVector {
    CVector::new(a.x * scalar, a.y * scalar, a.z * scalar)
}

fn dot(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
