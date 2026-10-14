//! Retained Bevy nodes for CHud's ordered immediate draw descriptions.
//! Gameplay timers remain in CHud; this cache owns only submitted UI state.
use super::radar::RadarMaterial;
use bevy::{prelude::*, ui_render::ui_material::MaterialNode};

#[derive(Clone)]
pub(super) struct HudDraw {
    node: Node,
    image: Option<ImageNode>,
    material: Option<Handle<RadarMaterial>>,
    transform: UiTransform,
    background: BackgroundColor,
    z: Option<GlobalZIndex>,
    name: Name,
    parent: Option<usize>,
}

impl HudDraw {
    fn node(node: Node) -> Self {
        Self {
            node,
            image: None,
            material: None,
            transform: UiTransform::IDENTITY,
            background: BackgroundColor(Color::NONE),
            z: None,
            name: Name::new("re3 HUD widget"),
            parent: None,
        }
    }
}
impl From<(Node, ImageNode, Name)> for HudDraw {
    fn from((node, image, name): (Node, ImageNode, Name)) -> Self {
        Self {
            image: Some(image),
            name,
            ..Self::node(node)
        }
    }
}
impl From<(Node, UiTransform, ImageNode, Name)> for HudDraw {
    fn from((node, transform, image, name): (Node, UiTransform, ImageNode, Name)) -> Self {
        Self {
            image: Some(image),
            transform,
            name,
            ..Self::node(node)
        }
    }
}
impl From<(Node, MaterialNode<RadarMaterial>, Name)> for HudDraw {
    fn from((node, material, name): (Node, MaterialNode<RadarMaterial>, Name)) -> Self {
        Self {
            material: Some(material.0),
            name,
            ..Self::node(node)
        }
    }
}
impl From<(Node, BackgroundColor)> for HudDraw {
    fn from((node, background): (Node, BackgroundColor)) -> Self {
        Self {
            background,
            ..Self::node(node)
        }
    }
}
impl From<(Node, GlobalZIndex)> for HudDraw {
    fn from((node, z): (Node, GlobalZIndex)) -> Self {
        Self {
            z: Some(z),
            ..Self::node(node)
        }
    }
}

pub(super) struct HudPainter<'a> {
    pub draws: &'a mut Vec<HudDraw>,
    pub parent: Option<usize>,
}
pub(super) struct HudChild<'a> {
    draws: &'a mut Vec<HudDraw>,
    index: usize,
}
impl HudPainter<'_> {
    pub fn spawn(&mut self, draw: impl Into<HudDraw>) -> HudChild<'_> {
        let mut draw = draw.into();
        draw.parent = self.parent;
        let index = self.draws.len();
        self.draws.push(draw);
        HudChild {
            draws: self.draws,
            index,
        }
    }
}
impl HudChild<'_> {
    pub fn with_children(&mut self, f: impl FnOnce(&mut HudPainter)) {
        f(&mut HudPainter {
            draws: self.draws,
            parent: Some(self.index),
        });
    }
}

struct HudSlot {
    entity: Entity,
    draw: HudDraw,
}
#[derive(Default)]
pub(super) struct RetainedHud {
    pub root: Option<Entity>,
    pub root_node: Option<Node>,
    slots: Vec<HudSlot>,
    // Reuse draw-description storage; no UI entity allocation on stable frames.
    pub draws: Vec<HudDraw>,
}

fn image_matches(a: &Option<ImageNode>, b: &Option<ImageNode>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.image == b.image
                && a.color == b.color
                && a.flip_x == b.flip_x
                && a.flip_y == b.flip_y
                && a.rect == b.rect
                && a.image_mode == b.image_mode
        }
        _ => false,
    }
}

impl RetainedHud {
    pub fn clear(&mut self) {
        self.root = None;
        self.root_node = None;
        self.slots.clear();
        self.draws.clear();
    }

