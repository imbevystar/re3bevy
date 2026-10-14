//! re3 reference: references/re-GTA/src/core/CdStream.cpp
//! re3 reference: references/re-GTA/src/core/CdStream.h
//!
//! Sector-addressed IMG streaming with the same channel/request-queue shape as
//! re3. C++ stores a raw destination pointer per channel; Rust keeps completed
//! reads as owned buffers and lets callers copy/take the data after completion.

use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

pub const CDSTREAM_SECTOR_SIZE: usize = 2048;
pub const MAX_CDIMAGES: usize = 8;

pub fn get_image_index(offset: u32) -> usize {
    (offset >> 24) as usize
}

pub fn get_sector_offset(offset: u32) -> u32 {
    offset & 0x00FF_FFFF
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamStatus {
    None = 0,
    Success = 1,
    Reading = 0xFF,
    Error = 0xFE,
    ErrorNoCd = 0xFD,
    ErrorWrongCd = 0xFC,
    ErrorOpenCd = 0xFB,
    Waiting = 0xFA,
}

#[derive(Debug, Clone)]
pub struct CdImage {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct CdReadInfo {
    sector_offset: u32,
    sectors_to_read: u32,
    image_index: usize,
    locked: bool,
    reading: bool,
    status: StreamStatus,
    completed_data: Option<Vec<u8>>,
}

impl Default for CdReadInfo {
    fn default() -> Self {
        Self {
            sector_offset: 0,
            sectors_to_read: 0,
            image_index: 0,
            locked: false,
            reading: false,
            status: StreamStatus::None,
            completed_data: None,
        }
    }
}

#[derive(Debug)]
struct RequestQueue {
    items: Vec<usize>,
    head: usize,
    tail: usize,
}

impl RequestQueue {
    fn new(num_channels: usize) -> Self {
        Self {
            items: vec![0; num_channels + 1],
            head: 0,
            tail: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    fn add(&mut self, item: usize) {
        self.items[self.tail] = item;
        self.tail = (self.tail + 1) % self.items.len();
        if self.head == self.tail {
            eprintln!("CdStream request queue is full");
        }
    }

    fn first(&self) -> Option<usize> {
        (!self.is_empty()).then_some(self.items[self.head])
    }

    fn remove_first(&mut self) {
        if self.is_empty() {
            eprintln!("CdStream request queue is empty");
            return;
        }
        self.head = (self.head + 1) % self.items.len();
    }
}

#[derive(Debug)]
struct CdStreamInner {
    channels: Vec<CdReadInfo>,
    images: Vec<CdImage>,
    queue: RequestQueue,
    last_posn_read: u32,
    shutdown: bool,
}

pub struct CdStream {
    num_channels: usize,
    shared: Arc<(Mutex<CdStreamInner>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl fmt::Debug for CdStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lock, _) = &*self.shared;
        let inner = lock.lock().map_err(|_| fmt::Error)?;
        f.debug_struct("CdStream")
            .field("num_channels", &self.num_channels)
            .field("num_images", &inner.images.len())
            .field("last_posn_read", &inner.last_posn_read)
            .field("worker_running", &self.worker.is_some())
            .finish()
    }
}

impl CdStream {
    pub fn init(num_channels: usize) -> Self {
        let inner = CdStreamInner {
            channels: vec![CdReadInfo::default(); num_channels],
            images: Vec::new(),
            queue: RequestQueue::new(num_channels),
            last_posn_read: 0,
            shutdown: false,
        };
        let shared = Arc::new((Mutex::new(inner), Condvar::new()));
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("CdStream".to_owned())
            .spawn(move || cd_stream_worker(worker_shared))
            .expect("failed to spawn CdStream worker");

        Self {
            num_channels,
            shared,
            worker: Some(worker),
        }
    }

    pub fn shutdown(&mut self) {
        self.remove_images();
        {
            let (lock, condvar) = &*self.shared;
            let mut inner = lock.lock().expect("CdStream mutex poisoned");
            inner.shutdown = true;
            condvar.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    pub fn add_image(&mut self, path: impl AsRef<Path>) -> io::Result<bool> {
        let path = path.as_ref();
        File::open(path)?;

        let (lock, _) = &*self.shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        assert!(inner.images.len() < MAX_CDIMAGES);
        let name = path.to_string_lossy().into_owned();
        inner.images.push(CdImage {
            name,
            path: path.to_path_buf(),
        });
        Ok(true)
    }

    pub fn remove_images(&mut self) {
        for channel in 0..self.num_channels {
            let _ = self.sync(channel);
        }

        let (lock, _) = &*self.shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        inner.images.clear();
    }

    pub fn num_images(&self) -> usize {
        let (lock, _) = &*self.shared;
        lock.lock().expect("CdStream mutex poisoned").images.len()
    }

    pub fn image_name(&self, index: usize) -> Option<String> {
        let (lock, _) = &*self.shared;
        lock.lock()
            .expect("CdStream mutex poisoned")
            .images
            .get(index)
            .map(|image| image.name.clone())
    }

    pub fn image_path(&self, index: usize) -> Option<PathBuf> {
        let (lock, _) = &*self.shared;
        lock.lock()
            .expect("CdStream mutex poisoned")
            .images
            .get(index)
            .map(|image| image.path.clone())
    }

    pub fn gta3_img_size(&self) -> io::Result<u32> {
        let path = self
            .image_path(0)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no CD image loaded"))?;
        Ok(std::fs::metadata(path)?.len() as u32)
    }

    pub fn read(
        &mut self,
        channel: usize,
        buffer: &mut [u8],
        offset: u32,
        size_in_sectors: u32,
    ) -> io::Result<StreamStatus> {
        let byte_len = size_in_sectors as usize * CDSTREAM_SECTOR_SIZE;
        assert!(buffer.len() >= byte_len);

        let read_status = self.read_async(channel, offset, size_in_sectors)?;
        if read_status == StreamStatus::None {
            return Ok(StreamStatus::None);
        }

        let sync_status = self.sync(channel);
        if sync_status != StreamStatus::None {
            return Ok(sync_status);
        }

        let Some(data) = self.take_completed(channel) else {
            return Ok(StreamStatus::Error);
        };
        buffer[..byte_len].copy_from_slice(&data[..byte_len]);
        Ok(StreamStatus::Success)
    }

    pub fn read_async(
        &mut self,
        channel: usize,
        offset: u32,
        size_in_sectors: u32,
    ) -> io::Result<StreamStatus> {
        assert!(channel < self.num_channels);

        let image_index = get_image_index(offset);
        let sector_offset = get_sector_offset(offset);

        let (lock, condvar) = &*self.shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        if image_index >= inner.images.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "CD image index is not loaded",
            ));
        }

        inner.last_posn_read = offset.wrapping_add(size_in_sectors);
        let read_info = &mut inner.channels[channel];
        if read_info.sectors_to_read != 0 || read_info.reading {
            return Ok(StreamStatus::None);
        }

        read_info.status = StreamStatus::None;
        read_info.sector_offset = sector_offset;
        read_info.sectors_to_read = size_in_sectors;
        read_info.image_index = image_index;
        read_info.locked = false;
        read_info.completed_data = None;
        inner.queue.add(channel);
        condvar.notify_one();
        Ok(StreamStatus::Success)
    }

    pub fn get_status(&self, channel: usize) -> StreamStatus {
        assert!(channel < self.num_channels);
        let (lock, _) = &*self.shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        let read_info = &mut inner.channels[channel];

        if read_info.reading {
            return StreamStatus::Reading;
        }
        if read_info.sectors_to_read != 0 {
            return StreamStatus::Waiting;
        }
        if read_info.status != StreamStatus::None {
            let status = read_info.status;
            read_info.status = StreamStatus::None;
            return status;
        }
        StreamStatus::None
    }

    pub fn sync(&self, channel: usize) -> StreamStatus {
        assert!(channel < self.num_channels);
        let (lock, condvar) = &*self.shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        if inner.channels[channel].sectors_to_read != 0 {
            inner.channels[channel].locked = true;
            while inner.channels[channel].locked && inner.channels[channel].sectors_to_read != 0 {
                inner = condvar.wait(inner).expect("CdStream mutex poisoned");
            }
            inner.channels[channel].locked = false;
        }

        inner.channels[channel].reading = false;
        inner.channels[channel].status
    }

    pub fn take_completed(&self, channel: usize) -> Option<Vec<u8>> {
        assert!(channel < self.num_channels);
        let (lock, _) = &*self.shared;
        lock.lock().expect("CdStream mutex poisoned").channels[channel]
            .completed_data
            .take()
    }

    pub fn last_posn_read(&self) -> u32 {
        let (lock, _) = &*self.shared;
        lock.lock().expect("CdStream mutex poisoned").last_posn_read
    }
}

impl Drop for CdStream {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn cd_stream_worker(shared: Arc<(Mutex<CdStreamInner>, Condvar)>) {
    let mut file_cache = std::collections::HashMap::<PathBuf, File>::new();
    loop {
        let (channel, path, sector_offset, sectors_to_read) = {
            let (lock, condvar) = &*shared;
            let mut inner = lock.lock().expect("CdStream mutex poisoned");
            while inner.queue.is_empty() && !inner.shutdown {
                inner = condvar.wait(inner).expect("CdStream mutex poisoned");
            }
            if inner.shutdown {
                return;
            }

            let channel = inner.queue.first().expect("CdStream queue disappeared");
            inner.channels[channel].reading = true;
            let read_info = &inner.channels[channel];
            let path = inner.images[read_info.image_index].path.clone();
            (
                channel,
                path,
                read_info.sector_offset,
                read_info.sectors_to_read,
            )
        };

        let byte_len = sectors_to_read as usize * CDSTREAM_SECTOR_SIZE;
        let mut data = vec![0; byte_len];
        let file = match file_cache.entry(path.clone()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                File::open(&path).map(|f| entry.insert(f))
            }
        };
        let status = match file.and_then(|file| {
            file.seek(SeekFrom::Start(
                u64::from(sector_offset) * CDSTREAM_SECTOR_SIZE as u64,
            ))?;
            file.read_exact(&mut data)
        }) {
            Ok(()) => StreamStatus::None,
            Err(_) => {
                file_cache.remove(&path);
                StreamStatus::Error
            }
        };

        let (lock, condvar) = &*shared;
        let mut inner = lock.lock().expect("CdStream mutex poisoned");
        if status == StreamStatus::None {
            inner.channels[channel].completed_data = Some(data);
        } else {
            inner.channels[channel].completed_data = None;
        }
        inner.channels[channel].status = status;
        inner.queue.remove_first();
        inner.channels[channel].sectors_to_read = 0;
        if inner.channels[channel].locked {
            inner.channels[channel].locked = false;
        }
        inner.channels[channel].reading = false;
        condvar.notify_all();
    }
}
