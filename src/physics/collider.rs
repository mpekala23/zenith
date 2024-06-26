use bevy::prelude::*;

use crate::{
    math::{MathLine, MathTriangle},
    meta::consts::MAX_COLLISIONS_PER_FRAME,
    uid::{UId, UIdMarker},
};

use super::dyno::{IntDyno, StaticCollision};

#[derive(Component, Debug)]
pub struct ColliderBoundary {
    pub points: Vec<IVec2>,
    pub lines: Vec<MathLine>,
    pub triangles: Vec<MathTriangle>,
    pub center: Vec2,
    pub bound_squared: f32,
}
impl ColliderBoundary {
    pub fn from_points(boundary_points: Vec<IVec2>) -> Self {
        let mut center = Vec2::ZERO;
        for point in boundary_points.iter() {
            center += point.as_vec2();
        }
        if boundary_points.len() > 0 {
            center /= boundary_points.len() as f32;
        }
        let mut max_dist_sq: f32 = 0.0;
        for point in boundary_points.iter() {
            max_dist_sq = max_dist_sq.max(point.as_vec2().distance_squared(center));
        }
        let fpoints: Vec<Vec2> = boundary_points.iter().map(|p| p.as_vec2()).collect();
        let lines = MathLine::from_points(&fpoints);
        let triangles = MathTriangle::triangulate(&fpoints);
        ColliderBoundary {
            points: boundary_points,
            lines,
            triangles,
            center,
            bound_squared: max_dist_sq,
        }
    }

    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        let mut min_dist_sq = f32::MAX;
        let mut min_point = Vec2 {
            x: f32::MAX,
            y: f32::MAX,
        };
        for line in self.lines.iter() {
            let close_point = line.closest_point_on_segment(&point);
            let dist_sq = point.distance_squared(close_point);
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                min_point = close_point;
            }
        }
        min_point
    }

    pub fn effective_mult(&self, point: Vec2, radius: f32) -> f32 {
        let mut total_em = 0.0;
        for triangle in self.triangles.iter() {
            let signed_dist = triangle.signed_distance_from_point(&point);
            let this_em = ((-1.0 * (signed_dist / radius) + 1.0) / 2.0)
                .min(1.0)
                .max(0.0);
            total_em += this_em;
        }
        total_em.min(1.0).max(0.0)
    }
}

#[derive(Component, Debug)]
pub struct ColliderStatic {
    pub bounciness: f32,
    pub friction: f32,
}

#[derive(Component, Debug, Default)]
pub struct ColliderTrigger {
    pub refresh_period: u32,
}

#[derive(Component, Debug)]
pub struct ColliderActive;

#[derive(Component, Debug)]
pub struct TrickleColliderActive(pub bool);

pub struct ColliderStaticStub {
    pub uid: UId,
    pub points: Vec<IVec2>,
    pub active: bool,
    pub bounciness: f32,
    pub friction: f32,
}

#[derive(Component)]
pub struct ColliderStaticStubs(pub Vec<ColliderStaticStub>);

#[derive(Bundle)]
pub struct ColliderStaticBundle {
    _static: ColliderStatic,
    boundary: ColliderBoundary,
}

pub struct ColliderTriggerStub {
    pub uid: UId,
    pub refresh_period: u32,
    pub points: Vec<IVec2>,
    pub active: bool,
}

#[derive(Component)]
pub struct ColliderTriggerStubs(pub Vec<ColliderTriggerStub>);

#[derive(Bundle)]
pub struct ColliderTriggerBundle {
    pub trigger: ColliderTrigger,
    pub boundary: ColliderBoundary,
}

/// Materialize the collider stubs, creating actual colliders
pub(super) fn materialize_collider_stubs(
    mut commands: Commands,
    static_stubs: Query<(Entity, &ColliderStaticStubs)>,
    trigger_stubs: Query<(Entity, &ColliderTriggerStubs)>,
) {
    // Statics
    for (eid, stubs) in static_stubs.iter() {
        for stub in stubs.0.iter() {
            commands.entity(eid).with_children(|parent| {
                let mut res = parent.spawn((
                    UIdMarker(stub.uid),
                    ColliderStaticBundle {
                        _static: ColliderStatic {
                            bounciness: stub.bounciness,
                            friction: stub.friction,
                        },
                        boundary: ColliderBoundary::from_points(stub.points.clone()),
                    },
                ));
                if stub.active {
                    res.insert(ColliderActive);
                }
            });
        }
        commands.entity(eid).remove::<ColliderStaticStubs>();
    }
    // Triggers
    for (eid, stubs) in trigger_stubs.iter() {
        for stub in stubs.0.iter() {
            commands.entity(eid).with_children(|parent| {
                let mut res = parent.spawn((
                    UIdMarker(stub.uid),
                    ColliderTriggerBundle {
                        trigger: ColliderTrigger {
                            refresh_period: stub.refresh_period,
                        },
                        boundary: ColliderBoundary::from_points(stub.points.clone()),
                    },
                    Name::new("ColliderTrigger"),
                ));
                if stub.active {
                    res.insert(ColliderActive);
                }
            });
        }
        commands.entity(eid).remove::<ColliderTriggerStubs>();
    }
}

