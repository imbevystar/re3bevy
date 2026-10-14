//! re3 reference: references/re-GTA/src/collision/VuCollision.cpp
//! re3 reference: references/re-GTA/src/collision/VuCollision.h
//!
//! PC fallback for VU triangle collision routines.

use crate::math::VuVector::{CVuVector, CrossProduct, DotProduct};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VuTriangle {
    pub v0: [i32; 4],
    pub v1: [i32; 4],
    pub v2: [i32; 4],
    pub plane: [i32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VuCollisionResult {
    pub hit: bool,
    pub point: CVuVector,
    pub normal: CVuVector,
    pub distance: f32,
}

impl Default for VuCollisionResult {
    fn default() -> Self {
        Self {
            hit: false,
            point: CVuVector::default(),
            normal: CVuVector::default(),
            distance: 0.0,
        }
    }
}

pub fn LineToTriangleCollision(
    p0: CVuVector,
    p1: CVuVector,
    v0: CVuVector,
    v1: CVuVector,
    v2: CVuVector,
    plane: CVuVector,
) -> VuCollisionResult {
    let dot0 = DotProduct(plane, p0);
    let dot1 = DotProduct(plane, p1);
    let dist0 = plane.w - dot0;
    let dist1 = plane.w - dot1;
    if dist0 * dist1 > 0.0 {
        return VuCollisionResult::default();
    }

    let diff = p1 - p0;
    let t = dist0 / (dot1 - dot0);
    let mut point = p0 + diff * t;
    point.w = 0.0;

    let cross1 = CrossProduct(point - v0, v1 - v0);
    let cross2 = CrossProduct(point - v1, v2 - v1);
    let cross3 = CrossProduct(point - v2, v0 - v2);
    let flagmask = plane_flag_mask(plane, 0.5);
    let flags1 = sign_flags(cross1) & flagmask;
    let flags2 = sign_flags(cross2) & flagmask;
    let flags3 = sign_flags(cross3) & flagmask;
    if flags1 != flags2 || flags1 != flags3 {
        return VuCollisionResult::default();
    }

    VuCollisionResult {
        hit: true,
        point,
        normal: plane,
        distance: t,
    }
}

pub fn LineToTriangleCollisionCompressed(
    p0: CVuVector,
    p1: CVuVector,
    tri: VuTriangle,
) -> VuCollisionResult {
    let (v0, v1, v2, plane) = unpack_triangle(tri);
    LineToTriangleCollision(p0, p1, v0, v1, v2, plane)
}

pub fn SphereToTriangleCollision(
    sph: CVuVector,
    v0: CVuVector,
    v1: CVuVector,
    v2: CVuVector,
    plane: CVuVector,
) -> VuCollisionResult {
    let mut planedist = DotProduct(plane, sph) - plane.w;
    if planedist.abs() > sph.w {
        return VuCollisionResult::default();
    }

    let mut point = sph - plane * planedist;
    point.w = 0.0;
    planedist = planedist.abs();

    let v01 = v1 - v0;
    let v12 = v2 - v1;
    let v20 = v0 - v2;
    let cross1 = CrossProduct(point - v0, v01);
    let cross2 = CrossProduct(point - v1, v12);
    let cross3 = CrossProduct(point - v2, v20);
    let flagmask = plane_flag_mask(plane, 0.1);
    let nflags = sign_flags(plane) & flagmask;
    let flags1 = sign_flags(cross1) & flagmask;
    let flags2 = sign_flags(cross2) & flagmask;
    let flags3 = sign_flags(cross3) & flagmask;

    let mut testcase = 0;
    let mut closest = CVuVector::default();
    if flags1 == nflags {
        closest += v2;
        testcase += 1;
    }
    if flags2 == nflags {
        closest += v0;
        testcase += 1;
    }
    if flags3 == nflags {
        closest += v1;
        testcase += 1;
    }

    if testcase == 3 {
        return VuCollisionResult {
            hit: true,
            point,
            normal: plane,
            distance: planedist,
        };
    }

    if testcase == 1 {
        return sphere_closest_result(sph, closest);
    }

    let closest = if flags1 != nflags {
        DistanceBetweenSphereAndLine(sph, v0, v01)
    } else if flags2 != nflags {
        DistanceBetweenSphereAndLine(sph, v1, v12)
    } else {
        DistanceBetweenSphereAndLine(sph, v2, v20)
    };
    sphere_closest_result(sph, closest)
}

pub fn SphereToTriangleCollisionCompressed(sph: CVuVector, tri: VuTriangle) -> VuCollisionResult {
    let (v0, v1, v2, plane) = unpack_triangle(tri);
    SphereToTriangleCollision(sph, v0, v1, v2, plane)
}

pub fn DistanceBetweenSphereAndLine(
    center: CVuVector,
    p0: CVuVector,
    line: CVuVector,
) -> CVuVector {
    let p1 = p0 + line;
    let dist0 = center - p0;
    let dist1 = center - p1;
    let len_sq = line.MagnitudeSqr();
    let dist_sq0 = dist0.MagnitudeSqr();
    let dist_sq1 = dist1.MagnitudeSqr();
    let dot = DotProduct(dist0, line);
    if dot < 0.0 {
        let mut ret = p0;
        ret.w = dist_sq0;
        return ret;
    }
    let t = dot / len_sq;
    if t > 1.0 {
        let mut ret = p1;
        ret.w = dist_sq1;
        return ret;
    }
    let mut ret = p0 + line * t;
    ret.w = (ret - center).MagnitudeSqr();
    ret
}

fn sphere_closest_result(sph: CVuVector, closest: CVuVector) -> VuCollisionResult {
    let delta = sph - closest;
    let dist_sq = delta.MagnitudeSqr();
    if sph.w * sph.w <= dist_sq {
        return VuCollisionResult::default();
    }
    let distance = dist_sq.sqrt();
    let normal = if distance > 0.0 {
        delta * (1.0 / distance)
    } else {
        CVuVector::new(0.0, 0.0, 1.0)
    };
    VuCollisionResult {
        hit: true,
        point: closest,
        normal,
        distance,
    }
}

fn unpack_triangle(tri: VuTriangle) -> (CVuVector, CVuVector, CVuVector, CVuVector) {
    (
        unpack_vertex(tri.v0),
        unpack_vertex(tri.v1),
        unpack_vertex(tri.v2),
        CVuVector::new4(
            tri.plane[0] as f32 / 4096.0,
            tri.plane[1] as f32 / 4096.0,
            tri.plane[2] as f32 / 4096.0,
            tri.plane[3] as f32 / 128.0,
        ),
    )
}

fn unpack_vertex(v: [i32; 4]) -> CVuVector {
    CVuVector::new4(
        v[0] as f32 / 128.0,
        v[1] as f32 / 128.0,
        v[2] as f32 / 128.0,
        v[3] as f32 / 128.0,
    )
}

fn sign_flags(v: CVuVector) -> i32 {
    let mut flags = 0;
    if v.x < 0.0 {
        flags |= 1;
    }
    if v.y < 0.0 {
        flags |= 2;
    }
    if v.z < 0.0 {
        flags |= 4;
    }
    flags
}

fn plane_flag_mask(plane: CVuVector, threshold: f32) -> i32 {
    let mut flagmask = 0;
    if plane.x.abs() > threshold {
        flagmask |= 1;
    }
    if plane.y.abs() > threshold {
        flagmask |= 2;
    }
    if plane.z.abs() > threshold {
        flagmask |= 4;
    }
    flagmask
}
