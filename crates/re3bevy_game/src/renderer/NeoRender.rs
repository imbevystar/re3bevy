//! Opt-in RenderWare Neo material submission through Bevy's mesh/material ECS.
//! The reference uses separate D3D9/GL callbacks; Bevy 0.20 owns the same
//! drawables and their resident asset handles instead of translating callbacks.

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use re3bevy_core::core::Schedules::GtaProcessSet;
use std::collections::BTreeMap;

use re3bevy_assets::{
    install::OriginalGtaInstall::OriginalGtaInstall,
    streaming::TxdStore::CTxdStoreAsset,
};

use super::{
    BevyRender::{GtaRuntimePoolMesh, GtaRuntimePoolMeshSource, GtaWorldTextureCache, ModelMaterialDrawable},
    RendererMaterials::{GtaNeoMaterial, GtaNeoMode, GtaUnlitMaterial},
};
use crate::extras::custompipes::{
    Color as NeoColor, CustomPipesState, InterpolatedColor, InterpolatedFloat,
    InterpolationContext, VEHICLEPIPE_NEO,
};
use crate::core::FileLoader::CFileLoader;
use re3bevy_core::core::Clock::CClock;
use super::Weather::CWeather;
use super::PcMatFxRender::{GtaOriginalMaterial, GtaPcEnvMaterial, GtaPcEnvMaterialLink};

/// Material-side inputs retained on the same Bevy entity as the DFF submesh.
/// Material FX's dual texture and Neo gloss texture must be resolved from real
/// source data; leaving either absent keeps its draw on the normal RW pipeline.
#[derive(Component, Clone, Default)]
pub struct GtaNeoSurface {
    pub specular: f32,
    pub explicit_mode: Option<GtaNeoMode>,
    pub lightmap_texture: Option<Handle<Image>>,
    pub gloss_texture: Option<Handle<Image>>,
    pub gloss_name: Option<String>,
}

/// re3 CustomPipes::GetGlossTex looks up `<base>_gloss` in the optional Neo
/// dictionary. Read its already-loaded TXD entry; the renderer never inserts
/// an invented reflection, gloss or lightmap raster.
pub(super) fn surface_for_drawable(
    drawable: &ModelMaterialDrawable,
    file_loader: &CFileLoader,
    cache: &GtaWorldTextureCache,
) -> GtaNeoSurface {
    let texture_slot = drawable.texture_txd_slot.or_else(|| {
        file_loader.model_info.get_model_info(drawable.texture_model_index)
            .and_then(|model| usize::try_from(model.base().txd_slot).ok())
    });
    let lightmap_texture = texture_slot.zip(drawable.dual_texture_name.as_ref())
        .and_then(|(slot, name)| cache.images.get(&(slot, name.to_ascii_lowercase())).cloned());
    let gloss_texture = file_loader.txd_store.find_txd_slot("neo")
        .and_then(|slot| {
            let name = drawable.texture_name.as_deref()?;
            cache.images.get(&(slot, format!("{}_gloss", name.to_ascii_lowercase()))).cloned()
        });
    GtaNeoSurface {
        specular: drawable.specular,
        lightmap_texture,
        gloss_texture,
        gloss_name: drawable.texture_name.as_ref().map(|name| format!("{}_gloss", name.to_ascii_lowercase())),
        ..default()
    }
}

/// re3 reads `neo/neo.txd` as a separate dictionary; it is not in GTA III's
/// streaming CTxdStore. Decode it exactly once, and never manufacture rasters
/// when the optional Neo extension is not installed.
#[derive(Resource, Default)]
struct NeoTextureDictionary {
    attempted: bool,
    textures: BTreeMap<String, Handle<Image>>,
}

/// The original ReadTweakValueTable consumes exactly 24 lines per table and
/// four tab-separated weather cells per line, ignoring blank/comment lines.
fn tweak_cells(bytes: &[u8]) -> Vec<Vec<String>> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('\t').map(|cell| cell.trim().to_owned()).collect())
        .filter(|cells: &Vec<String>| cells.len() >= 4)
        .collect()
}

