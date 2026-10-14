//! Bevy presentation backend for re3 `CHud::Draw`, `DrawAfterFade` and `CRadar`.
//! All coordinates below are the original PC HUD's 640×448 virtual screen.

use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemParam,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::widget::NodeImageMode,
};
use rustgta3_assets::install::OriginalGtaInstall::OriginalGtaInstall;
use rustgta3_core::core::{Clock::CClock, Coordinates::CVector2D, Timer::CTimer};
use rustgta3_formats::formats::TexRead::{decode_txd_texture, read_txd};
use std::hash::{DefaultHasher, Hash, Hasher};

use super::HudActors::HudActorsRead;
#[path = "HudRetained.rs"]
mod retained;
#[path = "HudRadar.rs"]
pub mod radar;
use retained::{HudChild, HudPainter, RetainedHud};

use crate::{
    control::{
        OnscreenTimer::{COUNTER_DISPLAY_NUMBER, COnscreenTimer},
        Script::{CTheScripts, MAX_NUM_SCRIPT_SPRITES},
        ScriptEntities::ScriptGameplayBridge,
    },
    core::{
        Cam::eCamMode,
        Camera::CCamera,
        FileLoader::CFileLoader,
        Frontend::CMenuManager,
        FrontendFlow::{
            FrontendTxdTextures, bitmap_text_advance_width, bitmap_text_handle, ensure_hud_textures,
        },
        LocalCoop::{LocalCoopCameras, LocalCoopSession, LocalCoopSplitLayout},
        PlayerInfo::CPlayerInfo,
        Radar::{
            CRadar, RADAR_BOTTOM, RADAR_HEIGHT, RADAR_LEFT, RADAR_MIN_X, RADAR_MIN_Y,
            RADAR_NUM_TILES, RADAR_TILE_SIZE, RADAR_WIDTH, eBlipDisplay, eBlipType, eRadarSprite,
        },
    },
    peds::PlayerRead::CWorldPlayersRead,
    renderer::{
        Font::{FONT_BANK, FONT_HEADING, FONT_PAGER},
        Hud::{CHud, ITEM_ARMOUR, ITEM_HEALTH, ITEM_RADAR},
    },
    text::Text::CText,
    weapons::WeaponType::eWeaponType,
};

const W: f32 = 640.0;
const H: f32 = 448.0;
const MONEY: Color = Color::srgba_u8(89, 115, 150, 255);
const HEALTH: Color = Color::srgba_u8(186, 101, 50, 255);
const ARMOUR: Color = Color::srgba_u8(124, 140, 95, 255);
const WANTED: Color = Color::srgba_u8(193, 164, 120, 255);
const CLOCK: Color = Color::srgba_u8(194, 165, 120, 255);
const TIMER: Color = Color::srgba_u8(186, 101, 50, 255);
const COUNTER: Color = Color::srgba_u8(0, 106, 164, 255);
const TIMER_RIGHT_OFFSET: f32 = 34.0;
const ZONE: [u8; 3] = [152, 154, 82];
const VEHICLE: [u8; 3] = [194, 165, 120];

struct HudCanvasLayout {
    size: Vec2,
    offset: Vec2,
}

fn hud_canvas_layout(viewport: Vec2) -> HudCanvasLayout {
    let scale = (viewport.x / W).min(viewport.y / H).max(0.0);
    let size = Vec2::new(W, H) * scale;
    HudCanvasLayout {
        size,
        offset: (viewport - size) * 0.5,
    }
}

#[derive(Component)]
pub struct GtaHudRoot;

#[derive(Default, Resource)]
pub struct GtaHudRasterCache {
    /// Fingerprint of the last submitted draw inputs; unchanged HUD frames
    /// need no presentation work. Live timers advance before this gate.
    last_draw_fingerprint: Option<u64>,
    retained: [RetainedHud; 2],
}

#[derive(Resource)]
pub struct GtaScriptUiTextureCache {
    txd_name: Option<[u8; 8]>,
    sprite_names: [[u8; 8]; MAX_NUM_SCRIPT_SPRITES],
    sprite_images: [Option<Handle<Image>>; MAX_NUM_SCRIPT_SPRITES],
}

impl Default for GtaScriptUiTextureCache {
    fn default() -> Self {
        Self {
            txd_name: None,
            sprite_names: [[0; 8]; MAX_NUM_SCRIPT_SPRITES],
            sprite_images: std::array::from_fn(|_| None),
        }
    }
}

#[derive(SystemParam)]
pub struct GtaHudContext<'w, 's> {
    players: Option<CWorldPlayersRead<'w, 's>>,
    player_info: Option<Res<'w, CPlayerInfo>>,
    clock: Option<Res<'w, CClock>>,
    camera: Option<Res<'w, CCamera>>,
    local_coop: Option<Res<'w, LocalCoopSession>>,
    local_coop_cameras: Option<Res<'w, LocalCoopCameras>>,
    timer: Option<Res<'w, CTimer>>,
    file_loader: Option<Res<'w, CFileLoader>>,
    script_entities: Option<HudActorsRead<'w, 's>>,
    install: Option<Res<'w, OriginalGtaInstall>>,
    menu: Option<Res<'w, CMenuManager>>,
    scripts: Option<Res<'w, CTheScripts>>,
    bridge: Option<Res<'w, ScriptGameplayBridge>>,
    onscreen_timer: Option<Res<'w, COnscreenTimer>>,
    text: Option<Res<'w, CText>>,
}

