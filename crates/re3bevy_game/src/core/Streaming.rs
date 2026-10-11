//! re3 reference: references/re-GTA/src/core/Streaming.cpp
//! re3 reference: references/re-GTA/src/core/Streaming.h
//!
//! Streaming list/channel state port. Object conversion, TXD lookup and
//! modelinfo-specific decisions stay outside this module until those C++
//! systems are ported.

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use bevy::prelude::Resource;
use re3bevy_assets::install::FileMgr::resolve_existing_case_path_from;
use re3bevy_assets::streaming::CdStream::{CDSTREAM_SECTOR_SIZE, CdStream, StreamStatus};
use re3bevy_formats::formats::Directory::{CDirectory, DirectoryInfo};

use crate::modelinfo::ModelIndices::NUM_DEFAULT_MODELS;

pub const MODELINFOSIZE: usize = 5500;
pub const TXDSTORESIZE: usize = 850;
pub const EXTRADIRSIZE: usize = 128;
pub const MAXVEHICLESLOADED: usize = 50;
pub const STREAM_OFFSET_TXD: usize = MODELINFOSIZE;
pub const NUMSTREAMINFO: usize = STREAM_OFFSET_TXD + TXDSTORESIZE;

pub const STREAMFLAGS_DONT_REMOVE: u8 = 0x01;
pub const STREAMFLAGS_SCRIPTOWNED: u8 = 0x02;
pub const STREAMFLAGS_DEPENDENCY: u8 = 0x04;
pub const STREAMFLAGS_PRIORITY: u8 = 0x08;
pub const STREAMFLAGS_NOFADE: u8 = 0x10;
pub const STREAMFLAGS_CANT_REMOVE: u8 = STREAMFLAGS_DONT_REMOVE | STREAMFLAGS_SCRIPTOWNED;
pub const STREAMFLAGS_KEEP_IN_MEMORY: u8 =
    STREAMFLAGS_DONT_REMOVE | STREAMFLAGS_SCRIPTOWNED | STREAMFLAGS_DEPENDENCY;
