//! re3 reference: references/re-GTA/src/core/Pad.cpp
//! re3 reference: references/re-GTA/src/core/Pad.h
//! re3 reference: references/re-GTA/src/skel/events.cpp
//!
//! GTA III PC and controller cheat codes implemented in Bevy ECS.
//! Matches original reverse-string rolling comparison in `CPad::AddToPCCheatString`
//! and controller button transitions in `CPad::AddToCheatString`.

use bevy::{
    input::{ButtonInput, keyboard::KeyCode},
    prelude::*,
};
use rustgta3_core::core::{Clock::CClock, Coordinates::CVector, General::CGeneral, Timer::CTimer};

use crate::{
    control::{
        ScriptEntities::{ScriptEntityIndex, ScriptVehicleEntity},
        ScriptTypes::CTheScripts,
    },
    core::{
        FileLoader::CFileLoader,
        Pad::{CControllerState, CPads},
        PlayerInfo::CPlayerInfo,
        Pools::{CPoolEntry, CPoolEntryKind, CPools},
    },
    entities::Entity::eEntityStatus,
    modelinfo::ModelIndices::{MI_MALE01, MI_RHINO},
    peds::{
        Ped::give_ped_weapon_with_info,
        PedType::{
            CPedTypeRuntime, PED_FLAG_CIVFEMALE, PED_FLAG_CIVMALE, PED_FLAG_COP, PED_FLAG_CRIMINAL,
            PED_FLAG_EMERGENCY, PED_FLAG_GANG1, PED_FLAG_GANG2, PED_FLAG_GANG3, PED_FLAG_GANG4,
            PED_FLAG_GANG5, PED_FLAG_GANG6, PED_FLAG_GANG7, PED_FLAG_GANG8, PED_FLAG_GANG9,
            PED_FLAG_PLAYER1, PED_FLAG_PLAYER2, PED_FLAG_PLAYER3, PED_FLAG_PLAYER4,
            PED_FLAG_PROSTITUTE, PED_FLAG_SPECIAL, ePedType,
        },
        PlayerPed::{CWorldPlayers, WorldPlayerAccess},
    },
    renderer::{
        Hud::CHud,
        Weather::{CWeather, WEATHER_CLOUDY, WEATHER_FOGGY, WEATHER_RAINY, WEATHER_SUNNY},
    },
    text::Text::{CText, ascii_to_unicode},
    vehicles::{
        Automobile::{CAutomobile, eBombType},
        HandlingMgr::cHandlingDataMgr,
        Vehicle::{RANDOM_VEHICLE, eCarLock},
    },
    weapons::{WeaponInfo::CWeaponInfoStore, WeaponType::eWeaponType},
};

pub const MI_CAS_WOM: usize = 82;

/// All cheat codes supported in GTA III.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheatCode {
    Weapon,
    Money,
    Health,
    WantedLevelUp,
    WantedLevelDown,
    Tank,
    BlowUpCars,
    ChangePlayer,
    Mayhem,
    EverybodyAttacksPlayer,
    WeaponsForAll,
    FastTime,
    SlowTime,
    Armour,
    SunnyWeather,
    CloudyWeather,
    RainyWeather,
    FoggyWeather,
    FastWeather,
    OnlyRenderWheels,
    ChittyChittyBangBang,
    StrongGrip,
    NastyLimbs,
    Kangaroo,
}

impl CheatCode {
    /// Original GTA III GXT help message key.
    pub fn help_message_key(&self) -> &'static str {
        match self {
            Self::Weapon => "CHEAT2",
            Self::Health => "CHEAT3",
            Self::Armour => "CHEAT4",
            Self::WantedLevelUp | Self::WantedLevelDown => "CHEAT5",
            Self::Money => "CHEAT6",
            Self::SunnyWeather | Self::CloudyWeather | Self::RainyWeather | Self::FoggyWeather => {
                "CHEAT7"
            }
            _ => "CHEAT1",
        }
    }
}