fn read_float_table(table: &mut InterpolatedFloat, rows: &[Vec<String>], offset: usize) {
    if rows.len() < offset + 24 { return; }
    for (hour, row) in rows[offset..offset + 24].iter().enumerate() {
        for (weather, cell) in row.iter().take(4).enumerate() {
            let _ = table.read(cell, hour, weather);
        }
    }
    table.cur_interpolator = 61;
}

fn read_color_table(table: &mut InterpolatedColor, rows: &[Vec<String>], offset: usize, alpha_div: f32) {
    if rows.len() < offset + 24 { return; }
    for (hour, row) in rows[offset..offset + 24].iter().enumerate() {
        for (weather, cell) in row.iter().take(4).enumerate() {
            let values: Vec<f32> = cell.split(',')
                .map(str::trim).filter_map(|field| field.parse::<f32>().ok()).collect();
            if values.len() != 4 { continue; }
            table.data[hour][weather] = NeoColor {
                r: values[0] / 255.0,
                g: values[1] / 255.0,
                b: values[2] / 255.0,
                a: values[3] / alpha_div,
            };
        }
    }
    table.cur_interpolator = 61;
}

fn load_tweak_files(config: &mut CustomPipesState, install: &OriginalGtaInstall) {
    if let Ok(bytes) = install.read("neo/carTweakingTable.dat") {
        let rows = tweak_cells(&bytes);
        read_float_table(&mut config.fresnel, &rows, 0);
        read_float_table(&mut config.power, &rows, 24);
        read_color_table(&mut config.diff_color, &rows, 48, 100.0);
        read_color_table(&mut config.spec_color, &rows, 72, 100.0);
    }
    if let Ok(bytes) = install.read("neo/worldTweakingTable.dat") {
        read_float_table(&mut config.world_lightmap_blend, &tweak_cells(&bytes), 0);
    }
    if let Ok(bytes) = install.read("neo/rimTweakingTable.dat") {
        let rows = tweak_cells(&bytes);
        read_color_table(&mut config.ramp_start, &rows, 0, 255.0);
        read_color_table(&mut config.ramp_end, &rows, 24, 255.0);
        read_float_table(&mut config.offset, &rows, 48);
        read_float_table(&mut config.scale, &rows, 72);
        read_float_table(&mut config.scaling, &rows, 96);
    }
}

fn interpolate_neo_settings(
    clock: Option<Res<CClock>>,
    weather: Option<Res<CWeather>>,
    mut config: ResMut<CustomPipesState>,
) {
    let (Some(clock), Some(weather)) = (clock, weather) else { return };
    let context = InterpolationContext {
        hour: clock.hours() as usize,
        minute: clock.minutes(),
        second: clock.seconds().min(59) as u8,
        old_weather: weather.old_weather_type.max(0) as usize,
        new_weather: weather.new_weather_type.max(0) as usize,
        weather_interpolation: weather.interpolation_value.clamp(0.0, 1.0),
    };
    config.fresnel.get(context);
    config.power.get(context);
    config.diff_color.get(context);
    config.spec_color.get(context);
    config.world_lightmap_blend.get(context);
    config.ramp_start.get(context);
    config.ramp_end.get(context);
    config.offset.get(context);
    config.scale.get(context);
    config.scaling.get(context);
}

fn load_neo_texture_dictionary(
    install: Option<Res<OriginalGtaInstall>>,
    mut dictionary: ResMut<NeoTextureDictionary>,
    mut config: ResMut<CustomPipesState>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(install) = install else { return };
    if dictionary.attempted { return; }
    dictionary.attempted = true;
    load_tweak_files(&mut config, &install);
    // The Neo pack is an optional re3 extension. Missing files must not
    // disable normal RW rendering or the available tuning tables.
    if let Ok(txd) = CTxdStoreAsset::load(&install, "neo/neo.txd") {
      for decoded in txd.decoded_textures {
        // TexRead skips unsupported rasters. Iterate successful decodes, not a
        // zip against source descriptors, which can silently misalign names.
        let mut image = Image::new(
            Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            decoded.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::default());
        dictionary.textures.insert(decoded.name.to_ascii_lowercase(), images.add(image));
      }
    }
    config.neo_txd_loaded = !dictionary.textures.is_empty();
    info!("Neo texture dictionary: {} decoded textures", dictionary.textures.len());
}

