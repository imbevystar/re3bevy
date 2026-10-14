//! re3 reference: references/re-GTA/src/rw/TexRead.cpp
//!
//! GTA TXD stream reader metadata and CPU-side raster decode helpers.

use crate::formats::ClumpRead::{
    RW_CHUNK_HEADER_LEN, RW_ID_STRUCT, count_chunks, read_chunk_header, rw_stream_find_chunk,
};

pub const RW_ID_TEXTURENATIVE: u32 = 0x15;
pub const RW_ID_TEXDICTIONARY: u32 = 0x16;
pub const RW_PLATFORM_D3D8: u32 = 8;
pub const RW_PLATFORM_D3D9: u32 = 9;
pub const RW_RASTER_PAL8: i32 = 0x2000;
pub const RW_RASTER_PAL4: i32 = 0x4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwTextureNativeDescriptor {
    pub platform_id: u32,
    pub chunk_version: u32,
    pub chunk_length: u32,
    pub filter_addressing: u32,
    pub name: String,
    pub mask: String,
    pub raster_format: i32,
    pub d3d_format: Option<i32>,
    pub has_alpha: bool,
    pub width: u16,
    pub height: u16,
    pub depth: u8,
    pub num_levels: u8,
    pub raster_type: u8,
    pub compression_or_flags: u8,
    pub palette: Vec<[u8; 4]>,
    pub mip_level_sizes: Vec<u32>,
    pub mip_level_payloads: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BevyRwImageDescriptor {
    pub texture_index: usize,
    pub platform_id: u32,
    pub width: u32,
    pub height: u32,
    pub mip_level_count: u32,
    pub has_alpha: bool,
    pub encoded_byte_len: usize,
    pub mip_level_sizes: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwDecodedTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwTexDictionarySummary {
    pub num_textures: i32,
    pub first_phase_textures: i32,
    pub texture_native_chunks: i32,
    pub textures: Vec<RwTextureNativeDescriptor>,
    pub bevy_images: Vec<BevyRwImageDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwTexDictionaryReadState {
    pub summary: RwTexDictionarySummary,
    pub stream_position: usize,
}

pub fn read_txd(bytes: &[u8]) -> Option<RwTexDictionarySummary> {
    // Normal streamed TXDs arrive as one complete buffer.  The old path ran
    // the two RenderWare split-read helpers back-to-back, which recursively
    // counted the same chunk tree twice, cloned the first half of every native
    // texture (including mip payloads), and built Bevy image descriptors twice.
    // Keep the split helpers for genuinely large/two-phase streams, but parse a
    // complete dictionary once here.
    let txd = rw_stream_find_chunk(bytes, RW_ID_TEXDICTIONARY)?;
    let payload = &bytes[txd.data_start..txd.data_end];
    let struct_chunk = rw_stream_find_chunk(payload, RW_ID_STRUCT)?;
    let struct_payload = &payload[struct_chunk.data_start..struct_chunk.data_end];
    let num_textures = read_i32_le(struct_payload, 0)?;
    if num_textures < 0 {
        return None;
    }

    let texture_native_chunks = count_chunks(payload, RW_ID_TEXTURENATIVE) as i32;
    if texture_native_chunks < num_textures {
        return None;
    }

    let mut textures = Vec::with_capacity(num_textures as usize);
    let mut cursor = 0;
    while cursor + RW_CHUNK_HEADER_LEN <= payload.len() && textures.len() < num_textures as usize {
        let header = read_chunk_header(payload, cursor)?;
        if header.id == RW_ID_TEXTURENATIVE {
            let inner_payload = &payload[header.data_start..header.data_end];
            if let Some(mut descriptor) = read_texture_native_descriptor(inner_payload) {
                descriptor.chunk_version = header.version;
                descriptor.chunk_length = header.length;
                textures.push(descriptor);
            }
        }
        cursor = header.data_end;
    }
    if textures.len() != num_textures as usize {
        return None;
    }

    let bevy_images = bevy_image_descriptors(&textures);
    Some(RwTexDictionarySummary {
        num_textures,
        first_phase_textures: num_textures / 2,
        texture_native_chunks,
        textures,
        bevy_images,
    })
}

/// D3D FourCC / d3d8 compression id → DXT family used by GTA III TXDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DxtFormat {
    Dxt1,
    Dxt3,
    Dxt5,
}

pub fn decode_txd_texture(texture: &RwTextureNativeDescriptor) -> Option<RwDecodedTexture> {
    let width = u32::from(texture.width);
    let height = u32::from(texture.height);
    let payload = texture.mip_level_payloads.first()?;
    let rgba = if let Some(dxt) = detect_dxt_format(texture) {
        decode_dxt(payload, width as usize, height as usize, dxt)?
    } else {
        match texture.depth {
            32 => bgra_to_rgba(payload),
            16 => decode_16bit_rgba(payload, texture.raster_format)?,
            8 => pal8_to_rgba(payload, &texture.palette),
            4 => pal4_to_rgba(payload, &texture.palette, width as usize * height as usize),
            _ => return None,
        }
    };
    if rgba.len() != width as usize * height as usize * 4 {
        return None;
    }
    Some(RwDecodedTexture {
        name: texture.name.clone(),
        width,
        height,
        rgba,
    })
}

fn detect_dxt_format(texture: &RwTextureNativeDescriptor) -> Option<DxtFormat> {
    // D3D9: FourCC in d3d_format field.
    if let Some(fmt) = texture.d3d_format {
        return match fmt as u32 {
            0x3154_5844 => Some(DxtFormat::Dxt1), // 'DXT1'
            0x3354_5844 => Some(DxtFormat::Dxt3), // 'DXT3'
            0x3554_5844 => Some(DxtFormat::Dxt5), // 'DXT5'
            _ => None,
        };
    }
    // D3D8: compression byte (1/3/5) when non-zero.
    match texture.compression_or_flags {
        1 => Some(DxtFormat::Dxt1),
        3 => Some(DxtFormat::Dxt3),
        5 => Some(DxtFormat::Dxt5),
        _ => None,
    }
}

/// re3/librw `RwRasterFormat` 16-bit pixels.
///
/// The pixel family is encoded in bits `0x0f00`; D3D's `hasAlpha` field does
/// not select the packing. Treating every alpha raster as 4444 corrupts GTA
/// III's 1555 cutscene/ped textures (notably turning skin blue).
fn decode_16bit_rgba(payload: &[u8], raster_format: i32) -> Option<Vec<u8>> {
    const RW_RASTER_1555: i32 = 0x0100;
    const RW_RASTER_565: i32 = 0x0200;
    const RW_RASTER_4444: i32 = 0x0300;

    let pixels = payload.len() / 2;
    let mut out = Vec::with_capacity(pixels * 4);
    let format_bits = raster_format & 0x0F00;
    for chunk in payload.chunks_exact(2) {
        let v = u16::from_le_bytes([chunk[0], chunk[1]]);
        let (r, g, b, a) = match format_bits {
            RW_RASTER_1555 => (
                expand_5((v >> 10) & 0x1F),
                expand_5((v >> 5) & 0x1F),
                expand_5(v & 0x1F),
                if v & 0x8000 != 0 { 255 } else { 0 },
            ),
            RW_RASTER_565 => (
                expand_5((v >> 11) & 0x1F),
                expand_6((v >> 5) & 0x3F),
                expand_5(v & 0x1F),
                255,
            ),
            RW_RASTER_4444 => (
                expand_4((v >> 8) & 0xF),
                expand_4((v >> 4) & 0xF),
                expand_4(v & 0xF),
                expand_4((v >> 12) & 0xF),
            ),
            _ => return None,
        };
        out.extend_from_slice(&[r, g, b, a]);
    }
    Some(out)
}

#[inline(always)]
fn expand_4(value: u16) -> u8 {
    (value as u8) * 17
}

#[inline(always)]
fn expand_5(value: u16) -> u8 {
    ((value << 3) | (value >> 2)) as u8
}

#[inline(always)]
fn expand_6(value: u16) -> u8 {
    ((value << 2) | (value >> 4)) as u8
}

fn decode_dxt(payload: &[u8], width: usize, height: usize, format: DxtFormat) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let block_size = match format {
        DxtFormat::Dxt1 => 8,
        DxtFormat::Dxt3 | DxtFormat::Dxt5 => 16,
    };
    let expected = blocks_x * blocks_y * block_size;
    if payload.len() < expected {
        return None;
    }
    let mut out = vec![0u8; width * height * 4];
    let mut offset = 0;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = &payload[offset..offset + block_size];
            offset += block_size;
            let (colors, alphas) = match format {
                DxtFormat::Dxt1 => decode_dxt1_block(block),
                DxtFormat::Dxt3 => decode_dxt3_block(block),
                DxtFormat::Dxt5 => decode_dxt5_block(block),
            };

            let base_x = bx * 4;
            let base_y = by * 4;
            if base_x + 4 <= width && base_y + 4 <= height {
                for py in 0..4 {
                    let y = base_y + py;
                    let row_idx = (y * width + base_x) * 4;
                    let row_out = &mut out[row_idx..row_idx + 16];
                    for px in 0..4 {
                        let idx = py * 4 + px;
                        let p = px * 4;
                        let c = colors[idx];
                        row_out[p] = c[0];
                        row_out[p + 1] = c[1];
                        row_out[p + 2] = c[2];
                        row_out[p + 3] = alphas[idx];
                    }
                }
            } else {
                for py in 0..4 {
                    let y = base_y + py;
                    if y >= height {
                        continue;
                    }
                    for px in 0..4 {
                        let x = base_x + px;
                        if x >= width {
                            continue;
                        }
                        let idx = py * 4 + px;
                        let i = (y * width + x) * 4;
                        out[i] = colors[idx][0];
                        out[i + 1] = colors[idx][1];
                        out[i + 2] = colors[idx][2];
                        out[i + 3] = alphas[idx];
                    }
                }
            }
        }
    }
    Some(out)
}

