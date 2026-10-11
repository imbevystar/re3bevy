//! re3 reference: references/re-GTA/src/renderer/SpecialFX.cpp
//! re3 reference: references/re-GTA/src/renderer/SpecialFX.h
//!
//! Bullet traces, motion blur streaks, 3D markers — render-ready state.

use bevy::prelude::*;
use re3bevy_core::core::{Coordinates::CVector, General::CGeneral};

use crate::{
    math::Matrix::CMatrix,
    renderer::ParticleType::{PARTICLE_STEAM_NY, PARTICLE_STEAM2, tParticleType},
};

pub const NUMBULLETTRACES: usize = 16;
pub const NUMMBLURSTREAKS: usize = 4;
pub const NUM3DMARKERS: usize = 32;
pub const NUMBRIGHTLIGHTS: usize = 32;
pub const NUMSHINYTEXTS: usize = 32;
pub const MARKERTYPE_INVALID: u16 = 0x101;
pub const MARKERTYPE_ARROW: u16 = 0;
pub const MARKERTYPE_CYLINDER: u16 = 1;
pub const MARKERTYPE_TORUS: u16 = 2;
pub const MARKERTYPE_0: u16 = 3;
pub const MARKERTYPE_1: u16 = 4;
pub const BRIGHTLIGHT_TRAFFIC_GREEN: u8 = 1;
pub const BRIGHTLIGHT_TRAFFIC_YELLOW: u8 = 2;
pub const BRIGHTLIGHT_TRAFFIC_RED: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CBulletTraceState {
    pub in_use: bool,
    pub start: CVector,
    pub end: CVector,
    pub time_ms: u32,
    pub life_ms: u32,
    pub thickness: f32,
    pub colour: [u8; 4],
    pub native_motion: bool,
    pub frames_in_use: u8,
    pub lifetime_frames: u8,
}

