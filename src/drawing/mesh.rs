use bevy::{
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
    sprite::Mesh2dHandle,
};

use crate::meta::consts::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub fn points_to_mesh(points: &[Vec2], meshes: &mut ResMut<Assets<Mesh>>) -> Mesh2dHandle {
    let mut points_vec: Vec<f32> = vec![];
    let mut top_left = Vec2::new(f32::MAX, f32::MAX);
    let mut bot_right = Vec2::new(f32::MIN, f32::MIN);
    for point in points.iter() {
        points_vec.push(point.x);
        points_vec.push(point.y);
        top_left.x = top_left.x.min(point.x);
        top_left.y = top_left.y.min(point.y);
        bot_right.x = bot_right.x.max(point.x);
        bot_right.y = bot_right.y.max(point.y);
    }
    let verts: Vec<u32> = earcutr::earcut(&points_vec, &[], 2)
        .unwrap()
        .into_iter()
        .map(|val| val as u32)
        .collect();
    let mut triangle = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mut positions: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    for p in points.iter() {
        positions.push([p.x, p.y, 0.0]);
        normals.push([0.0, 0.0, 1.0]);
        let uv_x = (p.x - top_left.x) / (bot_right.x - top_left.x);
        // I'm only 80% confident this should be 1.0 -
        let uv_y = 1.0 - (p.y - top_left.y) / (bot_right.y - top_left.y);
        uvs.push([uv_x, uv_y]);
    }
    triangle.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    triangle.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    triangle.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    triangle.insert_indices(Indices::U32(verts));
    meshes.add(triangle).into()
}

/// Returns a mesh that covers the screen
pub fn generate_new_screen_mesh(meshes: &mut ResMut<Assets<Mesh>>) -> Mesh2dHandle {
    let x = SCREEN_WIDTH as f32 / 2.0;
    let y = SCREEN_HEIGHT as f32 / 2.0;
    let points = vec![
        Vec2::new(-x, -y),
        Vec2::new(-x, y),
        Vec2::new(x, y),
        Vec2::new(x, -y),
    ];
    points_to_mesh(&points, meshes)
}

/// Returns the smallest UVec 2 such that an aabb of that size could cover points
pub fn uvec2_bound(points: &Vec<Vec2>) -> UVec2 {
    let mut mins = Vec2::new(f32::MAX, f32::MAX);
    let mut maxs = Vec2::new(f32::MIN, f32::MIN);
    for vec in points {
        mins = mins.min(*vec);
        maxs = maxs.max(*vec);
    }
    UVec2 {
        x: (maxs.x - mins.x).ceil() as u32,
        y: (maxs.y - mins.y).ceil() as u32,
    }
}

/// Given a list of points, return points that retain the same shape, but produce an outline
pub fn outline_points(points: &Vec<Vec2>, width: f32) -> Vec<Vec2> {
    let mut new_points = vec![];
    for (ix, point) in points.iter().enumerate() {
        let point_before = points[if ix == 0 { points.len() - 1 } else { ix - 1 }];
        let point_after = points[if ix == points.len() - 1 { 0 } else { ix + 1 }];
        let slope_before = (*point - point_before).normalize_or_zero();
        let slope_after = (point_after - *point).normalize_or_zero();
        let tang = (slope_before + slope_after).normalize_or_zero();
        let perp = Vec2::new(-tang.y, tang.x);
        new_points.push(*point + perp * width);
    }
    new_points
}

/// Given a list of integer points, return points that retain the same shape, but produce an outline
pub fn ioutline_points(points: &Vec<IVec2>, width: f32) -> Vec<IVec2> {
    let fpoints: Vec<Vec2> = points.clone().into_iter().map(|p| p.as_vec2()).collect();
    let fres = outline_points(&fpoints, width);
    fres.into_iter()
        .map(|p| IVec2::new(p.x.round() as i32, p.y.round() as i32))
        .collect()
}