pub fn flush_hud(
    mut commands: Commands,
    existing: Query<Entity, With<GtaHudRoot>>,
    mut hud: Option<ResMut<CHud>>,
    context: GtaHudContext,
    mut txd: ResMut<FrontendTxdTextures>,
    mut images: ResMut<Assets<Image>>,
    mut raster: ResMut<GtaHudRasterCache>,
    mut script_ui_textures: ResMut<GtaScriptUiTextureCache>,
    mut radar_gpu: radar::RadarGpu,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let GtaHudContext {
        players,
        player_info,
        clock,
        camera,
        local_coop,
        local_coop_cameras,
        timer,
        file_loader,
        script_entities,
        install,
        menu,
        scripts,
        bridge,
        onscreen_timer,
        text,
    } = context;
    // CHud::Draw suppresses ordinary HUD in widescreen, but subtitles and
    // SCM text are outside that gate. DrawAfterFade's help also remains live.
    let hidden = menu.as_ref().is_some_and(|menu| menu.menu_active);
    let widescreen = camera.as_ref().is_some_and(|camera| camera.wide_screen_on)
        || bridge.as_ref().is_some_and(|bridge| bridge.widescreen);
    if hidden {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        raster.last_draw_fingerprint = None;
        for retained in &mut raster.retained { retained.clear(); }
        radar_gpu.retire_all();
        return;
    }
    let frame = timer.as_ref().map(|t| t.frame_counter()).unwrap_or(0);
    let now_ms = timer
        .as_ref()
        .map(|t| t.time_in_milliseconds())
        .unwrap_or(0);
    let Some(hud) = hud.as_mut() else {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        raster.last_draw_fingerprint = None;
        for retained in &mut raster.retained { retained.clear(); }
        radar_gpu.retire_all();
        return;
    };
    let step_ms = timer
        .as_ref()
        .map(|t| t.time_step_in_milliseconds())
        .unwrap_or(16);
    let time_step = timer.as_ref().map(|t| t.time_step()).unwrap_or(1.0);
    hud.process_draw_state(step_ms, time_step);
    let normal_hud_visible = hud.wants_to_draw_hud && !widescreen;
    ensure_hud_textures(&mut txd, &mut images, install.as_deref());
    sync_script_ui_textures(
        &mut script_ui_textures,
        scripts.as_deref(),
        install.as_deref(),
        &mut images,
    );
    let Some(players) = players.as_ref() else {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        raster.last_draw_fingerprint = None;
        for retained in &mut raster.retained { retained.clear(); }
        radar_gpu.retire_all();
        return;
    };
    let split = local_coop
        .as_ref()
        .is_some_and(|session| session.enabled && session.player_two_spawned)
        && players
            .player(1)
            .is_some_and(|player| player.active_in_world);
    let player_indices = if split {
        [Some(0usize), Some(1usize)]
    } else {
        [Some(players.player_in_focus()), None]
    };
    let window_size = windows
        .single()
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(Vec2::new(W, H));
    let fingerprint = hud_draw_fingerprint(
        &*hud,
        players,
        player_info.as_deref(),
        clock.as_deref(),
        camera.as_deref(),
        local_coop.as_deref(),
        local_coop_cameras.as_deref(),
        file_loader.as_deref(),
        script_entities.as_ref(),
        install.is_some(),
        menu.as_deref(),
        scripts.as_deref(),
        onscreen_timer.as_deref(),
        &txd,
        split,
        player_indices,
        frame,
        now_ms,
    );
    let mut viewport_hash = DefaultHasher::new();
    fingerprint.hash(&mut viewport_hash);
    normal_hud_visible.hash(&mut viewport_hash);
    widescreen.hash(&mut viewport_hash);
    window_size.x.to_bits().hash(&mut viewport_hash);
    window_size.y.to_bits().hash(&mut viewport_hash);
    let fingerprint = viewport_hash.finish();
    if raster.last_draw_fingerprint == Some(fingerprint) && !existing.is_empty() {
        return;
    }
    if !normal_hud_visible {
        radar_gpu.retire_all();
    }
    for index in 0..2 {
        if !player_indices.contains(&Some(index)) {
            if let Some(root) = raster.retained[index].root { commands.entity(root).despawn(); }
            raster.retained[index].clear();
            radar_gpu.retire_player(index);
        }
    }
    for local_player in player_indices.into_iter().flatten() {
        let Some(player) = players.player(local_player as i32) else {
            continue;
        };
        let view_camera = if local_player == 1 {
            local_coop_cameras
                .as_ref()
                .map(|cameras| &cameras.player_two)
        } else {
            camera.as_deref()
        };
        let split_layout = local_coop
            .as_ref()
            .map_or(LocalCoopSplitLayout::TopBottom, |session| {
                session.split_layout
            });
        let viewport_size = Vec2::new(
            window_size.x
                / if split && split_layout == LocalCoopSplitLayout::LeftRight {
                    2.0
                } else {
                    1.0
                },
            window_size.y
                / if split && split_layout == LocalCoopSplitLayout::TopBottom {
                    2.0
                } else {
                    1.0
                },
        );
        // Keep the authored 640x448 geometry at one uniform scale. Both text
        // and its panels share this canvas, including in split-screen views.
        let canvas = hud_canvas_layout(viewport_size);
        let canvas_size = canvas.size;
        let offset = canvas.offset
            + Vec2::new(
                if split && split_layout == LocalCoopSplitLayout::LeftRight {
                    local_player as f32 * viewport_size.x
                } else {
                    0.0
                },
                if split && split_layout == LocalCoopSplitLayout::TopBottom {
                    local_player as f32 * viewport_size.y
                } else {
                    0.0
                },
            );
        let root_node = Node {
            position_type: PositionType::Absolute,
            left: Val::Px(offset.x), top: Val::Px(offset.y),
            width: Val::Px(canvas_size.x), height: Val::Px(canvas_size.y),
            overflow: Overflow::clip(), ..default()
        };
        let retained = &mut raster.retained[local_player];
        if retained.root.is_some_and(|root| existing.get(root).is_err()) {
            retained.clear();
        }
        let root = if let Some(root) = retained.root {
            if retained.root_node.as_ref() != Some(&root_node) {
                commands.entity(root).insert(root_node.clone());
            }
            root
        } else {
            let root = commands.spawn((GtaHudRoot, bevy::picking::Pickable::IGNORE, root_node.clone(), GlobalZIndex(19_100),
                Name::new(format!("re3 CHud Player {}", local_player + 1)))).id();
            retained.root = Some(root);
            root
        };
        retained.root_node = Some(root_node);
        let mut draws = std::mem::take(&mut retained.draws);
        draws.clear();
        {
        let p = &mut HudPainter { draws: &mut draws, parent: None };
            if normal_hud_visible {
                let cam_mode = view_camera
                    .and_then(|c| c.cams.get(c.active_cam))
                    .map(|c| c.mode);
                let in_1st_person_sight = matches!(
                    cam_mode,
                    Some(eCamMode::MODE_ROCKETLAUNCHER | eCamMode::MODE_SNIPER | eCamMode::MODE_M16_1STPERSON)
                );
                if let Some(mode) = cam_mode {
                    match mode {
                        eCamMode::MODE_M16_1STPERSON => texture(p, &txd, "sitem16",
                            W * 0.5 - 32.0, H * 0.5 - 32.0, 64.0, 64.0, Color::WHITE, "M16Sight"),
                        eCamMode::MODE_ROCKETLAUNCHER => texture(
                            p,
                            &txd,
                            "siterocket",
                            W * 0.5 - 40.0,
                            H * 0.5 - 40.0,
                            80.0,
                            80.0,
                            Color::srgba_u8(100, 200, 100, 255),
                            "RocketSight",
                        ),
                        eCamMode::MODE_SNIPER => {
                            if let Some(handle) = txd.images.get("sitesniper") {
                                // CHud::Draw mirrors the authored quarter-scope four times.
                                for (right, bottom) in [(false,false), (true,false), (false,true), (true,true)] {
                                    p.spawn((Node { position_type: PositionType::Absolute,
                                        left: pct(W * 0.5 - if right { 0.0 } else { 210.0 }, W),
                                        top: pct(H * 0.5 - if bottom { 0.0 } else { 210.0 }, H),
                                        width: pct(210.0,W), height: pct(210.0,H), ..default() },
                                        ImageNode { image: handle.clone(), image_mode: NodeImageMode::Stretch,
                                            flip_x: right, flip_y: bottom, ..default() }, Name::new("SniperSight")));
                                }
                            }
                        },
                        _ => {}
                    }
                }
                let player_in_vehicle = players.relation(local_player as i32).is_some();
                let current_wep = player.ped.current_weapon as usize;
                let current_wep_type = player.ped.weapons.get(current_wep).map(|w| w.weapon_type);
                let has_crosshair_weapon = matches!(
                    current_wep_type,
                    Some(
                        eWeaponType::WEAPONTYPE_COLT45
                            | eWeaponType::WEAPONTYPE_UZI
                            | eWeaponType::WEAPONTYPE_SHOTGUN
                            | eWeaponType::WEAPONTYPE_AK47
                            | eWeaponType::WEAPONTYPE_M16
                            | eWeaponType::WEAPONTYPE_SNIPERRIFLE
                            | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
                            | eWeaponType::WEAPONTYPE_FLAMETHROWER
                    )
                );
                if !in_1st_person_sight && !player_in_vehicle && has_crosshair_weapon {
                    texture(
                        p,
                        &txd,
                        "sitem16",
                        (viewport_size.x * 0.53 - canvas.offset.x) / canvas_size.x * W - 16.0,
                        (viewport_size.y * 0.4 - canvas.offset.y) / canvas_size.y * H - 16.0,
                        32.0,
                        32.0,
                        Color::srgba_u8(255, 255, 255, 230),
                        "Crosshair",
                    );
                }
                // Radar is painted first, then its black disc frame and blips.
                if hud.item_to_flash != ITEM_RADAR || frame & 8 != 0 {
                    if let Some(loader) = file_loader.as_ref() {
                        // C++ CRadar::CalculateCachedSinCos uses the active
                        // camera's Front.Heading(), including the camera/player
                        // axis conversion and the north-up top-down special case.
                        let player_vehicle = players
                            .relation(local_player as i32)
                            .and_then(|relation| {
                                script_entities
                                    .as_ref()?
                                    .vehicle_handle_for_entity(relation.vehicle)
                            })
                            .and_then(|handle| script_entities.as_ref()?.vehicle_snapshot(handle));
                        let radar_heading =
                            radar_camera_heading(view_camera, player.locomotion.rotation_cur);
                        let radar_range = radar_range_for_speed(
                            player_vehicle.as_ref().map_or(0.0, |vehicle| vehicle.speed),
                        );
                        let player_heading = player_vehicle
                            .as_ref()
                            .map(|vehicle| vehicle.heading_degrees.to_radians())
                            .unwrap_or(player.locomotion.rotation_cur);
                        // `FindPlayerCentreOfWorld_NoSniperShift()` follows the
                        // occupied vehicle in re3. Using the ped transform while
                        // driving offsets both the sampled map and every blip.
                        let radar_origin = player_vehicle
                            .as_ref()
                            .map(|vehicle| vehicle.position)
                            .unwrap_or_else(|| player.position());
                        // re3 `CRadar::DrawBlips`: the centre sprite is rotated by
                        // the player's heading relative to the same camera heading
                        // used for radar-space transforms.
                        let centre_angle = player_heading - (std::f32::consts::PI + radar_heading);
                        radar_gpu.draw(p, loader, &mut images, local_player,
                            radar_origin.x, radar_origin.y, radar_heading, radar_range);
                        texture(
                            p,
                            &txd,
                            "radardisc",
                            RADAR_LEFT - 4.0,
                            H - RADAR_BOTTOM - RADAR_HEIGHT - 4.0,
                            RADAR_WIDTH + 8.0,
                            RADAR_HEIGHT + 8.0,
                            Color::BLACK,
                            "RadarDisc",
                        );
                        draw_blips(
                            p,
                            &loader.radar,
                            &txd,
                            script_entities.as_ref(),
                            radar_origin.x,
                            radar_origin.y,
                            radar_heading,
                            radar_range,
                            centre_angle,
                            scripts
                                .as_deref()
                                .is_some_and(CTheScripts::is_player_on_a_mission),
                        );
                    }
                }

                let hours = clock.as_ref().map(|c| c.hours()).unwrap_or(0);
                let minutes = clock.as_ref().map(|c| c.minutes()).unwrap_or(0);
                outlined_text(
                    p,
                    &mut txd,
                    &mut images,
                    &format!("{hours:02}:{minutes:02}"),
                    FONT_HEADING,
                    0.8,
                    1.35,
                    W - 111.0,
                    22.0,
                    Align::Right,
                    CLOCK,
                    2.0,
                );
                let money = player_info.as_ref().map(|i| i.m_nVisibleMoney).unwrap_or(0);
                outlined_text(
                    p,
                    &mut txd,
                    &mut images,
                    &format!("${money:08}"),
                    FONT_HEADING,
                    0.8,
                    1.35,
                    W - 110.0,
                    43.0,
                    Align::Right,
                    MONEY,
                    2.0,
                );

                let weapon_type = player.ped.current_weapon as usize;
                let weapon_names = [
                    "fist",
                    "bat",
                    "pistol",
                    "uzi",
                    "shotgun",
                    "ak47",
                    "m16",
                    "sniper",
                    "rocket",
                    "flame",
                    "molotov",
                    "grenade",
                    "detonator",
                ];
                if let Some(name) = weapon_names.get(weapon_type) {
                    texture(
                        p,
                        &txd,
                        name,
                        W - 99.0,
                        27.0,
                        64.0,
                        64.0,
                        Color::WHITE,
                        "WeaponIcon",
                    );
                }
                if weapon_type > 1 && weapon_type < player.ped.weapons.len() {
                    let weapon = &player.ped.weapons[weapon_type];
                    let ammo = if weapon_type == 9 {
                        format!(
                            "{}-{}",
                            ((weapon.ammo_total - weapon.ammo_in_clip) / 10).min(9999),
                            weapon.ammo_in_clip / 10
                        )
                    } else {
                        format!(
                            "{}-{}",
                            (weapon.ammo_total - weapon.ammo_in_clip).min(9999),
                            weapon.ammo_in_clip
                        )
                    };
                    bitmap(
                        p,
                        &mut txd,
                        &mut images,
                        &ammo,
                        FONT_BANK,
                        0.4,
                        0.6,
                        W - 66.0,
                        73.0,
                        Align::Center,
                        Color::BLACK,
                        false,
                        "Ammo",
                    );
                }

                if (hud.item_to_flash != ITEM_HEALTH || frame & 8 != 0)
                    && (player.vitality.health >= 10.0 || frame & 8 != 0)
                {
                    outlined_text(
                        p,
                        &mut txd,
                        &mut images,
                        &format!("{:03}", player.vitality.health.round().clamp(0.0, 999.0) as i32),
                        FONT_HEADING,
                        0.8,
                        1.35,
                        W - 110.0,
                        65.0,
                        Align::Right,
                        HEALTH,
                        2.0,
                    );
                    outlined_text(
                        p,
                        &mut txd,
                        &mut images,
                        "{",
                        FONT_HEADING,
                        0.8,
                        1.35,
                        W - 166.0,
                        65.0,
                        Align::Right,
                        HEALTH,
                        2.0,
                    );
                }
                if player.vitality.armour > 1.0 && (hud.item_to_flash != ITEM_ARMOUR || frame & 8 != 0) {
                    outlined_text(
                        p,
                        &mut txd,
                        &mut images,
                        &format!("{:03}", player.vitality.armour.round().clamp(0.0, 999.0) as i32),
                        FONT_HEADING,
                        0.8,
                        1.35,
                        W - 182.0,
                        65.0,
                        Align::Right,
                        ARMOUR,
                        2.0,
                    );
                    outlined_text(
                        p,
                        &mut txd,
                        &mut images,
                        "[",
                        FONT_HEADING,
                        0.8,
                        1.35,
                        W - 236.0,
                        65.0,
                        Align::Right,
                        ARMOUR,
                        2.0,
                    );
                }
                for i in 0..6 {
                    // re3 only flashes lit stars for two seconds after the wanted
                    // level changes. Afterwards the active stars stay solid.
                    let wanted_settled =
                        now_ms > player.wanted.m_nLastWantedLevelChange.saturating_add(2000);
                    let lit =
                        player.wanted.get_wanted_level() > i && (wanted_settled || frame & 4 != 0);
                    outlined_text(
                        p,
                        &mut txd,
                        &mut images,
                        "]",
                        FONT_HEADING,
                        0.8,
                        1.35,
                        W - 60.0 - i as f32 * 23.0,
                        87.0,
                        Align::Left,
                        if lit { WANTED } else { Color::BLACK },
                        2.0,
                    );
                }

                if let Some(onscreen_timer) = onscreen_timer.as_deref() {
                    draw_onscreen_timer(
                        p,
                        &mut txd,
                        &mut images,
                        onscreen_timer,
                        &hud,
                        text.as_deref(),
                        frame,
                    );
                }

                if hud.pager_on != 0 {
                    let x = 26.0 - hud.pager_x_offset;
                    texture(
                        p,
                        &txd,
                        "pager",
                        x,
                        27.0,
                        160.0,
                        80.0,
                        Color::WHITE,
                        "Pager",
                    );
                    bitmap(
                        p,
                        &mut txd,
                        &mut images,
                        &wide(&hud.pager_message),
                        FONT_PAGER,
                        0.84,
                        1.0,
                        52.0 - hud.pager_x_offset,
                        54.0,
                        Align::Left,
                        Color::srgba_u8(32, 162, 66, 205),
                        false,
                        "PagerText",
                    );
                }
            }

            if let (Some(scripts), Some(text)) = (scripts.as_deref(), text.as_deref()) {
                draw_script_ui(
                    p,
                    &mut txd,
                    &mut images,
                    scripts,
                    text,
                    &script_ui_textures,
                    true,
                );
            }

            after_fade_canvas(p).with_children(|p| {
            if hud.help_message_state != 0 && !hud.help_message_to_print.is_empty() {
                let alpha = match hud.help_message_state {
                    1 if hud.help_message_fade_timer > 0 => 225.0,
                    2..=4 => (hud.help_message_fade_timer as f32 / 1000.0).clamp(0.0, 1.0) * 225.0,
                    _ => 0.0,
                };
                let s = wrap_bitmap_text(
                    &txd,
                    &wide(&hud.help_message_to_print),
                    FONT_BANK,
                    0.52,
                    196.0,
                );
                let help_colour = Color::srgba_u8(175, 175, 175, 255);
                let help_height = bitmap_text_handle(
                    &mut txd,
                    &mut images,
                    &s,
                    FONT_BANK,
                    0.52,
                    1.1,
                    help_colour,
                    false,
                )
                .map_or(0.0, |raster| raster.bitmap_height / txd_scale_y(&txd));
                panel(
                    p,
                    22.0,
                    24.0 + (150.0 - hud.pager_x_offset) * 0.6,
                    208.0,
                    8.0 + help_height,
                    Color::srgba_u8(0, 0, 0, (alpha * 0.9) as u8),
                );
                bitmap(
                    p,
                    &mut txd,
                    &mut images,
                    &s,
                    FONT_BANK,
                    0.52,
                    1.1,
                    26.0,
                    28.0 + (150.0 - hud.pager_x_offset) * 0.6,
                    Align::Left,
                    Color::srgba_u8(175, 175, 175, alpha as u8),
                    false,
                    "HelpMessage",
                );
            }

            if let (Some(scripts), Some(text)) = (scripts.as_deref(), text.as_deref()) {
                draw_script_ui(
                    p,
                    &mut txd,
                    &mut images,
                    scripts,
                    text,
                    &script_ui_textures,
                    false,
                );
            }

            });

            if normal_hud_visible && hud.big_message[2].is_empty() {
                draw_right_names(p, &mut txd, &mut images, hud);
            }
            draw_messages(
                p,
                &mut txd,
                &mut images,
                hud,
                !widescreen
                    || menu
                        .as_ref()
                        .is_none_or(|menu| menu.prefs_show_subtitles != 0),
            );
            // main.cpp draws a black full-screen quad when brightness is below
            // the neutral 256 midpoint (with a floor of 50). It is deliberately
            // last so HUD and world are dimmed together.
            if let Some(brightness) = menu
                .as_ref()
                .map(|menu| menu.prefs_brightness)
                .filter(|brightness| *brightness < 256)
            {
                let alpha = (256 - brightness.max(50)) as f32 / 256.0;
                panel(p, 0.0, 0.0, W, H, Color::srgba(0.0, 0.0, 0.0, alpha));
            }
        }
        let retained = &mut raster.retained[local_player];
        retained.draws = draws;
        retained.submit(root, &mut commands);
    }
    raster.last_draw_fingerprint = Some(fingerprint);
}

