//! re3 reference: references/re-GTA/src/core/Stats.cpp
//! re3 reference: references/re-GTA/src/core/Stats.h
//!
//! `CStats::Init` state.

use bevy::prelude::*;

use crate::save::SaveBuf::{SaveBufError, SaveBufReader, SaveBufWriter};

pub const TOTAL_FASTEST_TIMES: usize = 16;
pub const TOTAL_HIGHEST_SCORES: usize = 16;
pub const NUM_PEDTYPES: usize = 23;

#[derive(Debug, Clone, Resource)]
pub struct CStats {
    pub days_passed: i32,
    pub heads_popped: i32,
    pub commercial_passed: i32,
    pub industrial_passed: i32,
    pub suburban_passed: i32,
    pub number_kill_frenzies_passed: i32,
    pub people_killed_by_others: i32,
    pub helis_destroyed: i32,
    pub peds_killed_of_this_type: [i32; NUM_PEDTYPES],
    pub times_died: i32,
    pub times_arrested: i32,
    pub kills_since_last_checkpoint: i32,
    pub distance_travelled_in_vehicle: f32,
    pub distance_travelled_on_foot: f32,
    pub cars_exploded: i32,
    pub people_killed_by_player: i32,
    pub progress_made: i32,
    pub total_progress_in_game: i32,
    pub maximum_jump_distance: f32,
    pub maximum_jump_height: f32,
    pub maximum_jump_flips: i32,
    pub maximum_jump_spins: i32,
    pub best_stunt_jump: i32,
    pub number_of_unique_jumps_found: i32,
    pub total_number_of_unique_jumps: i32,
    pub passengers_dropped_off_with_taxi: i32,
    pub money_made_with_taxi: i32,
    pub missions_given: i32,
    pub missions_passed: i32,
    pub last_mission_passed_name: [u8; 8],
    pub total_legitimate_kills: i32,
    pub el_burro_time: i32,
    pub record_4x4_one: i32,
    pub record_4x4_two: i32,
    pub record_4x4_three: i32,
    pub record_4x4_mayhem: i32,
    pub lives_saved_with_ambulance: i32,
    pub criminals_caught: i32,
    pub highest_level_ambulance_mission: i32,
    pub fires_extinguished: i32,
    pub longest_flight_in_dodo: i32,
    pub time_taken_defuse_mission: i32,
    pub total_number_kill_frenzies: i32,
    pub total_number_missions: i32,
    pub rounds_fired_by_player: i32,
    pub kgs_of_explosives_used: i32,
    pub instant_hits_fired_by_player: i32,
    pub instant_hits_hit_by_player: i32,
    pub best_time_bomb_defusal: i32,
    pub mm_rain: i32,
    pub cars_crushed: i32,
    pub fastest_times: [i32; TOTAL_FASTEST_TIMES],
    pub highest_scores: [i32; TOTAL_HIGHEST_SCORES],
}

impl CStats {
    pub fn init() -> Self {
        Self {
            days_passed: 0,
            heads_popped: 0,
            commercial_passed: 0,
            industrial_passed: 0,
            suburban_passed: 0,
            number_kill_frenzies_passed: 0,
            people_killed_by_others: 0,
            helis_destroyed: 0,
            peds_killed_of_this_type: [0; NUM_PEDTYPES],
            times_died: 0,
            times_arrested: 0,
            kills_since_last_checkpoint: 0,
            distance_travelled_in_vehicle: 0.0,
            distance_travelled_on_foot: 0.0,
            cars_exploded: 0,
            people_killed_by_player: 0,
            progress_made: 0,
            total_progress_in_game: 0,
            maximum_jump_distance: 0.0,
            maximum_jump_height: 0.0,
            maximum_jump_flips: 0,
            maximum_jump_spins: 0,
            best_stunt_jump: 0,
            number_of_unique_jumps_found: 0,
            total_number_of_unique_jumps: 0,
            passengers_dropped_off_with_taxi: 0,
            money_made_with_taxi: 0,
            missions_given: 0,
            missions_passed: 0,
            last_mission_passed_name: [0; 8],
            total_legitimate_kills: 0,
            el_burro_time: 0,
            record_4x4_one: 0,
            record_4x4_two: 0,
            record_4x4_three: 0,
            record_4x4_mayhem: 0,
            lives_saved_with_ambulance: 0,
            criminals_caught: 0,
            highest_level_ambulance_mission: 0,
            fires_extinguished: 0,
            longest_flight_in_dodo: 0,
            time_taken_defuse_mission: 0,
            total_number_kill_frenzies: 0,
            total_number_missions: 0,
            rounds_fired_by_player: 0,
            kgs_of_explosives_used: 0,
            instant_hits_fired_by_player: 0,
            instant_hits_hit_by_player: 0,
            best_time_bomb_defusal: 0,
            mm_rain: 0,
            cars_crushed: 0,
            fastest_times: [0; TOTAL_FASTEST_TIMES],
            highest_scores: [0; TOTAL_HIGHEST_SCORES],
        }
    }

