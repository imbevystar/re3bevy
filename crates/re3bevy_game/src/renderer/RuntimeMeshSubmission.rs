//! Persistent actor mesh state, original 30 Hz refresh cadence, and retirement.

use super::{
    GtaRuntimePoolMesh, GtaRuntimePoolMeshSource, GtaUnlitMaterial, VehicleResidentMeshState, VehicleColourRemap, ModelMaterialDrawable, GtaWorldTextureCache,
    build_model_material_drawables, build_vehicle_component_transforms, sync_model_texture_cache,
    sync_txd_slot_texture_cache,
    compatible_model_index, log_render_audit,
    request_radar_sections_after_world_render,
};
#[cfg(test)]
use super::{
    VehicleDamageRenderState, VehicleDoorRenderState, VehicleWheelRenderState,
};
use crate::{
    animation::CutsceneMgr::CCutsceneMgr,
    core::{FileLoader::CFileLoader, Pools::CPools},
};
use crate::{
    control::ScriptEntities::{ScriptEntityIndex, ScriptObjectIdentity, ScriptPedIdentity},
    objects::Object::CObject,
    peds::Ped::CPed,
};
use bevy::prelude::*;
use re3bevy_core::core::{GameState::GtaAppState, Schedules::GtaProcessSet, Timer::CTimer};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct RuntimeMeshSubmissionPlugin;

/// Required renderer inputs: monotonically counted without fabricating assets.
#[derive(Debug, Default, Resource)]
pub struct GtaRuntimeSubmissionDiagnostics {
    pub missing_file_loader_frames: u64,
    pub missing_timer_frames: u64,
}


impl Plugin for RuntimeMeshSubmissionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::super::PersistentMeshUploads::PersistentMeshUploadsPlugin);
        app.init_resource::<GtaRuntimeMeshRefresh>()
            .init_resource::<GtaAnimatedRuntimeMeshRegistry>()
            .init_resource::<GtaRuntimeSubmissionDiagnostics>()
            .add_systems(
                Update,
                (
                    prepare_runtime_pool_mesh_refresh,
                    flush_runtime_pool_meshes,
                    cleanup_stale_static_runtime_pool_meshes,
                    cleanup_animated_runtime_pool_meshes,
                )
                    .chain()
                    .after(request_radar_sections_after_world_render)
                    .after(crate::vehicles::VehicleConstruction::VehicleConstructionSet::Suspension)
                    .before(log_render_audit)
                    .in_set(GtaProcessSet::RenderPrep)
                    .run_if(in_state(GtaAppState::InGame)),
            );
    }
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub(super) struct GtaRuntimeMeshRefresh {
    pub(super) rebuild_this_frame: bool,
    pub(super) static_rebuild: bool,
    /// Streaming can finish a cutscene DFF/TXD between the normal 30 Hz
    /// renderer cadence.  The next render-prep pass must rebuild immediately
    /// so the object is not invisible until an unrelated cadence tick.
    pub(super) force_rebuild: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct GtaAnimatedRuntimeMeshKey {
    pub(super) source: GtaRuntimePoolMeshSource,
    pub(super) slot: usize,
    pub(super) model_index: usize,
}

pub(super) fn animated_runtime_mesh_key(
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
) -> GtaAnimatedRuntimeMeshKey {
    GtaAnimatedRuntimeMeshKey {
        source,
        slot,
        model_index,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RuntimePoolReuseKey {
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
    render_damaged: bool,
}

pub(super) fn runtime_pool_reuse_key(
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
    render_damaged: bool,
) -> RuntimePoolReuseKey {
    RuntimePoolReuseKey {
        source,
        slot,
        model_index,
        render_damaged,
    }
}

#[derive(Debug, Clone, Default, Resource)]
pub(super) struct GtaAnimatedRuntimeMeshRegistry {
    pub(super) slots: BTreeMap<GtaAnimatedRuntimeMeshKey, GtaAnimatedRuntimeMeshSlot>,
    pub(super) touched: BTreeSet<GtaAnimatedRuntimeMeshKey>,
}

#[derive(Debug, Clone)]
pub(super) struct GtaAnimatedRuntimeMeshSlot {
    pub(super) entities: Vec<Entity>,
    pub(super) meshes: Vec<Handle<Mesh>>,
    pub(super) materials: Vec<Handle<GtaUnlitMaterial>>,
    /// Material alpha before RenderWare clump alpha modulation. Clump alpha
    /// belongs to the RpClump, not the RpMaterial, so keep the authored value
    /// separately and derive the final GPU alpha every frame.
    pub(super) material_base_alphas: Vec<f32>,
    pub(super) material_base_alpha_modes: Vec<AlphaMode>,
    /// Submesh indices backed by wheel/door/rotor frames. re3 keeps the
    /// vehicle clump resident and updates these frames in place; retaining the
    /// indices avoids rebuilding static body geometry at 30 Hz.
    pub(super) dynamic_vehicle_parts: Option<Vec<usize>>,
    /// Frame baked into each resident vehicle submesh at allocation time.
    /// `CAutomobile::Render` changes the corresponding RW frames, not the
    /// immutable geometry, so later frames apply only the relative transform.
    pub(super) dynamic_vehicle_baselines: Option<Vec<Option<Transform>>>,
    /// Last transform submitted for each resident entity. Avoid marking every
    /// vehicle submesh changed when neither its clump nor component frame moved.
    pub(super) rendered_transforms: Vec<Transform>,
    /// Root world transform used to produce `rendered_transforms`. Runtime
    /// geometry/skinning is intentionally refreshed at 30 Hz, but the live
    /// CEntity matrix still changes every game frame. Keeping the root lets a
    /// lightweight pass move the resident clump without rebuilding geometry.
    pub(super) rendered_root_transform: Transform,
    /// Geometry/material-affecting state. Wheel rotation, suspension, steering
    /// and door hinge angles deliberately do not participate.
    pub(super) vehicle_resident_state: Option<VehicleResidentMeshState>,
    /// One RW atomic frame per material submesh. Present for GTA III PC's
    /// non-skinned, frame-separated ped/cutscene clumps.
    pub(super) rigid_frame_indices: Option<Vec<i32>>,
    pub(super) hide_original_head: bool,
    /// Identity of the currently resident original DFF. Special cutscene
    /// slots 185..189 are reused with different names between BET/JB, so
    /// `(pool slot, model id)` alone is not a RenderWare object identity.
    pub(super) geometry_signature: u64,
    /// Vertex/index layout of each resident mesh. Bevy's render-world mesh
    /// buffer is allocated from this shape; replacing a Mesh with a different
    /// layout while retaining the handle can make the render queue write past
    /// the old GPU buffer (especially when many animated traffic peds spawn).
    pub(super) mesh_layout_signature: Vec<(usize, usize)>,
    pub(super) is_visible: bool,
}

pub(super) fn prepare_runtime_pool_mesh_refresh(
    mut refresh: ResMut<GtaRuntimeMeshRefresh>,
    mut animated_registry: ResMut<GtaAnimatedRuntimeMeshRegistry>,
    timer: Res<CTimer>,
    mut commands: Commands,
    existing: Query<
        (
            Entity,
            &GtaRuntimePoolMesh,
            &Mesh3d,
            &MeshMaterial3d<GtaUnlitMaterial>,
        ),
        With<GtaRuntimePoolMesh>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut last_refresh_ms: Local<u32>,
    mut refresh_started: Local<bool>,
    mut persistent_entities: Local<BTreeSet<Entity>>,
) {
    // GTA III's animation/render target is 30 Hz. Rebuilding CPU-skinned
    // geometry at the display refresh rate made a 60/144 Hz window perform
    // two to five times the reference work.
    let now_ms = timer.time_in_milliseconds();
    let forced = refresh.force_rebuild;
    refresh.force_rebuild = false;
    refresh.rebuild_this_frame =
        forced || runtime_mesh_refresh_due(now_ms, *last_refresh_ms, *refresh_started);
    if !refresh.rebuild_this_frame {
        return;
    }
    *refresh_started = true;
    *last_refresh_ms = now_ms;
    // re3 does not destroy and recreate every static RwObject every four
    // seconds. Stale keyed entities are removed explicitly after the live
    // runtime set has been flushed.
    refresh.static_rebuild = false;
    animated_registry.touched.clear();

    persistent_entities.clear();
    persistent_entities.extend(
        animated_registry
            .slots
            .values()
            .flat_map(|slot| slot.entities.iter().copied()),
    );
    for (entity, runtime, mesh, material) in &existing {
        if is_persistently_animated_runtime_source(runtime.source)
            && persistent_entities.contains(&entity)
        {
            continue;
        }
        let transient = is_transient_runtime_source(runtime.source);
        if transient || refresh.static_rebuild {
            meshes.remove(mesh.0.id());
            materials.remove(material.0.id());
            commands.entity(entity).despawn();
        }
    }
}

pub(super) fn runtime_mesh_refresh_due(
    now_ms: u32,
    last_refresh_ms: u32,
    refresh_started: bool,
) -> bool {
    !refresh_started || now_ms.wrapping_sub(last_refresh_ms) >= (1_000 / 30)
}

#[cfg(test)]
mod runtime_mesh_refresh_tests {
    use super::*;

    #[test]
    fn moving_vehicles_use_the_persistent_mesh_registry() {
        assert!(is_persistently_animated_runtime_source(
            GtaRuntimePoolMeshSource::ScriptVehicleDynamic
        ));
        assert!(!is_persistently_animated_runtime_source(
            GtaRuntimePoolMeshSource::ScriptVehicle
        ));
        assert!(!is_transient_runtime_source(
            GtaRuntimePoolMeshSource::ScriptVehicle
        ));
        assert!(!is_transient_runtime_source(
            GtaRuntimePoolMeshSource::ScriptVehicleDynamic
        ));
        assert!(is_persistently_animated_runtime_source(
            GtaRuntimePoolMeshSource::WorldPlayerWeapon
        ));
        assert!(!is_transient_runtime_source(
            GtaRuntimePoolMeshSource::WorldPlayerWeapon
        ));
    }

    #[test]
    fn runtime_geometry_refresh_keeps_the_re3_thirty_hz_boundary() {
        assert!(runtime_mesh_refresh_due(0, 0, false));
        assert!(!runtime_mesh_refresh_due(32, 0, true));
        assert!(runtime_mesh_refresh_due(33, 0, true));
    }

    #[test]
    fn empty_runtime_scene_does_not_force_a_mesh_rebuild_every_render_frame() {
        assert!(runtime_mesh_refresh_due(0, 0, false));
        assert!(!runtime_mesh_refresh_due(0, 0, true));
        assert!(!runtime_mesh_refresh_due(32, 0, true));
        assert!(runtime_mesh_refresh_due(33, 0, true));
        assert!(runtime_mesh_refresh_due(25, u32::MAX - 20, true));
    }

    #[test]
    fn scm_hidden_object_hides_and_restores_its_existing_static_draws() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<GtaUnlitMaterial>>()
            .insert_resource(GtaRuntimeMeshRefresh {
                rebuild_this_frame: true,
                ..Default::default()
            })
            .add_systems(Update, cleanup_stale_static_runtime_pool_meshes);
        let mut general = crate::core::General::CGeneral::default();
        let mut object = CObject::new(&mut general);
        object.physical.entity.model_index = 1393;
        let owner = app
            .world_mut()
            .spawn((ScriptObjectIdentity { handle: 7 }, object))
            .id();
        let draw = app
            .world_mut()
            .spawn((
                GtaRuntimePoolMesh {
                    source: GtaRuntimePoolMeshSource::ScriptObject,
                    slot: 7,
                    model_index: 1393,
                    render_damaged: false,
                },
                Mesh3d(Handle::default()),
                MeshMaterial3d::<GtaUnlitMaterial>(Handle::default()),
                Visibility::Visible,
            ))
            .id();
        app.update();
        assert!(app.world().get_entity(draw).is_ok());
        app.world_mut()
            .get_mut::<CObject>(owner)
            .unwrap()
            .physical
            .entity
            .flags
            .is_visible = false;
        app.update();
        assert_eq!(app.world().get::<Visibility>(draw), Some(&Visibility::Hidden));
        assert!(app.world().get_entity(owner).is_ok());
        app.world_mut().get_mut::<CObject>(owner).unwrap().physical.entity.flags.is_visible = true;
        app.update();
        assert_eq!(app.world().get::<Visibility>(draw), Some(&Visibility::Visible));
        app.world_mut().despawn(owner);
        app.update();
        assert!(app.world().get_entity(draw).is_err());
    }

    #[test]
    fn damaged_script_object_cannot_reuse_the_intact_mesh() {
        let intact = runtime_pool_reuse_key(GtaRuntimePoolMeshSource::ScriptObject, 7, 1393, false);
        let damaged = runtime_pool_reuse_key(GtaRuntimePoolMeshSource::ScriptObject, 7, 1393, true);
        assert_ne!(intact, damaged);
    }

    #[test]
    fn skipped_geometry_refresh_rebases_the_resident_clump_root() {
        let previous_root = Transform::from_translation(Vec3::new(10.0, 2.0, -4.0))
            .with_rotation(Quat::from_rotation_y(0.25));
        let local_part = Transform::from_translation(Vec3::new(0.8, 0.3, -0.2))
            .with_rotation(Quat::from_rotation_x(-0.4));
        let rendered = previous_root.mul_transform(local_part);
        let next_root = Transform::from_translation(Vec3::new(12.0, 2.5, -3.0))
            .with_rotation(Quat::from_rotation_y(0.55));

        let rebased = rebase_runtime_render_transform(rendered, previous_root, next_root);
        let expected = next_root.mul_transform(local_part);
        assert!(
            rebased
                .translation
                .abs_diff_eq(expected.translation, 1.0e-5)
        );
        assert!(rebased.rotation.abs_diff_eq(expected.rotation, 1.0e-5));
        assert!(rebased.scale.abs_diff_eq(expected.scale, 1.0e-5));
    }

    #[test]
    fn normal_driving_updates_components_without_rewriting_the_body() {
        let mut normal = VehicleColourRemap::default();
        normal.wheel_state = Some(VehicleWheelRenderState {
            rotation: [1.0; 4],
            position: [-0.4; 4],
            suspension_initialized: true,
            steer_angle: 0.2,
            status: [0; 4],
        });
        assert!(!normal.requires_full_dynamic_refresh());

        let mut open_door = normal;
        open_door.door_state = Some(VehicleDoorRenderState {
            angles: [0.0, 0.0, 0.5, 0.0, 0.0, 0.0],
            axes: [0; 6],
        });
        // Door frames are now persistent ECS transforms and do not rebuild
        // resident geometry/materials when their authored angle changes.
        assert!(!open_door.requires_full_dynamic_refresh());

        let mut damaged = normal;
        damaged.damage_state = Some(VehicleDamageRenderState {
            door_status: [0, 0, 2, 0, 0, 0],
            panel_status: [0; 7],
        });
        assert!(damaged.requires_full_dynamic_refresh());
    }
}

pub(super) fn cleanup_stale_static_runtime_pool_meshes(
    refresh: Res<GtaRuntimeMeshRefresh>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    objects: Query<(&ScriptObjectIdentity, &CObject)>,
    pools: Option<Res<CPools>>,
    mut existing: Query<
        (
            Entity,
            &GtaRuntimePoolMesh,
            &Mesh3d,
            &MeshMaterial3d<GtaUnlitMaterial>,
            &mut Visibility,
        ),
        With<GtaRuntimePoolMesh>,
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut live: Local<BTreeSet<RuntimePoolReuseKey>>,
    mut hidden: Local<BTreeSet<RuntimePoolReuseKey>>,
) {
    if !refresh.rebuild_this_frame {
        return;
    }
    live.clear();
    hidden.clear();
    for entity in vehicles.iter() {
        let handle = entity.handle();
        let base = entity.base();
        if !base.physical.entity.flags.is_visible {
            continue;
        }
        // A pool vehicle keeps one RenderWare clump for its whole lifetime.
        // Switching registry identity when its velocity crosses zero leaves
        // the old clump alive until the next refresh and visibly duplicates
        // SCM cars as their first physics frame starts.
        let source = GtaRuntimePoolMeshSource::ScriptVehicleDynamic;
        live.insert(runtime_pool_reuse_key(
            source,
            handle as u32 as usize,
            base.physical.entity.model_index.max(0) as usize,
            false,
        ));
    }
    for (identity, object) in &objects {
        let key = runtime_pool_reuse_key(
            GtaRuntimePoolMeshSource::ScriptObject,
            identity.handle as u32 as usize,
            object.physical.entity.model_index.max(0) as usize,
            object.physical.entity.flags.render_damaged,
        );
        live.insert(key);
        // SCM bIsVisible hides the RwObject, retaining its geometry and TXD
        // ownership until the object is shown again or deleted.
        if !object.physical.entity.flags.is_visible || object.physical.entity.flags.do_not_render {
            hidden.insert(key);
        }
    }
    if let Some(pools) = pools {
        for (slot, record) in pools.compatible_ped_records() {
            if let Some(model) = compatible_model_index(record.model_index) {
                live.insert(runtime_pool_reuse_key(
                    GtaRuntimePoolMeshSource::SavePed,
                    slot,
                    model,
                    false,
                ));
            }
        }
        for (slot, record) in pools.compatible_vehicle_records() {
            if let Some(model) = compatible_model_index(record.model_index) {
                live.insert(runtime_pool_reuse_key(
                    GtaRuntimePoolMeshSource::SaveVehicle,
                    slot,
                    model,
                    false,
                ));
            }
        }
        for (slot, record) in pools.compatible_object_records() {
            if let Some(model) = compatible_model_index(record.model_index) {
                live.insert(runtime_pool_reuse_key(
                    GtaRuntimePoolMeshSource::SaveObject,
                    slot,
                    model,
                    record.entity_flags.flags.render_damaged,
                ));
            }
        }
    }

    for (entity, runtime, mesh, material, mut visibility) in &mut existing {
        let key = runtime_pool_reuse_key(
            runtime.source, runtime.slot, runtime.model_index, runtime.render_damaged,
        );
        if runtime.source == GtaRuntimePoolMeshSource::ScriptObject && live.contains(&key) {
            let desired = if hidden.contains(&key) {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            if *visibility != desired {
                *visibility = desired;
            }
        }
        if !matches!(
            runtime.source,
            GtaRuntimePoolMeshSource::ScriptVehicle
                | GtaRuntimePoolMeshSource::ScriptObject
                | GtaRuntimePoolMeshSource::SavePed
                | GtaRuntimePoolMeshSource::SaveVehicle
                | GtaRuntimePoolMeshSource::SaveObject
        ) || live.contains(&runtime_pool_reuse_key(
            runtime.source,
            runtime.slot,
            runtime.model_index,
            runtime.render_damaged,
        )) {
            continue;
        }
        meshes.remove(mesh.0.id());
        materials.remove(material.0.id());
        commands.entity(entity).despawn();
    }
}

pub(super) fn is_persistently_animated_runtime_source(source: GtaRuntimePoolMeshSource) -> bool {
    matches!(
        source,
        GtaRuntimePoolMeshSource::WorldPlayer
            | GtaRuntimePoolMeshSource::WorldPlayerWeapon
            | GtaRuntimePoolMeshSource::Projectile
            | GtaRuntimePoolMeshSource::ScriptPed
            | GtaRuntimePoolMeshSource::ScriptPedWeapon
            | GtaRuntimePoolMeshSource::CutsceneObject
            | GtaRuntimePoolMeshSource::ScriptVehicleDynamic
    )
}

pub(super) fn is_transient_runtime_source(source: GtaRuntimePoolMeshSource) -> bool {
    matches!(
        source,
        GtaRuntimePoolMeshSource::ReplayPed | GtaRuntimePoolMeshSource::ReplayVehicle
    )
}

pub(super) fn cleanup_animated_runtime_pool_meshes(
    refresh: Res<GtaRuntimeMeshRefresh>,
    cutscene: Option<Res<CCutsceneMgr>>,
    script_index: Option<Res<ScriptEntityIndex>>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore,
    peds: Query<&ScriptPedIdentity, With<CPed>>,
    mut registry: ResMut<GtaAnimatedRuntimeMeshRegistry>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut stale: Local<Vec<GtaAnimatedRuntimeMeshKey>>,
) {
    let cutscene_active = cutscene
        .as_ref()
        .is_some_and(|cutscene| cutscene.loaded || cutscene.running);
    if !refresh.rebuild_this_frame {
        return;
    }
    stale.clear();
    let GtaAnimatedRuntimeMeshRegistry { slots, touched } = &mut *registry;
    for (&key, slot) in slots.iter_mut() {
        let source = key.source;
        let handle = key.slot;
        let is_cutscene_stale =
            !cutscene_active && source == GtaRuntimePoolMeshSource::CutsceneObject;
        let is_script_vehicle_stale = source == GtaRuntimePoolMeshSource::ScriptVehicleDynamic
            && script_index.as_ref().is_some_and(|index| {
                index
                    .vehicle(handle as i32)
                    .and_then(|entity| vehicles.get(entity).ok())
                    .is_none_or(|vehicle| vehicle.handle() != handle as i32)
            });
        let is_script_ped_stale = source == GtaRuntimePoolMeshSource::ScriptPed
            && script_index.as_ref().is_some_and(|index| {
                index
                    .ped(handle as i32)
                    .and_then(|entity| peds.get(entity).ok())
                    .is_none_or(|ped| ped.handle != handle as i32)
            });
        let is_weapon_stale = matches!(
            source,
            GtaRuntimePoolMeshSource::WorldPlayerWeapon | GtaRuntimePoolMeshSource::ScriptPedWeapon
        ) && !touched.contains(&key);
        let is_projectile_stale =
            source == GtaRuntimePoolMeshSource::Projectile && !touched.contains(&key);
        let is_stale = is_cutscene_stale
            || is_script_vehicle_stale
            || is_script_ped_stale
            || is_weapon_stale
            || is_projectile_stale;
        if is_stale {
            stale.push(key);
        } else if !touched.contains(&key) {
            // re3 `CAutomobile::Render` / `CPed::Render` only touch visible entities.
            // An off-screen vehicle or ped still exists in the world: keep its Bevy
            // entities, meshes, and materials resident and hide them rather than
            // despawning and allocating new GPU assets at 30 Hz.
            if slot.is_visible {
                slot.is_visible = false;
                for entity in &slot.entities {
                    if let Ok(mut entity_commands) = commands.get_entity(*entity) {
                        entity_commands.insert(Visibility::Hidden);
                    }
                }
            }
        }
    }
    for key in stale.iter().copied() {
        if let Some(slot) = registry.slots.remove(&key) {
            for entity in slot.entities {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn();
                }
            }
            for mesh in slot.meshes {
                meshes.remove(mesh.id());
            }
            for material in slot.materials {
                materials.remove(material.id());
            }
        }
    }
}

/// Immutable presentation join. The index is the generation authority; queries
/// borrow live components after the gameplay command barrier without copying
/// actors or acquiring SCM's pending construction/mutation buffers.
#[derive(SystemParam)]
pub(super) struct RuntimeActorsRead<'w, 's> {
    index: Res<'w, ScriptEntityIndex>,
    peds: Query<'w, 's, (Entity, &'static ScriptPedIdentity, &'static CPed, &'static PedAnimation, &'static PedVitality, &'static PedLocomotion, Option<&'static InVehicle>)>,
    vehicles: crate::control::ScriptEntities::VehicleReadStore<'w, 's>,
    objects: Query<'w, 's, (Entity, &'static ScriptObjectIdentity, &'static CObject)>,
    relations: Query<'w, 's, (Entity, &'static InVehicle, Option<&'static ScriptPedIdentity>), With<PedEntity>>,
}

impl RuntimeActorsRead<'_, '_> {
    pub(super) fn ped_entity(&self, handle: i32) -> Option<Entity> {
        let entity = self.index.ped(handle)?;
        let (_, identity, _, _, _, _, _) = self.peds.get(entity).ok()?;
        (identity.handle == handle).then_some(entity)
    }

    pub(super) fn vehicle_entity(&self, handle: i32) -> Option<Entity> {
        let entity = self.index.vehicle(handle)?;
        let vehicle = self.vehicles.get(entity).ok()?;
        (vehicle.handle() == handle).then_some(entity)
    }

    pub(super) fn current_vehicle_handle_for_char(&self, handle: i32) -> Option<i32> {
        let relation = self.relation_for_char(handle)?;
        self.vehicle_handle_for_entity(relation.vehicle)
    }

    pub(super) fn relation_occupants(&self, vehicle: Entity)
        -> impl Iterator<Item = (Entity, crate::peds::VehicleRelations::VehicleSeat)> + '_
    {
        let vehicle_is_live = self.vehicle_handle_for_entity(vehicle).is_some();
        self.relations.iter().filter_map(move |(ped, relation, identity)| {
            if !vehicle_is_live || relation.vehicle != vehicle {
                return None;
            }
            if let Some(identity) = identity
                && self.index.ped(identity.handle) != Some(ped)
            {
                return None;
            }
            Some((ped, relation.seat))
        })
    }

    pub(super) fn ped_get(&self, handle: i32) -> Option<ScriptPedRef<'_>> {
        let (_, identity, ped, animation, vitality, locomotion, _) = self.peds.get(self.ped_entity(handle)?).ok()?;
        Some(ScriptPedRef { handle: identity.handle, model_id: identity.model_id, ped, animation, vitality, locomotion })
    }

    pub(super) fn object_get(&self, handle: i32) -> Option<ScriptObjectRef<'_>> {
        let (_, identity, object) = self.objects.get(self.index.object(handle)?).ok()?;
        (identity.handle == handle).then_some(ScriptObjectRef { handle, object })
    }

    pub(super) fn relation_for_char(&self, handle: i32) -> Option<&InVehicle> {
        self.peds.get(self.ped_entity(handle)?).ok()?.6
    }

    pub(super) fn vehicle_handle_for_entity(&self, entity: Entity) -> Option<i32> {
        let vehicle = self.vehicles.get(entity).ok()?;
        let handle = vehicle.handle();
        (self.index.vehicle(handle) == Some(entity)).then_some(handle)
    }

    pub(super) fn vehicle_get(&self, handle: i32) -> Option<crate::control::ScriptEntities::ScriptVehicleRef<'_>> {
        let vehicle = self.vehicles.get(self.index.vehicle(handle)?).ok()?;
        (vehicle.handle() == handle).then_some(vehicle)
    }

    pub(super) fn peds(&self) -> impl Iterator<Item = (i32, ScriptPedRef<'_>)> {
        self.peds.iter().filter_map(|(entity, identity, ped, animation, vitality, locomotion, _)| {
            (self.index.ped(identity.handle) == Some(entity)).then_some((identity.handle,
                ScriptPedRef { handle: identity.handle, model_id: identity.model_id, ped, animation, vitality, locomotion }))
        })
    }

    pub(super) fn vehicles(&self) -> impl Iterator<Item = (i32, crate::control::ScriptEntities::ScriptVehicleRef<'_>)> {
        self.vehicles.entities.iter().filter_map(|item| {
            let entity=item.0; let vehicle=crate::control::ScriptEntities::vehicle_ref(item)?;
            (self.index.vehicle(vehicle.handle())==Some(entity)).then_some((vehicle.handle(),vehicle))
        })
    }

    pub(super) fn objects(&self) -> impl Iterator<Item = ScriptObjectRef<'_>> {
        self.objects.iter().filter_map(|(entity, identity, object)| {
            (self.index.object(identity.handle) == Some(entity)).then_some(ScriptObjectRef { handle: identity.handle, object })
        })
    }
}

