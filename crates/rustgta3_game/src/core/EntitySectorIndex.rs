//! Derived Entity membership in native CWorld sectors. No gameplay pose copies.
use bevy::prelude::Entity;
use rustgta3_core::core::Coordinates::CVector;
use std::collections::{HashMap, HashSet};
use super::World::CWorld;

pub use crate::math::Rect::CRect as SectorRect;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Sector { x: usize, y: usize }
#[derive(Clone, Copy, PartialEq, Eq)]
struct SectorRange { min_x: usize, min_y: usize, max_x: usize, max_y: usize }
impl SectorRange {
    fn from_rect(rect: SectorRect) -> Option<Self> {
        ([rect.left, rect.top, rect.right, rect.bottom].iter().all(|value| value.is_finite())
            && rect.left <= rect.right && rect.top <= rect.bottom).then(|| Self {
            min_x: CWorld::get_sector_index_x(rect.left), max_x: CWorld::get_sector_index_x(rect.right),
            min_y: CWorld::get_sector_index_y(rect.top), max_y: CWorld::get_sector_index_y(rect.bottom),
        })
    }
    fn cells(self) -> impl Iterator<Item = Sector> {
        (self.min_y..=self.max_y).flat_map(move |y| (self.min_x..=self.max_x).map(move |x| Sector { x, y }))
    }
}

#[derive(Default)]
pub struct EntitySectorIndex {
    members: HashMap<Entity, SectorRange>,
    sectors: HashMap<Sector, HashSet<Entity>>,
}
impl EntitySectorIndex {
    pub fn contains(&self, entity: &Entity) -> bool { self.members.contains_key(entity) }
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ { self.members.keys().copied() }
    pub fn remove(&mut self, entity: Entity) {
        if let Some(range) = self.members.remove(&entity) {
            for sector in range.cells() {
                if let Some(members) = self.sectors.get_mut(&sector) {
                    members.remove(&entity);
                    if members.is_empty() { self.sectors.remove(&sector); }
                }
            }
        }
    }
    pub fn update(&mut self, entity: Entity, position: CVector) -> bool {
        if !position.z.is_finite() { self.remove(entity); return false; }
        self.update_rect(entity, SectorRect::new(position.x, position.y, position.x, position.y))
    }
    pub fn update_rect(&mut self, entity: Entity, rect: SectorRect) -> bool {
        let Some(range) = SectorRange::from_rect(rect) else { self.remove(entity); return false; };
        if self.members.get(&entity) == Some(&range) { return true; }
        self.remove(entity);
        self.members.insert(entity, range);
        for sector in range.cells() { self.sectors.entry(sector).or_default().insert(entity); }
        true
    }
    pub fn candidates(&self, position: CVector, radius: f32, output: &mut Vec<Entity>) -> bool {
        if !radius.is_finite() || radius < 0.0 || !position.z.is_finite() { output.clear(); return false; }
        self.candidates_rect(SectorRect::new(position.x - radius, position.y - radius,
            position.x + radius, position.y + radius), output)
    }
    pub fn candidates_rect(&self, rect: SectorRect, output: &mut Vec<Entity>) -> bool {
        output.clear();
        let Some(range) = SectorRange::from_rect(rect) else { return false; };
        for sector in range.cells() {
            if let Some(members) = self.sectors.get(&sector) { output.extend(members.iter().copied()); }
        }
        // Native scan codes visit overlap members once. This list intentionally
        // provides identity uniqueness, not native list provenance/order.
        output.sort_unstable_by_key(|entity| entity.to_bits());
        output.dedup();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;

    #[test]
    fn overlap_members_are_unique_and_obsolete_cells_retire_on_move_and_remove() {
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let mut index = EntitySectorIndex::default();
        let wide = SectorRect::new(-60.0, -60.0, 60.0, 60.0);
        assert!(index.update_rect(actor, wide));
        let mut found = Vec::new();
        index.candidates_rect(wide, &mut found);
        assert_eq!(found, [actor]);
        index.candidates(CVector::new(50.0, 50.0, 0.0), 0.0, &mut found);
        assert_eq!(found, [actor], "overlap sector must find an actor whose center is elsewhere");
        index.update(actor, CVector::new(500.0, 500.0, 0.0));
        index.candidates_rect(wide, &mut found);
        assert!(found.is_empty());
        index.remove(actor);
        assert!(!index.contains(&actor));
        assert!(index.entities().next().is_none());
    }

    #[test]
    fn malformed_bounds_remove_membership_and_invalid_queries_clear_results() {
        let mut world = World::new();
        let actor = world.spawn_empty().id();
        let mut index = EntitySectorIndex::default();
        index.update(actor, CVector::ZERO);
        let invalid = SectorRect::new(f32::NAN, 0.0, 1.0, 1.0);
        assert!(!index.update_rect(actor, invalid));
        assert!(!index.contains(&actor));
        let mut found = vec![actor];
        assert!(!index.candidates_rect(invalid, &mut found));
        assert!(found.is_empty());
    }

    #[test]
    fn native_col_box_uses_the_live_entity_matrix_and_canonical_rect_type() {
        let mut random = rustgta3_core::core::General::CGeneral::default();
        let mut entity = crate::entities::Entity::CEntity::new(&mut random);
        entity.set_position(CVector::new(10.0, 20.0, 30.0));
        let col = crate::collision::ColModel::CColModel::new(
            crate::collision::ColModel::CColSphere::new(
                5.0,
                crate::collision::ColModel::CColVector::new(0.0, 0.0, 0.0),
                0,
                0,
            ),
            crate::collision::ColModel::CColBox::new(
                crate::collision::ColModel::CColVector::new(-2.0, -3.0, -1.0),
                crate::collision::ColModel::CColVector::new(3.0, 4.0, 2.0),
                0,
                0,
            ),
            Vec::new(),
        );
        let rect: crate::math::Rect::CRect = entity.get_bound_rect(&col);
        assert_eq!(rect, SectorRect::new(8.0, 17.0, 13.0, 24.0));
        entity.set_position(CVector::new(100.0, 200.0, 300.0));
        assert_eq!(entity.get_bound_rect(&col), SectorRect::new(98.0, 197.0, 103.0, 204.0));
        let mut physical = crate::entities::Physical::CPhysical::new(&mut random);
        physical.entity = entity;
        assert_eq!(physical.get_bound_rect(&col), SectorRect::new(95.0, 195.0, 105.0, 205.0),
            "moving actors use the physical sphere override, not the entity box");
    }
}
