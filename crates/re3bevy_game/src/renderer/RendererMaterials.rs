//! re3 reference: references/re-GTA/src/rw/VisibilityPlugins.cpp
//! re3 reference: references/re-GTA/src/extras/custompipes.cpp
//!
//! Bevy/WESL material resource handles for RenderWare-style render intents.

use bevy::{
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    render::storage::ShaderBuffer,
    shader::{Shader, ShaderRef},
};

use super::ShaderAssets;
use super::Timecycle::TimeCycleCurrent;

pub const RE3_ALPHA_TEST_REF: f32 = 2.0 / 255.0;
pub const GTA_GLOBAL_LIGHTING_BUFFER: Handle<ShaderBuffer> =
    bevy::asset::uuid_handle!("877728e2-020c-4df8-aea5-a68317c26c69");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RwTextureAlphaMode {
    #[default]
    Opaque,
    Cutout,
    Blend,
}

pub fn classify_rw_texture_alpha(declared_has_alpha: bool, rgba: &[u8]) -> RwTextureAlphaMode {
    if !declared_has_alpha {
        return RwTextureAlphaMode::Opaque;
    }
    let mut transparent = false;
    let chunks = rgba.chunks_exact(16);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let a0 = chunk[3];
        let a1 = chunk[7];
        let a2 = chunk[11];
        let a3 = chunk[15];

        // If all 4 alphas are 255 (the vast majority of pixels in non-cutout areas), skip immediately
        if a0 == 255 && a1 == 255 && a2 == 255 && a3 == 255 {
            continue;
        }

        for &a in &[a0, a1, a2, a3] {
            match a {
                255 => {}
                0 => transparent = true,
                _ => return RwTextureAlphaMode::Blend,
            }
        }
    }

    for chunk in remainder.chunks_exact(4) {
        match chunk[3] {
            255 => {}
            0 => transparent = true,
            _ => return RwTextureAlphaMode::Blend,
        }
    }

    if transparent {
        RwTextureAlphaMode::Cutout
    } else {
        RwTextureAlphaMode::Opaque
    }
}

/// One shared RenderWare frame-light block. re3 installs ambient/directional
/// lights once per frame; a storage buffer avoids rewriting every resident
/// material when timecycle or camera fog changes.
#[repr(C)]
#[derive(Debug, Clone, Copy, ShaderType, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GtaGlobalLighting {
    pub fog_colour_and_start: Vec4,
    pub ambient: Vec4,
    pub directional: Vec4,
    pub light_direction_and_far_clip: Vec4,
    pub postfx_blur_colour: Vec4,
}

