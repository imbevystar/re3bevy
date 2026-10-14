use std::collections::{BTreeMap, BTreeSet, HashMap};
use crate::vehicle_construction::{ArenaVehicleBuildError, ArenaVehicleInput,
    ArenaVehicleRuntimeOutcome, ArenaVehicleSpawnDiagnostics, ArenaVehicleSpawnResult};

use bevy::{
    asset::RenderAssetUsages,
    camera::{Viewport, visibility::RenderLayers},
    ecs::query::QueryFilter,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::view::{DebandDither, Msaa, Tonemapping},
    window::PrimaryWindow,
};
use rust_gta_3_assets::assets::WorldAssets::{CHandlingDataAsset, CSurfaceTableAsset};
use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};
use rust_gta_3_game::{
    core::{
        FileLoader::CFileLoader,
        LocalCoop::{LocalCoopSession, LocalCoopSplitLayout},
        World::CWorldEntity,
    },
    entities::Entity::eEntityStatus,
    modelinfo::ModelInfo::ModelInfoSlot,
    modelinfo::VehicleModelInfo::{CVehicleColours, eVehicleType},
    renderer::BevyRender::{
        ModelMaterialDrawable, VehicleColourRemap, VehicleResidentMeshState,
        build_model_material_drawables, build_rigid_ped_material_drawables,
        build_vehicle_component_transforms, vehicle_component_relative_transform,
        world_entity_snapshot, world_model_snapshot,
    },
    renderer::{
        BevyRender::{
            GtaWorldTextureCache, arena_texture_bind_for_drawable, sync_world_texture_cache,
        },
        RendererMaterials::{GtaUnlitMaterial, RwTextureAlphaMode},
    },
    vehicles::{Automobile::CAutomobile, HandlingMgr::cHandlingDataMgr, Vehicle::MISSION_VEHICLE},
};

use crate::{
    ARENA_LIGHTING_BUFFER, ARENA_PANEL_WIDTH, ARENA_RENDER_LAYER,
    ArenaCamera, ArenaCharacterModelPart, ArenaCharacterVisual, ArenaCoopPlayerTwo, ArenaGrid,
    ArenaModelPart, ArenaOriginalRuntime, ArenaOwned, ArenaPlayer, ArenaSceneRoot, ArenaSpawned,
    ArenaVehicle, ArenaVehicleModelPart, ArenaWorldMap, ArenaWorldMapEntity, DeveloperArena,
};

const GRID_HALF_EXTENT: i32 = 200;
const GRID_STEP: i32 = 5;
const ARENA_WORLD_RENDER_RADIUS: f32 = 450.0;
const ARENA_WORLD_BIG_BUILDING_RADIUS: f32 = 900.0;

struct PreparedArenaDrawable {
    drawable: ModelMaterialDrawable,
    texture: Option<Handle<Image>>,
    texture_alpha: RwTextureAlphaMode,
    source_part_index: usize,
    source_part_indices: Vec<usize>,
    attach_vehicle_part: bool,
}

fn grid_lines_mesh(
    coordinates: &[f32],
    thickness: f32,
    height: f32,
    include_x_lines: bool,
    include_z_lines: bool,
) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();
    let half_extent = GRID_HALF_EXTENT as f32;
    let half_thickness = thickness * 0.5;
    let mut push_quad = |min_x: f32, max_x: f32, min_z: f32, max_z: f32| {
        let base = positions.len() as u32;
        positions.extend([
            [min_x, height, min_z],
            [max_x, height, min_z],
            [max_x, height, max_z],
            [min_x, height, max_z],
        ]);
        normals.extend([[0.0, 1.0, 0.0]; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    };
    for &coordinate in coordinates {
        if include_x_lines {
            push_quad(
                -half_extent,
                half_extent,
                coordinate - half_thickness,
                coordinate + half_thickness,
            );
        }
        if include_z_lines {
            push_quad(
                coordinate - half_thickness,
                coordinate + half_thickness,
                -half_extent,
                half_extent,
            );
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(crate) fn spawn_arena(
    commands: &mut Commands,
    arena: &mut DeveloperArena,
    player_dto: rust_gta_3_game::peds::PlayerPed::CPlayerPed,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gta_materials: ResMut<Assets<GtaUnlitMaterial>>,
    layer: RenderLayers,
) {
    let scene_root = commands
        .spawn((
            ArenaOwned,
            ArenaSceneRoot,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new("Developer Arena Scene"),
        ))
        .id();

    let ground_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.055, 0.062, 0.072),
        unlit: true,
        ..default()
    });
    let minor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.28, 0.32),
        emissive: LinearRgba::new(0.035, 0.04, 0.045, 1.0),
        unlit: true,
        ..default()
    });
    let major_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.40, 0.48),
        emissive: LinearRgba::new(0.08, 0.10, 0.12, 1.0),
        unlit: true,
        ..default()
    });
    let axis_x_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.12, 0.12),
        emissive: LinearRgba::new(0.15, 0.01, 0.01, 1.0),
        unlit: true,
        ..default()
    });
    let axis_z_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.32, 0.85),
        emissive: LinearRgba::new(0.01, 0.04, 0.18, 1.0),
        unlit: true,
        ..default()
    });
    let grid = commands
        .spawn((
            ArenaGrid,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new("Infinite-style Test Grid"),
        ))
        .id();
    commands.entity(scene_root).add_child(grid);

    let ground = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(400.0, 0.025, 400.0))),
            MeshMaterial3d(ground_material),
            Transform::from_xyz(0.0, -0.025, 0.0),
            GlobalTransform::default(),
            layer.clone(),
        ))
        .id();
    commands.entity(grid).add_child(ground);

    // Four batched meshes replace 322 Cuboid entities/draw calls. The grid is
    // a diagnostic overlay, not a GTA collision surface; batching does not
    // alter gameplay or the authored obstacle geometry.
    let mut minor_coordinates = Vec::new();
    let mut major_coordinates = Vec::new();
    for coordinate in (-GRID_HALF_EXTENT..=GRID_HALF_EXTENT).step_by(GRID_STEP as usize) {
        if coordinate == 0 {
            continue;
        }
        if coordinate % 25 == 0 {
            major_coordinates.push(coordinate as f32);
        } else {
            minor_coordinates.push(coordinate as f32);
        }
    }
    let grid_batches = [
        (
            grid_lines_mesh(&minor_coordinates, 0.040, 0.006, true, true),
            minor_material,
            "Minor grid lines",
        ),
        (
            grid_lines_mesh(&major_coordinates, 0.060, 0.008, true, true),
            major_material,
            "Major grid lines",
        ),
        (
            grid_lines_mesh(&[0.0], 0.075, 0.010, true, false),
            axis_z_material,
            "Grid Z axis",
        ),
        (
            grid_lines_mesh(&[0.0], 0.075, 0.010, false, true),
            axis_x_material,
            "Grid X axis",
        ),
    ];
    for (mesh, material, name) in grid_batches {
        let lines = commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                GlobalTransform::default(),
                layer.clone(),
                Name::new(name),
            ))
            .id();
        commands.entity(grid).add_child(lines);
    }

    let player_position = player_dto.position().to_bevy();
    let player = commands
        .spawn((
            ArenaPlayer {
                horizontal_velocity: Vec2::ZERO,
                vertical_velocity: 0.0,
                grounded: true,
                previous_position: player_position,
            },
            player_dto.into_bundle(),
            Transform::from_translation(player_position),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new("Arena Test Player"),
        ))
        .id();
    commands.entity(scene_root).add_child(player);
    arena.player_entity = Some(player);
    spawn_character_visual(
        commands,
        player,
        0,
        crate::catalog::CHARACTERS[arena.selected_character].model_index,
        file_loader,
        texture_cache,
        &mut meshes,
        &mut gta_materials,
        &layer,
    );

    for local_player in 0..2u8 {
        commands.spawn((
            ArenaOwned,
            ArenaCamera { local_player },
            Camera3d::default(),
            Camera {
                order: 100 + local_player as isize,
                is_active: local_player == 0,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.025, 0.03, 0.04)),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 55.0_f32.to_radians(),
                near: 0.05,
                far: 2_500.0,
                ..default()
            }),
            // RenderWare supplies already-authored sRGB colours.  Do not run
            // Bevy's default filmic operator (which requires the optional LUT
            // feature and otherwise clears the view magenta).
            Tonemapping::None,
            DebandDither::Disabled,
            Msaa::Off,
            Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::Y, Vec3::Y),
            GlobalTransform::default(),
            layer.clone(),
            Name::new(format!(
                "Developer Arena Player {} Viewport Camera",
                local_player + 1
            )),
        ));
    }

    // re3 draws ped/object shadows as projected textured geometry in
    // CShadows::RenderStoredShadows; it does not render a cascaded PBR shadow
    // map. Arena DFFs receive the same ambient/directional RenderWare terms in
    // rw_unlit.wesl, while the diagnostic grid is deliberately unlit above.
}

