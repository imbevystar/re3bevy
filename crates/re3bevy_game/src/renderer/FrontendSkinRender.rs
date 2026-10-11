//! Frontend player-skin preview target, camera, texture, and lifecycle ownership.

use super::runtime_submission::flush_runtime_pool_meshes;
use super::model_drawables::{
    append_decoded_geometry_drawables, append_geometry_drawables, atomic_frame_world_matrix,
    deform_skin_geometry,
};
use super::{
    GtaWorldTextureCache, ModelMaterialDrawable, VehicleColourRemap,
};
use crate::{
    core::Frontend::{CMenuManager, eMenuScreen},
    modelinfo::ModelIndices::MI_PLAYER,
    renderer::{
        PlayerSkin::{CPlayerSkin, DEFAULT_SKIN_NAME},
        RendererMaterials::GtaUnlitMaterial,
    },
};
use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, visibility::RenderLayers},
    image::{
        CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
        ImageSamplerDescriptor, ImageType,
    },
    prelude::*,
    render::render_resource::TextureFormat,
    render::view::{DebandDither, Msaa, NoIndirectDrawing, Tonemapping},
};
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_core::core::{GameState::GtaAppState, Timer::CTimer};
use re3bevy_formats::formats::{
    ClumpRead::{RwClumpSummary, read_clump},
    Directory::CDirectory,
};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
};

pub(super) struct FrontendSkinRenderPlugin;

impl Plugin for FrontendSkinRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CPlayerSkin>()
            .init_resource::<GtaFrontendSkinRenderDiagnostics>()
            .add_systems(Startup, initialise_player_skin_preview_target)
            .add_systems(
                Update,
                sync_player_skin_render_state.before(flush_runtime_pool_meshes),
            );
    }
}

/// Active-preview failure counters. Ordinary closed-menu filtering does not count.
#[derive(Debug, Default, Resource)]
pub struct GtaFrontendSkinRenderDiagnostics {
    pub missing_install_frames: u64,
    pub missing_timer_frames: u64,
    pub waiting_target_frames: u64,
    pub failed_clump_loads: u64,
    pub missing_skin_frames: u64,
    pub invalid_geometry_frames: u64,
}

const FRONTEND_SKIN_RENDER_LAYER: usize = 30;

#[derive(Debug, Clone, Copy, Component)]
struct FrontendSkinPreviewRoot;

#[derive(Debug, Clone, Copy, Component)]
struct FrontendSkinPreviewCamera;

fn initialise_player_skin_preview_target(
    mut images: ResMut<Assets<Image>>,
    mut player_skin: ResMut<CPlayerSkin>,
) {
    if player_skin.preview_target.is_some() {
        return;
    }
    let mut target = Image::new_target_texture(640, 448, TextureFormat::Rgba8UnormSrgb, None);
    target.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    });
    player_skin.preview_target = Some(images.add(target));
}

