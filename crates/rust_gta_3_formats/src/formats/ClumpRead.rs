//! re3 reference: references/re-GTA/src/rw/ClumpRead.cpp
//!
//! GTA clump stream reader metadata. This mirrors the split-load decisions in
//! `RpClumpGtaStreamRead1/2` without constructing RenderWare pointers yet.

pub const RW_GEOMETRY_TRISTRIP: u32 = 0x01;
pub const RW_GEOMETRY_POSITIONS: u32 = 0x02;
pub const RW_GEOMETRY_TEXTURED: u32 = 0x04;
pub const RW_GEOMETRY_PRELIT: u32 = 0x08;
pub const RW_GEOMETRY_NORMALS: u32 = 0x10;
pub const RW_GEOMETRY_TEXTURED2: u32 = 0x80;
pub const RW_GEOMETRY_NATIVE: u32 = 0x01000000;

pub const RW_ID_STRUCT: u32 = 0x01;
pub const RW_ID_STRING: u32 = 0x02;
pub const RW_ID_EXTENSION: u32 = 0x03;
pub const RW_ID_TEXTURE: u32 = 0x06;
pub const RW_ID_MATERIAL: u32 = 0x07;
/// Criterion MatFX material extension (not the Atomic MatFX flag).
pub const RW_ID_MATFX: u32 = 0x120;
pub const RW_ID_MATLIST: u32 = 0x08;
pub const RW_ID_FRAMELIST: u32 = 0x0E;
pub const RW_ID_GEOMETRY: u32 = 0x0F;
pub const RW_ID_CLUMP: u32 = 0x10;
pub const RW_ID_ATOMIC: u32 = 0x14;
pub const RW_ID_GEOMETRYLIST: u32 = 0x1A;
/// Standard RenderWare Skin PLG (`rwID_SKINPLUGIN`).
pub const RW_ID_SKIN: u32 = 0x116;
/// Standard RenderWare HAnim frame plugin (`rwID_HANIMPLUGIN`).
pub const RW_ID_HANIM: u32 = 0x11E;

/// re3 / VehicleModelInfo primary remap colour key (R,G,B).
pub const VEHICLE_COLOUR_PRIMARY_KEY: [u8; 3] = [0x3C, 0xFF, 0x00];
/// re3 / VehicleModelInfo secondary remap colour key (R,G,B).
pub const VEHICLE_COLOUR_SECONDARY_KEY: [u8; 3] = [0xFF, 0x00, 0xAF];
pub const RW_CHUNK_HEADER_LEN: usize = 12;
pub const RW_VENDORID_ROCKSTAR: u32 = 0x0253F2;
pub const RW_ID_NODENAME: u32 = make_chunk_id(RW_VENDORID_ROCKSTAR, 0xFE);

