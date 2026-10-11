//! re3 reference: references/re-GTA/src/control/Script5.cpp
//!
//! Compatible CTheScripts::SaveAllScripts / LoadAllScripts payloads.

use std::fmt;
use std::collections::HashSet;

use super::Script::{CRunningScript, CTheScripts};
use super::ScriptTypes::{
    MAX_NUM_BUILDING_SWAPS, MAX_NUM_COLLECTIVES, MAX_NUM_CONTACTS, MAX_NUM_INVISIBILITY_SETTINGS,
    MAX_NUM_SCRIPTS, ScriptInvisibilityTarget,
};
use crate::core::Pools::{CPoolHandle, CPools};
use crate::control::ScriptEntities::ScriptEntityWorld;
use crate::save::SaveBuf::{SAVE_HEADER_SIZE, SaveBufError, SaveBufReader, SaveBufWriter};
use crate::core::World::{CWorld, EEntityType};
use re3bevy_formats::formats::Script::ScmFile;

const SCRIPT_DATA_SIZE: usize = 968;
const RUNNING_SCRIPT_SAVE_SIZE: usize = 136;

#[derive(Debug)]
pub enum ScriptSaveError {
    Buffer(SaveBufError),
    Invalid(&'static str),
}

impl fmt::Display for ScriptSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(error) => write!(f, "{error}"),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ScriptSaveError {}

impl From<SaveBufError> for ScriptSaveError {
    fn from(value: SaveBufError) -> Self {
        Self::Buffer(value)
    }
}

impl CTheScripts {
    /// re3 CTheScripts::SaveAllScripts.
    pub fn save_all_scripts(&self) -> Result<Vec<u8>, ScriptSaveError> {
        self.save_all_scripts_with_bindings(None, None)
    }

    pub fn save_all_scripts_with_world(&self, world: &CWorld) -> Result<Vec<u8>, ScriptSaveError> {
        self.save_all_scripts_with_bindings(Some(world), None)
    }

    pub fn save_all_scripts_with_live_objects(
        &self,
        world: &CWorld,
        live_object_handles: &HashSet<i32>,
    ) -> Result<Vec<u8>, ScriptSaveError> {
        self.save_all_scripts_with_bindings(Some(world), Some(live_object_handles))
    }