/// Hash of every value the HUD draw code reads, so an unchanged hash proves a
/// draw descriptions would be identical and reconciliation can be skipped. Conventions used throughout:
/// - drawn strings/colours/visibilities are hashed post-quantisation (the
///   exact values the widgets receive), never raw timers;
/// - continuous values (positions, headings, ranges) are quantised below a
///   radar pixel, so only visible changes flip the hash;
/// - resource presence gating whole blocks (`file_loader`, `install`) is
///   hashed; entity snapshots only contribute resolved values because every
///   use has a value fallback.
#[allow(clippy::too_many_arguments)]
fn hud_draw_fingerprint(
    hud: &CHud,
    players: &CWorldPlayersRead,
    player_info: Option<&CPlayerInfo>,
    clock: Option<&CClock>,
    camera: Option<&CCamera>,
    local_coop: Option<&LocalCoopSession>,
    local_coop_cameras: Option<&LocalCoopCameras>,
    file_loader: Option<&CFileLoader>,
    script_entities: Option<&HudActorsRead>,
    install_present: bool,
    menu: Option<&CMenuManager>,
    scripts: Option<&CTheScripts>,
    onscreen_timer: Option<&COnscreenTimer>,
    txd: &FrontendTxdTextures,
    split: bool,
    player_indices: [Option<usize>; 2],
    frame: u32,
    now_ms: u32,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    split.hash(&mut hasher);
    player_indices.hash(&mut hasher);
    players.player_in_focus().hash(&mut hasher);
    players
        .player(1)
        .map(|player| player.active_in_world)
        .hash(&mut hasher);
    local_coop
        .map(|session| (session.enabled, session.player_two_spawned))
        .hash(&mut hasher);
    let split_layout = local_coop.map_or(LocalCoopSplitLayout::TopBottom, |session| {
        session.split_layout
    });
    (match split_layout {
        LocalCoopSplitLayout::TopBottom => 0u8,
        LocalCoopSplitLayout::LeftRight => 1u8,
    })
    .hash(&mut hasher);
    install_present.hash(&mut hasher);
    txd.loaded.hash(&mut hasher);
    txd.hud_loaded.hash(&mut hasher);
    txd.images.len().hash(&mut hasher);
    let (text_scale_x, text_scale_y) = txd.text_scale();
    text_scale_x.to_bits().hash(&mut hasher);
    text_scale_y.to_bits().hash(&mut hasher);
    file_loader.is_some().hash(&mut hasher);
    file_loader.map(|loader| loader.txd_store.store_revision()).hash(&mut hasher);
    menu.map(|menu| menu.prefs_show_subtitles).hash(&mut hasher);
    menu.map(|menu| menu.prefs_brightness).hash(&mut hasher);
    scripts
        .is_some_and(CTheScripts::is_player_on_a_mission)
        .hash(&mut hasher);
    if let Some(scripts) = scripts {
        scripts
            .number_of_intro_text_lines_this_frame
            .hash(&mut hasher);
        scripts.script_texture_dictionary.hash(&mut hasher);
        scripts.script_sprite_names.hash(&mut hasher);
        for line in scripts
            .intro_text_lines
            .iter()
            .take(scripts.number_of_intro_text_lines_this_frame as usize)
        {
            line.text_key.hash(&mut hasher);
            line.scale.map(f32::to_bits).hash(&mut hasher);
            line.color.hash(&mut hasher);
            line.justify.hash(&mut hasher);
            line.centered.hash(&mut hasher);
            line.right_justify.hash(&mut hasher);
            line.background.hash(&mut hasher);
            line.background_only.hash(&mut hasher);
            line.background_color.hash(&mut hasher);
            line.wrap_x.to_bits().hash(&mut hasher);
            line.center_size.to_bits().hash(&mut hasher);
            line.proportional.hash(&mut hasher);
            line.before_fade.hash(&mut hasher);
            line.font.hash(&mut hasher);
            line.position.map(f32::to_bits).hash(&mut hasher);
        }
        scripts
            .number_of_intro_rectangles_this_frame
            .hash(&mut hasher);
        for rect in scripts
            .intro_rectangles
            .iter()
            .take(scripts.number_of_intro_rectangles_this_frame as usize)
        {
            rect.is_used.hash(&mut hasher);
            rect.before_fade.hash(&mut hasher);
            rect.texture_id.hash(&mut hasher);
            rect.rect.map(f32::to_bits).hash(&mut hasher);
            rect.color.hash(&mut hasher);
        }
    }
    if let Some(onscreen_timer) = onscreen_timer {
        onscreen_timer.processed.hash(&mut hasher);
        let entry = &onscreen_timer.entries[0];
        entry.timer_processed.hash(&mut hasher);
        entry.counter_processed.hash(&mut hasher);
        entry.timer_buffer.hash(&mut hasher);
        entry.counter_buffer.hash(&mut hasher);
        entry.timer_text.hash(&mut hasher);
        entry.counter_text.hash(&mut hasher);
        entry.counter_type.hash(&mut hasher);
        if hud.timer_flash_timer != 0 || hud.counter_flash_timer != 0 {
            (frame & 4).hash(&mut hasher);
        }
        hud.timer_flash_timer.hash(&mut hasher);
        hud.counter_flash_timer.hash(&mut hasher);
    }
    // Blink phases only matter while the corresponding flash is armed;
    // hashing them unconditionally would rebuild every few frames forever.
    let flash8 = if hud.item_to_flash == ITEM_RADAR
        || hud.item_to_flash == ITEM_HEALTH
        || hud.item_to_flash == ITEM_ARMOUR
    {
        frame & 8
    } else {
        0
    };
    hud.item_to_flash.hash(&mut hasher);
    flash8.hash(&mut hasher);
    hud.pager_on.hash(&mut hasher);
    hud.pager_x_offset.to_bits().hash(&mut hasher);
    hud.pager_message.hash(&mut hasher);
    hud.help_message_state.hash(&mut hasher);
    hud.help_message_to_print.hash(&mut hasher);
    let help_alpha = match hud.help_message_state {
        1 if hud.help_message_fade_timer > 0 => 225.0,
        2..=4 => (hud.help_message_fade_timer as f32 / 1000.0).clamp(0.0, 1.0) * 225.0,
        _ => 0.0,
    };
    ((help_alpha * 0.9) as u8).hash(&mut hasher);
    (help_alpha as u8).hash(&mut hasher);
    hud.zone_state.hash(&mut hasher);
    hud.zone_to_print.hash(&mut hasher);
    (((hud.zone_fade_timer as f32 / 1000.0) * 255.0).clamp(0.0, 255.0) as u8).hash(&mut hasher);
    hud.vehicle_state.hash(&mut hasher);
    hud.vehicle_name_to_print.hash(&mut hasher);
    (((hud.vehicle_fade_timer as f32 / 1000.0) * 255.0).clamp(0.0, 255.0) as u8).hash(&mut hasher);
    hud.message.hash(&mut hasher);
    hud.odd_job2_on.hash(&mut hasher);
    hud.odd_job2_timer.hash(&mut hasher);
    hud.odd_job2_x_offset.to_bits().hash(&mut hasher);
    hud.odd_job2_off_timer.to_bits().hash(&mut hasher);
    for (text, alpha) in hud.big_message.iter().zip(hud.big_message_alpha.iter()) {
        text.hash(&mut hasher);
        (*alpha as u8).hash(&mut hasher);
    }
    for local_player in player_indices.into_iter().flatten() {
        local_player.hash(&mut hasher);
        let Some(player) = players.player(local_player as i32) else {
            continue;
        };
        let view_camera = if local_player == 1 {
            local_coop_cameras
                .as_ref()
                .map(|cameras| &cameras.player_two)
        } else {
            camera
        };
        let sight = view_camera
            .and_then(|c| c.cams.get(c.active_cam))
            .map(|c| c.mode);
        sight.map(|mode| mode as usize).hash(&mut hasher);
        // Mirrors the draw gate: the radar block (and its snapshot reads)
        // only exists when the flash state shows it.
        let radar_visible = hud.item_to_flash != ITEM_RADAR || flash8 != 0;
        radar_visible.hash(&mut hasher);
        if radar_visible {
            if let Some(loader) = file_loader {
                // Same active-camera heading as the draw code.
                let player_vehicle = players
                    .relation(local_player as i32)
                    .and_then(|relation| {
                        script_entities
                            .as_ref()?
                            .vehicle_handle_for_entity(relation.vehicle)
                    })
                    .and_then(|handle| script_entities.as_ref()?.vehicle_snapshot(handle));
                let radar_heading = radar_camera_heading(view_camera, player.locomotion.rotation_cur);
                let radar_range = radar_range_for_speed(
                    player_vehicle.as_ref().map_or(0.0, |vehicle| vehicle.speed),
                );
                let position = player_vehicle
                    .as_ref()
                    .map(|vehicle| vehicle.position)
                    .unwrap_or_else(|| player.position());
                position.x.to_bits().hash(&mut hasher);
                position.y.to_bits().hash(&mut hasher);
                radar_heading.to_bits().hash(&mut hasher);
                radar_range.to_bits().hash(&mut hasher);
                player_vehicle.as_ref().map(|v| v.heading_degrees.to_bits()).hash(&mut hasher);
                player.locomotion.rotation_cur.to_bits().hash(&mut hasher);
                hash_radar_blips(&mut hasher, &loader.radar, script_entities);
            }
        }
        clock
            .as_ref()
            .map(|c| c.hours())
            .unwrap_or(0)
            .hash(&mut hasher);
        clock
            .as_ref()
            .map(|c| c.minutes())
            .unwrap_or(0)
            .hash(&mut hasher);
        player_info
            .as_ref()
            .map(|i| i.m_nVisibleMoney)
            .unwrap_or(0)
            .hash(&mut hasher);
        let weapon_type = player.ped.current_weapon as usize;
        weapon_type.hash(&mut hasher);
        player.ped.weapons.len().hash(&mut hasher);
        if weapon_type > 1 && weapon_type < player.ped.weapons.len() {
            let weapon = &player.ped.weapons[weapon_type];
            weapon.ammo_total.hash(&mut hasher);
            weapon.ammo_in_clip.hash(&mut hasher);
        }
        let health_text = player.vitality.health.round().clamp(0.0, 999.0) as i32;
        let health_visible = (hud.item_to_flash != ITEM_HEALTH || flash8 != 0)
            && (player.vitality.health >= 10.0 || flash8 != 0);
        health_visible.hash(&mut hasher);
        health_text.hash(&mut hasher);
        let armour_visible =
            player.vitality.armour > 1.0 && (hud.item_to_flash != ITEM_ARMOUR || flash8 != 0);
        armour_visible.hash(&mut hasher);
        (player.vitality.armour.round().clamp(0.0, 999.0) as i32).hash(&mut hasher);
        let wanted = player.wanted.get_wanted_level();
        wanted.hash(&mut hasher);
        let wanted_settled = now_ms > player.wanted.m_nLastWantedLevelChange.saturating_add(2000);
        (if wanted > 0 && !wanted_settled {
            frame & 4
        } else {
            0
        })
        .hash(&mut hasher);
        wanted_settled.hash(&mut hasher);
    }
    hasher.finish()
}