    pub fn register_fastest_time(&mut self, index: i32, time: i32) {
        if let Some(record) = usize::try_from(index)
            .ok()
            .and_then(|index| self.fastest_times.get_mut(index))
        {
            *record = if *record == 0 {
                time
            } else {
                (*record).min(time)
            };
        }
    }

    pub fn register_highest_score(&mut self, index: i32, score: i32) {
        if let Some(record) = usize::try_from(index)
            .ok()
            .and_then(|index| self.highest_scores.get_mut(index))
        {
            *record = (*record).max(score);
        }
    }

    pub fn register_el_burro_time(&mut self, time: i32) {
        self.el_burro_time = minimum_nonzero(self.el_burro_time, time);
    }

    pub fn register_4x4_time(&mut self, race: usize, time: i32) {
        let record = match race {
            0 => &mut self.record_4x4_one,
            1 => &mut self.record_4x4_two,
            2 => &mut self.record_4x4_three,
            _ => &mut self.record_4x4_mayhem,
        };
        *record = minimum_nonzero(*record, time);
    }

    pub fn register_ambulance_level(&mut self, level: i32) {
        self.highest_level_ambulance_mission = self.highest_level_ambulance_mission.max(level);
    }

    pub fn register_longest_dodo_flight(&mut self, time: i32) {
        self.longest_flight_in_dodo = self.longest_flight_in_dodo.max(time);
    }

    pub fn register_defuse_bomb_time(&mut self, time: i32) {
        self.time_taken_defuse_mission = minimum_nonzero(self.time_taken_defuse_mission, time);
    }

    pub fn save_stats(&self) -> Vec<u8> {
        let mut writer = SaveBufWriter::new();
        writer.write(self.people_killed_by_player);
        writer.write(self.people_killed_by_others);
        writer.write(self.cars_exploded);
        writer.write(self.rounds_fired_by_player);
        write_i32_array(&mut writer, &self.peds_killed_of_this_type);
        writer.write(self.helis_destroyed);
        writer.write(self.progress_made);
        writer.write(self.total_progress_in_game);
        writer.write(self.kgs_of_explosives_used);
        writer.write(self.instant_hits_fired_by_player);
        writer.write(self.instant_hits_hit_by_player);
        writer.write(self.cars_crushed);
        writer.write(self.heads_popped);
        writer.write(self.times_arrested);
        writer.write(self.times_died);
        writer.write(self.days_passed);
        writer.write(self.mm_rain);
        writer.write(self.maximum_jump_distance);
        writer.write(self.maximum_jump_height);
        writer.write(self.maximum_jump_flips);
        writer.write(self.maximum_jump_spins);
        writer.write(self.best_stunt_jump);
        writer.write(self.number_of_unique_jumps_found);
        writer.write(self.total_number_of_unique_jumps);
        writer.write(self.missions_given);
        writer.write(self.missions_passed);
        writer.write(self.passengers_dropped_off_with_taxi);
        writer.write(self.money_made_with_taxi);
        writer.write(self.industrial_passed);
        writer.write(self.commercial_passed);
        writer.write(self.suburban_passed);
        writer.write(self.el_burro_time);
        writer.write(self.distance_travelled_on_foot);
        writer.write(self.distance_travelled_in_vehicle);
        writer.write(self.record_4x4_one);
        writer.write(self.record_4x4_two);
        writer.write(self.record_4x4_three);
        writer.write(self.record_4x4_mayhem);
        writer.write(self.lives_saved_with_ambulance);
        writer.write(self.criminals_caught);
        writer.write(self.highest_level_ambulance_mission);
        writer.write(self.fires_extinguished);
        writer.write(self.longest_flight_in_dodo);
        writer.write(self.time_taken_defuse_mission);
        writer.write(self.number_kill_frenzies_passed);
        writer.write(self.total_number_kill_frenzies);
        writer.write(self.total_number_missions);
        write_i32_array(&mut writer, &self.fastest_times);
        write_i32_array(&mut writer, &self.highest_scores);
        writer.write(self.kills_since_last_checkpoint);
        writer.write(self.total_legitimate_kills);
        for byte in self.last_mission_passed_name {
            writer.write(byte);
        }
        writer.into_inner()
    }

