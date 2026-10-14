//! re3 reference: references/re-GTA/src/save/SaveBuf.h
//!
//! Save-buffer cursor helpers matching the re3 pointer macros.

use std::{fmt, mem};

pub const SAVE_HEADER_SIZE: usize = 4 * mem::size_of::<u8>() + mem::size_of::<u32>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveBufError {
    ReadPastEnd {
        offset: usize,
        len: usize,
        available: usize,
    },
    HeaderMismatch {
        expected: [u8; 4],
        found: [u8; 4],
    },
    HeaderSizeMismatch {
        expected: u32,
        found: u32,
    },
}

impl fmt::Display for SaveBufError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveBufError::ReadPastEnd {
                offset,
                len,
                available,
            } => write!(
                f,
                "save buffer read past end at {offset} len {len} available {available}"
            ),
            SaveBufError::HeaderMismatch { expected, found } => write!(
                f,
                "save header mismatch expected {} found {}",
                String::from_utf8_lossy(expected),
                String::from_utf8_lossy(found)
            ),
            SaveBufError::HeaderSizeMismatch { expected, found } => {
                write!(
                    f,
                    "save header size mismatch expected {expected} found {found}"
                )
            }
        }
    }
}

impl std::error::Error for SaveBufError {}

pub trait SavePrimitive: Sized {
    const SIZE: usize;

    fn read_from_le(bytes: &[u8]) -> Self;
    fn write_to_le(self, out: &mut Vec<u8>);
}

macro_rules! impl_save_primitive {
    ($ty:ty) => {
        impl SavePrimitive for $ty {
            const SIZE: usize = mem::size_of::<$ty>();

            fn read_from_le(bytes: &[u8]) -> Self {
                let mut raw = [0; mem::size_of::<$ty>()];
                raw.copy_from_slice(&bytes[..mem::size_of::<$ty>()]);
                <$ty>::from_le_bytes(raw)
            }

            fn write_to_le(self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        }
    };
}

impl_save_primitive!(u8);
impl_save_primitive!(i8);
impl_save_primitive!(u16);
impl_save_primitive!(i16);
impl_save_primitive!(u32);
impl_save_primitive!(i32);
impl_save_primitive!(u64);
impl_save_primitive!(i64);
impl_save_primitive!(f32);
impl_save_primitive!(f64);

#[derive(Debug, Clone)]
pub struct SaveBufReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    validate_count: usize,
}

impl<'a> SaveBufReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            validate_count: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn validate_count(&self) -> usize {
        self.validate_count
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn skip(&mut self, len: usize) -> Result<(), SaveBufError> {
        self.slice(len).map(|_| ())
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], SaveBufError> {
        self.slice(len)
    }

    pub fn read<T: SavePrimitive>(&mut self) -> Result<T, SaveBufError> {
        let bytes = self.slice(T::SIZE)?;
        Ok(T::read_from_le(bytes))
    }

    pub fn check_header(&mut self, ident: [u8; 4], expected_size: u32) -> Result<(), SaveBufError> {
        let found = [
            self.read::<u8>()?,
            self.read::<u8>()?,
            self.read::<u8>()?,
            self.read::<u8>()?,
        ];
        if found != ident {
            return Err(SaveBufError::HeaderMismatch {
                expected: ident,
                found,
            });
        }

        let found_size = self.read::<u32>()?;
        if found_size != expected_size {
            return Err(SaveBufError::HeaderSizeMismatch {
                expected: expected_size,
                found: found_size,
            });
        }
        Ok(())
    }

    fn slice(&mut self, len: usize) -> Result<&'a [u8], SaveBufError> {
        let start = self.offset;
        let end = start.checked_add(len).ok_or(SaveBufError::ReadPastEnd {
            offset: start,
            len,
            available: self.bytes.len(),
        })?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(SaveBufError::ReadPastEnd {
                offset: start,
                len,
                available: self.bytes.len(),
            })?;
        self.offset = end;
        self.validate_count += len;
        Ok(slice)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SaveBufWriter {
    bytes: Vec<u8>,
    validate_count: usize,
}

impl SaveBufWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let validate_count = bytes.len();
        Self {
            bytes,
            validate_count,
        }
    }

    pub fn offset(&self) -> usize {
        self.bytes.len()
    }

    pub fn validate_count(&self) -> usize {
        self.validate_count
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    pub fn skip(&mut self, len: usize) {
        self.bytes.resize(self.bytes.len() + len, 0);
        self.validate_count += len;
    }

    pub fn write<T: SavePrimitive>(&mut self, value: T) -> usize {
        let offset = self.bytes.len();
        value.write_to_le(&mut self.bytes);
        self.validate_count += T::SIZE;
        offset
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> usize {
        let offset = self.bytes.len();
        self.bytes.extend_from_slice(bytes);
        self.validate_count += bytes.len();
        offset
    }

    pub fn zero(&mut self, len: usize) {
        self.skip(len);
    }

    pub fn write_header(&mut self, ident: [u8; 4], size: u32) {
        for byte in ident {
            self.write(byte);
        }
        self.write(size);
    }
}

pub fn align4bytes(size: usize) -> usize {
    (size + 3) & !3
}

pub fn checksum_add_aligned(checksum: &mut i32, size: u32, data: &[u8]) {
    for byte in size.to_le_bytes() {
        *checksum += byte as i32;
    }
    for byte in data.iter().take(align4bytes(size as usize)) {
        *checksum += *byte as i32;
    }
}
