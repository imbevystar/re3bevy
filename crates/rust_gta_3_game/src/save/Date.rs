//! re3 reference: references/re-GTA/src/save/Date.cpp
//! re3 reference: references/re-GTA/src/save/Date.h
//!
//! `CDate` save timestamp value object.

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CDate {
    pub m_nSecond: i32,
    pub m_nMinute: i32,
    pub m_nHour: i32,
    pub m_nDay: i32,
    pub m_nMonth: i32,
    pub m_nYear: i32,
}

impl CDate {
    pub fn PopulateDateFields(
        &mut self,
        second: i8,
        minute: i8,
        hour: i8,
        day: i8,
        month: i8,
        year: i16,
    ) {
        self.m_nSecond = second as i32;
        self.m_nMinute = minute as i32;
        self.m_nHour = hour as i32;
        self.m_nDay = day as i32;
        self.m_nMonth = month as i32;
        self.m_nYear = year as i32;
    }

    fn ordering_tuple(&self) -> (i32, i32, i32, i32, i32, i32) {
        (
            self.m_nYear,
            self.m_nMonth,
            self.m_nDay,
            self.m_nHour,
            self.m_nMinute,
            self.m_nSecond,
        )
    }
}

impl PartialOrd for CDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CDate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ordering_tuple().cmp(&other.ordering_tuple())
    }
}
