//! re3 reference: references/re-GTA/src/renderer/Glass.cpp
//! re3 reference: references/re-GTA/src/renderer/Glass.h
//!
//! Falling window panes and glass response state. World collision, RNG, audio
//! and particle creation remain explicit ECS boundaries.

use bevy::prelude::*;
use std::collections::BTreeMap;
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral, Timer::CTimer};

use super::RenderBuffer::RwIm3DVertexIntent;
use crate::{
    audio::{
        AudioScriptObject::cAudioScriptObject,
        DMAudio::cDMAudio,
        soundlist::{
            SCRIPT_SOUND_GLASS_BREAK_L, SCRIPT_SOUND_GLASS_BREAK_S, SCRIPT_SOUND_GLASS_CRACK,
            SCRIPT_SOUND_GLASS_LIGHT_BREAK,
        },
    },
    math::Matrix::CMatrix,
    renderer::Particle::CParticle,
};

pub const NUM_GLASSPANES: usize = 45;
pub const NUM_GLASSTRIANGLES: usize = 5;
pub const NUM_GLASSENTITIES: usize = 32;

/// CFallingGlassPane::Update's static nFrameGen, shared for the app lifetime.
#[derive(Default, Resource)]
pub struct GlassDebrisSequence { generated: u32 }

impl GlassDebrisSequence {
    pub fn generated_fragments(&self) -> u32 { self.generated }
    fn next_frame(&mut self) -> i32 {
        self.generated = self.generated.wrapping_add(1);
        (self.generated & 3) as i32
    }
}

pub struct GtaGlassPlugin;
#[derive(Component)]
struct GlassPaneSlot(usize);
impl Plugin for GtaGlassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlassDebrisSequence>().add_systems(Update,
            update_glass.in_set(rust_gta_3_core::core::Schedules::GtaProcessSet::Glass))
            .add_systems(Update, reconcile_glass_panes
                .before(rust_gta_3_core::core::Schedules::GtaProcessSet::Glass))
            .add_systems(Update, reconcile_glass_panes
                .after(rust_gta_3_core::core::Schedules::GtaProcessSet::World)
                .before(rust_gta_3_core::core::Schedules::GtaProcessSet::Camera));
    }
}

/// Unguarded lifecycle: native reset retires actors; external removal releases
/// source slots even while gameplay/loading/pause guards suppress the Glass stage.
fn reconcile_glass_panes(mut commands: Commands, glass: Option<ResMut<CGlass>>,
    panes: Query<Entity, With<GlassPaneSlot>>,
    live: Query<(), With<CFallingGlassPane>>,
    added: Query<(Entity, &GlassPaneSlot), Added<CFallingGlassPane>>,
    mut removed: RemovedComponents<CFallingGlassPane>) {
    let removed: std::collections::HashSet<_> = removed.read().collect();
    let Some(mut glass) = glass else {
        for entity in &panes { commands.entity(entity).despawn(); }
        return;
    };
    if !glass.lifecycle_ready || glass.retire_panes {
        for entity in &panes { commands.entity(entity).despawn(); }
        glass.pane_entities.clear();
        glass.retire_panes = false;
        glass.lifecycle_ready = true;
    } else {
        for &entity in &removed {
            if panes.contains(entity) && !live.contains(entity) {
                commands.entity(entity).despawn();
            }
        }
        if !removed.is_empty() {
            glass.pane_entities.retain(|_, entity| !removed.contains(entity));
        }
        for (entity, slot) in &added {
            if glass.pane_entities.get(&slot.0).is_some_and(|existing| *existing != entity) {
                warn_once!("glass_lifecycle_duplicate_native_slot slot={}", slot.0);
                commands.entity(entity).despawn();
                continue;
            }
            glass.pane_entities.insert(slot.0, entity);
        }
    }
    if !glass.pending_panes.is_empty() {
        for (slot, pane) in std::mem::take(&mut glass.pending_panes) {
            let entity = commands.spawn((pane, GlassPaneSlot(slot))).id();
            glass.pane_entities.insert(slot, entity);
        }
    }
}