    fn save_all_scripts_with_bindings(
        &self,
        world: Option<&CWorld>,
        live_object_handles: Option<&HashSet<i32>>,
    ) -> Result<Vec<u8>, ScriptSaveError> {
        let var_space = self.variable_space_size;
        if var_space > self.script_space.len() {
            return Err(ScriptSaveError::Invalid(
                "script variable space exceeds loaded MAIN.SCM buffer",
            ));
        }
        if self.active_scripts.len() > MAX_NUM_SCRIPTS {
            return Err(ScriptSaveError::Invalid("too many active scripts to save"));
        }

        let size = SAVE_HEADER_SIZE
            + 3 * size_of::<u32>()
            + var_space
            + SCRIPT_DATA_SIZE
            + RUNNING_SCRIPT_SAVE_SIZE * self.active_scripts.len();
        let mut writer = SaveBufWriter::new();
        writer.write_header(*b"SCR\0", (size - SAVE_HEADER_SIZE) as u32);
        writer.write(var_space as u32);
        writer.write_bytes(&self.script_space[..var_space]);
        writer.write(SCRIPT_DATA_SIZE as u32);
        writer.write(self.on_a_mission_flag);

        for index in 0..MAX_NUM_CONTACTS {
            writer.write(self.on_a_mission_for_contact_flag[index]);
            writer.write(self.base_brief_id_for_contact[index]);
        }
        for index in 0..MAX_NUM_COLLECTIVES {
            let collective = self
                .collectives
                .get(index)
                .ok_or(ScriptSaveError::Invalid("collective array is undersized"))?;
            writer.write(collective.col_index);
            writer.write(collective.ped_index);
        }
        writer.write(self.next_free_collective_index as u32);

        for index in 0..MAX_NUM_BUILDING_SWAPS {
            let swap = self
                .building_swaps
                .get(index)
                .ok_or(ScriptSaveError::Invalid(
                    "building swap array is undersized",
                ))?;
            let (entity_type, handle) = match (swap.building_ref, world) {
                (Some(index), Some(world)) => save_world_entity(world, index, true)?,
                (Some(index), None) if index >= 0 => (2_u32, index as u32 + 1),
                _ => (0_u32, 0_u32),
            };
            writer.write(entity_type);
            writer.write(handle);
            writer.write(swap.new_model);
            writer.write(swap.old_model);
        }

        for index in 0..MAX_NUM_INVISIBILITY_SETTINGS {
            let entity = self
                .invisibility_settings
                .get(index)
                .ok_or(ScriptSaveError::Invalid("invisibility array is undersized"))?;
            let (entity_type, handle) = match (entity, world, live_object_handles) {
                (Some(ScriptInvisibilityTarget::ScriptObject(handle)), _, Some(live))
                    if !live.contains(handle) => (0_u32, 0_u32),
                (Some(ScriptInvisibilityTarget::WorldEntity(index)), Some(world), _) => {
                    save_world_entity(world, *index, false)?
                }
                (Some(ScriptInvisibilityTarget::WorldEntity(index)), None, _) if *index >= 0 => {
                    (2_u32, *index as u32 + 1)
                }
                (Some(ScriptInvisibilityTarget::ScriptObject(handle)), _, _) => {
                    let slot = CPoolHandle(*handle).index();
                    (3_u32, slot as u32 + 1)
                }
                (Some(ScriptInvisibilityTarget::ObjectPoolSlot(slot)), _, _) => {
                    (3_u32, *slot as u32 + 1)
                }
                _ => (0_u32, 0_u32),
            };
            writer.write(entity_type);
            writer.write(handle);
        }

        writer.write(u8::from(self.using_a_multi_script_file));
        writer.zero(3);
        writer.write(self.main_script_size);
        writer.write(self.largest_mission_script_size);
        writer.write(self.number_of_mission_scripts);
        writer.zero(2);
        writer.write(self.active_scripts.len() as u32);

        for &index in &self.active_scripts {
            let script = self
                .scripts
                .get(index)
                .ok_or(ScriptSaveError::Invalid("active script index is invalid"))?;
            save_running_script(&mut writer, script);
        }

        debug_assert_eq!(writer.offset(), size);
        Ok(writer.into_inner())
    }

    /// re3 CTheScripts::LoadAllScripts.
    pub fn load_all_scripts(&mut self, bytes: &[u8]) -> Result<(), ScriptSaveError> {
        // A malformed save must not overwrite live globals or destroy threads.
        // Loading is infrequent; stage the owned SCM state and commit once.
        let mut staged = self.clone();
        staged.load_all_scripts_checked(bytes, None)?;
        *self = staged;
        Ok(())
    }

    pub fn load_all_scripts_with_world(&mut self, bytes: &[u8], world: &mut CWorld) -> Result<(), ScriptSaveError> {
        // CTheScripts::LoadAllScripts begins with Init: recreate transient VM
        // state from the already loaded installation MAIN.SCM, then restore
        // saved globals and threads. Pending effects from the previous session
        // must not execute after loading another save.
        let scm = ScmFile::try_parse_main_scm(&self.main_scm_bytes)
            .map_err(|_| ScriptSaveError::Invalid("original MAIN.SCM cannot be reinitialised"))?;
        let mut staged = Self::from_scm(scm, self.main_scm_bytes.clone());
        // UpdateObjectIndices depends on IDE metadata. Preserve the resolved
        // indices for the same MAIN.SCM until the live adapter refreshes them.
        staged.used_objects.clone_from(&self.used_objects);
        staged.object_indices_updated = self.object_indices_updated;
        staged.load_all_scripts_checked(bytes, Some(world))?;
        for swap in &staged.building_swaps {
            if swap.building_ref.is_some() {
                if swap.new_model < 0 {
                    return Err(ScriptSaveError::Invalid("saved building replacement model is invalid"));
                }
            }
        }
        for swap in &staged.building_swaps {
            if let Some(index) = swap.building_ref {
                world.replace_entity_model(index as usize, swap.new_model as usize);
            }
        }
        for target in staged.invisibility_settings.iter().flatten() {
            if let ScriptInvisibilityTarget::WorldEntity(index) = target {
                world.entities[*index as usize].is_visible = false;
            }
        }
        staged.static_area_world_entity_count = usize::MAX;
        *self = staged;
        Ok(())
    }

