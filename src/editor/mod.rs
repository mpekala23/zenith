pub mod draggable;
pub mod editable_goal;
pub mod editable_point;
pub mod editable_rock;
pub mod editable_starting_point;
pub mod saver;
pub mod state_machine;

use self::{
    draggable::register_draggables, editable_point::register_editable_points,
    editable_rock::register_editable_rocks, saver::register_saver,
    state_machine::register_editor_state_machine,
};
use crate::{
    meta::{
        game_state::{entered_editor, EditorState, GameState, MetaState},
        level_data::{get_level_folder, LevelData},
    },
    when_becomes_false, when_becomes_true,
};
use bevy::prelude::*;

fn is_editing_helper(gs: &GameState) -> bool {
    match gs.meta {
        MetaState::Editor(editor_state) => match editor_state {
            EditorState::Editing(_) => true,
            _ => false,
        },
        _ => false,
    }
}
pub fn is_editing(gs: Res<GameState>) -> bool {
    is_editing_helper(&gs)
}
when_becomes_true!(is_editing_helper, entered_editing);
when_becomes_false!(is_editing_helper, left_editing);

fn is_testing_helper(gs: &GameState) -> bool {
    match gs.meta {
        MetaState::Editor(editor_state) => match editor_state {
            EditorState::Testing => true,
            _ => false,
        },
        _ => false,
    }
}
pub fn is_testing(gs: Res<GameState>) -> bool {
    is_testing_helper(&gs)
}
when_becomes_true!(is_testing_helper, entered_testing);
when_becomes_false!(is_testing_helper, left_testing);

pub fn setup_editor(mut commands: Commands) {
    let level_data = LevelData::load(get_level_folder().join("editing.zenith"));
    if let Some(level_data) = level_data {
        level_data.load_editor(&mut commands);
    } else {
        let blank_level = LevelData::blank();
        blank_level.load_editor(&mut commands);
    };
}

pub fn register_editor(app: &mut App) {
    app.add_systems(Update, setup_editor.run_if(entered_editor));
    register_draggables(app);
    register_editable_points(app);
    register_editable_rocks(app);
    register_editor_state_machine(app);
    register_saver(app);
}
