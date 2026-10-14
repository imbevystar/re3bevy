//! Camera viewport bounding and render-pass scissor safety.
//!
//! re3 / RenderWare reference:
//! - `skeleton.cpp` / `win.cpp`: `RsEventHandler(rsWINDOWRESIZE)` updates window
//!   dimensions and re-binds the RenderWare camera raster (`RwCameraSetRaster`).
//! - `librw/src/camera.cpp`: `RwCameraBeginUpdate` sets viewport / scissor rects
//!   constrained strictly by `pRaster->width` and `pRaster->height`.
//! - `core/config.h`: original PC `NUMPLAYERS=1`; rust_gta_3 extends this to two
//!   local players via split-screen camera viewports.
//!
//! When a window is resized (e.g. dragging borders, changing video modes, or moving
//! between monitors with different scale factors), Bevy's main-world `Window.resolution`
//! can update before the GPU swapchain texture has reallocated, or the swapchain may
//! return a buffer with previous frame dimensions. Without strict clamping against
//! the actual backing texture extent, `bevy_core_pipeline`'s upscaling pass passes
//! out-of-bounds scissor rects to `wgpu`, causing a fatal validation error.
//!
//! This module provides:
//! 1. `clamp_viewport_to_target`: safe clamping logic for `Viewport` against physical targets.
//! 2. `clamp_extracted_camera_viewports`: a `RenderApp` system running in `PrepareViews`
//!    that guarantees all `ExtractedCamera.viewport` values are contained within the
//!    active GPU target texture dimensions before any render pass commands are recorded.

use bevy::{
    camera::{NormalizedRenderTarget, Viewport},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_asset::RenderAssets,
        sync_world::MainEntity,
        texture::{GpuImage, ManualTextureViews},
        view::window::ExtractedWindow,
    },
};

/// Clamps a camera [`Viewport`] to fit within `target_size`.
///
/// Returns `true` if the viewport is valid with non-zero dimensions, or `false`
/// if `target_size` has zero width or height (e.g. minimized window).
pub fn clamp_viewport_to_target(viewport: &mut Viewport, target_size: UVec2) -> bool {
    if target_size.x == 0 || target_size.y == 0 {
        viewport.physical_position = UVec2::ZERO;
        viewport.physical_size = UVec2::ZERO;
        return false;
    }
    viewport.clamp_to_size(target_size);
    viewport.physical_size.x > 0 && viewport.physical_size.y > 0
}