/// Global cheat state matching re3 C++ globals:
/// - `CVehicle::bWheelsOnlyCheat`
/// - `CVehicle::bAllDodosCheat`
/// - `CVehicle::bCheat3`
/// - `CPed::bNastyLimbsCheat`
/// - `CPopulation::ms_bGivePedsWeapons`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Resource)]
pub struct CCheatState {
    pub wheels_only: bool,
    pub all_dodos: bool,
    pub strong_grip: bool,
    pub nasty_limbs: bool,
    pub weapons_for_all: bool,
    pub mayhem: bool,
    pub everybody_attacks_player: bool,
    pub kangaroo: bool,
}

impl CCheatState {
    /// Matches `CPad::ResetCheats()`.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// re3 CPad PC and controller rolling cheat string buffers.
/// `KeyBoardCheatString[20]` and `CheatString[12]`.
#[derive(Debug, Clone, Resource)]
pub struct CCheatBuffer {
    pub keyboard_cheat_string: [char; 20],
    pub controller_cheat_string: [char; 12],
}

impl Default for CCheatBuffer {
    fn default() -> Self {
        Self {
            keyboard_cheat_string: ['\0'; 20],
            controller_cheat_string: ['\0'; 12],
        }
    }
}

/// Reversed cheat strings from `CPad::AddToPCCheatString`.
/// New keystrokes are prepended at index 0, so strings match in reverse.
const PC_CHEATS: &[(&str, CheatCode)] = &[
    ("SNUGSNUGSNUG", CheatCode::Weapon),            // "GUNSGUNSGUNS"
    ("NAMHCIRAEREWIFI", CheatCode::Money),          // "IFIWEREARICHMAN"
    ("TIEHDNUSEG", CheatCode::Health),              // "GESUNDHEIT"
    ("ESAELPECILOPEROM", CheatCode::WantedLevelUp), // "MOREPOLICEPLEASE"
    ("ESAELPECILOPON", CheatCode::WantedLevelDown), // "NOPOLICEPLEASE"
    ("KNATASUEVIG", CheatCode::Tank),               // "GIVEUSATANK"
    ("GNABGNABGNAB", CheatCode::BlowUpCars),        // "BANGBANGBANG"
    ("PUGNISSERDEKILI", CheatCode::ChangePlayer),   // "ILIKEDRESSINGUP"
    ("DAAAMGNIOGLLASTI", CheatCode::Mayhem),        // "ITSALLGOINGMAAAD"
    ("EMSEKILYDOBON", CheatCode::EverybodyAttacksPlayer), // "NOBODYLIKESME"
    ("LLAROFSNOPAEW", CheatCode::WeaponsForAll),    // "WEAPONSFORALL"
    ("UOYNEHWSEILFEMIT", CheatCode::FastTime),      // "TIMEFLIESWHENYOU"
    ("GNIROOOOOB", CheatCode::SlowTime),            // "BOOOOORING"
    ("ESIOTRUT", CheatCode::Armour),                // "TURTOISE" (PC 1.0)
    ("ESIOTROT", CheatCode::Armour),                // "TORTOISE" (PC 1.1)
    ("EMROFRECNACNIKS", CheatCode::SunnyWeather),   // "SKINCANCERFORME"
    ("DNALTOCSEKILI", CheatCode::CloudyWeather),    // "ILIKESCOTLAND"
    ("DNALTOCSEVOLI", CheatCode::RainyWeather),     // "ILOVESCOTLAND"
    ("PUOSAEP", CheatCode::FoggyWeather),           // "PEASOUP"
    ("REHTAEWDAM", CheatCode::FastWeather),         // "MADWEATHER"
    ("SLEEHWFOTESECINA", CheatCode::OnlyRenderWheels), // "ANICESETOFWHEELS"
    ("BBYTTIHCYTTIHC", CheatCode::ChittyChittyBangBang), // "CHITTYCHITTYBB"
    ("DAMEKILSRENROC", CheatCode::StrongGrip),      // "CORNERSLIKEMAD"
    ("TAEHCSBMILYTSAN", CheatCode::NastyLimbs),     // "NASTYLIMBSCHEAT"
    ("OORAGNAK", CheatCode::Kangaroo),              // "KANGAROO"
];

/// Controller button sequences from `CPad::AddToCheatString`.
const CONTROLLER_CHEATS: &[(&str, CheatCode)] = &[
    ("URDLURDL4144", CheatCode::Weapon),
    ("URDLURDL1144", CheatCode::Money),
    ("URDLURDL2144", CheatCode::Armour),
    ("URDLURDL3144", CheatCode::Health),
    ("RLRLRL4144", CheatCode::WantedLevelUp),
    ("DUDUDU4144", CheatCode::WantedLevelDown),
    ("T2344321", CheatCode::SunnyWeather),
    ("S2344321", CheatCode::CloudyWeather),
    ("C2344321", CheatCode::RainyWeather),
    ("X2344321", CheatCode::FoggyWeather),
    ("TCT123CCCCCC", CheatCode::Tank),
    ("TCT1SSSSSCCC", CheatCode::FastWeather),
    ("12TCST423142", CheatCode::BlowUpCars),
    ("RDLU21ULDR", CheatCode::ChangePlayer),
    ("1243XULUD", CheatCode::Mayhem),
    ("2143XULUD", CheatCode::EverybodyAttacksPlayer),
    ("DU12XT34", CheatCode::WeaponsForAll),
    ("21SDRUT", CheatCode::FastTime),
    ("43SDRUT", CheatCode::SlowTime),
    ("T1T4S11", CheatCode::OnlyRenderWheels),
    ("31D23C4R", CheatCode::ChittyChittyBangBang),
    ("T33L1413", CheatCode::StrongGrip),
    ("X1RT31DC1S", CheatCode::NastyLimbs),
];

impl CCheatBuffer {
    /// Mirrors `CPad::AddToPCCheatString(char c)`.
    pub fn add_to_pc_cheat_string(&mut self, c: char) -> Option<CheatCode> {
        let c = c.to_ascii_uppercase();
        for i in (0..self.keyboard_cheat_string.len() - 1).rev() {
            self.keyboard_cheat_string[i + 1] = self.keyboard_cheat_string[i];
        }
        self.keyboard_cheat_string[0] = c;

        for &(pattern, code) in PC_CHEATS {
            if self.matches_pc(pattern) {
                return Some(code);
            }
        }
        None
    }

