//! Validate production WESL through Bevy's own shader cache and GPU interfaces.
use bevy::{
    asset::AssetPlugin,
    pbr::MeshRenderPlugin,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    shader::{ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader},
    ui_render::UiMaterialPlugin,
};
use std::path::Path;

struct ValidatedShader {
    binding_arrays: usize,
}

fn validate_module(
    _: &(),
    source: ShaderCacheSource,
    _: &ValidateShader,
) -> Result<ValidatedShader, ShaderCacheError> {
    let ShaderCacheSource::Wgsl(source) = source else {
        panic!("WESL must compile to WGSL")
    };
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| panic!("{error}"));
    Ok(ValidatedShader {
        binding_arrays: module
            .types
            .iter()
            .filter(|(_, ty)| matches!(ty.inner, naga::TypeInner::BindingArray { .. }))
            .count(),
    })
}

#[test]
fn production_shader_interfaces_validate() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        RenderPlugin {
            render_creation: WgpuSettings {
                backends: None,
                ..default()
            }
            .into(),
            ..default()
        },
        MeshRenderPlugin::new(default()),
        UiMaterialPlugin::<re3bevy_game::renderer::HudBevy::radar::RadarMaterial>::default(),
    ));
    let server = app.world().resource::<AssetServer>();
    let embedded = server.get_source("embedded").unwrap();
    let mut cache = ShaderCache::new((), validate_module);
    let mut shaders = Assets::<Shader>::default();
    // Use engine-owned definitions and the actual Bevy resolver, including
    // embedded package paths and relative imports in program-owned assets.
    for asset in [
        "bevy_render/view.wesl",
        "bevy_pbr/render/mesh_types.wesl",
        "bevy_pbr/render/forward_io.wesl",
        "bevy_ui_render/ui_vertex_output.wesl",
    ] {
        let source = bevy::tasks::block_on(async {
            let mut reader = embedded.reader().read(Path::new(asset)).await.unwrap();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            String::from_utf8(bytes).unwrap()
        });
        let shader = Shader::from_wesl(source, format!("embedded://{asset}"));
        let handle = shaders.add(shader.clone());
        cache.set_shader(handle.id(), shader);
    }
    let mut roots = Vec::new();
    for (file, source) in [
        (
            "rw_unlit_util.wesl",
            include_str!("../../../assets/shaders/gta/rw_unlit_util.wesl"),
        ),
        (
            "rw_unlit.wesl",
            include_str!("../../../assets/shaders/gta/rw_unlit.wesl"),
        ),
        (
            "rw_water.wesl",
            include_str!("../../../assets/shaders/gta/rw_water.wesl"),
        ),
        (
            "rw_sky.wesl",
            include_str!("../../../assets/shaders/gta/rw_sky.wesl"),
        ),
        (
            "particle_sprite.wesl",
            include_str!("../../../assets/shaders/gta/particle_sprite.wesl"),
        ),
        (
            "hud_radar.wesl",
            include_str!("../../../assets/shaders/gta/hud_radar.wesl"),
        ),
    ] {
        let shader = Shader::from_wesl(source, format!("shaders/gta/{file}"));
        let handle = shaders.add(shader.clone());
        cache.set_shader(handle.id(), shader);
        if file != "rw_unlit_util.wesl" {
            roots.push((file, handle));
        }
    }
    for (file, handle) in roots {
        for bindless in [false, true] {
            for vertex_colors in [false, true] {
                let defs = [
                    ShaderDefVal::Bool("BINDLESS".into(), bindless),
                    ShaderDefVal::Bool("VERTEX_COLORS".into(), vertex_colors),
                    "VERTEX_UVS_A".into(),
                    "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
                ];
                let shader = cache.get(0, handle.id(), &defs).unwrap_or_else(|error| {
                    panic!("{file}, BINDLESS={bindless}, VERTEX_COLORS={vertex_colors}: {error}")
                });
                if file == "rw_unlit.wesl" {
                    assert_eq!(shader.binding_arrays, if bindless { 3 } else { 0 });
                }
            }
        }
    }
}