#[allow(clippy::too_many_arguments)]
fn sync_player_skin_render_state(
    mut commands: Commands,
    menu: Option<Res<CMenuManager>>,
    install: Option<Res<OriginalGtaInstall>>,
    app_state: Res<State<GtaAppState>>,
    timer: Option<Res<CTimer>>,
    mut player_skin: ResMut<CPlayerSkin>,
    mut diagnostics: ResMut<GtaFrontendSkinRenderDiagnostics>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut preview_roots: Query<&mut Transform, With<FrontendSkinPreviewRoot>>,
) {
    let preview_active = *app_state.get() == GtaAppState::Frontend
        && menu
            .as_ref()
            .is_some_and(|menu| menu.curr_screen == eMenuScreen::MENUPAGE_SKIN_SELECT as i32);
    if !preview_active {
        if let Some(root) = player_skin.preview_root.take() {
            commands.entity(root).despawn();
        }
        if let Some(camera) = player_skin.preview_camera.take() {
            commands.entity(camera).despawn();
        }
        player_skin.player_clump_loaded = false;
        player_skin.preview_skin_name.clear();
        player_skin.frontend_clump = None;
        player_skin.frontend_clump_load_attempted = false;
        return;
    }

    let Some(install) = install else {
        diagnostics.missing_install_frames += 1;
        return;
    };
    let menu = menu.expect("active skin-select screen has a menu resource");

    if player_skin.frontend_clump.is_none() && !player_skin.frontend_clump_load_attempted {
        player_skin.frontend_clump_load_attempted = true;
        match load_frontend_player_clump(&install) {
            Ok(clump) => player_skin.frontend_clump = Some(clump),
            Err(error) => {
                diagnostics.failed_clump_loads += 1;
                warn!("CPlayerSkin::BeginFrontendSkinEdit: {error}");
            }
        }
    }

    let Some(target) = player_skin.preview_target.clone() else {
        diagnostics.waiting_target_frames += 1;
        return;
    };
    if player_skin.preview_camera.is_none() {
        let camera = commands
            .spawn((
                Camera3d::default(),
                NoIndirectDrawing,
                Camera {
                    order: -1,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    ..default()
                },
                RenderTarget::Image(target.into()),
                Projection::Perspective(PerspectiveProjection {
                    // `CDraw::SetFOV(30)` is a horizontal RenderWare FOV.
                    // Bevy's perspective projection takes a vertical FOV, so
                    // reproduce `CameraSize`: vw.x=tan(hfov/2),
                    // vw.y=target_height/target_width*vw.x.
                    fov: 2.0 * ((30.0_f32.to_radians() * 0.5).tan() / (640.0 / 448.0)).atan(),
                    aspect_ratio: 640.0 / 448.0,
                    near: 0.1,
                    far: 100.0,
                    ..default()
                }),
                // The clump root below carries re3's camera-relative offset;
                // this camera supplies only the matching 7.725 depth.
                Transform::from_xyz(0.0, 0.0, 7.725).looking_at(Vec3::ZERO, Vec3::Y),
                Tonemapping::None,
                DebandDither::Disabled,
                Msaa::Off,
                RenderLayers::layer(FRONTEND_SKIN_RENDER_LAYER),
                FrontendSkinPreviewCamera,
                Name::new("CPlayerSkin Frontend Camera"),
            ))
            .id();
        player_skin.preview_camera = Some(camera);
    }

    if player_skin.preview_root.is_none() || player_skin.preview_skin_name != menu.skin_name {
        if let Some(root) = player_skin.preview_root.take() {
            commands.entity(root).despawn();
        }
        let texture =
            load_player_skin_image(&menu.skin_name, &install, &mut texture_cache, &mut images)
                .or_else(|| {
                    load_player_skin_image(
                        DEFAULT_SKIN_NAME,
                        &install,
                        &mut texture_cache,
                        &mut images,
                    )
                });
        // CPlayerInfo::LoadPlayerSkin retries DEFAULT_SKIN_NAME. A failure of
        // both authored BMPs must stay visible as failure, never a white clump.
        let Some(texture) = texture else {
            diagnostics.missing_skin_frames += 1;
            return;
        };
        let drawables = player_skin
            .frontend_clump
            .as_ref()
            .map(build_frontend_player_drawables)
            .unwrap_or_default();
        if drawables.is_empty() {
            diagnostics.invalid_geometry_frames += 1;
            return;
        }
        let root = commands
            .spawn((
                // PlayerSkin.cpp: pos { 1.35, .35, 7.725 }, then X -90.
                // `CVector::to_bevy` already performs that GTA/RW -> Bevy
                // axis conversion. RenderWare camera X projects oppositely,
                // hence -1.35; the vertical .35 belongs to the clump, not the
                // camera (moving the camera had inverted the placement).
                Transform::from_xyz(-1.35, 0.35, 0.0),
                Visibility::Visible,
                FrontendSkinPreviewRoot,
                RenderLayers::layer(FRONTEND_SKIN_RENDER_LAYER),
                Name::new("CPlayerSkin Frontend Clump"),
            ))
            .id();
        for drawable in drawables {
            commands.spawn((
                Mesh3d(meshes.add(drawable.mesh)),
                MeshMaterial3d(materials.add(GtaUnlitMaterial::from_gta(
                    // RwRGBAReal ambient values are linear light factors.
                    Color::linear_rgb(0.65, 0.65, 0.65),
                    Some(texture.clone()),
                    false,
                    false,
                    [0.0, 0.0, 0.0],
                    0.0,
                ))),
                Transform::default(),
                Visibility::Visible,
                RenderLayers::layer(FRONTEND_SKIN_RENDER_LAYER),
                ChildOf(root),
            ));
        }
        player_skin.preview_root = Some(root);
        player_skin.preview_skin_name.clone_from(&menu.skin_name);
        player_skin.player_clump_loaded = true;
    }

    let Some(timer) = timer else {
        diagnostics.missing_timer_frames += 1;
        return;
    };
    let render = player_skin.render_frontend_skin_edit(timer.time_in_milliseconds_pause_mode());
    if let Some(root) = player_skin.preview_root
        && let Ok(mut transform) = preview_roots.get_mut(root)
    {
        transform.rotation = Quat::from_rotation_y(render.axis2_rotation_deg.to_radians());
    }
}

