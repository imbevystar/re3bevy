//! re3 reference: references/re-GTA/src/save/PCSave.cpp
//! re3 reference: references/re-GTA/src/save/PCSave.h
//!
//! PC save-slot helper state.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use bevy::prelude::Resource;

use super::{
    GenericGameStorage::{
        CGenericGameStorage, SLOT_COUNT, SaveBlock, SaveFileBlocks, SaveSimpleVars, SaveSlotState,
        SaveStorageError, TOP_LINE_EMPTY_FILE, build_generic_save_blocks,
        default_pc_save_file_name, encode_pc_save_image, parse_save_file_blocks,
        read_save_file_blocks, save_slot_path, split_generic_load_blocks,
    },
    MemoryCard::{CMemoryCard, SLOTINFO},
    SaveBuf::align4bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum eSaveStatus {
    SAVESTATUS_SUCCESSFUL = 0,
    SAVESTATUS_ERR_SAVE_CREATE,
    SAVESTATUS_ERR_SAVE_WRITE,
    SAVESTATUS_ERR_SAVE_CLOSE,
    SAVESTATUS_ERR_LOAD_OPEN,
    SAVESTATUS_ERR_LOAD_READ,
    SAVESTATUS_ERR_LOAD_CLOSE,
    SAVESTATUS_ERR_DATA_INVALID,
    SAVESTATUS_DELETEFAILED8,
    SAVESTATUS_DELETEFAILED9,
    SAVESTATUS_DELETEFAILED10,
}

#[derive(Debug, Clone, Resource)]
pub struct C_PcSave {
    pub nErrorCode: eSaveStatus,
}

impl Default for C_PcSave {
    fn default() -> Self {
        Self {
            nErrorCode: eSaveStatus::SAVESTATUS_SUCCESSFUL,
        }
    }
}

impl C_PcSave {
    /// Mirror PC slot metadata into the frontend's re3 `CMemoryCard` facade.
    pub fn SyncMemoryCardSlots(storage: &CGenericGameStorage, memory_card: &mut CMemoryCard) {
        for slot in 0..SLOT_COUNT {
            sync_memory_card_slot_from_storage(memory_card, storage, slot);
        }
    }

    pub fn SetSaveDirectory(storage: &mut CGenericGameStorage, path: impl AsRef<Path>) {
        storage.DefaultPCSaveFileName = default_pc_save_file_name(path);
    }

    pub fn DeleteSlot(
        &mut self,
        storage: &mut CGenericGameStorage,
        slot: usize,
    ) -> io::Result<bool> {
        self.nErrorCode = eSaveStatus::SAVESTATUS_SUCCESSFUL;
        let path = save_slot_path(&storage.DefaultPCSaveFileName, slot);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if let Some(save_date) = storage.SlotSaveDate.get_mut(slot) {
            save_date.clear();
        }
        if let Some(slot_state) = storage.Slots.get_mut(slot + 1) {
            *slot_state = SaveSlotState::SlotEmpty;
        }
        Ok(true)
    }

    pub fn DeleteSlotWithMemoryCard(
        &mut self,
        storage: &mut CGenericGameStorage,
        memory_card: &mut CMemoryCard,
        slot: usize,
    ) -> io::Result<bool> {
        match self.DeleteSlot(storage, slot) {
            Ok(success) => {
                if success {
                    memory_card.DeleteSlot(slot);
                    memory_card.ClearError();
                }
                Ok(success)
            }
            Err(error) => {
                self.nErrorCode = eSaveStatus::SAVESTATUS_DELETEFAILED10;
                memory_card.ApplyPcSaveStatus(self.nErrorCode);
                Err(error)
            }
        }
    }

    pub fn PcClassSaveRoutine(
        &mut self,
        file: &mut File,
        data: &[u8],
        checksum: &mut i32,
    ) -> io::Result<bool> {
        let size = data.len() as u32;
        file.write_all(&size.to_le_bytes())?;
        let aligned = align4bytes(data.len());
        file.write_all(data)?;
        if aligned > data.len() {
            file.write_all(&vec![0; aligned - data.len()])?;
        }

        for byte in size.to_le_bytes() {
            *checksum += byte as i32;
        }
        for byte in data {
            *checksum += *byte as i32;
        }
        for _ in data.len()..aligned {
            *checksum += 0;
        }
        Ok(true)
    }

    pub fn PopulateSlotInfo(
        &mut self,
        storage: &mut CGenericGameStorage,
        save_root: impl AsRef<Path>,
    ) -> io::Result<()> {
        self.nErrorCode = eSaveStatus::SAVESTATUS_SUCCESSFUL;
        Self::SetSaveDirectory(storage, save_root);
        for index in 0..SLOT_COUNT {
            storage.Slots[index + 1] = SaveSlotState::SlotEmpty;
            storage.SlotFileName[index].clear();
            storage.SlotSaveDate[index].clear();
        }

        for index in 0..SLOT_COUNT {
            let path = save_slot_path(&storage.DefaultPCSaveFileName, index);
            if !path.exists() {
                continue;
            }

            let bytes = fs::read(&path)?;
            if starts_with_empty_file_marker(&bytes) {
                continue;
            }

            storage.Slots[index + 1] = match parse_save_file_blocks(&bytes) {
                Ok(blocks) => {
                    storage.m_LevelToLoad = blocks.level_to_load;
                    storage.CheckSum = blocks.checksum;
                    storage.SlotFileName[index] = slot_name_from_header(&bytes)
                        .unwrap_or_else(|| format!("GTA3sf{}", index + 1));
                    storage.SlotSaveDate[index] = slot_date_from_header(&bytes).unwrap_or_default();
                    SaveSlotState::SlotOk
                }
                Err(_) => SaveSlotState::SlotCorrupted,
            };
        }
        Ok(())
    }

    pub fn SaveSlotPath(storage: &mut CGenericGameStorage, slot: usize) -> PathBuf {
        PathBuf::from(storage.MakeValidSaveName(slot))
    }

    pub fn SaveSlotBlocks(
        &mut self,
        storage: &mut CGenericGameStorage,
        slot: usize,
        blocks: &[SaveBlock],
    ) -> Result<bool, SaveStorageError> {
        self.nErrorCode = eSaveStatus::SAVESTATUS_SUCCESSFUL;
        let path = Self::SaveSlotPath(storage, slot);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                self.nErrorCode = eSaveStatus::SAVESTATUS_ERR_SAVE_CREATE;
                SaveStorageError::Io(error.to_string())
            })?;
        }

        let (image, checksum) = encode_pc_save_image(blocks).inspect_err(|_| {
            self.nErrorCode = eSaveStatus::SAVESTATUS_ERR_DATA_INVALID;
        })?;

        fs::write(&path, image).map_err(|error| {
            self.nErrorCode = eSaveStatus::SAVESTATUS_ERR_SAVE_WRITE;
            storage.SaveFileNameJustSaved = storage.ValidSaveName.clone();
            SaveStorageError::Io(error.to_string())
        })?;

        storage.CheckSum = checksum;
        storage.SaveFileNameJustSaved = storage.ValidSaveName.clone();
        if let Some(slot_state) = storage.Slots.get_mut(slot + 1) {
            *slot_state = SaveSlotState::SlotOk;
        }
        Ok(true)
    }

    pub fn SaveSlotBlocksWithMemoryCard(
        &mut self,
        storage: &mut CGenericGameStorage,
        memory_card: &mut CMemoryCard,
        slot: usize,
        blocks: &[SaveBlock],
    ) -> Result<bool, SaveStorageError> {
        let result = self.SaveSlotBlocks(storage, slot, blocks);
        memory_card.ApplySaveStorageResult(&result, eSaveStatus::SAVESTATUS_ERR_SAVE_WRITE);
        if result.is_ok() {
            sync_memory_card_slot_from_storage(memory_card, storage, slot);
        }
        result
    }

    pub fn SaveSlotGeneric(
        &mut self,
        storage: &mut CGenericGameStorage,
        slot: usize,
        simple: &SaveSimpleVars,
        scripts: &[u8],
        subsystem_blocks: impl IntoIterator<Item = SaveBlock>,
    ) -> Result<bool, SaveStorageError> {
        let blocks = build_generic_save_blocks(simple, scripts, subsystem_blocks)?;
        self.SaveSlotBlocks(storage, slot, &blocks)
    }

    pub fn LoadSlotBlocks(
        &mut self,
        storage: &mut CGenericGameStorage,
        slot: usize,
    ) -> Result<SaveFileBlocks, SaveStorageError> {
        self.nErrorCode = eSaveStatus::SAVESTATUS_SUCCESSFUL;
        let path = save_slot_path(&storage.DefaultPCSaveFileName, slot);
        storage.LoadFileName = path.to_string_lossy().into_owned();
        let blocks = read_save_file_blocks(&path).map_err(|error| {
            self.nErrorCode = match error {
                SaveStorageError::Io(_) => eSaveStatus::SAVESTATUS_ERR_LOAD_OPEN,
                SaveStorageError::ChecksumMismatch { .. }
                | SaveStorageError::MissingChecksum
                | SaveStorageError::InvalidBlockOrder { .. }
                | SaveStorageError::MissingBlock { .. }
                | SaveStorageError::BlockTooLarge { .. }
                | SaveStorageError::WrongBlockCount { .. }
                | SaveStorageError::SaveImageTooLarge { .. }
                | SaveStorageError::Buffer(_) => eSaveStatus::SAVESTATUS_ERR_DATA_INVALID,
            };
            error
        })?;
        storage.CheckSum = blocks.checksum;
        storage.m_LevelToLoad = blocks.level_to_load;
        if let Some(slot_state) = storage.Slots.get_mut(slot + 1) {
            *slot_state = SaveSlotState::SlotOk;
        }
        Ok(blocks)
    }

    pub fn LoadSlotBlocksWithMemoryCard(
        &mut self,
        storage: &mut CGenericGameStorage,
        memory_card: &mut CMemoryCard,
        slot: usize,
    ) -> Result<SaveFileBlocks, SaveStorageError> {
        let result = self.LoadSlotBlocks(storage, slot);
        memory_card.ApplySaveStorageResult(&result, eSaveStatus::SAVESTATUS_ERR_LOAD_OPEN);
        if result.is_ok() {
            sync_memory_card_slot_from_storage(memory_card, storage, slot);
            memory_card.LoadSlotToBuffer(slot);
            memory_card.m_LevelToLoad = storage.m_LevelToLoad;
        }
        result
    }

    pub fn LoadSlotGeneric(
        &mut self,
        storage: &mut CGenericGameStorage,
        slot: usize,
    ) -> Result<super::GenericGameStorage::GenericLoadBlocks, SaveStorageError> {
        let blocks = self.LoadSlotBlocks(storage, slot)?;
        split_generic_load_blocks(&blocks)
    }
}

