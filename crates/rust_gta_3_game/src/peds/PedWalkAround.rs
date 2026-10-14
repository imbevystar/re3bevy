//! CPed::SetDirectionToWalkAroundObject's NPC vehicle corner decisions.
//! Native corner classification and directional hysteresis, without a second route cache.
//! TODO(parity): CPed::SetDirectionToWalkAroundObject non-vehicle pole geometry
//! and native corner probes against live movable CObjects.
//! Vehicle entry retains the selected side and rear-van approach. The caller
//! supplies player move speed without duplicating its component ownership.
use super::*;
use crate::peds::PedLocomotion::PedLocomotion;
use rust_gta_3_core::core::Coordinates::CVector2D;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Occupancy {
    Clear,
    Building,
    Vehicle,
    Object,
}
impl Occupancy {
    fn blocked(self) -> bool {
        self != Self::Clear
    }
    fn blocks_turn(self, opposite_clear: bool) -> bool {
        self == Self::Building || self.blocked() && opposite_clear
    }
}
#[derive(Clone, Copy)]
enum Region {
    Behind,
    Right,
    Left,
    Front,
}
#[derive(Clone, Copy)]
struct Corners {
    top_left: Occupancy,
    top_right: Occupancy,
    bottom_left: Occupancy,
    bottom_right: Occupancy,
}

#[cfg(test)]
pub(super) fn native_vehicle_corner(
    ped: &mut CPed,
    locomotion: &mut PedLocomotion,
    avoidance: &mut crate::peds::PedObstacle::PedObstacleAvoidance,
    start: CVector,
    blocker: &PedVehicleCollider,
    entity: Entity,
    previous_obstacle: Option<Entity>,
    colliders: &[PedVehicleCollider],
    loader: &CFileLoader,
    now: u32,
    general: &mut CGeneral,
) -> Option<CVector> {
    native_vehicle_corner_for_entry(ped, locomotion, avoidance, start, blocker,
        entity, previous_obstacle, colliders, loader, now, general, None, None)
}

