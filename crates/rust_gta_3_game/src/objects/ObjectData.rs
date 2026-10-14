//! re3 reference: references/re-GTA/src/objects/ObjectData.cpp
//! re3 reference: references/re-GTA/src/objects/ObjectData.h
//!
//! Object physics data loaded from `DATA\OBJECT.DAT`.

pub const GRAVITY: f32 = 0.008;

#[derive(Debug, Clone, PartialEq)]
pub struct CObjectInfo {
    pub mass: f32,
    pub turn_mass: f32,
    pub air_resistance: f32,
    pub elasticity: f32,
    pub buoyancy: f32,
    pub uproot_limit: f32,
    pub collision_damage_multiplier: f32,
    pub collision_damage_effect: u8,
    pub special_collision_response_cases: u8,
    pub camera_to_avoid_this_object: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CObjectData {
    pub object_info: Vec<CObjectInfo>,
    pub bound_models: Vec<(usize, String)>,
    pub missing_models: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDataRecord {
    pub name: String,
    pub info: CObjectInfo,
}

impl CObjectData {
    pub fn parse_records(text: &str) -> Vec<ObjectDataRecord> {
        let mut records = Vec::new();
        for raw in object_data_lines(text) {
            let line = raw.replace(',', " ");
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 {
                continue;
            }

            let Some(percent_submerged) = parts.get(5).and_then(|part| part.parse::<f32>().ok())
            else {
                continue;
            };
            let Some(mass) = parts.get(1).and_then(|part| part.parse::<f32>().ok()) else {
                continue;
            };

            records.push(ObjectDataRecord {
                name: parts[0].to_owned(),
                info: CObjectInfo {
                    mass,
                    turn_mass: parts[2].parse().unwrap_or(0.0),
                    air_resistance: parts[3].parse().unwrap_or(0.0),
                    elasticity: parts[4].parse().unwrap_or(0.0),
                    buoyancy: 100.0 / percent_submerged * GRAVITY * mass,
                    uproot_limit: parts[6].parse().unwrap_or(0.0),
                    collision_damage_multiplier: parts[7].parse().unwrap_or(0.0),
                    collision_damage_effect: parts[8].parse().unwrap_or(0),
                    special_collision_response_cases: parts[9].parse().unwrap_or(0),
                    camera_to_avoid_this_object: parts[10].parse::<i32>().unwrap_or(0) != 0,
                },
            });
        }
        records
    }
}

fn object_data_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('*') {
            break;
        }
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        lines.push(line.to_owned());
    }
    lines
}
