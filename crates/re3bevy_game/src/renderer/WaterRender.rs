//! CWaterLevel original-data render adapter; ECS owns meshes and materials.
//! re3 reference: references/re-GTA/src/renderer/WaterLevel.cpp

use super::*;

#[path = "WaterSectors.rs"]
mod sectors;
use sectors::{WaterSector, select_water_sectors};
#[path = "WaterWake.rs"]
mod wake;
use crate::peds::{PlayerPed::CWorldPlayerState, VehicleRelations::InVehicle};
use wake::WaterBoats;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaWaterMesh;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct GtaWaterWaveMesh;

#[derive(Debug, Clone, Resource, Default)]
pub(super) struct GtaWaterAnimationState {
    texture_u: f32,
    texture_v: f32,
    flat_sectors: Vec<WaterSector>,
    wake_sector_boats: [Option<Entity>; 4],
    bird_previous_position: CVector,
    bird_stationary_since: u32,
    bird_last_emitted: u32,
}

/// Observable gates for original-data water rendering.
#[derive(Debug, Clone, Resource, Default)]
pub struct GtaWaterRenderStats {
    pub missing_level_frames: u64,
    pub missing_timecycle_frames: u64,
    pub missing_texture_frames: u64,
    pub empty_grid_frames: u64,
    pub missing_material_frames: u64,
    pub missing_weather_frames: u64,
    pub missing_camera_frames: u64,
    pub missing_base_mesh_frames: u64,
    pub flat_vertices: usize,
    pub wave_vertices: usize,
    pub surface_updates: u64,
    pub wavy_draw_distance: f32,
    pub wake_vertices: usize,
    pub bird_emissions: u64,
}

/// CWaterLevel::Shutdown / DestroyWavyAtomic: release render ownership before
/// a fresh game's original water data is installed. Pausing keeps assets alive.
pub(super) fn reset_water_render(
    mut commands: Commands,
    water: Query<
        (Entity, &Mesh3d, &MeshMaterial3d<GtaWaterMaterial>),
        Or<(With<GtaWaterMesh>, With<GtaWaterWaveMesh>)>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaWaterMaterial>>,
    mut animation: ResMut<GtaWaterAnimationState>,
    mut stats: ResMut<GtaWaterRenderStats>,
) {
    for (entity, mesh, material) in &water {
        meshes.remove(mesh.0.id());
        materials.remove(material.0.id());
        commands.entity(entity).despawn();
    }
    *animation = GtaWaterAnimationState::default();
    *stats = GtaWaterRenderStats::default();
}

pub(super) fn spawn_water_mesh_once(
    mut commands: Commands,
    water_level: Option<Res<CWaterLevel>>,
    timecycle: Option<Res<CTimeCycle>>,
    camera: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    file_loader: Option<Res<CFileLoader>>,
    existing: Query<Entity, With<GtaWaterMesh>>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut materials: ResMut<Assets<GtaWaterMaterial>>,
    mut stats: ResMut<GtaWaterRenderStats>,
) {
    if existing.iter().next().is_some() {
        return;
    }
    let Some(water_level) = water_level else {
        stats.missing_level_frames += 1;
        return;
    };
    let Some(timecycle) = timecycle.as_deref() else {
        stats.missing_timecycle_frames += 1;
        return;
    };
    let water_texture = file_loader.as_deref().and_then(|file_loader| {
        resolve_txd_texture_asset(
            file_loader,
            &mut texture_cache,
            &mut images,
            "particle",
            "water_old",
        )
        .map(|(handle, _)| handle)
    });
    let Some(water_texture) = water_texture else {
        stats.missing_texture_frames += 1;
        if stats.missing_texture_frames == 1 || stats.missing_texture_frames % 300 == 0 {
            warn!(target: "gta_assets", frames = stats.missing_texture_frames,
                "water rendering waiting for particle TXD texture water_old");
        }
        return;
    };
    let Some(camera) = camera.as_deref() else {
        stats.missing_camera_frames += 1;
        return;
    };
    let plan = select_water_sectors(&water_level, camera, 70.0);
    let mesh = build_water_level_mesh(&plan.flat);
    if mesh.count_vertices() == 0 && plan.wave.is_empty() {
        stats.empty_grid_frames += 1;
        return;
    }
    stats.flat_vertices = mesh.count_vertices();
    info!(target: "gta_assets", levels = water_level.no_of_water_levels,
        vertices = stats.flat_vertices, "water mesh uses original DATA/waterpro.dat and particle/water_old");
    commands.spawn((
        GtaWaterMesh,
        // RenderWater already culls selected sectors with the native camera.
        // Camera-dependent mesh contents must not reuse an initial Bevy AABB.
        NoFrustumCulling,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(water_material_from_timecycle(
            timecycle,
            postfx.as_deref(),
            Some(camera),
            Some(water_texture),
        ))),
        Transform::default(),
        GlobalTransform::default(),
    ));
}

