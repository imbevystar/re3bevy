//! re3 reference: references/re-GTA/src/core/FileMgr.cpp
//! re3 reference: references/re-GTA/src/core/FileMgr.h
//!
//! Fixed-size GTA file table with root/current-directory semantics.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub const NUMFILES: usize = 20;

#[derive(Debug)]
struct CFileSlot {
    file: File,
    is_text: bool,
    pending_cr: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CSeekWhence {
    Start,
    Current,
    End,
}

#[derive(Debug)]
pub struct CFileMgr {
    root_dir_name: PathBuf,
    dir_name: PathBuf,
    files: Vec<Option<CFileSlot>>,
}

impl CFileMgr {
    pub fn initialise(root_dir_name: impl Into<PathBuf>) -> Self {
        let root_dir_name = root_dir_name.into();
        let dir_name = root_dir_name.clone();
        let mut files = Vec::with_capacity(NUMFILES);
        files.resize_with(NUMFILES, || None);
        Self {
            root_dir_name,
            dir_name,
            files,
        }
    }

    pub fn root_dir_name(&self) -> &Path {
        &self.root_dir_name
    }

    pub fn dir_name(&self) -> &Path {
        &self.dir_name
    }

    pub fn change_dir(&mut self, dir: &str) {
        if dir.starts_with('\\') || dir.starts_with('/') {
            self.dir_name = self.root_dir_name.clone();
            self.push_gta_dir(dir.trim_start_matches(['\\', '/']));
        } else if !dir.is_empty() {
            self.push_gta_dir(dir);
        }
    }

    pub fn set_dir(&mut self, dir: &str) {
        self.dir_name = self.root_dir_name.clone();
        if !dir.is_empty() {
            self.push_gta_dir(dir);
        }
    }

    pub fn set_dir_my_documents(&mut self, user_folder: impl AsRef<Path>) {
        self.set_dir("");
        self.dir_name = user_folder.as_ref().to_path_buf();
    }

