use re3bevy_game::modelinfo::ModelIndices::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaCatalogEntry {
    pub label: &'static str,
    pub model_index: usize,
}

pub const CHARACTERS: &[ArenaCatalogEntry] = &[
    ArenaCatalogEntry {
        label: "Claude",
        model_index: MI_PLAYER,
    },
    ArenaCatalogEntry {
        label: "Police",
        model_index: MI_COP,
    },
    ArenaCatalogEntry {
        label: "SWAT",
        model_index: MI_SWAT,
    },
    ArenaCatalogEntry {
        label: "FBI",
        model_index: MI_FBI,
    },
    ArenaCatalogEntry {
        label: "Army",
        model_index: MI_ARMY,
    },
    ArenaCatalogEntry {
        label: "Medic",
        model_index: MI_MEDIC,
    },
    ArenaCatalogEntry {
        label: "Fireman",
        model_index: MI_FIREMAN,
    },
    ArenaCatalogEntry {
        label: "Civilian",
        model_index: MI_MALE01,
    },
    // GTA III's ped.dat assigns the remaining civilian/gang models to the
    // contiguous model slots below. Keep them in the arena catalogue so every
    // streamed ped can be exercised, even when a particular install has a
    // custom name for the slot.
    ArenaCatalogEntry {
        label: "Ped 08",
        model_index: 8,
    },
    ArenaCatalogEntry {
        label: "Ped 09",
        model_index: 9,
    },
    ArenaCatalogEntry {
        label: "Ped 10",
        model_index: 10,
    },
    ArenaCatalogEntry {
        label: "Ped 11",
        model_index: 11,
    },
    ArenaCatalogEntry {
        label: "Ped 12",
        model_index: 12,
    },
    ArenaCatalogEntry {
        label: "Ped 13",
        model_index: 13,
    },
    ArenaCatalogEntry {
        label: "Ped 14",
        model_index: 14,
    },
    ArenaCatalogEntry {
        label: "Ped 15",
        model_index: 15,
    },
    ArenaCatalogEntry {
        label: "Ped 16",
        model_index: 16,
    },
    ArenaCatalogEntry {
        label: "Ped 17",
        model_index: 17,
    },
    ArenaCatalogEntry {
        label: "Ped 18",
        model_index: 18,
    },
    ArenaCatalogEntry {
        label: "Ped 19",
        model_index: 19,
    },
    ArenaCatalogEntry {
        label: "Ped 20",
        model_index: 20,
    },
    ArenaCatalogEntry {
        label: "Ped 21",
        model_index: 21,
    },
    ArenaCatalogEntry {
        label: "Ped 22",
        model_index: 22,
    },
    ArenaCatalogEntry {
        label: "Ped 23",
        model_index: 23,
    },
    ArenaCatalogEntry {
        label: "Ped 24",
        model_index: 24,
    },
    ArenaCatalogEntry {
        label: "Ped 25",
        model_index: 25,
    },
    ArenaCatalogEntry {
        label: "Ped 26",
        model_index: 26,
    },
    ArenaCatalogEntry {
        label: "Ped 27",
        model_index: 27,
    },
    ArenaCatalogEntry {
        label: "Ped 28",
        model_index: 28,
    },
    ArenaCatalogEntry {
        label: "Ped 29",
        model_index: 29,
    },
    ArenaCatalogEntry {
        label: "Ped 30",
        model_index: 30,
    },
    ArenaCatalogEntry {
        label: "Ped 31",
        model_index: 31,
    },
    ArenaCatalogEntry {
        label: "Ped 32",
        model_index: 32,
    },
];