// Submission consumes drawable/material owners; Bevy owns the live actor queries.
use super::{
    CAnimManager,
    CPedVehicleAnimation,
    CTimeCycle,
    CVector,
    CWorldPlayersRead,
    GtaImmediateMesh,
    GtaRenderLighting,
    GtaWorldRenderStats,
    MI_PLAYER,
    NoFrustumCulling,
    PedEntity,
    RenderAuditTimer,
    ReplayPedPoolOverlay,
    ReplayPedRuntimeState,
    ReplayVehiclePoolOverlay,
    ReplayVehicleRuntimeState,
    ScriptGameplayBridge,
    apply_player_mouse_diagonal_leg_pose,
    build_cutscene_drawables,
    build_rigid_cutscene_drawables,
    cutscene_object_render_transform,
    ensure_colour_texture_table,
    ensure_environment_maps,
    entity_matrix_to_bevy_transform,
    gta_world_material,
    model_uses_cpu_skinning,
    object_record_transform,
    ped_bone_poses_from_association,
    ped_frame_transform,
    ped_ifp_bone_poses,
    ped_ifp_bone_poses_for_one_shot,
    ped_record_transform,
    player_anim_clump_bone_poses,
    player_anim_clump_bone_poses_with_overlay,
    replay_matrix_transform,
    resolve_drawable_material_bind,
    rigid_cutscene_part_transforms,
    srgb_i32_to_linear,
    vehicle_colour_remap_from_indices,
    vehicle_record_transform,
    world_distance
};
use bevy::ecs::system::SystemParam;
use bevy::mesh::Indices;
use crate::{control::ScriptEntities::{ScriptPedRef, ScriptObjectRef}, peds::{PedAnimation::PedAnimation, PedVitality::PedVitality, PedLocomotion::PedLocomotion, VehicleRelations::InVehicle}};

/// Keep optional gameplay resources grouped so the renderer system remains
/// below Bevy's system-function parameter arity while retaining ECS injection.
#[derive(SystemParam)]
pub(super) struct RuntimeMeshAnimationResources<'w, 's> {
    anim_manager: Option<Res<'w, CAnimManager>>,
    vehicle_animations: Query<'w, 's, &'static CPedVehicleAnimation, With<PedEntity>>,
    timer: Option<Res<'w, CTimer>>,
    weapon_info: Option<Res<'w, crate::weapons::WeaponInfo::CWeaponInfoStore>>,
    projectiles: Option<Res<'w, crate::weapons::ProjectileInfo::CProjectileInfoPool>>,
    handling_manager: Option<Res<'w, crate::vehicles::HandlingMgr::cHandlingDataMgr>>,
    ped_animation_offsets: Option<Res<'w, crate::peds::Ped::CPedAnimationOffsets>>,
    refresh: Res<'w, GtaRuntimeMeshRefresh>,
    diagnostics: ResMut<'w, GtaRuntimeSubmissionDiagnostics>,
    existing_runtime: Query<'w, 's, (Entity, &'static GtaRuntimePoolMesh, &'static Transform)>,
    animated_registry: ResMut<'w, GtaAnimatedRuntimeMeshRegistry>,
    reusable_runtime: Local<'s, BTreeMap<RuntimePoolReuseKey, Vec<(Entity, Transform)>>>,
}

#[derive(SystemParam)]
pub(super) struct RuntimeReplayEntities<'w, 's> {
    vehicles: Query<'w, 's, &'static ReplayVehicleRuntimeState>,
    peds: Query<'w, 's, &'static ReplayPedRuntimeState>,
}


pub(super) fn rebase_runtime_render_transform(
    rendered: Transform,
    previous_root: Transform,
    next_root: Transform,
) -> Transform {
    Transform::from_matrix(Mat4::from(
        next_root.compute_affine()
            * previous_root.compute_affine().inverse()
            * rendered.compute_affine(),
    ))
}

pub(super) fn rebase_cached_runtime_mesh_root(
    cached: &mut GtaAnimatedRuntimeMeshSlot,
    next_root: Transform,
    commands: &mut Commands,
) {
    if cached.rendered_root_transform == next_root
        || cached.entities.len() != cached.rendered_transforms.len()
    {
        return;
    }
    let previous_root = cached.rendered_root_transform;
    for (entity, rendered) in cached
        .entities
        .iter()
        .copied()
        .zip(cached.rendered_transforms.iter_mut())
    {
        let desired = rebase_runtime_render_transform(*rendered, previous_root, next_root);
        if *rendered != desired {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(desired);
            }
            *rendered = desired;
        }
    }
    cached.rendered_root_transform = next_root;
}

pub(super) fn rebase_cached_runtime_mesh_relative(
    cached: &mut GtaAnimatedRuntimeMeshSlot,
    previous_parent: Transform,
    next_parent: Transform,
    commands: &mut Commands,
) {
    if previous_parent == next_parent || cached.entities.len() != cached.rendered_transforms.len() {
        return;
    }
    for (entity, rendered) in cached
        .entities
        .iter()
        .copied()
        .zip(cached.rendered_transforms.iter_mut())
    {
        let desired = rebase_runtime_render_transform(*rendered, previous_parent, next_parent);
        if *rendered != desired {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.insert(desired);
            }
            *rendered = desired;
        }
    }
    cached.rendered_root_transform = rebase_runtime_render_transform(
        cached.rendered_root_transform,
        previous_parent,
        next_parent,
    );
}

/// `GtaRuntimeMeshRefresh` limits CPU skinning/component rebuild work to the
/// original 30 Hz cadence. The live CEntity matrices are not 30 Hz state,
/// though: player/vehicle control and the camera consume them every Update.
/// Move the resident clump root on skipped refresh frames so rendering cannot
/// lag one or more simulation frames and then visibly snap forward.
fn sync_persistent_runtime_root_transforms(
    file_loader: &CFileLoader,
    world_players: Option<&CWorldPlayersRead>,
    script_entities: Option<&RuntimeActorsRead>,
    vehicle_animations: &Query<&CPedVehicleAnimation, With<PedEntity>>,
    handling_manager: Option<&crate::vehicles::HandlingMgr::cHandlingDataMgr>,
    ped_animation_offsets: Option<&crate::peds::Ped::CPedAnimationOffsets>,
    now_ms: u32,
    registry: &mut GtaAnimatedRuntimeMeshRegistry,
    commands: &mut Commands,
) {
    if let Some(world_players) = world_players {
        for (slot, player) in world_players.iter_slots() {
            let Some(player) = player.as_ref() else {
                continue;
            };
            if !player.active_in_world {
                continue;
            }
            let raw_model = player.ped.physical.entity.model_index;
            let model_index = if raw_model <= 0 {
                MI_PLAYER
            } else {
                raw_model as usize
            };
            let active_vehicle_anim = world_players
                .player_entity(slot as i32)
                .and_then(|entity| vehicle_animations.get(entity).ok());
            // re3 mutates RW frames every render, not at the 30 Hz geometry
            // cadence. Rebase the seated/line-up root every frame with the
            // same matrix choice as the flush path; skipping animated peds
            // here left the mesh updating at 30 Hz while the car moved every
            // frame, which reads as judder/teleport during entry, exit and
            // while seated.
            let seated_matrix = if active_vehicle_anim.is_none() {
                world_players
                    .relation(slot as i32)
                    .and_then(|relation| {
                        script_entities.and_then(|world| {
                            world
                                .vehicle_handle_for_entity(relation.vehicle)
                                .and_then(|handle| world.vehicle_get(handle))
                                .map(|vehicle| (vehicle, relation.seat))
                        })
                    })
                    .and_then(|(vehicle, seat)| match vehicle {
                        crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                            crate::peds::Ped::automobile_occupant_matrix(file_loader, &car, seat)
                        }
                        _ => None,
                    })
            } else {
                None
            };
            let line_up_matrix = active_vehicle_anim
                .zip(handling_manager)
                .zip(ped_animation_offsets)
                .and_then(|((active, handling_manager), offsets)| {
                    let vehicle_handle = script_entities
                        .as_ref()?
                        .vehicle_handle_for_entity(active.vehicle)?;
                    let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } =
                        script_entities.as_ref()?.vehicle_get(vehicle_handle)?
                    else {
                        return None;
                    };
                    let handling = car
                        .vehicle.core.handling_id
                        .and_then(|id| usize::try_from(id).ok())
                        .and_then(|id| handling_manager.get_handling_data(id))?;
                    crate::peds::PedAI::automobile_ped_line_up_matrix(
                        file_loader,
                        &car,
                        handling,
                        offsets,
                        active.door,
                        crate::peds::PedAI::PedVehicleLineUpContext {
                            current_position: player.ped.physical.entity.get_position(),
                            current_heading: player.locomotion.rotation_cur,
                            offset_seek: player.ped.offset_seek,
                            state_timer_ms: player.ped.ped_state_timer,
                            now_ms,
                        },
                        active.phase,
                        &active.association,
                    )
                });
            let root = entity_matrix_to_bevy_transform(
                line_up_matrix
                    .as_ref()
                    .or(seated_matrix.as_ref())
                    .unwrap_or(&player.ped.physical.entity.matrix),
            );
            let key =
                animated_runtime_mesh_key(GtaRuntimePoolMeshSource::WorldPlayer, slot, model_index);
            let previous_root = registry
                .slots
                .get(&key)
                .map(|cached| cached.rendered_root_transform);
            if let Some(cached) = registry.slots.get_mut(&key) {
                rebase_cached_runtime_mesh_root(cached, root, commands);
            }
            // Also rebase the player's weapon mesh if attached
            if let Some(previous_root) = previous_root {
                for (key, cached) in registry.slots.iter_mut() {
                    if key.source == GtaRuntimePoolMeshSource::WorldPlayerWeapon && key.slot == slot
                    {
                        rebase_cached_runtime_mesh_relative(cached, previous_root, root, commands);
                    }
                }
            }
        }
    }

    let Some(script_entities) = script_entities else {
        return;
    };
    for (handle, entity) in script_entities.peds() {
        let active_vehicle_anim = script_entities
            .ped_entity(handle)
            .and_then(|ped_entity| vehicle_animations.get(ped_entity).ok());
        // Same per-frame root rule as the flush path below: a seated ped
        // rides the occupant matrix (except during exit anims, which own the
        // authored root), so rebase it instead of leaving it at 30 Hz.
        let exit_animation_active = active_vehicle_anim.is_some_and(|active| {
            matches!(
                active.phase,
                crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::Exit
                    | crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::CloseExit
                    | crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::DraggedOut
            )
        });
        let seated_matrix = (!exit_animation_active)
            .then_some(script_entities.relation_for_char(handle))
            .flatten()
            .and_then(|relation| {
                script_entities
                    .vehicle_handle_for_entity(relation.vehicle)
                    .and_then(|vehicle_handle| script_entities.vehicle_get(vehicle_handle))
                    .and_then(|vehicle| {
                        crate::peds::Ped::vehicle_occupant_matrix(
                            file_loader,
                            &vehicle.base(),
                            relation.seat,
                        )
                    })
            });
        if active_vehicle_anim.is_some() && seated_matrix.is_none() {
            continue;
        }
        let model_index = entity.model_id.max(0) as usize;
        let key = animated_runtime_mesh_key(
            GtaRuntimePoolMeshSource::ScriptPed,
            handle as u32 as usize,
            model_index,
        );
        if let Some(cached) = registry.slots.get_mut(&key) {
            rebase_cached_runtime_mesh_root(
                cached,
                entity_matrix_to_bevy_transform(
                    seated_matrix
                        .as_ref()
                        .unwrap_or(&entity.ped.physical.entity.matrix),
                ),
                commands,
            );
        }
    }
    for (handle, entity) in script_entities.vehicles() {
        let base = entity.base();
        let model_index = base.physical.entity.model_index.max(0) as usize;
        let key = animated_runtime_mesh_key(
            GtaRuntimePoolMeshSource::ScriptVehicleDynamic,
            handle as u32 as usize,
            model_index,
        );
        if let Some(cached) = registry.slots.get_mut(&key) {
            rebase_cached_runtime_mesh_root(
                cached,
                entity_matrix_to_bevy_transform(&base.physical.entity.matrix),
                commands,
            );
        }
    }
}