    pub fn submit(&mut self, root: Entity, commands: &mut Commands) {
        let layout_changed = self.slots.len() != self.draws.len()
            || self
                .slots
                .iter()
                .zip(&self.draws)
                .any(|(old, new)| old.draw.parent != new.parent);
        // Detach before retiring a surplus container: its active children may
        // have moved to other slots and must survive linked-child despawning.
        if layout_changed {
            commands.entity(root).replace_children(&[]);
            for slot in &self.slots {
                commands.entity(slot.entity).replace_children(&[]);
            }
        }
        while self.slots.len() > self.draws.len() {
            commands.entity(self.slots.pop().unwrap().entity).despawn();
        }
        for (index, draw) in self.draws.iter().enumerate() {
            if let Some(slot) = self.slots.get_mut(index) {
                let old = &slot.draw;
                let mut entity = commands.entity(slot.entity);
                if old.node != draw.node {
                    entity.insert(draw.node.clone());
                }
                if !image_matches(&old.image, &draw.image) {
                    if let Some(image) = &draw.image {
                        entity.insert(image.clone());
                    } else {
                        entity.remove::<ImageNode>();
                    }
                }
                if old.material != draw.material {
                    if let Some(material) = &draw.material {
                        entity.insert(MaterialNode(material.clone()));
                    } else {
                        entity.remove::<MaterialNode<RadarMaterial>>();
                    }
                }
                if old.transform != draw.transform {
                    entity.insert(draw.transform);
                }
                if old.background != draw.background {
                    entity.insert(draw.background);
                }
                if old.z != draw.z {
                    if let Some(z) = draw.z {
                        entity.insert(z);
                    } else {
                        entity.remove::<GlobalZIndex>();
                    }
                }
                if old.name != draw.name {
                    entity.insert(draw.name.clone());
                }
                slot.draw = draw.clone();
            } else {
                let mut entity = commands.spawn((
                    bevy::picking::Pickable::IGNORE,
                    draw.node.clone(),
                    draw.transform,
                    draw.background,
                    draw.name.clone(),
                ));
                if let Some(image) = &draw.image {
                    entity.insert(image.clone());
                }
                if let Some(material) = &draw.material {
                    entity.insert(MaterialNode(material.clone()));
                }
                if let Some(z) = draw.z {
                    entity.insert(z);
                }
                self.slots.push(HudSlot {
                    entity: entity.id(),
                    draw: draw.clone(),
                });
            }
        }
        if layout_changed {
            let mut children = vec![Vec::new(); self.slots.len() + 1];
            for slot in &self.slots {
                let parent = slot.draw.parent.map_or(0, |index| index + 1);
                children[parent].push(slot.entity);
            }
            commands.entity(root).replace_children(&children[0]);
            for (index, list) in children.iter().skip(1).enumerate() {
                if !list.is_empty() {
                    commands
                        .entity(self.slots[index].entity)
                        .replace_children(list);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn submit(world: &mut World, retained: &mut RetainedHud, root: Entity) {
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        retained.submit(root, &mut commands);
        queue.apply(world);
    }
    #[test]
    fn hierarchy_reconciliation_retains_live_children_during_container_retirement() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut retained = RetainedHud::default();
        {
            let mut p = HudPainter {
                draws: &mut retained.draws,
                parent: None,
            };
            p.spawn((Node::default(), GlobalZIndex(20_100)))
                .with_children(|p| {
                    p.spawn((Node::default(), ImageNode::default(), Name::new("Subtitle")));
                });
        }
        submit(&mut world, &mut retained, root);
        let widget = retained.slots[0].entity;
        let child = retained.slots[1].entity;
        retained.draws.clear();
        HudPainter {
            draws: &mut retained.draws,
            parent: None,
        }
        .spawn((Node::default(), ImageNode::default(), Name::new("Subtitle")));
        submit(&mut world, &mut retained, root);
        assert!(world.get_entity(widget).is_ok());
        assert!(world.get_entity(child).is_err());
        assert_eq!(world.get::<ChildOf>(widget).unwrap().parent(), root);
        assert!(world.get::<ImageNode>(widget).is_some());
        assert!(world.get::<GlobalZIndex>(widget).is_none());
        let entity_count = world.entities().len();
        submit(&mut world, &mut retained, root);
        assert_eq!(retained.slots[0].entity, widget);
        assert_eq!(world.entities().len(), entity_count);
        assert_eq!(world.get::<Children>(root).unwrap().as_ref(), &[widget]);
    }
}
