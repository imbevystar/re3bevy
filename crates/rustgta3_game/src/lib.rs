#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

#[cfg(any(feature = "profiling", feature = "profile-capture"))]
macro_rules! profile_scope {
    ($name:literal) => {
        let _rustgta3_profile_span = bevy::log::info_span!($name).entered();
    };
}

#[cfg(not(any(feature = "profiling", feature = "profile-capture")))]
macro_rules! profile_scope {
    ($name:literal) => {};
}

pub(crate) use profile_scope;

pub mod animation;
pub mod audio;
pub mod buildings;
pub mod collision;
pub mod control;
pub mod core;
pub mod entities;
pub mod extras;
pub mod fakerw;
pub mod math;
pub mod modelinfo;
pub mod objects;
pub mod peds;
pub mod renderer;
pub mod rw;
pub mod save;
pub mod skel;
pub mod text;
pub mod vehicles;
pub mod weapons;
