//! Shared weapon behavior at the Bevy animation/world interfaces.
//! Reference: Weapon.cpp, PedFight.cpp and PlayerPed.cpp.
use crate::animation::AnimationId::*;
use super::WeaponInfo::CWeaponInfo;
use re3bevy_core::core::Coordinates::CVector;

/// CWeapon::FireProjectile: clear release path, or drop/detonate a blocked throw.
pub(crate) fn fire_projectile(pool: &mut super::ProjectileInfo::CProjectileInfoPool,
    shooter: u32, shooter_position: CVector, forward: CVector, mut muzzle: CVector,
    launch_direction: CVector, weapon: super::WeaponType::eWeaponType, power: f32, now: u32,
    world: &crate::control::ScriptEntities::ScriptEntityWorld,
    loader: &crate::core::FileLoader::CFileLoader) {
    use super::WeaponType::eWeaponType::*;
    let source;
    let target;
    if weapon == WEAPONTYPE_ROCKETLAUNCHER {
        source = muzzle;
        muzzle = add(muzzle, launch_direction);
        target = muzzle;
    } else {
        let projection = dot(muzzle - shooter_position, forward);
        if projection < 0.3 { muzzle = add(muzzle, scaled_vector(forward, 0.3 - projection)); }
        target = if muzzle.z > shooter_position.z { add(muzzle, scaled_vector(forward, 0.6)) } else { muzzle };
        source = add(muzzle, scaled_vector(forward, -dot(muzzle - shooter_position, forward)));
    }
    let blocked = loader.world.process_line_of_sight_static_hit(
        [source.x, source.y, source.z], [target.x, target.y, target.z], &loader.model_info, None, false).is_some()
        || world.vehicles.values().any(|vehicle| {
            let entity = &vehicle.base().physical.entity;
            crate::peds::PlayerPed::weapon_col_model_hit(source, target, &entity.matrix,
                entity.model_index.max(0) as usize, entity.flags.uses_collision, loader).is_some()
        }) || world.objects.values().any(|object| {
            let entity = &object.object.physical.entity;
            crate::peds::PlayerPed::weapon_col_model_hit(source, target, &entity.matrix,
                entity.model_index.max(0) as usize, entity.flags.uses_collision, loader).is_some()
        });
    let mut release = muzzle;
    let mut release_power = power;
    if blocked && weapon == WEAPONTYPE_GRENADE {
        release = add(shooter_position, scaled_vector(forward, -1.0));
        release.z -= 0.4;
        release_power = 0.0;
        if world.peds.values().any(|ped| crate::peds::PlayerPed::melee_ped_collision(&ped.ped, &ped.animation, release, 0.5, loader).is_some()) {
            pool.RemoveNotAdd(Some(shooter), weapon, muzzle);
            return;
        }
    } else if blocked {
        pool.RemoveNotAdd(Some(shooter), weapon, muzzle);
        return;
    }
    if !pool.AddProjectileNative(Some(shooter), weapon, release, launch_direction, release_power, now, None) {
        bevy::log::warn_once!(target: "re3bevy::weapons", "projectile pool exhausted at weapon release");
    }
}

fn add(a: CVector, b: CVector) -> CVector { CVector::new(a.x+b.x, a.y+b.y, a.z+b.z) }
fn dot(a: CVector, b: CVector) -> f32 { a.x*b.x+a.y*b.y+a.z*b.z }

/// CWeapon::BlowUpExplosiveThings, resolving native names through loaded IDE data.
pub(crate) fn blow_up_explosive_object(handle: i32, source: Option<u32>,
    world: &mut crate::control::ScriptEntities::ScriptEntityWorld,
    loader: &crate::core::FileLoader::CFileLoader,
    explosions: &mut super::Explosion::CExplosionPool,
    general: &mut crate::core::General::CGeneral, now: u32) {
    let Some(object) = world.objects.get_mut(&world.index, &handle) else { return; };
    blow_up_explosive_object_state(handle, source, &mut *object.object, loader, explosions, general, now);
}

