//! Update resident Bevy mesh allocations when only their contents change.
use bevy::{
    camera::primitives::MeshAabb,
    mesh::MeshVertexBufferLayouts,
    prelude::*,
    render::{
        GpuResourceAppExt, Render, RenderApp, RenderSystems,
        mesh::{
            RenderMesh, RenderMeshBufferInfo,
            allocator::{MeshAllocator, allocate_and_free_meshes},
        },
        render_asset::{ExtractedAssets, RenderAssetBytesPerFrameLimiter, RenderAssets},
        render_resource::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor},
        renderer::{PendingCommandBuffers, RenderDevice, RenderQueue},
    },
};

pub(super) struct PersistentMeshUploadsPlugin;
impl Plugin for PersistentMeshUploadsPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var("RE3_PERSISTENT_MESH_UPLOADS").as_deref() == Ok("0") {
            return;
        }
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<PersistentMeshUploadDiagnostics>()
                .init_gpu_resource::<MeshUploadBatch>()
                .add_systems(
                    Render,
                    upload_resident_mesh_contents
                        .in_set(RenderSystems::PrepareAssets)
                        .before(allocate_and_free_meshes),
                );
            render_app.add_systems(
                Render,
                copy_resident_mesh_batch
                    .in_set(RenderSystems::PrepareAssets)
                    .after(allocate_and_free_meshes),
            );
        }
    }
}

#[derive(Default, Resource)]
pub struct PersistentMeshUploadDiagnostics {
    pub resident_uploads: u64,
    pub uploaded_bytes: u64,
    pub normal_preparations: u64,
    pub unavailable_staging_views: u64,
    pub staging_batches: u64,
    frames: u64,
}

#[derive(Default, Resource)]
struct MeshUploadBatch {
    bytes: Vec<u8>,
    copies: Vec<MeshUploadCopy>,
    buffer: Option<Buffer>,
}

struct MeshUploadCopy {
    mesh: AssetId<Mesh>,
    source_offset: u64,
    size: u64,
    indices: bool,
    element_size: u64,
}

fn append_aligned(bytes: &mut Vec<u8>, size: usize) -> std::ops::Range<usize> {
    let start = bytes.len().next_multiple_of(4);
    let end = start + size.next_multiple_of(4);
    bytes.resize(end, 0);
    start..start + size
}

// Resolve targets after normal allocation: slab growth may replace a buffer
// even when this mesh's own allocation remains resident.
fn copy_resident_mesh_batch(
    mut batch: ResMut<MeshUploadBatch>,
    allocator: Res<MeshAllocator>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut pending: ResMut<PendingCommandBuffers>,
    mut diagnostics: ResMut<PersistentMeshUploadDiagnostics>,
) {
    if batch.bytes.is_empty() {
        return;
    }
    let size = batch.bytes.len() as u64;
    if batch
        .buffer
        .as_ref()
        .is_none_or(|buffer| buffer.size() < size)
    {
        batch.buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("resident mesh upload batch"),
            size: size.next_power_of_two(),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    let buffer = batch.buffer.as_ref().expect("upload buffer allocated");
    queue.write_buffer(buffer, 0, &batch.bytes);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("resident mesh upload copies"),
    });
    for copy in &batch.copies {
        let target = if copy.indices {
            allocator.mesh_index_slice(&copy.mesh)
        } else {
            allocator.mesh_vertex_slice(&copy.mesh)
        }
        .expect("compatible resident allocation must survive slab growth");
        encoder.copy_buffer_to_buffer(
            buffer,
            copy.source_offset,
            &target.buffer,
            target.range.start as u64 * copy.element_size,
            copy.size,
        );
    }
    // PrepareAssets precedes all render graph encoders. Queue staging writes
    // execute before these copies, and the copies precede draws in submission.
    pending.push([encoder.finish()]);
    batch.bytes.clear();
    batch.copies.clear();
    diagnostics.staging_batches += 1;
}

fn compatible(mesh: &Mesh, resident: &RenderMesh, layouts: &mut MeshVertexBufferLayouts) -> bool {
    if mesh.has_morph_targets()
        || resident.has_morph_targets()
        || mesh.count_vertices() != resident.vertex_count as usize
        || mesh.primitive_topology() != resident.primitive_topology()
        || mesh.get_mesh_vertex_buffer_layout(layouts) != resident.layout
    {
        return false;
    }
    match (mesh.indices(), &resident.buffer_info) {
        (None, RenderMeshBufferInfo::NonIndexed) => true,
        (
            Some(indices),
            RenderMeshBufferInfo::Indexed {
                count,
                index_format,
            },
        ) => {
            indices.len() == *count as usize
                && bevy::render::render_resource::IndexFormat::from(indices) == *index_format
        }
        _ => false,
    }
}