/// A helper function to resolve collisions between an IntDyno and a ColliderStatic
pub(super) fn resolve_static_collisions(
    dyno: &mut IntDyno,
    statics: &Query<(Entity, &ColliderBoundary, &ColliderStatic, &Parent), With<ColliderActive>>,
) -> bool {
    let mut fpos = dyno.fpos.truncate();
    let mut min_dist_sq: Option<f32> = None;
    let mut min_point: Option<Vec2> = None;
    let mut min_id: Option<Entity> = None;
    let mut min_parent_id: Option<Entity> = None;
    for (eid, boundary, _, parent) in statics.iter() {
        // We use bounding circles to cut down on the number of checks we actually have to do
        let prune_dist = fpos.distance_squared(boundary.center) - dyno.radius.powi(2) * 16.0;
        if prune_dist > boundary.bound_squared {
            continue;
        }
        let closest_point = boundary.closest_point(fpos);
        let dist_sq = fpos.distance_squared(closest_point);
        if min_dist_sq.is_none() || min_dist_sq.unwrap() > dist_sq {
            min_dist_sq = Some(dist_sq);
            min_point = Some(closest_point);
            min_id = Some(eid);
            min_parent_id = Some(parent.get());
        }
    }
    // Early exit when there's no collision
    if min_dist_sq.unwrap_or(f32::MAX) > dyno.radius.powi(2) {
        return false;
    }
    let (_, Some(min_point), Some(min_id), Some(min_parent_id)) =
        (min_dist_sq, min_point, min_id, min_parent_id)
    else {
        error!("Weird stuff happened in resolving static collisions...");
        return false;
    };
    let Ok((_, _, stat, _)) = statics.get(min_id) else {
        error!("Weird stuff2 happened in resolving static collisions...");
        return false;
    };

    let diff = fpos - min_point;
    let normal = diff.normalize_or_zero();
    if normal.dot(dyno.vel) >= 0.0 {
        return false;
    }

    let pure_parr = -1.0 * dyno.vel.dot(normal) * normal + dyno.vel;
    if dyno.statics.len() < MAX_COLLISIONS_PER_FRAME {
        dyno.statics.insert(
            min_parent_id,
            StaticCollision {
                pos: fpos,
                norm_vel: normal * normal.dot(dyno.vel),
                par_vel: pure_parr,
            },
        );
    }

    let new_vel =
        pure_parr * (1.0 - stat.friction) - 1.0 * dyno.vel.dot(normal) * normal * stat.bounciness;
    dyno.vel = new_vel;
    let diff = fpos - min_point;
    let normal = diff.normalize_or_zero();
    fpos += normal * (dyno.radius - fpos.distance(min_point));
    dyno.fpos.x = fpos.x;
    dyno.fpos.y = fpos.y;
    true
}

/// A helper function to resolve collisions between an IntDyno and a ColliderStatic
pub(super) fn resolve_trigger_collisions(
    dyno: &mut IntDyno,
    triggers: &Query<(&ColliderBoundary, &ColliderTrigger, &Parent), With<ColliderActive>>,
) {
    let fpos = dyno.fpos.truncate();
    for (boundary, _, parent) in triggers.iter() {
        // We use bounding circles to cut down on the number of checks we actually have to do
        let prune_dist = fpos.distance_squared(boundary.center) - dyno.radius.powi(2);
        if prune_dist > boundary.bound_squared {
            continue;
        }
        let em = boundary.effective_mult(dyno.fpos.truncate(), dyno.radius);
        if em < 0.001 {
            continue;
        }
        if dyno.triggers.len() < MAX_COLLISIONS_PER_FRAME {
            dyno.triggers.insert(parent.get(), em);
        }
    }
}

/// A function that trickles ColliderActive down to the child. Basically allows you to add ColliderActive
/// to the parent and have it auto-update the child
pub(super) fn trickle_active(
    parents: Query<&TrickleColliderActive>,
    colliders: Query<(Entity, &Parent), With<ColliderBoundary>>,
    mut commands: Commands,
) {
    for (cid, parent) in colliders.iter() {
        let Ok(trickle) = parents.get(parent.get()) else {
            continue;
        };
        if trickle.0 {
            commands.entity(cid).insert(ColliderActive);
        } else {
            commands.entity(cid).remove::<ColliderActive>();
        }
    }
}

pub fn update_triggers() {}