/// re3 `LoadPlayerDff`: locate player.dff in gta3.dir and read its sector range
/// directly from gta3.img.  Frontend runs before level IDE data is guaranteed,
/// so this clump must not depend on MI_PLAYER already existing in ModelInfo.
fn load_frontend_player_clump(install: &OriginalGtaInstall) -> Result<RwClumpSummary, String> {
    let directory_file = install
        .open("models\\gta3.dir")
        .map_err(|error| format!("models/gta3.dir: {error}"))?;
    let directory = CDirectory::read_from(directory_file, 20_000)
        .map_err(|error| format!("models/gta3.dir decode: {error:?}"))?;
    let (offset_sectors, size_sectors) = directory
        .find_item("player.dff")
        .ok_or_else(|| "models/gta3.dir has no player.dff entry".to_owned())?;
    let mut image = install
        .open("models\\gta3.img")
        .map_err(|error| format!("models/gta3.img: {error}"))?;
    image
        .seek(SeekFrom::Start(u64::from(offset_sectors) * 2048))
        .map_err(|error| format!("player.dff sector seek: {error}"))?;
    let byte_len = usize::try_from(size_sectors)
        .ok()
        .and_then(|size| size.checked_mul(2048))
        .ok_or_else(|| "player.dff sector range exceeds addressable memory".to_owned())?;
    let mut bytes = vec![0_u8; byte_len];
    image
        .read_exact(&mut bytes)
        .map_err(|error| format!("player.dff sector read: {error}"))?;
    read_clump(&bytes).ok_or_else(|| "player.dff is not a valid RenderWare clump".to_owned())
}

fn build_frontend_player_drawables(summary: &RwClumpSummary) -> Vec<ModelMaterialDrawable> {
    let mut drawables = Vec::new();
    let colours = VehicleColourRemap::default();
    let mut drew_atomic = false;
    for atomic in &summary.atomics {
        let Some(geometry_index) = usize::try_from(atomic.geometry_index).ok() else {
            continue;
        };
        let Some(geometry) = summary.geometries.get(geometry_index) else {
            continue;
        };
        if let Some(deformed) = deform_skin_geometry(geometry, summary, None, None) {
            append_decoded_geometry_drawables(
                MI_PLAYER,
                geometry,
                &deformed,
                colours,
                None,
                &mut drawables,
            );
        } else {
            let frame = atomic_frame_world_matrix(summary, atomic.frame_index);
            append_geometry_drawables(MI_PLAYER, geometry, colours, frame, &mut drawables);
        }
        drew_atomic = true;
    }
    if !drew_atomic {
        for geometry in &summary.geometries {
            append_geometry_drawables(MI_PLAYER, geometry, colours, None, &mut drawables);
        }
    }
    drawables
}

/// re3 `CPlayerSkin::GetSkinTexture`: exact authored BMP lookup and skin dictionary cache.
fn load_player_skin_image(
    skin_name: &str,
    install: &OriginalGtaInstall,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    if let Some(handle) = texture_cache.player_skins.get(skin_name) {
        return Some(handle.clone());
    }
    let gta_path = if skin_name == DEFAULT_SKIN_NAME {
        "models\\generic\\player.bmp".to_owned()
    } else {
        format!("skins\\{skin_name}.bmp")
    };
    let path = install.resolve_existing_case_path(&gta_path);
    let bytes = fs::read(&path).ok()?;
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension("bmp"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Descriptor(ImageSamplerDescriptor {
            // `DefinedState` uses rwTEXTUREADDRESSWRAP. Player DFF UVs rely on
            // it (the torso reaches V=1.95 and upper arms U=1.15); clamping
            // stretches the atlas edge across the arms and neck.
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            address_mode_w: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Nearest,
            ..default()
        }),
        RenderAssetUsages::default(),
    )
    .map_err(|error| {
        warn!("CPlayerSkin failed to decode {}: {error}", path.display());
        error
    })
    .ok()?;
    let handle = images.add(image);
    texture_cache
        .player_skins
        .insert(skin_name.to_owned(), handle.clone());
    Some(handle)
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn closing_preview_releases_entities_without_install_or_timer() {
        let mut world = World::new();
        world.insert_resource(State::new(GtaAppState::InGame));
        world.init_resource::<GtaFrontendSkinRenderDiagnostics>();
        world.init_resource::<GtaWorldTextureCache>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<GtaUnlitMaterial>>();
        let root = world
            .spawn((FrontendSkinPreviewRoot, Transform::default()))
            .id();
        let child = world.spawn(ChildOf(root)).id();
        let camera = world.spawn(FrontendSkinPreviewCamera).id();
        let mut skin = CPlayerSkin::default();
        skin.preview_root = Some(root);
        skin.preview_camera = Some(camera);
        skin.preview_skin_name = "selected".into();
        skin.player_clump_loaded = true;
        skin.frontend_clump_load_attempted = true;
        world.insert_resource(skin);

        world
            .run_system_once(sync_player_skin_render_state)
            .unwrap();

        let skin = world.resource::<CPlayerSkin>();
        assert!(skin.preview_root.is_none());
        assert!(skin.preview_camera.is_none());
        assert!(!skin.player_clump_loaded);
        assert!(!skin.frontend_clump_load_attempted);
        assert!(skin.preview_skin_name.is_empty());
        assert!(world.get_entity(root).is_err());
        assert!(world.get_entity(child).is_err());
        assert!(world.get_entity(camera).is_err());
        assert_eq!(
            world
                .resource::<GtaFrontendSkinRenderDiagnostics>()
                .missing_install_frames,
            0
        );
    }
}