#[inline(always)]
fn decode_rgb565(c: u16) -> [u8; 3] {
    let r = expand_5((c >> 11) & 0x1F);
    let g = expand_6((c >> 5) & 0x3F);
    let b = expand_5(c & 0x1F);
    [r, g, b]
}

#[inline(always)]
fn decode_dxt_color_block(block: &[u8], allow_dxt1_transparent_mode: bool) -> [[u8; 3]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let rgb0 = decode_rgb565(c0);
    let rgb1 = decode_rgb565(c1);
    let mut palette = [[0u8; 3]; 4];
    palette[0] = rgb0;
    palette[1] = rgb1;
    // BC1/DXT1 alone uses `c0 <= c1` to select its three-colour plus
    // transparent mode. BC2/DXT3 and BC3/DXT5 carry alpha in a separate
    // block, so their colour block always has four interpolated colours.
    // Treating BC2/BC3 as BC1 turns every colour-index 3 texel black.
    if c0 > c1 || !allow_dxt1_transparent_mode {
        for i in 0..3 {
            palette[2][i] = ((2 * u16::from(rgb0[i]) + u16::from(rgb1[i])) / 3) as u8;
            palette[3][i] = ((u16::from(rgb0[i]) + 2 * u16::from(rgb1[i])) / 3) as u8;
        }
    } else {
        for i in 0..3 {
            palette[2][i] = ((u16::from(rgb0[i]) + u16::from(rgb1[i])) / 2) as u8;
        }
        palette[3] = [0, 0, 0];
    }
    let bits = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let mut colors = [[0u8; 3]; 16];
    for i in 0..16 {
        let code = ((bits >> (i * 2)) & 0x3) as usize;
        colors[i] = palette[code];
    }
    colors
}

