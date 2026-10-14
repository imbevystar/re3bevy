//! re3 reference: references/re-GTA/src/weapons/WeaponType.h
//!
//! Weapon enum values used by ped and weapon state.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eWeaponType {
    WEAPONTYPE_UNARMED = 0,
    WEAPONTYPE_BASEBALLBAT = 1,
    WEAPONTYPE_COLT45 = 2,
    WEAPONTYPE_UZI = 3,
    WEAPONTYPE_SHOTGUN = 4,
    WEAPONTYPE_AK47 = 5,
    WEAPONTYPE_M16 = 6,
    WEAPONTYPE_SNIPERRIFLE = 7,
    WEAPONTYPE_ROCKETLAUNCHER = 8,
    WEAPONTYPE_FLAMETHROWER = 9,
    WEAPONTYPE_MOLOTOV = 10,
    WEAPONTYPE_GRENADE = 11,
    WEAPONTYPE_DETONATOR = 12,
    WEAPONTYPE_HELICANNON = 13,
    WEAPONTYPE_LAST_WEAPONTYPE = 14,
    WEAPONTYPE_ARMOUR = 15,
    WEAPONTYPE_RAMMEDBYCAR = 16,
    WEAPONTYPE_RUNOVERBYCAR = 17,
    WEAPONTYPE_EXPLOSION = 18,
    WEAPONTYPE_UZI_DRIVEBY = 19,
    WEAPONTYPE_DROWNING = 20,
    WEAPONTYPE_FALL = 21,
    WEAPONTYPE_UNIDENTIFIED = 22,
}

pub(crate) fn script_weapon_type(value: i32) -> Option<eWeaponType> {
    Some(match value {
        0 => eWeaponType::WEAPONTYPE_UNARMED,
        1 => eWeaponType::WEAPONTYPE_BASEBALLBAT,
        2 => eWeaponType::WEAPONTYPE_COLT45,
        3 => eWeaponType::WEAPONTYPE_UZI,
        4 => eWeaponType::WEAPONTYPE_SHOTGUN,
        5 => eWeaponType::WEAPONTYPE_AK47,
        6 => eWeaponType::WEAPONTYPE_M16,
        7 => eWeaponType::WEAPONTYPE_SNIPERRIFLE,
        8 => eWeaponType::WEAPONTYPE_ROCKETLAUNCHER,
        9 => eWeaponType::WEAPONTYPE_FLAMETHROWER,
        10 => eWeaponType::WEAPONTYPE_MOLOTOV,
        11 => eWeaponType::WEAPONTYPE_GRENADE,
        12 => eWeaponType::WEAPONTYPE_DETONATOR,
        13 => eWeaponType::WEAPONTYPE_HELICANNON,
        14 => eWeaponType::WEAPONTYPE_LAST_WEAPONTYPE,
        15 => eWeaponType::WEAPONTYPE_ARMOUR,
        16 => eWeaponType::WEAPONTYPE_RAMMEDBYCAR,
        17 => eWeaponType::WEAPONTYPE_RUNOVERBYCAR,
        18 => eWeaponType::WEAPONTYPE_EXPLOSION,
        19 => eWeaponType::WEAPONTYPE_UZI_DRIVEBY,
        20 => eWeaponType::WEAPONTYPE_DROWNING,
        21 => eWeaponType::WEAPONTYPE_FALL,
        22 => eWeaponType::WEAPONTYPE_UNIDENTIFIED,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eWeaponFire {
    WEAPON_FIRE_MELEE = 0,
    WEAPON_FIRE_INSTANT_HIT = 1,
    WEAPON_FIRE_PROJECTILE = 2,
    WEAPON_FIRE_AREA_EFFECT = 3,
    WEAPON_FIRE_USE = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum eWeaponState {
    WEAPONSTATE_READY = 0,
    WEAPONSTATE_FIRING = 1,
    WEAPONSTATE_RELOADING = 2,
    WEAPONSTATE_OUT_OF_AMMO = 3,
    WEAPONSTATE_MELEE_MADECONTACT = 4,
}

pub const WEAPONTYPE_TOTALWEAPONS: usize = eWeaponType::WEAPONTYPE_LAST_WEAPONTYPE as usize;
pub const WEAPONTYPE_TOTAL_INVENTORY_WEAPONS: usize = 13;