/// Blip draw inputs: identity fields plus quantised world position. Screen
/// position and alpha are pure functions of these plus the already-hashed
/// player position/heading/range, so they are covered transitively.
fn hash_radar_blips(
    hasher: &mut DefaultHasher,
    radar: &CRadar,
    entities: Option<&HudActorsRead>,
) {
    for trace in radar.radar_trace.iter().filter(|t| {
        t.in_use
            && matches!(
                t.blip_display,
                eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY | eBlipDisplay::BLIP_DISPLAY_BOTH
            )
    }) {
        let world_position = match trace.blip_type {
            eBlipType::BLIP_CAR => entities
                .and_then(|e| e.vehicle_snapshot(trace.entity_handle))
                .map(|snapshot| snapshot.position),
            eBlipType::BLIP_CHAR => entities.and_then(|e| {
                let snapshot = e.char_snapshot(trace.entity_handle)?;
                // re3 CRadar::DrawBlips follows a ped's current vehicle
                // when the blip belongs to a character that is seated.
                // Drawing the ped transform itself leaves mission/contact
                // icons visibly behind the car.
                snapshot
                    .current_vehicle
                    .and_then(|vehicle| e.vehicle_snapshot(vehicle))
                    .map(|vehicle| vehicle.position)
                    .or(Some(snapshot.position))
            }),
            eBlipType::BLIP_OBJECT => entities
                .and_then(|entities| entities.object_get(trace.entity_handle))
                .map(|entity| entity.object.physical.entity.get_position()),
            _ => None,
        };
        let world = match trace.blip_type {
            eBlipType::BLIP_CAR | eBlipType::BLIP_CHAR | eBlipType::BLIP_OBJECT => {
                let Some(position) = world_position else {
                    continue;
                };
                (position.x, position.y)
            }
            eBlipType::BLIP_COORD | eBlipType::BLIP_CONTACT_POINT => {
                (trace.vec2d_pos.x, trace.vec2d_pos.y)
            }
            _ => continue,
        };
        (trace.blip_type as u32).hash(hasher);
        (trace.radar_sprite as u16).hash(hasher);
        trace.color.hash(hasher);
        trace.scale.hash(hasher);
        trace.dim.hash(hasher);
        (trace.blip_display as u16).hash(hasher);
        trace.entity_handle.hash(hasher);
        ((world.0 * 2.0).round() as i32).hash(hasher);
        ((world.1 * 2.0).round() as i32).hash(hasher);
    }
}