#[inline(always)]
fn decode_dxt1_block(block: &[u8]) -> ([[u8; 3]; 16], [u8; 16]) {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let colors = decode_dxt_color_block(block, true);
    let bits = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let mut alphas = [255u8; 16];
    if c0 <= c1 {
        for i in 0..16 {
            if ((bits >> (i * 2)) & 0x3) == 3 {
                alphas[i] = 0;
            }
        }
    }
    (colors, alphas)
}

#[inline(always)]
fn decode_dxt3_block(block: &[u8]) -> ([[u8; 3]; 16], [u8; 16]) {
    let mut alphas = [0u8; 16];
    for i in 0..8 {
        let b = block[i];
        alphas[i * 2] = (b & 0x0F) * 17;
        alphas[i * 2 + 1] = (b >> 4) * 17;
    }
    let colors = decode_dxt_color_block(&block[8..16], false);
    (colors, alphas)
}

fn decode_dxt5_block(block: &[u8]) -> ([[u8; 3]; 16], [u8; 16]) {
    let a0 = block[0];
    let a1 = block[1];
    let mut alpha_palette = [0u8; 8];
    alpha_palette[0] = a0;
    alpha_palette[1] = a1;
    if a0 > a1 {
        for i in 1..7 {
            alpha_palette[i + 1] =
                (((7 - i) as u16 * u16::from(a0) + i as u16 * u16::from(a1)) / 7) as u8;
        }
    } else {
        for i in 1..5 {
            alpha_palette[i + 1] =
                (((5 - i) as u16 * u16::from(a0) + i as u16 * u16::from(a1)) / 5) as u8;
        }
        alpha_palette[6] = 0;
        alpha_palette[7] = 255;
    }
    // 6 bytes of 3-bit indices starting at block[2]
    let mut alpha_bits: u64 = 0;
    for (i, b) in block[2..8].iter().enumerate() {
        alpha_bits |= u64::from(*b) << (8 * i);
    }
    let mut alphas = [0u8; 16];
    for i in 0..16 {
        let code = ((alpha_bits >> (3 * i)) & 0x7) as usize;
        alphas[i] = alpha_palette[code];
    }
    let colors = decode_dxt_color_block(&block[8..16], false);
    (colors, alphas)
}

