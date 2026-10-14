//! re3 reference: references/re-GTA/src/animation/AnimBlendList.h
//!
//! Minimal link primitive mirroring `CAnimBlendLink`.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CAnimBlendLink {
    pub next: Option<usize>,
    pub prev: Option<usize>,
}

impl CAnimBlendLink {
    pub fn init(&mut self) {
        self.next = None;
        self.prev = None;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CAnimBlendLinkList {
    pub links: Vec<CAnimBlendLink>,
    pub head: CAnimBlendLink,
}

impl CAnimBlendLinkList {
    pub fn prepend(&mut self, link_index: usize) {
        if link_index >= self.links.len() {
            self.links.resize(link_index + 1, CAnimBlendLink::default());
        }
        let old_next = self.head.next;
        if let Some(next) = old_next {
            if let Some(next_link) = self.links.get_mut(next) {
                next_link.prev = Some(link_index);
            }
        }
        self.links[link_index].next = old_next;
        self.links[link_index].prev = None;
        self.head.next = Some(link_index);
    }

    pub fn remove(&mut self, link_index: usize) {
        let Some(link) = self.links.get(link_index).copied() else {
            return;
        };
        if let Some(prev) = link.prev {
            if let Some(prev_link) = self.links.get_mut(prev) {
                prev_link.next = link.next;
            }
        } else {
            self.head.next = link.next;
        }
        if let Some(next) = link.next {
            if let Some(next_link) = self.links.get_mut(next) {
                next_link.prev = link.prev;
            }
        }
        if let Some(slot) = self.links.get_mut(link_index) {
            slot.init();
        }
    }
}