fn draw_right_names(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    hud: &CHud,
) {
    if hud.message.is_empty() {
        if let Some(text) = hud.zone_to_print.as_ref().filter(|_| hud.zone_state != 0) {
            let a = ((hud.zone_fade_timer as f32 / 1000.0) * 255.0).clamp(0.0, 255.0) as u8;
            outlined_text(
                p,
                txd,
                images,
                &wide(text),
                FONT_BANK,
                1.2,
                1.2,
                W - 32.0,
                H - 30.0,
                Align::Right,
                Color::srgba_u8(ZONE[0], ZONE[1], ZONE[2], a),
                1.0,
            );
        }
        if let Some(text) = hud
            .vehicle_name_to_print
            .as_ref()
            .filter(|_| hud.vehicle_state != 0)
        {
            let a = ((hud.vehicle_fade_timer as f32 / 1000.0) * 255.0).clamp(0.0, 255.0) as u8;
            outlined_text(
                p,
                txd,
                images,
                &wide(text),
                FONT_BANK,
                1.2,
                1.2,
                W - 32.0,
                H - 55.0,
                Align::Right,
                Color::srgba_u8(VEHICLE[0], VEHICLE[1], VEHICLE[2], a),
                1.0,
            );
        }
    }
}

fn draw_messages(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    hud: &CHud,
    show_subtitles: bool,
) {
    if show_subtitles && !hud.message.is_empty() && hud.big_message[2].is_empty() {
        let message = wrap_bitmap_text(txd, &wide(&hud.message), FONT_BANK, 0.48, W - 106.0);
        bitmap(
            p,
            txd,
            images,
            &message,
            FONT_BANK,
            0.48,
            1.12,
            W * 0.5,
            H - 68.0,
            Align::Center,
            Color::srgba_u8(235, 235, 235, 255),
            true,
            "Subtitle",
        );
    }
    if !hud.big_message[0].is_empty() {
        let a = hud.big_message_alpha[0] as u8;
        shadowed_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[0]),
            FONT_HEADING,
            1.8,
            1.8,
            W * 0.5,
            H * 0.5 - 18.0,
            Align::Center,
            Color::srgba_u8(85, 119, 133, a),
            Color::srgba_u8(0, 0, 0, a),
            2.0,
            2.0,
        );
    }
    if !hud.big_message[2].is_empty() {
        let a = hud.big_message_alpha[2] as u8;
        shadowed_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[2]),
            FONT_HEADING,
            2.0,
            2.0,
            W - 20.0,
            H - 82.0,
            Align::Right,
            Color::srgba_u8(170, 123, 87, a),
            Color::srgba_u8(0, 0, 0, ((a as f32) * 0.75) as u8),
            4.0,
            4.0,
        );
    }
    after_fade_canvas(p).with_children(|p| {
    if !hud.big_message[3].is_empty() {
        outlined_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[3]),
            FONT_BANK,
            1.2,
            1.5,
            W * 0.5,
            H * 0.5 - 84.0,
            Align::Center,
            Color::srgba_u8(89, 115, 150, 255),
            2.0,
        );
    }
    if hud.big_message[1].is_empty() && !hud.big_message[4].is_empty() {
        shadowed_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[4]),
            FONT_BANK,
            1.2,
            1.5,
            W * 0.5,
            H * 0.5 - 84.0,
            Align::Center,
            Color::srgba_u8(89, 115, 150, 255),
            Color::BLACK,
            -2.0,
            -2.0,
        );
    }
    if hud.big_message[1].is_empty()
        && !hud.big_message[5].is_empty()
        && hud.odd_job2_off_timer <= 0.0
    {
        outlined_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[5]),
            FONT_BANK,
            1.0,
            1.2,
            W * 0.5,
            H * 0.5 - 20.0,
            Align::Center,
            Color::srgba_u8(156, 91, 40, 255),
            2.0,
        );
    }
    if !hud.big_message[1].is_empty() {
        let a = hud.big_message_alpha[1] as u8;
        shadowed_text(
            p,
            txd,
            images,
            &wide(&hud.big_message[1]),
            FONT_HEADING,
            1.04,
            1.6,
            W - 20.0,
            H - 120.0,
            Align::Right,
            Color::srgba_u8(220, 172, 2, a),
            Color::srgba_u8(40, 40, 40, a),
            2.0,
            2.0,
        );
    }
    });
}

