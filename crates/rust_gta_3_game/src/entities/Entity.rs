//! re3 reference: references/re-GTA/src/entities/Entity.cpp
//! re3 reference: references/re-GTA/src/entities/Entity.h
//!
//! Core entity state shared by buildings, vehicles, peds, objects and dummies.

use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

use crate::math::Matrix::CMatrix;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eEntityType {
    ENTITY_TYPE_NOTHING = 0,
    ENTITY_TYPE_BUILDING = 1,
    ENTITY_TYPE_VEHICLE = 2,
    ENTITY_TYPE_PED = 3,
    ENTITY_TYPE_OBJECT = 4,
    ENTITY_TYPE_DUMMY = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eEntityStatus {
    STATUS_PLAYER = 0,
    STATUS_PLAYER_PLAYBACKFROMBUFFER = 1,
    STATUS_SIMPLE = 2,
    STATUS_PHYSICS = 3,
    STATUS_ABANDONED = 4,
    STATUS_WRECKED = 5,
    STATUS_TRAIN_MOVING = 6,
    STATUS_TRAIN_NOT_MOVING = 7,
    STATUS_HELI = 8,
    STATUS_PLANE = 9,
    STATUS_PLAYER_REMOTE = 10,
    STATUS_PLAYER_DISABLED = 11,
}

pub(crate) fn script_entity_status(value: i32) -> Option<eEntityStatus> {
    Some(match value {
        0 => eEntityStatus::STATUS_PLAYER,
        1 => eEntityStatus::STATUS_PLAYER_PLAYBACKFROMBUFFER,
        2 => eEntityStatus::STATUS_SIMPLE,
        3 => eEntityStatus::STATUS_PHYSICS,
        4 => eEntityStatus::STATUS_ABANDONED,
        5 => eEntityStatus::STATUS_WRECKED,
        6 => eEntityStatus::STATUS_TRAIN_MOVING,
        7 => eEntityStatus::STATUS_TRAIN_NOT_MOVING,
        8 => eEntityStatus::STATUS_HELI,
        9 => eEntityStatus::STATUS_PLANE,
        10 => eEntityStatus::STATUS_PLAYER_REMOTE,
        11 => eEntityStatus::STATUS_PLAYER_DISABLED,
        _ => return None,
    })
}

pub use crate::math::Rect::CRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CEntityFlags {
    pub uses_collision: bool,
    pub collision_processed: bool,
    pub is_static: bool,
    pub has_contacted: bool,
    pub ped_physics: bool,
    pub is_stuck: bool,
    pub is_in_safe_position: bool,
    pub use_collision_records: bool,
    pub was_postponed: bool,
    pub explosion_proof: bool,
    pub is_visible: bool,
    pub has_collided: bool,
    pub render_scorched: bool,
    pub has_blip: bool,
    pub is_big_building: bool,
    pub render_damaged: bool,
    pub bullet_proof: bool,
    pub fire_proof: bool,
    pub collision_proof: bool,
    pub melee_proof: bool,
    pub only_damaged_by_player: bool,
    pub streaming_dont_delete: bool,
    pub zone_culled: bool,
    pub zone_culled2: bool,
    pub remove_from_world: bool,
    pub has_hit_wall: bool,
    pub im_being_rendered: bool,
    pub touching_water: bool,
    pub is_subway: bool,
    pub draw_last: bool,
    pub no_bright_head_lights: bool,
    pub do_not_render: bool,
    pub distance_fade: bool,
    pub flag_e2: bool,
}

impl Default for CEntityFlags {
    fn default() -> Self {
        Self {
            uses_collision: false,
            collision_processed: false,
            is_static: false,
            has_contacted: false,
            ped_physics: false,
            is_stuck: false,
            is_in_safe_position: false,
            use_collision_records: false,
            was_postponed: false,
            explosion_proof: false,
            is_visible: true,
            has_collided: false,
            render_scorched: false,
            has_blip: false,
            is_big_building: false,
            render_damaged: false,
            bullet_proof: false,
            fire_proof: false,
            collision_proof: false,
            melee_proof: false,
            only_damaged_by_player: false,
            streaming_dont_delete: false,
            zone_culled: false,
            zone_culled2: false,
            remove_from_world: false,
            has_hit_wall: false,
            im_being_rendered: false,
            touching_water: false,
            is_subway: false,
            draw_last: false,
            no_bright_head_lights: false,
            do_not_render: false,
            distance_fade: false,
            flag_e2: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CEntity {
    pub entity_type: eEntityType,
    pub status: eEntityStatus,
    pub flags: CEntityFlags,
    pub scan_code: u16,
    pub random_seed: u16,
    pub model_index: i16,
    pub level: u16,
    pub first_reference: Option<usize>,
    pub rw_object_loaded: bool,
    /// RenderWare clump alpha. Ambient peds/vehicles are born at zero and
    /// `ProcessControl` blends them in by 16 per frame; fade-out subtracts 8.
    pub clump_alpha: u8,
    pub matrix: CMatrix,
}

impl CEntity {
    pub fn new(general: &mut CGeneral) -> Self {
        Self {
            entity_type: eEntityType::ENTITY_TYPE_NOTHING,
            status: eEntityStatus::STATUS_ABANDONED,
            flags: CEntityFlags::default(),
            scan_code: 0,
            random_seed: general.get_random_number(),
            model_index: -1,
            level: 0,
            first_reference: None,
            rw_object_loaded: false,
            clump_alpha: u8::MAX,
            matrix: CMatrix::unity(),
        }
    }

    pub fn set_type(&mut self, entity_type: eEntityType) {
        self.entity_type = entity_type;
    }

    pub fn set_status(&mut self, status: eEntityStatus) {
        self.status = status;
    }

    pub fn set_is_static(&mut self, state: bool) {
        self.flags.is_static = state;
    }

    pub fn set_model_index_no_create(&mut self, id: usize) {
        self.model_index = id as i16;
    }

    pub fn set_model_index_loaded_marker(&mut self, id: usize) {
        self.model_index = id as i16;
        self.rw_object_loaded = true;
    }

    pub fn delete_rw_object_marker(&mut self) {
        self.rw_object_loaded = false;
    }

    pub fn get_position(&self) -> CVector {
        self.matrix.position
    }

    pub fn set_position(&mut self, position: CVector) {
        self.matrix.position = position;
    }

    pub fn translate(&mut self, translation: CVector) {
        self.matrix.translate(translation);
    }

    pub fn get_right(&self) -> CVector {
        self.matrix.right
    }

    pub fn get_forward(&self) -> CVector {
        self.matrix.forward
    }

    pub fn get_up(&self) -> CVector {
        self.matrix.up
    }

    pub fn set_heading(&mut self, angle: f32) {
        let position = self.matrix.position;
        self.matrix.set_rotate_z(angle);
        self.matrix.translate(position);
    }

    /// re3 `CPlaceable::GetHeading` — Z rotation from matrix right basis.
    pub fn get_heading(&self) -> f32 {
        self.matrix.right.y.atan2(self.matrix.right.x)
    }

    pub fn get_heading_degrees(&self) -> f32 {
        self.get_heading().to_degrees()
    }

    pub fn set_orientation(&mut self, x: f32, y: f32, z: f32) {
        let position = self.matrix.position;
        self.matrix.set_rotate(x, y, z);
        self.matrix.translate(position);
    }

    pub fn set_matrix(&mut self, matrix: CMatrix) {
        self.matrix = matrix;
    }

    pub fn transform_point(&self, point: CVector) -> CVector {
        self.matrix.transform_point(point)
    }

    pub fn transform_vector(&self, vector: CVector) -> CVector {
        self.matrix.multiply3x3(vector)
    }

    pub fn is_building(&self) -> bool {
        self.entity_type == eEntityType::ENTITY_TYPE_BUILDING
    }

    pub fn is_vehicle(&self) -> bool {
        self.entity_type == eEntityType::ENTITY_TYPE_VEHICLE
    }

    pub fn is_ped(&self) -> bool {
        self.entity_type == eEntityType::ENTITY_TYPE_PED
    }

    pub fn is_object(&self) -> bool {
        self.entity_type == eEntityType::ENTITY_TYPE_OBJECT
    }

    pub fn is_dummy(&self) -> bool {
        self.entity_type == eEntityType::ENTITY_TYPE_DUMMY
    }

    pub fn get_bound_rect_from_sphere(&self, center: CVector, radius: f32) -> CRect {
        CRect::new(
            center.x - radius,
            center.y - radius,
            center.x + radius,
            center.y + radius,
        )
    }

    /// Native CEntity::GetBoundRect: four authored COL box corners transformed
    /// by the live matrix, including native min/max Z choice on each Y edge.
    pub fn get_bound_rect(&self, col: &crate::collision::ColModel::CColModel) -> CRect {
        let min = col.bounding_box.min;
        let max = col.bounding_box.max;
        let mut rect = CRect::default();
        for point in [CVector::new(min.x, min.y, min.z), CVector::new(max.x, max.y, max.z),
            CVector::new(max.x, min.y, min.z), CVector::new(min.x, max.y, max.z)] {
            rect.ContainPoint(self.matrix.transform_point(point));
        }
        rect
    }
}
