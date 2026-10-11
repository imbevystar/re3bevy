//! re3 reference: references/re-GTA/src/core/config.h
//!
//! PC `Config` enum constants used to size re3 runtime state.

pub const NUMPLAYERS: usize = 1;
pub const NUMCDIMAGES: usize = 12;
pub const MAX_CDIMAGES: usize = 8;
pub const MAX_CDCHANNELS: usize = 5;

pub const MODELINFOSIZE: usize = 5500;
pub const TXDSTORESIZE: usize = 1024;
pub const EXTRADIRSIZE: usize = 128;
pub const CUTSCENEDIRSIZE: usize = 512;

pub const SIMPLEMODELSIZE: usize = 5000;
pub const MLOMODELSIZE: usize = 1;
pub const MLOINSTANCESIZE: usize = 1;
pub const TIMEMODELSIZE: usize = 30;
pub const CLUMPMODELSIZE: usize = 5;
pub const PEDMODELSIZE: usize = 90;
pub const VEHICLEMODELSIZE: usize = 120;
pub const XTRACOMPSMODELSIZE: usize = 2;
pub const TWODFXSIZE: usize = 2000;

pub const MAXVEHICLESLOADED: usize = 50;
pub const NUMOBJECTINFO: usize = 168;

pub const NUMPTRNODES: usize = 30000;
pub const NUMENTRYINFOS: usize = 5400;
pub const NUMPEDS: usize = 140;
pub const NUMVEHICLES: usize = 110;
pub const NUMBUILDINGS: usize = 5500;
pub const NUMTREADABLES: usize = 1214;
pub const NUMOBJECTS: usize = 450;
pub const NUMDUMMIES: usize = 2802;
pub const NUMAUDIOSCRIPTOBJECTS: usize = 256;
pub const NUMCUTSCENEOBJECTS: usize = 50;

pub const NUMANIMBLOCKS: usize = 2;
pub const NUMANIMATIONS: usize = 250;
pub const NUMTEMPOBJECTS: usize = 30;

pub const NUM_PATHNODES: usize = 4930;
pub const NUM_CARPATHLINKS: usize = 2076;
pub const NUM_MAPOBJECTS: usize = 1250;
pub const NUM_PATHCONNECTIONS: usize = 10260;

pub const NUMALPHALIST: usize = 20;
pub const NUMALPHAENTITYLIST: usize = 150;
pub const NUMCOLCACHELINKS: usize = 200;
pub const NUMREFERENCES: usize = 800;

pub const NUMAUDIOZONES: usize = 36;
pub const NUMZONES: usize = 50;
pub const NUMMAPZONES: usize = 25;

pub const NUMCULLZONES: usize = 512;
pub const NUMATTRIBZONES: usize = 288;
pub const NUMZONEINDICES: usize = 55000;

pub const NUMWEATHERS: usize = 4;
pub const NUMHOURS: usize = 24;
pub const NUMPICKUPS: usize = 336;
pub const NUMCOLLECTEDPICKUPS: usize = 20;
pub const NUMPACMANPICKUPS: usize = 256;
pub const NUM_CARGENS: usize = 160;
pub const NUM_PATH_NODES_IN_AUTOPILOT: usize = 8;
pub const NUM_GARAGES: usize = 32;
pub const NUM_PROJECTILES: usize = 32;
pub const NUM_CRANES: usize = 8;
pub const NUM_EXPLOSIONS: usize = 48;

pub const GTA3_PS2_140: i32 = 300;
pub const GTA3_PS2_160: i32 = 301;
pub const GTA3_PC_10: i32 = 310;
pub const GTA3_PC_11: i32 = 311;
pub const GTA3_PC_STEAM: i32 = 312;
pub const GTA_VERSION: i32 = GTA3_PC_11;
