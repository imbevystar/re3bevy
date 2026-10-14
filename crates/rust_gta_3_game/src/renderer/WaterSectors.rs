//! Camera-dependent CWaterLevel::RenderWater selection, independent of GPU assets.
use crate::core::Camera::CCamera;
use crate::renderer::WaterLevel::*;
use rust_gta_3_core::core::Coordinates::CVector;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct WaterSector {
    pub x: f32,
    pub y: f32,
    pub level: f32,
    pub size: f32,
}

#[derive(Default)]
pub(super) struct WaterSectorPlan {
    pub flat: Vec<WaterSector>,
    pub wave: Vec<WaterSector>,
}

pub(super) fn water_draw_distance(height: f32) -> f32 {
    (height.clamp(15.0, 60.0) - 15.0) * 800.0 / 45.0 + 1200.0
}

pub(super) fn select_water_sectors(
    water: &CWaterLevel,
    camera: &CCamera,
    wavy_distance: f32,
) -> WaterSectorPlan {
    let mut plan = WaterSectorPlan::default();
    let position = camera.position;
    let cam = &camera.cams[camera.active_cam];
    let frustum = camera.frustum_for_source_and_fov(position, cam.fov);
    let front = frustum.front;
    let mut start_x = false;
    let mut end_x = false;
    let mut start_y = false;
    let mut end_y = false;
    // _GetCamBounds: only discard the dominant rear half when not looking down.
    if front.z > -0.8 {
        if front.x.abs() > front.y.abs() {
            start_x = front.x > 0.0;
            end_x = !start_x;
        } else {
            start_y = front.y > 0.0;
            end_y = !start_y;
        }
    }
    let draw_distance = water_draw_distance(position.z);
    let draw_squared = draw_distance * draw_distance;
    let index = |coordinate: f32| ((coordinate - WATER_START_X) / HUGE_SECTOR_SIZE) as i32;
    let clamp = |index: i32| index.clamp(0, MAX_HUGE_SECTORS as i32 - 1) as usize;
    let min_x = clamp(index(if start_x {
        position.x
    } else {
        position.x - draw_distance
    }));
    let max_x = clamp(if end_x {
        index(position.x)
    } else {
        index(position.x + draw_distance) + 1
    });
    let min_y = clamp(index(if start_y {
        position.y
    } else {
        position.y - draw_distance
    }));
    let max_y = clamp(if end_y {
        index(position.y)
    } else {
        index(position.y + draw_distance) + 1
    });
    let distance_squared = |x: f32, y: f32| (x - position.x).powi(2) + (y - position.y).powi(2);
    let visible = |x: f32, y: f32, size: f32| {
        frustum.is_sphere_visible(CVector::new(x, y, 0.0), size * std::f32::consts::SQRT_2)
    };
    let level = |block: i8| (block >= 0).then(|| water.water_zs[block as usize]);
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            // The native far-sector height is the LAST present coarse block in
            // SW, SE, NW, NE order; it is not an average of the four heights.
            let coarse = [
                (2 * x, 2 * y),
                (2 * x + 1, 2 * y),
                (2 * x, 2 * y + 1),
                (2 * x + 1, 2 * y + 1),
            ];
            let Some(z) = coarse
                .iter()
                .filter_map(|&(cx, cy)| level(water.water_block_list[cx * MAX_LARGE_SECTORS + cy]))
                .last()
            else {
                continue;
            };
            let hx = WATER_START_X + x as f32 * HUGE_SECTOR_SIZE;
            let hy = WATER_START_Y + y as f32 * HUGE_SECTOR_SIZE;
            let distance = distance_squared(hx + 64.0, hy + 64.0);
            if distance >= draw_squared || !visible(hx + 64.0, hy + 64.0, HUGE_SECTOR_SIZE) {
                continue;
            }
            if distance >= 500.0_f32.powi(2) {
                plan.flat.push(WaterSector {
                    x: hx,
                    y: hy,
                    level: z,
                    size: HUGE_SECTOR_SIZE,
                });
                continue;
            }
            for cx in 2 * x..=2 * x + 1 {
                for cy in 2 * y..=2 * y + 1 {
                    let Some(z) = level(water.water_block_list[cx * MAX_LARGE_SECTORS + cy]) else {
                        continue;
                    };
                    let lx = WATER_START_X + cx as f32 * LARGE_SECTOR_SIZE;
                    let ly = WATER_START_Y + cy as f32 * LARGE_SECTOR_SIZE;
                    let distance = distance_squared(lx + 32.0, ly + 32.0);
                    if distance >= draw_squared || !visible(lx + 32.0, ly + 32.0, LARGE_SECTOR_SIZE)
                    {
                        continue;
                    }
                    if distance >= 176.0_f32.powi(2) {
                        plan.flat.push(WaterSector {
                            x: lx,
                            y: ly,
                            level: z,
                            size: LARGE_SECTOR_SIZE,
                        });
                        continue;
                    }
                    for (sx, sy) in [
                        (2 * cx, 2 * cy),
                        (2 * cx + 1, 2 * cy),
                        (2 * cx, 2 * cy + 1),
                        (2 * cx + 1, 2 * cy + 1),
                    ] {
                        let Some(z) =
                            level(water.water_fine_block_list[sx * MAX_SMALL_SECTORS + sy])
                        else {
                            continue;
                        };
                        let x = WATER_START_X + sx as f32 * SMALL_SECTOR_SIZE;
                        let y = WATER_START_Y + sy as f32 * SMALL_SECTOR_SIZE;
                        let sector = WaterSector {
                            x,
                            y,
                            level: z,
                            size: SMALL_SECTOR_SIZE,
                        };
                        if distance_squared(x + 16.0, y + 16.0) < wavy_distance.powi(2) {
                            plan.wave.push(sector);
                        } else {
                            plan.flat.push(sector);
                        }
                    }
                }
            }
        }
    }
    // RenderWater's exterior ocean is executable-authored geometry at z=0.
    // Keep its unusual distance-vs-squared-distance comparison verbatim.
    let mut ocean = |x: f32, y: f32| {
        let cx = x + EXTRAHUGE_SECTOR_SIZE * 0.5;
        let cy = y + EXTRAHUGE_SECTOR_SIZE * 0.5;
        if distance_squared(cx, cy).sqrt() < draw_squared && visible(cx, cy, EXTRAHUGE_SECTOR_SIZE)
        {
            plan.flat.push(WaterSector {
                x,
                y,
                level: 0.0,
                size: EXTRAHUGE_SECTOR_SIZE,
            });
        }
    };
    if !start_y {
        for x in 0..26 {
            for y in 0..5 {
                ocean(
                    WATER_START_X - 1280.0 + x as f32 * EXTRAHUGE_SECTOR_SIZE,
                    WATER_START_Y - 1280.0 + y as f32 * EXTRAHUGE_SECTOR_SIZE,
                );
            }
        }
    }
    for y in 5..21 {
        for x in 0..5 {
            let x = WATER_START_X - 1280.0 + x as f32 * EXTRAHUGE_SECTOR_SIZE;
            let y = WATER_START_Y - 1280.0 + y as f32 * EXTRAHUGE_SECTOR_SIZE;
            if !start_x {
                ocean(x, y);
            }
            if !end_x {
                ocean(-x - EXTRAHUGE_SECTOR_SIZE, y);
            }
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_water() -> CWaterLevel {
        let mut water = CWaterLevel::default();
        water.no_of_water_levels = 1;
        water.water_zs[0] = 6.0;
        water.water_block_list.fill(0);
        water.water_fine_block_list.fill(0);
        water
    }

    fn camera() -> CCamera {
        let mut camera = CCamera::default();
        camera.position = CVector::new(16.0, 16.0, 60.0);
        camera.cams[camera.active_cam].source = camera.position;
        camera.cams[camera.active_cam].front = CVector::new(0.0, 1.0, -0.5);
        camera
    }

    #[test]
    fn native_height_draw_distance_endpoints_and_midpoint() {
        assert_eq!(water_draw_distance(0.0), 1200.0);
        assert_eq!(water_draw_distance(37.5), 1600.0);
        assert_eq!(water_draw_distance(100.0), 2000.0);
    }

    #[test]
    fn fixed_water_camera_uses_presented_pose_instead_of_stale_gameplay_cam() {
        let water = open_water();
        let mut camera = camera();
        camera.SetCamPositionForFixedMode(CVector::new(16.0, -44.0, 46.0), CVector::new(0.0, 60.0, -40.0));
        camera.take_control_no_entity(CVector::new(16.0, 16.0, 6.0));
        camera.cams[camera.active_cam].source = CVector::new(-10000.0, -10000.0, 0.0);
        camera.cams[camera.active_cam].front = CVector::new(-1.0, 0.0, 0.0);
        let plan = select_water_sectors(&water, &camera, 70.0);
        assert!(!plan.flat.is_empty());
        assert!(!plan.wave.is_empty());
        assert!(camera.is_sphere_visible(CVector::new(16.0, 16.0, 6.0), 16.0));
    }

    #[test]
    fn camera_selection_partitions_lods_without_flat_wave_overlap() {
        let water = open_water();
        let plan = select_water_sectors(&water, &camera(), 70.0);
        assert!(!plan.wave.is_empty());
        for size in [SMALL_SECTOR_SIZE, LARGE_SECTOR_SIZE, HUGE_SECTOR_SIZE] {
            assert!(
                plan.flat.iter().any(|sector| sector.size == size),
                "missing size {size}"
            );
        }
        for wave in &plan.wave {
            assert!(
                !plan
                    .flat
                    .iter()
                    .any(|flat| flat.x == wave.x && flat.y == wave.y)
            );
        }
    }

    #[test]
    fn boat_expands_wave_selection_and_ocean_exists_beyond_authored_grid() {
        let water = open_water();
        let camera = camera();
        let normal = select_water_sectors(&water, &camera, 70.0);
        let boat = select_water_sectors(&water, &camera, 120.0);
        assert!(boat.wave.len() > normal.wave.len());
        let mut exterior = camera;
        exterior.position = CVector::new(2100.0, 0.0, 40.0);
        exterior.cams[exterior.active_cam].source = exterior.position;
        exterior.cams[exterior.active_cam].front = CVector::new(1.0, 0.0, -0.3);
        let plan = select_water_sectors(&water, &exterior, 70.0);
        assert!(
            plan.flat
                .iter()
                .any(|sector| sector.size == EXTRAHUGE_SECTOR_SIZE && sector.level == 0.0)
        );
    }
}
