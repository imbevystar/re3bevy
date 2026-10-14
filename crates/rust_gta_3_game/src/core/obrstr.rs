//! re3 reference: references/re-GTA/src/core/obrstr.cpp
//! re3 reference: references/re-GTA/src/core/obrstr.h
//!
//! Obfuscated string helper facade.

#[allow(non_snake_case)]
pub fn ObrsDecode(input: &[u8]) -> String {
    String::from_utf8_lossy(input).to_string()
}
