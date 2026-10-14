//! Developer renderer inspection and clipboard reports.

use super::{
    GtaRenderPassPlanState, GtaWorldMeshRegistry, GtaWorldModelAssetCache, entity_world_point,
    world_distance, world_entity_snapshot_for_camera, world_model_snapshot,
    world_model_txd_generation,
};
use crate::{
    core::{
        Camera::{CCamera, CameraScriptMode},
        FileLoader::CFileLoader,
        Streaming::CStreaming,
    },
    peds::PlayerRead::CWorldPlayersRead,
    renderer::Renderer::{
        CRenderer, RendererPassPlan, entity_cull_zone_visible, zone_cull_disabled,
    },
};
use bevy::prelude::*;
use rust_gta_3_core::core::Coordinates::CVector;

pub(super) struct RenderDiagnosticsPlugin;

impl Plugin for RenderDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, copy_f9_look_probe_to_clipboard);
    }
}

// Developer F9 inspection copies camera, model, material and streaming state
// to the clipboard and a temporary report for diagnosing renderer inputs.
fn copy_f9_look_probe_to_clipboard(
    keys: Res<ButtonInput<KeyCode>>,
    the_camera: Option<Res<CCamera>>,
    file_loader: Option<Res<CFileLoader>>,
    world_players: CWorldPlayersRead,
    mesh_registry: Option<Res<GtaWorldMeshRegistry>>,
    model_assets: Option<Res<GtaWorldModelAssetCache>>,
    pass_state: Option<Res<GtaRenderPassPlanState>>,
    streaming: Option<Res<CStreaming>>,
    vis_query: Query<&Visibility>,
) {
    if !keys.just_pressed(KeyCode::F9) {
        return;
    }
    // Live Bevy visibility per world-entity slot: names the mesh that
    // actually draws when zone-cull hides the detail entity underneath.
    let mut slot_visibility: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();
    if let Some(registry) = mesh_registry.as_deref() {
        for (ent_idx, slot) in &registry.slots {
            let mut shown = 0usize;
            let mut hidden = 0usize;
            for bevy_entity in &slot.bevy_entities {
                match vis_query.get(*bevy_entity) {
                    Ok(Visibility::Hidden) => hidden += 1,
                    Ok(_) => shown += 1,
                    Err(_) => {}
                }
            }
            slot_visibility.insert(*ent_idx, format!("V{shown}H{hidden}"));
        }
    }
    let report = build_f9_probe_report(
        the_camera.as_deref(),
        file_loader.as_deref(),
        world_players.focused().map(|player| player.position()),
        mesh_registry.as_deref(),
        model_assets.as_deref(),
        pass_state
            .as_deref()
            .and_then(|state| state.last_plan.as_ref()),
        streaming.as_deref(),
        file_loader.as_deref().map(|loader| &loader.renderer),
        Some(&slot_visibility),
    );
    let file_path = std::env::temp_dir().join("rust_gta_3_f9_probe.txt");
    let file_ok = std::fs::write(&file_path, &report).is_ok();
    let clipboard_ok = copy_f9_probe_text_to_clipboard(&report);
    info!(clipboard_ok, file_ok, path = %file_path.display(), report_len = report.len(),
        "F9 renderer report saved");
}

#[cfg(target_os = "windows")]
fn copy_f9_probe_text_to_clipboard(text: &str) -> bool {
    use std::io::Write;
    let mut child = match std::process::Command::new("clip")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let wrote = match child.stdin.take() {
        Some(mut stdin) => stdin.write_all(text.as_bytes()).is_ok(),
        None => false,
    };
    wrote && child.wait().map(|status| status.success()).unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn copy_f9_probe_text_to_clipboard(_text: &str) -> bool {
    false
}

/// TEMP-DEBUG-F9-PROBE: project points onto the ray, keep those within
/// `max_perp`, sorted by distance along the ray. Pure so tests cover it.
pub(super) fn march_f9_ray(
    origin: [f32; 3],
    dir: [f32; 3],
    points: &[[f32; 3]],
    max_dist: f32,
    max_perp: f32,
) -> Vec<(usize, f32, f32)> {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len <= f32::EPSILON || max_dist <= 0.0 {
        return Vec::new();
    }
    let d = [dir[0] / len, dir[1] / len, dir[2] / len];
    let mut hits = Vec::new();
    for (index, point) in points.iter().enumerate() {
        let rx = point[0] - origin[0];
        let ry = point[1] - origin[1];
        let rz = point[2] - origin[2];
        let t = rx * d[0] + ry * d[1] + rz * d[2];
        if t < 0.0 || t > max_dist {
            continue;
        }
        let px = rx - d[0] * t;
        let py = ry - d[1] * t;
        let pz = rz - d[2] * t;
        let perp = (px * px + py * py + pz * pz).sqrt();
        if perp <= max_perp {
            hits.push((index, t, perp));
        }
    }
    hits.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));
    hits
}

