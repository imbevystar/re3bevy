//! re3 reference: references/re-GTA/src/control/Phones.cpp
//! re3 reference: references/re-GTA/src/control/Phones.h
//!
//! `CPhoneInfo::Initialise` state.

use bevy::prelude::*;
use rustgta3_core::core::{Coordinates::CVector, Timer::CTimer};

use crate::control::Script::{CTheScripts, ScriptCommandEffect};
use crate::core::World::{CWorld, EEntityType};
use crate::peds::{Ped::PedState, PlayerPed::CWorldPlayers};
use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};
use crate::text::{Messages::CMessages, Text::CText};

pub const NUMPHONES: usize = 50;
pub const PHONE_SAVE_SIZE: u32 = 0x34;
pub const PHONEINFO_SAVE_SIZE: u32 = 0xA30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PhoneState {
    Free = 0,
    ReportingCrime = 1,
    State2 = 2,
    MessageRemoved = 3,
    OneTimeMessageSet = 4,
    RepeatedMessageSet = 5,
    RepeatedMessageShownOnce = 6,
    OneTimeMessageStarted = 7,
    RepeatedMessageStarted = 8,
    State9 = 9,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CPhone {
    pub pos: CVector,
    pub message_keys: [[u8; 8]; 6],
    pub repeated_message_pickup_start: u32,
    pub entity_index: i32,
    pub state: PhoneState,
    pub visible_to_cam: bool,
}

impl CPhone {
    pub const fn init() -> Self {
        Self {
            pos: CVector::ZERO,
            message_keys: [[0; 8]; 6],
            repeated_message_pickup_start: 0,
            entity_index: 0,
            state: PhoneState::Free,
            visible_to_cam: false,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CPhoneInfo {
    pub callback_ped_present: bool,
    pub displaying_phone_message: bool,
    pub picking_up_phone: bool,
    pub phone_displaying_messages_present: bool,
    pub phone_displaying_message_slot: Option<usize>,
    pub phone_enable_controls_timer: u32,
    pub max: i32,
    pub script_phones_max: i32,
    pub building_pool_scanned: bool,
    pub phones: [CPhone; NUMPHONES],
    pub phone_positions: Vec<CVector>,
}

impl CPhoneInfo {
    pub fn initialise_without_building_pool() -> Self {
        Self {
            callback_ped_present: false,
            displaying_phone_message: false,
            picking_up_phone: false,
            phone_displaying_messages_present: false,
            phone_displaying_message_slot: None,
            phone_enable_controls_timer: 0,
            max: 0,
            script_phones_max: 0,
            building_pool_scanned: false,
            phones: [CPhone::init(); NUMPHONES],
            phone_positions: Vec::new(),
        }
    }

    pub fn initialise_from_world(world: &CWorld) -> Self {
        let phone_entries: Vec<(usize, CVector)> = world
            .building_pool
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(building_pool_slot, entity_index)| {
                let entity = world.entities.get(*entity_index)?;
                (entity.entity_type == EEntityType::Building
                    && entity.name.eq_ignore_ascii_case("phonebooth1"))
                .then_some((
                    building_pool_slot,
                    CVector::new(entity.position[0], entity.position[1], entity.position[2]),
                ))
            })
            .collect();
        let mut phones = [CPhone::init(); NUMPHONES];
        for (slot, (building_pool_slot, pos)) in
            phone_entries.iter().copied().take(NUMPHONES).enumerate()
        {
            phones[slot] = CPhone {
                pos,
                entity_index: building_pool_slot as i32 + 1,
                ..CPhone::init()
            };
        }
        let phone_positions: Vec<CVector> = phone_entries
            .iter()
            .take(NUMPHONES)
            .map(|(_, pos)| *pos)
            .collect();

        Self {
            max: phone_positions.len() as i32,
            phones,
            phone_positions,
            building_pool_scanned: true,
            ..Self::initialise_without_building_pool()
        }
    }

    pub fn save(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write(self.max);
        writer.write(self.script_phones_max);
        for phone in self.phones {
            write_phone(&mut writer, phone);
        }
        writer.into_inner()
    }

    pub fn load(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut out = Self::initialise_without_building_pool();
        let mut reader = SaveBufReader::new(bytes);
        out.max = reader.read::<i32>()?;
        out.script_phones_max = reader.read::<i32>()?;
        for phone in &mut out.phones {
            *phone = read_phone(&mut reader)?;
        }
        out.phone_positions = out
            .phones
            .iter()
            .take(out.max.clamp(0, NUMPHONES as i32) as usize)
            .map(|phone| phone.pos)
            .collect();
        out.building_pool_scanned = true;
        Ok(out)
    }

    pub fn find_nearest_free_phone(&self, pos: CVector) -> i32 {
        let mut nearest_phone_id = -1;
        let mut nearest_phone_dist = 60.0_f32;
        for (phone_id, phone) in self
            .phones
            .iter()
            .take(self.max.clamp(0, NUMPHONES as i32) as usize)
            .enumerate()
        {
            if phone.state != PhoneState::Free {
                continue;
            }
            let dx = phone.pos.x - pos.x;
            let dy = phone.pos.y - pos.y;
            let phone_dist = (dx * dx + dy * dy).sqrt();
            if phone_dist < nearest_phone_dist {
                nearest_phone_dist = phone_dist;
                nearest_phone_id = phone_id as i32;
            }
        }
        nearest_phone_id
    }

    pub fn phone_at_this_position(&self, pos: CVector) -> bool {
        self.phones
            .iter()
            .take(self.max.clamp(0, NUMPHONES as i32) as usize)
            .any(|phone| phone.pos.x == pos.x && phone.pos.y == pos.y)
    }

    pub fn has_message_been_displayed(&self, phone_id: usize) -> bool {
        if self.displaying_phone_message {
            return false;
        }
        self.phones.get(phone_id).is_some_and(|phone| {
            matches!(
                phone.state,
                PhoneState::RepeatedMessageShownOnce
                    | PhoneState::OneTimeMessageStarted
                    | PhoneState::RepeatedMessageStarted
            )
        })
    }

    pub fn is_message_being_displayed(&self, phone_id: usize) -> bool {
        self.phone_displaying_message_slot == Some(phone_id)
    }

    pub fn set_phone_messages(
        &mut self,
        phone_id: usize,
        keys: [[u8; 8]; 6],
        count: usize,
        repeated: bool,
    ) {
        let Some(phone) = self.phones.get_mut(phone_id) else {
            return;
        };
        phone.message_keys = [[0; 8]; 6];
        for (dst, src) in phone
            .message_keys
            .iter_mut()
            .zip(keys.into_iter())
            .take(count.min(6))
        {
            *dst = src;
        }
        phone.state = if repeated {
            PhoneState::RepeatedMessageSet
        } else {
            PhoneState::OneTimeMessageSet
        };
    }

    pub fn remove_phone_message(&mut self, phone_id: usize) {
        let Some(phone) = self.phones.get_mut(phone_id) else {
            return;
        };
        phone.message_keys = [[0; 8]; 6];
        phone.state = PhoneState::MessageRemoved;
    }

    pub fn grab_phone(&mut self, x: f32, y: f32) -> i32 {
        let start = self.script_phones_max.clamp(0, NUMPHONES as i32) as usize;
        let end = self.max.clamp(0, NUMPHONES as i32) as usize;
        if start >= end {
            return -1;
        }

        let mut nearest_phone_id = None;
        let mut nearest_phone_dist = 100.0_f32;
        for phone_id in start..end {
            let phone = self.phones[phone_id];
            let dx = phone.pos.x - x;
            let dy = phone.pos.y - y;
            let phone_dist = (dx * dx + dy * dy).sqrt();
            if phone_dist < nearest_phone_dist {
                nearest_phone_dist = phone_dist;
                nearest_phone_id = Some(phone_id);
            }
        }
        let Some(nearest_phone_id) = nearest_phone_id else {
            return -1;
        };

        self.phones[nearest_phone_id].state = PhoneState::MessageRemoved;
        self.phones.swap(start, nearest_phone_id);
        self.script_phones_max += 1;
        self.refresh_phone_positions();
        self.script_phones_max - 1
    }

    pub fn update_repeated_message_states(&mut self, now_ms: u32) {
        let script_phones_max = self.script_phones_max.clamp(0, NUMPHONES as i32) as usize;
        for phone in self.phones.iter_mut().take(script_phones_max) {
            if phone.state == PhoneState::RepeatedMessageStarted
                && now_ms.wrapping_sub(phone.repeated_message_pickup_start) > 60_000
            {
                phone.state = PhoneState::RepeatedMessageShownOnce;
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.max = 0;
        self.script_phones_max = 0;
        self.phone_positions.clear();
    }

    pub fn sync_script_mirror(&self, scripts: &mut CTheScripts) {
        scripts.phone_max = self.max;
        scripts.script_phones_max = self.script_phones_max;
        scripts.phone_displaying_message = self.displaying_phone_message;
        scripts.phone_displaying_message_slot = self
            .phone_displaying_message_slot
            .and_then(|slot| i32::try_from(slot).ok())
            .unwrap_or(-1);
        for (index, phone) in self.phones.iter().enumerate() {
            scripts.phone_positions[index] = phone.pos;
            scripts.phone_states[index] = phone.state as u8;
            scripts.phone_message_displayed[index] = self.has_message_been_displayed(index);
        }
    }

    fn refresh_phone_positions(&mut self) {
        self.phone_positions.clear();
        self.phone_positions.extend(
            self.phones
                .iter()
                .take(self.max.clamp(0, NUMPHONES as i32) as usize)
                .map(|phone| phone.pos),
        );
    }
}

fn write_phone(writer: &mut SaveBufWriter, phone: CPhone) {
    writer.write(phone.pos.x);
    writer.write(phone.pos.y);
    writer.write(phone.pos.z);
    writer.zero(6 * 4);
    writer.write(phone.repeated_message_pickup_start);
    writer.write(phone.entity_index);
    writer.write(phone.state as i32);
    writer.write(phone.visible_to_cam as u8);
    writer.zero(3);
}

fn read_phone(reader: &mut SaveBufReader<'_>) -> Result<CPhone, SaveBufError> {
    let pos = CVector {
        x: reader.read::<f32>()?,
        y: reader.read::<f32>()?,
        z: reader.read::<f32>()?,
    };
    reader.skip(6 * 4)?;
    let repeated_message_pickup_start = reader.read::<u32>()?;
    let entity_index = reader.read::<i32>()?;
    let state = phone_state_from_i32(reader.read::<i32>()?);
    let visible_to_cam = reader.read::<u8>()? != 0;
    reader.skip(3)?;
    Ok(CPhone {
        pos,
        message_keys: [[0; 8]; 6],
        repeated_message_pickup_start,
        entity_index,
        state,
        visible_to_cam,
    })
}

fn phone_state_from_i32(value: i32) -> PhoneState {
    match value {
        1 => PhoneState::ReportingCrime,
        2 => PhoneState::State2,
        3 => PhoneState::MessageRemoved,
        4 => PhoneState::OneTimeMessageSet,
        5 => PhoneState::RepeatedMessageSet,
        6 => PhoneState::RepeatedMessageShownOnce,
        7 => PhoneState::OneTimeMessageStarted,
        8 => PhoneState::RepeatedMessageStarted,
        9 => PhoneState::State9,
        _ => PhoneState::Free,
    }
}

// === FAZ8c-phones: moved from core/Game.rs (verbatim, parity-preserving) ===

pub(crate) fn apply_script_phone_effects(
    mut scripts: ResMut<CTheScripts>,
    mut phones: Option<ResMut<CPhoneInfo>>,
) {
    let effects = std::mem::take(&mut scripts.pending_effects);
    let mut leftover = Vec::new();
    for effect in effects {
        match effect {
            ScriptCommandEffect::GrabPhone { x, y } => {
                if let Some(phones) = phones.as_mut() {
                    phones.grab_phone(x, y);
                }
            }
            ScriptCommandEffect::SetPhoneMessages {
                phone_id,
                keys,
                count,
                repeated,
            } => {
                if let Ok(slot) = usize::try_from(phone_id)
                    && let Some(phones) = phones.as_mut()
                {
                    phones.set_phone_messages(slot, keys, count as usize, repeated);
                }
            }
            ScriptCommandEffect::TurnPhoneOff { phone_id } => {
                if let Ok(slot) = usize::try_from(phone_id)
                    && let Some(phones) = phones.as_mut()
                {
                    phones.remove_phone_message(slot);
                }
            }
            ScriptCommandEffect::TurnPhoneOn { phone_id } => {
                if let Ok(slot) = usize::try_from(phone_id)
                    && let Some(phone) = phones
                        .as_mut()
                        .and_then(|phones| phones.phones.get_mut(slot))
                {
                    phone.state = PhoneState::State9;
                }
            }
            other => leftover.push(other),
        }
    }
    scripts.pending_effects = leftover;
}

pub(crate) fn update_phone_info(
    timer: Res<CTimer>,
    mut phones: Option<ResMut<CPhoneInfo>>,
    mut players: CWorldPlayers,
    mut messages: Option<ResMut<CMessages>>,
    text: Option<Res<CText>>,
) {
    let Some(phones) = phones.as_mut() else {
        return;
    };
    let now = timer.time_in_milliseconds();
    phones.update_repeated_message_states(now);

    let focus = players.player_in_focus() as i32;
    if phones.displaying_phone_message {
        if now > phones.phone_enable_controls_timer {
            phones.displaying_phone_message = false;
            phones.phone_displaying_message_slot = None;
            if let Some(player) = players.player_mut(focus)
                && player.ped.ped_state == PedState::PED_MAKE_CALL
            {
                player.ped.ped_state = PedState::PED_IDLE;
            }
        }
        return;
    }
    if phones.picking_up_phone || players.relation(focus).is_some() {
        return;
    }
    let Some(player) = players.player(focus) else {
        return;
    };
    if !player.ped.is_ped_in_control(&player.vitality, ) {
        return;
    }
    let player_pos = player.position();
    let candidate = phones
        .phones
        .iter()
        .take(phones.script_phones_max.clamp(0, NUMPHONES as i32) as usize)
        .enumerate()
        .find(|(_, phone)| {
            matches!(
                phone.state,
                PhoneState::OneTimeMessageSet
                    | PhoneState::RepeatedMessageSet
                    | PhoneState::RepeatedMessageShownOnce
            ) && (player_pos.x - phone.pos.x).abs() < 1.0
                && (player_pos.y - phone.pos.y).abs() < 1.0
        })
        .map(|(slot, _)| slot);
    let Some(slot) = candidate else {
        return;
    };
    let (Some(messages), Some(text)) = (messages.as_mut(), text.as_deref()) else {
        return;
    };
    let keys = phones.phones[slot].message_keys;
    let mut display_time = 0_u32;
    for key in keys {
        if key[0] == 0 {
            continue;
        }
        messages.add_message(text.get_key_bytes(&key), 3000, 0, now);
        display_time = display_time.saturating_add(3000);
    }
    if display_time == 0 {
        return;
    }
    if let Some(player) = players.player_mut(focus) {
        player.ped.ped_state = PedState::PED_MAKE_CALL;
    }
    phones.picking_up_phone = false;
    phones.displaying_phone_message = true;
    phones.phone_displaying_message_slot = Some(slot);
    phones.phone_enable_controls_timer = now.saturating_add(display_time);
    let phone = &mut phones.phones[slot];
    if phone.state == PhoneState::OneTimeMessageSet {
        phone.state = PhoneState::OneTimeMessageStarted;
    } else {
        phone.state = PhoneState::RepeatedMessageStarted;
        phone.repeated_message_pickup_start = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone(pos: CVector, state: PhoneState) -> CPhone {
        CPhone {
            pos,
            state,
            ..CPhone::init()
        }
    }

    #[test]
    fn grab_phone_selects_nearest_unallocated_booth_and_swaps_it_into_prefix() {
        let mut info = CPhoneInfo::initialise_without_building_pool();
        info.max = 3;
        info.phones[0] = phone(CVector::new(0.0, 0.0, 0.0), PhoneState::Free);
        info.phones[1] = phone(CVector::new(20.0, 0.0, 0.0), PhoneState::Free);
        info.phones[2] = phone(CVector::new(5.0, 0.0, 0.0), PhoneState::Free);

        assert_eq!(info.grab_phone(6.0, 0.0), 0);
        assert_eq!(info.script_phones_max, 1);
        assert_eq!(info.phones[0].pos, CVector::new(5.0, 0.0, 0.0));
        assert_eq!(info.phones[0].state, PhoneState::MessageRemoved);
        assert_eq!(info.phones[2].pos, CVector::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn nearest_free_phone_obeys_state_and_sixty_unit_radius() {
        let mut info = CPhoneInfo::initialise_without_building_pool();
        info.max = 3;
        info.phones[0] = phone(CVector::new(10.0, 0.0, 0.0), PhoneState::MessageRemoved);
        info.phones[1] = phone(CVector::new(20.0, 0.0, 0.0), PhoneState::Free);
        info.phones[2] = phone(CVector::new(70.0, 0.0, 0.0), PhoneState::Free);

        assert_eq!(info.find_nearest_free_phone(CVector::ZERO), 1);
        assert_eq!(
            info.find_nearest_free_phone(CVector::new(-50.0, 0.0, 0.0)),
            -1
        );
    }

    #[test]
    fn phone_message_queries_match_re3_state_rules() {
        let mut info = CPhoneInfo::initialise_without_building_pool();
        info.max = 1;
        info.phones[0].state = PhoneState::OneTimeMessageStarted;
        assert!(info.has_message_been_displayed(0));
        info.displaying_phone_message = true;
        assert!(!info.has_message_been_displayed(0));
        info.phone_displaying_message_slot = Some(0);
        assert!(info.is_message_being_displayed(0));
    }

    #[test]
    fn repeated_message_becomes_available_only_after_sixty_seconds() {
        let mut info = CPhoneInfo::initialise_without_building_pool();
        info.max = 1;
        info.script_phones_max = 1;
        info.phones[0].state = PhoneState::RepeatedMessageStarted;
        info.phones[0].repeated_message_pickup_start = 1_000;

        info.update_repeated_message_states(61_000);
        assert_eq!(info.phones[0].state, PhoneState::RepeatedMessageStarted);
        info.update_repeated_message_states(61_001);
        assert_eq!(info.phones[0].state, PhoneState::RepeatedMessageShownOnce);
    }
}
