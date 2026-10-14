//! re3 reference: references/re-GTA/src/renderer/Renderer.cpp
//!
//! Debug view of **GTA logical render passes** (roads / everything-bar-roads /
//! fading / runtime pool), not Bevy PBR prepasses.
//!
//! Earlier this file toggled Depth/Normal/SSAO prepass components. Those only
//! enable buffer writes for other effects — they do **not** replace the final
//! framebuffer, so "PageUp to see depth pass" looked like nothing was
//! rasterizing. The real Bevy raster path is `Mesh3d` + `StandardMaterial`
//! spawned by `GtaBevyRenderPlugin` (`flush_world_render_pass`).

use bevy::prelude::*;
use rustgta3_core::core::Schedules::GtaProcessSet;
use rustgta3_game::renderer::BevyRender::{
    GtaRenderPassPlanState, GtaRuntimePoolMesh, GtaSceneCamera, GtaWorldMesh, GtaWorldRenderPass,
    GtaWorldRenderStats,
};

pub struct GtaRenderPassDebugPlugin;

impl Plugin for GtaRenderPassDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderPassDebugState>()
            // After RenderPrep so filters/tints hit meshes spawned this frame.
            .add_systems(
                Update,
                (
                    cycle_render_pass_debug_mode,
                    apply_gta_pass_visibility,
                    apply_pass_tint_materials,
                    sync_render_pass_debug_overlay,
                )
                    .chain()
                    .after(GtaProcessSet::RenderPrep),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct RenderPassDebugState {
    pub mode: RenderPassDebugMode,
    dirty: bool,
    visible_revision: u32,
}

/// Isolates what re3 would draw in each RenderWare-style pass list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPassDebugMode {
    /// All world + runtime pool meshes (default gameplay view).
    All,
    /// Only `RendererPassPlan.roads` (treadable).
    RoadsOnly,
    /// Only `everything_bar_roads`.
    EverythingBarRoadsOnly,
    /// Only alpha/fade-sorted list.
    FadingOnly,
    /// Only runtime pool (player / vehicles / cutscene / script).
    RuntimePoolOnly,
    /// Keep everything visible but tint materials by pass colour.
    PassTint,
}

#[derive(Component)]
struct RenderPassDebugOverlay;

const RENDER_PASS_DEBUG_MODES: [RenderPassDebugMode; 6] = [
    RenderPassDebugMode::All,
    RenderPassDebugMode::RoadsOnly,
    RenderPassDebugMode::EverythingBarRoadsOnly,
    RenderPassDebugMode::FadingOnly,
    RenderPassDebugMode::RuntimePoolOnly,
    RenderPassDebugMode::PassTint,
];

impl Default for RenderPassDebugState {
    fn default() -> Self {
        Self {
            mode: RenderPassDebugMode::All,
            dirty: true,
            visible_revision: 1,
        }
    }
}

impl RenderPassDebugMode {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All passes",
            Self::RoadsOnly => "Roads only",
            Self::EverythingBarRoadsOnly => "Everything bar roads",
            Self::FadingOnly => "Fading / alpha",
            Self::RuntimePoolOnly => "Runtime pool",
            Self::PassTint => "Pass tint (roads=R bar=G fade=B pool=Y)",
        }
    }

    fn shows_world_pass(self, pass: GtaWorldRenderPass) -> bool {
        match self {
            Self::All | Self::PassTint => true,
            Self::RoadsOnly => pass == GtaWorldRenderPass::Roads,
            Self::EverythingBarRoadsOnly => pass == GtaWorldRenderPass::EverythingBarRoads,
            Self::FadingOnly => pass == GtaWorldRenderPass::FadingIn,
            Self::RuntimePoolOnly => false,
        }
    }

    fn shows_runtime_pool(self) -> bool {
        matches!(self, Self::All | Self::RuntimePoolOnly | Self::PassTint)
    }
}

fn cycle_render_pass_debug_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RenderPassDebugState>,
) {
    let direction = match (
        keys.just_pressed(KeyCode::PageUp),
        keys.just_pressed(KeyCode::PageDown),
        keys.just_pressed(KeyCode::F3),
    ) {
        (true, false, false) => -1,
        (false, true, false) | (false, false, true) => 1,
        _ => 0,
    };
    if direction == 0 {
        return;
    }

    let current = RENDER_PASS_DEBUG_MODES
        .iter()
        .position(|mode| *mode == state.mode)
        .unwrap_or(0) as isize;
    let len = RENDER_PASS_DEBUG_MODES.len() as isize;
    let next = (current + direction).rem_euclid(len) as usize;
    state.mode = RENDER_PASS_DEBUG_MODES[next];
    state.dirty = true;
    state.visible_revision = state.visible_revision.wrapping_add(1);
    info!("GTA render pass debug: {}", state.mode.label());
}

