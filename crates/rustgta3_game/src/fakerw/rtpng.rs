//! re3 reference: references/re-GTA/src/fakerw/rtpng.h
//!
//! PNG image IO request facade.

use super::{
    rtbmp::{RtImageFormat, RtImageIoRequest},
    rwcore::RwImage,
};

pub fn RtPNGImageWrite(
    image: RwImage,
    image_name: impl Into<String>,
) -> (RwImage, RtImageIoRequest) {
    (
        image,
        RtImageIoRequest {
            path: image_name.into(),
            format: RtImageFormat::Png,
        },
    )
}

pub fn RtPNGImageRead(image_name: impl Into<String>) -> RtImageIoRequest {
    RtImageIoRequest {
        path: image_name.into(),
        format: RtImageFormat::Png,
    }
}
