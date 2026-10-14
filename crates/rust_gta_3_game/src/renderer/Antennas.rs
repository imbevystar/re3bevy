//! re3 reference: references/re-GTA/src/renderer/Antennas.cpp
//! re3 reference: references/re-GTA/src/renderer/Antennas.h

use bevy::prelude::*;
use rust_gta_3_core::core::Coordinates::CVector;

use super::Lines::{LineIntent, LineVertex};
use crate::modelinfo::ModelIndices::MI_RCBANDIT;

pub const NUMANTENNAS: usize = 8;
pub const NUM_ANTENNA_SEGMENT_POINTS: usize = 6;

pub(crate) fn update_antennas(antennas: Option<ResMut<CAntennas>>) {
    if let Some(mut antennas) = antennas {
        antennas.update();
    }
}

/// re3 `CAutomobile::PreRender` registers a one-metre antenna only for the RC
/// Bandit. The SCM vehicle handle is the stable ECS identity corresponding to
/// the pointer-derived id used by the original `CAntennas::RegisterOne` call.
pub(crate) fn register_automobile_antennas(
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    antennas: Option<ResMut<CAntennas>>,
) {
    let Some(mut antennas) = antennas else {
        return;
    };

    for vehicle in vehicles.iter() {
        let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { handle, car } = vehicle else {
            continue;
        };
        let entity = &car.vehicle.core.physical.entity;
        if entity.model_index != MI_RCBANDIT as i16 {
            continue;
        }

        let position = entity.transform_point(CVector::new(0.218, -0.444, 0.391));
        antennas.register_one(handle as u32, entity.get_up(), position, 1.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CAntenna {
    pub active: bool,
    pub updated_last_frame: bool,
    pub id: u32,
    pub segment_length: f32,
    pub positions: [CVector; NUM_ANTENNA_SEGMENT_POINTS],
    pub speeds: [CVector; NUM_ANTENNA_SEGMENT_POINTS],
}

impl Default for CAntenna {
    fn default() -> Self {
        Self {
            active: false,
            updated_last_frame: false,
            id: 0,
            segment_length: 0.0,
            positions: [CVector::ZERO; NUM_ANTENNA_SEGMENT_POINTS],
            speeds: [CVector::ZERO; NUM_ANTENNA_SEGMENT_POINTS],
        }
    }
}

impl CAntenna {
    /// re3 `CAntenna::Update`: the first two points are rigidly attached and
    /// the remaining four retain their spring speed from the previous frame.
    pub fn update(&mut self, direction: CVector, base_position: CVector) {
        self.positions[0] = base_position;
        self.positions[1] = add(base_position, scale(direction, self.segment_length));

        for i in 2..NUM_ANTENNA_SEGMENT_POINTS {
            let base_direction = sub(self.positions[i - 1], self.positions[i - 2]);
            let mut new_direction = add(
                add(
                    sub(self.positions[i], self.positions[i - 1]),
                    scale(direction, 0.1),
                ),
                self.speeds[i],
            );
            new_direction = scale(new_direction, 1.0 / magnitude(new_direction));
            new_direction = scale(new_direction, self.segment_length);
            let new_position = add(
                self.positions[i - 1],
                scale(add(base_direction, new_direction), 0.5),
            );
            self.speeds[i] = scale(sub(new_position, self.positions[i]), 0.9);
            self.positions[i] = new_position;
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CAntennas {
    pub antennas: [CAntenna; NUMANTENNAS],
    /// Frame-local, untextured `rwPRIMTYPEPOLYLINE` equivalents.
    pub render_lines: Vec<LineIntent>,
}

impl CAntennas {
    pub fn init() -> Self {
        Self {
            antennas: [CAntenna::default(); NUMANTENNAS],
            render_lines: Vec::with_capacity(NUMANTENNAS * (NUM_ANTENNA_SEGMENT_POINTS - 1)),
        }
    }

    /// Frees antennas that were not registered during the preceding vehicle render.
    pub fn update(&mut self) {
        for antenna in &mut self.antennas {
            if antenna.active && !antenna.updated_last_frame {
                antenna.active = false;
            }
            antenna.updated_last_frame = false;
        }
    }

    /// Adds an antenna to the first free slot, or advances the spring of an
    /// existing antenna with the same pointer-derived id.
    pub fn register_one(
        &mut self,
        id: u32,
        direction: CVector,
        position: CVector,
        length: f32,
    ) -> bool {
        if let Some(antenna) = self
            .antennas
            .iter_mut()
            .find(|antenna| antenna.active && antenna.id == id)
        {
            antenna.update(direction, position);
            antenna.updated_last_frame = true;
            return true;
        }

        let Some(antenna) = self.antennas.iter_mut().find(|antenna| !antenna.active) else {
            return false;
        };
        antenna.active = true;
        antenna.updated_last_frame = true;
        antenna.id = id;
        antenna.segment_length = length / NUM_ANTENNA_SEGMENT_POINTS as f32;
        for i in 0..NUM_ANTENNA_SEGMENT_POINTS {
            antenna.positions[i] = add(
                position,
                scale(direction, i as f32 * antenna.segment_length),
            );
            antenna.speeds[i] = CVector::ZERO;
        }
        true
    }

    pub fn render(&mut self) {
        self.render_lines.clear();
        let vertex_colour = [200, 200, 200, 100];
        for antenna in self.antennas.iter().filter(|antenna| antenna.active) {
            for segment in 0..NUM_ANTENNA_SEGMENT_POINTS - 1 {
                self.render_lines.push(LineIntent {
                    start: LineVertex {
                        position: antenna.positions[segment],
                        colour: vertex_colour,
                    },
                    end: LineVertex {
                        position: antenna.positions[segment + 1],
                        colour: vertex_colour,
                    },
                });
            }
        }
    }

    pub fn drain_render_lines(&mut self) -> Vec<LineIntent> {
        std::mem::take(&mut self.render_lines)
    }
}

impl Default for CAntennas {
    fn default() -> Self {
        Self::init()
    }
}

fn add(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: CVector, b: CVector) -> CVector {
    CVector::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(value: CVector, factor: f32) -> CVector {
    CVector::new(value.x * factor, value.y * factor, value.z * factor)
}

fn magnitude(value: CVector) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}