pub(crate) fn blow_up_explosive_object_state(handle: i32, source: Option<u32>,
    object: &mut crate::objects::Object::CObject,
    loader: &crate::core::FileLoader::CFileLoader,
    explosions: &mut super::Explosion::CExplosionPool,
    general: &mut crate::core::General::CGeneral, now: u32) {
    let model = object.physical.entity.model_index.max(0) as usize;
    let Some(model) = loader.model_info.get_model_info(model) else { return; };
    let name = model.base().name.as_str();
    if !matches!(name, "barrel4" | "petrolpump") || object.flags.has_been_damaged { return; }
    object.flags.has_been_damaged = true;
    let mut position = object.physical.entity.get_position();
    position.z += 0.5;
    if !explosions.AddExplosion(
        Some(crate::entities::EntityReference::EntityReference::Object(handle)),
        source.map(|handle| crate::entities::EntityReference::EntityReference::Ped(handle as i32)),
        super::Explosion::eExplosionType::EXPLOSION_BARREL, position, 100, now) {
        bevy::log::warn_once!(target: "re3bevy::weapons", "explosion pool exhausted by explosive object");
    }
    let speed = &mut object.physical.move_speed;
    let scale = crate::entities::Physical::GTA_PHYSICS_HZ;
    speed.z += if name == "barrel4" { 0.75 * scale } else { 0.45 * scale };
    speed.x += ((general.get_random_number() & 255) as i32 - 128) as f32 * 0.0002 * scale;
    speed.y += ((general.get_random_number() & 255) as i32 - 128) as f32 * 0.0002 * scale;
    object.physical.entity.flags.is_static = false;
}

pub(crate) fn bullet_hit_glass(handle: i32, point: CVector,
    world: &mut crate::control::ScriptEntities::ScriptEntityWorld,
    loader: &crate::core::FileLoader::CFileLoader, glass: &mut crate::renderer::Glass::CGlass,
    general: &mut crate::core::General::CGeneral, now: u32) {
    let Some(object) = world.objects.get_mut(&world.index, &handle) else { return; };
    bullet_hit_glass_state(point, object.object, loader, glass, general, now);
}

/// Bullet collision operates on the live CObject without acquiring another
/// actor owner or retaining pane state after the glass request is produced.
pub(crate) fn bullet_hit_glass_state(point: CVector,
    object: &mut crate::objects::Object::CObject,
    loader: &crate::core::FileLoader::CFileLoader, glass: &mut crate::renderer::Glass::CGlass,
    general: &mut crate::core::General::CGeneral, now: u32) {
    let entity = &object.physical.entity;
    let model_id = entity.model_index.max(0) as usize;
    let Some(model) = loader.model_info.get_model_info(model_id) else { return; };
    if !matches!(model.base().name.as_str(), "glassfx1" | "glassfx2" | "glassfx3" | "glassfx4"
        | "glassfx55" | "glassfxsub1" | "glassfxsub2" | "glassfx_composh") { return; }
    let Some(col) = loader.model_info.col_model(model_id) else {
        bevy::log::warn_once!(target: "re3bevy::weapons", "glass object collision model unavailable");
        return;
    };
    if col.vertices.len() < 4 || col.triangles.len() < 2 {
        bevy::log::warn_once!(target: "re3bevy::weapons", "glass collision model lacks native pane geometry");
        return;
    }
    let mut window = crate::renderer::Glass::GlassWindowState {
        position: entity.get_position(), forward: entity.get_forward(), random_seed: entity.random_seed,
        cracked: object.flags.glass_cracked, broken: object.flags.glass_broken,
        corners: std::array::from_fn(|i| { let v = col.vertices[i].get(); entity.transform_point(CVector::new(v.x, v.y, v.z)) }),
        triangles: std::array::from_fn(|i| { let t = &col.triangles[i]; [t.a, t.b, t.c] }),
    };
    glass.was_glass_hit_by_bullet(&mut window, point, true, now,
        &mut || general.get_random_number() as u32,
        &mut |x,y,z| loader.world.find_ground_z_for_3d_coord(x,y,z,&loader.model_info));
    object.flags.glass_cracked = window.cracked;
    object.flags.glass_broken = window.broken;
    if window.broken {
        object.physical.entity.flags.uses_collision = false;
        object.physical.entity.flags.is_visible = false;
    }
}