/// Replace the diagnostic grid with the streamed original GTA III district
/// around `destination`, while keeping the developer arena cameras, controls
/// and UI active. This uses the arena's isolated CFileLoader/CStreaming pair;
/// production `GtaAppState`, MAIN.SCM and the frontend are never touched.
pub(crate) fn replace_arena_world_map(
    commands: &mut Commands,
    scene_root: Entity,
    destination: CVector,
    map_roots: &Query<Entity, With<ArenaWorldMap>>,
    grids: &mut Query<&mut Visibility, With<ArenaGrid>>,
    original: &mut ArenaOriginalRuntime,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
) -> Result<(usize, f32), String> {
    let data = original.ensure_loaded()?;
    data.file_loader
        .load_scene_streaming(
            &mut data.streaming,
            &mut data.before_rw.cd_stream,
            [destination.x, destination.y, destination.z],
        )
        .map_err(|error| format!("GTA district streaming failed: {error}"))?;
    sync_world_texture_cache(&data.file_loader, texture_cache, images);

    let ground_z = data
        .file_loader
        .world
        .find_ground_z_for_3d_coord(
            destination.x,
            destination.y,
            destination.z + 100.0,
            &data.file_loader.model_info,
        )
        .ok_or_else(|| {
            format!(
                "no original GTA COL ground at ({:.1}, {:.1}, {:.1})",
                destination.x, destination.y, destination.z
            )
        })?;

    // RenderWare's LoadScene replaces its loaded RW objects at the destination.
    // The arena mirrors that ownership boundary by replacing only its prior
    // world-map subtree; the grid/player/panel remain the same arena session.
    for root in map_roots.iter() {
        commands.entity(root).despawn();
    }
    for mut visibility in grids {
        *visibility = Visibility::Hidden;
    }

    let map_root = commands
        .spawn((
            ArenaWorldMap,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new("Developer Arena Original GTA Map"),
        ))
        .id();
    commands.entity(scene_root).add_child(map_root);

    let mut candidates = data
        .file_loader
        .world
        .entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| arena_world_entity_in_range(entity, destination))
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|(_, left), (_, right)| {
        arena_world_entity_distance_squared(left, destination)
            .total_cmp(&arena_world_entity_distance_squared(right, destination))
    });

    let mut rendered = 0;
    for (entity_index, entity) in candidates {
        if spawn_arena_world_entity(
            commands,
            map_root,
            entity_index,
            entity,
            &data.file_loader,
            texture_cache,
            meshes,
            materials,
            arena_world_entity_distance_squared(entity, destination).sqrt(),
        ) {
            rendered += 1;
        }
    }
    Ok((rendered, ground_z))
}

/// Isolated equivalent of re3 `CStreaming::Update` plus the renderer's moving
/// sector scan. `LoadScene` seeds the first frame; this system continuously
/// requests DFF/TXD data around every active Arena camera and incrementally
/// updates the IPL subtree as those real assets become resident.
pub(crate) fn update_arena_world_streaming(
    mut commands: Commands,
    mut original: ResMut<ArenaOriginalRuntime>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    map_roots: Query<Entity, With<ArenaWorldMap>>,
    existing: Query<(Entity, &ArenaWorldMapEntity)>,
    cameras: Query<(&Camera, &GlobalTransform, &ArenaCamera)>,
) {
    let Ok(map_root) = map_roots.single() else {
        return;
    };
    let Some(data) = original.data.as_mut() else {
        return;
    };
    let mut positions = cameras
        .iter()
        .filter(|(camera, _, _)| camera.is_active)
        .map(|(_, transform, marker)| {
            let position = CVector::from_bevy(transform.translation());
            (marker.local_player, [position.x, position.y, position.z])
        })
        .collect::<Vec<_>>();
    positions.sort_unstable_by_key(|(local_player, _)| *local_player);
    let positions = positions
        .into_iter()
        .map(|(_, position)| position)
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return;
    }

    if let Err(error) = data.file_loader.update_streaming_around_positions(
        &mut data.streaming,
        &mut data.before_rw.cd_stream,
        &positions,
        // The dev arena has no subway gameplay; keep streaming to cameras.
        None,
    ) {
        warn!("Arena CStreaming::Update failed: {error}");
        return;
    }
    sync_world_texture_cache(&data.file_loader, &mut texture_cache, &mut images);

    let gta_positions = positions
        .iter()
        .map(|position| CVector::new(position[0], position[1], position[2]))
        .collect::<Vec<_>>();
    let primary = gta_positions[0];
    let lod_multiplier = 1.0;
    let render_inputs = data
        .file_loader
        .world
        .entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| {
            let nearest_camera = gta_positions
                .iter()
                .copied()
                .min_by(|left, right| {
                    arena_world_entity_distance_squared(entity, *left)
                        .total_cmp(&arena_world_entity_distance_squared(entity, *right))
                })
                .unwrap_or(primary);
            let entity_snapshot = world_entity_snapshot(entity, &data.file_loader.model_info);
            let model_snapshot = world_model_snapshot(
                entity.model_index,
                entity.position,
                &data.file_loader.model_info,
                nearest_camera,
                lod_multiplier,
                entity.is_big_building,
            )?;
            Some((index, entity_snapshot, model_snapshot))
        })
        .collect::<Vec<_>>();
    let mut plan = rust_gta_3_game::renderer::Renderer::RendererPassPlan::default();
    let current_hour = data.file_loader.clock.hours();
    data.file_loader
        .renderer
        .begin_construct_render_list_for_cameras(&gta_positions, current_hour, lod_multiplier);
    data.file_loader
        .renderer
        .construct_render_pass_plan_after_begin(
            render_inputs,
            &data.file_loader.world.entities,
            &mut plan,
        );
    if !plan.stream_requests.is_empty() {
        data.file_loader.request_models_for_runtime(
            &mut data.streaming,
            plan.stream_requests.iter().copied(),
            0,
        );
    }
    let desired = plan
        .roads
        .iter()
        .chain(plan.everything_bar_roads.iter())
        .chain(plan.fading_in_entities.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    let current = existing
        .iter()
        .map(|(entity, marker)| (marker.entity_index, (entity, marker.lod_index)))
        .collect::<BTreeMap<_, _>>();

    for (entity_index, (bevy_entity, current_lod_index)) in &current {
        if !desired.contains(entity_index) {
            commands.entity(*bevy_entity).despawn();
            continue;
        }
        let Some(world_entity) = data.file_loader.world.entities.get(*entity_index) else {
            commands.entity(*bevy_entity).despawn();
            continue;
        };
        let distance = nearest_arena_camera_distance(world_entity, &gta_positions);
        if arena_world_lod_index(&data.file_loader, world_entity.model_index, distance)
            != *current_lod_index
        {
            commands.entity(*bevy_entity).despawn();
        }
    }
    for entity_index in desired {
        let Some(entity) = data.file_loader.world.entities.get(entity_index) else {
            continue;
        };
        let distance = nearest_arena_camera_distance(entity, &gta_positions);
        let lod_index = arena_world_lod_index(&data.file_loader, entity.model_index, distance);
        if current
            .get(&entity_index)
            .is_some_and(|(_, current_lod_index)| *current_lod_index == lod_index)
        {
            continue;
        }
        spawn_arena_world_entity(
            &mut commands,
            map_root,
            entity_index,
            entity,
            &data.file_loader,
            &texture_cache,
            &mut meshes,
            &mut materials,
            distance,
        );
    }
}