pub(super) fn apply_runtime_clump_alpha(
    registry: &GtaAnimatedRuntimeMeshRegistry,
    materials: &mut Assets<GtaUnlitMaterial>,
    key: GtaAnimatedRuntimeMeshKey,
    alpha: u8,
) {
    let Some(cached) = registry.slots.get(&key) else {
        return;
    };
    let factor = f32::from(alpha) / 255.0;
    for (part, handle) in cached.materials.iter().enumerate() {
        let Some(base_alpha) = cached.material_base_alphas.get(part).copied() else {
            continue;
        };
        let Some(base_mode) = cached.material_base_alpha_modes.get(part) else {
            continue;
        };
        let desired_alpha = base_alpha * factor;
        let desired_mode = if alpha < u8::MAX
            && !matches!(base_mode, AlphaMode::Add | AlphaMode::Multiply)
        {
            AlphaMode::Blend
        } else {
            *base_mode
        };
        if materials.get(handle).is_some_and(|material| {
            material.uniforms.colour.w != desired_alpha || material.alpha_mode != desired_mode
        }) && let Some(mut material) = materials.get_mut(handle) {
            material.uniforms.colour.w = desired_alpha;
            material.alpha_mode = desired_mode;
        }
    }
}

fn sync_script_runtime_clump_alphas(
    script_entities: &RuntimeActorsRead,
    registry: &GtaAnimatedRuntimeMeshRegistry,
    materials: &mut Assets<GtaUnlitMaterial>,
) {
    for (handle, entity) in script_entities.peds() {
        let model_index = entity.ped.physical.entity.model_index.max(0) as usize;
        apply_runtime_clump_alpha(
            registry,
            materials,
            animated_runtime_mesh_key(
                GtaRuntimePoolMeshSource::ScriptPed,
                handle as u32 as usize,
                model_index,
            ),
            entity.ped.physical.entity.clump_alpha,
        );
    }
    for (handle, entity) in script_entities.vehicles() {
        let base = entity.base();
        let model_index = base.physical.entity.model_index.max(0) as usize;
        apply_runtime_clump_alpha(
            registry,
            materials,
            animated_runtime_mesh_key(
                GtaRuntimePoolMeshSource::ScriptVehicleDynamic,
                handle as u32 as usize,
                model_index,
            ),
            base.physical.entity.clump_alpha,
        );
    }
}