/// ECS adapter for re3 `CGlass::Update`, including the one-shot audio and
/// debris side effects produced by the glass subsystem.
pub(crate) fn update_glass(
    timer: Res<CTimer>,
    glass: Option<ResMut<CGlass>>,
    mut particles: Option<ResMut<CParticle>>,
    mut dmaudio: Option<ResMut<cDMAudio>>,
    mut general: ResMut<CGeneral>,
    particle_systems: Res<super::ParticleMgr::cParticleSystemMgr>,
    camera: Res<crate::core::Camera::CCamera>,
    mut sequence: ResMut<GlassDebrisSequence>,
    mut panes: Query<&mut CFallingGlassPane>,
    mut commands: Commands,
) {
    if let Some(mut glass) = glass {
        glass.update(
            timer.time_in_milliseconds(),
            timer.time_step(),
            &mut general,
            &mut sequence,
            &mut panes,
            &mut commands,
        );
        if let Some(dmaudio) = dmaudio.as_mut() {
            for sound in glass.drain_sound_intents() {
                let sound_id = match sound.sound {
                    GlassSound::LightBreak => SCRIPT_SOUND_GLASS_LIGHT_BREAK,
                    GlassSound::LargeBreak => SCRIPT_SOUND_GLASS_BREAK_L,
                    GlassSound::SmallBreak => SCRIPT_SOUND_GLASS_BREAK_S,
                    GlassSound::Crack => SCRIPT_SOUND_GLASS_CRACK,
                };
                let mut object = cAudioScriptObject::new(sound_id as i16, sound.position);
                dmaudio.CreateOneShotScriptObject(&mut object);
            }
        }
        if let Some(particles) = particles.as_mut() {
            let mut particles = particles.emitter(&particle_systems, camera.position);
            for debris in glass.drain_debris_intents() {
                particles.add_particle_coloured(
                    crate::renderer::ParticleType::PARTICLE_CAR_DEBRIS,
                    debris.position,
                    debris.direction,
                    None,
                    debris.size,
                    super::Particle::ParticleColour { red: 255, green: 255, blue: 255, alpha: 255 },
                    debris.rotation_speed,
                    0,
                    debris.frame,
                    debris.lifespan_ms as i32,
                );
            }
        }
    }
}