/// re3 `CRadar::CalculateCachedSinCos`: radar space follows the active
/// camera's forward heading, while top-down cameras keep north at the top.
fn radar_camera_heading(camera: Option<&CCamera>, fallback: f32) -> f32 {
    let Some(camera) = camera else {
        return fallback;
    };
    if matches!(
        camera.cams.get(camera.active_cam).map(|cam| cam.mode),
        Some(eCamMode::MODE_TOPDOWN | eCamMode::MODE_TOP_DOWN_PED)
    ) {
        0.0
    } else {
        camera.follow_ped_heading()
    }
}

fn draw_blips(
    p: &mut HudPainter,
    radar: &CRadar,
    txd: &FrontendTxdTextures,
    entities: Option<&HudActorsRead<'_, '_>>,
    px: f32,
    py: f32,
    heading: f32,
    range: f32,
    centre_angle: f32,
    player_on_mission: bool,
) {
    // Player centre arrow and north marker are always present in re3.
    texture_rotated(
        p,
        txd,
        "radar_centre",
        RADAR_LEFT + RADAR_WIDTH / 2.0 - 8.0,
        H - RADAR_BOTTOM - RADAR_HEIGHT + RADAR_HEIGHT / 2.0 - 8.0,
        16.0,
        16.0,
        Color::WHITE,
        centre_angle,
        "RadarCentre",
    );
    let (sin, cos) = heading.sin_cos();
    let to_radar = |world: CVector2D| {
        let x = (world.x - px) / range;
        let y = (world.y - py) / range;
        CVector2D::new(sin * y + cos * x, cos * y - sin * x)
    };
    let limit = |point: &mut CVector2D| {
        let dist = (point.x * point.x + point.y * point.y).sqrt();
        if dist > 1.0 {
            point.x /= dist;
            point.y /= dist;
        }
        dist
    };
    let to_screen = |point: CVector2D| {
        CVector2D::new(
            (point.x + 1.0) * 0.5 * RADAR_WIDTH + RADAR_LEFT,
            (1.0 - point.y) * 0.5 * RADAR_HEIGHT + H - (RADAR_BOTTOM + RADAR_HEIGHT),
        )
    };
    let mut north = to_radar(CVector2D::new(px, py + std::f32::consts::SQRT_2 * range));
    limit(&mut north);
    let mut screen = to_screen(north);
    texture(
        p,
        txd,
        "radar_north",
        screen.x - 8.0,
        screen.y - 8.0,
        16.0,
        16.0,
        Color::WHITE,
        "RadarNorth",
    );
    for trace in radar.radar_trace.iter().filter(|t| {
        t.in_use
            && matches!(
                t.blip_display,
                eBlipDisplay::BLIP_DISPLAY_BLIP_ONLY | eBlipDisplay::BLIP_DISPLAY_BOTH
            )
    }) {
        // re3 CRadar::DrawBlips suppresses every coordinate contact-point
        // blip while the SCM mission flag is set. MAIN.SCM creates the three
        // save-house contact points early; this gate is what makes them appear
        // only after the intro/mission state allows them.
        if trace.blip_type == eBlipType::BLIP_CONTACT_POINT && player_on_mission {
            continue;
        }
        let world_position = match trace.blip_type {
            eBlipType::BLIP_CAR => entities
                .and_then(|e| e.vehicle_snapshot(trace.entity_handle))
                .map(|snapshot| snapshot.position),
            eBlipType::BLIP_CHAR => entities.and_then(|e| {
                let snapshot = e.char_snapshot(trace.entity_handle)?;
                snapshot
                    .current_vehicle
                    .and_then(|vehicle| e.vehicle_snapshot(vehicle))
                    .map(|vehicle| vehicle.position)
                    .or(Some(snapshot.position))
            }),
            eBlipType::BLIP_OBJECT => entities
                .and_then(|entities| entities.object_get(trace.entity_handle))
                .map(|entity| entity.object.physical.entity.get_position()),
            _ => None,
        };
        let world = match trace.blip_type {
            eBlipType::BLIP_CAR | eBlipType::BLIP_CHAR | eBlipType::BLIP_OBJECT => {
                let Some(position) = world_position else {
                    continue;
                };
                CVector2D::new(position.x, position.y)
            }
            eBlipType::BLIP_COORD | eBlipType::BLIP_CONTACT_POINT => trace.vec2d_pos,
            _ => continue,
        };
        let mut point = to_radar(world);
        let dist = limit(&mut point);
        screen = to_screen(point);
        if let Some(name) = radar_sprite_name(trace.radar_sprite) {
            texture(
                p,
                txd,
                name,
                screen.x - 8.0,
                screen.y - 8.0,
                16.0,
                16.0,
                Color::srgba_u8(255, 255, 255, CRadar::calculate_blip_alpha(dist)),
                "RadarBlip",
            );
        } else {
            let rgba = CRadar::get_radar_trace_colour(trace.color, trace.dim).to_be_bytes();
            // re3 `ShowRadarTrace` draws a one-unit black backing rectangle
            // before the coloured trace, giving unsprited blips their original
            // outlined look.
            let outer = trace.scale as f32 + 1.0;
            panel(
                p,
                screen.x - outer,
                screen.y - outer,
                outer * 2.0,
                outer * 2.0,
                Color::BLACK,
            );
            panel(
                p,
                screen.x - trace.scale as f32,
                screen.y - trace.scale as f32,
                trace.scale as f32 * 2.0,
                trace.scale as f32 * 2.0,
                Color::srgba_u8(rgba[0], rgba[1], rgba[2], 255),
            );
        }
    }
}

fn radar_sprite_name(sprite: eRadarSprite) -> Option<&'static str> {
    Some(match sprite {
        eRadarSprite::RADAR_SPRITE_ASUKA => "radar_asuka",
        eRadarSprite::RADAR_SPRITE_BOMB => "radar_bomb",
        eRadarSprite::RADAR_SPRITE_CAT => "radar_cat",
        eRadarSprite::RADAR_SPRITE_COPCAR => "radar_copcar",
        eRadarSprite::RADAR_SPRITE_DON => "radar_don",
        eRadarSprite::RADAR_SPRITE_EIGHT => "radar_eight",
        eRadarSprite::RADAR_SPRITE_EL => "radar_el",
        eRadarSprite::RADAR_SPRITE_ICE => "radar_ice",
        eRadarSprite::RADAR_SPRITE_JOEY => "radar_joey",
        eRadarSprite::RADAR_SPRITE_KENJI => "radar_kenji",
        eRadarSprite::RADAR_SPRITE_LIZ => "radar_liz",
        eRadarSprite::RADAR_SPRITE_LUIGI => "radar_luigi",
        eRadarSprite::RADAR_SPRITE_RAY => "radar_ray",
        eRadarSprite::RADAR_SPRITE_SAL => "radar_sal",
        eRadarSprite::RADAR_SPRITE_SAVE => "radar_save",
        eRadarSprite::RADAR_SPRITE_SPRAY => "radar_spray",
        eRadarSprite::RADAR_SPRITE_TONY => "radar_tony",
        eRadarSprite::RADAR_SPRITE_WEAPON => "radar_weapon",
        _ => return None,
    })
}