/// Render-world system that inspects all [`ExtractedCamera`] viewports and clamps them
/// to the actual backing GPU texture extent of their render target.
///
/// Runs in `RenderSystems::PrepareViews` after `prepare_windows` has acquired the current
/// swapchain texture. This ensures that even during asynchronous window resize events,
/// `set_scissor_rect` commands can never exceed the render target dimensions.
pub fn clamp_extracted_camera_viewports(
    extracted_windows: Query<(MainEntity, &ExtractedWindow)>,
    images: Res<RenderAssets<GpuImage>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut cameras: Query<&mut ExtractedCamera>,
) {
    for mut camera in &mut cameras {
        if camera.viewport.is_none() {
            continue;
        }
        let Some(target) = camera.target.as_ref() else {
            continue;
        };
        let target_size = match target {
            NormalizedRenderTarget::Window(window_ref) => extracted_windows
                .iter()
                .find(|(entity, _)| window_ref.entity() == *entity)
                .and_then(|(_, w)| {
                    if let Some(surface_texture) = &w.swap_chain_texture {
                        Some(UVec2::new(
                            surface_texture.texture.width(),
                            surface_texture.texture.height(),
                        ))
                    } else if w.physical_width > 0 && w.physical_height > 0 {
                        Some(UVec2::new(w.physical_width, w.physical_height))
                    } else {
                        None
                    }
                }),
            NormalizedRenderTarget::Image(image_target) => {
                images.get(&image_target.handle).map(|img| img.size_2d())
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| tex.size)
            }
            NormalizedRenderTarget::None { width, height } => Some(UVec2::new(*width, *height)),
        };

        let Some(target_size) = target_size else {
            continue;
        };

        let phys_size = if let Some(ref mut viewport) = camera.viewport {
            clamp_viewport_to_target(viewport, target_size);
            Some(viewport.physical_size)
        } else {
            None
        };

        if let Some(phys_size) = phys_size
            && let Some(ref mut target_phys_size) = camera.physical_viewport_size
        {
            *target_phys_size = phys_size;
        }
        camera.physical_target_size = Some(target_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_viewport_exceeding_render_target() {
        // Scenario from user error log:
        // Render target is (1920, 1080), but window resolution resized to 1995x1228.
        let target_size = UVec2::new(1920, 1080);

        // Player 1 viewport: pos (0, 0), size (1995, 614)
        let mut p1_viewport = Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(1995, 614),
            depth: 0.0..1.0,
        };
        let valid1 = clamp_viewport_to_target(&mut p1_viewport, target_size);
        assert!(valid1);
        assert_eq!(p1_viewport.physical_position, UVec2::new(0, 0));
        assert_eq!(p1_viewport.physical_size, UVec2::new(1920, 614));
        assert!(
            p1_viewport.physical_position.x + p1_viewport.physical_size.x <= target_size.x,
            "P1 width must be contained in target"
        );
        assert!(
            p1_viewport.physical_position.y + p1_viewport.physical_size.y <= target_size.y,
            "P1 height must be contained in target"
        );

        // Player 2 viewport: pos (0, 614), size (1995, 614)
        let mut p2_viewport = Viewport {
            physical_position: UVec2::new(0, 614),
            physical_size: UVec2::new(1995, 614),
            depth: 0.0..1.0,
        };
        let valid2 = clamp_viewport_to_target(&mut p2_viewport, target_size);
        assert!(valid2);
        assert_eq!(p2_viewport.physical_position, UVec2::new(0, 614));
        assert_eq!(p2_viewport.physical_size, UVec2::new(1920, 466));
        assert!(
            p2_viewport.physical_position.x + p2_viewport.physical_size.x <= target_size.x,
            "P2 width must be contained in target"
        );
        assert!(
            p2_viewport.physical_position.y + p2_viewport.physical_size.y <= target_size.y,
            "P2 height must be contained in target"
        );
    }

    #[test]
    fn test_clamp_viewport_left_right_split() {
        let target_size = UVec2::new(1920, 1080);
        // Window resized to 2200x1080: left pane (0, 0, 1100, 1080), right pane (1100, 0, 1100, 1080)
        let mut right_pane = Viewport {
            physical_position: UVec2::new(1100, 0),
            physical_size: UVec2::new(1100, 1080),
            depth: 0.0..1.0,
        };
        let valid = clamp_viewport_to_target(&mut right_pane, target_size);
        assert!(valid);
        assert_eq!(right_pane.physical_position, UVec2::new(1100, 0));
        assert_eq!(right_pane.physical_size, UVec2::new(820, 1080));
        assert!(right_pane.physical_position.x + right_pane.physical_size.x <= target_size.x);
        assert!(right_pane.physical_position.y + right_pane.physical_size.y <= target_size.y);
    }

    #[test]
    fn test_clamp_viewport_minimized_or_zero_target() {
        let zero_target = UVec2::ZERO;
        let mut viewport = Viewport {
            physical_position: UVec2::new(100, 100),
            physical_size: UVec2::new(800, 600),
            depth: 0.0..1.0,
        };
        let valid = clamp_viewport_to_target(&mut viewport, zero_target);
        assert!(!valid);
        assert_eq!(viewport.physical_position, UVec2::ZERO);
        assert_eq!(viewport.physical_size, UVec2::ZERO);
    }

    #[test]
    fn test_clamp_viewport_inside_target_preserved() {
        let target_size = UVec2::new(1920, 1080);
        let mut viewport = Viewport {
            physical_position: UVec2::new(100, 100),
            physical_size: UVec2::new(800, 600),
            depth: 0.0..1.0,
        };
        let valid = clamp_viewport_to_target(&mut viewport, target_size);
        assert!(valid);
        assert_eq!(viewport.physical_position, UVec2::new(100, 100));
        assert_eq!(viewport.physical_size, UVec2::new(800, 600));
    }
}
