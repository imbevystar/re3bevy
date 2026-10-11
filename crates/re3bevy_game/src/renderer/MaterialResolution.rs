//! Original RenderWare TXD dictionary residency, raster caching and material lookup.

use super::{ModelMaterialDrawable, VehicleColourRemap};
use crate::{
    core::{FileLoader::CFileLoader, Streaming::TXDSTORESIZE},
    renderer::RendererMaterials::{RwTextureAlphaMode, classify_rw_texture_alpha},
};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};
use re3bevy_formats::formats::TexRead::{RwTextureNativeDescriptor, decode_txd_texture};
use std::collections::BTreeMap;

pub(super) struct MaterialResolutionPlugin;

impl Plugin for MaterialResolutionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GtaWorldTextureCache>();
    }
}

/// Per-TXD-slot texture handle cache keyed by lowercase RW texture name.
///
/// re3 binds materials via `RwTextureRead(name)` against the model's current TXD
/// (`CStreaming::ConvertBufferToObject` → `CTxdStore::SetCurrentTxd`). There is
/// **no** cross-dictionary or "first texture" fallback — a missing name leaves
/// the material untextured (colour only), exactly as the current RW dictionary
/// lookup does.
#[derive(Debug, Clone, Default, Resource)]
pub struct GtaWorldTextureCache {
    /// `(txd_slot, lowercase_name) → Bevy image handle`
    pub images: BTreeMap<(usize, String), Handle<Image>>,
    /// Same key → TXD `has_alpha` (alpha-test / blend).
    pub has_alpha: BTreeMap<(usize, String), bool>,
    /// Decoded raster alpha content, preserving re3's low alpha-test
    /// threshold while allowing genuinely fractional rasters to blend.
    pub alpha_modes: BTreeMap<(usize, String), RwTextureAlphaMode>,
    /// re3 `CVehicleModelInfo::CreateCarColourTexture` — solid 2×2 paint rasters
    /// keyed by RGB (alpha always 255). Used for `@` paint materials.
    pub solid_colours: BTreeMap<[u8; 3], Handle<Image>>,
    /// re3 `CVehicleModelInfo::ms_colourTextureTable[256]` — colour-index → solid paint.
    pub colour_texture_table: BTreeMap<u8, Handle<Image>>,
    /// re3 `ms_pEnvironmentMaps[0]` (`reflection01` from particle TXD).
    pub env_map: Option<Handle<Image>>,
    /// re3 `gpWhiteTexture` (particle `white`, name starts with `@` at runtime).
    pub white_texture: Option<Handle<Image>>,
    /// CPlayerSkin's dedicated TXD dictionary, keyed by original skin name.
    pub player_skins: BTreeMap<String, Handle<Image>>,
    /// Resident RenderWare dictionary identity already decoded for each fixed
    /// CTxdStore slot. re3 can unload/reload a slot with the same texture
    /// count, so count-only cache validation leaves stale Bevy images bound.
    pub synced_txd_generations: BTreeMap<usize, u64>,
    /// Renderer-visible content revision of that same resident dictionary.
    /// `StartLoadTxd` and `FinishLoadTxd` retain one RW dictionary identity,
    /// while the finish phase adds the remaining rasters.
    pub synced_txd_revisions: BTreeMap<usize, u64>,
    /// Global revision of the CTxdStore when this cache was last fully synchronized.
    pub synced_store_revision: Option<u64>,
}

/// Synchronize decoded RenderWare TXD rasters for tools that spawn model
/// geometry outside the normal GTA render pass (for example the developer
/// arena).  Keeping this on the production cache guarantees identical
/// dictionary lookup and channel decoding in both paths.
pub fn sync_world_texture_cache(
    file_loader: &CFileLoader,
    cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
) {
    crate::profile_scope!("sync_world_texture_cache");
    let store_rev = file_loader.txd_store.store_revision();
    let particle_slot = file_loader.txd_store.find_txd_slot("particle");
    if cache.synced_store_revision == Some(store_rev)
        && (cache.env_map.is_some() || particle_slot.is_none())
        && (cache.white_texture.is_some() || particle_slot.is_none())
    {
        return;
    }

    for slot in 0..TXDSTORESIZE {
        let _ = sync_txd_slot_texture_cache(slot, file_loader, cache, images);
    }
    // re3 CVehicleModelInfo::LoadEnvironmentMaps — particle TXD resident maps.
    ensure_environment_maps(file_loader, cache);
    cache.synced_store_revision = Some(store_rev);
}

