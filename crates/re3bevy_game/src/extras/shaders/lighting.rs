//! re3 reference: references/re-GTA/src/extras/shaders/lighting.h
//!
//! CPU-side equivalents of the small HLSL lighting helpers.

use re3bevy_core::core::Coordinates::CVector;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub color: [f32; 4],
    pub position: [f32; 4],
    pub direction: [f32; 4],
}

pub fn do_dir_light(light: Light, normal: CVector) -> [f32; 3] {
    let direction = CVector::new(
        -light.direction[0],
        -light.direction[1],
        -light.direction[2],
    );
    let amount = dot(normal, direction).max(0.0);
    mul3(light.color, amount)
}

pub fn do_point_light(light: Light, vertex: CVector, normal: CVector) -> [f32; 3] {
    let dir = CVector::new(
        vertex.x - light.position[0],
        vertex.y - light.position[1],
        vertex.z - light.position[2],
    );
    let dist = dir.magnitude();
    if dist <= f32::EPSILON {
        return [0.0; 3];
    }
    let atten = (1.0 - dist / light.color[3]).max(0.0);
    let l = dot(
        normal,
        CVector::new(-dir.x / dist, -dir.y / dist, -dir.z / dist),
    )
    .max(0.0);
    mul3(light.color, l * atten)
}

pub fn do_spot_light(light: Light, vertex: CVector, normal: CVector) -> [f32; 3] {
    let mut dir = CVector::new(
        vertex.x - light.position[0],
        vertex.y - light.position[1],
        vertex.z - light.position[2],
    );
    let dist = dir.magnitude();
    if dist <= f32::EPSILON {
        return [0.0; 3];
    }
    let atten = (1.0 - dist / light.color[3]).max(0.0);
    dir = CVector::new(dir.x / dist, dir.y / dist, dir.z / dist);
    let mut l = dot(normal, CVector::new(-dir.x, -dir.y, -dir.z)).max(0.0);
    let pcos = dir.x * light.direction[0] + dir.y * light.direction[1] + dir.z * light.direction[2];
    let ccos = -light.position[3];
    let falloff = (pcos - ccos) / (1.0 - ccos);
    if falloff < 0.0 {
        l = 0.0;
    }
    l *= falloff.max(light.direction[3]);
    mul3(light.color, l * atten)
}

fn dot(a: CVector, b: CVector) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn mul3(color: [f32; 4], amount: f32) -> [f32; 3] {
    [color[0] * amount, color[1] * amount, color[2] * amount]
}
