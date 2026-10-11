//! re3 reference: references/re-GTA/src/renderer/Sprite2d.cpp
//! re3 reference: references/re-GTA/src/renderer/Sprite2d.h
//!
//! 2D sprite state and draw-intent primitives.

use crate::{entities::Entity::CRect, objects::ParticleObject::CRGBA};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSprite2d {
    pub texture_name: Option<String>,
    pub mask_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sprite2dDrawCommand {
    Sprite {
        texture_name: Option<String>,
        rect: CRect,
        color: CRGBA,
    },
    Rect {
        rect: CRect,
        colors: [CRGBA; 4],
    },
    Polygon {
        points: [(f32, f32); 4],
        color: CRGBA,
    },
}

impl CSprite2d {
    pub fn set_texture(&mut self, name: &str, mask: Option<&str>) {
        self.texture_name = (!name.is_empty()).then(|| name.to_owned());
        self.mask_name = mask.filter(|value| !value.is_empty()).map(str::to_owned);
    }

    pub fn delete(&mut self) {
        self.texture_name = None;
        self.mask_name = None;
    }

    pub fn draw(&self, rect: CRect, color: CRGBA) -> Sprite2dDrawCommand {
        Sprite2dDrawCommand::Sprite {
            texture_name: self.texture_name.clone(),
            rect,
            color,
        }
    }

    pub fn draw_rect(rect: CRect, color: CRGBA) -> Sprite2dDrawCommand {
        Sprite2dDrawCommand::Rect {
            rect,
            colors: [color; 4],
        }
    }

    pub fn draw_rect_gradient(rect: CRect, colors: [CRGBA; 4]) -> Sprite2dDrawCommand {
        Sprite2dDrawCommand::Rect { rect, colors }
    }

    pub fn draw_2d_polygon(points: [(f32, f32); 4], color: CRGBA) -> Sprite2dDrawCommand {
        Sprite2dDrawCommand::Polygon { points, color }
    }
}
