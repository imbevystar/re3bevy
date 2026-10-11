//! CShadows::RenderStoredShadows, CastShadowSectorList and CastShadowEntity.
//! Clip the original texture footprint to building COL triangles, preserving
//! their plane heights and interpolated UVs. GTA coordinates throughout.

use super::CarLightShadowIntent;
use crate::{
    collision::ColModel::CColModel,
    core::World::{CWorld, CWorldEntity, EEntityType, NUMSECTORS_X, world_entity_gta_matrix},
    modelinfo::ModelInfo::CModelInfo,
};
use bevy::prelude::*;
use re3bevy_core::core::Coordinates::CVector;
use std::collections::HashSet;

pub const CAR_LIGHT_Z_DISTANCE: f32 = 6.0;
pub const SHADOW_SURFACE_OFFSET: f32 = 0.03;

#[derive(Debug, Default)]
pub struct ProjectedShadowGeometry {
    pub positions: Vec<CVector>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Default, Resource)]
pub struct ShadowProjectionDiagnostics {
    pub frames: u64,
    pub intents: usize,
    pub receiver_buildings: usize,
    pub tested_triangles: usize,
    pub polygons: usize,
    pub sloped_polygons: usize,
    pub visible_meshes: usize,
    pub no_receiver: usize,
    pub missing_col: u64,
    pub invalid_triangles: u64,
    pub missing_dependencies: u64,
    pub missing_texture: u64,
}

impl ShadowProjectionDiagnostics {
    pub fn begin_frame(&mut self) {
        self.frames += 1;
        self.intents = 0;
        self.receiver_buildings = 0;
        self.tested_triangles = 0;
        self.polygons = 0;
        self.sloped_polygons = 0;
        self.visible_meshes = 0;
        self.no_receiver = 0;
    }
}

#[derive(Clone, Copy)]
struct ClipVertex {
    xy: Vec2,
    uv: Vec2,
}

/// Read only the building/overlap lists of sectors covered by the footprint.
/// The visited set is per projection, equivalent to the native scan code.
pub fn project_car_light(
    light: &CarLightShadowIntent,
    world: &CWorld,
    models: &CModelInfo,
    diagnostics: &mut ShadowProjectionDiagnostics,
) -> ProjectedShadowGeometry {
    project_additive_shadow(light, CAR_LIGHT_Z_DISTANCE, world, models, diagnostics)
}

pub fn project_additive_shadow(
    light: &CarLightShadowIntent,
    z_distance: f32,
    world: &CWorld,
    models: &CModelInfo,
    diagnostics: &mut ShadowProjectionDiagnostics,
) -> ProjectedShadowGeometry {
    let mut geometry = ProjectedShadowGeometry::default();
    let width = light.front[0].abs() + light.side[0].abs();
    let height = light.front[1].abs() + light.side[1].abs();
    let min = [light.position.x - width, light.position.y - height];
    let max = [light.position.x + width, light.position.y + height];
    let mut visited = HashSet::new();
    for y in CWorld::get_sector_index_y(min[1])..=CWorld::get_sector_index_y(max[1]) {
        for x in CWorld::get_sector_index_x(min[0])..=CWorld::get_sector_index_x(max[0]) {
            let sector = &world.sectors[y * NUMSECTORS_X + x];
            for &index in sector.buildings.iter().chain(&sector.buildings_overlap) {
                if !visited.insert(index) {
                    continue;
                }
                let Some(entity) = world.entities.get(index) else {
                    diagnostics.missing_dependencies += 1;
                    continue;
                };
                if !entity.uses_collision || entity.entity_type != EEntityType::Building {
                    continue;
                }
                let Some(col) = models.col_model(entity.model_index) else {
                    diagnostics.missing_col += 1;
                    continue;
                };
                let Some(bounds) = entity.get_bound_rect(models) else {
                    diagnostics.missing_col += 1;
                    continue;
                };
                if min[0] >= bounds[1]
                    || max[0] <= bounds[0]
                    || min[1] >= bounds[3]
                    || max[1] <= bounds[2]
                    || light.position.z - z_distance
                        >= entity.position[2] + col.bounding_box.max.z
                    || entity.position[2] + col.bounding_box.min.z >= light.position.z
                {
                    continue;
                }
                diagnostics.receiver_buildings += 1;
                cast_entity(light, z_distance, entity, col, &mut geometry, diagnostics);
            }
        }
    }
    if geometry.indices.is_empty() {
        diagnostics.no_receiver += 1;
    }
    geometry
}