fn sync_memory_card_slot_from_storage(
    memory_card: &mut CMemoryCard,
    storage: &CGenericGameStorage,
    slot: usize,
) {
    if slot >= SLOT_COUNT {
        return;
    }
    memory_card.Slots[slot] = match storage.Slots.get(slot + 1).copied() {
        Some(SaveSlotState::SlotOk) => SLOTINFO::SLOT_PRESENT,
        Some(SaveSlotState::SlotCorrupted) => SLOTINFO::SLOT_CORRUPTED,
        _ => SLOTINFO::SLOT_NOTPRESENT,
    };
    memory_card.SlotFileName[slot] = storage.SlotFileName[slot].clone();
    memory_card.SlotSaveDate[slot] = storage.SlotSaveDate[slot].clone();
    memory_card.SaveFileNameJustSaved = storage.SaveFileNameJustSaved.clone();
    memory_card.ValidSaveName = storage.ValidSaveName.clone();
}

fn starts_with_empty_file_marker(bytes: &[u8]) -> bool {
    bytes.starts_with(TOP_LINE_EMPTY_FILE.as_bytes())
}

fn slot_name_from_header(bytes: &[u8]) -> Option<String> {
    let name_bytes = bytes.get(4..4 + 24 * 2)?;
    let mut utf16 = Vec::new();
    for raw in name_bytes.chunks_exact(2) {
        let value = u16::from_le_bytes([raw[0], raw[1]]);
        if value == 0 {
            break;
        }
        utf16.push(value);
    }
    String::from_utf16(&utf16).ok()
}

fn slot_date_from_header(bytes: &[u8]) -> Option<String> {
    let date_offset = 4 + 24 * 2;
    let date_bytes = bytes.get(date_offset..date_offset + 16)?;
    let year = u16::from_le_bytes([date_bytes[0], date_bytes[1]]);
    let month = u16::from_le_bytes([date_bytes[2], date_bytes[3]]);
    let day = u16::from_le_bytes([date_bytes[6], date_bytes[7]]);
    let hour = u16::from_le_bytes([date_bytes[8], date_bytes[9]]);
    let minute = u16::from_le_bytes([date_bytes[10], date_bytes[11]]);
    let second = u16::from_le_bytes([date_bytes[12], date_bytes[13]]);
    let month = match month {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        12 => "DEC",
        _ => return None,
    };
    Some(format!(
        "{day:02} {month} {year:04} {hour:02}:{minute:02}:{second:02}"
    ))
}
