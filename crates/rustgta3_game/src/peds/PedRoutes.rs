//! re3 reference: references/re-GTA/src/peds/PedRoutes.cpp
//! re3 reference: references/re-GTA/src/peds/PedRoutes.h
//!
//! `CRouteNode::Initialise` state.

use bevy::prelude::*;
use rustgta3_core::core::Coordinates::CVector;

pub const NUMPEDROUTES: usize = 200;

#[derive(Debug, Clone, Copy)]
pub struct CRouteNode {
    pub route: i16,
    pub pos: CVector,
}

impl CRouteNode {
    pub const fn initialise() -> Self {
        Self {
            route: -1,
            pos: CVector::ZERO,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPedRoutes {
    pub routes: [CRouteNode; NUMPEDROUTES],
}

impl CPedRoutes {
    pub fn initialise() -> Self {
        Self {
            routes: [CRouteNode::initialise(); NUMPEDROUTES],
        }
    }

    pub fn get_route_this_point_is_on(&self, point: i16) -> Option<i16> {
        self.routes
            .get(usize::try_from(point).ok()?)
            .map(|node| node.route)
    }

    /// Despite the re3 name, this searches for the first point of a route.
    pub fn get_route_start(&self, route: i16) -> i16 {
        self.routes
            .iter()
            .position(|node| node.route == route)
            .map_or(-1, |point| point as i16)
    }

    pub fn get_point_position(&self, point: i16) -> Option<CVector> {
        self.routes
            .get(usize::try_from(point).ok()?)
            .map(|node| node.pos)
    }

    /// Returns the physical point slot selected by re3's first-free scan.
    pub fn add_route_point(&mut self, route: i16, position: CVector) -> Option<i16> {
        let point = self.routes.iter().position(|node| node.route == -1)?;
        self.routes[point].route = route;
        self.routes[point].pos = position;
        Some(point as i16)
    }

    /// Preserves the default PC re3 compaction loop (rather than the
    /// `FIX_BUGS` loop): the following `diff` records are copied over the
    /// removed contiguous run, then the tail route markers are cleared.
    pub fn remove_route(&mut self, route: i16) {
        let Some(first_point) = self.routes.iter().position(|node| node.route == route) else {
            return;
        };
        let last_point = (first_point..NUMPEDROUTES)
            .find(|&point| self.routes[point].route != route)
            .unwrap_or(NUMPEDROUTES);
        let diff = last_point - first_point;
        for offset in 0..diff {
            let destination = first_point + offset;
            let source = last_point + offset;
            self.routes[destination] = if source < NUMPEDROUTES {
                self.routes[source]
            } else {
                CRouteNode::initialise()
            };
        }
        for point in NUMPEDROUTES - diff..NUMPEDROUTES {
            self.routes[point].route = -1;
        }
    }
}

impl Default for CPedRoutes {
    fn default() -> Self {
        Self::initialise()
    }
}
