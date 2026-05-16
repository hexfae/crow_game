use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;
use rand::RngExt;

use crate::{
    audio::{PlaySfx, Sfx},
    crow::{Carrying, Crow, CrowState, DesiredVelocity, InjuredTimer, Species, Velocity},
    particles::{Particle, SpawnParticles},
    world::{Carryable, MissionPhase, Mobbable},
};

const CAT_HOME: Vec3 = Vec3::new(5., 0., -5.);

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
        app.add_systems(Startup, setup)
            .add_systems(OnExit(MissionPhase::Results), reset_cat)
            .add_systems(
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
        Mobbable::minimum(4.),
        Transform::from_scale(Vec3::splat(0.6))
            .with_translation(CAT_HOME)
            .with_rotation(Quat::from_rotation_y(PI * 0.75)),
    ));
}

fn reset_cat(
    mut commands: Commands,
    cats: Query<(Entity, &mut Transform, &mut Mobbable), With<Cat>>,
) {
    for (entity, mut transform, mut mobbable) in cats {
        commands
            .entity(entity)
            .remove::<(PouncedOn, InjureCrowTimer, Scared, WalkTo)>();
        transform.translation = CAT_HOME;
        transform.rotation = Quat::from_rotation_y(PI * 0.75);
        mobbable.time = 0.;
    }
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
            let position = Vec3::new(rng.random_range(-1.5..5.5), 0., rng.random_range(-1.5..5.5));
            commands.entity(entity).insert(WalkTo(position));
        }
    }
}

fn pounce(
    mut commands: Commands,
    cats: Query<(Entity, &Transform, &Mobbable), (With<Cat>, Without<PouncedOn>, Without<Scared>)>,
    mut crows: Query<
        (
            Entity,
            &mut Transform,
            &mut CrowState,
            &mut Velocity,
            &mut DesiredVelocity,
            &Species,
        ),
        (Without<Cat>, Without<InjuredTimer>),
    >,
) {
    for (cat_entity, cat_transform, mobbable) in cats {
        let nearby_strength: f32 = crows
            .iter()
            .filter(|(_, _, state, _, _, _)| state.is_attackable())
            .map(|(_, transform, _, _, _, species)| {
                let distance = transform.translation.distance(cat_transform.translation);
                mob_contribution(distance, 6., *species)
            })
            .sum();
        if nearby_strength >= mobbable.minimum {
            continue;
        }
        for (crow_entity, mut crow_transform, mut state, mut velocity, mut desired, _) in &mut crows
        {
            if let CrowState::CapturedBy(_) | CrowState::RecoveringFromInjury = *state {
                continue;
            }
            if cat_transform
                .translation
                .distance(crow_transform.translation)
                > 2.
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
            commands.trigger(PlaySfx {
                sound: Sfx::Impact,
                position: cat_transform.translation,
            });
            commands.trigger(SpawnParticles {
                kind: Particle::FeatherBurst,
                position: crow_transform.translation,
            });
            break;
        }
    }
}

fn get_mobbed(
    mut commands: Commands,
    cats: Query<(Entity, &Transform, &mut Mobbable, &PouncedOn)>,
    mut crows: Query<(&Transform, &mut CrowState, &Species)>,
    carrying_query: Query<(), With<Carrying>>,
    time: Res<Time>,
) {
    for (cat_entity, cat_transform, mut mobbable, pounced_on) in cats {
        let nearby_strength: f32 = crows
            .iter()
            .filter(|(_, state, _)| state.is_attackable())
            .map(|(transform, _, species)| {
                let distance = transform.translation.distance(cat_transform.translation);
                mob_contribution(distance, 4., *species)
            })
            .sum();
        if nearby_strength >= mobbable.minimum {
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
                .insert(WalkTo(CAT_HOME));
            for (_, mut state, _) in crows
                .iter_mut()
                .filter(|(_, state, _)| **state == CrowState::Mobbing(cat_entity))
            {
                *state = CrowState::FollowLeader;
            }
        }
    }
}

fn injure_pounced_crow(
    mut commands: Commands,
    cats: Query<(Entity, &mut InjureCrowTimer, &PouncedOn), With<Cat>>,
    crows: Query<(&Transform, Option<&Carrying>), With<Crow>>,
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
            if let Ok((crow_transform, carrying)) = crows.get(pounced_on.0) {
                if let Some(carrying) = carrying {
                    commands
                        .entity(carrying.0)
                        .remove_parent_in_place()
                        .insert(Carryable);
                }
                commands.trigger(SpawnParticles {
                    kind: Particle::FeatherBurst,
                    position: crow_transform.translation,
                });
            }
        }
    }
}

fn mob_contribution(distance: f32, radius: f32, species: Species) -> f32 {
    const PLATEAU: f32 = 1.0;
    species.strength() * ((radius - distance) / (radius - PLATEAU)).clamp(0., 1.)
}

impl Default for InjureCrowTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs(10), TimerMode::Once))
    }
}