    fn matches_pc(&self, pattern: &str) -> bool {
        for (i, ch) in pattern.chars().enumerate() {
            if i >= self.keyboard_cheat_string.len() || self.keyboard_cheat_string[i] != ch {
                return false;
            }
        }
        true
    }

    /// Mirrors `CPad::AddToCheatString(char c)` for controller inputs.
    pub fn add_to_controller_cheat_string(&mut self, c: char) -> Option<CheatCode> {
        for i in (0..self.controller_cheat_string.len() - 1).rev() {
            self.controller_cheat_string[i + 1] = self.controller_cheat_string[i];
        }
        self.controller_cheat_string[0] = c;

        for &(pattern, code) in CONTROLLER_CHEATS {
            if self.matches_controller(pattern) {
                return Some(code);
            }
        }
        None
    }

    fn matches_controller(&self, pattern: &str) -> bool {
        for (i, ch) in pattern.chars().enumerate() {
            if i >= self.controller_cheat_string.len() || self.controller_cheat_string[i] != ch {
                return false;
            }
        }
        true
    }

    pub fn reset(&mut self) {
        self.keyboard_cheat_string.fill('\0');
        self.controller_cheat_string.fill('\0');
    }
}

/// Convert Bevy `KeyCode` to standard uppercase ASCII character.
pub fn keycode_to_cheat_char(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::KeyA => Some('A'),
        KeyCode::KeyB => Some('B'),
        KeyCode::KeyC => Some('C'),
        KeyCode::KeyD => Some('D'),
        KeyCode::KeyE => Some('E'),
        KeyCode::KeyF => Some('F'),
        KeyCode::KeyG => Some('G'),
        KeyCode::KeyH => Some('H'),
        KeyCode::KeyI => Some('I'),
        KeyCode::KeyJ => Some('J'),
        KeyCode::KeyK => Some('K'),
        KeyCode::KeyL => Some('L'),
        KeyCode::KeyM => Some('M'),
        KeyCode::KeyN => Some('N'),
        KeyCode::KeyO => Some('O'),
        KeyCode::KeyP => Some('P'),
        KeyCode::KeyQ => Some('Q'),
        KeyCode::KeyR => Some('R'),
        KeyCode::KeyS => Some('S'),
        KeyCode::KeyT => Some('T'),
        KeyCode::KeyU => Some('U'),
        KeyCode::KeyV => Some('V'),
        KeyCode::KeyW => Some('W'),
        KeyCode::KeyX => Some('X'),
        KeyCode::KeyY => Some('Y'),
        KeyCode::KeyZ => Some('Z'),
        _ => None,
    }
}

