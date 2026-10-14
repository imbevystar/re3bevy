//! re3 reference: references/re-GTA/src/core/Directory.cpp
//! re3 reference: references/re-GTA/src/core/Directory.h
//!
//! PC IMG directory entries are fixed 32-byte records.

use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryInfo {
    pub offset: u32,
    pub size: u32,
    pub name: [u8; 24],
}

impl DirectoryInfo {
    pub const BYTE_LEN: usize = 32;

    pub fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        let offset = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let mut name = [0; 24];
        name.copy_from_slice(&bytes[8..32]);
        Self { offset, size, name }
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(&self.offset.to_le_bytes())?;
        writer.write_all(&self.size.to_le_bytes())?;
        writer.write_all(&self.name)?;
        Ok(())
    }

    pub fn name_bytes_trimmed(&self) -> &[u8] {
        let end = self
            .name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.name.len());
        &self.name[..end]
    }

    pub fn name_string(&self) -> String {
        String::from_utf8_lossy(self.name_bytes_trimmed()).into_owned()
    }

    pub fn with_image_id(mut self, image_id: i32) -> Self {
        self.offset |= (image_id as u32) << 24;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = [0; 24];
        let bytes = name.as_bytes();
        let len = bytes.len().min(self.name.len());
        self.name[..len].copy_from_slice(&bytes[..len]);
        self
    }
}

#[derive(Debug, Clone)]
pub struct CDirectory {
    max_entries: usize,
    entries: Vec<DirectoryInfo>,
}

impl CDirectory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Vec::with_capacity(max_entries),
        }
    }

    pub fn read_from<R: Read>(mut reader: R, max_entries: usize) -> io::Result<Self> {
        let mut directory = Self::new(max_entries);
        loop {
            let mut bytes = [0; DirectoryInfo::BYTE_LEN];
            match reader.read_exact(&mut bytes) {
                Ok(()) => directory.add_item(DirectoryInfo::from_bytes(bytes)),
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error),
            }
        }
        Ok(directory)
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        for entry in &self.entries {
            entry.write_to(&mut writer)?;
        }
        Ok(())
    }

    pub fn add_item(&mut self, entry: DirectoryInfo) {
        // re3 builds the extra-object directory with FIX_BUGS enabled.  An
        // entry that is already present must not consume another slot; this
        // matters when several IMG files contain the same loose DFF and when
        // the frontend rebuilds the streaming plan during a second New Game.
        if self.entries.iter().any(|existing| {
            ascii_eq_ignore_case(existing.name_bytes_trimmed(), entry.name_bytes_trimmed())
        }) {
            return;
        }
        assert!(self.entries.len() < self.max_entries);
        self.entries.push(entry);
    }

    pub fn add_item_with_image_id(&mut self, entry: DirectoryInfo, image_id: i32) {
        self.add_item(entry.with_image_id(image_id));
    }

    pub fn find_item(&self, name: &str) -> Option<(u32, u32)> {
        self.find_item_bytes(name.as_bytes())
    }

    pub fn find_item_bytes(&self, name: &[u8]) -> Option<(u32, u32)> {
        self.entries
            .iter()
            .find(|entry| ascii_eq_ignore_case(entry.name_bytes_trimmed(), name))
            .map(|entry| (entry.offset, entry.size))
    }

    pub fn entries(&self) -> &[DirectoryInfo] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn ascii_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, offset: u32) -> DirectoryInfo {
        DirectoryInfo {
            offset,
            size: 1,
            name: [0; 24],
        }
        .with_name(name)
    }

    #[test]
    fn duplicate_name_does_not_consume_directory_capacity() {
        let mut directory = CDirectory::new(1);
        directory.add_item(entry("building.dff", 10));
        directory.add_item(entry("BUILDING.DFF", 20));

        assert_eq!(directory.len(), 1);
        assert_eq!(directory.find_item("building.dff"), Some((10, 1)));
    }
}
