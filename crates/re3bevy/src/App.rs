//! re3 reference: references/re-GTA/src/core/Game.cpp
//! re3 reference: references/re-GTA/src/core/main.cpp
//!
//! Application wiring for the first GTA-owned core systems.

#[cfg(any(
    feature = "profiling",
    feature = "profiling-memory",
    feature = "profiling-chrome"
))]
use bevy::diagnostic::{
    EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
};
use bevy::{
    light::cluster::GlobalClusterSettings,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::{PresentMode, WindowResolution},
};
use re3bevy_core::core::{
    Clock::GtaClockPlugin,
    GameState::{CGameState, GtaAppState, GtaBootRoute},
    General::GtaGeneralPlugin,
    Schedules::GtaSchedulePlugin,
    Timer::GtaTimerPlugin,
};
use re3bevy_dev_arena::DeveloperArenaPlugin;
use re3bevy_game::core::Game::GtaGamePlugin;
use re3bevy_game::renderer::BevyRender::GtaBevyRenderPlugin;
use re3bevy_game::renderer::RendererMaterials::GtaRenderMaterialsPlugin;

#[cfg(feature = "profiling-toggle")]
#[path = "Profile.rs"]
mod Profile;

pub fn build_app() -> App {
    build_app_with_route(GtaBootRoute::Frontend)
}

/// Build the shared game app with an explicit re3 startup route.
/// Both binaries use this function, so assets, plugins and runtime systems are
/// compiled and executed only once per process.
pub fn build_app_with_route(boot_route: GtaBootRoute) -> App {
    build_app_with_route_and_visibility(boot_route, true, true)
}

/// Build the normal game runtime without presenting a desktop window. This is
/// reserved for Bevy-internal runtime probes that drive ECS input directly.
pub fn build_hidden_app_with_route(boot_route: GtaBootRoute) -> App {
    build_app_with_route_and_visibility(boot_route, false, true)
}

/// Make `TRACE_CHROME` safe before Bevy's `LogPlugin` reads it.
///
/// `tracing-chrome` panics with "Failed to create trace file" (os error 3) when
/// the directory of `TRACE_CHROME` does not exist, which killed the game on
/// startup. This creates the directory, falls back to
/// a process-specific OS temporary directory when the requested path is unusable, and picks
/// a default path when the variable is unset.
#[cfg(feature = "profiling-chrome")]
fn prepare_chrome_trace_path() {
    use std::path::PathBuf;

    let default_path = || -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        re3bevy_assets::install::RuntimePaths::diagnostic_root()
            .join(format!("re3bevy-chrome-{stamp}.json"))
    };

    let requested = std::env::var_os("TRACE_CHROME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(&path))
                    .unwrap_or(path)
            }
        });

    let usable = |path: &PathBuf| -> bool {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && std::fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        std::fs::File::create(path).is_ok()
    };

    let chosen = match requested {
        Some(path) if usable(&path) => path,
        Some(path) => {
            let fallback = default_path();
            eprintln!(
                "re3bevy profiling: TRACE_CHROME path {} is not writable, using {} instead",
                path.display(),
                fallback.display()
            );
            fallback
        }
        None => default_path(),
    };
    if !usable(&chosen) {
        eprintln!(
            "re3bevy profiling: cannot create trace file {}, Chrome trace disabled",
            chosen.display()
        );
        // SAFETY: called once at startup, before any other thread is spawned.
        unsafe { std::env::remove_var("TRACE_CHROME") };
        return;
    }
    eprintln!("re3bevy profiling: writing Chrome trace to {}", chosen.display());
    // SAFETY: called once at startup, before any other thread is spawned.
    unsafe { std::env::set_var("TRACE_CHROME", &chosen) };
}

fn build_app_with_route_and_visibility(
    boot_route: GtaBootRoute,
    visible: bool,
    with_event_loop: bool,
) -> App {
    #[cfg(feature = "profiling-chrome")]
    prepare_chrome_trace_path();

    let mut app = App::new();
    let mut default_plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "re3bevy".to_owned(),
                resolution: WindowResolution::new(1280, 720),
                present_mode: if cfg!(target_os = "android")
                    || std::env::var("RE3_VSYNC").as_deref() == Ok("1")
                {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                visible,
                #[cfg(target_os = "android")]
                mode: bevy::window::WindowMode::BorderlessFullscreen(
                    bevy::window::MonitorSelection::Primary,
                ),
                ..default()
            }),
            ..default()
        })
        .set(bevy::render::RenderPlugin {
            render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                bevy::render::settings::WgpuSettings {
                    // Capability fallback also provides a reproducible comparison
                    // of resident material grouping on the same GPU and scene.
                    disabled_features: (cfg!(target_os = "android")
                        || std::env::var("RE3_MATERIAL_BINDLESS").as_deref() == Ok("0"))
                    .then_some(bevy::render::settings::WgpuFeatures::BUFFER_BINDING_ARRAY),
                    instance_flags: bevy::render::settings::InstanceFlags::default()
                        & !bevy::render::settings::InstanceFlags::VALIDATION
                        & !bevy::render::settings::InstanceFlags::GPU_BASED_VALIDATION,
                    ..default()
                },
            )),
            ..default()
        });

    // Schedule-only tests must not create Winit's OS event loop on a test worker.
    // All gameplay/render plugin system registrations are otherwise identical.
    if !with_event_loop {
        default_plugins = default_plugins.disable::<bevy::winit::WinitPlugin>();
    }

    #[cfg(feature = "profiling-toggle")]
    let default_plugins = default_plugins.set(bevy::log::LogPlugin {
        custom_layer: Profile::profile_capture_layer,
        ..default()
    });

    app.add_plugins(default_plugins)
        // FrontendFlow::apply_frame_limit owns pacing in both focus states.
        // Winit's separate unfocused 60 Hz wait must not cap UNLIMITED.
        .insert_resource(bevy::winit::WinitSettings::continuous())
        .init_state::<GtaAppState>()
        .init_resource::<CGameState>()
        .insert_resource(boot_route)
        .add_plugins((
            GtaSchedulePlugin,
            GtaGeneralPlugin,
            GtaTimerPlugin,
            GtaClockPlugin,
            GtaRenderMaterialsPlugin,
            GtaBevyRenderPlugin,
            GtaGamePlugin,
            DeveloperArenaPlugin,
        ))
        // The streamed GTA scene can contain more than Bevy's conservative
        // default index-list capacity.  Allocate the observed upper bound up
        // front so clustered lighting never renders a corrupted/magenta frame
        // while the GPU buffer is resized.
        .add_systems(First, configure_gpu_clustering_capacity)
        .add_systems(Update, save_screenshot_on_f12);

    #[cfg(feature = "profiling-toggle")]
    app.add_systems(Update, Profile::toggle_profile_capture);

    #[cfg(any(
        feature = "profiling",
        feature = "profiling-memory",
        feature = "profiling-chrome"
    ))]
    {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
        app.add_systems(Update, profile_auto_exit_system);
    }

    app
}

