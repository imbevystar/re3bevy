//! re3 reference: references/re-GTA/src/animation/AnimManager.cpp
//! re3 reference: references/re-GTA/src/animation/AnimManager.h
//!
//! IFP/ANPK metadata reader mirroring `CAnimManager::LoadAnimFile`.

#[derive(Debug, Clone, PartialEq)]
pub struct IfpFile {
    pub blocks: Vec<IfpAnimBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfpAnimBlock {
    pub name: String,
    pub animations: Vec<IfpAnimation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfpAnimation {
    pub name: String,
    pub sequences: Vec<IfpSequence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfpSequence {
    pub name: String,
    pub bone_tag: Option<i32>,
    pub frames: Vec<IfpKeyFrame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfpKeyFrame {
    pub rotation: [f32; 4],
    pub translation: Option<[f32; 3]>,
    pub delta_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IfpHeader {
    ident: [u8; 4],
    size: usize,
}

impl IfpFile {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        Self::parse_with_error(bytes).ok()
    }

    pub fn parse_with_error(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = 0;
        let first = read_header(bytes, &mut cursor)?;
        let num_blocks = if &first.ident == b"ANLF" {
            let payload = read_payload(bytes, &mut cursor, first.size)?;
            read_i32_le(payload, 0)? as usize
        } else if &first.ident == b"ANPK" {
            cursor = 0;
            1
        } else {
            return Err(format!(
                "expected ANLF or ANPK at 0, got {}",
                ident_string(&first.ident)
            ));
        };

        let mut blocks = Vec::with_capacity(num_blocks);
        for block_index in 0..num_blocks {
            let block_offset = cursor;
            blocks.push(read_anim_block(bytes, &mut cursor)?);
            let _ = block_index;
            let _ = block_offset;
        }
        Ok(Self { blocks })
    }
}

fn read_anim_block(bytes: &[u8], cursor: &mut usize) -> Result<IfpAnimBlock, String> {
    let anpk_offset = *cursor;
    let anpk = read_header(bytes, cursor)?;
    if &anpk.ident != b"ANPK" {
        return Err(format!(
            "expected ANPK at {anpk_offset}, got {}",
            ident_string(&anpk.ident)
        ));
    }
    let info_offset = *cursor;
    let info = read_header(bytes, cursor)?;
    if &info.ident != b"INFO" {
        return Err(format!(
            "expected INFO at {info_offset}, got {}",
            ident_string(&info.ident)
        ));
    }
    let payload = read_payload(bytes, cursor, info.size)?;
    let num_anims = read_i32_le(payload, 0)?.max(0) as usize;
    let name = read_c_string(
        payload
            .get(4..)
            .ok_or_else(|| "missing animation block name".to_owned())?,
    );

    let mut animations = Vec::with_capacity(num_anims);
    for anim_index in 0..num_anims {
        let anim_offset = *cursor;
        animations.push(read_animation(bytes, cursor)?);
        let _ = anim_index;
        let _ = anim_offset;
    }
    Ok(IfpAnimBlock { name, animations })
}

fn read_animation(bytes: &[u8], cursor: &mut usize) -> Result<IfpAnimation, String> {
    let name_offset = *cursor;
    let name_header = read_header(bytes, cursor)?;
    if &name_header.ident != b"NAME" {
        return Err(format!(
            "expected NAME at {name_offset}, got {}",
            ident_string(&name_header.ident)
        ));
    }
    let name = read_c_string(read_payload(bytes, cursor, name_header.size)?);

    let dgan_offset = *cursor;
    let dgan = read_header(bytes, cursor)?;
    if &dgan.ident != b"DGAN" {
        return Err(format!(
            "expected DGAN at {dgan_offset}, got {}",
            ident_string(&dgan.ident)
        ));
    }
    let info_offset = *cursor;
    let info = read_header(bytes, cursor)?;
    if &info.ident != b"INFO" {
        return Err(format!(
            "expected INFO at {info_offset}, got {}",
            ident_string(&info.ident)
        ));
    }
    let payload = read_payload(bytes, cursor, info.size)?;
    let num_sequences = read_i32_le(payload, 0)?.max(0) as usize;

    let mut sequences = Vec::with_capacity(num_sequences);
    for sequence_index in 0..num_sequences {
        let sequence_offset = *cursor;
        sequences.push(read_sequence(bytes, cursor)?);
        let _ = sequence_index;
        let _ = sequence_offset;
    }
    Ok(IfpAnimation { name, sequences })
}

fn read_sequence(bytes: &[u8], cursor: &mut usize) -> Result<IfpSequence, String> {
    let cpan_offset = *cursor;
    let cpan = read_header(bytes, cursor)?;
    if &cpan.ident != b"CPAN" {
        return Err(format!(
            "expected CPAN at {cpan_offset}, got {}",
            ident_string(&cpan.ident)
        ));
    }
    let anim_offset = *cursor;
    let anim = read_header(bytes, cursor)?;
    if &anim.ident != b"ANIM" {
        return Err(format!(
            "expected ANIM at {anim_offset}, got {}",
            ident_string(&anim.ident)
        ));
    }
    let payload = read_payload(bytes, cursor, anim.size)?;
    let name = read_c_string(slice_at(payload, 0, 24)?);
    let num_frames = read_i32_le(payload, 28)?.max(0) as usize;
    let bone_tag = if anim.size == 44 {
        Some(read_i32_le(payload, 40)?)
    } else {
        None
    };
    if num_frames == 0 {
        return Ok(IfpSequence {
            name,
            bone_tag,
            frames: Vec::new(),
        });
    }

    let key_offset = *cursor;
    let key_header = read_header(bytes, cursor)?;
    let (frame_size, has_translation) = match &key_header.ident {
        b"KRTS" => (0x2C, true),
        b"KRT0" => (0x20, true),
        b"KR00" => (0x14, false),
        _ => {
            return Err(format!(
                "expected KRTS/KRT0/KR00 at {key_offset}, got {} for sequence {name}",
                ident_string(&key_header.ident)
            ));
        }
    };

    let mut frames = Vec::with_capacity(num_frames);
    for _ in 0..num_frames {
        let raw = read_exact(bytes, cursor, frame_size)?;
        let mut rotation = [
            read_f32_le(raw, 0)?,
            read_f32_le(raw, 4)?,
            read_f32_le(raw, 8)?,
            read_f32_le(raw, 12)?,
        ];
        rotation[0] = -rotation[0];
        rotation[1] = -rotation[1];
        rotation[2] = -rotation[2];
        let translation = if has_translation {
            Some([
                read_f32_le(raw, 16)?,
                read_f32_le(raw, 20)?,
                read_f32_le(raw, 24)?,
            ])
        } else {
            None
        };
        let delta_time_offset = if frame_size == 0x2C {
            40
        } else if has_translation {
            28
        } else {
            16
        };
        frames.push(IfpKeyFrame {
            rotation,
            translation,
            delta_time: read_f32_le(raw, delta_time_offset)?,
        });
    }

    for index in (1..frames.len()).rev() {
        frames[index].delta_time -= frames[index - 1].delta_time;
    }
    remove_quaternion_flips(&mut frames);

    Ok(IfpSequence {
        name,
        bone_tag,
        frames,
    })
}

fn remove_quaternion_flips(frames: &mut [IfpKeyFrame]) {
    if frames.len() < 2 {
        return;
    }

    let mut last = frames[0].rotation;
    for frame in frames.iter_mut().skip(1) {
        if quaternion_dot(last, frame.rotation) < 0.0 {
            for component in &mut frame.rotation {
                *component = -*component;
            }
        }
        last = frame.rotation;
    }
}

fn quaternion_dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

fn read_header(bytes: &[u8], cursor: &mut usize) -> Result<IfpHeader, String> {
    let raw = read_exact(bytes, cursor, 8)?;
    let mut ident = [0; 4];
    ident.copy_from_slice(&raw[..4]);
    Ok(IfpHeader {
        ident,
        size: round_size(read_u32_le(raw, 4)? as usize),
    })
}

fn read_payload<'a>(bytes: &'a [u8], cursor: &mut usize, size: usize) -> Result<&'a [u8], String> {
    read_exact(bytes, cursor, size)
}

fn read_exact<'a>(bytes: &'a [u8], cursor: &mut usize, size: usize) -> Result<&'a [u8], String> {
    let start = *cursor;
    let end = cursor
        .checked_add(size)
        .ok_or_else(|| format!("offset overflow at {start} size {size}"))?;
    let slice = bytes
        .get(start..end)
        .ok_or_else(|| format!("read past end at {start} size {size} len {}", bytes.len()))?;
    *cursor = end;
    Ok(slice)
}