pub(super) fn native_vehicle_corner_for_entry(
    ped: &mut CPed,
    locomotion: &mut PedLocomotion,
    avoidance: &mut crate::peds::PedObstacle::PedObstacleAvoidance,
    start: CVector,
    blocker: &PedVehicleCollider,
    entity: Entity,
    previous_obstacle: Option<Entity>,
    colliders: &[PedVehicleCollider],
    loader: &CFileLoader,
    now: u32,
    general: &mut CGeneral,
    entry: Option<(i32, bool)>,
    player_move_speed: Option<f32>,
) -> Option<CVector> {
    if matches!(
        ped.move_state,
        eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL
    ) {
        return None;
    }
    let min = CVector::new(blocker.min.x - 0.35, blocker.min.y - 0.35, 0.0);
    let max = CVector::new(blocker.max.x + 0.35, blocker.max.y + 0.35, 0.0);
    let center = blocker
        .matrix
        .transform_point((blocker.min + blocker.max) * 0.5);
    let heading = |v: CVector| CVector2D::from_vector(v).Heading();
    let center_heading = heading(center - start);
    let object_heading = heading(blocker.matrix.forward);
    let angle = CGeneral::limit_radian_angle(object_heading - center_heading);
    let corner_angle = (max.x - min.x).atan2(max.y - min.y);
    let region = if angle.abs() < corner_angle {
        Region::Behind
    } else if angle.abs() <= std::f32::consts::PI - corner_angle {
        if angle <= 0.0 {
            Region::Right
        } else {
            Region::Left
        }
    } else {
        Region::Front
    };
    let old_rotation_dest = locomotion.rotation_dest;
    let turn = CGeneral::limit_radian_angle(old_rotation_dest - center_heading);
    let upside_down = blocker.matrix.up.z < 0.0;
    let mut corners = Corners {
        top_left: Occupancy::Clear, top_right: Occupancy::Clear,
        bottom_left: Occupancy::Clear, bottom_right: Occupancy::Clear,
    };
    if now > avoidance.colliding_thing_timer || previous_obstacle != Some(entity) {
        let interval = ((blocker.max.y - blocker.min.y) * 0.1)
            .max(0.5)
            .min((blocker.max.z - blocker.min.z) * 0.5)
            .min((max.x - min.x) * 0.5);
        let inset = interval * 0.7;
        let probe = |x: f32, y: f32| {
            let mut p = blocker.matrix.transform_point(CVector::new(x, y, 0.0));
            p.z += 0.6;
            if loader
                .world
                .sphere_contact_buildings_only([p.x, p.y, p.z], interval, &loader.model_info)
                .is_some()
            {
                Occupancy::Building
            } else if script_ped_sphere_hits_vehicle_except(
                p,
                interval,
                colliders,
                blocker.vehicle_slot,
            ) {
                Occupancy::Vehicle
            } else if loader.world.test_sphere_against_static_world(
                [p.x, p.y, p.z],
                interval,
                &loader.model_info,
            ) {
                Occupancy::Object
            } else {
                Occupancy::Clear
            }
        };
        corners = if upside_down {
            Corners {
                top_left: Occupancy::Clear,
                top_right: Occupancy::Clear,
                bottom_left: Occupancy::Clear,
                bottom_right: Occupancy::Clear,
            }
        } else {
            Corners {
                top_left: probe(min.x + inset, max.y - inset),
                top_right: probe(max.x - inset, max.y - inset),
                bottom_left: probe(min.x + inset, min.y + inset),
                bottom_right: probe(max.x - inset, min.y + inset),
            }
        };
        let entering_same_side = entry.is_some_and(|(door, _)| match region {
            Region::Right => matches!(door, 3 | 5),
            Region::Left => matches!(door, 2 | 4),
            _ => false,
        });
        avoidance.walk_around_type = if upside_down || entering_same_side
            || [
                corners.top_left,
                corners.top_right,
                corners.bottom_left,
                corners.bottom_right,
            ]
            .iter()
            .all(|c| c.blocked())
        {
            0
        } else {
            let entry_turn = if entry.is_some_and(|(door, _)| matches!(door, 2 | 4)) {
                match region { Region::Behind => turn.max(f32::EPSILON), Region::Front => turn.min(-f32::EPSILON), _ => turn }
            } else { turn };
            classify_turn(region, entry_turn, corners, avoidance.walk_around_type)
        };
    }
    avoidance.colliding_thing_timer = now
        .wrapping_add(512)
        .wrapping_add(u32::from(general.get_random_number()));
    let kind = avoidance.walk_around_type;
    let (x, y) = local_vehicle_corner(region, kind, turn, entry, corners, min, max)?;
    let mut corner =
        blocker
            .matrix
            .transform_point(CVector::new(if upside_down { -x } else { x }, y, 0.0));
    corner.z = start.z;
    ped.action_x = corner.x;
    ped.action_y = corner.y;
    locomotion.rotation_dest = CGeneral::limit_radian_angle(heading(corner - start));
    if locomotion.rotation_dest != locomotion.rotation_cur
        && ped.flags.hit_something_last_frame
        && locomotion.rotation_dest != old_rotation_dest
    {
        locomotion.rotation_dest =
            CGeneral::limit_radian_angle(std::f32::consts::PI + object_heading);
    }
    let interval = if let Some(speed) = player_move_speed {
        if entry.is_some() || speed <= 0.0 { 0.0 } else { 2.0 / speed }
    } else { match ped.move_state {
        eMoveState::PEDMOVE_WALK => 2.0,
        eMoveState::PEDMOVE_RUN | eMoveState::PEDMOVE_SPRINT => 0.5,
        _ => 0.0,
    }};
    let distance = (corner.x - start.x)
        .hypot(corner.y - start.y)
        .clamp(0.5, 8.0);
    // Native adds current time in float precision before assigning the u32.
    ped.ped_state_timer = (now as f32 + 280.0 * distance * interval) as u32;
    Some(corner)
}

