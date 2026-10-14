//! re3 reference: references/re-GTA/src/peds/PedAI.cpp
//! re3 reference: references/re-GTA/src/vehicles/Automobile.cpp
//!
//! Persistent `m_pVehicleAnim` ownership for ECS peds. RenderWare stores this
//! association on the ped clump; ECS stores the same per-ped state directly on
//! the ped entity so gameplay, vehicle-door processing and renderer observe one
//! timeline without a parallel handle-keyed world map.

use bevy::prelude::{Component, Entity};

use super::{
    AnimAssocDefinitions::ASSOCGRP_STD, AnimBlendAssociation::CAnimBlendAssociation,
    AnimManager::CAnimManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedVehicleAnimationPhase {
    External,
    Align,
    LockedDoor,
    OpenDoor,
    /// CPed::PedAnimPullPedOutCB: the carjacker stays outside the selected
    /// door while pulling the current occupant out.
    PullOut,
    /// CPed::BeingDraggedFromCar: the displaced occupant moves from the seat
    /// to the authored door position before its vehicle relation is cleared.
    DraggedOut,
    GetIn,
    CloseEnter,
    /// `CPed::PedAnimDoorCloseCB` right-front driver transition. The ped has
    /// entered through the passenger side and slides across to the driver's
    /// seat before `PedSetInCarCB` commits the driver relation/engine state.
    Shuffle,
    Exit,
    CloseExit,
    /// `CPlayerPed::ProcessControl` rolling-close association used when the
    /// driver resumes control with the left front door still swinging.
    /// Unlike `CloseExit`, this never changes the ped's in-vehicle state.
    RollingDoorClose,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CPedVehicleAnimation {
    /// Vehicle whose door/seat timeline this association is driving. This is
    /// an ECS relationship, not a CPools/SCM integer back-reference.
    pub vehicle: Entity,
    /// `eDoors` index: bonnet, boot, LF, RF, LR, RR.
    pub door: usize,
    pub association: CAnimBlendAssociation,
    pub phase: PedVehicleAnimationPhase,
    pub enter_as_driver: bool,
    /// `PedSetDraggedOutCarCB` runs when the dragged animation finishes, then
    /// the position/AI callback runs when that same association is deleted.
    /// Keep those two callback moments distinct even though both belong to one
    /// ECS component.
    pub dragged_out_finish_ran: bool,
}

impl CPedVehicleAnimation {
    /// re3 `CAnimManager::AddAnimation(... ASSOCGRP_STD, anim)` for an
    /// enter/exit sequence. The caller controls the actual ped state-machine
    /// transition; this API deliberately does not invent one.
    pub fn start(
        manager: &CAnimManager,
        vehicle: Entity,
        door: usize,
        animation_id: i32,
    ) -> Option<Self> {
        let Some(mut association) =
            manager.create_anim_association(ASSOCGRP_STD, animation_id as usize)
        else {
            return None;
        };
        association.start(0.0);
        Some(Self {
            vehicle,
            door,
            association,
            phase: PedVehicleAnimationPhase::External,
            enter_as_driver: false,
            dragged_out_finish_ran: false,
        })
    }

    pub fn start_phase(
        manager: &CAnimManager,
        vehicle: Entity,
        door: usize,
        animation_id: i32,
        phase: PedVehicleAnimationPhase,
        enter_as_driver: bool,
    ) -> Option<Self> {
        let Some(mut association) =
            manager.create_anim_association(ASSOCGRP_STD, animation_id as usize)
        else {
            return None;
        };
        association.start(0.0);
        Some(Self {
            vehicle,
            door,
            association,
            phase,
            enter_as_driver,
            dragged_out_finish_ran: false,
        })
    }
}
