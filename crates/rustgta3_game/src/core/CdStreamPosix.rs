//! re3 reference: references/re-GTA/src/core/CdStreamPosix.cpp
//!
//! POSIX CdStream backend descriptor. Actual streaming uses the shared CdStream queue.

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdStreamPosixFile {
    pub path: String,
    pub handle_index: usize,
}

#[allow(non_snake_case)]
pub fn CdStreamPosixOpen(path: impl Into<String>, handle_index: usize) -> CdStreamPosixFile {
    CdStreamPosixFile {
        path: path.into(),
        handle_index,
    }
}
