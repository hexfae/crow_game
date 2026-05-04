use std::f32::consts::PI;

use bevy::prelude::*;
use rand::RngExt;

#[derive(Component)]
pub struct Carryable;

#[derive(Component)]
pub struct Roost;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(7., 7.)))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(2., 0., 2.0),
    ));
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(1., 1., -1.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Roost,
        Mesh3d(meshes.add(Cuboid::new(2., 0.5, 2.))),
        MeshMaterial3d(materials.add(Color::srgb_u8(150, 75, 0))),
        Transform::from_xyz(-2.5, 0.25, -2.5),
    ));

    let mut rng = rand::rng();
    for _ in 0..8 {
        let position = Vec3::new(rng.random_range(-3.0..7.), 0., rng.random_range(-3.0..7.));
        let rotation = rng.random_range(0.0..PI * 2.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2., 0.2, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 255, 0))),
            Carryable,
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(rotation)),
        ));
    }
}
