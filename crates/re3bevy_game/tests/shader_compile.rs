//! Validate production WESL through Bevy's own shader cache and GPU interfaces.
use bevy::tasks::futures_lite::StreamExt;
use bevy::{
    asset::AssetPlugin,
    pbr::MeshRenderPlugin,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    shader::{ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal, ValidateShader},
    ui_render::UiMaterialPlugin,
};

struct ValidatedShader {
    binding_arrays: usize,
    material_uniform_span: Option<u32>,
    has_vertex_stage: bool,
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
        material_uniform_span: module.global_variables.iter().find_map(|(_, variable)| {
            if variable.space == naga::AddressSpace::Uniform
                && variable.binding.as_ref().is_some_and(|binding| binding.group == 3 && binding.binding == 0)
                && let naga::TypeInner::Struct { span, .. } = module.types[variable.ty].inner {
                Some(span)
            } else {
                None
            }
        }),
        has_vertex_stage: module.entry_points.iter().any(|entry| entry.stage == naga::ShaderStage::Vertex),
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
    // Vertex-stage adapters use the real Bevy instance and vertex interfaces.
    // Register engine modules through the same embedded reader
    // instead of keeping hand-copied shader interfaces in the test.
    let assets = bevy::tasks::block_on(async {
        let mut directories = vec![std::path::PathBuf::from("")];
        let mut assets = Vec::new();
        while let Some(directory) = directories.pop() {
            let mut entries = embedded.reader().read_directory(&directory).await.unwrap();
            while let Some(path) = entries.next().await {
                if embedded.reader().is_directory(&path).await.unwrap() {
                    directories.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "wesl")
                {
                    assets.push(path);
                }
            }
        }
        assets
    });
    for asset in assets {
        let source = bevy::tasks::block_on(async {
            let mut reader = embedded.reader().read(&asset).await.unwrap();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            String::from_utf8(bytes).unwrap()
        });
        let shader = Shader::from_wesl(
            source,
            format!("embedded://{}", asset.to_string_lossy().replace('\\', "/")),
        );
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
        (
            "rw_postfx.wesl",
            include_str!("../../../assets/shaders/gta/rw_postfx.wesl"),
        ),
        (
            "rw_screen_droplet.wesl",
            include_str!("../../../assets/shaders/gta/rw_screen_droplet.wesl"),
        ),
        (
            "rw_neo.wesl",
            include_str!("../../../assets/shaders/gta/rw_neo.wesl"),
        ),
        (
            "rw_pc_envmap.wesl",
            include_str!("../../../assets/shaders/gta/rw_pc_envmap.wesl"),
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
        if matches!(file, "rw_postfx.wesl" | "rw_screen_droplet.wesl") {
            for encoded in [false, true] {
                cache
                    .get(
                        0,
                        handle.id(),
                        &[ShaderDefVal::Bool("GTA_ENCODED_TARGET".into(), encoded)],
                    )
                    .unwrap_or_else(|error| panic!("{file}, encoded={encoded}: {error}"));
            }
        }
        if file == "particle_sprite.wesl" {
            for encoded in [false, true] {
                cache
                    .get(
                        0,
                        handle.id(),
                        &[ShaderDefVal::Bool(
                            "PARTICLE_ENCODED_TARGET".into(),
                            encoded,
                        )],
                    )
                    .unwrap_or_else(|error| panic!("{file}, encoded={encoded}: {error}"));
            }
        }
        for bindless in [false, true] {
            for vertex_colors in [false, true] {
                for vertex_uv_b in [false, true] {
                    for encoded in [false, true] {
                        let defs = [
                            ShaderDefVal::Bool("GTA_ENCODED_TARGET".into(), encoded),
                            ShaderDefVal::Bool("BINDLESS".into(), bindless),
                            ShaderDefVal::Bool("VERTEX_COLORS".into(), vertex_colors),
                            ShaderDefVal::Bool("VERTEX_UVS_B".into(), vertex_uv_b),
                            "VERTEX_UVS_A".into(),
                            "VERTEX_POSITIONS".into(),
                            "VERTEX_NORMALS".into(),
                            "VERTEX_OUTPUT_INSTANCE_INDEX".into(),
                        ];
                        let shader = cache.get(0, handle.id(), &defs).unwrap_or_else(|error| {
                    panic!("{file}, BINDLESS={bindless}, VERTEX_COLORS={vertex_colors}: {error}")
                });
                        if file == "rw_unlit.wesl" {
                            assert_eq!(shader.binding_arrays, if bindless { 3 } else { 0 });
                        }
                        if file == "rw_pc_envmap.wesl" {
                            assert_eq!(shader.material_uniform_span, Some(112));
                            assert!(shader.has_vertex_stage);
                        }
                    }
                }
            }
        }
    }
}
