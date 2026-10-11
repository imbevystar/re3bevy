//! re3 reference: references/re-GTA/src/skel/glfw/glfw.cpp (GS_INIT_FRONTEND)
//! re3 reference: references/re-GTA/src/core/main.cpp (LoadingScreen)
//!
//! The original draws `loadsc0` immediately before entering the frontend.
//! Keep that transition in Bevy's state lifecycle and show the original TXD
//! texture for enough updates to reach the renderer before opening the menu.

use std::fs;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::widget::NodeImageMode,
};
use re3bevy_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use re3bevy_core::core::GameState::GtaAppState;
use re3bevy_formats::formats::TexRead::{decode_txd_texture, read_txd};

const PRESENTATION_UPDATES: u8 = 2;

#[derive(Resource, Default)]
pub struct FrontendLoadingScreen {
    root: Option<Entity>,
    updates: u8,
}

#[derive(Component)]
struct FrontendLoadingScreenRoot;

pub fn enter_frontend_loading_screen(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut screen: ResMut<FrontendLoadingScreen>,
    install: Option<Res<OriginalGtaInstall>>,
) {
    screen.updates = 0;

    let image = install.as_deref().and_then(|install| {
        let path = install.resolve_existing_case_path("TXD/loadsc0.txd");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                warn!(
                    "Frontend loading splash could not read {}: {error}",
                    path.display()
                );
                return None;
            }
        };
        let Some(summary) = read_txd(&bytes) else {
            warn!("Frontend loading splash TXD is invalid: {}", path.display());
            return None;
        };
        let Some(texture) = summary
            .textures
            .iter()
            .find(|texture| texture.name.eq_ignore_ascii_case("loadsc0"))
        else {
            warn!(
                "Frontend loading splash texture loadsc0 is missing from {}",
                path.display()
            );
            return None;
        };
        let Some(decoded) = decode_txd_texture(texture) else {
            warn!("Frontend loading splash texture loadsc0 could not be decoded");
            return None;
        };

        Some(images.add(Image::new(
            Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            decoded.rgba,
            TextureFormat::Rgba8UnormSrgb,
            default(),
        )))
    });

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            FrontendLoadingScreenRoot,
            Name::new("FrontendLoadingScreen"),
        ))
        .id();

    if let Some(image) = image {
        let splash = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode {
                    image,
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                Name::new("OriginalLoadsc0"),
            ))
            .id();
        commands.entity(root).add_child(splash);
    }

    screen.root = Some(root);
}

pub fn present_frontend_loading_screen(
    mut screen: ResMut<FrontendLoadingScreen>,
    mut next_state: ResMut<NextState<GtaAppState>>,
) {
    screen.updates = screen.updates.saturating_add(1);
    if screen.updates >= PRESENTATION_UPDATES {
        next_state.set(GtaAppState::Frontend);
    }
}

pub fn exit_frontend_loading_screen(
    mut commands: Commands,
    mut screen: ResMut<FrontendLoadingScreen>,
) {
    if let Some(root) = screen.root.take() {
        commands.entity(root).despawn();
    }
}
