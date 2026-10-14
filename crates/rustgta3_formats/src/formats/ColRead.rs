//! re3 reference: references/re-GTA/src/core/FileLoader.cpp
//! re3 reference: references/re-GTA/src/collision/ColModel.h
//!
//! Binary COL block reader used by `CFileLoader::LoadCollisionFile`.

use std::io;

#[derive(Debug, Clone, PartialEq)]
pub struct GtaColFile {
    pub models: Vec<GtaNamedColModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaNamedColModel {
    pub name: String,
    pub model: GtaColModel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaColModel {
    pub bounding_sphere: GtaColSphere,
    pub bounding_box: GtaColBox,
    pub spheres: Vec<GtaColSphere>,
    pub lines: Vec<GtaColLine>,
    pub boxes: Vec<GtaColBox>,
    pub vertices: Vec<GtaCompressedVector>,
    pub triangles: Vec<GtaColTriangle>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GtaColVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GtaCompressedVector {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GtaColSphere {
    pub radius: f32,
    pub center: GtaColVector,
    pub surface: u8,
    pub piece: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GtaColBox {
    pub min: GtaColVector,
    pub max: GtaColVector,
    pub surface: u8,
    pub piece: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GtaColLine {
    pub p0: GtaColVector,
    pub p1: GtaColVector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtaColTriangle {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub surface: u8,
    pub piece: u8,
}

pub fn read_col_file(bytes: &[u8]) -> io::Result<GtaColFile> {
    let mut offset = 0;
    let mut models = Vec::new();

    while offset + 8 <= bytes.len() {
        let ident = &bytes[offset..offset + 4];
        offset += 4;
        if ident != b"COLL" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid collision header ident",
            ));
        }

        let size = read_u32(bytes, &mut offset)? as usize;
        if size < 24 || offset + size > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated collision block",
            ));
        }

        let block = &bytes[offset..offset + size];
        offset += size;
        models.push(GtaNamedColModel {
            name: read_model_name(&block[..24]),
            model: read_col_model(&block[24..])?,
        });
    }

    Ok(GtaColFile { models })
}

pub fn read_col_model(bytes: &[u8]) -> io::Result<GtaColModel> {
    let mut offset = 0;

    let bounding_sphere = GtaColSphere {
        radius: read_f32(bytes, &mut offset)?,
        center: GtaColVector {
            x: read_f32(bytes, &mut offset)?,
            y: read_f32(bytes, &mut offset)?,
            z: read_f32(bytes, &mut offset)?,
        },
        surface: 0,
        piece: 0,
    };
    let bounding_box = GtaColBox {
        min: read_vec3(bytes, &mut offset)?,
        max: read_vec3(bytes, &mut offset)?,
        surface: 0,
        piece: 0,
    };

    let num_spheres = read_i16(bytes, &mut offset)?.max(0) as usize;
    offset += 2;
    let mut spheres = Vec::with_capacity(num_spheres);
    for _ in 0..num_spheres {
        spheres.push(GtaColSphere {
            radius: read_f32(bytes, &mut offset)?,
            center: read_vec3(bytes, &mut offset)?,
            surface: read_u8(bytes, &mut offset)?,
            piece: read_u8(bytes, &mut offset)?,
        });
        offset += 2;
    }

    let num_lines = read_i16(bytes, &mut offset)?.max(0) as usize;
    offset += 2;
    let mut lines = Vec::with_capacity(num_lines);
    for _ in 0..num_lines {
        lines.push(GtaColLine {
            p0: read_vec3(bytes, &mut offset)?,
            p1: read_vec3(bytes, &mut offset)?,
        });
    }

    let num_boxes = read_i16(bytes, &mut offset)?.max(0) as usize;
    offset += 2;
    let mut boxes = Vec::with_capacity(num_boxes);
    for _ in 0..num_boxes {
        boxes.push(GtaColBox {
            min: read_vec3(bytes, &mut offset)?,
            max: read_vec3(bytes, &mut offset)?,
            surface: read_u8(bytes, &mut offset)?,
            piece: read_u8(bytes, &mut offset)?,
        });
        offset += 2;
    }

    let num_vertices = read_i16(bytes, &mut offset)?.max(0) as usize;
    offset += 2;
    let mut vertices = Vec::with_capacity(num_vertices);
    for _ in 0..num_vertices {
        vertices.push(GtaCompressedVector {
            x: (read_f32(bytes, &mut offset)? * 128.0) as i16,
            y: (read_f32(bytes, &mut offset)? * 128.0) as i16,
            z: (read_f32(bytes, &mut offset)? * 128.0) as i16,
        });
    }

    let num_triangles = read_i16(bytes, &mut offset)?.max(0) as usize;
    offset += 2;
    let mut triangles = Vec::with_capacity(num_triangles);
    for _ in 0..num_triangles {
        triangles.push(GtaColTriangle {
            a: read_i32(bytes, &mut offset)? as u16,
            b: read_i32(bytes, &mut offset)? as u16,
            c: read_i32(bytes, &mut offset)? as u16,
            surface: read_u8(bytes, &mut offset)?,
            piece: read_u8(bytes, &mut offset)?,
        });
        offset += 2;
    }

    Ok(GtaColModel {
        bounding_sphere,
        bounding_box,
        spheres,
        lines,
        boxes,
        vertices,
        triangles,
    })
}

fn read_model_name(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn read_vec3(bytes: &[u8], offset: &mut usize) -> io::Result<GtaColVector> {
    Ok(GtaColVector {
        x: read_f32(bytes, offset)?,
        y: read_f32(bytes, offset)?,
        z: read_f32(bytes, offset)?,
    })
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> io::Result<u8> {
    let byte = *bytes
        .get(*offset)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "truncated u8"))?;
    *offset += 1;
    Ok(byte)
}

fn read_i16(bytes: &[u8], offset: &mut usize) -> io::Result<i16> {
    let raw = read_array::<2>(bytes, offset, "truncated i16")?;
    Ok(i16::from_le_bytes(raw))
}

fn read_i32(bytes: &[u8], offset: &mut usize) -> io::Result<i32> {
    let raw = read_array::<4>(bytes, offset, "truncated i32")?;
    Ok(i32::from_le_bytes(raw))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> io::Result<u32> {
    let raw = read_array::<4>(bytes, offset, "truncated u32")?;
    Ok(u32::from_le_bytes(raw))
}

fn read_f32(bytes: &[u8], offset: &mut usize) -> io::Result<f32> {
    let raw = read_array::<4>(bytes, offset, "truncated f32")?;
    Ok(f32::from_le_bytes(raw))
}

fn read_array<const N: usize>(
    bytes: &[u8],
    offset: &mut usize,
    message: &'static str,
) -> io::Result<[u8; N]> {
    let end = *offset + N;
    let slice = bytes
        .get(*offset..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, message))?;
    *offset = end;
    Ok(slice.try_into().expect("slice length is checked"))
}
