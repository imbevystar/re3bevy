//! re3 reference: references/re-GTA/src/math/Matrix.cpp
//! re3 reference: references/re-GTA/src/math/Matrix.h
//!
//! Matrix basis used by `CPlaceable`.

use re3bevy_core::core::Coordinates::CVector;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CMatrix {
    pub right: CVector,
    pub forward: CVector,
    pub up: CVector,
    pub position: CVector,
    pub has_rw_matrix: bool,
}

impl CMatrix {
    pub fn new() -> Self {
        Self {
            right: CVector::ZERO,
            forward: CVector::ZERO,
            up: CVector::ZERO,
            position: CVector::ZERO,
            has_rw_matrix: false,
        }
    }

    pub fn unity() -> Self {
        let mut matrix = Self::new();
        matrix.set_scale(1.0);
        matrix
    }

    pub fn set_unity(&mut self) {
        self.right = CVector::new(1.0, 0.0, 0.0);
        self.forward = CVector::new(0.0, 1.0, 0.0);
        self.up = CVector::new(0.0, 0.0, 1.0);
        self.position = CVector::ZERO;
    }

    pub fn reset_orientation(&mut self) {
        self.right = CVector::new(1.0, 0.0, 0.0);
        self.forward = CVector::new(0.0, 1.0, 0.0);
        self.up = CVector::new(0.0, 0.0, 1.0);
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.right = CVector::new(scale, 0.0, 0.0);
        self.forward = CVector::new(0.0, scale, 0.0);
        self.up = CVector::new(0.0, 0.0, scale);
        self.position = CVector::ZERO;
    }

    pub fn set_translate(&mut self, x: f32, y: f32, z: f32) {
        self.reset_orientation();
        self.position = CVector::new(x, y, z);
    }

    pub fn translate(&mut self, vector: CVector) {
        self.position = add_vec3(self.position, vector);
    }

    pub fn set_rotate_x_only(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        self.right = CVector::new(1.0, 0.0, 0.0);
        self.forward = CVector::new(0.0, c, s);
        self.up = CVector::new(0.0, -s, c);
    }

    pub fn set_rotate_y_only(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        self.right = CVector::new(c, 0.0, -s);
        self.forward = CVector::new(0.0, 1.0, 0.0);
        self.up = CVector::new(s, 0.0, c);
    }

    pub fn set_rotate_z_only(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        self.right = CVector::new(c, s, 0.0);
        self.forward = CVector::new(-s, c, 0.0);
        self.up = CVector::new(0.0, 0.0, 1.0);
    }

    pub fn set_rotate_x(&mut self, angle: f32) {
        self.set_rotate_x_only(angle);
        self.position = CVector::ZERO;
    }

    pub fn set_rotate_y(&mut self, angle: f32) {
        self.set_rotate_y_only(angle);
        self.position = CVector::ZERO;
    }

    pub fn set_rotate_z(&mut self, angle: f32) {
        self.set_rotate_z_only(angle);
        self.position = CVector::ZERO;
    }

    pub fn set_rotate(&mut self, x_angle: f32, y_angle: f32, z_angle: f32) {
        let c_x = x_angle.cos();
        let s_x = x_angle.sin();
        let c_y = y_angle.cos();
        let s_y = y_angle.sin();
        let c_z = z_angle.cos();
        let s_z = z_angle.sin();

        self.right = CVector::new(
            c_z * c_y - (s_z * s_x) * s_y,
            (c_z * s_x) * s_y + s_z * c_y,
            -c_x * s_y,
        );
        self.forward = CVector::new(-s_z * c_x, c_z * c_x, s_x);
        self.up = CVector::new(
            (s_z * s_x) * c_y + c_z * s_y,
            s_z * s_y - (c_z * s_x) * c_y,
            c_x * c_y,
        );
        self.position = CVector::ZERO;
    }

    pub fn transform_point(&self, vector: CVector) -> CVector {
        add_vec3(self.multiply3x3(vector), self.position)
    }

    pub fn multiply3x3(&self, vector: CVector) -> CVector {
        CVector::new(
            self.right.x * vector.x + self.forward.x * vector.y + self.up.x * vector.z,
            self.right.y * vector.x + self.forward.y * vector.y + self.up.y * vector.z,
            self.right.z * vector.x + self.forward.z * vector.y + self.up.z * vector.z,
        )
    }

    pub fn multiply_inverse_point(&self, vector: CVector) -> CVector {
        let v = sub_vec3(vector, self.position);
        CVector::new(
            self.right.x * v.x + self.right.y * v.y + self.right.z * v.z,
            self.forward.x * v.x + self.forward.y * v.y + self.forward.z * v.z,
            self.up.x * v.x + self.up.y * v.y + self.up.z * v.z,
        )
    }