pub const NUMCDIMAGES: usize = 12;
pub const DESIRED_NUM_VEHICLES_LOADED: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamLoadState {
    NotLoaded = 0,
    Loaded = 1,
    InQueue = 2,
    Reading = 3,
    Started = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ChannelState {
    Idle = 0,
    Reading = 1,
    Started = 2,
    Error = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModelInfoType {
    Na = 0,
    Simple = 1,
    Mlo = 2,
    Time = 3,
    Clump = 4,
    Vehicle = 5,
    Ped = 6,
    ExtraComps = 7,
}

impl ModelInfoType {
    pub fn is_simple(self) -> bool {
        matches!(self, Self::Simple | Self::Time)
    }

    pub fn is_clump(self) -> bool {
        matches!(
            self,
            Self::Clump | Self::Ped | Self::Vehicle | Self::Mlo | Self::ExtraComps
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertResult {
    Loaded,
    Started,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CStreamingConvertContext {
    pub loading_big_model: bool,
    pub cd_size_sectors: u32,
    /// Flags from the request being converted. re3 consumes these after the
    /// DFF has loaded to decide whether a simple model starts fully opaque or
    /// enters the normal distance-fade path.
    pub stream_flags: u8,
}

pub trait CStreamingModelInfo {
    fn model_name(&self, model_id: usize) -> Option<String>;
    fn model_id_by_name(&mut self, name: &str) -> Option<usize>;
    fn set_model_name(&mut self, model_id: usize, name: &str);
    fn txd_id_by_name_or_add(&mut self, name: &str) -> Option<usize>;
    fn txd_id_by_name(&self, name: &str) -> Option<usize>;
    fn txd_slot_for_model(&self, model_id: usize) -> Option<usize>;
    fn clear_txd_slot_for_model(&mut self, model_id: usize);
    fn set_txd_slot_for_model(&mut self, model_id: usize, name: &str);
    fn model_type(&self, model_id: usize) -> Option<ModelInfoType>;
    fn model_num_refs(&self, model_id: usize) -> u16;
    fn remove_model_ref(&mut self, model_id: usize) -> Option<usize>;
    fn txd_num_refs(&self, txd_id: usize) -> u16;
    fn is_txd_loaded(&self, txd_id: usize) -> bool;
    fn add_txd_ref(&mut self, txd_id: usize);
    fn remove_txd_ref_without_delete(&mut self, txd_id: usize);
    fn delete_model_rw_object(&mut self, model_id: usize);
    fn remove_txd(&mut self, txd_id: usize);
}

pub trait CStreamingObjectLoader {
    fn convert_buffer_to_object(
        &mut self,
        stream_id: usize,
        bytes: &[u8],
        context: CStreamingConvertContext,
    ) -> ConvertResult;
    fn finish_loading_large_file(&mut self, stream_id: usize, bytes: &[u8]) -> bool;
}

pub trait CStreamingRuntime: CStreamingModelInfo + CStreamingObjectLoader {}

impl<T> CStreamingRuntime for T where T: CStreamingModelInfo + CStreamingObjectLoader {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamList {
    Loaded,
    Requested,
}

struct StreamListIter<'a> {
    info_for_model: &'a [CStreamingInfo],
    next: Option<usize>,
}

impl Iterator for StreamListIter<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next?;
        let info = self.info_for_model.get(id)?;
        self.next = info.list_next;
        Some(id)
    }
}

#[derive(Debug, Clone)]
pub struct CStreamingInfo {
    pub load_state: StreamLoadState,
    pub flags: u8,
    pub next_id: i16,
    pub position: u32,
    pub size: u32,
    list: Option<StreamList>,
    list_prev: Option<usize>,
    list_next: Option<usize>,
}

impl Default for CStreamingInfo {
    fn default() -> Self {
        Self {
            load_state: StreamLoadState::NotLoaded,
            flags: 0,
            next_id: -1,
            position: 0,
            size: 0,
            list: None,
            list_prev: None,
            list_next: None,
        }
    }
}

impl CStreamingInfo {
    pub fn get_cd_posn_and_size(&self) -> Option<(u32, u32)> {
        (self.size != 0).then_some((self.position, self.size))
    }

    pub fn set_cd_posn_and_size(&mut self, posn: u32, size: u32) {
        self.position = posn;
        self.size = size;
    }

    pub fn get_cd_size(&self) -> u32 {
        self.size
    }

    pub fn is_priority(&self) -> bool {
        self.flags & STREAMFLAGS_PRIORITY != 0
    }
}

#[derive(Debug, Clone)]
pub struct CStreamingChannel {
    pub stream_ids: [i32; 4],
    pub offsets: [i32; 4],
    pub state: ChannelState,
    pub field24: i32,
    pub position: i32,
    pub size: i32,
    pub num_tries: i32,
    pub status: StreamStatus,
    /// Completed CD bytes retained while a two-phase conversion is `Started`.
    pub completed_data: Option<Vec<u8>>,
}

impl Default for CStreamingChannel {
    fn default() -> Self {
        Self {
            stream_ids: [-1; 4],
            offsets: [-1; 4],
            state: ChannelState::Idle,
            field24: 0,
            position: 0,
            size: 0,
            num_tries: 0,
            status: StreamStatus::None,
            completed_data: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    Added,
    Updated,
    AlreadyLoaded,
    IgnoredOutOfRange,
}

#[derive(Debug, Clone, Resource)]
pub struct CStreaming {
    pub disable_streaming: bool,
    pub loading_big_model: bool,
    pub num_models_requested: i32,
    pub info_for_model: Vec<CStreamingInfo>,
    /// re3 stores these as intrusive doubly-linked `CStreamingInfo` lists.
    /// Keep only the two list endpoints here; each model owns its prev/next
    /// indices above, so remove/reinsert stays O(1) instead of scanning a
    /// `VecDeque` for every already-loaded world model requested this frame.
    requested_head: Option<usize>,
    requested_tail: Option<usize>,
    loaded_head: Option<usize>,
    loaded_tail: Option<usize>,
    /// Reused snapshot for C++-style requested-list scans.  The list can be
    /// modified while dependencies are repaired, so iteration cannot borrow
    /// the intrusive list directly; retaining this buffer avoids a heap
    /// allocation on every `GetNextFileOnCd` call.
    requested_scan: Vec<usize>,
    pub old_sector_x: i32,
    pub old_sector_y: i32,
    pub streaming_buffer_size: i32,
    pub memory_used: usize,
    pub channel: [CStreamingChannel; 2],
    /// re3 `CStreaming::LoadRequestedModels` function-static channel selector.
    pub current_channel: usize,
    pub channel_error: i32,
    pub num_vehicles_loaded: i32,
    pub vehicles_loaded: [i32; MAXVEHICLESLOADED],
    pub last_vehicle_deleted: i32,
    pub num_priority_requests: i32,
    pub has_loaded_lods: bool,
    pub current_ped_grp: i32,
    pub last_cull_zone: i32,
    pub loaded_gangs: u16,
    pub loaded_gang_cars: u16,
    pub current_ped_loading: i32,
    /// re3 `StreamVehiclesAndPeds` function statics, kept on the streaming
    /// resource so their lifetime follows the ECS-owned streaming session.
    pub time_before_next_vehicle_load: i32,
    pub model_quality_class: usize,
    pub image_offsets: Vec<i32>,
    pub last_image_read: i32,
    pub image_size: i32,
    pub memory_available: usize,
    pub extra_objects_dir: CDirectory,
}

impl Default for CStreaming {
    fn default() -> Self {
        Self::init2_shape(NUMSTREAMINFO)
    }
}

impl CStreaming {
    pub fn init2_shape(num_stream_info: usize) -> Self {
        Self {
            disable_streaming: false,
            loading_big_model: false,
            num_models_requested: 0,
            info_for_model: vec![CStreamingInfo::default(); num_stream_info],
            requested_head: None,
            requested_tail: None,
            loaded_head: None,
            loaded_tail: None,
            requested_scan: Vec::new(),
            old_sector_x: 0,
            old_sector_y: 0,
            streaming_buffer_size: 0,
            memory_used: 0,
            channel: [CStreamingChannel::default(), CStreamingChannel::default()],
            current_channel: 0,
            channel_error: -1,
            num_vehicles_loaded: 0,
            vehicles_loaded: [-1; MAXVEHICLESLOADED],
            last_vehicle_deleted: 0,
            num_priority_requests: 0,
            has_loaded_lods: true,
            current_ped_grp: -1,
            last_cull_zone: -1,
            loaded_gangs: 0,
            loaded_gang_cars: 0,
            current_ped_loading: 8,
            time_before_next_vehicle_load: 0,
            model_quality_class: 0,
            image_offsets: vec![-1; NUMCDIMAGES],
            last_image_read: 0,
            image_size: 0,
            memory_available: 0,
            extra_objects_dir: CDirectory::new(EXTRADIRSIZE),
        }
    }

    pub fn requested_list(&self) -> impl Iterator<Item = usize> + '_ {
        StreamListIter {
            info_for_model: &self.info_for_model,
            next: self.requested_head,
        }
    }

    pub fn loaded_list(&self) -> impl Iterator<Item = usize> + '_ {
        StreamListIter {
            info_for_model: &self.info_for_model,
            next: self.loaded_head,
        }
    }

    pub fn has_model_loaded(&self, id: usize) -> bool {
        self.info_for_model[id].load_state == StreamLoadState::Loaded
    }

    pub fn can_remove_model(&self, id: usize) -> bool {
        self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE == 0
    }

    pub fn request_model(&mut self, id: usize, flags: u8) -> RequestOutcome {
        self.request_model_inner(id, flags, None::<usize>)
    }

    pub fn request_model_with_adapter<M: CStreamingModelInfo>(
        &mut self,
        id: usize,
        flags: u8,
        model_info: &M,
    ) -> RequestOutcome {
        let txd_slot = (id < STREAM_OFFSET_TXD)
            .then(|| model_info.txd_slot_for_model(id))
            .flatten();
        self.request_model_inner(id, flags, txd_slot)
    }

    fn request_model_inner(
        &mut self,
        id: usize,
        mut flags: u8,
        dependent_txd: Option<usize>,
    ) -> RequestOutcome {
        if id >= self.info_for_model.len() {
            return RequestOutcome::IgnoredOutOfRange;
        }

        let load_state = self.info_for_model[id].load_state;
        if load_state == StreamLoadState::InQueue {
            if flags & STREAMFLAGS_PRIORITY != 0 && !self.info_for_model[id].is_priority() {
                self.num_priority_requests += 1;
                self.info_for_model[id].flags |= STREAMFLAGS_PRIORITY;
            }
        } else if load_state != StreamLoadState::NotLoaded {
            flags &= !STREAMFLAGS_PRIORITY;
        }

        self.info_for_model[id].flags |= flags;

        if load_state == StreamLoadState::Loaded {
            self.reinsert_loaded_model(id);
            RequestOutcome::AlreadyLoaded
        } else if load_state == StreamLoadState::NotLoaded {
            if let Some(txd_id) = dependent_txd {
                self.request_txd(txd_id, flags);
            }
            self.add_to_requested_list_front(id);
            self.num_models_requested += 1;
            if flags & STREAMFLAGS_PRIORITY != 0 {
                self.num_priority_requests += 1;
            }
            self.info_for_model[id].load_state = StreamLoadState::InQueue;
            self.info_for_model[id].flags = flags;
            RequestOutcome::Added
        } else {
            RequestOutcome::Updated
        }
    }

    pub fn request_txd(&mut self, txd: usize, flags: u8) -> RequestOutcome {
        self.request_model(txd + STREAM_OFFSET_TXD, flags)
    }

    pub fn re_request_model(&mut self, id: usize) -> RequestOutcome {
        let flags = self.info_for_model[id].flags;
        self.request_model(id, flags)
    }

    pub fn re_request_txd(&mut self, txd: usize) -> RequestOutcome {
        self.re_request_model(txd + STREAM_OFFSET_TXD)
    }

    pub fn remove_current_zones_models(&mut self) {
        self.current_ped_grp = -1;
        self.loaded_gangs = 0;
        self.loaded_gang_cars = 0;
    }

    pub fn request_special_model<M: CStreamingModelInfo>(
        &mut self,
        model_id: usize,
        model_name: &str,
        flags: u8,
        model_info: &mut M,
    ) -> RequestOutcome {
        if model_info
            .model_name(model_id)
            .is_some_and(|name| name.eq_ignore_ascii_case(model_name))
        {
            return self.request_model_with_adapter(model_id, flags, model_info);
        }

        let old_name = model_info.model_name(model_id).unwrap_or_default();
        model_info.set_model_name(model_id, model_name);

        if model_info.model_id_by_name(&old_name).is_some() {
            if let Some(txd_id) = model_info.txd_id_by_name(&old_name) {
                if model_info.is_txd_loaded(txd_id) {
                    model_info.add_txd_ref(txd_id);
                    self.remove_model_with_adapter(model_id, Some(model_info));
                    model_info.remove_txd_ref_without_delete(txd_id);
                } else {
                    self.remove_model_with_adapter(model_id, Some(model_info));
                }
            } else {
                self.remove_model_with_adapter(model_id, Some(model_info));
            }
        } else {
            self.remove_model_with_adapter(model_id, Some(model_info));
        }

        let (position, size) = self
            .extra_objects_dir
            .find_item(model_name)
            .expect("special model is missing from extra objects directory");
        model_info.clear_txd_slot_for_model(model_id);
        if model_info.txd_id_by_name(model_name).is_none() {
            model_info.set_txd_slot_for_model(model_id, "generic");
        } else {
            model_info.set_txd_slot_for_model(model_id, model_name);
        }
        self.info_for_model[model_id].set_cd_posn_and_size(position, size);
        self.request_model_with_adapter(model_id, flags, model_info)
    }

    pub fn request_special_char<M: CStreamingModelInfo>(
        &mut self,
        char_id: usize,
        model_name: &str,
        flags: u8,
        model_info: &mut M,
    ) -> RequestOutcome {
        self.request_special_model(
            crate::modelinfo::ModelIndices::MI_SPECIAL01 + char_id,
            model_name,
            flags,
            model_info,
        )
    }

    pub fn set_mission_doesnt_require_special_char(&mut self, char_id: usize) {
        let model_id = crate::modelinfo::ModelIndices::MI_SPECIAL01 + char_id;
        self.set_mission_doesnt_require_model(model_id);
    }

    pub fn set_mission_doesnt_require_model(&mut self, model_id: usize) {
        let info = &mut self.info_for_model[model_id];
        info.flags &= !STREAMFLAGS_SCRIPTOWNED;
        if info.flags & STREAMFLAGS_DONT_REMOVE == 0 && info.load_state != StreamLoadState::Loaded {
            self.remove_model(model_id);
        }
    }

    /// re3 `CStreaming::SetModelIsDeletable` for model/stream entries whose
    /// metadata is available through the normal FileLoader adapter.
    pub fn set_model_is_deletable<M: CStreamingModelInfo>(
        &mut self,
        model_id: usize,
        model_info: &M,
    ) {
        if model_id >= self.info_for_model.len() {
            return;
        }
        self.info_for_model[model_id].flags &= !STREAMFLAGS_DONT_REMOVE;
        let is_vehicle = model_id < STREAM_OFFSET_TXD
            && model_info.model_type(model_id) == Some(ModelInfoType::Vehicle);
        if !is_vehicle
            && self.info_for_model[model_id].flags & STREAMFLAGS_SCRIPTOWNED == 0
            && self.info_for_model[model_id].load_state != StreamLoadState::Loaded
        {
            self.remove_model(model_id);
        }
    }

    /// re3 `CStreaming::SetModelTxdIsDeletable`.
    pub fn set_model_txd_is_deletable<M: CStreamingModelInfo>(
        &mut self,
        model_id: usize,
        model_info: &M,
    ) {
        let Some(txd_slot) = model_info.txd_slot_for_model(model_id) else {
            return;
        };
        self.set_model_is_deletable(txd_slot + STREAM_OFFSET_TXD, model_info);
    }

    pub fn remove_model(&mut self, id: usize) {
        self.remove_model_with_adapter(id, None::<&mut NullStreamingModelInfo>);
    }

    pub fn remove_model_ref_with_adapter<M: CStreamingModelInfo>(
        &mut self,
        model_id: usize,
        model_info: &mut M,
    ) {
        // In re3 C++, removing an entity's RwObject only decrements the model ref count
        // and CTxdStore ref count. It NEVER deletes the TXD or the model from memory.
        // Loaded TXDs and DFF geometries remain resident in memory until memory limits
        // force eviction via RemoveLeastUsedModel or level transition.
        let _ = model_info.remove_model_ref(model_id);
    }

    pub fn remove_model_with_adapter<M: CStreamingModelInfo>(
        &mut self,
        id: usize,
        mut model_info: Option<&mut M>,
    ) {
        if id >= self.info_for_model.len() {
            return;
        }
        if self.info_for_model[id].load_state == StreamLoadState::NotLoaded {
            return;
        }

        if self.info_for_model[id].load_state == StreamLoadState::Loaded {
            if let Some(model_info) = model_info.as_deref_mut() {
                if id < STREAM_OFFSET_TXD {
                    model_info.delete_model_rw_object(id);
                } else {
                    model_info.remove_txd(id - STREAM_OFFSET_TXD);
                }
            }
        }

        match self.info_for_model[id].list {
            Some(StreamList::Requested) => {
                if self.info_for_model[id].load_state == StreamLoadState::InQueue {
                    self.decrement_ref(id);
                }
                self.remove_from_requested_list(id);
            }
            Some(StreamList::Loaded) => {
                self.remove_from_loaded_list(id);
            }
            None if self.info_for_model[id].load_state == StreamLoadState::Reading => {
                for channel in &mut self.channel {
                    for stream_id in &mut channel.stream_ids {
                        if *stream_id == id as i32 {
                            *stream_id = -1;
                        }
                    }
                }
            }
            _ => {}
        }

        if self.info_for_model[id].load_state == StreamLoadState::Loaded {
            self.memory_used = self.memory_used.saturating_sub(
                self.info_for_model[id].get_cd_size() as usize * CDSTREAM_SECTOR_SIZE,
            );
        }

        self.info_for_model[id].load_state = StreamLoadState::NotLoaded;
        self.info_for_model[id].list = None;
    }

    pub fn get_next_file_on_cd<M: CStreamingModelInfo>(
        &mut self,
        last_posn: i32,
        priority: bool,
        model_info: &mut M,
    ) -> Option<usize> {
        let mut stream_id_first = None;
        let mut stream_id_next = None;
        let mut posn_first = u32::MAX;
        let mut posn_next = u32::MAX;

        self.snapshot_requested_list();
        let mut scan_index = 0;
        while scan_index < self.requested_scan.len() {
            let stream_id = self.requested_scan[scan_index];
            scan_index += 1;
            let is_priority = self.info_for_model[stream_id].is_priority();
            let cd_pos = self.info_for_model[stream_id].get_cd_posn_and_size();
            if priority && self.num_priority_requests != 0 && !is_priority {
                continue;
            }

            if stream_id < STREAM_OFFSET_TXD {
                if let Some(txd_id) = model_info.txd_slot_for_model(stream_id) {
                    // TxdStore may already hold loose TXDs (generic.txd) while stream
                    // state is still NotLoaded — sync and proceed (re3 has data either way).
                    if model_info.is_txd_loaded(txd_id) {
                        let txd_stream = txd_id + STREAM_OFFSET_TXD;
                        if txd_stream < self.info_for_model.len()
                            && self.info_for_model[txd_stream].load_state != StreamLoadState::Loaded
                        {
                            if self.info_for_model[txd_stream].load_state
                                == StreamLoadState::InQueue
                            {
                                self.decrement_ref(txd_stream);
                                self.remove_from_requested_list(txd_stream);
                            }
                            self.info_for_model[txd_stream].load_state = StreamLoadState::Loaded;
                        }
                    } else if self.txd_not_loaded(txd_id) {
                        let txd_stream = txd_id + STREAM_OFFSET_TXD;
                        let txd_has_cd = self
                            .info_for_model
                            .get(txd_stream)
                            .and_then(|i| i.get_cd_posn_and_size())
                            .is_some();
                        if txd_has_cd {
                            self.re_request_txd(txd_id);
                            continue;
                        }
                        // No CD + not in TxdStore: do not block DFF forever (textures skip).
                    }
                }
            }

            if let Some((posn, _size)) = cd_pos {
                if posn < posn_first {
                    stream_id_first = Some(stream_id);
                    posn_first = posn;
                }
                if posn < posn_next && posn >= last_posn as u32 {
                    stream_id_next = Some(stream_id);
                    posn_next = posn;
                }
            } else {
                // re3 GetNextFileOnCd treats an empty directory entry as a
                // successfully resident stream. Leaving it NotLoaded makes
                // renderer VIS_STREAMME re-request the same id every frame.
                self.mark_empty_file_loaded(stream_id);
            }
        }

        let next = stream_id_next.or(stream_id_first);
        if next.is_none() && self.num_priority_requests != 0 {
            self.num_priority_requests = 0;
            return self.get_next_file_on_cd(last_posn, false, model_info);
        }
        next
    }

    pub fn get_cd_image_offset(&mut self, last_posn: i32) -> i32 {
        let mut image = -1;
        let mut min_dist = i32::MAX;
        let mut offset = self.image_offsets[self.last_image_read as usize];
        if last_posn <= offset || last_posn > offset + self.image_size {
            for (index, candidate) in self.image_offsets.iter().copied().enumerate() {
                if candidate == -1 {
                    continue;
                }
                let dist = if (last_posn as u32) > candidate as u32 {
                    last_posn - (candidate + self.image_size)
                } else {
                    candidate - last_posn
                };
                if dist < min_dist {
                    image = index as i32;
                    min_dist = dist;
                }
            }
            assert!(image >= 0);
            offset = self.image_offsets[image as usize];
            self.last_image_read = image;
        }
        offset
    }

    pub fn load_cd_directory<M: CStreamingModelInfo>(
        &mut self,
        cd_stream: &CdStream,
        model_info: &mut M,
    ) -> io::Result<()> {
        self.image_offsets.fill(-1);
        self.image_offsets[0] = 0;
        self.image_size = cd_stream.gta3_img_size()? as i32;

        for image_index in (0..cd_stream.num_images()).rev() {
            let image_path = cd_stream.image_path(image_index).ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "CD image index is not loaded")
            })?;
            let mut dir_path = image_path;
            dir_path.set_extension("DIR");
            self.load_cd_directory_file(&dir_path, image_index as i32, model_info)?;
        }

        self.last_image_read = 0;
        self.image_size /= CDSTREAM_SECTOR_SIZE as i32;
        Ok(())
    }

    pub fn load_cd_directory_file<M: CStreamingModelInfo>(
        &mut self,
        dirname: &Path,
        image_index: i32,
        model_info: &mut M,
    ) -> io::Result<()> {
        // re3's PC filename casing must also resolve on case-sensitive installs.
        let filename = dirname.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "CD directory filename is missing")
        })?;
        let path = resolve_existing_case_path_from(
            dirname.parent().unwrap_or_else(|| Path::new(".")),
            &filename.to_string_lossy(),
        );
        let mut file = File::open(&path).map_err(|error| {
            io::Error::new(error.kind(), format!("CD directory {}: {error}", path.display()))
        })?;
        let mut last_id: Option<usize> = None;
        let image_selector = (image_index as u32) << 24;

        loop {
            let mut bytes = [0; DirectoryInfo::BYTE_LEN];
            match file.read_exact(&mut bytes) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error),
            }

            let entry = DirectoryInfo::from_bytes(bytes);
            let name = entry.name_string();
            let Some((stem, extension)) = split_img_entry_name(&name) else {
                last_id = None;
                continue;
            };

            if entry.size > self.streaming_buffer_size as u32 {
                self.streaming_buffer_size = entry.size as i32;
            }

            let stream_id = if extension.eq_ignore_ascii_case("DFF") {
                model_info.model_id_by_name(stem)
            } else if extension.eq_ignore_ascii_case("TXD") {
                model_info
                    .txd_id_by_name_or_add(stem)
                    .map(|txd_id| txd_id + STREAM_OFFSET_TXD)
            } else {
                last_id = None;
                continue;
            };

            let Some(stream_id) = stream_id else {
                if extension.eq_ignore_ascii_case("DFF") {
                    self.extra_objects_dir
                        .add_item_with_image_id(entry.with_name(stem), image_index);
                }
                last_id = None;
                continue;
            };

            if self.info_for_model[stream_id]
                .get_cd_posn_and_size()
                .is_some()
            {
                last_id = None;
                continue;
            }

            self.info_for_model[stream_id]
                .set_cd_posn_and_size(entry.offset | image_selector, entry.size);
            if let Some(last_id) = last_id {
                self.info_for_model[last_id].next_id = stream_id as i16;
            }
            last_id = Some(stream_id);
        }

        Ok(())
    }

    pub fn request_model_stream<M: CStreamingModelInfo>(
        &mut self,
        channel_index: usize,
        cd_stream: &mut CdStream,
        model_info: &mut M,
    ) -> io::Result<()> {
        let last_posn = cd_stream.last_posn_read() as i32;
        let image_offset = self.get_cd_image_offset(last_posn);
        let Some(mut stream_id) =
            self.get_next_file_on_cd(last_posn - image_offset, true, model_info)
        else {
            return Ok(());
        };

        while stream_id >= STREAM_OFFSET_TXD {
            let txd_id = stream_id - STREAM_OFFSET_TXD;
            if self.info_for_model[stream_id].flags & STREAMFLAGS_KEEP_IN_MEMORY != 0
                || self.is_txd_used_by_requested_models(txd_id, model_info)
            {
                break;
            }
            let Some((posn, size)) = self.info_for_model[stream_id].get_cd_posn_and_size() else {
                self.remove_model_with_adapter(stream_id, Some(model_info));
                return Ok(());
            };
            self.remove_model_with_adapter(stream_id, Some(model_info));
            let Some(next_stream_id) =
                self.get_next_file_on_cd((posn + size) as i32, true, model_info)
            else {
                return Ok(());
            };
            stream_id = next_stream_id;
        }

        let Some((posn, size)) = self.info_for_model[stream_id].get_cd_posn_and_size() else {
            return Ok(());
        };
        if size > self.streaming_buffer_size as u32 {
            if channel_index == 1 || self.channel[1].state != ChannelState::Idle {
                return Ok(());
            }
            self.loading_big_model = true;
        }

        let mut total_size = 0;
        let mut have_big_file = false;
        let mut have_ped = false;
        let mut slot = 0;
        while slot < 4 {
            if self.info_for_model[stream_id].load_state != StreamLoadState::InQueue {
                break;
            }

            let Some((_unused, size)) = self.info_for_model[stream_id].get_cd_posn_and_size()
            else {
                break;
            };
            if self.num_priority_requests != 0 && !self.info_for_model[stream_id].is_priority() {
                break;
            }

            if stream_id < STREAM_OFFSET_TXD {
                let model_type = model_info
                    .model_type(stream_id)
                    .unwrap_or(ModelInfoType::Na);
                if (have_ped && model_type == ModelInfoType::Ped)
                    || (have_big_file && model_type == ModelInfoType::Vehicle)
                    || model_info
                        .txd_slot_for_model(stream_id)
                        .is_some_and(|txd_id| self.txd_not_loaded(txd_id))
                {
                    break;
                }
            } else if have_big_file && size > 200 {
                break;
            }

            self.channel[channel_index].stream_ids[slot] = stream_id as i32;
            self.channel[channel_index].offsets[slot] = total_size;
            total_size += size as i32;
            if total_size > self.streaming_buffer_size && slot > 0 {
                total_size -= size as i32;
                break;
            }

            if stream_id < STREAM_OFFSET_TXD {
                let model_type = model_info
                    .model_type(stream_id)
                    .unwrap_or(ModelInfoType::Na);
                if model_type == ModelInfoType::Ped {
                    have_ped = true;
                }
                if model_type == ModelInfoType::Vehicle {
                    have_big_file = true;
                }
            } else if size > 200 {
                have_big_file = true;
            }

            self.info_for_model[stream_id].load_state = StreamLoadState::Reading;
            self.remove_from_requested_list(stream_id);
            self.decrement_ref(stream_id);

            if self.info_for_model[stream_id].next_id == -1 {
                slot += 1;
                break;
            }
            stream_id = self.info_for_model[stream_id].next_id as usize;
            slot += 1;
        }

        for remaining in slot..4 {
            self.channel[channel_index].stream_ids[remaining] = -1;
        }

        assert!(!(self.loading_big_model && channel_index == 1));
        if total_size > 0 {
            let status = cd_stream.read_async(
                channel_index,
                image_offset as u32 + posn,
                total_size as u32,
            )?;
            if status != StreamStatus::None {
                self.channel[channel_index].state = ChannelState::Reading;
                self.channel[channel_index].field24 = 0;
                self.channel[channel_index].size = total_size;
                self.channel[channel_index].position = image_offset + posn as i32;
                self.channel[channel_index].num_tries = 0;
            }
        }
        Ok(())
    }

    pub fn process_loading_channel<R: CStreamingRuntime>(
        &mut self,
        channel_index: usize,
        cd_stream: &CdStream,
        runtime: &mut R,
    ) -> bool {
        if self.channel[channel_index].state != ChannelState::Started {
            let status = cd_stream.get_status(channel_index);
            if status != StreamStatus::None {
                if status != StreamStatus::Reading && status != StreamStatus::Waiting {
                    self.channel_error = channel_index as i32;
                    self.channel[channel_index].state = ChannelState::Error;
                    self.channel[channel_index].status = status;
                }
                return false;
            }
        }

        if self.channel[channel_index].state == ChannelState::Started {
            let data = self.channel[channel_index]
                .completed_data
                .take()
                .unwrap_or_default();
            self.channel[channel_index].state = ChannelState::Idle;
            let id = self.channel[channel_index].stream_ids[0];
            if id != -1 {
                if runtime.finish_loading_large_file(id as usize, &data) {
                    self.mark_stream_loaded(id as usize, runtime);
                } else {
                    self.remove_model_with_adapter(id as usize, Some(runtime));
                    self.re_request_model(id as usize);
                }
                self.channel[channel_index].stream_ids[0] = -1;
                self.channel[channel_index].offsets[0] = -1;
            }
        } else {
            let data = cd_stream.take_completed(channel_index).unwrap_or_default();
            self.channel[channel_index].state = ChannelState::Idle;
            for slot in 0..4 {
                let id = self.channel[channel_index].stream_ids[slot];
                if id == -1 {
                    continue;
                }
                let id = id as usize;
                let cd_size = self.info_for_model[id].get_cd_size();
                if id < STREAM_OFFSET_TXD
                    && runtime.model_type(id) == Some(ModelInfoType::Vehicle)
                    && self.num_vehicles_loaded >= DESIRED_NUM_VEHICLES_LOADED
                    && !self.remove_loaded_vehicle(runtime)
                    && (self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE == 0
                        || self.get_available_vehicle_slot() == -1)
                {
                    self.remove_model_with_adapter(id, Some(runtime));
                    if self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE != 0 {
                        self.re_request_model(id);
                    } else if runtime
                        .txd_slot_for_model(id)
                        .is_some_and(|txd_id| runtime.txd_num_refs(txd_id) == 0)
                    {
                        if let Some(txd_id) = runtime.txd_slot_for_model(id) {
                            self.remove_model_with_adapter(
                                txd_id + STREAM_OFFSET_TXD,
                                Some(runtime),
                            );
                        }
                    }
                } else {
                    let offset =
                        self.channel[channel_index].offsets[slot] as usize * CDSTREAM_SECTOR_SIZE;
                    let byte_len = cd_size as usize * CDSTREAM_SECTOR_SIZE;
                    let end = offset.saturating_add(byte_len).min(data.len());
                    let context = CStreamingConvertContext {
                        loading_big_model: self.loading_big_model,
                        cd_size_sectors: cd_size,
                        stream_flags: self.info_for_model[id].flags,
                    };
                    match runtime.convert_buffer_to_object(id, &data[offset..end], context) {
                        ConvertResult::Started => {
                            // The split reader needs the same object bytes during phase two.
                            // Keep them on the channel; CdStream completion is consumable.
                            self.channel[channel_index].completed_data =
                                Some(data[offset..end].to_vec());
                            self.info_for_model[id].load_state = StreamLoadState::Started;
                            self.channel[channel_index].state = ChannelState::Started;

                            // A channel can own only one split conversion. Return every
                            // unprocessed batch member to the request list before reusing slot 0.
                            let remaining: Vec<usize> = ((slot + 1)..4)
                                .filter_map(|remaining_slot| {
                                    let remaining_id =
                                        self.channel[channel_index].stream_ids[remaining_slot];
                                    (remaining_id != -1).then_some(remaining_id as usize)
                                })
                                .collect();
                            for remaining_id in remaining {
                                self.remove_model_with_adapter(remaining_id, Some(runtime));
                                self.re_request_model(remaining_id);
                            }
                            self.channel[channel_index].stream_ids = [id as i32, -1, -1, -1];
                            self.channel[channel_index].offsets = [0, -1, -1, -1];
                            break;
                        }
                        ConvertResult::Loaded => {
                            self.mark_stream_loaded(id, runtime);
                            self.channel[channel_index].stream_ids[slot] = -1;
                        }
                        ConvertResult::Failed => {
                            self.remove_model_with_adapter(id, Some(runtime));
                            self.re_request_model(id);
                            self.channel[channel_index].stream_ids[slot] = -1;
                        }
                    }
                }
            }
        }

        if self.loading_big_model && self.channel[channel_index].state != ChannelState::Started {
            self.loading_big_model = false;
            self.channel[1].stream_ids = [-1; 4];
            self.channel[1].state = ChannelState::Idle;
        }

        true
    }

    pub fn retry_load_file<R: CStreamingRuntime>(
        &mut self,
        channel_index: usize,
        cd_stream: &mut CdStream,
        runtime: &mut R,
    ) -> io::Result<()> {
        match self.channel[channel_index].state {
            ChannelState::Error => {
                self.channel[channel_index].num_tries += 1;
                let status = cd_stream.get_status(channel_index);
                if status == StreamStatus::Reading || status == StreamStatus::Waiting {
                    return Ok(());
                }
                cd_stream.read_async(
                    channel_index,
                    self.channel[channel_index].position as u32,
                    self.channel[channel_index].size as u32,
                )?;
                self.channel[channel_index].state = ChannelState::Reading;
                self.channel[channel_index].field24 = -600;
            }
            ChannelState::Idle => {
                cd_stream.read_async(
                    channel_index,
                    self.channel[channel_index].position as u32,
                    self.channel[channel_index].size as u32,
                )?;
                self.channel[channel_index].state = ChannelState::Reading;
                self.channel[channel_index].field24 = -600;
            }
            ChannelState::Reading => {
                if self.process_loading_channel(channel_index, cd_stream, runtime) {
                    self.channel_error = -1;
                }
            }
            ChannelState::Started => {}
        }
        Ok(())
    }

    pub fn load_requested_models<R: CStreamingRuntime>(
        &mut self,
        cd_stream: &mut CdStream,
        runtime: &mut R,
        current_channel: &mut usize,
    ) -> io::Result<()> {
        // re3 advances exactly one CD channel per CGame::Process call.  Doing
        // several passes here batches DFF/TXD decode and model materialisation
        // onto one Update, producing large traversal hitches whenever the
        // player crosses a streaming boundary.
        if self.loading_big_model {
            *current_channel = 0;
        }

        if matches!(
            self.channel[*current_channel].state,
            ChannelState::Reading | ChannelState::Started
        ) {
            self.process_loading_channel(*current_channel, cd_stream, runtime);
        }

        if self.channel_error == -1 {
            if self.channel[*current_channel].state == ChannelState::Idle {
                self.request_model_stream(*current_channel, cd_stream, runtime)?;
            }
            if self.channel[*current_channel].state != ChannelState::Started {
                *current_channel = 1 - *current_channel;
            }
        }
        Ok(())
    }

    pub fn flush_request_list(&mut self) {
        self.snapshot_requested_list();
        let mut scan_index = 0;
        while scan_index < self.requested_scan.len() {
            let id = self.requested_scan[scan_index];
            scan_index += 1;
            self.remove_model(id);
        }
        self.flush_channels_shape();
    }

    pub fn flush_channels_shape(&mut self) {
        for channel in &mut self.channel {
            if channel.state == ChannelState::Started || channel.state == ChannelState::Reading {
                channel.state = ChannelState::Idle;
            }
        }
    }

    pub fn prepare_immediate_load(&mut self, id: usize) {
        if id >= self.info_for_model.len() {
            return;
        }
        if self.info_for_model[id].load_state == StreamLoadState::InQueue {
            self.remove_from_requested_list(id);
            self.decrement_ref(id);
        }
        self.info_for_model[id].load_state = StreamLoadState::Reading;
    }

    pub fn add_loaded_model_front(&mut self, id: usize) {
        if id >= self.info_for_model.len() {
            return;
        }
        self.remove_from_requested_list(id);
        self.remove_from_loaded_list(id);
        self.add_to_list_front(id, StreamList::Loaded);
    }

    pub fn is_txd_used_by_requested_models<M: CStreamingModelInfo>(
        &self,
        txd_id: usize,
        model_info: &M,
    ) -> bool {
        for stream_id in self.requested_list() {
            if stream_id < STREAM_OFFSET_TXD
                && model_info.txd_slot_for_model(stream_id) == Some(txd_id)
            {
                return true;
            }
        }

        for channel in &self.channel {
            for stream_id in channel.stream_ids {
                if stream_id != -1
                    && (stream_id as usize) < STREAM_OFFSET_TXD
                    && model_info.txd_slot_for_model(stream_id as usize) == Some(txd_id)
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_txd_used_by_loaded_or_requested_models<M: CStreamingModelInfo>(
        &self,
        txd_id: usize,
        model_info: &M,
    ) -> bool {
        if self.is_txd_used_by_requested_models(txd_id, model_info) {
            return true;
        }
        if model_info.txd_num_refs(txd_id) > 0 {
            return true;
        }
        for stream_id in self.loaded_list() {
            if stream_id < STREAM_OFFSET_TXD
                && model_info.txd_slot_for_model(stream_id) == Some(txd_id)
            {
                return true;
            }
        }
        for &vehicle_model in &self.vehicles_loaded {
            if vehicle_model >= 0
                && model_info.txd_slot_for_model(vehicle_model as usize) == Some(txd_id)
            {
                return true;
            }
        }
        for (stream_id, info) in self
            .info_for_model
            .iter()
            .enumerate()
            .take(STREAM_OFFSET_TXD)
        {
            if info.load_state == StreamLoadState::Loaded
                && model_info.txd_slot_for_model(stream_id) == Some(txd_id)
            {
                return true;
            }
        }
        false
    }

    pub fn get_available_vehicle_slot(&self) -> i32 {
        self.vehicles_loaded
            .iter()
            .position(|model| *model == -1)
            .map(|slot| slot as i32)
            .unwrap_or(-1)
    }

    pub fn remove_loaded_vehicle<M: CStreamingModelInfo>(&mut self, model_info: &mut M) -> bool {
        for _ in 0..MAXVEHICLESLOADED {
            self.last_vehicle_deleted += 1;
            if self.last_vehicle_deleted == MAXVEHICLESLOADED as i32 {
                self.last_vehicle_deleted = 0;
            }
            let id = self.vehicles_loaded[self.last_vehicle_deleted as usize];
            if id != -1
                && self.info_for_model[id as usize].flags & STREAMFLAGS_CANT_REMOVE == 0
                && model_info.model_num_refs(id as usize) == 0
                && self.info_for_model[id as usize].load_state == StreamLoadState::Loaded
            {
                self.remove_model_with_adapter(id as usize, Some(model_info));
                self.num_vehicles_loaded -= 1;
                self.vehicles_loaded[self.last_vehicle_deleted as usize] = -1;
                return true;
            }
        }
        false
    }

    pub fn add_to_loaded_vehicles_list<M: CStreamingModelInfo>(
        &mut self,
        model_id: usize,
        model_info: &mut M,
    ) -> bool {
        if self.num_vehicles_loaded < DESIRED_NUM_VEHICLES_LOADED {
            for _ in 0..MAXVEHICLESLOADED {
                if self.vehicles_loaded[self.last_vehicle_deleted as usize] == -1 {
                    break;
                }
                self.last_vehicle_deleted += 1;
                if self.last_vehicle_deleted == MAXVEHICLESLOADED as i32 {
                    self.last_vehicle_deleted = 0;
                }
            }
            assert_eq!(self.vehicles_loaded[self.last_vehicle_deleted as usize], -1);
            self.num_vehicles_loaded += 1;
        } else {
            let mut found = -1;
            for _ in 0..MAXVEHICLESLOADED {
                let id = self.vehicles_loaded[self.last_vehicle_deleted as usize];
                if id != -1
                    && self.info_for_model[id as usize].flags & STREAMFLAGS_CANT_REMOVE == 0
                    && model_info.model_num_refs(id as usize) == 0
                {
                    found = id;
                    break;
                }
                self.last_vehicle_deleted += 1;
                if self.last_vehicle_deleted == MAXVEHICLESLOADED as i32 {
                    self.last_vehicle_deleted = 0;
                }
            }

            if found == -1 {
                let slot = self.get_available_vehicle_slot();
                if slot == -1 {
                    return false;
                }
                self.last_vehicle_deleted = slot;
                self.num_vehicles_loaded += 1;
            } else {
                self.remove_model_with_adapter(found as usize, Some(model_info));
            }
        }

        self.vehicles_loaded[self.last_vehicle_deleted as usize] = model_id as i32;
        self.last_vehicle_deleted += 1;
        if self.last_vehicle_deleted == MAXVEHICLESLOADED as i32 {
            self.last_vehicle_deleted = 0;
        }
        true
    }

    fn reinsert_loaded_model(&mut self, id: usize) {
        if self.loaded_head == Some(id) {
            return;
        }
        if self.info_for_model[id].list.is_some() {
            self.remove_from_loaded_list(id);
            if self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE == 0 {
                self.add_to_list_front(id, StreamList::Loaded);
            }
        }
    }

    fn add_to_requested_list_front(&mut self, id: usize) {
        self.remove_from_requested_list(id);
        self.add_to_list_front(id, StreamList::Requested);
    }

    pub(crate) fn remove_from_requested_list(&mut self, id: usize) {
        self.remove_from_list(id, StreamList::Requested);
    }

    fn remove_from_loaded_list(&mut self, id: usize) {
        self.remove_from_list(id, StreamList::Loaded);
    }

    fn add_to_list_front(&mut self, id: usize, list: StreamList) {
        let old_head = match list {
            StreamList::Requested => self.requested_head,
            StreamList::Loaded => self.loaded_head,
        };
        {
            let info = &mut self.info_for_model[id];
            debug_assert!(info.list.is_none());
            info.list = Some(list);
            info.list_prev = None;
            info.list_next = old_head;
        }
        if let Some(head) = old_head {
            self.info_for_model[head].list_prev = Some(id);
        } else {
            match list {
                StreamList::Requested => self.requested_tail = Some(id),
                StreamList::Loaded => self.loaded_tail = Some(id),
            }
        }
        match list {
            StreamList::Requested => self.requested_head = Some(id),
            StreamList::Loaded => self.loaded_head = Some(id),
        }
    }

    fn remove_from_list(&mut self, id: usize, list: StreamList) {
        if self.info_for_model.get(id).and_then(|info| info.list) != Some(list) {
            return;
        }
        let prev = self.info_for_model[id].list_prev;
        let next = self.info_for_model[id].list_next;
        if let Some(prev) = prev {
            self.info_for_model[prev].list_next = next;
        } else {
            match list {
                StreamList::Requested => self.requested_head = next,
                StreamList::Loaded => self.loaded_head = next,
            }
        }
        if let Some(next) = next {
            self.info_for_model[next].list_prev = prev;
        } else {
            match list {
                StreamList::Requested => self.requested_tail = prev,
                StreamList::Loaded => self.loaded_tail = prev,
            }
        }
        let info = &mut self.info_for_model[id];
        info.list = None;
        info.list_prev = None;
        info.list_next = None;
    }

    fn snapshot_requested_list(&mut self) {
        self.requested_scan.clear();
        let mut next = self.requested_head;
        while let Some(id) = next {
            self.requested_scan.push(id);
            next = self.info_for_model[id].list_next;
        }
    }

    pub(crate) fn decrement_ref(&mut self, id: usize) {
        self.num_models_requested -= 1;
        if self.info_for_model[id].is_priority() {
            self.info_for_model[id].flags &= !STREAMFLAGS_PRIORITY;
            self.num_priority_requests -= 1;
        }
    }

    pub(crate) fn mark_empty_file_loaded(&mut self, id: usize) {
        if self.info_for_model[id].load_state == StreamLoadState::InQueue {
            self.decrement_ref(id);
            self.remove_from_requested_list(id);
        }
        self.info_for_model[id].load_state = StreamLoadState::Loaded;
        self.info_for_model[id].list = None;
    }

    pub(crate) fn mark_streaming_txd_loaded(&mut self, slot: usize) {
        let stream_id = slot + STREAM_OFFSET_TXD;
        if stream_id >= self.info_for_model.len() {
            return;
        }
        if self.info_for_model[stream_id].load_state == StreamLoadState::InQueue
            || self.info_for_model[stream_id].list == Some(StreamList::Requested)
        {
            self.decrement_ref(stream_id);
            self.remove_from_requested_list(stream_id);
        }
        self.info_for_model[stream_id].load_state = StreamLoadState::Loaded;
    }

    fn txd_not_loaded(&self, txd_id: usize) -> bool {
        let stream_id = txd_id + STREAM_OFFSET_TXD;
        // Started = two-phase load mid-flight; still not usable for materials.
        !matches!(
            self.info_for_model[stream_id].load_state,
            StreamLoadState::Loaded | StreamLoadState::Reading
        )
    }

    fn mark_stream_loaded<M: CStreamingModelInfo>(&mut self, id: usize, model_info: &mut M) {
        if self.info_for_model[id].load_state == StreamLoadState::InQueue
            || self.info_for_model[id].list == Some(StreamList::Requested)
        {
            self.decrement_ref(id);
            self.remove_from_requested_list(id);
        }

        if id < STREAM_OFFSET_TXD {
            if model_info.model_type(id) == Some(ModelInfoType::Vehicle) {
                bevy::log::debug!(target: "re3bevy::streaming", id, "vehicle model loaded");
                self.add_to_loaded_vehicles_list(id, model_info);
            } else if model_info.model_type(id) != Some(ModelInfoType::Ped)
                && self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE == 0
            {
                self.add_loaded_model_front(id);
            }
        } else if self.info_for_model[id].flags & STREAMFLAGS_CANT_REMOVE == 0 {
            self.add_loaded_model_front(id);
        }

        if self.info_for_model[id].load_state != StreamLoadState::Loaded {
            self.memory_used +=
                self.info_for_model[id].get_cd_size() as usize * CDSTREAM_SECTOR_SIZE;
        }
        self.info_for_model[id].load_state = StreamLoadState::Loaded;
    }

    pub fn memory_card_save(&self) -> Vec<u8> {
        let mut out = vec![0xFF; NUM_DEFAULT_MODELS];
        for (index, byte) in out.iter_mut().enumerate() {
            if self
                .info_for_model
                .get(index)
                .is_some_and(|info| info.load_state == StreamLoadState::Loaded)
            {
                *byte = self.info_for_model[index].flags;
            }
        }
        out
    }

    pub fn memory_card_load(&mut self, bytes: &[u8]) -> Result<(), usize> {
        if bytes.len() != NUM_DEFAULT_MODELS {
            return Err(bytes.len());
        }
        for (index, byte) in bytes.iter().copied().enumerate() {
            if byte != 0xFF
                && self
                    .info_for_model
                    .get(index)
                    .is_some_and(|info| info.load_state == StreamLoadState::Loaded)
            {
                self.info_for_model[index].flags = byte;
            }
        }
        Ok(())
    }
}

pub struct NullStreamingModelInfo;

impl CStreamingModelInfo for NullStreamingModelInfo {
    fn model_name(&self, _model_id: usize) -> Option<String> {
        None
    }

    fn model_id_by_name(&mut self, _name: &str) -> Option<usize> {
        None
    }

    fn set_model_name(&mut self, _model_id: usize, _name: &str) {}

    fn txd_id_by_name_or_add(&mut self, _name: &str) -> Option<usize> {
        None
    }

    fn txd_id_by_name(&self, _name: &str) -> Option<usize> {
        None
    }

    fn txd_slot_for_model(&self, _model_id: usize) -> Option<usize> {
        None
    }

    fn clear_txd_slot_for_model(&mut self, _model_id: usize) {}

    fn set_txd_slot_for_model(&mut self, _model_id: usize, _name: &str) {}

    fn model_type(&self, _model_id: usize) -> Option<ModelInfoType> {
        None
    }

    fn model_num_refs(&self, _model_id: usize) -> u16 {
        0
    }

    fn remove_model_ref(&mut self, _model_id: usize) -> Option<usize> {
        None
    }

    fn txd_num_refs(&self, _txd_id: usize) -> u16 {
        0
    }

    fn is_txd_loaded(&self, _txd_id: usize) -> bool {
        false
    }

    fn add_txd_ref(&mut self, _txd_id: usize) {}

    fn remove_txd_ref_without_delete(&mut self, _txd_id: usize) {}

    fn delete_model_rw_object(&mut self, _model_id: usize) {}

    fn remove_txd(&mut self, _txd_id: usize) {}
}

#[cfg(test)]
mod empty_file_tests {
    use super::*;

    #[test]
    fn empty_cd_request_becomes_loaded_and_drains_request_counters() {
        let mut streaming = CStreaming::default();
        let mut model_info = NullStreamingModelInfo;
        let stream_id = STREAM_OFFSET_TXD;

        streaming.request_model(stream_id, STREAMFLAGS_PRIORITY);
        assert_eq!(streaming.num_models_requested, 1);
        assert_eq!(streaming.num_priority_requests, 1);

        assert_eq!(
            streaming.get_next_file_on_cd(0, true, &mut model_info),
            None
        );
        assert_eq!(
            streaming.info_for_model[stream_id].load_state,
            StreamLoadState::Loaded
        );
        assert_eq!(streaming.num_models_requested, 0);
        assert_eq!(streaming.num_priority_requests, 0);
        assert!(streaming.requested_list().next().is_none());
    }

    #[test]
    fn empty_cd_loaded_model_is_not_requeued() {
        let mut streaming = CStreaming::default();
        let mut model_info = NullStreamingModelInfo;
        let stream_id = STREAM_OFFSET_TXD;

        streaming.request_model(stream_id, 0);
        assert_eq!(
            streaming.get_next_file_on_cd(0, false, &mut model_info),
            None
        );

        assert_eq!(
            streaming.request_model(stream_id, STREAMFLAGS_PRIORITY),
            RequestOutcome::AlreadyLoaded
        );
        assert_eq!(streaming.num_models_requested, 0);
        assert_eq!(streaming.num_priority_requests, 0);
        assert!(streaming.requested_list().next().is_none());
    }

    #[test]
    fn mark_stream_loaded_drains_requested_list_for_vehicles_and_peds() {
        let mut streaming = CStreaming::default();
        let mut model_info = NullStreamingModelInfo;
        let vehicle_id = 90;
        let ped_id = 1;

        streaming.request_model(vehicle_id, STREAMFLAGS_DEPENDENCY);
        streaming.request_model(ped_id, STREAMFLAGS_DONT_REMOVE);
        assert_eq!(streaming.num_models_requested, 2);

        streaming.mark_stream_loaded(vehicle_id, &mut model_info);
        assert_eq!(streaming.num_models_requested, 1);
        assert_eq!(
            streaming.info_for_model[vehicle_id].load_state,
            StreamLoadState::Loaded
        );

        streaming.mark_stream_loaded(ped_id, &mut model_info);
        assert_eq!(streaming.num_models_requested, 0);
        assert_eq!(
            streaming.info_for_model[ped_id].load_state,
            StreamLoadState::Loaded
        );

        assert!(streaming.requested_list().next().is_none());
    }

    #[test]
    fn remove_model_ref_preserves_txd_in_loaded_state() {
        struct MockModelInfo {
            txd_id: usize,
            model_refs: u16,
            txd_refs: u16,
        }
        impl CStreamingModelInfo for MockModelInfo {
            fn model_name(&self, _: usize) -> Option<String> {
                None
            }
            fn model_id_by_name(&mut self, _: &str) -> Option<usize> {
                None
            }
            fn set_model_name(&mut self, _: usize, _: &str) {}
            fn txd_id_by_name_or_add(&mut self, _: &str) -> Option<usize> {
                None
            }
            fn txd_id_by_name(&self, _: &str) -> Option<usize> {
                None
            }
            fn txd_slot_for_model(&self, _: usize) -> Option<usize> {
                Some(self.txd_id)
            }
            fn clear_txd_slot_for_model(&mut self, _: usize) {}
            fn set_txd_slot_for_model(&mut self, _: usize, _: &str) {}
            fn model_type(&self, _: usize) -> Option<ModelInfoType> {
                Some(ModelInfoType::Simple)
            }
            fn model_num_refs(&self, _: usize) -> u16 {
                self.model_refs
            }
            fn remove_model_ref(&mut self, _: usize) -> Option<usize> {
                self.model_refs = self.model_refs.saturating_sub(1);
                self.txd_refs = self.txd_refs.saturating_sub(1);
                (self.txd_refs == 0).then_some(self.txd_id)
            }
            fn txd_num_refs(&self, _: usize) -> u16 {
                self.txd_refs
            }
            fn is_txd_loaded(&self, _: usize) -> bool {
                true
            }
            fn add_txd_ref(&mut self, _: usize) {
                self.txd_refs += 1;
            }
            fn remove_txd_ref_without_delete(&mut self, _: usize) {
                self.txd_refs = self.txd_refs.saturating_sub(1);
            }
            fn delete_model_rw_object(&mut self, _: usize) {}
            fn remove_txd(&mut self, _: usize) {}
        }

        let mut streaming = CStreaming::default();
        let mut mock = MockModelInfo {
            txd_id: 10,
            model_refs: 1,
            txd_refs: 1,
        };
        let txd_stream = 10 + STREAM_OFFSET_TXD;
        streaming.info_for_model[txd_stream].load_state = StreamLoadState::Loaded;

        streaming.remove_model_ref_with_adapter(100, &mut mock);

        assert_eq!(
            streaming.info_for_model[txd_stream].load_state,
            StreamLoadState::Loaded,
            "re3 keeps TXD loaded in memory when entity instance is removed"
        );
    }
}

fn split_img_entry_name(name: &str) -> Option<(&str, &str)> {
    let dot = name.rfind('.')?;
    Some((&name[..dot], &name[dot + 1..]))
}
