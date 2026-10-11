//! Static-world subpixel crack coverage, using the resident original mesh/TXD.
//! Ordinary RenderWare shading remains authoritative. A depth-biased conservative
//! draw behind it covers tiny authored gaps between independently placed surfaces.

use super::{BevyRender::GtaWorldMesh, RendererMaterials::GtaUnlitMaterial};
use bevy::{
    asset::AssetId,
    camera::visibility::NoFrustumCulling,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::{render_resource::WgpuFeatures, renderer::RenderDevice},
};
use std::collections::{HashMap, HashSet};

pub struct WorldSurfaceCoveragePlugin;

impl Plugin for WorldSurfaceCoveragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSurfaceCoverageState>()
            .add_systems(
                Update,
                sync_coverage.after(re3bevy_core::core::Schedules::GtaProcessSet::RenderPrep),
            );
    }
}

#[derive(Component)]
struct CoverageSource {
    child: Option<Entity>,
}

#[derive(Component)]
pub struct WorldSurfaceCoverage;

#[derive(Clone, Copy)]
struct SourceAssets {
    mesh: AssetId<Mesh>,
    material: AssetId<GtaUnlitMaterial>,
}

/// Derived asset membership, maintained from ECS changes/removals even outside gameplay.
#[derive(Resource, Default)]
pub struct WorldSurfaceCoverageState {
    sources: HashMap<Entity, SourceAssets>,
    mesh_users: HashMap<AssetId<Mesh>, HashSet<Entity>>,
    material_users: HashMap<AssetId<GtaUnlitMaterial>, HashSet<Entity>>,
    ground_meshes: HashMap<AssetId<Mesh>, bool>,
    materials: HashMap<AssetId<GtaUnlitMaterial>, Handle<GtaUnlitMaterial>>,
    pending: HashSet<Entity>,
    pub created_draws: u64,
    pub backend_waits: u64,
    pub mesh_waits: u64,
    pub material_waits: u64,
    pub unsupported_backend: u64,
    reported_backend: bool,
}

impl WorldSurfaceCoverageState {
    fn forget(&mut self, entity: Entity) {
        self.pending.remove(&entity);
        if let Some(source) = self.sources.remove(&entity) {
            if let Some(users) = self.mesh_users.get_mut(&source.mesh) {
                users.remove(&entity);
                if users.is_empty() {
                    self.mesh_users.remove(&source.mesh);
                }
            }
            if let Some(users) = self.material_users.get_mut(&source.material) {
                users.remove(&entity);
                if users.is_empty() {
                    self.material_users.remove(&source.material);
                }
            }
        }
    }

    fn track(&mut self, entity: Entity, source: SourceAssets) {
        self.forget(entity);
        self.mesh_users
            .entry(source.mesh)
            .or_default()
            .insert(entity);
        self.material_users
            .entry(source.material)
            .or_default()
            .insert(entity);
        self.sources.insert(entity, source);
    }
}

/// Only meshes with upward-facing authored surfaces need ground seam coverage.
/// This is presentation policy; no source vertex, index, UV or COL is changed.
fn has_ground_surface(mesh: &Mesh) -> bool {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return false;
    };
    let triangle_is_ground = |a: usize, b: usize, c: usize| {
        let (Some(a), Some(b), Some(c)) = (positions.get(a), positions.get(b), positions.get(c))
        else {
            return false;
        };
        let normal = (Vec3::from_array(*b) - Vec3::from_array(*a))
            .cross(Vec3::from_array(*c) - Vec3::from_array(*a));
        normal.y > 0.0 && normal.y * normal.y > normal.length_squared() * 0.5
    };
    match mesh.indices() {
        Some(Indices::U16(indices)) => indices
            .chunks_exact(3)
            .any(|t| triangle_is_ground(t[0] as usize, t[1] as usize, t[2] as usize)),
        Some(Indices::U32(indices)) => indices
            .chunks_exact(3)
            .any(|t| triangle_is_ground(t[0] as usize, t[1] as usize, t[2] as usize)),
        None => (0..positions.len() / 3).any(|t| triangle_is_ground(t * 3, t * 3 + 1, t * 3 + 2)),
    }
}

type ChangedWorldDraws = Or<(
    Added<GtaWorldMesh>,
    Changed<Mesh3d>,
    Changed<MeshMaterial3d<GtaUnlitMaterial>>,
)>;

