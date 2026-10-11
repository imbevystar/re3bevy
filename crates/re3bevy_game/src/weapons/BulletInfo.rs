//! re3 reference: references/re-GTA/src/weapons/BulletInfo.cpp
//! re3 reference: references/re-GTA/src/weapons/BulletInfo.h
//!
//! Moving bullet slots and their deterministic line-of-sight decisions. The
//! ECS collision queries supply each trace result; typed side effects preserve
//! the reference update ordering at the gameplay and rendering interfaces.

use bevy::prelude::Resource;
use re3bevy_core::core::Coordinates::CVector;

use super::{WeaponInfo::CWeaponInfoStore, WeaponType::eWeaponType};

pub const NUM_BULLETS: usize = 100;
pub const BULLET_LIFETIME_MS: f32 = 1_000.0;
pub const MAP_BORDER: f32 = 1_960.0;
pub const BULLET_HIT_FORCE: f32 = 7.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CBulletInfo {
    pub weapon_type: eWeaponType,
    pub source_entity: Option<u32>,
    /// Absolute expiry (`GetTimeInMilliseconds() + 1000`).
    pub timer: f32,
    pub in_use: bool,
    pub position: CVector,
    pub speed: CVector,
    pub damage: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulletHitEntityType {
    Building,
    Object,
    Dummy,
    Ped,
    Vehicle,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletHit {
    pub entity: u32,
    pub entity_type: BulletHitEntityType,
    pub point: CVector,
    pub normal: CVector,
    pub piece: i32,
    pub ped_dying_or_dead: bool,
    pub ped_in_control: bool,
    pub ped_ducking: bool,
    pub ped_los_hit: bool,
    pub ped_is_cop: bool,
    pub ped_position: CVector,
    pub object_infinite_mass: bool,
    pub object_static: bool,
    pub object_uproot_limit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletTrace {
    pub bullet_slot: usize,
    pub source: Option<u32>,
    pub weapon_type: eWeaponType,
    pub start: CVector,
    pub end: CVector,
    pub include_car_tyres: bool,
    pub include_dead_peds: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulletUpdateIntent {
    IncrementInstantHits,
    ClearPedAttackAndPlayFrontHit {
        ped: u32,
    },
    DamagePed {
        ped: u32,
        source: Option<u32>,
        weapon: eWeaponType,
        damage: i16,
        piece: i32,
        hit_point: CVector,
    },
    RegisterShootEvent {
        ped: u32,
        source: Option<u32>,
        cop: bool,
    },
    DamageVehicle {
        vehicle: u32,
        source: Option<u32>,
        weapon: eWeaponType,
        damage: i16,
    },
    StartVehicleFire {
        vehicle: u32,
        source: Option<u32>,
    },
    SpawnBlood {
        point: CVector,
        direction: CVector,
        count: u8,
    },
    SpawnSparks {
        point: CVector,
        direction: CVector,
        count: u8,
    },
    UprootObject {
        object: u32,
    },
    ApplyObjectMoveForce {
        object: u32,
        force: CVector,
    },
    SniperHitSound {
        entity: u32,
        entity_type: BulletHitEntityType,
        point: CVector,
    },
    GlassHit {
        entity: u32,
        point: CVector,
    },
    BlowUpExplosiveThings {
        entity: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CBulletInfoPool {
    pub bullets: [CBulletInfo; NUM_BULLETS],
    pub player_sniper_bullet: bool,
    pub player_sniper_bullet_start: CVector,
    pub player_sniper_bullet_end: CVector,
}

impl Default for CBulletInfo {
    fn default() -> Self {
        Self {
            weapon_type: eWeaponType::WEAPONTYPE_COLT45,
            source_entity: None,
            timer: 0.0,
            in_use: false,
            position: CVector::ZERO,
            speed: CVector::ZERO,
            damage: 0,
        }
    }
}

impl Default for CBulletInfoPool {
    fn default() -> Self {
        Self::Initialise()
    }
}

impl CBulletInfoPool {
    pub fn Initialise() -> Self {
        Self {
            bullets: [CBulletInfo::default(); NUM_BULLETS],
            player_sniper_bullet: false,
            player_sniper_bullet_start: CVector::ZERO,
            player_sniper_bullet_end: CVector::ZERO,
        }
    }

    /// re3 shutdown has no allocation and does not clear the static array.
    pub fn Shutdown(&mut self) {}

    pub fn AddBullet(
        &mut self,
        source: Option<u32>,
        weapon_type: eWeaponType,
        position: CVector,
        speed: CVector,
        current_time_ms: u32,
        weapon_info: &CWeaponInfoStore,
    ) -> bool {
        let Some(slot) = self.bullets.iter_mut().find(|bullet| !bullet.in_use) else {
            return false;
        };
        let damage = weapon_info
            .GetWeaponInfo(weapon_type)
            .map(|info| info.damage as i16)
            .unwrap_or(0);
        *slot = CBulletInfo {
            weapon_type,
            source_entity: source,
            timer: current_time_ms as f32 + BULLET_LIFETIME_MS,
            in_use: true,
            position,
            speed,
            damage,
        };
        true
    }

    /// `CBulletInfo::Update`. Collision is queried synchronously because re3
    /// changes the final segment end and bullet lifetime in the same loop.
    pub fn Update(
        &mut self,
        current_time_ms: u32,
        time_step: f32,
        nasty_game: bool,
        source_ped_pointer_valid: impl Fn(u32) -> bool,
        mut line_of_sight: impl FnMut(BulletTrace) -> Option<BulletHit>,
    ) -> Vec<BulletUpdateIntent> {
        let mut intents = Vec::new();
        self.player_sniper_bullet = false;
        for (slot_index, bullet) in self.bullets.iter_mut().enumerate() {
            if let Some(source) = bullet.source_entity {
                if !source_ped_pointer_valid(source) {
                    bullet.source_entity = None;
                }
            }
            if !bullet.in_use {
                continue;
            }
            if current_time_ms as f32 > bullet.timer {
                bullet.in_use = false;
                // re3 intentionally continues processing this final slot frame.
            }
            let old = bullet.position;
            let mut new = CVector::new(
                old.x + bullet.speed.x * time_step * 0.5,
                old.y + bullet.speed.y * time_step * 0.5,
                old.z + bullet.speed.z * time_step * 0.5,
            );
            let trace = BulletTrace {
                bullet_slot: slot_index,
                source: bullet.source_entity,
                weapon_type: bullet.weapon_type,
                start: old,
                end: new,
                include_car_tyres: true,
                include_dead_peds: true,
            };
            if let Some(hit) = line_of_sight(trace) {
                if bullet.source_entity.is_some()
                    && matches!(
                        hit.entity_type,
                        BulletHitEntityType::Ped | BulletHitEntityType::Vehicle
                    )
                {
                    intents.push(BulletUpdateIntent::IncrementInstantHits);
                }
                let mut add_sniper_sound = true;
                match hit.entity_type {
                    BulletHitEntityType::Ped => {
                        if !hit.ped_dying_or_dead && Some(hit.entity) != bullet.source_entity {
                            if hit.ped_los_hit {
                                if hit.ped_in_control && !hit.ped_ducking {
                                    intents.push(
                                        BulletUpdateIntent::ClearPedAttackAndPlayFrontHit {
                                            ped: hit.entity,
                                        },
                                    );
                                }
                                intents.push(BulletUpdateIntent::DamagePed {
                                    ped: hit.entity,
                                    source: bullet.source_entity,
                                    weapon: bullet.weapon_type,
                                    damage: bullet.damage,
                                    piece: hit.piece,
                                    hit_point: hit.point,
                                });
                                intents.push(BulletUpdateIntent::RegisterShootEvent {
                                    ped: hit.entity,
                                    source: bullet.source_entity,
                                    cop: hit.ped_is_cop,
                                });
                                bullet.in_use = false;
                                new = hit.point;
                            } else {
                                add_sniper_sound = false;
                            }
                        }
                        if nasty_game {
                            let direction = CVector::new(
                                (hit.point.x - hit.ped_position.x) * 0.01,
                                (hit.point.y - hit.ped_position.y) * 0.01,
                                0.01,
                            );
                            intents.push(BulletUpdateIntent::SpawnBlood {
                                point: hit.point,
                                direction,
                                count: 8,
                            });
                            bullet.in_use = false;
                            new = hit.point;
                        }
                    }
                    BulletHitEntityType::Vehicle => {
                        intents.push(BulletUpdateIntent::DamageVehicle {
                            vehicle: hit.entity,
                            source: bullet.source_entity,
                            weapon: bullet.weapon_type,
                            damage: bullet.damage,
                        });
                        if bullet.weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER {
                            intents.push(BulletUpdateIntent::StartVehicleFire {
                                vehicle: hit.entity,
                                source: bullet.source_entity,
                            });
                        } else {
                            intents.push(BulletUpdateIntent::SpawnSparks {
                                point: hit.point,
                                direction: scale(hit.normal, 1.0 / 20.0),
                                count: 16,
                            });
                        }
                        // Current re3 config defines FIX_BUGS.
                        bullet.in_use = false;
                        new = hit.point;
                    }
                    _ => {
                        intents.push(BulletUpdateIntent::SpawnSparks {
                            point: hit.point,
                            direction: scale(hit.normal, 1.0 / 20.0),
                            count: 8,
                        });
                        if hit.entity_type == BulletHitEntityType::Object
                            && !hit.object_infinite_mass
                        {
                            if hit.object_static && hit.object_uproot_limit <= 0.0 {
                                intents
                                    .push(BulletUpdateIntent::UprootObject { object: hit.entity });
                            }
                            if !hit.object_static || hit.object_uproot_limit <= 0.0 {
                                intents.push(BulletUpdateIntent::ApplyObjectMoveForce {
                                    object: hit.entity,
                                    force: scale(hit.normal, -BULLET_HIT_FORCE),
                                });
                            }
                        }
                        bullet.in_use = false;
                        new = hit.point;
                    }
                }
                if bullet.weapon_type == eWeaponType::WEAPONTYPE_SNIPERRIFLE && add_sniper_sound {
                    intents.push(BulletUpdateIntent::SniperHitSound {
                        entity: hit.entity,
                        entity_type: hit.entity_type,
                        point: hit.point,
                    });
                }
                intents.push(BulletUpdateIntent::GlassHit {
                    entity: hit.entity,
                    point: hit.point,
                });
                intents.push(BulletUpdateIntent::BlowUpExplosiveThings { entity: hit.entity });
            }
            if bullet.weapon_type == eWeaponType::WEAPONTYPE_SNIPERRIFLE {
                self.player_sniper_bullet = true;
                self.player_sniper_bullet_start = old;
                self.player_sniper_bullet_end = new;
            }
            bullet.position = new;
            if new.x < -MAP_BORDER
                || new.x > MAP_BORDER
                || new.y < -MAP_BORDER
                || new.y > MAP_BORDER
            {
                bullet.in_use = false;
            }
        }
        intents
    }

    /// FIX_BUGS re3 path: `CCollision::TestLineBox` against the one sniper
    /// segment recorded by the most recent update.
    pub fn TestForSniperBullet(
        &self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
    ) -> bool {
        if !self.player_sniper_bullet {
            return false;
        }
        segment_intersects_box(
            self.player_sniper_bullet_start,
            self.player_sniper_bullet_end,
            CVector::new(x1, y1, z1),
            CVector::new(x2, y2, z2),
        )
    }
}

fn scale(vector: CVector, amount: f32) -> CVector {
    CVector::new(vector.x * amount, vector.y * amount, vector.z * amount)
}

fn segment_intersects_box(start: CVector, end: CVector, min: CVector, max: CVector) -> bool {
    let direction = CVector::new(end.x - start.x, end.y - start.y, end.z - start.z);
    let mut enter = 0.0_f32;
    let mut leave = 1.0_f32;
    for (origin, delta, lower, upper) in [
        (start.x, direction.x, min.x, max.x),
        (start.y, direction.y, min.y, max.y),
        (start.z, direction.z, min.z, max.z),
    ] {
        if delta.abs() < f32::EPSILON {
            if origin < lower || origin > upper {
                return false;
            }
            continue;
        }
        let mut first = (lower - origin) / delta;
        let mut second = (upper - origin) / delta;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
        enter = enter.max(first);
        leave = leave.min(second);
        if enter > leave {
            return false;
        }
    }
    true
}
