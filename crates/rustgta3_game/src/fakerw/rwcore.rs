//! re3 reference: references/re-GTA/src/fakerw/rwcore.h
//!
//! RenderWare core facade used by re3-facing modules.

pub use super::rwplcore::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwRaster {
    pub width: RwInt32,
    pub height: RwInt32,
    pub depth: RwInt32,
    pub flags: RwInt32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwImage {
    pub width: RwInt32,
    pub height: RwInt32,
    pub depth: RwInt32,
    pub stride: RwInt32,
    pub pixels: Vec<RwUInt8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwTexture {
    pub name: String,
    pub mask_name: String,
    pub raster: Option<RwRaster>,
    pub ref_count: u32,
    pub max_anisotropy: RwInt8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwFrame {
    pub matrix: RwMatrix,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RwCamera {
    pub view_offset: RwV2d,
    pub view_window: RwV2d,
    pub near_plane: RwReal,
    pub far_plane: RwReal,
    pub fog_plane: RwReal,
}

impl RwRaster {
    pub fn create(width: RwInt32, height: RwInt32, depth: RwInt32, flags: RwInt32) -> Self {
        Self {
            width,
            height,
            depth,
            flags,
        }
    }
}

impl RwTexture {
    pub fn create(raster: RwRaster) -> Self {
        Self {
            name: String::new(),
            mask_name: String::new(),
            raster: Some(raster),
            ref_count: 1,
            max_anisotropy: 1,
        }
    }

    pub fn add_ref(&mut self) {
        self.ref_count = self.ref_count.saturating_add(1);
    }
}

impl Default for RwFrame {
    fn default() -> Self {
        Self {
            matrix: RwMatrix::default(),
            parent: None,
            children: Vec::new(),
        }
    }
}

impl Default for RwCamera {
    fn default() -> Self {
        Self {
            view_offset: RwV2d::default(),
            view_window: RwV2d { x: 0.7, y: 0.7 },
            near_plane: 0.9,
            far_plane: 1000.0,
            fog_plane: 1000.0,
        }
    }
}

pub fn RwRasterCreate(width: RwInt32, height: RwInt32, depth: RwInt32, flags: RwInt32) -> RwRaster {
    RwRaster::create(width, height, depth, flags)
}

pub fn RwTextureCreate(raster: RwRaster) -> RwTexture {
    RwTexture::create(raster)
}

pub fn RwTextureAddRef(texture: &mut RwTexture) {
    texture.add_ref();
}

pub fn RwTextureSetName(texture: &mut RwTexture, name: &str) {
    texture.name = name.chars().take(31).collect();
}

pub fn RwTextureSetMaskName(texture: &mut RwTexture, name: &str) {
    texture.mask_name = name.chars().take(31).collect();
}

pub fn RwFrameCreate() -> RwFrame {
    RwFrame::default()
}

pub fn RwCameraCreate() -> RwCamera {
    RwCamera::default()
}
