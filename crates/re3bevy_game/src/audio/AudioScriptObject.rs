//! re3 reference: references/re-GTA/src/audio/AudioScriptObject.cpp
//! re3 reference: references/re-GTA/src/audio/AudioScriptObject.h

use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;

use crate::{
    core::Pools::{CPool, CPoolHandle, NUMAUDIOSCRIPTOBJECTS},
    save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter},
};

use super::DMAudio::cDMAudio;
use super::soundlist::SCRIPT_SOUND_INVALID;

pub const NUM_SCRIPT_MAX_ENTITIES: usize = 40;
pub const AEHANDLE_NONE: i32 = -5;
pub const AUDIO_SCRIPT_OBJECT_SAVE_RECORD_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct cAudioScriptObject {
    pub AudioId: i16,
    pub Posn: CVector,
    pub AudioEntity: i32,
}

impl Default for cAudioScriptObject {
    fn default() -> Self {
        let mut object = Self {
            AudioId: SCRIPT_SOUND_INVALID as i16,
            Posn: CVector::ZERO,
            AudioEntity: AEHANDLE_NONE,
        };
        object.Reset();
        object
    }
}

impl cAudioScriptObject {
    pub fn Reset(&mut self) {
        self.AudioId = SCRIPT_SOUND_INVALID as i16;
        self.Posn = CVector::ZERO;
        self.AudioEntity = AEHANDLE_NONE;
    }

    pub fn new(audio_id: i16, posn: CVector) -> Self {
        Self {
            AudioId: audio_id,
            Posn: posn,
            AudioEntity: AEHANDLE_NONE,
        }
    }

    pub fn read_from_save(reader: &mut SaveBufReader<'_>) -> Result<Self, SaveBufError> {
        let audio_id = reader.read::<i16>()?;
        reader.skip(2)?;
        Ok(Self {
            AudioId: audio_id,
            Posn: CVector::new(
                reader.read::<f32>()?,
                reader.read::<f32>()?,
                reader.read::<f32>()?,
            ),
            AudioEntity: reader.read::<i32>()?,
        })
    }

    pub fn write_to_save(self, writer: &mut SaveBufWriter) {
        writer.write(self.AudioId);
        writer.write(0_i16);
        writer.write(self.Posn.x);
        writer.write(self.Posn.y);
        writer.write(self.Posn.z);
        writer.write(self.AudioEntity);
    }
}

#[derive(Debug, Clone, Resource)]
pub struct cAudioScriptObjectPool {
    pub pool: CPool<cAudioScriptObject>,
}

impl Default for cAudioScriptObjectPool {
    fn default() -> Self {
        Self::initialise()
    }
}

impl cAudioScriptObjectPool {
    pub fn initialise() -> Self {
        Self {
            pool: CPool::new(NUMAUDIOSCRIPTOBJECTS),
        }
    }

    pub fn New(&mut self, object: cAudioScriptObject) -> Option<CPoolHandle> {
        self.pool.new_entry(object)
    }

    pub fn NewAt(&mut self, handle: CPoolHandle, object: cAudioScriptObject) -> Option<usize> {
        self.pool.new_at(handle, object)
    }

    pub fn Delete(&mut self, handle: CPoolHandle) -> Option<cAudioScriptObject> {
        self.pool.delete_by_slot(handle.index())
    }

    pub fn GetAt(&self, handle: CPoolHandle) -> Option<&cAudioScriptObject> {
        self.pool.get_at(handle)
    }

    pub fn GetAtMut(&mut self, handle: CPoolHandle) -> Option<&mut cAudioScriptObject> {
        self.pool.get_at_mut(handle)
    }

    pub fn SaveAllAudioScriptObjects(&self) -> Vec<u8> {
        let pool_size = self.pool.get_no_of_used_spaces();
        let payload_size = 4 + pool_size * (4 + AUDIO_SCRIPT_OBJECT_SAVE_RECORD_SIZE);
        let mut writer = SaveBufWriter::new();
        writer.write_header([b'A', b'U', b'D', 0], payload_size as u32);
        writer.write(pool_size as i32);

        let mut i = self.pool.get_size();
        while i > 0 {
            i -= 1;
            if let Some(object) = self.pool.get_slot(i) {
                writer.write(self.pool.get_index_from_slot(i).0);
                object.write_to_save(&mut writer);
            }
        }

        writer.into_inner()
    }

    pub fn LoadAllAudioScriptObjects(
        bytes: &[u8],
        dmaudio: Option<&mut cDMAudio>,
    ) -> Result<Self, SaveBufError> {
        let mut reader = SaveBufReader::new(bytes);
        let payload_size = bytes.len().saturating_sub(SAVE_HEADER_SIZE) as u32;
        reader.check_header([b'A', b'U', b'D', 0], payload_size)?;

        let pool_size = reader.read::<i32>()?.max(0) as usize;
        let mut pool = Self::initialise();
        let mut dmaudio = dmaudio;
        for _ in 0..pool_size {
            let handle = CPoolHandle(reader.read::<i32>()?);
            let mut object = cAudioScriptObject::read_from_save(&mut reader)?;
            if let Some(audio) = dmaudio.as_deref_mut() {
                object.AudioEntity = audio.CreateLoopingScriptObject(&mut object);
            }
            let _ = pool.NewAt(handle, object);
        }

        Ok(pool)
    }
}
