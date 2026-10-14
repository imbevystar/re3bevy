//! re3 reference: references/re-GTA/src/renderer/Skidmarks.cpp
//! re3 reference: references/re-GTA/src/renderer/Skidmarks.h

use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, Timer::CTimer};

use super::RenderBuffer::RwIm3DVertexIntent;

pub const NUMSKIDMARKS: usize = 32;
pub const SKIDMARK_LENGTH: usize = 16;
pub const SKIDMARK_TEXTURE_NAMES: [&str; 3] =
    ["particleskid", "particleskidblood", "particleskidmud"];

pub(crate) fn update_skidmarks(timer: Res<CTimer>, skidmarks: Option<ResMut<CSkidmarks>>) {
    if let Some(mut skidmarks) = skidmarks {
        skidmarks.update(timer.time_in_milliseconds());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CSkidmarkState {
    /// 0: free, 1: being extended, 2: fading.
    pub state: u8,
    pub was_updated: bool,
    pub is_bloody: bool,
    pub is_muddy: bool,
    pub id: usize,
    pub last: i16,
    pub last_update: u32,
    pub fade_start: u32,
    pub fade_end: u32,
    pub positions: [CVector; SKIDMARK_LENGTH],
    pub sides: [CVector; SKIDMARK_LENGTH],
}

impl Default for CSkidmarkState {
    fn default() -> Self {
        Self {
            state: 0,
            was_updated: false,
            is_bloody: false,
            is_muddy: false,
            id: 0,
            last: 0,
            last_update: 0,
            fade_start: 0,
            fade_end: 0,
            positions: [CVector::ZERO; SKIDMARK_LENGTH],
            sides: [CVector::ZERO; SKIDMARK_LENGTH],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkidmarkRenderBatch {
    pub texture_name: &'static str,
    pub vertices: Vec<RwIm3DVertexIntent>,
    pub indices: Vec<u16>,
    pub z_write: bool,
    pub vertex_alpha: bool,
}

#[derive(Debug, Clone, Resource)]
pub struct CSkidmarks {
    pub skidmarks: [CSkidmarkState; NUMSKIDMARKS],
    pub index_list: [u16; SKIDMARK_LENGTH * 6],
    pub texture_names: [&'static str; 3],
    pub render_batches: Vec<SkidmarkRenderBatch>,
}

impl CSkidmarks {
    pub fn init() -> Self {
        let mut index_list = [0; SKIDMARK_LENGTH * 6];
        let mut ix = 0;
        for i in 0..SKIDMARK_LENGTH {
            index_list[i * 6] = ix;
            index_list[i * 6 + 1] = ix + 2;
            index_list[i * 6 + 2] = ix + 1;
            index_list[i * 6 + 3] = ix + 1;
            index_list[i * 6 + 4] = ix + 2;
            index_list[i * 6 + 5] = ix + 3;
            ix += 2;
        }

        Self {
            skidmarks: [CSkidmarkState::default(); NUMSKIDMARKS],
            index_list,
            texture_names: SKIDMARK_TEXTURE_NAMES,
            render_batches: Vec::new(),
        }
    }

    pub fn shutdown(&mut self) {
        // TXD-store images own the Bevy handles; release only render references.
        self.render_batches.clear();
    }

    pub fn clear(&mut self) {
        for skidmark in &mut self.skidmarks {
            skidmark.state = 0;
            skidmark.was_updated = false;
        }
    }

    pub fn update(&mut self, time_ms: u32) {
        let t1 = time_ms.wrapping_add(2_500);
        let t2 = time_ms.wrapping_add(5_000);
        let t3 = time_ms.wrapping_add(10_000);
        let t4 = time_ms.wrapping_add(20_000);
        for skidmark in &mut self.skidmarks {
            match skidmark.state {
                1 if !skidmark.was_updated => {
                    skidmark.state = 2;
                    if skidmark.last < 4 {
                        skidmark.fade_start = t1;
                        skidmark.fade_end = t2;
                    } else if skidmark.last < 9 {
                        skidmark.fade_start = t2;
                        skidmark.fade_end = t3;
                    } else {
                        skidmark.fade_start = t3;
                        skidmark.fade_end = t4;
                    }
                }
                2 if time_ms > skidmark.fade_end => skidmark.state = 0,
                _ => {}
            }
            skidmark.was_updated = false;
        }
    }

    /// `is_bloody` is intentionally in/out: re3 clears the caller's wheel
    /// blood flag after eight points, when the strip fills, or if no slot exists.
    pub fn register_one(
        &mut self,
        id: usize,
        position: CVector,
        fwd_x: f32,
        fwd_y: f32,
        is_muddy: &mut bool,
        is_bloody: &mut bool,
        time_ms: u32,
        replay_playback: bool,
    ) {
        if replay_playback {
            return;
        }

        if let Some(index) = self
            .skidmarks
            .iter()
            .position(|mark| mark.state == 1 && mark.id == id)
        {
            let skidmark = &mut self.skidmarks[index];
            if skidmark.is_bloody != *is_bloody {
                skidmark.state = 2;
                skidmark.fade_start = time_ms.wrapping_add(10_000);
                skidmark.fade_end = time_ms.wrapping_add(20_000);
                return;
            }

            skidmark.was_updated = true;
            if time_ms.wrapping_sub(skidmark.last_update) <= 100 {
                skidmark.positions[skidmark.last as usize] = position;
                return;
            }
            skidmark.last_update = time_ms;
            if skidmark.last as usize >= SKIDMARK_LENGTH - 1 {
                skidmark.state = 2;
                skidmark.fade_start = time_ms.wrapping_add(10_000);
                skidmark.fade_end = time_ms.wrapping_add(20_000);
                *is_bloody = false;
                return;
            }

            skidmark.last += 1;
            let last = skidmark.last as usize;
            skidmark.positions[last] = position;
            let previous = skidmark.positions[last - 1];
            let mut right = [position.y - previous.y, previous.x - position.x];
            normalise_safe_2d(&mut right);
            let mut forward = [fwd_x, fwd_y];
            normalise_safe_2d(&mut forward);
            let turn = (forward[0] * right[0] + forward[1] * right[1]).abs() + 1.0;
            skidmark.sides[last] =
                CVector::new(right[0] * turn * 0.125, right[1] * turn * 0.125, 0.0);
            if last == 1 {
                skidmark.sides[0] = skidmark.sides[1];
            }
            if last > 8 {
                *is_bloody = false;
            }
            return;
        }

        if let Some(skidmark) = self.skidmarks.iter_mut().find(|mark| mark.state == 0) {
            skidmark.state = 1;
            skidmark.id = id;
            skidmark.positions[0] = position;
            skidmark.sides[0] = CVector::ZERO;
            skidmark.was_updated = true;
            skidmark.last = 0;
            skidmark.last_update = time_ms.wrapping_sub(1_000);
            skidmark.is_bloody = *is_bloody;
            skidmark.is_muddy = *is_muddy;
        } else {
            *is_bloody = false;
        }
    }

    pub fn render(&mut self, time_ms: u32) {
        self.render_batches.clear();
        for skidmark in &self.skidmarks {
            if skidmark.state == 0 || skidmark.last < 1 {
                continue;
            }
            let texture_name = if skidmark.is_bloody {
                self.texture_names[1]
            } else if skidmark.is_muddy {
                self.texture_names[2]
            } else {
                self.texture_names[0]
            };
            let fade = if skidmark.state == 1 || time_ms < skidmark.fade_start {
                255
            } else {
                255_u32.wrapping_mul(skidmark.fade_end.wrapping_sub(time_ms))
                    / skidmark.fade_end.wrapping_sub(skidmark.fade_start)
            };
            let last = skidmark.last as usize;
            let mut vertices = Vec::with_capacity((last + 1) * 2);
            for j in 0..=last {
                let endpoint = j == 0 || (j == last && skidmark.state == 2);
                let alpha = (if endpoint { 0 } else { 128 } * fade / 256) as u8;
                let position = skidmark.positions[j];
                let side = skidmark.sides[j];
                let colour = [255, 255, 255, alpha];
                vertices.push(RwIm3DVertexIntent {
                    position: CVector::new(
                        position.x + side.x,
                        position.y + side.y,
                        position.z + 0.1,
                    ),
                    colour,
                    uv: [0.0, j as f32 * 5.01],
                });
                vertices.push(RwIm3DVertexIntent {
                    position: CVector::new(
                        position.x - side.x,
                        position.y - side.y,
                        position.z + 0.1,
                    ),
                    colour,
                    uv: [1.0, j as f32 * 5.01],
                });
            }
            self.render_batches.push(SkidmarkRenderBatch {
                texture_name,
                vertices,
                indices: self.index_list[..6 * last].to_vec(),
                z_write: false,
                vertex_alpha: true,
            });
        }
    }
}

impl Default for CSkidmarks {
    fn default() -> Self {
        Self::init()
    }
}

fn normalise_safe_2d(value: &mut [f32; 2]) {
    let magnitude = (value[0] * value[0] + value[1] * value[1]).sqrt();
    if magnitude > 0.0 {
        value[0] /= magnitude;
        value[1] /= magnitude;
    }
}