/// Camera, resident model and visibility report for the F9 clipboard/file inspection.
/// Pure over borrowed render state so unit tests cover the formatting.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_f9_probe_report(
    camera: Option<&CCamera>,
    file_loader: Option<&CFileLoader>,
    player_pos: Option<CVector>,
    mesh_registry: Option<&GtaWorldMeshRegistry>,
    model_assets: Option<&GtaWorldModelAssetCache>,
    last_plan: Option<&RendererPassPlan>,
    streaming: Option<&CStreaming>,
    renderer: Option<&CRenderer>,
    slot_visibility: Option<&std::collections::HashMap<usize, String>>,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("RUST_GTA_3_F9_PROBE\n");
    let player_str = player_pos
        .map(|v| format!("{:.2},{:.2},{:.2}", v.x, v.y, v.z))
        .unwrap_or_else(|| "none".to_string());
    let _ = writeln!(out, "player={player_str}");
    let active = camera.and_then(|cam| cam.cams.get(cam.active_cam));
    let eye = active
        .map(|cam| [cam.source.x, cam.source.y, cam.source.z])
        .or_else(|| camera.map(|cam| [cam.position.x, cam.position.y, cam.position.z]))
        .or_else(|| player_pos.map(|v| [v.x, v.y, v.z]))
        .unwrap_or([0.0, 0.0, 0.0]);
    let raw_dir = active
        .map(|cam| [cam.front.x, cam.front.y, cam.front.z])
        .unwrap_or([0.0, 1.0, 0.0]);
    let dir_len =
        (raw_dir[0] * raw_dir[0] + raw_dir[1] * raw_dir[1] + raw_dir[2] * raw_dir[2]).sqrt();
    let dir = if dir_len > f32::EPSILON {
        [
            raw_dir[0] / dir_len,
            raw_dir[1] / dir_len,
            raw_dir[2] / dir_len,
        ]
    } else {
        [0.0, 1.0, 0.0]
    };
    let (yaw, pitch) = f9_yaw_pitch_deg(dir);
    let cam_pos = camera
        .map(|cam| {
            format!(
                "{:.2},{:.2},{:.2}",
                cam.position.x, cam.position.y, cam.position.z
            )
        })
        .unwrap_or_else(|| "none".to_string());
    let active_idx = camera.map(|cam| cam.active_cam).unwrap_or(usize::MAX);
    let _ = writeln!(
        out,
        "camera_pos={cam_pos} eye={:.2},{:.2},{:.2} dir={:.3},{:.3},{:.3} yaw={yaw:.1} pitch={pitch:.1} active_cam={active_idx}",
        eye[0], eye[1], eye[2], dir[0], dir[1], dir[2]
    );
    const MAX_DIST: f32 = 400.0;
    // Road origins sit several metres apart; a 4m tube misses the
    // ground below a slanted look direction. 12m still ranks by
    // distance/perp so the closest hit stays on top.
    const MAX_PERP: f32 = 12.0;
    let Some(loader) = file_loader else {
        let _ = writeln!(out, "world=none(no CFileLoader)");
        return out;
    };
    // Ray-march over world entity origins (gun-ray style): the first entries
    // are what the camera looks at, e.g. looking down means the road below.
    let mut order = Vec::with_capacity(loader.world.entities.len());
    let mut points = Vec::with_capacity(loader.world.entities.len());
    for (index, entity) in loader.world.entities.iter().enumerate() {
        order.push(index);
        points.push(entity.position);
    }
    let hits = march_f9_ray(eye, dir, &points, MAX_DIST, MAX_PERP);
    let _ = writeln!(
        out,
        "ray_hits={} (max_dist={MAX_DIST} max_perp={MAX_PERP} top8)",
        hits.len()
    );
    for (rank, (point_idx, dist, perp)) in hits.iter().take(8).enumerate() {
        let ent_idx = order[*point_idx];
        append_f9_entity_line(
            &mut out,
            loader,
            mesh_registry,
            model_assets,
            last_plan,
            streaming,
            slot_visibility,
            camera,
            renderer,
            ent_idx,
            Some((rank, *dist, *perp)),
        );
    }
    if hits.is_empty() {
        let _ = writeln!(out, "hit=none");
    }
    // Cull-zone ground truth for the camera: re3 drives zone transitions
    // from the game-camera position, and a camera inside a zone culls its
    // member list (open roads included). Report it so a white/hole ground
    // can be blamed on zone cull instead of materials.
    {
        let mut cam_zone: i32 = -1;
        let mut cam_bounds = String::from("none");
        for (zi, zone) in loader.world.cull_zones.resolved_zones.iter().enumerate() {
            if eye[0] >= zone.min[0]
                && eye[0] <= zone.max[0]
                && eye[1] >= zone.min[1]
                && eye[1] <= zone.max[1]
                && eye[2] >= zone.min[2]
                && eye[2] <= zone.max[2]
            {
                cam_zone = zi as i32;
                cam_bounds = format!(
                    "[{:.1},{:.1},{:.1}]-[{:.1},{:.1},{:.1}]",
                    zone.min[0], zone.min[1], zone.min[2], zone.max[0], zone.max[1], zone.max[2]
                );
                break;
            }
        }
        let _ = writeln!(
            out,
            "cam_zone={} bounds={} zones={} old={}",
            cam_zone,
            cam_bounds,
            loader.world.cull_zones.resolved_zones.len(),
            loader.world.cull_zones.old_cull_zone
        );
    }
    // Logic-vs-render camera divergence: the pass culls with the Bevy
    // transform position (renderer.camera_position) but the CCam basis, so
    // any gap here moves the frustum away from what the eye reports.
    if let Some(camera) = camera {
        let src = camera
            .cams
            .get(camera.active_cam)
            .map(|cam| cam.source)
            .unwrap_or(camera.position);
        let rendcam = loader.renderer.camera_position;
        let _ = writeln!(
            out,
            "cam_logic pos={:.2},{:.2},{:.2} src={:.2},{:.2},{:.2} rendcam={:.2},{:.2},{:.2} src_rendcam_dist={:.2}",
            camera.position.x,
            camera.position.y,
            camera.position.z,
            src.x,
            src.y,
            src.z,
            rendcam.x,
            rendcam.y,
            rendcam.z,
            world_distance(src, rendcam),
        );
    }
    // Ground directly below the eye (vertical drop): catches the road the
    // player stands on even when the slanted ray threads between origins.
    {
        let mut below: Vec<(usize, f32)> = loader
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.position[2] <= eye[2] + 1.0)
            .map(|(index, entity)| {
                let dx = entity.position[0] - eye[0];
                let dy = entity.position[1] - eye[1];
                (index, (dx * dx + dy * dy).sqrt())
            })
            .filter(|(_, d)| *d <= 12.0)
            .collect();
        below.sort_by(|a, b| a.1.total_cmp(&b.1));
        let _ = writeln!(
            out,
            "ground_below_eye={} (within 12m horiz, below eye+1 top5)",
            below.len()
        );
        for (index, hdist) in below.iter().take(5) {
            append_f9_entity_line(
                &mut out,
                loader,
                mesh_registry,
                model_assets,
                last_plan,
                streaming,
                slot_visibility,
                camera,
                renderer,
                *index,
                None,
            );
            let _ = writeln!(out, "  ground_hdist={hdist:.2}");
        }
    }
    // Ground-truth under the player: closest entities regardless of the ray.
    if let Some(player) = player_pos {
        let mut near: Vec<(usize, f32)> = loader
            .world
            .entities
            .iter()
            .enumerate()
            .map(|(index, entity)| {
                let dx = entity.position[0] - player.x;
                let dy = entity.position[1] - player.y;
                let dz = entity.position[2] - player.z;
                (index, (dx * dx + dy * dy + dz * dz).sqrt())
            })
            .filter(|(_, d)| *d <= 60.0)
            .collect();
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        let _ = writeln!(out, "near_player={} (within 60m top5)", near.len());
        for (index, dist) in near.iter().take(5) {
            append_f9_entity_line(
                &mut out,
                loader,
                mesh_registry,
                model_assets,
                last_plan,
                streaming,
                slot_visibility,
                camera,
                renderer,
                *index,
                None,
            );
            let _ = writeln!(out, "  near_dist={dist:.2}");
        }
    }
    // Meshes that actually draw near the player: names the white/untextured
    // ground when the detail underneath is zone-culled or off-screen.
    if let (Some(plan), Some(player)) = (last_plan, player_pos) {
        let mut drawn: Vec<(usize, f32, String)> = Vec::new();
        for ent_idx in plan.roads.iter().chain(plan.everything_bar_roads.iter()) {
            let bv = slot_visibility
                .and_then(|map| map.get(ent_idx))
                .cloned()
                .unwrap_or_else(|| "-".to_string());
            let shown: usize = bv
                .strip_prefix('V')
                .and_then(|rest| rest.split('H').next())
                .and_then(|shown| shown.parse().ok())
                .unwrap_or(0);
            if shown == 0 {
                continue;
            }
            if let Some(entity) = loader.world.entities.get(*ent_idx) {
                let dx = entity.position[0] - player.x;
                let dy = entity.position[1] - player.y;
                let dz = entity.position[2] - player.z;
                drawn.push((
                    *ent_idx,
                    (dx * dx + dy * dy + dz * dz).sqrt(),
                    format!("{} model={} bv={}", entity.name, entity.model_index, bv),
                ));
            }
        }
        drawn.sort_by(|a, b| a.1.total_cmp(&b.1));
        let _ = writeln!(
            out,
            "drawn_near_player={} (V>0 in plan, top8 by dist)",
            drawn.len()
        );
        for (ent_idx, dist, desc) in drawn.iter().take(8) {
            let _ = writeln!(out, "draw ent={ent_idx} {desc} dist={dist:.1}");
        }
    }
    if let Some(plan) = last_plan {
        let _ = writeln!(
            out,
            "plan roads={} everything={} fading_in={} invisible_prerender={} priority_stream={} stream_req={} rw_creates={} rw_deletes={}",
            plan.roads.len(),
            plan.everything_bar_roads.len(),
            plan.fading_in_entities.len(),
            plan.invisible_prerender.len(),
            plan.priority_stream_requests.len(),
            plan.stream_requests.len(),
            plan.rw_object_creates.len(),
            plan.rw_object_deletes.len(),
        );
    } else {
        let _ = writeln!(out, "plan=none");
    }
    if let Some(renderer) = renderer {
        let zonecull = if zone_cull_disabled(renderer) {
            "off"
        } else {
            "on"
        };
        let _ = writeln!(
            out,
            "renderer loading_priority={} cam_lod_mult={:.2} visible={} invisible={} stream_req={} zonecull={zonecull} rendcam={:.1},{:.1},{:.1}",
            renderer.loading_priority,
            renderer.camera_lod_dist_multiplier,
            renderer.visible_entities.len(),
            renderer.invisible_entities.len(),
            renderer.stream_requests.len(),
            renderer.camera_position.x,
            renderer.camera_position.y,
            renderer.camera_position.z,
        );
    }
    out
}

