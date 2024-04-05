use std::{fs, ops::Deref};

use bevy::{
    ecs::{entity::EntityHashMap, system::SystemState},
    prelude::*,
    sprite::Mesh2dHandle,
    utils::HashSet,
};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use super::{
    help::HelpBarEvent,
    planet::EPlanet,
    start_goal::{EGoal, EStart},
    EditingSceneRoot,
};
use crate::{
    drawing::sprite_mat::SpriteMaterial,
    meta::game_state::{EditingMode, SetGameState},
};

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct SaveMarker;

#[derive(Event)]
pub(super) struct SaveEditorEvent;

#[derive(Event)]
pub(super) struct LoadEditorEvent;

fn unfuck_serialization(fucked: String) -> String {
    let mut result = String::new();
    let mut it = fucked.lines().into_iter();

    loop {
        let Some(line) = it.next() else {
            break;
        };
        let processing = line.contains("zenith::physics::dyno::IntMoveable");
        result.push_str(line);
        result.push('\n');

        if processing {
            let mut ihatemylife = vec![];
            for _ in 0..14 {
                ihatemylife.push(it.next().unwrap());
            }
            let x1 = ihatemylife[1].split(":").nth(1).unwrap().replace(",", "");
            let x1 = x1.trim();
            let x1 = x1.parse::<f32>().unwrap();
            let y1 = ihatemylife[2].split(":").nth(1).unwrap().replace(",", "");
            let y1 = y1.trim();
            let y1 = y1.parse::<f32>().unwrap();
            let x2 = ihatemylife[5].split(":").nth(1).unwrap().replace(",", "");
            let x2 = x2.trim();
            let x2 = x2.parse::<f32>().unwrap();
            let y2 = ihatemylife[6].split(":").nth(1).unwrap().replace(",", "");
            let y2 = y2.trim();
            let y2 = y2.parse::<f32>().unwrap();
            let z2 = ihatemylife[7].split(":").nth(1).unwrap().replace(",", "");
            let z2 = z2.trim();
            let z2 = z2.parse::<f32>().unwrap();
            let x3 = ihatemylife[10].split(":").nth(1).unwrap().replace(",", "");
            let x3 = x3.trim();
            let x3 = x3.parse::<f32>().unwrap();
            let y3 = ihatemylife[11].split(":").nth(1).unwrap().replace(",", "");
            let y3 = y3.trim();
            let y3 = y3.parse::<f32>().unwrap();
            result.push_str(&format!(
                r#"
    vel: ({}, {}),
    pos: ({}, {}, {}),
    rem: ({}, {}),
),
            "#,
                x1, y1, x2, y2, z2, x3, y3
            ))
        }

        let processing = line.contains("points: [")
            && !line.contains("rock")
            && !line.contains("wild")
            && !line.contains("field");
        while processing {
            let first = it.next().unwrap();
            if first.contains("]") {
                result.push_str(first);
                result.push('\n');
                break;
            }
            let x = it.next().unwrap().split(":").nth(1);
            let x = x.unwrap().replace(",", "");
            let x = x.trim();
            let x = x.parse::<f32>().unwrap();
            let y = it
                .next()
                .unwrap()
                .split(":")
                .nth(1)
                .unwrap()
                .replace(",", "");
            let y = y.trim();
            let y = y.parse::<f32>().unwrap();
            result.push_str(&format!("({}, {}),", x, y));
            it.next();
        }

        let processing =
            line.contains("size: (") || line.contains("bounds: (") || line.contains("Repeating((");
        if processing {
            let x = it.next().unwrap().split(":").nth(1);
            let x = x.unwrap().replace(",", "");
            let x = x.trim();
            let x = x.parse::<i32>().unwrap();
            let y = it
                .next()
                .unwrap()
                .split(":")
                .nth(1)
                .unwrap()
                .replace(",", "");
            let y = y.trim();
            let y = y.parse::<i32>().unwrap();
            result.push_str(&format!("{}, {},", x, y));
        }

        let processing =
            line.contains("vel: (") || line.contains("scroll: (") || line.contains("dir: (");
        if processing {
            let x = it.next().unwrap().split(":").nth(1);
            let x = x.unwrap().replace(",", "");
            let x = x.trim();
            let x = x.parse::<f32>().unwrap();
            let y = it
                .next()
                .unwrap()
                .split(":")
                .nth(1)
                .unwrap()
                .replace(",", "");
            let y = y.trim();
            let y = y.parse::<f32>().unwrap();
            result.push_str(&format!("{}, {},", x, y));
        }
        let processing = line.contains("offset: (");
        if processing {
            let x = it.next().unwrap().split(":").nth(1);
            let x = x.unwrap().replace(",", "");
            let x = x.trim();
            let x = x.parse::<f32>().unwrap();
            let y = it
                .next()
                .unwrap()
                .split(":")
                .nth(1)
                .unwrap()
                .replace(",", "");
            let y = y.trim();
            let y = y.parse::<f32>().unwrap();
            let z = it
                .next()
                .unwrap()
                .split(":")
                .nth(1)
                .unwrap()
                .replace(",", "");
            let z = z.trim();
            let z = z.parse::<f32>().unwrap();
            result.push_str(&format!("{}, {}, {}", x, y, z));
        }
    }

    result
}