pub(super) fn sync_water_timecycle_material(
    timecycle: Option<Res<CTimeCycle>>,
    camera: Option<Res<CCamera>>,
    postfx: Option<Res<CPostFX>>,
    file_loader: Option<Res<CFileLoader>>,
    mut existing: Query<
        (
            &MeshMaterial3d<GtaWaterMaterial>,
            &mut Visibility,
            Has<GtaWaterWaveMesh>,
        ),
        Or<(With<GtaWaterMesh>, With<GtaWaterWaveMesh>)>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut materials: ResMut<Assets<GtaWaterMaterial>>,
    mut stats: ResMut<GtaWaterRenderStats>,
) {
    let Some(timecycle) = timecycle.as_deref() else {
        stats.missing_timecycle_frames += 1;
        return;
    };
    let streamed_water_texture = file_loader.as_deref().and_then(|file_loader| {
        resolve_txd_texture_asset(
            file_loader,
            &mut texture_cache,
            &mut images,
            "particle",
            "water_old",
        )
        .map(|(handle, _)| handle)
    });
    for (handle, mut visibility, is_wave) in &mut existing {
        let Some(mut material) = materials.get_mut(&handle.0) else {
            stats.missing_material_frames += 1;
            continue;
        };
        let uv_offset = material.uv_offset;
        // Re-resolve the current TXD incarnation; a released dictionary must
        // not keep rendering through a stale raster handle or white fallback.
        let water_texture = streamed_water_texture.clone();
        if water_texture.is_none() {
            stats.missing_texture_frames += 1;
            material.water_texture = None;
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility =
            if (is_wave && stats.wave_vertices == 0) || (!is_wave && stats.flat_vertices == 0) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        *material = water_material_from_timecycle(
            timecycle,
            postfx.as_deref(),
            camera.as_deref(),
            water_texture,
        );
        material.uv_offset = uv_offset;
    }
}

/// RenderWater's stationary-camera sunny-weather bird branch. Particle texture
/// and lifetime resolution remain owned by the original particle subsystem.
pub(super) fn emit_water_birds(
    timer: Res<CTimer>,
    clock: Res<re3bevy_core::core::Clock::CClock>,
    weather: Res<CWeather>,
    camera: Res<CCamera>,
    loader: Res<CFileLoader>,
    mut general: ResMut<CGeneral>,
    mut particles: ResMut<crate::renderer::Particle::CParticle>,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    mut animation: ResMut<GtaWaterAnimationState>,
    mut stats: ResMut<GtaWaterRenderStats>,
) {
    let cull = &loader.world.cull_zones;
    if cull.camera_no_rain()
        || cull.player_no_rain()
        || weather.new_weather_type != crate::renderer::Weather::WEATHER_SUNNY
        || clock.hours() <= 6
        || clock.hours() >= 20
        || stats.wave_vertices == 0
    {
        return;
    }
    let position = camera.position;
    let previous = animation.bird_previous_position;
    let now = timer.time_in_milliseconds();
    if (previous.x - position.x).abs()
        + (previous.y - position.y).abs()
        + (previous.z - position.z).abs()
        > 1.5
    {
        animation.bird_previous_position = position;
        animation.bird_stationary_since = now;
    } else if now.wrapping_sub(animation.bird_stationary_since) > 5000
        && now.wrapping_sub(animation.bird_last_emitted) > 1000
    {
        animation.bird_last_emitted = now;
        let radius = general.get_random_number_in_range_f32(90.0, 150.0);
        // Native expression is (random % table_size) - 1. Evaluating the angle
        // also handles its -1 case without indexing before the sine table.
        let rotation = general.get_random_number() as i32
            % crate::renderer::Particle::SIN_COS_TABLE_SIZE as i32
            - 1;
        let angle = rotation as f32 * std::f32::consts::TAU
            / crate::renderer::Particle::SIN_COS_TABLE_SIZE as f32;
        let (sin, cos) = angle.sin_cos();
        let position = CVector::new(
            position.x + (cos - sin) * radius,
            position.y + (sin + cos) * radius,
            position.z + general.get_random_number_in_range_f32(10.0, 30.0),
        );
        let direction = CVector::new(
            general.get_random_number_in_range_f32(-1.0, 1.0),
            general.get_random_number_in_range_f32(-1.0, 1.0),
            0.0,
        );
        particles.emitter(&particle_systems, camera.position).add_particle(
            crate::renderer::ParticleType::PARTICLE_BIRD_FRONT,
            position,
            direction,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
        stats.bird_emissions += 1;
    }
}

pub(super) fn sync_water_surface_animation(
    mut commands: Commands,
    timer: Res<CTimer>,
    mut general: ResMut<CGeneral>,
    weather: Option<Res<crate::renderer::Weather::CWeather>>,
    water_level: Option<Res<CWaterLevel>>,
    camera: Option<Res<CCamera>>,
    players: Res<CWorldPlayerState>,
    occupants: Query<&InVehicle>,
    boats: WaterBoats,
    mut base_water: Query<
        (&Mesh3d, &MeshMaterial3d<GtaWaterMaterial>, &mut Visibility),
        (With<GtaWaterMesh>, Without<GtaWaterWaveMesh>),
    >,
    mut wave_water: Query<
        (&Mesh3d, &MeshMaterial3d<GtaWaterMaterial>, &mut Visibility),
        (With<GtaWaterWaveMesh>, Without<GtaWaterMesh>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaWaterMaterial>>,
    mut animation: ResMut<GtaWaterAnimationState>,
    mut stats: ResMut<GtaWaterRenderStats>,
) {
    let now_ms = timer.time_in_milliseconds();
    let Some((base_mesh, water_material, mut base_visibility)) = base_water.iter_mut().next()
    else {
        stats.missing_base_mesh_frames += 1;
        return;
    };
    let Some(material) = materials.get(&water_material.0) else {
        stats.missing_material_frames += 1;
        return;
    };
    if material.water_texture.is_none() {
        stats.missing_texture_frames += 1;
        *base_visibility = Visibility::Hidden;
        for (_, _, mut visibility) in &mut wave_water {
            *visibility = Visibility::Hidden;
        }
        stats.wave_vertices = 0;
        stats.wake_vertices = 0;
        return;
    }
    let Some(weather) = weather.as_deref() else {
        stats.missing_weather_frames += 1;
        return;
    };
    let wind = weather.wind;
    let wind_add_uv = wind * 0.0015 + 0.0005;
    // CWaterLevel::RenderWater (FIX_BUGS): UV motion runs each render frame.
    // The game timer already applies time scale; do not apply it a second time.
    if !timer.get_is_paused() {
        let step = timer.get_time_step_fix();
        animation.texture_u +=
            (general.get_random_number_in_range_f32(-0.0005, 0.0005) + wind_add_uv) * step;
        animation.texture_v +=
            (general.get_random_number_in_range_f32(-0.0005, 0.0005) + wind_add_uv) * step;
    }
    if animation.texture_u >= 1.0 {
        animation.texture_u = 0.0;
    }
    if animation.texture_v >= 1.0 {
        animation.texture_v = 0.0;
    }

    if let Some(mut material) = materials.get_mut(&water_material.0) {
        material.uv_offset = Vec4::new(animation.texture_u, animation.texture_v, 0.0, 0.0);
    }
    for (_, handle, _) in &wave_water {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.uv_offset = Vec4::new(animation.texture_v, animation.texture_u, 1.0, 0.0);
        }
    }
    let Some(water_level) = water_level.as_deref() else {
        stats.missing_level_frames += 1;
        return;
    };
    let Some(camera) = camera.as_deref() else {
        stats.missing_camera_frames += 1;
        return;
    };
    let player_in_boat = players
        .entities
        .get(players.player_in_focus)
        .copied()
        .flatten()
        .and_then(|entity| occupants.get(entity).ok())
        .is_some_and(|relation| boats.get(relation.vehicle).is_ok());
    let wavy_distance = if player_in_boat { 120.0 } else { 70.0 };
    stats.wavy_draw_distance = wavy_distance;
    let plan = select_water_sectors(water_level, camera, wavy_distance);
    stats.flat_vertices = plan.flat.len() * 4;
    if animation.flat_sectors != plan.flat || stats.surface_updates == 0 {
        // An empty selection is visibility state, not a zero-sized GPU mesh
        // resize. Keep the last allocation hidden until sectors return.
        if !plan.flat.is_empty() {
            let Some(mut mesh) = meshes.get_mut(&base_mesh.0) else {
                stats.missing_base_mesh_frames += 1;
                return;
            };
            *mesh = build_water_level_mesh(&plan.flat);
        }
        animation.flat_sectors = plan.flat;
    }
    *base_visibility = if stats.flat_vertices == 0 {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    stats.surface_updates += 1;
    let base_colour = materials
        .get(&water_material.0)
        .map(|material| material.colour.to_array());
    let Some(base_colour) = base_colour else {
        stats.missing_material_frames += 1;
        return;
    };
    let mut wake_colours = Vec::with_capacity(plan.wave.len() * 81);
    stats.wake_vertices = 0;
    let selected = if player_in_boat {
        wake::select_wake_boats(&boats)
    } else {
        [None; 4]
    };
    let mut wake_sectors_used = 0;
    for sector in &plan.wave {
        // AllocateBoatWakeArray is tied to the player's boat entry/exit.
        // ECS mesh streams provide eight affected sectors without RW clones.
        let affected = player_in_boat
            && wake_sectors_used < 8
            && wake::sector_affected(
                &boats,
                &selected,
                &mut animation.wake_sector_boats,
                sector.x + 16.0,
                sector.y + 16.0,
            );
        if affected {
            wake_sectors_used += 1;
        }
        for i in 0..9 {
            for j in 0..9 {
                let intensity = if affected {
                    animation
                        .wake_sector_boats
                        .iter()
                        .flatten()
                        .filter_map(|entity| boats.get(*entity).ok())
                        .map(|(_, _, boat)| {
                            wake::vertex_influence(
                                boat,
                                sector.x + i as f32 * 4.0,
                                sector.y + j as f32 * 4.0,
                            )
                        })
                        .sum()
                } else {
                    0.0
                };
                if intensity > 0.0 {
                    stats.wake_vertices += 1;
                }
                wake_colours.push(if intensity > 0.0 {
                    wake::wake_colour(base_colour, intensity)
                } else {
                    base_colour
                });
            }
        }
    }
    let Some(mut wave_mesh) = build_wavy_water_mesh(&plan.wave, wind, now_ms) else {
        stats.wave_vertices = 0;
        if let Some((_, _, mut visibility)) = wave_water.iter_mut().next() {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    wave_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, wake_colours);
    stats.wave_vertices = wave_mesh.count_vertices();

    if let Some((mesh_handle, _, mut visibility)) = wave_water.iter_mut().next() {
        if let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
            *mesh = wave_mesh;
        } else {
            stats.missing_base_mesh_frames += 1;
            return;
        }
        *visibility = Visibility::Inherited;
    } else {
        let Some(mut wave_material) = materials.get(&water_material.0).cloned() else {
            stats.missing_material_frames += 1;
            return;
        };
        // RenderOneWavySector swaps ADDV/ADDU relative to flat sectors.
        wave_material.uv_offset = Vec4::new(animation.texture_v, animation.texture_u, 1.0, 0.0);
        commands.spawn((
            GtaWaterWaveMesh,
            NoFrustumCulling,
            Mesh3d(meshes.add(wave_mesh)),
            MeshMaterial3d(materials.add(wave_material)),
            Transform::default(),
            GlobalTransform::default(),
        ));
    }
}

fn water_material_from_timecycle(
    timecycle: &CTimeCycle,
    postfx: Option<&CPostFX>,
    camera: Option<&CCamera>,
    water_texture: Option<Handle<Image>>,
) -> GtaWaterMaterial {
    let mut params = {
        let current = &timecycle.current;
        let water_colour = [
            (current.ambient[0] + current.directional[0] * 0.5).clamp(0.0, 1.0),
            (current.ambient[1] + current.directional[1] * 0.5).clamp(0.0, 1.0),
            (current.ambient[2] + current.directional[2] * 0.5).clamp(0.0, 1.0),
            1.0,
        ];
        // RenderWater converts each lighting channel to an RwRGBA byte before
        // either flat submission or wake prelight calculations.
        let water_colour = water_colour.map(|value| (value * 255.0) as u8 as f32 / 255.0);
        RwMaterialParams::from_timecycle(current, water_colour, 0.0)
    };
    params.postfx_blur_colour = postfx
        .zip(camera)
        .and_then(|(postfx, camera)| postfx.effective_colour_filter(camera))
        .unwrap_or([1.0, 1.0, 1.0, 0.0]);
    let mut material = GtaWaterMaterial::from(params);
    material.water_texture = water_texture;
    material
}

fn build_water_level_mesh(sectors: &[WaterSector]) -> Mesh {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for sector in sectors {
        let WaterSector { x, y, level, size } = *sector;
        let base = positions.len() as u32;
        positions.extend([
            gta_to_bevy_position(x, y, level - WATER_Z_OFFSET),
            gta_to_bevy_position(x, y + size, level - WATER_Z_OFFSET),
            gta_to_bevy_position(x + size, y + size, level - WATER_Z_OFFSET),
            gta_to_bevy_position(x + size, y, level - WATER_Z_OFFSET),
        ]);
        let scale = size / SMALL_SECTOR_SIZE;
        uvs.extend([[0.0, 0.0], [0.0, scale], [scale, scale], [scale, 0.0]]);
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0; 4]; count])
}

fn build_wavy_water_mesh(sectors: &[WaterSector], wind: f32, time_ms: u32) -> Option<Mesh> {
    const GRID_SIZE: usize = 9;
    const GRID_STEP: f32 = SMALL_SECTOR_SIZE / (GRID_SIZE as f32 - 1.0);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colours = Vec::new();
    let mut indices = Vec::new();
    let angle = (time_ms & 4095) as f32 * (std::f32::consts::TAU / 4096.0);
    for sector in sectors {
        let WaterSector {
            x: x0,
            y: y0,
            level,
            ..
        } = *sector;
        let base = positions.len() as u32;
        for i in 0..GRID_SIZE {
            for j in 0..GRID_SIZE {
                let i_f = i as f32;
                let j_f = j as f32;
                let wave = (wind * 0.7 + 0.3)
                    * ((i_f + j_f) * std::f32::consts::FRAC_PI_4 + angle).sin()
                    + wind * 0.2 * ((j_f - i_f) * std::f32::consts::PI + 2.0 * angle).sin();
                positions.push(gta_to_bevy_position(
                    x0 + i_f * GRID_STEP,
                    y0 + j_f * GRID_STEP,
                    level + wave,
                ));
                normals.push([0.0, 1.0, 0.0]);
                uvs.push([
                    i_f / (GRID_SIZE as f32 - 1.0),
                    j_f / (GRID_SIZE as f32 - 1.0),
                ]);
                colours.push([1.0, 1.0, 1.0, 1.0]);
            }
        }

        for i in 0..(GRID_SIZE - 1) {
            for j in 0..(GRID_SIZE - 1) {
                let a = base + (i * GRID_SIZE + j) as u32;
                let b = a + 1;
                let d = base + ((i + 1) * GRID_SIZE + j) as u32;
                let c = d + 1;
                indices.extend([a, c, b, a, d, c]);
            }
        }
    }

    if positions.is_empty() {
        return None;
    }
    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours),
    )
}