pub fn decode_txd_textures(summary: &RwTexDictionarySummary) -> Vec<RwDecodedTexture> {
    summary
        .textures
        .iter()
        .filter_map(decode_txd_texture)
        .collect()
}

pub fn gta_tex_dictionary_stream_read1(bytes: &[u8]) -> Option<RwTexDictionaryReadState> {
    let txd = rw_stream_find_chunk(bytes, RW_ID_TEXDICTIONARY)?;
    let payload = &bytes[txd.data_start..txd.data_end];
    let struct_chunk = rw_stream_find_chunk(payload, RW_ID_STRUCT)?;
    let struct_payload = &payload[struct_chunk.data_start..struct_chunk.data_end];
    let num_textures = read_i32_le(struct_payload, 0)?;
    let first_phase_textures = num_textures / 2;

    let mut textures = Vec::new();
    let mut cursor = 0;
    while cursor + RW_CHUNK_HEADER_LEN <= payload.len()
        && textures.len() < (num_textures - first_phase_textures) as usize
    {
        let Some(header) = read_chunk_header(payload, cursor) else {
            break;
        };
        if header.id == RW_ID_TEXTURENATIVE {
            let inner_payload = &payload[header.data_start..header.data_end];
            if let Some(mut descriptor) = read_texture_native_descriptor(inner_payload) {
                descriptor.chunk_version = header.version;
                descriptor.chunk_length = header.length;
                textures.push(descriptor);
            }
        }
        cursor = header.data_end;
    }
    let bevy_images = bevy_image_descriptors(&textures);

    Some(RwTexDictionaryReadState {
        summary: RwTexDictionarySummary {
            num_textures,
            first_phase_textures,
            texture_native_chunks: count_chunks(payload, RW_ID_TEXTURENATIVE) as i32,
            textures,
            bevy_images,
        },
        stream_position: txd.data_start + cursor,
    })
}

