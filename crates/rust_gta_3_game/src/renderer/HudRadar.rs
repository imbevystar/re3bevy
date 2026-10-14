//! CRadar::DrawRadarSection: sample each resident original TXD on the GPU.
use super::{
    CFileLoader, H, HudPainter, RADAR_BOTTOM, RADAR_HEIGHT, RADAR_LEFT, RADAR_MIN_X, RADAR_MIN_Y,
    RADAR_NUM_TILES, RADAR_TILE_SIZE, RADAR_WIDTH, W, pct,
};
use crate::renderer::BevyRender::{GtaWorldTextureCache, sync_txd_slot_texture_cache};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderType},
        storage::ShaderBuffer,
    },
    shader::ShaderRef,
    ui_render::{
        UiMaterialPlugin,
        ui_material::{MaterialNode, UiMaterial},
    },
};
use std::collections::BTreeMap;

pub const RADAR_SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("c7e41e61-826f-47b9-919c-c6a44e0e3e01");
const RADAR_FRAME: Handle<ShaderBuffer> =
    bevy::asset::uuid_handle!("c7e41e61-826f-47b9-919c-c6a44e0e3e02");

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RadarMaterial {
    #[uniform(0)]
    pub tile: Vec4,
    #[uniform(1)]
    pub sample_origin: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub texture: Handle<Image>,
    #[storage(4, read_only)]
    pub frame: Handle<ShaderBuffer>,
}
impl UiMaterial for RadarMaterial {
    fn fragment_shader() -> ShaderRef {
        RADAR_SHADER.into()
    }
}
#[repr(C)]
#[derive(Default, Clone, Copy, PartialEq, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
struct RadarFrame {
    origins: [Vec4; 2],
    headings: [Vec4; 2],
}
#[derive(Default, Resource)]
pub struct RadarDiagnostics {
    pub waiting_tile_draws: u64,
    pub submitted_tiles: u64,
    pub frame_uploads: u64,
}
struct TileSlot {
    texture: Handle<Image>,
    material: Handle<RadarMaterial>,
}
#[derive(Default, Resource)]
pub struct RadarState {
    frame: RadarFrame,
    tiles: [BTreeMap<(i32, i32), TileSlot>; 2],
}

pub struct RadarPlugin;
impl Plugin for RadarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiMaterialPlugin::<RadarMaterial>::default())
            .init_resource::<RadarState>()
            .init_resource::<RadarDiagnostics>();
        let buffer = ShaderBuffer::from(vec![RadarFrame::default()]);
        app.world_mut()
            .resource_mut::<Assets<ShaderBuffer>>()
            .insert(RADAR_FRAME.id(), buffer)
            .expect("program radar frame UUID");
    }
}