pub(crate) fn process_sniper_control(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &crate::core::Pad::CPad, camera: &mut crate::core::Camera::CCamera,
    bullets: &mut super::BulletInfo::CBulletInfoPool, info: &super::WeaponInfo::CWeaponInfoStore,
    previous_ms: u32, now_ms: u32, fast_reload: bool,
    mut audio: Option<&mut crate::audio::DMAudio::cDMAudio>,
    stats: Option<&mut crate::core::Stats::CStats>) {
    let weapon = &mut player.ped.weapons[player.ped.current_weapon as usize];
    if weapon.Update(previous_ms, now_ms, player.ped.physical.audio_entity_id >= 0, info)
        && let Some(audio) = audio.as_deref_mut() {
        audio.PlayOneShot(player.ped.physical.audio_entity_id, crate::audio::soundlist::SOUND_WEAPON_RELOAD as u16, 0.0);
    }
    if !pad.weapon_just_down() || weapon.weapon_state == super::WeaponType::eWeaponState::WEAPONSTATE_RELOADING
        || weapon.ammo_in_clip <= 0 { return; }
    let cam = &camera.cams[camera.active_cam];
    if !bullets.AddBullet(Some(player.state.pool_handle as u32), weapon.weapon_type, cam.source,
        scaled_vector(cam.front, 16.0), now_ms, info) {
        bevy::log::warn_once!(target: "re3bevy::weapons", "sniper bullet pool exhausted");
        return;
    }
    weapon.finish_successful_fire(now_ms, true, fast_reload, info);
    if let Some(stats) = stats { stats.instant_hits_fired_by_player += 1; }
    camera.cam_shake_no_position(0.2, now_ms);
    if let Some(audio) = audio {
        audio.PlayOneShot(player.ped.physical.audio_entity_id, crate::audio::soundlist::SOUND_WEAPON_SHOT_FIRED as u16, 0.0);
    }
}

/// CCamera::Find3rdPersonCamTargetVector, including the PC HUD offset.
pub(crate) fn third_person_direction(cam: &crate::core::Cam::CCam, aspect: f32) -> CVector {
    let horizontal = ((0.53_f32 - 0.5) * 0.9 * cam.fov * aspect).to_radians().tan();
    let vertical = ((0.5_f32 - 0.4) * 0.9 * cam.fov).to_radians().tan();
    let left = CVector::new(
        cam.front.y * cam.up.z - cam.front.z * cam.up.y,
        cam.front.z * cam.up.x - cam.front.x * cam.up.z,
        cam.front.x * cam.up.y - cam.front.y * cam.up.x,
    );
    let direction = CVector::new(
        cam.front.x + cam.up.x * vertical + left.x * horizontal,
        cam.front.y + cam.up.y * vertical + left.y * horizontal,
        cam.front.z + cam.up.z * vertical + left.z * horizontal,
    );
    scaled_vector(direction, 1.0 / direction.magnitude())
}

pub(crate) fn project_camera_source(camera: CVector, muzzle: CVector, direction: CVector) -> CVector {
    let distance = (muzzle.x - camera.x) * direction.x
        + (muzzle.y - camera.y) * direction.y
        + (muzzle.z - camera.z) * direction.z;
    CVector::new(camera.x + direction.x * distance,
        camera.y + direction.y * distance, camera.z + direction.z * distance)
}

pub(crate) fn scaled_vector(v: CVector, scale: f32) -> CVector {
    CVector::new(v.x * scale, v.y * scale, v.z * scale)
}

/// CPed::Attack chooses reload clips by attack association, not weapon index.
pub(crate) fn reload_animation(info: &CWeaponInfo) -> Option<AnimationId> {
    match info.anim_to_play {
        ANIM_STD_WEAPON_HGUN_BODY => Some(ANIM_STD_HGUN_RELOAD),
        ANIM_STD_WEAPON_AK_BODY => Some(ANIM_STD_AK_RELOAD),
        _ => None,
    }
}

/// CPed::Attack removes the hand model at the actual throw's fire frame;
/// FinishedAttackCB restores it after the throw association completes.
pub(crate) fn hand_weapon_visible(
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    info: &CWeaponInfo,
) -> bool {
    if !info.IsFlagSet(super::WeaponInfo::WEAPONFLAG_THROW) { return true; }
    !clump.associations.iter().any(|association| {
        let frame = if association.anim_id == ANIM_STD_THROW_UNDER { Some(0.2) }
            else if association.anim_id == ANIM_STD_WEAPON_THROW { Some(info.anim2_frame_fire) }
            else { None };
        frame.is_some_and(|frame| association.current_time > frame && association.blend_delta >= 0.0)
    })
}
