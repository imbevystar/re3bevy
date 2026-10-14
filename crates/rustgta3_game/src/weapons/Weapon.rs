//! re3 reference: references/re-GTA/src/weapons/Weapon.cpp
//! re3 reference: references/re-GTA/src/weapons/Weapon.h
//!
//! Weapon inventory slot state.

use crate::weapons::{
    WeaponInfo::CWeaponInfoStore,
    WeaponType::{eWeaponFire, eWeaponState, eWeaponType},
};

/// `gReloadSampleTime`, indexed by inventory weapon type.
pub const RELOAD_SAMPLE_TIME: [u16; 14] = [0, 0, 250, 400, 650, 300, 300, 423, 400, 0, 0, 0, 0, 0];
pub const DRIVEBYAUTOAIMING_MAXDIST: f32 = 2.5;
pub const DOOMAUTOAIMING_MAXDIST: f32 = 9_000.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponFireContext {
    pub shooter_is_player: bool,
    pub shooter_has_seek_target: bool,
    pub seek_target_distance: f32,
    pub player_attack_button_counter: f32,
    pub m16_first_person: bool,
    pub helicannon_first_person: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeaponFireRequest {
    Melee,
    InstantHit,
    Shotgun,
    Sniper,
    M16FirstPerson,
    Projectile { power: f32 },
    AreaEffect,
    UseDetonator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CWeapon {
    pub weapon_type: eWeaponType,
    pub weapon_state: eWeaponState,
    pub ammo_in_clip: i32,
    pub ammo_total: i32,
    pub timer: u32,
    pub add_rot_offset: bool,
}

impl CWeapon {
    pub fn empty_unarmed() -> Self {
        Self {
            weapon_type: eWeaponType::WEAPONTYPE_UNARMED,
            weapon_state: eWeaponState::WEAPONSTATE_READY,
            ammo_in_clip: 0,
            ammo_total: 0,
            timer: 0,
            add_rot_offset: false,
        }
    }

    pub fn initialise(type_: eWeaponType, ammo: i32) -> Self {
        Self {
            weapon_type: type_,
            weapon_state: eWeaponState::WEAPONSTATE_READY,
            ammo_in_clip: 0,
            ammo_total: ammo.min(99999),
            timer: 0,
            add_rot_offset: false,
        }
    }

    /// Full `CWeapon::Initialise`; clip capacity comes from WEAPON.DAT.
    pub fn initialise_with_info(
        type_: eWeaponType,
        ammo: i32,
        weapon_info: &CWeaponInfoStore,
    ) -> Self {
        let mut weapon = Self::initialise(type_, ammo);
        weapon.Reload(weapon_info);
        weapon
    }

    pub fn Reload(&mut self, weapon_info: &CWeaponInfoStore) {
        if self.ammo_total == 0 {
            return;
        }
        let Some(info) = weapon_info.GetWeaponInfo(self.weapon_type) else {
            return;
        };
        self.ammo_in_clip = self.ammo_total.min(info.amount_of_ammunition);
    }

    /// State/timer half of `CWeapon::Update`. Returns true at the exact frame
    /// on which re3 emits `SOUND_WEAPON_RELOAD` for a valid audio entity.
    pub fn Update(
        &mut self,
        previous_time_ms: u32,
        current_time_ms: u32,
        audio_entity_valid: bool,
        weapon_info: &CWeaponInfoStore,
    ) -> bool {
        let mut play_reload_sample = false;
        match self.weapon_state {
            eWeaponState::WEAPONSTATE_MELEE_MADECONTACT => {
                self.weapon_state = eWeaponState::WEAPONSTATE_READY;
            }
            eWeaponState::WEAPONSTATE_FIRING => {
                if self.weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN && audio_entity_valid {
                    let sample_time = self.timer.wrapping_sub(
                        RELOAD_SAMPLE_TIME[eWeaponType::WEAPONTYPE_SHOTGUN as usize] as u32,
                    );
                    play_reload_sample =
                        previous_time_ms < sample_time && current_time_ms >= sample_time;
                }
                if current_time_ms > self.timer {
                    let non_melee = weapon_info
                        .GetWeaponInfo(self.weapon_type)
                        .is_some_and(|info| info.weapon_fire != eWeaponFire::WEAPON_FIRE_MELEE);
                    self.weapon_state = if non_melee && self.ammo_total == 0 {
                        eWeaponState::WEAPONSTATE_OUT_OF_AMMO
                    } else {
                        eWeaponState::WEAPONSTATE_READY
                    };
                }
            }
            eWeaponState::WEAPONSTATE_RELOADING => {
                let type_index = self.weapon_type as usize;
                if audio_entity_valid && type_index < RELOAD_SAMPLE_TIME.len() {
                    let sample_time = self
                        .timer
                        .wrapping_sub(RELOAD_SAMPLE_TIME[type_index] as u32);
                    play_reload_sample =
                        previous_time_ms < sample_time && current_time_ms >= sample_time;
                }
                if current_time_ms > self.timer {
                    self.Reload(weapon_info);
                    self.weapon_state = eWeaponState::WEAPONSTATE_READY;
                }
            }
            _ => {}
        }
        play_reload_sample
    }

    /// re3 `CWeapon::Fire` dispatch before weapon-specific world mutation.
    /// `None` means the original function returns false before attempting fire.
    pub fn prepare_fire(
        &mut self,
        current_time_ms: u32,
        context: WeaponFireContext,
        weapon_info: &CWeaponInfoStore,
    ) -> Option<WeaponFireRequest> {
        if !matches!(
            self.weapon_state,
            eWeaponState::WEAPONSTATE_READY | eWeaponState::WEAPONSTATE_FIRING
        ) {
            return None;
        }
        let info = weapon_info.GetWeaponInfo(self.weapon_type)?;
        if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE {
            if self.weapon_state != eWeaponState::WEAPONSTATE_FIRING {
                self.timer = current_time_ms.wrapping_add(info.reload);
                self.weapon_state = eWeaponState::WEAPONSTATE_FIRING;
            }
            return Some(WeaponFireRequest::Melee);
        }
        if self.ammo_in_clip <= 0 {
            return None;
        }

        match self.weapon_type {
            eWeaponType::WEAPONTYPE_SHOTGUN => Some(WeaponFireRequest::Shotgun),
            eWeaponType::WEAPONTYPE_COLT45
            | eWeaponType::WEAPONTYPE_UZI
            | eWeaponType::WEAPONTYPE_AK47 => Some(WeaponFireRequest::InstantHit),
            eWeaponType::WEAPONTYPE_SNIPERRIFLE => Some(WeaponFireRequest::Sniper),
            eWeaponType::WEAPONTYPE_M16 => {
                Some(if context.m16_first_person && context.shooter_is_player {
                    WeaponFireRequest::M16FirstPerson
                } else {
                    WeaponFireRequest::InstantHit
                })
            }
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => {
                if context.shooter_has_seek_target
                    && context.seek_target_distance <= 8.0
                    && !context.shooter_is_player
                {
                    None
                } else {
                    Some(WeaponFireRequest::Projectile { power: 0.0 })
                }
            }
            eWeaponType::WEAPONTYPE_MOLOTOV | eWeaponType::WEAPONTYPE_GRENADE => {
                let power = if context.shooter_is_player {
                    context.player_attack_button_counter * 0.0375
                } else if context.shooter_has_seek_target {
                    ((context.seek_target_distance - 10.0) * 0.02).clamp(0.2, 1.0)
                } else {
                    0.3
                };
                Some(WeaponFireRequest::Projectile { power })
            }
            eWeaponType::WEAPONTYPE_FLAMETHROWER => Some(WeaponFireRequest::AreaEffect),
            eWeaponType::WEAPONTYPE_DETONATOR => Some(WeaponFireRequest::UseDetonator),
            eWeaponType::WEAPONTYPE_HELICANNON => Some(
                if (context.helicannon_first_person || context.m16_first_person)
                    && context.shooter_is_player
                {
                    WeaponFireRequest::M16FirstPerson
                } else {
                    WeaponFireRequest::InstantHit
                },
            ),
            _ => None,
        }
    }

    /// Complete the world-side request returned by `prepare_fire`. Detonator
    /// ammo reset occurs before common decrement exactly as in re3.
    pub fn complete_fire(
        &mut self,
        request: WeaponFireRequest,
        fired: bool,
        current_time_ms: u32,
        shooter_is_player: bool,
        fast_reload: bool,
        weapon_info: &CWeaponInfoStore,
    ) -> bool {
        if matches!(request, WeaponFireRequest::Melee) {
            return true;
        }
        if !fired {
            return false;
        }
        if matches!(request, WeaponFireRequest::UseDetonator) {
            self.ammo_total = 1;
            self.ammo_in_clip = 1;
        }
        self.finish_successful_fire(current_time_ms, shooter_is_player, fast_reload, weapon_info);
        true
    }

    /// Common post-fire bookkeeping from `CWeapon::Fire`. Weapon-specific hit,
    /// projectile or area-effect code calls this only after reporting success.
    pub fn finish_successful_fire(
        &mut self,
        current_time_ms: u32,
        shooter_is_player: bool,
        fast_reload: bool,
        weapon_info: &CWeaponInfoStore,
    ) {
        if self.ammo_in_clip > 0 {
            self.ammo_in_clip -= 1;
        }
        if self.ammo_total > 0 && (self.ammo_total < 25_000 || shooter_is_player) {
            self.ammo_total -= 1;
        }
        self.weapon_state = eWeaponState::WEAPONSTATE_FIRING;
        if self.ammo_in_clip == 0 {
            if self.ammo_total == 0 {
                return;
            }
            self.weapon_state = eWeaponState::WEAPONSTATE_RELOADING;
            let reload = weapon_info
                .GetWeaponInfo(self.weapon_type)
                .map(|info| info.reload)
                .unwrap_or(0);
            self.timer = current_time_ms.wrapping_add(if shooter_is_player && fast_reload {
                reload / 4
            } else {
                reload
            });
        } else {
            self.timer = current_time_ms.wrapping_add(1_000);
        }
    }

    pub fn IsTypeMelee(&self) -> bool {
        matches!(
            self.weapon_type,
            eWeaponType::WEAPONTYPE_UNARMED | eWeaponType::WEAPONTYPE_BASEBALLBAT
        )
    }

    pub fn IsType2Handed(&self) -> bool {
        (self.weapon_type as u8) >= eWeaponType::WEAPONTYPE_SHOTGUN as u8
            && (self.weapon_type as u8) <= eWeaponType::WEAPONTYPE_FLAMETHROWER as u8
            && self.weapon_type != eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
    }

    pub fn HasWeaponAmmoToBeUsed(&self) -> bool {
        self.IsTypeMelee() || self.ammo_total != 0
    }
}

impl Default for CWeapon {
    fn default() -> Self {
        Self {
            add_rot_offset: false,
            ..Self::empty_unarmed()
        }
    }
}