fn local_vehicle_corner(
    region: Region, kind: i32, turn: f32, entry: Option<(i32, bool)>,
    corners: Corners, min: CVector, max: CVector,
) -> Option<(f32, f32)> {
    Some(match region {
        Region::Behind => {
            if entry.is_some_and(|(door, van)| van && matches!(door, 4 | 5)) { return None; }
            if entry.map_or(kind != 1 && kind != 4 && (kind != 0 || turn <= 0.0),
                |(door, _)| !matches!(door, 2 | 4) && (!corners.bottom_right.blocked() || corners.bottom_left.blocked())) {
                (max.x, min.y)
            } else {
                (min.x, min.y)
            }
        }
        Region::Right => {
            if entry.is_some_and(|(door, van)| door == 3 || door == 5 && !van) { return None; }
            if entry.is_some_and(|(door, van)| van && matches!(door, 4 | 5)) {
                (max.x, min.y)
            } else
            if kind == 4 || kind == 3 || kind == 0 && turn > 0.0 {
                (max.x, min.y)
            } else {
                (max.x, max.y)
            }
        }
        Region::Left => {
            if entry.is_some_and(|(door, van)| door == 2 || door == 4 && !van) { return None; }
            if entry.is_some_and(|(door, van)| van && matches!(door, 4 | 5)) {
                (min.x, min.y)
            } else
            if kind == 1 || kind == 2 || kind == 0 && turn > 0.0 {
                (min.x, max.y)
            } else {
                (min.x, min.y)
            }
        }
        Region::Front => {
            let entering_side = entry.filter(|(door, van)| !*van || !matches!(door, 4 | 5));
            if entering_side.map_or(kind == 2 || kind == 3 || kind == 0 && turn > 0.0,
                |(door, _)| !matches!(door, 2 | 4) && (!corners.top_right.blocked() || corners.top_left.blocked())) {
                (max.x, max.y)
            } else {
                (min.x, max.y)
            }
        }
    })
}