fn round_size(size: usize) -> usize {
    if size & 3 == 0 {
        size
    } else {
        size + 4 - (size & 3)
    }
}

fn read_c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes([
        *bytes
            .get(offset)
            .ok_or_else(|| format!("missing u32 byte 0 at {offset}"))?,
        *bytes
            .get(offset + 1)
            .ok_or_else(|| format!("missing u32 byte 1 at {offset}"))?,
        *bytes
            .get(offset + 2)
            .ok_or_else(|| format!("missing u32 byte 2 at {offset}"))?,
        *bytes
            .get(offset + 3)
            .ok_or_else(|| format!("missing u32 byte 3 at {offset}"))?,
    ]))
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, String> {
    read_u32_le(bytes, offset).map(|value| value as i32)
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, String> {
    Ok(f32::from_le_bytes([
        *bytes
            .get(offset)
            .ok_or_else(|| format!("missing f32 byte 0 at {offset}"))?,
        *bytes
            .get(offset + 1)
            .ok_or_else(|| format!("missing f32 byte 1 at {offset}"))?,
        *bytes
            .get(offset + 2)
            .ok_or_else(|| format!("missing f32 byte 2 at {offset}"))?,
        *bytes
            .get(offset + 3)
            .ok_or_else(|| format!("missing f32 byte 3 at {offset}"))?,
    ]))
}

fn slice_at(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8], String> {
    bytes
        .get(offset..offset + len)
        .ok_or_else(|| format!("missing slice at {offset} len {len}"))
}

fn ident_string(ident: &[u8; 4]) -> String {
    ident
        .iter()
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            }
        })
        .collect()
}