    fn load_all_scripts_checked(&mut self, bytes: &[u8], world: Option<&CWorld>) -> Result<(), ScriptSaveError> {
        let mut reader = SaveBufReader::new(bytes);
        let payload_size =
            bytes
                .len()
                .checked_sub(SAVE_HEADER_SIZE)
                .ok_or(ScriptSaveError::Invalid(
                    "script save block is shorter than its header",
                ))?;
        reader.check_header(*b"SCR\0", payload_size as u32)?;

        let var_space = reader.read::<u32>()? as usize;
        if var_space > self.script_space.len() {
            return Err(ScriptSaveError::Invalid(
                "saved script variable space exceeds loaded MAIN.SCM buffer",
            ));
        }
        self.variable_space_size = var_space;
        self.script_space[..var_space].copy_from_slice(reader.read_bytes(var_space)?);

        if reader.read::<u32>()? as usize != SCRIPT_DATA_SIZE {
            return Err(ScriptSaveError::Invalid(
                "script data size does not match re3",
            ));
        }
        self.on_a_mission_flag = reader.read::<u32>()?;

        for index in 0..MAX_NUM_CONTACTS {
            self.on_a_mission_for_contact_flag[index] = reader.read::<i32>()?;
            self.base_brief_id_for_contact[index] = reader.read::<i32>()?;
        }
        if self.collectives.len() < MAX_NUM_COLLECTIVES {
            return Err(ScriptSaveError::Invalid("collective array is undersized"));
        }
        for index in 0..MAX_NUM_COLLECTIVES {
            self.collectives[index].col_index = reader.read::<i32>()?;
            self.collectives[index].ped_index = reader.read::<i32>()?;
        }
        self.next_free_collective_index = reader.read::<u32>()? as usize;
        if self.next_free_collective_index > i32::MAX as usize {
            return Err(ScriptSaveError::Invalid("saved collective allocation index is invalid"));
        }

        if self.building_swaps.len() < MAX_NUM_BUILDING_SWAPS {
            return Err(ScriptSaveError::Invalid(
                "building swap array is undersized",
            ));
        }
        for index in 0..MAX_NUM_BUILDING_SWAPS {
            let entity_type = reader.read::<u32>()?;
            let handle = reader.read::<u32>()?;
            if entity_type > 2 {
                return Err(ScriptSaveError::Invalid("invalid saved building type"));
            }
            self.building_swaps[index].building_ref = load_world_entity(world, entity_type, handle)?;
            self.building_swaps[index].new_model = reader.read::<i32>()?;
            self.building_swaps[index].old_model = reader.read::<i32>()?;
        }

        if self.invisibility_settings.len() < MAX_NUM_INVISIBILITY_SETTINGS {
            return Err(ScriptSaveError::Invalid("invisibility array is undersized"));
        }
        for index in 0..MAX_NUM_INVISIBILITY_SETTINGS {
            let entity_type = reader.read::<u32>()?;
            let handle = reader.read::<u32>()?;
            if entity_type > 4 {
                return Err(ScriptSaveError::Invalid(
                    "invalid saved invisible entity type",
                ));
            }
            self.invisibility_settings[index] =
                load_invisibility_target(world, entity_type, handle)?;
        }

        let using_multi_script = reader.read::<u8>()? != 0;
        reader.skip(3)?;
        if using_multi_script != self.using_a_multi_script_file {
            return Err(ScriptSaveError::Invalid(
                "saved multi-script mode does not match MAIN.SCM",
            ));
        }
        if reader.read::<u32>()? != self.main_script_size {
            return Err(ScriptSaveError::Invalid(
                "saved main script size does not match MAIN.SCM",
            ));
        }
        if reader.read::<u32>()? != self.largest_mission_script_size {
            return Err(ScriptSaveError::Invalid(
                "saved mission script size does not match MAIN.SCM",
            ));
        }
        if reader.read::<u16>()? != self.number_of_mission_scripts {
            return Err(ScriptSaveError::Invalid(
                "saved mission script count does not match MAIN.SCM",
            ));
        }
        reader.skip(2)?;

        let running_scripts = reader.read::<u32>()? as usize;
        if running_scripts > MAX_NUM_SCRIPTS {
            return Err(ScriptSaveError::Invalid(
                "saved active script count is invalid",
            ));
        }
        if reader.remaining() != running_scripts * RUNNING_SCRIPT_SAVE_SIZE {
            return Err(ScriptSaveError::Invalid(
                "saved running-script payload has the wrong size",
            ));
        }

        self.active_scripts.clear();
        self.idle_scripts.clear();
        self.idle_scripts.extend(0..self.scripts.len());
        for script in &mut self.scripts {
            script.init();
        }
        self.active_script_count = 0;
        self.idle_script_count = self.idle_scripts.len();

        for _ in 0..running_scripts {
            let script_index = self
                .start_new_script(0)
                .map_err(|_| ScriptSaveError::Invalid("no idle script slot while loading"))?;
            load_running_script(&mut reader, &mut self.scripts[script_index])?;
            let script = &self.scripts[script_index];
            if script.stack_pointer as usize > script.stack.len() {
                return Err(ScriptSaveError::Invalid("saved script stack pointer is invalid"));
            }
            if script.ip as usize >= self.script_space.len()
                || script.stack.iter().take(script.stack_pointer as usize)
                    .any(|ip| *ip as usize >= self.script_space.len())
            {
                return Err(ScriptSaveError::Invalid("saved script instruction address is invalid"));
            }
        }
        Ok(())
    }
}

impl CTheScripts {
    /// re3 `CTheScripts::AddToInvisibilitySwapArray`, with a pool-safe
    /// identity for dynamic ECS objects.
    pub fn add_to_invisibility_swap_array(
        &mut self,
        target: ScriptInvisibilityTarget,
        remove: bool,
    ) {
        if let Some(index) = self
            .invisibility_settings
            .iter()
            .position(|entry| *entry == Some(target))
        {
            if remove {
                self.invisibility_settings[index] = None;
            }
            return;
        }
        if !remove {
            if let Some(index) = self.invisibility_settings.iter().position(Option::is_none) {
                self.invisibility_settings[index] = Some(target);
            } else {
                bevy::log::warn_once!(target: "re3bevy::script_visibility", "script_invisibility_reference_capacity_exhausted");
            }
        }
    }

