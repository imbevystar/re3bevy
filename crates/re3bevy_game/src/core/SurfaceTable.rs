//! re3 reference: references/re-GTA/src/core/SurfaceTable.cpp
//! re3 reference: references/re-GTA/src/core/SurfaceTable.h
//!
//! Surface adhesion helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eSurfaceType {
    SURFACE_DEFAULT = 0,
    SURFACE_TARMAC = 1,
    SURFACE_GRASS = 2,
    SURFACE_GRAVEL = 3,
    SURFACE_MUD_DRY = 4,
    SURFACE_PAVEMENT = 5,
    SURFACE_CAR = 6,
    SURFACE_GLASS = 7,
    SURFACE_TRANSPARENT_CLOTH = 8,
    SURFACE_GARAGE_DOOR = 9,
    SURFACE_CAR_PANEL = 10,
    SURFACE_THICK_METAL_PLATE = 11,
    SURFACE_SCAFFOLD_POLE = 12,
    SURFACE_LAMP_POST = 13,
    SURFACE_FIRE_HYDRANT = 14,
    SURFACE_GIRDER = 15,
    SURFACE_METAL_CHAIN_FENCE = 16,
    SURFACE_PED = 17,
    SURFACE_SAND = 18,
    SURFACE_WATER = 19,
    SURFACE_WOOD_CRATES = 20,
    SURFACE_WOOD_BENCH = 21,
    SURFACE_WOOD_SOLID = 22,
    SURFACE_RUBBER = 23,
    SURFACE_PLASTIC = 24,
    SURFACE_HEDGE = 25,
    SURFACE_STEEP_CLIFF = 26,
    SURFACE_CONTAINER = 27,
    SURFACE_NEWS_VENDOR = 28,
    SURFACE_WHEELBASE = 29,
    SURFACE_CARDBOARDBOX = 30,
    SURFACE_TRANSPARENT_STONE = 31,
    SURFACE_METAL_GATE = 32,
    SURFACE_SAND_BEACH = 33,
    SURFACE_CONCRETE_BEACH = 34,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eAdhesiveGroup {
    ADHESIVE_RUBBER = 0,
    ADHESIVE_HARD = 1,
    ADHESIVE_ROAD = 2,
    ADHESIVE_LOOSE = 3,
    ADHESIVE_WET = 4,
}

pub const NUMADHESIVEGROUPS: usize = re3bevy_formats::formats::DataRead::SURFACE_ADHESIVE_GROUP_COUNT;

pub struct CSurfaceTable;

impl CSurfaceTable {
    /// re3 `CSurfaceTable::GetAdhesiveLimit`. `surface.dat` stores the
    /// symmetric lower-triangular matrix by adhesive-group name; keep the
    /// lookup data-driven instead of treating the arena's grid as unit grip.
    pub fn get_adhesive_limit(
        table: &re3bevy_formats::formats::DataRead::SurfaceDat,
        surface_a: u8,
        surface_b: u8,
    ) -> f32 {
        let group_a = Self::get_adhesion_group(surface_a) as usize;
        let group_b = Self::get_adhesion_group(surface_b) as usize;
        let (row, column) = if group_a >= group_b {
            (group_a, group_b)
        } else {
            (group_b, group_a)
        };
        match table
            .rows
            .get(row)
            .and_then(|row_data| row_data.adhesive_limits.get(column))
        {
            Some(Some(value)) => *value,
            // re3 initialises `-` entries in SURFACE.DAT to zero adhesion.
            Some(None) => 0.0,
            None => panic!("surface_adhesion_missing_required_cell: row={row} column={column}; required SURFACE.DAT must pass the asset loader"),
        }
    }

    pub fn get_adhesion_group(surface_type: u8) -> eAdhesiveGroup {
        match surface_type {
            0 | 1 | 5 | 20 | 21 | 22 => eAdhesiveGroup::ADHESIVE_ROAD,
            2 | 3 | 18 | 25 | 26 | 30 => eAdhesiveGroup::ADHESIVE_LOOSE,
            4 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 24 | 27 | 28 | 31 | 32 => {
                eAdhesiveGroup::ADHESIVE_HARD
            }
            17 | 23 | 29 => eAdhesiveGroup::ADHESIVE_RUBBER,
            19 => eAdhesiveGroup::ADHESIVE_WET,
            _ => eAdhesiveGroup::ADHESIVE_ROAD,
        }
    }

    pub fn get_wet_multiplier(surface_type: u8, wet_roads: f32) -> f32 {
        match surface_type {
            0 | 1 | 4 | 5 | 8 | 20 | 21 | 22 | 25 | 30 | 31 => 1.0 - wet_roads * 0.25,
            2 | 6 | 7 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 23 | 24 | 26 | 27 | 28 | 29
            | 32 => 1.0 - wet_roads * 0.4,
            _ => 1.0,
        }
    }
}
