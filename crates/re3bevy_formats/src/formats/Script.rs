//! re3 reference: references/re-GTA/src/control/Script.cpp
//! re3 reference: references/re-GTA/src/control/Script.h
//!
//! Raw SCM script-space segmentation. Opcode execution belongs to the SCM VM
//! port, not to this parser.

pub const SIZE_MAIN_SCRIPT: usize = 128 * 1024;
pub const SIZE_MISSION_SCRIPT: usize = 32 * 1024;
pub const SIZE_SCRIPT_SPACE: usize = SIZE_MAIN_SCRIPT + SIZE_MISSION_SCRIPT;
pub const MAX_NUM_MISSION_SCRIPTS: usize = 120;
pub const USED_OBJECT_NAME_LENGTH: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmFile {
    pub script_space: Vec<u8>,
    pub loaded_main_bytes: usize,
    pub variable_space_size: i32,
    pub used_objects: Vec<ScmUsedObject>,
    pub multiscript: ScmMultiScriptHeader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmUsedObject {
    pub name: [u8; USED_OBJECT_NAME_LENGTH],
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmMultiScriptHeader {
    pub object_size: i32,
    pub main_script_size: i32,
    pub largest_mission_script_size: i32,
    pub number_of_mission_scripts: i16,
    pub offsets: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScmInstructionHeader {
    pub offset: usize,
    pub opcode: u16,
    pub not_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScmParseError(pub String);

impl std::fmt::Display for ScmParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScmParseError {}

impl ScmFile {
    /// Parse and validate the complete on-disk MAIN.SCM. Production loaders
    /// use this entry point so a truncated header is never accepted through
    /// the zero-filled script-space tail.
    pub fn try_parse_main_scm(bytes: &[u8]) -> Result<Self, ScmParseError> {
        let file = Self::parse_main_scm(bytes);
        let main = file.loaded_main_bytes;
        let var_space = usize::try_from(file.variable_space_size).map_err(|_| {
            ScmParseError(format!(
                "MAIN.SCM has negative variable-space size {}",
                file.variable_space_size
            ))
        })?;
        let require_main = |end: usize, label: &str| {
            if end > main {
                Err(ScmParseError(format!(
                    "MAIN.SCM {label} ends at {end}, beyond loaded main segment {main}"
                )))
            } else {
                Ok(())
            }
        };

        require_main(var_space.saturating_add(12), "variable/object header")?;
        let object_count = read_i16_le(&file.script_space, var_space + 8).max(0) as usize;
        require_main(
            var_space
                .saturating_add(12)
                .saturating_add(object_count.saturating_mul(USED_OBJECT_NAME_LENGTH)),
            "used-object table",
        )?;

        let object_size = usize::try_from(file.multiscript.object_size).map_err(|_| {
            ScmParseError(format!(
                "MAIN.SCM has negative object-section size {}",
                file.multiscript.object_size
            ))
        })?;
        let mission_count = usize::try_from(file.multiscript.number_of_mission_scripts)
            .map_err(|_| ScmParseError("MAIN.SCM has a negative mission count".to_owned()))?;
        if mission_count > MAX_NUM_MISSION_SCRIPTS {
            return Err(ScmParseError(format!(
                "MAIN.SCM mission count {mission_count} exceeds {MAX_NUM_MISSION_SCRIPTS}"
            )));
        }
        require_main(
            object_size
                .saturating_add(20)
                .saturating_add(mission_count.saturating_mul(4)),
            "multi-script header",
        )?;
        if file.multiscript.main_script_size < 0
            || file.multiscript.main_script_size as usize > SIZE_MAIN_SCRIPT
        {
            return Err(ScmParseError(format!(
                "MAIN.SCM main script size {} exceeds fixed segment {}",
                file.multiscript.main_script_size, SIZE_MAIN_SCRIPT
            )));
        }
        if file.multiscript.largest_mission_script_size < 0
            || file.multiscript.largest_mission_script_size as usize > SIZE_MISSION_SCRIPT
        {
            return Err(ScmParseError(format!(
                "MAIN.SCM largest mission size {} exceeds fixed segment {}",
                file.multiscript.largest_mission_script_size, SIZE_MISSION_SCRIPT
            )));
        }
        for (index, &offset) in file.multiscript.offsets.iter().enumerate() {
            if offset < 0 || offset as usize >= bytes.len() {
                return Err(ScmParseError(format!(
                    "MAIN.SCM mission {index} offset {offset} is outside {} bytes",
                    bytes.len()
                )));
            }
            let end = file
                .multiscript
                .offsets
                .get(index + 1)
                .copied()
                .map_or(bytes.len(), |next| next.max(0) as usize);
            let start = offset as usize;
            if end <= start || end - start > SIZE_MISSION_SCRIPT {
                return Err(ScmParseError(format!(
                    "MAIN.SCM mission {index} range {start}..{end} is invalid for a {SIZE_MISSION_SCRIPT}-byte mission segment"
                )));
            }
        }
        Ok(file)
    }

    pub fn parse_main_scm(bytes: &[u8]) -> Self {
        let loaded_main_bytes = bytes.len().min(SIZE_MAIN_SCRIPT);
        let mut script_space = vec![0; SIZE_SCRIPT_SPACE];
        script_space[..loaded_main_bytes].copy_from_slice(&bytes[..loaded_main_bytes]);

        let variable_space_size = read_i32_le(&script_space, 3);
        let used_objects = read_used_objects(&script_space, variable_space_size);
        let multiscript = read_multiscript_header(&script_space, variable_space_size);

        Self {
            script_space,
            loaded_main_bytes,
            variable_space_size,
            used_objects,
            multiscript,
        }
    }
}

fn read_i16_le(bytes: &[u8], offset: usize) -> i16 {
    let lo = bytes.get(offset).copied().unwrap_or(0);
    let hi = bytes.get(offset + 1).copied().unwrap_or(0);
    i16::from_le_bytes([lo, hi])
}

fn read_i32_le(bytes: &[u8], offset: usize) -> i32 {
    let b0 = bytes.get(offset).copied().unwrap_or(0);
    let b1 = bytes.get(offset + 1).copied().unwrap_or(0);
    let b2 = bytes.get(offset + 2).copied().unwrap_or(0);
    let b3 = bytes.get(offset + 3).copied().unwrap_or(0);
    i32::from_le_bytes([b0, b1, b2, b3])
}

fn read_used_objects(bytes: &[u8], variable_space_size: i32) -> Vec<ScmUsedObject> {
    let Ok(var_space) = usize::try_from(variable_space_size) else {
        return Vec::new();
    };
    let mut ip = var_space + 8;
    let count = read_i16_le(bytes, ip).max(0) as usize;
    ip += 4;

    let mut objects = Vec::with_capacity(count);
    for _ in 0..count {
        let mut name = [0; USED_OBJECT_NAME_LENGTH];
        for byte in name.iter_mut() {
            *byte = bytes.get(ip).copied().unwrap_or(0);
            ip += 1;
        }
        objects.push(ScmUsedObject { name, index: 0 });
    }
    objects
}

fn read_multiscript_header(bytes: &[u8], variable_space_size: i32) -> ScmMultiScriptHeader {
    let Ok(var_space) = usize::try_from(variable_space_size) else {
        return ScmMultiScriptHeader::default();
    };
    let object_size = read_i32_le(bytes, var_space + 3);
    let Ok(object_size_offset) = usize::try_from(object_size) else {
        return ScmMultiScriptHeader {
            object_size,
            ..ScmMultiScriptHeader::default()
        };
    };

    let mut ip = object_size_offset + 8;
    let main_script_size = read_i32_le(bytes, ip);
    ip += 4;
    let largest_mission_script_size = read_i32_le(bytes, ip);
    ip += 4;
    let number_of_mission_scripts = read_i16_le(bytes, ip);
    ip += 4;

    let count = number_of_mission_scripts.max(0) as usize;
    let mut offsets = Vec::with_capacity(count.min(MAX_NUM_MISSION_SCRIPTS));
    for _ in 0..count {
        offsets.push(read_i32_le(bytes, ip));
        ip += 4;
    }

    ScmMultiScriptHeader {
        object_size,
        main_script_size,
        largest_mission_script_size,
        number_of_mission_scripts,
        offsets,
    }
}

impl Default for ScmMultiScriptHeader {
    fn default() -> Self {
        Self {
            object_size: 0,
            main_script_size: 0,
            largest_mission_script_size: 0,
            number_of_mission_scripts: 0,
            offsets: Vec::new(),
        }
    }
}

pub fn scan_instruction_headers(bytes: &[u8], max_count: usize) -> Vec<ScmInstructionHeader> {
    let mut offset = 0;
    let mut out = Vec::new();
    while offset + 2 <= bytes.len() && out.len() < max_count {
        let raw = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        out.push(ScmInstructionHeader {
            offset,
            opcode: raw & 0x7FFF,
            not_flag: raw & 0x8000 != 0,
        });
        offset += 2;
    }
    out
}
