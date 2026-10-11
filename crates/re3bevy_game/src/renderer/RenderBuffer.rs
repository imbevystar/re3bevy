//! re3 reference: references/re-GTA/src/renderer/RenderBuffer.cpp
//! re3 reference: references/re-GTA/src/renderer/RenderBuffer.h

use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;

pub const TEMPBUFFERVERTSIZE: usize = 256;
pub const TEMPBUFFERINDEXSIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwIm3DVertexIntent {
    pub position: CVector,
    pub colour: [u8; 4],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderBufferBatch {
    pub vertices: Vec<RwIm3DVertexIntent>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone, Resource)]
pub struct RenderBuffer {
    pub vertices_to_be_stored: usize,
    pub indices_to_be_stored: usize,
    pub temp_vertices: Vec<RwIm3DVertexIntent>,
    pub temp_indices: Vec<u16>,
    pub flushed_batches: Vec<RenderBufferBatch>,
}

impl Default for RenderBuffer {
    fn default() -> Self {
        Self {
            vertices_to_be_stored: 0,
            indices_to_be_stored: 0,
            temp_vertices: Vec::with_capacity(TEMPBUFFERVERTSIZE),
            temp_indices: Vec::with_capacity(TEMPBUFFERINDEXSIZE),
            flushed_batches: Vec::new(),
        }
    }
}

impl RenderBuffer {
    pub fn clear_render_buffer(&mut self) {
        self.temp_vertices.clear();
        self.temp_indices.clear();
    }

    pub fn start_storing(&mut self, num_indices: usize, num_vertices: usize) {
        if self.temp_indices.len() + num_indices >= TEMPBUFFERINDEXSIZE {
            self.render_stuff_in_buffer();
        }
        if self.temp_vertices.len() + num_vertices >= TEMPBUFFERVERTSIZE {
            self.render_stuff_in_buffer();
        }
        self.indices_to_be_stored = num_indices;
        self.vertices_to_be_stored = num_vertices;
    }

    pub fn stop_storing(
        &mut self,
        vertices: impl IntoIterator<Item = RwIm3DVertexIntent>,
        indices: impl IntoIterator<Item = u16>,
    ) {
        let vertex_base = self.temp_vertices.len() as u16;
        self.temp_indices
            .extend(indices.into_iter().map(|index| index + vertex_base));
        self.temp_vertices.extend(vertices);
    }

    pub fn render_stuff_in_buffer(&mut self) {
        if !self.temp_vertices.is_empty() {
            self.flushed_batches.push(RenderBufferBatch {
                vertices: self.temp_vertices.clone(),
                indices: self.temp_indices.clone(),
            });
        }
        self.clear_render_buffer();
    }
}
