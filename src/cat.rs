use std::f32::consts::PI;

use bevy::prelude::*;
use rand::RngExt;

use crate::crow::{CrowState, DesiredVelocity, Velocity};

const SPEED: f32 = 5.0;

#[derive(Component)]
struct Cat;

#[derive(Component)]
struct PouncedOn(Entity);

#[derive(Component)]
struct WalkTo(Vec3);

#[derive(Component)]
struct Boredom(Timer);

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (walk, experience_boredom, pounce));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat/cat.glb"))),
        Boredom(Timer::from_seconds(1., TimerMode::Repeating)),
        Transform::from_scale(Vec3::splat(0.6))
            .with_translation(Vec3::new(8., -0.5, -3.))
            .with_rotation(Quat::from_rotation_y(PI * 0.75)),
    ));
}

fn walk(mut commands: Commands, cats: Query<(Entity, &mut Transform, &WalkTo)>, time: Res<Time>) {
    for (entity, mut transform, walk_to) in cats {
        let offset = walk_to.0 - transform.translation;
        let distance = offset.length();
        if distance < 0.1 {
            commands.entity(entity).remove::<WalkTo>();
            continue;
        }
        let direction = offset / distance;
        let step = (SPEED * time.delta_secs()).min(distance);
        transform.translation += direction * step;
        transform.look_to(direction, Vec3::Y);
    }
}

fn experience_boredom(
    mut commands: Commands,
    cats: Query<(Entity, &mut Boredom), (Without<WalkTo>, Without<PouncedOn>)>,
    time: Res<Time>,
) {
    let mut rng = rand::rng();
    for (entity, mut boredom) in cats {
        boredom.0.tick(time.delta());
        if boredom.0.just_finished() && rng.random_bool(0.5) {
            let position = Vec3::new(
                rng.random_range(-1.5..5.5),
                -0.5,
                rng.random_range(-1.5..5.5),
            );
            commands.entity(entity).insert(WalkTo(position));
        }
    }
}

fn pounce(
    mut commands: Commands,
    cats: Query<(Entity, &Transform), (With<Cat>, Without<PouncedOn>)>,
    mut crows: Query<
        (
            Entity,
            &mut Transform,
            &mut CrowState,
            &mut Velocity,
            &mut DesiredVelocity,
        ),
        Without<Cat>,
    >,
) {
    for (cat_entity, cat_transform) in cats {
        for (crow_entity, mut crow_transform, mut state, mut velocity, mut desired) in &mut crows {
            if let CrowState::CapturedBy(_) = *state {
                continue;
            }
            if cat_transform
                .translation
                .distance(crow_transform.translation)
                > 1.0
            {
                continue;
            }
            commands
                .entity(cat_entity)
                .insert(PouncedOn(crow_entity))
                .remove::<WalkTo>();
            *state = CrowState::CapturedBy(cat_entity);
            crow_transform.translation =
                cat_transform.translation + (Vec3::Y + *cat_transform.forward()) * 0.6;
            crow_transform.rotate_local_z(PI / 2.);
            velocity.0 = Vec3::ZERO;
            desired.0 = Vec3::ZERO;
            break;
        }
    }
}
