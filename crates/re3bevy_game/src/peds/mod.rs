#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub mod CivilianPed;
pub mod CopPed;
pub mod DummyPed;
pub mod EmergencyPed;
pub mod Gangs;
pub mod Ped;
pub mod PedAI;
pub(crate) mod PedWeaponAttack;
mod PedWander;
mod PedPopulationLifecycle;
pub mod PedAnimation;
mod PedSeated;
mod PedEvasion;
mod PedEvasionTraffic;
pub use PedEvasionTraffic::PedEvasionDiagnostics;
pub(crate) mod PedRecovery;
pub use PedRecovery::apply_pending_fall_animation;
pub(crate) mod PedDeath;
mod PedCorpse;
pub use PedCorpse::PedCorpseDiagnostics;
mod PedWater;
pub mod PedChat;
pub mod PedDebug;
pub mod PedFight;
pub mod PedIK;
pub mod PedPlacement;
pub mod PedRoutes;
pub mod PedType;
pub mod PlayerPed;
pub mod PlayerRead;
pub mod Population;
pub mod VehicleRelations;
mod PedVehicleRetirement;

pub mod PedSimulationPlugin;

pub mod PedVitality;

pub mod PedLocomotion;
pub mod PedObstacle;
pub mod PedNearby;
