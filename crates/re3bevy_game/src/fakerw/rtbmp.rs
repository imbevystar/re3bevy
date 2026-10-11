//! re3 reference: references/re-GTA/src/fakerw/rtbmp.h
//!
//! BMP image IO request facade.

use super::rwcore::RwImage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtImageIoRequest {
    pub path: String,
    pub format: RtImageFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtImageFormat {
    Bmp,
    Png,
}

pub fn RtBMPImageWrite(
    image: RwImage,
    image_name: impl Into<String>,
) -> (RwImage, RtImageIoRequest) {
    (
        image,
        RtImageIoRequest {
            path: image_name.into(),
            format: RtImageFormat::Bmp,
        },
    )
}

pub fn RtBMPImageRead(image_name: impl Into<String>) -> RtImageIoRequest {
    RtImageIoRequest {
        path: image_name.into(),
        format: RtImageFormat::Bmp,
    }
}
