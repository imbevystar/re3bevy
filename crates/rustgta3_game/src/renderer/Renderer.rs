//! re3 reference: references/re-GTA/src/renderer/Renderer.cpp
//! re3 reference: references/re-GTA/src/renderer/Renderer.h
//!
//! Data-level renderer state used before the full visibility/render pass is
//! ported.

use crate::core::World::{CWorldEntity, EEntityType};
use bevy::prelude::Resource;
use rustgta3_core::core::Coordinates::CVector;

pub(crate) fn entity_cull_zone_visible(disable_zone_cull: bool, entity: &CWorldEntity) -> bool {
    if disable_zone_cull {
        return true;
    }
    if entity.zone_culled {
        return false;
    }
    match entity.entity_type {
        EEntityType::Vehicle | EEntityType::Ped => !entity.zone_culled2,
        EEntityType::Object if entity.is_static => !entity.zone_culled2,
        _ => true,
    }
}

/// TEMP-DEBUG-ZONECULL: live-readable bypass mirroring re3 `gbDisableZoneCull`
/// (`renderer/Renderer.cpp:33`, `core/re3.cpp:1045`).
/// In re3, `gbDisableZoneCull` was introduced because cullzone.dat uses static
/// pool indices from the 2001 release that cause false occlusion culling under
/// modern resolutions and dynamic pool allocations.
/// Disabled by default in rustgta3 to prevent roads and buildings disappearing.
/// Can be forced on for testing via `RUSTGTA3_ENABLE_ZONE_CULL=1`.
pub(crate) fn zone_cull_disabled(renderer: &CRenderer) -> bool {
    if std::env::var_os("RUSTGTA3_ENABLE_ZONE_CULL").is_some_and(|value| value != "0") {
        return false;
    }
    renderer.disable_zone_cull
        || std::env::var_os("RUSTGTA3_DISABLE_ZONE_CULL").is_some_and(|value| value != "0")
}