#[derive(Resource, Default)]
pub struct FuckySceneResource(pub Option<Handle<DynamicScene>>);

pub(super) fn save_editor(
    world: &mut World,
    params: &mut SystemState<(
        EventReader<SaveEditorEvent>,
        Query<Entity, With<EditingSceneRoot>>,
        Query<&Children>,
        Query<&Parent>,
        Query<&SaveMarker>,
    )>,
) {
    let (mut events, eroot_q, _, parent_q, _) = params.get(world);
    if events.read().count() <= 0 {
        return;
    }
    let Ok(eroot) = eroot_q.get_single() else {
        return;
    };
    let old_parent = match parent_q.get(eroot) {
        Ok(old) => Some(old.get()),
        Err(_) => None,
    };

    // Remove the parent to avoid fuckery on load
    world.entity_mut(eroot).remove::<Parent>();

    let (_, _, children, _, save_marker) = params.get(world);
    let mut keep = HashSet::new();
    keep.insert(eroot);
    for id in children.iter_descendants(eroot) {
        if save_marker.get(id).is_ok() {
            keep.insert(id);
        }
    }
    let mut scene = DynamicSceneBuilder::from_world(&world)
        .deny_all_resources()
        .extract_entities(world.iter_entities().map(|entity| entity.id()))
        .build();
    scene
        .entities
        .retain(|entity| keep.contains(&entity.entity));

    let type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.deref();
    let serialized_scene = scene.serialize_ron(type_registry);

    match serialized_scene {
        Ok(text) => {
            fs::write("assets/test.scn.ron", text.clone()).expect("Unable to write file");
            let unfucked = unfuck_serialization(text);
            world.send_event(HelpBarEvent("Saved scene successfully".to_string()));
            fs::write("assets/test.scn.ron", unfucked.clone()).expect("Unable to write file");
        }
        Err(e) => {
            println!("{:?}", e);
            world.send_event(HelpBarEvent("Failed to save scene".to_string()));
        }
    }

    // Add the parent back to avoid fuckery in real time
    if let Some(old_parent) = old_parent {
        world.entity_mut(old_parent).add_child(eroot);
    }
}

pub(super) fn load_editor(
    world: &mut World,
    params: &mut SystemState<(
        EventReader<LoadEditorEvent>,
        Res<AssetServer>,
        ResMut<FuckySceneResource>,
        EventWriter<SetGameState>,
    )>,
) {
    let (mut loads, _, _, _) = params.get_mut(world);
    if loads.read().count() <= 0 {
        return;
    }
    let (_, asset_server, mut fucky_scene, mut set_gs) = params.get_mut(world);
    let scene_handle: Handle<DynamicScene> = asset_server.load("test.scn.ron");
    *fucky_scene = FuckySceneResource(Some(scene_handle));
    set_gs.send(SetGameState(EditingMode::Free.to_game_state()));
}

pub(super) fn fix_after_load(
    world: &mut World,
    params: &mut SystemState<(
        ResMut<FuckySceneResource>,
        ResMut<Assets<DynamicScene>>,
        Query<Entity, With<EditingSceneRoot>>,
    )>,
) {
    if thread_rng().gen_bool(0.1) {
        let (mut fucky_scene, mut scenes, root_q) = params.get_mut(world);
        let roots: Vec<Entity> = root_q.iter().collect();
        let Some(scene_handle) = fucky_scene.0.clone() else {
            return;
        };
        let Some(scene) = scenes.remove(scene_handle.id()) else {
            return;
        };
        *fucky_scene = FuckySceneResource(None);
        let mut entity_map = EntityHashMap::default();
        for root in roots {
            world.entity_mut(root).despawn_recursive();
        }
        scene.write_to_world(world, &mut entity_map).unwrap();
    }
}

#[derive(Event)]
pub struct CleanupLoadEvent;

pub(super) fn cleanup_load(
    mut commands: Commands,
    dynamic: Query<(Entity, &Handle<DynamicScene>)>,
    mut reader: EventReader<CleanupLoadEvent>,
) {
    if reader.read().count() > 0 {
        for thing in dynamic.iter() {
            commands.entity(thing.0).despawn_recursive();
        }
    }
}

pub(super) fn connect_parents(
    mut commands: Commands,
    eroot: Query<Entity, With<EditingSceneRoot>>,
    orphan_planets: Query<Entity, (With<EPlanet>, Without<Parent>)>,
    orphan_start: Query<Entity, (With<EStart>, Without<Parent>)>,
    orphan_goal: Query<Entity, (With<EGoal>, Without<Parent>)>,
) {
    let Ok(eroot) = eroot.get_single() else {
        return;
    };
    for id in orphan_planets
        .iter()
        .chain(orphan_start.iter())
        .chain(orphan_goal.iter())
    {
        commands.entity(eroot).push_children(&[id]);
    }
}