pub(super) fn flush_runtime_pool_meshes(
    file_loader: Option<Res<CFileLoader>>,

    pools: Option<Res<CPools>>,
    script_entities: Option<RuntimeActorsRead>,
    world_players: CWorldPlayersRead,
    bridge: Option<Res<ScriptGameplayBridge>>,
    cutscene: Option<Res<CCutsceneMgr>>,
    replay: RuntimeReplayEntities,
    mut animation_resources: RuntimeMeshAnimationResources,
    vehicle_colour_table: Option<Res<crate::modelinfo::VehicleModelInfo::CVehicleColours>>,
    timecycle: Option<Res<CTimeCycle>>,
    mut world_stats: ResMut<GtaWorldRenderStats>,
    mut texture_cache: ResMut<GtaWorldTextureCache>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GtaUnlitMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    crate::profile_scope!("flush_runtime_pool_meshes");
    let _audit_timer = RenderAuditTimer::start("flush_runtime_pool_meshes");
    if file_loader.is_none() {
        animation_resources.diagnostics.missing_file_loader_frames += 1;
        if animation_resources.diagnostics.missing_file_loader_frames == 1 {
            warn!("runtime_mesh_submission_missing_file_loader");
        }
        return;
    }
    if animation_resources.timer.is_none() {
        animation_resources.diagnostics.missing_timer_frames += 1;
        if animation_resources.diagnostics.missing_timer_frames == 1 {
            warn!("runtime_mesh_submission_missing_timer");
        }
        return;
    }
    if !animation_resources.refresh.rebuild_this_frame {
        if let Some(file_loader) = file_loader.as_deref() {
            let now_ms = animation_resources
                .timer
                .as_ref()
                .map(|timer| timer.time_in_milliseconds())
                .unwrap_or(0);
            sync_persistent_runtime_root_transforms(
                file_loader,
                Some(&world_players),
                script_entities.as_ref(),
                &animation_resources.vehicle_animations,
                animation_resources.handling_manager.as_deref(),
                animation_resources.ped_animation_offsets.as_deref(),
                now_ms,
                &mut animation_resources.animated_registry,
                &mut commands,
            );
        }
        if let Some(script_entities) = script_entities.as_ref() {
            sync_script_runtime_clump_alphas(
                script_entities,
                &animation_resources.animated_registry,
                &mut materials,
            );
        }
        return;
    }
    for entities in animation_resources.reusable_runtime.values_mut() {
        entities.clear();
    }
    if !animation_resources.refresh.static_rebuild {
        for (entity, runtime, transform) in &animation_resources.existing_runtime {
            if matches!(
                runtime.source,
                GtaRuntimePoolMeshSource::ScriptVehicle
                    | GtaRuntimePoolMeshSource::ScriptObject
                    | GtaRuntimePoolMeshSource::SavePed
                    | GtaRuntimePoolMeshSource::SaveVehicle
                    | GtaRuntimePoolMeshSource::SaveObject
            ) {
                animation_resources
                    .reusable_runtime
                    .entry(runtime_pool_reuse_key(
                        runtime.source,
                        runtime.slot,
                        runtime.model_index,
                        runtime.render_damaged,
                    ))
                    .or_default()
                    .push((entity, *transform));
            }
        }
    }
    let Some(file_loader) = file_loader else {
        return;
    };
    let colour_table = vehicle_colour_table.as_ref().map(|t| t.as_ref());
    let animation_time_seconds = animation_resources
        .timer
        .as_ref()
        .map(|timer| timer.time_in_milliseconds() as f32 * 0.001)
        .unwrap_or(0.0);
    if let Some(table) = colour_table {
        ensure_colour_texture_table(&mut texture_cache, &mut images, table);
    }
    ensure_environment_maps(&file_loader, &mut texture_cache);
    let fog = timecycle
        .as_ref()
        .map(|tc| {
            let c = &tc.current;
            let sun = tc.vector_to_sun[tc.current_stored_value.rem_euclid(16) as usize]
                .to_bevy()
                .normalize_or_zero();
            GtaRenderLighting {
                fog_colour: srgb_i32_to_linear(c.fog_colour),
                fog_start: c.fog_start,
                far_clip: c.far_clip,
                ambient: c.ambient,
                directional: c.directional,
                sun_direction: if sun.length_squared() > 1.0e-6 {
                    sun.to_array()
                } else {
                    [0.0, 0.0, 1.0]
                },
                ambient_multiplier: 1.0,
                directional_enabled: true,
            }
        })
        .unwrap_or_default()
        .for_peds_cars_and_objects();
    let camera_gta = file_loader.renderer.camera_position;

    // Live player after CREATE_PLAYER (re3 CWorld::Players[] → render).
    // Hidden during cutscene when bridge says not visible.
    let player_visible = bridge
        .as_ref()
        .map(|b| b.player_spawned && b.player_visible)
        .unwrap_or(true);
    if player_visible {
        {
            for (slot, player) in world_players.iter_slots() {
                let Some(player) = player else {
                    continue;
                };
                if !player.active_in_world || !player.ped.physical.entity.flags.is_visible {
                    continue;
                }
                let model_index = {
                    let raw = player.ped.physical.entity.model_index;
                    if raw <= 0 { MI_PLAYER } else { raw as usize }
                };
                let active_vehicle_anim = world_players
                    .player_entity(slot as i32)
                    .and_then(|entity| animation_resources.vehicle_animations.get(entity).ok());
                let player_relation = world_players.relation(slot as i32);
                let seated_matrix = if active_vehicle_anim.is_none() {
                    player_relation
                        .and_then(|relation| {
                            script_entities.as_ref().and_then(|world| {
                                world
                                    .vehicle_handle_for_entity(relation.vehicle)
                                    .and_then(|handle| world.vehicle_get(handle))
                                    .map(|vehicle| (vehicle, relation.seat))
                            })
                        })
                        .and_then(|(vehicle, seat)| match vehicle {
                            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                                crate::peds::Ped::automobile_occupant_matrix(
                                    &file_loader,
                                    &car,
                                    seat,
                                )
                            }
                            _ => None,
                        })
                } else {
                    None
                };
                let line_up_matrix = if let Some(active) = active_vehicle_anim {
                    script_entities
                        .as_ref()
                        .and_then(|world| {
                            world
                                .vehicle_handle_for_entity(active.vehicle)
                                .and_then(|handle| world.vehicle_get(handle))
                        })
                        .and_then(|vehicle| match vehicle {
                            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                                let handling = car
                                    .vehicle.core.handling_id
                                    .and_then(|id| usize::try_from(id).ok())
                                    .and_then(|id| {
                                        animation_resources
                                            .handling_manager
                                            .as_ref()
                                            .and_then(|mgr| mgr.get_handling_data(id))
                                    })?;
                                let offsets =
                                    animation_resources.ped_animation_offsets.as_deref()?;
                                crate::peds::PedAI::automobile_ped_line_up_matrix(
                                    &file_loader,
                                    &car,
                                    handling,
                                    offsets,
                                    active.door,
                                    crate::peds::PedAI::PedVehicleLineUpContext {
                                        current_position: player.ped.physical.entity.get_position(),
                                        current_heading: player.locomotion.rotation_cur,
                                        offset_seek: player.ped.offset_seek,
                                        state_timer_ms: player.ped.ped_state_timer,
                                        now_ms: animation_resources
                                            .timer
                                            .as_ref()
                                            .map(|timer| timer.time_in_milliseconds())
                                            .unwrap_or(0),
                                    },
                                    active.phase,
                                    &active.association,
                                )
                            }
                            _ => None,
                        })
                } else {
                    None
                };
                let transform = entity_matrix_to_bevy_transform(
                    line_up_matrix
                        .as_ref()
                        .or(seated_matrix.as_ref())
                        .unwrap_or(&player.ped.physical.entity.matrix),
                );
                let poses = animation_resources.anim_manager.as_ref().map(|manager| {
                    if matches!(player.ped.ped_state,
                        crate::peds::Ped::PedState::PED_FALL | crate::peds::Ped::PedState::PED_GETUP
                        | crate::peds::Ped::PedState::PED_DIE | crate::peds::Ped::PedState::PED_DEAD) {
                        player_anim_clump_bone_poses(&player.animation.movement_anim_clump)
                    } else if let Some(animation_id) = player.ped.impact_animation_id {
                        let elapsed = (animation_time_seconds
                            - player.ped.impact_animation_start_ms as f32 * 0.001)
                            .max(0.0)
                            * player.ped.impact_animation_speed;
                        ped_ifp_bone_poses_for_one_shot(
                            manager,
                            crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                            animation_id,
                            elapsed,
                        )
                    } else if let Some(active) = active_vehicle_anim {
                        ped_bone_poses_from_association(&active.association)
                    } else if player_relation.is_some() {
                        player_anim_clump_bone_poses(&player.animation.movement_anim_clump)
                    } else if player.ped.flags.is_landing
                        || player.ped.ped_state == crate::peds::Ped::PedState::PED_JUMP
                        || player.ped.flags.is_in_the_air
                    {
                        let mut poses = player_anim_clump_bone_poses_with_overlay(
                            &player.animation.movement_anim_clump,
                            player.jump_association.as_ref(),
                        );
                        apply_player_mouse_diagonal_leg_pose(
                            &mut poses,
                            &player.animation.movement_anim_clump,
                            player.mouse_leg_turn_angle,
                        );
                        poses
                    } else {
                        let mut poses =
                            player_anim_clump_bone_poses(&player.animation.movement_anim_clump);
                        apply_player_mouse_diagonal_leg_pose(
                            &mut poses,
                            &player.animation.movement_anim_clump,
                            player.mouse_leg_turn_angle,
                        );
                        poses
                    }
                });
                if poses.as_ref().is_some_and(|poses| !poses.is_empty()) {
                    let Some(poses) = poses.as_ref() else {
                        continue;
                    };
                    spawn_animated_ped_pool_mesh(
                        GtaRuntimePoolMeshSource::WorldPlayer,
                        slot,
                        model_index,
                        transform,
                        &poses,
                        // player_anim_clump_bone_poses already performs the
                        // association-aware 2D root extraction. Re-extracting
                        // here would also erase the authored landing/collapse Z.
                        false,
                        None,
                        false,
                        None,
                        None,
                        &file_loader,
                        &mut texture_cache,
                        &mut images,
                        &mut animation_resources.animated_registry,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut world_stats,
                        camera_gta,
                        fog,
                    );
                    if player_relation.is_none()
                        && let Some(info) = animation_resources
                            .weapon_info
                            .as_ref()
                            .and_then(|store| store.GetWeaponInfo(player.ped.current_weapon))
                        && info.model_id >= 0
                        && crate::weapons::WeaponRuntime::hand_weapon_visible(
                            &player.animation.movement_anim_clump,
                            info,
                        )
                        && let Some(hand) =
                            ped_frame_transform(model_index, &poses, "Rhand", &file_loader)
                    {
                        spawn_animated_ped_pool_mesh(
                            GtaRuntimePoolMeshSource::WorldPlayerWeapon,
                            slot,
                            info.model_id as usize,
                            transform.mul_transform(hand),
                            &[],
                            false,
                            None,
                            false,
                            None,
                            None,
                            &file_loader,
                            &mut texture_cache,
                            &mut images,
                            &mut animation_resources.animated_registry,
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &mut world_stats,
                            camera_gta,
                            fog,
                        );
                    }
                } else {
                    spawn_animated_ped_pool_mesh(
                        GtaRuntimePoolMeshSource::WorldPlayer,
                        slot,
                        model_index,
                        transform,
                        &[],
                        true,
                        None,
                        false,
                        None,
                        None,
                        &file_loader,
                        &mut texture_cache,
                        &mut images,
                        &mut animation_resources.animated_registry,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut world_stats,
                        camera_gta,
                        fog,
                    );
                }
            }
        }
    }

    // re3 keeps a real CProjectile CObject next to each CProjectileInfo slot.
    // Mirror that world object through the existing persistent GTA model/TXD
    // render path so grenade/molotov/rocket geometry follows the projectile
    // without allocating fresh meshes every display frame.
    if let Some(projectiles) = animation_resources.projectiles.as_ref() {
        use crate::modelinfo::ModelIndices::{MI_GRENADE, MI_MISSILE, MI_MOLOTOV};
        use crate::weapons::WeaponType::eWeaponType;

        for (slot, projectile) in projectiles.info.iter().enumerate() {
            if !projectile.in_use {
                continue;
            }
            let model_index = match projectile.weapon_type {
                eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => MI_MISSILE,
                eWeaponType::WEAPONTYPE_MOLOTOV => MI_MOLOTOV,
                eWeaponType::WEAPONTYPE_GRENADE => MI_GRENADE,
                _ => continue,
            };
            spawn_animated_ped_pool_mesh(
                GtaRuntimePoolMeshSource::Projectile,
                slot,
                model_index,
                Transform::from_translation(projectile.pos.to_bevy()),
                &[],
                false,
                None,
                false,
                None,
                None,
                &file_loader,
                &mut texture_cache,
                &mut images,
                &mut animation_resources.animated_registry,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_stats,
                camera_gta,
                fog,
            );
        }
    }

    // Cutscene objects while cutscene loaded/running (re3 world-added cutscene objs).
    if let Some(cutscene) = cutscene.as_ref() {
        if cutscene.loaded || cutscene.running {
            for (slot, object) in cutscene.cutscene_objects.iter().enumerate() {
                // re3: only objects that have been CWorld::Add'd render. Animated
                // actors enter the world in SetupCutsceneToStart; the head is added
                // at CREATE_CUTSCENE_HEAD; non-animated slots never appear.
                if !object.in_world {
                    continue;
                }
                if object.model_id < 0 {
                    continue;
                }
                let transform = cutscene_object_render_transform(
                    object,
                    &cutscene.cutscene_objects,
                    &file_loader,
                );
                let hide_original_head = !object.is_head
                    && cutscene.cutscene_objects.iter().any(|candidate| {
                        candidate.is_head && candidate.parent_object_slot == Some(slot)
                    });
                // IFP drives ordinary cutscene clumps. A cutscene head instead
                // feeds every ANM node directly to its RpHAnim skin hierarchy.
                if !object.bone_poses.is_empty() || object.head_animation_pose.is_some() {
                    spawn_animated_ped_pool_mesh(
                        GtaRuntimePoolMeshSource::CutsceneObject,
                        slot,
                        object.model_id as usize,
                        transform,
                        &object.bone_poses,
                        !object.is_head,
                        object.head_animation_pose.as_ref(),
                        hide_original_head,
                        Some(object.optional_components),
                        object.model_snapshot.as_ref(),
                        &file_loader,
                        &mut texture_cache,
                        &mut images,
                        &mut animation_resources.animated_registry,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut world_stats,
                        camera_gta,
                        fog,
                    );
                } else {
                    spawn_animated_ped_pool_mesh(
                        GtaRuntimePoolMeshSource::CutsceneObject,
                        slot,
                        object.model_id as usize,
                        transform,
                        &[],
                        !object.is_head,
                        object.head_animation_pose.as_ref(),
                        hide_original_head,
                        Some(object.optional_components),
                        object.model_snapshot.as_ref(),
                        &file_loader,
                        &mut texture_cache,
                        &mut images,
                        &mut animation_resources.animated_registry,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &mut world_stats,
                        camera_gta,
                        fog,
                    );
                }
            }
        }
    }

    if !animation_resources.refresh.rebuild_this_frame {
        return;
    }

    if let Some(script_entities) = script_entities.as_ref() {
        for (handle, entity) in script_entities.peds() {
            // CPools handles, not HashMap iteration order, are the stable
            // RenderWare clump identity. Reindexing surviving peds after one
            // deletion left the previous slot's mesh alive and drew a clone.
            let slot = handle as u32 as usize;
            let model_index = entity.model_id.max(0) as usize;
            let active_vehicle_anim = script_entities
                .ped_entity(handle)
                .and_then(|ped_entity| animation_resources.vehicle_animations.get(ped_entity).ok());
            // re3 `SetPedPositionInCar` owns the full ped matrix while seated.
            // Keep exit/close-exit on the authored animation root instead of
            // snapping it back to the seat during the final door movement.
            let exit_animation_active = active_vehicle_anim.is_some_and(|active| {
                    matches!(
                        active.phase,
                        crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::Exit
                            | crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::CloseExit
                            | crate::animation::PedVehicleAnimation::PedVehicleAnimationPhase::DraggedOut
                    )
                });
            let relation = script_entities.relation_for_char(handle);
            let seated_matrix = (!exit_animation_active)
                .then_some(relation)
                .flatten()
                .and_then(|relation| {
                    script_entities
                        .vehicle_handle_for_entity(relation.vehicle)
                        .and_then(|vehicle_handle| script_entities.vehicle_get(vehicle_handle))
                        .and_then(|vehicle| match vehicle {
                            crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                                crate::peds::Ped::automobile_occupant_matrix(
                                    &file_loader,
                                    &car,
                                    relation.seat,
                                )
                            }
                            _ => None,
                        })
                });
            let transform = entity_matrix_to_bevy_transform(
                seated_matrix
                    .as_ref()
                    .unwrap_or(&entity.ped.physical.entity.matrix),
            );
            let poses = animation_resources
                .anim_manager
                .as_ref()
                .and_then(|manager| {
                    use crate::animation::AnimationId::{ANIM_STD_EVADE_DIVE, ANIM_STD_EVADE_STEP,
                        ANIM_STD_HANDSUP, ANIM_STD_HAILTAXI, ANIM_STD_GET_UP, ANIM_STD_GET_UP_FRONT};
                    if entity.ped.dying_or_dead() {
                        return Some(player_anim_clump_bone_poses(&entity.animation.movement_anim_clump));
                    }
                    if entity.animation.movement_anim_clump.associations.iter().any(|association|
                        matches!(association.anim_id, ANIM_STD_EVADE_DIVE | ANIM_STD_EVADE_STEP |
                            ANIM_STD_HANDSUP | ANIM_STD_HAILTAXI | ANIM_STD_GET_UP | ANIM_STD_GET_UP_FRONT)
                            && association.blend_amount > 0.0) {
                        return Some(player_anim_clump_bone_poses(&entity.animation.movement_anim_clump));
                    }
                    entity.ped.impact_animation_id.map(|animation_id| {
                        let elapsed = (animation_time_seconds
                            - entity.ped.impact_animation_start_ms as f32 * 0.001)
                            .max(0.0)
                            * entity.ped.impact_animation_speed;
                        ped_ifp_bone_poses_for_one_shot(
                            manager,
                            crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                            animation_id,
                            elapsed,
                        )
                    })
                })
                .or_else(|| {
                    let animation_id = entity.ped.weapon_attack_animation_id?;
                    let association = entity
                        .animation
                        .movement_anim_clump
                        .associations
                        .iter()
                        .find(|association| association.anim_id == animation_id)?;
                    Some(ped_bone_poses_from_association(association))
                })
                .or_else(|| {
                    active_vehicle_anim
                        .map(|active| ped_bone_poses_from_association(&active.association))
                })
                .or_else(|| {
                    if relation.is_some() {
                        Some(player_anim_clump_bone_poses(&entity.animation.movement_anim_clump))
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    if entity.ped.stored_move_state != entity.ped.move_state {
                        return None;
                    }
                    let poses = player_anim_clump_bone_poses(&entity.animation.movement_anim_clump);
                    (!poses.is_empty()).then_some(poses)
                })
                .or_else(|| {
                    animation_resources.anim_manager.as_ref().map(|manager| {
                        let group = file_loader
                            .model_info
                            .get_model_info(model_index)
                            .and_then(|model| match model {
                                crate::modelinfo::ModelInfo::ModelInfoSlot::Ped(ped) => {
                                    manager.assoc_group_id_by_name(&ped.anim_group)
                                }
                                _ => None,
                            })
                            .unwrap_or(crate::animation::AnimAssocDefinitions::ASSOCGRP_STD);
                        ped_ifp_bone_poses(
                            manager,
                            group,
                            entity.ped.move_state,
                            animation_time_seconds,
                        )
                    })
                });
            if let Some(poses) = poses.filter(|poses| !poses.is_empty()) {
                spawn_animated_ped_pool_mesh(
                    GtaRuntimePoolMeshSource::ScriptPed,
                    slot,
                    model_index,
                    transform,
                    &poses,
                    // The pose builders above already mirror re3's
                    // association-aware root extraction: only associations
                    // carrying ASSOC_HAS_TRANSLATION move root X/Y onto the
                    // entity, while authored Z remains on the clump. Vehicle
                    // enter/exit/carjack associations do not carry that flag,
                    // so stripping sequence-zero translation here made dragged
                    // peds float at the physical door root instead of following
                    // the authored JACKEDCAR root motion.
                    false,
                    None,
                    false,
                    None,
                    None,
                    &file_loader,
                    &mut texture_cache,
                    &mut images,
                    &mut animation_resources.animated_registry,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut world_stats,
                    camera_gta,
                    fog,
                );
            } else {
                spawn_animated_ped_pool_mesh(
                    GtaRuntimePoolMeshSource::ScriptPed,
                    slot,
                    model_index,
                    transform,
                    &[],
                    false,
                    None,
                    false,
                    None,
                    None,
                    &file_loader,
                    &mut texture_cache,
                    &mut images,
                    &mut animation_resources.animated_registry,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut world_stats,
                    camera_gta,
                    fog,
                );
            }
        }
        for (handle, entity) in script_entities.vehicles() {
            let slot = handle as u32 as usize;
            let base = match entity {
                crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => car.vehicle,
                crate::control::ScriptEntities::ScriptVehicleRef::Boat { boat, .. } => boat.vehicle,
                crate::control::ScriptEntities::ScriptVehicleRef::Heli { heli, .. } => heli.vehicle,
                crate::control::ScriptEntities::ScriptVehicleRef::Plane { plane, .. } => plane.vehicle,
            };
            if !base.physical.entity.flags.is_visible {
                continue;
            }
            let model_index = base.physical.entity.model_index.max(0) as usize;
            let transform = entity_matrix_to_bevy_transform(&base.physical.entity.matrix);
            let vehicle_source = GtaRuntimePoolMeshSource::ScriptVehicleDynamic;
            if reuse_runtime_pool_mesh(
                &*animation_resources.reusable_runtime,
                vehicle_source,
                slot,
                model_index,
                false,
                transform,
                &mut commands,
                &mut world_stats,
            ) {
                continue;
            }
            let mut remap = vehicle_colour_remap_from_indices(
                base.appearance.current_colour1,
                base.appearance.current_colour2,
                colour_table,
            );
            if let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = entity {
                remap = remap.with_automobile_render_state(&car);
            } else if let crate::control::ScriptEntities::ScriptVehicleRef::Heli { heli, .. } = entity {
                remap.heli_rotor_rotation = Some(heli.rotor_rotation);
            }
            if vehicle_source == GtaRuntimePoolMeshSource::ScriptVehicleDynamic {
                spawn_or_update_dynamic_runtime_mesh(
                    vehicle_source,
                    slot,
                    model_index,
                    transform,
                    &file_loader,
                    &mut texture_cache,
                    &mut images,
                    remap,
                    &mut animation_resources.animated_registry,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut world_stats,
                    fog,
                );
            } else {
                spawn_runtime_pool_mesh(
                    vehicle_source,
                    slot,
                    Some(model_index),
                    transform,
                    false,
                    &file_loader,
                    &mut texture_cache,
                    &mut images,
                    remap,
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &mut world_stats,
                    camera_gta,
                    fog,
                );
            }
        }
        for entity in script_entities.objects() {
            let flags = &entity.object.physical.entity.flags;
            if !flags.is_visible || flags.do_not_render {
                continue;
            }
            if !file_loader.renderer.disable_zone_cull {
                let surface_culled = flags.is_static
                    && entity
                        .object
                        .current_surface
                        .and_then(|surface| file_loader.world.entities.get(surface))
                        .is_some_and(|surface| surface.zone_culled2);
                if flags.zone_culled || surface_culled {
                    continue;
                }
            }
            let slot = entity.handle as u32 as usize;
            let model_index = entity.object.physical.entity.model_index.max(0) as usize;
            let transform = entity_matrix_to_bevy_transform(&entity.object.physical.entity.matrix);
            let render_damaged = entity.object.physical.entity.flags.render_damaged;
            if reuse_runtime_pool_mesh(
                &*animation_resources.reusable_runtime,
                GtaRuntimePoolMeshSource::ScriptObject,
                slot,
                model_index,
                render_damaged,
                transform,
                &mut commands,
                &mut world_stats,
            ) {
                continue;
            }
            let object_remap = if entity.object.flags.use_vehicle_colours {
                vehicle_colour_remap_from_indices(
                    entity.object.colour1.max(0) as u8,
                    entity.object.colour2.max(0) as u8,
                    colour_table,
                )
            } else {
                VehicleColourRemap::default()
            };
            spawn_runtime_pool_mesh(
                GtaRuntimePoolMeshSource::ScriptObject,
                slot,
                Some(model_index),
                transform,
                render_damaged,
                &file_loader,
                &mut texture_cache,
                &mut images,
                object_remap,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_stats,
                camera_gta,
                fog,
            );
        }
    }

    if let Some(pools) = pools {
        for (slot, record) in pools.compatible_ped_records() {
            let model_index = compatible_model_index(record.model_index);
            let transform = ped_record_transform(record);
            if model_index.is_some_and(|model| {
                reuse_runtime_pool_mesh(
                    &*animation_resources.reusable_runtime,
                    GtaRuntimePoolMeshSource::SavePed,
                    slot,
                    model,
                    false,
                    transform,
                    &mut commands,
                    &mut world_stats,
                )
            }) {
                continue;
            }
            spawn_runtime_pool_mesh(
                GtaRuntimePoolMeshSource::SavePed,
                slot,
                model_index,
                transform,
                false,
                &file_loader,
                &mut texture_cache,
                &mut images,
                VehicleColourRemap::default(),
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_stats,
                camera_gta,
                fog,
            );
        }
        for (slot, record) in pools.compatible_vehicle_records() {
            let model_index = compatible_model_index(record.model_index);
            let transform = vehicle_record_transform(record);
            if model_index.is_some_and(|model| {
                reuse_runtime_pool_mesh(
                    &*animation_resources.reusable_runtime,
                    GtaRuntimePoolMeshSource::SaveVehicle,
                    slot,
                    model,
                    false,
                    transform,
                    &mut commands,
                    &mut world_stats,
                )
            }) {
                continue;
            }
            let remap = vehicle_colour_remap_from_indices(
                record.base.current_colour1,
                record.base.current_colour2,
                colour_table,
            );
            spawn_runtime_pool_mesh(
                GtaRuntimePoolMeshSource::SaveVehicle,
                slot,
                model_index,
                transform,
                false,
                &file_loader,
                &mut texture_cache,
                &mut images,
                remap,
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_stats,
                camera_gta,
                fog,
            );
        }
        for (slot, record) in pools.compatible_object_records() {
            let model_index = compatible_model_index(record.model_index);
            let transform = object_record_transform(record);
            let render_damaged = record.entity_flags.flags.render_damaged;
            if model_index.is_some_and(|model| {
                reuse_runtime_pool_mesh(
                    &*animation_resources.reusable_runtime,
                    GtaRuntimePoolMeshSource::SaveObject,
                    slot,
                    model,
                    render_damaged,
                    transform,
                    &mut commands,
                    &mut world_stats,
                )
            }) {
                continue;
            }
            spawn_runtime_pool_mesh(
                GtaRuntimePoolMeshSource::SaveObject,
                slot,
                model_index,
                transform,
                render_damaged,
                &file_loader,
                &mut texture_cache,
                &mut images,
                VehicleColourRemap::default(),
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut world_stats,
                camera_gta,
                fog,
            );
        }
    }

    for vehicle in &replay.vehicles {
        spawn_replay_vehicle_mesh(
            ReplayVehiclePoolOverlay {
                replay_index: vehicle.replay_index,
                model_index: vehicle.model_index,
                update: vehicle.update,
            },
            &file_loader,
            &mut texture_cache,
            &mut images,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut world_stats,
            camera_gta,
            fog,
            colour_table,
        );
    }
    for ped in &replay.peds {
        spawn_replay_ped_mesh(
            ReplayPedPoolOverlay {
                replay_index: ped.replay_index,
                header: ped.header,
                update: ped.update,
            },
            &file_loader,
            &mut texture_cache,
            &mut images,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut world_stats,
            camera_gta,
            fog,
        );
    }
    if let Some(script_entities) = script_entities.as_ref() {
        sync_script_runtime_clump_alphas(
            script_entities,
            &animation_resources.animated_registry,
            &mut materials,
        );
    }
}