fn classify_turn(region: Region, turn: f32, c: Corners, previous: i32) -> i32 {
    let top_clear = !c.top_left.blocked() && !c.top_right.blocked();
    let bottom_clear = !c.bottom_left.blocked() && !c.bottom_right.blocked();
    let left_clear = !c.top_left.blocked() && !c.bottom_left.blocked();
    let right_clear = !c.top_right.blocked() && !c.bottom_right.blocked();
    match region {
        Region::Right if turn >= 0.0 => {
            if c.bottom_right.blocks_turn(top_clear) || c.bottom_left.blocks_turn(top_clear) {
                1
            } else {
                previous
            }
        }
        Region::Right => {
            if c.top_right.blocks_turn(bottom_clear) || c.top_left.blocks_turn(bottom_clear) {
                4
            } else {
                previous
            }
        }
        Region::Left if turn <= 0.0 => {
            if c.bottom_left.blocks_turn(top_clear) || c.bottom_right.blocks_turn(top_clear) {
                2
            } else {
                previous
            }
        }
        Region::Left => {
            if c.top_left.blocks_turn(bottom_clear) || c.top_right.blocks_turn(bottom_clear) {
                3
            } else {
                previous
            }
        }
        Region::Front if turn < 0.0 => {
            if c.top_left.blocks_turn(right_clear) {
                3
            } else {
                previous
            }
        }
        Region::Front => {
            if c.top_right.blocks_turn(left_clear) {
                4
            } else {
                previous
            }
        }
        Region::Behind if turn > 0.0 => {
            if c.bottom_left.blocks_turn(right_clear) {
                2
            } else {
                previous
            }
        }
        Region::Behind if c.bottom_right.blocks_turn(left_clear) => 1,
        Region::Behind => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peds::PedType::ePedType;

    fn clear_corners() -> Corners {
        Corners {
            top_left: Occupancy::Clear,
            top_right: Occupancy::Clear,
            bottom_left: Occupancy::Clear,
            bottom_right: Occupancy::Clear,
        }
    }

    #[test]
    fn entry_from_front_or_back_retains_the_selected_door_side() {
        let min = CVector::new(-1.35, -2.35, 0.0);
        let max = CVector::new(1.35, 2.35, 0.0);
        for door in 2..=5 {
            let x = if matches!(door, 2 | 4) { min.x } else { max.x };
            for turn in [-2.0, 0.0, 2.0] {
                assert_eq!(local_vehicle_corner(Region::Behind, 4, turn, Some((door, false)), clear_corners(), min, max), Some((x, min.y)));
                assert_eq!(local_vehicle_corner(Region::Front, 1, turn, Some((door, false)), clear_corners(), min, max), Some((x, max.y)));
            }
        }
    }

    #[test]
    fn entry_on_the_correct_side_approaches_the_door_without_an_extra_corner() {
        let min = CVector::new(-1.35, -2.35, 0.0);
        let max = CVector::new(1.35, 2.35, 0.0);
        for door in [2, 4] {
            assert_eq!(local_vehicle_corner(Region::Left, 0, 0.0, Some((door, false)), clear_corners(), min, max), None);
            assert!(local_vehicle_corner(Region::Right, 0, 0.0, Some((door, false)), clear_corners(), min, max).is_some());
        }
        for door in [3, 5] {
            assert_eq!(local_vehicle_corner(Region::Right, 0, 0.0, Some((door, false)), clear_corners(), min, max), None);
            assert!(local_vehicle_corner(Region::Left, 0, 0.0, Some((door, false)), clear_corners(), min, max).is_some());
        }
    }

    #[test]
    fn rear_van_entry_uses_the_rear_instead_of_a_side_door_route() {
        let min = CVector::new(-1.35, -2.35, 0.0);
        let max = CVector::new(1.35, 2.35, 0.0);
        for door in [4, 5] {
            assert_eq!(local_vehicle_corner(Region::Behind, 0, 0.0, Some((door, true)), clear_corners(), min, max), None);
            assert_eq!(local_vehicle_corner(Region::Left, 0, 0.0, Some((door, true)), clear_corners(), min, max), Some((min.x, min.y)));
            assert_eq!(local_vehicle_corner(Region::Right, 0, 0.0, Some((door, true)), clear_corners(), min, max), Some((max.x, min.y)));
        }
    }

    #[test]
    fn right_door_entry_avoids_a_blocked_right_corner_when_left_is_clear() {
        let min = CVector::new(-1.35, -2.35, 0.0);
        let max = CVector::new(1.35, 2.35, 0.0);
        let mut corners = clear_corners();
        corners.bottom_right = Occupancy::Building;
        corners.top_right = Occupancy::Vehicle;
        assert_eq!(local_vehicle_corner(Region::Behind, 0, 0.0, Some((3, false)), corners, min, max), Some((min.x, min.y)));
        assert_eq!(local_vehicle_corner(Region::Front, 0, 0.0, Some((3, false)), corners, min, max), Some((min.x, max.y)));
    }

    #[test]
    fn behind_positive_turn_preserves_direction_when_left_is_clear() {
        let mut corners = clear_corners();
        corners.bottom_right = Occupancy::Building;
        // Native's positive-turn arm only examines bottom-left. The right
        // obstacle belongs to the nonpositive arm and must not select type 1.
        assert_eq!(classify_turn(Region::Behind, 0.1, corners, 4), 4);
        assert_eq!(classify_turn(Region::Behind, 0.0, corners, 4), 1);
        corners.bottom_left = Occupancy::Building;
        assert_eq!(classify_turn(Region::Behind, 0.1, corners, 4), 2);
    }

    #[test]
    fn idle_actor_does_not_change_action_heading_or_hysteresis() {
        let mut general = CGeneral::with_seed(1);
        let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
        let mut avoidance = crate::peds::PedObstacle::PedObstacleAvoidance::default();
        ped.move_state = eMoveState::PEDMOVE_STILL;
        ped.action_x = 7.0;
        ped.action_y = 8.0;
        avoidance.colliding_thing_timer = 40;
        ped.ped_state_timer = 50;
        avoidance.walk_around_type = 3;
        let mut locomotion = PedLocomotion::default();
        let before = locomotion.clone();
        let collider = PedVehicleCollider {
            vehicle_slot: 1,
            matrix: CMatrix::unity(),
            min: CVector::new(2.0, -1.0, -0.2),
            max: CVector::new(4.0, 1.0, 1.0),
        };
        let entity = bevy::prelude::World::new().spawn_empty().id();
        assert!(
            native_vehicle_corner(
                &mut ped,
                &mut locomotion,
                &mut avoidance,
                CVector::ZERO,
                &collider,
                entity,
                None,
                &[collider],
                &CFileLoader::default(),
                100,
                &mut general
            )
            .is_none()
        );
        assert_eq!(locomotion, before);
        assert_eq!(ped.action_x, 7.0);
        assert_eq!(ped.action_y, 8.0);
        assert_eq!(avoidance.colliding_thing_timer, 40);
        assert_eq!(ped.ped_state_timer, 50);
        assert_eq!(avoidance.walk_around_type, 3);
    }

    #[test]
    fn persistent_heading_selects_side_and_collision_changes_heading_only() {
        let collider = PedVehicleCollider {
            vehicle_slot: 1,
            matrix: CMatrix::unity(),
            min: CVector::new(2.0, -1.0, -0.2),
            max: CVector::new(4.0, 1.0, 1.0),
        };
        let entity = bevy::prelude::World::new().spawn_empty().id();
        struct Case {
            old_heading: f32,
            hit: bool,
            expected_y: f32,
            now: u32,
            deadline: u32,
        }
        for Case {
            old_heading,
            hit,
            expected_y,
            now,
            deadline,
        } in [
            Case {
                old_heading: -std::f32::consts::FRAC_PI_2,
                hit: false,
                expected_y: -1.35,
                now: 100,
                deadline: 398,
            },
            Case {
                old_heading: 0.0,
                hit: false,
                expected_y: 1.35,
                now: 100,
                deadline: 398,
            },
            Case {
                old_heading: -std::f32::consts::FRAC_PI_2,
                hit: true,
                expected_y: -1.35,
                now: 100,
                deadline: 398,
            },
            // Native converts this odd millisecond value to f32 first.
            // Truncating duration before integer addition would give 16777515.
            Case {
                old_heading: -std::f32::consts::FRAC_PI_2,
                hit: false,
                expected_y: -1.35,
                now: 16_777_217,
                deadline: 16_777_514,
            },
        ] {
            let mut general = CGeneral::with_seed(1);
            let mut ped = CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general);
            let mut avoidance = crate::peds::PedObstacle::PedObstacleAvoidance::default();
            ped.move_state = eMoveState::PEDMOVE_RUN;
            ped.flags.hit_something_last_frame = hit;
            let mut locomotion = PedLocomotion {
                rotation_dest: old_heading,
                ..Default::default()
            };
            let corner = native_vehicle_corner(
                &mut ped,
                &mut locomotion,
                &mut avoidance,
                CVector::ZERO,
                &collider,
                entity,
                None,
                &[collider],
                &CFileLoader::default(),
                now,
                &mut general,
            )
            .unwrap();
            assert_eq!(corner.y, expected_y);
            assert_eq!(ped.action_y, expected_y);
            let heading = if hit {
                -std::f32::consts::PI
            } else {
                (-corner.x).atan2(corner.y)
            };
            assert_eq!(locomotion.rotation_dest, heading);
            assert_eq!(ped.ped_state_timer, deadline);
        }
    }
}
