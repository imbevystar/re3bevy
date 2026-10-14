//! CEntity::PreRender pickup branch and CPickups::Do*Effects.
//! TODO(parity): CPickups::RenderPickUpText world labels for shop stock/prices
//! and bonus vehicle names still require the native screen projection/font pass.
use super::*;
use crate::renderer::{
    Coronas::*,
    PointLights::{FOG_NONE, LIGHT_POINT},
    Shadows::StaticShadowIntent,
};
use crate::{
    control::{
        Darkel::CDarkel,
        PickupRuntime::{PickupModel, pickup_model},
        ScriptEntities::{ScriptEntityIndex, ScriptObjectIdentity},
    },
    objects::Object::CObject,
};

const REDS: [u8; 16] = [
    255, 0, 128, 255, 255, 0, 255, 0, 128, 128, 255, 255, 128, 0, 255, 0,
];
const GREENS: [u8; 16] = [
    0, 255, 128, 255, 0, 255, 128, 255, 0, 255, 255, 0, 255, 0, 255, 0,
];
const BLUES: [u8; 16] = [
    0, 0, 255, 0, 255, 255, 0, 128, 255, 0, 255, 0, 128, 255, 0, 0,
];
const SCALES: [f32; 15] = [
    1.0, 2.0, 1.5, 1.0, 1.0, 1.5, 1.0, 2.0, 1.0, 2.0, 2.5, 1.0, 1.0, 1.0, 1.0,
];