pub fn gta_tex_dictionary_stream_read2(
    bytes: &[u8],
    state: &RwTexDictionaryReadState,
) -> Option<RwTexDictionarySummary> {
    let txd = rw_stream_find_chunk(bytes, RW_ID_TEXDICTIONARY)?;
    let payload = &bytes[txd.data_start..txd.data_end];
    let texture_native_chunks = count_chunks(payload, RW_ID_TEXTURENATIVE) as i32;
    if texture_native_chunks < state.summary.num_textures {
        return None;
    }

    let mut summary = state.summary.clone();
    summary.texture_native_chunks = texture_native_chunks;

    let mut textures = summary.textures.clone();
    let mut cursor = state.stream_position;
    while cursor + RW_CHUNK_HEADER_LEN <= txd.data_end
        && textures.len() < summary.num_textures as usize
    {
        let Some(header) = read_chunk_header(bytes, cursor) else {
            break;
        };
        if header.id == RW_ID_TEXTURENATIVE {
            let inner_payload = &bytes[header.data_start..header.data_end];
            if let Some(mut descriptor) = read_texture_native_descriptor(inner_payload) {
                descriptor.chunk_version = header.version;
                descriptor.chunk_length = header.length;
                textures.push(descriptor);
            }
        }
        cursor = header.data_end;
    }
    summary.textures = textures;
    summary.bevy_images = bevy_image_descriptors(&summary.textures);
    Some(summary)
}

fn bevy_image_descriptors(textures: &[RwTextureNativeDescriptor]) -> Vec<BevyRwImageDescriptor> {
    textures
        .iter()
        .enumerate()
        .map(|(texture_index, texture)| BevyRwImageDescriptor {
            texture_index,
            platform_id: texture.platform_id,
            width: u32::from(texture.width),
            height: u32::from(texture.height),
            mip_level_count: u32::from(texture.num_levels),
            has_alpha: texture.has_alpha,
            encoded_byte_len: texture.chunk_length as usize,
            mip_level_sizes: texture.mip_level_sizes.clone(),
        })
        .collect()
}

fn read_texture_native_descriptor(bytes: &[u8]) -> Option<RwTextureNativeDescriptor> {
    let struct_chunk = rw_stream_find_chunk(bytes, RW_ID_STRUCT)?;
    let payload = &bytes[struct_chunk.data_start..struct_chunk.data_end];
    let platform_id = read_u32_le(payload, 0)?;
    match platform_id {
        RW_PLATFORM_D3D8 => read_d3d8_texture_native(payload),
        RW_PLATFORM_D3D9 => read_d3d9_texture_native(payload),
        _ => Some(RwTextureNativeDescriptor {
            platform_id,
            chunk_version: struct_chunk.version,
            chunk_length: struct_chunk.length,
            filter_addressing: 0,
            name: String::new(),
            mask: String::new(),
            raster_format: 0,
            d3d_format: None,
            has_alpha: false,
            width: 0,
            height: 0,
            depth: 0,
            num_levels: 0,
            raster_type: 0,
            compression_or_flags: 0,
            palette: Vec::new(),
            mip_level_sizes: Vec::new(),
            mip_level_payloads: Vec::new(),
        }),
    }
}

