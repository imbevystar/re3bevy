//! re3 reference: references/re-GTA/src/collision/ColModel.cpp
//! re3 reference: references/re-GTA/src/collision/ColModel.h
//!
//! Collision model data shape. Solver behavior and ownership/free-list details
//! remain in the collision port.

#[path = "TriangleSpatialIndex.rs"]
mod triangle_spatial_index;
pub use triangle_spatial_index::TriangleSpatialIndex;

pub const LEVEL_GENERIC: i32 = 0;
pub const SURFACE_CAR_PANEL: u8 = 10;
pub const SURFACE_PED: u8 = 17;
pub const SURFACE_WHEELBASE: u8 = 29;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CColVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl CColVector {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CColSphere {
    pub center: CColVector,
    pub radius: f32,
    pub surface: u8,
    pub piece: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CColPoint {
    pub point: CColVector,
    pub normal: CColVector,
    pub surface_a: u8,
    pub piece_a: u8,
    pub surface_b: u8,
    pub piece_b: u8,
    pub depth: f32,
}

impl CColPoint {
    pub const fn new() -> Self {
        Self {
            point: CColVector::new(0.0, 0.0, 0.0),
            normal: CColVector::new(0.0, 0.0, 0.0),
            surface_a: 0,
            piece_a: 0,
            surface_b: 0,
            piece_b: 0,
            depth: 0.0,
        }
    }

    pub fn set_surfaces(&mut self, surface_a: u8, piece_a: u8, surface_b: u8, piece_b: u8) {
        self.surface_a = surface_a;
        self.piece_a = piece_a;
        self.surface_b = surface_b;
        self.piece_b = piece_b;
    }
}

impl CColSphere {
    pub const fn new(radius: f32, center: CColVector, surface: u8, piece: u8) -> Self {
        Self {
            center,
            radius,
            surface,
            piece,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CColBox {
    pub min: CColVector,
    pub max: CColVector,
    pub surface: u8,
    pub piece: u8,
}

impl CColBox {
    pub const fn new(min: CColVector, max: CColVector, surface: u8, piece: u8) -> Self {
        Self {
            min,
            max,
            surface,
            piece,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CColLine {
    pub p0: CColVector,
    pub p1: CColVector,
}

impl CColLine {
    pub const fn new(p0: CColVector, p1: CColVector) -> Self {
        Self { p0, p1 }
    }
}

pub fn test_sphere_sphere(first: &CColSphere, second: &CColSphere) -> bool {
    let distance = first.radius + second.radius;
    dot_col_vec(
        sub_col_vec(first.center, second.center),
        sub_col_vec(first.center, second.center),
    ) < distance * distance
}

pub fn test_sphere_box(sphere: &CColSphere, box_: &CColBox) -> bool {
    !(sphere.center.x + sphere.radius < box_.min.x
        || sphere.center.x - sphere.radius > box_.max.x
        || sphere.center.y + sphere.radius < box_.min.y
        || sphere.center.y - sphere.radius > box_.max.y
        || sphere.center.z + sphere.radius < box_.min.z
        || sphere.center.z - sphere.radius > box_.max.z)
}

pub fn test_vertical_line_box(line: &CColLine, box_: &CColBox) -> bool {
    if line.p0.x <= box_.min.x
        || line.p0.y <= box_.min.y
        || line.p0.x >= box_.max.x
        || line.p0.y >= box_.max.y
    {
        return false;
    }
    if line.p0.z < line.p1.z {
        line.p0.z <= box_.max.z && line.p1.z >= box_.min.z
    } else {
        line.p1.z <= box_.max.z && line.p0.z >= box_.min.z
    }
}

/// re3 deliberately does not report a hit when `p0` is inside the sphere.
pub fn test_line_sphere(line: &CColLine, sphere: &CColSphere) -> bool {
    let v01 = sub_col_vec(line.p1, line.p0);
    let v0c = sub_col_vec(sphere.center, line.p0);
    let linesq = dot_col_vec(v01, v01);
    let projline = -2.0 * dot_col_vec(v01, v0c);
    let tangent = 4.0
        * linesq
        * (dot_col_vec(sphere.center, sphere.center) - 2.0 * dot_col_vec(sphere.center, line.p0)
            + dot_col_vec(line.p0, line.p0)
            - sphere.radius * sphere.radius);
    let difference = projline * projline - tangent;
    if difference < 0.0 {
        return false;
    }
    let fraction = (-projline - difference.sqrt()) / (2.0 * linesq);
    (0.0..=1.0).contains(&fraction)
}

pub fn process_sphere_sphere(
    first: &CColSphere,
    second: &CColSphere,
    point: &mut CColPoint,
    mindistsq: &mut f32,
) -> bool {
    let mut delta = sub_col_vec(first.center, second.center);
    let mut distance = magnitude_col_vec(delta) - second.radius;
    let depth = first.radius - distance;
    if distance < 0.0 {
        distance = 0.0;
    }
    if distance * distance < *mindistsq && distance < first.radius {
        delta = normalize_col_vec(delta);
        point.point = sub_col_vec(first.center, mul_col_vec(delta, distance));
        point.normal = delta;
        point.set_surfaces(first.surface, first.piece, second.surface, second.piece);
        point.depth = depth;
        *mindistsq = distance * distance;
        return true;
    }
    false
}

pub fn process_sphere_box(
    sphere: &CColSphere,
    box_: &CColBox,
    point: &mut CColPoint,
    mindistsq: &mut f32,
) -> bool {
    if !test_sphere_box(sphere, box_) {
        return false;
    }
    let x_position = if sphere.center.x < box_.min.x {
        1
    } else if sphere.center.x > box_.max.x {
        2
    } else {
        0
    };
    let y_position = if sphere.center.y < box_.min.y {
        1
    } else if sphere.center.y > box_.max.y {
        2
    } else {
        0
    };
    let z_position = if sphere.center.z < box_.min.z {
        1
    } else if sphere.center.z > box_.max.z {
        2
    } else {
        0
    };

    if x_position == 0 && y_position == 0 && z_position == 0 {
        let centre = mul_col_vec(add_col_vec(box_.min, box_.max), 0.5);
        let delta = sub_col_vec(sphere.center, centre);
        let length_squared = dot_col_vec(delta, delta);
        if length_squared < *mindistsq {
            point.normal = mul_col_vec(delta, 1.0 / length_squared.sqrt());
            point.point = sub_col_vec(sphere.center, point.normal);
            point.set_surfaces(sphere.surface, sphere.piece, box_.surface, box_.piece);
            let dx = if delta.x > 0.0 {
                box_.max.x - sphere.center.x
            } else {
                sphere.center.x - box_.min.x
            };
            let dy = if delta.y > 0.0 {
                box_.max.y - sphere.center.y
            } else {
                sphere.center.y - box_.min.y
            };
            let dz = if delta.z > 0.0 {
                box_.max.z - sphere.center.z
            } else {
                sphere.center.z - box_.min.z
            };
            point.depth = if dx > dy && dx > dz {
                dx
            } else if dy > dz {
                dy
            } else {
                dz
            };
            return true;
        }
    } else {
        let closest = CColVector::new(
            if x_position == 1 {
                box_.min.x
            } else if x_position == 2 {
                box_.max.x
            } else {
                sphere.center.x
            },
            if y_position == 1 {
                box_.min.y
            } else if y_position == 2 {
                box_.max.y
            } else {
                sphere.center.y
            },
            if z_position == 1 {
                box_.min.z
            } else if z_position == 2 {
                box_.max.z
            } else {
                sphere.center.z
            },
        );
        let delta = sub_col_vec(sphere.center, closest);
        let length_squared = dot_col_vec(delta, delta);
        if length_squared < *mindistsq {
            let length = length_squared.sqrt();
            point.point = closest;
            point.normal = mul_col_vec(delta, 1.0 / length);
            point.set_surfaces(sphere.surface, sphere.piece, box_.surface, box_.piece);
            point.depth = sphere.radius - length;
            *mindistsq = length_squared;
            return true;
        }
    }
    false
}

pub fn test_line_box(line: &CColLine, box_: &CColBox) -> bool {
    if point_inside_box(line.p0, box_) || point_inside_box(line.p1, box_) {
        return true;
    }

    if (box_.min.x - line.p1.x) * (box_.min.x - line.p0.x) < 0.0 {
        let t = (box_.min.x - line.p0.x) / (line.p1.x - line.p0.x);
        let y = line.p0.y + (line.p1.y - line.p0.y) * t;
        if y > box_.min.y && y < box_.max.y {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z {
                return true;
            }
        }
    }

    if (line.p1.x - box_.max.x) * (line.p0.x - box_.max.x) < 0.0 {
        let t = (line.p0.x - box_.max.x) / (line.p0.x - line.p1.x);
        let y = line.p0.y + (line.p1.y - line.p0.y) * t;
        if y > box_.min.y && y < box_.max.y {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z {
                return true;
            }
        }
    }

    if (box_.min.y - line.p0.y) * (box_.min.y - line.p1.y) < 0.0 {
        let t = (box_.min.y - line.p0.y) / (line.p1.y - line.p0.y);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z {
                return true;
            }
        }
    }

    if (line.p0.y - box_.max.y) * (line.p1.y - box_.max.y) < 0.0 {
        let t = (line.p0.y - box_.max.y) / (line.p0.y - line.p1.y);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z {
                return true;
            }
        }
    }

    if (box_.min.z - line.p0.z) * (box_.min.z - line.p1.z) < 0.0 {
        let t = (box_.min.z - line.p0.z) / (line.p1.z - line.p0.z);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let y = line.p0.y + (line.p1.y - line.p0.y) * t;
            if y > box_.min.y && y < box_.max.y {
                return true;
            }
        }
    }

    if (line.p0.z - box_.max.z) * (line.p1.z - box_.max.z) < 0.0 {
        let t = (line.p0.z - box_.max.z) / (line.p0.z - line.p1.z);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let y = line.p0.y + (line.p1.y - line.p0.y) * t;
            if y > box_.min.y && y < box_.max.y {
                return true;
            }
        }
    }

    false
}

pub fn process_line_box(
    line: &CColLine,
    box_: &CColBox,
    point: &mut CColPoint,
    mindist: &mut f32,
) -> bool {
    let mut mint = 1.0;
    let mut hit_point = CColVector::new(0.0, 0.0, 0.0);
    let mut normal = CColVector::new(0.0, 0.0, 0.0);

    if (box_.min.x - line.p1.x) * (box_.min.x - line.p0.x) < 0.0 {
        let t = (box_.min.x - line.p0.x) / (line.p1.x - line.p0.x);
        let y = line.p0.y + (line.p1.y - line.p0.y) * t;
        if y > box_.min.y && y < box_.max.y {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z && t < mint {
                mint = t;
                hit_point = CColVector::new(box_.min.x, y, z);
                normal = CColVector::new(-1.0, 0.0, 0.0);
            }
        }
    }

    if (line.p1.x - box_.max.x) * (line.p0.x - box_.max.x) < 0.0 {
        let t = (line.p0.x - box_.max.x) / (line.p0.x - line.p1.x);
        let y = line.p0.y + (line.p1.y - line.p0.y) * t;
        if y > box_.min.y && y < box_.max.y {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z && t < mint {
                mint = t;
                hit_point = CColVector::new(box_.max.x, y, z);
                normal = CColVector::new(1.0, 0.0, 0.0);
            }
        }
    }

    if (box_.min.y - line.p0.y) * (box_.min.y - line.p1.y) < 0.0 {
        let t = (box_.min.y - line.p0.y) / (line.p1.y - line.p0.y);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z && t < mint {
                mint = t;
                hit_point = CColVector::new(x, box_.min.y, z);
                normal = CColVector::new(0.0, -1.0, 0.0);
            }
        }
    }

    if (line.p0.y - box_.max.y) * (line.p1.y - box_.max.y) < 0.0 {
        let t = (line.p0.y - box_.max.y) / (line.p0.y - line.p1.y);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let z = line.p0.z + (line.p1.z - line.p0.z) * t;
            if z > box_.min.z && z < box_.max.z && t < mint {
                mint = t;
                hit_point = CColVector::new(x, box_.max.y, z);
                normal = CColVector::new(0.0, 1.0, 0.0);
            }
        }
    }

    if (box_.min.z - line.p0.z) * (box_.min.z - line.p1.z) < 0.0 {
        let t = (box_.min.z - line.p0.z) / (line.p1.z - line.p0.z);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let y = line.p0.y + (line.p1.y - line.p0.y) * t;
            if y > box_.min.y && y < box_.max.y && t < mint {
                mint = t;
                hit_point = CColVector::new(x, y, box_.min.z);
                normal = CColVector::new(0.0, 0.0, -1.0);
            }
        }
    }

    if (line.p0.z - box_.max.z) * (line.p1.z - box_.max.z) < 0.0 {
        let t = (line.p0.z - box_.max.z) / (line.p0.z - line.p1.z);
        let x = line.p0.x + (line.p1.x - line.p0.x) * t;
        if x > box_.min.x && x < box_.max.x {
            let y = line.p0.y + (line.p1.y - line.p0.y) * t;
            if y > box_.min.y && y < box_.max.y && t < mint {
                mint = t;
                hit_point = CColVector::new(x, y, box_.max.z);
                normal = CColVector::new(0.0, 0.0, 1.0);
            }
        }
    }

    if mint >= *mindist {
        return false;
    }

    point.point = hit_point;
    point.normal = normal;
    point.set_surfaces(0, 0, box_.surface, box_.piece);
    *mindist = mint;
    true
}

pub fn process_line_sphere(
    line: &CColLine,
    sphere: &CColSphere,
    point: &mut CColPoint,
    mindist: &mut f32,
) -> bool {
    let v01 = sub_col_vec(line.p1, line.p0);
    let v0c = sub_col_vec(sphere.center, line.p0);
    let linesq = dot_col_vec(v01, v01);
    let projline = dot_col_vec(v01, v0c);
    let tansq = (dot_col_vec(v0c, v0c) - sphere.radius * sphere.radius) * linesq;
    let diffsq = projline * projline - tansq;
    if diffsq < 0.0 {
        return false;
    }
    let t = (projline - diffsq.sqrt()) / linesq;
    if t < 0.0 || t > 1.0 || t >= *mindist {
        return false;
    }

    point.point = add_col_vec(line.p0, mul_col_vec(v01, t));
    point.normal = normalize_col_vec(sub_col_vec(point.point, sphere.center));
    point.set_surfaces(0, 0, sphere.surface, sphere.piece);
    *mindist = t;
    true
}

pub fn process_line_triangle(
    line: &CColLine,
    verts: &[CompressedVector],
    tri: &CColTriangle,
    plane: &CColTrianglePlane,
    point: &mut CColPoint,
    mindist: &mut f32,
) -> bool {
    if plane.calc_point(line.p0) * plane.calc_point(line.p1) > 0.0 {
        return false;
    }

    let p0dist = dot_col_vec(sub_col_vec(line.p1, line.p0), plane.normal);
    if p0dist == 0.0 {
        return false;
    }

    let t = -plane.calc_point(line.p0) / p0dist;
    if t >= *mindist {
        return false;
    }

    let p = add_col_vec(line.p0, mul_col_vec(sub_col_vec(line.p1, line.p0), t));
    let Some(va) = verts.get(tri.a as usize).map(CompressedVector::get) else {
        return false;
    };
    let Some(vb) = verts.get(tri.b as usize).map(CompressedVector::get) else {
        return false;
    };
    let Some(vc) = verts.get(tri.c as usize).map(CompressedVector::get) else {
        return false;
    };

    let (vec1, vec2, vec3, vect) = match plane.dir {
        CColTrianglePlaneDirection::XPos => ([va.y, va.z], [vc.y, vc.z], [vb.y, vb.z], [p.y, p.z]),
        CColTrianglePlaneDirection::XNeg => ([va.y, va.z], [vb.y, vb.z], [vc.y, vc.z], [p.y, p.z]),
        CColTrianglePlaneDirection::YPos => ([va.z, va.x], [vc.z, vc.x], [vb.z, vb.x], [p.z, p.x]),
        CColTrianglePlaneDirection::YNeg => ([va.z, va.x], [vb.z, vb.x], [vc.z, vc.x], [p.z, p.x]),
        CColTrianglePlaneDirection::ZPos => ([va.x, va.y], [vc.x, vc.y], [vb.x, vb.y], [p.x, p.y]),
        CColTrianglePlaneDirection::ZNeg => ([va.x, va.y], [vb.x, vb.y], [vc.x, vc.y], [p.x, p.y]),
    };

    let min_x = vec1[0].min(vec2[0]).min(vec3[0]);
    let max_x = vec1[0].max(vec2[0]).max(vec3[0]);
    let min_y = vec1[1].min(vec2[1]).min(vec3[1]);
    let max_y = vec1[1].max(vec2[1]).max(vec3[1]);
    if vect[0] < min_x || vect[0] > max_x || vect[1] < min_y || vect[1] > max_y {
        return false;
    }

    if cross_2d(sub_2d(vec2, vec1), sub_2d(vect, vec1)) < 0.0 {
        return false;
    }
    if cross_2d(sub_2d(vec3, vec1), sub_2d(vect, vec1)) > 0.0 {
        return false;
    }
    if cross_2d(sub_2d(vec3, vec2), sub_2d(vect, vec2)) < 0.0 {
        return false;
    }
    if t >= *mindist {
        return false;
    }

    point.point = p;
    point.normal = plane.normal;
    point.set_surfaces(0, 0, tri.surface, 0);
    *mindist = t;
    true
}

pub fn dist_to_line(start: CColVector, end: CColVector, point: CColVector) -> f32 {
    dist_to_line_with_closest(start, end, point).0
}

pub fn dist_to_line_with_closest(
    start: CColVector,
    end: CColVector,
    point: CColVector,
) -> (f32, CColVector) {
    let segment = sub_col_vec(end, start);
    let length_squared = dot_col_vec(segment, segment);
    let projection = dot_col_vec(sub_col_vec(point, start), segment);
    let closest = if projection <= 0.0 {
        start
    } else if projection >= length_squared {
        end
    } else {
        add_col_vec(start, mul_col_vec(segment, projection / length_squared))
    };
    (magnitude_col_vec(sub_col_vec(point, closest)), closest)
}

pub fn test_sphere_triangle(
    sphere: &CColSphere,
    vertices: &[CompressedVector],
    triangle: &CColTriangle,
    plane: &CColTrianglePlane,
) -> bool {
    sphere_triangle_closest(sphere, vertices, triangle, plane)
        .is_some_and(|(distance, _)| distance < sphere.radius)
}

pub fn process_sphere_triangle(
    sphere: &CColSphere,
    vertices: &[CompressedVector],
    triangle: &CColTriangle,
    plane: &CColTrianglePlane,
    point: &mut CColPoint,
    mindistsq: &mut f32,
) -> bool {
    let plane_distance = plane.calc_point(sphere.center);
    let plane_distance_squared = plane_distance * plane_distance;
    if plane_distance.abs() > sphere.radius || plane_distance_squared > *mindistsq {
        return false;
    }
    let Some((distance, closest)) = sphere_triangle_closest(sphere, vertices, triangle, plane)
    else {
        return false;
    };
    if distance >= sphere.radius || distance * distance >= *mindistsq {
        return false;
    }
    point.point = closest;
    point.normal = normalize_col_vec(sub_col_vec(sphere.center, closest));
    point.set_surfaces(sphere.surface, sphere.piece, triangle.surface, 0);
    point.depth = sphere.radius - distance;
    *mindistsq = distance * distance;
    true
}

fn sphere_triangle_closest(
    sphere: &CColSphere,
    vertices: &[CompressedVector],
    triangle: &CColTriangle,
    plane: &CColTrianglePlane,
) -> Option<(f32, CColVector)> {
    let plane_distance = plane.calc_point(sphere.center);
    if plane_distance.abs() > sphere.radius {
        return None;
    }
    let va = vertices.get(triangle.a as usize)?.get();
    let vb = vertices.get(triangle.b as usize)?.get();
    let vc = vertices.get(triangle.c as usize)?.get();
    let edge = sub_col_vec(vb, va);
    let edge_length = magnitude_col_vec(edge);
    let axis_2 = mul_col_vec(edge, 1.0 / edge_length);
    let axis_1 = cross_col_vec(axis_2, plane.normal);
    let va_to_c = sub_col_vec(vc, va);
    let va_to_sphere = sub_col_vec(sphere.center, va);
    let b = [0.0, edge_length];
    let c = [dot_col_vec(axis_1, va_to_c), dot_col_vec(axis_2, va_to_c)];
    let s = [
        dot_col_vec(axis_1, va_to_sphere),
        dot_col_vec(axis_2, va_to_sphere),
    ];
    let inside_ab = cross_2d(s, b) >= 0.0;
    let inside_ac = cross_2d(c, s) >= 0.0;
    let inside_bc = cross_2d(sub_2d(s, b), sub_2d(c, b)) >= 0.0;
    match inside_ab as u8 + inside_ac as u8 + inside_bc as u8 {
        1 => {
            let closest = if inside_ab {
                vc
            } else if inside_ac {
                vb
            } else {
                va
            };
            Some((
                magnitude_col_vec(sub_col_vec(sphere.center, closest)),
                closest,
            ))
        }
        2 => {
            let (start, end) = if !inside_ab {
                (va, vb)
            } else if !inside_ac {
                (va, vc)
            } else {
                (vb, vc)
            };
            Some(dist_to_line_with_closest(start, end, sphere.center))
        }
        3 => {
            let closest = sub_col_vec(sphere.center, mul_col_vec(plane.normal, plane_distance));
            Some((plane_distance.abs(), closest))
        }
        _ => None,
    }
}

fn point_inside_box(point: CColVector, box_: &CColBox) -> bool {
    point.x > box_.min.x
        && point.x < box_.max.x
        && point.y > box_.min.y
        && point.y < box_.max.y
        && point.z > box_.min.z
        && point.z < box_.max.z
}

fn add_col_vec(a: CColVector, b: CColVector) -> CColVector {
    CColVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub_col_vec(a: CColVector, b: CColVector) -> CColVector {
    CColVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn mul_col_vec(value: CColVector, scale: f32) -> CColVector {
    CColVector::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot_col_vec(a: CColVector, b: CColVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn magnitude_col_vec(value: CColVector) -> f32 {
    dot_col_vec(value, value).sqrt()
}

fn normalize_col_vec(value: CColVector) -> CColVector {
    let len = magnitude_col_vec(value);
    if len == 0.0 {
        return value;
    }
    mul_col_vec(value, 1.0 / len)
}

fn cross_col_vec(a: CColVector, b: CColVector) -> CColVector {
    CColVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn sub_2d(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn cross_2d(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressedVector {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl CompressedVector {
    pub fn set(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: (x * 128.0) as i16,
            y: (y * 128.0) as i16,
            z: (z * 128.0) as i16,
        }
    }

    pub fn get(&self) -> CColVector {
        CColVector::new(
            self.x as f32 / 128.0,
            self.y as f32 / 128.0,
            self.z as f32 / 128.0,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CColTriangle {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub surface: u8,
    pub piece: u8,
}

impl CColTriangle {
    pub const fn new(a: u16, b: u16, c: u16, surface: u8, piece: u8) -> Self {
        Self {
            a,
            b,
            c,
            surface,
            piece,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CColTrianglePlaneDirection {
    XPos,
    XNeg,
    YPos,
    YNeg,
    ZPos,
    ZNeg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CColTrianglePlane {
    pub normal: CColVector,
    pub dist: f32,
    pub dir: CColTrianglePlaneDirection,
}

impl CColTrianglePlane {
    pub fn set(va: CColVector, vb: CColVector, vc: CColVector) -> Self {
        let normal = normalize_col_vec(cross_col_vec(sub_col_vec(vc, va), sub_col_vec(vb, va)));
        let dist = dot_col_vec(normal, va);
        let abs_normal = CColVector::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        let dir = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
            if normal.x < 0.0 {
                CColTrianglePlaneDirection::XNeg
            } else {
                CColTrianglePlaneDirection::XPos
            }
        } else if abs_normal.y > abs_normal.z {
            if normal.y < 0.0 {
                CColTrianglePlaneDirection::YNeg
            } else {
                CColTrianglePlaneDirection::YPos
            }
        } else if normal.z < 0.0 {
            CColTrianglePlaneDirection::ZNeg
        } else {
            CColTrianglePlaneDirection::ZPos
        };

        Self { normal, dist, dir }
    }

    pub fn from_triangle(verts: &[CompressedVector], tri: &CColTriangle) -> Option<Self> {
        Some(Self::set(
            verts.get(tri.a as usize)?.get(),
            verts.get(tri.b as usize)?.get(),
            verts.get(tri.c as usize)?.get(),
        ))
    }

    pub fn calc_point(&self, point: CColVector) -> f32 {
        dot_col_vec(self.normal, point) - self.dist
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CColModel {
    pub bounding_sphere: CColSphere,
    pub bounding_box: CColBox,
    pub level: i32,
    pub owns_collision_volumes: bool,
    pub spheres: Vec<CColSphere>,
    pub lines: Vec<CColLine>,
    pub boxes: Vec<CColBox>,
    pub vertices: Vec<CompressedVector>,
    pub triangles: Vec<CColTriangle>,
    /// C++ `CCollision::CalculateTrianglePlanes` caches one plane per
    /// triangle on the collision model. Keeping the cache beside the parsed
    /// COL avoids rebuilding normals in every vehicle/world collision query.
    pub triangle_planes: Vec<CColTrianglePlane>,
    /// Conservative local-space AABBs for the same triangles. C++ first
    /// rejects triangles against the transformed vehicle bound; this cache
    /// gives the Rust narrow phase an equivalent cheap rejection before the
    /// closest-point math.
    pub triangle_bounds: Vec<(CColVector, CColVector)>,
    pub triangle_spatial_index: TriangleSpatialIndex,
}

impl CColModel {
    pub fn new(
        bounding_sphere: CColSphere,
        bounding_box: CColBox,
        spheres: Vec<CColSphere>,
    ) -> Self {
        Self {
            bounding_sphere,
            bounding_box,
            level: LEVEL_GENERIC,
            owns_collision_volumes: false,
            spheres,
            lines: Vec::new(),
            boxes: Vec::new(),
            vertices: Vec::new(),
            triangles: Vec::new(),
            triangle_planes: Vec::new(),
            triangle_bounds: Vec::new(),
            triangle_spatial_index: TriangleSpatialIndex::default(),
        }
    }

    /// Return the resident plane generated by the C++-equivalent cache. The
    /// fallback keeps hand-authored/test models valid when they were built
    /// before triangle planes were populated.
    pub fn triangle_plane(&self, index: usize) -> Option<CColTrianglePlane> {
        self.triangle_planes.get(index).copied().or_else(|| {
            self.triangles
                .get(index)
                .and_then(|tri| CColTrianglePlane::from_triangle(&self.vertices, tri))
        })
    }

    pub fn triangle_maybe_overlaps_sphere(
        &self,
        index: usize,
        center: CColVector,
        radius: f32,
    ) -> bool {
        let Some((min, max)) = self.triangle_bounds.get(index).copied().or_else(|| {
            let triangle = self.triangles.get(index)?;
            let a = self.vertices.get(triangle.a as usize)?.get();
            let b = self.vertices.get(triangle.b as usize)?.get();
            let c = self.vertices.get(triangle.c as usize)?.get();
            Some((
                CColVector::new(
                    a.x.min(b.x).min(c.x),
                    a.y.min(b.y).min(c.y),
                    a.z.min(b.z).min(c.z),
                ),
                CColVector::new(
                    a.x.max(b.x).max(c.x),
                    a.y.max(b.y).max(c.y),
                    a.z.max(b.z).max(c.z),
                ),
            ))
        }) else {
            return false;
        };
        let dx = if center.x < min.x {
            min.x - center.x
        } else if center.x > max.x {
            center.x - max.x
        } else {
            0.0
        };
        let dy = if center.y < min.y {
            min.y - center.y
        } else if center.y > max.y {
            center.y - max.y
        } else {
            0.0
        };
        let dz = if center.z < min.z {
            min.z - center.z
        } else if center.z > max.z {
            center.z - max.z
        } else {
            0.0
        };
        dx * dx + dy * dy + dz * dz <= radius * radius
    }

    pub fn process_line_of_sight_primitives(
        &self,
        line: &CColLine,
        point: &mut CColPoint,
        mindist: &mut f32,
    ) -> bool {
        if !test_line_box(line, &self.bounding_box) {
            return false;
        }

        let start_mindist = *mindist;
        for sphere in &self.spheres {
            process_line_sphere(line, sphere, point, mindist);
        }
        for box_ in &self.boxes {
            process_line_box(line, box_, point, mindist);
        }
        let line_min_x = line.p0.x.min(line.p1.x);
        let line_max_x = line.p0.x.max(line.p1.x);
        let line_min_y = line.p0.y.min(line.p1.y);
        let line_max_y = line.p0.y.max(line.p1.y);
        let line_min_z = line.p0.z.min(line.p1.z);
        let line_max_z = line.p0.z.max(line.p1.z);

        for (index, tri) in self.triangles.iter().enumerate() {
            if let Some(&(bound_min, bound_max)) = self.triangle_bounds.get(index) {
                if line_min_x > bound_max.x
                    || line_max_x < bound_min.x
                    || line_min_y > bound_max.y
                    || line_max_y < bound_min.y
                    || line_min_z > bound_max.z
                    || line_max_z < bound_min.z
                {
                    continue;
                }
            }
            if let Some(plane) = self.triangle_plane(index) {
                process_line_triangle(line, &self.vertices, tri, &plane, point, mindist);
            }
        }
        *mindist < start_mindist
    }

    pub fn remove_collision_volumes(&mut self) {
        self.spheres.clear();
        self.lines.clear();
        self.boxes.clear();
        self.vertices.clear();
        self.triangles.clear();
        self.triangle_planes.clear();
        self.triangle_bounds.clear();
        self.triangle_spatial_index = TriangleSpatialIndex::default();
        self.owns_collision_volumes = false;
    }
}