pub const VEHICLES: &[ArenaCatalogEntry] = &[
    ArenaCatalogEntry {
        label: "Landstalker",
        model_index: MI_LANDSTAL,
    },
    ArenaCatalogEntry {
        label: "Idaho",
        model_index: MI_IDAHO,
    },
    ArenaCatalogEntry {
        label: "Stinger",
        model_index: MI_STINGER,
    },
    ArenaCatalogEntry {
        label: "Linerunner",
        model_index: MI_LINERUN,
    },
    ArenaCatalogEntry {
        label: "Perennial",
        model_index: MI_PEREN,
    },
    ArenaCatalogEntry {
        label: "Sentinel",
        model_index: MI_SENTINEL,
    },
    ArenaCatalogEntry {
        label: "Patriot",
        model_index: MI_PATRIOT,
    },
    ArenaCatalogEntry {
        label: "Firetruck",
        model_index: MI_FIRETRUCK,
    },
    ArenaCatalogEntry {
        label: "Trashmaster",
        model_index: MI_TRASH,
    },
    ArenaCatalogEntry {
        label: "Stretch",
        model_index: MI_STRETCH,
    },
    ArenaCatalogEntry {
        label: "Manana",
        model_index: MI_MANANA,
    },
    ArenaCatalogEntry {
        label: "Infernus",
        model_index: MI_INFERNUS,
    },
    ArenaCatalogEntry {
        label: "Blista",
        model_index: MI_BLISTA,
    },
    ArenaCatalogEntry {
        label: "Pony",
        model_index: MI_PONY,
    },
    ArenaCatalogEntry {
        label: "Mule",
        model_index: MI_MULE,
    },
    ArenaCatalogEntry {
        label: "Cheetah",
        model_index: MI_CHEETAH,
    },
    ArenaCatalogEntry {
        label: "Ambulance",
        model_index: MI_AMBULAN,
    },
    ArenaCatalogEntry {
        label: "FBI Car",
        model_index: MI_FBICAR,
    },
    ArenaCatalogEntry {
        label: "Moonbeam",
        model_index: MI_MOONBEAM,
    },
    ArenaCatalogEntry {
        label: "Esperanto",
        model_index: MI_ESPERANT,
    },
    ArenaCatalogEntry {
        label: "Taxi",
        model_index: MI_TAXI,
    },
    ArenaCatalogEntry {
        label: "Kuruma",
        model_index: MI_KURUMA,
    },
    ArenaCatalogEntry {
        label: "Bobcat",
        model_index: MI_BOBCAT,
    },
    ArenaCatalogEntry {
        label: "Mr Whoopee",
        model_index: MI_MRWHOOP,
    },
    ArenaCatalogEntry {
        label: "BF Injection",
        model_index: MI_BFINJECT,
    },
    ArenaCatalogEntry {
        label: "Corpse",
        model_index: MI_CORPSE,
    },
    ArenaCatalogEntry {
        label: "Police",
        model_index: MI_POLICE,
    },
    ArenaCatalogEntry {
        label: "Enforcer",
        model_index: MI_ENFORCER,
    },
    ArenaCatalogEntry {
        label: "Securicar",
        model_index: MI_SECURICA,
    },
    ArenaCatalogEntry {
        label: "Banshee",
        model_index: MI_BANSHEE,
    },
    ArenaCatalogEntry {
        label: "Predator",
        model_index: MI_PREDATOR,
    },
    ArenaCatalogEntry {
        label: "Bus",
        model_index: MI_BUS,
    },
    ArenaCatalogEntry {
        label: "Rhino",
        model_index: MI_RHINO,
    },
    ArenaCatalogEntry {
        label: "Barracks OL",
        model_index: MI_BARRACKS,
    },
    ArenaCatalogEntry {
        label: "Train",
        model_index: MI_TRAIN,
    },
    ArenaCatalogEntry {
        label: "Chopper",
        model_index: MI_CHOPPER,
    },
    ArenaCatalogEntry {
        label: "Dodo",
        model_index: MI_DODO,
    },
    ArenaCatalogEntry {
        label: "Coach",
        model_index: MI_COACH,
    },
    ArenaCatalogEntry {
        label: "Cabbie",
        model_index: MI_CABBIE,
    },
    ArenaCatalogEntry {
        label: "Stallion",
        model_index: MI_STALLION,
    },
    ArenaCatalogEntry {
        label: "Rumpo",
        model_index: MI_RUMPO,
    },
    ArenaCatalogEntry {
        label: "RC Bandit",
        model_index: MI_RCBANDIT,
    },
    ArenaCatalogEntry {
        label: "Belly-Up",
        model_index: MI_BELLYUP,
    },
    ArenaCatalogEntry {
        label: "Mr Wong's",
        model_index: MI_MRWONGS,
    },
    ArenaCatalogEntry {
        label: "Mafia Sentinel",
        model_index: MI_MAFIA,
    },
    ArenaCatalogEntry {
        label: "Yardie Lobo",
        model_index: MI_YARDIE,
    },
    ArenaCatalogEntry {
        label: "Yakuza Stinger",
        model_index: MI_YAKUZA,
    },
    ArenaCatalogEntry {
        label: "Diablo Stallion",
        model_index: MI_DIABLOS,
    },
    ArenaCatalogEntry {
        label: "Cartel Cruiser",
        model_index: MI_COLUMB,
    },
    ArenaCatalogEntry {
        label: "Hoods Rumpo XL",
        model_index: MI_HOODS,
    },
    ArenaCatalogEntry {
        label: "Airtrain",
        model_index: MI_AIRTRAIN,
    },
    ArenaCatalogEntry {
        label: "Dead Dodo",
        model_index: MI_DEADDODO,
    },
    ArenaCatalogEntry {
        label: "Speeder",
        model_index: MI_SPEEDER,
    },
    ArenaCatalogEntry {
        label: "Reefer",
        model_index: MI_REEFER,
    },
    ArenaCatalogEntry {
        label: "Panlantic",
        model_index: MI_PANLANT,
    },
    ArenaCatalogEntry {
        label: "Flatbed",
        model_index: MI_FLATBED,
    },
    ArenaCatalogEntry {
        label: "Yankee",
        model_index: MI_YANKEE,
    },
    ArenaCatalogEntry {
        label: "Escape",
        model_index: MI_ESCAPE,
    },
    ArenaCatalogEntry {
        label: "Borgnine Taxi",
        model_index: MI_BORGNINE,
    },
    ArenaCatalogEntry {
        label: "Toyz",
        model_index: MI_TOYZ,
    },
    ArenaCatalogEntry {
        label: "Ghost",
        model_index: MI_GHOST,
    },
];