pub const NUMVISIBLEENTITIES: usize = 2000;
pub const NUMINVISIBLEENTITIES: usize = 150;
pub const LOD_DISTANCE: f32 = 300.0;
pub const FADE_DISTANCE: f32 = 20.0;
pub const STREAM_DISTANCE: f32 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererVisibility {
    Invisible,
    Visible,
    Offscreen,
    StreamMe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererModelType {
    Simple,
    Time,
    Vehicle,
    Ped,
    Clump,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererModelSnapshot {
    pub model_type: RendererModelType,
    pub loaded_for_distance: bool,
    pub loaded_for_fade_distance: bool,
    pub largest_lod_distance: f32,
    pub near_distance: f32,
    pub no_fade: bool,
    pub draw_last: bool,
    pub alpha: u8,
    pub time_on: Option<u8>,
    pub time_off: Option<u8>,
    pub other_time_model_exists: bool,
    pub other_time_model_loaded: bool,
    pub related_model_exists: bool,
    pub related_model_loaded: bool,
    pub related_model_alpha: u8,
    pub related_time_other_loaded: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererEntitySnapshot {
    pub entity_type: EEntityType,
    pub model_index: usize,
    pub position: CVector,
    pub rw_object_loaded: bool,
    pub visible: bool,
    pub on_screen: bool,
    pub on_screen_complex: bool,
    pub draw_last: bool,
    pub render_damaged: bool,
    pub first_person_player_vehicle: bool,
    pub first_person_camera_forward: bool,
    pub first_person_big_vehicle_exception: bool,
    pub temporary_object: bool,
    pub big_building: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererVisibilitySideEffects {
    pub create_rw_object: bool,
    pub delete_rw_object: bool,
    pub set_model_damaged: bool,
    pub clear_model_damaged: bool,
    pub increase_alpha: bool,
    pub force_alpha_255: bool,
    pub distance_fade: bool,
    pub no_bright_headlights: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RendererVisibilityDecision {
    pub visibility: RendererVisibility,
    pub distance: f32,
    pub sorted_distance: Option<f32>,
    pub side_effects: RendererVisibilitySideEffects,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererFrameLists {
    pub visible: Vec<usize>,
    pub invisible_prerender: Vec<usize>,
    pub sorted_alpha: Vec<usize>,
    pub stream_requests: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RendererPassPlan {
    pub roads: Vec<usize>,
    pub everything_bar_roads: Vec<usize>,
    pub fading_in_entities: Vec<usize>,
    /// Subset of `fading_in_entities` for which re3 sets
    /// `CEntity::bDistanceFade`. Draw-last entities share the same sorted
    /// alpha list but must stay fully opaque.
    pub distance_fade_entities: Vec<usize>,
    pub invisible_prerender: Vec<usize>,
    /// VIS_STREAMME models first seen by re3 `ScanSectorList_Priority`.
    pub priority_stream_requests: Vec<usize>,
    pub stream_requests: Vec<usize>,
    /// `CEntity::CreateRwObject()` side effects emitted while scanning the
    /// world. The bool selects the faded atomic fallback used by the second
    /// half of `SetupEntityVisibility`.
    pub rw_object_creates: Vec<(usize, bool)>,
    /// Per-entity shared-model alpha mutations, preserved in scan order as
    /// `(entity_index, increase_alpha, force_alpha_255)`.
    pub model_alpha_updates: Vec<(usize, bool, bool)>,
    /// Entity-side `DeleteRwObject()` calls emitted by visibility setup.
    /// Currently this is the big-building LOD path from re3
    /// `CRenderer::SetupBigBuildingVisibility`.
    pub rw_object_deletes: Vec<usize>,
}

impl RendererVisibilityDecision {
    fn new(visibility: RendererVisibility, distance: f32) -> Self {
        Self {
            visibility,
            distance,
            sorted_distance: None,
            side_effects: RendererVisibilitySideEffects::default(),
        }
    }
}

impl Default for RendererVisibilitySideEffects {
    fn default() -> Self {
        Self {
            create_rw_object: false,
            delete_rw_object: false,
            set_model_damaged: false,
            clear_model_damaged: false,
            increase_alpha: false,
            force_alpha_255: false,
            distance_fade: false,
            no_bright_headlights: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CRenderer {
    pub loading_priority: bool,
    pub disable_zone_cull: bool,
    pub sorted_vehicles_and_peds_capacity: usize,
    pub visible_entities_capacity: usize,
    pub invisible_entities_capacity: usize,
    pub sort_big_buildings_called: bool,
    pub entities_rendered: i32,
    pub entities_not_rendered: i32,
    pub rendered_big_buildings: i32,
    pub rendered_buildings: i32,
    pub rendered_cars: i32,
    pub rendered_peds: i32,
    pub rendered_objects: i32,
    pub rendered_dummies: i32,
    pub lod_dist_scale_tenths: i32,
    /// Final `TheCamera.LODDistMultiplier` for the current render frame.
    pub camera_lod_dist_multiplier: f32,
    pub camera_position: CVector,
    /// Mod-only extra viewport positions. Empty in the original single-player
    /// path, so every existing re3 visibility transcript remains unchanged.
    local_coop_camera_positions: Vec<CVector>,
    pub current_hour: u8,
    pub visible_entities: Vec<usize>,
    pub invisible_entities: Vec<usize>,
    pub sorted_alpha_entities: Vec<(usize, f32)>,
    distance_fade_entities: Vec<usize>,
    pub stream_requests: Vec<usize>,
}

impl Default for CRenderer {
    fn default() -> Self {
        Self {
            loading_priority: false,
            disable_zone_cull: true,
            sorted_vehicles_and_peds_capacity: 0,
            visible_entities_capacity: NUMVISIBLEENTITIES,
            invisible_entities_capacity: NUMINVISIBLEENTITIES,
            sort_big_buildings_called: false,
            entities_rendered: 0,
            entities_not_rendered: 0,
            rendered_big_buildings: 0,
            rendered_buildings: 0,
            rendered_cars: 0,
            rendered_peds: 0,
            rendered_objects: 0,
            rendered_dummies: 0,
            lod_dist_scale_tenths: 12,
            camera_lod_dist_multiplier: 1.2,
            camera_position: CVector::ZERO,
            local_coop_camera_positions: Vec::with_capacity(1),
            current_hour: 0,
            visible_entities: Vec::with_capacity(NUMVISIBLEENTITIES),
            invisible_entities: Vec::with_capacity(NUMINVISIBLEENTITIES),
            sorted_alpha_entities: Vec::new(),
            distance_fade_entities: Vec::new(),
            stream_requests: Vec::new(),
        }
    }
}

impl CRenderer {
    pub fn lod_distance_multiplier(&self) -> f32 {
        self.camera_lod_dist_multiplier
    }

    pub fn renderer_lod_scale(&self) -> f32 {
        self.lod_dist_scale_tenths as f32 / 10.0
    }

    pub fn init(&mut self) {
        self.sorted_vehicles_and_peds_capacity = 40;
        self.sort_big_buildings_called = true;
    }

    pub fn is_entity_cull_zone_visible(&self, entity: &CWorldEntity) -> bool {
        entity_cull_zone_visible(zone_cull_disabled(self), entity)
    }

    pub fn is_vehicle_cull_zone_visible(&self, entity: &CWorldEntity) -> bool {
        !entity.zone_culled2
    }

    pub fn clear_for_frame(&mut self) {
        self.visible_entities.clear();
        self.invisible_entities.clear();
        self.sorted_alpha_entities.clear();
        self.distance_fade_entities.clear();
        self.stream_requests.clear();
        self.entities_rendered = 0;
        self.entities_not_rendered = 0;
        self.rendered_big_buildings = 0;
        self.rendered_buildings = 0;
        self.rendered_cars = 0;
        self.rendered_peds = 0;
        self.rendered_objects = 0;
        self.rendered_dummies = 0;
    }

    pub fn begin_construct_render_list(
        &mut self,
        camera_position: CVector,
        current_hour: u8,
        camera_lod_dist_multiplier: f32,
    ) {
        self.camera_position = camera_position;
        self.local_coop_camera_positions.clear();
        self.current_hour = current_hour;
        self.camera_lod_dist_multiplier = camera_lod_dist_multiplier;
        self.clear_for_frame();
    }

    /// Split-screen adapter: preserve player zero as the authored camera while
    /// visibility/LOD distance uses the closest active viewport.
    pub fn begin_construct_render_list_for_cameras(
        &mut self,
        camera_positions: &[CVector],
        current_hour: u8,
        camera_lod_dist_multiplier: f32,
    ) {
        let primary = camera_positions.first().copied().unwrap_or(CVector::ZERO);
        self.begin_construct_render_list(primary, current_hour, camera_lod_dist_multiplier);
        self.local_coop_camera_positions
            .extend(camera_positions.iter().copied().skip(1));
    }

    pub fn construct_render_list<I>(&mut self, entities: I) -> RendererFrameLists
    where
        I: IntoIterator<Item = (usize, RendererEntitySnapshot, RendererModelSnapshot)>,
    {
        self.clear_for_frame();
        for (index, entity, model) in entities {
            let decision = if entity.big_building {
                self.setup_big_building_visibility(&entity, &model)
            } else {
                self.setup_entity_visibility(&entity, &model)
            };
            match decision.visibility {
                RendererVisibility::Visible => self.visible_entities.push(index),
                RendererVisibility::Invisible => {
                    if decision.sorted_distance.is_some() {
                        self.sorted_alpha_entities
                            .push((index, decision.sorted_distance.unwrap()));
                        if decision.side_effects.distance_fade {
                            self.distance_fade_entities.push(index);
                        }
                    } else {
                        self.invisible_entities.push(index);
                    }
                }
                RendererVisibility::Offscreen => self.invisible_entities.push(index),
                RendererVisibility::StreamMe => self.stream_requests.push(entity.model_index),
            }
        }
        self.sorted_alpha_entities
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        RendererFrameLists {
            visible: self.visible_entities.clone(),
            invisible_prerender: self.invisible_entities.clone(),
            sorted_alpha: self
                .sorted_alpha_entities
                .iter()
                .map(|(index, _)| *index)
                .collect(),
            stream_requests: self.stream_requests.clone(),
        }
    }

    pub fn build_render_pass_plan(
        &self,
        frame_lists: &RendererFrameLists,
        world_entities: &[CWorldEntity],
    ) -> RendererPassPlan {
        let mut roads = Vec::new();
        let mut everything_bar_roads = Vec::new();
        for index in &frame_lists.visible {
            let Some(entity) = world_entities.get(*index) else {
                continue;
            };
            if entity.treadable_pool_slot.is_some() {
                roads.push(*index);
            } else {
                everything_bar_roads.push(*index);
            }
        }
        RendererPassPlan {
            roads,
            everything_bar_roads,
            fading_in_entities: frame_lists.sorted_alpha.clone(),
            distance_fade_entities: self.distance_fade_entities.clone(),
            invisible_prerender: frame_lists.invisible_prerender.clone(),
            priority_stream_requests: Vec::new(),
            stream_requests: frame_lists.stream_requests.clone(),
            rw_object_creates: Vec::new(),
            model_alpha_updates: Vec::new(),
            rw_object_deletes: Vec::new(),
        }
    }

    /// Builds the render pass directly from the C++-style visibility lists.
    ///
    /// `ConstructRenderList` in re3 writes its fixed pointer arrays once and
    /// the renderer consumes those arrays immediately.  The old Bevy adapter
    /// materialised a `Vec` of snapshots, cloned every internal list into a
    /// `RendererFrameLists`, then cloned those lists again into the pass plan.
    /// Keep the legacy methods above for parity callers, but use this path in
    /// the frame renderer so the visibility pass has one traversal and reuses
    /// the caller-provided plan capacities.
    pub fn construct_render_pass_plan_after_begin<I>(
        &mut self,
        entities: I,
        world_entities: &[CWorldEntity],
        plan: &mut RendererPassPlan,
    ) where
        I: IntoIterator<Item = (usize, RendererEntitySnapshot, RendererModelSnapshot)>,
    {
        self.construct_render_pass_plan_after_begin_with_scan_flags(
            entities
                .into_iter()
                .map(|(index, entity, model)| (index, entity, model, false, true)),
            world_entities,
            plan,
        );
    }

    pub fn construct_render_pass_plan_after_begin_with_scan_flags<I>(
        &mut self,
        entities: I,
        world_entities: &[CWorldEntity],
        plan: &mut RendererPassPlan,
    ) where
        I: IntoIterator<
            Item = (
                usize,
                RendererEntitySnapshot,
                RendererModelSnapshot,
                bool,
                bool,
            ),
        >,
    {
        self.clear_for_frame();
        plan.roads.clear();
        plan.everything_bar_roads.clear();
        plan.fading_in_entities.clear();
        plan.distance_fade_entities.clear();
        plan.invisible_prerender.clear();
        plan.priority_stream_requests.clear();
        plan.stream_requests.clear();
        plan.rw_object_creates.clear();
        plan.model_alpha_updates.clear();
        plan.rw_object_deletes.clear();

        for (index, entity, model, priority_scan, accept_visibility) in entities {
            let decision = if entity.big_building {
                self.setup_big_building_visibility(&entity, &model)
            } else {
                self.setup_entity_visibility(&entity, &model)
            };
            if decision.side_effects.create_rw_object {
                plan.rw_object_creates.push((
                    index,
                    !model.loaded_for_distance && model.loaded_for_fade_distance,
                ));
            }
            if decision.side_effects.increase_alpha || decision.side_effects.force_alpha_255 {
                plan.model_alpha_updates.push((
                    index,
                    decision.side_effects.increase_alpha,
                    decision.side_effects.force_alpha_255,
                ));
            }
            if decision.side_effects.delete_rw_object {
                plan.rw_object_deletes.push(index);
            }
            if !accept_visibility {
                if decision.visibility == RendererVisibility::StreamMe {
                    self.stream_requests.push(entity.model_index);
                    if priority_scan {
                        plan.priority_stream_requests.push(entity.model_index);
                    } else {
                        plan.stream_requests.push(entity.model_index);
                    }
                }
                continue;
            }
            match decision.visibility {
                RendererVisibility::Visible => {
                    self.visible_entities.push(index);
                    if world_entities
                        .get(index)
                        .is_some_and(|world_entity| world_entity.treadable_pool_slot.is_some())
                    {
                        plan.roads.push(index);
                    } else {
                        plan.everything_bar_roads.push(index);
                    }
                }
                RendererVisibility::Invisible => {
                    if decision.sorted_distance.is_some() {
                        let distance = decision.sorted_distance.unwrap_or_default();
                        self.sorted_alpha_entities.push((index, distance));
                        plan.fading_in_entities.push(index);
                        if decision.side_effects.distance_fade {
                            plan.distance_fade_entities.push(index);
                        }
                    } else {
                        self.invisible_entities.push(index);
                        plan.invisible_prerender.push(index);
                    }
                }
                RendererVisibility::Offscreen => {
                    self.invisible_entities.push(index);
                    plan.invisible_prerender.push(index);
                }
                RendererVisibility::StreamMe => {
                    self.stream_requests.push(entity.model_index);
                    if priority_scan {
                        plan.priority_stream_requests.push(entity.model_index);
                    } else {
                        plan.stream_requests.push(entity.model_index);
                    }
                }
            }
        }

        self.sorted_alpha_entities
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        // The pass plan must follow the same far-to-near alpha order as the
        // legacy frame-list path. Sorting in place avoids constructing a
        // second temporary list, while retaining the renderer's diagnostic
        // tuples for parity callers.
        plan.fading_in_entities.clear();
        plan.fading_in_entities
            .extend(self.sorted_alpha_entities.iter().map(|(index, _)| *index));
    }

    pub fn should_model_be_streamed(
        &self,
        entity: &RendererEntitySnapshot,
        model: &RendererModelSnapshot,
    ) -> bool {
        let dist = self.distance_to_camera(entity.position);
        if model.no_fade {
            dist - STREAM_DISTANCE < model.largest_lod_distance
        } else {
            dist - FADE_DISTANCE - STREAM_DISTANCE < model.largest_lod_distance
        }
    }

    pub fn setup_entity_visibility(
        &self,
        entity: &RendererEntitySnapshot,
        model: &RendererModelSnapshot,
    ) -> RendererVisibilityDecision {
        let mut request = true;
        let mut force_time_alpha_255 = false;
        if model.model_type == RendererModelType::Time {
            // re3 `CANTIMECULL` is true when there is no alternate time model,
            // or when that alternate model's RW object is resident.  Being in
            // range never short-circuits normal simple-model visibility; it
            // only forces the time model fully opaque before continuing.
            let can_time_cull = !model.other_time_model_exists || model.other_time_model_loaded;
            if time_in_range(model.time_on, model.time_off, self.current_hour) {
                force_time_alpha_255 = can_time_cull;
            } else if can_time_cull {
                return RendererVisibilityDecision::new(
                    RendererVisibility::Invisible,
                    self.distance_to_camera(entity.position),
                );
            } else {
                request = false;
            }
        } else if model.model_type != RendererModelType::Simple {
            if entity.first_person_player_vehicle {
                let mut decision =
                    RendererVisibilityDecision::new(RendererVisibility::Offscreen, 0.0);
                decision.side_effects.no_bright_headlights = Some(
                    entity.first_person_camera_forward || entity.first_person_big_vehicle_exception,
                );
                return decision;
            }
            if !entity.rw_object_loaded || !entity.visible {
                return RendererVisibilityDecision::new(RendererVisibility::Invisible, 0.0);
            }
            if !entity.on_screen {
                return RendererVisibilityDecision::new(RendererVisibility::Offscreen, 0.0);
            }
            if entity.draw_last {
                let dist = self.distance_to_camera(entity.position);
                let mut decision =
                    RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
                decision.sorted_distance = Some(dist);
                decision.side_effects.distance_fade = false;
                return decision;
            }
            return RendererVisibilityDecision::new(RendererVisibility::Visible, 0.0);
        }

        // The temporary-object special case sits inside the non-time branch in
        // re3. Time models always continue through the simple-model path.
        if model.model_type != RendererModelType::Time && entity.temporary_object {
            if !entity.rw_object_loaded || !entity.visible {
                return RendererVisibilityDecision::new(RendererVisibility::Invisible, 0.0);
            }
            return RendererVisibilityDecision::new(
                if entity.on_screen {
                    RendererVisibility::Visible
                } else {
                    RendererVisibility::Offscreen
                },
                0.0,
            );
        }

        let mut dist = self.distance_to_camera(entity.position);
        // re3 `Renderer.h:6` (FIX_BUGS): `LOD_DISTANCE` is
        // `300 * TheCamera.LODDistMultiplier`; `STREAM_DISTANCE` stays
        // constant. `model.largest_lod_distance` already carries the same
        // multiplier via the visibility snapshot.
        let lod_threshold = LOD_DISTANCE * self.camera_lod_dist_multiplier + STREAM_DISTANCE;
        if lod_threshold < dist && dist < model.largest_lod_distance {
            dist = model.largest_lod_distance;
        }

        if model.loaded_for_distance {
            let mut decision = RendererVisibilityDecision::new(RendererVisibility::Visible, dist);
            decision.side_effects.force_alpha_255 = force_time_alpha_255;
            decision.side_effects.set_model_damaged = entity.render_damaged;
            decision.side_effects.clear_model_damaged = true;
            decision.side_effects.create_rw_object = !entity.rw_object_loaded;
            decision.side_effects.increase_alpha = true;
            if !entity.visible {
                decision.visibility = RendererVisibility::Invisible;
                return decision;
            }
            if !entity.on_screen {
                decision.visibility = RendererVisibility::Offscreen;
                decision.side_effects.force_alpha_255 = true;
                return decision;
            }
            if model.alpha != 255 && !force_time_alpha_255 {
                decision.visibility = RendererVisibility::Invisible;
                decision.sorted_distance = Some(dist);
                decision.side_effects.distance_fade = true;
                return decision;
            }
            if model.draw_last || entity.draw_last {
                decision.visibility = RendererVisibility::Invisible;
                decision.sorted_distance = Some(dist);
                decision.side_effects.distance_fade = false;
                return decision;
            }
            return decision;
        }

        if model.no_fade {
            let visibility = if dist - STREAM_DISTANCE < model.largest_lod_distance && request {
                RendererVisibility::StreamMe
            } else {
                RendererVisibility::Invisible
            };
            let mut decision = RendererVisibilityDecision::new(visibility, dist);
            decision.side_effects.force_alpha_255 = force_time_alpha_255;
            decision.side_effects.clear_model_damaged = true;
            return decision;
        }

        if !model.loaded_for_fade_distance {
            let visibility =
                if dist - FADE_DISTANCE - STREAM_DISTANCE < model.largest_lod_distance && request {
                    RendererVisibility::StreamMe
                } else {
                    RendererVisibility::Invisible
                };
            let mut decision = RendererVisibilityDecision::new(visibility, dist);
            decision.side_effects.force_alpha_255 = force_time_alpha_255;
            decision.side_effects.clear_model_damaged = true;
            return decision;
        }

        let mut decision = RendererVisibilityDecision::new(RendererVisibility::Offscreen, dist);
        decision.side_effects.force_alpha_255 = force_time_alpha_255;
        decision.side_effects.create_rw_object = !entity.rw_object_loaded;
        decision.side_effects.increase_alpha = true;
        decision.side_effects.clear_model_damaged = true;
        if !entity.visible {
            decision.visibility = RendererVisibility::Invisible;
        } else if entity.on_screen {
            decision.sorted_distance = Some(dist);
            decision.side_effects.distance_fade = true;
        } else {
            decision.side_effects.force_alpha_255 = true;
        }
        decision
    }

    pub fn setup_big_building_visibility(
        &self,
        entity: &RendererEntitySnapshot,
        model: &RendererModelSnapshot,
    ) -> RendererVisibilityDecision {
        let dist = self.distance_to_camera(entity.position);
        if model.model_type == RendererModelType::Time
            && (!model.other_time_model_exists || model.other_time_model_loaded)
            && !time_in_range(model.time_on, model.time_off, self.current_hour)
        {
            return RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
        }
        if model.model_type == RendererModelType::Vehicle {
            return RendererVisibilityDecision::new(
                if entity.visible {
                    RendererVisibility::Visible
                } else {
                    RendererVisibility::Invisible
                },
                dist,
            );
        }
        // re3 `Renderer.cpp:907` with the FIX_BUGS `Renderer.h:6` macro:
        // `LOD_DISTANCE` is scaled, `STREAM_DISTANCE` is not.
        // `model.near_distance` already carries the multiplier.
        if dist < model.near_distance
            && dist < LOD_DISTANCE * self.camera_lod_dist_multiplier + STREAM_DISTANCE
            && (!model.related_model_exists
                || model.related_model_loaded && model.related_model_alpha == 255
                || model.related_time_other_loaded)
        {
            return RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
        }
        if model.loaded_for_distance {
            let mut decision = RendererVisibilityDecision::new(
                if entity.visible && entity.on_screen_complex {
                    RendererVisibility::Visible
                } else {
                    RendererVisibility::Invisible
                },
                dist,
            );
            decision.side_effects.create_rw_object = !entity.rw_object_loaded;
            if model.draw_last && decision.visibility == RendererVisibility::Visible {
                decision.visibility = RendererVisibility::Invisible;
                decision.sorted_distance = Some(dist);
            }
            return decision;
        }
        if model.no_fade {
            let mut decision = RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
            decision.side_effects.delete_rw_object = true;
            return decision;
        }
        if !model.loaded_for_fade_distance {
            let mut decision = RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
            decision.side_effects.delete_rw_object = true;
            return decision;
        }
        let mut decision = RendererVisibilityDecision::new(RendererVisibility::Invisible, dist);
        decision.side_effects.create_rw_object = !entity.rw_object_loaded;
        if entity.visible && entity.on_screen_complex {
            decision.sorted_distance = Some(dist);
        }
        decision
    }

    fn camera_position(&self) -> CVector {
        self.camera_position
    }

    fn distance_to_camera(&self, position: CVector) -> f32 {
        self.local_coop_camera_positions.iter().fold(
            distance(position, self.camera_position()),
            |nearest, camera| nearest.min(distance(position, *camera)),
        )
    }
}

fn distance(a: CVector, b: CVector) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn time_in_range(time_on: Option<u8>, time_off: Option<u8>, hour: u8) -> bool {
    match (time_on, time_off) {
        (Some(on), Some(off)) if on > off => hour >= on || hour < off,
        (Some(on), Some(off)) => hour >= on && hour < off,
        _ => true,
    }
}