fn cast_entity(
    light: &CarLightShadowIntent,
    z_distance: f32,
    entity: &CWorldEntity,
    col: &CColModel,
    geometry: &mut ProjectedShadowGeometry,
    diagnostics: &mut ShadowProjectionDiagnostics,
) {
    let matrix = world_entity_gta_matrix(entity);
    let right = Vec2::new(matrix.right.x, matrix.right.y);
    let forward = Vec2::new(matrix.forward.x, matrix.forward.y);
    let relative = Vec2::new(
        light.position.x - entity.position[0],
        light.position.y - entity.position[1],
    );
    let center = Vec2::new(relative.dot(right), relative.dot(forward));
    let light_front = Vec2::from_array(light.front);
    let light_side = Vec2::from_array(light.side);
    let front = Vec2::new(light_front.dot(right), light_front.dot(forward));
    let side = Vec2::new(light_side.dot(right), light_side.dot(forward));
    let footprint = [
        center + front - side,
        center + front + side,
        center - front + side,
        center - front - side,
    ];
    let min = footprint
        .iter()
        .copied()
        .fold(Vec2::splat(f32::INFINITY), Vec2::min);
    let max = footprint
        .iter()
        .copied()
        .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
    let max_z = light.position.z - entity.position[2];
    let min_z = max_z - z_distance;
    let uvs = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
    let mut polygon = Vec::with_capacity(8);
    let mut scratch = Vec::with_capacity(8);
    for (index, triangle) in col.triangles.iter().enumerate() {
        diagnostics.tested_triangles += 1;
        let (Some(plane), Some(a), Some(b), Some(c)) = (
            col.triangle_plane(index),
            col.vertices.get(triangle.a as usize).map(|v| v.get()),
            col.vertices.get(triangle.b as usize).map(|v| v.get()),
            col.vertices.get(triangle.c as usize).map(|v| v.get()),
        ) else {
            diagnostics.invalid_triangles += 1;
            continue;
        };
        if plane.normal.z.abs() <= 0.1 {
            continue;
        }
        if a.x.max(b.x).max(c.x) <= min.x
            || a.x.min(b.x).min(c.x) >= max.x
            || a.y.max(b.y).max(c.y) <= min.y
            || a.y.min(b.y).min(c.y) >= max.y
            || a.z.min(b.z).min(c.z) >= max_z
            || a.z.max(b.z).max(c.z) <= min_z
        {
            continue;
        }
        polygon.clear();
        polygon.extend(
            footprint
                .iter()
                .zip(uvs)
                .map(|(&xy, uv)| ClipVertex { xy, uv }),
        );
        let triangle_xy = [
            Vec2::new(a.x, a.y),
            Vec2::new(b.x, b.y),
            Vec2::new(c.x, c.y),
        ];
        for edge in 0..3 {
            clip_edge(
                &polygon,
                &mut scratch,
                triangle_xy[edge],
                triangle_xy[(edge + 1) % 3],
            );
            std::mem::swap(&mut polygon, &mut scratch);
            if polygon.len() < 3 {
                break;
            }
        }
        if polygon.len() < 3 {
            continue;
        }
        diagnostics.polygons += 1;
        if plane.normal.x.abs() > 0.001 || plane.normal.y.abs() > 0.001 {
            diagnostics.sloped_polygons += 1;
        }
        let base = geometry.positions.len() as u32;
        for vertex in &polygon {
            let local_z =
                (plane.dist - plane.normal.x * vertex.xy.x - plane.normal.y * vertex.xy.y)
                    / plane.normal.z;
            geometry.positions.push(CVector::new(
                entity.position[0] + vertex.xy.x * right.x + vertex.xy.y * forward.x,
                entity.position[1] + vertex.xy.x * right.y + vertex.xy.y * forward.y,
                entity.position[2] + local_z + SHADOW_SURFACE_OFFSET,
            ));
            geometry.uvs.push(vertex.uv.to_array());
        }
        // RenderBuffer uses ShadowIndexList: 0,2,1; 0,3,2; ...
        for vertex in 2..polygon.len() as u32 {
            geometry
                .indices
                .extend([base, base + vertex, base + vertex - 1]);
        }
    }
}