pub const PROPS: &[ArenaCatalogEntry] = &[
    ArenaCatalogEntry {
        label: "Briefcase",
        model_index: MI_DONKEYMAG,
    },
    ArenaCatalogEntry {
        label: "Bullion",
        model_index: MI_BULLION,
    },
    ArenaCatalogEntry {
        label: "Car Mine",
        model_index: MI_CARMINE,
    },
    ArenaCatalogEntry {
        label: "Sea Mine",
        model_index: MI_NAUTICALMINE,
    },
    ArenaCatalogEntry {
        label: "Pickup Bonus",
        model_index: MI_PICKUP_BONUS,
    },
    ArenaCatalogEntry {
        label: "Car Wheel",
        model_index: MI_CAR_WHEEL,
    },
];

pub const GTA_TELEPORTS: &[(&str, [f32; 3])] = &[
    ("Arena Origin", [0.0, 0.0, 0.0]),
    ("Portland / Industrial District", [1138.0, -936.0, 14.0]),
    ("Portland / Harbor", [1370.0, -940.0, 14.0]),
    ("Portland / Chinatown", [1050.0, -630.0, 14.0]),
    ("Portland / Saint Mark's", [1185.0, -400.0, 24.0]),
    ("Portland / Red Light District", [953.0, -291.0, 8.0]),
    ("Portland / Hideout", [885.0, -309.0, 8.0]),
    ("Staunton / Belleville Park", [79.0, -786.0, 14.0]),
    ("Staunton / Newport", [350.0, -550.0, 24.0]),
    ("Staunton / Campus", [-150.0, -1100.0, 26.0]),
    ("Staunton / Hideout", [100.0, -475.0, 16.0]),
    ("Shoreside / Airport", [-1100.0, -185.0, 14.0]),
    ("Shoreside / Wichita Gardens", [-780.0, -560.0, 18.0]),
    ("Shoreside / Hideout", [-655.0, -930.0, 32.0]),
    ("Shoreside / Cochrane Dam", [-1100.0, 100.0, 60.0]),
];