pub(super) fn runtime_mesh_layout_signature(drawables: &[ModelMaterialDrawable]) -> Vec<(usize, usize)> {
    drawables
        .iter()
        .map(|drawable| {
            (
                drawable.mesh.count_vertices(),
                drawable.mesh.indices().map(Indices::len).unwrap_or(0),
            )
        })
        .collect()
}

pub(super) fn runtime_mesh_handles_match_layout(
    meshes: &Assets<Mesh>,
    handles: &[Handle<Mesh>],
    layout: &[(usize, usize)],
) -> bool {
    handles.len() == layout.len()
        && handles
            .iter()
            .zip(layout)
            .all(|(handle, (vertices, indices))| {
                meshes.get(handle).is_some_and(|mesh| {
                    mesh.count_vertices() == *vertices
                        && mesh.indices().map(Indices::len) == Some(*indices)
                })
            })
}


pub(super) fn reuse_runtime_pool_mesh(
    reusable: &BTreeMap<RuntimePoolReuseKey, Vec<(Entity, Transform)>>,
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
    render_damaged: bool,
    transform: Transform,
    commands: &mut Commands,

    stats: &mut GtaWorldRenderStats,
) -> bool {
    let Some(entities) = reusable.get(&runtime_pool_reuse_key(
        source,
        slot,
        model_index,
        render_damaged,
    )) else {
        return false;
    };
    for (entity, current_transform) in entities {
        // CEntity keeps its RwFrame until its matrix changes. Re-inserting an
        // identical Transform marks hundreds of static script-object meshes as
        // changed and forces Bevy render extraction work re3 never performs.
        if *current_transform != transform {
            commands.entity(*entity).insert(transform);
        }
    }
    stats.spawned_pool_meshes += entities.len();
    true
}

pub fn vehicle_component_relative_transform(baseline: Transform, current: Transform) -> Transform {
    Transform::from_matrix(Mat4::from(
        current.compute_affine() * baseline.compute_affine().inverse(),
    ))
}

