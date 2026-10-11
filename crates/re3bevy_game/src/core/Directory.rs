//! re3 reference: references/re-GTA/src/core/Directory.cpp
//! re3 reference: references/re-GTA/src/core/Directory.h
//!
//! IMG directory entries.

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DirectoryInfo {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CDirectory {
    pub entries: Vec<DirectoryInfo>,
}

impl CDirectory {
    #[allow(non_snake_case)]
    pub fn AddItem(&mut self, name: impl Into<String>, offset: u32, size: u32) {
        self.entries.push(DirectoryInfo {
            name: name.into(),
            offset,
            size,
        });
    }

    #[allow(non_snake_case)]
    pub fn FindItem(&self, name: &str) -> Option<&DirectoryInfo> {
        self.entries
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
    }
}
