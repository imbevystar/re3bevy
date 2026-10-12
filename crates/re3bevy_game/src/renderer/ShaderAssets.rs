//! Program-owned RenderWare rendering adapters. GTA textures and tables remain
//! external installation assets; only our WESL source ships inside the binary.

use bevy::{prelude::*, shader::Shader};

pub const RW_UNLIT: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a002-30b2-443f-8d31-6355a16272ee");
pub const RW_UNLIT_UTIL: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a003-30b2-443f-8d31-6355a16272ee");
pub const RW_SKY: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a004-30b2-443f-8d31-6355a16272ee");
pub const RW_WATER: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a005-30b2-443f-8d31-6355a16272ee");
pub const PARTICLE_SPRITE: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a006-30b2-443f-8d31-6355a16272ee");
pub const RW_POSTFX: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a007-30b2-443f-8d31-6355a16272ee");
pub const RW_SCREEN_DROPLET: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a008-30b2-443f-8d31-6355a16272ee");
pub const RW_NEO: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a009-30b2-443f-8d31-6355a16272ee");
pub const RW_PC_ENVMAP: Handle<Shader> =
    bevy::asset::uuid_handle!("ac07a00a-30b2-443f-8d31-6355a16272ee");

pub struct GtaShaderAssetsPlugin;

impl Plugin for GtaShaderAssetsPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        macro_rules! register {
            ($handle:expr, $file:literal) => {
                shaders
                    .insert(
                        $handle.id(),
                        Shader::from_wesl(
                            include_str!(concat!("../../../../assets/shaders/gta/", $file)),
                            concat!("shaders/gta/", $file),
                        ),
                    )
                    .expect("program shader UUID must be valid");
            };
        }
        // Virtual paths preserve super:: imports in Bevy's WESL resolver.
        register!(RW_UNLIT_UTIL, "rw_unlit_util.wesl");
        register!(RW_UNLIT, "rw_unlit.wesl");
        register!(RW_SKY, "rw_sky.wesl");
        register!(RW_WATER, "rw_water.wesl");
        register!(PARTICLE_SPRITE, "particle_sprite.wesl");
        register!(super::HudBevy::radar::RADAR_SHADER, "hud_radar.wesl");
        register!(RW_POSTFX, "rw_postfx.wesl");
        register!(RW_SCREEN_DROPLET, "rw_screen_droplet.wesl");
        register!(RW_NEO, "rw_neo.wesl");
        register!(RW_PC_ENVMAP, "rw_pc_envmap.wesl");
        info!(target: "gta_assets", "registered 10 program-owned WESL shaders; no external shader directory required");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_shaders_register_without_a_file_asset_server() {
        let mut app = App::new();
        app.insert_resource(Assets::<Shader>::default())
            .add_plugins(GtaShaderAssetsPlugin);
        let shaders = app.world().resource::<Assets<Shader>>();
        for handle in [RW_UNLIT_UTIL, RW_UNLIT, RW_SKY, RW_WATER, PARTICLE_SPRITE,
            super::super::HudBevy::radar::RADAR_SHADER, RW_POSTFX, RW_SCREEN_DROPLET, RW_NEO, RW_PC_ENVMAP] {
            let shader = shaders.get(&handle).expect("embedded program shader");
            assert!(matches!(shader.source, bevy::shader::Source::Wesl(_)));
            assert!(shader.path.starts_with("shaders/gta/"));
        }
    }
}
