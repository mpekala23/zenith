pub mod background;
pub mod field;
pub mod goal;
pub mod particle;
pub mod rock;
pub mod start;

use self::particle::register_particles;
use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        register_particles(app);
    }
}