/// TEMP-DEBUG-F9-PROBE: one entity diagnostic line for the F9 report.
fn append_f9_entity_line(
    out: &mut String,
    loader: &CFileLoader,
    mesh_registry: Option<&GtaWorldMeshRegistry>,
    model_assets: Option<&GtaWorldModelAssetCache>,
    last_plan: Option<&RendererPassPlan>,
    streaming: Option<&CStreaming>,
    slot_visibility: Option<&std::collections::HashMap<usize, String>>,
    camera: Option<&CCamera>,
    renderer: Option<&CRenderer>,
    ent_idx: usize,
    ray: Option<(usize, f32, f32)>,
) {
    use std::fmt::Write as _;
    let Some(entity) = loader.world.entities.get(ent_idx) else {
        let _ = writeln!(out, "ent={ent_idx} missing");
        return;
    };
    let model = entity.model_index;
    let slot = loader.model_info.get_model_info(model);
    let (model_name, txd_slot, rw_loaded, ref_count, lod) = match slot {
        Some(slot) => {
            let base = slot.base();
            let lod = match slot {
                crate::modelinfo::ModelInfo::ModelInfoSlot::Simple(m)
                | crate::modelinfo::ModelInfo::ModelInfoSlot::Time(m) => Some(m.lod_distances),
                _ => None,
            };
            (
                base.name.clone(),
                base.txd_slot,
                base.rw_object_loaded,
                base.ref_count,
                lod,
            )
        }
        None => ("<no-slot>".to_string(), -1, false, 0, None),
    };
    let has_col = slot.is_some_and(|slot| slot.base().col_model.is_some());
    let (stream_state, stream_flags) = streaming
        .and_then(|s| s.info_for_model.get(model))
        .map(|info| (format!("{:?}", info.load_state), info.flags))
        .unwrap_or(("no-streaming".to_string(), 0));
    let (txd_name, txd_loaded) = if txd_slot >= 0 {
        let slot_id = txd_slot as usize;
        (
            loader
                .txd_store
                .get_txd_name(slot_id)
                .unwrap_or("<bad-slot>")
                .to_string(),
            loader.txd_store.is_txd_loaded(slot_id),
        )
    } else {
        ("<none>".to_string(), false)
    };
    let txd_gen = world_model_txd_generation(loader, model);
    let geom: Vec<String> = model_assets
        .map(|assets| {
            assets
                .geometry
                .iter()
                .filter(|((m, _), _)| *m == model)
                .map(|((_, lod), g)| {
                    format!("L{lod}:mesh={} rec={}", g.meshes.len(), g.recipes.len())
                })
                .collect()
        })
        .unwrap_or_default();
    let mats: Vec<String> = model_assets
        .map(|assets| {
            assets
                .entries
                .iter()
                .filter(|(key, _)| key.model_index == model)
                .map(|(key, a)| {
                    format!(
                        "L{}:tex={} missing={}",
                        key.lod_index, a.textured_count, a.missing_texture_count
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let spawned = mesh_registry
        .and_then(|reg| reg.slots.get(&ent_idx))
        .map(|slot| {
            format!(
                "n={} sub={} m={}",
                slot.bevy_entities.len(),
                slot.submesh_count,
                slot.model_index
            )
        })
        .unwrap_or_else(|| "none".to_string());
    // Live Bevy draw state: V = drawn submeshes, H = hidden ones.
    let bv = slot_visibility
        .and_then(|map| map.get(&ent_idx))
        .cloned()
        .unwrap_or_else(|| "-".to_string());
    let (in_roads, in_everything, in_invisible) = match last_plan {
        Some(plan) => (
            plan.roads.contains(&ent_idx),
            plan.everything_bar_roads.contains(&ent_idx),
            plan.invisible_prerender.contains(&ent_idx),
        ),
        None => (false, false, false),
    };
    let stream_req_hit = last_plan.is_some_and(|plan| {
        plan.stream_requests.contains(&ent_idx) || plan.stream_requests.contains(&model)
    });
    // TEMP-DEBUG-F9-PROBE: replay the exact visibility decision the render
    // pass makes for this entity (same camera source, FOV, LOD multiplier
    // and shared-model alpha the frame used). Free-roam F9 presses never run
    // with a cutscene FOV rescale, so the cutscene branch stays unscaled.
    // TEMP-DEBUG-F9-PROBE: render-plan gate (`IsEntityCullZoneVisible`,
    // `Renderer.cpp:1528`). `dec` replays the C++ visibility computation
    // (zone-blind, like the original); `gate` shows whether the plan admits it.
    let gate_str = renderer.map_or("-", |r| {
        if entity_cull_zone_visible(zone_cull_disabled(r), entity) {
            "open"
        } else {
            "culled"
        }
    });
    let mut decision_str = "-".to_string();
    let mut on_str = "-".to_string();
    let mut vis_extra = String::new();
    if let (Some(camera), Some(renderer)) = (camera, renderer) {
        let lod_mult = renderer.lod_distance_multiplier();
        let fov_src = if camera.script_mode == CameraScriptMode::FlyBy {
            camera.flyby_fov
        } else {
            camera
                .cams
                .get(camera.active_cam)
                .map(|cam| cam.fov)
                .unwrap_or(70.0)
        };
        let fov = CCamera::scaled_horizontal_fov(fov_src, camera.aspect_ratio, false);
        let rw_loaded = mesh_registry.is_some_and(|reg| reg.has_current_slot(ent_idx, model));
        let frustum = camera.frustum_for_source_and_fov(renderer.camera_position, fov);
        let esnap =
            world_entity_snapshot_for_camera(entity, &loader.model_info, Some(&frustum), rw_loaded);
        match world_model_snapshot(
            model,
            entity.position,
            &loader.model_info,
            renderer.camera_position,
            lod_mult,
            entity.is_big_building,
        ) {
            Some(msnap) => {
                let decision = if esnap.big_building {
                    renderer.setup_big_building_visibility(&esnap, &msnap)
                } else {
                    renderer.setup_entity_visibility(&esnap, &msnap)
                };
                let (bound_center, bound_radius) = match loader.model_info.col_model(model) {
                    Some(col) => {
                        let world = entity_world_point(
                            entity,
                            [
                                col.bounding_sphere.center.x,
                                col.bounding_sphere.center.y,
                                col.bounding_sphere.center.z,
                            ],
                        );
                        (
                            CVector::new(world[0], world[1], world[2]),
                            col.bounding_sphere.radius,
                        )
                    }
                    None => (
                        CVector::new(entity.position[0], entity.position[1], entity.position[2]),
                        0.0,
                    ),
                };
                let cam_space =
                    camera.camera_space_point_from_source(bound_center, renderer.camera_position);
                decision_str = format!("{:?}", decision.visibility);
                on_str = format!("{}/{}", esnap.on_screen, esnap.on_screen_complex);
                let _ = std::fmt::write(
                    &mut vis_extra,
                    format_args!(
                        " a={} dl={}/{} nf={} lfd={}/{} dist={:.1} bnd={:.1},{:.1},{:.1}+r{:.1} camsp={:.1},{:.1},{:.1} fov={:.0}",
                        msnap.alpha,
                        msnap.draw_last,
                        esnap.draw_last,
                        msnap.no_fade,
                        msnap.loaded_for_distance,
                        msnap.loaded_for_fade_distance,
                        decision.distance,
                        bound_center.x,
                        bound_center.y,
                        bound_center.z,
                        bound_radius,
                        cam_space.x,
                        cam_space.y,
                        cam_space.z,
                        fov,
                    ),
                );
            }
            None => decision_str = "nomodel".to_string(),
        }
    }
    let lod_str = lod
        .map(|l| format!("{:.0}/{:.0}/{:.0}", l[0], l[1], l[2]))
        .unwrap_or_else(|| "-".to_string());
    if let Some((rank, dist, perp)) = ray {
        let _ = writeln!(
            out,
            "hit#{rank} ent={ent_idx} name={name} model={model} modelname={model_name} type={:?} pos={:.1},{:.1},{:.1} dist={dist:.1} perp={perp:.2} vis={} zone={}/{} col_use={} hascol={has_col} rw={rw_loaded} ref={ref_count} lod={lod_str} stream={stream_state} flags={stream_flags} txd={txd_name}/{txd_loaded} txdgen={txd_gen} geom=[{geom}] mats=[{mats}] spawned={spawned} bv={bv} plan={in_roads}/{in_everything}/{in_invisible}/{stream_req_hit} gate={gate_str} dec={decision_str} on={on_str}{vis_extra}",
            entity.entity_type,
            entity.position[0],
            entity.position[1],
            entity.position[2],
            entity.is_visible,
            entity.zone_culled,
            entity.zone_culled2,
            entity.uses_collision,
            name = entity.name,
            geom = geom.join(" "),
            mats = mats.join(" "),
        );
    } else {
        let _ = writeln!(
            out,
            "near ent={ent_idx} name={name} model={model} modelname={model_name} type={:?} pos={:.1},{:.1},{:.1} vis={} zone={}/{} col_use={} hascol={has_col} rw={rw_loaded} ref={ref_count} lod={lod_str} stream={stream_state} txd={txd_name}/{txd_loaded} spawned={spawned} bv={bv} plan={in_roads}/{in_everything}/{in_invisible}/{stream_req_hit} gate={gate_str} dec={decision_str} on={on_str}{vis_extra}",
            entity.entity_type,
            entity.position[0],
            entity.position[1],
            entity.position[2],
            entity.is_visible,
            entity.zone_culled,
            entity.zone_culled2,
            entity.uses_collision,
            name = entity.name,
        );
    }
}

/// GTA heading convention (`atan2(x, y)`) plus pitch.
pub(super) fn f9_yaw_pitch_deg(dir: [f32; 3]) -> (f32, f32) {
    let yaw = dir[0].atan2(dir[1]).to_degrees();
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let pitch = if len > f32::EPSILON {
        (dir[2] / len).asin().to_degrees()
    } else {
        0.0
    };
    (yaw, pitch)
}
