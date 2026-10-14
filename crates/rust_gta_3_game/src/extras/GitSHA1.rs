//! re3 reference: references/re-GTA/src/extras/GitSHA1.cpp
//!
//! Build revision string holder.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitSHA1 {
    pub sha1: String,
}
