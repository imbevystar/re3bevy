//! re3 reference: references/re-GTA/src/vehicles/HandlingMgr.cpp
//! re3 reference: references/re-GTA/src/core/SurfaceTable.cpp
//! re3 reference: references/re-GTA/src/weapons/WeaponInfo.cpp
//! re3 reference: references/re-GTA/src/modelinfo/VehicleModelInfo.cpp
//! re3 reference: references/re-GTA/src/peds/Population.cpp
//!
//! Text table parsers for DATA files that are read with `CFileMgr::LoadFile`.

#[derive(Debug, Clone, PartialEq)]
pub struct HandlingCfg {
    pub records: Vec<HandlingRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HandlingRecord {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceDat {
    pub rows: Vec<SurfaceRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceRow {
    pub name: String,
    pub adhesive_limits: Vec<Option<f32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeaponDat {
    pub records: Vec<WeaponRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeaponRecord {
    pub weapon_name: String,
    pub fire_type: String,
    pub range: f32,
    pub firing_rate: i32,
    pub reload: i32,
    pub ammo_amount: i32,
    pub damage: i32,
    pub speed: f32,
    pub radius: f32,
    pub life_span: f32,
    pub spread: f32,
    pub fire_offset: [f32; 3],
    pub anim_to_play: String,
    pub anim2_to_play: String,
    pub anim_loop_start: f32,
    pub anim_loop_end: f32,
    pub delay_between_anim_and_fire: f32,
    pub delay_between_anim2_and_fire: f32,
    pub model_id: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CarcolsDat {
    pub colours: Vec<CarcolsColour>,
    pub cars: Vec<CarcolsVehicle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarcolsColour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarcolsVehicle {
    pub name: String,
    pub pairs: Vec<(u8, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedgrpDat {
    pub groups: Vec<PedgrpGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PedgrpGroup {
    pub models: [String; 8],
}

impl HandlingCfg {
    pub fn parse(text: &str) -> Self {
        let mut records = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line == ";the end" {
                break;
            }
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            records.push(HandlingRecord {
                name: name.to_owned(),
                fields: parts.map(str::to_owned).collect(),
            });
        }
        Self { records }
    }
}

impl SurfaceDat {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let mut parts = line.split([',', ' ', '\t']).filter(|part| !part.is_empty());
            let Some(name) = parts.next() else {
                continue;
            };
            let row = rows.len();
            if row == SURFACE_ADHESIVE_GROUP_COUNT { break; }
            let mut adhesive_limits = Vec::with_capacity(row + 1);
            for column in 0..=row {
                let token = parts.next().ok_or_else(||
                    format!("SURFACE.DAT missing cell at row {row}, column {column}"))?;
                let value = if token == "-" { None } else {
                    let value: f32 = token.parse().map_err(|_|
                        format!("SURFACE.DAT invalid number '{token}' at row {row}, column {column}"))?;
                    if !value.is_finite() {
                        return Err(format!("SURFACE.DAT non-finite value at row {row}, column {column}"));
                    }
                    Some(value)
                };
                adhesive_limits.push(value);
            }
            rows.push(SurfaceRow { name: name.to_owned(), adhesive_limits });
        }
        let table = Self { rows };
        table.validate()?;
        Ok(table)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.rows.len() != SURFACE_ADHESIVE_GROUP_COUNT {
            return Err(format!("SURFACE.DAT requires {SURFACE_ADHESIVE_GROUP_COUNT} adhesive rows, got {}", self.rows.len()));
        }
        for (row, data) in self.rows.iter().enumerate() {
            for column in 0..=row {
                let value = data.adhesive_limits.get(column).ok_or_else(||
                    format!("SURFACE.DAT missing cell at row {row}, column {column}"))?;
                if value.is_some_and(|value| !value.is_finite()) {
                    return Err(format!("SURFACE.DAT non-finite value at row {row}, column {column}"));
                }
            }
        }
        Ok(())
    }
}

/// re3 CSurfaceTable::Initialise reads this many triangular adhesive rows.
pub const SURFACE_ADHESIVE_GROUP_COUNT: usize = 5;

#[cfg(test)]
mod surface_tests {
    use super::*;

    // Synthetic values identify cells; installed GTA data remains external.
    const TABLE: &str = "Rubber 10\nHard 20 21\nRoad 30 31 32\nLoose 40 41 42 43\nWet 50 51 52 53 54";

    #[test]
    fn native_triangular_rows_preserve_explicit_dash_and_required_values() {
        let table = SurfaceDat::parse(&TABLE.replace("Hard 20", "Hard -")).unwrap();
        assert_eq!(table.rows[1].adhesive_limits[0], None);
        assert_eq!(table.rows[2].adhesive_limits[0], Some(30.0));
        assert!(SurfaceDat::parse(&format!("{TABLE}\nignored native tail")).is_ok());
    }

    #[test]
    fn malformed_required_input_cannot_become_dash_or_default_grip() {
        for input in ["".to_owned(), TABLE.replace("Road 30", "Road broken"),
            TABLE.replace("Road 30", "Road NaN"), TABLE.replace("Wet 50 51 52 53 54", "Wet 50")] {
            assert!(SurfaceDat::parse(&input).is_err(), "invalid required surface input: {input}");
        }
        let mut table = SurfaceDat::parse(TABLE).unwrap();
        table.rows[2].adhesive_limits.clear();
        assert!(table.validate().is_err());
    }
}

impl WeaponDat {
    pub fn parse(text: &str) -> Self {
        let mut records = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts
                .first()
                .is_some_and(|name| name.starts_with("ENDWEAPONDATA"))
            {
                break;
            }
            if parts.len() < 22 {
                continue;
            }
            let Some(record) = parse_weapon_record(&parts) else {
                continue;
            };
            records.push(record);
        }
        Self { records }
    }
}

impl CarcolsDat {
    pub fn parse(text: &str) -> Self {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Section {
            None,
            Colours,
            Cars,
        }

        let mut colours = Vec::new();
        let mut cars = Vec::new();
        let mut section = Section::None;

        for raw_line in text.lines() {
            let mut line = raw_line.replace([',', '\r'], " ");
            let start = line
                .bytes()
                .position(|byte| byte > b' ' || byte == b'\0' || byte == b'\n')
                .unwrap_or(line.len());
            if start >= line.len() {
                continue;
            }
            line.truncate(
                line[start..]
                    .bytes()
                    .position(|byte| byte == b'\0' || byte == b'\n')
                    .map(|offset| start + offset)
                    .unwrap_or(line.len()),
            );
            let active = &line[start..];
            if active.starts_with('#') || active.is_empty() {
                continue;
            }

            if section == Section::None {
                if active.starts_with("col") {
                    section = Section::Colours;
                } else if active.starts_with("car") {
                    section = Section::Cars;
                }
                continue;
            }

            if active.starts_with("end") {
                section = Section::None;
                continue;
            }

            match section {
                Section::None => {}
                Section::Colours => {
                    let values: Vec<i32> = active
                        .split_whitespace()
                        .filter_map(|part| part.parse::<i32>().ok())
                        .collect();
                    if values.len() >= 3 {
                        colours.push(CarcolsColour {
                            red: values[0] as u8,
                            green: values[1] as u8,
                            blue: values[2] as u8,
                            alpha: 0xFF,
                        });
                    }
                }
                Section::Cars => {
                    let mut parts = active.split_whitespace();
                    let Some(name) = parts.next() else {
                        continue;
                    };
                    let values: Vec<u8> = parts
                        .take(16)
                        .filter_map(|part| part.parse::<i32>().ok())
                        .map(|value| value as u8)
                        .collect();
                    let pairs = values
                        .chunks_exact(2)
                        .map(|pair| (pair[0], pair[1]))
                        .collect();
                    cars.push(CarcolsVehicle {
                        name: name.to_owned(),
                        pairs,
                    });
                }
            }
        }

        Self { colours, cars }
    }
}

impl PedgrpDat {
    pub fn parse(text: &str) -> Self {
        let mut groups = Vec::new();

        for raw_line in text.lines() {
            let line: String = raw_line
                .chars()
                .map(|ch| if ch == ',' || ch == '\r' { ' ' } else { ch })
                .collect();
            let mut models = Vec::with_capacity(8);

            for part in line.split_whitespace() {
                if part.starts_with('#') {
                    break;
                }
                if models.len() >= 8 {
                    break;
                }
                models.push(part.to_owned());
            }

            if models.len() == 8 {
                groups.push(PedgrpGroup {
                    models: [
                        models[0].clone(),
                        models[1].clone(),
                        models[2].clone(),
                        models[3].clone(),
                        models[4].clone(),
                        models[5].clone(),
                        models[6].clone(),
                        models[7].clone(),
                    ],
                });
            }
        }

        Self { groups }
    }
}

fn parse_weapon_record(parts: &[&str]) -> Option<WeaponRecord> {
    Some(WeaponRecord {
        weapon_name: parts.first()?.to_string(),
        fire_type: parts.get(1)?.to_string(),
        range: parts.get(2)?.parse().ok()?,
        firing_rate: parts.get(3)?.parse().ok()?,
        reload: parts.get(4)?.parse().ok()?,
        ammo_amount: parts.get(5)?.parse().ok()?,
        damage: parts.get(6)?.parse().ok()?,
        speed: parts.get(7)?.parse().ok()?,
        radius: parts.get(8)?.parse().ok()?,
        life_span: parts.get(9)?.parse().ok()?,
        spread: parts.get(10)?.parse().ok()?,
        fire_offset: [
            parts.get(11)?.parse().ok()?,
            parts.get(12)?.parse().ok()?,
            parts.get(13)?.parse().ok()?,
        ],
        anim_to_play: parts.get(14)?.to_string(),
        anim2_to_play: parts.get(15)?.to_string(),
        anim_loop_start: parts.get(16)?.parse().ok()?,
        anim_loop_end: parts.get(17)?.parse().ok()?,
        delay_between_anim_and_fire: parts.get(18)?.parse().ok()?,
        delay_between_anim2_and_fire: parts.get(19)?.parse().ok()?,
        model_id: parts.get(20)?.parse().ok()?,
        flags: parts.get(21)?.parse().ok()?,
    })
}
