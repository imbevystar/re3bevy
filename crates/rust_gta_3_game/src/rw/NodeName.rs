//! re3 reference: references/re-GTA/src/rw/NodeName.cpp
//! re3 reference: references/re-GTA/src/rw/NodeName.h
//!
//! RenderWare frame node-name plugin payload.

pub const ID_NODENAME: u32 = 0x0253_00FE;
pub const NODE_NAME_PLUGIN_SIZE: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwFrameNodeName {
    bytes: [u8; NODE_NAME_PLUGIN_SIZE],
}

impl RwFrameNodeName {
    pub fn new(name: &str) -> Self {
        let mut bytes = [0; NODE_NAME_PLUGIN_SIZE];
        let raw = name.as_bytes();
        let len = raw.len().min(NODE_NAME_PLUGIN_SIZE - 1);
        bytes[..len].copy_from_slice(&raw[..len]);
        Self { bytes }
    }

    pub fn constructor() -> Self {
        Self {
            bytes: [0; NODE_NAME_PLUGIN_SIZE],
        }
    }

    pub fn copy_from(&mut self, src: &Self) {
        self.bytes[..23].copy_from_slice(&src.bytes[..23]);
        self.bytes[23] = 0;
    }

    pub fn stream_read(&mut self, data: &[u8]) {
        let len = data.len().min(NODE_NAME_PLUGIN_SIZE - 1);
        self.bytes = [0; NODE_NAME_PLUGIN_SIZE];
        self.bytes[..len].copy_from_slice(&data[..len]);
    }

    pub fn stream_write(&self) -> &[u8] {
        &self.bytes[..self.stream_size()]
    }

    pub fn stream_size(&self) -> usize {
        self.bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NODE_NAME_PLUGIN_SIZE)
    }

    pub fn get_frame_node_name(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.stream_size()]).unwrap_or("")
    }
}

pub fn node_name_plugin_attach() -> bool {
    true
}
