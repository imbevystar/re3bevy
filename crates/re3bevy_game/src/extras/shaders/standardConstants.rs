//! re3 reference: references/re-GTA/src/extras/shaders/standardConstants.h
//!
//! Register layout used by the original standard shader constants.

pub const COMBINED_MAT_REGISTER: u32 = 0;
pub const WORLD_MAT_REGISTER: u32 = 4;
pub const NORMAL_MAT_REGISTER: u32 = 8;
pub const MAT_COL_REGISTER: u32 = 12;
pub const SURF_PROPS_REGISTER: u32 = 13;
pub const FOG_DATA_REGISTER: u32 = 14;
pub const AMBIENT_LIGHT_REGISTER: u32 = 15;
pub const FIRST_LIGHT_REGISTER: u32 = 16;
pub const LIGHTS_REGISTER: u32 = 17;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StandardShaderConstants {
    pub combined_mat: [[f32; 4]; 4],
    pub world_mat: [[f32; 4]; 4],
    pub normal_mat: [[f32; 3]; 3],
    pub mat_col: [f32; 4],
    pub surf_props: [f32; 4],
    pub fog_data: [f32; 4],
    pub ambient_light: [f32; 4],
    pub first_light: [i32; 4],
}

impl StandardShaderConstants {
    pub fn surf_ambient(&self) -> f32 {
        self.surf_props[0]
    }

    pub fn surf_specular(&self) -> f32 {
        self.surf_props[1]
    }

    pub fn surf_diffuse(&self) -> f32 {
        self.surf_props[2]
    }

    pub fn fog_start(&self) -> f32 {
        self.fog_data[0]
    }

    pub fn fog_end(&self) -> f32 {
        self.fog_data[1]
    }

    pub fn fog_range(&self) -> f32 {
        self.fog_data[2]
    }

    pub fn fog_disable(&self) -> f32 {
        self.fog_data[3]
    }
}
