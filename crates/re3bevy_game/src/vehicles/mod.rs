#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub mod Automobile;
mod AutomobileDamage;
pub mod AutomobileEffects;
pub mod Bike;
pub mod Boat;
pub mod BoatControl;
pub mod BoatEffects;
pub mod BoatPhysics;
pub mod CarGen;
pub mod Cranes;
pub mod DamageManager;
pub mod Door;
pub mod Floater;
pub mod HandlingMgr;
pub mod Heli;
pub mod Plane;
pub mod Train;
pub mod Transmission;
pub mod Vehicle;
mod VehicleReferences;
pub(crate) use VehicleReferences::retire_vehicle_damage_references;
pub(crate) use VehicleReferences::clear_vehicle_damage_reference_targets;

pub mod VehicleSimulationPlugin;
pub mod VehicleConstruction;

pub mod VehicleDrive;
pub mod VehicleAppearance;
pub mod VehicleEntry;
pub mod VehicleAudio;
pub mod AutomobileEngineEffects;
