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
    fn resident_material_shaders_validate_with_both_binding_layouts() {
        use bevy::shader::{ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal};
        // Load Bevy's actual embedded imports without opening a window or GPU.
        let mut app = App::new();
        let mut settings = bevy::render::settings::WgpuSettings::default();
        settings.backends = None;
        app.add_plugins(DefaultPlugins
            .set(WindowPlugin { primary_window: None, ..default() })
            .set(bevy::render::RenderPlugin { render_creation: settings.into(), ..default() })
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>())
            .add_plugins(GtaShaderAssetsPlugin);
        app.finish();
        app.cleanup();
        let mut cache = ShaderCache::new((), |_, source, _| {
            let ShaderCacheSource::Wgsl(source) = source else { panic!("WESL must compose to WGSL") };
            let module = naga::front::wgsl::parse_str(&source)
                .map_err(|error| ShaderCacheError::CreateShaderModule(error.emit_to_string(&source)))?;
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
                .validate(&module)
                .map_err(|error| ShaderCacheError::CreateShaderModule(format!("{error:?}")))?;
            Ok(module)
        });
        for handle in [RW_UNLIT, RW_PC_ENVMAP] {
            for bindless in [false, true] {
                let defs = [
                    ShaderDefVal::Bool("VERTEX_POSITIONS".into(), true),
                    ShaderDefVal::Bool("VERTEX_NORMALS".into(), true),
                    ShaderDefVal::Bool("VERTEX_UVS_A".into(), true),
                    ShaderDefVal::Bool("VERTEX_COLORS".into(), true),
                    ShaderDefVal::Bool("GTA_ENCODED_TARGET".into(), true),
                    ShaderDefVal::Bool("BINDLESS".into(), bindless),
                ];
                let started = std::time::Instant::now();
                let module = loop {
                    app.update();
                    for (id, shader) in app.world().resource::<Assets<Shader>>().iter() {
                        cache.set_shader(id, shader.clone());
                    }
                    match cache.get(0, handle.id(), &defs) {
                        Ok(module) => break module,
                        Err(ShaderCacheError::ShaderImportNotYetAvailable) if started.elapsed().as_secs() < 10 => {
                            std::thread::yield_now();
                        }
                        Err(error) => panic!("{handle:?} bindless={bindless}: {error}"),
                    }
                };
                let mut bindings = module.global_variables.iter()
                    .filter_map(|(_, variable)| variable.binding.as_ref())
                    .filter(|binding| binding.group == 3)
                    .map(|binding| binding.binding).collect::<Vec<_>>();
                bindings.sort_unstable();
                assert_eq!(bindings, if bindless { vec![0, 1, 5, 10, 11] }
                    else if handle == RW_PC_ENVMAP { vec![0, 1, 2, 3, 4, 5] }
                    else { vec![0, 1, 2, 3] });
            }
        }
    }

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
