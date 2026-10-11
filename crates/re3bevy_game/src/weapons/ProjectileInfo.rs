//! re3 reference: references/re-GTA/src/weapons/ProjectileInfo.cpp
//! re3 reference: references/re-GTA/src/weapons/ProjectileInfo.h
//!
//! Projectile slot table and object-handle association.

use bevy::prelude::{Res, ResMut, Resource};
use re3bevy_core::core::Coordinates::CVector;

use crate::{
    core::{FileLoader::CFileLoader, config::NUM_PROJECTILES},
    peds::PlayerRead::CWorldPlayersRead,
    renderer::{Particle::CParticle, ParticleType::PARTICLE_SMOKE, WaterLevel::CWaterLevel},
    weapons::Explosion::{CExplosionPool, eExplosionType},
};
use re3bevy_core::core::Timer::CTimer;

use super::{
    WeaponType::eWeaponType, WeaponRaycast::{WeaponRayHit, closest_weapon_ray_hit},
    WeaponRaycastAccess::WeaponRaycastAccess,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CProjectileInfo {
    pub weapon_type: eWeaponType,
    pub source_entity: Option<u32>,
    pub explosion_time: u32,
    pub in_use: bool,
    pub pos: CVector,
    pub launch_pos: CVector,
    pub previous_pos: CVector,
    pub velocity: CVector,
    pub affected_by_gravity: bool,
    pub elasticity: f32,
    pub native_motion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectileExplosionIntent {
    pub source_entity: Option<u32>,
    pub weapon_type: eWeaponType,
    pub position: CVector,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CProjectileInfoPool {
    pub info: [CProjectileInfo; NUM_PROJECTILES],
    pub projectile_handles: [Option<u32>; NUM_PROJECTILES],
    pub pending_explosions: Vec<ProjectileExplosionIntent>,
}

impl Default for CProjectileInfo {
    fn default() -> Self {
        Self {
            weapon_type: eWeaponType::WEAPONTYPE_GRENADE,
            source_entity: None,
            explosion_time: 0,
            in_use: false,
            pos: CVector::ZERO,
            launch_pos: CVector::ZERO,
            previous_pos: CVector::ZERO,
            velocity: CVector::ZERO,
            affected_by_gravity: false,
            elasticity: 0.75,
            native_motion: false,
        }
    }
}

impl Default for CProjectileInfoPool {
    fn default() -> Self {
        Self::Initialise()
    }
}

impl CProjectileInfoPool {
    pub fn Initialise() -> Self {
        Self {
            info: [CProjectileInfo::default(); NUM_PROJECTILES],
            projectile_handles: [None; NUM_PROJECTILES],
            pending_explosions: Vec::new(),
        }
    }

    /// Static projectile slots are released by `RemoveAllProjectiles`, not by
    /// re3's allocation-free shutdown routine.
    pub fn Shutdown(&mut self) {}

    pub fn GetProjectileInfo(&self, id: i32) -> Option<&CProjectileInfo> {
        self.info.get(usize::try_from(id).ok()?)
    }

    pub fn AddProjectile(
        &mut self,
        source: Option<u32>,
        weapon: eWeaponType,
        pos: CVector,
        explosion_time: u32,
        projectile_handle: Option<u32>,
    ) -> bool {
        let Some(index) = self.info.iter().position(|info| !info.in_use) else {
            return false;
        };
        self.info[index] = CProjectileInfo {
            weapon_type: weapon,
            source_entity: source,
            explosion_time,
            in_use: true,
            pos,
            launch_pos: pos,
            previous_pos: pos,
            velocity: CVector::ZERO,
            affected_by_gravity: false,
            elasticity: 0.75,
            native_motion: false,
        };
        self.projectile_handles[index] = projectile_handle;
        true
    }

    /// re3 `CProjectileInfo::AddProjectile` movement setup for the player
    /// weapon path. Velocities are native GTA units and are integrated with
    /// `CTimer::GetTimeStep()` rather than seconds.
    pub fn AddProjectileNative(
        &mut self,
        source: Option<u32>,
        weapon: eWeaponType,
        pos: CVector,
        forward: CVector,
        power: f32,
        now_ms: u32,
        projectile_handle: Option<u32>,
    ) -> bool {
        let Some(index) = self.info.iter().position(|info| !info.in_use) else {
            return false;
        };
        let planar = (forward.x * forward.x + forward.y * forward.y)
            .sqrt()
            .max(f32::EPSILON);
        let fx = forward.x / planar;
        let fy = forward.y / planar;
        let (velocity, affected_by_gravity, elasticity, explosion_time) = match weapon {
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => (
                CVector::new(forward.x * 1.25, forward.y * 1.25, forward.z * 1.25),
                false,
                0.75,
                now_ms.wrapping_add(1_400),
            ),
            eWeaponType::WEAPONTYPE_MOLOTOV => {
                let scale = (0.22 * power + 0.15).max(0.2);
                (
                    CVector::new(fx * scale, fy * scale, (0.2 * power + 0.4) * scale),
                    true,
                    0.75,
                    now_ms.wrapping_add(2_000),
                )
            }
            eWeaponType::WEAPONTYPE_GRENADE => {
                let scale = if power == 0.0 {
                    0.0
                } else {
                    0.22 * power + 0.15
                };
                (
                    CVector::new(fx * scale, fy * scale, (0.4 * power + 0.4) * scale),
                    true,
                    0.5,
                    now_ms.wrapping_add(2_000),
                )
            }
            _ => return false,
        };
        self.info[index] = CProjectileInfo {
            weapon_type: weapon,
            source_entity: source,
            explosion_time,
            in_use: true,
            pos,
            launch_pos: pos,
            previous_pos: pos,
            velocity,
            affected_by_gravity,
            elasticity,
            native_motion: true,
        };
        self.projectile_handles[index] = projectile_handle;
        true
    }

    /// Integrates native projectile motion and returns slots that must explode.
    /// Collision is supplied by the ECS/world owner so this resource stays free
    /// of fake entity pools.
    pub fn UpdateNative(
        &mut self,
        now_ms: u32,
        time_step: f32,
        mut line_hit: impl FnMut(CVector, CVector) -> Option<(CVector, CVector)>,
        dynamic_hit: impl FnMut(eWeaponType, CVector) -> bool,
    ) -> Vec<ProjectileExplosionIntent> {
        self.UpdateNativeWithSource(now_ms, time_step,
            |_, from, to| line_hit(from, to), dynamic_hit)
    }

    pub fn UpdateNativeWithSource(
        &mut self,
        now_ms: u32,
        time_step: f32,
        mut line_hit: impl FnMut(Option<u32>, CVector, CVector) -> Option<(CVector, CVector)>,
        mut dynamic_hit: impl FnMut(eWeaponType, CVector) -> bool,
    ) -> Vec<ProjectileExplosionIntent> {
        const GRAVITY: f32 = 0.008;
        let mut explosions = Vec::new();
        for index in 0..NUM_PROJECTILES {
            let projectile = &mut self.info[index];
            if !projectile.in_use || !projectile.native_motion {
                continue;
            }
            projectile.previous_pos = projectile.pos;
            if projectile.affected_by_gravity {
                projectile.velocity.z -= GRAVITY * time_step;
            }
            let next = CVector::new(
                projectile.pos.x + projectile.velocity.x * time_step,
                projectile.pos.y + projectile.velocity.y * time_step,
                projectile.pos.z + projectile.velocity.z * time_step,
            );
            let hit = line_hit(projectile.source_entity, projectile.pos, next);
            let timed_out = now_ms > projectile.explosion_time;
            let mut detonate = timed_out;
            if let Some((point, normal)) = hit {
                projectile.pos = point;
                if !timed_out && projectile.weapon_type == eWeaponType::WEAPONTYPE_GRENADE {
                    let dot = projectile.velocity.x * normal.x
                        + projectile.velocity.y * normal.y
                        + projectile.velocity.z * normal.z;
                    projectile.velocity.x =
                        (projectile.velocity.x - 2.0 * dot * normal.x) * projectile.elasticity;
                    projectile.velocity.y =
                        (projectile.velocity.y - 2.0 * dot * normal.y) * projectile.elasticity;
                    projectile.velocity.z =
                        (projectile.velocity.z - 2.0 * dot * normal.z) * projectile.elasticity;
                } else {
                    detonate = true;
                }
            } else {
                projectile.pos = next;
            }
            if projectile.weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
                && dynamic_hit(projectile.weapon_type, projectile.pos)
            {
                detonate = true;
            }
            if detonate {
                explosions.push(ProjectileExplosionIntent {
                    source_entity: projectile.source_entity,
                    weapon_type: projectile.weapon_type,
                    position: projectile.pos,
                });
                self.delete_projectile(index);
            }
        }
        explosions
    }

    pub fn RemoveProjectile(&mut self, index: usize) {
        if index >= NUM_PROJECTILES || !self.info[index].in_use {
            return;
        }
        let info = self.info[index];
        self.pending_explosions.push(ProjectileExplosionIntent {
            source_entity: info.source_entity,
            weapon_type: info.weapon_type,
            position: info.pos,
        });
        self.delete_projectile(index);
    }

    pub fn RemoveNotAdd(&mut self, source: Option<u32>, weapon_type: eWeaponType, position: CVector) {
        self.pending_explosions.push(ProjectileExplosionIntent { source_entity: source, weapon_type, position });
    }

    fn delete_projectile(&mut self, index: usize) {
        self.info[index] = CProjectileInfo::default();
        self.projectile_handles[index] = None;
    }

    pub fn RemoveAllProjectiles(&mut self) {
        for index in 0..NUM_PROJECTILES {
            if self.info[index].in_use {
                // re3 RemoveAllProjectiles deletes objects without detonating.
                self.delete_projectile(index);
            }
        }
    }

    pub fn Update(&mut self, now_ms: u32) -> Vec<ProjectileExplosionIntent> {
        for index in 0..NUM_PROJECTILES {
            if self.info[index].in_use && now_ms > self.info[index].explosion_time {
                self.RemoveProjectile(index);
            }
        }
        std::mem::take(&mut self.pending_explosions)
    }

    pub fn IsProjectileInRange(
        &mut self,
        x1: f32,
        x2: f32,
        y1: f32,
        y2: f32,
        z1: f32,
        z2: f32,
        remove: bool,
    ) -> bool {
        let mut result = false;
        for index in 0..NUM_PROJECTILES {
            let info = self.info[index];
            if !info.in_use
                || !matches!(
                    info.weapon_type,
                    eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
                        | eWeaponType::WEAPONTYPE_MOLOTOV
                        | eWeaponType::WEAPONTYPE_GRENADE
                )
            {
                continue;
            }
            if info.pos.x >= x1
                && info.pos.x <= x2
                && info.pos.y >= y1
                && info.pos.y <= y2
                && info.pos.z >= z1
                && info.pos.z <= z2
            {
                result = true;
                if remove {
                    // Range opcodes remove the world object but do not call
                    // `RemoveNotAdd`, hence no explosion.
                    self.delete_projectile(index);
                }
            }
        }
        result
    }
}

/// re3 `CProjectileInfo::Update` world-side runtime. Projectile slots remain a
/// compact resource while peds/vehicles stay authoritative Bevy ECS entities.
pub(crate) fn process_projectile_runtime(
    mut projectiles: ResMut<CProjectileInfoPool>,
    timer: Res<CTimer>,
    file_loader: Res<CFileLoader>,
    water: Option<Res<CWaterLevel>>,
    mut particles: ResMut<CParticle>,
    mut explosions: ResMut<CExplosionPool>,
    actors: WeaponRaycastAccess,
    players: CWorldPlayersRead,
    particle_systems: Res<crate::renderer::ParticleMgr::cParticleSystemMgr>,
    camera: Res<crate::core::Camera::CCamera>,
) {
    let mut particles = particles.emitter(&particle_systems, camera.position);
    let now_ms = timer.time_in_milliseconds();
    let time_step = timer.time_step().max(0.0);

    // Native emits one smoke particle at the rocket's current position before
    // integrating/checking the segment for this frame.
    for projectile in projectiles.info.iter().filter(|projectile| {
        projectile.in_use
            && projectile.native_motion
            && projectile.weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
    }) {
        particles.add_particle(
            PARTICLE_SMOKE,
            projectile.pos,
            CVector::ZERO,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    }

    let detonation_intents = projectiles.UpdateNativeWithSource(
        now_ms,
        time_step,
        |source, from, to| {
            let delta = CVector::new(to.x - from.x, to.y - from.y, to.z - from.z);
            let range = delta.magnitude();
            if range <= f32::EPSILON { return None; }
            let direction = super::WeaponRuntime::scaled_vector(delta, 1.0 / range);
            let target_player = players.iter().find(|player|
                Some(player.pool_handle as u32) != source && player.active_in_world);
            let hit = closest_weapon_ray_hit(from, direction, range, target_player,
                source.map(|handle| handle as i32), None, &actors, &file_loader)
                .map(|hit| match hit {
                    WeaponRayHit::Static {point, normal, ..}
                    | WeaponRayHit::Vehicle {point, normal, ..}
                    | WeaponRayHit::Object {point, normal, ..} => (point, normal),
                    WeaponRayHit::Player {point, ..} | WeaponRayHit::Ped {point, ..} =>
                        (point, super::WeaponRuntime::scaled_vector(direction, -1.0)),
                });
            if hit.is_some() {
                return hit;
            }
            if let Some(ground_z) = file_loader.world.find_ground_z_for_3d_coord(
                to.x,
                to.y,
                to.z.max(from.z) + 2.0,
                &file_loader.model_info,
            ) {
                if to.z <= ground_z || from.z <= ground_z {
                    return Some((
                        CVector::new(to.x, to.y, ground_z),
                        CVector::new(0.0, 0.0, 1.0),
                    ));
                }
            }
            None
        },
        |_, _| false,
    );

    for mut intent in detonation_intents {
        let explosion_type = match intent.weapon_type {
            eWeaponType::WEAPONTYPE_GRENADE => eExplosionType::EXPLOSION_GRENADE,
            eWeaponType::WEAPONTYPE_MOLOTOV => eExplosionType::EXPLOSION_MOLOTOV,
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => eExplosionType::EXPLOSION_ROCKET,
            _ => continue,
        };
        if intent.weapon_type == eWeaponType::WEAPONTYPE_MOLOTOV {
            // CExplosion::AddExplosion cancels Molotov fire when the ground
            // beneath the impact is submerged (within the native 6-unit guard).
            let ground_z = file_loader
                .world
                .find_ground_z_for_3d_coord(
                    intent.position.x,
                    intent.position.y,
                    intent.position.z + 3.0,
                    &file_loader.model_info,
                )
                .unwrap_or(intent.position.z);
            if water
                .as_deref()
                .and_then(|water| {
                    water.get_water_level_no_waves(intent.position.x, intent.position.y, ground_z)
                })
                .is_some_and(|sample| ground_z < sample.level && sample.level - 6.0 < ground_z)
            {
                continue;
            }
            intent.position.z = ground_z;
        }
        if !explosions.AddExplosion(
            None,
            intent.source_entity.map(|handle| crate::entities::EntityReference::EntityReference::Ped(handle as i32)),
            explosion_type,
            intent.position,
            0,
            now_ms,
        ) {
            continue;
        }

        // CWorld::TriggerExplosion and Molotov fire propagation are consumed
        // centrally by process_explosion_runtime from CExplosionPool intents.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_rocket_uses_re3_speed_and_fuse() {
        let mut pool = CProjectileInfoPool::default();
        assert!(pool.AddProjectileNative(
            Some(7),
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER,
            CVector::new(1.0, 2.0, 3.0),
            CVector::new(0.0, 1.0, 0.0),
            0.0,
            100,
            None,
        ));
        let rocket = pool.info.iter().find(|slot| slot.in_use).unwrap();
        assert_eq!(rocket.explosion_time, 1_500);
        assert_eq!(rocket.velocity, CVector::new(0.0, 1.25, 0.0));
        assert!(!rocket.affected_by_gravity);
        assert_eq!(rocket.elasticity, 0.75);
    }

    #[test]
    fn native_grenade_uses_power_velocity_gravity_and_half_elasticity() {
        let mut pool = CProjectileInfoPool::default();
        assert!(pool.AddProjectileNative(
            Some(7),
            eWeaponType::WEAPONTYPE_GRENADE,
            CVector::ZERO,
            CVector::new(0.0, 1.0, 0.0),
            1.0,
            200,
            None,
        ));
        let grenade = pool.info.iter().find(|slot| slot.in_use).unwrap();
        assert_eq!(grenade.explosion_time, 2_200);
        assert!((grenade.velocity.y - 0.37).abs() < 1.0e-6);
        assert!((grenade.velocity.z - 0.296).abs() < 1.0e-6);
        assert!(grenade.affected_by_gravity);
        assert_eq!(grenade.elasticity, 0.5);
    }

    #[test]
    fn native_grenade_bounces_instead_of_exploding_before_fuse() {
        let mut pool = CProjectileInfoPool::default();
        assert!(pool.AddProjectileNative(
            None,
            eWeaponType::WEAPONTYPE_GRENADE,
            CVector::ZERO,
            CVector::new(0.0, 1.0, 0.0),
            1.0,
            0,
            None,
        ));
        let explosions = pool.UpdateNative(
            16,
            1.0,
            |_from, _to| Some((CVector::new(0.0, 0.1, 0.0), CVector::new(0.0, -1.0, 0.0))),
            |_weapon, _position| false,
        );
        assert!(explosions.is_empty());
        let grenade = pool.info.iter().find(|slot| slot.in_use).unwrap();
        assert!(grenade.velocity.y < 0.0);
    }

    #[test]
    fn native_molotov_detonates_immediately_on_impact() {
        let mut pool = CProjectileInfoPool::default();
        assert!(pool.AddProjectileNative(
            None,
            eWeaponType::WEAPONTYPE_MOLOTOV,
            CVector::ZERO,
            CVector::new(0.0, 1.0, 0.0),
            1.0,
            0,
            None,
        ));
        let explosions = pool.UpdateNative(
            16,
            1.0,
            |_from, _to| Some((CVector::new(0.0, 0.1, 0.0), CVector::new(0.0, -1.0, 0.0))),
            |_weapon, _position| false,
        );
        assert_eq!(explosions.len(), 1);
        assert_eq!(explosions[0].weapon_type, eWeaponType::WEAPONTYPE_MOLOTOV);
        assert_eq!(explosions[0].position, CVector::new(0.0, 0.1, 0.0));
    }
}
