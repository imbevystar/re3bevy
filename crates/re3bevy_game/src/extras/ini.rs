//! re3 reference: references/re-GTA/src/extras/ini.h
//!
//! Minimal INI structure/parser equivalent to the vendored mINI usage.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct INIStructure {
    pub sections: BTreeMap<String, BTreeMap<String, String>>,
}

impl INIStructure {
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(&normalise(section))
            .and_then(|items| items.get(&normalise(key)))
            .map(String::as_str)
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        self.sections
            .entry(normalise(section))
            .or_default()
            .insert(normalise(key), value.trim().to_owned());
    }

    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        let mut current_section = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some(section) = line.strip_prefix('[').and_then(|s| s.split(']').next()) {
                current_section = normalise(section);
                out.sections.entry(current_section.clone()).or_default();
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                out.set(&current_section, key.replace("\\=", "=").as_str(), value);
            }
        }
        out
    }

    pub fn generate(&self, pretty: bool) -> String {
        let sep = if pretty { " = " } else { "=" };
        let mut lines = Vec::new();
        for (section, values) in &self.sections {
            lines.push(format!("[{section}]"));
            for (key, value) in values {
                lines.push(format!("{}{sep}{}", key.replace('=', "\\="), value.trim()));
            }
        }
        lines.join("\r\n")
    }
}

fn normalise(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