    pub fn load_stats(bytes: &[u8]) -> Result<Self, SaveBufError> {
        let mut out = Self::init();
        let mut reader = SaveBufReader::new(bytes);
        out.people_killed_by_player = reader.read::<i32>()?;
        out.people_killed_by_others = reader.read::<i32>()?;
        out.cars_exploded = reader.read::<i32>()?;
        out.rounds_fired_by_player = reader.read::<i32>()?;
        read_i32_array(&mut reader, &mut out.peds_killed_of_this_type)?;
        out.helis_destroyed = reader.read::<i32>()?;
        out.progress_made = reader.read::<i32>()?;
        out.total_progress_in_game = reader.read::<i32>()?;
        out.kgs_of_explosives_used = reader.read::<i32>()?;
        out.instant_hits_fired_by_player = reader.read::<i32>()?;
        out.instant_hits_hit_by_player = reader.read::<i32>()?;
        out.cars_crushed = reader.read::<i32>()?;
        out.heads_popped = reader.read::<i32>()?;
        out.times_arrested = reader.read::<i32>()?;
        out.times_died = reader.read::<i32>()?;
        out.days_passed = reader.read::<i32>()?;
        out.mm_rain = reader.read::<i32>()?;
        out.maximum_jump_distance = reader.read::<f32>()?;
        out.maximum_jump_height = reader.read::<f32>()?;
        out.maximum_jump_flips = reader.read::<i32>()?;
        out.maximum_jump_spins = reader.read::<i32>()?;
        out.best_stunt_jump = reader.read::<i32>()?;
        out.number_of_unique_jumps_found = reader.read::<i32>()?;
        out.total_number_of_unique_jumps = reader.read::<i32>()?;
        out.missions_given = reader.read::<i32>()?;
        out.missions_passed = reader.read::<i32>()?;
        out.passengers_dropped_off_with_taxi = reader.read::<i32>()?;
        out.money_made_with_taxi = reader.read::<i32>()?;
        out.industrial_passed = reader.read::<i32>()?;
        out.commercial_passed = reader.read::<i32>()?;
        out.suburban_passed = reader.read::<i32>()?;
        out.el_burro_time = reader.read::<i32>()?;
        out.distance_travelled_on_foot = reader.read::<f32>()?;
        out.distance_travelled_in_vehicle = reader.read::<f32>()?;
        out.record_4x4_one = reader.read::<i32>()?;
        out.record_4x4_two = reader.read::<i32>()?;
        out.record_4x4_three = reader.read::<i32>()?;
        out.record_4x4_mayhem = reader.read::<i32>()?;
        out.lives_saved_with_ambulance = reader.read::<i32>()?;
        out.criminals_caught = reader.read::<i32>()?;
        out.highest_level_ambulance_mission = reader.read::<i32>()?;
        out.fires_extinguished = reader.read::<i32>()?;
        out.longest_flight_in_dodo = reader.read::<i32>()?;
        out.time_taken_defuse_mission = reader.read::<i32>()?;
        out.number_kill_frenzies_passed = reader.read::<i32>()?;
        out.total_number_kill_frenzies = reader.read::<i32>()?;
        out.total_number_missions = reader.read::<i32>()?;
        read_i32_array(&mut reader, &mut out.fastest_times)?;
        read_i32_array(&mut reader, &mut out.highest_scores)?;
        out.kills_since_last_checkpoint = reader.read::<i32>()?;
        out.total_legitimate_kills = reader.read::<i32>()?;
        for byte in &mut out.last_mission_passed_name {
            *byte = reader.read::<u8>()?;
        }
        Ok(out)
    }
}

fn minimum_nonzero(current: i32, value: i32) -> i32 {
    if current != 0 && current < value {
        current
    } else {
        value
    }
}

fn write_i32_array<const N: usize>(writer: &mut SaveBufWriter, values: &[i32; N]) {
    for value in values {
        writer.write(*value);
    }
}

fn read_i32_array<const N: usize>(
    reader: &mut SaveBufReader<'_>,
    values: &mut [i32; N],
) -> Result<(), SaveBufError> {
    for value in values {
        *value = reader.read::<i32>()?;
    }
    Ok(())
}