#[derive(SystemParam)]
pub struct RadarGpu<'w> {
    state: ResMut<'w, RadarState>,
    materials: ResMut<'w, Assets<RadarMaterial>>,
    buffers: ResMut<'w, Assets<ShaderBuffer>>,
    textures: ResMut<'w, GtaWorldTextureCache>,
    diagnostics: ResMut<'w, RadarDiagnostics>,
}
impl RadarGpu<'_> {
    pub(super) fn retire_player(&mut self, player: usize) {
        for (_, slot) in std::mem::take(&mut self.state.tiles[player]) {
            self.materials.remove(slot.material.id());
        }
    }
    pub(super) fn retire_all(&mut self) {
        self.retire_player(0);
        self.retire_player(1);
    }
    pub(super) fn draw(
        &mut self,
        p: &mut HudPainter,
        loader: &CFileLoader,
        images: &mut Assets<Image>,
        player: usize,
        px: f32,
        py: f32,
        heading: f32,
        range: f32,
    ) {
        let (sin, cos) = heading.sin_cos();
        let origin = Vec4::new(px, py, range, 0.0);
        let angles = Vec4::new(cos, sin, 0.0, 0.0);
        if self.state.frame.origins[player] != origin || self.state.frame.headings[player] != angles
        {
            self.state.frame.origins[player] = origin;
            self.state.frame.headings[player] = angles;
            if let Some(mut buffer) = self.buffers.get_mut(&RADAR_FRAME) {
                buffer.clear();
                buffer.extend_from_slice(std::slice::from_ref(&self.state.frame));
                self.diagnostics.frame_uploads += 1;
            }
        }
        let x = ((px - RADAR_MIN_X) / RADAR_TILE_SIZE).floor() as i32;
        let y = ((RADAR_NUM_TILES - 1) as f32 - (py - RADAR_MIN_Y) / RADAR_TILE_SIZE).ceil() as i32;
        self.state.tiles[player].retain(|&(tx, ty), slot| {
            let keep = (x - 1..=x + 1).contains(&tx) && (y - 1..=y + 1).contains(&ty);
            if !keep {
                self.materials.remove(slot.material.id());
            }
            keep
        });
        for ty in y - 1..=y + 1 {
            for tx in x - 1..=x + 1 {
                // Native GetTextureCorners runs before ClipRadarTileCoords:
                // edge cells extend outside the map but sample clamped rasters.
                let sx = tx.clamp(0, RADAR_NUM_TILES as i32 - 1);
                let sy = ty.clamp(0, RADAR_NUM_TILES as i32 - 1);
                let name = format!("radar{:02}", sx + sy * RADAR_NUM_TILES as i32);
                let texture = loader.txd_store.find_txd_slot(&name).and_then(|slot| {
                    if !sync_txd_slot_texture_cache(slot, loader, &mut self.textures, images) {
                        return None;
                    }
                    let raster = loader
                        .txd_store
                        .slot(slot)?
                        .texture_summary
                        .as_ref()?
                        .textures
                        .first()?;
                    self.textures
                        .images
                        .get(&(slot, raster.name.to_ascii_lowercase()))
                        .cloned()
                });
                let Some(texture) = texture else {
                    self.diagnostics.waiting_tile_draws += 1;
                    if let Some(slot) = self.state.tiles[player].remove(&(tx, ty)) {
                        self.materials.remove(slot.material.id());
                    }
                    continue;
                };
                let tile = Vec4::new(
                    RADAR_MIN_X + tx as f32 * RADAR_TILE_SIZE,
                    RADAR_MIN_Y + (RADAR_NUM_TILES as i32 - 1 - ty) as f32 * RADAR_TILE_SIZE,
                    RADAR_TILE_SIZE,
                    player as f32,
                );
                let sample_origin = Vec4::new(
                    RADAR_MIN_X + sx as f32 * RADAR_TILE_SIZE,
                    RADAR_MIN_Y + (RADAR_NUM_TILES as i32 - 1 - sy) as f32 * RADAR_TILE_SIZE,
                    0.0,
                    0.0,
                );
                let material = if let Some(slot) = self.state.tiles[player].get_mut(&(tx, ty)) {
                    if slot.texture != texture {
                        if let Some(mut material) = self.materials.get_mut(&slot.material) {
                            material.texture = texture.clone();
                        }
                        slot.texture = texture;
                    }
                    slot.material.clone()
                } else {
                    let material = self.materials.add(RadarMaterial {
                        tile,
                        sample_origin,
                        texture: texture.clone(),
                        frame: RADAR_FRAME.clone(),
                    });
                    self.state.tiles[player].insert(
                        (tx, ty),
                        TileSlot {
                            texture,
                            material: material.clone(),
                        },
                    );
                    material
                };
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: pct(RADAR_LEFT, W),
                        top: pct(H - RADAR_BOTTOM - RADAR_HEIGHT, H),
                        width: pct(RADAR_WIDTH, W),
                        height: pct(RADAR_HEIGHT, H),
                        ..default()
                    },
                    MaterialNode(material),
                    Name::new("RadarMap"),
                ));
                self.diagnostics.submitted_tiles += 1;
            }
        }
    }
}