fn spawn_arena_world_entity(
    commands: &mut Commands,
    map_root: Entity,
    entity_index: usize,
    entity: &CWorldEntity,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    model_distance: f32,
) -> bool {
    let lod_index = arena_world_lod_index(file_loader, entity.model_index, model_distance);
    let object = commands
        .spawn((
            ArenaWorldMapEntity {
                entity_index,
                lod_index,
            },
            arena_world_entity_transform(entity),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!(
                "Arena GTA Map {} #{}",
                entity.name, entity.model_index
            )),
        ))
        .id();
    commands.entity(map_root).add_child(object);
    if spawn_prop_visual(
        commands,
        object,
        entity.model_index,
        Some(file_loader),
        texture_cache,
        meshes,
        materials,
        &RenderLayers::layer(ARENA_RENDER_LAYER),
        model_distance,
    ) {
        true
    } else {
        commands.entity(object).despawn();
        false
    }
}

fn nearest_arena_camera_distance(entity: &CWorldEntity, cameras: &[CVector]) -> f32 {
    cameras
        .iter()
        .copied()
        .map(|camera| arena_world_entity_distance_squared(entity, camera))
        .min_by(f32::total_cmp)
        .unwrap_or(0.0)
        .sqrt()
}

fn arena_world_lod_index(
    file_loader: &CFileLoader,
    model_index: usize,
    distance: f32,
) -> Option<usize> {
    file_loader
        .model_info
        .simple_model_info(model_index)
        .and_then(|model| {
            model
                .atomic_binding_from_distance(
                    distance,
                    file_loader.renderer.lod_distance_multiplier(),
                    false,
                )
                .map(|binding| binding.lod_index)
        })
}

/// Restore the diagnostic grid without leaving the arena.
pub(crate) fn clear_arena_world_map(
    commands: &mut Commands,
    map_roots: &Query<Entity, With<ArenaWorldMap>>,
    grids: &mut Query<&mut Visibility, With<ArenaGrid>>,
) {
    for root in map_roots.iter() {
        commands.entity(root).despawn();
    }
    for mut visibility in grids {
        *visibility = Visibility::Visible;
    }
}

fn arena_world_entity_distance_squared(entity: &CWorldEntity, destination: CVector) -> f32 {
    let dx = entity.position[0] - destination.x;
    let dy = entity.position[1] - destination.y;
    dx * dx + dy * dy
}

fn arena_world_entity_in_range(entity: &CWorldEntity, destination: CVector) -> bool {
    if !entity.is_visible || entity.model_index == 0 {
        return false;
    }
    let radius = if entity.is_big_building {
        ARENA_WORLD_BIG_BUILDING_RADIUS
    } else {
        ARENA_WORLD_RENDER_RADIUS
    };
    arena_world_entity_distance_squared(entity, destination) <= radius * radius
}

/// Same IPL instance transform re3 uses for an RW frame. IPL's scale triplet
/// remains intentionally unused: CFileLoader::LoadObjectInstance parses it
/// but does not apply it to the RenderWare matrix.
fn arena_world_entity_transform(entity: &CWorldEntity) -> Transform {
    let translation = CVector::new(
        entity.position[0] + entity.render_position_offset[0],
        entity.position[1] + entity.render_position_offset[1],
        entity.position[2] + entity.render_position_offset[2],
    )
    .to_bevy();
    let axis = CVector::new(
        entity.rotation_axis[0],
        entity.rotation_axis[1],
        entity.rotation_axis[2],
    )
    .to_bevy();
    let rotation = if axis.length_squared() > f32::EPSILON {
        Quat::from_axis_angle(axis.normalize(), entity.rotation_degrees.to_radians())
    } else {
        Quat::IDENTITY
    };
    Transform {
        translation,
        rotation,
        scale: Vec3::ONE,
    }
}

pub(crate) fn spawn_character_visual(
    commands: &mut Commands,
    parent: Entity,
    local_player: u8,
    model_index: usize,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    layer: &RenderLayers,
) -> bool {
    let visual = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!("Character model {model_index}")),
        ))
        .id();
    commands.entity(parent).add_child(visual);
    let loaded_dff = spawn_loaded_dff_parts(
        commands,
        visual,
        model_index,
        file_loader,
        texture_cache,
        meshes,
        materials,
        layer,
        true,
        local_player,
        None,
        VehicleColourRemap::default(),
        0.0,
    );
    commands.entity(visual).insert(ArenaCharacterVisual {
        local_player,
        model_index,
        loaded_dff,
    });
    if loaded_dff {
        return true;
    }
    false
}

pub(crate) fn spawn_vehicle_visual(
    commands: &mut Commands,
    parent: Entity,
    model_index: usize,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    layer: &RenderLayers,
    colours: VehicleColourRemap,
) -> bool {
    let visual = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!("Vehicle model {model_index}")),
        ))
        .id();
    commands.entity(parent).add_child(visual);
    if spawn_loaded_dff_parts(
        commands,
        visual,
        model_index,
        file_loader,
        texture_cache,
        meshes,
        materials,
        layer,
        false,
        0,
        Some(parent),
        colours,
        0.0,
    ) {
        return true;
    }

    false
}

pub(crate) fn spawn_prop_visual(
    commands: &mut Commands,
    parent: Entity,
    model_index: usize,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    layer: &RenderLayers,
    model_distance: f32,
) -> bool {
    let visual = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!("Prop model {model_index}")),
        ))
        .id();
    commands.entity(parent).add_child(visual);
    if spawn_loaded_dff_parts(
        commands,
        visual,
        model_index,
        file_loader,
        texture_cache,
        meshes,
        materials,
        layer,
        false,
        0,
        None,
        VehicleColourRemap::default(),
        model_distance,
    ) {
        return true;
    }
    false
}

