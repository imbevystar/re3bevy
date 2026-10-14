//! re3 reference: references/re-GTA/src/weapons/WeaponInfo.cpp
//! re3 reference: references/re-GTA/src/weapons/WeaponInfo.h
//!
//! Weapon metadata table loaded from `DATA\WEAPON.DAT`.

use bevy::prelude::Resource;
use rust_gta_3_core::core::Coordinates::CVector;
use rust_gta_3_formats::formats::DataRead::{WeaponDat, WeaponRecord};

use crate::animation::AnimationId::{ANIM_STD_NUM, ANIM_STD_PUNCH, AnimationId};

use super::WeaponType::{WEAPONTYPE_TOTALWEAPONS, eWeaponFire, eWeaponType};

pub const WEAPONFLAG_USE_GRAVITY: u32 = 1;
pub const WEAPONFLAG_SLOWS_DOWN: u32 = 1 << 1;
pub const WEAPONFLAG_DISSIPATES: u32 = 1 << 2;
pub const WEAPONFLAG_RAND_SPEED: u32 = 1 << 3;
pub const WEAPONFLAG_EXPANDS: u32 = 1 << 4;
pub const WEAPONFLAG_EXPLODES: u32 = 1 << 5;
pub const WEAPONFLAG_CANAIM: u32 = 1 << 6;
pub const WEAPONFLAG_CANAIM_WITHARM: u32 = 1 << 7;
pub const WEAPONFLAG_1ST_PERSON: u32 = 1 << 8;
pub const WEAPONFLAG_HEAVY: u32 = 1 << 9;
pub const WEAPONFLAG_THROW: u32 = 1 << 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CWeaponInfo {
    pub weapon_fire: eWeaponFire,
    pub range: f32,
    pub firing_rate: u32,
    pub reload: u32,
    pub amount_of_ammunition: i32,
    pub damage: u32,
    pub speed: f32,
    pub radius: f32,
    pub lifespan: f32,
    pub spread: f32,
    pub fire_offset: CVector,
    pub anim_to_play: AnimationId,
    pub anim2_to_play: AnimationId,
    pub anim_loop_start: f32,
    pub anim_loop_end: f32,
    pub anim_frame_fire: f32,
    pub anim2_frame_fire: f32,
    pub model_id: i32,
    pub flags: u32,
}

impl CWeaponInfo {
    /// Time on the looping attack association after re3 reaches
    /// `m_fAnimLoopEnd` and restarts at `m_fAnimLoopStart`.
    pub fn looping_attack_time(&self, elapsed: f32) -> f32 {
        let loop_len = self.anim_loop_end - self.anim_loop_start;
        if elapsed <= self.anim_loop_end || loop_len <= f32::EPSILON {
            elapsed.max(0.0)
        } else {
            self.anim_loop_start + (elapsed - self.anim_loop_start).rem_euclid(loop_len)
        }
    }

    /// Whether one fixed update crossed the authored fire frame. Gameplay
    /// uses this together with the same loop timing the renderer samples.
    pub fn crossed_attack_fire_frame(&self, previous_elapsed: f32, elapsed: f32) -> bool {
        self.crossed_attack_frame(previous_elapsed, elapsed, self.anim_frame_fire)
    }