pub(super) fn prepare_pickup_effects(
    index: Res<ScriptEntityIndex>,
    mut objects: Query<(Entity, &ScriptObjectIdentity, &mut CObject)>,
    loader: Res<CFileLoader>,
    timer: Res<CTimer>,
    camera: Res<CCamera>,
    mut general: ResMut<CGeneral>,
    mut shadows: ResMut<CShadows>,
    mut coronas: ResMut<CCoronas>,
    mut lights: ResMut<CPointLights>,
    scripts: Res<crate::control::Script::CTheScripts>,
    darkel: Res<CDarkel>,
    game: Res<rustgta3_core::core::GameState::CGameState>,
) {
    let now = timer.time_in_milliseconds();
    for (entity, identity, mut object) in &mut objects {
        if index.objects.get(&identity.handle) != Some(&entity) {
            continue;
        }
        let Some(model) = loader
            .model_info
            .get_model_info(object.physical.entity.model_index.max(0) as usize)
        else {
            warn_once!(target: "rustgta3::pickups", "pickup effects waiting for original model metadata");
            continue;
        };
        let name = model.base().name.to_ascii_lowercase();
        let kind = pickup_model(&name);
        let collectable = name == "package1";
        let money = name == "money";
        let mine = object.flags.is_pickup
            && matches!(name.as_str(), "carmine" | "nauticalmine" | "briefcase");
        if !(collectable || money || mine || object.flags.is_pickup && kind != PickupModel::Other) {
            continue;
        }
        if kind == PickupModel::Frenzy {
            object.physical.entity.flags.do_not_render =
                scripts.is_player_on_a_mission() || darkel.frenzy_on_going() || !game.nasty_game;
        }
        if object.physical.entity.flags.do_not_render {
            continue;
        }
        let id = 0x7000_0000 | (identity.handle as u32 & 0x0FFF_FFFF);
        let position = object.physical.entity.get_position();
        let distance = (position - camera.position).magnitude();
        let seed = entity.to_bits() as u32;
        let (colour, size, draw_distance, rotation_period, scale) = if collectable || money || mine
        {
            let (range, mask, sine_scale, sine_bias, intensity, channels, size, draw) =
                if collectable {
                    (14.0, 0x7FF, 0.5, 0.5, 255.0, [1, 1, 1], 0.6, 40.0)
                } else if money {
                    (20.0, 0x3FF, 0.2, 0.3, 64.0, [0, 1, 0], 0.4, 40.0)
                } else {
                    (20.0, 0x1FF, 0.5, 0.5, 64.0, [1, 0, 0], 0.6, 60.0)
                };
            let sine = ((now.wrapping_add(seed) & mask) as f32 * std::f32::consts::TAU
                / (mask + 1) as f32)
                .sin();
            let intensity = if distance < range {
                ((range - distance) * (sine_scale * sine + sine_bias) / range * intensity) as u8
            } else {
                0
            };
            let colour = channels.map(|c| c * intensity);
            if distance < range {
                store_glow(&mut shadows, id, position, colour);
            }
            (
                colour,
                size,
                draw,
                if collectable {
                    4096
                } else if money {
                    2048
                } else {
                    1024
                },
                1.0,
            )
        } else {
            let colour_index = match kind {
                PickupModel::Adrenaline | PickupModel::Camera => 11,
                PickupModel::Armour | PickupModel::Bribe => 12,
                PickupModel::Info | PickupModel::Frenzy => 13,
                PickupModel::Health | PickupModel::Bonus => 14,
                PickupModel::Weapon(w) => w as usize - 1,
                _ => continue,
            };
            let colour = [
                REDS[colour_index],
                GREENS[colour_index],
                BLUES[colour_index],
            ];
            let modified_sin = 0.3
                * (((now.wrapping_add(seed) & 0x7FF) as f32 * std::f32::consts::TAU / 2048.0)
                    .sin()
                    + 1.0);
            let modifier =
                ((general.get_random_number() & 0x1F) as f32 * 0.015 + 1.0) * modified_sin * 0.15;
            if distance < 40.0 {
                store_glow(
                    &mut shadows,
                    id,
                    position,
                    colour.map(|c| (c as f32 * modifier) as u8),
                );
            }
            let radius = (general.get_random_number() & 0xF) as f32 * 0.1 + 3.0;
            lights.add_light(
                LIGHT_POINT,
                position,
                CVector::ZERO,
                radius,
                colour[0] as f32 * modified_sin / 256.0,
                colour[1] as f32 * modified_sin / 256.0,
                colour[2] as f32 * modified_sin / 256.0,
                FOG_NONE,
                true,
                camera.position,
            );
            let size = (general.get_random_number() & 0xF) as f32 * 0.0005 + 0.6;
            (
                colour.map(|c| (c as f32 * modified_sin / 2.0) as u8),
                size,
                65.0,
                2048,
                SCALES[colour_index],
            )
        };
        // re3's collectable/money/mine routines register only inside their
        // 14/20/20m effect radius. Ordinary pickups are pre-rendered only
        // when resident in the visible world; RegisterCorona also rejects
        // positions outside its 2D draw distance. Iterating every live ECS
        // pickup without that gate filled all 128 corona slots with distant
        // invisible rings before the nearby street lamps were visited.
        let corona_in_range = if collectable {
            distance < 14.0
        } else if money || mine {
            distance < 20.0
        } else {
            let dx = position.x - camera.position.x;
            let dy = position.y - camera.position.y;
            dx * dx + dy * dy < draw_distance * draw_distance
        };
        if corona_in_range {
            coronas.register_corona(
                id,
                colour[0],
                colour[1],
                colour[2],
                255,
                position,
                size,
                draw_distance,
                TYPE_RING,
                FLARE_NONE,
                REFLECTION_OFF,
                LOSCHECK_OFF,
                STREAK_OFF,
                0.0,
            );
        }
        let angle =
            (now & (rotation_period - 1)) as f32 * std::f32::consts::TAU / rotation_period as f32;
        let (sine, cosine) = angle.sin_cos();
        object.physical.entity.matrix.right = CVector::new(cosine * scale, sine * scale, 0.0);
        object.physical.entity.matrix.forward = CVector::new(-sine * scale, cosine * scale, 0.0);
        object.physical.entity.matrix.up = CVector::new(0.0, 0.0, scale);
        object.object_matrix = object.physical.entity.matrix;
    }
}

fn store_glow(shadows: &mut CShadows, key: u32, position: CVector, color: [u8; 3]) {
    // gpShadowExplosionTex is resolved by CShadows::Init from particle TXD.
    if let Some(texture) = shadows
        .texture_names
        .iter()
        .position(|name| *name == "shad_exp")
    {
        shadows.pickup_shadows.push(StaticShadowIntent {
            key,
            texture: texture as u8,
            position,
            front: [2.0, 0.0],
            side: [0.0, -2.0],
            intensity: 0,
            color,
        });
    } else {
        warn_once!(target: "rustgta3::pickups", "pickup shadow waiting for shad_exp mapping");
    }
}
