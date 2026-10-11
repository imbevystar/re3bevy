//! re3 reference: references/re-GTA/src/core/FileLoader.cpp
//! re3 reference: references/re-GTA/src/core/FileLoader.h
//!
//! DAT/IDE/IPL command parsing helpers used by the runtime FileLoader port.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtaDatPlan {
    pub commands: Vec<GtaDatCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GtaDatCommand {
    ImagePath(String),
    TexDictionary(String),
    ColFile { level: i32, path: String },
    ModelFile(String),
    HierFile(String),
    Ide(String),
    Ipl(String),
    MapZone(String),
    Splash,
    CdImage(String),
    Exit,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdePlan {
    pub records: Vec<GtaIdeRecord>,
    pub invalid_vehicle_records: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GtaIdeRecord {
    Object(GtaIdeObject),
    TimeObject(GtaIdeTimeObject),
    Clump(GtaIdeClump),
    Vehicle(GtaIdeVehicle),
    Ped(GtaIdePed),
    MloHeader(GtaIdeMloHeader),
    MloInstance(GtaIdeMloInstance),
    PathHeader(GtaIdePathHeader),
    PathNode(GtaPathNode),
    TwoDfx(GtaTwoDfx),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdeObject {
    pub id: usize,
    pub model: String,
    pub txd: String,
    pub num_objs: u8,
    pub distances: [f32; 3],
    pub flags: u32,
    pub first_damaged: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdeTimeObject {
    pub object: GtaIdeObject,
    pub time_on: i32,
    pub time_off: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtaIdeClump {
    pub id: usize,
    pub model: String,
    pub txd: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdeVehicle {
    pub id: usize,
    pub model: String,
    pub txd: String,
    pub vehicle_type: String,
    pub handling_id: String,
    pub game_name: String,
    pub vehicle_class: String,
    pub frequency: u32,
    pub level: i32,
    pub component_rules: u32,
    pub misc: i32,
    pub wheel_scale: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtaIdePed {
    pub id: usize,
    pub model: String,
    pub txd: String,
    pub ped_type: String,
    pub ped_stats: String,
    pub anim_group: String,
    pub cars_can_drive: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdeMloHeader {
    pub id: usize,
    pub name: String,
    pub draw_dist: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtaIdePathHeader {
    pub id: usize,
    pub path_type: String,
    pub model_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIdeMloInstance {
    pub mlo_id: usize,
    pub model_index: usize,
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_axis: [f32; 3],
    pub angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaPathNode {
    pub node_type: i32,
    pub next: i32,
    pub cross: i32,
    pub position: [f32; 3],
    pub width: f32,
    pub num_left: Option<i32>,
    pub num_right: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaTwoDfxLight {
    pub dist: f32,
    pub range: f32,
    pub size: f32,
    pub shadow_size: f32,
    pub shadow_intensity: i32,
    pub light_type: i32,
    pub road_reflection: i32,
    pub flare_type: i32,
    pub flags: i32,
    pub corona: String,
    pub shadow: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaTwoDfx {
    pub id: usize,
    pub position: [f32; 3],
    pub color: [u8; 4],
    pub effect_type: i32,
    pub light: Option<GtaTwoDfxLight>,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIplPlan {
    pub records: Vec<GtaIplRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GtaIplRecord {
    Instance(GtaIplInstance),
    Zone(GtaZone),
    Cull(GtaCullZone),
    Pickup(GtaPickup),
    PathHeader(GtaIdePathHeader),
    PathNode(String),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaIplInstance {
    pub id: usize,
    pub name: String,
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_axis: [f32; 3],
    pub angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaZone {
    pub name: String,
    pub zone_type: i32,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaCullZone {
    pub position: [f32; 3],
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub flags: i32,
    pub wanted_level_drop: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtaPickup {
    pub id: usize,
    pub position: [f32; 3],
}

impl GtaDatPlan {
    pub fn parse(text: &str) -> Self {
        let mut commands = Vec::new();
        for line in text.lines().map(normalise_load_line) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let command = parse_dat_command(&line);
            let exit = command == GtaDatCommand::Exit;
            commands.push(command);
            if exit {
                break;
            }
        }
        Self { commands }
    }
}

impl GtaIdePlan {
    pub fn parse(text: &str) -> Self {
        let mut records = Vec::new();
        let mut invalid_vehicle_records = Vec::new();
        let mut section = GtaIdeSection::None;
        let mut path_index = -1;
        let mut path_force_car_fields = false;
        let mut current_mlo_id = 0;

        for line in text.lines().map(normalise_load_line) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if section == GtaIdeSection::None {
                section = GtaIdeSection::from_header(&line);
                continue;
            }

            if is_end_line(&line) {
                section = if section == GtaIdeSection::Mlo {
                    GtaIdeSection::Objs
                } else {
                    GtaIdeSection::None
                };
                continue;
            }

            match section {
                GtaIdeSection::Objs => {
                    if is_line3(&line, "sta") {
                        if let Some(record) = parse_mlo_header(&line) {
                            current_mlo_id = record.id;
                            records.push(GtaIdeRecord::MloHeader(record));
                            section = GtaIdeSection::Mlo;
                        }
                    } else if let Some(record) = parse_ide_object(&line) {
                        records.push(GtaIdeRecord::Object(record));
                    }
                }
                GtaIdeSection::Mlo => {
                    if let Some(record) = parse_mlo_instance(&line, current_mlo_id) {
                        records.push(GtaIdeRecord::MloInstance(record));
                    }
                }
                GtaIdeSection::Tobj => {
                    if let Some(record) = parse_ide_time_object(&line) {
                        records.push(GtaIdeRecord::TimeObject(record));
                    }
                }
                GtaIdeSection::Hier => {
                    if let Some(record) = parse_ide_clump(&line) {
                        records.push(GtaIdeRecord::Clump(record));
                    }
                }
                GtaIdeSection::Cars => {
                    if let Some(record) = parse_ide_vehicle(&line) {
                        records.push(GtaIdeRecord::Vehicle(record));
                    } else {
                        invalid_vehicle_records.push(line);
                    }
                }
                GtaIdeSection::Peds => {
                    if let Some(record) = parse_ide_ped(&line) {
                        records.push(GtaIdeRecord::Ped(record));
                    }
                }
                GtaIdeSection::Path => {
                    if path_index == -1 {
                        if let Some(record) = parse_path_header(&line) {
                            path_force_car_fields = record.path_type == "car";
                            records.push(GtaIdeRecord::PathHeader(record));
                            path_index = 0;
                        }
                    } else {
                        if let Some(record) = parse_path_node(&line, Some(path_force_car_fields)) {
                            records.push(GtaIdeRecord::PathNode(record));
                        }
                        path_index += 1;
                        if path_index == 12 {
                            path_index = -1;
                        }
                    }
                }
                GtaIdeSection::TwoDfx => {
                    if let Some(record) = parse_2dfx(&line) {
                        records.push(GtaIdeRecord::TwoDfx(record));
                    }
                }
                GtaIdeSection::None => records.push(GtaIdeRecord::Unknown(line)),
            }
        }

        Self {
            records,
            invalid_vehicle_records,
        }
    }
}

impl GtaIplPlan {
    pub fn parse(text: &str) -> Self {
        let mut records = Vec::new();
        let mut section = GtaIplSection::None;
        let mut path_index = -1;

        for line in text.lines().map(normalise_load_line) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if section == GtaIplSection::None {
                section = GtaIplSection::from_header(&line);
                continue;
            }

            if is_end_line(&line) {
                section = GtaIplSection::None;
                continue;
            }

            match section {
                GtaIplSection::Inst => {
                    if let Some(record) = parse_ipl_instance(&line) {
                        records.push(GtaIplRecord::Instance(record));
                    }
                }
                GtaIplSection::Zone => {
                    if let Some(record) = parse_zone(&line) {
                        records.push(GtaIplRecord::Zone(record));
                    }
                }
                GtaIplSection::Cull => {
                    if let Some(record) = parse_cull_zone(&line) {
                        records.push(GtaIplRecord::Cull(record));
                    }
                }
                GtaIplSection::Pick => {
                    if let Some(record) = parse_pickup(&line) {
                        records.push(GtaIplRecord::Pickup(record));
                    }
                }
                GtaIplSection::Path => {
                    if path_index == -1 {
                        if let Some(record) = parse_path_header(&line) {
                            records.push(GtaIplRecord::PathHeader(record));
                            path_index = 0;
                        }
                    } else {
                        records.push(GtaIplRecord::PathNode(line));
                        path_index += 1;
                        if path_index == 12 {
                            path_index = -1;
                        }
                    }
                }
                GtaIplSection::None => records.push(GtaIplRecord::Unknown(line)),
            }
        }

        Self { records }
    }
}

pub fn normalise_load_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for ch in line.chars() {
        if ch < ' ' || ch == ',' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out.trim_start().to_owned()
}

fn parse_dat_command(line: &str) -> GtaDatCommand {
    if line.starts_with("EXIT") {
        GtaDatCommand::Exit
    } else if line.starts_with("IMAGEPATH") {
        GtaDatCommand::ImagePath(argument_after(line, 10))
    } else if line.starts_with("TEXDICTION") {
        GtaDatCommand::TexDictionary(argument_after(line, 11))
    } else if line.starts_with("COLFILE") {
        let rest = argument_after(line, 8);
        let mut parts = rest.split_whitespace();
        let level = parts
            .next()
            .and_then(|part| part.parse::<i32>().ok())
            .unwrap_or(0);
        let path = parts.collect::<Vec<_>>().join(" ");
        GtaDatCommand::ColFile { level, path }
    } else if line.starts_with("MODELFILE") {
        GtaDatCommand::ModelFile(argument_after(line, 10))
    } else if line.starts_with("HIERFILE") {
        GtaDatCommand::HierFile(argument_after(line, 9))
    } else if line.starts_with("IDE") {
        GtaDatCommand::Ide(argument_after(line, 4))
    } else if line.starts_with("IPL") {
        GtaDatCommand::Ipl(argument_after(line, 4))
    } else if line.starts_with("MAPZONE") {
        GtaDatCommand::MapZone(argument_after(line, 8))
    } else if line.starts_with("SPLASH") {
        GtaDatCommand::Splash
    } else if line.starts_with("CDIMAGE") {
        GtaDatCommand::CdImage(argument_after(line, 8))
    } else {
        GtaDatCommand::Unknown(line.to_owned())
    }
}

fn argument_after(line: &str, offset: usize) -> String {
    line.get(offset..).unwrap_or_default().trim().to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GtaIdeSection {
    None,
    Objs,
    Mlo,
    Tobj,
    Hier,
    Cars,
    Peds,
    Path,
    TwoDfx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GtaIplSection {
    None,
    Inst,
    Zone,
    Cull,
    Pick,
    Path,
}

impl GtaIplSection {
    fn from_header(line: &str) -> Self {
        if is_line4(line, "inst") {
            Self::Inst
        } else if is_line4(line, "zone") {
            Self::Zone
        } else if is_line4(line, "cull") {
            Self::Cull
        } else if is_line4(line, "pick") {
            Self::Pick
        } else if is_line4(line, "path") {
            Self::Path
        } else {
            Self::None
        }
    }
}

impl GtaIdeSection {
    fn from_header(line: &str) -> Self {
        if is_line4(line, "objs") {
            Self::Objs
        } else if is_line4(line, "tobj") {
            Self::Tobj
        } else if is_line4(line, "hier") {
            Self::Hier
        } else if is_line4(line, "cars") {
            Self::Cars
        } else if is_line4(line, "peds") {
            Self::Peds
        } else if is_line4(line, "path") {
            Self::Path
        } else if is_line4(line, "2dfx") {
            Self::TwoDfx
        } else {
            Self::None
        }
    }
}

fn parse_ide_object(line: &str) -> Option<GtaIdeObject> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }
    let id = parts[0].parse().ok()?;
    let model = parts[1].to_owned();
    let txd = parts[2].to_owned();
    let num_objs = parts[3].parse::<u8>().ok()?;
    if !(1..=3).contains(&num_objs) || parts.len() < 5 + num_objs as usize {
        return None;
    }

    let mut distances = [0.0; 3];
    for index in 0..num_objs as usize {
        distances[index] = parts[4 + index].parse().ok()?;
    }
    let flags = parts[4 + num_objs as usize].parse().ok()?;
    let first_damaged = first_damaged(num_objs, distances);

    Some(GtaIdeObject {
        id,
        model,
        txd,
        num_objs,
        distances,
        flags,
        first_damaged,
    })
}

fn parse_ide_time_object(line: &str) -> Option<GtaIdeTimeObject> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 8 {
        return None;
    }
    let num_objs = parts.get(3)?.parse::<u8>().ok()?;
    let time_on_index = 5 + num_objs as usize;
    let time_on = parts.get(time_on_index)?.parse().ok()?;
    let time_off = parts.get(time_on_index + 1)?.parse().ok()?;
    Some(GtaIdeTimeObject {
        object: parse_ide_object(line)?,
        time_on,
        time_off,
    })
}

fn parse_ide_clump(line: &str) -> Option<GtaIdeClump> {
    let mut parts = line.split_whitespace();
    Some(GtaIdeClump {
        id: parts.next()?.parse().ok()?,
        model: parts.next()?.to_owned(),
        txd: parts.next()?.to_owned(),
    })
}

fn parse_ide_vehicle(line: &str) -> Option<GtaIdeVehicle> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }
    // LoadVehicleObject only consumes the tail fields for car/bike/plane.
    // Boats, trains and helicopters use the common ten authored fields.
    let (misc, wheel_scale) = match parts[3] {
        "car" | "bike" => (parts.get(10)?.parse().ok()?, parts.get(11)?.parse().ok()?),
        "plane" => (parts.get(10)?.parse().ok()?, 1.0),
        "boat" | "train" | "heli" => (-1, 1.0), // Unused DTO slots; model defaults stay intact.
        _ => return None,
    };
    Some(GtaIdeVehicle {
        id: parts[0].parse().ok()?,
        model: parts[1].to_owned(),
        txd: parts[2].to_owned(),
        vehicle_type: parts[3].to_owned(),
        handling_id: parts[4].to_owned(),
        game_name: parts[5].replace('_', " "),
        vehicle_class: parts[6].to_owned(),
        frequency: parts[7].parse().ok()?,
        level: parts[8].parse().ok()?,
        component_rules: u32::from_str_radix(parts[9], 16).ok()?,
        misc,
        wheel_scale,
    })
}

fn parse_ide_ped(line: &str) -> Option<GtaIdePed> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }
    Some(GtaIdePed {
        id: parts[0].parse().ok()?,
        model: parts[1].to_owned(),
        txd: parts[2].to_owned(),
        ped_type: parts[3].to_owned(),
        ped_stats: parts[4].to_owned(),
        anim_group: parts[5].to_owned(),
        cars_can_drive: u32::from_str_radix(parts[6], 16).ok()?,
    })
}

fn parse_mlo_header(line: &str) -> Option<GtaIdeMloHeader> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    Some(GtaIdeMloHeader {
        id: parts.get(2)?.parse().ok()?,
        name: parts.get(1)?.to_string(),
        draw_dist: parts.get(3)?.parse().ok()?,
    })
}

fn parse_path_header(line: &str) -> Option<GtaIdePathHeader> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    Some(GtaIdePathHeader {
        path_type: parts.first()?.to_string(),
        id: parts.get(1)?.parse().ok()?,
        model_name: parts.get(2)?.to_string(),
    })
}

fn parse_mlo_instance(line: &str, mlo_id: usize) -> Option<GtaIdeMloInstance> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 12 {
        return None;
    }
    Some(GtaIdeMloInstance {
        mlo_id,
        model_index: parts[0].parse().ok()?,
        name: parts[1].to_owned(),
        position: parse_vec3(&parts[2..5])?,
        scale: parse_vec3(&parts[5..8])?,
        rotation_axis: parse_vec3(&parts[8..11])?,
        angle: parts[11].parse().ok()?,
    })
}

fn parse_path_node(line: &str, force_car_fields: Option<bool>) -> Option<GtaPathNode> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }
    let has_car_fields = force_car_fields.unwrap_or(parts.len() >= 9);
    Some(GtaPathNode {
        node_type: parts[0].parse().ok()?,
        next: parts[1].parse().ok()?,
        cross: parts[2].parse().ok()?,
        position: parse_vec3(&parts[3..6])?,
        width: parts[6].parse().ok()?,
        num_left: has_car_fields.then(|| parts.get(7)?.parse().ok()).flatten(),
        num_right: has_car_fields.then(|| parts.get(8)?.parse().ok()).flatten(),
    })
}

fn parse_2dfx(line: &str) -> Option<GtaTwoDfx> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }
    let id: usize = parts[0].parse().ok()?;
    let position = parse_vec3(&parts[1..4])?;
    let color = [
        parts[4].parse().ok()?,
        parts[5].parse().ok()?,
        parts[6].parse().ok()?,
        parts[7].parse().ok()?,
    ];
    let effect_type: i32 = parts[8].parse().ok()?;
    let light = if effect_type == 0 && parts.len() >= 20 {
        let corona = parts[9].trim_matches('"').to_owned();
        let shadow = parts[10].trim_matches('"').to_owned();
        let dist = parts[11].parse().ok()?;
        let range = parts[12].parse().ok()?;
        let size = parts[13].parse().ok()?;
        let shadow_size = parts[14].parse().ok()?;
        let shadow_intensity = parts[15].parse().ok()?;
        let light_type = parts[16].parse().ok()?;
        let road_reflection = parts[17].parse().ok()?;
        let flare_type = parts[18].parse().ok()?;
        let flags = parts[19].parse().ok()?;
        Some(GtaTwoDfxLight {
            corona,
            shadow,
            dist,
            range,
            size,
            shadow_size,
            shadow_intensity,
            light_type,
            road_reflection,
            flare_type,
            flags,
        })
    } else {
        None
    };

    Some(GtaTwoDfx {
        id,
        position,
        color,
        effect_type,
        light,
        raw: line.to_owned(),
    })
}

fn parse_ipl_instance(line: &str) -> Option<GtaIplInstance> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 12 {
        return None;
    }
    Some(GtaIplInstance {
        id: parts[0].parse().ok()?,
        name: parts[1].to_owned(),
        position: parse_vec3(&parts[2..5])?,
        scale: parse_vec3(&parts[5..8])?,
        rotation_axis: parse_vec3(&parts[8..11])?,
        angle: parts[11].parse().ok()?,
    })
}

fn parse_zone(line: &str) -> Option<GtaZone> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }
    Some(GtaZone {
        name: parts[0].to_owned(),
        zone_type: parts[1].parse().ok()?,
        min: parse_vec3(&parts[2..5])?,
        max: parse_vec3(&parts[5..8])?,
        level: parts[8].parse().ok()?,
    })
}

fn parse_cull_zone(line: &str) -> Option<GtaCullZone> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }
    Some(GtaCullZone {
        position: parse_vec3(&parts[0..3])?,
        min: parse_vec3(&parts[3..6])?,
        max: parse_vec3(&parts[6..9])?,
        flags: parts[9].parse().ok()?,
        wanted_level_drop: parts
            .get(10)
            .and_then(|part| part.parse().ok())
            .unwrap_or(0),
    })
}