pub(super) fn invalidate_txd_slot_texture_cache(slot: usize, cache: &mut GtaWorldTextureCache) {
    let stale_keys = cache
        .images
        .range((slot, String::new())..)
        .take_while(|((cached_slot, _), _)| *cached_slot == slot)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    for key in stale_keys {
        if let Some(handle) = cache.images.remove(&key) {
            if cache.env_map.as_ref() == Some(&handle) {
                cache.env_map = None;
            }
            if cache.white_texture.as_ref() == Some(&handle) {
                cache.white_texture = None;
            }
        }
        cache.has_alpha.remove(&key);
        cache.alpha_modes.remove(&key);
    }
    cache.synced_txd_generations.remove(&slot);
    cache.synced_txd_revisions.remove(&slot);
    cache.synced_store_revision = None;
}

pub(crate) fn sync_txd_slot_texture_cache(
    slot: usize,
    file_loader: &CFileLoader,
    cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
) -> bool {
    let txd = file_loader.txd_store.slot(slot);
    let Some(txd) = txd else {
        if cache.synced_txd_generations.contains_key(&slot) {
            invalidate_txd_slot_texture_cache(slot, cache);
        }
        return false;
    };

    let generation = txd.generation;
    let revision = txd.texture_revision;
    let cached_generation = cache.synced_txd_generations.get(&slot).copied();
    let cached_revision = cache.synced_txd_revisions.get(&slot).copied();
    if cached_generation != Some(generation) {
        // C++ destroys the old RwTexDictionary and all name lookups belong to
        // the replacement dictionary. Keep old Bevy Image assets alive for
        // any deferred/hidden material users, but make them unreachable from
        // the current TXD cache so newly built materials cannot bind stale
        // rasters from a previous streaming incarnation.
        invalidate_txd_slot_texture_cache(slot, cache);
    } else if cached_revision == Some(revision) {
        return txd.texture_summary.is_some();
    }
    // `FinishLoadTxd` appends the second half of a large TXD to the same
    // RwTexDictionary*. The dictionary generation stays stable while only the
    // content revision advances. Preserve first-phase handles and let the loop
    // below decode only texture names newly present in the completed summary;
    // invalidating here would needlessly decode and upload the first half twice.
    let Some(summary) = txd.texture_summary.as_ref() else {
        cache.synced_txd_generations.insert(slot, txd.generation);
        cache
            .synced_txd_revisions
            .insert(slot, txd.texture_revision);
        return false;
    };
    for texture in &summary.textures {
        let key = (slot, texture.name.to_ascii_lowercase());
        if cache.images.contains_key(&key) {
            continue;
        }
        let Some(decoded) = decode_txd_texture(texture) else {
            continue;
        };
        let alpha_mode = classify_rw_texture_alpha(texture.has_alpha, &decoded.rgba);
        let mut image = Image::new(
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
        image.sampler = ImageSampler::Descriptor(rw_image_sampler(texture));
        let handle = images.add(image);
        cache.has_alpha.insert(key.clone(), texture.has_alpha);
        cache.alpha_modes.insert(key.clone(), alpha_mode);
        cache.images.insert(key, handle);
    }
    cache.synced_txd_generations.insert(slot, txd.generation);
    cache
        .synced_txd_revisions
        .insert(slot, txd.texture_revision);
    true
}

/// Synchronize only the dictionaries required by one resident model. This is
/// the same current-TXD lookup used by `CStreaming::ConvertBufferToObject`,
/// without decoding every loaded dictionary when the arena spawns one car.
pub fn sync_model_texture_cache(
    model_index: usize,
    file_loader: &CFileLoader,
    cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
) -> bool {
    let Some(model) = file_loader.model_info.get_model_info(model_index) else {
        return false;
    };
    let Some(model_slot) = usize::try_from(model.base().txd_slot).ok() else {
        return false;
    };
    let mut synced = sync_txd_slot_texture_cache(model_slot, file_loader, cache, images);

    if let crate::modelinfo::ModelInfo::ModelInfoSlot::Vehicle(vehicle) = model
        && let Ok(wheel_model_index) = usize::try_from(vehicle.wheel_id)
        && let Some(wheel_model) = file_loader.model_info.get_model_info(wheel_model_index)
        && let Ok(wheel_slot) = usize::try_from(wheel_model.base().txd_slot)
    {
        synced |= sync_txd_slot_texture_cache(wheel_slot, file_loader, cache, images);
    }
    if let Some(particle_slot) = file_loader.txd_store.find_txd_slot("particle") {
        let _ = sync_txd_slot_texture_cache(particle_slot, file_loader, cache, images);
    }
    ensure_environment_maps(file_loader, cache);
    synced
}

/// re3 `CVehicleModelInfo::LoadEnvironmentMaps`: `reflection01` + `white` from particle.
pub(super) fn ensure_environment_maps(file_loader: &CFileLoader, cache: &mut GtaWorldTextureCache) {
    let Some(particle_slot) = file_loader.txd_store.find_txd_slot("particle") else {
        return;
    };
    if cache.env_map.is_none() {
        if let Some((handle, _)) = texture_cache_lookup(cache, particle_slot, "reflection01") {
            cache.env_map = Some(handle);
        }
    }
    if cache.white_texture.is_none() {
        if let Some((handle, _)) = texture_cache_lookup(cache, particle_slot, "white") {
            cache.white_texture = Some(handle);
        }
    }
}

/// re3 `ms_colourTextureTable[i] = CreateCarColourTexture(r,g,b)` for carcols table.
pub(super) fn ensure_colour_texture_table(
    cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    table: &crate::modelinfo::VehicleModelInfo::CVehicleColours,
) {
    for (index, colour) in table.vehicle_colour_table.iter().enumerate() {
        if index > 255 {
            break;
        }
        let idx = index as u8;
        if cache.colour_texture_table.contains_key(&idx) {
            continue;
        }
        let rgb = [colour.red, colour.green, colour.blue];
        let handle = ensure_solid_colour_texture(cache, images, rgb);
        cache.colour_texture_table.insert(idx, handle);
    }
}

/// Decode RW `filter_addressing` packed field into a Bevy sampler.
///
/// Layout (librw / GTA TXD native):
/// - bits 0–7: filter mode (`rwFILTER*`)
/// - bits 8–11: addressing U (`rwTEXTUREADDRESS*`)
/// - bits 12–15: addressing V
///
/// Defaults match world materials: linear filter + wrap (tiling roads/buildings).
pub(super) fn rw_image_sampler(texture: &RwTextureNativeDescriptor) -> ImageSamplerDescriptor {
    let packed = texture.filter_addressing;
    let filter = (packed & 0xFF) as u8;
    let addr_u = ((packed >> 8) & 0xF) as u8;
    let addr_v = ((packed >> 12) & 0xF) as u8;
    let (mag, min) = match filter {
        // NEAREST / MIPNEAREST
        1 | 3 => (ImageFilterMode::Nearest, ImageFilterMode::Nearest),
        // LINEAR / MIPLINEAR / LINEARMIPNEAREST / LINEARMIPLINEAR / default
        _ => (ImageFilterMode::Linear, ImageFilterMode::Linear),
    };
    ImageSamplerDescriptor {
        address_mode_u: rw_address_mode(addr_u),
        address_mode_v: rw_address_mode(addr_v),
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: mag,
        min_filter: min,
        mipmap_filter: ImageFilterMode::Linear,
        ..ImageSamplerDescriptor::default()
    }
}

/// RW addressing enum → Bevy. Unknown / 0 → Wrap (GTA world default).
pub(super) fn rw_address_mode(mode: u8) -> ImageAddressMode {
    match mode {
        // rwTEXTUREADDRESSWRAP = 1
        0 | 1 => ImageAddressMode::Repeat,
        // rwTEXTUREADDRESSMIRROR = 2
        2 => ImageAddressMode::MirrorRepeat,
        // rwTEXTUREADDRESSCLAMP = 3
        3 => ImageAddressMode::ClampToEdge,
        // rwTEXTUREADDRESSBORDER = 4
        4 => ImageAddressMode::ClampToBorder,
        _ => ImageAddressMode::Repeat,
    }
}

/// re3-style texture find for a material on a model.
///
/// Match `CStreaming::ConvertBufferToObject`: RenderWare resolves material
/// names only against `CTxdStore::SetCurrentTxd(mi->GetTxdSlot())`.
pub(super) fn texture_handle_for_model_material(
    model_index: usize,
    texture_name: Option<&str>,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
) -> Option<Handle<Image>> {
    texture_bind_for_model_material(model_index, texture_name, file_loader, texture_cache)
        .map(|(handle, _)| handle)
}

/// Resolve one DFF material's TXD texture and native alpha flag for an
/// external diagnostic scene. Vehicle `@` paint names intentionally return
/// `None`; callers should keep the material colour for those generated
/// rasters.
pub fn arena_texture_bind_for_drawable(
    drawable: &ModelMaterialDrawable,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
) -> Option<(Handle<Image>, RwTextureAlphaMode)> {
    texture_bind_for_drawable(drawable, file_loader, texture_cache)
}

pub(super) fn texture_bind_for_drawable(
    drawable: &ModelMaterialDrawable,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
) -> Option<(Handle<Image>, RwTextureAlphaMode)> {
    let name = drawable.texture_name.as_deref()?.trim();
    if name.is_empty() || name.starts_with('@') {
        return None;
    }
    if let Some(slot) = drawable.texture_txd_slot {
        return texture_cache_lookup(texture_cache, slot, &name.to_ascii_lowercase());
    }
    texture_bind_for_model_material(
        drawable.texture_model_index,
        Some(name),
        file_loader,
        texture_cache,
    )
}

/// Same as [`texture_handle_for_model_material`] but also returns TXD alpha flag.
pub(super) fn texture_bind_for_model_material(
    model_index: usize,
    texture_name: Option<&str>,
    file_loader: &CFileLoader,
    texture_cache: &GtaWorldTextureCache,
) -> Option<(Handle<Image>, RwTextureAlphaMode)> {
    let name = texture_name?.trim();
    if name.is_empty() {
        return None;
    }
    // re3 `CVehicleModelInfo`: textures renamed to start with '@' are solid paint
    // remaps (gpWhiteTexture / generated colour rasters), not TXD name lookups.
    // Material base_color already carries the vehicle colour — leave untextured.
    if name.starts_with('@') {
        return None;
    }
    let name_key = name.to_ascii_lowercase();
    let model = file_loader.model_info.get_model_info(model_index)?;
    let txd_slot = model.base().txd_slot;
    if txd_slot >= 0 {
        let slot = txd_slot as usize;
        if let Some(bind) = texture_cache_lookup(texture_cache, slot, &name_key) {
            return Some(bind);
        }
    }
    None
}

pub(super) fn texture_cache_lookup(
    texture_cache: &GtaWorldTextureCache,
    slot: usize,
    name_key: &str,
) -> Option<(Handle<Image>, RwTextureAlphaMode)> {
    let key = (slot, name_key.to_owned());
    let handle = texture_cache.images.get(&key)?.clone();
    let alpha_mode = texture_cache
        .alpha_modes
        .get(&key)
        .copied()
        .unwrap_or_else(|| {
            if texture_cache.has_alpha.get(&key).copied().unwrap_or(false) {
                RwTextureAlphaMode::Cutout
            } else {
                RwTextureAlphaMode::Opaque
            }
        });
    Some((handle, alpha_mode))
}

/// GTA's named TXD lookup expressed through Bevy assets.
///
/// This is the renderer-facing asset layer for code paths like
/// `CTxdStore::SetCurrentTxd(...)` + `RwTextureRead(name, nil)`: resolve the
/// dictionary by GTA name, synchronize its native rasters into Bevy `Image`
/// assets, then perform the texture-name lookup inside that dictionary only.
pub(super) fn resolve_txd_texture_asset(
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    txd_name: &str,
    texture_name: &str,
) -> Option<(Handle<Image>, RwTextureAlphaMode)> {
    let slot = file_loader.txd_store.find_txd_slot(txd_name)?;
    if !sync_txd_slot_texture_cache(slot, file_loader, texture_cache, images) {
        return None;
    }
    texture_cache_lookup(texture_cache, slot, &texture_name.to_ascii_lowercase())
}

/// re3 `CVehicleModelInfo::CreateCarColourTexture` — 2×2 solid paint raster.
pub(super) fn ensure_solid_colour_texture(
    cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    rgb: [u8; 3],
) -> Handle<Image> {
    if let Some(handle) = cache.solid_colours.get(&rgb) {
        return handle.clone();
    }
    let [r, g, b] = rgb;
    // 2×2 RGBA like re3 CreateCarColourTexture (each texel = solid colour).
    let mut rgba = Vec::with_capacity(2 * 2 * 4);
    for _ in 0..4 {
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    let mut image = Image::new(
        Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..ImageSamplerDescriptor::default()
    });
    let handle = images.add(image);
    cache.solid_colours.insert(rgb, handle.clone());
    handle
}

/// Resolve base colour + texture for a drawable, including re3 vehicle paint.
///
/// re3 `CVehicleModelInfo::SetVehicleColour`:
/// - material has non-`@` texture → keep texture, set material colour to table RGB
/// - else (nil texture or `@` colour tex) → set texture to `ms_colourTextureTable[ci]`
///   and material colour white
///
/// re3 `SetEnvironmentMapCB`: specular > 0 and no texture → bind `gpWhiteTexture`.
pub(super) fn resolve_drawable_material_bind(
    drawable: &ModelMaterialDrawable,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    vehicle_colours: VehicleColourRemap,
) -> (Option<Handle<Image>>, RwTextureAlphaMode, Color) {
    let base_color = drawable.base_color;
    let tex_name = drawable.texture_name.as_deref();
    let is_at_paint = tex_name.is_some_and(|n| n.starts_with('@'));

    // Vehicle primary/secondary colour slots (re3 editable materials).
    if let Some(slot) = drawable.vehicle_colour_slot {
        let colour_index = match slot {
            1 => vehicle_colours.colour1_index,
            2 => vehicle_colours.colour2_index,
            _ => None,
        };
        let has_named_non_at = tex_name.is_some_and(|n| !n.is_empty() && !n.starts_with('@'));
        if has_named_non_at {
            if let Some((handle, alpha_mode)) =
                texture_bind_for_drawable(drawable, file_loader, texture_cache)
            {
                return (Some(handle), alpha_mode, base_color);
            }
            // Missing TXD name: fall through to solid colour table texture.
        }
        // `@` / nil / missing → ms_colourTextureTable solid raster + white material.
        if let Some(idx) = colour_index {
            if let Some(handle) = texture_cache.colour_texture_table.get(&idx).cloned() {
                return (Some(handle), RwTextureAlphaMode::Opaque, Color::WHITE);
            }
        }
        let srgba = base_color.to_srgba();
        let rgb = [
            (srgba.red * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgba.green * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgba.blue * 255.0).round().clamp(0.0, 255.0) as u8,
        ];
        let handle = ensure_solid_colour_texture(texture_cache, images, rgb);
        return (Some(handle), RwTextureAlphaMode::Opaque, Color::WHITE);
    }

    if is_at_paint {
        let srgba = base_color.to_srgba();
        let rgb = [
            (srgba.red * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgba.green * 255.0).round().clamp(0.0, 255.0) as u8,
            (srgba.blue * 255.0).round().clamp(0.0, 255.0) as u8,
        ];
        let handle = ensure_solid_colour_texture(texture_cache, images, rgb);
        return (Some(handle), RwTextureAlphaMode::Opaque, Color::WHITE);
    }

    if let Some((handle, alpha_mode)) =
        texture_bind_for_drawable(drawable, file_loader, texture_cache)
    {
        return (Some(handle), alpha_mode, base_color);
    }

    // re3 SetEnvironmentMapCB: specular materials without diffuse get white.
    if drawable.specular > 0.0 {
        if let Some(white) = texture_cache.white_texture.clone() {
            return (Some(white), RwTextureAlphaMode::Opaque, base_color);
        }
    }
    (None, RwTextureAlphaMode::Opaque, base_color)
}