/// re3 `CRadar::DrawMap` speed interpolation (`RADAR_MIN/MAX_*`).
fn radar_range_for_speed(speed: f32) -> f32 {
    const MIN_RANGE: f32 = 120.0;
    const MAX_RANGE: f32 = 350.0;
    const MIN_SPEED: f32 = 0.3;
    const MAX_SPEED: f32 = 0.9;
    if speed < MIN_SPEED {
        MIN_RANGE
    } else if speed < MAX_SPEED {
        (speed - MIN_SPEED) / (MAX_SPEED - MIN_SPEED) * (MAX_RANGE - MIN_RANGE) + MIN_RANGE
    } else {
        MAX_RANGE
    }
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Center,
    Right,
}
fn wide(v: &[u16]) -> String {
    let n = v.iter().position(|&c| c == 0).unwrap_or(v.len());
    let text = &v[..n];
    let mut visible = Vec::with_capacity(text.len());
    let mut index = 0;
    while index < text.len() {
        if text[index] == b'~' as u16
            && let Some(token_end) = text[index + 1..]
                .iter()
                .position(|character| *character == b'~' as u16)
                .map(|offset| index + 1 + offset)
        {
            // re3 `CFont::ParseToken`: colour/button tokens affect font state
            // and consume no glyph; N/n starts a new line.
            if text[index + 1..token_end] == [b'n' as u16]
                || text[index + 1..token_end] == [b'N' as u16]
            {
                visible.push(b'\n' as u16);
            }
            index = token_end + 1;
            continue;
        }
        visible.push(text[index]);
        index += 1;
    }
    String::from_utf16_lossy(&visible)
}

fn wrap_bitmap_text(
    txd: &FrontendTxdTextures,
    text: &str,
    style: i16,
    scale_x: f32,
    max_width: f32,
) -> String {
    let max_width = max_width.max(1.0);
    let mut output = String::new();
    for (paragraph_index, paragraph) in text.split('\n').enumerate() {
        if paragraph_index != 0 {
            output.push('\n');
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty()
                && bitmap_text_advance_width(txd, &candidate, style, scale_x) > max_width
            {
                output.push_str(&line);
                output.push('\n');
                line.clear();
                line.push_str(word);
            } else {
                line = candidate;
            }
        }
        output.push_str(&line);
    }
    output
}

fn draw_script_ui(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    scripts: &CTheScripts,
    text: &CText,
    script_textures: &GtaScriptUiTextureCache,
    before_fade: bool,
) {
    for rect in scripts
        .intro_rectangles
        .iter()
        .take(scripts.number_of_intro_rectangles_this_frame as usize)
        .filter(|rect| rect.is_used && rect.before_fade == before_fade)
    {
        if rect.texture_id >= 0 {
            if let Ok(slot) = usize::try_from(rect.texture_id)
                && let Some(handle) = script_textures
                    .sprite_images
                    .get(slot)
                    .and_then(Option::as_ref)
            {
                let [left, top, right, bottom] = rect.rect;
                image_flipped_y(
                    p,
                    handle.clone(),
                    left.min(right),
                    top.min(bottom),
                    (right - left).abs(),
                    (bottom - top).abs(),
                    Color::srgba_u8(rect.color[0], rect.color[1], rect.color[2], rect.color[3]),
                    "ScriptSprite",
                );
            }
            continue;
        }
        let [left, top, right, bottom] = rect.rect;
        panel(
            p,
            left.min(right),
            top.min(bottom),
            (right - left).abs(),
            (bottom - top).abs(),
            Color::srgba_u8(rect.color[0], rect.color[1], rect.color[2], rect.color[3]),
        );
    }

    for line in scripts
        .intro_text_lines
        .iter()
        .take(scripts.number_of_intro_text_lines_this_frame as usize)
        .filter(|line| line.before_fade == before_fade && line.text_key[0] != 0)
    {
        let raw = wide(&text.get_key_bytes(&line.text_key));
        if raw.is_empty() {
            continue;
        }
        let style = line.font.clamp(0, 2) as i16;
        let sx = line.scale[0].max(0.01);
        // re3's PC HUD halves script text Y scale in both Draw paths.
        let sy = (line.scale[1] * 0.5).max(0.01);
        let x = W - line.position[0];
        let y = H - line.position[1];
        let align = if line.centered {
            Align::Center
        } else if line.right_justify {
            Align::Right
        } else {
            Align::Left
        };
        let wrap_width = if line.centered {
            line.center_size.abs().max(1.0)
        } else {
            (line.wrap_x - x).abs().max(1.0)
        }
        .min(W);
        let rendered = wrap_bitmap_text(txd, &raw, style, sx, wrap_width);
        let text_width = rendered
            .lines()
            .map(|part| bitmap_text_advance_width(txd, part, style, sx))
            .fold(0.0_f32, f32::max);
        let line_count = rendered.lines().count().max(1) as f32;
        let line_height = 20.0 * sy;
        if line.background {
            let background_width = if line.background_only {
                text_width + 8.0
            } else {
                wrap_width + 8.0
            };
            let left = match align {
                Align::Left => x - 4.0,
                Align::Center => x - background_width * 0.5,
                Align::Right => x - background_width + 4.0,
            };
            panel(
                p,
                left,
                y - 3.0,
                background_width,
                line_height * line_count + 6.0,
                Color::srgba_u8(
                    line.background_color[0],
                    line.background_color[1],
                    line.background_color[2],
                    line.background_color[3],
                ),
            );
        }
        bitmap(
            p,
            txd,
            images,
            &rendered,
            style,
            sx,
            sy,
            x,
            y,
            align,
            Color::srgba_u8(line.color[0], line.color[1], line.color[2], line.color[3]),
            false,
            "ScriptText",
        );
    }
}

fn script_key_name(key: &[u8; 8]) -> String {
    let end = key.iter().position(|byte| *byte == 0).unwrap_or(key.len());
    String::from_utf8_lossy(&key[..end])
        .trim()
        .to_ascii_lowercase()
}

fn sync_script_ui_textures(
    cache: &mut GtaScriptUiTextureCache,
    scripts: Option<&CTheScripts>,
    install: Option<&OriginalGtaInstall>,
    images: &mut Assets<Image>,
) {
    let (txd_name, sprite_names) = scripts
        .map(|scripts| {
            (
                scripts.script_texture_dictionary,
                scripts.script_sprite_names,
            )
        })
        .unwrap_or((None, [[0; 8]; MAX_NUM_SCRIPT_SPRITES]));
    if cache.txd_name == txd_name && cache.sprite_names == sprite_names {
        return;
    }

    cache.txd_name = txd_name;
    cache.sprite_names = sprite_names;
    cache.sprite_images = std::array::from_fn(|_| None);

    let (Some(txd_key), Some(install)) = (txd_name, install) else {
        return;
    };
    let txd_name = script_key_name(&txd_key);
    if txd_name.is_empty() {
        return;
    }
    // re3's script opcode spells this as MODELS/<name>.txd. Some supported
    // PC distributions ship the same loose script dictionaries under TXD/,
    // so preserve the authored name while accepting either retail layout.
    let model_path = format!("MODELS\\{txd_name}.txd");
    let txd_path = format!("TXD\\{txd_name}.txd");
    let (bytes, gta_path) = match install.read(&model_path) {
        Ok(bytes) => (bytes, model_path),
        Err(_) => match install.read(&txd_path) {
            Ok(bytes) => (bytes, txd_path),
            Err(_) => {
                warn!("script TXD load failed: {model_path} / {txd_path}");
                return;
            }
        },
    };
    let Some(summary) = read_txd(&bytes) else {
        warn!("script TXD parse failed: {gta_path}");
        return;
    };

    for descriptor in &summary.textures {
        let requested_slots: Vec<usize> = sprite_names
            .iter()
            .enumerate()
            .filter_map(|(slot, name)| {
                let requested = script_key_name(name);
                (!requested.is_empty() && requested.eq_ignore_ascii_case(&descriptor.name))
                    .then_some(slot)
            })
            .collect();
        if requested_slots.is_empty() {
            continue;
        }
        let Some(decoded) = decode_txd_texture(descriptor) else {
            warn!(
                "script TXD texture decode failed: {} in {gta_path}",
                descriptor.name
            );
            continue;
        };
        let image = Image::new(
            Extent3d {
                width: decoded.width,
                height: decoded.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            decoded.rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        let handle = images.add(image);
        for slot in requested_slots {
            cache.sprite_images[slot] = Some(handle.clone());
        }
    }
}
fn pct(v: f32, b: f32) -> Val {
    Val::Percent(v / b * 100.0)
}

fn bitmap(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    text: &str,
    style: i16,
    sx: f32,
    sy: f32,
    x: f32,
    y: f32,
    align: Align,
    colour: Color,
    shadow: bool,
    name: &'static str,
) {
    // CFont changes vertex alpha without recreating its TXD raster. Cache
    // text coverage at full alpha and let the UI shader apply the live fade.
    let alpha = colour.alpha();
    let Some(r) = bitmap_text_handle(txd, images, text, style, sx, sy, colour.with_alpha(1.0), shadow) else {
        return;
    };
    let lw = r.bitmap_width / txd_scale_x(txd);
    let lh = r.bitmap_height / txd_scale_y(txd);
    let advance = r.advance_width / txd_scale_x(txd);
    let left = match align {
        Align::Left => x,
        Align::Center => x - advance / 2.0,
        Align::Right => x - advance,
    };
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: pct(left, W),
            top: pct(y, H),
            width: pct(lw, W),
            height: pct(lh, H),
            ..default()
        },
        ImageNode {
            image: r.handle,
            image_mode: NodeImageMode::Stretch,
            color: Color::WHITE.with_alpha(alpha),
            ..default()
        },
        Name::new(name),
    ));
}
fn txd_scale_x(txd: &FrontendTxdTextures) -> f32 {
    txd.text_scale().0
}
fn txd_scale_y(txd: &FrontendTxdTextures) -> f32 {
    txd.text_scale().1
}