#[derive(Component)]
pub(super) struct NeoMaterialLink {
    original: Handle<GtaUnlitMaterial>,
    neo: Handle<GtaNeoMaterial>,
    mode: GtaNeoMode,
    overlay: Option<Entity>,
}

impl NeoMaterialLink {
    pub(super) fn replaces_base(&self) -> bool { self.overlay.is_none() }
}

fn is_vehicle(source: GtaRuntimePoolMeshSource) -> bool {
    matches!(source,
        GtaRuntimePoolMeshSource::SaveVehicle |
        GtaRuntimePoolMeshSource::ReplayVehicle |
        GtaRuntimePoolMeshSource::ScriptVehicle |
        GtaRuntimePoolMeshSource::ScriptVehicleDynamic)
}

fn is_ped(source: GtaRuntimePoolMeshSource) -> bool {
    matches!(source,
        GtaRuntimePoolMeshSource::SavePed |
        GtaRuntimePoolMeshSource::ReplayPed |
        GtaRuntimePoolMeshSource::WorldPlayer |
        GtaRuntimePoolMeshSource::ScriptPed)
}

/// Vanilla GTA III (the default) must not scan all world meshes each frame
/// just because the optional Neo adapter is installed. Still run on newly
/// attached explicit effects, and for one last frame while links need cleanup.
fn neo_processing_needed(
    settings: Res<CustomPipesState>,
    linked: Query<(), With<NeoMaterialLink>>,
    changed: Query<&GtaNeoSurface, Changed<GtaNeoSurface>>,
) -> bool {
    settings.vehicle_pipe_switch == VEHICLEPIPE_NEO
        || settings.lightmap_enable
        || settings.gloss_enable
        || settings.rimlight_enable
        || !linked.is_empty()
        || changed.iter().any(|surface| surface.explicit_mode.is_some())
}

fn requested_mode(
    surface: &GtaNeoSurface,
    source: Option<&GtaRuntimePoolMesh>,
    config: &CustomPipesState,
    environment: Option<&Handle<Image>>,
    has_uv_b: bool,
) -> Option<GtaNeoMode> {
    if let Some(mode) = surface.explicit_mode {
        return match mode {
            GtaNeoMode::Vehicle if environment.is_some() => Some(mode),
            GtaNeoMode::World if surface.lightmap_texture.is_some() && has_uv_b => Some(mode),
            GtaNeoMode::Gloss if surface.gloss_texture.is_some() => Some(mode),
            GtaNeoMode::Rim => Some(mode),
            _ => None,
        };
    }
    if config.vehicle_pipe_switch == VEHICLEPIPE_NEO
        && environment.is_some()
        && source.is_some_and(|entity| is_vehicle(entity.source))
    {
        Some(GtaNeoMode::Vehicle)
    } else if config.lightmap_enable && surface.lightmap_texture.is_some() && has_uv_b {
        Some(GtaNeoMode::World)
    } else if config.gloss_enable && surface.gloss_texture.is_some() {
        Some(GtaNeoMode::Gloss)
    } else if config.rimlight_enable && source.is_some_and(|entity| is_ped(entity.source)) {
        Some(GtaNeoMode::Rim)
    } else {
        None
    }
}