/// Sutherland-Hodgman edge clipping with the original clockwise COL winding
/// and the same linear texture interpolation as CastShadowEntity.
fn clip_edge(input: &[ClipVertex], output: &mut Vec<ClipVertex>, start: Vec2, end: Vec2) {
    output.clear();
    let Some(&last) = input.last() else { return };
    let edge = end - start;
    let mut previous = last;
    let mut previous_distance = (previous.xy - start).perp_dot(edge);
    for &current in input {
        let distance = (current.xy - start).perp_dot(edge);
        if (distance > 0.0) != (previous_distance > 0.0) {
            let fraction = previous_distance.abs() / (previous_distance.abs() + distance.abs());
            output.push(ClipVertex {
                xy: previous.xy.lerp(current.xy, fraction),
                uv: previous.uv.lerp(current.uv, fraction),
            });
        }
        if distance > 0.0 {
            output.push(current);
        }
        previous = current;
        previous_distance = distance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::ColModel::{
        CColBox, CColSphere, CColTriangle, CColVector, CompressedVector,
    };

    fn fixture() -> (CWorld, CModelInfo, CarLightShadowIntent) {
        let mut models = CModelInfo::initialise_empty();
        let mut col = CColModel::new(
            CColSphere::new(16.0, CColVector::new(0.0, 0.0, 0.0), 0, 0),
            CColBox::new(
                CColVector::new(-10.0, -10.0, -1.0),
                CColVector::new(10.0, 10.0, 1.0),
                0,
                0,
            ),
            Vec::new(),
        );
        col.vertices = vec![
            CompressedVector::set(-10.0, -10.0, -1.0),
            CompressedVector::set(-10.0, 10.0, 1.0),
            CompressedVector::set(10.0, 10.0, 1.0),
            CompressedVector::set(10.0, -10.0, -1.0),
        ];
        col.triangles = vec![
            CColTriangle::new(0, 1, 2, 0, 0),
            CColTriangle::new(0, 2, 3, 0, 0),
        ];
        models.add_simple_model(1).base.set_col_model(col, true);
        let mut world = CWorld::default();
        world.entities.push(CWorldEntity {
            entity_type: EEntityType::Building,
            model_index: 1,
            name: "road".into(),
            position: [0.0; 3],
            scale: [1.0; 3],
            rotation_axis: [0.0, 0.0, 1.0],
            rotation_w: 1.0,
            rotation_degrees: 0.0,
            matrix_basis_override: None,
            render_position_offset: [0.0; 3],
            level: 1,
            is_static: true,
            is_visible: true,
            is_big_building: false,
            is_subway: false,
            streaming_dont_delete: false,
            uses_collision: true,
            zone_culled: false,
            zone_culled2: false,
            building_pool_slot: None,
            treadable_pool_slot: None,
            map_object_slot: None,
        });
        world.add(0, &models);
        let light = CarLightShadowIntent {
            key: 22,
            texture: 4,
            position: CVector::new(0.0, 0.0, 2.0),
            front: [0.0, 7.0],
            side: [7.0, 0.0],
            color: [45; 3],
        };
        (world, models, light)
    }

    #[test]
    fn ramp_light_follows_each_col_plane_with_continuous_original_uvs() {
        let (world, models, light) = fixture();
        let mut diagnostics = ShadowProjectionDiagnostics::default();
        let geometry = project_car_light(&light, &world, &models, &mut diagnostics);
        assert_eq!(diagnostics.polygons, 2);
        assert_eq!(
            diagnostics.receiver_buildings, 1,
            "sector overlap must not draw twice"
        );
        assert_eq!(diagnostics.sloped_polygons, 2);
        for (position, uv) in geometry.positions.iter().zip(&geometry.uvs) {
            assert!((position.z - (position.y * 0.1 + SHADOW_SURFACE_OFFSET)).abs() < 0.0001);
            assert!((uv[0] - (position.x + 7.0) / 14.0).abs() < 0.0001);
            assert!((uv[1] - (7.0 - position.y) / 14.0).abs() < 0.0001);
        }
        let area: f32 = geometry
            .indices
            .chunks_exact(3)
            .map(|triangle| {
                let a = geometry.positions[triangle[0] as usize];
                let b = geometry.positions[triangle[1] as usize];
                let c = geometry.positions[triangle[2] as usize];
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5
            })
            .sum();
        assert!(
            (area - 196.0).abs() < 0.001,
            "clipping must cover the footprint without double brightness"
        );
    }

    #[test]
    fn rotated_receiver_retains_its_slope_and_no_flat_quad_crosses_the_road_edge() {
        let (mut world, models, mut light) = fixture();
        world.entities[0].rotation_degrees = 90.0;
        light.position.x = 8.0;
        let geometry = project_car_light(
            &light,
            &world,
            &models,
            &mut ShadowProjectionDiagnostics::default(),
        );
        assert!(!geometry.indices.is_empty());
        for position in geometry.positions {
            assert!(position.x <= 10.001 && position.x >= 0.999);
            assert!((position.z - (-position.x * 0.1 + SHADOW_SURFACE_OFFSET)).abs() < 0.0001);
        }
    }

    #[test]
    fn native_six_metre_volume_and_collisionless_receivers_produce_no_floating_light() {
        let (mut world, models, mut light) = fixture();
        light.position.z = 9.0;
        assert!(
            project_car_light(
                &light,
                &world,
                &models,
                &mut ShadowProjectionDiagnostics::default()
            )
            .indices
            .is_empty()
        );
        light.position.z = 2.0;
        world.entities[0].uses_collision = false;
        assert!(
            project_car_light(
                &light,
                &world,
                &models,
                &mut ShadowProjectionDiagnostics::default()
            )
            .indices
            .is_empty()
        );
    }
}