fn draw_onscreen_timer(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    onscreen_timer: &COnscreenTimer,
    hud: &CHud,
    text: Option<&CText>,
    frame: u32,
) {
    if !onscreen_timer.processed {
        return;
    }
    let entry = &onscreen_timer.entries[0];

    if entry.timer_processed && (frame & 4 != 0 || hud.timer_flash_timer == 0) {
        outlined_text(
            p,
            txd,
            images,
            &entry.timer_buffer,
            FONT_HEADING,
            0.8,
            1.35,
            W - TIMER_RIGHT_OFFSET,
            110.0,
            Align::Right,
            TIMER,
            2.0,
        );
        if !entry.timer_text.is_empty() {
            let label = text
                .map(|text| wide(&text.get(&entry.timer_text)))
                .unwrap_or_else(|| entry.timer_text.clone());
            outlined_text(
                p,
                txd,
                images,
                &label,
                FONT_HEADING,
                0.8 * 0.8,
                1.35,
                W - TIMER_RIGHT_OFFSET - 80.0,
                110.0,
                Align::Right,
                TIMER,
                2.0,
            );
        }
    }

    if entry.counter_processed && (frame & 4 != 0 || hud.counter_flash_timer == 0) {
        if entry.counter_type == COUNTER_DISPLAY_NUMBER {
            outlined_text(
                p,
                txd,
                images,
                &entry.counter_buffer,
                FONT_HEADING,
                0.8,
                1.35,
                W - TIMER_RIGHT_OFFSET,
                132.0,
                Align::Right,
                COUNTER,
                2.0,
            );
        } else {
            let counter = entry
                .counter_buffer
                .parse::<i32>()
                .unwrap_or(0)
                .clamp(0, 100) as f32;
            let x = W - TIMER_RIGHT_OFFSET - 50.0 + 4.0;
            panel(p, x, 140.0, 50.0, 11.0, Color::srgba_u8(0, 106, 164, 80));
            panel(p, x, 140.0, counter * 0.5, 11.0, COUNTER);
        }

        if !entry.counter_text.is_empty() {
            let label = text
                .map(|text| wide(&text.get(&entry.counter_text)))
                .unwrap_or_else(|| entry.counter_text.clone());
            outlined_text(
                p,
                txd,
                images,
                &label,
                FONT_HEADING,
                0.8,
                1.35,
                W - TIMER_RIGHT_OFFSET - 61.0,
                132.0,
                Align::Right,
                COUNTER,
                2.0,
            );
        }
    }
}

fn outlined_text(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    text: &str,
    style: i16,
    sx: f32,
    sy: f32,
    x: f32,
    y: f32,
    align: Align,
    colour: Color,
    offset: f32,
) {
    shadowed_text(
        p,
        txd,
        images,
        text,
        style,
        sx,
        sy,
        x,
        y,
        align,
        colour,
        Color::BLACK,
        offset,
        offset,
    );
}

#[allow(clippy::too_many_arguments)]
fn shadowed_text(
    p: &mut HudPainter,
    txd: &mut FrontendTxdTextures,
    images: &mut Assets<Image>,
    text: &str,
    style: i16,
    sx: f32,
    sy: f32,
    x: f32,
    y: f32,
    align: Align,
    colour: Color,
    shadow_colour: Color,
    shadow_x: f32,
    shadow_y: f32,
) {
    bitmap(
        p,
        txd,
        images,
        text,
        style,
        sx,
        sy,
        x + shadow_x,
        y + shadow_y,
        align,
        shadow_colour,
        false,
        "HudTextShadow",
    );
    bitmap(
        p, txd, images, text, style, sx, sy, x, y, align, colour, false, "HudText",
    );
}
fn texture(
    p: &mut HudPainter,
    txd: &FrontendTxdTextures,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    label: &'static str,
) {
    if let Some(handle) = txd.images.get(name) {
        image(p, handle.clone(), x, y, w, h, color, label)
    }
}

#[allow(clippy::too_many_arguments)]
fn texture_rotated(
    p: &mut HudPainter,
    txd: &FrontendTxdTextures,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    rotation_radians: f32,
    label: &'static str,
) {
    if let Some(handle) = txd.images.get(name) {
        image_rotated(
            p,
            handle.clone(),
            x,
            y,
            w,
            h,
            color,
            rotation_radians,
            label,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn image_rotated(
    p: &mut HudPainter,
    handle: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    rotation_radians: f32,
    label: &'static str,
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: pct(x, W),
            top: pct(y, H),
            width: pct(w, W),
            height: pct(h, H),
            ..default()
        },
        // re3 DrawRotatingRadarSprite is not a plain rotation: its custom
        // vertex order reflects the texture vertically and then applies the
        // radar angle. Bevy's UI transform applies scale before its clockwise
        // rotation, so (1,-1) plus -angle reproduces the RenderWare matrix.
        // Omitting the reflection makes radar_centre point exactly backwards.
        UiTransform {
            scale: Vec2::new(1.0, -1.0),
            rotation: Rot2::radians(-rotation_radians),
            ..UiTransform::IDENTITY
        },
        ImageNode {
            image: handle,
            image_mode: NodeImageMode::Stretch,
            color,
            ..default()
        },
        Name::new(label),
    ));
}
fn image(
    p: &mut HudPainter,
    handle: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    label: &'static str,
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: pct(x, W),
            top: pct(y, H),
            width: pct(w, W),
            height: pct(h, H),
            ..default()
        },
        ImageNode {
            image: handle,
            image_mode: NodeImageMode::Stretch,
            color,
            ..default()
        },
        Name::new(label),
    ));
}

fn image_flipped_y(
    p: &mut HudPainter,
    handle: Handle<Image>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    label: &'static str,
) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: pct(x, W),
            top: pct(y, H),
            width: pct(w, W),
            height: pct(h, H),
            ..default()
        },
        UiTransform {
            scale: Vec2::new(1.0, -1.0),
            ..UiTransform::IDENTITY
        },
        ImageNode {
            image: handle,
            image_mode: NodeImageMode::Stretch,
            color,
            ..default()
        },
        Name::new(label),
    ));
}
/// CHud::DrawAfterFade is above CCamera's full-screen fade quad.
fn after_fade_canvas<'a>(p: &'a mut HudPainter<'_>) -> HudChild<'a> {
    p.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0),
        height: Val::Percent(100.0), ..default() }, GlobalZIndex(20_100)))
}

fn panel(p: &mut HudPainter, x: f32, y: f32, w: f32, h: f32, color: Color) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: pct(x, W),
            top: pct(y, H),
            width: pct(w, W),
            height: pct(h, H),
            ..default()
        },
        BackgroundColor(color),
    ));
}

#[cfg(test)]
mod responsive_hud_tests {
    use super::*;

    #[test]
    fn radar_text_and_panels_share_one_scale_for_arbitrary_viewports() {
        for viewport in [
            Vec2::new(1920.0, 1080.0),
            Vec2::new(400.0, 1200.0),
            Vec2::new(320.0, 200.0),
            Vec2::new(3840.0, 540.0),
            Vec2::new(640.0, 448.0),
        ] {
            let canvas = hud_canvas_layout(viewport);
            assert!((canvas.size.x / W - canvas.size.y / H).abs() < 0.00001);
            assert!(canvas.offset.min_element() >= 0.0);
            assert!(
                (canvas.offset * 2.0 + canvas.size - viewport)
                    .abs()
                    .max_element()
                    < 0.001
            );
            assert!(canvas.size.x <= viewport.x && canvas.size.y <= viewport.y);
        }
    }
}
