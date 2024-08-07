use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin},
};

pub const SPRITE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8267429772218888889);

#[derive(Default)]
pub struct AnimationMaterialPlugin;

impl Plugin for AnimationMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<AnimationMaterial>::default());
    }
}

#[derive(AsBindGroup, Debug, Clone, Asset, Reflect, PartialEq)]
pub struct AnimationMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    #[uniform(3)]
    pub index: f32,
    #[uniform(4)]
    pub length: f32,
    #[uniform(5)]
    pub x_offset: f32,
    #[uniform(6)]
    pub y_offset: f32,
    #[uniform(7)]
    pub x_repetitions: f32,
    #[uniform(8)]
    pub y_repetitions: f32,
    #[uniform(9)]
    pub r: f32,
    #[uniform(10)]
    pub g: f32,
    #[uniform(11)]
    pub b: f32,
    #[uniform(12)]
    pub rot: f32,
    pub ephemeral: bool,
}
impl AnimationMaterial {
    pub fn from_handle(
        handle: Handle<Image>,
        length: u32,
        repetitions: Vec2,
        color: Color,
    ) -> Self {
        Self {
            texture: handle,
            index: 0.0,
            length: length as f32,
            x_offset: 0.0,
            y_offset: 0.0,
            x_repetitions: repetitions.x,
            y_repetitions: repetitions.y,
            r: color.r(),
            g: color.g(),
            b: color.b(),
            rot: 0.0,
            ephemeral: false,
        }
    }
}

impl Material2d for AnimationMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/animation_mat.wgsl".into()
    }
}
