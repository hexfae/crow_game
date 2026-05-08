use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use rand::RngExt;

use crate::{
    crow::{Carrying, Crow, CrowState, DesiredVelocity, InjuredTimer, Velocity},
    world::{Carryable, Mobbable},
};

const SPEED: f32 = 5.0;

#[derive(Component)]
struct Cat;

#[derive(Component)]
struct PouncedOn(Entity);

#[derive(Component)]
struct WalkTo(Vec3);

#[derive(Component)]
struct Boredom(Timer);

#[derive(Component)]
struct InjureCrowTimer(Timer);

#[derive(Component)]
struct Scared;

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                walk,
                experience_boredom,
                pounce,
                get_mobbed,
                injure_pounced_crow,
            )
                .chain(),
        );
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat/cat.glb"))),
        Boredom(Timer::from_seconds(1., TimerMode::Repeating)),
        Mobbable::minimum(4),
        Transform::from_scale(Vec3::splat(0.6))
            .with_translation(Vec3::new(5., -0.5, -5.))
            .with_rotation(Quat::from_rotation_y(PI * 0.75)),
    ));
}

fn walk(mut commands: Commands, cats: Query<(Entity, &mut Transform, &WalkTo)>, time: Res<Time>) {
    for (entity, mut transform, walk_to) in cats {
        let offset = walk_to.0 - transform.translation;
        let distance = offset.length();
        if distance < 0.1 {
            commands
                .entity(entity)
                .remove::<WalkTo>()
                .try_remove::<Scared>();
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
    cats: Query<(Entity, &Transform), (With<Cat>, Without<PouncedOn>, Without<Scared>)>,
    mut crows: Query<
        (
            Entity,
            &mut Transform,
            &mut CrowState,
            &mut Velocity,
            &mut DesiredVelocity,
        ),
        (Without<Cat>, Without<InjuredTimer>),
    >,
) {
    for (cat_entity, cat_transform) in cats {
        if crows
            .iter()
            .filter(|(_, transform, state, _, _)| {
                !matches!(state, CrowState::CapturedBy(_))
                    && transform.translation.distance(cat_transform.translation) < 6.
            })
            .count()
            >= 4
        {
            continue;
        }
        for (crow_entity, mut crow_transform, mut state, mut velocity, mut desired) in &mut crows {
            if let CrowState::CapturedBy(_) | CrowState::RecoveringFromInjury = *state {
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
                .insert((PouncedOn(crow_entity), InjureCrowTimer::default()))
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
    cats: Query<(Entity, &Transform, &mut Mobbable, &PouncedOn)>,
    crows: Query<(&Transform, &CrowState)>,
    carrying_query: Query<(), With<Carrying>>,
    time: Res<Time>,
) {
    for (cat_entity, cat_transform, mut mobbable, pounced_on) in cats {
        if crows
            .iter()
            .filter(|crow| {
                !matches!(crow.1, CrowState::CapturedBy(_))
                    && crow.0.translation.distance(cat_transform.translation) < 4.
            })
            .count()
            >= 4
        {
            mobbable.time += time.delta_secs();
        } else {
            mobbable.time = (mobbable.time - time.delta_secs()).max(0.);
        }
        if mobbable.time >= 2. {
            mobbable.time = 0.;
            let new_state = if carrying_query.contains(pounced_on.0) {
                CrowState::ReturningToRoost
            } else {
                CrowState::FollowLeader
            };
            commands.entity(pounced_on.0).insert(new_state);
            commands
                .entity(cat_entity)
                .remove::<PouncedOn>()
                .insert(Scared)
                .insert(WalkTo(Vec3::new(8., -0.5, -3.)));
        }
    }
}

fn injure_pounced_crow(
    mut commands: Commands,
    cats: Query<(Entity, &mut InjureCrowTimer, &PouncedOn), With<Cat>>,
    crows: Query<&Carrying, With<Crow>>,
    time: Res<Time>,
) {
    for (cat_entity, mut injure_crow_timer, pounced_on) in cats {
        injure_crow_timer.0.tick(time.delta());
        if injure_crow_timer.0.just_finished() {
            commands
                .entity(cat_entity)
                .remove::<(InjureCrowTimer, PouncedOn)>();
            commands
                .entity(pounced_on.0)
                .insert((InjuredTimer::default(), CrowState::ReturningToRoost))
                .try_remove::<Carrying>();
            if let Ok(carrying) = crows.get(pounced_on.0) {
                commands
                    .entity(carrying.0)
                    .remove_parent_in_place()
                    .insert(Carryable);
            }
        }
    }
}

impl Default for InjureCrowTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs(10), TimerMode::Once))
    }
}