    /// re3 `CTheScripts::UndoEntityInvisibilitySettings`.
    pub fn undo_entity_invisibility_settings(
        &mut self,
        world: &mut CWorld,
        script_entities: &mut ScriptEntityWorld<'_, '_>,
    ) {
        for target in self.invisibility_settings.iter_mut().filter_map(Option::take) {
            match target {
                ScriptInvisibilityTarget::WorldEntity(index) => {
                    if let Ok(index) = usize::try_from(index) {
                        world.set_entity_visible(index, true);
                    }
                }
                ScriptInvisibilityTarget::ScriptObject(handle) => {
                    if let Some(object) = script_entities.object_get_mut(handle) {
                        object.object.physical.entity.flags.is_visible = true;
                    }
                }
                ScriptInvisibilityTarget::ObjectPoolSlot(_) => {}
            }
        }
    }

    /// Resolve type-3 compatible references after `CPools::LoadObjectPool` has
    /// recreated the canonical generation-bearing handles and ECS objects.
    pub(crate) fn resolve_loaded_object_invisibility(
        &mut self,
        pools: &CPools,
        script_entities: &mut ScriptEntityWorld<'_, '_>,
    ) -> Result<(), ScriptSaveError> {
        for target in self.invisibility_settings.iter_mut().flatten() {
            let ScriptInvisibilityTarget::ObjectPoolSlot(slot) = *target else {
                continue;
            };
            let handle = pools
                .get_object_ref(slot)
                .ok_or(ScriptSaveError::Invalid("saved SCM object pool slot is unavailable"))?;
            let Some(object) = script_entities.object_get_mut(handle.0) else {
                return Err(ScriptSaveError::Invalid(
                    "saved SCM object pool slot has no restored ECS object",
                ));
            };
            object.object.physical.entity.flags.is_visible = false;
            *target = ScriptInvisibilityTarget::ScriptObject(handle.0);
        }
        Ok(())
    }
}

fn save_world_entity(world: &CWorld, index: i32, building_only: bool) -> Result<(u32, u32), ScriptSaveError> {
    let entity = usize::try_from(index).ok().and_then(|index| world.entities.get(index))
        .ok_or(ScriptSaveError::Invalid("saved SCM world entity is unavailable"))?;
    let (kind, slot) = match entity.entity_type {
        EEntityType::Building => {
            if let Some(slot) = entity.treadable_pool_slot {
                (1, slot)
            } else {
                (2, entity.building_pool_slot.ok_or(ScriptSaveError::Invalid("SCM building has no pool slot"))?)
            }
        }
        EEntityType::Object if !building_only => {
            return Err(ScriptSaveError::Invalid(
                "SCM world object has no live CObject pool identity",
            ));
        }
        EEntityType::Dummy if !building_only => {
            let slot = world.object_dummy_pool.iter().position(|value| *value == index as usize)
                .ok_or(ScriptSaveError::Invalid("SCM dummy has no pool slot"))?;
            (4, slot)
        }
        _ => return Err(ScriptSaveError::Invalid("SCM save reference has the wrong entity kind")),
    };
    Ok((kind, slot as u32 + 1))
}

fn load_invisibility_target(
    world: Option<&CWorld>,
    entity_type: u32,
    handle: u32,
) -> Result<Option<ScriptInvisibilityTarget>, ScriptSaveError> {
    if entity_type == 0 {
        return if handle == 0 {
            Ok(None)
        } else {
            Err(ScriptSaveError::Invalid(
                "null SCM entity has a nonzero handle",
            ))
        };
    }
    if entity_type == 3 {
        let slot = handle
            .checked_sub(1)
            .ok_or(ScriptSaveError::Invalid("SCM object pool handle is zero"))?
            as usize;
        return Ok(Some(ScriptInvisibilityTarget::ObjectPoolSlot(slot)));
    }
    load_world_entity(world, entity_type, handle)
        .map(|entity| entity.map(ScriptInvisibilityTarget::WorldEntity))
}

fn load_world_entity(world: Option<&CWorld>, kind: u32, handle: u32) -> Result<Option<i32>, ScriptSaveError> {
    if kind == 0 {
        return if handle == 0 { Ok(None) } else { Err(ScriptSaveError::Invalid("null SCM entity has a nonzero handle")) };
    }
    let slot = handle.checked_sub(1).ok_or(ScriptSaveError::Invalid("SCM entity handle is zero"))? as usize;
    let Some(world) = world else { return Ok(Some(slot as i32)) };
    let pool = match kind {
        1 => &world.treadable_pool,
        2 => &world.building_pool,
        3 => &world.map_objects,
        4 => &world.object_dummy_pool,
        _ => return Err(ScriptSaveError::Invalid("SCM entity kind is invalid")),
    };
    let index = *pool.get(slot).ok_or(ScriptSaveError::Invalid("saved SCM world pool slot is unavailable"))?;
    if index >= world.entities.len() {
        return Err(ScriptSaveError::Invalid("saved SCM world pool points outside the world"));
    }
    Ok(Some(index as i32))
}

fn save_running_script(writer: &mut SaveBufWriter, script: &CRunningScript) {
    let start = writer.offset();
    writer.zero(8);
    writer.write_bytes(&script.script_name);
    writer.write(script.ip);
    for value in script.stack {
        writer.write(value);
    }
    writer.write(script.stack_pointer);
    writer.zero(2);
    for value in script.local_variables {
        writer.write(value);
    }
    writer.write(u8::from(script.cond_result));
    writer.write(u8::from(script.is_mission_script));
    writer.write(u8::from(script.skip_wake_time));
    writer.zero(1);
    writer.write(script.wake_time);
    writer.write(script.and_or_state);
    writer.write(u8::from(script.not_flag));
    writer.write(u8::from(script.deatharrest_enabled));
    writer.write(u8::from(script.deatharrest_executed));
    writer.write(u8::from(script.mission_flag));
    writer.zero(2);
    debug_assert_eq!(writer.offset() - start, RUNNING_SCRIPT_SAVE_SIZE);
}

fn load_running_script(
    reader: &mut SaveBufReader<'_>,
    script: &mut CRunningScript,
) -> Result<(), ScriptSaveError> {
    reader.skip(8)?;
    script.script_name.copy_from_slice(reader.read_bytes(8)?);
    script.ip = reader.read::<u32>()?;
    for value in &mut script.stack {
        *value = reader.read::<u32>()?;
    }
    script.stack_pointer = reader.read::<u16>()?;
    reader.skip(2)?;
    for value in &mut script.local_variables {
        *value = reader.read::<i32>()?;
    }
    script.cond_result = reader.read::<u8>()? != 0;
    script.is_mission_script = reader.read::<u8>()? != 0;
    script.skip_wake_time = reader.read::<u8>()? != 0;
    reader.skip(1)?;
    script.wake_time = reader.read::<u32>()?;
    script.and_or_state = reader.read::<u16>()?;
    script.not_flag = reader.read::<u8>()? != 0;
    script.deatharrest_enabled = reader.read::<u8>()? != 0;
    script.deatharrest_executed = reader.read::<u8>()? != 0;
    script.mission_flag = reader.read::<u8>()? != 0;
    reader.skip(2)?;
    script.skip_message_wait_this_frame = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ScriptTypes::{MAX_NUM_SCRIPTS, NUM_LOCAL_VARS, NUM_TIMERS};

    #[test]
    fn running_script_compatible_layout_is_136_bytes() {
        let mut writer = SaveBufWriter::new();
        let script = CRunningScript::new();
        save_running_script(&mut writer, &script);
        assert_eq!(writer.offset(), RUNNING_SCRIPT_SAVE_SIZE);
        assert_eq!(NUM_LOCAL_VARS + NUM_TIMERS, 18);
        assert_eq!(MAX_NUM_SCRIPTS, 128);
    }

    #[test]
    fn running_script_roundtrip_preserves_re3_saved_state() {
        let mut script = CRunningScript::new();
        script.script_name = *b"mission\0";
        script.ip = 0x1234_5678;
        script.stack = [11, 22, 33, 44, 55, 66];
        script.stack_pointer = 3;
        script.local_variables = std::array::from_fn(|index| index as i32 * 17 - 100);
        script.cond_result = true;
        script.is_mission_script = true;
        script.skip_wake_time = true;
        script.wake_time = 45_678;
        script.and_or_state = 23;
        script.not_flag = true;
        script.deatharrest_enabled = false;
        script.deatharrest_executed = true;
        script.mission_flag = true;

        let mut writer = SaveBufWriter::new();
        save_running_script(&mut writer, &script);
        let bytes = writer.into_inner();
        let mut reader = SaveBufReader::new(&bytes);
        let mut loaded = CRunningScript::new();
        load_running_script(&mut reader, &mut loaded).unwrap();

        assert_eq!(loaded, script);
        assert_eq!(reader.remaining(), 0);
    }
}
