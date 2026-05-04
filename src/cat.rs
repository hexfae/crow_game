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

#[derive(Default, Component)]
struct MobMeter(f32);

#[derive(Component)]
struct Scared;

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (walk, experience_boredom, pounce, get_mobbed));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat/cat.glb"))),
        Boredom(Timer::from_seconds(1., TimerMode::Repeating)),
        MobMeter::default(),
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
            commands
                .entity(entity)
                .insert(WalkTo(position))
                .try_remove::<Scared>();
        }
    }
}

fn pounce(
    mut commands: Commands,
    cats: Query<(Entity, &Transform), (With<Cat>, Without<PouncedOn>, Without<Scared>)>,
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
                > 1.
            {
                continue;
            }
            commands
                .entity(cat_entity)
                .insert(PouncedOn(crow_entity))
                .try_remove::<WalkTo>();
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

fn get_mobbed(
    mut commands: Commands,
    cats: Query<(Entity, &Transform, &mut MobMeter, &PouncedOn)>,
    crows: Query<(&Transform, &CrowState)>,
    time: Res<Time>,
) {
    for (cat_entity, cat_transform, mut mob_meter, pounced_on) in cats {
        if crows
            .iter()
            .filter(|crow| {
                !matches!(crow.1, CrowState::CapturedBy(_))
                    && crow.0.translation.distance(cat_transform.translation) < 4.
            })
            .count()
            >= 4
        {
            mob_meter.0 += time.delta_secs();
        } else {
            mob_meter.0 = (mob_meter.0 - time.delta_secs()).max(0.);
        }
        if mob_meter.0 >= 2. {
            mob_meter.0 = 0.;
            commands
                .entity(pounced_on.0)
                .insert(CrowState::FollowLeader);
            commands
                .entity(cat_entity)
                .remove::<PouncedOn>()
                .insert(Scared)
                .insert(WalkTo(Vec3::new(8., -0.5, -3.)));
        }
    }
}
