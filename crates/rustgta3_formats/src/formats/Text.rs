//! re3 reference: references/re-GTA/src/text/Text.cpp
//! re3 reference: references/re-GTA/src/text/Text.h
//!
//! GXT `TKEY`/`TDAT` section reader.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GxtFile {
    pub entries: Vec<GxtKeyEntry>,
    pub data: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GxtKeyEntry {
    pub value_offset: u32,
    pub key: String,
}

impl GxtFile {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let mut offset = 0;
        let mut entries = Vec::new();
        let mut data = Vec::new();

        while offset + 8 <= bytes.len() {
            let section = bytes.get(offset..offset + 4)?;
            offset += 4;
            let section_len = read_u32_le(bytes, offset)? as usize;
            offset += 4;
            let section_end = offset.checked_add(section_len)?;
            if section_end > bytes.len() {
                return None;
            }

            if section == b"TKEY" {
                entries = read_tkey(&bytes[offset..section_end])?;
            } else if section == b"TDAT" {
                data = read_tdat(&bytes[offset..section_end])?;
            }
            offset = section_end;
        }

        Some(Self { entries, data })
    }

    pub fn strings(&self) -> BTreeMap<String, Vec<u16>> {
        let mut strings = BTreeMap::new();
        for entry in &self.entries {
            let start = (entry.value_offset / 2) as usize;
            let Some(mut end) = self
                .data
                .get(start..)
                .and_then(|tail| tail.iter().position(|ch| *ch == 0).map(|len| start + len))
            else {
                continue;
            };
            if end > self.data.len() {
                end = self.data.len();
            }
            strings.insert(entry.key.clone(), self.data[start..end].to_vec());
        }
        strings
    }
}

fn read_tkey(bytes: &[u8]) -> Option<Vec<GxtKeyEntry>> {
    let mut entries = Vec::with_capacity(bytes.len() / 12);
    for raw in bytes.chunks_exact(12) {
        let value_offset = read_u32_le(raw, 0)?;
        let key = read_key(&raw[4..12]);
        entries.push(GxtKeyEntry { value_offset, key });
    }
    Some(entries)
}

fn read_tdat(bytes: &[u8]) -> Option<Vec<u16>> {
    if bytes.len() % 2 != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(2)
            .map(|raw| u16::from_le_bytes([raw[0], raw[1]]))
            .collect(),
    )
}

fn read_key(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
        *bytes.get(offset + 2)?,
        *bytes.get(offset + 3)?,
    ]))
}
