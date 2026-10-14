//! re3 reference: references/re-GTA/src/core/re3.cpp
//!
//! Build/version switches from the original project.

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Re3BuildInfo {
    pub project_name: &'static str,
    pub version: &'static str,
}

pub const RE3_BUILD_INFO: Re3BuildInfo = Re3BuildInfo {
    project_name: "rust-gta-3",
    version: env!("CARGO_PKG_VERSION"),
};