#[cfg(test)]
mod water_render_tests {
    use super::*;

    #[test]
    fn flat_water_keeps_re3_z_offset_below_wavy_surface() {
        let sectors = [WaterSector {
            x: 0.0,
            y: 0.0,
            level: 10.0,
            size: SMALL_SECTOR_SIZE,
        }];
        let flat = build_water_level_mesh(&sectors);
        let wave = build_wavy_water_mesh(&sectors, 0.0, 0).unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(flat_positions)) =
            flat.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("flat positions missing")
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(wave_positions)) =
            wave.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("wave positions missing")
        };
        assert_eq!(flat_positions[0][1], 10.0 - WATER_Z_OFFSET);
        assert_eq!(wave_positions.len(), 81);
        assert_eq!(wave_positions[0][1], 10.0);
    }

    #[test]
    fn wave_geometry_changes_inside_the_former_33_ms_gate() {
        let sectors = [WaterSector {
            x: 0.0,
            y: 0.0,
            level: 0.0,
            size: SMALL_SECTOR_SIZE,
        }];
        let first = build_wavy_water_mesh(&sectors, 0.5, 0).unwrap();
        let next = build_wavy_water_mesh(&sectors, 0.5, 16).unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(first)) =
            first.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions missing")
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(next)) =
            next.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions missing")
        };
        assert_ne!(first, next);
    }

    #[test]
    fn missing_original_texture_hides_both_surfaces_without_advancing_animation() {
        let mut app = App::new();
        app.init_resource::<CTimer>()
            .init_resource::<CGeneral>()
            .init_resource::<CWorldPlayerState>()
            .init_resource::<GtaWaterAnimationState>()
            .init_resource::<GtaWaterRenderStats>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<GtaWaterMaterial>>()
            .add_systems(Update, sync_water_surface_animation);
        let material = app
            .world_mut()
            .resource_mut::<Assets<GtaWaterMaterial>>()
            .add(GtaWaterMaterial::from(RwMaterialParams {
                colour: [1.0; 4],
                fog_colour_and_factor: [0.0; 4],
                postfx_blur_colour: [0.0; 4],
            }));
        let base = app
            .world_mut()
            .spawn((
                GtaWaterMesh,
                Mesh3d::default(),
                MeshMaterial3d(material.clone()),
                Visibility::Inherited,
            ))
            .id();
        let wave = app
            .world_mut()
            .spawn((
                GtaWaterWaveMesh,
                Mesh3d::default(),
                MeshMaterial3d(material),
                Visibility::Inherited,
            ))
            .id();
        app.world_mut()
            .resource_mut::<GtaWaterRenderStats>()
            .wave_vertices = 81;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(base),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().get::<Visibility>(wave),
            Some(&Visibility::Hidden)
        );
        let stats = app.world().resource::<GtaWaterRenderStats>();
        assert_eq!(stats.wave_vertices, 0);
        assert_eq!(stats.missing_texture_frames, 1);
        assert_eq!(stats.surface_updates, 0);
    }
}