fn spawn_loaded_dff_parts(
    commands: &mut Commands,
    parent: Entity,
    model_index: usize,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    layer: &RenderLayers,
    animated_character: bool,
    local_player: u8,
    vehicle_owner: Option<Entity>,
    vehicle_colours: VehicleColourRemap,
    model_distance: f32,
) -> bool {
    let Some(file_loader) = file_loader else {
        return false;
    };
    let (drawables, rigid_frame_indices) = if animated_character {
        build_rigid_ped_material_drawables(model_index, file_loader).map_or_else(
            || {
                (
                    build_model_material_drawables(
                        model_index,
                        model_distance,
                        false,
                        file_loader,
                        vehicle_colours,
                    ),
                    None,
                )
            },
            |(drawables, frame_indices)| (drawables, Some(frame_indices)),
        )
    } else {
        (
            build_model_material_drawables(
                model_index,
                model_distance,
                false,
                file_loader,
                vehicle_colours,
            ),
            None,
        )
    };
    if drawables.is_empty() {
        return false;
    }
    for drawable in &drawables {
        if let Some(name) = drawable.texture_name.as_deref().map(str::trim)
            && !name.is_empty() && !name.starts_with('@')
            && arena_texture_bind_for_drawable(drawable, file_loader, texture_cache).is_none() {
            warn_once!(model_index, texture = name, "arena_drawable_waiting_for_original_txd_texture");
            return false;
        }
    }
    // CVehicleModelInfo's '@' paint remaps and original untextured materials
    // intentionally use authored colour; named TXD textures are required above.
    let prepared = drawables
        .into_iter()
        .enumerate()
        .map(|(part_index, drawable)| {
            let (texture, texture_alpha) =
                arena_texture_bind_for_drawable(&drawable, file_loader, texture_cache).map_or(
                    (None, RwTextureAlphaMode::Opaque),
                    |(handle, alpha_mode)| (Some(handle), alpha_mode),
                );
            PreparedArenaDrawable {
                drawable,
                texture,
                texture_alpha,
                source_part_index: part_index,
                source_part_indices: vec![part_index],
                attach_vehicle_part: vehicle_owner.is_some(),
            }
        })
        .collect::<Vec<_>>();

    let mut character_parts = Vec::with_capacity(if animated_character { prepared.len() } else { 0 });
    for prepared in prepared {
        let PreparedArenaDrawable {
            drawable,
            texture,
            texture_alpha,
            source_part_index,
            source_part_indices,
            attach_vehicle_part,
        } = prepared;
        let dynamic_vehicle_component = drawable.dynamic_vehicle_component;
        let component_baseline = drawable.vehicle_component_transform;
        // CPed/CVehicle::SetupLighting activates the directional light and
        // uses the 1.3 ambient colour maintained by Lights.cpp. Static
        // CEntity props use ambient only. Keep this on a separate arena
        // buffer so pausing/resuming GTA cannot invalidate production light
        // state.
        let vehicle_or_ped = animated_character || vehicle_owner.is_some();
        let mut material = GtaUnlitMaterial::from_gta_lit(
            drawable.base_color,
            texture,
            texture_alpha,
            false,
            [0.0, 0.0, 0.0],
            2_500.0,
            2_500.0,
            [0.0; 3],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            drawable.surface_ambient,
            drawable.surface_diffuse,
            drawable.has_prelit,
            if vehicle_or_ped { 1.3 } else { 1.0 },
            vehicle_or_ped,
        );
        material.global_lighting = ARENA_LIGHTING_BUFFER.clone();
        let material = materials.add(material);
        let part = commands
            .spawn((
                ArenaModelPart,
                Mesh3d(meshes.add(drawable.mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                GlobalTransform::default(),
                layer.clone(),
            ))
            .id();
        if animated_character {
            character_parts.push(part);
            commands.entity(part).insert(ArenaCharacterModelPart {
                local_player,
                part_index: source_part_index,
                rigid_frame_index: rigid_frame_indices
                    .as_ref()
                    .and_then(|indices| indices.get(source_part_index).copied()),
            });
        }
        if let Some(vehicle) = vehicle_owner
            && attach_vehicle_part
        {
            commands.entity(part).insert(ArenaVehicleModelPart {
                vehicle,
                dynamic: dynamic_vehicle_component,
                component_baseline,
                source_part_indices,
            });
        }
        commands.entity(parent).add_child(part);
    }
    if animated_character {
        commands.entity(parent).insert(crate::ArenaCharacterPoseManifest {
            model_index, parts: character_parts, rigid_frame_indices,
        });
    }
    true
}

pub(crate) fn spawn_vehicle(
    commands: &mut Commands,
    scene_root: Entity,
    position: Vec3,
    model_index: usize,
    mut file_loader: Option<&mut CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    handling_asset: Option<&CHandlingDataAsset>,
    surface_table: Option<&CSurfaceTableAsset>,
    general: Option<&mut CGeneral>,
    colour_table: Option<&CVehicleColours>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
    diagnostics: &mut ArenaVehicleSpawnDiagnostics,
) -> ArenaVehicleSpawnResult {
    let runtime = build_automobile_runtime(
        model_index,
        position,
        file_loader.as_deref(),
        handling_asset,
        surface_table,
        general,
        None,
    );
    let (mut runtime, outcome) = match runtime {
        Ok(Some(runtime)) => (Some(runtime), ArenaVehicleRuntimeOutcome::Ready),
        Ok(None) => (None, ArenaVehicleRuntimeOutcome::UnsupportedKind),
        Err(error) => {
            diagnostics.record(model_index, &error);
            return ArenaVehicleSpawnResult {
                entity: None, visual_loaded: false, runtime: ArenaVehicleRuntimeOutcome::Failed(error),
            };
        }
    };
    // Advance the authored colour cursor only after required construction
    // inputs pass. A failed request must not allocate or modify an actor.
    let selected_colours = file_loader.as_deref_mut()
        .and_then(|loader| select_vehicle_colours(loader, model_index));
    if let (Some((spawn, _)), Some((first, second))) = (runtime.as_mut(), selected_colours) {
        spawn.car.vehicle.appearance.current_colour1 = first;
        spawn.car.vehicle.appearance.current_colour2 = second;
    }
    let colour_remap = vehicle_colour_remap(selected_colours, colour_table);
    let initial_transform = runtime.as_ref()
        .map(|(runtime, _)| automobile_transform(&runtime.car.as_ref()))
        .unwrap_or_else(|| Transform::from_translation(position));
    let vehicle = commands
        .spawn((
            ArenaVehicle {
                model_index,
                speed: 0.0,
                steering: 0.0,
                occupied: false,
                colour_remap,
            },
            ArenaSpawned,
            initial_transform,
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!("Arena Vehicle {model_index}")),
        ))
        .id();
    commands.entity(scene_root).add_child(vehicle);
    if let Some((runtime, history)) = runtime {
        commands.entity(vehicle).insert(runtime.into_bundle(history));
    }
    let loaded = spawn_vehicle_visual(
        commands,
        vehicle,
        model_index,
        file_loader.as_deref(),
        texture_cache,
        meshes,
        materials,
        &RenderLayers::layer(ARENA_RENDER_LAYER),
        colour_remap,
    );
    ArenaVehicleSpawnResult { entity: Some(vehicle), visual_loaded: loaded, runtime: outcome }
}

fn build_automobile_runtime(
    model_index: usize,
    position: Vec3,
    file_loader: Option<&CFileLoader>,
    handling_asset: Option<&CHandlingDataAsset>,
    surface_table: Option<&CSurfaceTableAsset>,
    general: Option<&mut CGeneral>,
    colours: Option<(u8, u8)>,
) -> Result<Option<(crate::ArenaAutomobileSpawn, rust_gta_3_game::vehicles::Automobile::VehicleCollisionState)>, ArenaVehicleBuildError> {
    let file_loader = file_loader.ok_or_else(|| ArenaVehicleBuildError::new(
        ArenaVehicleInput::ModelLoader, "installed model loader is unavailable"))?;
    let info = file_loader.model_info.get_model_info(model_index).ok_or_else(||
        ArenaVehicleBuildError::new(ArenaVehicleInput::ModelInfo, format!("model {model_index}")))?;
    let ModelInfoSlot::Vehicle(model) = info else {
        return Ok(None);
    };
    // Boats, trains, helicopters and planes have their own re3 control
    // classes. Never wrap them in CAutomobile merely because they also have a
    // HANDLING.CFG identifier.
    if model.vehicle_type != eVehicleType::VEHICLE_TYPE_CAR as i32 {
        return Ok(None);
    }
    let handling_asset = handling_asset.ok_or_else(|| ArenaVehicleBuildError::new(
        ArenaVehicleInput::HandlingData, "HANDLING.CFG is unavailable"))?;
    let general = general.ok_or_else(|| ArenaVehicleBuildError::new(
        ArenaVehicleInput::RandomState, "native random manager is unavailable"))?;
    let surface_table = surface_table.ok_or_else(|| ArenaVehicleBuildError::new(
        ArenaVehicleInput::SurfaceData, "SURFACE.DAT is unavailable"))?;
    surface_table.data.validate().map_err(|error|
        ArenaVehicleBuildError::new(ArenaVehicleInput::SurfaceData, error))?;
    let wheel_scale = model.wheel_scale;
    let handling_id = cHandlingDataMgr::get_handling_id(&model.handling_id).ok_or_else(||
        ArenaVehicleBuildError::new(ArenaVehicleInput::HandlingIdentity, &model.handling_id))?;
    if !handling_asset.data.records.iter().any(|record| record.name == model.handling_id) {
        return Err(ArenaVehicleBuildError::new(ArenaVehicleInput::HandlingRecord, &model.handling_id));
    }
    let mut manager = cHandlingDataMgr::new();
    manager.initialise_from_cfg(&handling_asset.data).map_err(|error|
        ArenaVehicleBuildError::new(ArenaVehicleInput::HandlingData, error))?;
    let handling = *manager.get_handling_data(handling_id).ok_or_else(||
        ArenaVehicleBuildError::new(ArenaVehicleInput::HandlingRecord, &model.handling_id))?;
    let surface_adhesion = rust_gta_3_game::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                &surface_table.data,
                rust_gta_3_game::core::SurfaceTable::eSurfaceType::SURFACE_TARMAC as u8,
                rust_gta_3_game::core::SurfaceTable::eSurfaceType::SURFACE_WHEELBASE as u8,
            );
    model.clump.base.clump_summary.as_ref().ok_or_else(||
        ArenaVehicleBuildError::new(ArenaVehicleInput::ModelClump, format!("original DFF for model {model_index}")))?;
    if file_loader.model_info.col_model(model_index).is_none() {
        return Err(ArenaVehicleBuildError::new(ArenaVehicleInput::ModelCollision, format!("original COL for model {model_index}")));
    }
    let wheel_positions = model.authored_positions().and_then(|descriptors| descriptors.wheels.local_positions()).ok_or_else(||
        ArenaVehicleBuildError::new(ArenaVehicleInput::Suspension, format!("original DFF wheel frames for model {model_index}")))?;
    let mut car = CAutomobile::new(
        model_index as i32,
        MISSION_VEHICLE,
        0,
        general,
        manager.automobile_snapshot(handling_id),
        colours,
        false,
    );
    car.setup_suspension_lines(wheel_positions, wheel_scale, &handling);
    let mut gta_position = CVector::from_bevy(position);
    // re3 places the vehicle matrix origin at `m_fHeightAboveRoad`; the DFF
    // origin is not the tyre contact plane. Spawning at grid Y=0 submerged the
    // body and made its wheel torques act around the road surface.
    gta_position.z += car.height_above_road;
    car.vehicle.physical.entity.set_position(gta_position);
    car.vehicle
        .physical
        .entity
        .set_status(eEntityStatus::STATUS_ABANDONED);
    car.vehicle.drive.engine_on = true;
    car.vehicle.flags.is_locked = false;
    // CAutomobile's constructor/ResetSuspension starts with no wheel contact.
    // The first real COL raycasts populate timers and spring compression.
    // Pretending all wheels were grounded while the car was still airborne
    // applied tyre/brake impulses at non-existent contacts and launched it.
    let history = rust_gta_3_game::vehicles::Automobile::VehicleCollisionState {
        previous_matrix: car.vehicle.physical.entity.matrix,
    };
    Ok(Some((crate::ArenaAutomobileSpawn {
        car,
        settings: crate::ArenaVehicleSettings {
        handling,
        wheel_scale,
        wheel_friction: manager.wheel_friction,
        surface_adhesion,
        },
    }, history)))
}