fn read_d3d8_texture_native(payload: &[u8]) -> Option<RwTextureNativeDescriptor> {
    let platform_id = read_u32_le(payload, 0)?;
    let filter_addressing = read_u32_le(payload, 4)?;
    let name = read_c_string_32(payload, 8)?;
    let mask = read_c_string_32(payload, 40)?;
    let raster_format = read_i32_le(payload, 72)?;
    let has_alpha = read_i32_le(payload, 76)? != 0;
    let width = read_u16_le(payload, 80)?;
    let height = read_u16_le(payload, 82)?;
    let depth = *payload.get(84)?;
    let num_levels = *payload.get(85)?;
    let raster_type = *payload.get(86)?;
    let compression = *payload.get(87)?;
    let palette = read_palette(payload, 88, raster_format);
    let mip_cursor = 88 + palette_byte_len(raster_format);
    let (mip_level_sizes, mip_level_payloads) =
        read_mip_levels(payload, mip_cursor, num_levels as usize);
    Some(RwTextureNativeDescriptor {
        platform_id,
        chunk_version: 0,
        chunk_length: 0,
        filter_addressing,
        name,
        mask,
        raster_format,
        d3d_format: None,
        has_alpha,
        width,
        height,
        depth,
        num_levels,
        raster_type,
        compression_or_flags: compression,
        palette,
        mip_level_sizes,
        mip_level_payloads,
    })
}

fn read_d3d9_texture_native(payload: &[u8]) -> Option<RwTextureNativeDescriptor> {
    let platform_id = read_u32_le(payload, 0)?;
    let filter_addressing = read_u32_le(payload, 4)?;
    let name = read_c_string_32(payload, 8)?;
    let mask = read_c_string_32(payload, 40)?;
    let raster_format = read_i32_le(payload, 72)?;
    let d3d_format = read_i32_le(payload, 76)?;
    let width = read_u16_le(payload, 80)?;
    let height = read_u16_le(payload, 82)?;
    let depth = *payload.get(84)?;
    let num_levels = *payload.get(85)?;
    let raster_type = *payload.get(86)?;
    let flags = *payload.get(87)?;
    let palette = read_palette(payload, 88, raster_format);
    let mip_cursor = 88 + palette_byte_len(raster_format);
    let (mip_level_sizes, mip_level_payloads) =
        read_mip_levels(payload, mip_cursor, num_levels as usize);
    Some(RwTextureNativeDescriptor {
        platform_id,
        chunk_version: 0,
        chunk_length: 0,
        filter_addressing,
        name,
        mask,
        raster_format,
        d3d_format: Some(d3d_format),
        has_alpha: flags & 1 != 0,
        width,
        height,
        depth,
        num_levels,
        raster_type,
        compression_or_flags: flags,
        palette,
        mip_level_sizes,
        mip_level_payloads,
    })
}

fn read_palette(payload: &[u8], offset: usize, raster_format: i32) -> Vec<[u8; 4]> {
    let len = palette_byte_len(raster_format);
    payload
        .get(offset..offset + len)
        .unwrap_or(&[])
        .chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect()
}

fn read_mip_levels(
    payload: &[u8],
    mut cursor: usize,
    num_levels: usize,
) -> (Vec<u32>, Vec<Vec<u8>>) {
    let mut sizes = Vec::new();
    let mut mip_payloads = Vec::new();
    for _ in 0..num_levels {
        let Some(size) = read_u32_le(payload, cursor) else {
            break;
        };
        sizes.push(size);
        cursor = cursor.saturating_add(4);
        let end = cursor.saturating_add(size as usize);
        let Some(bytes) = payload.get(cursor..end) else {
            break;
        };
        mip_payloads.push(bytes.to_vec());
        cursor = end;
    }
    (sizes, mip_payloads)
}

fn palette_byte_len(raster_format: i32) -> usize {
    if raster_format & RW_RASTER_PAL4 != 0 {
        4 * 16
    } else if raster_format & RW_RASTER_PAL8 != 0 {
        4 * 256
    } else {
        0
    }
}

fn bgra_to_rgba(payload: &[u8]) -> Vec<u8> {
    payload
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], pixel[3]])
        .collect()
}

fn pal8_to_rgba(payload: &[u8], palette: &[[u8; 4]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() * 4);
    for index in payload {
        // D3D8 native textures store palette entries as PALETTEENTRY
        // (red, green, blue, flags). re3 passes READNATIVE's palette through
        // unchanged; it is not a D3DCOLOR/BGRA raster payload.
        let rgba = palette
            .get(*index as usize)
            .copied()
            .unwrap_or([0, 0, 0, 0]);
        out.extend_from_slice(&rgba);
    }
    out
}