fn upload_resident_mesh_contents(
    mut extracted: ResMut<ExtractedAssets<RenderMesh>>,
    mut resident: ResMut<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    mut batch: ResMut<MeshUploadBatch>,
    mut layouts: ResMut<MeshVertexBufferLayouts>,
    budget: Res<RenderAssetBytesPerFrameLimiter>,
    mut diagnostics: ResMut<PersistentMeshUploadDiagnostics>,
) {
    // Bounded uploads belong to Bevy's normal preparation/retry lifecycle.
    if budget.max_bytes.is_some() {
        return;
    }
    let pending = std::mem::take(&mut extracted.extracted);
    for (id, mesh) in pending {
        let updated = if extracted.modified.contains(&id)
            && !extracted.removed.contains(&id)
            && let Some(gpu_mesh) = resident.get_mut(id)
            && compatible(&mesh, gpu_mesh, &mut layouts)
            && allocator.mesh_vertex_slice(&id).is_some()
            && mesh.get_vertex_buffer_size() > 0
        {
            let stride = gpu_mesh.layout.0.layout().array_stride;
            let index_slice = allocator.mesh_index_slice(&id);
            let indices = mesh.get_index_buffer_bytes();
            if indices.is_some() && index_slice.is_none() {
                false
            } else {
                let size = mesh.get_vertex_buffer_size();
                let range = append_aligned(&mut batch.bytes, size);
                mesh.write_packed_vertex_buffer_data((&mut batch.bytes[range.clone()]).into());
                batch.copies.push(MeshUploadCopy {
                    mesh: id,
                    source_offset: range.start as u64,
                    size: size as u64,
                    indices: false,
                    element_size: stride,
                });
                if let Some(bytes) = indices.filter(|bytes| !bytes.is_empty()) {
                    let width = match gpu_mesh.index_format().unwrap() {
                        bevy::render::render_resource::IndexFormat::Uint16 => 2,
                        bevy::render::render_resource::IndexFormat::Uint32 => 4,
                    };
                    let range = append_aligned(&mut batch.bytes, bytes.len());
                    batch.bytes[range.clone()].copy_from_slice(bytes);
                    batch.copies.push(MeshUploadCopy {
                        mesh: id,
                        source_offset: range.start as u64,
                        size: bytes.len().next_multiple_of(4) as u64,
                        indices: true,
                        element_size: width,
                    });
                }
                gpu_mesh.aabb_center = mesh
                    .get_aabb()
                    .map(|aabb| aabb.center.into())
                    .unwrap_or(Vec3::ZERO);
                diagnostics.resident_uploads += 1;
                diagnostics.uploaded_bytes +=
                    size as u64 + indices.map_or(0, |bytes| bytes.len() as u64);
                true
            }
        } else {
            false
        };
        if updated {
            extracted.modified.remove(&id);
            extracted.added.remove(&id);
        } else {
            diagnostics.normal_preparations += 1;
            extracted.extracted.push((id, mesh));
        }
    }
    diagnostics.frames += 1;
    if diagnostics.frames % 600 == 0 {
        info!(
            resident_uploads = diagnostics.resident_uploads,
            staging_batches = diagnostics.staging_batches,
            uploaded_bytes = diagnostics.uploaded_bytes,
            normal_preparations = diagnostics.normal_preparations,
            unavailable_staging_views = diagnostics.unavailable_staging_views,
            "persistent mesh uploads"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        asset::RenderAssetUsages,
        mesh::{BaseMeshPipelineKey, Indices},
        render::render_resource::{IndexFormat, PrimitiveTopology},
    };

    #[test]
    fn mixed_mesh_batch_preserves_packed_bytes_and_index_padding_on_reuse() {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1., 2., 3.]; 3])
        .with_inserted_indices(Indices::U16(vec![0, 1, 2]));
        let mut bytes = Vec::new();
        for _ in 0..2 {
            let vertex = append_aligned(&mut bytes, mesh.get_vertex_buffer_size());
            mesh.write_packed_vertex_buffer_data((&mut bytes[vertex.clone()]).into());
            assert_eq!(&bytes[vertex], mesh.create_packed_vertex_buffer_data());
            let indices = mesh.get_index_buffer_bytes().unwrap();
            let index = append_aligned(&mut bytes, indices.len());
            bytes[index.clone()].copy_from_slice(indices);
            assert_eq!(&bytes[index.clone()], indices);
            assert_eq!(index.start % 4, 0);
            assert_eq!(&bytes[index.end..], &[0, 0]);
            let next = append_aligned(&mut bytes, 4);
            assert_eq!(next.start % 4, 0);
            bytes.fill(255);
            bytes.clear();
        }
    }

    #[test]
    fn content_updates_reuse_but_allocation_or_pipeline_changes_do_not() {
        let mut layouts = MeshVertexBufferLayouts::default();
        let original = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2]));
        let resident = RenderMesh {
            vertex_count: 3,
            aabb_center: Vec3::ZERO,
            buffer_info: RenderMeshBufferInfo::Indexed {
                count: 3,
                index_format: IndexFormat::Uint32,
            },
            key_bits: BaseMeshPipelineKey::from_primitive_topology_and_strip_index(
                PrimitiveTopology::TriangleList,
                Some(IndexFormat::Uint32),
            ),
            layout: original.get_mesh_vertex_buffer_layout(&mut layouts),
        };
        let mut changed = original.clone();
        changed.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[8., 0., 0.], [9., 0., 0.], [8., 1., 0.]],
        );
        assert!(compatible(&changed, &resident, &mut layouts));
        changed.insert_indices(Indices::U16(vec![0, 1, 2]));
        assert!(!compatible(&changed, &resident, &mut layouts));
        changed.insert_indices(Indices::U32(vec![0, 1, 2, 2, 1, 0]));
        assert!(!compatible(&changed, &resident, &mut layouts));
        changed = original.clone();
        changed.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
        assert!(!compatible(&changed, &resident, &mut layouts));
        changed = original;
        changed.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0., 0., 0.]; 4]);
        assert!(!compatible(&changed, &resident, &mut layouts));
    }
}