fn automobile_transform(car: &rust_gta_3_game::vehicles::Automobile::AutomobileRef<'_>) -> Transform {
    let matrix = &car.vehicle.physical.entity.matrix;
    let right = matrix.right.to_bevy().normalize_or_zero();
    let up = matrix.up.to_bevy().normalize_or_zero();
    let back = (-matrix.forward.to_bevy()).normalize_or_zero();
    let rotation = if right.length_squared() > f32::EPSILON
        && up.length_squared() > f32::EPSILON
        && back.length_squared() > f32::EPSILON
    {
        Quat::from_mat3(&Mat3::from_cols(right, up, back)).normalize()
    } else {
        Quat::IDENTITY
    };
    Transform {
        translation: matrix.position.to_bevy(),
        rotation,
        scale: Vec3::ONE,
    }
}

/// Refresh only the live component meshes that re3 mutates from
/// `CAutomobile::Render`. The body and materials stay resident; wheel, door
/// and damage atomics are rebuilt from the same production drawable adapter
/// at GTA III's 30 Hz render cadence.
pub(crate) fn update_arena_vehicle_visuals(
    mut commands: Commands,
    time: Res<Time>,
    original: Res<ArenaOriginalRuntime>,
    texture_cache: Res<GtaWorldTextureCache>,
    vehicles: Query<(Entity, &ArenaVehicle, crate::vehicle_access::ArenaAutomobileRuntimeReadOnly)>,
    mut parts: Query<(
        Entity,
        &ChildOf,
        &mut ArenaVehicleModelPart,
        &Mesh3d,
        &mut Transform,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut accumulator: Local<f32>,
    mut last_resident_states: Local<HashMap<Entity, VehicleResidentMeshState>>,
) {
    // `CAutomobile::Render` updates every visible car, including abandoned
    // and collision-pushed vehicles. Their wheels, suspension and damaged
    // atomics must not freeze merely because no local player occupies them.
    let live_vehicles = vehicles
        .iter()
        .map(|(entity, _, _)| entity)
        .collect::<Vec<_>>();
    *accumulator += time.delta_secs().max(0.0);
    let interval = 1.0 / 30.0;
    if *accumulator < interval {
        return;
    }
    *accumulator %= interval;

    let Some(data) = original.data.as_ref() else {
        return;
    };
    last_resident_states.retain(|entity, _| live_vehicles.contains(entity));

    for live_vehicle in live_vehicles {
        let Ok((vehicle_entity, vehicle, runtime)) = vehicles.get(live_vehicle) else {
            continue;
        };
        let runtime = runtime.into_view();
        let remap = vehicle
            .colour_remap
            .with_automobile_render_state(&runtime.car);
        let resident_state = remap.resident_mesh_state();
        let resident_changed = last_resident_states
            .get(&vehicle_entity)
            .is_some_and(|previous| *previous != resident_state);

        // Damage, a missing wheel, or a paint remap can change the resident
        // atomics. This branch is event-driven; wheel rotation/steering and
        // suspension never enter it.
        if resident_changed {
            let visual_parent = parts
                .iter()
                .find(|(_, _, part, _, _)| part.vehicle == vehicle_entity)
                .map(|(_, parent, _, _, _)| parent.parent());
            if let Some(visual_parent) = visual_parent {
                // A damage transition can change geometry, material order and
                // topology (notably missing wheels). Replacing only Mesh3d by
                // the old source index leaves the previous texture handle on
                // the new `_dam` atomic and leaves removed wheels resident.
                // Rebuild this vehicle's RW clump atomically, just as re3
                // swaps resident atomics as a unit.
                for (part_entity, _, part, _, _) in &mut parts {
                    if part.vehicle == vehicle_entity {
                        commands.entity(part_entity).despawn();
                    }
                }
                spawn_loaded_dff_parts(
                    &mut commands,
                    visual_parent,
                    vehicle.model_index,
                    Some(&data.file_loader),
                    &texture_cache,
                    &mut meshes,
                    &mut materials,
                    &RenderLayers::layer(ARENA_RENDER_LAYER),
                    false,
                    0,
                    Some(vehicle_entity),
                    remap,
                    0.0,
                );
                last_resident_states.insert(vehicle_entity, resident_state);
                continue;
            }
        }
        last_resident_states.insert(vehicle_entity, resident_state);

        let Some(component_transforms) =
            build_vehicle_component_transforms(vehicle.model_index, 0.0, &data.file_loader, remap)
        else {
            continue;
        };
        for (_, _, part, _, mut part_transform) in &mut parts {
            if part.vehicle != vehicle_entity || !part.dynamic {
                continue;
            }
            let [source_index] = part.source_part_indices.as_slice() else {
                continue;
            };
            let Some((_, Some(current))) = component_transforms
                .iter()
                .find(|(index, _)| index == source_index)
            else {
                continue;
            };
            let Some(baseline) = part.component_baseline else {
                continue;
            };
            let desired = vehicle_component_relative_transform(baseline, *current);
            if *part_transform != desired {
                *part_transform = desired;
            }
        }
    }
}

fn select_vehicle_colours(file_loader: &mut CFileLoader, model_index: usize) -> Option<(u8, u8)> {
    let ModelInfoSlot::Vehicle(model) = file_loader.model_info.get_model_info_mut(model_index)?
    else {
        return None;
    };
    // re3 `CAutomobile` calls `CVehicleModelInfo::ChooseVehicleColour` during
    // construction, advancing the model's variation cursor.
    Some(model.choose_vehicle_colour(None))
}

fn vehicle_colour_remap(
    indices: Option<(u8, u8)>,
    table: Option<&CVehicleColours>,
) -> VehicleColourRemap {
    let Some((colour1, colour2)) = indices else {
        return VehicleColourRemap::default();
    };
    let rgba = |index: u8| {
        table
            .and_then(|table| table.vehicle_colour_table.get(index as usize))
            .map(|colour| [colour.red, colour.green, colour.blue, colour.alpha])
    };
    VehicleColourRemap {
        colour1: rgba(colour1),
        colour2: rgba(colour2),
        colour1_index: Some(colour1),
        colour2_index: Some(colour2),
        ..VehicleColourRemap::default()
    }
}

pub(crate) fn spawn_prop(
    commands: &mut Commands,
    scene_root: Entity,
    position: Vec3,
    model_index: usize,
    file_loader: Option<&CFileLoader>,
    texture_cache: &GtaWorldTextureCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GtaUnlitMaterial>,
) -> (Entity, bool) {
    let mut position = position;
    // GTA object origins are not guaranteed to sit on the road plane.  Use
    // the authored COL bounding box when available so a spawned mesh rests on
    // its actual lowest collision point instead of being half buried.
    if let Some(min_y) = file_loader.and_then(|loader| model_visual_min_y(loader, model_index)) {
        position.y -= min_y;
    } else if let Some(min_z) = file_loader
        .and_then(|loader| loader.model_info.col_model(model_index))
        .map(|col| col.bounding_box.min.z)
    {
        position.y -= min_z;
    }
    let prop = commands
        .spawn((
            crate::ArenaProp { model_index },
            ArenaSpawned,
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::Visible,
            Name::new(format!("Arena Prop {model_index}")),
        ))
        .id();
    commands.entity(scene_root).add_child(prop);
    let loaded = spawn_prop_visual(
        commands,
        prop,
        model_index,
        file_loader,
        texture_cache,
        meshes,
        materials,
        &RenderLayers::layer(ARENA_RENDER_LAYER),
        0.0,
    );
    (prop, loaded)
}

fn model_visual_min_y(file_loader: &CFileLoader, model_index: usize) -> Option<f32> {
    build_model_material_drawables(
        model_index,
        0.0,
        false,
        file_loader,
        VehicleColourRemap::default(),
    )
    .into_iter()
    .filter_map(
        |drawable| match drawable.mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(positions)) => positions
                .iter()
                .map(|position| position[1])
                .min_by(f32::total_cmp),
            _ => None,
        },
    )
    .min_by(f32::total_cmp)
}

