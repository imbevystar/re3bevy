//! re3 reference: references/re-GTA/src/renderer/Sprite.cpp
//! re3 reference: references/re-GTA/src/renderer/Sprite.h
//!
//! RenderWare immediate 2D sprite buffer represented as Bevy-consumable draw intents.

use bevy::prelude::Resource;

use crate::{entities::Entity::CRect, objects::ParticleObject::CRGBA};

pub const SPRITEBUFFERSIZE: usize = 64;
pub const DEFAULT_FOV: f32 = 70.0;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CSprite {
    pub near_screen_z_2d: f32,
    pub far_screen_z_2d: f32,
    pub recip_near_clip_plane: f32,
    pub flush_sprite_buffer_switch_z_test: bool,
    pub buffered_vertices: Vec<SpriteVertex>,
    pub flushed_batches: Vec<SpriteBatch>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteVertex {
    pub screen: [f32; 3],
    pub camera_z: f32,
    pub recip_camera_z: f32,
    pub uv: [f32; 2],
    pub colour: CRGBA,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteBatch {
    pub vertices: Vec<SpriteVertex>,
    pub z_test_disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
    pub recip_z: f32,
}

impl CSprite {
    pub fn init_sprite_buffer(&mut self, near_screen_z: f32, far_screen_z: f32) {
        self.near_screen_z_2d = near_screen_z;
        self.far_screen_z_2d = far_screen_z;
    }

    pub fn init_sprite_buffer_2d(
        &mut self,
        near_screen_z: f32,
        far_screen_z: f32,
        near_clip_plane: f32,
    ) {
        self.recip_near_clip_plane = 1.0 / near_clip_plane;
        self.init_sprite_buffer(near_screen_z, far_screen_z);
    }

    pub fn flush_sprite_buffer(&mut self) {
        if self.buffered_vertices.is_empty() {
            return;
        }

        self.flushed_batches.push(SpriteBatch {
            vertices: std::mem::take(&mut self.buffered_vertices),
            z_test_disabled: self.flush_sprite_buffer_switch_z_test,
        });
        self.flush_sprite_buffer_switch_z_test = false;
    }

    pub fn render_buffered_one_xlu_sprite(
        &mut self,
        sprite: ScreenSprite,
        colour: CRGBA,
        intensity: i16,
        screen: [f32; 2],
        near_clip_z: f32,
        far_clip_z: f32,
    ) {
        self.flush_sprite_buffer_switch_z_test = false;
        self.push_sprite_quad(
            sprite,
            colour,
            intensity,
            screen,
            near_clip_z,
            far_clip_z,
            0.0,
        );
    }

    pub fn render_buffered_one_xlu_sprite_rotate_aspect(
        &mut self,
        sprite: ScreenSprite,
        colour: CRGBA,
        intensity: i16,
        screen: [f32; 2],
        near_clip_z: f32,
        far_clip_z: f32,
        rotation: f32,
    ) {
        self.flush_sprite_buffer_switch_z_test = false;
        self.push_sprite_quad(
            sprite,
            colour,
            intensity,
            screen,
            near_clip_z,
            far_clip_z,
            rotation,
        );
    }

    pub fn render_buffered_one_xlu_sprite_2d(
        &mut self,
        rect: CRect,
        colour: CRGBA,
        intensity: i16,
        alpha: u8,
    ) {
        self.flush_sprite_buffer_switch_z_test = true;
        let colour = modulated_colour(colour, intensity, alpha);
        self.push_2d_rect(rect, [colour; 4]);
    }

    pub fn set_6_vertices_2d(&self, rect: CRect, colours: [CRGBA; 4]) -> [SpriteVertex; 6] {
        rect_to_triangles(rect, colours, 0.0, self.recip_near_clip_plane)
    }

    pub fn drain_flushed_batches(&mut self) -> Vec<SpriteBatch> {
        std::mem::take(&mut self.flushed_batches)
    }

    fn push_sprite_quad(
        &mut self,
        sprite: ScreenSprite,
        colour: CRGBA,
        intensity: i16,
        screen: [f32; 2],
        near_clip_z: f32,
        far_clip_z: f32,
        rotation: f32,
    ) {
        if self.buffered_vertices.len() / 6 >= SPRITEBUFFERSIZE {
            self.flush_sprite_buffer();
        }
        let Some(vertices) = sprite_quad(
            sprite,
            colour,
            intensity,
            screen,
            near_clip_z,
            far_clip_z,
            rotation,
        ) else {
            return;
        };
        self.buffered_vertices.extend(vertices);
    }

    fn push_2d_rect(&mut self, rect: CRect, colours: [CRGBA; 4]) {
        if self.buffered_vertices.len() / 6 >= SPRITEBUFFERSIZE {
            self.flush_sprite_buffer();
        }
        self.buffered_vertices.extend(rect_to_triangles(
            rect,
            colours,
            0.0,
            self.recip_near_clip_plane,
        ));
    }
}

impl Default for CSprite {
    fn default() -> Self {
        Self {
            near_screen_z_2d: 0.0,
            far_screen_z_2d: 1.0,
            recip_near_clip_plane: 1.0,
            flush_sprite_buffer_switch_z_test: false,
            buffered_vertices: Vec::with_capacity(SPRITEBUFFERSIZE * 6),
            flushed_batches: Vec::new(),
        }
    }
}

fn sprite_quad(
    sprite: ScreenSprite,
    colour: CRGBA,
    intensity: i16,
    screen: [f32; 2],
    near_clip_z: f32,
    far_clip_z: f32,
    rotation: f32,
) -> Option<[SpriteVertex; 6]> {
    if sprite.z <= near_clip_z + 1.0 || sprite.z >= far_clip_z {
        return None;
    }

    let (sin, cos) = rotation.sin_cos();
    let corners = [
        (-sprite.w, -sprite.h, 0.0, 0.0),
        (-sprite.w, sprite.h, 0.0, 1.0),
        (sprite.w, sprite.h, 1.0, 1.0),
        (sprite.w, -sprite.h, 1.0, 0.0),
    ];
    let mut quad = [SpriteVertex::default(); 4];
    for (index, (local_x, local_y, u, v)) in corners.into_iter().enumerate() {
        let x = if rotation == 0.0 {
            sprite.x + local_x
        } else {
            sprite.x + local_x * cos - local_y * sin
        };
        let y = if rotation == 0.0 {
            sprite.y + local_y
        } else {
            sprite.y + local_x * sin + local_y * cos
        };
        quad[index] = SpriteVertex {
            screen: [x.clamp(0.0, screen[0]), y.clamp(0.0, screen[1]), sprite.z],
            camera_z: sprite.z,
            recip_camera_z: sprite.recip_z,
            uv: [u, v],
            colour: modulated_colour(colour, intensity, colour.alpha),
        };
    }

    Some([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]])
}

fn rect_to_triangles(
    rect: CRect,
    colours: [CRGBA; 4],
    screen_z: f32,
    recip_camera_z: f32,
) -> [SpriteVertex; 6] {
    let quad = [
        sprite_vertex_2d(
            rect.left,
            rect.top,
            screen_z,
            recip_camera_z,
            [0.0, 0.0],
            colours[0],
        ),
        sprite_vertex_2d(
            rect.left,
            rect.bottom,
            screen_z,
            recip_camera_z,
            [0.0, 1.0],
            colours[1],
        ),
        sprite_vertex_2d(
            rect.right,
            rect.bottom,
            screen_z,
            recip_camera_z,
            [1.0, 1.0],
            colours[2],
        ),
        sprite_vertex_2d(
            rect.right,
            rect.top,
            screen_z,
            recip_camera_z,
            [1.0, 0.0],
            colours[3],
        ),
    ];
    [quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]
}

fn sprite_vertex_2d(
    x: f32,
    y: f32,
    z: f32,
    recip_camera_z: f32,
    uv: [f32; 2],
    colour: CRGBA,
) -> SpriteVertex {
    SpriteVertex {
        screen: [x, y, z],
        camera_z: 1.0,
        recip_camera_z,
        uv,
        colour,
    }
}

fn modulated_colour(colour: CRGBA, intensity: i16, alpha: u8) -> CRGBA {
    let scale = intensity.max(0) as u16;
    CRGBA::new(
        ((colour.red as u16 * scale) >> 8) as u8,
        ((colour.green as u16 * scale) >> 8) as u8,
        ((colour.blue as u16 * scale) >> 8) as u8,
        alpha,
    )
}

impl Default for SpriteVertex {
    fn default() -> Self {
        Self {
            screen: [0.0; 3],
            camera_z: 0.0,
            recip_camera_z: 0.0,
            uv: [0.0; 2],
            colour: CRGBA::new(255, 255, 255, 255),
        }
    }
}