    pub fn load_file(&mut self, file: &str, max_len: usize, mode: &str) -> io::Result<Vec<u8>> {
        let fd = self.open_file(file, mode)?;
        let mut out = Vec::new();
        loop {
            let mut chunk = vec![0; 0x4000];
            let read = self.read(fd, &mut chunk)?;
            out.extend_from_slice(&chunk[..read]);
            if out.len() >= max_len {
                self.close_file(fd)?;
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("CFileMgr::LoadFile {file} exceeded max length {max_len}"),
                ));
            }
            if read != 0x4000 {
                break;
            }
        }
        out.push(0);
        self.close_file(fd)?;
        Ok(out)
    }

    /// Read a binary GTA data file to EOF without inventing an arbitrary size
    /// ceiling. This is used for container-style files such as MAIN.SCM whose
    /// mission payload follows re3's fixed 128 KiB main script-space segment.
    pub fn read_file_to_end(&mut self, file: &str) -> io::Result<Vec<u8>> {
        let fd = self.open_file(file, "rb")?;
        let result = (|| {
            let mut out = Vec::new();
            let mut chunk = [0u8; 0x4000];
            loop {
                let read = self.read(fd, &mut chunk)?;
                if read == 0 {
                    break;
                }
                out.extend_from_slice(&chunk[..read]);
            }
            Ok(out)
        })();
        let close_result = self.close_file(fd);
        match (result, close_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(bytes), Ok(())) => Ok(bytes),
        }
    }

    pub fn open_file(&mut self, file: &str, mode: &str) -> io::Result<usize> {
        let fd = self
            .first_free_fd()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "CFileMgr fd table is full"))?;
        let path = self.resolve_existing_case_path(file);
        let is_text = !mode.contains('b');
        let mut options = OpenOptions::new();

        if mode.contains('r') {
            options.read(true);
        }
        if mode.contains('w') {
            options.write(true).create(true).truncate(true);
        }
        if mode.contains('a') {
            options.append(true).create(true);
        }
        if mode.contains('+') {
            options.read(true).write(true);
        }

        let file = options.open(path)?;
        self.files[fd] = Some(CFileSlot {
            file,
            is_text,
            pending_cr: false,
        });
        Ok(fd)
    }

    pub fn open_file_for_writing(&mut self, file: &str) -> io::Result<usize> {
        self.open_file(file, "wb")
    }

    pub fn read(&mut self, fd: usize, buf: &mut [u8]) -> io::Result<usize> {
        let slot = self.slot_mut(fd)?;
        if !slot.is_text {
            return slot.file.read(buf);
        }

        let mut written = 0;
        while written < buf.len() {
            let byte = match read_text_byte(slot)? {
                Some(byte) => byte,
                None => break,
            };
            buf[written] = byte;
            written += 1;
        }
        Ok(written)
    }

    pub fn write(&mut self, fd: usize, buf: &[u8]) -> io::Result<usize> {
        let slot = self.slot_mut(fd)?;
        if !slot.is_text {
            return slot.file.write(buf);
        }

        let mut written = 0;
        for byte in buf {
            if *byte == b'\n' {
                slot.file.write_all(b"\r")?;
            }
            slot.file.write_all(&[*byte])?;
            written += 1;
        }
        Ok(written)
    }

    pub fn seek(&mut self, fd: usize, offset: i64, whence: CSeekWhence) -> io::Result<bool> {
        let slot = self.slot_mut(fd)?;
        let seek_from = match whence {
            CSeekWhence::Start => SeekFrom::Start(offset as u64),
            CSeekWhence::Current => SeekFrom::Current(offset),
            CSeekWhence::End => SeekFrom::End(offset),
        };
        slot.pending_cr = false;
        Ok(slot.file.seek(seek_from).is_err())
    }

    pub fn read_line(&mut self, fd: usize, buf: &mut String, max_len: usize) -> io::Result<bool> {
        buf.clear();
        if max_len == 0 {
            return Ok(false);
        }

        let slot = self.slot_mut(fd)?;
        while buf.len() + 1 < max_len {
            let byte = if slot.is_text {
                read_text_byte(slot)?
            } else {
                let mut one = [0];
                match slot.file.read(&mut one)? {
                    0 => None,
                    _ => Some(one[0]),
                }
            };

            let Some(byte) = byte else {
                return Ok(!buf.is_empty());
            };

            buf.push(byte as char);
            if byte == b'\n' {
                break;
            }
        }

        Ok(true)
    }

    pub fn close_file(&mut self, fd: usize) -> io::Result<()> {
        self.validate_fd(fd)?;
        self.files[fd] = None;
        Ok(())
    }

    pub fn get_error_read_write(&mut self, fd: usize) -> io::Result<bool> {
        let slot = self.slot_mut(fd)?;
        let pos = slot.file.stream_position()?;
        let len = slot.file.metadata()?.len();
        Ok(pos >= len)
    }

    pub fn resolve(&self, file: &str) -> PathBuf {
        let base = if file.starts_with(['\\', '/']) {
            &self.root_dir_name
        } else {
            &self.dir_name
        };
        resolve_gta_path_from(base, file)
    }

    pub fn resolve_existing_case_path(&self, file: &str) -> PathBuf {
        let base = if file.starts_with(['\\', '/']) {
            &self.root_dir_name
        } else {
            &self.dir_name
        };
        resolve_existing_case_path_from(base, file)
    }

    fn push_gta_dir(&mut self, dir: &str) {
        self.dir_name = resolve_existing_case_path_from(&self.dir_name, dir);
    }

    fn first_free_fd(&self) -> Option<usize> {
        (1..NUMFILES).find(|fd| self.files[*fd].is_none())
    }

    fn slot_mut(&mut self, fd: usize) -> io::Result<&mut CFileSlot> {
        self.validate_fd(fd)?;
        self.files[fd]
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "CFileMgr fd is closed"))
    }

    fn validate_fd(&self, fd: usize) -> io::Result<()> {
        if fd < NUMFILES {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CFileMgr fd is out of range",
            ))
        }
    }
}

pub fn resolve_gta_path_from(base: &Path, gta_path: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in gta_path
        .split(['\\', '/'])
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != ".")
    {
        if part == ".." {
            path.pop();
        } else {
            path.push(part);
        }
    }
    path
}

pub fn resolve_existing_case_path_from(base: &Path, gta_path: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in gta_path
        .split(['\\', '/'])
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != ".")
    {
        if part == ".." {
            path.pop();
            continue;
        }
        let next = case_insensitive_child(&path, part).unwrap_or_else(|| path.join(part));
        path = next;
    }
    path
}

fn case_insensitive_child(parent: &Path, wanted: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if file_name.to_string_lossy().eq_ignore_ascii_case(wanted) {
            return Some(entry.path());
        }
    }
    None
}

fn read_text_byte(slot: &mut CFileSlot) -> io::Result<Option<u8>> {
    if slot.pending_cr {
        slot.pending_cr = false;
        return Ok(Some(b'\r'));
    }

    let mut one = [0];
    if slot.file.read(&mut one)? == 0 {
        return Ok(None);
    }

    if one[0] == b'\r' {
        let mut next = [0];
        if slot.file.read(&mut next)? == 0 {
            return Ok(Some(b'\r'));
        }
        if next[0] == b'\n' {
            return Ok(Some(b'\n'));
        }
        slot.file.seek(SeekFrom::Current(-1))?;
        return Ok(Some(b'\r'));
    }

    Ok(Some(one[0]))
}