pub(crate) fn focus_position<PF: QueryFilter, VF: QueryFilter>(
    arena: &DeveloperArena,
    player: &Query<&Transform, PF>,
    vehicles: &Query<&Transform, VF>,
) -> Vec3 {
    if let Some(vehicle) = arena.controlled_vehicle
        && let Ok(transform) = vehicles.get(vehicle)
    {
        return transform.translation;
    }
    player
        .iter()
        .next()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO)
}

pub(crate) fn view_forward(arena: &DeveloperArena) -> Vec3 {
    Vec3::new(-arena.camera_yaw.sin(), 0.0, -arena.camera_yaw.cos()).normalize_or_zero()
}

pub(crate) fn update_grid_origin(
    arena: Res<DeveloperArena>,
    player: Query<&Transform, (With<ArenaPlayer>, Without<ArenaGrid>)>,
    vehicles: Query<&Transform, (With<ArenaVehicle>, Without<ArenaGrid>)>,
    mut grid: Query<&mut Transform, With<ArenaGrid>>,
) {
    let focus = focus_position(&arena, &player, &vehicles);
    let snapped = Vec3::new(
        (focus.x / 50.0).round() * 50.0,
        0.0,
        (focus.z / 50.0).round() * 50.0,
    );
    for mut transform in &mut grid {
        transform.translation = snapped;
    }
}

pub(crate) fn update_arena_camera(
    arena: Res<DeveloperArena>,
    player: Query<&Transform, (With<ArenaPlayer>, Without<ArenaCamera>)>,
    player_two: Query<
        (&Transform, &ArenaCoopPlayerTwo),
        (With<ArenaCoopPlayerTwo>, Without<ArenaCamera>),
    >,
    vehicles: Query<&Transform, (With<ArenaVehicle>, Without<ArenaCamera>)>,
    mut cameras: Query<(&ArenaCamera, &mut Transform)>,
) {
    let player_one_focus = focus_position(&arena, &player, &vehicles) + Vec3::Y * 1.35;
    let player_two_state = player_two.single().ok().map(|(transform, player)| {
        (
            transform.translation + Vec3::Y * 1.35,
            player.camera_yaw,
            player.camera_pitch,
            player.camera_distance,
        )
    });
    for (camera, mut transform) in &mut cameras {
        let (focus, yaw, pitch, distance) = if camera.local_player == 1 {
            player_two_state.unwrap_or((
                player_one_focus,
                arena.camera_yaw,
                arena.camera_pitch,
                arena.camera_distance,
            ))
        } else {
            (
                player_one_focus,
                arena.camera_yaw,
                arena.camera_pitch,
                arena.camera_distance,
            )
        };
        let horizontal = pitch.cos() * distance;
        let offset = Vec3::new(
            yaw.sin() * horizontal,
            pitch.sin() * distance + 0.8,
            yaw.cos() * horizontal,
        );
        let eye = focus + offset;
        let direction = (focus - eye).normalize_or(Vec3::NEG_Z);
        // A fully unbounded orbit crosses both poles. `looking_at(..., Y)` is
        // singular exactly there, so select a stable orthogonal up vector
        // instead of reintroducing a hidden pitch clamp.
        let up = if direction.dot(Vec3::Y).abs() > 0.999 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        *transform = Transform::from_translation(eye).looking_at(focus, up);
    }
}