fn make_material(
    original: &GtaUnlitMaterial,
    surface: &GtaNeoSurface,
    mode: GtaNeoMode,
    config: &CustomPipesState,
    environment: Option<&Handle<Image>>,
) -> GtaNeoMaterial {
    let mut material = GtaNeoMaterial::from_unlit(original, mode);
    material.reflection_texture = environment.cloned();
    material.lightmap_texture = surface.lightmap_texture.clone();
    material.gloss_texture = surface.gloss_texture.clone();
    material.params.reflection = Vec4::new(
        config.fresnel.cur_val,
        config.spec_color.cur_val.a,
        surface.specular * config.vehicle_shininess,
        if surface.specular > 0.0 { config.vehicle_specularity } else { 0.0 },
    );
    material.params.spec_color_and_power = Vec4::new(
        config.spec_color.cur_val.r,
        config.spec_color.cur_val.g,
        config.spec_color.cur_val.b,
        config.power.cur_val.max(1.0),
    );
    material.params.lightmap = Vec4::new(
        config.world_lightmap_blend.cur_val * config.lightmap_mult,
        config.world_lightmap_blend.cur_val * config.lightmap_mult,
        config.world_lightmap_blend.cur_val * config.lightmap_mult,
        1.0,
    );
    material.params.rim_start = Vec4::new(
        config.ramp_start.cur_val.r, config.ramp_start.cur_val.g,
        config.ramp_start.cur_val.b, config.ramp_start.cur_val.a,
    );
    material.params.rim_end = Vec4::new(
        config.ramp_end.cur_val.r, config.ramp_end.cur_val.g,
        config.ramp_end.cur_val.b, config.ramp_end.cur_val.a,
    );
    material.params.rim_data = Vec4::new(
        config.offset.cur_val, config.scale.cur_val,
        config.scaling.cur_val * config.rimlight_mult, config.gloss_mult,
    );
    if mode == GtaNeoMode::Gloss {
        material.alpha_mode = AlphaMode::Add;
    }
    material
}

/// Applies pipeline changes after the owner submits model meshes. New Neo
/// material assets are created only on transitions; all updates reuse them.
fn sync_neo_materials(
    mut commands: Commands,
    config: Res<CustomPipesState>,
    texture_cache: Res<GtaWorldTextureCache>,
    neo_textures: Res<NeoTextureDictionary>,
    originals: Res<Assets<GtaUnlitMaterial>>,
    meshes: Res<Assets<Mesh>>,
    mut neo_materials: ResMut<Assets<GtaNeoMaterial>>,
    mut pc_materials: ResMut<Assets<GtaPcEnvMaterial>>,
    drawables: Query<(
        Entity,
        &GtaNeoSurface,
        &Mesh3d,
        Option<&GtaRuntimePoolMesh>,
        Option<&MeshMaterial3d<GtaUnlitMaterial>>,
        Option<&NeoMaterialLink>,
        Option<&GtaOriginalMaterial>,
        Option<&GtaPcEnvMaterialLink>,
    )>,
) {
    for (entity, surface, mesh, source, base, link, original_owner, pc_link) in &drawables {
        let resolved_gloss = surface.gloss_texture.clone().or_else(|| {
            surface.gloss_name.as_ref().and_then(|name| neo_textures.textures.get(name).cloned())
        });
        let mut resolved = surface.clone();
        resolved.gloss_texture = resolved_gloss;
        let has_uv_b = meshes.get(&mesh.0).is_some_and(|mesh| mesh.attribute(Mesh::ATTRIBUTE_UV_1).is_some());
        let mode = requested_mode(&resolved, source, &config, texture_cache.env_map.as_ref(), has_uv_b);
        if mode.is_none() {
            if let Some(link) = link {
                if let Some(overlay) = link.overlay {
                    commands.entity(overlay).despawn();
                } else {
                    commands.entity(entity)
                        .remove::<MeshMaterial3d<GtaNeoMaterial>>()
                        .insert(MeshMaterial3d(link.original.clone()));
                }
                commands.entity(entity).remove::<NeoMaterialLink>();
                neo_materials.remove(link.neo.id());
            }
            continue;
        }
        let mode = mode.unwrap();
        let original_handle = link.map(|l| l.original.clone())
            .or_else(|| original_owner.map(|owner| owner.0.clone()))
            .or_else(|| base.map(|b| b.0.clone()));
        let Some(original_handle) = original_handle else { continue };
        let Some(original) = originals.get(&original_handle) else { continue };
        let material = make_material(original, &resolved, mode, &config, texture_cache.env_map.as_ref());
        if let Some(link) = link {
            if link.mode == mode {
                if neo_materials.get(&link.neo) != Some(&material)
                    && let Some(mut existing) = neo_materials.get_mut(&link.neo)
                {
                    *existing = material;
                }
                continue;
            }
            if let Some(overlay) = link.overlay {
                commands.entity(overlay).despawn();
            } else {
                commands.entity(entity).remove::<MeshMaterial3d<GtaNeoMaterial>>();
            }
            neo_materials.remove(link.neo.id());
        }
        let neo = neo_materials.add(material);
        if mode != GtaNeoMode::Gloss && let Some(pc_link) = pc_link {
            commands.entity(entity).remove::<MeshMaterial3d<GtaPcEnvMaterial>>()
                .remove::<GtaPcEnvMaterialLink>();
            pc_materials.remove(pc_link.0.id());
        }
        let overlay = if mode == GtaNeoMode::Gloss {
            // The source Neo gloss pipe uses additive blending over the normal
            // world material, rather than replacing its base draw.
            if pc_link.is_none() {
                commands.entity(entity).insert(MeshMaterial3d(original_handle.clone()));
            }
            let child = commands.spawn((
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(neo.clone()),
                Transform::default(),
                Visibility::Visible,
            )).id();
            commands.entity(entity).add_child(child);
            Some(child)
        } else {
            commands.entity(entity)
                .remove::<MeshMaterial3d<GtaUnlitMaterial>>()
                .insert(MeshMaterial3d(neo.clone()));
            None
        };
        commands.entity(entity).insert(NeoMaterialLink {
            original: original_handle, neo, mode, overlay,
        });
    }
}

