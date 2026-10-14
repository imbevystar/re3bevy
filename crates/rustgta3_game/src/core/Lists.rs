//! re3 reference: references/re-GTA/src/core/Lists.cpp
//! re3 reference: references/re-GTA/src/core/Lists.h
//!
//! Intrusive list index helpers.

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CPtrNode {
    pub item: usize,
    pub prev: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CPtrList {
    pub head: Option<usize>,
    pub nodes: Vec<CPtrNode>,
}

impl CPtrList {
    #[allow(non_snake_case)]
    pub fn InsertItem(&mut self, item: usize) -> usize {
        let index = self.nodes.len();
        self.nodes.push(CPtrNode {
            item,
            prev: None,
            next: self.head,
        });
        self.head = Some(index);
        index
    }
}