/// Hide/show world + pool meshes according to the selected GTA pass filter.
fn apply_gta_pass_visibility(
    state: Res<RenderPassDebugState>,
    mut world_meshes: Query<(&GtaWorldMesh, &mut Visibility), Without<GtaRuntimePoolMesh>>,
    mut pool_meshes: Query<(&GtaRuntimePoolMesh, &mut Visibility), Without<GtaWorldMesh>>,
) {
    // Meshes are re-spawned every frame; re-apply filter continuously so the
    // mode sticks without depending on `dirty` across despawn/respawn.
    for (mesh, mut visibility) in &mut world_meshes {
        *visibility = if state.mode.shows_world_pass(mesh.pass) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let show_pool = state.mode.shows_runtime_pool();
    for (_mesh, mut visibility) in &mut pool_meshes {
        *visibility = if show_pool {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// PassTint: overwrite materials with solid unlit colours per pass so the user
/// can see that Bevy is rasterizing distinct draw lists.
///
/// Meshes respawn every frame, so we re-apply tints continuously while mode is
/// active (no restore path needed — next frame spawns fresh materials).
fn apply_pass_tint_materials(
    mut commands: Commands,
    state: Res<RenderPassDebugState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tint_handles: Local<Option<PassTintHandles>>,
    world_meshes: Query<(Entity, &GtaWorldMesh), With<Mesh3d>>,
    pool_meshes: Query<Entity, (With<GtaRuntimePoolMesh>, With<Mesh3d>)>,
) {
    if state.mode != RenderPassDebugMode::PassTint {
        return;
    }

    let handles = tint_handles.get_or_insert_with(|| PassTintHandles {
        roads: materials.add(tint_material(Color::srgb(1.0, 0.2, 0.2))),
        bar: materials.add(tint_material(Color::srgb(0.2, 1.0, 0.3))),
        fading: materials.add(tint_material(Color::srgb(0.2, 0.4, 1.0))),
        pool: materials.add(tint_material(Color::srgb(1.0, 0.9, 0.15))),
    });

    for (entity, mesh) in &world_meshes {
        let tint = match mesh.pass {
            GtaWorldRenderPass::Roads => handles.roads.clone(),
            GtaWorldRenderPass::EverythingBarRoads => handles.bar.clone(),
            GtaWorldRenderPass::FadingIn => handles.fading.clone(),
        };
        commands.entity(entity).insert(MeshMaterial3d(tint));
    }
    for entity in &pool_meshes {
        commands
            .entity(entity)
            .insert(MeshMaterial3d(handles.pool.clone()));
    }
}

struct PassTintHandles {
    roads: Handle<StandardMaterial>,
    bar: Handle<StandardMaterial>,
    fading: Handle<StandardMaterial>,
    pool: Handle<StandardMaterial>,
}

fn tint_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

fn sync_render_pass_debug_overlay(
    mut commands: Commands,
    state: Res<RenderPassDebugState>,
    plan_state: Option<Res<GtaRenderPassPlanState>>,
    world_stats: Option<Res<GtaWorldRenderStats>>,
    camera: Query<Entity, With<GtaSceneCamera>>,
    overlay: Query<Entity, With<RenderPassDebugOverlay>>,
    mut last_key: Local<(u32, usize, usize, usize, usize)>,
) {
    let plan_counts = plan_state
        .as_ref()
        .and_then(|p| p.last_plan.as_ref())
        .map(|p| {
            (
                p.roads.len(),
                p.everything_bar_roads.len(),
                p.fading_in_entities.len(),
            )
        })
        .unwrap_or((0, 0, 0));
    let stats_spawned = world_stats
        .as_ref()
        .map(|s| s.spawned_meshes + s.spawned_pool_meshes)
        .unwrap_or(0);
    let key = (
        state.visible_revision,
        plan_counts.0,
        plan_counts.1,
        plan_counts.2,
        stats_spawned,
    );
    if *last_key == key && !overlay.is_empty() {
        return;
    }
    *last_key = key;

    for entity in &overlay {
        commands.entity(entity).despawn();
    }

    let camera_ok = !camera.is_empty();
    let stats_line = world_stats
        .as_ref()
        .map(|s| {
            format!(
                "spawn={} pool={} tex={} miss_geo={} miss_tex={}",
                s.spawned_meshes,
                s.spawned_pool_meshes,
                s.textured_meshes,
                s.skipped_missing_geometry,
                s.skipped_missing_texture
            )
        })
        .unwrap_or_else(|| "spawn=n/a (not InGame)".to_owned());

    let body = format!(
        "GTA PASS DEBUG: {}\n\
         PageUp/PageDown or F3 cycle\n\
         plan roads={} bar={} fade={} | {}\n\
         Camera3d+GtaSceneCamera: {}  | Bevy DOES rasterize Mesh3d",
        state.mode.label(),
        plan_counts.0,
        plan_counts.1,
        plan_counts.2,
        stats_line,
        if camera_ok { "ok" } else { "MISSING" },
    );

    commands
        .spawn((
            RenderPassDebugOverlay,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            BorderColor::all(Color::srgb(0.35, 0.7, 1.0)),
            ZIndex(1000),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(body),
                TextFont {
                    font_size: bevy::prelude::FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.95, 1.0)),
            ));
        });
}