pub(crate) fn update_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    session: Res<LocalCoopSession>,
    mut cameras: Query<(&ArenaCamera, &mut Camera, &mut Projection)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical_width = window.physical_width();
    let physical_height = window.physical_height().max(2);
    if physical_width == 0 || window.physical_height() == 0 {
        for (marker, mut camera, _) in &mut cameras {
            camera.viewport = None;
            if marker.local_player != 0 {
                camera.is_active = false;
            }
        }
        return;
    }
    let primary_target_size = cameras
        .iter()
        .find(|(marker, _, _)| marker.local_player == 0)
        .and_then(|(_, camera, _)| camera.physical_target_size())
        .filter(|size| size.x > 0 && size.y > 0);
    let target_size =
        primary_target_size.unwrap_or_else(|| UVec2::new(physical_width, physical_height));
    let effective_width = physical_width.min(target_size.x);
    let effective_height = physical_height.min(target_size.y).max(2);
    let panel_width = (ARENA_PANEL_WIDTH * window.scale_factor()) as u32;
    let width = effective_width.saturating_sub(panel_width).max(1);
    let origin_x = panel_width.min(effective_width.saturating_sub(1));
    let split = session.enabled && session.player_two_spawned;
    for (marker, mut camera, mut projection) in &mut cameras {
        let (position, size) = if !split {
            (UVec2::new(origin_x, 0), UVec2::new(width, effective_height))
        } else {
            match session.split_layout {
                LocalCoopSplitLayout::TopBottom => {
                    let top_height = effective_height / 2;
                    if marker.local_player == 0 {
                        (UVec2::new(origin_x, 0), UVec2::new(width, top_height))
                    } else {
                        (
                            UVec2::new(origin_x, top_height),
                            UVec2::new(width, effective_height - top_height),
                        )
                    }
                }
                LocalCoopSplitLayout::LeftRight => {
                    let left_width = width / 2;
                    if marker.local_player == 0 {
                        (
                            UVec2::new(origin_x, 0),
                            UVec2::new(left_width, effective_height),
                        )
                    } else {
                        (
                            UVec2::new(origin_x + left_width, 0),
                            UVec2::new(width - left_width, effective_height),
                        )
                    }
                }
            }
        };
        camera.is_active = marker.local_player == 0 || split;
        let mut vp = Viewport {
            physical_position: position,
            physical_size: size,
            depth: 0.0..1.0,
        };
        vp.clamp_to_size(target_size);
        camera.viewport = Some(vp);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.aspect_ratio = size.x.max(1) as f32 / size.y.max(1) as f32;
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArenaParticleStage { Inputs, Effects, Consume }

pub(crate) fn sync_arena_particle_creation_inputs(
    particles: Option<ResMut<rust_gta_3_game::renderer::Particle::CParticle>>,
    timer: Option<Res<rust_gta_3_core::core::Timer::CTimer>>,
    policy: Res<rust_gta_3_game::renderer::Particle::ParticleCreationPolicy>,
) {
    let Some(mut particles) = particles else { return; };
    if let Some(timer) = timer {
        particles.set_creation_inputs(&timer, &policy,
            rust_gta_3_game::renderer::Particle::ParticleSimulationDomain::IsolatedDeveloperScene);
    } else {
        particles.invalidate_creation_inputs();
    }
}

pub(crate) fn update_arena_vehicle_effects(
    timer: Option<Res<rust_gta_3_core::core::Timer::CTimer>>,
    mut vehicles: Query<(Entity, crate::vehicle_access::ArenaAutomobileEffects)>,
    mut particles: ResMut<rust_gta_3_game::renderer::Particle::CParticle>,
    mut skidmarks: Option<ResMut<rust_gta_3_game::renderer::Skidmarks::CSkidmarks>>,
    weather: Option<Res<rust_gta_3_game::renderer::Weather::CWeather>>,
    original: Option<Res<ArenaOriginalRuntime>>,
    mut random: ResMut<crate::ArenaRandomState>,
    particle_systems: Res<rust_gta_3_game::renderer::ParticleMgr::cParticleSystemMgr>,
    cameras: Query<(&Camera, &Transform, &ArenaCamera)>,
) {
    if vehicles.is_empty() { return; }
    let Some(timer) = timer else {
        warn_once!("arena_vehicle_effects_missing_timer");
        return;
    };
    let Some(weather) = weather else {
        warn_once!("arena_vehicle_effects_missing_weather");
        return;
    };
    let Some(file_loader) = original
        .as_ref()
        .and_then(|orig| orig.data.as_ref())
        .map(|data| &data.file_loader) else {
        warn_once!("arena_vehicle_effects_missing_original_data");
        return;
    };
    let wet_roads = weather.wet_roads;
    let Some((_, camera, _)) = cameras.iter().find(|(camera, _, arena_camera)|
        camera.is_active && arena_camera.local_player == 0) else {
        warn_once!("arena_vehicle_effects_missing_primary_camera");
        return;
    };
    let mut particles = particles.emitter(&particle_systems, CVector::from_bevy(camera.translation));
    let frame_counter = timer.frame_counter();
    let time_ms = timer.time_in_milliseconds();

    for (entity, mut runtime) in &mut vehicles {
        let handling_flags = runtime.settings.handling.flags;
        rust_gta_3_game::vehicles::AutomobileEffects::prerender_wheel_effects(
            runtime.core,
            &mut runtime.state,
            file_loader,
            &mut particles,
            skidmarks.as_deref_mut(),
            entity.to_bits() as usize,
            wet_roads,
            frame_counter,
            time_ms,
            &mut random.0,
            false,
        );
        rust_gta_3_game::vehicles::AutomobileEffects::exhaust_effects(
            runtime.core,
            runtime.drive,
            handling_flags,
            file_loader,
            &mut particles,
            &mut random.0,
        );
    }
}

/// Audio lifetime maintenance must also run after clearing or closing the arena.
pub(crate) fn retire_arena_vehicle_audio(
    vehicles: Query<(Entity, &rust_gta_3_game::vehicles::Vehicle::VehicleCore),
        (With<crate::ArenaVehicleSettings>, With<rust_gta_3_game::vehicles::Automobile::AutomobileState>)>,
    dmaudio: Option<ResMut<rust_gta_3_game::audio::DMAudio::cDMAudio>>,
    mut identities: Local<HashMap<Entity, i32>>,
    mut player_engine: ResMut<rust_gta_3_game::audio::PlayerEngineAudio::PlayerEngineAudioState>,
) {
    let Some(mut dmaudio) = dmaudio else {
        if !identities.is_empty() { warn_once!("arena_audio_retirement_missing_manager"); }
        return;
    };
    identities.retain(|entity, id| {
        if vehicles.get(*entity).is_ok() { return true; }
        if dmaudio.AudioManager.m_asAudioEntities.get(*id as usize)
            .is_some_and(|slot| slot.m_bIsUsed && slot.m_pEntity == Some(entity.to_bits() as i32)) {
            if player_engine.bound_actor == Some(*entity) {
                player_engine.bind_actor(&mut dmaudio.AudioManager, None, None);
            }
            dmaudio.DestroyEntity(*id);
        }
        false
    });
    for (entity, core) in &vehicles {
        let id = core.physical.audio_entity_id;
        if id >= 0 { identities.insert(entity, id); }
    }
}

pub(crate) fn allocate_arena_vehicle_audio_identities(
    mut vehicles: Query<(Entity, &mut rust_gta_3_game::vehicles::Vehicle::VehicleCore),
        (With<crate::ArenaVehicleSettings>, With<rust_gta_3_game::vehicles::Automobile::AutomobileState>)>,
    dmaudio: Option<ResMut<rust_gta_3_game::audio::DMAudio::cDMAudio>>,
) {
    let Some(mut dmaudio) = dmaudio else {
        if !vehicles.is_empty() { warn_once!("arena_vehicle_identity_missing_audio_manager"); }
        return;
    };
    for (entity, mut core) in &mut vehicles {
        rust_gta_3_game::audio::AudioActorAccess::allocate_vehicle_audio_identity(
            &mut dmaudio, &mut core, entity.to_bits() as i32,
        );
    }
}

pub(crate) fn update_arena_vehicle_audio(
    arena: Res<DeveloperArena>,
    mut one_shots: ResMut<rust_gta_3_game::audio::AudioLogic::VehicleOneShotAudioState>,
    mut player_engine: ResMut<rust_gta_3_game::audio::PlayerEngineAudio::PlayerEngineAudioState>,
    channel: rust_gta_3_game::audio::DMAudio::PlayerEngineChannel,
    timer: Res<rust_gta_3_core::core::Timer::CTimer>,
    loader: Option<Res<CFileLoader>>,
    mut speech: ResMut<rust_gta_3_game::audio::PedTalk::PedTalkState>,
    coop_session: Option<Res<LocalCoopSession>>,
    player_two: Query<&ArenaCoopPlayerTwo>,
    dmaudio: Option<ResMut<rust_gta_3_game::audio::DMAudio::cDMAudio>>,
    mut vehicles: Query<(Entity, crate::vehicle_access::ArenaAutomobileAudio)>,
    cameras: Query<(&Camera, &Transform), With<ArenaCamera>>,
    weather: Option<Res<rust_gta_3_game::renderer::Weather::CWeather>>,
) {
    let Some(mut dmaudio) = dmaudio else {
        warn_once!("arena_vehicle_audio_missing_manager");
        return;
    };
    let Some(camera_pos) = cameras.iter().find(|(camera, _)| camera.is_active)
        .map(|(_, transform)| CVector::from_bevy(transform.translation)) else {
        warn_once!("arena_vehicle_audio_missing_listener");
        return;
    };
    let (Some(weather), Some(loader)) = (weather.as_deref(), loader.as_deref()) else {
        warn_once!("arena_vehicle_audio_missing_weather_or_world");
        return;
    };
    let wet_roads = weather.wet_roads;
    let cull = &loader.world.cull_zones;
    let rain = if !cull.camera_no_rain() || !cull.player_no_rain() { weather.rain } else { 0.0 };
    let p2_controlled = coop_session
        .filter(|session| session.enabled)
        .and_then(|_| player_two.iter().next())
        .and_then(|player| player.controlled_vehicle);

    let focused_vehicle = arena.controlled_vehicle.or(p2_controlled);
    let focused_audio = focused_vehicle.and_then(|entity| vehicles.get(entity).ok())
        .map(|(_, runtime)| runtime.core.physical.audio_entity_id).filter(|id| *id >= 0);
    player_engine.bind_actor(&mut dmaudio.AudioManager, focused_vehicle, focused_audio);

    for (entity, runtime) in &mut vehicles {
        let is_player = focused_vehicle == Some(entity);
        let handling = runtime.handling();
        let mut car = runtime.into_car();
        let channel_used = channel.is_used(car.vehicle.core.physical.audio_entity_id);
        rust_gta_3_game::audio::AudioLogic::process_single_automobile_audio(
            &mut dmaudio,
            &mut car,
            handling,
            entity.to_bits() as i32,
            camera_pos,
            wet_roads,
            rain,
            is_player,
            None,
            channel_used,
            &mut one_shots,
            &mut player_engine,
        );
    }

    rust_gta_3_game::audio::AudioLogic::flush_vehicle_comments(
        &mut dmaudio.AudioManager, &mut one_shots, &mut speech, loader,
        camera_pos, timer.time_in_milliseconds(),
    );

}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_gta_3_game::modelinfo::VehicleModelInfo::RwRGBA;

    #[test]
    fn noncar_filtering_precedes_unrelated_automobile_dependency_checks() {
        let mut loader = CFileLoader::new();
        loader.model_info.add_vehicle_model(110).vehicle_type = eVehicleType::VEHICLE_TYPE_BOAT as i32;
        assert!(matches!(build_automobile_runtime(110, Vec3::ZERO, Some(&loader),
            None, None, None, None), Ok(None)));
        let error = match build_automobile_runtime(110, Vec3::ZERO, None, None, None, None, None) {
            Err(error) => error,
            _ => panic!("missing loader must retain a named error"),
        };
        assert_eq!(error.input, ArenaVehicleInput::ModelLoader);
    }

    #[test]
    fn arena_audio_retirement_runs_without_an_active_arena_and_respects_slot_identity() {
        for replaced_owner in [false, true] {
            let mut app = App::new();
            let mut dmaudio = rust_gta_3_game::audio::DMAudio::cDMAudio::default();
            dmaudio.Initialise();
            app.insert_resource(dmaudio)
                .init_resource::<rust_gta_3_game::audio::PlayerEngineAudio::PlayerEngineAudioState>()
                .add_systems(PostUpdate, retire_arena_vehicle_audio);
            let mut general = CGeneral::default();
            let car = CAutomobile::new(rust_gta_3_game::modelinfo::ModelIndices::MI_KURUMA as i32,
                0, 0, &mut general, None, None, false);
            let collision_state = rust_gta_3_game::vehicles::Automobile::VehicleCollisionState {
                previous_matrix: car.vehicle.core.physical.entity.matrix,
            };
            let entity = app.world_mut().spawn(crate::ArenaAutomobileSpawn {
                car, settings: crate::ArenaVehicleSettings { handling: Default::default(), wheel_scale: 1.0,
                wheel_friction: 1.0, surface_adhesion: 1.0,
                },
            }.into_bundle(collision_state)).id();
            let id = {
                let mut audio = app.world_mut().resource_mut::<rust_gta_3_game::audio::DMAudio::cDMAudio>();
                let id = audio.CreateEntity(rust_gta_3_game::audio::audio_enums::AUDIOTYPE_PHYSICAL, Some(entity.to_bits() as i32));
                audio.SetEntityStatus(id, true);
                id
            };
            app.world_mut().get_mut::<rust_gta_3_game::vehicles::Vehicle::VehicleCore>(entity)
                .unwrap().physical.audio_entity_id = id;
            app.update();
            if replaced_owner {
                app.world_mut().resource_mut::<rust_gta_3_game::audio::DMAudio::cDMAudio>()
                    .AudioManager.m_asAudioEntities[id as usize].m_pEntity = Some(123);
            }
            app.world_mut().despawn(entity);
            app.update();
            assert_eq!(app.world().resource::<rust_gta_3_game::audio::DMAudio::cDMAudio>()
                .AudioManager.m_asAudioEntities[id as usize].m_bIsUsed, replaced_owner);
        }
    }

    #[test]
    fn view_forward_matches_the_third_person_camera_axes() {
        let mut arena = DeveloperArena::default();
        assert!(view_forward(&arena).abs_diff_eq(Vec3::NEG_Z, 1.0e-6));
        arena.camera_yaw = std::f32::consts::FRAC_PI_2;
        assert!(view_forward(&arena).abs_diff_eq(Vec3::NEG_X, 1.0e-6));
    }

    #[test]
    fn arena_vehicle_spawn_advances_the_re3_model_colour_variation() {
        let mut file_loader = CFileLoader::new();
        let model = file_loader.model_info.add_vehicle_model(110);
        model.num_colours = 3;
        model.colours1[..3].copy_from_slice(&[10, 11, 12]);
        model.colours2[..3].copy_from_slice(&[20, 21, 22]);

        assert_eq!(
            select_vehicle_colours(&mut file_loader, 110),
            Some((11, 21))
        );
        assert_eq!(
            select_vehicle_colours(&mut file_loader, 110),
            Some((12, 22))
        );
    }

    #[test]
    fn arena_vehicle_colour_indices_resolve_through_carcols_table() {
        let table = CVehicleColours {
            vehicle_colour_table: vec![
                RwRGBA {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                },
                RwRGBA {
                    red: 25,
                    green: 50,
                    blue: 75,
                    alpha: 255,
                },
                RwRGBA {
                    red: 100,
                    green: 125,
                    blue: 150,
                    alpha: 255,
                },
            ],
            vehicle_variations: Vec::new(),
        };
        let remap = vehicle_colour_remap(Some((1, 2)), Some(&table));
        assert_eq!(remap.colour1_index, Some(1));
        assert_eq!(remap.colour2_index, Some(2));
        assert_eq!(remap.colour1, Some([25, 50, 75, 255]));
        assert_eq!(remap.colour2, Some([100, 125, 150, 255]));
    }
}
