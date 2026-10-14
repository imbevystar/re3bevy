//! Live, immutable actor inputs for CHud::Draw and CRadar::DrawBlips.

use bevy::{ecs::system::SystemParam, prelude::*};
use rustgta3_core::core::Coordinates::CVector;

use crate::{
    control::ScriptEntities::{
        ScriptEntityIndex, ScriptObjectIdentity, ScriptObjectRef, ScriptPedIdentity,
    },
    objects::Object::CObject,
    peds::{Ped::CPed, VehicleRelations::InVehicle},
};

/// Frame-local draw inputs, calculated only for actors requested by the HUD.
pub(super) struct HudActorPose {
    pub position: CVector,
    pub heading_degrees: f32,
    pub speed: f32,
    pub current_vehicle: Option<i32>,
}

#[derive(SystemParam)]
pub(super) struct HudActorsRead<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    peds: Query<
        'w,
        's,
        (
            &'static ScriptPedIdentity,
            &'static CPed,
            Option<&'static InVehicle>,
        ),
    >,
    vehicles: crate::control::ScriptEntities::VehicleReadStore<'w, 's>,
    objects: Query<'w, 's, (&'static ScriptObjectIdentity, &'static CObject)>,
}

impl HudActorsRead<'_, '_> {
    pub fn vehicle_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        let handle = self.vehicles.get(entity).ok()?.handle();
        (self.index.vehicle(handle) == Some(entity)).then_some(handle)
    }

    pub fn vehicle_snapshot(&self, handle: i32) -> Option<HudActorPose> {
        let vehicle = self.vehicles.get(self.index.vehicle(handle)?).ok()?;
        if vehicle.handle() != handle {
            return None;
        }
        let base = vehicle.base();
        let physical = &base.physical;
        Some(HudActorPose {
            position: physical.entity.get_position(),
            heading_degrees: physical.entity.get_heading_degrees(),
            speed: physical.move_speed.magnitude(),
            current_vehicle: None,
        })
    }

    pub fn char_snapshot(&self, handle: i32) -> Option<HudActorPose> {
        let (identity, ped, relation) = self.peds.get(self.index.ped(handle)?).ok()?;
        if identity.handle != handle {
            return None;
        }
        Some(HudActorPose {
            position: ped.physical.entity.get_position(),
            heading_degrees: ped.physical.entity.get_heading_degrees(),
            speed: ped.physical.move_speed.magnitude(),
            current_vehicle: relation
                .and_then(|relation| self.vehicle_handle_for_entity(relation.vehicle)),
        })
    }

    pub fn object_get(&self, handle: i32) -> Option<ScriptObjectRef<'_>> {
        let (identity, object) = self.objects.get(self.index.object(handle)?).ok()?;
        (identity.handle == handle).then_some(ScriptObjectRef { handle, object })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::ScriptEntities::{
            ScriptPedEntity, script_entity_test_world, spawn_script_test_ped,
        },
        peds::PedType::ePedType,
    };
    use bevy::ecs::system::SystemState;
    use rustgta3_core::core::General::CGeneral;

    #[test]
    fn presentation_reads_live_mutations_and_rejects_stale_generations() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(1);
        let live_handle = 0x102;
        let stale_handle = 0x101;
        let entity = spawn_script_test_ped(
            &mut world,
            ScriptPedEntity {
                vitality: crate::peds::PedVitality::PedVitality::default(),
                locomotion: crate::peds::PedLocomotion::PedLocomotion::default(),
                animation: crate::peds::PedAnimation::PedAnimation::default(),
                handle: live_handle,
                ped: CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
                model_id: 0,
            },
        );
        world
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .insert(stale_handle, entity);
        let mut state = SystemState::<HudActorsRead>::new(&mut world);
        assert!(
            state
                .get(&world)
                .unwrap()
                .char_snapshot(stale_handle)
                .is_none()
        );
        let position = CVector::new(5.0, 8.0, 12.0);
        world
            .get_mut::<CPed>(entity)
            .unwrap()
            .physical
            .entity
            .set_position(position);
        assert_eq!(
            state
                .get(&world)
                .unwrap()
                .char_snapshot(live_handle)
                .unwrap()
                .position,
            position
        );
        world
            .resource_mut::<ScriptEntityIndex>()
            .peds
            .remove(&live_handle);
        assert!(
            state
                .get(&world)
                .unwrap()
                .char_snapshot(live_handle)
                .is_none()
        );
        world.despawn(entity);
        assert!(
            state
                .get(&world)
                .unwrap()
                .char_snapshot(stale_handle)
                .is_none()
        );
    }
}
