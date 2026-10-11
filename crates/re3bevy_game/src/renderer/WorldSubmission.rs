//! Grouped resident materials allow Bevy to submit world meshes with multi-draw.
use super::RendererMaterials::GtaUnlitMaterial;
use bevy::{
    prelude::*,
    render::{
        ExtractSchedule, MainWorld, RenderApp,
        batching::gpu_preprocessing::GpuPreprocessingSupport, render_resource::AsBindGroup,
        renderer::RenderDevice, settings::WgpuFeatures, view::NoIndirectDrawing,
    },
};

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
    mut selected: Local<Option<bool>>,
) {
    if selected.is_none() || device.is_changed() || support.is_changed() {
        let grouped = GtaUnlitMaterial::bindless_supported(&device);
        let multi_draw = device
            .features()
            .contains(WgpuFeatures::MULTI_DRAW_INDIRECT_COUNT);
        let force_direct = std::env::var("RE3_GPU_PREPROCESS").as_deref() == Ok("0");
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
    }
    // Bevy retains the mode used when creating a view's binned phases. Apply
    // the decision before its first extraction, including cameras spawned in
    // Startup or later gameplay stages after First has already run.
    configure_3d_cameras(&mut main_world, selected.expect("device policy selected"));
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
