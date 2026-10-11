//! ECS and authored-asset inputs for `CAutomobile::PreRender` lights.
//!
//! TODO(parity): CAutomobile::PreRender inactive FBI/rear corona coordinate
//! updates, alarm range/reverse semantics and player directional/rear lights.
//! TODO(parity): CBrightLights::RegisterOne front/rear automobile geometry.
//! StoreCarLightShadow's transient projections are owned by Shadows and
//! CarLightRender, including native receiver clipping and render state.

use crate::{
    control::ScriptEntities::{ScriptEntityIndex, ScriptVehicleIdentity},
    vehicles::{
        Automobile::AutomobileState,
        Vehicle::{VehicleAppearance, VehicleCore, VehicleDriveState},
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use re3bevy_core::core::Coordinates::CVector;

/// Executable roof-lamp layout from `CAutomobile::PreRender`'s model switch.
pub(super) struct EmergencyLightLayout {
    pub right: CVector,
    pub left: CVector,
    pub primary_colour: [u8; 3],
    pub secondary_colour: [u8; 3],
}

pub(super) fn emergency_light_layout(model_index: usize) -> Option<EmergencyLightLayout> {
    use crate::modelinfo::ModelIndices::{MI_AMBULAN, MI_ENFORCER, MI_FIRETRUCK, MI_POLICE};
    Some(match model_index {
        MI_FIRETRUCK => EmergencyLightLayout {
            right: CVector::new(1.1, 1.7, 2.0),
            left: CVector::new(-1.1, 1.7, 2.0),
            primary_colour: [255, 0, 0],
            secondary_colour: [255, 255, 0],
        },
        MI_AMBULAN => EmergencyLightLayout {
            right: CVector::new(1.1, 0.9, 1.6),
            left: CVector::new(-1.1, 0.9, 1.6),
            primary_colour: [255, 0, 0],
            secondary_colour: [255, 255, 255],
        },
        MI_POLICE => EmergencyLightLayout {
            right: CVector::new(0.7, -0.4, 1.0),
            left: CVector::new(-0.7, -0.4, 1.0),
            primary_colour: [255, 0, 0],
            secondary_colour: [0, 0, 255],
        },
        MI_ENFORCER => EmergencyLightLayout {
            right: CVector::new(1.1, 0.8, 1.2),
            left: CVector::new(-1.1, 0.8, 1.2),
            primary_colour: [255, 0, 0],
            secondary_colour: [0, 0, 255],
        },
        _ => return None,
    })
}

/// Named waiting/failure counts for the original model/DFF lighting boundary.
#[derive(Debug, Default, Resource)]
pub struct VehicleLightingDiagnostics {
    pub processed: u64,
    pub retired_entities: u64,
    pub missing_dependencies: u64,
    pub waiting_model: u64,
    pub waiting_clump: u64,
    pub missing_handling: u64,
    pub authored_lamps_resolved: u64,
    pub missing_headlights: u64,
    pub missing_taillights: u64,
    pub invalid_actor_kind: u64,
}

/// Light toggle writes common state; damage and taxi inputs are readonly.
/// Unrelated actor, vitality and collision-history queries remain independent.
#[derive(SystemParam)]
pub(super) struct VehicleLightingAccess<'w, 's> {
    pub(super) index: Option<Res<'w, ScriptEntityIndex>>,
    pub(super) cars: Query<
        'w,
        's,
        (
            Entity,
            &'static ScriptVehicleIdentity,
            &'static VehicleCore,
            &'static VehicleDriveState,
            &'static crate::vehicles::Vehicle::VehicleAudioState,
            &'static mut VehicleAppearance,
            &'static AutomobileState,
        ),
    >,
    pub(super) diagnostics: ResMut<'w, VehicleLightingDiagnostics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        control::ScriptEntities::ScriptVehicleKind,
        modelinfo::ModelIndices::MI_POLICE,
        peds::Ped::CPed,
        vehicles::{
            Automobile::{CAutomobile, VehicleCollisionState},
            Vehicle::VehicleVitality,
        },
    };
    use bevy::ecs::system::SystemState;
    use re3bevy_core::core::General::CGeneral;

    #[test]
    fn lighting_borrows_damage_readonly_and_coexists_with_vitality_and_history_writes() {
        let mut world = World::new();
        world.init_resource::<ScriptEntityIndex>();
        world.init_resource::<VehicleLightingDiagnostics>();
        let mut general = CGeneral::with_seed(29);
        let handle = 0x501;
        let (core, vitality, drive, appearance, entry, audio, car, audio_motion) = CAutomobile::new(
            MI_POLICE as i32,
            crate::vehicles::Vehicle::RANDOM_VEHICLE,
            0,
            &mut general,
            None,
            None,
            false,
        )
        .into_parts();
        let entity = world
            .spawn((
                ScriptVehicleIdentity {
                    handle,
                    kind: ScriptVehicleKind::Automobile,
                },
                core,
                vitality,
                drive,
                appearance,
                entry,
                audio,
                car,
                audio_motion,
            ))
            .id();
        world
            .resource_mut::<ScriptEntityIndex>()
            .vehicles
            .insert(handle, entity);
        let mut joined = SystemState::<(
            VehicleLightingAccess,
            Query<&mut VehicleVitality>,
            Query<&mut VehicleCollisionState>,
            Query<&mut CPed>,
        )>::new(&mut world);
        {
            let (mut lights, mut vitality, mut history, _) = joined.get_mut(&mut world).unwrap();
            vitality.get_mut(entity).unwrap().health = 57.0;
            assert!(history.get_mut(entity).is_ok());
            let (actor, identity, _core, _drive, _audio, mut appearance, car) =
                lights.cars.single_mut().unwrap();
            assert_eq!(
                lights.index.as_deref().unwrap().vehicle(identity.handle),
                Some(actor)
            );
            assert!(!car.taxi_light);
            appearance.lights_on = true;
        }
        assert!(world.get::<VehicleAppearance>(entity).unwrap().lights_on);
        assert_eq!(world.get::<VehicleVitality>(entity).unwrap().health, 57.0);
    }
}
