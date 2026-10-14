//! re3 reference: references/re-GTA/src/rw/RwHelper.cpp
//! re3 reference: references/re-GTA/src/rw/RwHelper.h
//!
//! RenderWare helper state that Bevy render adapters can consume.

use bevy::prelude::Resource;

use super::Lights::RwRGBAReal;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct RwHelperState {
    pub debug_render_groups: bool,
    pub ps2_alpha_test: bool,
    pub backface_culling: bool,
    pub charset_open: bool,
    pub debug_prints: Vec<DebugPrintf>,
    pub render_state: RwDefinedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugPrintf {
    pub text: String,
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwDefinedState {
    pub texture_address_wrap: bool,
    pub texture_perspective: bool,
    pub z_test: bool,
    pub z_write: bool,
    pub shade_mode_gouraud: bool,
    pub texture_filter_linear: bool,
    pub vertex_alpha: bool,
    pub src_blend: RwBlend,
    pub dst_blend: RwBlend,
    pub alpha_primitive_buffer: bool,
    pub border_color: RwRGBAReal,
    pub fog_enable: bool,
    pub fog_color: RwRGBAReal,
    pub fog_type_linear: bool,
    pub cull_mode: RwCullMode,
    pub alpha_ref: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwBlend {
    SrcAlpha,
    InvSrcAlpha,
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwCullMode {
    CullNone,
    CullBack,
}

impl RwHelperState {
    pub fn open_charset_safe(&mut self) {
        self.charset_open = true;
    }

    pub fn create_debug_font(&mut self) {
        self.open_charset_safe();
    }

    pub fn destroy_debug_font(&mut self) {
        self.charset_open = false;
        self.debug_prints.clear();
    }

    pub fn obrs_printf_string(&mut self, text: impl Into<String>, x: i16, y: i16) {
        self.debug_prints.push(DebugPrintf {
            text: text.into(),
            x: x * 8,
            y: y * 16,
        });
    }

    pub fn flush_obrs_printfs(&mut self) -> Vec<DebugPrintf> {
        std::mem::take(&mut self.debug_prints)
    }

    pub fn defined_state(&mut self, fog_color: RwRGBAReal) {
        self.render_state = RwDefinedState {
            texture_address_wrap: true,
            texture_perspective: true,
            z_test: true,
            z_write: true,
            shade_mode_gouraud: true,
            texture_filter_linear: true,
            vertex_alpha: false,
            src_blend: RwBlend::SrcAlpha,
            dst_blend: RwBlend::InvSrcAlpha,
            alpha_primitive_buffer: false,
            border_color: RwRGBAReal::rgb(0.0, 0.0, 0.0),
            fog_enable: false,
            fog_color,
            fog_type_linear: true,
            cull_mode: RwCullMode::CullNone,
            alpha_ref: 2,
        };
    }

    pub fn set_alpha_ref(&mut self, reference: i32) {
        self.render_state.alpha_ref = reference;
    }

    pub fn set_cull_mode(&mut self, mode: RwCullMode) {
        self.render_state.cull_mode = if self.backface_culling {
            mode
        } else {
            RwCullMode::CullNone
        };
    }
}

impl Default for RwHelperState {
    fn default() -> Self {
        Self {
            debug_render_groups: false,
            ps2_alpha_test: false,
            backface_culling: true,
            charset_open: false,
            debug_prints: Vec::new(),
            render_state: RwDefinedState::default(),
        }
    }
}

impl Default for RwDefinedState {
    fn default() -> Self {
        Self {
            texture_address_wrap: true,
            texture_perspective: true,
            z_test: true,
            z_write: true,
            shade_mode_gouraud: true,
            texture_filter_linear: true,
            vertex_alpha: false,
            src_blend: RwBlend::SrcAlpha,
            dst_blend: RwBlend::InvSrcAlpha,
            alpha_primitive_buffer: false,
            border_color: RwRGBAReal::rgb(0.0, 0.0, 0.0),
            fog_enable: false,
            fog_color: RwRGBAReal::rgb(0.0, 0.0, 0.0),
            fog_type_linear: true,
            cull_mode: RwCullMode::CullNone,
            alpha_ref: 2,
        }
    }
}

pub fn convert_platform_atomic(atomic_id: usize) -> usize {
    atomic_id
}
