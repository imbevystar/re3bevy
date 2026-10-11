//! Headless resident-vehicle render probe using the real Steam GTA III DFF.
//!
//! It verifies that the full allocation pass and the steady frame-only pass
//! address the same wheel/door/rotor submeshes. No Bevy window or renderer is
//! created.

use re3bevy_assets::{
    install::OriginalGtaInstall::OriginalGtaInstall, streaming::CdStream::CdStream,
};
use re3bevy_game::{
    core::{FileLoader::CFileLoader, Streaming::CStreaming},
    modelinfo::VehicleModelInfo::vehicle_dummy_model_position,
    renderer::BevyRender::{
        VehicleColourRemap, VehicleDamageRenderState, VehicleWheelRenderState,
        build_model_material_drawables, build_vehicle_component_transforms,
    },
};
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let config = env::args()
        .nth(1)
        .unwrap_or_else(|| "config/original_gta3_path.txt".to_owned());
    let install = match OriginalGtaInstall::from_config_path(&config) {
        Ok(install) => install,
        Err(error) => return fail(2, &format!("install: {error}")),
    };
    let mut file_mgr = install.file_mgr();
    let mut cd_stream = CdStream::init(5);
    let mut file_loader = CFileLoader::default();
    if let Err(error) = file_loader.load_default_and_dat_plans(
        &install,
        &mut file_mgr,
        &mut cd_stream,
        "DATA\\GTA3.DAT",
    ) {
        return fail(3, &format!("DAT: {error}"));
    }
    let mut streaming = CStreaming::default();
    if let Err(error) = streaming.load_cd_directory(&cd_stream, &mut file_loader) {
        return fail(4, &format!("IMG directory: {error}"));
    }
    let Some((taxi_id, _)) = file_loader.model_info.get_model_info_by_name("taxi") else {
        return fail(5, "taxi model missing from real IDE data");
    };
    let Some((landstal_id, _)) = file_loader.model_info.get_model_info_by_name("landstal") else {
        return fail(5, "landstal model missing from real IDE data");
    };
    streaming.request_model_with_adapter(
        taxi_id,
        re3bevy_game::core::Streaming::STREAMFLAGS_DONT_REMOVE,
        &file_loader,
    );
    streaming.request_model_with_adapter(
        landstal_id,
        re3bevy_game::core::Streaming::STREAMFLAGS_DONT_REMOVE,
        &file_loader,
    );
    if let Err(error) = file_loader.load_all_requested_models(&mut streaming, &mut cd_stream, false)
    {
        return fail(6, &format!("taxi stream: {error}"));
    }

    let baseline = build_model_material_drawables(
        taxi_id,
        0.0,
        false,
        &file_loader,
        VehicleColourRemap::default(),
    );
    let resident_dynamic = baseline
        .iter()
        .enumerate()
        .filter_map(|(index, part)| part.dynamic_vehicle_component.then_some(index))
        .collect::<Vec<_>>();
    if baseline.is_empty() || resident_dynamic.is_empty() {
        return fail(7, "real taxi DFF produced no resident/dynamic submeshes");
    }
    if env::var_os("RE3BEVY_DUMP_VEHICLE_MATERIALS").is_some() {
        if let Some(summary) = file_loader
            .model_info
            .get_model_info(taxi_id)
            .and_then(|model| model.base().clump_summary.as_ref())
        {
            for atomic in &summary.atomics {
                let name = atomic.frame_name.as_deref().unwrap_or("<unnamed>");
                let Some(geometry) = usize::try_from(atomic.geometry_index)
                    .ok()
                    .and_then(|index| summary.geometries.get(index))
                else {
                    continue;
                };
                let mut counts = vec![0usize; geometry.materials.len()];
                if let Some(decoded) = geometry.decoded.as_ref() {
                    for &material in &decoded.triangle_materials {
                        if let Some(count) = counts.get_mut(material as usize) {
                            *count += 1;
                        }
                    }
                }
                let materials = geometry
                    .materials
                    .iter()
                    .enumerate()
                    .map(|(index, material)| {
                        format!(
                            "{index}:{}:{:?}:{:?}",
                            counts.get(index).copied().unwrap_or(0),
                            material.texture_name,
                            material.color
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                println!(
                    "ATOMIC {name} geometry={} {materials}",
                    atomic.geometry_index
                );
            }
        }
    }
    let landstal_seat = file_loader
        .model_info
        .get_model_info(landstal_id)
        .and_then(|model| model.base().clump_summary.as_ref())
        .and_then(|summary| vehicle_dummy_model_position(summary, "ped_frontseat"))
        .unwrap_or_default();
    if landstal_seat.x.abs() < 0.05 {
        return fail(15, "real landstal driver seat remained at vehicle centre");
    }
    let wheel_id = match file_loader.model_info.get_model_info(taxi_id) {
        Some(re3bevy_game::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle)) => {
            usize::try_from(vehicle.wheel_id).expect("taxi wheel model")
        }
        _ => return fail(8, "taxi IDE entry is not a vehicle model"),
    };
    if baseline
        .iter()
        .filter(|part| part.texture_model_index == wheel_id)
        .any(|part| part.vehicle_colour_slot.is_some())
    {
        return fail(9, "re3 wheel model leaked into vehicle CARCOLS materials");
    }

    let mut remap = VehicleColourRemap::default();
    remap.wheel_state = Some(VehicleWheelRenderState {
        rotation: [0.25, -0.5, 0.75, -1.0],
        position: [-0.18, -0.17, -0.19, -0.16],
        suspension_initialized: true,
        steer_angle: 0.2,
        status: [0; 4],
    });
    let Some(first_frames) = build_vehicle_component_transforms(taxi_id, 0.0, &file_loader, remap)
    else {
        return fail(10, "frame-only adapter rejected the resident taxi clump");
    };
    let frame_indices = first_frames
        .iter()
        .map(|(index, _)| *index)
        .collect::<Vec<_>>();
    if frame_indices != resident_dynamic {
        eprintln!("resident_indices={resident_dynamic:?}\nframe_indices={frame_indices:?}");
        return fail(11, "resident/frame-only submesh index mismatch");
    }

    // Select the bonnet `_dam` atomic from the real DFF.  re3 collapses its
    // marker frame onto `bonnet_dummy`; every resulting vertex must remain in
    // plausible vehicle-local bounds instead of exploding across the scene.
    let mut damaged_remap = remap;
    damaged_remap.damage_state = Some(VehicleDamageRenderState {
        door_status: [1, 0, 0, 0, 0, 0],
        panel_status: [0; 7],
    });
    let damaged = build_model_material_drawables(taxi_id, 0.0, false, &file_loader, damaged_remap);
    let mut max_abs = 0.0_f32;
    for part in &damaged {
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            part.mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_POSITION)
        {
            for position in positions {
                max_abs = max_abs.max(position[0].abs());
                max_abs = max_abs.max(position[1].abs());
                max_abs = max_abs.max(position[2].abs());
            }
        }
    }
    if damaged.is_empty() || max_abs > 10.0 {
        return fail(12, "collapsed damaged atomic escaped vehicle-local bounds");
    }

    let mut transform_updates = 0usize;
    for tick in 0..1_800 {
        let phase = tick as f32 * 0.03125;
        let wheel = remap.wheel_state.as_mut().expect("wheel state set above");
        wheel.rotation = [phase, -phase, phase * 0.75, -phase * 0.75];
        wheel.steer_angle = phase.sin() * 0.6;
        let Some(frames) = build_vehicle_component_transforms(taxi_id, 0.0, &file_loader, remap)
        else {
            return fail(13, "steady frame adapter failed");
        };
        if frames.len() != resident_dynamic.len() {
            return fail(14, "steady component count changed without topology damage");
        }
        transform_updates += frames.len();
    }

    println!(
        "PASS model=taxi id={taxi_id} resident_submeshes={} dynamic_submeshes={} damaged_max_abs={max_abs:.4} wheel_carcols=0 landstal_driver_seat_x={:.4} ticks=1800 transform_updates={transform_updates} steady_mesh_rebuilds=0 steady_material_rebuilds=0",
        baseline.len(),
        resident_dynamic.len(),
        -landstal_seat.x,
    );
    ExitCode::SUCCESS
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("FAIL: {message}");
    ExitCode::from(code)
}