use bevy::ecs::system::SystemParam;

/// SystemParam grouping world access for cheats, keeping system argument count small.
#[derive(SystemParam)]
pub struct CheatWorldContext<'w, 's> {
    pub weapon_info: Option<Res<'w, CWeaponInfoStore>>,
    pub world_players: CWorldPlayers<'w, 's>,
    pub player_info: Option<ResMut<'w, CPlayerInfo>>,
    pub hud: Option<ResMut<'w, CHud>>,
    pub text: Option<Res<'w, CText>>,
    pub weather: Option<ResMut<'w, CWeather>>,
    pub timer: ResMut<'w, CTimer>,
    pub clock: Option<ResMut<'w, CClock>>,
    pub scripts: Option<ResMut<'w, CTheScripts>>,
    pub ped_types: Option<ResMut<'w, CPedTypeRuntime>>,
    pub vehicles_query: crate::control::ScriptEntities::ScriptVehicleStore<'w, 's>,
    pub commands: Commands<'w, 's>,
    pub file_loader: Option<Res<'w, CFileLoader>>,
    pub handling_manager: Option<Res<'w, cHandlingDataMgr>>,
    pub pools: Option<ResMut<'w, CPools>>,
    pub general: Option<ResMut<'w, CGeneral>>,
    pub script_index: Option<ResMut<'w, ScriptEntityIndex>>,
}

/// System that checks keyboard and gamepad button inputs, detects cheat codes,
/// executes cheat actions, and posts HUD help messages.
pub fn process_cheats(
    mut cheat_buffer: ResMut<CCheatBuffer>,
    mut cheat_state: ResMut<CCheatState>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Option<Res<CPads>>,
    mut last_pad_state: Local<CControllerState>,
    mut ctx: CheatWorldContext,
) {
    let mut detected_cheats = Vec::new();

    // 1. Keyboard keys pressed this frame
    if let Some(keys) = keys {
        for &key in keys.get_just_pressed() {
            if let Some(c) = keycode_to_cheat_char(key) {
                if let Some(cheat) = cheat_buffer.add_to_pc_cheat_string(c) {
                    detected_cheats.push(cheat);
                }
            }
        }
    }

    // 2. Gamepad / Controller transitions on pad 0
    if let Some(pads) = pads {
        let cur = pads.get_pad(0).new_state;
        let prev = *last_pad_state;

        let mut check_btn = |is_now: bool, was: bool, tag: char| {
            if is_now && !was {
                if let Some(cheat) = cheat_buffer.add_to_controller_cheat_string(tag) {
                    detected_cheats.push(cheat);
                }
            }
        };

        check_btn(cur.triangle != 0, prev.triangle != 0, 'T');
        check_btn(cur.circle != 0, prev.circle != 0, 'C');
        check_btn(cur.cross != 0, prev.cross != 0, 'X');
        check_btn(cur.square != 0, prev.square != 0, 'S');
        check_btn(cur.dpad_up != 0, prev.dpad_up != 0, 'U');
        check_btn(cur.dpad_down != 0, prev.dpad_down != 0, 'D');
        check_btn(cur.dpad_left != 0, prev.dpad_left != 0, 'L');
        check_btn(cur.dpad_right != 0, prev.dpad_right != 0, 'R');
        check_btn(cur.left_shoulder1 != 0, prev.left_shoulder1 != 0, '1');
        check_btn(cur.left_shoulder2 != 0, prev.left_shoulder2 != 0, '2');
        check_btn(cur.right_shoulder1 != 0, prev.right_shoulder1 != 0, '3');
        check_btn(cur.right_shoulder2 != 0, prev.right_shoulder2 != 0, '4');

        *last_pad_state = cur;
    }

    // 3. Apply each detected cheat
    for cheat in detected_cheats {
        apply_cheat_effect(cheat, &mut cheat_state, &mut ctx);
    }
}

