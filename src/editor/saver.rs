use super::{
    editable_goal::EditableGoal, editable_point::EditablePoint, editable_rock::EditableRock,
    editable_starting_point::EditableStartingPoint,
};
use crate::{
    environment::field::Field,
    meta::{
        game_state::in_editor,
        level_data::{get_level_folder, LevelData, SaveableField, SaveableRock},
    },
};
use bevy::prelude::*;
use std::{fs::File, io::Write};

fn watch_for_save(
    keys: Res<ButtonInput<KeyCode>>,
    epoints: Query<&Transform, With<EditablePoint>>,
    erocks: Query<(&EditableRock, &Transform)>,
    estart: Query<&Transform, With<EditableStartingPoint>>,
    egoal: Query<&Transform, With<EditableGoal>>,
) {
    if !keys.pressed(KeyCode::SuperLeft) || !keys.pressed(KeyCode::KeyS) {
        // Don't save
        return;
    }
    // Construct the save state
    let mut srocks: Vec<SaveableRock> = vec![];
    let mut sfields: Vec<SaveableField> = vec![];
    for (erock, tran) in erocks.iter() {
        if !erock.closed {
            continue;
        }
        let (rock, reach) = erock.to_rock_n_reach(&epoints, tran.translation.truncate());
        srocks.push(SaveableRock {
            points: rock
                .points
                .clone()
                .into_iter()
                .map(|p| p + tran.translation.truncate())
                .collect(),
            kind: erock.kind.clone(),
            reach,
        });
        if let Some(reach) = reach {
            let fields = Field::uniform_around_rock(&rock, reach, 0.06);
            for field in fields {
                sfields.push(SaveableField {
                    points: field
                        .points
                        .into_iter()
                        .map(|p| p + tran.translation.truncate())
                        .collect(),
                    strength: field.strength,
                    dir: field.dir,
                    drag: field.drag,
                })
            }
        }
    }
    let level_data = LevelData {
        next_level: None,
        starting_point: estart.single().translation.truncate(),
        goal_point: egoal.single().translation.truncate(),
        rocks: srocks,
        fields: sfields,
    };
    // Write it to a simple file
    let mut fout = File::create(get_level_folder().join("editing.zenith")).unwrap();
    write!(fout, "{}", serde_json::to_string(&level_data).unwrap()).unwrap();
}

pub fn register_saver(app: &mut App) {
    app.add_systems(Update, watch_for_save.run_if(in_editor));
}