/// Keep a moving script vehicle's RenderWare atomics resident. This mirrors
/// re3 `CAutomobile::Render`: normal driving updates only wheel/door/rotor
/// frame matrices. Geometry and materials are touched only when their resident
/// damage/colour state actually changes.
pub(super) fn spawn_or_update_dynamic_runtime_mesh(
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
    transform: Transform,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    vehicle_colours: VehicleColourRemap,
    registry: &mut GtaAnimatedRuntimeMeshRegistry,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    stats: &mut GtaWorldRenderStats,
    lighting: GtaRenderLighting,
) {
    let key = animated_runtime_mesh_key(source, slot, model_index);
    let geometry_signature = runtime_model_geometry_signature(file_loader, model_index, None, None);
    registry.touched.insert(key);
    let distance = world_distance(
        CVector::from_bevy(transform.translation),
        file_loader.renderer.camera_position,
    );
    let additive = file_loader
        .model_info
        .simple_model_info(model_index)
        .is_some_and(|simple| simple.flags.additive);
    let resident_state = vehicle_colours.resident_mesh_state();

    let can_update_transforms = registry.slots.get(&key).is_some_and(|cached| {
        cached.geometry_signature == geometry_signature
            && cached.rigid_frame_indices.is_none()
            && !cached.hide_original_head
            && cached.vehicle_resident_state == Some(resident_state)
            && cached.entities.len() == cached.meshes.len()
            && cached.entities.len() == cached.materials.len()
            && cached.entities.len() == cached.rendered_transforms.len()
            && cached
                .meshes
                .iter()
                .all(|handle| meshes.get(handle).is_some())
            && cached
                .materials
                .iter()
                .all(|handle| materials.get(handle).is_some())
    });
    if can_update_transforms
        && let Some(component_transforms) =
            build_vehicle_component_transforms(model_index, distance, file_loader, vehicle_colours)
        && let Some(cached) = registry.slots.get_mut(&key)
        && let (Some(dynamic_parts), Some(baselines)) = (
            cached.dynamic_vehicle_parts.as_ref(),
            cached.dynamic_vehicle_baselines.as_ref(),
        )
        && dynamic_parts.len() == component_transforms.len()
        && dynamic_parts
            .iter()
            .zip(&component_transforms)
            .all(|(part, (current_part, _))| *part == *current_part)
    {
        if !cached.is_visible {
            cached.is_visible = true;
            for entity in &cached.entities {
                commands.entity(*entity).insert(Visibility::Visible);
            }
        }
        stats.textured_meshes += cached
            .materials
            .iter()
            .filter(|handle| {
                materials
                    .get(*handle)
                    .is_some_and(|material| material.base_color_texture.is_some())
            })
            .count();
        let mut dynamic_cursor = 0usize;
        for part in 0..cached.entities.len() {
            let desired = if dynamic_parts.get(dynamic_cursor).copied() == Some(part) {
                let current = component_transforms[dynamic_cursor].1;
                dynamic_cursor += 1;
                baselines
                    .get(part)
                    .copied()
                    .flatten()
                    .zip(current)
                    .map(|(baseline, current)| {
                        transform
                            .mul_transform(vehicle_component_relative_transform(baseline, current))
                    })
                    .unwrap_or(transform)
            } else {
                transform
            };
            if cached.rendered_transforms[part] != desired {
                commands.entity(cached.entities[part]).insert(desired);
                cached.rendered_transforms[part] = desired;
            }
        }
        cached.rendered_root_transform = transform;
        stats.spawned_pool_meshes += cached.entities.len();
        return;
    }

    let drawables =
        build_model_material_drawables(model_index, distance, false, file_loader, vehicle_colours);

    if drawables.is_empty() {
        // `CAutomobile::Render` never deletes the vehicle's `m_rwObject`
        // because a fresh lookup of the source model failed; that object stays
        // alive until `CEntity::DeleteRwObject`. Preserve an already resident
        // Bevy clone here as well. With the model-ref mirror above this branch
        // should only be reachable during an exceptional streaming transition,
        // but it must not turn that transition into a visibly missing car.
        if let Some(cached) = registry.slots.get_mut(&key) {
            if !cached.is_visible {
                cached.is_visible = true;
                for entity in &cached.entities {
                    commands.entity(*entity).insert(Visibility::Visible);
                }
            }
            stats.spawned_pool_meshes += cached.entities.len();
            return;
        }
        if file_loader.model_info.get_model_info(model_index).is_some() {
            stats.skipped_missing_geometry += 1;
        } else {
            stats.skipped_missing_model += 1;
        }
        return;
    }

    let mesh_layout_signature = runtime_mesh_layout_signature(&drawables);

    let can_update = registry.slots.get(&key).is_some_and(|cached| {
        cached.geometry_signature == geometry_signature
            && cached.rigid_frame_indices.is_none()
            && !cached.hide_original_head
            && cached.entities.len() == drawables.len()
            && cached.mesh_layout_signature == mesh_layout_signature
            && runtime_mesh_handles_match_layout(meshes, &cached.meshes, &mesh_layout_signature)
            && cached
                .meshes
                .iter()
                .all(|handle| meshes.get(handle).is_some())
            && cached
                .materials
                .iter()
                .all(|handle| materials.get(handle).is_some())
    });
    if can_update {
        // Existing runtime models can refresh their materials (paint, damage);
        // guarantee their dictionary is decoded before rebinding textures.
        sync_model_texture_cache(model_index, file_loader, texture_cache, images);
        let dynamic_vehicle_parts = drawables
            .iter()
            .enumerate()
            .filter_map(|(part, drawable)| drawable.dynamic_vehicle_component.then_some(part))
            .collect::<Vec<_>>();
        let dynamic_vehicle_baselines = drawables
            .iter()
            .map(|drawable| drawable.vehicle_component_transform)
            .collect::<Vec<_>>();
        let cached = registry
            .slots
            .get_mut(&key)
            .expect("dynamic vehicle slot checked above");
        if !cached.is_visible {
            cached.is_visible = true;
            for entity in &cached.entities {
                commands.entity(*entity).insert(Visibility::Visible);
            }
        }
        for (part, drawable) in drawables.into_iter().enumerate() {
            let named_texture = drawable.texture_name.is_some();
            let refreshed_material = resolve_drawable_material_bind(
                &drawable,
                file_loader,
                texture_cache,
                images,
                vehicle_colours,
            );
            // CAutomobile::Render keeps resident geometry. Even a same-handle
            // write causes Bevy GPU reallocation, so compare before mutation.
            super::reuse_persistent_mesh_shape(meshes, &cached.meshes[part], &drawable.mesh);
            let (base_color_texture, texture_alpha, base_color) = refreshed_material;
            if base_color_texture.is_some() {
                stats.textured_meshes += 1;
            } else if named_texture {
                stats.skipped_missing_texture += 1;
            }
            {
                let refreshed = gta_world_material(
                    base_color,
                    base_color_texture,
                    texture_alpha,
                    additive,
                    drawable.surface_ambient,
                    drawable.surface_diffuse,
                    drawable.has_prelit,
                    lighting,
                );
                if let Some(base_alpha) = cached.material_base_alphas.get_mut(part) {
                    *base_alpha = refreshed.uniforms.colour.w;
                }
                if let Some(base_mode) = cached.material_base_alpha_modes.get_mut(part) {
                    *base_mode = refreshed.alpha_mode.clone();
                }
                if materials.get(&cached.materials[part]) != Some(&refreshed)
                    && let Some(mut material) = materials.get_mut(&cached.materials[part])
                {
                    *material = refreshed;
                }
            }
            if cached.rendered_transforms.get(part).copied() != Some(transform) {
                commands.entity(cached.entities[part]).insert(transform);
            }
            stats.spawned_pool_meshes += 1;
        }
        cached.dynamic_vehicle_parts = Some(dynamic_vehicle_parts);
        cached.dynamic_vehicle_baselines = Some(dynamic_vehicle_baselines);
        cached.rendered_transforms.fill(transform);
        cached.rendered_root_transform = transform;
        cached.vehicle_resident_state = Some(resident_state);
        cached.mesh_layout_signature = mesh_layout_signature;
        return;
    }

    if let Some(old) = registry.slots.remove(&key) {
        for entity in old.entities {
            commands.entity(entity).despawn();
        }
        for mesh in old.meshes {
            meshes.remove(mesh.id());
        }
        for material in old.materials {
            materials.remove(material.id());
        }
    }

    let mut cached = GtaAnimatedRuntimeMeshSlot {
        entities: Vec::with_capacity(drawables.len()),
        meshes: Vec::with_capacity(drawables.len()),
        materials: Vec::with_capacity(drawables.len()),
        material_base_alphas: Vec::with_capacity(drawables.len()),
        material_base_alpha_modes: Vec::with_capacity(drawables.len()),
        dynamic_vehicle_parts: Some(
            drawables
                .iter()
                .enumerate()
                .filter_map(|(part, drawable)| drawable.dynamic_vehicle_component.then_some(part))
                .collect(),
        ),
        dynamic_vehicle_baselines: Some(
            drawables
                .iter()
                .map(|drawable| drawable.vehicle_component_transform)
                .collect(),
        ),
        rendered_transforms: Vec::with_capacity(drawables.len()),
        rendered_root_transform: transform,
        vehicle_resident_state: Some(resident_state),
        rigid_frame_indices: None,
        hide_original_head: false,
        geometry_signature,
        mesh_layout_signature,
        is_visible: true,
    };
    // Runtime models (cars/peds) decode their own dictionary on demand so they
    // never depend on the budgeted world texture sync having reached this frame.
    sync_model_texture_cache(model_index, file_loader, texture_cache, images);
    for drawable in drawables {
        let (base_color_texture, texture_alpha, base_color) = resolve_drawable_material_bind(
            &drawable,
            file_loader,
            texture_cache,
            images,
            vehicle_colours,
        );
        if base_color_texture.is_some() {
            stats.textured_meshes += 1;
        } else if drawable.texture_name.is_some() {
            stats.skipped_missing_texture += 1;
        }
        let mesh = meshes.add(drawable.mesh);
        let material_asset = gta_world_material(
            base_color,
            base_color_texture,
            texture_alpha,
            additive,
            drawable.surface_ambient,
            drawable.surface_diffuse,
            drawable.has_prelit,
            lighting,
        );
        cached
            .material_base_alphas
            .push(material_asset.uniforms.colour.w);
        cached
            .material_base_alpha_modes
            .push(material_asset.alpha_mode.clone());
        let material = materials.add(material_asset);
        let entity = commands
            .spawn((
                GtaImmediateMesh,
                GtaRuntimePoolMesh {
                    source,
                    slot,

                    model_index,
                    render_damaged: false,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                // Vehicle/ped visibility is decided by the GTA render path.
                // Their RenderWare frames are rebuilt independently of Bevy's
                // transform propagation, so a second mesh-AABB frustum test
                // can drop a nearby car for one frame (especially while a
                // door/wheel frame changes).
                NoFrustumCulling,
                Visibility::Visible,
                transform,
                GlobalTransform::default(),
            ))
            .id();
        cached.entities.push(entity);
        cached.meshes.push(mesh);
        cached.materials.push(material);
        cached.rendered_transforms.push(transform);
        stats.spawned_pool_meshes += 1;
    }
    registry.slots.insert(key, cached);
}

pub(super) fn spawn_runtime_pool_mesh(
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: Option<usize>,
    transform: Transform,
    render_damaged: bool,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    vehicle_colours: VehicleColourRemap,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    stats: &mut GtaWorldRenderStats,
    camera_gta: CVector,
    lighting: GtaRenderLighting,
) {
    let Some(model_index) = model_index else {
        stats.skipped_missing_model += 1;
        return;
    };
    let runtime_position = CVector::from_bevy(transform.translation);
    let distance = world_distance(runtime_position, camera_gta);
    let additive = file_loader
        .model_info
        .simple_model_info(model_index)
        .is_some_and(|s| s.flags.additive);
    let drawables = build_model_material_drawables(
        model_index,
        distance,
        render_damaged,
        file_loader,
        vehicle_colours,
    );
    if drawables.is_empty() {
        if file_loader.model_info.get_model_info(model_index).is_some() {
            stats.skipped_missing_geometry += 1;
        } else {
            stats.skipped_missing_model += 1;
        }
        return;
    }
    // Runtime models (cars/peds) decode their own dictionary on demand so they
    // never depend on the budgeted world texture sync having reached this frame.
    sync_model_texture_cache(model_index, file_loader, texture_cache, images);
    for drawable in drawables {
        match source {
            GtaRuntimePoolMeshSource::ReplayPed | GtaRuntimePoolMeshSource::ReplayVehicle => {
                stats.spawned_replay_meshes += 1
            }
            _ => stats.spawned_pool_meshes += 1,
        }
        let (base_color_texture, texture_alpha, base_color) = resolve_drawable_material_bind(
            &drawable,
            file_loader,
            texture_cache,
            images,
            vehicle_colours,
        );
        if base_color_texture.is_some() {
            stats.textured_meshes += 1;
        } else if drawable.texture_name.is_some() {
            stats.skipped_missing_texture += 1;
        }
        let _ = (drawable.specular, texture_cache.env_map.as_ref());
        commands.spawn((
            GtaImmediateMesh,
            GtaRuntimePoolMesh {
                source,
                slot,
                model_index,
                render_damaged,
            },
            Mesh3d(meshes.add(drawable.mesh)),
            MeshMaterial3d(materials.add(gta_world_material(
                base_color,
                base_color_texture,
                texture_alpha,
                additive,
                drawable.surface_ambient,
                drawable.surface_diffuse,
                drawable.has_prelit,
                lighting,
            ))),
            NoFrustumCulling,
            Visibility::Visible,
            transform,
            GlobalTransform::default(),
        ));
    }
}

pub(super) fn spawn_replay_vehicle_mesh(
    vehicle: ReplayVehiclePoolOverlay,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    stats: &mut GtaWorldRenderStats,
    camera_gta: CVector,
    lighting: GtaRenderLighting,
    colour_table: Option<&crate::modelinfo::VehicleModelInfo::CVehicleColours>,
) {
    // Replay packets don't always carry colours — fall back to first carcols pair / 0,0.
    let remap = vehicle_colour_remap_from_indices(0, 0, colour_table);
    spawn_runtime_pool_mesh(
        GtaRuntimePoolMeshSource::ReplayVehicle,
        vehicle.replay_index as usize,
        Some(vehicle.model_index as usize),
        replay_matrix_transform(vehicle.update.matrix),
        false,
        file_loader,
        texture_cache,
        images,
        remap,
        commands,
        meshes,
        materials,
        stats,
        camera_gta,
        lighting,
    );
}

pub(super) fn spawn_replay_ped_mesh(
    ped: ReplayPedPoolOverlay,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    stats: &mut GtaWorldRenderStats,
    camera_gta: CVector,
    lighting: GtaRenderLighting,
) {
    let model_index = ped.header.map(|header| header.model_index as usize);
    let transform = ped
        .update
        .map(|update| replay_matrix_transform(update.matrix))
        .unwrap_or_default();
    spawn_runtime_pool_mesh(
        GtaRuntimePoolMeshSource::ReplayPed,
        ped.replay_index as usize,
        model_index,
        transform,
        false,
        file_loader,
        texture_cache,
        images,
        VehicleColourRemap::default(),
        commands,
        meshes,
        materials,
        stats,
        camera_gta,
        lighting,
    );
}

/// Build per-material Bevy meshes from loaded DFF atomics / clump geometries.
pub(super) fn spawn_animated_ped_pool_mesh(
    source: GtaRuntimePoolMeshSource,
    slot: usize,
    model_index: usize,
    transform: Transform,
    bone_poses: &[crate::animation::CutsceneMgr::CutsceneBonePose],
    root_translation_on_entity: bool,
    head_animation: Option<&crate::animation::CutsceneMgr::CutsceneHeadAnimationPose>,
    hide_original_head: bool,
    optional_components: Option<[i8; 2]>,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
    file_loader: &CFileLoader,
    texture_cache: &mut GtaWorldTextureCache,
    images: &mut Assets<Image>,
    registry: &mut GtaAnimatedRuntimeMeshRegistry,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<GtaUnlitMaterial>>,
    stats: &mut GtaWorldRenderStats,
    _camera_gta: CVector,
    lighting: GtaRenderLighting,
) {
    let key = animated_runtime_mesh_key(source, slot, model_index);
    let geometry_signature = runtime_model_geometry_signature(
        file_loader,
        model_index,
        optional_components,
        model_snapshot,
    );
    registry.touched.insert(key);

    if let Some(cached) = registry.slots.get(&key)
        && cached.geometry_signature == geometry_signature
        && cached.hide_original_head == hide_original_head
        && let Some(frame_indices) = cached.rigid_frame_indices.as_deref()
        && let Some(part_transforms) = rigid_cutscene_part_transforms(
            model_index,
            file_loader,
            bone_poses,
            root_translation_on_entity,
            frame_indices,
            model_snapshot,
        )
        && part_transforms.len() == cached.entities.len()
    {
        let cached = registry
            .slots
            .get_mut(&key)
            .expect("rigid animated runtime slot checked above");
        if !cached.is_visible {
            cached.is_visible = true;
            for entity in &cached.entities {
                if let Ok(mut entity_commands) = commands.get_entity(*entity) {
                    entity_commands.insert(Visibility::Visible);
                }
            }
        }
        for (part, local) in part_transforms.into_iter().enumerate() {
            let desired = transform.mul_transform(local);
            if cached.rendered_transforms.get(part).copied() != Some(desired) {
                if let Ok(mut entity_commands) = commands.get_entity(cached.entities[part]) {
                    entity_commands.insert(desired);
                }
                cached.rendered_transforms[part] = desired;
            }
        }
        cached.rendered_root_transform = transform;
        stats.spawned_pool_meshes += cached.entities.len();
        return;
    }

    let rigid = head_animation.is_none()
        && !model_uses_cpu_skinning(model_index, file_loader, model_snapshot);
    let (drawables, rigid_frame_indices) = if rigid {
        let (drawables, frames) = build_rigid_cutscene_drawables(
            model_index,
            file_loader,
            hide_original_head,
            optional_components,
            model_snapshot,
        );
        (drawables, Some(frames))
    } else {
        (
            build_cutscene_drawables(
                model_index,
                file_loader,
                bone_poses,
                root_translation_on_entity,
                head_animation,
                hide_original_head,
                optional_components,
                model_snapshot,
            ),
            None,
        )
    };
    if drawables.is_empty() {
        stats.skipped_missing_geometry += 1;
        return;
    }
    let mesh_layout_signature = runtime_mesh_layout_signature(&drawables);
    if registry.slots.get(&key).is_some_and(|cached| {
        cached.geometry_signature == geometry_signature
            && cached.entities.len() == drawables.len()
            && cached.rigid_frame_indices.is_none()
            && rigid_frame_indices.is_none()
            && cached.hide_original_head == hide_original_head
            && cached.mesh_layout_signature == mesh_layout_signature
            && runtime_mesh_handles_match_layout(meshes, &cached.meshes, &mesh_layout_signature)
    }) {
        let cached = registry
            .slots
            .get_mut(&key)
            .expect("animated runtime slot checked above");
        if !cached.is_visible {
            cached.is_visible = true;
            for entity in &cached.entities {
                if let Ok(mut entity_commands) = commands.get_entity(*entity) {
                    entity_commands.insert(Visibility::Visible);
                }
            }
        }
        for (part, drawable) in drawables.into_iter().enumerate() {
            stats.spawned_pool_meshes += 1;
            // CPed::Render uploads changed skin poses; an unchanged pose must
            // not trigger extraction and GPU allocation of identical vertices.
            super::reuse_persistent_mesh_shape(meshes, &cached.meshes[part], &drawable.mesh);
            if cached.rendered_transforms.get(part).copied() != Some(transform)
                && let Ok(mut entity_commands) = commands.get_entity(cached.entities[part])
            {
                entity_commands.insert(transform);
            }
        }
        cached.rendered_transforms.fill(transform);
        cached.rendered_root_transform = transform;
        return;
    }

    if let Some(cached) = registry.slots.remove(&key) {
        for entity in cached.entities {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
        for mesh in cached.meshes {
            meshes.remove(mesh.id());
        }
        for material in cached.materials {
            materials.remove(material.id());
        }
    }

    let mut cached = GtaAnimatedRuntimeMeshSlot {
        entities: Vec::with_capacity(drawables.len()),
        meshes: Vec::with_capacity(drawables.len()),
        materials: Vec::with_capacity(drawables.len()),
        material_base_alphas: Vec::with_capacity(drawables.len()),
        material_base_alpha_modes: Vec::with_capacity(drawables.len()),
        dynamic_vehicle_parts: None,
        dynamic_vehicle_baselines: None,
        rendered_transforms: Vec::with_capacity(drawables.len()),
        rendered_root_transform: transform,
        vehicle_resident_state: None,
        rigid_frame_indices,
        hide_original_head,
        geometry_signature,
        mesh_layout_signature,
        is_visible: true,
    };
    let rigid_part_transforms = cached.rigid_frame_indices.as_deref().and_then(|frames| {
        rigid_cutscene_part_transforms(
            model_index,
            file_loader,
            bone_poses,
            root_translation_on_entity,
            frames,
            model_snapshot,
        )
    });
    // Animated runtime models resolve their own textures on demand. A cutscene
    // object's cloned clump owns the TXD identity it had at SetModelIndex time;
    // special slots such as BET's 188 may already point at a different DFF/TXD.
    if let Some(snapshot_txd_slot) =
        model_snapshot.and_then(|snapshot| usize::try_from(snapshot.txd_slot).ok())
    {
        let _ = sync_txd_slot_texture_cache(snapshot_txd_slot, file_loader, texture_cache, images);
        ensure_environment_maps(file_loader, texture_cache);
    } else {
        sync_model_texture_cache(model_index, file_loader, texture_cache, images);
    }
    for (part, drawable) in drawables.into_iter().enumerate() {
        stats.spawned_pool_meshes += 1;
        let (base_color_texture, texture_alpha, base_color) = resolve_drawable_material_bind(
            &drawable,
            file_loader,
            texture_cache,
            images,
            VehicleColourRemap::default(),
        );
        if base_color_texture.is_some() {
            stats.textured_meshes += 1;
        }
        let mesh = meshes.add(drawable.mesh);
        let material_asset = gta_world_material(
            base_color,
            base_color_texture,
            texture_alpha,
            false,
            drawable.surface_ambient,
            drawable.surface_diffuse,
            drawable.has_prelit,
            lighting,
        );
        cached
            .material_base_alphas
            .push(material_asset.uniforms.colour.w);
        cached
            .material_base_alpha_modes
            .push(material_asset.alpha_mode.clone());
        let material = materials.add(material_asset);
        let entity = commands
            .spawn((
                GtaImmediateMesh,
                GtaRuntimePoolMesh {
                    source,
                    slot,
                    model_index,
                    render_damaged: false,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                NoFrustumCulling,
                Visibility::Visible,
                rigid_part_transforms
                    .as_ref()
                    .and_then(|parts| parts.get(part))
                    .map(|local| transform.mul_transform(*local))
                    .unwrap_or(transform),
                GlobalTransform::default(),
            ))
            .id();
        cached.entities.push(entity);
        cached.meshes.push(mesh);
        cached.materials.push(material);
        cached.rendered_transforms.push(
            rigid_part_transforms
                .as_ref()
                .and_then(|parts| parts.get(part))
                .map(|local| transform.mul_transform(*local))
                .unwrap_or(transform),
        );
    }
    registry.slots.insert(key, cached);
}


pub(super) fn runtime_model_geometry_signature(
    file_loader: &CFileLoader,
    model_index: usize,
    optional_components: Option<[i8; 2]>,
    model_snapshot: Option<&crate::animation::CutsceneMgr::CutsceneObjectModelSnapshot>,
) -> u64 {
    // Deterministic FNV-1a over the resident model identity. `RequestSpecialModel`
    // changes the base name while retaining model ids 185..189; the serialized
    // clump shape fields protect ordinary streaming replacement as well.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    let live_model = file_loader.model_info.get_model_info(model_index);
    let model_name = model_snapshot
        .map(|snapshot| snapshot.model_name.as_str())
        .or_else(|| live_model.map(|model| model.base().name.as_str()))
        .unwrap_or_default();
    feed(model_name.as_bytes());
    if let Some(components) = optional_components {
        feed(&components.map(|component| component as u8));
    }
    let txd_slot = model_snapshot
        .map(|snapshot| snapshot.txd_slot)
        .or_else(|| live_model.map(|model| model.base().txd_slot))
        .unwrap_or(-1);
    feed(&txd_slot.to_le_bytes());
    let loaded = model_snapshot.is_some_and(|snapshot| snapshot.clump_summary.is_some())
        || live_model.is_some_and(|model| model.base().rw_object_loaded);
    feed(&[u8::from(loaded)]);
    let summary = model_snapshot
        .and_then(|snapshot| snapshot.clump_summary.as_ref())
        .or_else(|| live_model.and_then(|model| model.base().clump_summary.as_ref()));
    if let Some(summary) = summary {
        feed(&summary.num_atomics.to_le_bytes());
        feed(&summary.num_frames.to_le_bytes());
        feed(&summary.num_geometries.to_le_bytes());
        for geometry in &summary.geometries {
            feed(&geometry.chunk_version.to_le_bytes());
            feed(&geometry.chunk_length.to_le_bytes());
            feed(&geometry.num_vertices.to_le_bytes());
            feed(&geometry.num_triangles.to_le_bytes());
        }
    }
    hash
}


#[cfg(test)]
mod runtime_actor_read_tests {
    use super::*;
    use bevy::ecs::system::SystemState;
    use crate::{
        control::ScriptEntities::{ScriptPedEntity, script_entity_test_world, spawn_script_test_ped, spawn_script_test_vehicle},
        peds::PedType::ePedType,
    };
    use re3bevy_core::core::General::CGeneral;

    #[test]
    fn submission_reads_canonical_live_components_and_rejects_retired_generations() {
        let mut world = script_entity_test_world();
        let mut general = CGeneral::with_seed(1);
        let live_handle = 0x102;
        let stale_handle = 0x101;
        let entity = spawn_script_test_ped(&mut world, ScriptPedEntity {
            locomotion: Default::default(),
            handle: live_handle,
            model_id: 0,
            ped: CPed::new(ePedType::PEDTYPE_CIVMALE, &mut general),
            animation: PedAnimation::default(),
            vitality: PedVitality::default(),
        });
        world.resource_mut::<ScriptEntityIndex>().peds.insert(stale_handle, entity);
        let vehicle_handle = 0x402;
        let car = crate::vehicles::Automobile::CAutomobile::new(
            90, crate::vehicles::Vehicle::MISSION_VEHICLE, 0, &mut general, None, None, false);
        let vehicle_entity = spawn_script_test_vehicle(&mut world,
            crate::control::ScriptEntities::ScriptVehicleEntity::Automobile { handle: vehicle_handle, car });
        world.entity_mut(entity).insert(InVehicle::driver(vehicle_entity));
        let mut state = SystemState::<RuntimeActorsRead>::new(&mut world);
        {
            let actors = state.get(&world).unwrap();
            assert_eq!(actors.ped_entity(live_handle), Some(entity));
            assert!(actors.ped_entity(stale_handle).is_none());
            assert_eq!(actors.peds().count(), 1);
            assert_eq!(actors.current_vehicle_handle_for_char(live_handle), Some(vehicle_handle));
            assert_eq!(actors.relation_occupants(vehicle_entity).count(), 1);
        }
        let position = CVector::new(5.0, 8.0, 12.0);
        world.get_mut::<CPed>(entity).unwrap().physical.entity.set_position(position);
        world.get_mut::<PedVitality>(entity).unwrap().health = 71.0;
        {
            let actors = state.get(&world).unwrap();
            let (_, actor) = actors.peds().next().unwrap();
            assert_eq!(actor.ped.physical.entity.get_position(), position);
            assert_eq!(actor.vitality.health, 71.0);
        }
        world.resource_mut::<ScriptEntityIndex>().vehicles.remove(&vehicle_handle);
        {
            let actors = state.get(&world).unwrap();
            assert!(actors.current_vehicle_handle_for_char(live_handle).is_none());
            assert_eq!(actors.relation_occupants(vehicle_entity).count(), 0);
        }
        world.resource_mut::<ScriptEntityIndex>().peds.remove(&live_handle);
        {
            let actors = state.get(&world).unwrap();
            assert!(actors.ped_entity(live_handle).is_none());
            assert_eq!(actors.peds().count(), 0);
        }
        world.despawn(entity);
        assert_eq!(state.get(&world).unwrap().peds().count(), 0);
    }
}