fn parse_pickup(line: &str) -> Option<GtaPickup> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    Some(GtaPickup {
        id: parts[0].parse().ok()?,
        position: parse_vec3(&parts[1..4])?,
    })
}

fn parse_vec3(parts: &[&str]) -> Option<[f32; 3]> {
    Some([
        parts.first()?.parse().ok()?,
        parts.get(1)?.parse().ok()?,
        parts.get(2)?.parse().ok()?,
    ])
}

fn first_damaged(num_objs: u8, dist: [f32; 3]) -> u8 {
    match num_objs {
        2 => {
            if dist[0] < dist[1] {
                0
            } else {
                1
            }
        }
        3 => {
            if dist[0] < dist[1] {
                if dist[1] < dist[2] { 0 } else { 2 }
            } else {
                1
            }
        }
        _ => 0,
    }
}

fn is_line3(line: &str, tag: &str) -> bool {
    line.len() >= 3 && line[..3].eq_ignore_ascii_case(tag)
}

fn is_line4(line: &str, tag: &str) -> bool {
    line.len() >= 4 && line[..4].eq_ignore_ascii_case(tag)
}

fn is_end_line(line: &str) -> bool {
    is_line3(line, "end")
}

#[cfg(test)]
mod vehicle_ide_tests {
    use super::*;