fn sync_coverage(
    mut commands: Commands,
    device: Option<Res<RenderDevice>>,
    meshes: Res<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut state: ResMut<WorldSurfaceCoverageState>,
    changed: Query<Entity, (With<GtaWorldMesh>, ChangedWorldDraws)>,
    draws: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<GtaUnlitMaterial>,
            Option<&CoverageSource>,
        ),
        With<GtaWorldMesh>,
    >,
    sources: Query<&CoverageSource>,
    mut removed: RemovedComponents<GtaWorldMesh>,
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    mut material_events: MessageReader<AssetEvent<GtaUnlitMaterial>>,
) {
    for entity in removed.read() {
        if let Ok(source) = sources.get(entity) {
            if let Some(child) = source.child {
                commands.entity(child).despawn();
            }
            commands.entity(entity).remove::<CoverageSource>();
        }
        state.forget(entity);
    }
    for event in mesh_events.read() {
        if let AssetEvent::Modified { id } | AssetEvent::Removed { id } = event {
            state.ground_meshes.remove(id);
            if let Some(users) = state.mesh_users.get(id).cloned() {
                state.pending.extend(users);
            }
        }
    }
    for event in material_events.read() {
        if let AssetEvent::Modified { id } | AssetEvent::Removed { id } = event {
            state.materials.remove(id);
            if let Some(users) = state.material_users.get(id).cloned() {
                state.pending.extend(users);
            }
        }
    }
    state.pending.extend(changed.iter());
    let pending = std::mem::take(&mut state.pending);
    for entity in pending {
        let Ok((mesh, material, previous)) = draws.get(entity) else {
            state.forget(entity);
            continue;
        };
        if let Some(child) = previous.and_then(|source| source.child) {
            commands.entity(child).despawn();
        }
        commands
            .entity(entity)
            .insert(CoverageSource { child: None });
        state.track(
            entity,
            SourceAssets {
                mesh: mesh.0.id(),
                material: material.0.id(),
            },
        );
        let Some(source_material) = materials.get(&material.0) else {
            state.material_waits += 1;
            state.pending.insert(entity);
            continue;
        };
        // Alpha/fade draws must not acquire an opaque underlay or change native blending.
        if source_material.alpha_mode != AlphaMode::Opaque {
            continue;
        }
        let ground = if let Some(ground) = state.ground_meshes.get(&mesh.0.id()) {
            *ground
        } else {
            let Some(source_mesh) = meshes.get(&mesh.0) else {
                state.mesh_waits += 1;
                state.pending.insert(entity);
                continue;
            };
            let ground = has_ground_surface(source_mesh);
            state.ground_meshes.insert(mesh.0.id(), ground);
            ground
        };
        if !ground {
            continue;
        }
        let Some(device) = device.as_deref() else {
            state.backend_waits += 1;
            state.pending.insert(entity);
            continue;
        };
        let supported = device
            .features()
            .contains(WgpuFeatures::CONSERVATIVE_RASTERIZATION);
        if !state.reported_backend {
            state.reported_backend = true;
            if supported {
                info!(target: "gta_render", "static world subpixel surface coverage enabled");
            } else {
                warn!(target: "gta_render", "static world subpixel surface coverage unavailable: backend lacks conservative rasterization");
            }
        }
        if !supported {
            state.unsupported_backend += 1;
            continue;
        }
        let coverage = if let Some(handle) = state.materials.get(&material.0.id()) {
            handle.clone()
        } else {
            let mut coverage = source_material.clone();
            coverage.conservative_coverage = true;
            let handle = materials.add(coverage);
            state.materials.insert(material.0.id(), handle.clone());
            handle
        };
        let child = commands
            .spawn((
                WorldSurfaceCoverage,
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(coverage),
                Transform::IDENTITY,
                Visibility::Inherited,
                NoFrustumCulling,
                ChildOf(entity),
            ))
            .id();
        commands
            .entity(entity)
            .insert(CoverageSource { child: Some(child) });
        state.created_draws += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::PrimitiveTopology;

    #[test]
    fn removing_world_draw_releases_child_and_membership_outside_gameplay() {
        let mut app = App::new();
        app.insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<GtaUnlitMaterial>::default())
            .add_message::<AssetEvent<Mesh>>()
            .add_message::<AssetEvent<GtaUnlitMaterial>>()
            .add_plugins(WorldSurfaceCoveragePlugin);
        let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0., 0., 0.], [0., 0., 1.], [1., 0., 0.]],
            )
            .with_inserted_indices(Indices::U32(vec![0, 1, 2])),
        );
        let material = app
            .world_mut()
            .resource_mut::<Assets<GtaUnlitMaterial>>()
            .add(GtaUnlitMaterial::from_gta(
                Color::WHITE,
                None,
                false,
                false,
                [0.; 3],
                0.,
            ));
        let parent = app
            .world_mut()
            .spawn((
                GtaWorldMesh {
                    entity_index: 0,
                    pass: super::super::BevyRender::GtaWorldRenderPass::Roads,
                },
                Mesh3d(mesh),
                MeshMaterial3d(material),
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .resource::<WorldSurfaceCoverageState>()
                .sources
                .contains_key(&parent)
        );
        let child = app
            .world_mut()
            .spawn((WorldSurfaceCoverage, ChildOf(parent)))
            .id();
        app.world_mut()
            .entity_mut(parent)
            .insert(CoverageSource { child: Some(child) });
        app.world_mut().entity_mut(parent).remove::<GtaWorldMesh>();
        app.update();
        assert!(app.world().get_entity(child).is_err());
        let state = app.world().resource::<WorldSurfaceCoverageState>();
        assert!(state.sources.is_empty());
        assert!(state.mesh_users.is_empty());
        assert!(state.material_users.is_empty());
        assert!(state.pending.is_empty());
    }

    #[test]
    fn only_ground_bearing_original_triangles_request_coverage() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [0., 0., 1.], [1., 0., 0.]],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2]));
        assert!(has_ground_surface(&mesh));
        let wall = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [0., 1., 0.], [1., 0., 0.]],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2]));
        assert!(!has_ground_surface(&wall));
        assert_eq!(mesh.indices(), Some(&Indices::U32(vec![0, 1, 2])));
    }
}