/// Apply the gameplay outcome of an activated cheat.
pub fn apply_cheat_effect(
    cheat: CheatCode,
    cheat_state: &mut CCheatState,
    ctx: &mut CheatWorldContext,
) {
    let CheatWorldContext {
        weapon_info,
        world_players,
        player_info,
        hud,
        text,
        weather,
        timer,
        clock,
        scripts,
        ped_types,
        vehicles_query,
        commands,
        file_loader,
        handling_manager,
        pools,
        general,
        script_index,
    } = ctx;

    let mut custom_message_key: Option<&'static str> = None;

    match cheat {
        CheatCode::Weapon => {
            let Some(info) = weapon_info.as_deref() else {
                warn!(target: "rustgta3::cheats", "weapon_cheat_missing_weapon_data: original WEAPON.DAT store unavailable");
                return;
            };
            if let Some(mut player) = world_players.focused_mut() {
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_BASEBALLBAT as i32, 0, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_COLT45 as i32, 100, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_UZI as i32, 100, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_SHOTGUN as i32, 20, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_AK47 as i32, 200, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_M16 as i32, 200, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_SNIPERRIFLE as i32, 5, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_ROCKETLAUNCHER as i32, 5, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_MOLOTOV as i32, 5, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_GRENADE as i32, 5, info);
                give_ped_weapon_with_info(&mut player.ped, eWeaponType::WEAPONTYPE_FLAMETHROWER as i32, 200, info);
            } else {
                warn!(target: "rustgta3::cheats", "weapon_cheat_missing_player: focused player unavailable");
                return;
            }
        }

        CheatCode::Health => {
            if let Some(player) = world_players.focused_mut() {
                player.vitality.health = 100.0;
            }
            let focus_index = world_players.player_in_focus();
            if let Some(relation) = world_players.vehicle_relation(focus_index as i32) {
                if let Some(mut veh) = vehicles_query.get_entity_mut(relation.vehicle) {
                    match &mut veh {
                        crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } => {
                            car.vehicle.vitality.health = 1000.0;
                            car.damage.set_engine_status(0);
                        }
                        crate::control::ScriptEntities::ScriptVehicleMut::Boat { boat, .. } => {
                            boat.vehicle.vitality.health = 1000.0;
                        }
                        crate::control::ScriptEntities::ScriptVehicleMut::Heli { heli, .. } => {
                            heli.vehicle.vitality.health = 1000.0;
                        }
                        crate::control::ScriptEntities::ScriptVehicleMut::Plane { plane, .. } => {
                            plane.vehicle.vitality.health = 1000.0;
                        }
                    }
                }
            }
        }

        CheatCode::Armour => {
            if let Some(player) = world_players.focused_mut() {
                player.vitality.armour = 100.0;
            }
        }

        CheatCode::Money => {
            if let Some(info) = player_info.as_deref_mut() {
                info.m_nMoney = info.m_nMoney.saturating_add(250000);
            }
        }

        CheatCode::WantedLevelUp => {
            if let Some(mut player) = world_players.focused_mut() {
                let current = player.wanted.m_nWantedLevel;
                player.wanted.set_wanted_level((current + 2).min(6));
            }
        }

        CheatCode::WantedLevelDown => {
            if let Some(mut player) = world_players.focused_mut() {
                player.wanted.set_wanted_level(0);
            }
        }

        CheatCode::SunnyWeather => {
            if let Some(w) = weather.as_deref_mut() {
                w.force_weather_now(WEATHER_SUNNY);
            }
        }

        CheatCode::CloudyWeather => {
            if let Some(w) = weather.as_deref_mut() {
                w.force_weather_now(WEATHER_CLOUDY);
            }
        }

        CheatCode::RainyWeather => {
            if let Some(w) = weather.as_deref_mut() {
                w.force_weather_now(WEATHER_RAINY);
            }
        }

        CheatCode::FoggyWeather => {
            if let Some(w) = weather.as_deref_mut() {
                w.force_weather_now(WEATHER_FOGGY);
            }
        }

        CheatCode::FastWeather => {
            if let Some(c) = clock.as_deref_mut() {
                let new_fast = !c.fast_time();
                c.set_fast_time(new_fast);
            }
        }

        CheatCode::FastTime => {
            let scale = timer.time_scale();
            if scale < 4.0 {
                timer.set_time_scale(scale * 2.0);
            }
        }

        CheatCode::SlowTime => {
            let scale = timer.time_scale();
            if scale > 0.25 {
                timer.set_time_scale(scale * 0.5);
            }
        }

        CheatCode::BlowUpCars => {
            let now_ms = timer.time_in_milliseconds();
            for mut veh in vehicles_query.values_mut() {
                if !veh.base().vitality.can_be_damaged {
                    continue;
                }
                if let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. } = &mut veh {
                    car.damage.fuck_car_completely();
                    car.bomb_type = eBombType::CARBOMB_NONE;
                    car.taxi_light = false;
                }
                let mut base = veh.base_mut();
                base.physical.move_speed.z += 0.13;
                base.set_status(eEntityStatus::STATUS_WRECKED);
                base.vitality.time_of_death = now_ms;
                base.vitality.health = 0.0;
                base.vitality.bomb_timer = 0;
                base.drive.engine_on = false;
                base.appearance.lights_on = false;
                base.audio.siren_or_alarm = false;
                base.flags.is_law_enforcer = false;
                base.appearance.is_damaged = true;
            }
        }

        CheatCode::Tank => {
            if let Some(player) = world_players.focused() {
                let forward = player.ped.physical.entity.get_forward();
                let player_pos = player.position();
                let mut spawn_pos = CVector::new(
                    player_pos.x + forward.x * 8.0,
                    player_pos.y + forward.y * 8.0,
                    player_pos.z + 2.0,
                );

                if let Some(loader) = file_loader.as_deref() {
                    let node = loader.path_find.find_node_closest_to_coors(
                        player_pos,
                        crate::control::PathFind::PATH_CAR,
                        100.0,
                        false,
                        false,
                    );
                    if node >= 0 && (node as usize) < loader.path_find.path_nodes.len() {
                        let mut node_pos = loader.path_find.path_nodes[node as usize].pos;
                        node_pos.z += 4.0;
                        spawn_pos = node_pos;
                    }
                }

                let model_index = MI_RHINO;
                let handling_id = if let Some(loader) = file_loader.as_deref() {
                    let model_info = loader.model_info.get_model_info(model_index);
                    let handling_name = match model_info {
                        Some(crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(v)) => {
                            v.handling_id.as_str()
                        }
                        _ => "",
                    };
                    cHandlingDataMgr::get_handling_id(handling_name)
                } else {
                    None
                };

                let handling = handling_manager
                    .as_deref()
                    .and_then(|mgr| handling_id.and_then(|id| mgr.automobile_snapshot(id)));
                let handle = pools
                    .as_deref_mut()
                    .and_then(|p| {
                        p.vehicle_pool
                            .new_entry(CPoolEntry::new(CPoolEntryKind::Vehicle))
                    })
                    .map(|h| h.0)
                    .unwrap_or(122);

                let now_ms = timer.time_in_milliseconds();
                let mut fallback_general = CGeneral::default();
                let gen_ref = general.as_deref_mut().unwrap_or(&mut fallback_general);
                let mut car = CAutomobile::new(
                    model_index as i32,
                    RANDOM_VEHICLE,
                    now_ms,
                    gen_ref,
                    handling,
                    None,
                    false,
                );
                car.vehicle.core.physical.entity.set_position(spawn_pos);
                car.vehicle.set_status(eEntityStatus::STATUS_ABANDONED);
                car.vehicle.entry.door_lock = eCarLock::CARLOCK_UNLOCKED;

                let mut actor = commands.spawn_empty();
                ScriptVehicleEntity::Automobile { handle, car }.insert_into(&mut actor);
                let entity = actor.id();

                if let Some(index) = script_index.as_deref_mut() {
                    index.vehicles.insert(handle, entity);
                }
            }
        }

        CheatCode::ChangePlayer => {
            if let Some(player) = world_players.focused_mut() {
                let rand = if let Some(rng) = general.as_deref_mut() {
                    rng.get_random_number() as usize
                } else {
                    (player.ped.physical.entity.model_index.max(0) as usize) + 1
                };
                let min_m = MI_MALE01;
                let max_m = MI_CAS_WOM;
                let new_model = min_m + (rand % (max_m - min_m + 1));
                player.ped.physical.entity.model_index = new_model as i16;
            }
        }

        CheatCode::Mayhem => {
            cheat_state.mayhem = !cheat_state.mayhem;
            if let Some(ped_types) = ped_types.as_deref_mut() {
                let threats = PED_FLAG_PLAYER1
                    | PED_FLAG_PLAYER2
                    | PED_FLAG_PLAYER3
                    | PED_FLAG_PLAYER4
                    | PED_FLAG_CIVMALE
                    | PED_FLAG_CIVFEMALE
                    | PED_FLAG_COP
                    | PED_FLAG_GANG1
                    | PED_FLAG_GANG2
                    | PED_FLAG_GANG3
                    | PED_FLAG_GANG4
                    | PED_FLAG_GANG5
                    | PED_FLAG_GANG6
                    | PED_FLAG_GANG7
                    | PED_FLAG_GANG8
                    | PED_FLAG_GANG9
                    | PED_FLAG_EMERGENCY
                    | PED_FLAG_PROSTITUTE
                    | PED_FLAG_CRIMINAL
                    | PED_FLAG_SPECIAL;
                for i in (ePedType::PEDTYPE_CIVMALE as usize)..(ePedType::PEDTYPE_SPECIAL as usize)
                {
                    ped_types.SetThreats(i, threats);
                }
            }
        }

        CheatCode::EverybodyAttacksPlayer => {
            cheat_state.everybody_attacks_player = !cheat_state.everybody_attacks_player;
            if let Some(ped_types) = ped_types.as_deref_mut() {
                for i in (ePedType::PEDTYPE_CIVMALE as usize)..(ePedType::PEDTYPE_SPECIAL as usize)
                {
                    ped_types.AddThreat(i, PED_FLAG_PLAYER1);
                }
            }
        }

        CheatCode::WeaponsForAll => {
            cheat_state.weapons_for_all = !cheat_state.weapons_for_all;
        }

        CheatCode::OnlyRenderWheels => {
            cheat_state.wheels_only = !cheat_state.wheels_only;
        }

        CheatCode::ChittyChittyBangBang => {
            cheat_state.all_dodos = !cheat_state.all_dodos;
            if let Some(scripts) = scripts.as_deref_mut() {
                scripts.car_cheats_activated = cheat_state.all_dodos || cheat_state.strong_grip;
            }
        }

        CheatCode::StrongGrip => {
            cheat_state.strong_grip = !cheat_state.strong_grip;
            if let Some(scripts) = scripts.as_deref_mut() {
                scripts.car_cheats_activated = cheat_state.all_dodos || cheat_state.strong_grip;
            }
        }

        CheatCode::NastyLimbs => {
            cheat_state.nasty_limbs = !cheat_state.nasty_limbs;
        }

        CheatCode::Kangaroo => {
            cheat_state.kangaroo = !cheat_state.kangaroo;
            if let Some(player) = world_players.focused_mut() {
                if cheat_state.kangaroo {
                    player.ped.physical.mass = 15.0;
                    player.ped.physical.air_resistance = 0.4 / 15.0;
                    custom_message_key = Some("CHEAT1");
                } else {
                    player.ped.physical.mass = 70.0;
                    player.ped.physical.air_resistance = 0.4 / 70.0;
                    custom_message_key = Some("CHEATOF");
                }
            }
        }
    }

    // 4. Update HUD message
    if let Some(hud) = hud.as_deref_mut() {
        let key = custom_message_key.unwrap_or_else(|| cheat.help_message_key());
        let msg = text
            .as_deref()
            .map(|t| t.get(key))
            .unwrap_or_else(|| ascii_to_unicode(key));
        hud.set_help_message(&msg, true);
    }
}

