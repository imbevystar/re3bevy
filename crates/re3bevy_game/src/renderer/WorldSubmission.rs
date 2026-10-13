//! Keep resident materials grouped without rebuilding indirect bins per frame.
use super::RendererMaterials::GtaUnlitMaterial;
use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d},
    prelude::*,
    render::{
        ExtractSchedule, MainWorld, RenderApp,
        batching::gpu_preprocessing::GpuPreprocessingSupport,
        render_phase::{BinnedPhaseItem, ViewBinnedRenderPhases},
        render_resource::AsBindGroup,
        renderer::RenderDevice, settings::WgpuFeatures, view::{NoIndirectDrawing, RetainedViewEntity},
    },
};
use std::collections::HashMap;

pub(super) struct WorldSubmissionPlugin;

impl Plugin for WorldSubmissionPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                publish_device_policy
                    .before(bevy::render::camera::extract_cameras)
                    .before(bevy::core_pipeline::core_3d::extract_core_3d_camera_phases)
                    .before(bevy::core_pipeline::core_3d::extract_camera_prepass_phase)
                    .before(bevy::pbr::extract_meshes_for_gpu_building),
            );
        }
    }
}

fn indirect_eligible(
    grouped: bool,
    gpu_preprocessing: bool,
    multi_draw: bool,
    force_direct: bool,
) -> bool {
    grouped && gpu_preprocessing && multi_draw && !force_direct
}

fn publish_device_policy(
    device: Res<RenderDevice>,
    support: Res<GpuPreprocessingSupport>,
    mut main_world: ResMut<MainWorld>,
    mut opaque: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut selected: Local<Option<bool>>,
    mut phase_modes: Local<HashMap<RetainedViewEntity, bool>>,
) {
    if selected.is_none() || device.is_changed() || support.is_changed() {
        let grouped = GtaUnlitMaterial::bindless_supported(&device);
        let multi_draw = device
            .features()
            .contains(WgpuFeatures::MULTI_DRAW_INDIRECT_COUNT);
        // GTA has already selected visible atomics on the CPU. Hardware support
        // alone doesn't justify Bevy's extra per-bin GPU uploads: dense captures
        // spend several milliseconds there. Retain indirect as an explicit A/B
        // option, with identical native visibility and material inputs.
        let force_direct = std::env::var("RE3_GPU_PREPROCESS").as_deref() != Ok("1");
        let indirect = indirect_eligible(
            grouped,
            support.is_culling_supported(),
            multi_draw,
            force_direct,
        );
        info!(
            indirect,
            grouped, multi_draw, force_direct, "world mesh submission policy"
        );
        *selected = Some(indirect);
        phase_modes.clear();
    }
    // Bevy retains the mode used when creating a view's binned phases. Apply
    // the decision before its first extraction, including cameras spawned in
    // Startup or later gameplay stages after First has already run.
    configure_3d_cameras(&mut main_world, selected.expect("device policy selected"));
    let views = main_world
        .query_filtered::<(Entity, Has<NoIndirectDrawing>), With<Camera3d>>()
        .iter(&main_world)
        .map(|(entity, direct)| (RetainedViewEntity::new(entity.into(), None, 0), direct))
        .collect::<Vec<_>>();
    reset_incompatible_phases(&mut opaque, &views, &phase_modes);
    reset_incompatible_phases(&mut alpha_mask, &views, &phase_modes);
    phase_modes.clear();
    phase_modes.extend(views);
}

fn reset_incompatible_phases<BPI: BinnedPhaseItem>(
    phases: &mut ViewBinnedRenderPhases<BPI>,
    views: &[(RetainedViewEntity, bool)],
    previous_modes: &HashMap<RetainedViewEntity, bool>,
) {
    // prepare_for_new_frame retains a phase's original mode even when the
    // camera's NoIndirectDrawing changes. Recreate its derived bins once when
    // the policy differs; Bevy queues the same visible entities into fresh bins.
    for (view, direct) in views {
        if previous_modes.get(view) != Some(direct) {
            phases.remove(view);
        }
    }
}

fn configure_3d_cameras(world: &mut World, indirect: bool) {
    let changes = world
        .query_filtered::<(Entity, Has<NoIndirectDrawing>), With<Camera3d>>()
        .iter(world)
        .filter_map(|(entity, direct)| (indirect == direct).then_some(entity))
        .collect::<Vec<_>>();
    for entity in changes {
        if indirect {
            world.entity_mut(entity).remove::<NoIndirectDrawing>();
        } else {
            world.entity_mut(entity).insert(NoIndirectDrawing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::batching::gpu_preprocessing::GpuPreprocessingMode;

    #[test]
    fn camera_policy_retires_stale_indirect_bins_before_extraction() {
        let view = RetainedViewEntity::new(Entity::from_bits(1).into(), None, 0);
        let mut phases = ViewBinnedRenderPhases::<Opaque3d>::default();
        let mut previous_modes = HashMap::new();
        phases.prepare_for_new_frame(view, GpuPreprocessingMode::Culling);
        // Bevy deliberately retains Culling here despite the new requested mode.
        phases.prepare_for_new_frame(view, GpuPreprocessingMode::PreprocessingOnly);
        reset_incompatible_phases(&mut phases, &[(view, true)], &previous_modes);
        assert!(phases.get(&view).is_none());
        phases.prepare_for_new_frame(view, GpuPreprocessingMode::PreprocessingOnly);
        previous_modes.insert(view, true);
        reset_incompatible_phases(&mut phases, &[(view, true)], &previous_modes);
        assert!(phases.get(&view).is_some());
        reset_incompatible_phases(&mut phases, &[(view, false)], &previous_modes);
        assert!(phases.get(&view).is_none());
    }

    #[test]
    fn unsupported_or_forced_direct_never_selects_indirect() {
        for grouped in [false, true] {
            for preprocessing in [false, true] {
                for multi_draw in [false, true] {
                    assert!(!indirect_eligible(grouped, preprocessing, multi_draw, true));
                    assert_eq!(
                        indirect_eligible(grouped, preprocessing, multi_draw, false),
                        grouped && preprocessing && multi_draw
                    );
                }
            }
        }
    }

    #[test]
    fn policy_changes_and_new_cameras_keep_ui_direct() {
        let mut app = App::new();
        let scene = app
            .world_mut()
            .spawn((Camera3d::default(), NoIndirectDrawing))
            .id();
        let ui = app.world_mut().spawn((Camera2d, NoIndirectDrawing)).id();
        configure_3d_cameras(app.world_mut(), true);
        assert!(!app.world().entity(scene).contains::<NoIndirectDrawing>());
        assert!(app.world().entity(ui).contains::<NoIndirectDrawing>());
        let second = app
            .world_mut()
            .spawn((Camera3d::default(), NoIndirectDrawing))
            .id();
        configure_3d_cameras(app.world_mut(), true);
        assert!(!app.world().entity(second).contains::<NoIndirectDrawing>());
        configure_3d_cameras(app.world_mut(), false);
        for camera in [scene, second, ui] {
            assert!(app.world().entity(camera).contains::<NoIndirectDrawing>());
        }
    }
}