fn pal4_to_rgba(payload: &[u8], palette: &[[u8; 4]], pixel_count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixel_count * 4);
    for byte in payload {
        for index in [byte & 0x0F, byte >> 4] {
            if out.len() / 4 >= pixel_count {
                break;
            }
            let rgba = palette.get(index as usize).copied().unwrap_or([0, 0, 0, 0]);
            out.extend_from_slice(&rgba);
        }
    }
    out
}

fn read_c_string_32(bytes: &[u8], offset: usize) -> Option<String> {
    let data = bytes.get(offset..offset + 32)?;
    let len = data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(data.len());
    Some(String::from_utf8_lossy(&data[..len]).into_owned())
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(
        depth: u8,
        raster_format: i32,
        payload: Vec<u8>,
        palette: Vec<[u8; 4]>,
    ) -> RwTextureNativeDescriptor {
        RwTextureNativeDescriptor {
            platform_id: RW_PLATFORM_D3D8,
            chunk_version: 0,
            chunk_length: 0,
            filter_addressing: 0,
            name: "test".to_owned(),
            mask: String::new(),
            raster_format,
            d3d_format: None,
            has_alpha: true,
            width: 1,
            height: 1,
            depth,
            num_levels: 1,
            raster_type: 4,
            compression_or_flags: 0,
            palette,
            mip_level_sizes: vec![payload.len() as u32],
            mip_level_payloads: vec![payload],
        }
    }

    #[test]
    fn d3d_a8r8g8b8_payload_is_swizzled_bgra_to_rgba() {
        let decoded = decode_txd_texture(&texture(32, 0, vec![3, 2, 1, 255], vec![])).unwrap();
        assert_eq!(decoded.rgba, vec![1, 2, 3, 255]);
    }

    #[test]
    fn d3d_palette_entries_remain_paletteentry_rgba() {
        let decoded = decode_txd_texture(&texture(
            8,
            RW_RASTER_PAL8,
            vec![0],
            vec![[200, 80, 40, 255]],
        ))
        .unwrap();
        assert_eq!(decoded.rgba, vec![200, 80, 40, 255]);
    }

    #[test]
    fn raster_1555_decodes_rgb_and_alpha_bits() {
        let value = 0xFC00u16.to_le_bytes().to_vec();
        let decoded = decode_txd_texture(&texture(16, 0x0100, value, vec![])).unwrap();
        assert_eq!(decoded.rgba, vec![255, 0, 0, 255]);
    }

    #[test]
    fn dxt1_keeps_three_colour_transparent_mode_when_c0_is_not_greater_than_c1() {
        // c0 = black, c1 = white and every texel selects palette entry 3.
        let block = [0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let (colours, alphas) = decode_dxt1_block(&block);
        assert_eq!(colours[0], [0, 0, 0]);
        assert_eq!(alphas[0], 0);
    }

    #[test]
    fn dxt3_always_uses_four_colour_interpolation() {
        let mut block = [0xFF; 16];
        // Colour block: c0 = black, c1 = white, every texel selects entry 3.
        block[8..12].copy_from_slice(&[0, 0, 0xFF, 0xFF]);
        let (colours, alphas) = decode_dxt3_block(&block);
        assert_eq!(colours[0], [170, 170, 170]);
        assert_eq!(alphas[0], 255);
    }

    #[test]
    fn dxt5_always_uses_four_colour_interpolation() {
        let mut block = [0u8; 16];
        block[0] = 255;
        block[1] = 0;
        // Alpha indices select a0. Colour indices select entry 3.
        block[8..16].copy_from_slice(&[0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let (colours, alphas) = decode_dxt5_block(&block);
        assert_eq!(colours[0], [170, 170, 170]);
        assert_eq!(alphas[0], 255);
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}
