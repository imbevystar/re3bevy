//! re3 reference: references/re-GTA/src/peds/PlayerPed.cpp
//! re3 reference: references/re-GTA/src/peds/PlayerPed.h
//!
//! Player ped state, SetupPlayerPed, and Zelda on-foot pad control.

use super::PedWater::{apply_player_drowning_damage, process_player_water};
use crate::peds::PedLocomotion::PedLocomotion;

use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, Component, Entity, Local, Query, ResMut, Resource, With},
};
use re3bevy_core::core::{
    Coordinates::{CVector, CVector2D},
    General::CGeneral,
};

use crate::{
    animation::{
        AnimBlendAssociation::CAnimBlendAssociation,
    },
    core::{Pad::CPad, Wanted::CWanted},
    modelinfo::ModelIndices::MI_PLAYER,
    peds::{
        Ped::{CPed, MISSION_CHAR, PedFightMoves, PedState, PedWeaponDamageResult, eMoveState},
        PedFight::{
            CFightMoveTable, FIGHTSTATE_JUST_ATTACKED, FIGHTSTATE_MOVE_FINISHED,
            FIGHTSTATE_NO_MOVE, FightEndType, FightMove, FightMoveHitLevel, end_fight,
            start_fight_attack, start_fight_move,
        },
        PedType::{CPedStatsRuntime, CPedTypeRuntime, ePedType},
        VehicleRelations::{
            InVehicle, PedEntity, queue_clear_vehicle_relation, queue_set_vehicle_relation,
        },
    },
    weapons::{
        Explosion::CExplosionPool, ProjectileInfo::CProjectileInfoPool, ShotInfo::CShotInfoPool,
        WeaponType::eWeaponType,
    },
};

use super::PedVitality::PedVitality;

pub const NUM_PLAYER_TARGETTABLE_OBJECTS: usize = 4;
pub const NUM_PLAYER_SAFE_POSITIONS: usize = 6;
pub const MAX_PLAYERS: usize = 2;

/// re3 `PAD_MOVE_TO_GAME_WORLD_MOVE`.
const PAD_MOVE_TO_GAME_WORLD_MOVE: f32 = 60.0;

/// Owned construction/staging form, consumed into the live actor component bundle.
#[derive(Debug, Clone, PartialEq)]
pub struct CPlayerPed {
    pub ped: CPed,
    pub vitality: PedVitality,
    pub locomotion: PedLocomotion,
    pub animation: super::PedAnimation::PedAnimation,
    pub state: PlayerPedState,
}

/// Immutable join of the authoritative player components. No gameplay state is copied.
#[derive(Debug, Clone, Copy)]
pub struct PlayerPedRef<'a> {
    pub ped: &'a CPed,
    pub vitality: &'a PedVitality,
    pub locomotion: &'a PedLocomotion,
    pub animation: &'a super::PedAnimation::PedAnimation,
    pub state: &'a PlayerPedState,
}

/// Mutable join of common ped, shared animation and player-specific state.
pub struct PlayerPedMut<'a> {
    pub ped: &'a mut CPed,
    pub vitality: &'a mut PedVitality,
    pub locomotion: &'a mut PedLocomotion,
    pub animation: &'a mut super::PedAnimation::PedAnimation,
    pub state: &'a mut PlayerPedState,
}

impl std::ops::Deref for PlayerPedRef<'_> {
    type Target = PlayerPedState;
    fn deref(&self) -> &Self::Target { self.state }
}
impl std::ops::Deref for PlayerPedMut<'_> {
    type Target = PlayerPedState;
    fn deref(&self) -> &Self::Target { self.state }
}
impl std::ops::DerefMut for PlayerPedMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.state }
}
impl PlayerPedRef<'_> {
    pub fn as_ref(&self) -> PlayerPedRef<'_> { *self }
    pub fn position(&self) -> CVector { self.ped.physical.entity.get_position() }
    pub fn heading_degrees(&self) -> f32 { self.locomotion.rotation_cur.to_degrees() }
    pub fn is_stopped(&self) -> bool {
        self.state.move_speed.abs() < 0.01 && matches!(self.ped.move_state,
            eMoveState::PEDMOVE_NONE | eMoveState::PEDMOVE_STILL)
    }
}
impl PlayerPedMut<'_> {
    pub fn as_mut(&mut self) -> PlayerPedMut<'_> {
        PlayerPedMut { ped: self.ped, vitality: self.vitality, locomotion: self.locomotion, animation: self.animation, state: self.state }
    }
    pub fn as_ref(&self) -> PlayerPedRef<'_> {
        PlayerPedRef { ped: self.ped, vitality: self.vitality, locomotion: self.locomotion, animation: self.animation, state: self.state }
    }
}

/// Player-specific state; common ped behavior belongs to CPed.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct PlayerPedState {
    pub move_speed: f32,
    pub current_stamina: f32,
    pub max_stamina: f32,
    pub wanted: CWanted,
    pub arresting_cop: Option<usize>,
    pub selected_wep_slot: eWeaponType,
    pub speed_timer: u32,
    pub speed_timer_flag: bool,
    pub weapon_lock_on_target: Option<usize>,
    /// Script object handle when re3 `m_pPointGunAt` refers to one of
    /// `m_nTargettableObjects`. Ped targets continue to use the canonical ped
    /// pool slot in `weapon_lock_on_target`/`CPed::point_gun_at`.
    pub weapon_lock_on_object: Option<i32>,
    pub stamina_progress: f32,
    pub evade_amount: i32,
    pub field_1367: i32,
    pub hit_anim_delay_timer: u32,
    pub attack_button_counter: f32,
    pub have_target_selected: bool,
    pub adrenaline_active: bool,
    pub adrenaline_time: u32,
    pub has_lock_on_target: bool,
    pub can_be_damaged: bool,
    pub walk_angle: f32,
    /// Frame-local local-space leg turn used by re3's mouse locomotion pose
    /// correction in `CPed::CalculateNewVelocity`.
    pub mouse_leg_turn_angle: Option<f32>,
    pub fps_move_heading: f32,
    /// Current re3 jump/fall/landing association.
    pub jump_association: Option<CAnimBlendAssociation>,
    pub jump_animation_time: f32,
    pub targettable_objects: [i32; NUM_PLAYER_TARGETTABLE_OBJECTS],
    pub field_1413: i32,
    pub safe_pos: [CVector; NUM_PLAYER_SAFE_POSITIONS],
    pub ped_at_safe_pos: [Option<usize>; NUM_PLAYER_SAFE_POSITIONS],
    /// Synthetic ped-pool handle for GET_PLAYER_CHAR (re3 pool index).
    pub pool_handle: i32,
    pub active_in_world: bool,
    /// Transient authored-COL pose where this frame's subdivided
    /// `CPhysical::ProcessCollision` first found a dynamic contact. The live
    /// ped matrix has already been rolled back to the saved pre-integration
    /// pose; the later ECS collision stage uses this probe only for the
    /// reciprocal vehicle/object response at the contact pose.
    pub dynamic_collision_probe: Option<CVector>,
    /// Intended end pose paired with `dynamic_collision_probe`. If the later
    /// authoritative collision stage finds no responding impulse at the probe
    /// (for example the other physical moved away), native ProcessCollision
    /// accepts the movement rather than leaving the ped rolled back.
    pub dynamic_collision_target: Option<CVector>,
    /// re3 `CPed::ProcessBuoyancy` delayed water-ring/splash timers. The C++
    /// original stores these as function statics; ECS keeps them per player.
    pub water_raindrops_at: u32,
    pub water_circles_at: u32,
    /// re3 `CWeapon::FireInstantHit` keeps independent static counters for
    /// AK47 and M16 muzzle flashes. Keep that cadence on the player component
    /// so reloads do not reset the every-other-shot effect.
    pub ak47_muzzle_flash_counter: u8,
    pub m16_muzzle_flash_counter: u8,
}

impl std::ops::Deref for CPlayerPed {
    type Target = PlayerPedState;
    fn deref(&self) -> &Self::Target { &self.state }
}
impl std::ops::DerefMut for CPlayerPed {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.state }
}

impl CPlayerPed {
    pub fn into_bundle(self) -> impl bevy::prelude::Bundle {
        let death_lifecycle = super::PedDeath::PedDeathLifecycle::restored(&self.ped);
        (self.ped, self.vitality, self.locomotion, self.animation, self.state,
            super::PedObstacle::PedObstacleAvoidance::default(), death_lifecycle)
    }
    pub fn as_ref(&self) -> PlayerPedRef<'_> {
        PlayerPedRef { ped: &self.ped, vitality: &self.vitality, locomotion: &self.locomotion, animation: &self.animation, state: &self.state }
    }
    pub fn as_mut(&mut self) -> PlayerPedMut<'_> {
        PlayerPedMut { ped: &mut self.ped, vitality: &mut self.vitality, locomotion: &mut self.locomotion, animation: &mut self.animation, state: &mut self.state }
    }

    pub fn new(general: &mut CGeneral, ped_types: Option<&CPedTypeRuntime>) -> Self {
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, general);
        if let Some(ped_types) = ped_types {
            ped.initialise_fear_flags(ped_types);
        }
        ped.physical.entity.set_model_index_no_create(MI_PLAYER);
        ped.ped_state = PedState::PED_IDLE;
        ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        ped.char_created_by = MISSION_CHAR;

        Self {
            vitality: PedVitality::default(), locomotion: PedLocomotion::default(),
            ped,
            animation: super::PedAnimation::PedAnimation::default(),
            state: PlayerPedState {
            move_speed: 0.0,
            current_stamina: 150.0,
            max_stamina: 150.0,
            wanted: CWanted::initialise(),
            arresting_cop: None,
            selected_wep_slot: eWeaponType::WEAPONTYPE_UNARMED,
            speed_timer: 0,
            speed_timer_flag: false,
            weapon_lock_on_target: None,
            weapon_lock_on_object: None,
            stamina_progress: 0.0,
            evade_amount: 0,
            field_1367: 0,
            hit_anim_delay_timer: 0,
            attack_button_counter: 0.0,
            have_target_selected: false,
            adrenaline_active: false,
            adrenaline_time: 0,
            has_lock_on_target: false,
            can_be_damaged: true,
            walk_angle: 0.0,
            mouse_leg_turn_angle: None,
            fps_move_heading: 0.0,
            jump_association: None,
            jump_animation_time: 0.0,
            targettable_objects: [-1; NUM_PLAYER_TARGETTABLE_OBJECTS],
            field_1413: 0,
            safe_pos: [CVector::ZERO; NUM_PLAYER_SAFE_POSITIONS],
            ped_at_safe_pos: [None; NUM_PLAYER_SAFE_POSITIONS],
            pool_handle: 0,
            active_in_world: false,
            dynamic_collision_probe: None,
            dynamic_collision_target: None,
            water_raindrops_at: 0,
            water_circles_at: 0,
            ak47_muzzle_flash_counter: 0,
            m16_muzzle_flash_counter: 0,
            },
        }
    }

    pub fn position(&self) -> CVector {
        self.as_ref().position()
    }
    pub fn set_position(&mut self, position: CVector) {
        self.as_mut().set_position(position)
    }
    pub fn heading_degrees(&self) -> f32 {
        self.as_ref().heading_degrees()
    }
    pub fn set_heading_degrees(&mut self, degrees: f32) {
        self.as_mut().set_heading_degrees(degrees)
    }
    pub fn is_stopped(&self) -> bool {
        self.as_ref().is_stopped()
    }
    pub fn apply_on_foot_pad_step(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        in_vehicle: bool,
    ) -> bool {
        self.as_mut().apply_on_foot_pad_step(pad, dt_seconds, control_enabled, in_vehicle)
    }
    pub fn apply_on_foot_pad_step_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
    ) -> bool {
        self.as_mut().apply_on_foot_pad_step_camera(pad, dt_seconds, control_enabled, camera_heading, in_vehicle)
    }
    pub fn apply_on_foot_pad_input_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
        heavy_weapon: bool,
    ) -> bool {
        self.as_mut().apply_on_foot_pad_input_camera(pad, dt_seconds, control_enabled, camera_heading, in_vehicle, heavy_weapon)
    }
    pub fn apply_on_foot_mouse_runaround_input_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
        heavy_weapon: bool,
    ) -> bool {
        self.as_mut().apply_on_foot_mouse_runaround_input_camera(pad, dt_seconds, control_enabled, camera_heading, in_vehicle, heavy_weapon)
    }
    pub fn stage_anim_move_delta(&mut self, delta: Option<CVector>) {
        self.as_mut().stage_anim_move_delta(delta)
    }

    /// re3 `CPlayerPed::SetupPlayerPed` — allocate player into world players table.
    pub fn setup_player_ped(
        players: &mut dyn WorldPlayerAccess,
        index: i32,
        general: &mut CGeneral,
        ped_types: Option<&CPedTypeRuntime>,
    ) {
        let index = index.max(0) as usize;
        if index >= MAX_PLAYERS {
            return;
        }
        let mut player = Self::new(general, ped_types);
        player.ped.ped_type = match index {
            1 => ePedType::PEDTYPE_PLAYER2,
            _ => ePedType::PEDTYPE_PLAYER1,
        };
        player.state.wanted.maximum_wanted_level = players.maximum_wanted_level();
        player.state.wanted.maximum_chaos = players.maximum_chaos();
        // re3 pool-style handle for GET_PLAYER_CHAR: index+1 until real CPools ped slot.
        player.state.pool_handle = (index as i32) + 1;
        player.ped.physical.entity.set_orientation(0.0, 0.0, 0.0);
        player.ped.wep_accuracy = 100;
        // SetupPlayerPed calls CWorld::Add. CREATE_PLAYER explicitly deactivates
        // it again before setting the initial position.
        player.state.active_in_world = true;
        players.replace_player(index, player);
        // re3's PC build only creates player zero. Local co-op deliberately
        // keeps SCM/global ownership on player zero when it creates slot one.
        if index == 0 {
            players.set_player_in_focus(0);
        }
    }

    /// re3 `DeactivatePlayerPed` / `ReactivatePlayerPed` world membership flags.
    pub fn set_active_in_world(players: &mut dyn WorldPlayerAccess, index: i32, active: bool) {
        if let Some(player) = players.player_mut(index) {
            player.state.active_in_world = active;
        }
    }

}

impl PlayerPedMut<'_> {
    pub fn position(&self) -> CVector { self.as_ref().position() }

    pub fn set_position(&mut self, position: CVector) {
        self.ped.physical.entity.set_position(position);
    }

    /// Heading in degrees (GTA Z-rotation).
    pub fn heading_degrees(&self) -> f32 { self.as_ref().heading_degrees() }

    pub fn set_heading_degrees(&mut self, degrees: f32) {
        let rad = degrees.to_radians();
        self.locomotion.rotation_cur = rad;
        self.locomotion.rotation_dest = rad;
        self.ped.physical.entity.set_heading(rad);
    }

    pub fn is_stopped(&self) -> bool { self.as_ref().is_stopped() }

    /// Standalone Zelda-control step used outside the ECS animation pipeline.
    /// The live game calls [`Self::apply_on_foot_pad_step_camera`] and applies
    /// animation-root translation in `CPed::ProcessControl`; this convenience
    /// entry point advances position kinematically so callers still get a full
    /// control step rather than heading/state changes only.
    pub fn apply_on_foot_pad_step(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        in_vehicle: bool,
    ) -> bool {
        let moved =
            self.apply_on_foot_pad_step_camera(pad, dt_seconds, control_enabled, None, in_vehicle);
        if moved {
            let distance = self.state.move_speed * dt_seconds.max(0.0) * 3.0;
            let (sin, cos) = self.locomotion.rotation_cur.sin_cos();
            let mut position = self.position();
            position.x -= sin * distance;
            position.y += cos * distance;
            self.set_position(position);
        }
        moved
    }

    /// `PlayerControlZelda`: movement heading is relative to
    /// `TheCamera.Orientation`, not the ped's previous heading.
    pub fn apply_on_foot_pad_step_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
    ) -> bool {
        self.apply_on_foot_pad_step_camera_impl(
            pad,
            dt_seconds,
            control_enabled,
            camera_heading,
            in_vehicle,
            true,
            false,
            false,
        )
    }

    /// Live `CPlayerPed::PlayerControlZelda` input phase. Native heading
    /// interpolation belongs to the earlier `CPed::CalculateNewVelocity`
    /// phase, so this variant only writes the destination/scalar movement
    /// state that the next base-ped control pass will consume.
    pub fn apply_on_foot_pad_input_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
        heavy_weapon: bool,
    ) -> bool {
        self.apply_on_foot_pad_step_camera_impl(
            pad,
            dt_seconds,
            control_enabled,
            camera_heading,
            in_vehicle,
            false,
            false,
            heavy_weapon,
        )
    }

    /// re3 CPlayerPed::PlayerControl1stPersonRunAround, selected for the
    /// normal PC third-person mouse camera when FREE_CAM is disabled.
    pub fn apply_on_foot_mouse_runaround_input_camera(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
        heavy_weapon: bool,
    ) -> bool {
        self.apply_on_foot_pad_step_camera_impl(
            pad,
            dt_seconds,
            control_enabled,
            camera_heading,
            in_vehicle,
            false,
            true,
            heavy_weapon,
        )
    }

    pub fn stage_anim_move_delta(&mut self, delta: Option<CVector>) {
        let delta = delta.unwrap_or(CVector::ZERO);
        self.locomotion.anim_move_delta = CVector2D::new(delta.x, delta.y);
    }

    fn apply_on_foot_pad_step_camera_impl(
        &mut self,
        pad: &CPad,
        dt_seconds: f32,
        control_enabled: bool,
        camera_heading: Option<f32>,
        in_vehicle: bool,
        advance_heading_now: bool,
        mouse_runaround: bool,
        heavy_weapon: bool,
    ) -> bool {
        if !control_enabled || pad.disable_player_controls || !self.state.active_in_world {
            self.state.move_speed = 0.0;
            self.ped.move_state = eMoveState::PEDMOVE_STILL;
            return false;
        }
        if in_vehicle {
            return false;
        }

        let jumping = self.ped.ped_state == PedState::PED_JUMP;

        let walk_x = pad.get_ped_walk_left_right() as f32;
        let walk_y = pad.get_ped_walk_up_down() as f32;
        let smooth_spray = !mouse_runaround && self.ped.ped_state == PedState::PED_ATTACK
            && self.ped.point_gun_at.is_none()
            && matches!(self.ped.current_weapon, eWeaponType::WEAPONTYPE_FLAMETHROWER
                | eWeaponType::WEAPONTYPE_COLT45 | eWeaponType::WEAPONTYPE_UZI
                | eWeaponType::WEAPONTYPE_SHOTGUN | eWeaponType::WEAPONTYPE_AK47
                | eWeaponType::WEAPONTYPE_M16 | eWeaponType::WEAPONTYPE_HELICANNON);
        let spray_without_move = smooth_spray && walk_y > 0.0;
        let pad_move_in_game_units = if spray_without_move { 0.0 } else {
            (walk_x * walk_x + walk_y * walk_y).sqrt() / PAD_MOVE_TO_GAME_WORLD_MOVE
        };
        if pad_move_in_game_units <= 0.0 && !spray_without_move {
            self.state.move_speed = 0.0;
            // PlayerControl1stPersonRunAround skips SetRealMoveAnim in
            // PED_FIGHT, so native leaves m_nMoveState untouched here.
            if !(mouse_runaround && self.ped.ped_state == PedState::PED_FIGHT) {
                self.ped.move_state = eMoveState::PEDMOVE_STILL;
            }
            return false;
        }

        // CTimer::GetTimeStep is expressed in GTA's 50 Hz game units
        // (`Timer.cpp`: elapsed_seconds * 50), not in rendered 30 Hz frames.
        // Reconstructing it from wall-clock seconds keeps PlayerControlZelda
        // acceleration and heading turn-rate identical at 30/60/120 FPS.
        let gta_time_step = dt_seconds.max(0.0) * 50.0;
        self.state.move_speed = pad_move_in_game_units.min(self.state.move_speed + 0.07 * gta_time_step);
        if mouse_runaround {
            // Native PlayerControl1stPersonRunAround keeps the ped facing the
            // mouse camera; lateral/backward intent is handled by
            // WorkOutHeadingForMovingFirstPerson + ProcessAnimGroups instead
            // of rotating the ped toward the input vector.
            if let Some(heading) = camera_heading {
                self.locomotion.rotation_dest = CGeneral::limit_radian_angle(heading);
            }
        } else if smooth_spray {
            let divisor = if matches!(self.ped.current_weapon, eWeaponType::WEAPONTYPE_FLAMETHROWER
                | eWeaponType::WEAPONTYPE_COLT45 | eWeaponType::WEAPONTYPE_UZI) { 80.0 } else { 128.0 };
            self.locomotion.rotation_dest = self.locomotion.rotation_cur
                - walk_x / 128.0 * (std::f32::consts::PI / divisor) * gta_time_step;
        } else {
            let pad_heading = CGeneral::get_radian_angle_between_points(0.0, 0.0, -walk_x, walk_y);
            self.locomotion.rotation_dest =
                CGeneral::limit_radian_angle(pad_heading - camera_heading.unwrap_or(0.0));
        }
        if !heavy_weapon && pad.get_sprint() {
            self.ped.move_state = eMoveState::PEDMOVE_SPRINT;
        }

        if advance_heading_now {
            // Convenience/standalone callers do not run the base CPed phase,
            // so preserve the old all-in-one behaviour for them. The live ECS
            // path uses apply_on_foot_pad_input_camera and advances heading in
            // CPed::CalculateNewVelocity before this input phase instead.
            let mut current = CGeneral::limit_radian_angle(self.locomotion.rotation_cur);
            let mut destination = CGeneral::limit_radian_angle(self.locomotion.rotation_dest);
            if current - std::f32::consts::PI > destination {
                destination += std::f32::consts::TAU;
            } else if std::f32::consts::PI + current < destination {
                destination -= std::f32::consts::TAU;
            }
            let max_turn = self.locomotion.heading_rate.to_radians() * gta_time_step;
            let needed = destination - current;
            current += needed.clamp(-max_turn, max_turn);
            self.locomotion.rotation_cur = CGeneral::limit_radian_angle(current);
            self.ped.physical.entity.set_heading(self.locomotion.rotation_cur);
        }
        // PlayerControlZelda still reads movement/heading input while jumping.
        // On a landing frame it then clears only the scalar animation move
        // speed before SetRealMoveAnim; physical airborne velocity remains
        // owned by CPhysical.  Returning false keeps the standalone helper from
        // applying ground/root-motion displacement while the jump state owns it.
        if jumping && self.ped.flags.is_landing {
            self.state.move_speed = 0.0;
        }
        !jumping
    }
}

/// re3 `CPlayerPed::ProcessControl` `PED_SEEK_POS` / `PED_SEEK_ENTITY` branch
/// for script-authored objectives on the player ped. Pad input is ignored
/// while the objective owns movement, the ped turns toward the target at its
/// heading rate, and the per-move-state authored scalar speed feeds
/// `SetRealMoveAnim`, whose root motion advances the ped through the normal
/// collision pipeline. Returns false once the seek has arrived so the
/// objective state machine publishes completion.
pub(crate) fn apply_player_script_seek_step(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    target: CVector,
    dt_seconds: f32,
) -> bool {
    let position = player.position();
    let heading =
        CGeneral::get_radian_angle_between_points(target.x, target.y, position.x, position.y);
    player.locomotion.rotation_dest = heading;
    let mut current = CGeneral::limit_radian_angle(player.locomotion.rotation_cur);
    let mut destination = CGeneral::limit_radian_angle(player.locomotion.rotation_dest);
    if current - std::f32::consts::PI > destination {
        destination += std::f32::consts::TAU;
    } else if std::f32::consts::PI + current < destination {
        destination -= std::f32::consts::TAU;
    }
    let gta_time_step = dt_seconds.max(0.0) * 50.0;
    let max_turn = player.locomotion.heading_rate.to_radians() * gta_time_step;
    let needed = destination - current;
    current += needed.clamp(-max_turn, max_turn);
    player.locomotion.rotation_cur = CGeneral::limit_radian_angle(current);
    player
        .ped
        .physical
        .entity
        .set_heading(player.locomotion.rotation_cur);
    // `CPed::SetRealMoveAnim` speed table for PED_SEEK_POS.
    player.state.move_speed = match player.ped.move_state {
        eMoveState::PEDMOVE_RUN => 1.8,
        eMoveState::PEDMOVE_SPRINT => 2.5,
        eMoveState::PEDMOVE_WALK => 1.0,
        _ => 0.0,
    };
    player.ped.ped_state == PedState::PED_SEEK_ENTITY
        || player.ped.ped_state == PedState::PED_SEEK_POS
}

/// Non-payload metadata for re3 `CWorld::Players[]`.
///
/// Player ped payloads are Bevy components. This resource is deliberately an
/// index plus the true CWorld globals; it never owns a parallel `CPlayerPed`
/// collection.
#[derive(Debug, Clone, Resource)]
pub struct CWorldPlayerState {
    pub entities: [Option<Entity>; MAX_PLAYERS],
    pub player_in_focus: usize,
    /// re3 `CWanted` globals shared by existing and subsequently created players.
    pub maximum_wanted_level: i32,
    pub maximum_chaos: i32,
}

impl Default for CWorldPlayerState {
    fn default() -> Self {
        Self {
            entities: [None, None],
            player_in_focus: 0,
            maximum_wanted_level: crate::core::Wanted::MAXIMUM_WANTED_LEVEL,
            maximum_chaos: crate::core::Wanted::N_MAXIMUM_WANTED_LEVEL,
        }
    }
}

#[derive(Default)]
struct PendingWorldPlayers {
    players: [Option<CPlayerPed>; MAX_PLAYERS],
}

/// ECS access facade for the world-player array. The facade itself owns no
/// gameplay payload: live players are component joins and the resource
/// above stores only player-slot -> `Entity` identity.
#[derive(SystemParam)]
pub struct CWorldPlayers<'w, 's> {
    state: Option<ResMut<'w, CWorldPlayerState>>,
    players: Query<'w, 's, (Entity, &'static mut CPed, &'static mut PedVitality, &'static mut PedLocomotion, &'static mut super::PedAnimation::PedAnimation, &'static mut PlayerPedState, Option<&'static InVehicle>), With<PlayerPedState>>,
    relations: Query<'w, 's, (Entity, Option<&'static InVehicle>), With<PedEntity>>,
    commands: Commands<'w, 's>,
    pending: Local<'s, PendingWorldPlayers>,
}

/// Narrow interface shared by live ECS player access and boot/test staging.
/// Production world state is provided by [`CWorldPlayers`].
pub trait WorldPlayerAccess {
    fn player(&self, index: i32) -> Option<PlayerPedRef<'_>>;
    fn player_mut(&mut self, index: i32) -> Option<PlayerPedMut<'_>>;
    fn replace_player(&mut self, index: usize, player: CPlayerPed);
    fn remove_player(&mut self, index: usize);
    fn player_entity(&self, index: i32) -> Option<Entity>;
    fn vehicle_relation(&self, _index: i32) -> Option<InVehicle> {
        None
    }
    fn vehicle_relation_by_pool_handle(&self, handle: i32) -> Option<InVehicle> {
        let index = (0..MAX_PLAYERS).find(|index| {
            self.player(*index as i32)
                .is_some_and(|player| player.state.pool_handle == handle)
        })?;
        self.vehicle_relation(index as i32)
    }
    fn player_in_focus(&self) -> usize;
    fn set_player_in_focus(&mut self, index: usize);
    fn maximum_wanted_level(&self) -> i32;
    fn maximum_chaos(&self) -> i32;
    fn set_maximum_wanted_level(&mut self, level: i32);
    fn flush_pending(&mut self) {}

    fn focused(&self) -> Option<PlayerPedRef<'_>> {
        self.player(self.player_in_focus() as i32)
    }

    fn focused_mut(&mut self) -> Option<PlayerPedMut<'_>> {
        let index = self.player_in_focus() as i32;
        self.player_mut(index)
    }

    fn find_by_pool_handle(&self, handle: i32) -> Option<PlayerPedRef<'_>> {
        (0..MAX_PLAYERS)
            .filter_map(|index| self.player(index as i32))
            .find(|player| player.state.pool_handle == handle)
    }

    fn find_by_pool_handle_mut(&mut self, handle: i32) -> Option<PlayerPedMut<'_>> {
        let index = (0..MAX_PLAYERS).find(|index| {
            self.player(*index as i32)
                .is_some_and(|player| player.state.pool_handle == handle)
        })?;
        self.player_mut(index as i32)
    }
}

impl CWorldPlayers<'_, '_> {
    fn slot(index: i32) -> Option<usize> {
        usize::try_from(index.max(0))
            .ok()
            .filter(|index| *index < MAX_PLAYERS)
    }

    pub fn player(&self, index: i32) -> Option<PlayerPedRef<'_>> {
        WorldPlayerAccess::player(self, index)
    }

    pub fn player_mut(&mut self, index: i32) -> Option<PlayerPedMut<'_>> {
        WorldPlayerAccess::player_mut(self, index)
    }

    pub fn focused(&self) -> Option<PlayerPedRef<'_>> {
        WorldPlayerAccess::focused(self)
    }

    pub fn focused_mut(&mut self) -> Option<PlayerPedMut<'_>> {
        WorldPlayerAccess::focused_mut(self)
    }

    pub fn player_entity(&self, index: i32) -> Option<Entity> {
        WorldPlayerAccess::player_entity(self, index)
    }

    pub fn relation(&self, index: i32) -> Option<InVehicle> {
        let index = Self::slot(index)?;
        let entity = self.state.as_ref()?.entities[index]?;
        self.players
            .get(entity)
            .ok()
            .and_then(|(_, _, _, _, _, _, relation)| relation.copied())
    }

    pub fn set_relation(&mut self, index: i32, relation: InVehicle) -> bool {
        let Some(entity) = self.player_entity(index) else {
            return false;
        };
        queue_set_vehicle_relation(&mut self.commands, &self.relations, entity, relation)
    }

    pub fn clear_relation(&mut self, index: i32) -> bool {
        let Some(entity) = self.player_entity(index) else {
            return false;
        };
        queue_clear_vehicle_relation(&mut self.commands, entity);
        true
    }

    pub fn get_pair_mut(
        &mut self,
        idx0: i32,
        idx1: i32,
    ) -> (Option<PlayerPedMut<'_>>, Option<PlayerPedMut<'_>>) {
        let Some(s0) = Self::slot(idx0) else { return (None, None); };
        let Some(s1) = Self::slot(idx1).filter(|s1| *s1 != s0) else {
            return (self.player_mut(idx0), None);
        };
        let indexed = self.state.as_ref().map_or([None; MAX_PLAYERS], |state| state.entities);
        let (pending0, pending1) = if s0 < s1 {
            let (left, right) = self.pending.players.split_at_mut(s1);
            (&mut left[s0], &mut right[0])
        } else {
            let (left, right) = self.pending.players.split_at_mut(s0);
            (&mut right[0], &mut left[s1])
        };
        match (pending0.as_mut(), pending1.as_mut()) {
            (Some(p0), Some(p1)) => (Some(p0.as_mut()), Some(p1.as_mut())),
            (Some(p0), None) => {
                let p1 = indexed[s1].and_then(|entity| self.players.get_mut(entity).ok())
                    .map(|(_, ped, vitality, locomotion, animation, state, _)| PlayerPedMut {
                        ped: ped.into_inner(), vitality: vitality.into_inner(), locomotion: locomotion.into_inner(), animation: animation.into_inner(), state: state.into_inner()
                    });
                (Some(p0.as_mut()), p1)
            }
            (None, Some(p1)) => {
                let p0 = indexed[s0].and_then(|entity| self.players.get_mut(entity).ok())
                    .map(|(_, ped, vitality, locomotion, animation, state, _)| PlayerPedMut {
                        ped: ped.into_inner(), vitality: vitality.into_inner(), locomotion: locomotion.into_inner(), animation: animation.into_inner(), state: state.into_inner()
                    });
                (p0, Some(p1.as_mut()))
            }
            (None, None) => {
                let Some(e0) = indexed[s0] else { return (None, None); };
                if let Some(e1) = indexed[s1].filter(|e1| *e1 != e0 && self.players.contains(*e1)) {
                    if let Ok([(_, p0, v0, l0, a0, s0, _), (_, p1, v1, l1, a1, s1, _)]) = self.players.get_many_mut([e0, e1]) {
                        (Some(PlayerPedMut { ped: p0.into_inner(), vitality: v0.into_inner(), locomotion: l0.into_inner(), animation: a0.into_inner(), state: s0.into_inner() }),
                         Some(PlayerPedMut { ped: p1.into_inner(), vitality: v1.into_inner(), locomotion: l1.into_inner(), animation: a1.into_inner(), state: s1.into_inner() }))
                    } else { (None, None) }
                } else {
                    let p0 = self.players.get_mut(e0).ok().map(|(_, ped, vitality, locomotion, animation, state, _)| PlayerPedMut {
                        ped: ped.into_inner(), vitality: vitality.into_inner(), locomotion: locomotion.into_inner(), animation: animation.into_inner(), state: state.into_inner()
                    });
                    (p0, None)
                }
            }
        }
    }

    pub fn remove_player(&mut self, index: usize) {
        WorldPlayerAccess::remove_player(self, index);
    }

    pub fn flush_pending(&mut self) {
        WorldPlayerAccess::flush_pending(self);
    }

    pub fn set_maximum_wanted_level(&mut self, level: i32) {
        WorldPlayerAccess::set_maximum_wanted_level(self, level);
    }

    pub fn player_in_focus(&self) -> usize {
        WorldPlayerAccess::player_in_focus(self)
    }

    pub fn set_player_in_focus(&mut self, index: usize) {
        WorldPlayerAccess::set_player_in_focus(self, index);
    }

    pub fn for_each_mut(&mut self, mut f: impl FnMut(PlayerPedMut<'_>)) {
        for player in self.iter_mut() {
            f(player);
        }
    }

    fn remove_near_ped_handle_from_other_players(&mut self, handle: i32, removed_slot: usize) {
        for slot in 0..MAX_PLAYERS {
            if slot == removed_slot {
                continue;
            }
            if let Some(mut player) = self.player_mut(slot as i32) {
                crate::peds::PedNearby::remove_near_ped_reference(&mut player.ped, handle);
            }
        }
    }

    /// Number of player slots in the re3 world-player array.
    pub const fn slot_count(&self) -> usize {
        MAX_PLAYERS
    }

    /// Move boot-time staged players out of the short-lived adapter.  The
    /// returned array is immediately materialized as ECS entities by the
    /// bootstrap system; it is not retained as world gameplay state.
    pub fn take_pending_players(&mut self) -> [Option<CPlayerPed>; MAX_PLAYERS] {
        std::mem::take(&mut self.pending.players)
    }

    /// Iterate live player components in stable player-slot order.
    pub fn iter(&self) -> impl Iterator<Item = PlayerPedRef<'_>> + '_ {
        (0..MAX_PLAYERS).filter_map(|index| self.player(index as i32))
    }

    /// Preserve the slot number while reading live player components.
    pub fn iter_slots(&self) -> impl Iterator<Item = (usize, Option<PlayerPedRef<'_>>)> + '_ {
        (0..MAX_PLAYERS).map(|index| (index, self.player(index as i32)))
    }

    /// Iterate the authoritative player components while also exposing
    /// same-system CREATE_PLAYER replacements that are still pending command
    /// application. Query entries no longer present in the slot index (for
    /// example a deferred despawn) are excluded.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = PlayerPedMut<'_>> + '_ {
        let indexed_entities = self
            .state
            .as_ref()
            .map_or([None; MAX_PLAYERS], |state| state.entities);
        let mut pending_mask = [false; MAX_PLAYERS];
        for (index, pending) in self.pending.players.iter().enumerate() {
            pending_mask[index] = pending.is_some();
        }
        let pending = self.pending.players.iter_mut().flatten().map(CPlayerPed::as_mut);
        let live = self
            .players
            .iter_mut()
            .filter_map(move |(entity, ped, vitality, locomotion, animation, state, _)| {
                let slot = indexed_entities
                    .iter()
                    .position(|indexed| *indexed == Some(entity))?;
                (!pending_mask[slot]).then(|| PlayerPedMut { ped: ped.into_inner(), vitality: vitality.into_inner(), locomotion: locomotion.into_inner(), animation: animation.into_inner(), state: state.into_inner() })
            });
        pending.chain(live)
    }

    pub fn find_by_pool_handle(&self, handle: i32) -> Option<PlayerPedRef<'_>> {
        WorldPlayerAccess::find_by_pool_handle(self, handle)
    }

    pub fn find_by_pool_handle_mut(&mut self, handle: i32) -> Option<PlayerPedMut<'_>> {
        WorldPlayerAccess::find_by_pool_handle_mut(self, handle)
    }
}

impl WorldPlayerAccess for CWorldPlayers<'_, '_> {
    fn player(&self, index: i32) -> Option<PlayerPedRef<'_>> {
        let index = Self::slot(index)?;
        if let Some(player) = self.pending.players[index].as_ref() {
            return Some(player.as_ref());
        }
        let entity = self.state.as_ref()?.entities[index]?;
        self.players.get(entity).ok().map(|(_, ped, vitality, locomotion, animation, state, _)| PlayerPedRef { ped, vitality, locomotion, animation, state })
    }

    fn player_mut(&mut self, index: i32) -> Option<PlayerPedMut<'_>> {
        let index = Self::slot(index)?;
        if self.pending.players[index].is_some() {
            return self.pending.players[index].as_mut().map(CPlayerPed::as_mut);
        }
        let entity = self.state.as_ref()?.entities[index]?;
        self.players
            .get_mut(entity)
            .ok()
            .map(|(_, ped, vitality, locomotion, animation, state, _)| PlayerPedMut { ped: ped.into_inner(), vitality: vitality.into_inner(), locomotion: locomotion.into_inner(), animation: animation.into_inner(), state: state.into_inner() })
    }

    fn replace_player(&mut self, index: usize, player: CPlayerPed) {
        if index >= MAX_PLAYERS {
            bevy::log::warn_once!(target: "re3bevy::player_lifecycle", index, "player slot outside CWorld::Players capacity");
            return;
        }
        if self.state.is_none() {
            bevy::log::warn_once!(target: "re3bevy::player_lifecycle", "required CWorldPlayerState unavailable during player mutation");
            return;
        }
        let removed_handle = self.player(index as i32).map(|ped| ped.state.pool_handle);
        let old_entity = self.state.as_mut().and_then(|state| state.entities[index].take());
        if let Some(removed_handle) = removed_handle {
            self.remove_near_ped_handle_from_other_players(removed_handle, index);
        }
        if let Some(old_entity) = old_entity {
            self.commands.entity(old_entity).despawn();
        }
        let entity = self.commands.spawn(PedEntity).id();
        if let Some(state) = self.state.as_mut() {
            state.entities[index] = Some(entity);
        }
        self.pending.players[index] = Some(player);
    }

    fn remove_player(&mut self, index: usize) {
        if index >= MAX_PLAYERS {
            bevy::log::warn_once!(target: "re3bevy::player_lifecycle", index, "player slot outside CWorld::Players capacity");
            return;
        }
        let removed_handle = self.player(index as i32).map(|ped| ped.state.pool_handle);
        self.pending.players[index] = None;
        let Some(state) = self.state.as_mut() else {
            bevy::log::warn_once!(target: "re3bevy::player_lifecycle", "required CWorldPlayerState unavailable during player mutation");
            return;
        };
        let entity = state.entities[index].take();
        if let Some(removed_handle) = removed_handle {
            self.remove_near_ped_handle_from_other_players(removed_handle, index);
        }
        if let Some(entity) = entity {
            self.commands.entity(entity).despawn();
        }
    }

    fn player_entity(&self, index: i32) -> Option<Entity> {
        let index = Self::slot(index)?;
        self.state.as_ref()?.entities[index]
    }

    fn vehicle_relation(&self, index: i32) -> Option<InVehicle> {
        CWorldPlayers::relation(self, index)
    }

    fn player_in_focus(&self) -> usize {
        self.state.as_ref().map_or(0, |state| state.player_in_focus)
    }

    fn set_player_in_focus(&mut self, index: usize) {
        if let Some(state) = self.state.as_mut() {
            state.player_in_focus = index.min(MAX_PLAYERS.saturating_sub(1));
        }
    }

    fn maximum_wanted_level(&self) -> i32 {
        self.state
            .as_ref()
            .map_or(crate::core::Wanted::MAXIMUM_WANTED_LEVEL, |state| {
                state.maximum_wanted_level
            })
    }

    fn maximum_chaos(&self) -> i32 {
        self.state
            .as_ref()
            .map_or(crate::core::Wanted::N_MAXIMUM_WANTED_LEVEL, |state| {
                state.maximum_chaos
            })
    }

    fn set_maximum_wanted_level(&mut self, level: i32) {
        let mut wanted = CWanted::default();
        wanted.set_maximum_wanted_level(level);
        let Some(state) = self.state.as_mut() else {
            bevy::log::warn_once!(target: "re3bevy::player_lifecycle", "required CWorldPlayerState unavailable during player mutation");
            return;
        };
        state.maximum_wanted_level = wanted.maximum_wanted_level;
        state.maximum_chaos = wanted.maximum_chaos;
        let maximum_wanted_level = state.maximum_wanted_level;
        let maximum_chaos = state.maximum_chaos;
        for index in 0..MAX_PLAYERS {
            if let Some(player) = self.player_mut(index as i32) {
                player.state.wanted.maximum_wanted_level = maximum_wanted_level;
                player.state.wanted.maximum_chaos = maximum_chaos;
            }
        }
    }

    fn flush_pending(&mut self) {
        for index in 0..MAX_PLAYERS {
            if self.pending.players[index].is_none() {
                continue;
            }
            let Some(entity) = self.state.as_ref().and_then(|state| state.entities[index]) else {
                bevy::log::warn_once!(target: "re3bevy::player_lifecycle", index, "pending player lacks its required materialization identity");
                continue;
            };
            if let Some(player) = self.pending.players[index].take() {
                self.commands.entity(entity).insert(player.into_bundle());
            }
        }
    }
}

/// A player may own the driver control path only while re3 would still be in
/// PED_DRIVING and the car has not already transitioned to STATUS_WRECKED.
/// Keeping this boundary explicit prevents an InVehicle relation retained by
/// SetDead from resurrecting a wreck on the following frame.
pub(crate) fn player_can_drive_related_vehicle(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    relation: InVehicle,
    vehicle_status: eEntityStatus,
) -> bool {
    relation.seat == VehicleSeat::Driver
        && player.ped.ped_state == PedState::PED_DRIVING
        && !player.ped.dying_or_dead()
        && vehicle_status != eEntityStatus::STATUS_WRECKED
}

/// Short-lived non-ECS adapter used only while boot effects are evaluated
/// before the gameplay schedule owns a queryable World. It is materialized as
/// Bevy player entities before entering `InGame` and is never inserted as a
/// Resource.
#[derive(Debug, Clone)]
pub struct StagedWorldPlayers {
    pub players: [Option<CPlayerPed>; MAX_PLAYERS],
    /// Temporary ECS-shaped relation metadata used only before bootstrap
    /// materializes player components. It is never inserted as a Resource.
    pub relations: [Option<InVehicle>; MAX_PLAYERS],
    pub player_in_focus: usize,
    pub maximum_wanted_level: i32,
    pub maximum_chaos: i32,
}

impl Default for StagedWorldPlayers {
    fn default() -> Self {
        Self {
            players: [None, None],
            relations: [None, None],
            player_in_focus: 0,
            maximum_wanted_level: crate::core::Wanted::MAXIMUM_WANTED_LEVEL,
            maximum_chaos: crate::core::Wanted::N_MAXIMUM_WANTED_LEVEL,
        }
    }
}

impl StagedWorldPlayers {
    pub fn take_pending_players(&mut self) -> [Option<CPlayerPed>; MAX_PLAYERS] {
        self.relations = [None, None];
        std::mem::take(&mut self.players)
    }

    pub fn set_vehicle_relation(&mut self, index: usize, relation: InVehicle) {
        if let Some(slot) = self.relations.get_mut(index) {
            *slot = Some(relation);
        }
    }
}

impl WorldPlayerAccess for StagedWorldPlayers {
    fn player(&self, index: i32) -> Option<PlayerPedRef<'_>> {
        let index = index.max(0) as usize;
        self.players.get(index).and_then(Option::as_ref).map(CPlayerPed::as_ref)
    }

    fn player_mut(&mut self, index: i32) -> Option<PlayerPedMut<'_>> {
        let index = index.max(0) as usize;
        self.players.get_mut(index).and_then(Option::as_mut).map(CPlayerPed::as_mut)
    }

    fn replace_player(&mut self, index: usize, player: CPlayerPed) {
        if let Some(slot) = self.players.get_mut(index) {
            *slot = Some(player);
        }
    }

    fn remove_player(&mut self, index: usize) {
        if let Some(slot) = self.players.get_mut(index) {
            *slot = None;
        }
    }

    fn player_entity(&self, _index: i32) -> Option<Entity> {
        None
    }

    fn vehicle_relation(&self, index: i32) -> Option<InVehicle> {
        let index = index.max(0) as usize;
        self.relations.get(index).copied().flatten()
    }

    fn player_in_focus(&self) -> usize {
        self.player_in_focus
    }

    fn set_player_in_focus(&mut self, index: usize) {
        self.player_in_focus = index.min(MAX_PLAYERS.saturating_sub(1));
    }

    fn maximum_wanted_level(&self) -> i32 {
        self.maximum_wanted_level
    }

    fn maximum_chaos(&self) -> i32 {
        self.maximum_chaos
    }

    fn set_maximum_wanted_level(&mut self, level: i32) {
        let mut wanted = CWanted::default();
        wanted.set_maximum_wanted_level(level);
        self.maximum_wanted_level = wanted.maximum_wanted_level;
        self.maximum_chaos = wanted.maximum_chaos;
        for player in self.players.iter_mut().flatten() {
            player.state.wanted.maximum_wanted_level = self.maximum_wanted_level;
            player.state.wanted.maximum_chaos = self.maximum_chaos;
        }
    }
}

// CPlayerPed::ProcessControl owns on-foot input, wanted, weapons and jump
// behavior. Collision systems consume the ground-support helpers below;
// PedSimulationPlugin registers the ordered player-control systems.

#[allow(unused_imports)]
use crate::animation::AnimManager::CAnimManager;
#[allow(unused_imports)]
use crate::animation::PedVehicleAnimation::{CPedVehicleAnimation, PedVehicleAnimationPhase};
#[allow(unused_imports)]
use crate::audio::DMAudio::cDMAudio;
#[allow(unused_imports)]
use crate::audio::soundlist::{
    SOUND_FALL_COLLAPSE, SOUND_FALL_LAND, SOUND_FIGHT_HEADBUTT_35, SOUND_FIGHT_KICK_34,
    SOUND_FIGHT_KICK_44, SOUND_FIGHT_KNEE_OR_KICK_43, SOUND_FIGHT_PUNCH_33, SOUND_FIGHT_PUNCH_39,
    SOUND_FIGHT_PUNCH_OR_KICK_BELOW_40, SOUND_PED_HIT, SOUND_WEAPON_AK47_BULLET_ECHO,
    SOUND_WEAPON_BAT_ATTACK, SOUND_WEAPON_HIT_PED, SOUND_WEAPON_HIT_VEHICLE,
    SOUND_WEAPON_M16_BULLET_ECHO, SOUND_WEAPON_RELOAD, SOUND_WEAPON_SHOT_FIRED,
    SOUND_WEAPON_UZI_BULLET_ECHO,
};
#[allow(unused_imports)]
use crate::collision::ColModel::{CColBox, CColLine, CColPoint, CColSphere, CColVector};
#[allow(unused_imports)]
use crate::collision::Collision::CCollision;
#[allow(unused_imports)]
use crate::collision::Collision::{
    PlayerDynamicGroundSupport, PlayerDynamicMotionCandidates, PlayerStaticGroundSupport,
    ScriptObjectCollisionSnapshot, col_bound_spheres_overlap, resolve_player_dynamic_motion,
    resolve_player_ped_static_collision_result, script_object_collision_snapshots,
};
#[allow(unused_imports)]
use crate::control::Bridge::CBridge;
#[allow(unused_imports)]
use crate::control::Darkel::CDarkel;
#[allow(unused_imports)]
use crate::control::Replay::CReplay;
#[allow(unused_imports)]
use crate::control::Replay::ReplayMode;
#[allow(unused_imports)]
use crate::core::Camera::{CCamera, CameraScriptMode};
#[allow(unused_imports)]
use crate::core::EventList::{CEventList, EEventEntity, EEventType};
#[allow(unused_imports)]
use crate::core::FileLoader::CFileLoader;
#[allow(unused_imports)]
use crate::core::Frontend::{CMenuManager, CONTROL_STANDARD};
#[allow(unused_imports)]
use crate::core::Game::{ScriptEntityWorld, ScriptGameplayBridge, ScriptVehicleEntity};
#[allow(unused_imports)]
use crate::core::Pad::CPads;
#[allow(unused_imports)]
use crate::core::PlayerInfo::CPlayerInfo;
#[allow(unused_imports)]
use crate::core::Pools::CPoolHandle;
#[allow(unused_imports)]
use crate::core::Stats::CStats;
#[allow(unused_imports)]
use crate::entities::Entity::eEntityStatus;
#[allow(unused_imports)]
use crate::entities::Physical::GRAVITY_WORLD_UNITS_PER_SECOND_SQ;
#[allow(unused_imports)]
use crate::entities::Physical::GTA_PHYSICS_HZ;
#[allow(unused_imports)]
use crate::math::Matrix::CMatrix;
#[allow(unused_imports)]
use crate::modelinfo::ModelIndices::MI_DODO;
#[allow(unused_imports)]
use crate::modelinfo::ModelInfo::ModelInfoSlot;
#[allow(unused_imports)]
use crate::modelinfo::VehicleModelInfo::vehicle_dummy_model_position;
#[allow(unused_imports)]
use crate::objects::ParticleObject::{CParticleObjectRuntime, CRGBA, eParticleObjectType};
#[allow(unused_imports)]
use crate::peds::Ped::{CPedAnimationOffsets, eObjective};
#[allow(unused_imports)]
use crate::peds::Ped::{ped_seat_world_position, player_seat_index};
#[allow(unused_imports)]
use crate::peds::PedAI::{
    PedVehicleLineUpContext, automobile_ped_door_world_position, automobile_ped_line_up,
};
#[allow(unused_imports)]
use crate::peds::VehicleRelations::{
    PedAutomobileProfile, VehicleSeat, automobile_door_flag, queue_ped_vehicle_animation,
};
#[allow(unused_imports)]
use crate::renderer::Particle::{CParticle, ParticleEmitter};
#[allow(unused_imports)]
use crate::renderer::PointLights::{CPointLights, FOG_NONE, LIGHT_POINT};
#[allow(unused_imports)]
use crate::renderer::SpecialFX::CSpecialFX;
#[allow(unused_imports)]
use crate::renderer::Timecycle::CTimeCycle;
#[allow(unused_imports)]
use crate::renderer::WaterLevel::CWaterLevel;
#[allow(unused_imports)]
use crate::renderer::Weather::CWeather;
#[allow(unused_imports)]
use crate::vehicles::Automobile::{
    CAutomobileControlInput, CAutomobileDriveStepInput, automobile_road_contacts, eBombType,
    store_automobile_wheel_contacts,
};
#[allow(unused_imports)]
use crate::vehicles::HandlingMgr::cHandlingDataMgr;
#[allow(unused_imports)]
use crate::vehicles::Vehicle::eCarLock;
#[allow(unused_imports)]
use crate::weapons::WeaponInfo::{
    CWeaponInfoStore, WEAPONFLAG_CANAIM, WEAPONFLAG_HEAVY, WEAPONFLAG_THROW,
};
#[allow(unused_imports)]
use crate::weapons::WeaponType::{
    WEAPONTYPE_TOTAL_INVENTORY_WEAPONS, eWeaponFire, eWeaponState, script_weapon_type,
};
#[allow(unused_imports)]
use bevy::log::warn;
#[allow(unused_imports)]
use bevy::prelude::Res;
#[allow(unused_imports)]
use re3bevy_assets::assets::WorldAssets::CSurfaceTableAsset;
#[allow(unused_imports)]
use re3bevy_core::core::Timer::CTimer;
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

/// Seconds-based player-controller half of `CPhysical::ProcessControl`.
/// Collision friction is authored in native GTA move-speed units, so consume
/// last frame's accumulated friction before clearing the transient collision
/// state for the new world pass.
pub fn prepare_player_physical_for_control(ped: &mut CPed) -> bool {
    let hit_something_last_frame = ped.physical.entity.flags.has_hit_wall;
    ped.physical.move_speed.x += ped.physical.move_friction.x * GTA_PHYSICS_HZ;
    ped.physical.move_speed.y += ped.physical.move_friction.y * GTA_PHYSICS_HZ;
    ped.physical.move_speed.z += ped.physical.move_friction.z * GTA_PHYSICS_HZ;
    ped.physical.move_friction = CVector::ZERO;
    ped.physical.turn_friction = CVector::ZERO;
    ped.physical.entity.flags.has_contacted = false;
    ped.physical.entity.flags.is_in_safe_position = false;
    ped.physical.entity.flags.was_postponed = false;
    ped.physical.entity.flags.has_hit_wall = false;
    ped.physical.collision_records.clear();
    ped.physical.entity.flags.has_collided = false;
    ped.physical.damage.piece_type = 0;
    ped.physical.damage.impulse = 0.0;
    ped.physical.damage.entity = None;
    hit_something_last_frame
}

pub fn player_anim_group_for_weapon(weapon_type: eWeaponType) -> usize {
    use crate::animation::AnimAssocDefinitions::{
        ASSOCGRP_PLAYER, ASSOCGRP_PLAYER1ARMED, ASSOCGRP_PLAYER2ARMED, ASSOCGRP_PLAYERBBBAT,
        ASSOCGRP_PLAYERROCKET,
    };
    match weapon_type {
        eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => ASSOCGRP_PLAYERROCKET,
        eWeaponType::WEAPONTYPE_BASEBALLBAT => ASSOCGRP_PLAYERBBBAT,
        eWeaponType::WEAPONTYPE_COLT45 | eWeaponType::WEAPONTYPE_UZI => ASSOCGRP_PLAYER1ARMED,
        eWeaponType::WEAPONTYPE_SHOTGUN
        | eWeaponType::WEAPONTYPE_AK47
        | eWeaponType::WEAPONTYPE_M16
        | eWeaponType::WEAPONTYPE_SNIPERRIFLE
        | eWeaponType::WEAPONTYPE_FLAMETHROWER => ASSOCGRP_PLAYER2ARMED,
        _ => ASSOCGRP_PLAYER,
    }
}

pub fn reapply_player_move_anims(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    new_group: usize,
) {
    use crate::animation::{
        AnimBlendAssociation::ASSOC_DELETEFADEDOUT,
        AnimationId::{
            ANIM_STD_IDLE, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_STARTWALK, ANIM_STD_WALK,
        },
    };
    const MOVE_ANIMS: [crate::animation::AnimationId::AnimationId; 5] = [
        ANIM_STD_WALK,
        ANIM_STD_RUN,
        ANIM_STD_RUNFAST,
        ANIM_STD_IDLE,
        ANIM_STD_STARTWALK,
    ];

    for anim_id in MOVE_ANIMS {
        if let Some(pos) = clump.associations.iter().position(|a| a.anim_id == anim_id) {
            let cur = &clump.associations[pos];
            let Some(template) = manager.get_anim_association(new_group, anim_id as usize) else {
                continue;
            };
            // re3 ReApplyMoveAnims only replaces an association when the
            // target group resolves this AnimationId to a different hierarchy.
            // Common clips such as IDLE_STANCE must keep their current
            // association instead of being restarted on every strafe-group
            // transition.
            if template
                .hierarchy
                .name
                .eq_ignore_ascii_case(&cur.hierarchy.name)
            {
                continue;
            }
            let blend_delta = cur.blend_delta;
            let blend_amount = cur.blend_amount;
            if let Some(mut new_assoc) =
                manager.create_anim_association(new_group, anim_id as usize)
            {
                new_assoc.blend_delta = blend_delta;
                new_assoc.blend_amount = blend_amount;

                // CAnimManager::AddAnimation synchronises a new movement
                // association to the first movement clip already on the
                // clump. This preserves gait phase when ProcessAnimGroups
                // swaps player/playerleft/playerright/playerback groups.
                if new_assoc.is_movement() {
                    if let Some(sync) = clump
                        .associations
                        .iter()
                        .find(|association| association.is_movement())
                    {
                        new_assoc.sync_animation(sync);
                    }
                    new_assoc.set_run();
                } else {
                    new_assoc.start(0.0);
                }

                let cur_mut = &mut clump.associations[pos];
                cur_mut.blend_delta = -1000.0;
                cur_mut.flags |= ASSOC_DELETEFADEDOUT;

                // Native CAnimManager::AddAnimation prepends to the clump list.
                clump.associations.insert(0, new_assoc);
            }
        }
    }
}

fn blend_player_idle(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    group: usize,
    delta: f32,
) {
    use crate::animation::AnimationId::ANIM_STD_IDLE;

    if clump
        .associations
        .iter()
        .any(|association| association.anim_id == ANIM_STD_IDLE)
    {
        return;
    }
    blend_player_animation(clump, manager, group, ANIM_STD_IDLE, delta);
}

fn blend_player_tired_idle(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    delta: f32,
) {
    use crate::animation::{AnimAssocDefinitions::ASSOCGRP_STD, AnimationId::ANIM_STD_IDLE_TIRED};

    if clump
        .associations
        .iter()
        .any(|association| association.anim_id == ANIM_STD_IDLE_TIRED)
    {
        return;
    }
    blend_player_animation(clump, manager, ASSOCGRP_STD, ANIM_STD_IDLE_TIRED, delta);
}

/// Player-locomotion equivalent of re3 `CAnimManager::BlendAnimation`.
/// Native scans the head-first clump list, keeps the last movement clip it
/// encounters as the sync source, fades matching partial/full-body peers, then
/// prepends the new association through AddAnimationAndSync.
fn blend_player_animation(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    group: usize,
    animation_id: crate::animation::AnimationId::AnimationId,
    delta: f32,
) {
    use crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT;

    let Some(mut template) = manager.create_anim_association(group, animation_id as usize) else {
        return;
    };
    let is_movement = template.is_movement();
    let is_partial = template.is_partial();
    let mut found = None;
    let mut movement_sync = None;
    let mut removed_previous = false;

    for (index, association) in clump.associations.iter_mut().enumerate() {
        if is_movement && association.is_movement() {
            movement_sync = Some((association.current_time, association.hierarchy.total_length));
        }
        if association.anim_id == animation_id {
            found = Some(index);
            continue;
        }
        if association.is_partial() == is_partial {
            if association.blend_amount > 0.0 {
                let blend_delta = -delta * association.blend_amount;
                if blend_delta < association.blend_delta || !is_partial {
                    association.blend_delta = blend_delta;
                }
            } else {
                association.blend_delta = -1.0;
            }
            association.flags |= ASSOC_DELETEFADEDOUT;
            removed_previous = true;
        }
    }

    if let Some(index) = found {
        let association = &mut clump.associations[index];
        association.blend_delta = (1.0 - association.blend_amount) * delta;
        if !association.is_running()
            && association.current_time == association.hierarchy.total_length
        {
            association.start(0.0);
        }
        return;
    }

    if is_movement {
        if let Some((sync_time, sync_length)) = movement_sync {
            if sync_length > 0.0 && template.hierarchy.total_length > 0.0 {
                template
                    .set_current_time(sync_time / sync_length * template.hierarchy.total_length);
            }
            template.set_run();
        } else {
            template.start(0.0);
        }
    } else {
        template.start(0.0);
    }

    if !removed_previous && !is_partial {
        template.blend_amount = 1.0;
    } else {
        template.blend_amount = 0.0;
        template.blend_delta = delta;
    }
    clump.associations.insert(0, template);
}

/// Direct Rust adaptation of CPlayerPed::SetRealMoveAnim plus
/// RpAnimBlendClumpUpdateAnimations/FrameUpdateCallBackNonSkinned velocity extraction.
pub fn update_player_movement_animation(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    manager: &CAnimManager,
    file_loader: &CFileLoader,
    dt: f32,
    mouse_strafe_control: bool,
    infinite_sprint: bool,
    now_ms: u32,
    timer: &mut CTimer,
) -> CVector {
    use crate::animation::{
        AnimBlendAssociation::{ASSOC_DELETEFADEDOUT, ASSOC_IDLE, ASSOC_MOVEMENT, ASSOC_RUNNING},
        AnimationId::{
            ANIM_STD_IDLE, ANIM_STD_IDLE_TIRED, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_RUNSTOP1,
            ANIM_STD_RUNSTOP2, ANIM_STD_STARTWALK, ANIM_STD_WALK,
        },
        FrameUpdate::frame_update_callback_with_velocity_extraction_overlay_non_skinned,
    };
    let desired_group = player_anim_group_for_control(
        player.ped.current_weapon,
        player.state.walk_angle,
        mouse_strafe_control,
    );
    let anim_group = player.ped.anim_group as usize;
    // Native PlayerControl1stPersonRunAround does not call SetRealMoveAnim
    // while PED_FIGHT. ProcessAnimGroups and the clump update still run.
    let run_set_real_move_anim =
        !(mouse_strafe_control && player.ped.ped_state == PedState::PED_FIGHT);
    let Ok(model) = usize::try_from(player.ped.physical.entity.model_index) else {
        bevy::log::warn_once!("player_animation_invalid_original_model");
        return CVector::ZERO;
    };
    if !player.animation.ensure_original_frames(model, file_loader) {
        return CVector::ZERO;
    }
    let clump = &mut player.animation.movement_anim_clump;
    if player.ped.flags.reset_walk_anims {
        for association in &mut clump.associations {
            if matches!(
                association.anim_id,
                ANIM_STD_WALK | ANIM_STD_RUN | ANIM_STD_RUNFAST
            ) {
                association.set_current_time(0.0);
            }
        }
        player.ped.flags.reset_walk_anims = false;
    }

    // PlayerControlZelda calls SetRealMoveAnim even while PED_JUMP. Keep the
    // locomotion associations advancing/blending during launch and flight;
    // CPed::UpdatePosition is what prevents their extracted delta from driving
    // a non-standing physical ped.
    let moving = player.state.move_speed > 0.0;
    let had_idle = clump
        .associations
        .iter()
        .any(|association| matches!(association.anim_id, ANIM_STD_IDLE | ANIM_STD_IDLE_TIRED));
    let run_stop_running = clump.associations.iter().any(|association| {
        matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2)
            && association.is_running()
    });
    let run_stop_fading_in = clump.associations.iter().any(|association| {
        matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2)
            && association.blend_delta >= 0.0
    });

    // CPlayerPed::SetRealMoveAnim gives RUNSTOP1/2 exclusive control until
    // the non-looping stop clip finishes, then fades back to idle.
    if !run_set_real_move_anim {
        // Leave locomotion associations untouched this frame, matching the
        // mouse-control PED_FIGHT branch.
    } else if run_stop_running {
        // The association is advanced by the common update block below.
    } else if run_stop_fading_in {
        for association in &mut clump.associations {
            if matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2) {
                association.flags |= ASSOC_DELETEFADEDOUT;
                association.blend_amount = 1.0;
                association.blend_delta = -8.0;
            }
        }
        // Native SetRealMoveAnim restores m_headingRate as soon as the
        // non-looping run-stop is no longer running.
        player.locomotion.heading_rate = 15.0;
        if player.state.current_stamina < 0.0 {
            blend_player_tired_idle(clump, manager, 8.0);
        } else {
            blend_player_idle(clump, manager, anim_group, 8.0);
        }
        if let Some(idle) = clump
            .associations
            .iter_mut()
            .find(|association| matches!(association.anim_id, ANIM_STD_IDLE | ANIM_STD_IDLE_TIRED))
        {
            idle.blend_amount = 0.0;
            idle.blend_delta = 8.0;
        }
    } else if !moving
        && !clump
            .associations
            .iter()
            .any(|association| association.anim_id == ANIM_STD_RUNFAST)
    {
        let current_idle = clump
            .associations
            .iter()
            .find(|association| matches!(association.anim_id, ANIM_STD_IDLE | ANIM_STD_IDLE_TIRED))
            .map(|association| association.anim_id);
        if player.state.current_stamina < 0.0 && player.ped.ped_state != PedState::PED_FIGHT {
            if current_idle != Some(ANIM_STD_IDLE_TIRED) {
                blend_player_tired_idle(clump, manager, 4.0);
            }
        } else if current_idle != Some(ANIM_STD_IDLE) {
            blend_player_idle(clump, manager, anim_group, 4.0);
        }
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
    } else {
        if had_idle {
            if let Some(start) = clump
                .associations
                .iter_mut()
                .find(|association| association.anim_id == ANIM_STD_STARTWALK)
            {
                start.blend_amount = 1.0;
                start.blend_delta = 0.0;
            } else {
                add_player_movement_association(clump, manager, anim_group, ANIM_STD_STARTWALK);
            }
            for association in &mut clump.associations {
                if matches!(association.anim_id, ANIM_STD_WALK | ANIM_STD_RUN) {
                    association.set_current_time(0.0);
                }
            }
            clump.associations.retain(|association| {
                !matches!(
                    association.anim_id,
                    ANIM_STD_IDLE | ANIM_STD_IDLE_TIRED | ANIM_STD_RUNFAST
                )
            });
            player.ped.move_state = eMoveState::PEDMOVE_WALK;
        }
        let had_finished_run_stop = clump.associations.iter().any(|association| {
            matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2)
        });
        if had_finished_run_stop {
            clump.associations.retain(|association| {
                !matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2)
            });
            player.locomotion.heading_rate = 15.0;
        }
        ensure_player_move_association_zero_blend(clump, manager, anim_group, ANIM_STD_WALK);
        ensure_player_move_association_zero_blend(clump, manager, anim_group, ANIM_STD_RUN);
        let start_running = clump.associations.iter().any(|association| {
            association.anim_id == ANIM_STD_STARTWALK && association.is_running()
        });
        if !start_running {
            clump
                .associations
                .retain(|association| association.anim_id != ANIM_STD_STARTWALK);
            for association in &mut clump.associations {
                if matches!(association.anim_id, ANIM_STD_WALK | ANIM_STD_RUN) {
                    association.set_run();
                }
            }
        }
        let sprint_index = clump
            .associations
            .iter()
            .position(|association| association.anim_id == ANIM_STD_RUNFAST);
        if player.ped.move_state == eMoveState::PEDMOVE_SPRINT {
            let sprint_fading_out =
                sprint_index.is_some_and(|index| clump.associations[index].blend_delta < 0.0);
            if player.state.current_stamina < 0.0
                && (player.state.current_stamina <= -150.0 || sprint_index.is_none() || sprint_fading_out)
            {
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
            }
            if start_running {
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
            }
        }
        let sprint_stopping = sprint_index.is_some()
            && (player.ped.move_state != eMoveState::PEDMOVE_SPRINT || player.state.move_speed < 0.4);
        if sprint_stopping {
            let sprint = &clump.associations[sprint_index.expect("sprint association exists")];
            let sprint_blend_amount = sprint.blend_amount;
            let sprint_blend_delta = sprint.blend_delta;
            let sprint_current_time = sprint.current_time;
            let sprint_total_length = sprint.hierarchy.total_length;

            if sprint_blend_amount == 0.0 {
                let sprint =
                    &mut clump.associations[sprint_index.expect("sprint association exists")];
                sprint.blend_delta = -1000.0;
                sprint.flags |= ASSOC_DELETEFADEDOUT;
            } else if sprint_blend_delta >= 0.0 || sprint_blend_amount >= 0.8 {
                if should_start_player_run_stop(
                    player.ped.move_state,
                    player.state.move_speed,
                    sprint_blend_amount,
                    sprint_blend_delta,
                ) {
                    let stop_id = if sprint_total_length > 0.0
                        && sprint_current_time / sprint_total_length < 0.5
                    {
                        ANIM_STD_RUNSTOP1
                    } else {
                        ANIM_STD_RUNSTOP2
                    };
                    add_player_standard_association(clump, manager, stop_id);
                    if let Some(stop) = clump
                        .associations
                        .iter_mut()
                        .find(|association| association.anim_id == stop_id)
                    {
                        stop.blend_amount = 1.0;
                        stop.blend_delta = 0.0;
                    }
                    // RestoreHeadingRateCB owns this lock in re3. The player
                    // PEDSTAT heading-change rate is 15 degrees per GTA tick.
                    player.locomotion.heading_rate = 0.0;
                    for association in &mut clump.associations {
                        match association.anim_id {
                            ANIM_STD_RUNFAST => {
                                association.blend_delta = -1000.0;
                                association.flags |= ASSOC_DELETEFADEDOUT;
                            }
                            ANIM_STD_WALK | ANIM_STD_RUN => {
                                association.flags &= !ASSOC_RUNNING;
                                association.blend_amount = 0.0;
                                association.blend_delta = 0.0;
                            }
                            _ => {}
                        }
                    }
                } else if sprint_blend_delta >= 0.0 {
                    for association in &mut clump.associations {
                        match association.anim_id {
                            ANIM_STD_RUNFAST => {
                                association.blend_delta = -1.0;
                                association.flags |= ASSOC_DELETEFADEDOUT;
                            }
                            ANIM_STD_RUN => association.blend_delta = 1.0,
                            _ => {}
                        }
                    }
                }
            } else if player.state.move_speed < 1.0 {
                for association in &mut clump.associations {
                    match association.anim_id {
                        ANIM_STD_RUNFAST => association.blend_delta = -8.0,
                        ANIM_STD_RUN => association.blend_delta = 8.0,
                        _ => {}
                    }
                }
            }
        } else if start_running {
            for association in &mut clump.associations {
                if matches!(association.anim_id, ANIM_STD_WALK | ANIM_STD_RUN) {
                    association.flags &= !ASSOC_RUNNING;
                    association.blend_amount = 0.0;
                }
            }
        } else if player.ped.move_state == eMoveState::PEDMOVE_SPRINT {
            if let Some(index) = sprint_index {
                if clump.associations[index].blend_delta < 0.0 {
                    clump.associations[index].blend_delta = 2.0;
                    if let Some(run) = clump
                        .associations
                        .iter_mut()
                        .find(|association| association.anim_id == ANIM_STD_RUN)
                    {
                        run.blend_delta = -2.0;
                    }
                }
            } else {
                for association in &mut clump.associations {
                    match association.anim_id {
                        ANIM_STD_WALK => association.blend_amount = 0.0,
                        ANIM_STD_RUN => {
                            association.blend_amount = 1.0;
                        }
                        _ => {}
                    }
                }
                // re3 uses BlendAnimation here, not AddAnimation. That fades
                // WALK/RUN with delete-on-fade and synchronises RUNFAST to the
                // last movement association in the head-first clump list.
                blend_player_animation(clump, manager, anim_group, ANIM_STD_RUNFAST, 2.0);
            }
        } else {
            player.ped.move_state = if player.state.move_speed < 1.0 {
                eMoveState::PEDMOVE_WALK
            } else {
                eMoveState::PEDMOVE_RUN
            };
            for association in &mut clump.associations {
                match association.anim_id {
                    ANIM_STD_WALK => {
                        association.blend_amount = if player.state.move_speed < 1.0 {
                            1.0
                        } else if player.state.move_speed < 2.0 {
                            2.0 - player.state.move_speed
                        } else {
                            0.0
                        };
                    }
                    ANIM_STD_RUN => {
                        association.blend_amount = if player.state.move_speed < 1.0 {
                            0.0
                        } else if player.state.move_speed < 2.0 {
                            player.state.move_speed - 1.0
                        } else {
                            1.0
                        };
                    }
                    _ => {}
                }
            }
        }
    }

    // CPlayerPed::ProcessAnimGroups runs after PlayerControl* has called
    // SetRealMoveAnim. ReApplyMoveAnims therefore replaces the associations
    // created/selected above, rather than choosing the new group beforehand.
    if player.ped.anim_group != desired_group as i32 {
        player.ped.anim_group = desired_group as i32;
        reapply_player_move_anims(clump, manager, desired_group);
    }

    let mut index = 0;
    while index < clump.associations.len() {
        if clump.associations[index].update_blend(dt) {
            index += 1;
        } else {
            clump.associations.remove(index);
        }
    }
    let transform = frame_update_callback_with_velocity_extraction_overlay_non_skinned(
        clump,
        0,
        true,
        player.state.jump_association.as_mut(),
    );
    let mut total_length = 0.0;
    let mut total_blend = 0.0;
    for association in &clump.associations {
        if association.flags & ASSOC_MOVEMENT != 0 && association.speed != 0.0 {
            total_length +=
                association.hierarchy.total_length / association.speed * association.blend_amount;
            total_blend += association.blend_amount;
        }
    }
    let relative_speed = if total_length == 0.0 {
        1.0
    } else {
        total_blend / total_length
    };
    for association in &mut clump.associations {
        association.update_time(dt, relative_speed);
    }
    let idle_blend = clump
        .associations
        .iter()
        .find(|a| a.flags & ASSOC_IDLE != 0)
        .map_or(0.0, |a| a.blend_amount);
    if idle_blend < 1.0 {
        if let Some(walk_run) = clump
            .associations
            .iter()
            .find(|a| (a.flags & ASSOC_MOVEMENT != 0) && a.blend_amount > 0.5)
        {
            if walk_run.hierarchy.total_length > 0.0 {
                let step_start = 1.0 / 15.0;
                let step_end = walk_run.hierarchy.total_length / 2.0 + step_start;
                let current_time = walk_run.current_time;
                let prev_time = current_time - walk_run.time_step;
                let step_part = if current_time >= step_start && prev_time < step_start {
                    1
                } else if current_time >= step_end && prev_time < step_end {
                    2
                } else {
                    0
                };
                if step_part != 0 {
                    player.ped.queued_footstep = Some(if step_part == 1 {
                        crate::audio::soundlist::SOUND_STEP_START as u16
                    } else {
                        crate::audio::soundlist::SOUND_STEP_END as u16
                    });
                }
            }
        }
    }
    clump.associations.retain(|association| {
        !(association.flags & ASSOC_DELETEFADEDOUT != 0 && association.blend_amount <= 0.0)
    });

    // SetRealMoveAnim adrenaline branch: one-third world time while movement
    // associations run at double speed, then restore both exactly on expiry.
    if run_set_real_move_anim && player.state.adrenaline_active {
        let (time_scale, animation_speed) = if now_ms > player.state.adrenaline_time {
            player.state.adrenaline_active = false;
            (1.0, 1.0)
        } else {
            (1.0 / 3.0, 2.0)
        };
        timer.set_time_scale(time_scale);
        for association in &mut clump.associations {
            if matches!(
                association.anim_id,
                ANIM_STD_STARTWALK | ANIM_STD_WALK | ANIM_STD_RUN | ANIM_STD_RUNFAST
            ) {
                association.speed = animation_speed;
            }
        }
    }

    // CPlayerPed::UseSprintEnergy consumes CTimer::GetTimeStep (50 Hz).
    let gta_time_step = dt.max(0.0) * 50.0;
    let sprint_active = player.ped.move_state == eMoveState::PEDMOVE_SPRINT
        && clump.associations.iter().any(|association| {
            association.anim_id == ANIM_STD_RUNFAST && association.blend_amount > 0.0
        });
    if run_set_real_move_anim && sprint_active && player.state.current_stamina > -150.0 {
        // UseSprintEnergy: cheats and adrenaline suppress both stamina drain
        // and stamina-progress gains.
        if !infinite_sprint && !player.state.adrenaline_active {
            player.state.current_stamina -= gta_time_step;
            player.state.stamina_progress += gta_time_step;
        }
        if player.state.stamina_progress >= 500.0 {
            player.state.stamina_progress = 0.0;
            player.state.max_stamina = (player.state.max_stamina + 10.0).min(1000.0);
        }
    } else {
        let restore_speed = if player.ped.move_state == eMoveState::PEDMOVE_RUN {
            0.3
        } else {
            1.0
        };
        player.state.current_stamina =
            (player.state.current_stamina + restore_speed * gta_time_step * 0.5).min(player.state.max_stamina);
    }
    let (x, y) = transform.velocity2d.unwrap_or((0.0, 0.0));
    CVector::new(x, y, 0.0)
}

#[derive(SystemParam)]
pub(crate) struct PlayerControlFrame<'w> {
    pads: Res<'w, CPads>,
    vehicle_spatial: Res<'w, crate::collision::VehicleSpatial::VehicleCollisionSpatial>,
    cheats: Option<Res<'w, crate::core::Cheats::CCheatState>>,
    bridge_state: Option<Res<'w, CBridge>>,
    menu: Option<Res<'w, CMenuManager>>,
    handling_manager: ResMut<'w, cHandlingDataMgr>,
    ped_animation_offsets: Res<'w, CPedAnimationOffsets>,
    general: ResMut<'w, CGeneral>,
    timer: ResMut<'w, CTimer>,
    ped_stats: Res<'w, CPedStatsRuntime>,
    fight_moves: Res<'w, CFightMoveTable>,
    surface_table: Res<'w, CSurfaceTableAsset>,
    weather: Res<'w, CWeather>,
    water: Option<Res<'w, CWaterLevel>>,
    timecycle: Option<Res<'w, CTimeCycle>>,
    particle_objects: ResMut<'w, CParticleObjectRuntime>,
    coop_session: Option<ResMut<'w, crate::core::LocalCoop::LocalCoopSession>>,
    projectiles: Option<ResMut<'w, CProjectileInfoPool>>,
    explosions: Option<ResMut<'w, CExplosionPool>>,
    shots: Option<ResMut<'w, CShotInfoPool>>,
    special_fx: Option<ResMut<'w, CSpecialFX>>,
    point_lights: Option<ResMut<'w, CPointLights>>,
    bullets: Option<ResMut<'w, crate::weapons::BulletInfo::CBulletInfoPool>>,
    gameplay_input: Res<'w, crate::core::EnhancedInput::GtaGameplayInput>,
}

fn player_vehicle_door_occupant(
    script_entities: &ScriptEntityWorld,
    vehicle_handle: i32,
    door: usize,
) -> Option<i32> {
    let seat = match door {
        2 => VehicleSeat::Driver,
        3 => VehicleSeat::Passenger(0),
        4 => VehicleSeat::Passenger(1),
        5 => VehicleSeat::Passenger(2),
        _ => return None,
    };
    let Some(vehicle_entity) = script_entities.vehicle_entity(vehicle_handle) else {
        return None;
    };
    script_entities
        .script_occupant_handles(vehicle_entity)
        .find_map(|(ped_handle, occupied)| (occupied == seat).then_some(ped_handle))
}

fn player_should_carjack_door(
    script_entities: &ScriptEntityWorld,
    vehicle_handle: i32,
    door: usize,
) -> bool {
    let Some(vehicle) = script_entities.vehicles.get(&script_entities.index, &vehicle_handle) else {
        return false;
    };
    let occupant = if vehicle.base().flags.is_bus {
        player_vehicle_door_occupant(script_entities, vehicle_handle, 2)
    } else {
        player_vehicle_door_occupant(script_entities, vehicle_handle, door)
    };
    occupant
        .and_then(|ped_handle| script_entities.peds.get(&script_entities.index, &ped_handle))
        .is_some_and(|occupant| occupant.ped.ped_state == PedState::PED_DRIVING)
}
pub(crate) fn update_player_on_foot_control(
    frame: PlayerControlFrame,
    mut bridge: ResMut<ScriptGameplayBridge>,
    mut world_players: CWorldPlayers,
    active_animations: Query<&CPedVehicleAnimation>,
    mut script_entities: ScriptEntityWorld,
    mut camera: Option<ResMut<CCamera>>,
    file_loader: Res<CFileLoader>,
    anim_manager: Res<CAnimManager>,
    weapon_info: Res<CWeaponInfoStore>,
    particle_inputs: (ResMut<CParticle>, Res<crate::renderer::ParticleMgr::cParticleSystemMgr>),
    mut audio: Option<ResMut<cDMAudio>>,
    mut darkel: Option<ResMut<CDarkel>>,
    mut stats: Option<ResMut<CStats>>,
    mut events: ResMut<CEventList>,
    replay: Option<Res<CReplay>>,
    player_info: Option<Res<CPlayerInfo>>,
) {
    let (mut particles, particle_systems) = particle_inputs;
    let PlayerControlFrame {
        pads,
        vehicle_spatial,
        cheats,
        bridge_state,
        menu,
        mut handling_manager,
        ped_animation_offsets,
        mut general,
        mut timer,
        ped_stats,
        fight_moves,
        surface_table,
        weather,
        water,
        timecycle,
        mut particle_objects,
        mut coop_session,
        mut projectiles,
        mut explosions,
        mut shots,
        mut special_fx,
        mut point_lights,
        mut bullets,
        gameplay_input,
    } = frame;
    let object_colliders = script_object_collision_snapshots(&script_entities.objects);
    let control_enabled = bridge.player_control_enabled && !bridge.playing_intro;
    let dt = timer.time_step_in_seconds().clamp(0.0, 0.05);
    let now_ms = timer.time_in_milliseconds();
    let pad = *pads.get_pad(0);
    let index = bridge.player_index;
    let Some(player_entity) = world_players.player_entity(index) else {
        return;
    };
    let player_relation = world_players.relation(index);
    let other_index = if index == 0 { 1 } else { 0 };
    let (player_opt, mut other_player) = world_players.get_pair_mut(index, other_index);
    let Some(mut player) = player_opt else {
        return;
    };
    let player = &mut player;
    player.state.dynamic_collision_probe = None;
    player.state.dynamic_collision_target = None;
    // CPlayerPed::ProcessControl returns immediately for PED_DIE/PED_DEAD.
    // SetDead intentionally keeps the vehicle link, so retaining InVehicle is
    // correct; using that link to run driving/seat-sync code is not.
    if player.ped.dying_or_dead() && player_relation.is_some() {
        if active_animations.get(player_entity).is_ok() {
            script_entities
                .commands
                .entity(player_entity)
                .remove::<CPedVehicleAnimation>();
        }
        bridge.player_position = player.position();
        bridge.player_heading_degrees = player.heading_degrees();
        return;
    }
    if player.ped.ped_state == PedState::PED_DEAD {
        bridge.player_position = player.position();
        bridge.player_heading_degrees = player.heading_degrees();
        return;
    }
    if let Ok(active) = active_animations.get(player_entity) {
        // re3 `CPed::LineUpPedWithCar` (PedAI.cpp:3271-3279) owns the
        // entering/exiting ped matrix every frame; `SetPedPositionInCar`
        // only owns it once seated. The render line-up already uses this, so
        // the logic ped must follow the same matrix: otherwise the on-foot
        // pipeline below drags the logic ped with camera-relative input and
        // root motion while the mesh sits at the door, and the enter/exit
        // commit then visibly teleports.
        let exiting = matches!(
            active.phase,
            PedVehicleAnimationPhase::Exit | PedVehicleAnimationPhase::CloseExit
        );
        let (anim_vehicle, anim_door) = (active.vehicle, active.door);
        if let Some(relation) = player_relation
            && !exiting
            && let Some(handle) = script_entities.vehicle_handle_for_entity(relation.vehicle)
            && let Some(vehicle) = script_entities.vehicles.get(&script_entities.index, &handle)
        {
            // re3 `CWorld::Process` runs `CPed::SetPedPositionInCar` every
            // frame for seated peds: the seat dummy, not the vehicle origin.
            let base = vehicle.base();
            let seat = player_seat_index(relation.seat);
            let is_boat = matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. });
            let position = ped_seat_world_position(&base, seat, is_boat, &file_loader);
            player.set_position(position);
            bridge.player_position = position;
            bridge.player_heading_degrees = vehicle.base().physical.entity.get_heading_degrees();
            return;
        }
        if let Some(vehicle_handle) = script_entities.vehicle_handle_for_entity(anim_vehicle)
            && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                script_entities.vehicles.get(&script_entities.index, &vehicle_handle)
            && let Some(handling) = car
                .vehicle.core.handling_id
                .and_then(|id| usize::try_from(id).ok())
                .and_then(|id| handling_manager.get_handling_data(id))
            && let Some(line_up_result) = automobile_ped_line_up(
                &file_loader,
                &car,
                handling,
                &ped_animation_offsets,
                anim_door,
                PedVehicleLineUpContext {
                    current_position: player.position(),
                    current_heading: player.locomotion.rotation_cur,
                    offset_seek: player.ped.offset_seek,
                    state_timer_ms: player.ped.ped_state_timer,
                    now_ms,
                },
                active.phase,
                &active.association,
                player.ped.physical.move_speed.z,
                timer.time_step(),
            )
        {
            player.ped.physical.entity.set_matrix(line_up_result.matrix);
            player.ped.physical.move_speed.x = 0.0;
            player.ped.physical.move_speed.y = 0.0;
            player.ped.physical.move_speed.z = line_up_result.vertical_speed_z;
            let heading = line_up_result
                .matrix
                .right
                .y
                .atan2(line_up_result.matrix.right.x);
            player.locomotion.rotation_cur = heading;
            player.locomotion.rotation_dest = heading;
            player.state.move_speed = 0.0;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            bridge.player_position = player.position();
            bridge.player_heading_degrees = heading.to_degrees();
            return;
        }
        if let Some(relation) = player_relation
            && let Some(handle) = script_entities.vehicle_handle_for_entity(relation.vehicle)
            && let Some(vehicle) = script_entities.vehicles.get(&script_entities.index, &handle)
        {
            // Line-up data unavailable: keep the previous seat-snap behaviour
            // rather than running the on-foot pipeline on a seated ped.
            let base = vehicle.base();
            let seat = player_seat_index(relation.seat);
            let is_boat = matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. });
            let position = ped_seat_world_position(&base, seat, is_boat, &file_loader);
            player.set_position(position);
            bridge.player_position = position;
            bridge.player_heading_degrees = vehicle.base().physical.entity.get_heading_degrees();
            return;
        }
        // Entering without line-up data: freeze instead of drifting with
        // camera-relative on-foot input; the commit still warps to the seat.
        player.ped.physical.move_speed = CVector::ZERO;
        player.state.move_speed = 0.0;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        return;
    }
    if let Some(relation) = player_relation {
        let vehicle_handle = script_entities.vehicle_handle_for_entity(relation.vehicle);
        let exit_just_down = pad.exit_vehicle_just_down();
        let script_leave_car = player.ped.objective == eObjective::OBJECTIVE_LEAVE_CAR
            && now_ms > player.ped.leave_car_timer;
        let vehicle_is_on_moving_bridge = vehicle_handle
            .and_then(|handle| script_entities.vehicles.get(&script_entities.index, &handle))
            .and_then(|vehicle| vehicle.base().current_ground_entity)
            .is_some_and(|ground_entity| {
                bridge_state.as_ref().is_some_and(|bridge| {
                    bridge.this_is_a_bridge_object_moving_up(
                        bridge.lift_part == Some(ground_entity)
                            || bridge.lift_road == Some(ground_entity),
                    )
                })
            });

        // re3 `CPlayerPed::ProcessControl` (PlayerPed.cpp:1336-1361)
        // starts the rolling left-front door close while the player is in
        // `PED_DRIVING`.  It is an actual partial association on the ped;
        // the vehicle remains fully driveable, and the callback later clears
        // `m_nGettingOutFlags` and restores the door damage status.  Keep the
        // input quirk from the reference: pressing accelerate, steering or
        // brake leaves the door swinging until a no-input frame arrives.
        if !script_leave_car
            && player.ped.ped_state == PedState::PED_DRIVING
            && pad.get_accelerate() == 0
            && pad.get_steering_left_right() == 0
            && pad.get_brake() == 0
            && let Some(handle) = vehicle_handle
            && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                script_entities.vehicles.get(&script_entities.index, &handle)
            && relation.seat == VehicleSeat::Driver
            && car.damage.get_door_status(2)
                == crate::vehicles::DamageManager::eDoorStatus::DOOR_STATUS_SWINGING as u32
        {
            let animation = if car.vehicle.core.flags.low_vehicle {
                crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LO_LHS
            } else {
                crate::animation::AnimationId::ANIM_STD_CAR_CLOSE_DOOR_ROLLING_LHS
            };
            if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                vehicle.base_mut().entry.getting_out_flags |= automobile_door_flag(2);
            }
            if queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &anim_manager,
                player_entity,
                relation.vehicle,
                2,
                animation,
                PedVehicleAnimationPhase::RollingDoorClose,
                false,
            ) {
                return;
            }
            // The authored association is required by re3.  If this build's
            // animation table does not contain it, do not leave a stale
            // getting-out lock on the door; keep the observed swinging state
            // visible for the next frame and report the missing asset.
            if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                vehicle.base_mut().entry.getting_out_flags &= !automobile_door_flag(2);
            }
            warn!("player rolling door animation {animation} is unavailable");
        }
        if (script_leave_car || control_enabled && exit_just_down && !vehicle_is_on_moving_bridge)
            && let Some(handle) = vehicle_handle
            && let Some(crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. }) =
                script_entities.vehicles.get(&script_entities.index, &handle)
            && car.vehicle.can_ped_exit_car()
        {
            // re3 keeps `m_vehDoor` from the entry and SetExitCar reuses it;
            // fall back to the left front door only when no entry door was
            // recorded (e.g. script WARP_PLAYER_INTO_CAR).
            let door = usize::try_from(player.ped.veh_door)
                .ok()
                .filter(|door| (2..=5).contains(door))
                .unwrap_or(2);
            let animation = PedAutomobileProfile::from_car(&car).exit_animation(door);
            player.ped.ped_state = PedState::PED_EXIT_CAR;
            player.ped.veh_door = door as i32;
            if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                let mut base = vehicle.base_mut();
                base.entry.getting_out_flags |= automobile_door_flag(door);
                base.set_status(eEntityStatus::STATUS_PLAYER_DISABLED);
            }
            if queue_ped_vehicle_animation(
                &mut script_entities.commands,
                &anim_manager,
                player_entity,
                relation.vehicle,
                door,
                animation,
                PedVehicleAnimationPhase::Exit,
                false,
            ) {
                return;
            }
        }
        let wheel_friction = handling_manager.wheel_friction;
        if let Some(handle) = vehicle_handle
            && script_entities.vehicles.get(&script_entities.index, &handle).is_some_and(|vehicle| {
                matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. })
                    && player_can_drive_related_vehicle(player.as_ref(), relation, vehicle.base().physical.entity.status)
            })
        {
            script_entities.vehicles.capture_collision_pose(&script_entities.index, handle);
        }
        if let Some(handle) = vehicle_handle
            && let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. }) =
                script_entities.vehicles.get_mut(&script_entities.index, &handle)
            && player_can_drive_related_vehicle(player.as_ref(),
                relation,
                car.vehicle.core.physical.entity.status,
            )
        {
            let handling_id = car.vehicle.core.handling_id.or_else(|| {
                let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
                match file_loader.model_info.get_model_info(model_id) {
                    Some(ModelInfoSlot::Vehicle(v)) => {
                        cHandlingDataMgr::get_handling_id(&v.handling_id)
                            .and_then(|id| i32::try_from(id).ok())
                    }
                    _ => None,
                }
            });
            if let Some(handling) = handling_id
                .and_then(|id| usize::try_from(id).ok())
                .and_then(|id| handling_manager.get_handling_data_mut(id))
            {
                car.vehicle.set_status(eEntityStatus::STATUS_PLAYER);
                car.process_player_horn(
                    &pad,
                    cheats.as_ref().is_some_and(|cheats| cheats.strong_grip),
                );
                car.vehicle.drive.engine_on = true;
                {
                    let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
                    if let Some(ModelInfoSlot::Vehicle(model)) =
                        file_loader.model_info.get_model_info(model_id)
                    {
                        car.setup_suspension_from_model(model, handling);
                    }
                }
                if let Some(water) = water.as_deref() {
                    let model_id = car.vehicle.core.physical.entity.model_index.max(0) as usize;
                    if let Some(col) = file_loader.model_info.col_model(model_id) {
                        crate::vehicles::Automobile::process_automobile_buoyancy(
                            &mut car,
                            CVector::new(
                                col.bounding_box.min.x,
                                col.bounding_box.min.y,
                                col.bounding_box.min.z,
                            ),
                            CVector::new(
                                col.bounding_box.max.x,
                                col.bounding_box.max.y,
                                col.bounding_box.max.z,
                            ),
                            water,
                            now_ms,
                            weather.wind,
                            timer.time_step(),
                            &mut particle_objects,
                            &mut general,
                            timecycle.as_deref(),
                            crate::vehicles::Automobile::AutomobileBuoyancyEffects {
                                particles: camera.as_deref().map(|camera|
                                    particles.emitter(&particle_systems, camera.position)).as_mut(),
                                audio: audio.as_deref_mut(),
                                frame_counter: timer.frame_counter(),
                            },
                        );
                        if car.vehicle.core.physical.flags.is_in_water {
                            player.ped.physical.flags.is_in_water = true;
                            apply_player_drowning_damage(player, timer.time_step(), now_ms);
                            if player.vitality.health <= 0.0 && !player.ped.dying_or_dead() {
                                player.ped.set_dead(player.vitality, player.state.pool_handle, now_ms, &mut events);
                            }
                        }
                    }
                }
                let contacts = automobile_road_contacts(&car, &file_loader, timer.time_step());
                store_automobile_wheel_contacts(&mut car, &contacts, Some(&file_loader));
                let adhesive_limits = contacts.surfaces.map(|surface| {
                    crate::core::SurfaceTable::CSurfaceTable::get_adhesive_limit(
                        &surface_table.data,
                        crate::core::SurfaceTable::eSurfaceType::SURFACE_WHEELBASE as u8,
                        surface,
                    ) * crate::core::SurfaceTable::CSurfaceTable::get_wet_multiplier(
                        surface,
                        weather.wet_roads,
                    )
                });
                let controls_disabled = pad.disable_player_controls || !control_enabled;
                let damage_nodes = crate::modelinfo::VehicleModelInfo::resident_vehicle_damage_nodes(&file_loader, car.vehicle.core.physical.entity.model_index);
                car.process_player_drive_step(
                    CAutomobileDriveStepInput {
                        control: CAutomobileControlInput {
                            steering_left_right: pad.get_steering_left_right() as f32,
                            accelerate: pad.get_accelerate() as f32,
                            brake: pad.get_brake() as f32,
                            handbrake: pad.get_hand_brake() != 0,
                            exit_vehicle: exit_just_down,
                            controls_disabled,
                        },
                        local_contact_points: contacts.local_points,
                        contact_points: contacts.relative_world_points,
                        spring_ratios: contacts.spring_ratios,
                        spring_directions: contacts.spring_directions,
                        grounded: contacts.grounded,
                        collision_entities: contacts.collision_entities,
                        collision_time_ms: now_ms,
                        adhesive_limits,
                        wheel_scale: contacts.wheel_scale,
                        time_step: timer.time_step(),
                        time_step_seconds: dt,
                    },
                    handling,
                    wheel_friction,
                    &mut general,
                damage_nodes,
                );
            }
        }

        let target_vehicle = vehicle_handle.and_then(|handle| {
            let vehicle = script_entities.vehicles.get(&script_entities.index, &handle)?;
            let base = vehicle.base();
            let model_id = base.physical.entity.model_index.max(0) as usize;
            let col = file_loader.model_info.col_model(model_id)?;
            let bounds = &col.bounding_box;
            let is_boat = matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. });
            let mut first_person_local = file_loader
                .model_info
                .get_model_info(model_id)
                .and_then(|slot| match slot {
                    ModelInfoSlot::Vehicle(model) => model.clump.base.clump_summary.as_ref(),
                    _ => None,
                })
                .and_then(|summary| vehicle_dummy_model_position(summary, "ped_frontseat"))
                .unwrap_or(CVector::ZERO);
            // CCam::Process_1stPerson uses the authored front-seat CAR_POS,
            // centers it on X, then applies the camera-only Y/Z offsets.
            first_person_local.x = 0.0;
            first_person_local.y += 0.08;
            first_person_local.z += 0.62;
            let mut first_person_source =
                base.physical.entity.transform_point(first_person_local);
            if is_boat {
                first_person_source.z += 0.5;
            }
            let up = base.physical.entity.get_up();
            Some((
                handle,
                base.physical.entity.get_position(),
                base.physical.entity.get_forward(),
                up,
                base.physical.move_speed,
                base.physical.entity.get_heading_degrees(),
                CVector::new(
                    bounds.max.x - bounds.min.x,
                    bounds.max.y - bounds.min.y,
                    bounds.max.z - bounds.min.z,
                ),
                first_person_source,
                is_boat,
                up.z <= -0.9,
            ))
        });
        // re3 `CWorld::Process` runs `CPed::SetPedPositionInCar` every frame:
        // the ped rides at the seat dummy, not the vehicle origin.
        let seat_position = vehicle_handle.and_then(|handle| {
            let vehicle = script_entities.vehicles.get(&script_entities.index, &handle)?;
            let base = vehicle.base();
            let seat = player_seat_index(relation.seat);
            let is_boat = matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. });
            Some(ped_seat_world_position(&base, seat, is_boat, &file_loader))
        });
        if let (Some(position), Some((_, _, _, _, _, heading, _, _, _, _))) =
            (seat_position, target_vehicle)
        {
            player.set_position(position);
            player.set_heading_degrees(heading);
            player.ped.physical.move_speed = CVector::ZERO;
            player.state.move_speed = 0.0;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_standing = true;
            bridge.player_position = position;
            bridge.player_heading_degrees = heading;
        }
        if let (
            Some(camera),
            Some((
                handle,
                position,
                forward,
                up,
                speed,
                _,
                dimensions,
                first_person_source,
                is_boat,
                upside_down,
            )),
        ) =
            (camera.as_mut(), target_vehicle)
            && camera.script_mode == CameraScriptMode::FollowPlayer
        {
            let look_direction = if pad.get_look_behind_for_car() {
                crate::core::Cam::LOOKING_BEHIND
            } else if pad.get_look_left() {
                crate::core::Cam::LOOKING_LEFT
            } else if pad.get_look_right() {
                crate::core::Cam::LOOKING_RIGHT
            } else {
                crate::core::Cam::LOOKING_FORWARD
            };
            match camera.car_zoom_indicator {
                crate::core::Camera::CAM_ZOOM_1STPRS => camera.process_vehicle_first_person(
                    first_person_source,
                    forward,
                    up,
                    upside_down,
                    is_boat,
                    look_direction,
                ),
                crate::core::Camera::CAM_ZOOM_TOPDOWN => camera.process_vehicle_top_down(
                    position,
                    forward,
                    speed,
                    timer.time_step(),
                    |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    },
                ),
                _ => camera.process_cam_on_a_string(
                    position,
                    forward,
                    speed,
                    dimensions,
                    timer.time_step(),
                    look_direction,
                    |x, y, z| {
                        file_loader
                            .world
                            .find_ground_z_for_3d_coord(x, y, z, &file_loader.model_info)
                    },
                    |x, y, z| {
                        file_loader
                            .world
                            .find_roof_z_for_3d_coord(x, y, z, &file_loader.model_info)
                    },
                    |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    },
                    |target, source| {
                        camera_vehicle_obscurer_max_z(
                            &script_entities,
                            &file_loader,
                            handle,
                            target,
                            source,
                        )
                    },
                ),
            }
        }
        return;
    }

    // Run the transient/friction half of CPhysical::ProcessControl before the
    // controller mutates this frame's pose. The returned bit is the previous
    // frame's wall contact used by PlayerControlZelda's airborne force gate.
    let hit_something_last_frame = prepare_player_physical_for_control(&mut player.ped);
    apply_player_ped_air_resistance(&mut player.ped, dt);
    if let Some(water) = water.as_deref() {
        process_player_water(
            player,
            water,
            &file_loader,
            camera.as_deref().map(|camera|
                particles.emitter(&particle_systems, camera.position)).as_mut(),
            &mut particle_objects,
            &mut general,
            timecycle.as_deref(),
            audio.as_deref_mut(),
            now_ms,
            weather.wind,
            timer.time_step(),
        );
        if player.vitality.health <= 0.0 && !player.ped.dying_or_dead() {
            player.ped.set_dead(player.vitality, player.state.pool_handle, now_ms, &mut events);
        }
    }

    if matches!(
        player.ped.ped_state,
        PedState::PED_FALL | PedState::PED_GETUP | PedState::PED_DIE | PedState::PED_DEAD
    ) {
        // CPed::Fall/Die do not accept on-foot pad input. Keep the collision
        // velocity alive, apply the same GTA gravity scale, and let SetFall's
        // authored timer return a surviving ped to control.
        const PED_BASE_OFFSET: f32 = 1.04;
        let old_position = player.position();
        let moved = player.ped.calculate_new_velocity(player.locomotion, player.vitality,
            timer.time_step(), false);
        if player.ped.flags.is_standing {
            player.ped.physical.entity.set_heading(player.locomotion.rotation_cur);
        }
        let surface = player_current_surface_motion(player.ped, &script_entities);
        apply_player_standing_update_position(player.ped, player.locomotion,
            CVector::new(moved.x * timer.time_step(), moved.y * timer.time_step(), 0.0),
            surface, dt);
        begin_player_collision_frame(&mut player.ped);
        let mut collision_rejected_move = false;
        let mut position = old_position;
        let next_vertical_speed =
            player.ped.physical.move_speed.z - GRAVITY_WORLD_UNITS_PER_SECOND_SQ * dt;
        position.x += player.ped.physical.move_speed.x * dt;
        position.y += player.ped.physical.move_speed.y * dt;
        position.z += next_vertical_speed * dt;
        player.ped.physical.move_speed.z = next_vertical_speed;
        let attempted_position = position;
        let static_collision = resolve_player_ped_static_collision_result(
            old_position,
            position,
            &mut player.ped,
            &file_loader,
            &surface_table,
            timer.time_step(),
            now_ms,
        );
        position = static_collision.position;
        collision_rejected_move |= static_collision.movement_rejected;
        player.ped.flags.hit_steep_slope |= static_collision.hit_steep_slope;
        let attempted_delta_z = attempted_position.z - old_position.z;
        if (attempted_delta_z > 0.0 && position.z < attempted_position.z - 1.0e-5)
            || (attempted_delta_z < 0.0 && position.z > attempted_position.z + 1.0e-5)
        {
            player.ped.physical.move_speed.z = 0.0;
        }
        if (position.x - attempted_position.x).abs() > 1.0e-5
            || (position.y - attempted_position.y).abs() > 1.0e-5
            || (position.z - attempted_position.z).abs() > 1.0e-5
        {
            player.ped.physical.entity.flags.has_hit_wall = true;
        }
        let dynamic_target_position = position;
        let dynamic_motion = resolve_player_dynamic_motion(
            old_position,
            dynamic_target_position,
            &player.ped,
            &script_entities,
            &object_colliders,
            &file_loader,
            None,
        );
        position = dynamic_motion.position;
        if let Some(support) = dynamic_motion.landing_support {
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_standing = true;
            player.ped.flags.is_in_the_air = false;
            update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
        }
        if let Some(probe) = dynamic_motion.collision_probe {
            collision_rejected_move = true;
            player.state.dynamic_collision_probe = Some(probe);
            player.state.dynamic_collision_target = Some(CVector::new(
                dynamic_target_position.x,
                dynamic_target_position.y,
                if dynamic_motion.landing_support.is_some() {
                    dynamic_motion.position.z
                } else {
                    dynamic_target_position.z
                },
            ));
        }
        let static_support = player_static_ground_support(
            position.x,
            position.y,
            position.z,
            &player.ped,
            dt,
            &file_loader,
        );
        let dynamic_support = player_dynamic_ground_support(
            position.x,
            position.y,
            position.z,
            &player.ped,
            dt,
            &script_entities,
            &object_colliders,
            &file_loader,
        );
        let ground = match (
            static_support.map(|support| support.z),
            dynamic_support.map(|support| support.z),
        ) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        let collision_established_standing = player.ped.flags.is_standing;
        let grounded = player_grounded_after_collision_or_support(
            collision_established_standing,
            position.z,
            ground,
            player_ground_support_extension(player.ped, dt),
            player.ped.physical.move_speed.z,
        );
        if !collision_established_standing || static_support.is_some() || dynamic_support.is_some() {
            update_player_current_phys_surface(player.ped, static_support, dynamic_support, grounded);
        }
        if grounded {
            if let Some(ground) = ground {
                position.z = ground + PED_BASE_OFFSET;
            }
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_standing = true;
        } else {
            player.ped.flags.is_in_the_air = true;
            player.ped.flags.is_standing = false;
        }
        // SetGetUp placement/head/COL checks read the completed live physics pose.
        player.set_position(position);
        let recovery_inputs = super::PedRecovery::PedRecoveryInput {
            anim_manager: &anim_manager,
            file_loader: &file_loader,
            timer: &timer,
            player_can_be_damaged: player.can_be_damaged,
            player_controls_disabled: pads.get_pad(0).disable_player_controls,
        };
        super::PedRecovery::service_player_fall_getup(player, &recovery_inputs,
            &script_entities.index, &vehicle_spatial, |entity| {
                let vehicle = script_entities.vehicles.get_entity(entity)?;
                (script_entities.index.vehicle(vehicle.handle()) == Some(entity))
                    .then_some(vehicle.base().core)
            }, &mut general);
        finish_player_collision_frame(
            &mut player.ped,
            old_position,
            position,
            collision_rejected_move,
        );
        player.set_position(position);
        bridge.player_position = position;
        bridge.player_heading_degrees = player.heading_degrees();
        let lock_target_position = player.ped.point_gun_at.and_then(|slot| {
            let handle = script_entities.ped_handle_for_slot(slot)?;
            script_entities.peds.get(&script_entities.index, &handle)
                .map(|ped| ped.ped.physical.entity.get_position())
        });
        if let Some(camera) = camera.as_mut()
            && camera.script_mode == CameraScriptMode::FollowPlayer
        {
            if camera.ped_zoom_indicator == crate::core::Camera::CAM_ZOOM_TOPDOWN {
                camera.process_ped_top_down(
                    position,
                    player.ped.physical.move_speed,
                    lock_target_position,
                    timer.time_step(),
                    |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    },
                );
            } else {
                camera.process_follow_ped_with_mouse(position, player.locomotion.rotation_cur, |target, source| {
                    file_loader
                        .world
                        .process_line_of_sight_static_hit(
                            [target.x, target.y, target.z],
                            [source.x, source.y, source.z],
                            &file_loader.model_info,
                            None,
                            false,
                        )
                        .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                });
                if pad.get_look_behind_for_ped() {
                    camera.apply_ped_look_behind(position, |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    });
                }
            }
        }
        return;
    }
    // re3 `CPed::ProcessObjective` `OBJECTIVE_ENTER_CAR_AS_DRIVER` /
    // `OBJECTIVE_ENTER_CAR_AS_PASSENGER` drives `CPed::SetSeekCar` for every
    // ped, including the player. Player peds live in `CWorldPlayers`, so
    // `process_ped_vehicle_objectives` never performs that transition for
    // them: a mission script ordering the player into a car (intro's
    // "get in the car" step) left the player standing next to it while the
    // companion boarded. Pick the objective vehicle's door with
    // `CPed::GetPositionToOpenCarDoor` and let the existing PED_SEEK_CAR
    // branch walk there and start the enter animation chain.
    if player_relation.is_none()
        && !player.ped.dying_or_dead()
        && !bridge.cutscene_started
        && matches!(
            player.ped.objective,
            eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER
                | eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER
        )
        && !matches!(
            player.ped.ped_state,
            PedState::PED_SEEK_CAR | PedState::PED_ENTER_CAR
        )
    {
        let driver = player.ped.objective == eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER;
        let position = player.ped.physical.entity.get_position();
        let selected = player
            .ped
            .car_in_objective
            .and_then(|slot| script_entities.vehicle_handle_for_slot(slot))
            .and_then(|handle| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } =
                    script_entities.vehicles.get(&script_entities.index, &handle)?
                else {
                    return None;
                };
                let candidate = |door: usize| {
                    let flag = automobile_door_flag(door);
                    if car.vehicle.entry.getting_in_flags & flag != 0
                        || car.vehicle.entry.getting_out_flags & flag != 0
                    {
                        return None;
                    }
                    let handling = car
                        .vehicle.core.handling_id
                        .and_then(|id| usize::try_from(id).ok())
                        .and_then(|id| handling_manager.get_handling_data(id))?;
                    let door_position = automobile_ped_door_world_position(
                        &file_loader,
                        &car,
                        handling,
                        &ped_animation_offsets,
                        door,
                        1.0,
                    )?;
                    let dx = door_position.x - position.x;
                    let dy = door_position.y - position.y;
                    Some((door, dx * dx + dy * dy))
                };
                // re3 keeps driver-door selection and passenger-seat
                // selection separate. Passenger objectives only consider
                // vacant seats; driver objectives may deliberately choose an
                // occupied door and transition into the carjack state.
                let selected_door = if driver {
                    let left = candidate(2);
                    let right = candidate(3);
                    match (left, right) {
                        (Some(left), Some(right)) if left.1 < right.1 => Some(left.0),
                        (Some(left), Some(right)) => {
                            let player_slot = CPoolHandle(player.state.pool_handle).index();
                            let protected_right_passenger =
                                player_vehicle_door_occupant(&script_entities, handle, 3)
                                    .and_then(|ped_handle| script_entities.peds.get(&script_entities.index, &ped_handle))
                                    .is_some_and(|passenger| {
                                        passenger.ped.leader == Some(player_slot)
                                            || passenger.ped.flags.dont_drag_me_out_car
                                            || car.vehicle.core.vehicle_created_by
                                                == crate::vehicles::Vehicle::MISSION_VEHICLE
                                    });
                            let right_flag = automobile_door_flag(3);
                            if protected_right_passenger
                                || car.vehicle.entry.getting_in_flags & right_flag != 0
                            {
                                Some(left.0)
                            } else {
                                Some(right.0)
                            }
                        }
                        (Some(left), None) => Some(left.0),
                        (None, Some(right)) => Some(right.0),
                        (None, None) => None,
                    }
                } else if car.vehicle.core.flags.is_bus {
                    candidate(3).map(|candidate| candidate.0)
                } else {
                    [3usize, 4, 5]
                        .into_iter()
                        .filter(|&door| {
                            player_vehicle_door_occupant(&script_entities, handle, door).is_none()
                        })
                        .filter_map(candidate)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map(|candidate| candidate.0)
                };
                selected_door.map(|door| (handle, door))
            });
        if let Some((handle, door)) = selected {
            player.ped.ped_state = PedState::PED_SEEK_CAR;
            player.ped.seek_target = Some(CPoolHandle(handle).index());
            player.ped.veh_door = door as i32;
            player.ped.distance_to_count_seek_done = 0.5;
            player.ped.physical.entity.flags.uses_collision = true;
        }
    }
    let mut seek_vehicle_handle = None;
    if player.ped.ped_state == PedState::PED_SEEK_CAR {
        let movement_cancelled = (pad.new_state.left_stick_x != 0
            && pad.old_state.left_stick_x == 0)
            || (pad.new_state.left_stick_y != 0 && pad.old_state.left_stick_y == 0)
            || (pad.new_state.dpad_up != 0 && pad.old_state.dpad_up == 0)
            || (pad.new_state.dpad_down != 0 && pad.old_state.dpad_down == 0)
            || (pad.new_state.dpad_left != 0 && pad.old_state.dpad_left == 0)
            || (pad.new_state.dpad_right != 0 && pad.old_state.dpad_right == 0);
        if movement_cancelled {
            player.ped.ped_state = PedState::PED_IDLE;
            player.ped.move_state = eMoveState::PEDMOVE_STILL;
            player.ped.seek_target = None;
            player.ped.physical.move_speed = CVector::ZERO;
            player.ped.restore_previous_objective(None);
        } else {
            let vehicle_handle = player
                .ped
                .seek_target
                .and_then(|slot| script_entities.vehicle_handle_for_slot(slot));
            let door = usize::try_from(player.ped.veh_door).ok();
            let target = vehicle_handle.zip(door).and_then(|(handle, door)| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } =
                    script_entities.vehicles.get(&script_entities.index, &handle)?
                else {
                    return None;
                };
                let flag = automobile_door_flag(door);
                let blocked = car.vehicle.entry.getting_in_flags & flag != 0
                    || car.vehicle.entry.getting_out_flags & flag != 0;
                let handling = car
                    .vehicle.core.handling_id
                    .and_then(|id| usize::try_from(id).ok())
                    .and_then(|id| handling_manager.get_handling_data(id))?;
                automobile_ped_door_world_position(
                    &file_loader,
                    &car,
                    handling,
                    &ped_animation_offsets,
                    door,
                    1.0,
                )
                .map(|position| (handle, door, position, blocked))
            });
            if let Some((handle, door, door_position, blocked)) = target {
                player.ped.seek_pos = door_position;
                let position = player.position();
                let delta = CVector::new(
                    door_position.x - position.x,
                    door_position.y - position.y,
                    0.0,
                );
                let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
                if blocked {
                    player.state.move_speed = 0.0;
                    player.ped.move_state = eMoveState::PEDMOVE_STILL;
                    seek_vehicle_handle = Some(handle);
                } else if distance > player.ped.distance_to_count_seek_done {
                    // PlayerPed::ProcessControl(PED_SEEK_CAR) and CPed::Seek:
                    // authored locomotion approaches the live door position;
                    // no kinematic warp is used.
                    player.state.move_speed = (2.0 * distance).min(2.0);
                    player.ped.move_state = if pad.get_sprint() {
                        eMoveState::PEDMOVE_SPRINT
                    } else if distance < 2.0 {
                        eMoveState::PEDMOVE_WALK
                    } else {
                        eMoveState::PEDMOVE_RUN
                    };
                    player.locomotion.rotation_dest = CGeneral::get_radian_angle_between_points(
                        door_position.x,
                        door_position.y,
                        position.x,
                        position.y,
                    );
                    let turn = CGeneral::limit_radian_angle(
                        player.locomotion.rotation_dest - player.locomotion.rotation_cur,
                    );
                    let max_turn = player.locomotion.heading_rate.to_radians() * dt * 50.0;
                    player.locomotion.rotation_cur = CGeneral::limit_radian_angle(
                        player.locomotion.rotation_cur + turn.clamp(-max_turn, max_turn),
                    );
                    let heading = player.locomotion.rotation_cur;
                    player.ped.physical.entity.set_heading(heading);
                    seek_vehicle_handle = Some(handle);
                } else {
                    // re3 SeekCar first checks CVehicle::CanPedEnterCar here.
                    // Door locks are handled later by PedAnimAlignCB so the
                    // authored locked-door reaction can still play.
                    let cannot_physically_enter = script_entities.vehicles.get(&script_entities.index, &handle)
                        .map(|vehicle| !vehicle.base().can_ped_enter_car())
                        .unwrap_or(true);
                    if cannot_physically_enter {
                        player.ped.ped_state = PedState::PED_IDLE;
                        player.ped.move_state = eMoveState::PEDMOVE_STILL;
                        player.ped.seek_target = None;
                        player.ped.physical.move_speed = CVector::ZERO;
                        player.ped.restore_previous_objective(None);
                        // Skip the enter branch entirely for this frame.
                        return;
                    }
                    if !(position.z + 1.5 > door_position.z && position.z - 0.5 < door_position.z) {
                        player.state.move_speed = 0.0;
                        player.ped.move_state = eMoveState::PEDMOVE_STILL;
                        return;
                    }
                    use crate::animation::AnimationId::{
                        ANIM_STD_CAR_ALIGN_DOOR_LHS, ANIM_STD_CAR_ALIGN_DOOR_RHS,
                        ANIM_STD_CAR_ALIGNHI_DOOR_LHS, ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
                    };
                    let rhs = matches!(door, 3 | 5);
                    let high = door_position.z - position.z > 4.4;
                    let animation = match (rhs, high) {
                        (true, true) => ANIM_STD_CAR_ALIGNHI_DOOR_RHS,
                        (true, false) => ANIM_STD_CAR_ALIGN_DOOR_RHS,
                        (false, true) => ANIM_STD_CAR_ALIGNHI_DOOR_LHS,
                        (false, false) => ANIM_STD_CAR_ALIGN_DOOR_LHS,
                    };
                    let enter_as_driver =
                        player.ped.objective != eObjective::OBJECTIVE_ENTER_CAR_AS_PASSENGER;
                    let carjacking = player_should_carjack_door(&script_entities, handle, door);
                    player.ped.ped_state = if carjacking {
                        PedState::PED_CARJACK
                    } else {
                        PedState::PED_ENTER_CAR
                    };
                    // The relation is installed by the vehicle animation completion callback.
                    player.ped.veh_door = door as i32;
                    player.ped.physical.entity.flags.uses_collision = false;
                    player.ped.offset_seek = CVector::new(
                        door_position.x - position.x,
                        door_position.y - position.y,
                        door_position.z - position.z,
                    );
                    player.ped.ped_state_timer = now_ms.wrapping_add(600);
                    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. }) =
                        script_entities.vehicles.get_mut(&script_entities.index, &handle)
                    {
                        car.vehicle.entry.num_getting_in = car.vehicle.entry.num_getting_in.saturating_add(1);
                        car.vehicle.entry.getting_in_flags |= automobile_door_flag(door);
                        car.vehicle.entry.is_being_car_jacked =
                            carjacking || (enter_as_driver && door == 3);
                        car.vehicle.drive.gas_pedal = 0.0;
                    }
                    if let Some(vehicle_entity) = script_entities.vehicle_entity(handle)
                        && queue_ped_vehicle_animation(
                            &mut script_entities.commands,
                            &anim_manager,
                            player_entity,
                            vehicle_entity,
                            door,
                            animation,
                            PedVehicleAnimationPhase::Align,
                            enter_as_driver,
                        )
                    {
                        if let Some(camera) = camera.as_mut() {
                            camera.follow_initialised = false;
                        }
                        return;
                    }
                    if let Some(crate::control::ScriptEntities::ScriptVehicleMut::Automobile { car, .. }) =
                        script_entities.vehicles.get_mut(&script_entities.index, &handle)
                    {
                        car.vehicle.entry.num_getting_in = car.vehicle.entry.num_getting_in.saturating_sub(1);
                        car.vehicle.entry.getting_in_flags &= !automobile_door_flag(door);
                    }
                    player.ped.seek_target = None;
                    player.ped.physical.entity.flags.uses_collision = true;
                    player.ped.ped_state = PedState::PED_IDLE;
                    player.ped.restore_previous_objective(None);
                }
            } else {
                player.ped.ped_state = PedState::PED_IDLE;
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
                player.ped.seek_target = None;
                player.ped.restore_previous_objective(None);
            }
        }
    }
    let enter_just_down = pad.exit_vehicle_just_down();
    if control_enabled && enter_just_down {
        if let Some(session) = coop_session.as_mut() {
            if !session.revive_active {
                if let Some(other) = other_player.as_mut() {
                    let other_dead = other.vitality.health <= 0.0
                        || matches!(other.ped.ped_state, PedState::PED_DIE | PedState::PED_DEAD);
                    let ppos = player.position();
                    let opos = other.position();
                    let dx = opos.x - ppos.x;
                    let dy = opos.y - ppos.y;
                    let dz = opos.z - ppos.z;
                    if other_dead && (dx * dx + dy * dy + dz * dz) <= 3.0 * 3.0 {
                        session.revive_active = true;
                        session.revive_timer = 2.5;
                        session.revive_target_player = other_index as usize;
                        player.ped.physical.move_speed = CVector::ZERO;
                        player.state.move_speed = 0.0;
                        player.ped.move_state = eMoveState::PEDMOVE_STILL;
                        let angle = dy.atan2(dx) - std::f32::consts::FRAC_PI_2;
                        player.locomotion.rotation_cur = angle;
                        player.locomotion.rotation_dest = angle;
                        if let Some(mut association) = anim_manager.create_anim_association(
                            crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                            crate::animation::AnimationId::ANIM_MEDIC_CPR as usize,
                        ) {
                            association.set_current_time(0.0);
                            association.set_run();
                            player.animation.movement_anim_clump.associations.push(association);
                        }
                        if let Some(audio) = audio.as_deref_mut() {
                            audio.PlayOneShot(
                                player.ped.physical.audio_entity_id,
                                crate::audio::soundlist::SOUND_PED_HEALING as u16,
                                0.0,
                            );
                        }
                        return;
                    }
                }
            }
        }
        let player_position = player.position();
        // re3 CPlayerInfo::FindClosestCarSectorList (PlayerInfo.cpp:347-369) +
        // EvaluateCarPosition (373-393): ±10 m XY scan with |dz| < 2.0 against
        // the bound centre; wrecked, train, flipped and no-collision cars are
        // rejected. Door locks are NOT consulted here — CanPedOpenLocks gates
        // the actual entry after the walk, and bIsLocked is only re3's
        // deletion protection, so mission cars stay enterable.
        let player_forward = player.ped.physical.entity.get_forward();
        let nearest = script_entities
            .vehicles
            .iter()
            .filter_map(|(&handle, vehicle)| {
                let crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } = vehicle else {
                    return None;
                };
                let base = &car.vehicle;
                if !base.physical.entity.flags.uses_collision {
                    return None;
                }
                let status = base.physical.entity.status;
                if status == eEntityStatus::STATUS_WRECKED
                    || status == eEntityStatus::STATUS_TRAIN_MOVING
                {
                    return None;
                }
                if base.physical.entity.get_up().z <= 0.3 {
                    return None;
                }
                let position = base.physical.entity.get_position();
                if (player_position.z - position.z).abs() >= 2.0 {
                    return None;
                }
                let dx = position.x - player_position.x;
                let dy = position.y - player_position.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 10.0 {
                    return None;
                }
                // EvaluateCarPosition: heading-aware closeness, not raw distance.
                let mut needed_turn = player_forward.y.atan2(player_forward.x) - dy.atan2(dx);
                while needed_turn >= std::f32::consts::PI {
                    needed_turn -= 2.0 * std::f32::consts::PI;
                }
                while needed_turn < -std::f32::consts::PI {
                    needed_turn += 2.0 * std::f32::consts::PI;
                }
                let closeness =
                    (1.0 - needed_turn.abs() / (2.0 * std::f32::consts::PI)) * (10.0 - dist);
                Some((handle, closeness))
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(handle, _)| handle);
        if let Some(handle) = nearest {
            let Some((door, door_position)) =
                script_entities.vehicles.get(&script_entities.index, &handle)
                    .and_then(|vehicle| match vehicle {
                        crate::control::ScriptEntities::ScriptVehicleRef::Automobile { car, .. } => {
                            let candidate = |door: usize| {
                                let flag = automobile_door_flag(door);
                                if car.vehicle.entry.getting_in_flags & flag != 0
                                    || car.vehicle.entry.getting_out_flags & flag != 0
                                {
                                    return None;
                                }
                                let handling = car
                                    .vehicle.core.handling_id
                                    .and_then(|id| usize::try_from(id).ok())
                                    .and_then(|id| handling_manager.get_handling_data(id))?;
                                let position = automobile_ped_door_world_position(
                                    &file_loader,
                                    &car,
                                    handling,
                                    &ped_animation_offsets,
                                    door,
                                    1.0,
                                )?;
                                let dx = position.x - player_position.x;
                                let dy = position.y - player_position.y;
                                Some((door, position, dx * dx + dy * dy))
                            };
                            // re3 manual enter always sets
                            // OBJECTIVE_ENTER_CAR_AS_DRIVER. SeekCar then calls
                            // GetNearestDoor, which only considers the two front
                            // doors. Rear doors are reserved for explicit
                            // passenger objectives (missions/taxis/etc.).
                            [2_usize, 3]
                                .iter()
                                .filter_map(|&d| candidate(d))
                                .min_by(|a, b| a.2.total_cmp(&b.2))
                                .map(|(door, position, _)| (door, position))
                        }
                        _ => None,
                    })
            else {
                return;
            };
            let vehicle_slot = CPoolHandle(handle).index();
            player.ped.set_objective_entity(eObjective::OBJECTIVE_ENTER_CAR_AS_DRIVER,
                vehicle_slot,
                now_ms,
                None,
            );
            // CPed::SetSeekCar: the door is selected now, but it remains
            // unreserved and collision stays enabled until Seek() arrives.
            player.ped.last_ped_state = player.ped.ped_state;
            player.ped.ped_state = PedState::PED_SEEK_CAR;
            player.ped.seek_target = Some(vehicle_slot);
            player.ped.seek_pos = door_position;
            player.ped.distance_to_count_seek_done = 0.5;
            player.ped.veh_door = door as i32;
            player.ped.physical.entity.flags.uses_collision = true;
            return;
        }
    }
    // re3 `CPed::ProcessObjective` GOTO/FOLLOW objectives. Player peds live
    // outside `ScriptEntityWorld`, so `process_script_ped_on_foot_objectives`
    // never advances them: a mission script can assign the same GOTO objective
    // to the player and an escort ped, and only the escort walks because the
    // player objective is written but never consumed. Run the same state
    // machine here, including the `bScriptObjectiveCompleted` contract the SCM
    // waits on with IS_CHAR_OBJECTIVE_PASSED, and let `_moved` below steer the
    // walk with the player's own root-motion/collision pipeline.
    let mut player_script_seek = None;
    if player_relation.is_none() && !player.ped.dying_or_dead() && !bridge.cutscene_started {
        let objective = player.ped.objective;
        let target = match objective {
            eObjective::OBJECTIVE_GOTO_AREA_ANY_MEANS
            | eObjective::OBJECTIVE_GOTO_AREA_ON_FOOT
            | eObjective::OBJECTIVE_RUN_TO_AREA => Some(player.ped.seek_pos),
            eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
            | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION => {
                player.ped.ped_in_objective.and_then(|slot| {
                    script_entities
                        .peds
                        .values()
                        .find(|entity| CPoolHandle(entity.handle).index() == slot)
                        .map(|entity| entity.ped.physical.entity.get_position())
                })
            }
            _ => None,
        };
        if let Some(target) = target {
            let position = player.ped.physical.entity.get_position();
            let dx = target.x - position.x;
            let dy = target.y - position.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let safe_distance = player.ped.distance_to_count_seek_done.max(0.5);
            let is_entity_objective = matches!(
                objective,
                eObjective::OBJECTIVE_GOTO_CHAR_ON_FOOT
                    | eObjective::OBJECTIVE_FOLLOW_CHAR_IN_FORMATION
            );
            if distance <= safe_distance {
                player.ped.flags.script_objective_completed = true;
                player.ped.ped_state = PedState::PED_IDLE;
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
                player.ped.physical.move_speed = CVector::ZERO;
                player.state.move_speed = 0.0;
            } else {
                player.ped.flags.script_objective_completed = false;
                player.ped.ped_state = if is_entity_objective {
                    PedState::PED_SEEK_ENTITY
                } else {
                    PedState::PED_SEEK_POS
                };
                player.ped.move_state = if objective == eObjective::OBJECTIVE_RUN_TO_AREA {
                    eMoveState::PEDMOVE_RUN
                } else {
                    eMoveState::PEDMOVE_WALK
                };
                player_script_seek = Some(target);
            }
        }
    }

    let old_pos = player.position();
    // Keep the state from the beginning of the physical step. Both the static
    // and dynamic collision resolvers clear bIsInTheAir as soon as a landing
    // support is found. Native CPed still reaches InTheAir/SetLanding for that
    // frame; reading the flag after collision instead misclassifies touchdown
    // as the pre-launch PED_JUMP state and pins the player on the ground.
    let was_airborne = player.ped.flags.is_in_the_air;
    let started_standing = player.ped.flags.is_standing;
    let physical_motion_active = !started_standing;
    let mut integrated_physical_vertical_speed = None;
    if physical_motion_active {
        // CPhysical::ProcessControl applies gravity in the base ped phase,
        // before CheckIfInTheAir/InTheAir and before PlayerControlZelda.
        player.ped.physical.move_speed.z -= GRAVITY_WORLD_UNITS_PER_SECOND_SQ * dt;
        integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
    }
    let gta_time_step = dt * GTA_PHYSICS_HZ;
    let previous_surface_motion = player_current_surface_motion(&player.ped, &script_entities);
    // Native CPlayerPed::ProcessControl calls CPed::ProcessControl first.  The
    // base phase consumes the animation displacement produced by the previous
    // clump update, rotates it in CalculateNewVelocity, and runs UpdatePosition
    // before this frame's PlayerControlZelda input changes m_fMoveSpeed or
    // m_fRotationDest.  Consuming the freshly-produced delta in the same frame
    // made ground/landing velocity chase input and collision one frame out of
    // phase, which is visible as long sliding and tiny left/right jitter.
    // re3 CCam::Using3rdPersonMouseCam is gated by
    // CCamera::m_bUseMouse3rdPerson, which mirrors CONTROL_STANDARD on PC.
    let use_mouse_3rd_person = menu
        .as_ref()
        .is_none_or(|menu| menu.control_method == CONTROL_STANDARD);
    let using_mouse_cam = use_mouse_3rd_person
        && camera
            .as_ref()
            .is_some_and(|camera| camera.script_mode == CameraScriptMode::FollowPlayer);
    // Native CanStrafeOrMouseControl returns false while FREE_CAM is enabled.
    // Keep that gate on both CalculateNewVelocity's directional correction and
    // ProcessAnimGroups' side/back group selection.
    let mouse_strafe_control = using_mouse_cam
        && camera.as_ref().is_some_and(|camera| !camera.free_cam)
        && player.ped.can_strafe_or_mouse_control();
    let mut moved_native = player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, gta_time_step, true);
    player.state.mouse_leg_turn_angle = None;
    if mouse_strafe_control {
        moved_native = apply_player_mouse_movement_direction(player, &pad, moved_native);
    }
    player
        .ped
        .physical
        .entity
        .set_heading(player.locomotion.rotation_cur);
    let previous_animation_world_delta = CVector::new(
        moved_native.x * gta_time_step,
        moved_native.y * gta_time_step,
        0.0,
    );
    let mut pos = player.position();
    let mut standing_motion_active = false;
    if started_standing {
        let standing_delta = apply_player_standing_update_position(
            &mut player.ped,
            &mut player.locomotion,
            previous_animation_world_delta,
            previous_surface_motion,
            dt,
        );
        pos.x += standing_delta.x;
        pos.y += standing_delta.y;
        standing_motion_active = standing_delta.x.abs() > 1.0e-6 || standing_delta.y.abs() > 1.0e-6;
    }

    // This is still the pre-World::ProcessCollision root position, exactly
    // where CPed::CheckIfInTheAir/InTheAir run in re3. Starting landing here
    // gives FALL_COLLAPSE its authored lead-in instead of waiting for the feet
    // collision/snap and then playing the collapse about a second too late.
    let pre_collision_air_ground =
        player_air_ground_below(old_pos, &script_entities, &object_colliders, &file_loader);
    if let Some(was_falling) = service_player_air_state_before_collision(
        player,
        &anim_manager,
        old_pos,
        pre_collision_air_ground,
        &file_loader,
    ) && let Some(audio) = audio.as_deref_mut()
        && player.ped.physical.audio_entity_id >= 0
    {
        audio.PlayOneShot(
            player.ped.physical.audio_entity_id,
            if was_falling {
                SOUND_FALL_COLLAPSE
            } else {
                SOUND_FALL_LAND
            } as u16,
            0.0,
        );
    }
    if let Some(camera) = camera.as_deref_mut() {
        use crate::core::Cam::eCamMode;
        if player.ped.ped_state == PedState::PED_SNIPER_MODE && !pad.get_target() {
            player.ped.ped_state = player.ped.last_ped_state;
            player.ped.last_ped_state = PedState::PED_NONE;
            camera.cams[camera.active_cam].mode = eCamMode::MODE_FOLLOWPED;
            camera.follow_initialised = false;
            camera.near_clip_script = crate::core::Camera::DEFAULT_NEAR;
            camera.set_motion_blur(0, 0, 0, 0, crate::core::Camera::MOTION_BLUR_NONE);
        }
        if pad.target_just_down() && player.ped.is_ped_in_control(&player.vitality, ) && player.vitality.fire.is_none() {
            let mode = match player.ped.current_weapon {
                eWeaponType::WEAPONTYPE_M16 => Some(eCamMode::MODE_M16_1STPERSON),
                eWeaponType::WEAPONTYPE_SNIPERRIFLE => Some(eCamMode::MODE_SNIPER),
                eWeaponType::WEAPONTYPE_ROCKETLAUNCHER => Some(eCamMode::MODE_ROCKETLAUNCHER),
                _ => None,
            };
            if let Some(mode) = mode {
                player.ped.set_stored_state();
                player.ped.ped_state = PedState::PED_SNIPER_MODE;
                player.state.move_speed = 0.0;
                player.ped.move_state = eMoveState::PEDMOVE_STILL;
                blend_player_animation(&mut player.animation.movement_anim_clump, &anim_manager,
                    crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
                    crate::animation::AnimationId::ANIM_STD_IDLE, 1000.0);
                camera.cams[camera.active_cam].mode = mode;
                camera.cams[camera.active_cam].reset_statics = true;
            }
        }
        if camera.weapon_mode_active() {
            let head = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
                player.ped.physical.entity.model_index.max(0) as usize, &file_loader,
                &player.animation.movement_anim_clump,
                &player.ped.physical.entity.matrix, "Shead", CVector::ZERO);
            if let Some(head) = head {
                camera.process_weapon_camera(head, player.locomotion.rotation_cur, &pad,
                    gameplay_input.mouse_look, menu.as_ref().map_or(1.0, |m| m.mouse_sensitivity as f32 / 100.0),
                    menu.as_ref().is_some_and(|m| m.mouse_invert), timer.time_step(), |from, to|
                        closest_weapon_ray_hit(from, crate::weapons::WeaponRuntime::scaled_vector(to-from, 1.0/(to-from).magnitude()),
                            (to-from).magnitude(), None, None, None, &script_entities, &file_loader).is_some());
                let heading = camera.cams[camera.active_cam].beta - std::f32::consts::FRAC_PI_2;
                player.locomotion.rotation_cur = heading;
                player.locomotion.rotation_dest = heading;
                player.ped.physical.entity.set_heading(heading);
            } else {
                bevy::log::warn_once!(target: "re3bevy::weapons", "weapon camera cannot resolve player head frame");
            }
        }
        // CCamera::Process suppresses the target ped and its hand weapon in these modes.
        player.ped.physical.entity.flags.is_visible = !camera.weapon_mode_active();
    }
    let first_person_weapon = player.ped.ped_state == PedState::PED_SNIPER_MODE;
    let camera_heading = camera
        .as_ref()
        .filter(|camera| camera.script_mode == CameraScriptMode::FollowPlayer)
        .map(|camera| camera.gameplay_orientation());
    let camera_front = camera
        .as_ref()
        .filter(|camera| camera.script_mode == CameraScriptMode::FollowPlayer)
        .and_then(|camera| camera.cams.get(camera.active_cam).map(|cam|
            if first_person_weapon { cam.front } else {
                crate::weapons::WeaponRuntime::third_person_direction(cam, camera.aspect_ratio) }));
    if player.ped.objective == eObjective::OBJECTIVE_NONE {
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
    }
    if player.ped.flags.is_landing {
        // CPlayerPed::ProcessControl runs RunningLand before dispatching the
        // current PlayerControlZelda step. It intentionally sees the previous
        // frame's scalar move speed, then Zelda may zero that scalar for a
        // still-active PED_JUMP landing before SetRealMoveAnim runs.
        apply_player_running_land(player, &pad, &anim_manager);
    }
    let heavy_weapon = weapon_info
        .GetWeaponInfo(player.ped.current_weapon)
        .is_some_and(|info| info.IsFlagSet(WEAPONFLAG_HEAVY));
    let has_block_association = player
        .animation.movement_anim_clump
        .associations
        .iter()
        .any(|association| {
            association.flags & crate::animation::AnimBlendAssociation::ASSOC_BLOCK != 0
        });
    let mouse_runaround_state = matches!(
        player.ped.ped_state,
        PedState::PED_NONE
            | PedState::PED_IDLE
            | PedState::PED_FLEE_POS
            | PedState::PED_FLEE_ENTITY
            | PedState::PED_ATTACK
            | PedState::PED_FIGHT
            | PedState::PED_AIM_GUN
    );
    let use_mouse_runaround = mouse_runaround_state
        && !has_block_association
        && using_mouse_cam
        && camera.as_ref().is_some_and(|camera| !camera.free_cam);
    let _moved = if first_person_weapon {
        player.state.move_speed = 0.0;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        false
    } else if seek_vehicle_handle.is_some() {
        true
    } else if let Some(target) = player_script_seek {
        apply_player_script_seek_step(player, target, dt)
    } else if mouse_runaround_state && has_block_association {
        false
    } else if use_mouse_runaround {
        player.apply_on_foot_mouse_runaround_input_camera(
            &pad,
            dt,
            control_enabled,
            camera_heading,
            player_relation.is_some(),
            heavy_weapon,
        )
    } else {
        player.apply_on_foot_pad_input_camera(
            &pad,
            dt,
            control_enabled,
            camera_heading,
            player_relation.is_some(),
            heavy_weapon,
        )
    };

    if !use_mouse_runaround && using_mouse_cam && camera.as_ref().is_some_and(|camera| camera.free_cam)
        && player.ped.ped_state == PedState::PED_ATTACK && player.ped.point_gun_at.is_none()
        && matches!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_FLAMETHROWER
            | eWeaponType::WEAPONTYPE_SHOTGUN | eWeaponType::WEAPONTYPE_AK47
            | eWeaponType::WEAPONTYPE_M16 | eWeaponType::WEAPONTYPE_HELICANNON) {
        player.state.move_speed = 0.0;
    }

    // This force is part of PlayerControlZelda, i.e. after the base CPed
    // UpdatePosition phase but before CPhysical::ProcessCollision integrates
    // the frame. It therefore affects airborne integration immediately without
    // feeding back into this frame's standing root-motion calculation.
    if player.ped.ped_state == PedState::PED_JUMP
        && player.ped.flags.is_in_the_air
        && control_enabled
        && !player.ped.flags.hit_steep_slope
        && (!hit_something_last_frame || player.ped.physical.damage.normal.z > 0.6)
        && player.ped.physical.distance_travelled < dt
        && player.ped.physical.move_speed.magnitude_sqr() < 0.01 * GTA_PHYSICS_HZ * GTA_PHYSICS_HZ
    {
        let (sin, cos) = player.locomotion.rotation_cur.sin_cos();
        player.ped.physical.apply_move_force(CVector::new(
            -sin * 3.0 * GTA_PHYSICS_HZ,
            cos * 3.0 * GTA_PHYSICS_HZ,
            0.05 * GTA_PHYSICS_HZ,
        ));
    }

    // Zelda always calls SetRealMoveAnim, including PED_JUMP. The freshly
    // extracted displacement is staged for the *next* base CPed phase; it is
    // only converted into physical XY by UpdatePosition while standing.
    let local_delta = update_player_movement_animation(
        player,
        &anim_manager,
        &file_loader,
        dt,
        mouse_strafe_control,
        player_info
            .as_ref()
            .is_some_and(|info| info.m_bInfiniteSprint),
        now_ms,
        &mut timer,
    );
    player.stage_anim_move_delta(Some(local_delta));
    if let Some(sound) = player.ped.queued_footstep.take()
        && player.ped.flags.is_standing
        && let Some(audio) = audio.as_deref_mut()
        && player.ped.physical.audio_entity_id >= 0
    {
        audio.PlayOneShot(player.ped.physical.audio_entity_id, sound, 1.0);
    }

    if physical_motion_active {
        pos.x += player.ped.physical.move_speed.x * dt;
        pos.y += player.ped.physical.move_speed.y * dt;
        pos.z += player.ped.physical.move_speed.z * dt;
        integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
    }

    // CPed collision uses the ped model's authored COL against GTA world COL.
    // Standing contacts retain lateral separation; airborne contacts keep the
    // full 3D manifold so walls, slopes and ceilings participate.
    begin_player_collision_frame(&mut player.ped);
    let mut collision_rejected_move = false;
    let moved = (pos.x - old_pos.x).abs() > 1.0e-5
        || (pos.y - old_pos.y).abs() > 1.0e-5
        || (pos.z - old_pos.z).abs() > 1.0e-5;
    if standing_motion_active
        || player.ped.flags.is_in_the_air
        || player.ped.flags.is_landing
        || moved
    {
        let attempted_position = pos;
        let static_collision = resolve_player_ped_static_collision_result(
            old_pos,
            pos,
            &mut player.ped,
            &file_loader,
            &surface_table,
            timer.time_step(),
            now_ms,
        );
        pos = static_collision.position;
        collision_rejected_move |= static_collision.movement_rejected;
        if integrated_physical_vertical_speed.is_some() {
            integrated_physical_vertical_speed = Some(player.ped.physical.move_speed.z);
        }
        player.ped.flags.hit_steep_slope |= static_collision.hit_steep_slope;
        if (pos.x - attempted_position.x).abs() > 1.0e-5
            || (pos.y - attempted_position.y).abs() > 1.0e-5
            || (pos.z - attempted_position.z).abs() > 1.0e-5
        {
            player.ped.physical.entity.flags.has_hit_wall = true;
        }
        let dynamic_target_position = pos;
        let dynamic_motion = resolve_player_dynamic_motion(
            old_pos,
            dynamic_target_position,
            &player.ped,
            &script_entities,
            &object_colliders,
            &file_loader,
            seek_vehicle_handle,
        );
        pos = dynamic_motion.position;
        if let Some(support) = dynamic_motion.landing_support {
            player.ped.physical.move_speed.z = 0.0;
            if integrated_physical_vertical_speed.is_some() {
                integrated_physical_vertical_speed = Some(0.0);
            }
            player.ped.flags.is_standing = true;
            player.ped.flags.is_in_the_air = false;
            update_player_current_phys_surface(&mut player.ped, None, Some(support), true);
        }
        if let Some(probe) = dynamic_motion.collision_probe {
            collision_rejected_move = true;
            player.state.dynamic_collision_probe = Some(probe);
            player.state.dynamic_collision_target = Some(CVector::new(
                dynamic_target_position.x,
                dynamic_target_position.y,
                if dynamic_motion.landing_support.is_some() {
                    dynamic_motion.position.z
                } else {
                    dynamic_target_position.z
                },
            ));
        }
    }
    let collision_established_standing = player.ped.flags.is_standing;

    // re3 CPhysical::ApplyGravity uses GRAVITY=.008 per 50 Hz GTA time step,
    // equivalent to 20 world units/s² for this seconds-based controller.
    // Ground comes from the actual GTA COL
    // triangle/sphere/box data loaded by CFileLoader, not visible render meshes.
    let mut vertical_speed =
        integrated_physical_vertical_speed.unwrap_or(player.ped.physical.move_speed.z);
    let static_ground = file_loader
        .world
        .process_line_of_sight_static_contact(
            [pos.x, pos.y, pos.z],
            [pos.x, pos.y, pos.z - 4.04],
            &file_loader.model_info,
            None,
            false,
        )
        .map(|contact| contact.point[2]);
    let static_support =
        player_static_ground_support(pos.x, pos.y, pos.z, &player.ped, dt, &file_loader);
    let dynamic_ground_below = player_dynamic_ground_below(
        pos.x,
        pos.y,
        pos.z,
        &script_entities,
        &object_colliders,
        &file_loader,
    );
    let dynamic_support =
        player_dynamic_support_from_air_probe(pos.z, &player.ped, dynamic_ground_below).or_else(
            || {
                if player.ped.flags.was_standing {
                    player_dynamic_ground_support(
                        pos.x,
                        pos.y,
                        pos.z,
                        &player.ped,
                        dt,
                        &script_entities,
                        &object_colliders,
                        &file_loader,
                    )
                } else {
                    None
                }
            },
        );
    let dynamic_ground = dynamic_ground_below.map(|support| support.z);
    let air_ground = match (static_ground, dynamic_ground) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    // `CPed::ProcessEntityCollision` only turns a surface into standing
    // support when the short root->feet line hits it.  The much longer
    // CheckIfInTheAir/InTheAir probe is only an airborne/landing hint.  Using
    // that long probe as generic ground made a car bonnet/roof under the
    // player's XY position snap the ped upward while merely walking into the
    // car, then incorrectly mark the vehicle as m_pCurrentPhysSurface.
    let support_ground = match (
        static_support.map(|support| support.z),
        dynamic_support.map(|support| support.z),
    ) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    const PED_BASE_OFFSET: f32 = 1.04; // re3 FEET_OFFSET
    let support_extension = player_ground_support_extension(&player.ped, dt);
    let grounded = player_grounded_after_collision_or_support(
        collision_established_standing,
        pos.z,
        support_ground,
        support_extension,
        vertical_speed,
    );
    // If ProcessEntityCollision already established support but this secondary
    // metadata probe missed a seam, preserve the surface relation it published
    // instead of clearing it from a weaker query.
    if !collision_established_standing || static_support.is_some() || dynamic_support.is_some() {
        update_player_current_phys_surface(
            &mut player.ped,
            static_support,
            dynamic_support,
            grounded,
        );
    }
    // ProcessShift is positional penetration recovery, not acceleration.
    // `CPed::UpdatePosition` already published the intended XY velocity into
    // CPhysical and collision response has adjusted it in place. Rebuilding
    // velocity from final displacement would turn a separation shift into
    // momentum and drive the ped back into/away from the wall next frame.
    let mut horizontal_speed = CVector::new(
        player.ped.physical.move_speed.x,
        player.ped.physical.move_speed.y,
        0.0,
    );

    if player.ped.flags.is_landing {
        // `SetLanding` only flips the air/landing flags and installs the
        // landing association. CPhysical continues integrating the existing
        // move-speed; it does not erase horizontal momentum on touchdown.
        let touching_ground =
            player_landing_touching_ground(collision_established_standing, pos.z, support_ground);
        if touching_ground {
            if let Some(z) = support_ground {
                pos.z = z + PED_BASE_OFFSET;
            }
            vertical_speed = 0.0;
            player.ped.physical.move_speed.z = 0.0;
        }
        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_standing = touching_ground;
        if player.ped.ped_state == PedState::PED_JUMP {
            // PlayerControlZelda clears the scalar animation move speed while
            // a jump is landing; this is distinct from m_vecMoveSpeed.
            player.state.move_speed = 0.0;
        }
        if update_player_jump_animation(player, dt) {
            player.ped.flags.is_landing = false;
            if player.ped.ped_state == PedState::PED_JUMP {
                player.ped.ped_state = match player.ped.last_ped_state {
                    PedState::PED_NONE | PedState::PED_IDLE | PedState::PED_JUMP => {
                        PedState::PED_IDLE
                    }
                    previous => previous,
                };
                player.ped.last_ped_state = PedState::PED_NONE;
            }
            player.state.jump_association = None;
            player.state.jump_animation_time = 0.0;
        }
    } else if control_enabled
        // re3 `CPlayerPed::PlayerControlZelda` (PlayerPed.cpp:1280):
        // `!bIsInTheAir && JumpJustDown() && m_nPedState != PED_JUMP`.
        && !player.ped.flags.is_in_the_air
        && player.ped.ped_state != PedState::PED_JUMP
        && pad.jump_just_down()
        && weapon_info
            .GetWeaponInfo(player.ped.current_weapon)
            .is_none_or(|info| !info.IsFlagSet(WEAPONFLAG_HEAVY))
        && player_can_start_jump_from_surface(&player.ped, &player.locomotion)
    {
        // CPed::SetJump changes only ped state/animation/rotation destination.
        // It deliberately leaves standing flags, position and physical speed
        // untouched until FinishLaunchCB supplies the jump impulse.
        begin_player_jump(player, &anim_manager);
    } else if player.ped.ped_state == PedState::PED_JUMP
        && !player.ped.flags.is_in_the_air
        && !was_airborne
        && player.state.jump_association.as_ref().is_some_and(|association| {
            association.anim_id == crate::animation::AnimationId::ANIM_STD_JUMP_LAUNCH
        })
    {
        // Only JUMP_LAUNCH owns the grounded PED_JUMP phase. A collision can
        // clear bIsInTheAir on the exact touchdown frame while JUMP_GLIDE is
        // still selected; was_airborne keeps that frame on the landing path
        // below instead of zeroing XY velocity forever.
        if update_player_jump_animation(player, dt) {
            // re3 `CPed::FinishLaunchCB` forward-obstacle test
            // (`Ped.cpp:8050-8070`): the launch position is 0.15 ahead of the
            // ped plus the ped COL first-sphere centre z +0.25; a 0.25 radius
            // sphere is tested against the static world, then 0.15 further
            // forward/up. On hit the ped does not receive the 8.5 impulse;
            // instead it plays `ANIM_STD_HANDSCOWER` and `bIsLanding=true`.
            let forward = CVector::new(
                -player.locomotion.rotation_cur.sin(),
                player.locomotion.rotation_cur.cos(),
                0.0,
            );
            let ped_col_z = file_loader
                .model_info
                .col_model(player.ped.physical.entity.model_index.max(0) as usize)
                .map(|col| col.spheres.first().map(|s| s.center.z).unwrap_or(0.0))
                .unwrap_or(-0.25);
            let mut probe = CVector::new(
                pos.x + forward.x * 0.15,
                pos.y + forward.y * 0.15,
                pos.z + ped_col_z + 0.25,
            );
            let mut obstacle = file_loader
                .world
                .sphere_contact_buildings_only(
                    [probe.x, probe.y, probe.z],
                    0.25,
                    &file_loader.model_info,
                )
                .is_some();
            // C++ `FinishLaunchCB` calls `CWorld::TestSphereAgainstWorld`
            // with buildings *and* physical entities enabled.  The static
            // COL query above covers buildings; keep the dynamic half in the
            // same launch gate so a car/object parked immediately in front of
            // the player produces the original hands-cover landing instead of
            // launching the ped through it.
            obstacle |= player_jump_dynamic_sphere_blocked(
                probe,
                0.25,
                &script_entities,
                &object_colliders,
                &file_loader,
            );
            if !obstacle {
                probe = CVector::new(
                    probe.x + forward.x * 0.15,
                    probe.y + forward.y * 0.15,
                    probe.z + 0.15,
                );
                obstacle = file_loader
                    .world
                    .sphere_contact_buildings_only(
                        [probe.x, probe.y, probe.z],
                        0.25,
                        &file_loader.model_info,
                    )
                    .is_some();
                obstacle |= player_jump_dynamic_sphere_blocked(
                    probe,
                    0.25,
                    &script_entities,
                    &object_colliders,
                    &file_loader,
                );
            }
            if obstacle {
                set_player_jump_animation(
                    player,
                    &anim_manager,
                    crate::animation::AnimationId::ANIM_STD_HANDSCOWER,
                );
                player.ped.flags.is_landing = true;
                // Keep ped on ground like re3 `FinishHitHeadCB` path; the
                // `is_landing` branch will handle the blended fall.
                vertical_speed = 0.0;
            } else {
                // `FinishLaunchCB` applies an impulse (8.5 for players) via
                // `CPhysical::ApplyMoveForce`, which divides by mass.  The
                // on-foot path stores velocity in world-units/second while
                // re3's move-speed uses 50 physics units per real second;
                // convert that exact post-mass value once instead of baking a
                // second, divergent mass calculation into the controller.
                vertical_speed += 8.5 / player.ped.physical.mass.max(1.0) * GTA_PHYSICS_HZ;
                let sprint_blend = player
                    .animation.movement_anim_clump
                    .associations
                    .iter()
                    .find(|association| {
                        association.anim_id == crate::animation::AnimationId::ANIM_STD_RUNFAST
                    })
                    .map(|association| association.blend_amount);
                let run_blend = player
                    .animation.movement_anim_clump
                    .associations
                    .iter()
                    .find(|association| {
                        association.anim_id == crate::animation::AnimationId::ANIM_STD_RUN
                    })
                    .map(|association| association.blend_amount);
                let minimum_speed = if let Some(blend) = sprint_blend {
                    (0.05 * blend + 0.17) * GTA_PHYSICS_HZ
                } else if let Some(blend) = run_blend {
                    (0.07 * blend + 0.1) * GTA_PHYSICS_HZ
                } else {
                    0.1 * GTA_PHYSICS_HZ
                };
                if horizontal_speed.magnitude_sqr() < minimum_speed * minimum_speed {
                    // re3 FinishLaunchCB uses WorkOutHeadingForMovingFirstPerson
                    // in the third-person mouse camera. That preserves W+A/W+D
                    // as a diagonal launch instead of forcing the jump along
                    // the ped/camera forward vector.
                    let launch_heading = player_jump_launch_heading(player.as_ref(), mouse_strafe_control);
                    let (sin, cos) = launch_heading.sin_cos();
                    horizontal_speed.x = -sin * minimum_speed;
                    horizontal_speed.y = cos * minimum_speed;
                }
                player.ped.flags.is_standing = false;
                player.ped.flags.is_in_the_air = true;
                set_player_jump_animation(
                    player,
                    &anim_manager,
                    crate::animation::AnimationId::ANIM_STD_JUMP_GLIDE,
                );
            }
        }
    } else if grounded && !was_airborne && player.ped.ped_state != PedState::PED_JUMP {
        if let Some(z) = support_ground {
            pos.z = z + PED_BASE_OFFSET;
        }
        vertical_speed = 0.0;
        player.ped.flags.is_in_the_air = false;
        player.ped.flags.is_standing = true;
    } else {
        let vertical_ground = player_check_if_in_air_has_ground(pos.z, air_ground);
        let foot_sphere_ground = !vertical_ground
            && player_check_if_in_air_has_static_foot_sphere(
                player.ped.ped_state,
                pos,
                &file_loader,
            );
        let process_in_air = was_airborne || !(vertical_ground || foot_sphere_ground);
        if process_in_air && player.state.jump_association.is_none() {
            set_player_jump_animation(
                player,
                &anim_manager,
                crate::animation::AnimationId::ANIM_STD_FALL_GLIDE,
            );
        }
        // `air_ground` is already the exact finite 4.04-unit LOS. The old
        // FindGroundZ + FEET_OFFSET comparison effectively extended this probe
        // by 1.04 units and delayed ANIM_STD_FALL/FALL_COLLAPSE noticeably.
        let ground_within_reach = air_ground.is_some();
        if process_in_air
            && !ground_within_reach
            && vertical_speed < -0.1 * GTA_PHYSICS_HZ
            && player.state.jump_association.as_ref().is_none_or(|association| {
                association.anim_id != crate::animation::AnimationId::ANIM_STD_FALL
            })
        {
            set_player_jump_animation(
                player,
                &anim_manager,
                crate::animation::AnimationId::ANIM_STD_FALL,
            );
        }
        if process_in_air {
            let _ = update_player_jump_animation(player, dt);
        }
        if let Some(z) = support_ground
            && (pos.z <= z + PED_BASE_OFFSET + 0.08 || grounded)
        {
            let impact_speed = vertical_speed;
            pos.z = z + PED_BASE_OFFSET;
            vertical_speed = 0.0;
            player.ped.physical.move_speed.z = 0.0;
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_standing = true;
            if was_airborne
                && (player.ped.ped_state == PedState::PED_JUMP
                    || player.state.jump_animation_time > 0.1
                    || impact_speed < -0.5)
            {
                let was_falling = player.state.jump_association.as_ref().is_some_and(|association| {
                    association.anim_id == crate::animation::AnimationId::ANIM_STD_FALL
                });
                let landing_animation = if was_falling {
                    crate::animation::AnimationId::ANIM_STD_FALL_COLLAPSE
                } else {
                    crate::animation::AnimationId::ANIM_STD_FALL_LAND
                };
                player.ped.flags.is_landing = true;
                set_player_jump_animation(player, &anim_manager, landing_animation);
                if let Some(audio) = audio.as_deref_mut()
                    && player.ped.physical.audio_entity_id >= 0
                {
                    let sound = if was_falling {
                        SOUND_FALL_COLLAPSE
                    } else {
                        SOUND_FALL_LAND
                    };
                    audio.PlayOneShot(player.ped.physical.audio_entity_id, sound as u16, 0.0);
                }
            } else {
                player.ped.ped_state = PedState::PED_IDLE;
            }
        } else {
            player.ped.flags.is_in_the_air = process_in_air;
            player.ped.flags.is_standing = false;
        }
    }
    player.ped.physical.move_speed =
        CVector::new(horizontal_speed.x, horizontal_speed.y, vertical_speed);
    finish_player_collision_frame(&mut player.ped, old_pos, pos, collision_rejected_move);
    player.set_position(pos);
    process_player_weapon_switch(player, &pad, &weapon_info);
    update_player_weapon_attack_input(player, &pad, timer.time_step_non_clipped());
    let fight_owns_attack = process_player_unarmed_fight(
        player,
        other_player.as_mut(),
        &pad,
        &fight_moves,
        &anim_manager,
        &file_loader,
        &mut script_entities,
        &mut particles.emitter_with_context(
            Some(&particle_systems), camera.as_ref().map(|camera| camera.position)),
        &mut general,
        Some(&ped_stats),
        &mut events,
        timer.time_in_milliseconds(),
        audio.as_deref_mut(),
        darkel.as_deref_mut(),
        stats.as_deref_mut(),
        replay
            .as_ref()
            .is_some_and(|replay| replay.mode == ReplayMode::Playback),
    
    timer.frame_counter(),
);
    if !fight_owns_attack && first_person_weapon && player.ped.current_weapon == eWeaponType::WEAPONTYPE_SNIPERRIFLE {
        if let (Some(camera), Some(bullets)) = (camera.as_deref_mut(), bullets.as_deref_mut()) {
            crate::weapons::WeaponRuntime::process_sniper_control(player, &pad, camera, bullets,
                &weapon_info, timer.previous_time_in_milliseconds(), now_ms,
                player_info.as_ref().is_some_and(|info| info.m_bFastReload), audio.as_deref_mut(), stats.as_deref_mut());
        }
    } else if !fight_owns_attack {
        let ammo_before = player.ped.weapons[player.ped.current_weapon as usize].ammo_total;
        process_player_weapon_lock_on(
            player,
            &pad,
            using_mouse_cam,
            &weapon_info,
            &file_loader,
            &script_entities,
        );
        process_player_weapon(
            player,
            other_player.as_mut(),
            &pad,
            camera_heading,
            camera_front,
            using_mouse_cam,
            timer.previous_time_in_milliseconds(),
            timer.time_in_milliseconds(),
            player_info.as_ref().is_some_and(|info| info.m_bFastReload),
            &weapon_info,
            Some(&ped_stats),
            &anim_manager,
            &file_loader,
            &mut script_entities,
            &mut particles.emitter_with_context(
                Some(&particle_systems), camera.as_ref().map(|camera| camera.position)),
            special_fx.as_deref_mut(),
            &mut general,
            darkel.as_deref_mut(),
            stats.as_deref_mut(),
            &mut events,
            replay
                .as_ref()
                .is_some_and(|replay| replay.mode == ReplayMode::Playback),
            audio.as_deref_mut(),
            projectiles.as_deref_mut(),
            explosions.as_deref_mut(),
            shots.as_deref_mut(),
            water.as_deref(),
            point_lights.as_deref_mut(),
            camera.as_ref().map(|camera| camera.position),
        
    timer.frame_counter(),
);
        if first_person_weapon && player.ped.current_weapon == eWeaponType::WEAPONTYPE_M16
            && player.ped.weapons[player.ped.current_weapon as usize].ammo_total < ammo_before
            && let Some(camera) = camera.as_deref_mut() {
            let cam = &mut camera.cams[camera.active_cam];
            cam.beta += ((general.get_random_number() & 127) as i32 - 64) as f32 * 0.0003;
            cam.alpha += ((general.get_random_number() & 127) as i32 - 64) as f32 * 0.0003;
        }
    }

    let heading = player.heading_degrees();
    bridge.player_position = pos;
    bridge.player_heading_degrees = heading;
    let lock_target_position = player.ped.point_gun_at.and_then(|slot| {
        let handle = script_entities.ped_handle_for_slot(slot)?;
        script_entities.peds.get(&script_entities.index, &handle)
            .map(|ped| ped.ped.physical.entity.get_position())
    });
    if let Some(camera) = camera.as_mut() {
        if camera.script_mode == CameraScriptMode::FollowPlayer && !camera.weapon_mode_active() {
            if camera.ped_zoom_indicator == crate::core::Camera::CAM_ZOOM_TOPDOWN {
                camera.process_ped_top_down(
                    pos,
                    player.ped.physical.move_speed,
                    lock_target_position,
                    timer.time_step(),
                    |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    },
                );
            } else {
                camera.process_follow_ped_with_mouse(pos, heading.to_radians(), |target, source| {
                    file_loader
                        .world
                        .process_line_of_sight_static_hit(
                            [target.x, target.y, target.z],
                            [source.x, source.y, source.z],
                            &file_loader.model_info,
                            None,
                            false,
                        )
                        .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                });
                if pad.get_look_behind_for_ped() {
                    camera.apply_ped_look_behind(pos, |target, source| {
                        file_loader
                            .world
                            .process_line_of_sight_static_hit(
                                [target.x, target.y, target.z],
                                [source.x, source.y, source.z],
                                &file_loader.model_info,
                                None,
                                false,
                            )
                            .map(|(point, _)| CVector::new(point[0], point[1], point[2]))
                    });
                }
            }
            // re3 CCam::Process_FollowPedWithMouse feeds Front.Heading() back
            // into the ped unless FREE_CAM is enabled. Without this the Rust
            // camera always behaved like free cam and could orbit around the
            // player's face while the player kept the old heading.
            if control_enabled
                && use_mouse_3rd_person
                && !camera.free_cam
                && camera
                    .cams
                    .get(camera.active_cam)
                    .is_some_and(|cam| cam.mode == crate::core::Cam::eCamMode::MODE_FOLLOWPED)
                && player.ped.can_strafe_or_mouse_control()
            {
                let camera_heading = CGeneral::limit_radian_angle(camera.follow_ped_heading());
                player.locomotion.rotation_cur = camera_heading;
                player.locomotion.rotation_dest = camera_heading;
                player.ped.physical.entity.set_heading(camera_heading);
                bridge.player_heading_degrees = player.heading_degrees();
            }
        }
    }
}

pub(crate) fn update_player_wanted(
    mut world_players: CWorldPlayers,
    script_entities: ScriptEntityWorld,
    file_loader: Res<CFileLoader>,
    darkel: Option<Res<CDarkel>>,
    timer: Res<CTimer>,
) {
    use crate::modelinfo::ModelIndices::{
        MI_ARMY, MI_CHOPPER, MI_COP, MI_ENFORCER, MI_FBI, MI_POLICE, MI_PREDATOR, MI_SWAT,
    };

    let now_ms = timer.time_in_milliseconds();
    let frenzy_ongoing = darkel
        .as_ref()
        .is_some_and(|darkel| darkel.frenzy_on_going());
    let player_vehicle_by_handle: HashMap<i32, Entity> = (0..world_players.slot_count())
        .filter_map(|index| {
            Some((
                world_players.player(index as i32)?.pool_handle,
                world_players.relation(index as i32)?.vehicle,
            ))
        })
        .collect();
    for player in world_players.iter_mut() {
        let position = player.position();
        let mut police_presence = 0;
        for ped in script_entities.peds.values() {
            if !matches!(ped.model_id as usize, MI_COP | MI_SWAT | MI_FBI | MI_ARMY) {
                continue;
            }
            let other = ped.ped.physical.entity.get_position();
            let dx = position.x - other.x;
            let dy = position.y - other.y;
            let dz = position.z - other.z;
            if dx * dx + dy * dy + dz * dz < 18.0 * 18.0 {
                police_presence += 1;
            }
        }
        for (&handle, vehicle) in script_entities.vehicles.iter() {
            let base = vehicle.base();
            let model = base.physical.entity.model_index as usize;
            if !base.flags.is_law_enforcer
                || !matches!(model, MI_CHOPPER | MI_PREDATOR | MI_POLICE | MI_ENFORCER)
                || player_vehicle_by_handle.get(&player.state.pool_handle).copied()
                    == script_entities.vehicle_entity(handle)
                || matches!(
                    base.physical.entity.status,
                    eEntityStatus::STATUS_ABANDONED | eEntityStatus::STATUS_WRECKED
                )
            {
                continue;
            }
            let other = base.physical.entity.get_position();
            let dx = position.x - other.x;
            let dy = position.y - other.y;
            let dz = position.z - other.z;
            if dx * dx + dy * dy + dz * dz < 18.0 * 18.0 {
                police_presence += 1;
            }
        }
        let (_, wanted_level_drop) = file_loader
            .world
            .cull_zones
            .find_attributes_for_coors([position.x, position.y, position.z]);
        player
            .state.wanted
            .update(now_ms, police_presence, frenzy_ongoing, wanted_level_drop);
    }
}

pub(crate) fn process_player_weapon_switch(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>, pad: &crate::core::Pad::CPad, weapon_info: &CWeaponInfoStore) {
    if player.ped.ped_state == PedState::PED_SNIPER_MODE { return; }
    if player.vitality.health <= 0.0
        || player.ped.ped_state == PedState::PED_DEAD
        || player.ped.ped_state == PedState::PED_ARRESTED
    {
        return;
    }

    let (direction, steps) = if pad.cycle_weapon_right_just_down() {
        (1_i32, pad.cycle_weapon_steps.max(1))
    } else if pad.cycle_weapon_left_just_down() {
        (-1_i32, pad.cycle_weapon_steps.max(1))
    } else {
        (0_i32, 0)
    };
    if direction != 0 {
        let slots = WEAPONTYPE_TOTAL_INVENTORY_WEAPONS as i32;
        let mut target_slot = player.state.selected_wep_slot as i32;
        for _ in 0..steps {
            for offset in 1..=slots {
                let index = (target_slot + direction * offset).rem_euclid(slots) as usize;
                let weapon = &player.ped.weapons[index];
                let usable = index == 0
                    || (weapon.weapon_type as usize == index
                        && (weapon.ammo_total > 0
                            || matches!(weapon.weapon_type, eWeaponType::WEAPONTYPE_BASEBALLBAT)));
                if usable && let Some(type_) = script_weapon_type(index as i32) {
                    target_slot = index as i32;
                    player.state.selected_wep_slot = type_;
                    break;
                }
            }
        }
    } else if player.ped.current_weapon != eWeaponType::WEAPONTYPE_UNARMED
        && player.ped.current_weapon != eWeaponType::WEAPONTYPE_BASEBALLBAT
    {
        // re3 CPlayerPed::ProcessWeaponSwitch out-of-ammo auto-switch
        let slot = player.ped.current_weapon as usize;
        if slot < WEAPONTYPE_TOTAL_INVENTORY_WEAPONS && player.ped.weapons[slot].ammo_total <= 0 {
            for s in (0..slot).rev() {
                let weapon = &player.ped.weapons[s];
                if s == 0
                    || (s == 1 && weapon.weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT)
                    || (weapon.ammo_total > 0
                        && weapon.weapon_type != eWeaponType::WEAPONTYPE_MOLOTOV
                        && weapon.weapon_type != eWeaponType::WEAPONTYPE_GRENADE)
                {
                    if let Some(type_) = script_weapon_type(s as i32) {
                        player.state.selected_wep_slot = type_;
                        break;
                    }
                }
            }
        }
    }

    // Apply weapon switch if selected differs from current, but delay if currently attacking/fighting/aiming
    if player.state.selected_wep_slot != player.ped.current_weapon {
        if player.ped.ped_state != PedState::PED_ATTACK
            && player.ped.ped_state != PedState::PED_AIM_GUN
            && player.ped.ped_state != PedState::PED_FIGHT
        {
            let new_weapon = player.state.selected_wep_slot;
            let index = new_weapon as usize;
            // CPed::SetCurrentWeapon changes slots only when HasWeapon succeeds.
            if player.ped.weapons.get(index).is_some_and(|weapon| weapon.weapon_type == new_weapon) {
                player.ped.current_weapon = new_weapon;
            }
            let new_weapon = player.ped.current_weapon;
            let index = new_weapon as usize;
            if index < WEAPONTYPE_TOTAL_INVENTORY_WEAPONS {
                if let Some(info) = weapon_info.GetWeaponInfo(new_weapon) {
                    player.ped.weapons[index].ammo_in_clip = player.ped.weapons[index]
                        .ammo_total.min(info.amount_of_ammunition);
                    if !info.IsFlagSet(WEAPONFLAG_CANAIM) {
                        clear_player_weapon_lock(player);
                    }
                } else {
                    bevy::log::warn_once!(target: "re3bevy::weapons", "weapon switch has no loaded weapon data");
                }
            }
        }
    }
}

/// Mirrors the player-only input bookkeeping in re3
/// `CPlayerPed::ProcessPlayerWeapon`. `m_fAttackButtonCounter` is measured in
/// native GTA timesteps (50 Hz units), and a throwable keeps the accumulated
/// value after the button is released so the follow-up throw animation can
/// use it when its fire frame is reached.
pub(crate) fn update_player_weapon_attack_input(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &crate::core::Pad::CPad,
    time_step_non_clipped: f32,
) {
    let weapon_type = player.ped.current_weapon;
    if pad.get_weapon() && player.ped.move_state != eMoveState::PEDMOVE_SPRINT {
        if player.state.selected_wep_slot == weapon_type && weapon_type != eWeaponType::WEAPONTYPE_UNARMED
        {
            if player.ped.ped_state == PedState::PED_ATTACK {
                if pad.weapon_just_down() {
                    player.state.have_target_selected = true;
                } else if !player.state.have_target_selected {
                    player.state.attack_button_counter += time_step_non_clipped.max(0.0);
                }
            } else {
                player.state.attack_button_counter = 0.0;
                player.state.have_target_selected = false;
            }
        }
    } else if player.ped.ped_state == PedState::PED_ATTACK {
        // re3 uses this flag to choose THROW_UNDER when the player releases
        // a grenade/molotov during START_THROW. It does not clear the charge.
        player.state.have_target_selected = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerWeaponLockCandidate {
    Ped { slot: usize },
    Object { handle: i32 },
}

fn player_weapon_lock_position(
    target: PlayerWeaponLockCandidate,
    script_entities: &ScriptEntityWorld,
) -> Option<CVector> {
    match target {
        PlayerWeaponLockCandidate::Ped { slot } => {
            let handle = script_entities.ped_handle_for_slot(slot)?;
            script_entities.peds.get(&script_entities.index, &handle)
                .map(|ped| ped.ped.physical.entity.get_position())
        }
        PlayerWeaponLockCandidate::Object { handle } => script_entities.objects.get(&script_entities.index, &handle)
            .map(|object| object.object.physical.entity.get_position()),
    }
}

fn current_player_weapon_lock(player: crate::peds::PlayerPed::PlayerPedRef<'_>) -> Option<PlayerWeaponLockCandidate> {
    if let Some(handle) = player.state.weapon_lock_on_object {
        Some(PlayerWeaponLockCandidate::Object { handle })
    } else {
        player
            .state.weapon_lock_on_target
            .map(|slot| PlayerWeaponLockCandidate::Ped { slot })
    }
}

fn clear_player_weapon_lock(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>) {
    player.state.weapon_lock_on_target = None;
    player.state.weapon_lock_on_object = None;
    player.ped.point_gun_at = None;
    player.ped.flags.is_pointing_gun_at = false;
    player.state.has_lock_on_target = false;
}

fn set_player_weapon_lock(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>, target: PlayerWeaponLockCandidate) {
    clear_player_weapon_lock(player);
    match target {
        PlayerWeaponLockCandidate::Ped { slot } => {
            player.state.weapon_lock_on_target = Some(slot);
            player.ped.point_gun_at = Some(slot);
        }
        PlayerWeaponLockCandidate::Object { handle } => {
            player.state.weapon_lock_on_object = Some(handle);
        }
    }
    player.ped.flags.is_pointing_gun_at = true;
    player.state.has_lock_on_target = true;
}

fn player_weapon_target_must_break(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    target_position: CVector,
    weapon_range: f32,
) -> bool {
    let offset = target_position - player.position();
    if offset.magnitude() > weapon_range {
        return true;
    }
    if !matches!(
        player.ped.current_weapon,
        eWeaponType::WEAPONTYPE_SHOTGUN | eWeaponType::WEAPONTYPE_AK47
    ) {
        return false;
    }
    let magnitude = offset.magnitude();
    if magnitude <= f32::EPSILON {
        return false;
    }
    let direction = CVector::new(offset.x / magnitude, offset.y / magnitude, offset.z / magnitude);
    let forward = player.ped.physical.entity.get_forward();
    direction.x * forward.x + direction.y * forward.y + direction.z * forward.z < 0.4
}

fn player_can_see_lock_ped(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    target_position: CVector,
    file_loader: &CFileLoader,
) -> bool {
    let position = player.position();
    let offset = target_position - position;
    let forward = player.ped.physical.entity.get_forward();
    if offset.x * forward.x + offset.y * forward.y < 0.0
        || (offset.x * offset.x + offset.y * offset.y).sqrt() >= 40.0
    {
        return false;
    }
    let head = CVector::new(position.x, position.y, position.z + 1.0);
    file_loader
        .world
        .process_line_of_sight_static_hit(
            [head.x, head.y, head.z],
            [target_position.x, target_position.y, target_position.z],
            &file_loader.model_info,
            None,
            false,
        )
        .is_none()
}

fn script_ped_attacking_player(ped: &CPed, player_slot: usize) -> bool {
    ped.point_gun_at == Some(player_slot)
        || matches!(
            ped.objective,
            eObjective::OBJECTIVE_KILL_CHAR_ON_FOOT | eObjective::OBJECTIVE_KILL_CHAR_ANY_MEANS
        ) && ped.ped_in_objective == Some(player_slot)
}

fn acquire_player_weapon_lock(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    script_entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
    weapon_range: f32,
) -> Option<PlayerWeaponLockCandidate> {
    let position = player.position();
    let forward = player.ped.physical.entity.get_forward();
    let reference_beta = CGeneral::get_atan_of_xy(forward.x, forward.y);
    let player_slot = CPoolHandle(player.state.pool_handle).index();
    let mut best: Option<(f32, usize, PlayerWeaponLockCandidate)> = None;

    for (handle, entity) in script_entities.peds.iter() {
        let ped = &entity.ped;
        let slot = CPoolHandle(*handle).index();
        if slot == player_slot
            || ped.dying_or_dead()
            || script_entities.relation_for_char(*handle).is_some()
            || ped.leader == Some(player_slot)
        {
            continue;
        }
        let target_position = ped.physical.entity.get_position();
        if !player_can_see_lock_ped(player.as_ref(), target_position, file_loader)
            || player_weapon_target_must_break(player.as_ref(), target_position, weapon_range)
        {
            continue;
        }
        let offset = target_position - position;
        let distance = (offset.x * offset.x + offset.y * offset.y).sqrt();
        if distance > weapon_range {
            continue;
        }
        let angle = CGeneral::limit_angle(
            CGeneral::get_atan_of_xy(offset.x, offset.y) - reference_beta,
        );
        let mut closeness = -distance - 5.0 * angle.abs();
        if script_ped_attacking_player(ped, player_slot) {
            closeness += 5.0;
        }
        if best.is_none_or(|(best_score, best_slot, _)| {
            closeness > best_score || (closeness == best_score && slot > best_slot)
        }) {
            best = Some((
                closeness,
                slot,
                PlayerWeaponLockCandidate::Ped { slot },
            ));
        }
    }

    let mut best_score = best.map_or(-10_000.0, |(score, _, _)| score);
    let mut selected = best.map(|(_, _, target)| target);
    for handle in player.state.targettable_objects.iter().copied().filter(|handle| *handle >= 0) {
        let Some(object) = script_entities.objects.get(&script_entities.index, &handle) else {
            continue;
        };
        let target_position = object.object.physical.entity.get_position();
        if player_weapon_target_must_break(player.as_ref(), target_position, weapon_range) {
            continue;
        }
        let offset = target_position - position;
        let distance = (offset.x * offset.x + offset.y * offset.y).sqrt();
        if distance > weapon_range {
            continue;
        }
        let angle = CGeneral::limit_angle(
            CGeneral::get_atan_of_xy(offset.x, offset.y) - reference_beta,
        );
        let closeness = -distance - 5.0 * angle.abs();
        if closeness > best_score {
            best_score = closeness;
            selected = Some(PlayerWeaponLockCandidate::Object { handle });
        }
    }
    selected
}

fn shift_player_weapon_lock(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    previous: PlayerWeaponLockCandidate,
    look_left: bool,
    script_entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
    weapon_range: f32,
) -> Option<PlayerWeaponLockCandidate> {
    let position = player.position();
    let previous_position = player_weapon_lock_position(previous, script_entities)?;
    let previous_offset = previous_position - position;
    let reference_beta = CGeneral::get_atan_of_xy(previous_offset.x, previous_offset.y);
    let player_slot = CPoolHandle(player.state.pool_handle).index();
    let mut best_score = -10_000.0;
    let mut best_slot = 0usize;
    let mut selected = None;

    for (handle, entity) in script_entities.peds.iter() {
        let slot = CPoolHandle(*handle).index();
        let candidate = PlayerWeaponLockCandidate::Ped { slot };
        let ped = &entity.ped;
        if candidate == previous
            || slot == player_slot
            || ped.dying_or_dead()
            || script_entities.relation_for_char(*handle).is_some()
            || ped.leader == Some(player_slot)
        {
            continue;
        }
        let target_position = ped.physical.entity.get_position();
        if !player_can_see_lock_ped(player.as_ref(), target_position, file_loader)
            || player_weapon_target_must_break(player.as_ref(), target_position, weapon_range)
        {
            continue;
        }
        let offset = target_position - position;
        if (offset.x * offset.x + offset.y * offset.y).sqrt() > weapon_range {
            continue;
        }
        let angle = CGeneral::limit_angle(
            CGeneral::get_atan_of_xy(offset.x, offset.y) - reference_beta,
        );
        let score = if look_left {
            if angle > 0.0 { -angle.abs() } else { -100_000.0 }
        } else if angle > 0.0 {
            -100_000.0
        } else {
            -angle.abs()
        };
        if score > best_score || (score == best_score && slot > best_slot) {
            best_score = score;
            best_slot = slot;
            selected = Some(candidate);
        }
    }

    for handle in player.state.targettable_objects.iter().copied().filter(|handle| *handle >= 0) {
        let candidate = PlayerWeaponLockCandidate::Object { handle };
        if candidate == previous {
            continue;
        }
        let Some(object) = script_entities.objects.get(&script_entities.index, &handle) else {
            continue;
        };
        let target_position = object.object.physical.entity.get_position();
        if player_weapon_target_must_break(player.as_ref(), target_position, weapon_range) {
            continue;
        }
        let offset = target_position - position;
        if (offset.x * offset.x + offset.y * offset.y).sqrt() > weapon_range {
            continue;
        }
        let angle = CGeneral::limit_angle(
            CGeneral::get_atan_of_xy(offset.x, offset.y) - reference_beta,
        );
        let score = if look_left {
            if angle > 0.0 { -angle.abs() } else { -100_000.0 }
        } else if angle > 0.0 {
            -100_000.0
        } else {
            -angle.abs()
        };
        if score > best_score {
            best_score = score;
            selected = Some(candidate);
        }
    }
    selected
}

fn process_player_weapon_lock_on(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &CPad,
    using_mouse_cam: bool,
    weapon_info: &CWeaponInfoStore,
    file_loader: &CFileLoader,
    script_entities: &ScriptEntityWorld,
) {
    let Some(info) = weapon_info.GetWeaponInfo(player.ped.current_weapon) else {
        clear_player_weapon_lock(player);
        return;
    };
    if !pad.get_target()
        || player.state.selected_wep_slot != player.ped.current_weapon
        || player.ped.move_state == eMoveState::PEDMOVE_SPRINT
        || !info.IsFlagSet(WEAPONFLAG_CANAIM)
    {
        clear_player_weapon_lock(player);
        return;
    }

    if let Some(current) = current_player_weapon_lock(player.as_ref()) {
        let invalid = match current {
            PlayerWeaponLockCandidate::Ped { slot } => {
                let Some(handle) = script_entities.ped_handle_for_slot(slot) else {
                    clear_player_weapon_lock(player);
                    return;
                };
                script_entities.relation_for_char(handle).is_some()
                    || script_entities.peds.get(&script_entities.index, &handle)
                        .is_none_or(|ped| ped.ped.dying_or_dead())
            }
            PlayerWeaponLockCandidate::Object { handle } => {
                script_entities.objects.get(&script_entities.index, &handle).is_none()
            }
        };
        let target_position = player_weapon_lock_position(current, script_entities);
        if invalid
            || using_mouse_cam
            || target_position.is_none_or(|position| {
                player_weapon_target_must_break(player.as_ref(), position, info.range)
            })
        {
            clear_player_weapon_lock(player);
            return;
        }
        let shifted = if pad.shift_target_left_just_down() {
            shift_player_weapon_lock(player.as_ref(),
                current,
                true,
                script_entities,
                file_loader,
                info.range,
            )
        } else if pad.shift_target_right_just_down() {
            shift_player_weapon_lock(player.as_ref(),
                current,
                false,
                script_entities,
                file_loader,
                info.range,
            )
        } else {
            None
        };
        if let Some(target) = shifted {
            set_player_weapon_lock(player, target);
        }
        return;
    }

    if !using_mouse_cam
        && pad.target_just_down()
        && let Some(target) = acquire_player_weapon_lock(player.as_ref(),
            script_entities,
            file_loader,
            info.range,
        )
    {
        set_player_weapon_lock(player, target);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerFightVictim {
    OtherPlayer,
    ScriptPed(i32),
}

#[derive(Debug, Clone, Copy)]
struct FightTargetSample {
    position: CVector,
    state: PedState,
    is_player: bool,
}

fn fight_target_samples(
    other_player: Option<PlayerPedRef<'_>>,
    script_entities: &ScriptEntityWorld,
) -> Vec<FightTargetSample> {
    let mut targets =
        Vec::with_capacity(script_entities.peds.len() + usize::from(other_player.is_some()));
    if let Some(other) = other_player {
        targets.push(FightTargetSample {
            position: other.position(),
            state: other.ped.ped_state,
            is_player: true,
        });
    }
    targets.extend(
        script_entities
            .peds
            .values()
            .map(|entity| FightTargetSample {
                position: entity.ped.physical.entity.get_position(),
                state: entity.ped.ped_state,
                is_player: entity.ped.is_player(),
            }),
    );
    targets
}

fn fight_target_metrics(attacker: &CPed, heading_current: f32, target: CVector) -> (f32, f32, f32) {
    let position = attacker.physical.entity.get_position();
    let delta = CVector::new(
        target.x - position.x,
        target.y - position.y,
        target.z - position.z,
    );
    let distance = delta.magnitude();
    let heading =
        CGeneral::get_radian_angle_between_points(target.x, target.y, position.x, position.y);
    let mut needed_turn = (CGeneral::limit_radian_angle(heading)
        - CGeneral::limit_radian_angle(heading_current))
    .abs();
    if needed_turn > std::f32::consts::PI {
        needed_turn = std::f32::consts::TAU - needed_turn;
    }
    (distance, needed_turn, heading)
}

fn fight_target_on_ground(state: PedState) -> bool {
    matches!(
        state,
        PedState::PED_FALL | PedState::PED_GETUP | PedState::PED_DIE | PedState::PED_DEAD
    )
}

/// re3 `CheckForPedsOnGroundToAttack` player branch. The ECS world already
/// owns the nearby-ped set, so this scans only the actual player/script peds
/// instead of rebuilding re3's ten-entry near-ped cache.
fn initial_player_fight_move(
    attacker: &CPed,
    heading_current: f32,
    targets: &[FightTargetSample],
) -> (PedFightMoves, Option<f32>) {
    let mut live_candidate: Option<(f32, f32)> = None;
    let mut dead_candidate: Option<(f32, f32)> = None;
    for target in targets {
        if !fight_target_on_ground(target.state) {
            continue;
        }
        let (distance, turn, heading) = fight_target_metrics(attacker, heading_current, target.position);
        if distance >= 2.0 || turn >= 65.0_f32.to_radians() {
            continue;
        }
        if target.state == PedState::PED_DEAD {
            if dead_candidate.is_none_or(|(best, _)| distance < best) {
                dead_candidate = Some((distance, heading));
            }
        } else if live_candidate.is_none_or(|(best, _)| distance < best) {
            live_candidate = Some((distance, heading));
        }
    }
    if let Some((_, heading)) = live_candidate.or(dead_candidate) {
        (PedFightMoves::FIGHTMOVE_GROUNDKICK, Some(heading))
    } else {
        (PedFightMoves::FIGHTMOVE_STDPUNCH, None)
    }
}

/// Player half of re3 `CPed::Fight` move selection.
fn next_player_fight_move(
    attacker: &CPed,
    heading_current: f32,
    button_pressure: u8,
    targets: &[FightTargetSample],
) -> (PedFightMoves, Option<f32>) {
    let close_move = if button_pressure > 190 {
        PedFightMoves::FIGHTMOVE_HEADBUTT
    } else {
        PedFightMoves::FIGHTMOVE_KNEE
    };
    let mut nearest: Option<(f32, FightTargetSample, f32, f32)> = None;
    for target in targets {
        let (distance, turn, heading) = fight_target_metrics(attacker, heading_current, target.position);
        if distance >= 3.0 {
            continue;
        }
        if nearest.is_none_or(|(best, _, _, _)| distance < best) {
            nearest = Some((distance, *target, turn, heading));
        }
    }
    let Some((distance, target, turn, heading)) = nearest else {
        return (PedFightMoves::FIGHTMOVE_STDPUNCH, None);
    };

    if fight_target_on_ground(target.state) {
        // re3 refuses to strike a player during PED_GETUP but still permits a
        // kick while a player is fully down/dead.
        if !(target.is_player && target.state == PedState::PED_GETUP)
            && distance < 1.3
            && turn < 55.0_f32.to_radians()
        {
            return (PedFightMoves::FIGHTMOVE_GROUNDKICK, Some(heading));
        }
        if distance < 3.0 && turn < 30.0_f32.to_radians() {
            return (PedFightMoves::FIGHTMOVE_SHUFFLE_F, Some(heading));
        }
        return (PedFightMoves::FIGHTMOVE_STDPUNCH, None);
    }

    let selected = if distance < 0.8 && turn < 75.0_f32.to_radians() {
        close_move
    } else if distance < 1.3 && turn < 55.0_f32.to_radians() {
        if close_move == PedFightMoves::FIGHTMOVE_HEADBUTT {
            PedFightMoves::FIGHTMOVE_PUNCHHOOK
        } else {
            PedFightMoves::FIGHTMOVE_PUNCHJAB
        }
    } else if distance < 1.7 && turn < 35.0_f32.to_radians() {
        PedFightMoves::FIGHTMOVE_KICK
    } else if distance < 2.0 && turn < 30.0_f32.to_radians() {
        PedFightMoves::FIGHTMOVE_LONGKICK
    } else if turn < 30.0_f32.to_radians() {
        PedFightMoves::FIGHTMOVE_SHUFFLE_F
    } else {
        PedFightMoves::FIGHTMOVE_STDPUNCH
    };
    let face_target = !matches!(
        selected,
        PedFightMoves::FIGHTMOVE_STDPUNCH | PedFightMoves::FIGHTMOVE_SHUFFLE_F
    );
    (selected, face_target.then_some(heading))
}

fn fight_strike_frame(move_id: PedFightMoves) -> Option<(&'static str, f32)> {
    Some(match move_id {
        PedFightMoves::FIGHTMOVE_STDPUNCH
        | PedFightMoves::FIGHTMOVE_PUNCHHOOK
        | PedFightMoves::FIGHTMOVE_BODYBLOW => ("Rhand", 0.0),
        PedFightMoves::FIGHTMOVE_KNEE => ("lowerlegr", 0.0),
        PedFightMoves::FIGHTMOVE_HEADBUTT => ("head", 0.0),
        PedFightMoves::FIGHTMOVE_PUNCHJAB => ("Lhand", 0.1),
        PedFightMoves::FIGHTMOVE_KICK
        | PedFightMoves::FIGHTMOVE_LONGKICK
        | PedFightMoves::FIGHTMOVE_ROUNDHOUSE
        | PedFightMoves::FIGHTMOVE_GROUNDKICK => ("footr", 0.0),
        _ => return None,
    })
}

fn player_fight_strike_point(
    player: crate::peds::PlayerPed::PlayerPedRef<'_>,
    move_id: PedFightMoves,
    file_loader: &CFileLoader,
) -> Option<CVector> {
    let (frame_name, forward_offset) = fight_strike_frame(move_id)?;
    let model_index = if player.ped.physical.entity.model_index <= 0 {
        MI_PLAYER
    } else {
        player.ped.physical.entity.model_index as usize
    };
    let mut point = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
        model_index,
        file_loader,
        &player.animation.movement_anim_clump,
        &player.ped.physical.entity.matrix,
        frame_name,
        CVector::ZERO,
    )
    .or_else(|| {
        // TODO(parity): remove this CPed::TransformToNode fallback after every
        // gameplay ped model keeps its full RenderWare frame hierarchy.
        bevy::log::warn!(
            target: "re3bevy::fight",
            frame_name,
            "Fight: live ped frame unavailable; using entity-space strike point"
        );
        let local = match move_id {
            PedFightMoves::FIGHTMOVE_GROUNDKICK => CVector::new(0.0, 0.45, -0.55),
            PedFightMoves::FIGHTMOVE_KICK
            | PedFightMoves::FIGHTMOVE_LONGKICK
            | PedFightMoves::FIGHTMOVE_ROUNDHOUSE => CVector::new(0.0, 0.75, 0.15),
            PedFightMoves::FIGHTMOVE_KNEE => CVector::new(0.0, 0.45, 0.45),
            PedFightMoves::FIGHTMOVE_HEADBUTT => CVector::new(0.0, 0.3, 0.85),
            _ => CVector::new(0.0, 0.55, 0.65),
        };
        Some(player.ped.physical.entity.transform_point(local))
    })?;
    let extra = if move_id == PedFightMoves::FIGHTMOVE_PUNCHHOOK {
        0.22
    } else {
        forward_offset
    };
    if extra != 0.0 {
        let forward = player.ped.physical.entity.get_forward();
        point.x += forward.x * extra;
        point.y += forward.y * extra;
        point.z += forward.z * extra;
    }
    Some(point)
}

fn fight_defend_move(
    victim: &CPed,
    hit_level: FightMoveHitLevel,
    direction: u8,
    damage_mult: f32,
) -> PedFightMoves {
    if fight_target_on_ground(victim.ped_state) {
        return PedFightMoves::FIGHTMOVE_HITONFLOOR;
    }
    match hit_level {
        FightMoveHitLevel::HITLEVEL_GROUND => PedFightMoves::FIGHTMOVE_HITONFLOOR,
        FightMoveHitLevel::HITLEVEL_LOW => PedFightMoves::FIGHTMOVE_HITBODY,
        FightMoveHitLevel::HITLEVEL_HIGH => match direction & 3 {
            1 => PedFightMoves::FIGHTMOVE_HITLEFT,
            2 => PedFightMoves::FIGHTMOVE_HITBACK,
            3 => PedFightMoves::FIGHTMOVE_HITRIGHT,
            _ if damage_mult <= 5.0 => PedFightMoves::FIGHTMOVE_HITHEAD,
            _ => PedFightMoves::FIGHTMOVE_HITBIGSTEP,
        },
        FightMoveHitLevel::HITLEVEL_MEDIUM | FightMoveHitLevel::HITLEVEL_NULL => {
            match direction & 3 {
                1 => PedFightMoves::FIGHTMOVE_HITLEFT,
                2 => PedFightMoves::FIGHTMOVE_HITBACK,
                3 => PedFightMoves::FIGHTMOVE_HITRIGHT,
                _ if damage_mult <= 5.0 => PedFightMoves::FIGHTMOVE_HITCHEST,
                _ => PedFightMoves::FIGHTMOVE_HITBIGSTEP,
            }
        }
    }
}

fn install_fight_defend_animation(
    victim: &mut CPed,
    table: &CFightMoveTable,
    hit_level: FightMoveHitLevel,
    direction: u8,
    damage_mult: f32,
    now_ms: u32,
) {
    let defend_move = fight_defend_move(victim, hit_level, direction, damage_mult);
    let Some(fight_move) = table.get(defend_move as usize) else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            move_id = defend_move as usize,
            "StartFightDefend: authored defend move is unavailable"
        );
        return;
    };
    victim.flags.is_attacking = false;
    victim.flags.is_shooting = false;
    victim.impact_animation_id = Some(fight_move.anim_id);
    victim.impact_animation_start_ms = now_ms;
    victim.impact_animation_speed = if victim.is_player() { 1.3 } else { 1.0 };
    victim.impact_animation_blend_delta = 8.0;
}

fn fight_hit_sound(move_id: PedFightMoves) -> u16 {
    match move_id {
        PedFightMoves::FIGHTMOVE_HEADBUTT => SOUND_FIGHT_HEADBUTT_35 as u16,
        PedFightMoves::FIGHTMOVE_KNEE => SOUND_FIGHT_KNEE_OR_KICK_43 as u16,
        PedFightMoves::FIGHTMOVE_KICK | PedFightMoves::FIGHTMOVE_ROUNDHOUSE => {
            SOUND_FIGHT_KICK_34 as u16
        }
        PedFightMoves::FIGHTMOVE_LONGKICK => SOUND_FIGHT_KICK_44 as u16,
        PedFightMoves::FIGHTMOVE_GROUNDKICK => SOUND_FIGHT_PUNCH_OR_KICK_BELOW_40 as u16,
        PedFightMoves::FIGHTMOVE_PUNCHHOOK => SOUND_FIGHT_PUNCH_39 as u16,
        _ => SOUND_FIGHT_PUNCH_33 as u16,
    }
}

fn apply_fight_knockdown_and_force(
    victim: &mut CPed, vitality: &crate::peds::PedVitality::PedVitality,
    move_id: PedFightMoves,
    health_before: f32,
    damage_mult: f32,
    direction: u8,
    attacker_position: CVector,
    now_ms: u32,
    ped_stats: Option<&CPedStatsRuntime>,

    frame_counter: u32,
) {
    let was_on_ground = victim.use_ground_col_model();
    let one_hit_knockdown = ped_stats.is_some_and(|runtime| {
        runtime.GetPedStats(victim.ped_stats).m_flags & crate::peds::PedType::STAT_ONE_HIT_KNOCKDOWN
            != 0
    });
    let should_fall = vitality.health > 0.0
        && !was_on_ground
        && ((!victim.is_player() && vitality.health < 40.0 && health_before > 40.0)
            || (vitality.health < 20.0 && health_before > 20.0)
            || one_hit_knockdown);
    if should_fall {
        victim.flags.is_standing = false;
        let _ = victim.set_fall(vitality, 
            0,
            crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(direction & 3),
            false,
            now_ms,
            frame_counter,);
    }

    if victim.ped_state == PedState::PED_DIE || !victim.flags.is_standing {
        let victim_position = victim.physical.entity.get_position();
        let mut attack_distance = CVector::new(
            victim_position.x - attacker_position.x,
            victim_position.y - attacker_position.y,
            victim_position.z - attacker_position.z,
        );
        let magnitude = attack_distance.magnitude().max(1.0e-6);
        attack_distance.x /= magnitude;
        attack_distance.y /= magnitude;
        attack_distance.z = 1.0;
        let move_mult = if move_id == PedFightMoves::FIGHTMOVE_GROUNDKICK {
            (damage_mult * 0.6).min(4.0)
        } else if victim.ped_state != PedState::PED_DIE || damage_mult >= 20.0 {
            damage_mult
        } else {
            (damage_mult * 2.0).min(14.0)
        };
        let force = move_mult * 0.6;
        victim.physical.apply_move_force(CVector::new(
            attack_distance.x * force,
            attack_distance.y * force,
            attack_distance.z * force,
        ));
    }
}

fn emit_fight_death_blood(
    particles: &mut ParticleEmitter<'_>,
    strike_point: CVector,
    collision_dist: CVector,
    hit_level: FightMoveHitLevel,
    victim: &CPed,
) {
    if hit_level <= FightMoveHitLevel::HITLEVEL_MEDIUM || victim.ped_state != PedState::PED_DIE {
        return;
    }
    let blood_pos = CVector::new(
        strike_point.x + collision_dist.x * 0.5,
        strike_point.y + collision_dist.y * 0.5,
        strike_point.z + collision_dist.z * 0.5,
    );
    let magnitude = collision_dist.magnitude().max(1.0e-6);
    let direction = CVector::new(
        collision_dist.x / (10.0 * magnitude),
        collision_dist.y / (10.0 * magnitude),
        collision_dist.z / (10.0 * magnitude),
    );
    for _ in 0..4 {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_BLOOD,
            blood_pos,
            direction,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    }
}

fn find_player_fight_victim(
    other_player: Option<PlayerPedRef<'_>>,
    script_entities: &ScriptEntityWorld,
    strike_point: CVector,
    strike_radius: f32,
    file_loader: &CFileLoader,
) -> Option<(PlayerFightVictim, CVector)> {
    let mut nearest: Option<(PlayerFightVictim, CVector)> = None;
    if let Some(other) = other_player
        && let Some(collision_dist) =
            melee_ped_collision(&other.ped, &other.animation, strike_point, strike_radius, file_loader)
    {
        nearest = Some((PlayerFightVictim::OtherPlayer, collision_dist));
    }
    for (&handle, entity) in script_entities.peds.iter() {
        let Some(collision_dist) =
            melee_ped_collision(&entity.ped, &entity.animation, strike_point, strike_radius, file_loader)
        else {
            continue;
        };
        if nearest
            .as_ref()
            .is_none_or(|(_, best)| collision_dist.magnitude_sqr() < best.magnitude_sqr())
        {
            nearest = Some((PlayerFightVictim::ScriptPed(handle), collision_dist));
        }
    }
    nearest
}

#[allow(clippy::too_many_arguments)]
fn apply_player_fight_strike(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    mut other_player: Option<&mut PlayerPedMut<'_>>,
    move_id: PedFightMoves,
    fight_move: FightMove,
    fight_moves: &CFightMoveTable,
    file_loader: &CFileLoader,
    script_entities: &mut ScriptEntityWorld,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    ped_stats: Option<&CPedStatsRuntime>,
    events: &mut CEventList,
    now_ms: u32,
    mut audio: Option<&mut cDMAudio>,
    mut darkel: Option<&mut CDarkel>,
    mut stats: Option<&mut CStats>,
    replay_playing_back: bool,

    frame_counter: u32,
) -> bool {
    let Some(strike_point) = player_fight_strike_point(player.as_ref(), move_id, file_loader) else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            move_id = move_id as usize,
            "FightStrike: move has no authored strike node"
        );
        return false;
    };
    let Some((victim_key, collision_dist)) = find_player_fight_victim(
        other_player.as_ref().map(|player| player.as_ref()),
        script_entities,
        strike_point,
        fight_move.strike_radius,
        file_loader,
    ) else {
        return false;
    };

    let attacker_position = player.position();
    let mut damage_mult =
        f32::from(fight_move.damage) * f32::from((general.get_random_number() & 1) + 2) + 1.0;
    if player.state.adrenaline_active {
        damage_mult = 20.0;
    }

    let apply_to_ped = |victim: &mut CPed, victim_heading: f32, vitality: &mut PedVitality,
                        can_be_damaged: bool,
                        general: &mut CGeneral|
     -> PedWeaponDamageResult {
        if victim.is_player() && victim.ped_state == PedState::PED_GETUP {
            return PedWeaponDamageResult::Ignored;
        }
        let health_before = vitality.health;
        let victim_position = victim.physical.entity.get_position();
        let mut direction = victim.get_local_direction(victim_heading, CVector::new(
            attacker_position.x - victim_position.x,
            attacker_position.y - victim_position.y,
            0.0,
        ));
        if move_id == PedFightMoves::FIGHTMOVE_KICK && general.get_random_number() & 1 != 0 {
            direction = (direction + 1) & 3;
        }
        install_fight_defend_animation(
            victim,
            fight_moves,
            fight_move.hit_level,
            direction,
            damage_mult,
            now_ms,
        );
        let result = if victim.dying_or_dead() {
            PedWeaponDamageResult::Ignored
        } else {
            victim.inflict_weapon_damage(vitality, 
                eWeaponType::WEAPONTYPE_UNARMED,
                damage_mult * 3.0,
                ped_defend_weakness(victim, ped_stats),
                true,
                can_be_damaged,
                now_ms,
                direction,
            )
        };
        apply_fight_knockdown_and_force(
            victim, vitality,
            move_id,
            health_before,
            damage_mult,
            direction,
            attacker_position,
            now_ms,
            ped_stats,
        
    frame_counter,
);
        result
    };

    match victim_key {
        PlayerFightVictim::OtherPlayer => {
            let Some(victim) = other_player.as_deref_mut() else {
                return false;
            };
            if victim.ped.is_player() && victim.ped.ped_state == PedState::PED_GETUP {
                return false;
            }
            let can_be_damaged = victim.can_be_damaged;
            let _ = apply_to_ped(&mut victim.ped, victim.locomotion.rotation_cur, victim.vitality, can_be_damaged, general);
            emit_fight_death_blood(
                particles,
                strike_point,
                collision_dist,
                fight_move.hit_level,
                &victim.ped,
            );
        }
        PlayerFightVictim::ScriptPed(handle) => {
            let Some(victim) = script_entities.peds.get_mut(&script_entities.index, &handle) else {
                bevy::log::warn!(
                    target: "re3bevy::fight",
                    handle,
                    "FightStrike: selected script ped disappeared before damage"
                );
                return false;
            };
            let victim_position = victim.ped.physical.entity.get_position();
            let result = apply_to_ped(&mut *victim.ped, victim.locomotion.rotation_cur, victim.vitality, true, general);
            let event_type = if victim.ped.ped_type == ePedType::PEDTYPE_COP {
                EEventType::AssaultPolice
            } else {
                EEventType::Assault
            };
            let _ = events.register_entity_event(
                event_type,
                EEventEntity::Ped,
                handle,
                Some(player.state.pool_handle),
                victim_position,
                2_000,
                now_ms,
            );
            if result == PedWeaponDamageResult::Killed
                && let (Some(darkel), Some(stats)) = (darkel.as_deref_mut(), stats.as_deref_mut())
            {
                darkel.register_kill_by_player(
                    victim.model_id,
                    victim.ped.ped_type as usize,
                    victim.ped.flags.chris_criminal,
                    eWeaponType::WEAPONTYPE_UNARMED,
                    false,
                    replay_playing_back,
                    stats,
                    audio.as_deref_mut(),
                );
            }
            emit_fight_death_blood(
                particles,
                strike_point,
                collision_dist,
                fight_move.hit_level,
                &victim.ped,
            );
        }
    }
    if let Some(audio) = audio.as_deref_mut()
        && player.ped.physical.audio_entity_id >= 0
    {
        audio.PlayOneShot(player.ped.physical.audio_entity_id, fight_hit_sound(move_id), 0.0);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_player_unarmed_fight(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    mut other_player: Option<&mut PlayerPedMut<'_>>,
    pad: &CPad,
    fight_moves: &CFightMoveTable,
    anim_manager: &CAnimManager,
    file_loader: &CFileLoader,
    script_entities: &mut ScriptEntityWorld,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    ped_stats: Option<&CPedStatsRuntime>,
    events: &mut CEventList,
    now_ms: u32,
    audio: Option<&mut cDMAudio>,
    darkel: Option<&mut CDarkel>,
    stats: Option<&mut CStats>,
    replay_playing_back: bool,

    frame_counter: u32,
) -> bool {
    if player.ped.current_weapon != eWeaponType::WEAPONTYPE_UNARMED {
        if player.ped.ped_state == PedState::PED_FIGHT {
            end_fight(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                FightEndType::Fast,
            );
        }
        return false;
    }

    if player.ped.ped_state != PedState::PED_FIGHT {
        if !pad.weapon_just_down()
            || player.ped.move_state == eMoveState::PEDMOVE_SPRINT
            || player.state.move_speed >= 1.0
            || player.ped.flags.is_in_the_air
            || player.ped.dying_or_dead()
        {
            return false;
        }
        let targets = fight_target_samples(other_player.as_ref().map(|player| player.as_ref()), script_entities);
        let (initial_move, heading) = initial_player_fight_move(&player.ped, player.locomotion.rotation_cur, &targets);
        if let Some(heading) = heading {
            player.locomotion.rotation_dest = heading;
            player.locomotion.rotation_cur = heading;
        }
        return start_fight_attack(
            &mut player.ped, &player.vitality,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            initial_move,
            now_ms,
        );
    }

    let left_right = f32::from(pad.get_ped_walk_left_right());
    let up_down = f32::from(pad.get_ped_walk_up_down());
    let pad_move = left_right.hypot(up_down);
    if pad_move > 0.0 {
        player.ped.take_step_after_attack = pad_move > 2.0 * PAD_MOVE_TO_GAME_WORLD_MOVE;
        if pad.get_sprint() && pad_move > PAD_MOVE_TO_GAME_WORLD_MOVE {
            player.ped.flags.is_attacking = false;
        }
    }
    if !player.ped.flags.is_attacking {
        let end_type = if player.ped.take_step_after_attack {
            FightEndType::WithStep
        } else {
            FightEndType::Fast
        };
        end_fight(
            &mut player.ped,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            end_type,
        );
        return true;
    }
    if pad.weapon_just_down() {
        player.ped.fight_button_pressure = pad.new_state.circle.clamp(0, 255) as u8;
    }

    let move_id = player.ped.cur_fight_move;
    if move_id == PedFightMoves::FIGHTMOVE_IDLE {
        if player.ped.fight_button_pressure != 0 {
            let targets = fight_target_samples(other_player.as_ref().map(|player| player.as_ref()), script_entities);
            let pressure = player.ped.fight_button_pressure;
            let (next_move, heading) = next_player_fight_move(&player.ped, player.locomotion.rotation_cur, pressure, &targets);
            if let Some(heading) = heading {
                player.locomotion.rotation_dest = heading;
                player.locomotion.rotation_cur = heading;
            }
            player.ped.fight_button_pressure = 0;
            player.ped.fight_combo_move = false;
            let _ = start_fight_move(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                next_move,
                4.0,
                None,
            );
        } else if now_ms > player.ped.wait_timer {
            end_fight(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                FightEndType::Normal,
            );
        }
        return true;
    }

    let Some(fight_move) = fight_moves.get(move_id as usize).copied() else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            move_id = move_id as usize,
            "Fight: current move is absent from fistfite.dat"
        );
        end_fight(
            &mut player.ped,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            FightEndType::Fast,
        );
        return true;
    };
    let association_index = player
        .animation.movement_anim_clump
        .associations
        .iter()
        .rposition(|association| association.anim_id == fight_move.anim_id);
    let Some(association_index) = association_index else {
        bevy::log::warn!(
            target: "re3bevy::fight",
            move_id = move_id as usize,
            anim_id = fight_move.anim_id,
            "Fight: current authored association disappeared"
        );
        end_fight(
            &mut player.ped,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            FightEndType::Fast,
        );
        return true;
    };

    let (anim_time, association_running) = {
        let association = &player.animation.movement_anim_clump.associations[association_index];
        (association.current_time, association.is_running())
    };
    if !association_running && player.ped.fight_state > FIGHTSTATE_MOVE_FINISHED {
        player.ped.fight_state = FIGHTSTATE_MOVE_FINISHED;
        let association = &mut player.animation.movement_anim_clump.associations[association_index];
        association.blend_delta = -1000.0;
        association.flags |= crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT;
    }

    if fight_move.hit_level != FightMoveHitLevel::HITLEVEL_NULL
        && anim_time > fight_move.start_fire_time
        && anim_time <= fight_move.end_fire_time
        && player.ped.fight_state >= FIGHTSTATE_NO_MOVE
    {
        let hit = if player.ped.fight_state != FIGHTSTATE_JUST_ATTACKED {
            apply_player_fight_strike(
                player,
                other_player.as_deref_mut(),
                move_id,
                fight_move,
                fight_moves,
                file_loader,
                script_entities,
                particles,
                general,
                ped_stats,
                events,
                now_ms,
                audio,
                darkel,
                stats,
                replay_playing_back,
            
    frame_counter,
)
        } else {
            true
        };
        if hit {
            player.ped.fight_state = FIGHTSTATE_JUST_ATTACKED;
            player.animation.movement_anim_clump.associations[association_index].speed = 0.6;
        } else if player.ped.fight_state == FIGHTSTATE_NO_MOVE {
            player.ped.fight_state = FIGHTSTATE_NO_MOVE + 1;
        }
        player.ped.fight_button_pressure = 0;
        return true;
    }

    if fight_move.hit_level != FightMoveHitLevel::HITLEVEL_NULL
        && anim_time > fight_move.end_fire_time
    {
        player.animation.movement_anim_clump.associations[association_index].speed = 1.0;
        if !player.ped.fight_combo_move
            && fight_move.combo_follow_on_time > 0.0
            && player.ped.fight_button_pressure != 0
            && anim_time > fight_move.combo_follow_on_time
            && (move_id as usize + 1) <= PedFightMoves::FIGHTMOVE_GROUNDKICK as usize
            && let Some(next_move) =
                crate::peds::PedFight::fight_move_from_index(move_id as usize + 1)
        {
            player.ped.fight_button_pressure = 0;
            player.ped.fight_combo_move = true;
            let _ = start_fight_move(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                next_move,
                8.0,
                Some(0.1),
            );
            return true;
        }
    } else if fight_move.hit_level == FightMoveHitLevel::HITLEVEL_NULL
        && fight_move.start_fire_time > 0.0
        && move_id != PedFightMoves::FIGHTMOVE_SHUFFLE_F
        && anim_time > fight_move.start_fire_time
    {
        player.animation.movement_anim_clump.associations[association_index].speed = 1.3;
    }

    if association_running {
        return true;
    }

    player.ped.fight_combo_move = false;
    if player.ped.fight_button_pressure != 0 {
        let targets = fight_target_samples(other_player.as_ref().map(|player| player.as_ref()), script_entities);
        let pressure = player.ped.fight_button_pressure;
        let (next_move, heading) = next_player_fight_move(&player.ped, player.locomotion.rotation_cur, pressure, &targets);
        if let Some(heading) = heading {
            player.locomotion.rotation_dest = heading;
            player.locomotion.rotation_cur = heading;
        }
        player.ped.fight_button_pressure = 0;
        let _ = start_fight_move(
            &mut player.ped,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            next_move,
            4.0,
            None,
        );
    } else if player.ped.take_step_after_attack && move_id != PedFightMoves::FIGHTMOVE_SHUFFLE_F {
        let targets = fight_target_samples(other_player.as_ref().map(|player| player.as_ref()), script_entities);
        let has_front_target = targets.iter().any(|target| {
            let (distance, turn, _) = fight_target_metrics(&player.ped, player.locomotion.rotation_cur, target.position);
            !fight_target_on_ground(target.state) && distance < 3.0 && turn < 30.0_f32.to_radians()
        });
        if has_front_target {
            player.ped.take_step_after_attack = false;
            let _ = start_fight_move(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                PedFightMoves::FIGHTMOVE_SHUFFLE_F,
                32.0,
                None,
            );
        } else {
            end_fight(
                &mut player.ped,
                &mut player.animation.movement_anim_clump,
                anim_manager,
                fight_moves,
                FightEndType::Fast,
            );
        }
    } else if player.ped.take_step_after_attack {
        end_fight(
            &mut player.ped,
            &mut player.animation.movement_anim_clump,
            anim_manager,
            fight_moves,
            FightEndType::Fast,
        );
    } else {
        player.ped.cur_fight_move = PedFightMoves::FIGHTMOVE_IDLE;
        player.ped.fight_state = FIGHTSTATE_MOVE_FINISHED;
        player.ped.wait_timer = now_ms.saturating_add(500);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_player_weapon(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    mut other_player: Option<&mut PlayerPedMut<'_>>,
    pad: &crate::core::Pad::CPad,
    camera_heading: Option<f32>,
    camera_front: Option<CVector>,
    using_mouse_cam: bool,
    previous_ms: u32,
    now_ms: u32,
    fast_reload: bool,
    weapon_info: &CWeaponInfoStore,
    ped_stats: Option<&CPedStatsRuntime>,
    anim_manager: &CAnimManager,
    file_loader: &CFileLoader,
    script_entities: &mut ScriptEntityWorld,
    particles: &mut ParticleEmitter<'_>,
    mut special_fx: Option<&mut CSpecialFX>,
    general: &mut CGeneral,
    mut darkel: Option<&mut CDarkel>,
    mut stats: Option<&mut CStats>,
    events: &mut CEventList,
    replay_playing_back: bool,
    mut audio: Option<&mut cDMAudio>,
    mut projectiles: Option<&mut CProjectileInfoPool>,
    mut explosions: Option<&mut CExplosionPool>,
    mut shots: Option<&mut CShotInfoPool>,
    water: Option<&CWaterLevel>,
    mut point_lights: Option<&mut CPointLights>,
    camera_position: Option<CVector>,

    frame_counter: u32,
) {
    let jumping = player.ped.ped_state == PedState::PED_JUMP;
    let sprinting = player.ped.move_state == eMoveState::PEDMOVE_SPRINT;
    player.ped.flags.is_shooting = !jumping && !sprinting && pad.get_weapon();
    let weapon_type = player.ped.current_weapon;
    let slot = weapon_type as usize;
    let Some(info) = weapon_info.GetWeaponInfo(weapon_type).copied() else {
        return;
    };
    let Some(weapon) = player.ped.weapons.get_mut(slot) else {
        return;
    };
    let audio_entity_valid = player.ped.physical.audio_entity_id >= 0;
    let play_reload = weapon.Update(previous_ms, now_ms, audio_entity_valid, weapon_info);
    if play_reload && let Some(audio) = audio.as_deref_mut() {
        audio.PlayOneShot(player.ped.physical.audio_entity_id, SOUND_WEAPON_RELOAD as u16, 0.0);
    }
    // CPlayerPed::ProcessControl does not dispatch ProcessPlayerWeapon from
    // PED_JUMP. Keep the weapon timer serviced, but do not let aiming/firing
    // overwrite PED_JUMP and accidentally act as an escape hatch from a bad
    // landing state.
    if jumping {
        return;
    }
    let first_person = player.ped.ped_state == PedState::PED_SNIPER_MODE;

    // CPed::Attack starts a reload association after the last clip round.
    // CWeapon::Update refills the clip at its own timer; SetAttack remains
    // blocked until the authored reload association is deleted.
    if !first_person && let Some(reload_id) = crate::weapons::WeaponRuntime::reload_animation(&info) {
        if weapon.weapon_state == eWeaponState::WEAPONSTATE_RELOADING
            && !player.animation.movement_anim_clump.associations.iter().any(|a| a.anim_id == reload_id)
        {
            blend_player_animation(&mut player.animation.movement_anim_clump, anim_manager,
                crate::animation::AnimAssocDefinitions::ASSOCGRP_STD, reload_id, 8.0);
            player.ped.flags.is_attacking = false;
            player.ped.flags.is_pointing_gun_at = false;
        }
        if player.animation.movement_anim_clump.associations.iter().any(|a| a.anim_id == reload_id) {
            player.ped.flags.is_shooting = false;
            return;
        }
    }
    if weapon.weapon_state == eWeaponState::WEAPONSTATE_RELOADING {
        return;
    }

    if !first_person && matches!(weapon_type, eWeaponType::WEAPONTYPE_SNIPERRIFLE | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER) {
        return;
    }
    let (crossed_fire_frame, animation_id) = if first_person {
        (if weapon_type == eWeaponType::WEAPONTYPE_M16 { pad.get_weapon() } else { pad.weapon_just_down() }, info.anim_to_play)
    } else {
    let is_aimable_weapon = matches!(
        weapon_type,
        eWeaponType::WEAPONTYPE_COLT45
            | eWeaponType::WEAPONTYPE_UZI
            | eWeaponType::WEAPONTYPE_SHOTGUN
            | eWeaponType::WEAPONTYPE_AK47
            | eWeaponType::WEAPONTYPE_M16
            | eWeaponType::WEAPONTYPE_SNIPERRIFLE
            | eWeaponType::WEAPONTYPE_ROCKETLAUNCHER
            | eWeaponType::WEAPONTYPE_FLAMETHROWER
    );
    if is_aimable_weapon && pad.get_target() {
        if let Some(beta) = camera_heading {
            let heading = -beta;
            player.locomotion.rotation_cur = heading;
            player.locomotion.rotation_dest = heading;
            player.ped.physical.entity.set_heading(heading);
        }
        if player.ped.ped_state == PedState::PED_IDLE {
            player.ped.ped_state = PedState::PED_AIM_GUN;
        }
    } else if player.ped.ped_state == PedState::PED_AIM_GUN && !pad.get_target() {
        player.ped.ped_state = PedState::PED_IDLE;
    }

    if player.ped.ped_state == PedState::PED_AIM_GUN {
        let has_aim_assoc = player.animation.movement_anim_clump.associations.iter().any(|a| {
            [info.anim_to_play, info.anim2_to_play].contains(&a.anim_id) && a.blend_delta >= 0.0
        });
        if !has_aim_assoc {
            blend_player_animation(&mut player.animation.movement_anim_clump, anim_manager,
                crate::animation::AnimAssocDefinitions::ASSOCGRP_STD, info.anim_to_play, 8.0);
            if let Some(assoc) = player.animation.movement_anim_clump.associations.iter_mut()
                .find(|a| a.anim_id == info.anim_to_play) {
                assoc.set_current_time(info.anim_loop_start);
                assoc.flags &= !crate::animation::AnimBlendAssociation::ASSOC_RUNNING;
            }
        }
    } else if player.ped.ped_state != PedState::PED_ATTACK {
        player.animation.movement_anim_clump.associations.retain_mut(|a| {
            if [info.anim_to_play, info.anim2_to_play].contains(&a.anim_id) {
                a.blend_delta = -8.0;
                a.flags |= crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT;
                a.flags &= !crate::animation::AnimBlendAssociation::ASSOC_RUNNING;
            }
            true
        });
    }

    let trigger = if sprinting {
        false
    } else if weapon_type == eWeaponType::WEAPONTYPE_UNARMED {
        pad.weapon_just_down()
    } else {
        // re3 calls SetAttack every frame while GetWeapon is held. The
        // animation/weapon state controls the actual fire cadence.
        pad.get_weapon()
    };
    let throwable = info.IsFlagSet(WEAPONFLAG_THROW);
    let throw_under = crate::animation::AnimationId::ANIM_STD_THROW_UNDER;
    let weapon_throw = crate::animation::AnimationId::ANIM_STD_WEAPON_THROW;
    let start_throw = crate::animation::AnimationId::ANIM_STD_START_THROW;
    let attack_animation_ids = [info.anim_to_play, info.anim2_to_play, throw_under];

    // re3 FinishedAttackCB turns the short WEAPON_start_throw association into
    // the actual overhand/underhand throw. WEAPON.DAT deliberately places the
    // start animation's fire frame beyond that animation's length, so omitting
    // this callback means grenades/molotovs can never fire.
    if player.ped.ped_state == PedState::PED_ATTACK
        && throwable
        && let Some(index) =
            player
                .animation.movement_anim_clump
                .associations
                .iter()
                .position(|association| {
                    association.anim_id == start_throw
                        && !association.is_running()
                })
    {
        let next_anim = if player.state.have_target_selected {
            throw_under
        } else {
            weapon_throw
        };
        player.animation.movement_anim_clump.associations.remove(index);
        player.animation.movement_anim_clump.associations.retain(|association|
            association.anim_id != weapon_throw && association.anim_id != throw_under);
        if let Some(mut association) = anim_manager.create_anim_association(
            crate::animation::AnimAssocDefinitions::ASSOCGRP_STD,
            next_anim as usize,
        ) {
            association.set_current_time(0.0);
            association.set_run();
            player.animation.movement_anim_clump.associations.push(association);
            player.ped.ped_state = PedState::PED_ATTACK;
        } else {
            player.ped.ped_state = PedState::PED_IDLE;
        }
        return;
    }

    // FinishedAttackCB clears PED_ATTACK after the actual throw animation.
    if player.ped.ped_state == PedState::PED_ATTACK
        && throwable
        && let Some(index) =
            player
                .animation.movement_anim_clump
                .associations
                .iter()
                .position(|association| {
                    (association.anim_id == weapon_throw || association.anim_id == throw_under)
                        && !association.is_running()
                })
    {
        player.animation.movement_anim_clump.associations.remove(index);
        if player.ped.ped_state == PedState::PED_ATTACK {
            player.ped.ped_state = if pad.get_target() && is_aimable_weapon {
                PedState::PED_AIM_GUN
            } else {
                PedState::PED_IDLE
            };
        }
        return;
    }

    let attack_index = player
        .animation.movement_anim_clump
        .associations
        .iter()
        .position(|association| {
            attack_animation_ids.contains(&association.anim_id)
                && association.is_running()
                && association.blend_delta >= 0.0
        });
    let Some(attack_index) = attack_index else {
        if !trigger {
            if player.ped.ped_state == PedState::PED_ATTACK {
                player.ped.ped_state = if pad.get_target() && is_aimable_weapon {
                    PedState::PED_AIM_GUN
                } else {
                    PedState::PED_IDLE
                };
            }
            return;
        }
        player
            .animation.movement_anim_clump
            .associations
            .retain(|association| !attack_animation_ids.contains(&association.anim_id));
        blend_player_animation(&mut player.animation.movement_anim_clump, anim_manager,
            crate::animation::AnimAssocDefinitions::ASSOCGRP_STD, info.anim_to_play,
            if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE { 1000.0 } else { 8.0 });
        if let Some(association) = player.animation.movement_anim_clump.associations.iter_mut()
            .find(|a| a.anim_id == info.anim_to_play) {
            association.set_current_time(0.0);
            association.set_run();
            player.state.attack_button_counter = 0.0;
            player.ped.ped_state = PedState::PED_ATTACK;
            player.ped.move_state = eMoveState::PEDMOVE_NONE;
            player.ped.flags.is_attacking = true;
        } else {
            bevy::log::warn_once!(target: "re3bevy::weapons", "attack animation unavailable");
        }
        return;
    };

    let (animation_time, animation_step, animation_running, animation_id) = {
        let association = &player.animation.movement_anim_clump.associations[attack_index];
        (
            association.current_time,
            association.time_step,
            association.is_running(),
            association.anim_id,
        )
    };
    let fire_frame = if throwable && animation_id == throw_under {
        0.2
    } else if animation_id == info.anim2_to_play {
        info.anim2_frame_fire
    } else {
        info.anim_frame_fire
    };
    let crossed_fire_frame = animation_time > fire_frame
        && animation_time - animation_step <= fire_frame
        && animation_running;

    if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN
        && animation_time > 1.0
        && animation_time - animation_step <= 1.0
        && animation_running
    {
        let mut shell_source = player_weapon_fire_source(
            &player.ped.physical.entity,
            &player.animation.movement_anim_clump,
            &info,
            file_loader,
        );
        shell_source.z -= 0.15 * player.ped.physical.entity.get_up().z;
        add_weapon_gunshell(
            particles,
            general,
            shell_source,
            player.ped.physical.entity.get_forward(),
            0.6,
            0.025,
        );
    }

    let mut loop_end = info.anim_loop_end;
    if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE && animation_id == info.anim2_to_play {
        loop_end = 3.4 / 6.0;
    }
    if animation_time > loop_end
        || (!animation_running && info.weapon_fire != eWeaponFire::WEAPON_FIRE_PROJECTILE)
    {
        if animation_time - 2.0 * animation_step <= loop_end
            && trigger
            && weapon.weapon_state != eWeaponState::WEAPONSTATE_RELOADING
        {
            let association = &mut player.animation.movement_anim_clump.associations[attack_index];
            association.set_current_time(info.anim_loop_start);
            association.set_run();
            player.ped.ped_state = PedState::PED_ATTACK;
        } else {
            // re3 PedFight.cpp: automatic rifles/submachine gun emit their
            // distinct tail echo exactly when the attack loop ends.
            if animation_time - animation_step <= loop_end
                && let Some(sound) = match weapon_type {
                    eWeaponType::WEAPONTYPE_UZI => Some(SOUND_WEAPON_UZI_BULLET_ECHO),
                    eWeaponType::WEAPONTYPE_AK47 => Some(SOUND_WEAPON_AK47_BULLET_ECHO),
                    eWeaponType::WEAPONTYPE_M16 => Some(SOUND_WEAPON_M16_BULLET_ECHO),
                    _ => None,
                }
                && let Some(audio) = audio.as_deref_mut()
            {
                audio.PlayOneShot(player.ped.physical.audio_entity_id, sound as u16, 0.0);
            }
            let association = &mut player.animation.movement_anim_clump.associations[attack_index];
            association.blend_delta = -8.0;
            association.flags |= crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT;
            association.flags &= !crate::animation::AnimBlendAssociation::ASSOC_RUNNING;
            if player.ped.ped_state == PedState::PED_ATTACK {
                player.ped.ped_state = if pad.get_target() && is_aimable_weapon {
                    PedState::PED_AIM_GUN
                } else {
                    PedState::PED_IDLE
                };
            }
        }
    }
    (crossed_fire_frame, animation_id)
    };
    if !crossed_fire_frame {
        return;
    }

    if info.weapon_fire == eWeaponFire::WEAPON_FIRE_MELEE {
        let shooter_position = player.ped.physical.entity.get_position();
        let mut fire_offset = info.fire_offset;
        let bat_second_animation = weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT
            && animation_id == info.anim2_to_play;
        if bat_second_animation {
            fire_offset.z = 0.7 * info.radius - 1.0;
        }
        // re3 PedFight.cpp transforms unarmed/baseball-bat fire offsets by the
        // ped matrix before CWeapon::FireMelee performs sphere tests.
        let fire_source = player.ped.physical.entity.transform_point(fire_offset);
        let mut made_contact = false;

        if let Some(other) = other_player.as_deref_mut() {
            if let Some(collision_dist) =
                melee_ped_collision(&other.ped, &other.animation, fire_source, info.radius, file_loader)
            {
                let target = other.position();
                if !(other.ped.is_player() && other.ped.ped_state == PedState::PED_GETUP) {
                    let health_before = other.vitality.health;
                    let local_direction = other.ped.get_local_direction(other.locomotion.rotation_cur, CVector::new(
                        shooter_position.x - target.x,
                        shooter_position.y - target.y,
                        0.0,
                    ));
                    let damage = if bat_second_animation {
                        100.0
                    } else if player.state.adrenaline_active {
                        3.5 * info.damage as f32
                    } else if weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT {
                        2.0 * info.damage as f32
                    } else {
                        info.damage as f32
                    };
                    let defend_weakness = ped_defend_weakness(&other.ped, ped_stats);
                    let can_be_damaged = other.can_be_damaged;
                    let _ = other.ped.inflict_weapon_damage(other.vitality, 
                        weapon_type,
                        damage,
                        defend_weakness,
                        true,
                        can_be_damaged,
                        now_ms,
                        local_direction,
                    );
                    apply_melee_fall_response(
                        &mut other.ped, &other.vitality,
                        weapon_type,
                        animation_id == info.anim2_to_play,
                        health_before,
                        shooter_position,
                        target,
                        local_direction,
                        now_ms,
                    
    frame_counter,
);
                    emit_melee_blood(
                        particles,
                        fire_source,
                        collision_dist,
                        weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT,
                        general,
                    );
                    made_contact = true;
                }
            }
        }

        let hit_handles: Vec<(i32, CVector)> = script_entities
            .peds
            .iter()
            .filter_map(|(&handle, victim)| {
                melee_ped_collision(&victim.ped, &victim.animation, fire_source, info.radius, file_loader)
                    .map(|collision_dist| (handle, collision_dist))
            })
            .collect();
        for (handle, collision_dist) in hit_handles {
            let Some(victim) = script_entities.peds.get_mut(&script_entities.index, &handle) else {
                continue;
            };
            if victim.ped.is_player() && victim.ped.ped_state == PedState::PED_GETUP {
                continue;
            }
            let victim_position = victim.ped.physical.entity.get_position();
            let health_before = victim.vitality.health;
            let local_direction = victim.ped.get_local_direction(victim.locomotion.rotation_cur, CVector::new(
                shooter_position.x - victim_position.x,
                shooter_position.y - victim_position.y,
                0.0,
            ));
            let damage = if bat_second_animation {
                100.0
            } else if player.state.adrenaline_active {
                3.5 * info.damage as f32
            } else {
                info.damage as f32
            };
            let defend_weakness = ped_defend_weakness(&victim.ped, ped_stats);
            let result = victim.ped.inflict_weapon_damage(victim.vitality, 
                weapon_type,
                damage,
                defend_weakness,
                true,
                true,
                now_ms,
                local_direction,
            );
            apply_melee_fall_response(
                &mut *victim.ped, &victim.vitality,
                weapon_type,
                animation_id == info.anim2_to_play,
                health_before,
                shooter_position,
                victim_position,
                local_direction,
                now_ms,
            
    frame_counter,
);
            let event_type = if victim.ped.ped_type == ePedType::PEDTYPE_COP {
                EEventType::AssaultPolice
            } else {
                EEventType::Assault
            };
            let _ = events.register_entity_event(
                event_type,
                EEventEntity::Ped,
                handle,
                Some(player.state.pool_handle),
                victim_position,
                2_000,
                now_ms,
            );
            if result == PedWeaponDamageResult::Killed
                && let (Some(darkel), Some(stats)) = (darkel.as_deref_mut(), stats.as_deref_mut())
            {
                darkel.register_kill_by_player(
                    victim.model_id,
                    victim.ped.ped_type as usize,
                    victim.ped.flags.chris_criminal,
                    weapon_type,
                    false,
                    replay_playing_back,
                    stats,
                    audio.as_deref_mut(),
                );
            }
            emit_melee_blood(
                particles,
                fire_source,
                collision_dist,
                weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT,
                general,
            );
            made_contact = true;
        }

        if made_contact {
            weapon.weapon_state = eWeaponState::WEAPONSTATE_MELEE_MADECONTACT;
            if let Some(audio) = audio.as_deref_mut() {
                let sound = if weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT {
                    SOUND_WEAPON_BAT_ATTACK
                } else {
                    SOUND_FIGHT_PUNCH_39
                };
                audio.PlayOneShot(player.ped.physical.audio_entity_id, sound as u16, 0.0);
            }
        } else if weapon.weapon_state != eWeaponState::WEAPONSTATE_FIRING {
            weapon.timer = now_ms.wrapping_add(info.reload);
            weapon.weapon_state = eWeaponState::WEAPONSTATE_FIRING;
        }
        return;
    }
    if weapon.ammo_in_clip <= 0 || weapon.ammo_total <= 0 {
        if weapon.ammo_total > 0 {
            weapon.weapon_state = eWeaponState::WEAPONSTATE_RELOADING;
            weapon.timer = now_ms.wrapping_add(info.reload);
        } else {
            weapon.weapon_state = eWeaponState::WEAPONSTATE_OUT_OF_AMMO;
        }
        return;
    }

    if weapon_type == eWeaponType::WEAPONTYPE_DETONATOR {
        let bomb_rigger = usize::try_from(player.state.pool_handle).ok();
        if let Some(bomb_rigger) = bomb_rigger {
            for vehicle in script_entities.vehicles.values_mut() {
                let crate::control::ScriptEntities::ScriptVehicleMut::Automobile { mut car, .. } = vehicle else {
                    continue;
                };
                if car.bomb_type == eBombType::CARBOMB_REMOTE
                    && car.bomb_rigger == Some(bomb_rigger)
                {
                    car.bomb_type = eBombType::CARBOMB_NONE;
                    car.vehicle.vitality.bomb_timer = 500;
                    car.vehicle.vitality.blow_up_entity = car.bomb_rigger.map(|handle|
                        crate::entities::EntityReference::EntityReference::Ped(handle as i32));
                }
            }
        }
        // re3 CWeapon::Fire resets detonator ammo to one before the common
        // successful-fire decrement, leaving the detonator spent afterwards.
        weapon.ammo_total = 1;
        weapon.ammo_in_clip = 1;
        weapon.finish_successful_fire(now_ms, true, fast_reload, weapon_info);
        player.ped.flags.is_shooting = true;
        if let Some(audio) = audio.as_deref_mut() {
            audio.PlayOneShot(
                player.ped.physical.audio_entity_id,
                SOUND_WEAPON_SHOT_FIRED as u16,
                0.0,
            );
        }
        return;
    }

    // Mouse third-person fire faces the same horizontal direction as the
    // camera. GTA's CMatrix heading sign is opposite camera beta.
    if !first_person && let Some(beta) = camera_heading {
        let heading = -beta;
        player.locomotion.rotation_cur = heading;
        player.locomotion.rotation_dest = heading;
        player.ped.physical.entity.set_heading(heading);
    }
    let entity = &player.ped.physical.entity;
    let source = if first_person && weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER {
        entity.transform_point(CVector::new(0.0, 0.0, 0.6))
    } else if first_person { camera_position.unwrap_or(entity.get_position()) } else { player_weapon_fire_source(
        &player.ped.physical.entity,
        &player.animation.movement_anim_clump,
        &info,
        file_loader,
    ) };
    let ped_direction = entity.get_forward();
    let camera_direction = camera_front.and_then(|front| {
        let length = front.magnitude();
        (length > 1.0e-6).then_some(CVector::new(
            front.x / length,
            front.y / length,
            front.z / length,
        ))
    });
    let mut direction = if first_person || using_mouse_cam {
        camera_direction.unwrap_or(ped_direction)
    } else {
        ped_direction
    };
    if !first_person && let Some(target_position) = player.ped.point_gun_at
        .and_then(|slot| script_entities.ped_handle_for_slot(slot))
        .and_then(|handle| script_entities.peds.get(&script_entities.index, &handle))
        .and_then(|target| crate::renderer::BevyRender::ped_frame_world_point_static(
            target.model_id.max(0) as usize, file_loader, &target.ped.physical.entity.matrix,
            "Smid", CVector::ZERO))
        .or_else(|| player.state.weapon_lock_on_object.and_then(|handle| script_entities.objects.get(&script_entities.index, &handle))
            .map(|object| object.object.physical.entity.get_position()))
    {
        let to_target = CVector::new(target_position.x - source.x,
            target_position.y - source.y, target_position.z - source.z);
        if to_target.magnitude() > 1.0e-6 {
            direction = crate::weapons::WeaponRuntime::scaled_vector(to_target, 1.0 / to_target.magnitude());
        }
    }
    let ray_source = if !first_person && using_mouse_cam && player.ped.point_gun_at.is_none() && player.state.weapon_lock_on_object.is_none() {
        camera_position.map(|camera| crate::weapons::WeaponRuntime::project_camera_source(camera, source, direction)).unwrap_or(source)
    } else { source };

    if matches!(weapon_type, eWeaponType::WEAPONTYPE_ROCKETLAUNCHER | eWeaponType::WEAPONTYPE_GRENADE | eWeaponType::WEAPONTYPE_MOLOTOV) {
        let Some(projectiles) = projectiles.as_deref_mut() else {
            bevy::log::warn_once!(target: "re3bevy::weapons", "projectile runtime resource unavailable");
            return;
        };
        let power = if weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER { 0.0 } else { player.state.attack_button_counter * 0.0375 };
        crate::weapons::WeaponRuntime::fire_projectile(projectiles, player.state.pool_handle as u32,
            entity.get_position(), ped_direction, source,
            if weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER { camera_direction.unwrap_or(ped_direction) } else { ped_direction },
            weapon_type, power, now_ms, script_entities, file_loader);
        if weapon_type == eWeaponType::WEAPONTYPE_GRENADE && let Some(stats) = stats.as_deref_mut() {
            stats.kgs_of_explosives_used = stats.kgs_of_explosives_used.saturating_add(1);
        }
        weapon.finish_successful_fire(now_ms, true, fast_reload, weapon_info);
        player.ped.flags.is_shooting = true;
        if let Some(audio) = audio.as_deref_mut() {
            audio.PlayOneShot(player.ped.physical.audio_entity_id, SOUND_WEAPON_SHOT_FIRED as u16, 0.0);
        }
        return;
    }
    if weapon_type == eWeaponType::WEAPONTYPE_FLAMETHROWER {
        let area_dir = crate::weapons::WeaponRuntime::scaled_vector(direction, if using_mouse_cam { 1.0 } else { 0.5 });
        let target = CVector::new(
            ray_source.x + area_dir.x * if using_mouse_cam { info.range } else { 1.0 },
            ray_source.y + area_dir.y * if using_mouse_cam { info.range } else { 1.0 },
            ray_source.z + area_dir.z * if using_mouse_cam { info.range } else { 1.0 },
        );
        if let Some(shots) = shots.as_deref_mut() {
            shots.AddShot(
                Some(player.state.pool_handle as u32),
                weapon_type,
                source,
                target,
                now_ms,
                weapon_info,
                general,
            );
        }

        // re3 GenerateFlameThrowerParticles: five fireballs at the muzzle,
        // with velocity multiplied by 0.7 before each emission.
        let mut flame_dir = area_dir;
        for _ in 0..5 {
            flame_dir.x *= 0.7;
            flame_dir.y *= 0.7;
            flame_dir.z *= 0.7;
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_FIREBALL,
                source,
                flame_dir,
                None,
                0.0,
                0,
                0,
                0,
                0,
            );
        }
        let was_ready = weapon.weapon_state == eWeaponState::WEAPONSTATE_READY;
        weapon.finish_successful_fire(now_ms, true, fast_reload, weapon_info);
        player.ped.flags.is_shooting = true;
        if let Some(audio) = audio.as_deref_mut() {
            audio.PlayOneShot(
                player.ped.physical.audio_entity_id,
                SOUND_WEAPON_SHOT_FIRED as u16,
                0.0,
            );
            if was_ready {
                audio.PlayOneShot(
                    player.ped.physical.audio_entity_id,
                    crate::audio::soundlist::SOUND_WEAPON_FLAMETHROWER_FIRE as u16,
                    0.0,
                );
            }
        }
        return;
    }
    if first_person {
        // CWeapon::FireM16_1stPerson emits impact FX, not hand muzzle/shell FX.
    } else if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN {
        emit_shotgun_muzzle_fx(
            source,
            direction,
            particles,
            point_lights.as_deref_mut(),
            camera_position,
        );
    } else {
        let emit_automatic_flash = match weapon_type {
            eWeaponType::WEAPONTYPE_AK47 => {
                player.state.ak47_muzzle_flash_counter = player.state.ak47_muzzle_flash_counter.wrapping_add(1);
                player.state.ak47_muzzle_flash_counter & 1 == 0
            }
            eWeaponType::WEAPONTYPE_M16 => {
                player.state.m16_muzzle_flash_counter = player.state.m16_muzzle_flash_counter.wrapping_add(1);
                player.state.m16_muzzle_flash_counter & 1 == 0
            }
            _ => true,
        };
        emit_instant_hit_muzzle_fx(
            weapon_type,
            source,
            direction,
            crate::weapons::WeaponRuntime::scaled_vector(player.ped.physical.move_speed, 1.0 / GTA_PHYSICS_HZ),
            emit_automatic_flash,
            particles,
            general,
            point_lights.as_deref_mut(),
            camera_position,
        );
    }
    let _ = events.register_entity_event(
        EEventType::Gunshot,
        EEventEntity::Ped,
        player.state.pool_handle,
        Some(player.state.pool_handle),
        source,
        1_000,
        now_ms,
    );
    let target = CVector::new(
        ray_source.x + direction.x * info.range,
        ray_source.y + direction.y * info.range,
        ray_source.z + direction.z * info.range,
    );
    if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN {
        fire_player_shotgun_pellets(
            player.state.pool_handle,
            player.ped.physical.audio_entity_id,
            other_player.as_deref_mut(),
            source,
            ray_source,
            direction,
            using_mouse_cam,
            info.range,
            info.damage,
            now_ms,
            ped_stats,
            file_loader,
            script_entities,
            particles,
            special_fx.as_deref_mut(),
            general,
            darkel.as_deref_mut(),
            stats.as_deref_mut(),
            events,
            replay_playing_back,
            audio.as_deref_mut(),
        
    frame_counter,
);
        weapon.finish_successful_fire(now_ms, true, fast_reload, weapon_info);
        player.ped.flags.is_shooting = true;
        if let Some(audio) = audio.as_deref_mut() {
            audio.PlayOneShot(
                player.ped.physical.audio_entity_id,
                SOUND_WEAPON_SHOT_FIRED as u16,
                0.0,
            );
        }
        return;
    }

    if let Some(stats) = stats.as_deref_mut() {
        stats.instant_hits_fired_by_player += 1;
    }
    let mut trace_end = target;
    match closest_weapon_ray_hit(
        ray_source,
        direction,
        info.range,
        other_player.as_ref().map(|player| player.as_ref()),
        None,
        None,
        script_entities,
        file_loader,
    ) {
        Some(WeaponRayHit::Player { point, .. }) => {
            trace_end = point;
            if let Some(other) = other_player.as_deref_mut() {
                let victim_position = other.position();
                let direction = apply_player_gun_hit_reaction(other, weapon_type, source, now_ms,
    frame_counter,
);
                let defend_weakness = ped_defend_weakness(&other.ped, ped_stats);
                let _ = other.ped.inflict_weapon_damage(other.vitality, 
                    weapon_type,
                    info.damage as f32,
                    defend_weakness,
                    true,
                    other.can_be_damaged,
                    now_ms,
                    direction,
                );
                emit_weapon_blood(particles, point, source, victim_position, 8);
                register_player_shoot_ped_event(
                    events,
                    other.pool_handle,
                    &other.ped,
                    player.state.pool_handle,
                    now_ms,
                );
                if let Some(stats) = stats.as_deref_mut() {
                    stats.instant_hits_hit_by_player += 1;
                }
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(player.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 0.0);
                }
            }
        }
        Some(WeaponRayHit::Vehicle {
            handle,
            point,
            normal,
            ..
        }) => {
            trace_end = point;
            if let Some(mut veh) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                let mut base = veh.base_mut();
                base.inflict_weapon_damage(weapon_type, info.damage as f32);
                emit_bullet_surface_impact(particles, point, normal, 16, general);
                if let Some(stats) = stats.as_deref_mut() {
                    stats.instant_hits_hit_by_player += 1;
                }
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(
                        player.ped.physical.audio_entity_id,
                        SOUND_WEAPON_HIT_VEHICLE as u16,
                        0.0,
                    );
                }
            }
        }
        Some(WeaponRayHit::Ped { handle, point, .. }) => {
            trace_end = point;
            if let Some(victim) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                let victim_position = victim.ped.physical.entity.get_position();
                let direction =
                    apply_ped_gun_hit_reaction(&mut *victim.ped, victim.locomotion.rotation_cur, &victim.vitality, weapon_type, source, now_ms,
    frame_counter,
);
                let defend_weakness = ped_defend_weakness(&victim.ped, ped_stats);
                let damage_result = victim.ped.inflict_weapon_damage(victim.vitality, 
                    weapon_type,
                    info.damage as f32,
                    defend_weakness,
                    true,
                    true,
                    now_ms,
                    direction,
                );
                register_player_shoot_ped_event(
                    events,
                    handle,
                    &victim.ped,
                    player.state.pool_handle,
                    now_ms,
                );
                emit_weapon_blood(particles, point, source, victim_position, 8);
                if let Some(stats) = stats.as_deref_mut() {
                    stats.instant_hits_hit_by_player += 1;
                }
                if damage_result == PedWeaponDamageResult::Killed
                    && let (Some(darkel), Some(stats)) =
                        (darkel.as_deref_mut(), stats.as_deref_mut())
                {
                    darkel.register_kill_by_player(
                        victim.model_id,
                        victim.ped.ped_type as usize,
                        victim.ped.flags.chris_criminal,
                        weapon_type,
                        false,
                        replay_playing_back,
                        stats,
                        audio.as_deref_mut(),
                    );
                }
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(player.ped.physical.audio_entity_id, SOUND_WEAPON_HIT_PED as u16, 0.0);
                }
            }
        }
        Some(WeaponRayHit::Object {
            handle,
            point,
            normal,
            ..
        }) => {
            trace_end = point;
            // re3 CWeapon::DoBulletImpact object branch: eight sparks and
            // physical push, without the building/vehicle bullet-smoke puff.
            emit_bullet_sparks(particles, point, normal, 8);
            if let Some(object) = script_entities.objects.get_mut(&script_entities.index, &handle) {
                apply_weapon_force_to_object(object.object, normal, 4.0);
            }
            if let Some(explosions) = explosions.as_deref_mut() {
                crate::weapons::WeaponRuntime::blow_up_explosive_object(handle, Some(player.state.pool_handle as u32),
                    script_entities, file_loader, explosions, general, now_ms);
            }
        }
        Some(WeaponRayHit::Static { point, normal, .. }) => {
            trace_end = point;
            emit_bullet_surface_impact(particles, point, normal, 16, general);
        }
        None => {
            if let Some(sample) = water.and_then(|water| {
                water.get_water_level(target.x, target.y, target.z + 10.0, false, now_ms, 0.0)
            }) {
                particles.add_particle(
                    crate::renderer::ParticleType::PARTICLE_BOAT_SPLASH,
                    CVector::new(target.x, target.y, sample.level),
                    CVector::new(0.0, 0.0, 0.01),
                    None,
                    0.0,
                    0,
                    0,
                    0,
                    250,
                );
            }
        }
    }

    if let Some(special_fx) = special_fx.as_deref_mut() {
        special_fx.add_native_bullet_trace(source, trace_end, general);
    }

    weapon.finish_successful_fire(now_ms, true, fast_reload, weapon_info);
    player.ped.flags.is_shooting = true;
    if let Some(audio) = audio.as_deref_mut() {
        audio.PlayOneShot(
            player.ped.physical.audio_entity_id,
            SOUND_WEAPON_SHOT_FIRED as u16,
            0.0,
        );
    }
}

pub(crate) fn update_player_current_phys_surface(
    ped: &mut CPed,
    static_support: Option<PlayerStaticGroundSupport>,
    dynamic_support: Option<PlayerDynamicGroundSupport>,
    supported: bool,
) {
    let dynamic_is_selected = dynamic_support.is_some_and(|dynamic| {
        static_support.is_none_or(|static_support| dynamic.z >= static_support.z)
    });
    if supported && dynamic_is_selected {
        let support = dynamic_support.expect("selected dynamic support must exist");
        ped.current_phys_surface = Some(support.entity);
        ped.offset_from_phys_surface = support.offset_from_surface;
        ped.physical.surface_touched = support.surface;
        if support.surface == crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8 {
            ped.flags.hit_steep_slope = true;
            ped.physical.damage.normal = support.normal;
        }
    } else {
        ped.current_phys_surface = None;
        ped.offset_from_phys_surface = CVector::ZERO;
        if supported && let Some(support) = static_support {
            ped.physical.surface_touched = support.surface;
            if support.surface == crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
            {
                ped.flags.hit_steep_slope = true;
                ped.physical.damage.normal = support.normal;
            }
        }
    }
}

pub(crate) fn player_dynamic_vertical_hit(
    x: f32,
    y: f32,
    line_start_z: f32,
    line_end_z: f32,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
) -> Option<PlayerDynamicGroundSupport> {
    let mut best: Option<PlayerDynamicGroundSupport> = None;
    for (&handle, vehicle) in entities.vehicles.iter() {
        let Some(bevy_entity) = entities.vehicle_entity(handle) else {
            continue;
        };
        let physical_entity = &vehicle.base().physical.entity;
        let candidate = player_dynamic_vertical_hit_entity(
            x,
            y,
            line_start_z,
            line_end_z,
            bevy_entity,
            &physical_entity.matrix,
            physical_entity.model_index.max(0) as usize,
            physical_entity.flags.uses_collision,
            file_loader,
        );
        if let Some(candidate) = candidate
            && best.is_none_or(|current| candidate.z > current.z)
        {
            best = Some(candidate);
        }
    }
    for object in object_colliders {
        let candidate = player_dynamic_vertical_hit_entity(
            x,
            y,
            line_start_z,
            line_end_z,
            object.entity,
            &object.matrix,
            object.model_index,
            object.uses_collision,
            file_loader,
        );
        if let Some(candidate) = candidate
            && best.is_none_or(|current| candidate.z > current.z)
        {
            best = Some(candidate);
        }
    }
    best
}

pub(crate) fn player_dynamic_vertical_hit_entity(
    x: f32,
    y: f32,
    line_start_z: f32,
    line_end_z: f32,
    entity: Entity,
    matrix: &CMatrix,
    model_index: usize,
    uses_collision: bool,
    file_loader: &CFileLoader,
) -> Option<PlayerDynamicGroundSupport> {
    if !uses_collision {
        return None;
    }
    let col = file_loader.model_info.col_model(model_index)?;
    // Native reaches this query through sector/bound-sphere candidates. Keep
    // the line primitive local as well; the broad rejection is important for
    // the per-player support probes that run every frame.
    let bound_center = matrix.transform_point(CVector::new(
        col.bounding_sphere.center.x,
        col.bounding_sphere.center.y,
        col.bounding_sphere.center.z,
    ));
    let bound_radius = col.bounding_sphere.radius;
    let line_min_z = line_start_z.min(line_end_z);
    let line_max_z = line_start_z.max(line_end_z);
    if (x - bound_center.x).abs() > bound_radius
        || (y - bound_center.y).abs() > bound_radius
        || line_max_z < bound_center.z - bound_radius
        || line_min_z > bound_center.z + bound_radius
    {
        return None;
    }
    let to_local = |point: CVector| {
        let origin = matrix.position;
        let d = CVector::new(point.x - origin.x, point.y - origin.y, point.z - origin.z);
        let right = matrix.right;
        let forward = matrix.forward;
        let up = matrix.up;
        CColVector::new(
            d.x * right.x + d.y * right.y + d.z * right.z,
            d.x * forward.x + d.y * forward.y + d.z * forward.z,
            d.x * up.x + d.y * up.y + d.z * up.z,
        )
    };
    let line = CColLine::new(
        to_local(CVector::new(x, y, line_start_z)),
        to_local(CVector::new(x, y, line_end_z)),
    );
    let mut point = CColPoint::new();
    let mut distance = 1.0;
    if !col.process_line_of_sight_primitives(&line, &mut point, &mut distance) {
        return None;
    }
    let world = matrix.transform_point(CVector::new(point.point.x, point.point.y, point.point.z));
    let local_normal = CVector::new(point.normal.x, point.normal.y, point.normal.z);
    let world_normal = matrix.multiply3x3(local_normal);
    let normal_length = world_normal.magnitude();
    let normal = if normal_length > 1.0e-6 {
        CVector::new(
            world_normal.x / normal_length,
            world_normal.y / normal_length,
            world_normal.z / normal_length,
        )
    } else {
        CVector::ZERO
    };
    Some(PlayerDynamicGroundSupport {
        z: world.z,
        entity,
        offset_from_surface: CVector::new(
            world.x - matrix.position.x,
            world.y - matrix.position.y,
            world.z - matrix.position.z,
        ),
        surface: point.surface_b,
        normal,
    })
}

pub(crate) fn player_subdivided_dynamic_landing_support(
    position: CVector,
    ped: &CPed,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
    candidates: &PlayerDynamicMotionCandidates,
) -> Option<PlayerDynamicGroundSupport> {
    // `CPhysical::ProcessCollision` only copies the collision-adjusted Z into
    // savedMatrix for a ped that was not standing before this collision pass.
    // A previously-standing player therefore cannot turn an intermediate car
    // bonnet side contact into a persistent upward step.
    if ped.flags.was_standing {
        return None;
    }
    let raw_model = ped.physical.entity.model_index;
    let player_model_index = if raw_model <= 0 {
        MI_PLAYER
    } else {
        raw_model as usize
    };
    let player_col = file_loader.model_info.col_model(player_model_index)?;
    let mut player_matrix = ped.physical.entity.matrix;
    player_matrix.position = position;
    let mut best: Option<PlayerDynamicGroundSupport> = None;

    for &handle in &candidates.vehicles {
        let Some(vehicle) = entities.vehicles.get(&entities.index, &handle) else {
            continue;
        };
        let Some(entity) = entities.vehicle_entity(handle) else {
            continue;
        };
        let physical = &vehicle.base().physical.entity;
        let Some(col) = file_loader
            .model_info
            .col_model(physical.model_index.max(0) as usize)
        else {
            continue;
        };
        if !physical.flags.uses_collision
            || !col_bound_spheres_overlap(&player_matrix, player_col, &physical.matrix, col)
        {
            continue;
        }
        let support = player_dynamic_vertical_hit_entity(
            position.x,
            position.y,
            position.z,
            position.z - crate::peds::PedPlacement::PED_FEET_OFFSET,
            entity,
            &physical.matrix,
            physical.model_index.max(0) as usize,
            true,
            file_loader,
        );
        if let Some(support) = support
            && best.is_none_or(|current| support.z > current.z)
        {
            best = Some(support);
        }
    }
    for &index in &candidates.objects {
        let Some(object) = object_colliders.get(index) else {
            continue;
        };
        let Some(col) = file_loader.model_info.col_model(object.model_index) else {
            continue;
        };
        if !object.uses_collision
            || !col_bound_spheres_overlap(&player_matrix, player_col, &object.matrix, col)
        {
            continue;
        }
        let support = player_dynamic_vertical_hit_entity(
            position.x,
            position.y,
            position.z,
            position.z - crate::peds::PedPlacement::PED_FEET_OFFSET,
            object.entity,
            &object.matrix,
            object.model_index,
            true,
            file_loader,
        );
        if let Some(support) = support
            && best.is_none_or(|current| support.z > current.z)
        {
            best = Some(support);
        }
    }
    best
}

/// Static-building half of `CPed::ProcessEntityCollision`'s short support
/// line. Unlike the longer CheckIfInTheAir/InTheAir probes this preserves the
/// authored surface id and normal consumed by steep-cliff movement and jump
/// gating.
pub(crate) fn player_static_ground_support(
    x: f32,
    y: f32,
    root_z: f32,
    ped: &CPed,
    dt_seconds: f32,
    file_loader: &CFileLoader,
) -> Option<PlayerStaticGroundSupport> {
    let extension = player_ground_support_extension(ped, dt_seconds);
    let line_start_z = root_z - if ped.flags.was_standing { 0.25 } else { 0.0 };
    let line_end_z = root_z - crate::peds::PedPlacement::PED_FEET_OFFSET - extension;
    file_loader
        .world
        .process_line_of_sight_static_contact(
            [x, y, line_start_z],
            [x, y, line_end_z],
            &file_loader.model_info,
            None,
            false,
        )
        .map(|contact| PlayerStaticGroundSupport {
            z: contact.point[2],
            surface: contact.surface,
            normal: CVector::new(contact.normal[0], contact.normal[1], contact.normal[2]),
        })
}

/// `CPed::InTheAir` looks 4.04 world units below the ped and starts landing
/// once the nearest hit is within 1.3. Use the same finite dynamic query for
/// vehicles/objects instead of the old controller-specific 0.5-step filter.
/// The caller still applies the exact support/landing distance appropriate to
/// its current state.
pub(crate) fn player_dynamic_ground_below(
    x: f32,
    y: f32,
    root_z: f32,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
) -> Option<PlayerDynamicGroundSupport> {
    player_dynamic_vertical_hit(
        x,
        y,
        root_z,
        root_z - 4.04,
        entities,
        object_colliders,
        file_loader,
    )
}

pub(crate) fn ped_defend_weakness(ped: &CPed, ped_stats: Option<&CPedStatsRuntime>) -> f32 {
    ped_stats
        .map(|runtime| runtime.GetPedStats(ped.ped_stats).m_defend_weakness)
        .unwrap_or(1.0)
}

pub(crate) fn melee_ped_collision(
    ped: &CPed,
    animation: &crate::peds::PedAnimation::PedAnimation,
    fire_source: CVector,
    weapon_radius: f32,
    file_loader: &CFileLoader,
) -> Option<CVector> {
    if !ped.physical.entity.flags.uses_collision
        && ped.ped_state != PedState::PED_DEAD
        && ped.ped_state != PedState::PED_DRIVING
    {
        return None;
    }

    let position = ped.physical.entity.get_position();
    let Some(col) = file_loader.model_info.col_model(ped.physical.entity.model_index.max(0) as usize) else {
        bevy::log::warn_once!("melee_collision_waiting_original_ped_bounds");
        return None;
    };
    let broad_radius = col.bounding_sphere.radius + weapon_radius;
    let broad_delta = CVector::new(
        position.x - fire_source.x,
        position.y - fire_source.y,
        position.z - fire_source.z,
    );
    if broad_delta.magnitude() >= broad_radius {
        return None;
    }

    let ground_hit = if ped.on_ground() { true } else {
        let Some(head) = crate::renderer::BevyRender::ped_frame_world_point_from_clump(
            ped.physical.entity.model_index.max(0) as usize, file_loader,
            &animation.movement_anim_clump, &ped.physical.entity.matrix, "Shead", CVector::ZERO,
        ) else {
            bevy::log::warn_once!("melee_collision_waiting_original_head");
            return None;
        };
        head.z <= position.z - 0.3
    };
    let col_model = if ground_hit {
        &file_loader.temp_col_models.col_model_ped_ground_hit
    } else {
        &file_loader.temp_col_models.col_model_ped1
    };
    col_model.spheres.iter().find_map(|sphere| {
        let collision_dist = CVector::new(
            position.x + sphere.center.x - fire_source.x,
            position.y + sphere.center.y - fire_source.y,
            position.z + sphere.center.z - fire_source.z,
        );
        (collision_dist.magnitude() < sphere.radius + weapon_radius).then_some(collision_dist)
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_melee_fall_response(
    ped: &mut CPed, vitality: &crate::peds::PedVitality::PedVitality,
    weapon_type: eWeaponType,
    anim2_playing: bool,
    health_before: f32,
    shooter_position: CVector,
    victim_position: CVector,
    local_direction: u8,
    now_ms: u32,

    frame_counter: u32,
) {
    let is_bat = weapon_type == eWeaponType::WEAPONTYPE_BASEBALLBAT;
    let on_ground = ped.on_ground();
    let should_knock_down = vitality.health > 0.0
        && ((vitality.health < 20.0 && health_before > 20.0) || (is_bat && !ped.is_player()));
    if !on_ground && should_knock_down {
        let dx = shooter_position.x - victim_position.x;
        let dy = shooter_position.y - victim_position.y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0e-6);
        ped.flags.is_standing = false;
        ped.physical
            .apply_move_force(CVector::new(-5.0 * dx / len, -5.0 * dy / len, 3.0));
        let extra_time = if is_bat && ped.is_player() {
            3_000
        } else {
            1_500
        };
        let _ = ped.set_fall(vitality, 
            extra_time,
            crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(local_direction),
            false,
            now_ms,
            frame_counter,);
    } else if on_ground && ped.ped_state == PedState::PED_DIE && !anim2_playing {
        let dx = shooter_position.x - victim_position.x;
        let dy = shooter_position.y - victim_position.y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0e-6);
        ped.flags.is_standing = false;
        ped.physical
            .apply_move_force(CVector::new(-5.0 * dx / len, -5.0 * dy / len, 3.0));
    }
}

pub(crate) fn emit_melee_blood(
    particles: &mut ParticleEmitter<'_>,
    fire_source: CVector,
    collision_dist: CVector,
    is_bat: bool,
    general: &mut CGeneral,
) {
    let blood_pos = CVector::new(
        fire_source.x + collision_dist.x * 0.7,
        fire_source.y + collision_dist.y * 0.7,
        fire_source.z + collision_dist.z * 0.7,
    );
    let magnitude_sq = collision_dist.x * collision_dist.x
        + collision_dist.y * collision_dist.y
        + collision_dist.z * collision_dist.z;
    let scale = if magnitude_sq > 1.0e-8 {
        1.0 / (10.0 * magnitude_sq).sqrt()
    } else {
        0.0
    };
    let mut dir = CVector::new(
        collision_dist.x * scale,
        collision_dist.y * scale,
        collision_dist.z * scale,
    );
    for _ in 0..3 {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_BLOOD,
            blood_pos,
            dir,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    }
    if is_bat {
        for _ in 0..2 {
            dir.x += general.get_random_number_in_range_f32(-0.05, 0.05);
            dir.y += general.get_random_number_in_range_f32(-0.05, 0.05);
            particles.add_particle(
                crate::renderer::ParticleType::PARTICLE_BLOOD,
                blood_pos,
                dir,
                None,
                0.0,
                0,
                0,
                0,
                0,
            );
        }
    }
}

fn gun_hit_local_direction(ped: &CPed, heading: f32, source: CVector) -> u8 {
    let position = ped.physical.entity.get_position();
    ped.get_local_direction(heading, CVector::new(
        source.x - position.x,
        source.y - position.y,
        0.0,
    ))
}

pub(crate) fn install_gun_hit_animation(ped: &mut CPed, direction: u8, now_ms: u32) {
    ped.flags.is_attacking = false;
    ped.flags.is_shooting = false;
    ped.impact_animation_id =
        Some(crate::animation::AnimationId::ANIM_STD_HITBYGUN_FRONT + i32::from(direction));
    ped.impact_animation_start_ms = now_ms;
    ped.impact_animation_speed = 1.0;
    ped.impact_animation_blend_delta = 8.0;
}

pub(crate) fn apply_ped_gun_hit_reaction(
    ped: &mut CPed,
    heading: f32, vitality: &crate::peds::PedVitality::PedVitality,
    weapon_type: eWeaponType,
    source: CVector,
    now_ms: u32,

    frame_counter: u32,
) -> u8 {
    let direction = gun_hit_local_direction(ped, heading, source);
    if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN {
        let position = ped.physical.entity.get_position();
        let dx = source.x - position.x;
        let dy = source.y - position.y;
        let length = (dx * dx + dy * dy).sqrt().max(1.0e-6);
        // re3 FireShotgun uses a stronger standing knockback only if the ped
        // has not recently been in its get-up window. Otherwise it applies the
        // smaller horizontal shove and leaves the current fall/get-up state.
        let cant_stand_up = ped.get_up_timer <= now_ms.wrapping_sub(3_000);
        if ped.flags.is_standing && cant_stand_up {
            ped.flags.is_standing = false;
            ped.physical.apply_move_force(CVector::new(
                -6.0 * dx / length,
                -6.0 * dy / length,
                5.0,
            ));
        } else {
            ped.physical.apply_move_force(CVector::new(
                -2.0 * dx / length,
                -2.0 * dy / length,
                0.0,
            ));
        }
        if cant_stand_up {
            let _ = ped.set_fall(vitality, 
                1_500,
                crate::animation::AnimationId::ANIM_STD_HIGHIMPACT_FRONT + i32::from(direction),
                false,
                now_ms,
            frame_counter,);
        }
    } else {
        if !ped.is_ped_in_control(vitality, ) || ped.flags.is_ducking {
            return direction;
        }
        install_gun_hit_animation(ped, direction, now_ms);
    }
    direction
}

pub(crate) fn apply_player_gun_hit_reaction(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    weapon_type: eWeaponType,
    source: CVector,
    now_ms: u32,

    frame_counter: u32,
) -> u8 {
    let direction = gun_hit_local_direction(&player.ped, player.locomotion.rotation_cur, source);
    if weapon_type == eWeaponType::WEAPONTYPE_SHOTGUN {
        return apply_ped_gun_hit_reaction(&mut player.ped, player.locomotion.rotation_cur, &player.vitality, weapon_type, source, now_ms,
    frame_counter,
);
    }
    if player.ped.is_ped_in_control(&player.vitality, )
        && !player.ped.flags.is_ducking
        && player.state.hit_anim_delay_timer < now_ms
    {
        install_gun_hit_animation(&mut player.ped, direction, now_ms);
        player.state.hit_anim_delay_timer = now_ms.wrapping_add(
            if matches!(
                weapon_type,
                eWeaponType::WEAPONTYPE_AK47 | eWeaponType::WEAPONTYPE_M16
            ) {
                2_500
            } else {
                1_000
            },
        );
    }
    direction
}

fn register_player_shoot_ped_event(
    events: &mut CEventList,
    victim_handle: i32,
    victim: &CPed,
    shooter_handle: i32,
    now_ms: u32,
) {
    let event_type = if victim.ped_type == ePedType::PEDTYPE_COP {
        EEventType::ShootCop
    } else {
        EEventType::ShootPed
    };
    let _ = events.register_entity_event(
        event_type,
        EEventEntity::Ped,
        victim_handle,
        Some(shooter_handle),
        victim.physical.entity.get_position(),
        10_000,
        now_ms,
    );
}

fn player_weapon_fire_source(
    entity: &crate::entities::Entity::CEntity,
    clump: &crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    info: &crate::weapons::WeaponInfo::CWeaponInfo,
    file_loader: &CFileLoader,
) -> CVector {
    let model_index = if entity.model_index <= 0 {
        MI_PLAYER
    } else {
        entity.model_index as usize
    };
    crate::renderer::BevyRender::ped_frame_world_point_from_clump(
        model_index,
        file_loader,
        clump,
        &entity.matrix,
        "Rhand",
        info.fire_offset,
    )
    .unwrap_or_else(|| {
        // TODO(parity): remove this asset-missing fallback once every gameplay
        // ped clump retains the RenderWare frame hierarchy used by
        // CPed::TransformToNode.
        entity.transform_point(info.fire_offset)
    })
}

pub(crate) fn add_weapon_gunshell(
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    source: CVector,
    ahead: CVector,
    backwards: f32,
    size: f32,
) {
    let shell_pos = CVector::new(
        source.x - ahead.x * backwards,
        source.y - ahead.y * backwards,
        source.z,
    );
    // re3: CrossProduct(ahead, (0,0,5)), Normalise2D, then * 0.05.
    let length = (ahead.x * ahead.x + ahead.y * ahead.y).sqrt().max(1.0e-6);
    let shell_dir = CVector::new(
        ahead.y / length * 0.05,
        -ahead.x / length * 0.05,
        general.get_random_number_in_range_f32(0.02, 0.08),
    );
    particles.add_particle(
        crate::renderer::ParticleType::PARTICLE_GUNSHELL,
        shell_pos,
        shell_dir,
        None,
        size,
        general.get_random_number_in_range_i32(-20, 20),
        0,
        0,
        0,
    );
}

fn add_weapon_muzzle_light(
    point_lights: &mut Option<&mut CPointLights>,
    source: CVector,
    camera_position: Option<CVector>,
) {
    let (Some(point_lights), Some(camera_position)) =
        (point_lights.as_deref_mut(), camera_position)
    else {
        return;
    };
    point_lights.add_light(
        LIGHT_POINT,
        source,
        CVector::ZERO,
        5.0,
        1.0,
        0.8,
        0.0,
        FOG_NONE,
        false,
        camera_position,
    );
}

#[allow(clippy::too_many_arguments)]
fn fire_player_shotgun_pellets(
    shooter_handle: i32,
    shooter_audio_entity_id: i32,
    mut other_player: Option<&mut PlayerPedMut<'_>>,
    source: CVector,
    ray_source: CVector,
    direction: CVector,
    using_mouse_cam: bool,
    range: f32,
    damage: u32,
    now_ms: u32,
    ped_stats: Option<&CPedStatsRuntime>,
    file_loader: &CFileLoader,
    script_entities: &mut ScriptEntityWorld,
    particles: &mut ParticleEmitter<'_>,
    mut special_fx: Option<&mut CSpecialFX>,
    general: &mut CGeneral,
    mut darkel: Option<&mut CDarkel>,
    mut stats: Option<&mut CStats>,
    events: &mut CEventList,
    replay_playing_back: bool,
    mut audio: Option<&mut cDMAudio>,

    frame_counter: u32,
) {
    // CWeapon::FireShotgun uses unnormalised linear lateral offsets for PC
    // mouse aiming, and 7.5-degree heading steps for controller aiming.
    for pellet in 0..5 {
        let offset = (-15.0 + pellet as f32 * 7.5).to_radians();
        let pellet_dir = if using_mouse_cam {
            let lateral = (pellet as f32 - 2.0) * (7.5_f32.to_radians() / 2.0);
            let horizontal_length = direction.x.hypot(direction.y).max(1.0e-6);
            CVector::new(direction.x + direction.y / horizontal_length * lateral,
                direction.y - direction.x / horizontal_length * lateral, direction.z)
        } else { rotate_weapon_ray(direction, offset) };
        let mut trace_end = CVector::new(
            source.x + pellet_dir.x * range.min(30.0),
            source.y + pellet_dir.y * range.min(30.0),
            source.z + pellet_dir.z * range.min(30.0),
        );

        match closest_weapon_ray_hit(
            ray_source,
            pellet_dir,
            range,
            other_player.as_ref().map(|player| player.as_ref()),
            None,
            None,
            script_entities,
            file_loader,
        ) {
            Some(WeaponRayHit::Player { point, .. }) => {
                trace_end = point;
                if let Some(other) = other_player.as_deref_mut() {
                    let victim_position = other.position();
                    let can_be_damaged = other.can_be_damaged;
                    let direction = apply_player_gun_hit_reaction(
                        other,
                        eWeaponType::WEAPONTYPE_SHOTGUN,
                        source,
                        now_ms,
                    
    frame_counter,
);
                    let defend_weakness = ped_defend_weakness(&other.ped, ped_stats);
                    let _ = other.ped.inflict_weapon_damage(other.vitality, 
                        eWeaponType::WEAPONTYPE_SHOTGUN,
                        damage as f32,
                        defend_weakness,
                        true,
                        can_be_damaged,
                        now_ms,
                        direction,
                    );
                    emit_weapon_blood(particles, point, source, victim_position, 32);
                    register_player_shoot_ped_event(
                        events,
                        other.pool_handle,
                        &other.ped,
                        shooter_handle,
                        now_ms,
                    );
                    if let Some(audio) = audio.as_deref_mut() {
                        audio.PlayOneShot(
                            shooter_audio_entity_id,
                            SOUND_WEAPON_HIT_PED as u16,
                            1.0,
                        );
                    }
                }
            }
            Some(WeaponRayHit::Ped { handle, point, .. }) => {
                trace_end = point;
                if let Some(victim) = script_entities.peds.get_mut(&script_entities.index, &handle) {
                    let victim_position = victim.ped.physical.entity.get_position();
                    let direction = apply_ped_gun_hit_reaction(
                        &mut *victim.ped, victim.locomotion.rotation_cur, &victim.vitality,
                        eWeaponType::WEAPONTYPE_SHOTGUN,
                        source,
                        now_ms,
                    
    frame_counter,
);
                    let defend_weakness = ped_defend_weakness(&victim.ped, ped_stats);
                    let result = victim.ped.inflict_weapon_damage(victim.vitality, 
                        eWeaponType::WEAPONTYPE_SHOTGUN,
                        damage as f32,
                        defend_weakness,
                        true,
                        true,
                        now_ms,
                        direction,
                    );
                    register_player_shoot_ped_event(
                        events,
                        handle,
                        &victim.ped,
                        shooter_handle,
                        now_ms,
                    );
                    if result == PedWeaponDamageResult::Killed
                        && let (Some(darkel), Some(stats)) =
                            (darkel.as_deref_mut(), stats.as_deref_mut())
                    {
                        darkel.register_kill_by_player(
                            victim.model_id,
                            victim.ped.ped_type as usize,
                            victim.ped.flags.chris_criminal,
                            eWeaponType::WEAPONTYPE_SHOTGUN,
                            false,
                            replay_playing_back,
                            stats,
                            audio.as_deref_mut(),
                        );
                    }
                    emit_weapon_blood(particles, point, source, victim_position, 32);
                    if let Some(audio) = audio.as_deref_mut() {
                        audio.PlayOneShot(
                            shooter_audio_entity_id,
                            SOUND_WEAPON_HIT_PED as u16,
                            1.0,
                        );
                    }
                }
            }
            Some(WeaponRayHit::Vehicle {
                handle,
                point,
                normal,
                ..
            }) => {
                trace_end = point;
                if let Some(mut vehicle) = script_entities.vehicles.get_mut(&script_entities.index, &handle) {
                    let mut base = vehicle.base_mut();
                    base.inflict_weapon_damage(eWeaponType::WEAPONTYPE_SHOTGUN, damage as f32);
                }
                emit_bullet_surface_impact(particles, point, normal, 16, general);
                if let Some(audio) = audio.as_deref_mut() {
                    audio.PlayOneShot(
                        shooter_audio_entity_id,
                        SOUND_WEAPON_HIT_VEHICLE as u16,
                        1.0,
                    );
                }
            }
            Some(WeaponRayHit::Object {
                handle,
                point,
                normal,
                ..
            }) => {
                trace_end = point;
                emit_bullet_surface_impact(particles, point, normal, 16, general);
                if let Some(object) = script_entities.objects.get_mut(&script_entities.index, &handle) {
                    apply_weapon_force_to_object(object.object, normal, 5.0);
                }
            }
            Some(WeaponRayHit::Static { point, normal, .. }) => {
                trace_end = point;
                emit_bullet_surface_impact(particles, point, normal, 16, general);
            }
            None => {}
        }

        if let Some(special_fx) = special_fx.as_deref_mut() {
            special_fx.add_native_bullet_trace(source, trace_end, general);
        }
    }
}

pub(crate) fn emit_weapon_blood(
    particles: &mut ParticleEmitter<'_>,
    hit: CVector,
    source: CVector,
    victim_position: CVector,
    amount: usize,
) {
    let mut dir = CVector::new(
        (hit.x - victim_position.x) * 0.01,
        (hit.y - victim_position.y) * 0.01,
        0.01,
    );
    if dir.x == 0.0 && dir.y == 0.0 {
        dir.x = (hit.x - source.x) * 0.01;
        dir.y = (hit.y - source.y) * 0.01;
    }
    for _ in 0..amount {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_BLOOD_SMALL,
            hit,
            dir,
            None,
            0.0,
            0,
            0,
            0,
            0,
        );
    }
}

pub(crate) fn emit_bullet_surface_impact(
    particles: &mut ParticleEmitter<'_>,
    hit: CVector,
    normal: CVector,
    spark_count: usize,
    general: &mut CGeneral,
) {
    emit_bullet_sparks(particles, hit, normal, spark_count);
    let smoke = CVector::new(
        hit.x + general.get_random_number_in_range_f32(-0.2, 0.2),
        hit.y + general.get_random_number_in_range_f32(-0.2, 0.2),
        hit.z + general.get_random_number_in_range_f32(-0.2, 0.2),
    );
    particles.add_particle(
        crate::renderer::ParticleType::PARTICLE_BULLETHIT_SMOKE,
        smoke,
        CVector::ZERO,
        None,
        0.0,
        0,
        0,
        0,
        0,
    );
}

pub(crate) fn emit_bullet_sparks(
    particles: &mut ParticleEmitter<'_>,
    hit: CVector,
    normal: CVector,
    spark_count: usize,
) {
    let spark_velocity = CVector::new(normal.x * 0.05, normal.y * 0.05, normal.z * 0.05);
    for _ in 0..spark_count {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_SPARK,
            hit,
            spark_velocity,
            None,
            0.0,
            1,
            0,
            0,
            0,
        );
    }
}

pub(crate) fn apply_weapon_force_to_object(
    object: &mut crate::objects::Object::CObject,
    normal: CVector,
    force: f32,
) {
    if object.physical.flags.infinite_mass {
        return;
    }
    if object.physical.entity.flags.is_static && object.uproot_limit <= 0.0 {
        object.physical.entity.flags.is_static = false;
    }
    if !object.physical.entity.flags.is_static {
        object.physical.apply_move_force(CVector::new(
            -normal.x * force,
            -normal.y * force,
            -normal.z * force,
        ));
    }
}

pub(crate) fn emit_instant_hit_muzzle_fx(
    weapon_type: eWeaponType,
    source: CVector,
    direction: CVector,
    ped_velocity: CVector,
    emit_automatic_flash: bool,
    particles: &mut ParticleEmitter<'_>,
    general: &mut CGeneral,
    mut point_lights: Option<&mut CPointLights>,
    camera_position: Option<CVector>,
) {
    let planar_len = (direction.x * direction.x + direction.y * direction.y)
        .sqrt()
        .max(1.0e-6);
    let ahead = CVector::new(direction.x / planar_len, direction.y / planar_len, 0.0);
    let shooter_moving = ped_velocity.x.abs() > 0.0 && ped_velocity.y.abs() > 0.0;
    let add = |particles: &mut ParticleEmitter<'_>, kind, position, velocity, size| {
        particles.add_particle(kind, position, velocity, None, size, 0, 0, 0, 0);
    };
    let mut flash = source;

    match weapon_type {
        eWeaponType::WEAPONTYPE_COLT45 => {
            add_weapon_muzzle_light(&mut point_lights, source, camera_position);
            if shooter_moving {
                flash.x += 1.5 * ped_velocity.x;
                flash.y += 1.5 * ped_velocity.y;
            }
            for (advance, size) in [(0.0, 0.06), (0.06, 0.04), (0.04, 0.02)] {
                flash.x += ahead.x * advance;
                flash.y += ahead.y * advance;
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    flash,
                    CVector::ZERO,
                    size,
                );
            }
            for (speed, size) in [(0.10, 0.005), (0.15, 0.015), (0.20, 0.025)] {
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNSMOKE2,
                    source,
                    CVector::new(ahead.x * speed, ahead.y * speed, 0.0),
                    size,
                );
            }
            add_weapon_gunshell(particles, general, source, ahead, 0.2, 0.015);
        }
        eWeaponType::WEAPONTYPE_UZI => {
            add_weapon_muzzle_light(&mut point_lights, source, camera_position);
            if shooter_moving {
                flash.x += 1.5 * ped_velocity.x;
                flash.y += 1.5 * ped_velocity.y;
            }
            for (advance, size) in [
                (0.0, 0.07),
                (0.06, 0.05),
                (0.04, 0.04),
                (0.04, 0.03),
                (0.03, 0.03),
                (0.03, 0.02),
                (0.02, 0.01),
            ] {
                flash.x += ahead.x * advance;
                flash.y += ahead.y * advance;
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    flash,
                    CVector::ZERO,
                    size,
                );
            }
            let smoke_speed = general.get_random_number_in_range_f32(0.05, 0.25);
            add(
                particles,
                crate::renderer::ParticleType::PARTICLE_GUNSMOKE2,
                source,
                CVector::new(ahead.x * smoke_speed, ahead.y * smoke_speed, 0.0),
                0.0,
            );
            add_weapon_gunshell(particles, general, source, ahead, 0.2, 0.015);
        }
        eWeaponType::WEAPONTYPE_AK47 => {
            if !emit_automatic_flash {
                return;
            }
            add_weapon_muzzle_light(&mut point_lights, source, camera_position);
            for (advance, size) in [(0.06, 0.10), (0.06, 0.08), (0.05, 0.06), (0.04, 0.04)] {
                flash.x += ahead.x * advance;
                flash.y += ahead.y * advance;
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    flash,
                    CVector::ZERO,
                    size,
                );
            }
            let smoke_speed = general.get_random_number_in_range_f32(0.05, 0.25);
            add(
                particles,
                crate::renderer::ParticleType::PARTICLE_GUNSMOKE2,
                source,
                CVector::new(ahead.x * smoke_speed, ahead.y * smoke_speed, 0.0),
                0.0,
            );
            add_weapon_gunshell(particles, general, source, ahead, 0.5, 0.018);
        }
        eWeaponType::WEAPONTYPE_M16 => {
            if !emit_automatic_flash {
                return;
            }
            add_weapon_muzzle_light(&mut point_lights, source, camera_position);
            for (advance, size) in [(0.0, 0.08), (0.06, 0.06), (0.06, 0.06)] {
                flash.x += ahead.x * advance;
                flash.y += ahead.y * advance;
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    flash,
                    CVector::ZERO,
                    size,
                );
            }
            let rear = CVector::new(source.x - 0.1 * ahead.x, source.y - 0.1 * ahead.y, source.z);
            for (z, size) in [(0.04, 0.04), (0.08, 0.02), (0.11, 0.02)] {
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    CVector::new(rear.x, rear.y, rear.z + z),
                    CVector::ZERO,
                    size,
                );
                add(
                    particles,
                    crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                    CVector::new(rear.x, rear.y, rear.z - z),
                    CVector::ZERO,
                    size,
                );
            }
            let side = CVector::new(ahead.y, -ahead.x, 0.0);
            for side_sign in [1.0, -1.0] {
                let mut lateral = rear;
                for (advance, size) in [(0.06, 0.04), (0.04, 0.03), (0.03, 0.02)] {
                    lateral.x += side.x * advance * side_sign;
                    lateral.y += side.y * advance * side_sign;
                    add(
                        particles,
                        crate::renderer::ParticleType::PARTICLE_GUNFLASH_NOANIM,
                        lateral,
                        CVector::ZERO,
                        size,
                    );
                }
            }
            let smoke_speed = general.get_random_number_in_range_f32(0.05, 0.25);
            add(
                particles,
                crate::renderer::ParticleType::PARTICLE_GUNSMOKE2,
                source,
                CVector::new(ahead.x * smoke_speed, ahead.y * smoke_speed, 0.0),
                0.0,
            );
            add_weapon_gunshell(particles, general, source, ahead, 0.65, 0.02);
        }
        _ => {}
    }
}

pub(crate) fn emit_shotgun_muzzle_fx(
    source: CVector,
    direction: CVector,
    particles: &mut ParticleEmitter<'_>,
    mut point_lights: Option<&mut CPointLights>,
    camera_position: Option<CVector>,
) {
    add_weapon_muzzle_light(&mut point_lights, source, camera_position);
    let planar_len = (direction.x * direction.x + direction.y * direction.y)
        .sqrt()
        .max(1.0e-6);
    let ahead = CVector::new(direction.x / planar_len, direction.y / planar_len, 0.0);
    let mut flash = source;
    for (advance, size) in [(0.1, 0.0), (0.1, 0.15), (0.1, 0.2)] {
        flash.x += ahead.x * advance;
        flash.y += ahead.y * advance;
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_GUNFLASH,
            flash,
            CVector::ZERO,
            None,
            size,
            0,
            0,
            0,
            0,
        );
    }
    particles.add_particle(
        crate::renderer::ParticleType::PARTICLE_GUNFLASH,
        source,
        CVector::ZERO,
        None,
        0.0,
        0,
        0,
        0,
        0,
    );
    for speed in [0.10, 0.15, 0.20, 0.25] {
        particles.add_particle(
            crate::renderer::ParticleType::PARTICLE_GUNSMOKE2,
            source,
            CVector::new(ahead.x * speed, ahead.y * speed, 0.0),
            None,
            0.1,
            0,
            0,
            0,
            0,
        );
    }
}

// Compatibility names; collision math and live-actor ray access belong to weapons.
pub(crate) use crate::weapons::WeaponRaycast::{
    WeaponRayHit, closest_weapon_ray_hit, weapon_col_model_hit,
};

pub(crate) fn rotate_weapon_ray(direction: CVector, radians: f32) -> CVector {
    let (sin, cos) = radians.sin_cos();
    CVector::new(
        direction.x * cos - direction.y * sin,
        direction.x * sin + direction.y * cos,
        direction.z,
    )
}

/// Finite building/vehicle/object line used by re3 `CPed::InTheAir`.
/// Unlike `FindGroundZFor3DCoord`, this must not see surfaces farther than
/// 4.04 units below the ped root: that exact miss is what starts ANIM_STD_FALL.
pub(crate) fn player_air_ground_below(
    root: CVector,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
) -> Option<f32> {
    let static_ground = file_loader
        .world
        .process_line_of_sight_static_contact(
            [root.x, root.y, root.z],
            [root.x, root.y, root.z - 4.04],
            &file_loader.model_info,
            None,
            false,
        )
        .map(|contact| contact.point[2]);
    let dynamic_ground = player_dynamic_ground_below(
        root.x,
        root.y,
        root.z,
        entities,
        object_colliders,
        file_loader,
    )
    .map(|support| support.z);
    match (static_ground, dynamic_ground) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

pub fn apply_player_mouse_movement_direction(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &CPad,
    moved: CVector2D,
) -> CVector2D {
    let left_right = pad.get_ped_walk_left_right() as f32;
    let up_down = pad.get_ped_walk_up_down() as f32;
    if up_down != 0.0 {
        player.state.walk_angle =
            CGeneral::get_radian_angle_between_points(0.0, 0.0, -left_right, up_down);
    } else if left_right < 0.0 {
        player.state.walk_angle = std::f32::consts::FRAC_PI_2;
    } else if left_right > 0.0 {
        player.state.walk_angle = -std::f32::consts::FRAC_PI_2;
    }

    let walk_heading = CGeneral::limit_radian_angle(player.locomotion.rotation_cur + player.state.walk_angle);
    let ped_speed = (moved.x * moved.x + moved.y * moved.y).sqrt();
    let mut local_walk_angle = CGeneral::limit_radian_angle(walk_heading - player.locomotion.rotation_cur);
    if local_walk_angle < -std::f32::consts::FRAC_PI_2 {
        local_walk_angle += std::f32::consts::PI;
    } else if local_walk_angle > std::f32::consts::FRAC_PI_2 {
        local_walk_angle -= std::f32::consts::PI;
    }
    player.state.mouse_leg_turn_angle = Some(local_walk_angle);

    let fifty_degrees = 50.0_f32.to_radians();
    let adjusted = if local_walk_angle > -fifty_degrees && local_walk_angle < fifty_degrees {
        CVector2D::new(
            -walk_heading.sin() * ped_speed,
            walk_heading.cos() * ped_speed,
        )
    } else {
        moved
    };
    player.locomotion.moved = adjusted;
    adjusted
}

/// re3 `CPed::FinishLaunchCB` chooses the minimum jump XY direction from
/// `WorkOutHeadingForMovingFirstPerson(m_fRotationCur)` while the normal PC
/// third-person mouse camera is active. `walk_angle` is that function's cached
/// input-relative angle, maintained by `apply_player_mouse_movement_direction`.
fn player_jump_launch_heading(player: crate::peds::PlayerPed::PlayerPedRef<'_>, mouse_strafe_control: bool) -> f32 {
    if mouse_strafe_control {
        CGeneral::limit_radian_angle(player.locomotion.rotation_cur + player.state.walk_angle)
    } else {
        player.locomotion.rotation_cur
    }
}

fn player_anim_group_for_control(
    weapon_type: eWeaponType,
    walk_angle: f32,
    mouse_strafe_control: bool,
) -> usize {
    use crate::animation::AnimAssocDefinitions::{
        ASSOCGRP_PLAYERBACK, ASSOCGRP_PLAYERLEFT, ASSOCGRP_PLAYERRIGHT, ASSOCGRP_ROCKETBACK,
        ASSOCGRP_ROCKETLEFT, ASSOCGRP_ROCKETRIGHT,
    };

    let fifty_degrees = 50.0_f32.to_radians();
    if !mouse_strafe_control || walk_angle.abs() < fifty_degrees {
        return player_anim_group_for_weapon(weapon_type);
    }

    let one_thirty_degrees = 130.0_f32.to_radians();
    let rocket = weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER;
    if (-one_thirty_degrees..=one_thirty_degrees).contains(&walk_angle) {
        if walk_angle > 0.0 {
            if rocket {
                ASSOCGRP_ROCKETLEFT
            } else {
                ASSOCGRP_PLAYERLEFT
            }
        } else if rocket {
            ASSOCGRP_ROCKETRIGHT
        } else {
            ASSOCGRP_PLAYERRIGHT
        }
    } else if rocket {
        ASSOCGRP_ROCKETBACK
    } else {
        ASSOCGRP_PLAYERBACK
    }
}

pub(crate) fn player_in_the_air_should_land(root_z: f32, ground_z: f32) -> bool {
    root_z - ground_z < 1.3
}

/// `CPed::CheckIfInTheAir` + `SetInTheAir` + `InTheAir` phase. This belongs to
/// the base ped control pass, before `CPlayerPed::PlayerControlZelda` and before
/// world collision integrates the matrix. Returns the collapse/land choice if
/// SetLanding was entered so the caller can emit the matching sound.
pub(crate) fn service_player_air_state_before_collision(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    manager: &CAnimManager,
    root: CVector,
    finite_ground: Option<f32>,
    file_loader: &CFileLoader,
) -> Option<bool> {
    if !player.ped.flags.is_standing && !player.ped.flags.is_in_the_air {
        let vertical_ground = player_check_if_in_air_has_ground(root.z, finite_ground);
        let foot_sphere_ground = !vertical_ground
            && player_check_if_in_air_has_static_foot_sphere(
                player.ped.ped_state,
                root,
                file_loader,
            );
        if !(vertical_ground || foot_sphere_ground) {
            player.ped.flags.is_in_the_air = true;
            set_player_jump_animation(
                player,
                manager,
                crate::animation::AnimationId::ANIM_STD_FALL_GLIDE,
            );
        }
    }

    if !player.ped.flags.is_in_the_air || player.ped.physical.move_speed.z >= 0.0 {
        return None;
    }

    if let Some(ground_z) = finite_ground {
        if player_in_the_air_should_land(root.z, ground_z) {
            let was_falling = player.state.jump_association.as_ref().is_some_and(|association| {
                association.anim_id == crate::animation::AnimationId::ANIM_STD_FALL
            });
            player.ped.flags.is_in_the_air = false;
            player.ped.flags.is_landing = true;
            set_player_jump_animation(
                player,
                manager,
                if was_falling {
                    crate::animation::AnimationId::ANIM_STD_FALL_COLLAPSE
                } else {
                    crate::animation::AnimationId::ANIM_STD_FALL_LAND
                },
            );
            return Some(was_falling);
        }
    } else if player.ped.physical.move_speed.z < -0.1 * GTA_PHYSICS_HZ
        && player.state.jump_association.as_ref().is_none_or(|association| {
            association.anim_id != crate::animation::AnimationId::ANIM_STD_FALL
        })
    {
        set_player_jump_animation(
            player,
            manager,
            crate::animation::AnimationId::ANIM_STD_FALL,
        );
    }
    None
}

pub(crate) fn player_dynamic_support_from_air_probe(
    root_z: f32,
    ped: &CPed,
    air_probe: Option<PlayerDynamicGroundSupport>,
) -> Option<PlayerDynamicGroundSupport> {
    if ped.flags.was_standing {
        return None;
    }
    air_probe.filter(|support| {
        support.z >= root_z - crate::peds::PedPlacement::PED_FEET_OFFSET && support.z <= root_z
    })
}

/// Dynamic half of `CPed::ProcessEntityCollision`'s below-torso support line.
/// A previously-standing ped extends its feet line by
/// `0.15 * CTimer::GetTimeStep()` and starts it 0.25 below the root. This is
/// deliberately separate from the longer InTheAir ground probe: only this
/// short contact is allowed to become `m_pCurrentPhysSurface`.
pub(crate) fn player_dynamic_ground_support(
    x: f32,
    y: f32,
    root_z: f32,
    ped: &CPed,
    dt_seconds: f32,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
) -> Option<PlayerDynamicGroundSupport> {
    let extension = player_ground_support_extension(ped, dt_seconds);
    let line_start_z = root_z - if ped.flags.was_standing { 0.25 } else { 0.0 };
    let line_end_z = root_z - crate::peds::PedPlacement::PED_FEET_OFFSET - extension;
    let mut support = player_dynamic_vertical_hit(
        x,
        y,
        line_start_z,
        line_end_z,
        entities,
        object_colliders,
        file_loader,
    );

    // re3 gives a previously-standing boat one extra gravity-effect segment
    // below the normal support line when the first vertical test misses.
    if support.is_none() && ped.flags.was_standing && extension > 0.0 {
        let boat_support = entities.vehicles.iter().filter_map(|(&handle, vehicle)| {
            if !matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }) {
                return None;
            }
            let entity = entities.vehicle_entity(handle)?;
            let physical_entity = &vehicle.base().physical.entity;
            let col = file_loader
                .model_info
                .col_model(physical_entity.model_index.max(0) as usize)?;
            if !physical_entity.flags.uses_collision {
                return None;
            }
            let local_start = physical_entity
                .matrix
                .multiply_inverse_point(CVector::new(x, y, line_end_z));
            let local_end = physical_entity.matrix.multiply_inverse_point(CVector::new(
                x,
                y,
                line_end_z - extension,
            ));
            let line = CColLine::new(
                CColVector::new(local_start.x, local_start.y, local_start.z),
                CColVector::new(local_end.x, local_end.y, local_end.z),
            );
            let mut point = CColPoint::new();
            let mut distance = 1.0;
            if !col.process_line_of_sight_primitives(&line, &mut point, &mut distance) {
                return None;
            }
            let world = physical_entity.matrix.transform_point(CVector::new(
                point.point.x,
                point.point.y,
                point.point.z,
            ));
            Some(PlayerDynamicGroundSupport {
                z: world.z,
                entity,
                offset_from_surface: CVector::new(
                    world.x - physical_entity.matrix.position.x,
                    world.y - physical_entity.matrix.position.y,
                    world.z - physical_entity.matrix.position.z,
                ),
                surface: point.surface_b,
                normal: {
                    let local = CVector::new(point.normal.x, point.normal.y, point.normal.z);
                    let world = physical_entity.matrix.multiply3x3(local);
                    let length = world.magnitude();
                    if length > 1.0e-6 {
                        CVector::new(world.x / length, world.y / length, world.z / length)
                    } else {
                        CVector::ZERO
                    }
                },
            })
        });
        support = boat_support.max_by(|left, right| left.z.total_cmp(&right.z));
    }
    support
}

pub(crate) fn camera_vehicle_obscurer_max_z(
    entities: &ScriptEntityWorld,
    file_loader: &CFileLoader,
    target_handle: i32,
    target: CVector,
    source: CVector,
) -> Option<f32> {
    let mut closest_fraction = 1.0_f32;
    let mut result = None;
    for (&handle, vehicle) in entities.vehicles.iter() {
        if handle == target_handle {
            continue;
        }
        let base = vehicle.base();
        if !base.physical.entity.flags.uses_collision {
            continue;
        }
        let model_id = base.physical.entity.model_index.max(0) as usize;
        let Some(col_model) = file_loader.model_info.col_model(model_id) else {
            continue;
        };
        let matrix = &base.physical.entity.matrix;
        let local_target = matrix.multiply_inverse_point(target);
        let local_source = matrix.multiply_inverse_point(source);
        let line = CColLine::new(
            CColVector::new(local_target.x, local_target.y, local_target.z),
            CColVector::new(local_source.x, local_source.y, local_source.z),
        );
        let mut point = CColPoint::new();
        if col_model.process_line_of_sight_primitives(&line, &mut point, &mut closest_fraction) {
            result = Some(col_model.bounding_box.max.z);
        }
    }
    result
}

pub(crate) fn sphere_hits_col_model(
    center: CVector,
    radius: f32,
    matrix: &CMatrix,
    col_model: &crate::collision::ColModel::CColModel,
) -> bool {
    let sphere = CColSphere::new(radius, CColVector::new(0.0, 0.0, 0.0), 0, 0);
    let sphere_model = crate::collision::ColModel::CColModel::new(
        sphere.clone(),
        CColBox::new(
            CColVector::new(-radius, -radius, -radius),
            CColVector::new(radius, radius, radius),
            0,
            0,
        ),
        vec![sphere],
    );
    let mut sphere_matrix = CMatrix::unity();
    sphere_matrix.position = center;
    CCollision::test_col_models_spheres(&sphere_matrix, &sphere_model, matrix, col_model)
}

/// Dynamic half of re3 `CWorld::TestSphereAgainstWorld` used by
/// `CPed::FinishLaunchCB`. This deliberately tests the authored 0.25 probe
/// sphere, not the player's whole ped COL. The odd car-only bound fallback is
/// retained because it is part of the native world query.
pub(crate) fn player_jump_dynamic_sphere_blocked(
    center: CVector,
    radius: f32,
    entities: &ScriptEntityWorld,
    object_colliders: &[ScriptObjectCollisionSnapshot],
    file_loader: &CFileLoader,
) -> bool {
    for vehicle in entities.vehicles.values() {
        let base = vehicle.base();
        if !base.physical.entity.flags.uses_collision {
            continue;
        }
        let model_id = base.physical.entity.model_index.max(0) as usize;
        let Some(col_model) = file_loader.model_info.col_model(model_id) else {
            continue;
        };
        let matrix = &base.physical.entity.matrix;
        if sphere_hits_col_model(center, radius, matrix, col_model) {
            return true;
        }

        // `CWorld::TestSphereAgainstSectorList` has this extra car test after
        // ProcessColModels. Under FIX_BUGS the distance is measured from the
        // actual bound centre rather than the entity origin.
        if matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Automobile { .. }) && model_id != MI_DODO {
            let bound_center = matrix.transform_point(CVector::new(
                col_model.bounding_sphere.center.x,
                col_model.bounding_sphere.center.y,
                col_model.bounding_sphere.center.z,
            ));
            let distance = CVector::new(
                center.x - bound_center.x,
                center.y - bound_center.y,
                center.z - bound_center.z,
            )
            .magnitude();
            if radius + col_model.bounding_box.max.x > distance {
                return true;
            }
        }
    }

    object_colliders.iter().any(|object| {
        if !object.uses_collision {
            return false;
        }
        let Some(col_model) = file_loader.model_info.col_model(object.model_index) else {
            return false;
        };
        sphere_hits_col_model(center, radius, &object.matrix, col_model)
    })
}

// === FAZ4b: moved from core/Game.rs (verbatim, parity-preserving) ===
// Player jump cluster, collision-frame helpers, run-stop and movement
// associations. Shared with FAZ6 resolvers, which import them from here.

/// re3-shaped partial ProcessControl: pad → on-foot ped when script enables control.
pub(crate) fn set_player_jump_animation(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    manager: &CAnimManager,
    animation_id: crate::animation::AnimationId::AnimationId,
) {
    use crate::animation::AnimAssocDefinitions::ASSOCGRP_STD;
    // Match the call site in re3, not just the association template.
    // SetJump/FinishLaunchCB use BlendAnimation(..., 8), while the two
    // airborne fall transitions use BlendAnimation(..., 4).  Landing/glide
    // associations are AddAnimation and therefore start fully blended.
    let blend_in_delta = player_jump_animation_blend_in_delta(animation_id);
    player.state.jump_association = manager
        .create_anim_association(ASSOCGRP_STD, animation_id as usize)
        .map(|mut association| {
            association.set_current_time(0.0);
            association.set_run();
            if let Some(delta) = blend_in_delta {
                association.blend_amount = 0.0;
                association.blend_delta = delta;
            }
            association
        });
    player.state.jump_animation_time = 0.0;
}

fn player_jump_animation_blend_in_delta(
    animation_id: crate::animation::AnimationId::AnimationId,
) -> Option<f32> {
    use crate::animation::AnimationId::{
        ANIM_STD_FALL, ANIM_STD_FALL_GLIDE, ANIM_STD_HANDSCOWER, ANIM_STD_JUMP_LAUNCH,
    };
    match animation_id {
        ANIM_STD_JUMP_LAUNCH | ANIM_STD_HANDSCOWER => Some(8.0),
        ANIM_STD_FALL | ANIM_STD_FALL_GLIDE => Some(4.0),
        _ => None,
    }
}

pub fn begin_player_jump(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>, manager: &CAnimManager) {
    if player.ped.last_ped_state == PedState::PED_NONE {
        player.ped.last_ped_state = player.ped.ped_state;
    }
    player.ped.ped_state = PedState::PED_JUMP;
    player.locomotion.rotation_dest = player.locomotion.rotation_cur;
    set_player_jump_animation(
        player,
        manager,
        crate::animation::AnimationId::ANIM_STD_JUMP_LAUNCH,
    );
}

/// Advance re3's launch association and apply `CPed::FinishLaunchCB` once.
/// Returned velocity is expressed in GTA world units per second.
pub fn advance_player_jump_launch(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    manager: &CAnimManager,
    dt: f32,
) -> Option<CVector> {
    if player.ped.ped_state != PedState::PED_JUMP || player.ped.flags.is_in_the_air {
        return None;
    }
    if !update_player_jump_animation(player, dt) {
        return None;
    }

    let mut velocity = player.ped.physical.move_speed;
    let mass = player.ped.physical.mass.max(1.0);
    velocity.z = 8.5 / mass * GTA_PHYSICS_HZ;
    let sprint_blend = player
        .animation.movement_anim_clump
        .associations
        .iter()
        .find(|association| association.anim_id == crate::animation::AnimationId::ANIM_STD_RUNFAST)
        .map(|association| association.blend_amount);
    let run_blend = player
        .animation.movement_anim_clump
        .associations
        .iter()
        .find(|association| association.anim_id == crate::animation::AnimationId::ANIM_STD_RUN)
        .map(|association| association.blend_amount);
    let minimum_speed = if let Some(blend) = sprint_blend {
        (0.05 * blend + 0.17) * GTA_PHYSICS_HZ
    } else if let Some(blend) = run_blend {
        (0.07 * blend + 0.1) * GTA_PHYSICS_HZ
    } else {
        0.1 * GTA_PHYSICS_HZ
    };
    if velocity.x * velocity.x + velocity.y * velocity.y < minimum_speed * minimum_speed {
        // This standalone helper has no camera argument. A frame-local mouse
        // leg angle means the same mouse-strafe path ran this frame, so use the
        // stored WorkOutHeadingForMovingFirstPerson angle just like live re3.
        let launch_heading =
            player_jump_launch_heading(player.as_ref(), player.state.mouse_leg_turn_angle.is_some());
        let (sin, cos) = launch_heading.sin_cos();
        velocity.x = -sin * minimum_speed;
        velocity.y = cos * minimum_speed;
    }
    player.ped.flags.is_standing = false;
    player.ped.flags.is_in_the_air = true;
    player.ped.physical.move_speed = velocity;
    set_player_jump_animation(
        player,
        manager,
        crate::animation::AnimationId::ANIM_STD_JUMP_GLIDE,
    );
    Some(velocity)
}

pub fn update_player_jump_animation(player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>, dt: f32) -> bool {
    let Some(association) = player.state.jump_association.as_mut() else {
        return true;
    };
    association.update_blend(dt);
    association.update_time(dt, 1.0);
    player.state.jump_animation_time = association.current_time;
    let finished = !association.is_running();
    let finished_running_land =
        finished && association.anim_id == crate::animation::AnimationId::ANIM_STD_JUMP_LAND;
    if finished_running_land {
        // re3 CPed::FinishJumpCB: the running-landing clip resets locomotion
        // gait phase before the next SetRealMoveAnim pass.
        association.blend_delta = -1000.0;
        player.ped.flags.reset_walk_anims = true;
    }
    finished
}

/// `CPlayerPed::RunningLand`: a fast player who is still holding movement can
/// replace the just-started FALL_LAND with JUMP_LAND and restore the pre-jump
/// ped state immediately. This preserves the native run-through-landing feel
/// without zeroing the physical move vector.
pub fn apply_player_running_land(
    player: &mut crate::peds::PlayerPed::PlayerPedMut<'_>,
    pad: &crate::core::Pad::CPad,
    manager: &CAnimManager,
) {
    let running_land = player.ped.flags.is_landing
        && player.state.move_speed > 1.5
        && (pad.get_ped_walk_left_right() != 0 || pad.get_ped_walk_up_down() != 0)
        && player.state.jump_association.as_ref().is_some_and(|association| {
            association.anim_id == crate::animation::AnimationId::ANIM_STD_FALL_LAND
                && association.current_time == 0.0
        });
    if !running_land {
        return;
    }

    set_player_jump_animation(
        player,
        manager,
        crate::animation::AnimationId::ANIM_STD_JUMP_LAND,
    );
    if player.ped.ped_state == PedState::PED_JUMP {
        player.ped.ped_state = match player.ped.last_ped_state {
            PedState::PED_NONE | PedState::PED_IDLE => PedState::PED_IDLE,
            previous => previous,
        };
        player.ped.last_ped_state = PedState::PED_NONE;
    }
}

/// Player control stores linear velocity in world-units/second, while the
/// shared `CPhysical` collision implementation deliberately keeps re3's native
/// move-speed units. Convert only across that collision boundary so vehicles,
/// objects and peds all see the same units inside `ApplyCollision*`.
pub(crate) fn with_player_native_collision_speed<R>(
    ped: &mut CPed,
    f: impl FnOnce(&mut CPed) -> R,
) -> R {
    ped.physical.move_speed.x /= GTA_PHYSICS_HZ;
    ped.physical.move_speed.y /= GTA_PHYSICS_HZ;
    ped.physical.move_speed.z /= GTA_PHYSICS_HZ;
    let result = f(ped);
    ped.physical.move_speed.x *= GTA_PHYSICS_HZ;
    ped.physical.move_speed.y *= GTA_PHYSICS_HZ;
    ped.physical.move_speed.z *= GTA_PHYSICS_HZ;
    result
}

// FAZ4: moved to peds/PlayerPed.rs verbatim (18 items): struct PlayerControlFrame, fn prepare_player_physical_for_control, fn update_player_movement_animation, fn update_player_on_foot_control, fn camera_vehicle_obscurer_max_z, fn update_player_wanted, fn process_player_weapon_switch, fn process_player_weapon, fn update_player_current_phys_surface, fn player_dynamic_vertical_hit, fn player_dynamic_vertical_hit_entity, fn player_subdivided_dynamic_landing_support, fn player_static_ground_support, fn player_dynamic_ground_below, fn player_dynamic_support_from_air_probe, fn player_dynamic_ground_support, fn sphere_hits_col_model, fn player_jump_dynamic_sphere_blocked

/// Start the player-owned equivalent of re3 `CPhysical::ProcessCollision`.
/// `PlayerControlZelda` consumes the previous pass' distance/steep-slope state
/// before this point, so these values must not be cleared in
/// `prepare_player_physical_for_control`.
pub fn begin_player_collision_frame(ped: &mut CPed) {
    ped.physical.distance_travelled = 0.0;
    ped.flags.hit_steep_slope = false;
    // Native's first ProcessEntityCollision transfers bIsStanding into
    // bWasStanding and clears bIsStanding. The short feet line must establish
    // standing again each collision pass; stale standing otherwise lets
    // adjacent ground triangles repeatedly correct an already-supported ped.
    ped.flags.was_standing = ped.flags.is_standing;
    ped.flags.is_standing = false;
}

/// re3 leaves `m_fDistanceTravelled` at zero when `CheckCollision()` causes
/// the integrated matrix to be rejected. Only a collision-free physical move
/// publishes the actual displacement for the next `PlayerControlZelda` pass.
pub fn finish_player_collision_frame(
    ped: &mut CPed,
    old_position: CVector,
    new_position: CVector,
    collision_rejected_move: bool,
) {
    ped.physical.distance_travelled = if collision_rejected_move {
        0.0
    } else {
        CVector::new(
            new_position.x - old_position.x,
            new_position.y - old_position.y,
            new_position.z - old_position.z,
        )
        .magnitude()
    };
}

pub fn apply_player_ped_air_resistance(ped: &mut CPed, dt_seconds: f32) {
    if ped.flags.is_standing
        || ped.physical.move_speed.z <= 0.25 * GTA_PHYSICS_HZ
        || dt_seconds <= 0.0
    {
        return;
    }
    let factor = 0.95_f32.powf(dt_seconds * GTA_PHYSICS_HZ);
    ped.physical.move_speed.x *= factor;
    ped.physical.move_speed.y *= factor;
    ped.physical.move_speed.z *= factor;
}

/// `CPed::ProcessEntityCollision` extends a previously-standing ped's feet
/// support line by `-0.15 * CTimer::GetTimeStep()`. Airborne peds get no
/// tolerance. Keeping that distinction avoids both tiny ledge dropouts and
/// the controller-specific fixed hover margin that used to be applied in air.
pub(crate) fn player_ground_support_extension(ped: &CPed, dt_seconds: f32) -> f32 {
    if ped.flags.was_standing {
        0.15 * dt_seconds.max(0.0) * GTA_PHYSICS_HZ
    } else {
        0.0
    }
}

pub(crate) fn player_landing_touching_ground(
    collision_established_standing: bool,
    root_z: f32,
    support_ground: Option<f32>,
) -> bool {
    collision_established_standing
        || support_ground
            .is_some_and(|z| root_z <= z + crate::peds::PedPlacement::PED_FEET_OFFSET + 0.08)
}

pub(crate) fn player_grounded_after_collision_or_support(
    collision_established_standing: bool,
    root_z: f32,
    support_ground: Option<f32>,
    support_extension: f32,
    vertical_speed: f32,
) -> bool {
    collision_established_standing
        || support_ground.is_some_and(|z| {
            root_z <= z + crate::peds::PedPlacement::PED_FEET_OFFSET + support_extension
                && vertical_speed <= 0.0
        })
}

/// Vertical-line half of re3 `CPed::CheckIfInTheAir`: a ped does not enter the
/// airborne animation/state while world collision is still found within 1.54
/// units below its root. This is intentionally wider than the feet support
/// line; the ped can be non-standing and physically falling for a short gap
/// without yet being considered fully in the air.
pub(crate) fn player_check_if_in_air_has_ground(root_z: f32, ground_z: Option<f32>) -> bool {
    ground_z.is_some_and(|ground_z| ground_z <= root_z && root_z - ground_z <= 1.54)
}

/// Native `CPed::CheckIfInTheAir` fallback after the 1.54 vertical line
/// misses: outside PED_JUMP it tests a 0.15 sphere centred at FEET_OFFSET
/// below the root, against buildings only.
pub(crate) fn player_check_if_in_air_has_static_foot_sphere(
    ped_state: PedState,
    root: CVector,
    file_loader: &CFileLoader,
) -> bool {
    if ped_state == PedState::PED_JUMP {
        return false;
    }
    file_loader
        .world
        .sphere_contact_buildings_only(
            [
                root.x,
                root.y,
                root.z - crate::peds::PedPlacement::PED_FEET_OFFSET,
            ],
            0.15,
            &file_loader.model_info,
        )
        .is_some()
}

pub(crate) use super::PedLocomotion::PedPhysicalSurfaceMotion as PlayerPhysicalSurfaceMotion;

/// Resolve re3 `CPed::m_pCurrentPhysSurface` against the authoritative Bevy
/// entity relationship. Vehicle/object `CPhysical` values stay in native GTA
/// 50 Hz move-speed units; conversion happens only when they are combined with
/// the seconds-based local-player controller below.
pub(crate) fn player_current_surface_motion(
    ped: &CPed,
    entities: &ScriptEntityWorld,
) -> Option<PlayerPhysicalSurfaceMotion> {
    let surface = ped.current_phys_surface?;
    if let Some(handle) = entities.vehicle_handle_for_entity(surface)
        && let Some(vehicle) = entities.vehicle_get(handle)
    {
        let physical = &vehicle.base().physical;
        return Some(PlayerPhysicalSurfaceMotion {
            speed_native: physical.get_speed_at_point(ped.offset_from_phys_surface),
            turn_speed_z_native: physical.turn_speed.z,
            is_vehicle: true,
            is_boat: matches!(vehicle, crate::control::ScriptEntities::ScriptVehicleRef::Boat { .. }),
        });
    }
    let object = entities.object_get_entity(surface)?;
    let physical = &object.object.physical;
    Some(PlayerPhysicalSurfaceMotion {
        speed_native: physical.get_speed_at_point(ped.offset_from_phys_surface),
        turn_speed_z_native: physical.turn_speed.z,
        is_vehicle: false,
        is_boat: false,
    })
}

pub(crate) use super::PedLocomotion::apply_standing_update_position_seconds as apply_player_standing_update_position;

/// `CPed::SetJump` rejects a jump aimed into a steep-cliff surface. The
/// forward vector is the same GTA heading basis used by the player controller.
pub(crate) fn player_can_start_jump_from_surface(ped: &CPed, locomotion: &PedLocomotion) -> bool {
    if ped.physical.surface_touched
        != crate::core::SurfaceTable::eSurfaceType::SURFACE_STEEP_CLIFF as u8
    {
        return true;
    }
    let forward = CVector::new(-locomotion.rotation_cur.sin(), locomotion.rotation_cur.cos(), 0.0);
    forward.x * ped.physical.damage.normal.x
        + forward.y * ped.physical.damage.normal.y
        + forward.z * ped.physical.damage.normal.z
        >= 0.0
}

pub(crate) fn add_player_movement_association(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    group: usize,
    animation_id: crate::animation::AnimationId::AnimationId,
) {
    use crate::animation::AnimBlendAssociation::ASSOC_MOVEMENT;
    if clump
        .associations
        .iter()
        .any(|association| association.anim_id == animation_id)
    {
        return;
    }
    if let Some(mut association) = manager.create_anim_association(group, animation_id as usize) {
        // CAnimManager::AddAnimation synchronises movement clips with the
        // first movement association already present on the clump.
        if association.flags & ASSOC_MOVEMENT != 0 {
            if let Some(sync) = clump
                .associations
                .iter()
                .find(|existing| existing.flags & ASSOC_MOVEMENT != 0)
            {
                if sync.hierarchy.total_length > 0.0 && association.hierarchy.total_length > 0.0 {
                    association.set_current_time(
                        sync.current_time / sync.hierarchy.total_length
                            * association.hierarchy.total_length,
                    );
                }
            }
        } else {
            association.set_current_time(0.0);
        }
        association.set_run();
        // Native CAnimManager::AddAnimation prepends after synchronising to
        // the first movement association already present on the clump.
        clump.associations.insert(0, association);
    }
}

fn ensure_player_move_association_zero_blend(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    group: usize,
    animation_id: crate::animation::AnimationId::AnimationId,
) {
    let existed = clump
        .associations
        .iter()
        .any(|association| association.anim_id == animation_id);
    add_player_movement_association(clump, manager, group, animation_id);
    if !existed {
        if let Some(association) = clump
            .associations
            .iter_mut()
            .find(|association| association.anim_id == animation_id)
        {
            // CPlayerPed::SetRealMoveAnim explicitly zeros freshly-created
            // WALK/RUN associations immediately after AddAnimation.
            association.blend_amount = 0.0;
        }
    }
}

pub(crate) fn add_player_standard_association(
    clump: &mut crate::animation::AnimBlendClumpData::CAnimBlendClumpData,
    manager: &CAnimManager,
    animation_id: crate::animation::AnimationId::AnimationId,
) {
    use crate::animation::AnimAssocDefinitions::ASSOCGRP_STD;
    if clump
        .associations
        .iter()
        .any(|association| association.anim_id == animation_id)
    {
        return;
    }
    if let Some(mut association) =
        manager.create_anim_association(ASSOCGRP_STD, animation_id as usize)
    {
        association.set_current_time(0.0);
        association.set_run();
        // Native CAnimManager::AddAnimation prepends non-movement clips too.
        clump.associations.insert(0, association);
    }
}

pub(crate) fn should_start_player_run_stop(
    move_state: eMoveState,
    move_speed: f32,
    sprint_blend_amount: f32,
    sprint_blend_delta: f32,
) -> bool {
    (move_state != eMoveState::PEDMOVE_SPRINT || move_speed < 0.4)
        && sprint_blend_amount != 0.0
        && (sprint_blend_delta >= 0.0 || sprint_blend_amount >= 0.8)
        && move_speed < 0.4
}

#[cfg(test)]
mod tests {
    #[test]
    fn drowning_respects_cutscene_protection_native_scale_and_armour_exemption() {
        use super::*;
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.vitality.armour = 50.0;
        player.ped.physical.entity.flags.only_damaged_by_player = true;
        player.can_be_damaged = false;
        let health = player.vitality.health;
        assert_eq!(apply_player_drowning_damage(&mut player.as_mut(), 3.0, 100),
            PedWeaponDamageResult::Ignored);
        assert_eq!(player.vitality.health, health);
        player.can_be_damaged = true;
        assert_eq!(apply_player_drowning_damage(&mut player.as_mut(), 3.0, 101),
            PedWeaponDamageResult::Applied);
        assert!((player.vitality.health - (health - 0.99)).abs() < 0.0001);
        assert_eq!(player.vitality.armour, 50.0);
        assert_eq!(player.vitality.last_wep_dam, eWeaponType::WEAPONTYPE_DROWNING as i8);
        player.vitality.health = 1.0;
        assert_eq!(apply_player_drowning_damage(&mut player.as_mut(), 3.0, 102),
            PedWeaponDamageResult::Killed);
        assert_eq!(player.ped.impact_animation_id, Some(crate::animation::AnimationId::ANIM_STD_DROWN));
    }

    use super::*;
    use crate::animation::{
        AnimBlendAssocGroup::CAnimBlendAssocGroup,
        AnimBlendAssociation::{
            ASSOC_HAS_TRANSLATION, ASSOC_IDLE, ASSOC_MOVEMENT, ASSOC_PARTIAL, ASSOC_REPEAT,
        },
        AnimBlendClumpData::{CAnimBlendClumpData, VELOCITY_EXTRACTION},
        AnimBlendHierarchy::CAnimBlendHierarchy,
        AnimBlendSequence::{AnimKeyFrame, CAnimBlendSequence, KeyFrameTrans},
        FrameUpdate::frame_update_callback_with_velocity_extraction_overlay_non_skinned,
    };
    use crate::math::Vector::CVector as AnimVector;
    use crate::peds::Ped::give_ped_weapon;

    #[test]
    fn player_fight_prefers_live_ground_target_over_dead_target() {
        let attacker = CPlayerPed::new(&mut CGeneral::default(), None);
        let targets = [
            FightTargetSample {
                position: CVector::new(0.25, 0.7, 0.0),
                state: PedState::PED_DEAD,
                is_player: false,
            },
            FightTargetSample {
                position: CVector::new(0.0, 1.5, 0.0),
                state: PedState::PED_FALL,
                is_player: false,
            },
        ];

        let (move_id, heading) = initial_player_fight_move(&attacker.ped, attacker.locomotion.rotation_cur, &targets);

        assert_eq!(move_id, PedFightMoves::FIGHTMOVE_GROUNDKICK);
        assert!(heading.is_some_and(|heading| heading.abs() < 1.0e-4));
    }

    #[test]
    fn player_fight_uses_re3_close_range_combo_thresholds() {
        let attacker = CPlayerPed::new(&mut CGeneral::default(), None);
        let close = [FightTargetSample {
            position: CVector::new(0.0, 0.6, 0.0),
            state: PedState::PED_IDLE,
            is_player: false,
        }];
        let jab_range = [FightTargetSample {
            position: CVector::new(0.0, 1.0, 0.0),
            state: PedState::PED_IDLE,
            is_player: false,
        }];
        let down = [FightTargetSample {
            position: CVector::new(0.0, 1.0, 0.0),
            state: PedState::PED_FALL,
            is_player: false,
        }];

        assert_eq!(
            next_player_fight_move(&attacker.ped, attacker.locomotion.rotation_cur, 255, &close).0,
            PedFightMoves::FIGHTMOVE_HEADBUTT
        );
        assert_eq!(
            next_player_fight_move(&attacker.ped, attacker.locomotion.rotation_cur, 100, &jab_range).0,
            PedFightMoves::FIGHTMOVE_PUNCHJAB
        );
        assert_eq!(
            next_player_fight_move(&attacker.ped, attacker.locomotion.rotation_cur, 100, &down).0,
            PedFightMoves::FIGHTMOVE_GROUNDKICK
        );
    }

    #[test]
    fn dead_player_or_wrecked_car_cannot_reenter_driver_control() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        let relation = InVehicle::driver(
            Entity::from_raw_u32(7).expect("test vehicle entity must be representable"),
        );

        player.ped.ped_state = PedState::PED_DRIVING;
        assert!(player_can_drive_related_vehicle(player.as_ref(),
            relation,
            eEntityStatus::STATUS_PLAYER,
        ));

        player.ped.ped_state = PedState::PED_DEAD;
        assert!(!player_can_drive_related_vehicle(player.as_ref(),
            relation,
            eEntityStatus::STATUS_PLAYER,
        ));

        player.ped.ped_state = PedState::PED_DRIVING;
        assert!(!player_can_drive_related_vehicle(player.as_ref(),
            relation,
            eEntityStatus::STATUS_WRECKED,
        ));
    }

    #[test]
    fn throwable_attack_charge_uses_native_timestep_and_survives_release() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_GRENADE;
        player.state.selected_wep_slot = eWeaponType::WEAPONTYPE_GRENADE;
        player.ped.ped_state = PedState::PED_ATTACK;

        let mut pad = CPad::default();
        pad.old_state.circle = 255;
        pad.new_state.circle = 255;
        update_player_weapon_attack_input(&mut player.as_mut(), &pad, 1.0);
        assert_eq!(player.state.attack_button_counter, 1.0);
        assert!(!player.state.have_target_selected);

        pad.old_state.circle = 255;
        pad.new_state.circle = 0;
        update_player_weapon_attack_input(&mut player.as_mut(), &pad, 1.0);
        assert_eq!(player.state.attack_button_counter, 1.0);
        assert!(player.state.have_target_selected);
    }

    fn translated_test_association(
        end_y: f32,
        flags: i32,
        blend_amount: f32,
    ) -> CAnimBlendAssociation {
        let sequence = translated_test_sequence("root", end_y);
        let hierarchy = CAnimBlendHierarchy {
            sequences: vec![sequence],
            total_length: 1.0,
            ..Default::default()
        };
        let mut association = CAnimBlendAssociation::new();
        association.init_with_hierarchy(hierarchy, 1);
        association.flags = flags;
        association.blend_amount = blend_amount;
        association.start(0.0);
        association.time_step = 0.5;
        association
    }

    fn translated_test_sequence(name: &str, end_y: f32) -> CAnimBlendSequence {
        let mut sequence = CAnimBlendSequence::new();
        sequence.set_name(name);
        sequence.set_num_frames(2, true);
        sequence.set_key_frame(
            0,
            AnimKeyFrame::Trans(KeyFrameTrans {
                translation: AnimVector::ZERO,
                delta_time: 0.0,
                ..Default::default()
            }),
        );
        sequence.set_key_frame(
            1,
            AnimKeyFrame::Trans(KeyFrameTrans {
                translation: AnimVector::new(0.0, end_y, 0.0),
                delta_time: 1.0,
                ..Default::default()
            }),
        );
        sequence
    }

    #[test]
    fn live_player_input_defers_heading_interpolation_to_base_ped_phase() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.state.active_in_world = true;
        player.locomotion.rotation_cur = 0.0;
        player.locomotion.rotation_dest = 0.0;

        let mut pad = crate::core::Pad::CPad::default();
        pad.new_state.left_stick_x = 128;
        let moved = player.apply_on_foot_pad_input_camera(
            &pad,
            1.0 / GTA_PHYSICS_HZ,
            true,
            None,
            false,
            false,
        );

        assert!(moved);
        assert_eq!(player.locomotion.rotation_cur, 0.0);
        assert_ne!(player.locomotion.rotation_dest, 0.0);

        player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, 1.0, true);
        assert_ne!(player.locomotion.rotation_cur, 0.0);
    }

    #[test]
    fn mouse_fight_zero_input_preserves_native_move_state() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.state.active_in_world = true;
        player.ped.ped_state = PedState::PED_FIGHT;
        player.ped.move_state = eMoveState::PEDMOVE_RUN;

        let pad = CPad::default();
        let moved = player.apply_on_foot_mouse_runaround_input_camera(
            &pad,
            1.0 / GTA_PHYSICS_HZ,
            true,
            Some(0.5),
            false,
            false,
        );

        assert!(!moved);
        assert_eq!(player.state.move_speed, 0.0);
        assert_eq!(player.ped.move_state, eMoveState::PEDMOVE_RUN);
    }

    #[test]
    fn zelda_attack_state_keeps_processing_movement_input() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.state.active_in_world = true;
        player.ped.ped_state = PedState::PED_ATTACK;
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;

        let mut pad = CPad::default();
        pad.new_state.left_stick_y = -128;
        let moved = player.apply_on_foot_pad_input_camera(
            &pad,
            1.0 / GTA_PHYSICS_HZ,
            true,
            Some(0.0),
            false,
            false,
        );

        assert!(moved);
        assert!(player.state.move_speed > 0.0);
    }

    #[test]
    fn mouse_pc_walk_direction_rotates_forward_root_motion_for_wasd_diagonals_and_back() {
        fn moved_for(dpad_up: i16, dpad_down: i16, dpad_left: i16, dpad_right: i16) -> CVector2D {
            let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
            player.locomotion.rotation_cur = 0.0;
            let mut pad = CPad::default();
            pad.new_state.dpad_up = dpad_up;
            pad.new_state.dpad_down = dpad_down;
            pad.new_state.dpad_left = dpad_left;
            pad.new_state.dpad_right = dpad_right;
            apply_player_mouse_movement_direction(&mut player.as_mut(), &pad, CVector2D::new(0.0, 1.0))
        }

        let wa = moved_for(255, 0, 255, 0);
        let wd = moved_for(255, 0, 0, 255);
        let s = moved_for(0, 255, 0, 0);
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;

        assert!((wa.x + diagonal).abs() < 1.0e-5);
        assert!((wa.y - diagonal).abs() < 1.0e-5);
        assert!((wd.x - diagonal).abs() < 1.0e-5);
        assert!((wd.y - diagonal).abs() < 1.0e-5);
        assert!(s.x.abs() < 1.0e-5);
        assert!((s.y + 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn mouse_pc_jump_launch_keeps_wasd_diagonal_heading() {
        fn launch_dir(dpad_left: i16, dpad_right: i16) -> CVector2D {
            let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
            player.locomotion.rotation_cur = 0.0;
            let mut pad = CPad::default();
            pad.new_state.dpad_up = 255;
            pad.new_state.dpad_left = dpad_left;
            pad.new_state.dpad_right = dpad_right;

            // CalculateNewVelocity/WorkOutHeadingForMovingFirstPerson runs
            // before FinishLaunchCB and caches the same walk angle used there.
            let _ =
                apply_player_mouse_movement_direction(&mut player.as_mut(), &pad, CVector2D::new(0.0, 1.0));
            let heading = player_jump_launch_heading(player.as_ref(), true);
            CVector2D::new(-heading.sin(), heading.cos())
        }

        let wa = launch_dir(255, 0);
        let wd = launch_dir(0, 255);
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;

        assert!((wa.x + diagonal).abs() < 1.0e-5);
        assert!((wa.y - diagonal).abs() < 1.0e-5);
        assert!((wd.x - diagonal).abs() < 1.0e-5);
        assert!((wd.y - diagonal).abs() < 1.0e-5);
    }

    #[test]
    fn mouse_pc_process_anim_groups_selects_native_left_right_and_back_groups() {
        use crate::animation::AnimAssocDefinitions::{
            ASSOCGRP_PLAYER, ASSOCGRP_PLAYERBACK, ASSOCGRP_PLAYERLEFT, ASSOCGRP_PLAYERRIGHT,
        };

        assert_eq!(
            player_anim_group_for_control(
                eWeaponType::WEAPONTYPE_UNARMED,
                std::f32::consts::FRAC_PI_2,
                true,
            ),
            ASSOCGRP_PLAYERLEFT
        );
        assert_eq!(
            player_anim_group_for_control(
                eWeaponType::WEAPONTYPE_UNARMED,
                -std::f32::consts::FRAC_PI_2,
                true,
            ),
            ASSOCGRP_PLAYERRIGHT
        );
        assert_eq!(
            player_anim_group_for_control(
                eWeaponType::WEAPONTYPE_UNARMED,
                -std::f32::consts::PI,
                true,
            ),
            ASSOCGRP_PLAYERBACK
        );
        assert_eq!(
            player_anim_group_for_control(
                eWeaponType::WEAPONTYPE_UNARMED,
                std::f32::consts::FRAC_PI_4,
                true,
            ),
            ASSOCGRP_PLAYER
        );
    }

    #[test]
    fn reapply_move_anims_preserves_native_movement_phase_and_shared_idle() {
        use crate::animation::AnimationId::{ANIM_STD_IDLE, ANIM_STD_WALK};

        fn association(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            total_length: f32,
            flags: i32,
            current_time: f32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = flags;
            association.start(current_time);
            association
        }

        const NEW_GROUP: usize = 1;
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); NEW_GROUP + 1];
        manager.anim_assoc_groups[NEW_GROUP]
            .assoc_list
            .push(association(
                ANIM_STD_WALK,
                "walk_player_left",
                4.0,
                ASSOC_MOVEMENT | ASSOC_REPEAT,
                0.0,
            ));
        manager.anim_assoc_groups[NEW_GROUP]
            .assoc_list
            .push(association(
                ANIM_STD_IDLE,
                "IDLE_STANCE",
                3.0,
                ASSOC_REPEAT,
                0.0,
            ));

        let mut clump = CAnimBlendClumpData::new();
        let mut old_walk = association(
            ANIM_STD_WALK,
            "walk_player",
            2.0,
            ASSOC_MOVEMENT | ASSOC_REPEAT,
            1.0,
        );
        old_walk.blend_amount = 0.75;
        old_walk.blend_delta = 0.5;
        clump.associations.push(old_walk);
        clump.associations.push(association(
            ANIM_STD_IDLE,
            "IDLE_STANCE",
            3.0,
            ASSOC_REPEAT,
            0.7,
        ));

        reapply_player_move_anims(&mut clump, &manager, NEW_GROUP);

        assert_eq!(
            clump.associations[0].hierarchy.name, "walk_player_left",
            "native AddAnimation prepends ReApplyMoveAnims replacements"
        );
        let replacement = clump
            .associations
            .iter()
            .find(|association| association.hierarchy.name == "walk_player_left")
            .expect("strafe walk association should replace forward walk");
        assert!((replacement.current_time - 2.0).abs() < 1.0e-6);
        assert_eq!(replacement.blend_amount, 0.75);
        assert_eq!(replacement.blend_delta, 0.5);

        let idles = clump
            .associations
            .iter()
            .filter(|association| association.anim_id == ANIM_STD_IDLE)
            .collect::<Vec<_>>();
        assert_eq!(idles.len(), 1, "shared native idle must not be restarted");
        assert!((idles[0].current_time - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn add_player_movement_association_prepends_and_syncs_from_native_head() {
        use crate::animation::AnimationId::{ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK};

        fn association(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            total_length: f32,
            current_time: f32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = ASSOC_MOVEMENT | ASSOC_REPEAT;
            association.start(current_time);
            association
        }

        const GROUP: usize = 1;
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); GROUP + 1];
        manager.anim_assoc_groups[GROUP]
            .assoc_list
            .push(association(ANIM_STD_RUN, "run_new", 4.0, 0.0));

        let mut clump = CAnimBlendClumpData::new();
        clump
            .associations
            .push(association(ANIM_STD_WALK, "walk_head", 2.0, 0.5));
        clump
            .associations
            .push(association(ANIM_STD_RUNFAST, "sprint_tail", 2.0, 1.5));

        add_player_movement_association(&mut clump, &manager, GROUP, ANIM_STD_RUN);

        assert_eq!(clump.associations[0].anim_id, ANIM_STD_RUN);
        assert_eq!(clump.associations[0].hierarchy.name, "run_new");
        assert!(
            (clump.associations[0].current_time - 1.0).abs() < 1.0e-6,
            "new run must sync to the pre-existing head movement phase (0.5/2.0)"
        );
        assert_eq!(clump.associations[1].hierarchy.name, "walk_head");
    }

    #[test]
    fn sprint_blend_matches_native_fade_delete_and_last_movement_sync() {
        use crate::animation::AnimationId::{ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK};

        fn association(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            total_length: f32,
            current_time: f32,
            blend_amount: f32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = ASSOC_MOVEMENT | ASSOC_REPEAT;
            association.start(current_time);
            association.blend_amount = blend_amount;
            association
        }

        const GROUP: usize = 1;
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); GROUP + 1];
        manager.anim_assoc_groups[GROUP]
            .assoc_list
            .push(association(ANIM_STD_RUNFAST, "sprint_new", 4.0, 0.0, 1.0));

        // Native list traversal is head-first. The last movement encountered
        // here is WALK, so RUNFAST must sync to WALK's 0.5 gait phase.
        let mut clump = CAnimBlendClumpData::new();
        clump
            .associations
            .push(association(ANIM_STD_RUN, "run_head", 2.0, 0.5, 1.0));
        clump
            .associations
            .push(association(ANIM_STD_WALK, "walk_tail", 3.0, 1.5, 0.0));

        blend_player_animation(&mut clump, &manager, GROUP, ANIM_STD_RUNFAST, 2.0);

        let sprint = &clump.associations[0];
        assert_eq!(sprint.anim_id, ANIM_STD_RUNFAST);
        assert!((sprint.current_time - 2.0).abs() < 1.0e-6);
        assert_eq!(sprint.blend_amount, 0.0);
        assert_eq!(sprint.blend_delta, 2.0);

        let run = clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_RUN)
            .unwrap();
        let walk = clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_WALK)
            .unwrap();
        assert_eq!(run.blend_delta, -2.0);
        assert_ne!(
            run.flags & crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT,
            0
        );
        assert_eq!(walk.blend_delta, -1.0);
        assert_ne!(
            walk.flags & crate::animation::AnimBlendAssociation::ASSOC_DELETEFADEDOUT,
            0
        );
    }

    #[test]
    fn set_real_move_anim_recreates_missing_walk_run_at_zero_blend() {
        use crate::animation::AnimationId::{ANIM_STD_RUN, ANIM_STD_WALK};

        fn template(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
        ) -> CAnimBlendAssociation {
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(
                CAnimBlendHierarchy {
                    name: name.to_owned(),
                    total_length: 1.0,
                    ..Default::default()
                },
                0,
            );
            association.anim_id = anim_id;
            association.flags = ASSOC_MOVEMENT | ASSOC_REPEAT;
            association
        }

        const GROUP: usize = 1;
        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups = vec![CAnimBlendAssocGroup::new(); GROUP + 1];
        manager.anim_assoc_groups[GROUP].assoc_list = vec![
            template(ANIM_STD_WALK, "walk_player"),
            template(ANIM_STD_RUN, "run_player"),
        ];
        let mut clump = CAnimBlendClumpData::new();

        ensure_player_move_association_zero_blend(&mut clump, &manager, GROUP, ANIM_STD_WALK);
        ensure_player_move_association_zero_blend(&mut clump, &manager, GROUP, ANIM_STD_RUN);

        for animation_id in [ANIM_STD_WALK, ANIM_STD_RUN] {
            let association = clump
                .associations
                .iter()
                .find(|association| association.anim_id == animation_id)
                .unwrap();
            assert_eq!(association.blend_amount, 0.0);
        }
    }

    #[test]
    fn leaving_idle_restarts_walk_run_phase_and_uses_walk_start() {
        use crate::animation::{
            AnimAssocDefinitions::ASSOCGRP_PLAYER,
            AnimationId::{
                ANIM_STD_IDLE, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_STARTWALK, ANIM_STD_WALK,
            },
        };

        fn template(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            flags: i32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length: 1.0,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = flags;
            association
        }

        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups =
            vec![CAnimBlendAssocGroup::new(); ASSOCGRP_PLAYER.saturating_add(1)];
        manager.anim_assoc_groups[ASSOCGRP_PLAYER].assoc_list = vec![
            template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(
                ANIM_STD_RUNFAST,
                "SPRINT_civi",
                ASSOC_MOVEMENT | ASSOC_REPEAT,
            ),
            template(ANIM_STD_IDLE, "IDLE_STANCE", ASSOC_REPEAT | ASSOC_IDLE),
            template(ANIM_STD_STARTWALK, "walk_start", 0),
        ];

        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.anim_group = ASSOCGRP_PLAYER as i32;
        player.state.move_speed = 0.5;
        let mut walk = template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        walk.start(0.8);
        let mut run = template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        run.start(0.6);
        let mut sprint = template(
            ANIM_STD_RUNFAST,
            "SPRINT_civi",
            ASSOC_MOVEMENT | ASSOC_REPEAT,
        );
        sprint.start(0.4);
        let mut idle = template(ANIM_STD_IDLE, "IDLE_STANCE", ASSOC_REPEAT | ASSOC_IDLE);
        idle.start(0.3);
        player.animation.movement_anim_clump.associations = vec![walk, run, sprint, idle];

        let mut timer = CTimer::default();
        // Explicit animation-only unit clump; runtime requires installed DFF.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        update_player_movement_animation(&mut player.as_mut(), &manager, &CFileLoader::default(),
            0.0, false, false, 0, &mut timer);

        let walk = player
            .animation.movement_anim_clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_WALK)
            .expect("walk association should remain");
        let run = player
            .animation.movement_anim_clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_RUN)
            .expect("run association should remain");
        assert_eq!(walk.current_time, 0.0);
        assert_eq!(run.current_time, 0.0);
        assert!(!walk.is_running());
        assert!(!run.is_running());
        assert!(player.animation.movement_anim_clump.associations.iter().any(
            |association| association.anim_id == ANIM_STD_STARTWALK && association.is_running()
        ));
        assert!(
            !player
                .animation.movement_anim_clump
                .associations
                .iter()
                .any(|association| association.anim_id == ANIM_STD_RUNFAST)
        );
        assert!(
            !player
                .animation.movement_anim_clump
                .associations
                .iter()
                .any(|association| association.anim_id == ANIM_STD_IDLE)
        );
    }

    #[test]
    fn exhausted_sprint_drops_native_sprint_state_and_blends_back_to_run() {
        use crate::animation::{
            AnimAssocDefinitions::ASSOCGRP_PLAYER,
            AnimationId::{ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK},
        };

        fn template(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length: 1.0,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = ASSOC_MOVEMENT | ASSOC_REPEAT;
            association.start(0.25);
            association
        }

        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups =
            vec![CAnimBlendAssocGroup::new(); ASSOCGRP_PLAYER.saturating_add(1)];
        manager.anim_assoc_groups[ASSOCGRP_PLAYER].assoc_list = vec![
            template(ANIM_STD_WALK, "walk_player"),
            template(ANIM_STD_RUN, "run_player"),
            template(ANIM_STD_RUNFAST, "SPRINT_civi"),
        ];

        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.anim_group = ASSOCGRP_PLAYER as i32;
        player.ped.move_state = eMoveState::PEDMOVE_SPRINT;
        player.state.move_speed = 2.5;
        player.state.current_stamina = -150.0;

        let mut walk = template(ANIM_STD_WALK, "walk_player");
        walk.blend_amount = 0.0;
        let mut run = template(ANIM_STD_RUN, "run_player");
        run.blend_amount = 0.0;
        let mut sprint = template(ANIM_STD_RUNFAST, "SPRINT_civi");
        sprint.blend_amount = 1.0;
        sprint.blend_delta = 0.0;
        player.animation.movement_anim_clump.associations = vec![walk, run, sprint];

        let mut timer = CTimer::default();
        // Explicit animation-only unit clump; runtime requires installed DFF.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        update_player_movement_animation(&mut player.as_mut(), &manager, &CFileLoader::default(),
            0.0, false, false, 0, &mut timer);

        assert_eq!(player.ped.move_state, eMoveState::PEDMOVE_STILL);
        let sprint = player
            .animation.movement_anim_clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_RUNFAST)
            .expect("sprint association should fade out before deletion");
        let run = player
            .animation.movement_anim_clump
            .associations
            .iter()
            .find(|association| association.anim_id == ANIM_STD_RUN)
            .expect("run association should remain for sprint fade-out");
        assert_eq!(sprint.blend_delta, -1.0);
        assert_eq!(run.blend_delta, 1.0);
    }

    #[test]
    fn sprint_run_stop_locks_heading_until_native_stop_clip_finishes() {
        use crate::animation::{
            AnimAssocDefinitions::{ASSOCGRP_PLAYER, ASSOCGRP_STD},
            AnimBlendAssociation::ASSOC_RUNNING,
            AnimationId::{
                ANIM_STD_IDLE, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_RUNSTOP1,
                ANIM_STD_RUNSTOP2, ANIM_STD_WALK,
            },
        };

        fn template(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            flags: i32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length: 1.0,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = flags;
            association
        }

        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups =
            vec![CAnimBlendAssocGroup::new(); ASSOCGRP_PLAYER.saturating_add(1)];
        manager.anim_assoc_groups[ASSOCGRP_STD].assoc_list = vec![
            template(ANIM_STD_RUNSTOP1, "run_stop", 0),
            template(ANIM_STD_RUNSTOP2, "run_stopR", 0),
        ];
        manager.anim_assoc_groups[ASSOCGRP_PLAYER].assoc_list = vec![
            template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(
                ANIM_STD_RUNFAST,
                "SPRINT_civi",
                ASSOC_MOVEMENT | ASSOC_REPEAT,
            ),
            template(ANIM_STD_IDLE, "IDLE_STANCE", ASSOC_REPEAT | ASSOC_IDLE),
        ];

        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.anim_group = ASSOCGRP_PLAYER as i32;
        player.ped.move_state = eMoveState::PEDMOVE_STILL;
        player.locomotion.heading_rate = 15.0;
        player.state.move_speed = 0.0;

        let mut walk = template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        walk.start(0.25);
        let mut run = template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        run.start(0.25);
        let mut sprint = template(
            ANIM_STD_RUNFAST,
            "SPRINT_civi",
            ASSOC_MOVEMENT | ASSOC_REPEAT,
        );
        sprint.start(0.25);
        sprint.blend_amount = 1.0;
        player.animation.movement_anim_clump.associations = vec![walk, run, sprint];

        let mut timer = CTimer::default();
        // Explicit animation-only unit clump; runtime requires installed DFF.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        update_player_movement_animation(&mut player.as_mut(), &manager, &CFileLoader::default(),
            0.0, false, false, 0, &mut timer);

        assert_eq!(player.locomotion.heading_rate, 0.0);
        let stop = player
            .animation.movement_anim_clump
            .associations
            .iter_mut()
            .find(|association| {
                matches!(association.anim_id, ANIM_STD_RUNSTOP1 | ANIM_STD_RUNSTOP2)
            })
            .expect("run-stop association should start when sprinting stops sharply");
        stop.flags &= !ASSOC_RUNNING;

        // Explicit animation-only unit clump; runtime requires installed DFF.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        update_player_movement_animation(&mut player.as_mut(), &manager, &CFileLoader::default(),
            0.0, false, false, 0, &mut timer);

        assert_eq!(player.locomotion.heading_rate, 15.0);
    }

    #[test]
    fn running_land_finish_resets_native_locomotion_phase() {
        use crate::animation::{
            AnimAssocDefinitions::ASSOCGRP_PLAYER,
            AnimationId::{ANIM_STD_JUMP_LAND, ANIM_STD_RUN, ANIM_STD_RUNFAST, ANIM_STD_WALK},
        };

        fn template(
            anim_id: crate::animation::AnimationId::AnimationId,
            name: &str,
            flags: i32,
        ) -> CAnimBlendAssociation {
            let hierarchy = CAnimBlendHierarchy {
                name: name.to_owned(),
                total_length: 1.0,
                ..Default::default()
            };
            let mut association = CAnimBlendAssociation::new();
            association.init_with_hierarchy(hierarchy, 0);
            association.anim_id = anim_id;
            association.flags = flags;
            association
        }

        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        let mut land = template(ANIM_STD_JUMP_LAND, "jump_land", 0);
        land.start(0.9);
        player.state.jump_association = Some(land);

        assert!(update_player_jump_animation(&mut player.as_mut(), 0.2));
        assert!(player.ped.flags.reset_walk_anims);

        let mut manager = CAnimManager::initialise();
        manager.anim_assoc_groups =
            vec![CAnimBlendAssocGroup::new(); ASSOCGRP_PLAYER.saturating_add(1)];
        manager.anim_assoc_groups[ASSOCGRP_PLAYER].assoc_list = vec![
            template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT),
            template(
                ANIM_STD_RUNFAST,
                "SPRINT_civi",
                ASSOC_MOVEMENT | ASSOC_REPEAT,
            ),
        ];
        player.ped.anim_group = ASSOCGRP_PLAYER as i32;
        player.ped.move_state = eMoveState::PEDMOVE_SPRINT;
        player.state.move_speed = 2.5;

        let mut walk = template(ANIM_STD_WALK, "walk_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        walk.start(0.8);
        let mut run = template(ANIM_STD_RUN, "run_player", ASSOC_MOVEMENT | ASSOC_REPEAT);
        run.start(0.6);
        let mut sprint = template(
            ANIM_STD_RUNFAST,
            "SPRINT_civi",
            ASSOC_MOVEMENT | ASSOC_REPEAT,
        );
        sprint.start(0.4);
        sprint.blend_amount = 1.0;
        player.animation.movement_anim_clump.associations = vec![walk, run, sprint];

        let mut timer = CTimer::default();
        // Explicit animation-only unit clump; runtime requires installed DFF.
        player.animation.movement_anim_clump.set_number_of_frames(1);
        player.animation.movement_anim_clump.frames[0].frame_name = "test_ped_frame".into();
        update_player_movement_animation(&mut player.as_mut(), &manager, &CFileLoader::default(),
            0.0, false, false, 0, &mut timer);

        assert!(!player.ped.flags.reset_walk_anims);
        for animation_id in [ANIM_STD_WALK, ANIM_STD_RUN, ANIM_STD_RUNFAST] {
            let association = player
                .animation.movement_anim_clump
                .associations
                .iter()
                .find(|association| association.anim_id == animation_id)
                .expect("movement association should remain after phase reset");
            assert_eq!(association.current_time, 0.0);
        }
    }

    #[test]
    fn airborne_anim_delta_does_not_survive_into_touchdown_base_phase() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.locomotion.anim_move_delta = CVector2D::new(0.25, -0.5);

        player.stage_anim_move_delta(None);

        assert_eq!(player.locomotion.anim_move_delta, CVector2D::new(0.0, 0.0));
        let moved = player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, 1.0, true);
        assert_eq!(moved, CVector2D::new(0.0, 0.0));
    }

    #[test]
    fn standing_spawn_with_zero_staged_delta_does_not_create_xy_motion() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.flags.is_standing = true;
        player.ped.physical.move_speed = CVector::ZERO;
        player.stage_anim_move_delta(None);

        let moved_native = player.ped.calculate_new_velocity(&mut player.locomotion, &player.vitality, 1.0, true);
        let delta = apply_player_standing_update_position(
            &mut player.ped,
            &mut player.locomotion,
            CVector::new(moved_native.x, moved_native.y, 0.0),
            None,
            1.0 / GTA_PHYSICS_HZ,
        );

        assert_eq!(delta, CVector::ZERO);
        assert_eq!(player.ped.physical.move_speed.x, 0.0);
        assert_eq!(player.ped.physical.move_speed.y, 0.0);
    }

    #[test]
    fn fallen_states_use_ground_hit_col_model_but_getup_returns_to_normal_ped_col() {
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut CGeneral::default());
        for state in [
            PedState::PED_FALL,
            PedState::PED_DIVE_AWAY,
            PedState::PED_DIE,
            PedState::PED_DEAD,
        ] {
            ped.ped_state = state;
            assert!(
                ped.use_ground_col_model(),
                "{state:?} must use fallen ped COL"
            );
        }

        ped.ped_state = PedState::PED_GETUP;
        assert!(!ped.use_ground_col_model());
        ped.ped_state = PedState::PED_IDLE;
        assert!(!ped.use_ground_col_model());
    }

    #[test]
    fn mouse_control_state_gate_matches_re3() {
        let mut ped = CPed::new(ePedType::PEDTYPE_PLAYER1, &mut CGeneral::default());
        for state in [
            PedState::PED_NONE,
            PedState::PED_IDLE,
            PedState::PED_FLEE_POS,
            PedState::PED_FLEE_ENTITY,
            PedState::PED_ATTACK,
            PedState::PED_FIGHT,
            PedState::PED_AIM_GUN,
            PedState::PED_JUMP,
        ] {
            ped.ped_state = state;
            assert!(ped.can_strafe_or_mouse_control(), "{state:?}");
        }
        ped.ped_state = PedState::PED_FALL;
        assert!(!ped.can_strafe_or_mouse_control());
        ped.ped_state = PedState::PED_GETUP;
        assert!(!ped.can_strafe_or_mouse_control());
    }

    #[test]
    fn landing_keeps_collision_established_standing_when_second_probe_misses() {
        assert!(player_landing_touching_ground(true, 10.0, None));
        assert!(!player_landing_touching_ground(false, 10.0, None));
        assert!(!player_landing_touching_ground(false, 10.0, Some(8.0)));
        assert!(player_landing_touching_ground(false, 10.0, Some(8.90)));
    }

    #[test]
    fn bump_seam_keeps_authoritative_collision_standing_without_secondary_support() {
        assert!(player_grounded_after_collision_or_support(
            true, 10.0, None, 0.0, -2.0,
        ));
        assert!(!player_grounded_after_collision_or_support(
            false, 10.0, None, 0.0, -2.0,
        ));
        assert!(player_grounded_after_collision_or_support(
            false,
            10.0,
            Some(8.96),
            0.0,
            -2.0,
        ));
        assert!(!player_grounded_after_collision_or_support(
            false,
            10.0,
            Some(8.96),
            0.0,
            2.0,
        ));
    }

    #[test]
    fn landing_partial_blends_into_physics_root_velocity_without_advancing_time_twice() {
        let mut clump = CAnimBlendClumpData::new();
        clump.set_number_of_frames(1);
        clump.frames[0].flag |= VELOCITY_EXTRACTION;
        clump.velocity2d = Some((0.0, 0.0));
        clump.associations.push(translated_test_association(
            10.0,
            ASSOC_HAS_TRANSLATION,
            1.0,
        ));
        let mut landing =
            translated_test_association(20.0, ASSOC_PARTIAL | ASSOC_HAS_TRANSLATION, 0.25);

        let transform = frame_update_callback_with_velocity_extraction_overlay_non_skinned(
            &mut clump,
            0,
            true,
            Some(&mut landing),
        );

        let (_, y) = transform.velocity2d.expect("velocity extraction must run");
        // Base root: 10 * .5 * .75 = 3.75. Landing partial:
        // 20 * .5 * .25 = 2.5. Native clump result is their sum.
        assert!(
            (y - 6.25).abs() < 1.0e-5,
            "unexpected blended root delta: {y}"
        );
        assert_eq!(
            landing.current_time, 0.0,
            "frame callback must not update association time"
        );
    }

    #[test]
    fn upper_body_partial_does_not_steal_player_root_motion() {
        let mut clump = CAnimBlendClumpData::new();
        clump.set_number_of_frames(1);
        clump.frames[0].flag |= VELOCITY_EXTRACTION;
        clump.velocity2d = Some((0.0, 0.0));
        clump.associations.push(translated_test_association(
            10.0,
            ASSOC_HAS_TRANSLATION,
            1.0,
        ));

        // Weapon/punch associations are PARTIAL but have no root translation.
        // In native FrameUpdate they are absent from the root frame's node
        // list, so a fully blended upper-body attack cannot zero locomotion.
        let mut attack = CAnimBlendAssociation::new();
        attack.flags = ASSOC_PARTIAL;
        attack.blend_amount = 1.0;
        clump.associations.push(attack);

        let transform = frame_update_callback_with_velocity_extraction_overlay_non_skinned(
            &mut clump, 0, true, None,
        );

        let (_, y) = transform.velocity2d.expect("velocity extraction must run");
        assert!(
            (y - 5.0).abs() < 1.0e-5,
            "upper-body attack stole root motion: {y}"
        );
    }

    #[test]
    fn jump_and_fall_blend_animation_paths_start_with_native_fade_in_rates() {
        use crate::animation::AnimationId::{
            ANIM_STD_FALL, ANIM_STD_FALL_COLLAPSE, ANIM_STD_FALL_GLIDE, ANIM_STD_FALL_LAND,
            ANIM_STD_HANDSCOWER, ANIM_STD_JUMP_GLIDE, ANIM_STD_JUMP_LAND, ANIM_STD_JUMP_LAUNCH,
        };

        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_JUMP_LAUNCH),
            Some(8.0)
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_HANDSCOWER),
            Some(8.0)
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_FALL),
            Some(4.0)
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_FALL_GLIDE),
            Some(4.0)
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_JUMP_GLIDE),
            None
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_JUMP_LAND),
            None
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_FALL_LAND),
            None
        );
        assert_eq!(
            player_jump_animation_blend_in_delta(ANIM_STD_FALL_COLLAPSE),
            None
        );
    }

    #[test]
    fn velocity_extraction_ignores_translated_limb_tracks_after_root() {
        let mut clump = CAnimBlendClumpData::new();
        clump.set_number_of_frames(1);
        clump.frames[0].flag |= VELOCITY_EXTRACTION;
        clump.velocity2d = Some((0.0, 0.0));

        let hierarchy = CAnimBlendHierarchy {
            sequences: vec![
                translated_test_sequence("root", 10.0),
                translated_test_sequence("translated_limb", 100.0),
            ],
            total_length: 1.0,
            ..Default::default()
        };
        let mut association = CAnimBlendAssociation::new();
        association.init_with_hierarchy(hierarchy, 2);
        association.flags = ASSOC_HAS_TRANSLATION;
        association.blend_amount = 1.0;
        association.start(0.0);
        association.time_step = 0.5;
        clump.associations.push(association);

        let transform = frame_update_callback_with_velocity_extraction_overlay_non_skinned(
            &mut clump, 0, true, None,
        );
        let (_, y) = transform.velocity2d.expect("velocity extraction must run");

        // Only the root frame contributes to m_vecAnimMoveDelta. The limb's
        // authored translation must remain a skeletal pose, not ped momentum.
        assert!(
            (y - 5.0).abs() < 1.0e-5,
            "translated limb leaked into root velocity: {y}"
        );
    }

    #[test]
    fn in_the_air_landing_threshold_uses_root_height_without_feet_offset() {
        assert!(!player_in_the_air_should_land(10.0, 8.70));
        assert!(player_in_the_air_should_land(10.0, 8.71));
    }

    #[test]
    fn falling_assoc_enters_collapse_before_world_collision() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.flags.is_standing = false;
        player.ped.flags.is_in_the_air = true;
        player.ped.physical.move_speed.z = -0.2 * GTA_PHYSICS_HZ;
        let mut fall = CAnimBlendAssociation::new();
        fall.anim_id = crate::animation::AnimationId::ANIM_STD_FALL;
        player.state.jump_association = Some(fall);

        let collapsed = service_player_air_state_before_collision(&mut player.as_mut(),
            &CAnimManager::initialise(),
            CVector::new(0.0, 0.0, 10.0),
            Some(8.71),
            &CFileLoader::default(),
        );

        assert_eq!(collapsed, Some(true));
        assert!(!player.ped.flags.is_in_the_air);
        assert!(player.ped.flags.is_landing);
    }

    #[test]
    fn test_process_player_weapon_switch_next_and_prev() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        // An explicit test capacity checks data-driven reload on selection.
        // CWeaponInfoStore::default is an unloaded table, with zero capacity.
        const TEST_CLIP_CAPACITY: i32 = 7;
        let mut weapon_info = CWeaponInfoStore::default();
        weapon_info.weapon_info[eWeaponType::WEAPONTYPE_COLT45 as usize].amount_of_ammunition = TEST_CLIP_CAPACITY;
        give_ped_weapon(&mut player.ped, 1, 0); // Baseball bat
        give_ped_weapon(&mut player.ped, 2, 50); // Colt45
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;

        let mut pad = crate::core::Pad::CPad::default();

        // 1. Wheel Down / Next weapon (step = 1) -> Bat
        pad.mouse_wheel_down = true;
        pad.cycle_weapon_steps = 1;
        process_player_weapon_switch(&mut player.as_mut(), &pad, &weapon_info);
        assert_eq!(
            player.ped.current_weapon,
            eWeaponType::WEAPONTYPE_BASEBALLBAT
        );

        // 2. Wheel Down again -> Colt45
        process_player_weapon_switch(&mut player.as_mut(), &pad, &weapon_info);
        assert_eq!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_COLT45);
        assert_eq!(player.ped.weapons[eWeaponType::WEAPONTYPE_COLT45 as usize].ammo_in_clip, TEST_CLIP_CAPACITY);

        // 3. Wheel Up / Prev weapon -> Bat
        pad.mouse_wheel_down = false;
        pad.mouse_wheel_up = true;
        process_player_weapon_switch(&mut player.as_mut(), &pad, &weapon_info);
        assert_eq!(
            player.ped.current_weapon,
            eWeaponType::WEAPONTYPE_BASEBALLBAT
        );

        // 4. Wheel Up -> Unarmed
        process_player_weapon_switch(&mut player.as_mut(), &pad, &weapon_info);
        assert_eq!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_UNARMED);
    }

    #[test]
    fn test_process_player_weapon_switch_multi_notch_fast_scroll() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, 1, 0); // Baseball bat
        give_ped_weapon(&mut player.ped, 2, 50); // Colt45
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;

        let mut pad = crate::core::Pad::CPad::default();
        // 2 notches down in one frame: Unarmed -> Bat -> Colt45
        pad.mouse_wheel_down = true;
        pad.cycle_weapon_steps = 2;
        process_player_weapon_switch(&mut player.as_mut(), &pad, &CWeaponInfoStore::default());
        assert_eq!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_COLT45);
    }

    #[test]
    fn test_process_player_weapon_switch_out_of_ammo_auto_switch() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, 1, 0); // Baseball bat
        give_ped_weapon(&mut player.ped, 2, 0); // Colt45 with 0 ammo
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;

        let pad = crate::core::Pad::CPad::default();
        process_player_weapon_switch(&mut player.as_mut(), &pad, &CWeaponInfoStore::default());
        // Should auto-switch down to baseball bat
        assert_eq!(
            player.ped.current_weapon,
            eWeaponType::WEAPONTYPE_BASEBALLBAT
        );
    }

    #[test]
    fn test_process_player_weapon_switch_deferred_during_attack() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, 1, 0); // Baseball bat
        give_ped_weapon(&mut player.ped, 2, 50); // Colt45
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;
        player.state.selected_wep_slot = eWeaponType::WEAPONTYPE_UNARMED;

        // Ped is in the middle of attacking / melee swinging
        player.ped.ped_state = PedState::PED_ATTACK;

        let mut pad = crate::core::Pad::CPad::default();
        pad.mouse_wheel_down = true;
        pad.cycle_weapon_steps = 1;

        // Player tries to cycle weapon while attacking
        process_player_weapon_switch(&mut player.as_mut(), &pad, &CWeaponInfoStore::default());
        // Desired slot is updated, but current weapon remains unchanged during attack!
        assert_eq!(
            player.state.selected_wep_slot,
            eWeaponType::WEAPONTYPE_BASEBALLBAT
        );
        assert_eq!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_UNARMED);

        // Once attack finishes (ped_state returns to IDLE)
        player.ped.ped_state = PedState::PED_IDLE;
        let empty_pad = crate::core::Pad::CPad::default();
        process_player_weapon_switch(&mut player.as_mut(), &empty_pad, &CWeaponInfoStore::default());
        // Weapon should now switch to baseball bat!
        assert_eq!(
            player.ped.current_weapon,
            eWeaponType::WEAPONTYPE_BASEBALLBAT
        );
    }

    #[test]
    fn cleared_inventory_cannot_reactivate_the_previous_selected_weapon() {
        use crate::weapons::WeaponType::eWeaponState;
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, 2, 50);
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
        player.state.selected_wep_slot = eWeaponType::WEAPONTYPE_COLT45;
        player.ped.weapon_model_id = 173;
        player.ped.weapons[2].weapon_state = eWeaponState::WEAPONSTATE_RELOADING;
        player.ped.weapons[2].timer = 1000;
        player.ped.weapons[2].add_rot_offset = true;
        player.ped.clear_weapons();
        process_player_weapon_switch(&mut player.as_mut(), &CPad::default(), &CWeaponInfoStore::default());
        assert_eq!(player.ped.current_weapon, eWeaponType::WEAPONTYPE_UNARMED);
        assert_eq!(player.ped.weapon_model_id, -1);
        assert_eq!(player.ped.max_weapon_type_allowed, 1);
        assert!(player.ped.weapons[2].add_rot_offset);
        assert!(player.ped.weapons.iter().all(|weapon| {
            weapon.weapon_type == eWeaponType::WEAPONTYPE_UNARMED
                && weapon.weapon_state == eWeaponState::WEAPONSTATE_READY
                && weapon.ammo_total == 0 && weapon.ammo_in_clip == 0 && weapon.timer == 0
        }));
    }

    #[test]
    fn sprinting_player_does_not_start_weapon_attack() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.ped_state = PedState::PED_IDLE;
        player.ped.move_state = eMoveState::PEDMOVE_SPRINT;
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_UNARMED;

        let mut pad = CPad::default();
        pad.new_state.circle = 255;
        let weapon_info = CWeaponInfoStore::default();
        let anim_manager = CAnimManager::initialise();
        let file_loader = CFileLoader::default();
        let mut particles = CParticle::default();
        let mut events = CEventList::initialise();
        let mut projectiles = CProjectileInfoPool::default();
        let mut explosions = CExplosionPool::default();
        let mut test_world = crate::control::ScriptEntities::script_entity_test_world();

        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    Some(0.0),
                    None,
                    false,
                    0,
                    16,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        assert_eq!(player.ped.ped_state, PedState::PED_IDLE);
        assert!(!player.ped.flags.is_shooting);
    }

    #[test]
    fn test_process_player_weapon_aiming_flow() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, eWeaponType::WEAPONTYPE_COLT45 as i32, 50);
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_COLT45;
        player.ped.ped_state = PedState::PED_IDLE;

        let weapon_info = CWeaponInfoStore::default();
        let anim_manager = CAnimManager::initialise();
        let file_loader = CFileLoader::default();
        let mut particles = CParticle::default();
        let mut events = CEventList::initialise();
        let mut projectiles = crate::weapons::ProjectileInfo::CProjectileInfoPool::default();
        let mut explosions = crate::weapons::Explosion::CExplosionPool::default();
        let mut test_world = crate::control::ScriptEntities::script_entity_test_world();

        let mut pad = crate::core::Pad::CPad::default();
        pad.new_state.right_shoulder1 = 255; // Target pressed

        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    Some(1.25),
                    None,
                    false,
                    0,
                    16,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        // Player entered aim state and rotated to match camera heading (-beta)
        assert_eq!(player.ped.ped_state, PedState::PED_AIM_GUN);
        assert!((player.locomotion.rotation_cur - (-1.25)).abs() < 1e-4);

        // Now release target
        pad.new_state.right_shoulder1 = 0;
        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    Some(1.25),
                    None,
                    false,
                    16,
                    32,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        // Player reverted to IDLE state
        assert_eq!(player.ped.ped_state, PedState::PED_IDLE);
    }

    #[test]
    fn test_apply_player_script_seek_step_turns_and_sets_authored_speed() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        player.ped.ped_state = PedState::PED_SEEK_POS;
        player.ped.move_state = eMoveState::PEDMOVE_WALK;
        player
            .ped
            .physical
            .entity
            .set_position(CVector::new(0.0, 0.0, 10.0));

        let dt = 1.0 / 50.0;
        let still_seeking =
            apply_player_script_seek_step(&mut player.as_mut(), CVector::new(0.0, 100.0, 10.0), dt);
        assert!(still_seeking);
        assert_eq!(player.state.move_speed, 1.0);
        assert!((player.locomotion.rotation_dest - 0.0).abs() < 1e-4);

        player.ped.move_state = eMoveState::PEDMOVE_RUN;
        apply_player_script_seek_step(&mut player.as_mut(), CVector::new(100.0, 0.0, 10.0), dt);
        assert_eq!(player.state.move_speed, 1.8);
        assert!(
            (player.locomotion.rotation_dest - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-4,
            "rotation_dest should face +X, got {}",
            player.locomotion.rotation_dest
        );
    }

    #[test]
    fn test_process_player_weapon_rocket_launcher_firing() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(
            &mut player.ped,
            eWeaponType::WEAPONTYPE_ROCKETLAUNCHER as i32,
            10,
        );
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_ROCKETLAUNCHER;
        player.ped.ped_state = PedState::PED_SNIPER_MODE;

        let weapon_info = CWeaponInfoStore::default();
        let info = weapon_info
            .GetWeaponInfo(eWeaponType::WEAPONTYPE_ROCKETLAUNCHER)
            .unwrap();
        let anim_manager = CAnimManager::initialise();
        let file_loader = CFileLoader::default();
        let mut particles = CParticle::default();
        let mut events = CEventList::initialise();
        let mut projectiles = crate::weapons::ProjectileInfo::CProjectileInfoPool::default();
        let mut explosions = crate::weapons::Explosion::CExplosionPool::default();
        let mut test_world = crate::control::ScriptEntities::script_entity_test_world();

        // Inject active attack animation crossing fire frame
        let mut assoc = crate::animation::AnimBlendAssociation::CAnimBlendAssociation::new();
        assoc.anim_id = info.anim_to_play;
        assoc.flags = crate::animation::AnimBlendAssociation::ASSOC_RUNNING;
        assoc.current_time = info.anim_frame_fire + 0.05;
        assoc.time_step = 0.1;
        assoc.blend_delta = 0.0;
        player.animation.movement_anim_clump.associations.push(assoc);

        let mut pad = crate::core::Pad::CPad::default();
        pad.new_state.circle = 255;

        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    Some(0.0),
                    Some(CVector::new(0.0, 0.8, 0.6)),
                    false,
                    0,
                    16,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        // FireProjectile only creates the projectile here. re3 creates the
        // explosion later from CProjectileInfo::Update on hit/timeout.
        assert!(
            explosions.explosions.iter().all(Option::is_none),
            "rocket firing must not create an explosion before projectile update"
        );

        // Verify projectile info was added to CProjectileInfoPool
        let rocket_projectile = projectiles
            .info
            .iter()
            .find(|p| p.in_use && p.weapon_type == eWeaponType::WEAPONTYPE_ROCKETLAUNCHER)
            .expect("Rocket projectile info should be recorded");
        assert!(rocket_projectile.native_motion);
        assert!(!rocket_projectile.affected_by_gravity);
        assert_eq!(rocket_projectile.explosion_time, 1416);
        assert!((rocket_projectile.velocity.magnitude() - 1.25).abs() < 1.0e-4);
        assert!((rocket_projectile.velocity.y - 1.0).abs() < 1.0e-4);
        assert!((rocket_projectile.velocity.z - 0.75).abs() < 1.0e-4);
    }

    #[test]
    fn released_grenade_transitions_from_start_throw_and_spawns_projectile() {
        let mut player = CPlayerPed::new(&mut CGeneral::default(), None);
        give_ped_weapon(&mut player.ped, eWeaponType::WEAPONTYPE_GRENADE as i32, 5);
        player.ped.current_weapon = eWeaponType::WEAPONTYPE_GRENADE;
        player.state.selected_wep_slot = eWeaponType::WEAPONTYPE_GRENADE;
        player.ped.ped_state = PedState::PED_ATTACK;
        player.state.attack_button_counter = 8.0;
        player.state.have_target_selected = true;

        let mut weapon_info = CWeaponInfoStore::default();
        {
            let grenade = &mut weapon_info.weapon_info[eWeaponType::WEAPONTYPE_GRENADE as usize];
            grenade.weapon_fire = eWeaponFire::WEAPON_FIRE_PROJECTILE;
            grenade.flags |= WEAPONFLAG_THROW;
            grenade.anim_to_play = crate::animation::AnimationId::ANIM_STD_START_THROW;
            grenade.anim_loop_end = 0.5;
            grenade.amount_of_ammunition = 1;
        }
        let info = *weapon_info
            .GetWeaponInfo(eWeaponType::WEAPONTYPE_GRENADE)
            .unwrap();
        let mut anim_manager = CAnimManager::initialise();
        let mut std_group = crate::animation::AnimBlendAssocGroup::CAnimBlendAssocGroup::new();
        let mut throw_under = crate::animation::AnimBlendAssociation::CAnimBlendAssociation::new();
        throw_under.anim_id = crate::animation::AnimationId::ANIM_STD_THROW_UNDER;
        std_group.assoc_list.push(throw_under);
        anim_manager.anim_assoc_groups.push(std_group);
        let file_loader = CFileLoader::default();
        let mut particles = CParticle::default();
        let mut events = CEventList::initialise();
        let mut projectiles = CProjectileInfoPool::default();
        let mut explosions = CExplosionPool::default();
        let mut test_world = crate::control::ScriptEntities::script_entity_test_world();
        let pad = CPad::default();

        let mut start = crate::animation::AnimBlendAssociation::CAnimBlendAssociation::new();
        start.anim_id = info.anim_to_play;
        start.current_time = 0.3;
        start.blend_delta = 0.0;
        player.animation.movement_anim_clump.associations.push(start);

        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    None,
                    None,
                    false,
                    0,
                    16,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        let throw = player
            .animation.movement_anim_clump
            .associations
            .iter_mut()
            .find(|association| {
                association.anim_id == crate::animation::AnimationId::ANIM_STD_THROW_UNDER
                    && association.is_running()
            })
            .expect("released throwable must enter THROW_UNDER");
        throw.current_time = 0.25;
        throw.time_step = 0.1;
        throw.blend_delta = 0.0;

        crate::control::ScriptEntities::with_script_entity_test_world(
            &mut test_world,
            |script_entities| {
                process_player_weapon(&mut player.as_mut(),
                    None,
                    &pad,
                    None,
                    None,
                    false,
                    16,
                    32,
                    false,
                    &weapon_info,
                    None,
                    &anim_manager,
                    &file_loader,
                    script_entities,
                    &mut particles.emitter(&crate::renderer::ParticleMgr::cParticleSystemMgr::default(), CVector::ZERO),
                    None,
                    &mut CGeneral::default(),
                    None,
                    None,
                    &mut events,
                    false,
                    None,
                    Some(&mut projectiles),
                    Some(&mut explosions),
                    None,
                    None,
                    None,
                    None,
                
    0, // Explicit unit fixture frame.
);
            },
        );

        assert!(projectiles.info.iter().any(|projectile| {
            projectile.in_use && projectile.weapon_type == eWeaponType::WEAPONTYPE_GRENADE
        }));
        assert_eq!(
            player.ped.weapons[eWeaponType::WEAPONTYPE_GRENADE as usize].ammo_total,
            4
        );
    }
}
