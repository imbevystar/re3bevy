//! re3 reference: references/re-GTA/src/extras/debugmenu.cpp
//! re3 reference: references/re-GTA/src/extras/debugmenu.h
//!
//! Debug-menu tweak registry.

use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Resource, Default)]
pub struct CTweakVars {
    pub vars: Vec<CTweakVar>,
    pub menus_initialised: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CTweakVar {
    Func {
        path: String,
        name: String,
    },
    Bool {
        path: String,
        name: String,
        value: bool,
    },
    Switch {
        path: String,
        name: String,
        value: i32,
        min: i32,
        max: i32,
        strings: Vec<String>,
    },
    Int {
        path: String,
        name: String,
        value: i64,
        lower: i64,
        upper: i64,
        step: i64,
    },
    UInt {
        path: String,
        name: String,
        value: u64,
        lower: u64,
        upper: u64,
        step: u64,
    },
    Float {
        path: String,
        name: String,
        value: f32,
        lower: f32,
        upper: f32,
        step: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugMenuEntry {
    pub path: String,
    pub name: String,
    pub wrap: bool,
    pub strings: Vec<String>,
    pub address: Option<usize>,
}

impl CTweakVars {
    pub fn add(&mut self, var: CTweakVar) {
        self.vars.push(var);
    }

    pub fn add_dbg(&self, path: &str) -> Vec<DebugMenuEntry> {
        self.vars
            .iter()
            .filter_map(|var| debug_entry_for_var(var, path))
            .collect()
    }

    pub fn debug_menu_init(&mut self) {
        self.menus_initialised = true;
    }

    pub fn debug_menu_shutdown(&mut self) {
        self.menus_initialised = false;
    }
}

pub fn debug_menu_entry_set_wrap(entry: &mut DebugMenuEntry, wrap: bool) {
    entry.wrap = wrap;
}

pub fn debug_menu_entry_set_strings(entry: &mut DebugMenuEntry, strings: &[&str]) {
    entry.strings = strings.iter().map(|value| (*value).to_owned()).collect();
}

pub fn debug_menu_entry_set_address(entry: &mut DebugMenuEntry, address: usize) {
    entry.address = Some(address);
}

fn debug_entry_for_var(var: &CTweakVar, path: &str) -> Option<DebugMenuEntry> {
    let (var_path, name) = match var {
        CTweakVar::Func { path, name }
        | CTweakVar::Bool { path, name, .. }
        | CTweakVar::Switch { path, name, .. }
        | CTweakVar::Int { path, name, .. }
        | CTweakVar::UInt { path, name, .. }
        | CTweakVar::Float { path, name, .. } => (path, name),
    };
    if var_path != path {
        return None;
    }
    Some(DebugMenuEntry {
        path: var_path.clone(),
        name: name.clone(),
        wrap: false,
        strings: Vec::new(),
        address: None,
    })
}