pub const fn make_chunk_id(vendor_id: u32, chunk_id: u32) -> u32 {
    ((vendor_id & 0xFF_FFFF) << 8) | (chunk_id & 0xFF)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RwChunkHeader {
    pub id: u32,
    pub length: u32,
    pub version: u32,
    pub data_start: usize,
    pub data_end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwDecodedGeometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub prelit_colors: Vec<[u8; 4]>,
    pub tex_coords: Vec<Vec<[f32; 2]>>,
    pub indices: Vec<u32>,
    pub triangle_materials: Vec<u16>,
    pub bounding_sphere: Option<[f32; 4]>,
}

/// RenderWare material used by geometry (texture name + base colour).
#[derive(Debug, Clone, PartialEq)]
pub struct RwMaterialDescriptor {
    pub color: [u8; 4],
    pub textured: bool,
    pub ambient: f32,
    pub specular: f32,
    pub diffuse: f32,
    pub texture_name: Option<String>,
    pub mask_name: Option<String>,
    /// MatFX DUAL second texture, if the material carries effect 4.
    pub dual_texture_name: Option<String>,
}

impl RwMaterialDescriptor {
    /// re3 `CVehicleModelInfo` editable material slot: primary (1) or secondary (2).
    pub fn vehicle_colour_slot(&self) -> Option<u8> {
        let [r, g, b, _] = self.color;
        if [r, g, b] == VEHICLE_COLOUR_PRIMARY_KEY {
            Some(1)
        } else if [r, g, b] == VEHICLE_COLOUR_SECONDARY_KEY {
            Some(2)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwSkinDescriptor {
    /// `rw::Skin::{numBones,numUsedBones,numWeights}`.
    pub num_bones: u8,
    pub used_bones: Vec<u8>,
    pub max_vertex_weights: u8,
    /// One four-byte RenderWare index tuple for every geometry vertex.
    pub vertex_bone_indices: Vec<[u8; 4]>,
    /// One four-float RenderWare weight tuple for every geometry vertex.
    pub vertex_bone_weights: Vec<[f32; 4]>,
    /// RenderWare `skinToBone` matrices, in native RW right/up/at/position rows.
    pub skin_to_bone_matrices: Vec<[[f32; 4]; 4]>,
    /// Platform-neutral skin split header present in post-3.4 streams.
    pub bone_limit: Option<i32>,
    pub num_meshes: Option<i32>,
    pub rle_size: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwGeometryDescriptor {
    pub flags: u32,
    pub raw_flags: u32,
    pub num_triangles: i32,
    pub num_vertices: i32,
    pub num_morph_targets: i32,
    pub num_materials: i32,
    pub num_tex_coord_sets: i32,
    pub chunk_version: u32,
    pub chunk_length: u32,
    pub decoded: Option<RwDecodedGeometry>,
    /// Materials from geometry MATLIST (texture names for correct TXD bind).
    pub materials: Vec<RwMaterialDescriptor>,
    /// Optional Skin PLG decoded directly from this geometry's extension.
    pub skin: Option<RwSkinDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwAtomicDescriptor {
    pub frame_index: i32,
    pub geometry_index: i32,
    pub flags: i32,
    pub frame_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwFrameDescriptor {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
    pub position: [f32; 3],
    pub parent_index: i32,
    pub matrix_flags: i32,
    pub name: Option<String>,
    /// `rw::HAnimData::id`; hierarchy roots additionally contribute node IDs
    /// to [`RwClumpSummary::hanim_node_ids`].
    pub hanim_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BevyRwMeshDescriptor {
    pub geometry_index: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub index_count: usize,
    pub material_slots: usize,
    pub has_positions: bool,
    pub has_normals: bool,
    pub has_prelit_colors: bool,
    pub tex_coord_sets: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RwHAnimHierarchyNode {
    pub id: i32,
    pub index: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwClumpSummary {
    pub num_atomics: i32,
    pub num_lights: i32,
    pub num_cameras: i32,
    pub num_frames: i32,
    pub num_geometries: i32,
    pub first_phase_geometries: i32,
    pub atomic_chunks: i32,
    pub root_frame_name: Option<String>,
    pub frame_names: Vec<Option<String>>,
    pub frames: Vec<RwFrameDescriptor>,
    /// HAnim hierarchy nodes in interpolator/matrix-array order. RenderWare
    /// uses the serialized PUSH/POP flags to build the skin matrix palette.
    pub hanim_nodes: Vec<RwHAnimHierarchyNode>,
    pub hanim_hierarchy_flags: i32,
    pub hanim_root_frame_index: Option<usize>,
    /// Skin palette index → HAnim node ID from the hierarchy-root frame.
    pub hanim_node_ids: Vec<i32>,
    pub geometries: Vec<RwGeometryDescriptor>,
    pub atomics: Vec<RwAtomicDescriptor>,
    pub bevy_meshes: Vec<BevyRwMeshDescriptor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwClumpReadState {
    pub summary: RwClumpSummary,
    pub stream_position: usize,
}

pub fn rw_stream_find_chunk(bytes: &[u8], chunk_type: u32) -> Option<RwChunkHeader> {
    let mut cursor = 0;
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() {
        let header = read_chunk_header(bytes, cursor)?;
        if header.id == chunk_type {
            return Some(header);
        }
        cursor = header.data_end;
    }
    None
}

pub fn read_clump(bytes: &[u8]) -> Option<RwClumpSummary> {
    let state = gta_clump_stream_read1(bytes)?;
    gta_clump_stream_read2(bytes, &state)
}

pub fn gta_clump_stream_read1(bytes: &[u8]) -> Option<RwClumpReadState> {
    let clump = rw_stream_find_chunk(bytes, RW_ID_CLUMP)?;
    let payload = &bytes[clump.data_start..clump.data_end];

    let struct_chunk = rw_stream_find_chunk(payload, RW_ID_STRUCT)?;
    let struct_payload = &payload[struct_chunk.data_start..struct_chunk.data_end];
    let num_atomics = read_i32_le(struct_payload, 0)?;
    let (num_lights, num_cameras) =
        if rw_library_version(struct_chunk.version) >= 0x33000 && struct_payload.len() >= 12 {
            (
                read_i32_le(struct_payload, 4)?,
                read_i32_le(struct_payload, 8)?,
            )
        } else {
            (0, 0)
        };

    let frame_list = rw_stream_find_chunk(payload, RW_ID_FRAMELIST)?;
    let frame_payload = &payload[frame_list.data_start..frame_list.data_end];
    let frames = read_frame_descriptors(frame_payload);
    let frame_names: Vec<Option<String>> = frames.iter().map(|frame| frame.name.clone()).collect();
    let (hanim_nodes, hanim_hierarchy_flags, hanim_root_frame_index) =
        read_hanim_hierarchy_nodes(frame_payload);
    let hanim_node_ids = hanim_nodes.iter().map(|node| node.id).collect();
    let num_frames = read_frame_count(frame_payload)?;

    let geometry_list = rw_stream_find_chunk(payload, RW_ID_GEOMETRYLIST);
    let mut num_geometries = 0;
    let mut geometries = Vec::new();
    let mut stream_position = clump.data_start + frame_list.data_end;
    if let Some(geometry_list) = geometry_list {
        let geometry_payload = &payload[geometry_list.data_start..geometry_list.data_end];
        let geometry_struct = rw_stream_find_chunk(geometry_payload, RW_ID_STRUCT)?;
        num_geometries = read_i32_le(
            &geometry_payload[geometry_struct.data_start..geometry_struct.data_end],
            0,
        )?;
        let first_phase_geometries = num_geometries / 2;
        let mut cursor = 0;
        while cursor + RW_CHUNK_HEADER_LEN <= geometry_payload.len()
            && geometries.len() < first_phase_geometries as usize
        {
            let Some(header) = read_chunk_header(geometry_payload, cursor) else {
                break;
            };
            if header.id == RW_ID_GEOMETRY {
                if let Some(descriptor) = read_geometry_descriptor(
                    &geometry_payload[header.data_start..header.data_end],
                    header,
                ) {
                    geometries.push(descriptor);
                }
            }
            cursor = header.data_end;
        }
        stream_position = clump.data_start + geometry_list.data_start + cursor;
    }

    Some(RwClumpReadState {
        summary: RwClumpSummary {
            num_atomics,
            num_lights,
            num_cameras,
            num_frames,
            num_geometries,
            first_phase_geometries: num_geometries / 2,
            atomic_chunks: 0,
            root_frame_name: frame_names.first().cloned().flatten(),
            frame_names,
            frames,
            hanim_nodes,
            hanim_hierarchy_flags,
            hanim_root_frame_index,
            hanim_node_ids,
            bevy_meshes: bevy_mesh_descriptors(&geometries),
            geometries,
            atomics: Vec::new(),
        },
        stream_position,
    })
}

pub fn gta_clump_stream_read2(bytes: &[u8], state: &RwClumpReadState) -> Option<RwClumpSummary> {
    let clump = rw_stream_find_chunk(bytes, RW_ID_CLUMP)?;
    let payload = &bytes[clump.data_start..clump.data_end];

    let mut summary = state.summary.clone();

    let mut geometries = summary.geometries.clone();
    if let Some(geometry_list) = rw_stream_find_chunk(payload, RW_ID_GEOMETRYLIST) {
        let mut cursor = state.stream_position;
        let geometry_list_end = clump.data_start + geometry_list.data_end;
        while cursor + RW_CHUNK_HEADER_LEN <= geometry_list_end
            && geometries.len() < summary.num_geometries as usize
        {
            let Some(header) = read_chunk_header(bytes, cursor) else {
                break;
            };
            if header.id == RW_ID_GEOMETRY {
                if let Some(descriptor) =
                    read_geometry_descriptor(&bytes[header.data_start..header.data_end], header)
                {
                    geometries.push(descriptor);
                }
            }
            cursor = header.data_end;
        }
    }
    // RpClumpGtaStreamRead2 performs numAtomics RwStreamFindChunk calls on
    // the original stream. GTA III RW 3.1 files such as cs_truk.dff report
    // a clump length ending inside the final atomic; bounding this read by
    // that outer length drops the authored body while keeping its doors.
    // Child chunk lengths and the declared atomic count still bound reads.
    summary.atomics = collect_atomic_descriptors(
        &bytes[clump.data_start..],
        &summary.frame_names,
        &mut geometries,
        usize::try_from(summary.num_atomics).ok()?,
    );
    if summary.atomics.len() != summary.num_atomics as usize {
        return None;
    }
    summary.atomic_chunks = summary.atomics.len() as i32;
    summary.geometries = geometries;
    summary.bevy_meshes = bevy_mesh_descriptors(&summary.geometries);
    Some(summary)
}

pub fn read_chunk_header(bytes: &[u8], cursor: usize) -> Option<RwChunkHeader> {
    if cursor + RW_CHUNK_HEADER_LEN > bytes.len() {
        return None;
    }
    let id = read_u32_le(bytes, cursor)?;
    let length = read_u32_le(bytes, cursor + 4)?;
    let version = read_u32_le(bytes, cursor + 8)?;
    let data_start = cursor + RW_CHUNK_HEADER_LEN;
    let data_end = data_start.checked_add(length as usize)?;
    if data_end > bytes.len() {
        return None;
    }
    Some(RwChunkHeader {
        id,
        length,
        version,
        data_start,
        data_end,
    })
}

pub fn count_chunks(bytes: &[u8], chunk_type: u32) -> usize {
    let mut cursor = 0;
    let mut count = 0;
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() {
        let Some(header) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if header.id == chunk_type {
            count += 1;
        }
        count += count_chunks(&bytes[header.data_start..header.data_end], chunk_type);
        cursor = header.data_end;
    }
    count
}

fn read_frame_count(bytes: &[u8]) -> Option<i32> {
    let struct_chunk = rw_stream_find_chunk(bytes, RW_ID_STRUCT)?;
    read_i32_le(&bytes[struct_chunk.data_start..struct_chunk.data_end], 0)
}

fn read_frame_names(bytes: &[u8]) -> Vec<Option<String>> {
    let Some(struct_chunk) = rw_stream_find_chunk(bytes, RW_ID_STRUCT) else {
        return Vec::new();
    };
    let struct_payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let Some(num_frames) = read_i32_le(struct_payload, 0) else {
        return Vec::new();
    };
    let mut names = Vec::with_capacity(num_frames.max(0) as usize);
    let mut cursor = struct_chunk.data_end;
    for _ in 0..num_frames.max(0) {
        let Some(extension) = read_chunk_header(bytes, cursor) else {
            names.push(None);
            continue;
        };
        if extension.id != RW_ID_EXTENSION {
            names.push(None);
            cursor = extension.data_end;
            continue;
        }
        names.push(read_node_name_extension(
            &bytes[extension.data_start..extension.data_end],
        ));
        cursor = extension.data_end;
    }
    names
}

/// Read the per-frame `rw::HAnimData::id`. The hierarchy root has a non-zero
/// node count; child frames only carry their ID and are matched by that ID just
/// like librw `HAnimHierarchy::attachByIndex`.
fn read_frame_hanim_ids(bytes: &[u8]) -> Vec<Option<i32>> {
    let Some(struct_chunk) = rw_stream_find_chunk(bytes, RW_ID_STRUCT) else {
        return Vec::new();
    };
    let payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let Some(count) = read_i32_le(payload, 0) else {
        return Vec::new();
    };
    let mut ids = Vec::with_capacity(count.max(0) as usize);
    let mut cursor = struct_chunk.data_end;
    for _ in 0..count.max(0) {
        let Some(extension) = read_chunk_header(bytes, cursor) else {
            ids.push(None);
            continue;
        };
        ids.push(
            (extension.id == RW_ID_EXTENSION)
                .then(|| read_hanim_extension_id(&bytes[extension.data_start..extension.data_end]))
                .flatten(),
        );
        cursor = extension.data_end;
    }
    ids
}

fn read_hanim_extension_id(bytes: &[u8]) -> Option<i32> {
    let plugin = rw_stream_find_chunk(bytes, RW_ID_HANIM)?;
    let payload = &bytes[plugin.data_start..plugin.data_end];
    // `rw::HAnimData`: version, id, numNodes, ...
    read_i32_le(payload, 4)
}

fn read_hanim_hierarchy_nodes(bytes: &[u8]) -> (Vec<RwHAnimHierarchyNode>, i32, Option<usize>) {
    let Some(struct_chunk) = rw_stream_find_chunk(bytes, RW_ID_STRUCT) else {
        return (Vec::new(), 0, None);
    };
    let payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let Some(count) = read_i32_le(payload, 0) else {
        return (Vec::new(), 0, None);
    };
    let mut cursor = struct_chunk.data_end;
    for frame_index in 0..count.max(0) as usize {
        let Some(extension) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if extension.id == RW_ID_EXTENSION {
            let extension_bytes = &bytes[extension.data_start..extension.data_end];
            if let Some(plugin) = rw_stream_find_chunk(extension_bytes, RW_ID_HANIM) {
                let plugin_bytes = &extension_bytes[plugin.data_start..plugin.data_end];
                let Some(node_count) = read_i32_le(plugin_bytes, 8) else {
                    cursor = extension.data_end;
                    continue;
                };
                if node_count > 0 {
                    // ver/id/numNodes/flags/maxKeySize followed by
                    // (nodeId, serializedIndex, nodeFlags) triples.
                    let hierarchy_flags = read_i32_le(plugin_bytes, 12).unwrap_or(0);
                    let mut nodes = Vec::with_capacity(node_count as usize);
                    for node in 0..node_count as usize {
                        let offset = 20 + node * 12;
                        let Some(id) = read_i32_le(plugin_bytes, offset) else {
                            return (Vec::new(), 0, None);
                        };
                        let Some(index) = read_i32_le(plugin_bytes, offset + 4) else {
                            return (Vec::new(), 0, None);
                        };
                        let Some(flags) = read_i32_le(plugin_bytes, offset + 8) else {
                            return (Vec::new(), 0, None);
                        };
                        nodes.push(RwHAnimHierarchyNode { id, index, flags });
                    }
                    return (nodes, hierarchy_flags, Some(frame_index));
                }
            }
        }
        cursor = extension.data_end;
    }
    (Vec::new(), 0, None)
}

fn read_frame_descriptors(bytes: &[u8]) -> Vec<RwFrameDescriptor> {
    const FRAME_STREAM_DATA_SIZE: usize = 56;

    let Some(struct_chunk) = rw_stream_find_chunk(bytes, RW_ID_STRUCT) else {
        return Vec::new();
    };
    let struct_payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let Some(num_frames) = read_i32_le(struct_payload, 0) else {
        return Vec::new();
    };
    let frame_names = read_frame_names(bytes);
    let frame_hanim_ids = read_frame_hanim_ids(bytes);
    let mut frames = Vec::with_capacity(num_frames.max(0) as usize);
    for index in 0..num_frames.max(0) as usize {
        let offset = 4 + index * FRAME_STREAM_DATA_SIZE;
        let Some(frame) = read_frame_stream_data(
            struct_payload,
            offset,
            frame_names.get(index).cloned().flatten(),
            frame_hanim_ids.get(index).copied().flatten(),
        ) else {
            break;
        };
        frames.push(frame);
    }
    frames
}

fn read_frame_stream_data(
    bytes: &[u8],
    offset: usize,
    name: Option<String>,
    hanim_id: Option<i32>,
) -> Option<RwFrameDescriptor> {
    Some(RwFrameDescriptor {
        right: [
            read_f32_le(bytes, offset)?,
            read_f32_le(bytes, offset + 4)?,
            read_f32_le(bytes, offset + 8)?,
        ],
        up: [
            read_f32_le(bytes, offset + 12)?,
            read_f32_le(bytes, offset + 16)?,
            read_f32_le(bytes, offset + 20)?,
        ],
        at: [
            read_f32_le(bytes, offset + 24)?,
            read_f32_le(bytes, offset + 28)?,
            read_f32_le(bytes, offset + 32)?,
        ],
        position: [
            read_f32_le(bytes, offset + 36)?,
            read_f32_le(bytes, offset + 40)?,
            read_f32_le(bytes, offset + 44)?,
        ],
        parent_index: read_i32_le(bytes, offset + 48)?,
        matrix_flags: read_i32_le(bytes, offset + 52)?,
        name,
        hanim_id,
    })
}

fn read_node_name_extension(bytes: &[u8]) -> Option<String> {
    let mut cursor = 0;
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() {
        let header = read_chunk_header(bytes, cursor)?;
        if header.id == RW_ID_NODENAME {
            let payload = &bytes[header.data_start..header.data_end];
            let len = payload
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(payload.len());
            return Some(String::from_utf8_lossy(&payload[..len]).into_owned());
        }
        cursor = header.data_end;
    }
    None
}

fn read_geometry_descriptor(bytes: &[u8], header: RwChunkHeader) -> Option<RwGeometryDescriptor> {
    let struct_chunk = rw_stream_find_chunk(bytes, RW_ID_STRUCT)?;
    let struct_payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let raw_flags = read_u32_le(struct_payload, 0)?;
    let flags = raw_flags & 0x00FF_FFFF;
    let num_triangles = read_i32_le(struct_payload, 4)?;
    let num_vertices = read_i32_le(struct_payload, 8)?;
    let num_morph_targets = read_i32_le(struct_payload, 12)?;
    let num_tex_coord_sets = geometry_tex_coord_sets(raw_flags);
    let geometry_surface_props = if geometry_has_pre34_surface_props(struct_chunk.version) {
        Some([
            read_f32_le(struct_payload, 16)?,
            read_f32_le(struct_payload, 20)?,
            read_f32_le(struct_payload, 24)?,
        ])
    } else {
        None
    };
    let materials = rw_stream_find_chunk(bytes, RW_ID_MATLIST)
        .map(|matlist| {
            read_material_list(
                &bytes[matlist.data_start..matlist.data_end],
                geometry_surface_props,
            )
        })
        .unwrap_or_default();
    let num_materials = materials.len() as i32;

    Some(RwGeometryDescriptor {
        flags,
        raw_flags,
        num_triangles,
        num_vertices,
        num_morph_targets,
        num_materials,
        num_tex_coord_sets,
        chunk_version: header.version,
        chunk_length: header.length,
        decoded: decode_geometry_struct(
            struct_payload,
            struct_chunk.version,
            flags,
            raw_flags,
            num_vertices,
            num_triangles,
            num_morph_targets,
            num_tex_coord_sets,
            num_materials,
        ),
        materials,
        skin: read_geometry_skin(bytes, num_vertices.max(0) as usize),
    })
}

/// Parse a geometry's standard Skin PLG. Layout and old-format handling mirror
/// librw `readSkin`: indices precede weights; old (<= 3.3) streams put a
/// `0xDEADDEAD` word before every inverse matrix.
fn read_geometry_skin(bytes: &[u8], num_vertices: usize) -> Option<RwSkinDescriptor> {
    let extension = rw_stream_find_chunk(bytes, RW_ID_EXTENSION)?;
    let extension_bytes = &bytes[extension.data_start..extension.data_end];
    let skin = rw_stream_find_chunk(extension_bytes, RW_ID_SKIN)?;
    let payload = &extension_bytes[skin.data_start..skin.data_end];
    if payload.len() < 4 {
        return None;
    }
    let num_bones = payload[0];
    let old_format = payload[1] == 0;
    let used_count = if old_format { num_bones } else { payload[1] };
    let max_vertex_weights = payload[2];
    let mut cursor = 4usize;
    let mut used_bones = Vec::new();
    if !old_format {
        let end = cursor.checked_add(used_count as usize)?;
        used_bones.extend_from_slice(payload.get(cursor..end)?);
        cursor = end;
    }

    let index_bytes = num_vertices.checked_mul(4)?;
    let index_end = cursor.checked_add(index_bytes)?;
    let raw_indices = payload.get(cursor..index_end)?;
    let vertex_bone_indices = raw_indices
        .chunks_exact(4)
        .map(|indices| [indices[0], indices[1], indices[2], indices[3]])
        .collect();
    cursor = index_end;

    let weight_bytes = num_vertices.checked_mul(16)?;
    let weight_end = cursor.checked_add(weight_bytes)?;
    let raw_weights = payload.get(cursor..weight_end)?;
    let mut vertex_bone_weights = Vec::with_capacity(num_vertices);
    for weights in raw_weights.chunks_exact(16) {
        vertex_bone_weights.push([
            read_f32_le(weights, 0)?,
            read_f32_le(weights, 4)?,
            read_f32_le(weights, 8)?,
            read_f32_le(weights, 12)?,
        ]);
    }
    cursor = weight_end;

    let mut skin_to_bone_matrices = Vec::with_capacity(num_bones as usize);
    for _ in 0..num_bones {
        if old_format {
            cursor = cursor.checked_add(4)?;
        }
        let end = cursor.checked_add(64)?;
        let matrix = payload.get(cursor..end)?;
        let mut rw_matrix = [[0.0; 4]; 4];
        for (row, values) in rw_matrix.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                *value = read_f32_le(matrix, (row * 4 + column) * 4)?;
            }
        }
        skin_to_bone_matrices.push(rw_matrix);
        cursor = end;
    }

    let (bone_limit, num_meshes, rle_size) = if old_format || cursor + 12 > payload.len() {
        (None, None, None)
    } else {
        (
            read_i32_le(payload, cursor),
            read_i32_le(payload, cursor + 4),
            read_i32_le(payload, cursor + 8),
        )
    };
    Some(RwSkinDescriptor {
        num_bones,
        used_bones,
        max_vertex_weights,
        vertex_bone_indices,
        vertex_bone_weights,
        skin_to_bone_matrices,
        bone_limit,
        num_meshes,
        rle_size,
    })
}

/// Parse RenderWare MaterialList → material colour + optional texture name.
fn read_material_list(
    bytes: &[u8],
    geometry_surface_props: Option<[f32; 3]>,
) -> Vec<RwMaterialDescriptor> {
    let Some(struct_chunk) = rw_stream_find_chunk(bytes, RW_ID_STRUCT) else {
        return Vec::new();
    };
    let struct_payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let Some(num_materials) = read_i32_le(struct_payload, 0) else {
        return Vec::new();
    };
    let num_materials = num_materials.max(0) as usize;
    if struct_payload.len() < 4 + num_materials * 4 {
        return Vec::new();
    }

    // librw `MaterialList::streamRead`: a non-negative entry reuses an earlier
    // logical material; only -1 entries consume a MATERIAL chunk.
    let mut materials: Vec<RwMaterialDescriptor> = Vec::with_capacity(num_materials);
    let mut cursor = struct_chunk.data_end;
    for logical_index in 0..num_materials {
        let reference = read_i32_le(struct_payload, 4 + logical_index * 4).unwrap_or(-1);
        if reference >= 0 {
            let Some(material) = usize::try_from(reference)
                .ok()
                .and_then(|index| materials.get(index))
                .cloned()
            else {
                return Vec::new();
            };
            materials.push(material);
            continue;
        }

        let material_chunk = loop {
            let Some(header) = read_chunk_header(bytes, cursor) else {
                return Vec::new();
            };
            cursor = header.data_end;
            if header.id == RW_ID_MATERIAL {
                break header;
            }
        };
        let Some(material) = read_material_descriptor(
            &bytes[material_chunk.data_start..material_chunk.data_end],
            geometry_surface_props,
        ) else {
            return Vec::new();
        };
        materials.push(material);
    }
    materials
}

fn read_material_descriptor(
    bytes: &[u8],
    geometry_surface_props: Option<[f32; 3]>,
) -> Option<RwMaterialDescriptor> {
    let struct_chunk = rw_stream_find_chunk(bytes, RW_ID_STRUCT)?;
    let payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    // flags(4) + color rgba(4) + unused(4) + textured(4).
    if payload.len() < 16 {
        return None;
    }
    let color = [
        *payload.get(4)?,
        *payload.get(5)?,
        *payload.get(6)?,
        *payload.get(7)?,
    ];
    let textured = read_i32_le(payload, 12).unwrap_or(0) != 0;
    let [ambient, specular, diffuse] = if rw_library_version(struct_chunk.version) < 0x30400 {
        geometry_surface_props.unwrap_or([1.0, 1.0, 1.0])
    } else {
        [
            read_f32_le(payload, 16)?,
            read_f32_le(payload, 20)?,
            read_f32_le(payload, 24)?,
        ]
    };

    let mut texture_name = None;
    let mut mask_name = None;
    if textured {
        if let Some(tex_chunk) = rw_stream_find_chunk(bytes, RW_ID_TEXTURE) {
            let tex_bytes = &bytes[tex_chunk.data_start..tex_chunk.data_end];
            let names = read_texture_chunk_names(tex_bytes);
            texture_name = names.0;
            mask_name = names.1;
        }
    }

    let dual_texture_name = rw_stream_find_chunk(bytes, RW_ID_EXTENSION)
        .and_then(|extension| {
            let extension_bytes = &bytes[extension.data_start..extension.data_end];
            let matfx = rw_stream_find_chunk(extension_bytes, RW_ID_MATFX)?;
            read_matfx_dual_texture_name(&extension_bytes[matfx.data_start..matfx.data_end])
        });

    Some(RwMaterialDescriptor {
        color,
        textured,
        ambient,
        specular,
        diffuse,
        texture_name,
        mask_name,
        dual_texture_name,
    })
}

/// MatFX stream layout: effect type (4), first effect type (4), followed by
/// its payload. For DUAL: src blend (4), dst blend (4), texture flag (4),
/// optional nested RwTexture chunk. Never scan arbitrary bytes for a name: a
/// malformed extension must not produce a fabricated lightmap attachment.
fn read_matfx_dual_texture_name(payload: &[u8]) -> Option<String> {
    const DUAL_EFFECT: u32 = 4;
    if read_u32_le(payload, 0)? != DUAL_EFFECT || read_u32_le(payload, 4)? != DUAL_EFFECT {
        return None;
    }
    // Ignore blend functions here. Neo's `neoWorldIII_PS` does not use them;
    // it uses WorldLightmapBlend and the authored UV2 sample instead.
    read_i32_le(payload, 8)?;
    read_i32_le(payload, 12)?;
    if read_i32_le(payload, 16)? == 0 { return None; }
    let texture = read_chunk_header(payload, 20)?;
    if texture.id != RW_ID_TEXTURE { return None; }
    read_texture_chunk_names(&payload[texture.data_start..texture.data_end]).0
}

#[cfg(test)]
mod matfx_dual_stream_tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0x1803_FFFFu32.to_le_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn dual_effect_reads_only_its_embedded_rw_texture() {
        let texture = chunk(RW_ID_STRING, b"road_light\0");
        let nested = chunk(RW_ID_TEXTURE, &texture);
        let mut matfx = Vec::new();
        for value in [4u32, 4, 5, 6, 1] {
            matfx.extend_from_slice(&value.to_le_bytes());
        }
        matfx.extend_from_slice(&nested);
        matfx.extend_from_slice(&0u32.to_le_bytes()); // second effect = null
        assert_eq!(read_matfx_dual_texture_name(&matfx).as_deref(), Some("road_light"));
        matfx[0] = 2; // envmap must not be misclassified as a dual lightmap
        assert!(read_matfx_dual_texture_name(&matfx).is_none());
    }

    #[test]
    fn malformed_or_absent_dual_texture_has_no_binding() {
        let mut matfx = Vec::new();
        for value in [4u32, 4, 5, 6, 0] {
            matfx.extend_from_slice(&value.to_le_bytes());
        }
        assert!(read_matfx_dual_texture_name(&matfx).is_none());
        matfx.truncate(17);
        assert!(read_matfx_dual_texture_name(&matfx).is_none());
    }
}

fn read_texture_chunk_names(bytes: &[u8]) -> (Option<String>, Option<String>) {
    let mut names = Vec::new();
    let mut cursor = 0usize;
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() && names.len() < 2 {
        let Some(header) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if header.id == RW_ID_STRING {
            let payload = &bytes[header.data_start..header.data_end];
            let end = payload
                .iter()
                .position(|b| *b == 0)
                .unwrap_or(payload.len());
            let name = String::from_utf8_lossy(&payload[..end]).trim().to_owned();
            if !name.is_empty() {
                names.push(name);
            }
        }
        cursor = header.data_end;
    }
    (names.first().cloned(), names.get(1).cloned())
}

fn collect_atomic_descriptors(
    bytes: &[u8],
    frame_names: &[Option<String>],
    geometries: &mut Vec<RwGeometryDescriptor>,
    count: usize,
) -> Vec<RwAtomicDescriptor> {
    let mut cursor = 0;
    let mut descriptors = Vec::new();
    while cursor + RW_CHUNK_HEADER_LEN <= bytes.len() && descriptors.len() < count {
        let Some(header) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if header.id == RW_ID_ATOMIC {
            if let Some(descriptor) =
                read_atomic_descriptor(&bytes[header.data_start..header.data_end], frame_names)
            {
                if let Some(geometry_header) =
                    rw_stream_find_chunk(&bytes[header.data_start..header.data_end], RW_ID_GEOMETRY)
                {
                    let atomic_payload = &bytes[header.data_start..header.data_end];
                    if let Some(geometry) = read_geometry_descriptor(
                        &atomic_payload[geometry_header.data_start..geometry_header.data_end],
                        geometry_header,
                    ) {
                        geometries.push(geometry);
                    }
                }
                descriptors.push(descriptor);
            }
        }
        cursor = header.data_end;
    }
    descriptors
}

fn read_atomic_descriptor(
    bytes: &[u8],
    frame_names: &[Option<String>],
) -> Option<RwAtomicDescriptor> {
    let struct_chunk = rw_stream_find_chunk(bytes, RW_ID_STRUCT)?;
    let struct_payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let frame_index = read_i32_le(struct_payload, 0)?;
    Some(RwAtomicDescriptor {
        frame_index,
        geometry_index: read_i32_le(struct_payload, 4)?,
        flags: read_i32_le(struct_payload, 8)?,
        frame_name: (frame_index >= 0)
            .then(|| frame_names.get(frame_index as usize).cloned().flatten())
            .flatten(),
    })
}

fn bevy_mesh_descriptors(geometries: &[RwGeometryDescriptor]) -> Vec<BevyRwMeshDescriptor> {
    geometries
        .iter()
        .enumerate()
        .map(|(geometry_index, geometry)| BevyRwMeshDescriptor {
            geometry_index,
            vertex_count: geometry.num_vertices.max(0) as usize,
            triangle_count: geometry.num_triangles.max(0) as usize,
            index_count: geometry.num_triangles.max(0) as usize * 3,
            material_slots: geometry.num_materials.max(0) as usize,
            has_positions: geometry.flags & RW_GEOMETRY_POSITIONS != 0,
            has_normals: geometry.flags & RW_GEOMETRY_NORMALS != 0,
            has_prelit_colors: geometry.flags & RW_GEOMETRY_PRELIT != 0,
            tex_coord_sets: geometry.num_tex_coord_sets.max(0) as usize,
        })
        .collect()
}

fn decode_geometry_struct(
    bytes: &[u8],
    version: u32,
    flags: u32,
    raw_flags: u32,
    num_vertices: i32,
    num_triangles: i32,
    num_morph_targets: i32,
    num_tex_coord_sets: i32,
    _num_materials: i32,
) -> Option<RwDecodedGeometry> {
    if raw_flags & RW_GEOMETRY_NATIVE != 0 {
        return None;
    }

    let vertex_count = num_vertices.max(0) as usize;
    let triangle_count = num_triangles.max(0) as usize;
    let morph_count = num_morph_targets.max(0) as usize;
    let tex_coord_sets = num_tex_coord_sets.max(0) as usize;
    let mut cursor = 16;
    // librw `Geometry::streamRead`: pre-3.4 streams carry the geometry-level
    // SurfaceProperties triple immediately after the fixed header.
    if geometry_has_pre34_surface_props(version) {
        cursor += 12;
    }

    let mut prelit_colors = Vec::new();
    if flags & RW_GEOMETRY_PRELIT != 0 {
        let bytes_len = vertex_count.checked_mul(4)?;
        let color_bytes = bytes.get(cursor..cursor + bytes_len)?;
        prelit_colors = color_bytes
            .chunks_exact(4)
            .map(|rgba| [rgba[0], rgba[1], rgba[2], rgba[3]])
            .collect();
        cursor += bytes_len;
    }

    let mut tex_coords = Vec::with_capacity(tex_coord_sets);
    for _ in 0..tex_coord_sets {
        let bytes_len = vertex_count.checked_mul(8)?;
        let uv_bytes = bytes.get(cursor..cursor + bytes_len)?;
        tex_coords.push(
            uv_bytes
                .chunks_exact(8)
                .map(|uv| {
                    [
                        read_f32_le(uv, 0).unwrap_or(0.0),
                        read_f32_le(uv, 4).unwrap_or(0.0),
                    ]
                })
                .collect(),
        );
        cursor += bytes_len;
    }

    let raw_triangles = read_raw_triangles(bytes, cursor, triangle_count)?;
    cursor += triangle_count * 8;
    let mut indices = Vec::with_capacity(triangle_count * 3);
    let mut triangle_materials = Vec::with_capacity(triangle_count);
    for tri in raw_triangles {
        // librw `Geometry::streamRead` reads two little-endian u32 words:
        // v0=word0>>16, v1=word0, v2=word1>>16, matId=word1.
        // Therefore the serialized u16 lanes are [v1, v0, matId, v2].
        // This layout is not version-dependent and must never be inferred from
        // whether a lane happens to fit the material count.
        indices.extend_from_slice(&[tri[1] as u32, tri[0] as u32, tri[3] as u32]);
        triangle_materials.push(tri[2]);
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut bounding_sphere = None;
    for morph_index in 0..morph_count {
        let sphere = bytes.get(cursor..cursor + 16)?;
        if morph_index == 0 {
            bounding_sphere = Some([
                read_f32_le(sphere, 0)?,
                read_f32_le(sphere, 4)?,
                read_f32_le(sphere, 8)?,
                read_f32_le(sphere, 12)?,
            ]);
        }
        cursor += 16;
        let has_vertices = read_i32_le(bytes, cursor)?;
        cursor += 4;
        let has_normals = read_i32_le(bytes, cursor)?;
        cursor += 4;

        if has_vertices != 0 {
            let bytes_len = vertex_count.checked_mul(12)?;
            let vertex_bytes = bytes.get(cursor..cursor + bytes_len)?;
            if morph_index == 0 {
                positions = read_v3d_array(vertex_bytes);
            }
            cursor += bytes_len;
        }
        if has_normals != 0 {
            let bytes_len = vertex_count.checked_mul(12)?;
            let normal_bytes = bytes.get(cursor..cursor + bytes_len)?;
            if morph_index == 0 {
                normals = read_v3d_array(normal_bytes);
            }
            cursor += bytes_len;
        }
    }

    Some(RwDecodedGeometry {
        positions,
        normals,
        prelit_colors,
        tex_coords,
        indices,
        triangle_materials,
        bounding_sphere,
    })
}

fn read_raw_triangles(bytes: &[u8], cursor: usize, triangle_count: usize) -> Option<Vec<[u16; 4]>> {
    let mut raw = Vec::with_capacity(triangle_count);
    for index in 0..triangle_count {
        let offset = cursor + index * 8;
        let tri = bytes.get(offset..offset + 8)?;
        raw.push([
            read_u16_le(tri, 0)?,
            read_u16_le(tri, 2)?,
            read_u16_le(tri, 4)?,
            read_u16_le(tri, 6)?,
        ]);
    }
    Some(raw)
}

fn geometry_tex_coord_sets(raw_flags: u32) -> i32 {
    let explicit = ((raw_flags >> 16) & 0xFF) as i32;
    if explicit != 0 {
        explicit
    } else if raw_flags & RW_GEOMETRY_TEXTURED2 != 0 {
        2
    } else if raw_flags & RW_GEOMETRY_TEXTURED != 0 {
        1
    } else {
        0
    }
}

/// RenderWare chunk headers store a packed library ID, not a directly
/// comparable `0x3MMrr` version. This is librw `libraryIDUnpackVersion`.
fn rw_library_version(library_id: u32) -> u32 {
    if library_id & 0xFFFF_0000 != 0 {
        (((library_id >> 14) & 0x3FF00) + 0x30000) | ((library_id >> 16) & 0x3F)
    } else {
        library_id << 8
    }
}

/// Whether geometry STRUCT includes its legacy SurfaceProperties triple.
fn geometry_has_pre34_surface_props(library_id: u32) -> bool {
    rw_library_version(library_id) < 0x34000
}

fn read_v3d_array(bytes: &[u8]) -> Vec<[f32; 3]> {
    bytes
        .chunks_exact(12)
        .map(|v| {
            [
                read_f32_le(v, 0).unwrap_or(0.0),
                read_f32_le(v, 4).unwrap_or(0.0),
                read_f32_le(v, 8).unwrap_or(0.0),
            ]
        })
        .collect()
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Option<i32> {
    read_u32_le(bytes, offset).map(|value| value as i32)
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Option<f32> {
    Some(f32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}

#[cfg(test)]
mod stream_boundary_tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [id, payload.len() as u32, 0x310] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(payload);
        bytes
    }

    fn clump_with_atomic_overrunning_outer_length() -> Vec<u8> {
        let mut payload = chunk(RW_ID_STRUCT, &1_i32.to_le_bytes());
        payload.extend(chunk(
            RW_ID_FRAMELIST,
            &chunk(RW_ID_STRUCT, &0_i32.to_le_bytes()),
        ));
        payload.extend(chunk(
            RW_ID_GEOMETRYLIST,
            &chunk(RW_ID_STRUCT, &0_i32.to_le_bytes()),
        ));
        let atomic_struct = [0_i32, 0, 5, 0]
            .into_iter()
            .flat_map(i32::to_le_bytes)
            .collect::<Vec<_>>();
        payload.extend(chunk(RW_ID_ATOMIC, &chunk(RW_ID_STRUCT, &atomic_struct)));
        let mut bytes = chunk(RW_ID_CLUMP, &payload);
        let short_length = payload.len() as u32 - 8;
        bytes[4..8].copy_from_slice(&short_length.to_le_bytes());
        bytes
    }

    #[test]
    fn native_atomic_count_reads_past_legacy_outer_clump_length() {
        let bytes = clump_with_atomic_overrunning_outer_length();
        let summary = read_clump(&bytes).expect("complete authored child atomic");
        assert_eq!(summary.num_atomics, 1);
        assert_eq!(summary.atomic_chunks, 1);
        assert_eq!(summary.atomics[0].flags, 5);
    }

    #[test]
    fn declared_count_prevents_consuming_another_streamed_atomic() {
        let mut bytes = clump_with_atomic_overrunning_outer_length();
        let extra = chunk(RW_ID_ATOMIC, &chunk(RW_ID_STRUCT, &[0; 16]));
        bytes.extend(extra);
        assert_eq!(read_clump(&bytes).unwrap().atomics.len(), 1);
    }

    #[test]
    fn incomplete_child_atomic_is_rejected() {
        let mut bytes = clump_with_atomic_overrunning_outer_length();
        bytes.truncate(bytes.len() - 4);
        assert!(read_clump(&bytes).is_none());
    }
}
