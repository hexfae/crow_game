use std::f32::consts::PI;

use bevy::{camera::primitives::Aabb, prelude::*};
use rand::RngExt;

use crate::crow::Crow;

#[derive(Component)]
pub struct Carryable;

#[derive(Component)]
pub struct Mobbable {
    /// The minimum amount of crows to intimidate the attacker.
    pub minimum: usize,
    /// How long (in seconds) the attacker has been mobbed.
    pub time: f32,
}

#[derive(Component)]
pub struct Roost;

#[derive(Component)]
pub struct Cover;

#[derive(Component)]
pub struct UnderCover;

#[derive(Default, Resource)]
pub struct Score(pub u32);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, update_cover);
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
        Transform::from_xyz(-4.5, 0.25, -4.5),
    ));
    commands.spawn((
        Cover,
        Mesh3d(meshes.add(Cuboid::new(5., 0.5, 5.))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(-5., 8.0, -5.),
    ));

    let mut rng = rand::rng();
    for _ in 0..8 {
        let position = Vec3::new(rng.random_range(-5.0..5.), 0., rng.random_range(-5.0..5.));
        let rotation = rng.random_range(0.0..PI * 2.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2., 0.2, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 255, 0))),
            Carryable,
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(rotation)),
        ));
    }
}

fn update_cover(
    mut commands: Commands,
    crows: Query<(Entity, &Transform), With<Crow>>,
    covers: Query<(&GlobalTransform, &Aabb), With<Cover>>,
) {
    for (crow_entity, crow_transform) in &crows {
        let under = covers.iter().any(|(cover_xf, aabb)| {
            let center_xz = cover_xf.translation().xz() + Vec2::new(aabb.center.x, aabb.center.z);
            let he_xz = Vec2::new(aabb.half_extents.x, aabb.half_extents.z);
            let d = (crow_transform.translation.xz() - center_xz).abs();
            d.cmple(he_xz).all()
        });
        if under {
            commands.entity(crow_entity).insert(UnderCover);
        } else {
            commands.entity(crow_entity).try_remove::<UnderCover>();
        }
    }
}

impl Mobbable {
    pub fn minimum(minimum: usize) -> Self {
        Self { minimum, time: 0. }
    }
}
