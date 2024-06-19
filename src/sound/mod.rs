use bevy::prelude::*;
use effect::SoundEffectPlugin;

use self::music::MusicPlugin;

pub mod effect;
pub mod music;

#[derive(Debug, Resource)]
pub struct SoundSettings {
    main_volume: f32,
    effect_volume: f32,
    music_volume: f32,
}
impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            main_volume: 0.5,
            effect_volume: 0.5,
            music_volume: 0.5,
        }
    }
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SoundSettings::default());
        app.add_plugins(MusicPlugin);
        app.add_plugins(SoundEffectPlugin);
    }
}