/// Reset all active cheats and restore normal simulation state.
pub fn reset_cheats(
    cheat_state: &mut CCheatState,
    weather: Option<&mut CWeather>,
    clock: Option<&mut CClock>,
    scripts: Option<&mut CTheScripts>,
) {
    cheat_state.reset();
    if let Some(weather) = weather {
        weather.release_weather();
    }
    if let Some(clock) = clock {
        clock.set_fast_time(false);
    }
    if let Some(scripts) = scripts {
        scripts.car_cheats_activated = false;
    }
}

pub struct GtaCheatsPlugin;

impl Plugin for GtaCheatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CCheatBuffer>()
            .init_resource::<CCheatState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pc_cheat_code_detection_gunsgunsguns() {
        let mut buffer = CCheatBuffer::default();
        let mut last = None;
        for c in "GUNSGUNSGUNS".chars() {
            last = buffer.add_to_pc_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Weapon));
    }

    #[test]
    fn pc_cheat_code_detection_gesundheit() {
        let mut buffer = CCheatBuffer::default();
        let mut last = None;
        for c in "GESUNDHEIT".chars() {
            last = buffer.add_to_pc_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Health));
    }

    #[test]
    fn pc_cheat_code_detection_ifiwerearichman() {
        let mut buffer = CCheatBuffer::default();
        let mut last = None;
        for c in "ifiwerearichman".chars() {
            last = buffer.add_to_pc_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Money));
    }

    #[test]
    fn pc_cheat_code_detection_both_tortoise_and_turtoise() {
        let mut buffer = CCheatBuffer::default();
        let mut last = None;
        for c in "TURTOISE".chars() {
            last = buffer.add_to_pc_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Armour));

        buffer.reset();
        for c in "TORTOISE".chars() {
            last = buffer.add_to_pc_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Armour));
    }

    #[test]
    fn pc_cheat_code_detection_weather_and_time() {
        let mut buffer = CCheatBuffer::default();
        for c in "SKINCANCERFORME".chars() {
            if let Some(code) = buffer.add_to_pc_cheat_string(c) {
                assert_eq!(code, CheatCode::SunnyWeather);
            }
        }
        buffer.reset();
        for c in "TIMEFLIESWHENYOU".chars() {
            if let Some(code) = buffer.add_to_pc_cheat_string(c) {
                assert_eq!(code, CheatCode::FastTime);
            }
        }
        buffer.reset();
        for c in "BOOOOORING".chars() {
            if let Some(code) = buffer.add_to_pc_cheat_string(c) {
                assert_eq!(code, CheatCode::SlowTime);
            }
        }
    }

    #[test]
    fn controller_cheat_code_detection() {
        let mut buffer = CCheatBuffer::default();
        let mut last = None;
        // WeaponCheat: R2 R2 L1 R2 Left Down Right Up Left Down Right Up ("4414LDRULDRU")
        for c in "4414LDRULDRU".chars() {
            last = buffer.add_to_controller_cheat_string(c);
        }
        assert_eq!(last, Some(CheatCode::Weapon));
    }

    #[test]
    fn keycode_to_cheat_char_covers_full_alphabet() {
        assert_eq!(keycode_to_cheat_char(KeyCode::KeyA), Some('A'));
        assert_eq!(keycode_to_cheat_char(KeyCode::KeyZ), Some('Z'));
        assert_eq!(keycode_to_cheat_char(KeyCode::Space), None);
    }

    #[test]
    fn cheat_state_reset_clears_all_flags() {
        let mut state = CCheatState {
            wheels_only: true,
            all_dodos: true,
            strong_grip: true,
            nasty_limbs: true,
            weapons_for_all: true,
            mayhem: true,
            everybody_attacks_player: true,
            kangaroo: true,
        };
        state.reset();
        assert_eq!(state, CCheatState::default());
    }

    #[test]
    fn process_cheats_system_executes_pc_cheat() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<CCheatBuffer>();
        app.init_resource::<CCheatState>();
        app.init_resource::<CTimer>();
        app.insert_resource(CPlayerInfo {
            m_nMoney: 1000,
            ..CPlayerInfo::default()
        });
        app.add_systems(bevy::app::Update, process_cheats);

        for key in [
            KeyCode::KeyI,
            KeyCode::KeyF,
            KeyCode::KeyI,
            KeyCode::KeyW,
            KeyCode::KeyE,
            KeyCode::KeyR,
            KeyCode::KeyE,
            KeyCode::KeyA,
            KeyCode::KeyR,
            KeyCode::KeyI,
            KeyCode::KeyC,
            KeyCode::KeyH,
            KeyCode::KeyM,
            KeyCode::KeyA,
            KeyCode::KeyN,
        ] {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.reset_all();
            input.press(key);
            app.update();
        }

        let player_info = app.world().resource::<CPlayerInfo>();
        assert_eq!(player_info.m_nMoney, 251000);
    }
}