    #[test]
    fn vehicle_ide_consumes_only_kind_specific_tail_fields() {
        for kind in ["boat", "train", "heli"] {
            let row = format!("142 model txd {kind} HANDLING NAME ignore 1 7 0");
            let record = parse_ide_vehicle(&row).expect("native ten-field vehicle record");
            assert_eq!(record.vehicle_type, kind);
            assert_eq!(record.handling_id, "HANDLING");
        }
        let plane =
            parse_ide_vehicle("141 model txd plane HANDLING NAME ignore 1 7 0 150").unwrap();
        assert_eq!(plane.misc, 150);
        assert_eq!(plane.wheel_scale, 1.0);
        for kind in ["car", "bike"] {
            let row = format!("100 model txd {kind} HANDLING NAME worker 10 7 ffff 164 0.85");
            let record = parse_ide_vehicle(&row).unwrap();
            assert_eq!(record.misc, 164);
            assert_eq!(record.wheel_scale, 0.85);
            assert_eq!(record.component_rules, 0xffff);
        }
    }

    #[test]
    fn vehicle_ide_normalizes_commas_and_retains_invalid_required_rows() {
        let plan = GtaIdePlan::parse(
            "cars\n142, model, txd, boat, HANDLING, NAME, ignore, 1, 7, 0\n100 model txd car HANDLING NAME worker 10 7 ffff\nend\n",
        );
        assert_eq!(plan.records.len(), 1);
        assert!(matches!(plan.records[0], GtaIdeRecord::Vehicle(_)));
        assert_eq!(plan.invalid_vehicle_records.len(), 1);
        assert!(plan.invalid_vehicle_records[0].starts_with("100 model"));
    }
}
