use super::menu_asset::MenuAssetComponent;
use crate::{
    // environment::background::{HyperSpace, BASE_TITLE_HYPERSPACE_SPEED, MAX_HYPERSPACE_SPEED},
    drawing::layering::{bg_light_layer, bg_sprite_layer},
    environment::background::{BgOffset, BgOffsetSpleen, PlacedBgBundle},
    math::Spleen,
    meta::{
        consts::TuneableConsts,
        game_state::{GameState, MenuState, MetaState, SetGameState},
    },
    when_becomes_false,
    when_becomes_true,
};
use bevy::{prelude::*, render::view::RenderLayers};
use rand::{thread_rng, Rng};

const TITLE_SCREEN_RON_PATH: &'static str = "menus/title_screen.ron";

#[derive(Component)]
struct TitleScreenDeath {
    pub timer: Timer,
}

#[derive(Component)]
pub struct ColorMarker(pub Color);

#[derive(Bundle)]
struct TitleBgStarBundle {
    placement: PlacedBgBundle,
    sprite: SpriteBundle,
    color: ColorMarker,
    layers: RenderLayers,
}

fn setup_title_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tune: Res<TuneableConsts>,
) {
    MenuAssetComponent::spawn(
        &asset_server,
        &mut commands,
        TITLE_SCREEN_RON_PATH.to_string(),
    );
    let num_stars = tune.get_or("title_screen_num_stars", 0.0) as i32;
    let depth_min = tune.get_or("title_screen_depth_min", 0.0) as i32;
    let depth_max = (tune.get_or("title_screen_depth_max", 1.0) as i32).max(depth_min + 1);
    let scale_min = tune.get_or("title_screen_scale_min", 0.0);
    let scale_max = tune.get_or("title_screen_scale_max", 1.0);
    let mut rng = thread_rng();
    for _ in 0..num_stars {
        let depth: u8 = rng.gen_range(depth_min..depth_max) as u8;
        let frac_pos = Vec2 {
            x: -0.5 + rng.gen::<f32>(),
            y: -0.5 + rng.gen::<f32>(),
        };
        let scale = scale_min + rng.gen::<f32>() * (scale_max - scale_min);
        let mut placement = PlacedBgBundle::basic_stationary(&tune, depth, frac_pos, scale);
        placement.offset.vel = Vec2::new(2.0, 0.2);
        let color = Color::hsla(rng.gen::<f32>() * 360.0, 0.8, 0.4, 1.0);
        let sprite = SpriteBundle {
            texture: asset_server.load("sprites/stars/7a.png"),
            sprite: Sprite { color, ..default() },
            ..default()
        };
        let sprite_l = SpriteBundle {
            texture: asset_server.load("sprites/stars/7aL.png"),
            ..default()
        };
        commands.spawn(TitleBgStarBundle {
            placement: placement.clone(),
            sprite,
            layers: bg_sprite_layer(),
            color: ColorMarker(color),
        });
        commands.spawn(TitleBgStarBundle {
            placement,
            sprite: sprite_l,
            layers: bg_light_layer(),
            color: ColorMarker(color),
        });
    }
}

fn update_title_screen(
    mut commands: Commands,
    mut death: Query<(Entity, &mut TitleScreenDeath)>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut gs_writer: EventWriter<SetGameState>,
    bgs: Query<(Entity, &BgOffset)>,
) {
    let transition_time = 0.75;
    if keys.is_changed() && !keys.is_added() && death.iter().len() == 0 {
        for (id, offset) in bgs.iter() {
            commands.entity(id).remove::<BgOffsetSpleen>();
            commands.entity(id).insert(BgOffsetSpleen {
                vel_start: offset.vel,
                vel_goal: Vec2::new(2.0, 0.2) * 2_010.4,
                timer: Timer::from_seconds(transition_time, TimerMode::Once),
                spleen: Spleen::EaseInQuintic,
            });
        }
        commands.spawn(TitleScreenDeath {
            timer: Timer::from_seconds(transition_time + 0.25, TimerMode::Once),
        });
    }
    let Ok((id, mut death)) = death.get_single_mut() else {
        return;
    };
    death.timer.tick(time.delta());
    if death.timer.finished() {
        for (id, offset) in bgs.iter() {
            commands.entity(id).remove::<BgOffsetSpleen>();
            commands.entity(id).insert(BgOffsetSpleen {
                vel_start: offset.vel,
                vel_goal: Vec2::ZERO,
                timer: Timer::from_seconds(transition_time, TimerMode::Once),
                spleen: Spleen::EaseOutQuintic,
            });
        }
        commands.entity(id).despawn_recursive();
        gs_writer.send(SetGameState(GameState {
            meta: MetaState::Menu(MenuState::ConstellationSelect),
        }));
    }
}

fn destroy_title_screen(mut commands: Commands, mac: Query<(Entity, &MenuAssetComponent)>) {
    for (id, mac) in mac.iter() {
        if mac.path != TITLE_SCREEN_RON_PATH.to_string() {
            continue;
        }
        commands.entity(id).despawn_recursive();
        for curse in mac.cursed_children.iter() {
            commands.entity(*curse).despawn_recursive();
        }
    }
}

fn is_in_title_screen_helper(gs: &GameState) -> bool {
    match gs.meta {
        MetaState::Menu(menu_state) => match menu_state {
            MenuState::Title => true,
            _ => false,
        },
        _ => false,
    }
}
fn is_in_title_screen(gs: Res<GameState>) -> bool {
    is_in_title_screen_helper(&gs)
}
when_becomes_true!(is_in_title_screen_helper, entered_title_screen);
when_becomes_false!(is_in_title_screen_helper, left_title_screen);

pub fn register_title_screen(app: &mut App) {
    app.add_systems(Update, setup_title_screen.run_if(entered_title_screen));
    app.add_systems(Update, destroy_title_screen.run_if(left_title_screen));
    app.add_systems(Update, update_title_screen.run_if(is_in_title_screen));
}