    pub fn set_translate_only(&mut self, position: CVector) {
        self.position = position;
    }

    pub fn scale(&mut self, scale: f32) {
        self.right = scale_vec3(self.right, scale);
        self.forward = scale_vec3(self.forward, scale);
        self.up = scale_vec3(self.up, scale);
    }

    pub fn rotate_x(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        rotate_pair(&mut self.right.y, &mut self.right.z, c, s, false);
        rotate_pair(&mut self.forward.y, &mut self.forward.z, c, s, false);
        rotate_pair(&mut self.up.y, &mut self.up.z, c, s, false);
        rotate_pair(&mut self.position.y, &mut self.position.z, c, s, false);
    }

    pub fn rotate_y(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        rotate_pair(&mut self.right.x, &mut self.right.z, c, s, true);
        rotate_pair(&mut self.forward.x, &mut self.forward.z, c, s, true);
        rotate_pair(&mut self.up.x, &mut self.up.z, c, s, true);
        rotate_pair(&mut self.position.x, &mut self.position.z, c, s, true);
    }

    pub fn rotate_z(&mut self, angle: f32) {
        let c = angle.cos();
        let s = angle.sin();
        rotate_pair(&mut self.right.x, &mut self.right.y, c, s, false);
        rotate_pair(&mut self.forward.x, &mut self.forward.y, c, s, false);
        rotate_pair(&mut self.up.x, &mut self.up.y, c, s, false);
        rotate_pair(&mut self.position.x, &mut self.position.y, c, s, false);
    }

    pub fn rotate(&mut self, x: f32, y: f32, z: f32) {
        let mut rotation = Self::new();
        rotation.set_rotate(x, y, z);
        self.right = rotation.multiply3x3(self.right);
        self.forward = rotation.multiply3x3(self.forward);
        self.up = rotation.multiply3x3(self.up);
        self.position = rotation.multiply3x3(self.position);
    }

    pub fn add_assign_matrix(&mut self, rhs: &Self) {
        self.right = add_vec3(self.right, rhs.right);
        self.forward = add_vec3(self.forward, rhs.forward);
        self.up = add_vec3(self.up, rhs.up);
        self.position = add_vec3(self.position, rhs.position);
    }

    pub fn multiply(&self, rhs: &Self) -> Self {
        Self {
            right: self.multiply3x3(rhs.right),
            forward: self.multiply3x3(rhs.forward),
            up: self.multiply3x3(rhs.up),
            position: self.transform_point(rhs.position),
            has_rw_matrix: false,
        }
    }

    pub fn invert(&self) -> Self {
        let right = CVector::new(self.right.x, self.forward.x, self.up.x);
        let forward = CVector::new(self.right.y, self.forward.y, self.up.y);
        let up = CVector::new(self.right.z, self.forward.z, self.up.z);
        let position = CVector::new(
            -(self.right.x * self.position.x
                + self.right.y * self.position.y
                + self.right.z * self.position.z),
            -(self.forward.x * self.position.x
                + self.forward.y * self.position.y
                + self.forward.z * self.position.z),
            -(self.up.x * self.position.x
                + self.up.y * self.position.y
                + self.up.z * self.position.z),
        );
        Self {
            right,
            forward,
            up,
            position,
            has_rw_matrix: false,
        }
    }

    pub fn reorthogonalise(&mut self) {
        self.up = normalise(cross_vec3(self.right, self.forward));
        self.right = normalise(cross_vec3(self.forward, self.up));
        self.forward = cross_vec3(self.up, self.right);
    }
}

impl Default for CMatrix {
    fn default() -> Self {
        Self::unity()
    }
}

fn add_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale_vec3(vector: CVector, scale: f32) -> CVector {
    CVector::new(vector.x * scale, vector.y * scale, vector.z * scale)
}

fn rotate_pair(a: &mut f32, b: &mut f32, c: f32, s: f32, y_axis: bool) {
    let old_a = *a;
    let old_b = *b;
    if y_axis {
        *a = c * old_a + s * old_b;
        *b = c * old_b - s * old_a;
    } else {
        *a = c * old_a - s * old_b;
        *b = c * old_b + s * old_a;
    }
}

fn cross_vec3(a: CVector, b: CVector) -> CVector {
    CVector::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalise(vector: CVector) -> CVector {
    let magnitude = vector.magnitude();
    if magnitude > 0.0 {
        CVector::new(
            vector.x / magnitude,
            vector.y / magnitude,
            vector.z / magnitude,
        )
    } else {
        CVector::new(1.0, 0.0, 0.0)
    }
}