#[cfg(any(
    feature = "profiling",
    feature = "profiling-memory",
    feature = "profiling-chrome"
))]
fn profile_auto_exit_system(
    mut exit: bevy::ecs::message::MessageWriter<bevy::app::AppExit>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    *elapsed += time.delta_secs();

    if let Ok(target_frames) = std::env::var("RE3BEVY_PROFILE_FRAMES") {
        if let Ok(limit) = target_frames.trim().parse::<u32>() {
            if *frames >= limit {
                eprintln!(
                    "re3bevy profiling: reached RE3BEVY_PROFILE_FRAMES ({limit}), exiting cleanly to flush trace"
                );
                exit.write(bevy::app::AppExit::Success);
                return;
            }
        }
    }

    // Chrome traces grow by ~30 MB/s, so without a limit a forgotten session
    // produces tens of GB. Default to 90 s (menus/loading count too); override
    // with RE3BEVY_PROFILE_DURATION_SECS (or RE3BEVY_PROFILE_FRAMES above).
    let duration_limit = std::env::var("RE3BEVY_PROFILE_DURATION_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<f32>().ok())
        .or_else(|| {
            (cfg!(feature = "profiling-chrome")
                && std::env::var_os("RE3BEVY_PROFILE_FRAMES").is_none())
            .then_some(90.0)
        });
    if let Some(limit) = duration_limit {
        if *elapsed >= limit {
            eprintln!(
                "re3bevy profiling: reached profile duration limit ({limit}s), exiting cleanly to flush trace"
            );
            exit.write(bevy::app::AppExit::Success);
        }
    }
}

fn configure_gpu_clustering_capacity(settings: Option<ResMut<GlobalClusterSettings>>) {
    let Some(mut settings) = settings else {
        return;
    };
    if settings.gpu_clustering.as_ref().is_some_and(|gpu| gpu.initial_index_list_capacity < 131_072) {
        settings.gpu_clustering.as_mut().unwrap().initial_index_list_capacity = 131_072;
    }
}

fn save_screenshot_on_f12(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut counter: Local<u32>,
) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }

    let output_dir = re3bevy_assets::install::RuntimePaths::diagnostic_root();
    if let Err(error) = std::fs::create_dir_all(output_dir) {
        error!("Cannot create screenshot directory {}: {error}", output_dir.display());
        return;
    }
    let path = loop {
        let candidate = output_dir.join(format!("re3bevy-screenshot-{}.png", *counter));
        *counter += 1;
        if !candidate.exists() {
            break candidate;
        }
    };
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::Schedules;
    use bevy::light::cluster::GlobalClusterGpuSettings;

    #[test]
    fn production_update_schedule_has_no_ordering_cycles() {
        // Build the same plugin graph as the executable. A cargo check cannot
        // catch conflicting before/after edges; Bevy validates them here.
        let mut app = build_app_with_route_and_visibility(GtaBootRoute::Frontend, false, false);
        let mut update = app
            .world_mut()
            .resource_mut::<Schedules>()
            .remove(Update)
            .expect("production Update schedule");
        update.initialize(app.world_mut()).expect("acyclic Update schedule");
    }

    #[test]
    fn gta_cluster_capacity_is_allocated_before_rendering() {
        let mut app = App::new();
        app.insert_resource(GlobalClusterSettings {
            supports_storage_buffers: true,
            clustered_decals_are_usable: false,
            gpu_clustering: Some(GlobalClusterGpuSettings {
                initial_z_slice_list_capacity: 1_024,
                initial_index_list_capacity: 65_536,
            }),
            max_uniform_buffer_clusterable_objects: 204,
            view_cluster_bindings_max_indices: 16_384,
        })
        .add_systems(First, configure_gpu_clustering_capacity);
        app.update();

        assert_eq!(
            app.world()
                .resource::<GlobalClusterSettings>()
                .gpu_clustering
                .unwrap()
                .initial_index_list_capacity,
            131_072
        );
    }
}
