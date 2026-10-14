//! re3 reference: references/re-GTA/src/core/templates.h
//!
//! Small fixed-size containers used as Rust equivalents of re3 templates.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CVectorList<T> {
    pub values: Vec<T>,
    pub max_size: usize,
}

impl<T> CVectorList<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            values: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn add(&mut self, value: T) -> bool {
        if self.values.len() >= self.max_size {
            return false;
        }
        self.values.push(value);
        true
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CLink<T> {
    pub item: T,
    pub prev: Option<usize>,
    pub next: Option<usize>,
}
