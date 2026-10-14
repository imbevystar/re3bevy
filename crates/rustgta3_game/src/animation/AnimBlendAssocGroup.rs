//! re3 reference: references/re-GTA/src/animation/AnimBlendAssocGroup.cpp
//! re3 reference: references/re-GTA/src/animation/AnimBlendAssocGroup.h
//!
//! Group of animation association templates.

use crate::animation::{
    AnimBlendAssociation::CAnimBlendAssociation, AnimBlendHierarchy::CAnimBlendHierarchy,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CAnimBlendAssocGroup {
    pub assoc_list: Vec<CAnimBlendAssociation>,
}

impl CAnimBlendAssocGroup {
    pub fn new() -> Self {
        Self {
            assoc_list: Vec::new(),
        }
    }

    pub fn num_associations(&self) -> usize {
        self.assoc_list.len()
    }

    pub fn destroy_associations(&mut self) {
        self.assoc_list.clear();
    }

    pub fn get_animation(&self, id: usize) -> Option<&CAnimBlendAssociation> {
        // re3 callers address an association by AnimationId. Most PC tables are
        // contiguous, but missing IFP names must not shift later IDs.
        self.assoc_list
            .iter()
            .find(|association| association.anim_id == id as i32)
    }

    pub fn get_animation_mut(&mut self, id: usize) -> Option<&mut CAnimBlendAssociation> {
        let index = self
            .assoc_list
            .iter()
            .position(|association| association.anim_id == id as i32)?;
        self.assoc_list.get_mut(index)
    }

    pub fn get_animation_by_name(&self, name: &str) -> Option<&CAnimBlendAssociation> {
        let clean = name.trim().trim_matches('\0');
        self.assoc_list.iter().find(|assoc| {
            let hname = assoc.hierarchy.name.trim().trim_matches('\0');
            hname.eq_ignore_ascii_case(clean)
                || (clean.len() > 3
                    && clean.as_bytes()[2] == b'_'
                    && hname.eq_ignore_ascii_case(&clean[3..]))
                || (clean.len() > 3
                    && clean.as_bytes()[3] == b'_'
                    && hname.eq_ignore_ascii_case(&clean[4..]))
                || (hname.len() > 3
                    && hname.as_bytes()[2] == b'_'
                    && clean.eq_ignore_ascii_case(&hname[3..]))
                || (hname.len() > 3
                    && hname.as_bytes()[3] == b'_'
                    && clean.eq_ignore_ascii_case(&hname[4..]))
        })
    }

    pub fn copy_animation(&self, id: usize) -> Option<CAnimBlendAssociation> {
        let mut anim = self.get_animation(id)?.clone();
        anim.hierarchy.uncompress();
        Some(anim)
    }

    pub fn copy_animation_by_name(&self, name: &str) -> Option<CAnimBlendAssociation> {
        let mut anim = self.get_animation_by_name(name)?.clone();
        anim.hierarchy.uncompress();
        Some(anim)
    }

    pub fn create_associations_from_hierarchies(&mut self, hierarchies: &[CAnimBlendHierarchy]) {
        self.destroy_associations();
        for (index, hierarchy) in hierarchies.iter().enumerate() {
            let mut assoc = CAnimBlendAssociation::new();
            assoc.init_with_hierarchy(hierarchy.clone(), hierarchy.sequences.len());
            assoc.anim_id = index as i32;
            self.assoc_list.push(assoc);
        }
    }

    pub fn create_named_associations(
        &mut self,
        block_hierarchies: &[CAnimBlendHierarchy],
        anim_names: &[&str],
    ) {
        self.destroy_associations();
        for (index, name) in anim_names.iter().enumerate() {
            if let Some(hierarchy) = block_hierarchies
                .iter()
                .find(|hierarchy| hierarchy.name.eq_ignore_ascii_case(name))
            {
                let mut assoc = CAnimBlendAssociation::new();
                assoc.init_with_hierarchy(hierarchy.clone(), hierarchy.sequences.len());
                assoc.anim_id = index as i32;
                self.assoc_list.push(assoc);
            }
        }
    }
}

#[cfg(test)]
mod sparse_id_tests {
    use super::*;
    #[test]
    fn absent_id_cannot_resolve_to_an_unrelated_compacted_slot() {
        let mut first = CAnimBlendAssociation::new();
        first.anim_id = 0;
        let mut third = CAnimBlendAssociation::new();
        third.anim_id = 2;
        let mut group = CAnimBlendAssocGroup {
            assoc_list: vec![first, third],
        };
        assert!(group.get_animation(1).is_none());
        assert!(group.get_animation_mut(1).is_none());
        assert_eq!(group.get_animation(2).unwrap().anim_id, 2);
    }
}

impl Default for CAnimBlendAssocGroup {
    fn default() -> Self {
        Self::new()
    }
}

pub fn strcmp_ignoring_digits(left: &str, right: &str) -> bool {
    let mut left_iter = left.chars();
    let mut right_iter = right.chars();
    loop {
        let l = left_iter.next();
        let r = right_iter.next();
        match (l, r) {
            (None, None) => return true,
            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => continue,
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(&b) => continue,
            _ => return false,
        }
    }
}