    pub fn crossed_attack_frame(
        &self,
        previous_elapsed: f32,
        elapsed: f32,
        frame_time: f32,
    ) -> bool {
        if elapsed <= previous_elapsed {
            return false;
        }
        // CPed::Attack uses previous <= fire < current. In particular AK/M16
        // fire at the loop start; a strict previous < fire loses every repeat.
        if previous_elapsed <= frame_time && elapsed > frame_time {
            return true;
        }
        let length = self.anim_loop_end - self.anim_loop_start;
        if length <= f32::EPSILON
            || frame_time < self.anim_loop_start
            || frame_time > self.anim_loop_end
        {
            return false;
        }
        let first_repeat = self.anim_loop_end + frame_time - self.anim_loop_start;
        let repeat = ((previous_elapsed - first_repeat) / length).ceil().max(0.0);
        let next_fire = first_repeat + repeat * length;
        previous_elapsed <= next_fire && elapsed > next_fire
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CWeaponInfoStore {
    pub weapon_info: [CWeaponInfo; WEAPONTYPE_TOTALWEAPONS],
}

impl Default for CWeaponInfo {
    fn default() -> Self {
        Self {
            weapon_fire: eWeaponFire::WEAPON_FIRE_INSTANT_HIT,
            range: 0.0,
            firing_rate: 0,
            reload: 0,
            amount_of_ammunition: 0,
            damage: 0,
            speed: 0.0,
            radius: 0.0,
            lifespan: 0.0,
            spread: 0.0,
            fire_offset: CVector::ZERO,
            anim_to_play: ANIM_STD_PUNCH,
            anim2_to_play: ANIM_STD_NUM as AnimationId,
            anim_loop_start: 0.0,
            anim_loop_end: 0.0,
            anim_frame_fire: 0.0,
            anim2_frame_fire: 0.0,
            model_id: -1,
            flags: WEAPONFLAG_USE_GRAVITY | WEAPONFLAG_SLOWS_DOWN | WEAPONFLAG_RAND_SPEED
                | WEAPONFLAG_EXPANDS | WEAPONFLAG_EXPLODES,
        }
    }
}

impl Default for CWeaponInfoStore {
    fn default() -> Self {
        Self::Initialise()
    }
}

impl CWeaponInfo {
    pub fn IsFlagSet(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }
}

impl CWeaponInfoStore {
    pub fn Initialise() -> Self {
        Self {
            weapon_info: [CWeaponInfo::default(); WEAPONTYPE_TOTALWEAPONS],
        }
    }

    pub fn LoadWeaponData(&mut self, data: &WeaponDat) {
        for record in &data.records {
            let Some(weapon_type) = Self::FindWeaponType(&record.weapon_name) else {
                bevy::log::error!(target: "rust_gta_3::weapons", weapon = record.weapon_name,
                    "WEAPON.DAT has an unknown weapon type");
                continue;
            };
            let index = weapon_type as usize;
            if index >= WEAPONTYPE_TOTALWEAPONS {
                bevy::log::error!(target: "rust_gta_3::weapons", weapon = record.weapon_name,
                    "WEAPON.DAT weapon is outside the metadata table");
                continue;
            }
            if Self::FindWeaponFireType(&record.fire_type).is_none() {
                bevy::log::error!(target: "rust_gta_3::weapons", weapon = record.weapon_name,
                    fire_type = record.fire_type, "WEAPON.DAT has an unknown fire type");
                continue;
            }
            self.weapon_info[index] = weapon_info_from_record(record);
        }
    }

    pub fn from_weapon_dat(data: &WeaponDat) -> Self {
        let mut store = Self::Initialise();
        store.LoadWeaponData(data);
        store
    }

    pub fn GetWeaponInfo(&self, weapon_type: eWeaponType) -> Option<&CWeaponInfo> {
        self.weapon_info.get(weapon_type as usize)
    }

    pub fn FindWeaponFireType(name: &str) -> Option<eWeaponFire> {
        match normalise_name(name).as_str() {
            "MELEE" => Some(eWeaponFire::WEAPON_FIRE_MELEE),
            "INSTANT_HIT" | "INSTANTHIT" => Some(eWeaponFire::WEAPON_FIRE_INSTANT_HIT),
            "PROJECTILE" => Some(eWeaponFire::WEAPON_FIRE_PROJECTILE),
            "AREA_EFFECT" | "AREA" => Some(eWeaponFire::WEAPON_FIRE_AREA_EFFECT),
            "USE" => Some(eWeaponFire::WEAPON_FIRE_USE),
            _ => None,
        }
    }

    pub fn FindWeaponType(name: &str) -> Option<eWeaponType> {
        match normalise_name(name).as_str() {
            "UNARMED" => Some(eWeaponType::WEAPONTYPE_UNARMED),
            "BASEBALLBAT" | "BAT" => Some(eWeaponType::WEAPONTYPE_BASEBALLBAT),
            "COLT45" | "COLT" => Some(eWeaponType::WEAPONTYPE_COLT45),
            "UZI" => Some(eWeaponType::WEAPONTYPE_UZI),
            "SHOTGUN" => Some(eWeaponType::WEAPONTYPE_SHOTGUN),
            "AK47" => Some(eWeaponType::WEAPONTYPE_AK47),
            "M16" => Some(eWeaponType::WEAPONTYPE_M16),
            "SNIPERRIFLE" | "SNIPER" => Some(eWeaponType::WEAPONTYPE_SNIPERRIFLE),
            "ROCKETLAUNCHER" | "ROCKET" => Some(eWeaponType::WEAPONTYPE_ROCKETLAUNCHER),
            "FLAMETHROWER" | "FLAME" => Some(eWeaponType::WEAPONTYPE_FLAMETHROWER),
            "MOLOTOV" => Some(eWeaponType::WEAPONTYPE_MOLOTOV),
            "GRENADE" => Some(eWeaponType::WEAPONTYPE_GRENADE),
            "DETONATOR" => Some(eWeaponType::WEAPONTYPE_DETONATOR),
            "HELICANNON" => Some(eWeaponType::WEAPONTYPE_HELICANNON),
            _ => None,
        }
    }

    pub fn Shutdown(&mut self) {
        *self = Self::Initialise();
    }
}

fn weapon_info_from_record(record: &WeaponRecord) -> CWeaponInfo {
    CWeaponInfo {
        weapon_fire: CWeaponInfoStore::FindWeaponFireType(&record.fire_type)
            .unwrap_or(eWeaponFire::WEAPON_FIRE_INSTANT_HIT),
        range: record.range,
        firing_rate: record.firing_rate.max(0) as u32,
        reload: record.reload.max(0) as u32,
        amount_of_ammunition: record.ammo_amount,
        damage: record.damage.max(0) as u32,
        speed: record.speed,
        radius: record.radius,
        lifespan: record.life_span,
        spread: record.spread,
        fire_offset: CVector::new(
            record.fire_offset[0],
            record.fire_offset[1],
            record.fire_offset[2],
        ),
        anim_to_play: animation_id_from_name(&record.anim_to_play),
        anim2_to_play: animation_id_from_name(&record.anim2_to_play),
        anim_loop_start: record.anim_loop_start / 30.0,
        anim_loop_end: record.anim_loop_end / 30.0,
        anim_frame_fire: record.delay_between_anim_and_fire / 30.0,
        anim2_frame_fire: record.delay_between_anim2_and_fire / 30.0,
        model_id: record.model_id,
        flags: record.flags.max(0) as u32,
    }
}

fn animation_id_from_name(name: &str) -> AnimationId {
    let cleaned = normalise_name(name);
    if cleaned == "NULL" {
        return ANIM_STD_NUM as AnimationId;
    }
    crate::animation::AnimAssocDefinitions::MS_A_ANIM_ASSOC_DEFINITIONS
        .iter()
        .take(1) // CWeaponInfo::LoadWeaponData resolves names in ASSOCGRP_STD.
        .flat_map(|definition| {
            definition
                .anim_names
                .iter()
                .zip(definition.anim_descs.iter())
        })
        .find_map(|(anim_name, desc)| {
            (normalise_name(anim_name) == cleaned).then_some(desc.anim_id as AnimationId)
        })
        .unwrap_or_else(|| {
            bevy::log::error!(target: "rust_gta_3::weapons", animation = name,
                "WEAPON.DAT animation is absent from ASSOCGRP_STD");
            ANIM_STD_NUM as AnimationId
        })
}

fn normalise_name(name: &str) -> String {
    name.trim()
        .trim_matches('"')
        .replace(['-', ' '], "_")
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_rifle_fires_at_loop_start_on_every_repeat() {
        // AK47 WEAPON.DAT: loop 12..14, fire 12, authored at 30 fps.
        let info = CWeaponInfo { anim_loop_start: 12.0 / 30.0,
            anim_loop_end: 14.0 / 30.0, anim_frame_fire: 12.0 / 30.0, ..CWeaponInfo::default() };
        assert!(!info.crossed_attack_fire_frame(0.38, 0.4));
        assert!(info.crossed_attack_fire_frame(0.4, 0.42));
        for cycle in 0..6 {
            let fire = info.anim_loop_end + cycle as f32 * (info.anim_loop_end - info.anim_loop_start);
            assert!(info.crossed_attack_fire_frame(fire - 0.01, fire + 0.01), "repeat {cycle}");
            assert!(!info.crossed_attack_fire_frame(fire + 0.01, fire + 0.02), "repeat duplicated {cycle}");
        }
    }

    #[test]
    fn null_secondary_animation_is_the_native_sentinel() {
        assert_eq!(animation_id_from_name("NULL"), ANIM_STD_NUM as AnimationId);
    }

}
