//! re3 reference: references/re-GTA/src/skel/events.cpp
//! re3 reference: references/re-GTA/src/skel/events.h
//!
//! Event queue bridge matching the RenderWare skeleton event vocabulary.

use std::collections::VecDeque;

use bevy::prelude::Resource;

use crate::skel::skeleton::{RsEvent, RsEventStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct RsEventRecord {
    pub event: RsEvent,
    pub param: usize,
}

#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct RsEventQueue {
    pub events: VecDeque<RsEventRecord>,
}

#[allow(non_snake_case)]
pub fn RsQueueEvent(queue: &mut RsEventQueue, event: RsEvent, param: usize) {
    queue.events.push_back(RsEventRecord { event, param });
}

#[allow(non_snake_case)]
pub fn RsNextEvent(queue: &mut RsEventQueue) -> Option<RsEventRecord> {
    queue.events.pop_front()
}

#[allow(non_snake_case)]
pub fn RsDispatchQueuedEvents<F>(queue: &mut RsEventQueue, mut handler: F) -> RsEventStatus
where
    F: FnMut(RsEvent, usize) -> RsEventStatus,
{
    let mut status = RsEventStatus::rsEVENTNOTPROCESSED;
    while let Some(record) = RsNextEvent(queue) {
        status = handler(record.event, record.param);
        if status == RsEventStatus::rsEVENTERROR {
            break;
        }
    }
    status
}
