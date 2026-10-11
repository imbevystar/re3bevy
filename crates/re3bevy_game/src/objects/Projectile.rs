//! re3 reference: references/re-GTA/src/objects/Projectile.cpp
//! re3 reference: references/re-GTA/src/objects/Projectile.h
//!
//! Projectile object state.

use re3bevy_core::core::General::CGeneral;

use crate::{
    entities::Physical::GRAVITY,
    objects::Object::{CObject, MISSION_OBJECT},
};

#[derive(Debug, Clone, PartialEq)]
pub struct CProjectile {
    pub object: CObject,
}

impl CProjectile {
    pub fn new(model: i32, general: &mut CGeneral) -> Self {
        let mut object = CObject::new(general);
        object.physical.mass = 1.0;
        object.physical.turn_mass = 1.0;
        object.physical.air_resistance = 0.99999;
        object.physical.elasticity = 0.75;
        object.physical.buoyancy = GRAVITY * object.physical.mass * 0.1;
        object.physical.entity.flags.explosion_proof = true;
        object
            .physical
            .entity
            .set_model_index_no_create(model as usize);
        object.object_created_by = MISSION_OBJECT;
        Self { object }
    }
}