pub struct GtaNeoRenderPlugin;

impl Plugin for GtaNeoRenderPlugin {
    fn build(&self, app: &mut App) {
        // Default PC MatFX shares the same source entities and runs after Neo
        // transitions. Explicit Neo replaces it; disabling Neo restores it.
        app.add_plugins(MaterialPlugin::<GtaPcEnvMaterial>::default())
            .init_resource::<super::PcMatFxRender::GtaPcEnvMaterials>()
            .init_resource::<super::PcMatFxRender::GtaPcEnvDiagnostics>()
            .add_systems(Update, (super::PcMatFxRender::sync_pc_environment_materials,
                super::PcMatFxRender::refresh_restored_original_materials).chain()
                .after(GtaProcessSet::RenderPrep)
                .after(sync_neo_materials));
        app.init_resource::<CustomPipesState>()
            .init_resource::<NeoTextureDictionary>()
            .add_systems(Update, load_neo_texture_dictionary
                .after(GtaProcessSet::RenderPrep)
                .before(sync_neo_materials))
            .add_systems(Update, interpolate_neo_settings
                .after(load_neo_texture_dictionary)
                .before(sync_neo_materials))
            .add_systems(Update, sync_neo_materials
                .after(GtaProcessSet::RenderPrep)
                .run_if(neo_processing_needed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neo_switch_restores_pc_matfx_without_duplicate_draw_components() {
        use super::super::PcMatFxRender::{GtaPcEnvDiagnostics, GtaPcEnvMaterials,
            GtaPcEnvSurface, sync_pc_environment_materials};
        let mut app = App::new();
        app.init_resource::<Assets<GtaUnlitMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<GtaNeoMaterial>>()
            .init_resource::<Assets<GtaPcEnvMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<GtaWorldTextureCache>()
            .init_resource::<CustomPipesState>()
            .init_resource::<NeoTextureDictionary>()
            .init_resource::<GtaPcEnvMaterials>()
            .init_resource::<GtaPcEnvDiagnostics>()
            .add_systems(Update, (sync_neo_materials, sync_pc_environment_materials).chain());
        app.world_mut().resource_mut::<GtaWorldTextureCache>().env_map = Some(Handle::default());
        let original = app.world_mut().resource_mut::<Assets<GtaUnlitMaterial>>()
            .add(GtaUnlitMaterial::from_gta(Color::WHITE, None, false, false, [0.0; 3], 0.0));
        let entity = app.world_mut().spawn((
            GtaNeoSurface { specular: 1.0, ..default() },
            GtaRuntimePoolMesh { source: GtaRuntimePoolMeshSource::ScriptVehicleDynamic,
                slot: 1, model_index: 90, render_damaged: false },
            GtaOriginalMaterial(original.clone()),
            GtaPcEnvSurface { coefficient: 0.5 },
            Mesh3d(Handle::default()), MeshMaterial3d(original.clone()),
        )).id();
        app.update();
        assert!(app.world().get::<MeshMaterial3d<GtaPcEnvMaterial>>(entity).is_some());
        assert!(app.world().get::<MeshMaterial3d<GtaUnlitMaterial>>(entity).is_none());
        app.world_mut().resource_mut::<CustomPipesState>().vehicle_pipe_switch = VEHICLEPIPE_NEO;
        app.update();
        assert!(app.world().get::<MeshMaterial3d<GtaNeoMaterial>>(entity).is_some());
        assert!(app.world().get::<MeshMaterial3d<GtaPcEnvMaterial>>(entity).is_none());
        assert!(app.world().get::<MeshMaterial3d<GtaUnlitMaterial>>(entity).is_none());
        assert!(app.world().resource::<Assets<GtaPcEnvMaterial>>().is_empty());
        app.world_mut().resource_mut::<CustomPipesState>().vehicle_pipe_switch =
            crate::extras::custompipes::VEHICLEPIPE_MATFX;
        app.update();
        assert!(app.world().get::<MeshMaterial3d<GtaNeoMaterial>>(entity).is_none());
        assert!(app.world().get::<MeshMaterial3d<GtaPcEnvMaterial>>(entity).is_some());
        assert!(app.world().get::<MeshMaterial3d<GtaUnlitMaterial>>(entity).is_none());
        assert!(app.world().resource::<Assets<GtaNeoMaterial>>().is_empty());
        assert_eq!(app.world().get::<GtaOriginalMaterial>(entity).unwrap().0, original);
    }

    #[test]
    fn tweak_tables_keep_weather_columns_and_source_alpha_scales() {
        let mut cells = vec![vec!["0.25".to_owned(); 4]; 24];
        cells[8][3] = "0.75".to_owned();
        let mut table = InterpolatedFloat::new(0.4);
        read_float_table(&mut table, &cells, 0);
        assert_eq!(table.data[8][3], 0.75);
        assert_eq!(table.data[8][0], 0.25);

        let colours = vec![vec!["255, 128, 0, 75".to_owned(); 4]; 24];
        let mut specular = InterpolatedColor::new(NeoColor::BLACK);
        read_color_table(&mut specular, &colours, 0, 100.0);
        assert_eq!(specular.data[6][2].r, 1.0);
        assert!((specular.data[6][2].g - 128.0 / 255.0).abs() < 0.000001);
        assert_eq!(specular.data[6][2].a, 0.75);

        let mut rim = InterpolatedColor::new(NeoColor::BLACK);
        read_color_table(&mut rim, &colours, 0, 255.0);
        assert!((rim.data[6][2].a - 75.0 / 255.0).abs() < 0.000001);
    }

    #[test]
    fn table_parser_ignores_comments_and_incomplete_rows() {
        let rows = tweak_cells(b"# intro\n\n0.1\t0.2\t0.3\t0.4\ninvalid\trow\n\t 1 \t 2 \t 3 \t 4 \n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ["0.1", "0.2", "0.3", "0.4"]);
    }

    #[test]
    fn optional_effects_never_draw_with_missing_source_textures() {
        let mut config = CustomPipesState::default();
        config.vehicle_pipe_switch = VEHICLEPIPE_NEO;
        config.lightmap_enable = true;
        config.gloss_enable = true;
        config.rimlight_enable = true;
        let surface = GtaNeoSurface {
            explicit_mode: Some(GtaNeoMode::World),
            ..default()
        };
        assert_eq!(requested_mode(&surface, None, &config, None, true), None);

        let material = GtaNeoSurface {
            explicit_mode: Some(GtaNeoMode::Rim),
            ..default()
        };
        assert_eq!(requested_mode(&material, None, &config, None, false), Some(GtaNeoMode::Rim));

        let vehicle = GtaRuntimePoolMesh {
            source: GtaRuntimePoolMeshSource::ScriptVehicle,
            slot: 0,
            model_index: 0,
            render_damaged: false,
        };
        assert_eq!(requested_mode(&GtaNeoSurface::default(), Some(&vehicle), &config, None, true), None);
    }
}