pub const GLASS_TRIANGLES: [[[f32; 2]; 3]; NUM_GLASSTRIANGLES] = [
    [[0.0, 0.0], [0.0, 1.0], [0.4, 0.5]],
    [[0.0, 1.0], [1.0, 1.0], [0.4, 0.5]],
    [[0.0, 0.0], [0.4, 0.5], [0.7, 0.0]],
    [[0.7, 0.0], [0.4, 0.5], [1.0, 1.0]],
    [[0.7, 0.0], [1.0, 1.0], [1.0, 0.0]],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassSound {
    LightBreak,
    LargeBreak,
    SmallBreak,
    Crack,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassSoundIntent {
    pub sound: GlassSound,
    pub position: CVector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassDebrisIntent {
    pub position: CVector,
    pub direction: CVector,
    pub size: f32,
    pub rotation_speed: i32,
    pub frame: i32,
    pub lifespan_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct CFallingGlassPane {
    pub matrix: CMatrix,
    pub move_speed: CVector,
    pub turn: CVector,
    pub timer: u32,
    pub ground_z: f32,
    pub step: f32,
    pub triangle_index: u8,
    pub shattered: bool,
}

impl Default for CFallingGlassPane {
    fn default() -> Self {
        Self {
            matrix: CMatrix::unity(),
            move_speed: CVector::ZERO,
            turn: CVector::ZERO,
            timer: 0,
            ground_z: 0.0,
            step: 0.0,
            triangle_index: 0,
            shattered: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassBlend {
    Additive,
    Alpha,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlassRenderBatch {
    pub texture_name: &'static str,
    pub blend: GlassBlend,
    pub vertices: Vec<RwIm3DVertexIntent>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassWindowState {
    pub position: CVector,
    pub forward: CVector,
    pub random_seed: u16,
    pub cracked: bool,
    pub broken: bool,
    /// First four collision vertices transformed to world space.
    pub corners: [CVector; 4],
    /// First two collision triangles, in the same local vertex order as re3.
    pub triangles: [[u16; 3]; 2],
}

#[derive(Debug, Clone, Resource)]
pub struct CGlass {
    pane_entities: BTreeMap<usize, Entity>,
    pending_panes: BTreeMap<usize, CFallingGlassPane>,
    retire_panes: bool,
    lifecycle_ready: bool,
    pub triangle_centers: [[f32; 2]; NUM_GLASSTRIANGLES],
    /// Pool handles queued by `AskForObjectToBeRenderedInGlass`.
    pub entities_to_render: Vec<usize>,
    pub sound_intents: Vec<GlassSoundIntent>,
    pub debris_intents: Vec<GlassDebrisIntent>,
    pub render_batches: Vec<GlassRenderBatch>,
}

impl CGlass {
    pub fn init() -> Self {
        let mut triangle_centers = [[0.0; 2]; NUM_GLASSTRIANGLES];
        for (index, triangle) in GLASS_TRIANGLES.iter().enumerate() {
            triangle_centers[index] = [
                (triangle[0][0] + triangle[1][0] + triangle[2][0]) / 3.0,
                (triangle[0][1] + triangle[1][1] + triangle[2][1]) / 3.0,
            ];
        }
        Self {
            pane_entities: BTreeMap::new(),
            pending_panes: BTreeMap::new(),
            retire_panes: false,
            lifecycle_ready: false,
            triangle_centers,
            entities_to_render: Vec::with_capacity(NUM_GLASSENTITIES),
            sound_intents: Vec::new(),
            debris_intents: Vec::new(),
            render_batches: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.retire_panes = true;
        self.pane_entities.clear();
        self.pending_panes.clear();
        self.entities_to_render.clear();
        self.sound_intents.clear();
        self.debris_intents.clear();
        self.render_batches.clear();
    }

    pub fn find_free_pane(&self) -> Option<usize> {
        (0..NUM_GLASSPANES).find(|slot|
            !self.pane_entities.contains_key(slot) && !self.pending_panes.contains_key(slot))
    }

    /// Construction state is consumed by the lifecycle stage, never retained live.
    pub fn queue_pane(&mut self, pane: CFallingGlassPane) -> Option<usize> {
        if usize::from(pane.triangle_index) >= NUM_GLASSTRIANGLES {
            warn_once!("glass_creation_invalid_native_triangle");
            return None;
        }
        let slot = self.find_free_pane()?;
        self.pending_panes.insert(slot, pane);
        Some(slot)
    }

    pub fn live_pane_count(&self) -> usize { self.pane_entities.len() }
    pub fn pane_entity(&self, slot: usize) -> Option<Entity> { self.pane_entities.get(&slot).copied() }

    pub fn ask_for_object_to_be_rendered_in_glass(&mut self, pool_handle: usize) -> bool {
        // FIX_BUGS re3 path accepts all NUM_GLASSENTITIES slots.
        if self.entities_to_render.len() >= NUM_GLASSENTITIES {
            return false;
        }
        self.entities_to_render.push(pool_handle);
        true
    }

    pub fn update(&mut self, time_ms: u32, time_step: f32, general: &mut CGeneral,
        sequence: &mut GlassDebrisSequence, panes: &mut Query<&mut CFallingGlassPane>, commands: &mut Commands) {
        let mut retired = Vec::new();
        for (&slot, &entity) in &self.pane_entities {
            let Ok(mut pane) = panes.get_mut(entity) else { continue; };
            if time_ms < pane.timer {
                continue;
            }
            pane.matrix.position = add(pane.matrix.position, scale(pane.move_speed, time_step));
            pane.move_speed.z -= 0.02 * time_step;
            pane.matrix.right = add(pane.matrix.right, cross(pane.turn, pane.matrix.right));
            pane.matrix.forward = add(pane.matrix.forward, cross(pane.turn, pane.matrix.forward));
            pane.matrix.up = add(pane.matrix.up, cross(pane.turn, pane.matrix.up));

            if pane.matrix.position.z < pane.ground_z {
                commands.entity(entity).despawn();
                retired.push(slot);
                let position = CVector::new(
                    pane.matrix.position.x,
                    pane.matrix.position.y,
                    pane.ground_z,
                );
                self.sound_intents.push(GlassSoundIntent {
                    sound: GlassSound::LightBreak,
                    position,
                });
                for _ in 0..4 {
                    self.debris_intents.push(GlassDebrisIntent {
                        position,
                        direction: CVector::new(
                            general.get_random_number_in_range_f32(-0.35, 0.35),
                            general.get_random_number_in_range_f32(-0.35, 0.35),
                            general.get_random_number_in_range_f32(0.05, 0.20),
                        ),
                        size: general.get_random_number_in_range_f32(0.02, 0.2),
                        rotation_speed: general.get_random_number_in_range_i32(-40, 40),
                        frame: sequence.next_frame(),
                        lifespan_ms: 500,
                    });
                }
            }
        }
        for slot in retired { self.pane_entities.remove(&slot); }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn generate_panes_for_window(
        &mut self,
        generation_type: u32,
        position: CVector,
        up: CVector,
        right: CVector,
        speed: CVector,
        point: CVector,
        move_speed: f32,
        cracked: bool,
        explosion: bool,
        time_ms: u32,
        random: &mut impl FnMut() -> u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> Option<f32>,
    ) -> usize {
        let up_len = magnitude(up);
        let right_len = magnitude(right);
        let mut y_steps = (up_len + 0.75).max(1.0) as u32;
        let mut x_steps = (right_len + 0.75).max(1.0) as u32;
        y_steps = y_steps.min(3);
        x_steps = x_steps.min(3);
        if explosion {
            y_steps = y_steps.min(1);
            x_steps = x_steps.min(1);
        }
        let up_scale = up_len / y_steps as f32;
        let right_scale = right_len / x_steps as f32;
        let pane_ground_z =
            ground_z(position.x, position.y, position.z).unwrap_or(position.z - 2.0);
        let mut generated = 0;

        for y in 0..y_steps {
            for x in 0..x_steps {
                let step_y = y as f32 * up_len / y_steps as f32;
                let step_x = x as f32 * right_len / x_steps as f32;
                for triangle_index in 0..NUM_GLASSTRIANGLES {
                    let Some(pane_index) = self.find_free_pane() else {
                        continue;
                    };
                    let center = self.triangle_centers[triangle_index];
                    let mut pane = CFallingGlassPane::default();
                    pane.triangle_index = triangle_index as u8;
                    pane.matrix.right = scale(right, right_scale / right_len);
                    // re3 FIX_BUGS path divides the up basis by upLen.
                    pane.matrix.up = scale(up, up_scale / up_len);
                    pane.matrix.forward = normalise(cross(pane.matrix.right, pane.matrix.up));
                    pane.matrix.position = add(
                        add(
                            scale(right, (right_scale * center[0] + step_x) / right_len),
                            scale(up, (up_scale * center[1] + step_y) / up_len),
                        ),
                        position,
                    );
                    pane.move_speed = CVector::new(
                        ((random() & 127) as i32 - 64) as f32 * 0.0015 + speed.x,
                        ((random() & 127) as i32 - 64) as f32 * 0.0015 + speed.y,
                        speed.z,
                    );
                    if move_speed != 0.0 {
                        let direction = normalise(sub(pane.matrix.position, point));
                        pane.move_speed = add(pane.move_speed, scale(direction, move_speed));
                    }
                    pane.turn = CVector::new(
                        ((random() & 127) as i32 - 64) as f32 * 0.002,
                        ((random() & 127) as i32 - 64) as f32 * 0.002,
                        ((random() & 127) as i32 - 64) as f32 * 0.002,
                    );
                    pane.timer = match generation_type {
                        1 => time_ms.wrapping_add(
                            (magnitude(sub(pane.matrix.position, point)) * 100.0) as u32,
                        ),
                        _ => time_ms,
                    };
                    pane.ground_z = pane_ground_z;
                    pane.shattered = cracked;
                    pane.step = up_len / y_steps as f32;
                    self.pending_panes.insert(pane_index, pane);
                    generated += 1;
                }
            }
        }
        generated
    }

    pub fn render(&mut self, time_ms: u32, camera_position: CVector, camera_forward: CVector,
        panes: &Query<&CFallingGlassPane>) {
        let mut highlight_vertices = Vec::new();
        let mut highlight_indices = Vec::new();
        let mut shattered_vertices = Vec::new();
        let mut shattered_indices = Vec::new();

        for &entity in self.pane_entities.values() {
            let Ok(pane) = panes.get(entity) else { continue; };
            let triangle_index = pane.triangle_index as usize;
            let center = self.triangle_centers[triangle_index];
            let triangle = GLASS_TRIANGLES[triangle_index];
            let points = triangle.map(|point| {
                pane.matrix.transform_point(CVector::new(
                    point[0] - center[0],
                    0.0,
                    point[1] - center[1],
                ))
            });
            let forward = normalise(pane.matrix.forward);
            let alpha = calc_alpha_with_normal(forward, camera_forward);
            let elapsed = time_ms.saturating_sub(pane.timer).min(500) as f32;
            let colour = (alpha as f32 * (elapsed / 500.0)) as u8;
            push_triangle(
                &mut highlight_vertices,
                &mut highlight_indices,
                points,
                [[0.5, 0.5], [0.5, 0.6], [0.6, 0.6]],
                [colour; 4],
                true,
            );

            if pane.shattered {
                let distance = magnitude(sub(camera_position, pane.matrix.position));
                let shattered_colour = distance_fade_colour(distance, 255);
                let uvs =
                    triangle.map(|point| [4.0 * point[0] * pane.step, 4.0 * point[1] * pane.step]);
                push_triangle(
                    &mut shattered_vertices,
                    &mut shattered_indices,
                    points,
                    uvs,
                    [shattered_colour; 4],
                    true,
                );
            }
        }

        self.render_batches.clear();
        if !highlight_indices.is_empty() {
            self.render_batches.push(GlassRenderBatch {
                texture_name: "shad_exp",
                blend: GlassBlend::Additive,
                vertices: highlight_vertices,
                indices: highlight_indices,
            });
        }
        if !shattered_indices.is_empty() {
            self.render_batches.push(GlassRenderBatch {
                texture_name: "wincrack_32",
                blend: GlassBlend::Alpha,
                vertices: shattered_vertices,
                indices: shattered_indices,
            });
        }
    }

    /// Returns the queued object handles and resets `NumGlassEntities`, as re3
    /// does after `RenderEntityInGlass` has visited every entry.
    pub fn take_entities_to_render(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.entities_to_render)
    }

    pub fn render_window(
        &mut self,
        window: &GlassWindowState,
        camera_position: CVector,
        camera_forward: CVector,
    ) {
        if window.broken {
            return;
        }
        let distance = magnitude(sub(camera_position, window.position));
        if distance > 40.0 {
            return;
        }
        let indices = double_sided_window_indices(window.triangles);
        if window.cracked {
            let colour = distance_fade_colour(distance, 255);
            self.render_batches.push(GlassRenderBatch {
                texture_name: "wincrack_32",
                blend: GlassBlend::Alpha,
                vertices: window_vertices(
                    window.corners,
                    [colour; 4],
                    [[0.0, 0.0], [16.0, 0.0], [0.0, 16.0], [16.0, 16.0]],
                ),
                indices: indices.clone(),
            });
        }
        let colour = distance_fade_colour(distance, 100);
        let angle = camera_forward.x.atan2(camera_forward.y);
        let v = 2.0 * camera_forward.z * 0.2;
        let u = f32::from(window.random_seed & 15) * 0.02 + angle / std::f32::consts::TAU;
        self.render_batches.push(GlassRenderBatch {
            texture_name: "headlight",
            blend: GlassBlend::Alpha,
            vertices: window_vertices(
                window.corners,
                [colour; 4],
                [[u, v], [u + 0.2, v], [u, v + 0.2], [u + 0.2, v + 0.2]],
            ),
            indices,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn window_responds_to_collision(
        &mut self,
        window: &mut GlassWindowState,
        amount: f32,
        speed: CVector,
        point: CVector,
        explosion: bool,
        time_ms: u32,
        random: &mut impl FnMut() -> u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> Option<f32>,
    ) {
        if window.broken {
            return;
        }
        window.cracked = true;
        let (min, max) = corners_bounds(window.corners);
        self.sound_intents.push(GlassSoundIntent {
            sound: if amount > 300.0 {
                GlassSound::LargeBreak
            } else {
                GlassSound::SmallBreak
            },
            position: window.position,
        });
        self.generate_panes_for_window(
            if amount > 300.0 { 0 } else { 1 },
            min,
            CVector::new(0.0, 0.0, max.z - min.z),
            CVector::new(max.x - min.x, max.y - min.y, 0.0),
            speed,
            point,
            0.1,
            window.cracked,
            explosion,
            time_ms,
            random,
            ground_z,
        );
        window.broken = true;
        window.position.z = -100.0;
    }

    pub fn window_responds_to_soft_collision(
        &mut self,
        window: &mut GlassWindowState,
        amount: f32,
    ) {
        if amount > 50.0 && !window.cracked {
            self.sound_intents.push(GlassSoundIntent {
                sound: GlassSound::Crack,
                position: window.position,
            });
            window.cracked = true;
        }
    }

    pub fn was_glass_hit_by_bullet(
        &mut self,
        window: &mut GlassWindowState,
        point: CVector,
        is_glass_model: bool,
        time_ms: u32,
        random: &mut impl FnMut() -> u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> Option<f32>,
    ) {
        if !is_glass_model {
            return;
        }
        if !window.cracked {
            self.sound_intents.push(GlassSoundIntent {
                sound: GlassSound::Crack,
                position: window.position,
            });
            window.cracked = true;
        } else if random() & 3 == 2 {
            self.window_responds_to_collision(
                window,
                0.0,
                CVector::ZERO,
                point,
                false,
                time_ms,
                random,
                ground_z,
            );
        }
    }

    pub fn window_responds_to_explosion(
        &mut self,
        window: &mut GlassWindowState,
        point: CVector,
        time_ms: u32,
        random: &mut impl FnMut() -> u32,
        ground_z: &mut impl FnMut(f32, f32, f32) -> Option<f32>,
    ) {
        let mut distance_vector = sub(window.position, point);
        let distance = magnitude(distance_vector);
        if distance < 10.0 {
            distance_vector = scale(distance_vector, 0.3 / distance);
            let window_position = window.position;
            self.window_responds_to_collision(
                window,
                10_000.0,
                distance_vector,
                window_position,
                true,
                time_ms,
                random,
                ground_z,
            );
        } else if distance < 30.0 {
            window.cracked = true;
        }
    }

    pub fn drain_sound_intents(&mut self) -> Vec<GlassSoundIntent> {
        std::mem::take(&mut self.sound_intents)
    }

    pub fn drain_debris_intents(&mut self) -> Vec<GlassDebrisIntent> {
        std::mem::take(&mut self.debris_intents)
    }
}

impl Default for CGlass {
    fn default() -> Self {
        Self::init()
    }
}

pub fn calc_alpha_with_normal(normal: CVector, camera_forward: CVector) -> i32 {
    let forward_direction = 2.0 * dot(normal, camera_forward);
    let reflected = sub(camera_forward, scale(normal, forward_direction));
    let forward_dot = dot(reflected, CVector::new(0.57, 0.57, -0.57));
    (20.0 + forward_dot.powi(6) * (255.0 - 20.0)) as i32
}

fn distance_fade_colour(distance: f32, full: u8) -> u8 {
    if distance > 30.0 {
        ((1.0 - (distance - 30.0) * 4.0 / 40.0) * f32::from(full)) as u8
    } else {
        full
    }
}

fn push_triangle(
    vertices: &mut Vec<RwIm3DVertexIntent>,
    indices: &mut Vec<u16>,
    points: [CVector; 3],
    uvs: [[f32; 2]; 3],
    colour: [u8; 4],
    double_sided: bool,
) {
    let base = vertices.len() as u16;
    for i in 0..3 {
        vertices.push(RwIm3DVertexIntent {
            position: points[i],
            colour,
            uv: uvs[i],
        });
    }
    indices.extend([base, base + 1, base + 2]);
    if double_sided {
        indices.extend([base, base + 2, base + 1]);
    }
}

fn window_vertices(
    corners: [CVector; 4],
    colour: [u8; 4],
    uvs: [[f32; 2]; 4],
) -> Vec<RwIm3DVertexIntent> {
    (0..4)
        .map(|i| RwIm3DVertexIntent {
            position: corners[i],
            colour,
            uv: uvs[i],
        })
        .collect()
}

fn double_sided_window_indices(triangles: [[u16; 3]; 2]) -> Vec<u16> {
    let mut indices = Vec::with_capacity(12);
    for triangle in triangles {
        indices.extend(triangle);
    }
    for triangle in triangles {
        indices.extend([triangle[0], triangle[2], triangle[1]]);
    }
    indices
}

fn corners_bounds(corners: [CVector; 4]) -> (CVector, CVector) {
    let mut min = corners[0];
    let mut max = corners[0];
    for corner in &corners[1..] {
        min.x = min.x.min(corner.x);
        min.y = min.y.min(corner.y);
        min.z = min.z.min(corner.z);
        max.x = max.x.max(corner.x);
        max.y = max.y.max(corner.y);
        max.z = max.z.max(corner.z);
    }
    (min, max)
}

fn add(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn sub(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
fn scale(v: CVector, s: f32) -> CVector {
    CVector::new(v.x * s, v.y * s, v.z * s)
}
fn dot(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
fn cross(a: CVector, b: CVector) -> CVector {
    CVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
fn magnitude(v: CVector) -> f32 {
    dot(v, v).sqrt()
}
fn normalise(v: CVector) -> CVector {
    scale(v, 1.0 / magnitude(v))
}

#[cfg(test)]
mod sequence_tests {
    use super::*;

    #[test]
    fn shared_sequence_survives_clear_and_level_resource_replacement() {
        let mut app = App::new();
        app.insert_resource(CGlass::init()).insert_resource(CGeneral::with_seed(3))
            .insert_resource(CTimer::initialise()).insert_resource(CParticle::initialise())
            .insert_resource(super::super::ParticleMgr::cParticleSystemMgr::default())
            .insert_resource(crate::core::Camera::CCamera::default()).add_plugins(GtaGlassPlugin)
            .configure_sets(Update, (
                rust_gta_3_core::core::Schedules::GtaProcessSet::Glass,
                rust_gta_3_core::core::Schedules::GtaProcessSet::World,
                rust_gta_3_core::core::Schedules::GtaProcessSet::Camera,
            ).chain());
        for generation in 1..=2 {
            let mut glass = CGlass::init();
            let mut pane = CFallingGlassPane::default();
            pane.matrix.position.z = -1.0;
            glass.queue_pane(pane).unwrap();
            app.world_mut().insert_resource(glass);
            app.update();
            assert_eq!(app.world().resource::<CParticle>().add_intents.iter()
                .rev().take(4).rev().map(|intent| intent.current_frame).collect::<Vec<_>>(), [1, 2, 3, 0]);
            app.world_mut().resource_mut::<CGlass>().clear();
            app.world_mut().init_resource::<GlassDebrisSequence>();
            assert_eq!(app.world().resource::<GlassDebrisSequence>().generated_fragments(), generation * 4);
        }
    }

    #[test]
    fn low_bit_sequence_wraps_without_an_overflow_panic() {
        let mut sequence = GlassDebrisSequence { generated: u32::MAX - 1 };
        assert_eq!(sequence.next_frame(), 3);
        assert_eq!(sequence.next_frame(), 0);
        assert_eq!(sequence.next_frame(), 1);
    }

    #[test]
    fn entity_capacity_external_retirement_and_reset_use_one_lifecycle_owner() {
        let mut app = App::new();
        let mut glass = CGlass::init();
        for slot in 0..NUM_GLASSPANES {
            assert_eq!(glass.queue_pane(CFallingGlassPane::default()), Some(slot));
        }
        assert_eq!(glass.queue_pane(CFallingGlassPane::default()), None);
        app.insert_resource(glass).add_systems(Update, reconcile_glass_panes);
        app.update();
        app.update();
        let first = app.world().resource::<CGlass>().pane_entity(0).unwrap();
        assert_eq!(app.world().resource::<CGlass>().live_pane_count(), NUM_GLASSPANES);
        assert!(app.world().get::<CFallingGlassPane>(first).is_some());
        app.world_mut().entity_mut(first).remove::<CFallingGlassPane>();
        app.update();
        assert!(app.world().get_entity(first).is_err(), "component removal retires the full actor");
        assert_eq!(app.world_mut().resource_mut::<CGlass>().queue_pane(CFallingGlassPane::default()), Some(0));
        app.update();
        let replacement = app.world().resource::<CGlass>().pane_entity(0).unwrap();
        assert_ne!(replacement, first);
        app.world_mut().insert_resource(CGlass::init());
        app.update();
        assert!(app.world().get_entity(replacement).is_err());
        assert_eq!(app.world().resource::<CGlass>().live_pane_count(), 0);
    }
}