impl Default for GtaGlobalLighting {
    fn default() -> Self {
        Self {
            fog_colour_and_start: Vec4::new(0.5, 0.5, 0.55, 2_500.0),
            ambient: Vec4::ONE,
            directional: Vec4::ZERO,
            light_direction_and_far_clip: Vec4::new(0.0, 0.0, 1.0, 2_500.0),
            postfx_blur_colour: Vec4::new(1.0, 1.0, 1.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct GtaRenderShaderHandles {
    pub rw_unlit: Handle<Shader>,
    pub rw_unlit_util: Handle<Shader>,
    pub rw_sky: Handle<Shader>,
    pub rw_water: Handle<Shader>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwMaterialParams {
    pub colour: [f32; 4],
    pub fog_colour_and_factor: [f32; 4],
    pub postfx_blur_colour: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyMaterialParams {
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    /// Top-left-origin normalized screen Y of the horizon. re3 computes this
    /// in `CSprite::CalcHorizonCoors` before drawing the background gradient.
    pub horizon: f32,
    /// Bottom edge of re3 CClouds::RenderHorizon, normalized top-left screen Y.
    pub horizon_end: f32,
    pub dark_background: bool,
    pub postfx_blur_colour: [f32; 4],
}

/// GPU uniform block for [`GtaUnlitMaterial`] (16-byte aligned).
#[derive(Debug, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct GtaUnlitUniforms {
    pub colour: Vec4,
    /// x=has_texture, y=alpha_cutoff, z=additive, w=unused.
    pub flags: Vec4,
    /// x=surface ambient, y=surface diffuse, zw=reserved.
    pub surface: Vec4,
    /// xyz retained for diagnostic compatibility; w=-1 unlit, 0 no prelight,
    /// 1 prelit, 2 batched non-prelit material colour in the vertex stream.
    pub light_direction_and_prelit: Vec4,
    /// x=ambient multiplier, y=directional enabled.
    pub lighting_options: Vec4,
}

/// re3-style unlit world/vehicle material: base colour × texture × vertex colour × fog.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, PartialEq)]
#[bind_group_data(GtaUnlitMaterialKey)]
#[data(0, GtaUnlitUniforms, binding_array(10))]
// Bound descriptor slabs avoid the cost of the default 2048-entry arrays
// while grouping resident GTA materials without copying frame lighting.
#[bindless(limit(128))]
pub struct GtaUnlitMaterial {
    pub uniforms: GtaUnlitUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    #[storage(3, read_only, binding_array(11))]
    pub global_lighting: Handle<ShaderBuffer>,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    /// Cover subpixel cracks in separately authored static-world surfaces.
    /// Enabled only when the backend exposes conservative rasterization.
    pub conservative_coverage: bool,
}

impl From<&GtaUnlitMaterial> for GtaUnlitUniforms {
    fn from(material: &GtaUnlitMaterial) -> Self {
        material.uniforms
    }
}

#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub struct GtaUnlitMaterialKey {
    pub alpha_mode_bits: u8,
    pub double_sided: bool,
    pub conservative_coverage: bool,
}

impl From<&GtaUnlitMaterial> for GtaUnlitMaterialKey {
    fn from(material: &GtaUnlitMaterial) -> Self {
        let alpha_mode_bits = match material.alpha_mode {
            AlphaMode::Opaque => 0,
            AlphaMode::Mask(_) => 1,
            AlphaMode::Blend => 2,
            AlphaMode::Premultiplied => 3,
            AlphaMode::Add => 4,
            AlphaMode::Multiply => 5,
            AlphaMode::AlphaToCoverage => 6,
        };
        Self {
            alpha_mode_bits,
            double_sided: material.double_sided,
            conservative_coverage: material.conservative_coverage && alpha_mode_bits == 0,
        }
    }
}

impl GtaUnlitMaterial {
    pub fn from_gta(
        base_color: Color,
        texture: Option<Handle<Image>>,
        texture_has_alpha: bool,
        additive: bool,
        _fog_colour: [f32; 3],
        _fog_factor: f32,
    ) -> Self {
        let alpha = if texture_has_alpha {
            RwTextureAlphaMode::Cutout
        } else {
            RwTextureAlphaMode::Opaque
        };
        let mut material = Self::from_gta_lit(
            base_color,
            texture,
            alpha,
            additive,
            [0.0; 3],
            0.0,
            0.0,
            [0.0; 3],
            [0.0; 3],
            [0.0, 0.0, 1.0],
            0.0,
            0.0,
            false,
            1.0,
            false,
        );
        material.uniforms.light_direction_and_prelit.w = -1.0;
        material
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_gta_lit(
        base_color: Color,
        texture: Option<Handle<Image>>,
        texture_alpha: RwTextureAlphaMode,
        additive: bool,
        _fog_colour: [f32; 3],
        _fog_start: f32,
        _far_clip: f32,
        _ambient: [f32; 3],
        _directional: [f32; 3],
        light_direction: [f32; 3],
        surface_ambient: f32,
        surface_diffuse: f32,
        has_prelit: bool,
        ambient_multiplier: f32,
        directional_enabled: bool,
    ) -> Self {
        // GTA/RW material bytes are encoded channel values. Keep them encoded
        // until rw_unlit performs the D3D8 modulation order.
        let encoded = base_color.to_srgba();
        let r = encoded.red.clamp(0.0, 1.0);
        let g = encoded.green.clamp(0.0, 1.0);
        let b = encoded.blue.clamp(0.0, 1.0);
        let a = encoded.alpha.clamp(0.0, 1.0);
        let needs_blend = a < 0.999;
        let alpha_mode = if additive {
            AlphaMode::Add
        } else if needs_blend || texture_alpha == RwTextureAlphaMode::Blend {
            AlphaMode::Blend
        } else if texture_alpha == RwTextureAlphaMode::Cutout {
            AlphaMode::Mask(RE3_ALPHA_TEST_REF)
        } else {
            AlphaMode::Opaque
        };
        let cutoff = if additive || needs_blend || texture_alpha != RwTextureAlphaMode::Opaque {
            RE3_ALPHA_TEST_REF
        } else {
            0.0
        };
        Self {
            uniforms: GtaUnlitUniforms {
                colour: Vec4::new(r, g, b, a),
                flags: Vec4::new(
                    if texture.is_some() { 1.0 } else { 0.0 },
                    cutoff,
                    if additive { 1.0 } else { 0.0 },
                    0.0,
                ),
                surface: Vec4::new(surface_ambient.max(0.0), surface_diffuse.max(0.0), 0.0, 0.0),
                light_direction_and_prelit: Vec4::new(
                    light_direction[0],
                    light_direction[1],
                    light_direction[2],
                    if has_prelit { 1.0 } else { 0.0 },
                ),
                lighting_options: Vec4::new(
                    ambient_multiplier.max(0.0),
                    if directional_enabled { 1.0 } else { 0.0 },
                    0.0,
                    0.0,
                ),
            },
            base_color_texture: texture,
            global_lighting: GTA_GLOBAL_LIGHTING_BUFFER.clone(),
            alpha_mode,
            double_sided: true,
            conservative_coverage: false,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GtaSkyMaterial {
    #[uniform(0)]
    pub top: Vec4,
    #[uniform(1)]
    pub bottom: Vec4,
    #[uniform(2)]
    pub horizon: Vec4,
    #[uniform(3)]
    pub postfx_blur_colour: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GtaWaterMaterial {
    #[uniform(0)]
    pub colour: Vec4,
    #[storage(1, read_only)]
    pub global_lighting: Handle<ShaderBuffer>,
    #[uniform(2)]
    pub postfx_blur_colour: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub water_texture: Option<Handle<Image>>,
    /// re3 `TEXTURE_ADDU/TEXTURE_ADDV`. Keep the large flat-water mesh
    /// resident and scroll the raster in the material instead of rewriting
    /// every vertex UV (which makes Bevy's render mesh allocator re-upload the
    /// whole asset).
    #[uniform(5)]
    pub uv_offset: Vec4,
}

impl RwMaterialParams {
    pub fn from_timecycle(current: &TimeCycleCurrent, colour: [f32; 4], distance: f32) -> Self {
        let fog_range = (current.far_clip - current.fog_start).max(f32::EPSILON);
        let fog_factor = ((distance - current.fog_start) / fog_range).clamp(0.0, 1.0);
        Self {
            colour,
            fog_colour_and_factor: [
                current.fog_colour[0] as f32 / 255.0,
                current.fog_colour[1] as f32 / 255.0,
                current.fog_colour[2] as f32 / 255.0,
                fog_factor,
            ],
            postfx_blur_colour: normalised_postfx_blur(current.blur),
        }
    }
}

impl SkyMaterialParams {
    pub fn from_timecycle(current: &TimeCycleCurrent) -> Self {
        Self {
            top: [
                current.sky_top[0] as f32 / 255.0,
                current.sky_top[1] as f32 / 255.0,
                current.sky_top[2] as f32 / 255.0,
                1.0,
            ],
            bottom: [
                current.sky_bottom[0] as f32 / 255.0,
                current.sky_bottom[1] as f32 / 255.0,
                current.sky_bottom[2] as f32 / 255.0,
                1.0,
            ],
            horizon: 0.5,
            horizon_end: 0.75,
            dark_background: false,
            postfx_blur_colour: normalised_postfx_blur(current.blur),
        }
    }
}

fn normalised_postfx_blur(blur: [f32; 4]) -> [f32; 4] {
    [
        (blur[0] / 255.0).clamp(0.0, 1.0),
        (blur[1] / 255.0).clamp(0.0, 1.0),
        (blur[2] / 255.0).clamp(0.0, 1.0),
        (blur[3] / 255.0).clamp(0.0, 1.0),
    ]
}

pub struct GtaRenderMaterialsPlugin;

impl Plugin for GtaRenderMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShaderAssets::GtaShaderAssetsPlugin);
        app.add_plugins((
            MaterialPlugin::<GtaSkyMaterial>::default(),
            MaterialPlugin::<GtaWaterMaterial>::default(),
            MaterialPlugin::<GtaUnlitMaterial>::default(),
        ));
        app.add_plugins(super::WorldSurfaceCoverage::WorldSurfaceCoveragePlugin);
        app.world_mut()
            .resource_mut::<Assets<ShaderBuffer>>()
            .insert(
                GTA_GLOBAL_LIGHTING_BUFFER.id(),
                ShaderBuffer::from(vec![GtaGlobalLighting::default()]),
            )
            .expect("constant GTA global lighting buffer id must be valid");
        app.insert_resource(GtaRenderShaderHandles {
            rw_unlit: ShaderAssets::RW_UNLIT,
            rw_unlit_util: ShaderAssets::RW_UNLIT_UTIL,
            rw_sky: ShaderAssets::RW_SKY,
            rw_water: ShaderAssets::RW_WATER,
        });
    }
}

impl From<SkyMaterialParams> for GtaSkyMaterial {
    fn from(value: SkyMaterialParams) -> Self {
        Self {
            top: Vec4::from_array(value.top),
            bottom: Vec4::from_array(value.bottom),
            horizon: Vec4::new(
                value.horizon,
                value.horizon_end,
                if value.dark_background { 1.0 } else { 0.0 },
                0.0,
            ),
            postfx_blur_colour: Vec4::from_array(value.postfx_blur_colour),
        }
    }
}

impl From<RwMaterialParams> for GtaWaterMaterial {
    fn from(value: RwMaterialParams) -> Self {
        Self {
            colour: Vec4::from_array(value.colour),
            global_lighting: GTA_GLOBAL_LIGHTING_BUFFER.clone(),
            postfx_blur_colour: Vec4::from_array(value.postfx_blur_colour),
            water_texture: None,
            uv_offset: Vec4::ZERO,
        }
    }
}

impl Material for GtaSkyMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderAssets::RW_SKY.into()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // `CClouds::RenderBackground` is a screen-space background draw. Keep
        // the adapter quad from contributing depth so the late pre-projected
        // cloud pass can still use the untouched clear-depth value to remain
        // behind every world entity.
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        descriptor.primitive.cull_mode = None;
        Ok(())
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

impl Material for GtaWaterMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderAssets::RW_WATER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // re3 CWaterLevel::RenderWater uses rwBLENDONE/rwBLENDZERO.
        AlphaMode::Opaque
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

impl Material for GtaUnlitMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderAssets::RW_UNLIT.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.double_sided {
            descriptor.primitive.cull_mode = None;
        } else {
            descriptor.primitive.cull_mode = Some(Face::Back);
        }
        descriptor.primitive.conservative = key.bind_group_data.conservative_coverage;
        if key.bind_group_data.conservative_coverage
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            // Reverse-Z: keep coverage behind the ordinary authored surface.
            depth.bias.constant = -16;
            depth.bias.slope_scale = -1.0;
        }
        // Ensure mesh pipeline knows we want UVs (and colours when present).
        // VERTEX_COLORS is driven by mesh layout; do not force it if mesh lacks COLOR.
        let _ = layout;
        Ok(())
    }
}