impl Default for CBulletTraceState {
    fn default() -> Self {
        Self {
            in_use: false,
            start: CVector::ZERO,
            end: CVector::ZERO,
            time_ms: 0,
            life_ms: 0,
            thickness: 0.03,
            colour: [255, 255, 200, 180],
            native_motion: false,
            frames_in_use: 0,
            lifetime_frames: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CMotionBlurStreakState {
    pub id: usize,
    pub in_use: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct C3dMarkerState {
    pub atomic_present: bool,
    pub marker_type: u16,
    pub is_used: bool,
    pub identifier: u32,
    pub color: [u8; 4],
    pub pulse_period: u16,
    pub rotate_rate: i16,
    pub start_time: u32,
    pub pulse_fraction: f32,
    pub position: CVector,
    pub size: f32,
    pub standard_size: f32,
    pub z_shift: f32,
    pub rotation_degrees: f32,
    pub player_range: f32,
}

impl Default for C3dMarkerState {
    fn default() -> Self {
        Self {
            atomic_present: false,
            marker_type: MARKERTYPE_INVALID,
            is_used: false,
            identifier: 0,
            color: [255, 255, 255, 255],
            pulse_period: 1024,
            rotate_rate: 5,
            start_time: 0,
            pulse_fraction: 0.25,
            position: CVector::ZERO,
            size: 1.0,
            standard_size: 1.0,
            z_shift: 0.0,
            rotation_degrees: 0.0,
            player_range: 0.0,
        }
    }
}

/// One-frame draw intent for bullet tracer line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletTraceRenderIntent {
    pub start: CVector,
    pub end: CVector,
    pub colour: [u8; 4],
    pub thickness: f32,
}

/// One-frame draw intent for mission 3d marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker3dRenderIntent {
    pub position: CVector,
    pub size: f32,
    pub colour: [u8; 4],
    pub marker_type: u16,
    pub rotation_degrees: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrightLightRenderIntent {
    pub position: CVector,
    pub up: CVector,
    pub side: CVector,
    pub light_type: u8,
    pub camera_distance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShinyTextRenderIntent {
    pub vertices: [CVector; 4],
    pub tex_coords: [[f32; 2]; 4],
    pub colour: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoatFoamParticleIntent {
    pub particle_type: tParticleType,
    pub position: CVector,
    pub size: f32,
}

#[derive(Debug, Clone, Resource)]
pub struct CSpecialFX {
    pub bullet_traces: [CBulletTraceState; NUMBULLETTRACES],
    pub motion_blur_streaks: [CMotionBlurStreakState; NUMMBLURSTREAKS],
    pub markers: [C3dMarkerState; NUM3DMARKERS],
    pub streak_index_list: [u16; 12],
    pub trace_index_list: [u16; 12],
    pub bullet_render_intents: Vec<BulletTraceRenderIntent>,
    pub marker_render_intents: Vec<Marker3dRenderIntent>,
    pub bright_light_render_intents: Vec<BrightLightRenderIntent>,
    pub shiny_text_render_intents: Vec<ShinyTextRenderIntent>,
    pub boat_foam_from_start_ms: u32,
    pub boat_foam_frame: i32,
    pub boat_foam_position: CVector,
    pub boat_foam_delta: CVector,
}

impl CSpecialFX {
    pub fn init() -> Self {
        Self {
            bullet_traces: [CBulletTraceState::default(); NUMBULLETTRACES],
            motion_blur_streaks: [CMotionBlurStreakState::default(); NUMMBLURSTREAKS],
            markers: [C3dMarkerState::default(); NUM3DMARKERS],
            streak_index_list: [0, 1, 2, 1, 3, 2, 0, 2, 1, 1, 2, 3],
            trace_index_list: [0, 2, 1, 1, 2, 3, 2, 4, 3, 3, 4, 5],
            bullet_render_intents: Vec::new(),
            marker_render_intents: Vec::new(),
            bright_light_render_intents: Vec::with_capacity(NUMBRIGHTLIGHTS),
            shiny_text_render_intents: Vec::with_capacity(NUMSHINYTEXTS),
            boat_foam_from_start_ms: 0,
            boat_foam_frame: 0,
            boat_foam_position: CVector::ZERO,
            boat_foam_delta: CVector::ZERO,
        }
    }

    /// re3 `CSpecialParticleStuff::StartBoatFoamAnimation`.
    pub fn start_boat_foam_animation(&mut self, time_ms: u32) {
        self.boat_foam_from_start_ms = time_ms;
    }

    /// re3 `CSpecialParticleStuff::UpdateBoatFoamAnimation`, returning the
    /// exact particle calls for the renderer-owned CParticle pool.
    pub fn update_boat_foam_animation(
        &mut self,
        matrix: &CMatrix,
        camera_forward: CVector,
        previous_time_ms: u32,
        time_ms: u32,
        general: &mut CGeneral,
    ) -> Vec<BoatFoamParticleIntent> {
        let mut particles =
            self.create_foam_around_object(matrix, camera_forward, 107.0, 24.1, 30.5, 2, general);

        if self.boat_foam_frame != 0 {
            self.boat_foam_position.x += self.boat_foam_delta.x;
            self.boat_foam_position.y += self.boat_foam_delta.y;
            self.boat_foam_position.z += self.boat_foam_delta.z;
            particles.push(BoatFoamParticleIntent {
                particle_type: PARTICLE_STEAM_NY,
                position: matrix.transform_point(self.boat_foam_position),
                size: self.boat_foam_frame as f32 * 0.5 + 2.0,
            });
            self.boat_foam_frame += 1;
            if self.boat_foam_frame > 15 {
                self.boat_foam_frame = 0;
            }
        }

        if (time_ms & 0x3ff) < (previous_time_ms & 0x3ff) {
            self.boat_foam_frame = 1;
            let rnd = u32::from(general.get_random_number());
            self.boat_foam_position = CVector::new(
                signed_random_byte(rnd.wrapping_sub(128)) * 0.2,
                signed_random_byte((rnd >> 8).wrapping_sub(128)) * 0.2,
                10.0,
            );
            let rnd = u32::from(general.get_random_number());
            self.boat_foam_delta = CVector::new(
                signed_random_byte(rnd.wrapping_sub(128)) * 0.02,
                signed_random_byte((rnd >> 8).wrapping_sub(128)) * 0.02,
                2.0,
            );
        }

        particles
    }

    #[allow(clippy::too_many_arguments)]
    fn create_foam_around_object(
        &self,
        matrix: &CMatrix,
        camera_forward: CVector,
        inner_fw: f32,
        inner_rg: f32,
        inner_up: f32,
        mut particle_count: i32,
        general: &mut CGeneral,
    ) -> Vec<BoatFoamParticleIntent> {
        let outer_fw = inner_fw + 5.0;
        let outer_rg = inner_rg + 5.0;
        let outer_up = inner_up + 5.0;
        let matrix_position = matrix.position;
        let mut output = Vec::with_capacity(particle_count.max(0) as usize);
        for _ in 0..1000 {
            if particle_count <= 0 {
                break;
            }
            let rnd = u32::from(general.get_random_number());
            let mut position = CVector::new(
                signed_random_byte(rnd.wrapping_sub(128)) * inner_fw / 110.0,
                signed_random_byte((rnd >> 8).wrapping_sub(128)) * inner_fw / 110.0,
                0.0,
            );
            if position.x * camera_forward.x + position.y * camera_forward.y >= 0.0 {
                continue;
            }
            position.x += matrix_position.x;
            position.y += matrix_position.y;
            position.z += matrix_position.z;
            // This apparently odd world-Z assignment is present in GTA III.
            position.z = 2.0;
            let relative = CVector::new(
                position.x - matrix_position.x,
                position.y - matrix_position.y,
                position.z - matrix_position.z,
            );
            let forward = matrix.forward;
            let right = matrix.right;
            let up = matrix.up;
            let fw =
                (forward.x * relative.x + forward.y * relative.y + forward.z * relative.z).abs();
            if fw >= outer_fw {
                continue;
            }
            let rg = (right.x * relative.x + right.y * relative.y + right.z * relative.z).abs();
            if rg >= outer_rg {
                continue;
            }
            let up_distance = (up.x * relative.x + up.y * relative.y + up.z * relative.z).abs();
            if up_distance >= outer_up {
                continue;
            }
            if fw > inner_fw || rg > inner_rg || up_distance > inner_up {
                output.push(BoatFoamParticleIntent {
                    particle_type: PARTICLE_STEAM2,
                    position,
                    size: 4.0,
                });
                particle_count -= 1;
            }
        }
        output
    }

    /// re3 `CBulletTraces::AddTrace`.
    pub fn add_bullet_trace(
        &mut self,
        start: CVector,
        end: CVector,
        time_ms: u32,
        life_ms: u32,
        thickness: f32,
        colour: [u8; 4],
    ) {
        if let Some(slot) = self.bullet_traces.iter_mut().find(|t| !t.in_use) {
            *slot = CBulletTraceState {
                in_use: true,
                start,
                end,
                time_ms,
                life_ms: life_ms.max(1),
                thickness: thickness.max(0.01),
                colour,
                native_motion: false,
                frames_in_use: 0,
                lifetime_frames: 0,
            };
        }
    }

    /// re3 `CBulletTraces::AddTrace`. Native traces do not fade a static full
    /// line: after one held frame the current end advances 0.8 world units per
    /// update toward the target and expires after a random 25..56 frames.
    pub fn add_native_bullet_trace(
        &mut self,
        start: CVector,
        end: CVector,
        general: &mut CGeneral,
    ) {
        let Some(slot) = self.bullet_traces.iter_mut().find(|trace| !trace.in_use) else {
            return;
        };
        let lifetime_frames = 25 + (general.get_random_number() % 32) as u8;
        *slot = CBulletTraceState {
            in_use: true,
            start,
            end,
            time_ms: 0,
            life_ms: 0,
            thickness: 0.05,
            colour: [lifetime_frames, lifetime_frames, lifetime_frames, 255],
            native_motion: true,
            frames_in_use: 0,
            lifetime_frames,
        };
    }

    /// re3 `C3dMarkers::PlaceMarker`. A refreshed marker occupies one slot
    /// for this frame; equal identifiers in PlaceMarkerSet occupy three slots.
    pub fn place_marker(
        &mut self,
        identifier: u32,
        marker_type: u16,
        position: CVector,
        size: f32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        pulse_period: u16,
        pulse_fraction: f32,
        rotate_rate: i16,
        time_ms: u32,
        player_position: CVector,
        time_step: f32,
    ) {
        if !matches!(marker_type, MARKERTYPE_ARROW | MARKERTYPE_CYLINDER) {
            return;
        }
        let range = (position.x - player_position.x).hypot(position.y - player_position.y);
        let slot = self.markers.iter().position(|m| !m.is_used && m.identifier == identifier)
            .or_else(|| self.markers.iter().position(|m| m.marker_type == MARKERTYPE_INVALID))
            .or_else(|| (marker_type == MARKERTYPE_ARROW).then(|| self.markers.iter()
                .position(|m| m.marker_type == MARKERTYPE_ARROW && m.player_range > range)).flatten());
        let Some(slot) = slot else {
            warn_once!(target: "re3bevy::markers", "C3dMarkers marker pool exhausted");
            return;
        };
        let marker = &mut self.markers[slot];
        let reused = marker.identifier == identifier && marker.marker_type == marker_type;
        if !reused {
            *marker = C3dMarkerState {
                atomic_present: true,
                marker_type,
                is_used: true,
                identifier,
                color: [r, g, b, a],
                pulse_period: pulse_period.max(1),
                rotate_rate,
                start_time: time_ms,
                pulse_fraction,
                position,
                size,
                standard_size: size,
                z_shift: 0.0,
                rotation_degrees: 0.0,
                player_range: range,
            };
        }
        marker.player_range = range;
        if marker_type == MARKERTYPE_ARROW {
            marker.standard_size = if range < 25.0 {
                if range > 5.0 { size - (25.0 - range) * (0.3 * size) / 20.0 }
                else { size * 0.7 }
            } else { size };
        } else {
            marker.color[3] = if range < size + 12.0 {
                if range > size + 1.0 { ((1.0 - (size + 12.0 - range) * 0.7 / 11.0) * a as f32) as u8 }
                else { (a as f32 * 0.3) as u8 }
            } else { a };
        }
        if reused {
            let phase = std::f32::consts::TAU
                * ((marker.pulse_period - 1) as u32 & time_ms.wrapping_sub(marker.start_time)) as f32
                / marker.pulse_period as f32;
            let sine = phase.sin();
            marker.size = marker.standard_size * (1.0 - pulse_fraction * sine);
            marker.rotation_degrees += marker.rotate_rate as f32 * time_step;
            if marker_type == MARKERTYPE_ARROW {
                marker.position = position;
                marker.position.z += 0.25 * marker.standard_size * sine;
            }
        }
        marker.is_used = true;
    }

    /// re3 `CBrightLights::RegisterOne` for the traffic-light variants.
    pub fn register_traffic_bright_light(
        &mut self,
        position: CVector,
        up: CVector,
        side: CVector,
        light_type: u8,
        camera_position: CVector,
    ) {
        if self.bright_light_render_intents.len() >= NUMBRIGHTLIGHTS {
            return;
        }
        let camera_distance = CVector::new(
            position.x - camera_position.x,
            position.y - camera_position.y,
            position.z - camera_position.z,
        )
        .magnitude();
        if camera_distance > 60.0 {
            return;
        }
        self.bright_light_render_intents
            .push(BrightLightRenderIntent {
                position,
                up,
                side,
                light_type,
                camera_distance,
            });
    }

    /// re3 `CShinyTexts::RegisterOne`. Traffic lights use the shared
    /// `walk_dont` particle texture, so the renderer only needs this quad
    /// geometry, UV window and pre-faded colour.
    pub fn register_walk_dont_text(
        &mut self,
        vertices: [CVector; 4],
        tex_coords: [[f32; 2]; 4],
        mut colour: [u8; 3],
        max_distance: f32,
        camera_position: CVector,
    ) {
        if self.shiny_text_render_intents.len() >= NUMSHINYTEXTS || max_distance <= 0.0 {
            return;
        }
        let p0 = vertices[0];
        let camera_distance = CVector::new(
            p0.x - camera_position.x,
            p0.y - camera_position.y,
            p0.z - camera_position.z,
        )
        .magnitude();
        if camera_distance > max_distance {
            return;
        }
        let half_distance = max_distance * 0.5;
        if camera_distance > half_distance {
            let fade = (1.0 - (camera_distance - half_distance) / half_distance).clamp(0.0, 1.0);
            for channel in &mut colour {
                *channel = ((*channel as f32) * fade).clamp(0.0, 255.0) as u8;
            }
        }
        self.shiny_text_render_intents.push(ShinyTextRenderIntent {
            vertices,
            tex_coords,
            colour,
        });
    }

    pub fn update(&mut self, time_ms: u32) {
        for trace in &mut self.bullet_traces {
            if !trace.in_use {
                continue;
            }
            if trace.native_motion {
                if trace.frames_in_use == 0 {
                    trace.frames_in_use = 1;
                    continue;
                }
                if trace.frames_in_use > 60 || trace.lifetime_frames == 0 {
                    *trace = CBulletTraceState::default();
                    continue;
                }
                let diff = CVector::new(
                    trace.start.x - trace.end.x,
                    trace.start.y - trace.end.y,
                    trace.start.z - trace.end.z,
                );
                let remaining = diff.magnitude();
                if remaining > 0.8 {
                    let scale = (remaining - 0.8) / remaining;
                    trace.start = CVector::new(
                        trace.end.x + diff.x * scale,
                        trace.end.y + diff.y * scale,
                        trace.end.z + diff.z * scale,
                    );
                } else {
                    *trace = CBulletTraceState::default();
                    continue;
                }
                trace.lifetime_frames -= 1;
                if trace.lifetime_frames == 0 {
                    *trace = CBulletTraceState::default();
                    continue;
                }
                trace.colour = [
                    trace.lifetime_frames,
                    trace.lifetime_frames,
                    trace.lifetime_frames,
                    255,
                ];
                trace.frames_in_use = trace.frames_in_use.saturating_add(1);
                continue;
            }
            let age = time_ms.wrapping_sub(trace.time_ms);
            if age >= trace.life_ms {
                *trace = CBulletTraceState::default();
            }
        }
    }

    pub fn build_render_intents(&mut self, time_ms: u32) {
        self.bullet_render_intents.clear();
        self.marker_render_intents.clear();

        for trace in &self.bullet_traces {
            if !trace.in_use {
                continue;
            }
            let mut colour = trace.colour;
            if !trace.native_motion {
                let age = time_ms.wrapping_sub(trace.time_ms) as f32;
                let life = trace.life_ms.max(1) as f32;
                let fade = (1.0 - age / life).clamp(0.0, 1.0);
                colour[3] = ((colour[3] as f32) * fade) as u8;
            }
            if colour[3] == 0 {
                continue;
            }
            self.bullet_render_intents.push(BulletTraceRenderIntent {
                start: trace.start,
                end: trace.end,
                colour,
                thickness: trace.thickness,
            });
        }

        for marker in &mut self.markers {
            if !marker.is_used {
                if marker.atomic_present { *marker = C3dMarkerState::default(); }
                continue;
            }
            marker.is_used = false;
            if marker.player_range >= 120.0 { continue; }
            self.marker_render_intents.push(Marker3dRenderIntent {
                position: CVector::new(
                    marker.position.x,
                    marker.position.y,
                    marker.position.z + marker.z_shift,
                ),
                size: marker.size,
                colour: marker.color,
                marker_type: marker.marker_type,
                rotation_degrees: marker.rotation_degrees,
            });
        }
    }

    pub fn drain_bullet_intents(&mut self) -> Vec<BulletTraceRenderIntent> {
        std::mem::take(&mut self.bullet_render_intents)
    }

    pub fn drain_marker_intents(&mut self) -> Vec<Marker3dRenderIntent> {
        std::mem::take(&mut self.marker_render_intents)
    }

    pub fn drain_bright_light_intents(&mut self) -> Vec<BrightLightRenderIntent> {
        std::mem::take(&mut self.bright_light_render_intents)
    }

    pub fn drain_shiny_text_intents(&mut self) -> Vec<ShinyTextRenderIntent> {
        std::mem::take(&mut self.shiny_text_render_intents)
    }
}

fn signed_random_byte(value: u32) -> f32 {
    (value as u8 as i8) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_set_retains_three_slots_and_retires_unrefreshed_atomics() {
        let mut fx = CSpecialFX::init();
        let position = CVector::new(10.0, 10.0, 3.0);
        for (scale, rate) in [(1.0, 1), (0.93, 2), (0.86, -1)] {
            fx.place_marker(42, MARKERTYPE_CYLINDER, position, 2.0 * scale,
                0, 128, 255, 128, 2048, 0.1, rate, 100, position, 1.0);
        }
        fx.build_render_intents(100);
        assert_eq!(fx.drain_marker_intents().len(), 3);
        for (scale, rate) in [(1.0, 1), (0.93, 2), (0.86, -1)] {
            fx.place_marker(42, MARKERTYPE_CYLINDER, position, 2.0 * scale,
                0, 128, 255, 128, 2048, 0.1, rate, 612, position, 0.5);
        }
        fx.build_render_intents(612);
        let intents = fx.drain_marker_intents();
        assert_eq!(intents.len(), 3);
        assert!((intents[0].size - 1.8).abs() < 1e-5);
        assert_eq!(intents[0].colour[3], 38);
        assert_eq!(intents[0].rotation_degrees, 0.5);
        assert_eq!(intents[1].rotation_degrees, 1.0);
        assert_eq!(intents[2].rotation_degrees, -0.5);
        fx.build_render_intents(630);
        assert!(fx.markers.iter().all(|m| !m.atomic_present));
        assert!(fx.drain_marker_intents().is_empty());
    }

    #[test]
    fn native_bullet_trace_holds_first_frame_then_advances_point_eight_units() {
        let mut fx = CSpecialFX::init();
        let mut general = CGeneral::with_seed(1);
        fx.add_native_bullet_trace(
            CVector::new(0.0, 0.0, 0.0),
            CVector::new(0.0, 10.0, 0.0),
            &mut general,
        );

        fx.update(0);
        let trace = fx.bullet_traces.iter().find(|trace| trace.in_use).unwrap();
        assert_eq!(trace.start, CVector::new(0.0, 0.0, 0.0));
        assert_eq!(trace.frames_in_use, 1);

        fx.update(16);
        let trace = fx.bullet_traces.iter().find(|trace| trace.in_use).unwrap();
        assert!((trace.start.y - 0.8).abs() < 1.0e-5);
        assert_eq!(trace.frames_in_use, 2);
    }

    #[test]
    fn boat_foam_wrap_starts_fifteen_frame_steam_animation() {
        let mut fx = CSpecialFX::init();
        let matrix = CMatrix::unity();
        let camera_forward = CVector::new(0.0, 1.0, 0.0);
        let mut general = CGeneral::with_seed(1);

        let first =
            fx.update_boat_foam_animation(&matrix, camera_forward, 1023, 1024, &mut general);
        assert_eq!(fx.boat_foam_frame, 1);
        assert!(
            first
                .iter()
                .all(|particle| particle.particle_type == PARTICLE_STEAM2)
        );

        let second =
            fx.update_boat_foam_animation(&matrix, camera_forward, 1024, 1025, &mut general);
        let animated = second
            .iter()
            .find(|particle| particle.particle_type == PARTICLE_STEAM_NY)
            .unwrap();
        assert_eq!(animated.size, 2.5);
        assert_eq!(fx.boat_foam_frame, 2);
    }

    #[test]
    fn traffic_bright_lights_use_re3_capacity_and_distance_cutoff() {
        let mut fx = CSpecialFX::init();
        let camera = CVector::ZERO;
        fx.register_traffic_bright_light(
            CVector::new(60.1, 0.0, 0.0),
            CVector::new(0.0, 0.0, 1.0),
            CVector::new(1.0, 0.0, 0.0),
            BRIGHTLIGHT_TRAFFIC_RED,
            camera,
        );
        assert!(fx.bright_light_render_intents.is_empty());

        for _ in 0..(NUMBRIGHTLIGHTS + 4) {
            fx.register_traffic_bright_light(
                CVector::new(10.0, 0.0, 0.0),
                CVector::new(0.0, 0.0, 1.0),
                CVector::new(1.0, 0.0, 0.0),
                BRIGHTLIGHT_TRAFFIC_GREEN,
                camera,
            );
        }
        assert_eq!(fx.bright_light_render_intents.len(), NUMBRIGHTLIGHTS);
    }

    #[test]
    fn walk_dont_text_fades_after_half_of_max_distance() {
        let mut fx = CSpecialFX::init();
        let vertices = [
            CVector::new(45.0, 0.0, 0.0),
            CVector::new(45.0, 1.0, 0.0),
            CVector::new(45.0, 0.0, 1.0),
            CVector::new(45.0, 1.0, 1.0),
        ];
        fx.register_walk_dont_text(
            vertices,
            [[0.0, 0.0]; 4],
            [255, 128, 64],
            60.0,
            CVector::ZERO,
        );
        assert_eq!(fx.shiny_text_render_intents.len(), 1);
        assert_eq!(fx.shiny_text_render_intents[0].colour, [127, 64, 32]);
    }
}
