use bevy::prelude::*;

use crate::{
    drawing::{
        animation::{AnimationManager, AnimationNode, MultiAnimationManager, SpriteInfo},
        layering::light_layer_u8,
    },
    meta::level_data::{ExportedReplenish, Rehydrate},
    physics::collider::{ColliderActive, ColliderTriggerStub, ColliderTriggerStubs},
    uid::fresh_uid,
};

#[derive(Component)]
pub struct ReplenishMarker;

#[derive(Component)]
pub struct ReplenishCharging {
    pub timer: Timer,
}
impl ReplenishCharging {
    pub fn new() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        }
    }
}

#[derive(Bundle)]
pub struct ReplenishBundle {
    replenish: ReplenishMarker,
    multi: MultiAnimationManager,
    spatial: SpatialBundle,
    triggers: ColliderTriggerStubs,
    name: Name,
    active: ColliderActive,
}
impl ReplenishBundle {
    pub fn new(pos: IVec2) -> Self {
        let core = AnimationManager::from_nodes(vec![
            (
                "ready",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenish.png".to_string(),
                        size: UVec2::new(12, 12),
                    },
                    length: 1,
                    ..default()
                },
            ),
            (
                "exploding",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenish_explode.png".to_string(),
                        size: UVec2::new(12, 12),
                    },
                    length: 6,
                    next: Some("recharging".to_string()),
                    ..default()
                },
            ),
            (
                "recharging",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenish.png".to_string(),
                        size: UVec2::new(0, 0),
                    },
                    length: 1,
                    ..default()
                },
            ),
        ]);
        let mut light = AnimationManager::from_nodes(vec![
            (
                "ready",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenishL.png".to_string(),
                        size: UVec2::new(16, 16),
                    },
                    length: 3,
                    ..default()
                },
            ),
            (
                "exploding",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenish_explodeL.png".to_string(),
                        size: UVec2::new(16, 16),
                    },
                    length: 6,
                    next: Some("recharging".to_string()),
                    ..default()
                },
            ),
            (
                "recharging",
                AnimationNode {
                    sprite: SpriteInfo {
                        path: "sprites/replenishL.png".to_string(),
                        size: UVec2::new(0, 0),
                    },
                    length: 1,
                    ..default()
                },
            ),
        ]);
        light.set_render_layers(vec![light_layer_u8()]);
        let multi = MultiAnimationManager::from_pairs(vec![("core", core), ("light", light)]);
        let trigger = ColliderTriggerStub {
            uid: fresh_uid(),
            refresh_period: 0,
            points: vec![
                pos + IVec2::new(-7, 0),
                pos + IVec2::new(0, 7),
                pos + IVec2::new(7, 0),
                pos + IVec2::new(0, -7),
            ],
            active: true,
        };
        Self {
            replenish: ReplenishMarker,
            multi,
            spatial: SpatialBundle::from_transform(Transform::from_translation(
                pos.extend(0).as_vec3(),
            )),
            triggers: ColliderTriggerStubs(vec![trigger]),
            name: Name::new("Replenish"),
            active: ColliderActive(true),
        }
    }
}
impl Rehydrate<ReplenishBundle> for ExportedReplenish {
    fn rehydrate(self) -> ReplenishBundle {
        ReplenishBundle::new(self.pos)
    }
}

/// Checks replenishes that have ReplenishCharging and updates them
pub(super) fn update_replenishes(
    mut commands: Commands,
    mut replenishes: Query<
        (
            Entity,
            &mut ReplenishCharging,
            &mut MultiAnimationManager,
            &mut ColliderActive,
        ),
        With<ReplenishMarker>,
    >,
    time: Res<Time>,
) {
    for (eid, mut charge, mut multi, mut active) in replenishes.iter_mut() {
        charge.timer.tick(time.delta());
        if charge.timer.finished() {
            commands.entity(eid).remove::<ReplenishCharging>();
            let core: &mut AnimationManager = multi.map.get_mut("core").unwrap();
            core.reset_key("ready");
            let light: &mut AnimationManager = multi.map.get_mut("light").unwrap();
            light.reset_key("ready");
            active.0 = true;
        } else {
            active.0 = false;
        }
    }
}