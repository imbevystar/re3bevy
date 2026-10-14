//! Native CPed vitality state, shared by damage, fire, scripts and save/restore.
//! re3 references: CPed::CPed, CPed::InflictDamage, CPed::SetDie/SetDead.
use bevy::prelude::Component;

/// Native ePedPieceTypes values at the COL/damage/save boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PedDamagePiece { Torso, Mid, LeftArm, RightArm, LeftLeg, RightLeg, Head }

#[derive(Debug, Clone, PartialEq, Component)]
pub struct PedVitality {
    pub health: f32,
    pub armour: f32,
    pub fire: Option<usize>,
    pub body_part_bleeding: i8,
    pub bloody_footprint_count_or_death_time: u32,
    pub dead_bleeding: bool,
    pub last_wep_dam: i8,
}
impl Default for PedVitality {
    fn default() -> Self {
        Self {
            health: 100.0,
            armour: 0.0,
            fire: None,
            body_part_bleeding: -1,
            bloody_footprint_count_or_death_time: 0,
            dead_bleeding: false,
            last_wep_dam: -1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::Ped::{CPed, PedState, PedWeaponDamageResult};
    use crate::peds::PedType::ePedType;
    use crate::weapons::WeaponType::eWeaponType;
    use bevy::prelude::World;
    use rust_gta_3_core::core::General::CGeneral;

    #[test]
    fn damage_and_control_borrow_the_same_ecs_vitality() {
        let mut world = World::new();
        let actor = world
            .spawn(CPed::new(
                ePedType::PEDTYPE_CIVMALE,
                &mut CGeneral::with_seed(19),
            ))
            .id();
        world.get_mut::<PedVitality>(actor).unwrap().health = 0.0;
        assert!(
            !world
                .get::<CPed>(actor)
                .unwrap()
                .is_ped_in_control(world.get::<PedVitality>(actor).unwrap())
        );
        {
            let mut vitality = world.get_mut::<PedVitality>(actor).unwrap();
            vitality.health = 100.0;
            vitality.armour = 20.0;
        }
        let mut actors = world.query::<(&mut CPed, &mut PedVitality)>();
        let (mut ped, mut vitality) = actors.get_mut(&mut world, actor).unwrap();
        assert_eq!(
            ped.inflict_weapon_damage(
                &mut vitality,
                eWeaponType::WEAPONTYPE_COLT45,
                40.0,
                1.0,
                true,
                true,
                10,
                0
            ),
            PedWeaponDamageResult::Applied
        );
        assert_eq!(vitality.armour, 0.0);
        assert_eq!(vitality.health, 80.0);
        vitality.armour = 20.0;
        assert_eq!(
            ped.inflict_weapon_damage(
                &mut vitality,
                eWeaponType::WEAPONTYPE_DROWNING,
                20.0,
                1.0,
                true,
                true,
                20,
                0
            ),
            PedWeaponDamageResult::Applied
        );
        assert_eq!(vitality.armour, 20.0);
        assert_eq!(vitality.health, 60.0);
        assert_eq!(
            ped.inflict_weapon_damage(
                &mut vitality,
                eWeaponType::WEAPONTYPE_COLT45,
                1000.0,
                1.0,
                true,
                true,
                30,
                0
            ),
            PedWeaponDamageResult::Killed
        );
        assert_eq!(vitality.health, 0.0);
        assert_eq!(ped.ped_state, PedState::PED_DIE);
        assert!(!ped.is_ped_in_control(&vitality));
    }
}
