use std::cmp::Ordering;

use bevy::{
    prelude::*,
    utils::hashbrown::{HashMap, HashSet},
};
use serde::{Deserialize, Serialize};

use crate::{
    drawing::{
        layering::sprite_layer_u8,
        mesh::outline_points,
        mesh_head::{
            BorderedMeshHead, BorderedMeshHeadStub, BorderedMeshHeadStubs, MeshHead, MeshHeadStub,
            MeshHeadStubs, MeshTextureKind,
        },
    },
    editor::point::EPointKind,
    environment::rock::RockKind,
    input::MouseState,
    math::MathLine,
    meta::game_state::{EditingMode, GameState, SetGameState},
    physics::dyno::IntMoveable,
    uid::{fresh_uid, UId, UIdMarker, UIdTranslator},
};

use super::{
    point::{EPoint, EPointBundle},
    save::SaveMarker,
};

#[derive(Component)]
pub(super) struct FeralEPoint;

#[derive(Component, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub(super) struct EPlanetField {
    pub field_points: Vec<UId>,
    pub mesh_uid: UId,
    dir: Vec2,
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub(super) struct EPlanet {
    pub rock_points: Vec<UId>,
    pub rock_kind: RockKind,
    pub bordered_mesh_uid: UId,
    pub wild_points: Vec<UId>,
    pub fields: Vec<EPlanetField>,
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub(super) struct PendingField {
    groups: Vec<u32>,
}

#[derive(Bundle, Default)]
pub(super) struct EPlanetBundle {
    uid: UIdMarker,
    eplanet: EPlanet,
    spatial: SpatialBundle,
    moveable: IntMoveable,
    save: SaveMarker,
}
impl EPlanetBundle {
    pub fn new(pos: IVec2) -> (Self, impl Bundle) {
        let core_uid = fresh_uid();
        let bordered_mesh_uid = fresh_uid();
        let bordered_mesh_head_stubs = BorderedMeshHeadStubs(vec![BorderedMeshHeadStub {
            uid: bordered_mesh_uid,
            head: BorderedMeshHead {
                inner_path: "textures/play_inner.png".to_string(),
                inner_size: UVec2::new(36, 36),
                outer_path: "textures/play_outer.png".to_string(),
                outer_size: UVec2::new(36, 36),
                render_layers: vec![sprite_layer_u8()],
                border_width: 7.0,
                ..default()
            },
        }]);
        let bund = EPlanetBundle {
            uid: UIdMarker(core_uid),
            eplanet: EPlanet {
                bordered_mesh_uid,
                ..default()
            },
            spatial: SpatialBundle::from_transform(Transform::from_translation(
                pos.as_vec2().extend(0.0),
            )),
            moveable: IntMoveable::new(pos.extend(0)),
            save: SaveMarker,
        };
        (bund, bordered_mesh_head_stubs)
    }
}

/// Handle transitions between free, creating, editing
/// NOTE: Except the creating -> editing transition, that's handled in spawn points
pub(super) fn planet_state_input(
    mut commands: Commands,
    gs: Res<GameState>,
    mut gs_writer: EventWriter<SetGameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_state: Res<MouseState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    points: Query<(&EPoint, &Parent)>,
    eplanets: Query<Entity, With<EPlanet>>,
) {
    let Some(mode) = gs.get_editing_mode() else {
        return;
    };
    let num_points_hovered = points.iter().filter(|(p, _)| p.is_hovered).count();
    match mode {
        EditingMode::Free => {
            if keyboard.just_pressed(KeyCode::KeyP) {
                let bund = EPlanetBundle::new(mouse_state.world_pos);
                let id = commands.spawn(bund).id();
                gs_writer.send(SetGameState(
                    EditingMode::CreatingPlanet(id).to_game_state(),
                ));
            } else if mouse_buttons.just_pressed(MouseButton::Left) {
                for (point, parent) in points.iter() {
                    if point.is_hovered && eplanets.get(parent.get()).is_ok() {
                        gs_writer.send(SetGameState(
                            EditingMode::EditingPlanet(parent.get()).to_game_state(),
                        ));
                        return;
                    }
                }
            }
        }
        EditingMode::CreatingPlanet(_) | EditingMode::EditingPlanet(_) => {
            if mouse_buttons.just_pressed(MouseButton::Left) {
                if num_points_hovered == 0 {
                    gs_writer.send(SetGameState(EditingMode::Free.to_game_state()));
                }
            }
        }
    }
}

/// On cmd+f, redo the field of the active planet
/// NOTE: Must be editing the planet
pub(super) fn redo_fields(
    mut commands: Commands,
    mut eplanets: Query<&mut EPlanet>,
    points: Query<(Entity, &IntMoveable, &mut EPoint, &UIdMarker)>,
    gs: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    ut: Res<UIdTranslator>,
) {
    let planet_id = match gs.get_editing_mode() {
        Some(EditingMode::EditingPlanet(id)) => id,
        _ => return,
    };
    if !((keyboard.pressed(KeyCode::SuperLeft) && keyboard.just_pressed(KeyCode::KeyF))
        || (keyboard.just_pressed(KeyCode::SuperLeft) && keyboard.pressed(KeyCode::KeyF)))
    {
        return;
    }
    // Despawn and clear the old fields
    let mut eplanet = eplanets.get_mut(planet_id).unwrap();
    let mut despawned = HashSet::new();
    for field in eplanet.fields.iter() {
        for id in field.field_points.iter() {
            if eplanet.rock_points.contains(id) {
                continue;
            }
            if despawned.contains(id) {
                continue;
            }
            commands
                .entity(ut.get_entity(*id).unwrap())
                .despawn_recursive();
            despawned.insert(*id);
        }
        commands
            .entity(ut.get_entity(field.mesh_uid).unwrap())
            .despawn_recursive();
    }
    eplanet.fields.clear();
    // Make the new points
    let poses: Vec<Vec2> = eplanet
        .rock_points
        .iter()
        .map(|id| {
            points
                .get(ut.get_entity(*id).unwrap())
                .unwrap()
                .1
                .pos
                .truncate()
                .as_vec2()
        })
        .collect();
    let new_poses: Vec<IVec2> = outline_points(&poses, 40.0)
        .into_iter()
        .map(|pos| IVec2::new(pos.x.round() as i32, pos.y.round() as i32))
        .collect();

    for ix in 0..new_poses.len() {
        let next_ix = (ix + 1).rem_euclid(new_poses.len());
        let fp = new_poses[ix];
        let parent = points
            .get(ut.get_entity(eplanet.rock_points[ix]).unwrap())
            .unwrap();
        let mut fp_id = Entity::PLACEHOLDER;

        commands.entity(planet_id).with_children(|parent| {
            let bund = EPointBundle::new(fp, EPointKind::Wild);
            fp_id = parent.spawn(bund).id();
        });
        commands.entity(fp_id).insert(PendingField {
            groups: vec![ix as u32, next_ix as u32],
        });
        commands.entity(parent.0).insert(PendingField {
            groups: vec![ix as u32, next_ix as u32],
        });
    }
}

/// To make new fields, you mark all the points that should be part of that field
/// on the editing rock with Some([vec of u32s for which fields it's a part of])
/// This function goes in and resolves all that data, spitting out new fields
/// on the active editing rock as it goes
pub(super) fn resolve_pending_fields(
    mut commands: Commands,
    gs: Res<GameState>,
    mut eplanets: Query<(&mut EPlanet, &IntMoveable)>,
    stable_points: Query<&IntMoveable, Without<PendingField>>,
    mut points: Query<
        (
            Entity,
            &mut EPoint,
            &mut IntMoveable,
            &PendingField,
            &Parent,
            &UIdMarker,
        ),
        Without<EPlanet>,
    >,
    ut: Res<UIdTranslator>,
) {
    let planet_id = match gs.get_editing_mode() {
        Some(EditingMode::EditingPlanet(id)) => id,
        _ => return,
    };
    let Ok(mut eplanet) = eplanets.get_mut(planet_id) else {
        return;
    };
    // Wait until every point is in the ut
    for point in points.iter() {
        if ut.get_entity(point.5 .0).is_none() {
            return;
        }
    }

    // Construct the groupmap (fields)
    let mut rock_points = HashMap::new();
    let mut group_map = HashMap::<u32, Vec<(Entity, EPoint, IntMoveable, Entity, UId)>>::new();
    for (id, epoint, mv, pf, parent, eid) in points.iter() {
        for group in pf.groups.iter() {
            if group_map.contains_key(group) {
                let existing = group_map.get_mut(group).unwrap();
                existing.push((id, epoint.clone(), mv.clone(), parent.get(), eid.0));
            } else {
                group_map.insert(
                    *group,
                    vec![(id, epoint.clone(), mv.clone(), parent.get(), eid.0)],
                );
            }
        }
        if epoint.kind == EPointKind::Rock {
            rock_points.insert(id, mv.pos.truncate());
        }
    }

    // Spawn a mesh for each field, and add it to the planet
    let mut field_mesh_stubs: Vec<MeshHeadStub> = vec![];
    for (_, items) in group_map.into_iter() {
        let mut points_n_ids = vec![];
        let mut center = Vec2::ZERO;
        let to_ang = |thing: Vec2| (thing.x as f32).atan2(thing.y as f32);
        for (_id, epoint, mv, parent, uid) in items {
            match epoint.kind {
                EPointKind::Rock | EPointKind::Wild => {
                    let pos = mv.pos.truncate();
                    points_n_ids.push((uid, pos));
                    center += mv.pos.truncate().as_vec2();
                }
                EPointKind::Field => {
                    let mv = match points.get(parent) {
                        Ok(thing) => thing.2,
                        Err(_) => stable_points.get(parent).unwrap(),
                    };
                    let pos = mv.pos.truncate();
                    points_n_ids.push((uid, pos));
                    center += mv.pos.truncate().as_vec2();
                }
            }
        }
        center /= points_n_ids.len() as f32;
        points_n_ids.sort_by(|a, b| {
            let a_ang = to_ang(a.1.as_vec2() - center);
            let b_ang = to_ang(b.1.as_vec2() - center);
            if a_ang < b_ang {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        let mesh_points = points_n_ids
            .clone()
            .into_iter()
            .map(|thing| thing.1)
            .collect();
        let id_order: Vec<UId> = points_n_ids
            .clone()
            .into_iter()
            .map(|thing| thing.0)
            .collect();
        commands
            .entity(ut.get_entity(id_order[0]).unwrap())
            .insert(UpdateFieldGravity); // Triggers the field's gravity to update on the next frame

        let mesh_uid = fresh_uid();
        field_mesh_stubs.push(MeshHeadStub {
            uid: mesh_uid,
            head: MeshHead {
                path: "sprites/field/field_bg.png".to_string(),
                points: mesh_points,
                render_layers: vec![sprite_layer_u8()],
                texture_kind: MeshTextureKind::Repeating(UVec2::new(12, 12)),
                offset: Vec3::new(0.0, 0.0, -1.0),
                ..default()
            },
        });
        let field = EPlanetField {
            field_points: id_order,
            mesh_uid,
            dir: Vec2::ZERO,
        };
        eplanet.0.fields.push(field);
    }
    commands
        .entity(planet_id)
        .insert(MeshHeadStubs(field_mesh_stubs));

    // Cleanup all the groups and turn wild points into field points
    for (id, mut point, mut mv, _, parent, eid) in points.iter_mut() {
        commands.entity(id).remove::<PendingField>();
        if point.kind == EPointKind::Wild {
            let mut min_dist = i32::MAX;
            let mut new_dad = planet_id;
            let mut dad_pos = IVec2::ZERO;
            for (id, pos) in rock_points.iter() {
                let dist = mv.pos.truncate().distance_squared(*pos);
                if dist < min_dist {
                    min_dist = dist;
                    new_dad = *id;
                    dad_pos = *pos;
                }
            }
            // adoption????!@??!??
            commands.entity(parent.get()).remove_children(&[id]);
            commands.entity(new_dad).push_children(&[id]);
            point.kind = EPointKind::Field;
            eplanet.0.wild_points.retain(|p| *p != eid.0);
            let old_pos = mv.pos;
            mv.pos = old_pos - dad_pos.extend(0);
        }
    }
}

/// On cmd + / cmd -, nudge all field points closer/further from their parent
pub(super) fn nudge_fields(
    eplanets: Query<&EPlanet>,
    mut points: Query<(Entity, &EPoint, &mut IntMoveable, &UIdMarker)>,
    gs: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    ut: Res<UIdTranslator>,
) {
    if !keyboard.pressed(KeyCode::SuperLeft)
        || (!keyboard.pressed(KeyCode::Comma) && !keyboard.pressed(KeyCode::Period))
    {
        return;
    }
    let Some(EditingMode::EditingPlanet(planet_id)) = gs.get_editing_mode() else {
        return;
    };
    let Ok(eplanet) = eplanets.get(planet_id) else {
        return;
    };
    for field in eplanet.fields.iter() {
        for id in field.field_points.iter() {
            let ent = ut.get_entity(*id).unwrap();
            let Ok((_, point, mut mv, _)) = points.get_mut(ent) else {
                continue;
            };
            if point.kind != EPointKind::Field {
                continue;
            }
            let change = mv.pos.as_vec3().truncate().normalize();
            let mult = if keyboard.pressed(KeyCode::Period) {
                1.0
            } else {
                -1.0
            };
            let hmm = mv.pos.truncate().as_vec2() + mv.rem;
            if hmm.length_squared() < 16.0 {
                if mult < 0.0 {
                    continue;
                }
            }
            mv.rem += mult * change;
        }
    }
}

/// If multiple points from the same field are selected, delete that field
/// Turns points into wild points if needed
pub(super) fn remove_field(
    mut eplanets: Query<&mut EPlanet>,
    points: Query<(Entity, &EPoint, &UIdMarker)>,
    gs: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ut: Res<UIdTranslator>,
) {
    if !keyboard.just_pressed(KeyCode::KeyC) {
        return;
    }
    let Some(EditingMode::EditingPlanet(planet_id)) = gs.get_editing_mode() else {
        return;
    };
    let Ok(mut eplanet) = eplanets.get_mut(planet_id) else {
        return;
    };
    let selected_ids: Vec<UId> = points
        .iter()
        .filter(|p| p.1.is_selected)
        .map(|p| p.2 .0)
        .collect();
    let mut maybe_feral = HashSet::new();
    for field in eplanet.fields.iter_mut() {
        if selected_ids.iter().all(|p| field.field_points.contains(p)) {
            if let Some(eid) = ut.get_entity(field.mesh_uid) {
                commands.entity(eid).despawn_recursive();
            }
            for id in field.field_points.iter() {
                maybe_feral.insert(*id);
            }
            // hackety hack
            field.mesh_uid = 0;
        }
    }
    eplanet.fields.retain(|field| field.mesh_uid != 0);
    for id in maybe_feral.into_iter() {
        let point = points.get(ut.get_entity(id).unwrap()).unwrap();
        if point.1.kind != EPointKind::Field {
            continue;
        }
        if !eplanet
            .fields
            .iter()
            .any(|field| field.field_points.contains(&id))
        {
            commands
                .entity(ut.get_entity(id).unwrap())
                .insert(FeralEPoint);
        }
    }
}

/// Remove any fields with less than two points OR no field points, turning everyhting non-rock wild
pub(super) fn cleanup_degen_fields(
    mut commands: Commands,
    mut eplanets: Query<&mut EPlanet>,
    points: Query<(Entity, &EPoint)>,
    gs: Res<GameState>,
    ut: Res<UIdTranslator>,
) {
    let Some(EditingMode::EditingPlanet(planet_id)) = gs.get_editing_mode() else {
        return;
    };
    let Ok(mut eplanet) = eplanets.get_mut(planet_id) else {
        return;
    };
    let should_purge_field = |field: &EPlanetField| {
        field.field_points.len() < 3
            || !field.field_points.iter().any(|p| {
                let Some(eid) = ut.get_entity(*p) else {
                    return false;
                };
                let Ok(point) = points.get(eid) else {
                    return false;
                };
                point.1.kind == EPointKind::Field
            })
    };
    let mut purged_fields = vec![];
    for field in eplanet.fields.iter() {
        if should_purge_field(field) {
            purged_fields.push(field.clone());
        }
    }
    eplanet.fields.retain(|f| !should_purge_field(f));
    for field in purged_fields {
        if let Some(eid) = ut.get_entity(field.mesh_uid) {
            commands.entity(eid).despawn_recursive();
        }
        for uid in field.field_points {
            let Some(eid) = ut.get_entity(uid) else {
                continue;
            };
            if let Ok(point) = points.get(eid) {
                if point.1.kind == EPointKind::Field {
                    commands.entity(eid).insert(FeralEPoint);
                }
            }
        }
    }
}

/// Actually makes old field points wild
pub(super) fn handle_feral_points(
    stable: Query<(&IntMoveable, &Parent), (Without<FeralEPoint>, With<EPoint>)>,
    mut points: Query<(Entity, &mut EPoint, &mut IntMoveable, &Parent), With<FeralEPoint>>,
    mut commands: Commands,
) {
    for mut feral_point in points.iter_mut() {
        let parent = stable.get(feral_point.3.get()).unwrap();
        feral_point.2.pos += parent.0.pos;
        feral_point.1.kind = EPointKind::Wild;
        commands
            .entity(feral_point.3.get())
            .remove_children(&[feral_point.0]);
        commands
            .entity(parent.1.get())
            .push_children(&[feral_point.0]);
        commands.entity(feral_point.0).remove::<FeralEPoint>();
    }
}

/// Makes a new field from the selected points
pub(super) fn make_new_field(
    points: Query<(Entity, &EPoint)>,
    gs: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let Some(_) = gs.get_editing_mode() else {
        return;
    };
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let selected_info: Vec<(Entity, EPoint)> = points
        .iter()
        .filter(|p| p.1.is_selected)
        .map(|thing| (thing.0, thing.1.clone()))
        .collect();
    let valid = selected_info
        .iter()
        .any(|p| p.1.kind == EPointKind::Wild || p.1.kind == EPointKind::Field)
        && selected_info.iter().any(|p| p.1.kind == EPointKind::Rock);
    if !valid {
        return;
    }
    for (id, _) in selected_info {
        commands.entity(id).insert(PendingField { groups: vec![0] });
    }
}

/// Any point in a field that has this component should update it's gravity
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub(super) struct UpdateFieldGravity;

/// Updates the gravity direction of fields.
/// Fields with 0 rock points - do nothing
/// Fields with 1 rock point - get average of field points, direct towards rock point
/// Fields with 2 rock points - perpendicular, assuming clockwise like normal
/// Fields with 3+ rock points - get line of best fit. Then pick perp dir using center of field points
pub(super) fn update_field_gravity(
    mut eplanets: Query<&mut EPlanet>,
    points: Query<(
        Entity,
        &EPoint,
        &IntMoveable,
        Option<&UpdateFieldGravity>,
        &Parent,
        &UIdMarker,
    )>,
    gs: Res<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    ut: Res<UIdTranslator>,
) {
    let Some(EditingMode::EditingPlanet(planet_id)) = gs.get_editing_mode() else {
        return;
    };
    let Ok(mut eplanet) = eplanets.get_mut(planet_id) else {
        return;
    };
    // First do the input
    if keyboard.just_pressed(KeyCode::KeyG) {
        if keyboard.pressed(KeyCode::ShiftLeft) {
            for field in eplanet.fields.iter() {
                commands
                    .entity(ut.get_entity(field.field_points[0]).unwrap())
                    .insert(UpdateFieldGravity);
            }
        } else {
            for point in points.iter() {
                if point.1.is_selected {
                    commands.entity(point.0).insert(UpdateFieldGravity);
                }
            }
        }
    }

    // Then actually do the updating
    let mut affected_points = HashSet::<UId>::new();
    for point in points.iter() {
        if point.3.is_some() {
            affected_points.insert(point.5 .0);
        }
    }
    for field in eplanet.fields.iter_mut() {
        let adjusting = field
            .field_points
            .iter()
            .any(|id| affected_points.contains(id));
        if !adjusting {
            continue;
        }
        for id in field.field_points.iter() {
            commands
                .entity(ut.get_entity(*id).unwrap())
                .remove::<UpdateFieldGravity>();
        }
        let mut rock_points = vec![];
        let mut field_points = vec![];
        for point in field.field_points.iter() {
            let point = points.get(ut.get_entity(*point).unwrap()).unwrap();
            if point.1.kind == EPointKind::Rock {
                rock_points.push((point.2.pos.truncate()).as_vec2());
            } else {
                let parent = points.get(point.4.get()).unwrap();
                field_points.push((parent.2.pos.truncate() + point.2.pos.truncate()).as_vec2());
            }
        }
        if rock_points.len() == 0 {
            continue;
        }
        let field_center = field_points
            .clone()
            .into_iter()
            .reduce(|acc, e| acc + e)
            .unwrap()
            / field_points.len() as f32;
        let rock_center = rock_points
            .clone()
            .into_iter()
            .reduce(|acc, e| acc + e)
            .unwrap()
            / rock_points.len() as f32;
        if rock_points.len() == 1 {
            field.dir = (rock_points[0] - field_center).normalize_or_zero();
        } else if rock_points.len() == 2 {
            let pall = rock_points[1] - rock_points[0];
            let perp = Vec2::new(-pall.y, pall.x);
            field.dir = perp.normalize_or_zero();
            let check_diff = rock_center - field_center;
            if field.dir.dot(check_diff) < 0.0 {
                field.dir *= -1.0;
            }
        } else {
            let line = MathLine::slope_fit_points(&rock_points);
            let perp = Vec2::new(line.rise(), -line.run());
            field.dir = perp.normalize_or_zero();
            let check_diff = rock_center - field_center;
            if field.dir.dot(check_diff) < 0.0 {
                field.dir *= -1.0;
            }
        }
    }
}

pub(super) fn drive_planet_meshes(
    points: Query<(Entity, &UIdMarker, &EPoint, &IntMoveable, &Parent)>,
    eplanets: Query<&EPlanet>,
    mut bordered_mesh_heads: Query<&mut BorderedMeshHead>,
    mut mesh_heads: Query<&mut MeshHead>,
    ut: Res<UIdTranslator>,
) {
    for eplanet in eplanets.iter() {
        // First update the rock mesh
        let mut mesh_points = vec![];
        for pid in eplanet.rock_points.iter() {
            let Some(ent) = ut.get_entity(*pid) else {
                continue;
            };
            if let Ok((_, _, _, mv, _)) = points.get(ent) {
                mesh_points.push(mv.pos.truncate());
            }
        }
        if let Some(id) = ut.get_entity(eplanet.bordered_mesh_uid) {
            if let Ok(mut head) = bordered_mesh_heads.get_mut(id) {
                head.points = mesh_points;
            }
        }

        // Then update each of the field meshes
        for field in eplanet.fields.iter() {
            let mut mesh_points = vec![];
            for pid in field.field_points.iter() {
                let Some(ent) = ut.get_entity(*pid) else {
                    continue;
                };
                if let Ok((_, _, epoint, mv, parent)) = points.get(ent) {
                    match epoint.kind {
                        EPointKind::Rock => {
                            mesh_points.push(mv.pos.truncate());
                        }
                        EPointKind::Field => {
                            let (_, _, _, parent_mv, _) = points.get(parent.get()).unwrap();
                            mesh_points.push(parent_mv.pos.truncate() + mv.pos.truncate());
                        }
                        EPointKind::Wild => (),
                    }
                }
            }
            if let Some(eid) = ut.get_entity(field.mesh_uid) {
                if let Ok(mut head) = mesh_heads.get_mut(eid) {
                    head.points = mesh_points;
                    head.scroll = field.dir / 4.0;
                }
            }
        }
    }
}

pub(super) fn draw_field_parents(
    mut gzs: Gizmos,
    points: Query<(&EPoint, &GlobalTransform, &Parent)>,
) {
    for (epoint, gt, parent) in points.iter() {
        if epoint.kind == EPointKind::Field {
            let Ok((_, pgt, _)) = points.get(parent.get()) else {
                continue;
            };
            gzs.line_2d(
                gt.translation().truncate(),
                pgt.translation().truncate(),
                Color::WHITE,
            );
        }
    }
}
