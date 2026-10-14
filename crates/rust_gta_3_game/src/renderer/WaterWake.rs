//! CBoat's borrowed water-render interface. No duplicate live boat state.
use crate::{
    control::ScriptEntities::ScriptVehicleIdentity,
    core::Pools::CPoolHandle,
    vehicles::Boat::{BoatState, WAKE_LIFETIME},
};
use bevy::prelude::*;

pub(super) type WaterBoats<'w, 's> =
    Query<'w, 's, (Entity, &'static ScriptVehicleIdentity, &'static BoatState)>;

/// FillBoatList iterates descending vehicle pool slots and retains four trails.
/// Only references to queried boats are sorted; actor payloads are never copied.
pub(super) fn select_wake_boats(boats: &WaterBoats) -> [Option<Entity>; 4] {
    let mut candidates: Vec<_> = boats
        .iter()
        .filter(|(_, _, boat)| boat.num_wake_points > 0)
        .collect();
    candidates.sort_unstable_by_key(|(_, identity, _)| {
        std::cmp::Reverse(CPoolHandle(identity.handle).index())
    });
    let mut selected: [Option<(Entity, u16)>; 4] = [None; 4];
    let mut count = 0;
    for (entity, _, boat) in candidates {
        let mut points = boat.num_wake_points;
        if count < selected.len() {
            selected[count] = Some((entity, points));
            count += 1;
        } else {
            let mut replacement = None;
            for (index, entry) in selected.iter().enumerate() {
                if let Some((_, existing)) = entry {
                    if *existing < points {
                        replacement = Some(index);
                        points = *existing;
                    }
                }
            }
            if let Some(index) = replacement {
                selected[index] = Some((entity, boat.num_wake_points));
            }
        }
    }
    selected.map(|entry| entry.map(|(entity, _)| entity))
}

/// Native IsSectorAffectedByWake writes slots 0 and 1 and retains previous
/// entries. Its Y test uses the boat index rather than the trail point index.
/// Retained ECS identities are validated on every read so despawns are safe.
pub(super) fn sector_affected(
    boats: &WaterBoats,
    selected: &[Option<Entity>; 4],
    retained: &mut [Option<Entity>; 4],
    x: f32,
    y: f32,
) -> bool {
    let mut affected = false;
    for (boat_index, entity) in selected.iter().enumerate() {
        let Some(entity) = entity else { break };
        let Ok((_, _, boat)) = boats.get(*entity) else {
            continue;
        };
        for point in 0..boat.num_wake_points as usize {
            let radius = (WAKE_LIFETIME - boat.wake_point_life_time[point]) * 0.05
                + point as f32 * 0.4
                + 16.0;
            if (boat.wake_points[point].x - x).abs() < radius
                && (boat.wake_points[boat_index].y - y).abs() < radius
            {
                retained[usize::from(affected)] = Some(*entity);
                affected = true;
                break;
            }
        }
    }
    affected
}

/// IsVertexAffectedByWake: return the first enclosing trail point's intensity.
pub(super) fn vertex_influence(boat: &BoatState, x: f32, y: f32) -> f32 {
    for point in 0..boat.num_wake_points as usize {
        let age = WAKE_LIFETIME - boat.wake_point_life_time[point];
        let radius = age * 0.05 + point as f32 * 0.4;
        let distance_squared =
            (boat.wake_points[point].x - x).powi(2) + (boat.wake_points[point].y - y).powi(2);
        if distance_squared < radius * radius {
            return 1.0
                - (0.75 * (distance_squared / (radius * radius)).sqrt() + age / WAKE_LIFETIME)
                    .min(1.0);
        }
    }
    0.0
}

pub(super) fn wake_colour(base: [f32; 4], intensity: f32) -> [f32; 4] {
    let bytes = base.map(|channel| (channel * 255.0) as u8);
    let average = (bytes[0] as f32 + bytes[1] as f32 + bytes[2] as f32) / 3.0;
    let strength = 180.0 - (255.0 - average).max(80.0);
    let mut result = base;
    for (channel, multiplier) in [0.6, 1.0, 1.4].into_iter().enumerate() {
        let value = (bytes[channel] as i32 + (strength * multiplier * intensity) as i32).min(255);
        result[channel] = value as u8 as f32 / 255.0;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicles::Boat::CBoat;
    use rust_gta_3_core::core::{Coordinates::CVector, General::CGeneral};

    #[test]
    fn wake_fades_from_first_enclosing_point_and_expires_with_native_control() {
        let mut general = CGeneral::default();
        let mut boat = CBoat::new(0, 0, 0, &mut general, None, None);
        boat.add_wake_point(CVector::ZERO);
        boat.prune_wake_trail(100.0);
        assert!((vertex_influence(&boat.state, 0.0, 0.0) - 0.75).abs() < 0.00001);
        assert!((vertex_influence(&boat.state, 2.5, 0.0) - 0.375).abs() < 0.00001);
        assert_eq!(vertex_influence(&boat.state, 5.0, 0.0), 0.0);
        boat.prune_wake_trail(300.0);
        assert_eq!(vertex_influence(&boat.state, 0.0, 0.0), 0.0);
    }

    #[test]
    fn native_wake_prelight_uses_byte_quantization_and_channel_multipliers() {
        let colour = wake_colour([200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0], 0.5);
        assert_eq!(colour, [230.0 / 255.0, 250.0 / 255.0, 1.0, 1.0]);
    }
}
