use std::time::Duration;

use bevy::prelude::*;

use crate::{
    audio::{PlaySfx, Sfx},
    crow::{Carrying, Crow, CrowState, DesiredVelocity, InjuredTimer, Species, Velocity},
    particles::{Particle, SpawnParticles},
    world::{Carryable, MissionPhase, Mobbable},
};

const TAR_POSITION: Vec3 = Vec3::new(-2.0, 0.05, 2.0);
const TAR_DIMENSIONS: Vec3 = Vec3::new(1.5, 0.1, 1.5);
const TAR_CAPTURE_RADIUS: f32 = 0.7;
const TAR_MOB_THRESHOLD: f32 = 4.0;

const KITE_POSITION: Vec3 = Vec3::new(3.0, 4.0, 3.0);
const KITE_DIMENSIONS: Vec3 = Vec3::new(0.2, 1.5, 0.2);
const KITE_CAPTURE_RADIUS: f32 = 0.6;
const KITE_MOB_THRESHOLD: f32 = 4.0;

const MOB_OVERPOWER_RADIUS: f32 = 4.0;
/// Within this radius from a mobbed hazard, a crow contributes full strength.
const MOB_FULL_STRENGTH_PLATEAU: f32 = 1.0;
const MOB_REQUIRED_SECONDS: f32 = 2.0;
const INJURE_STUCK_SECONDS: u64 = 10;

#[derive(Component)]
pub struct Hazard {
    capture_radius: f32,
}

#[derive(Component)]
struct Stuck(Entity);

#[derive(Component)]
struct InjureStuckTimer(Timer);

pub struct HazardPlugin;

impl Plugin for HazardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(OnExit(MissionPhase::Results), reset_hazards)
            .add_systems(Update, (snare, get_mobbed, injure_stuck_crow).chain());
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_hazards(&mut commands, &mut meshes, &mut materials);
}

fn spawn_hazards(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Hazard {
            capture_radius: TAR_CAPTURE_RADIUS,
        },
        Mesh3d(meshes.add(Cuboid::from_size(TAR_DIMENSIONS))),
        MeshMaterial3d(materials.add(Color::srgb(0.04, 0.04, 0.05))),
        Mobbable::minimum(TAR_MOB_THRESHOLD),
        Transform::from_translation(TAR_POSITION),
    ));
    commands.spawn((
        Hazard {
            capture_radius: KITE_CAPTURE_RADIUS,
        },
        Mesh3d(meshes.add(Cuboid::from_size(KITE_DIMENSIONS))),
        MeshMaterial3d(materials.add(Color::srgb(0.85, 0.7, 0.4))),
        Mobbable::minimum(KITE_MOB_THRESHOLD),
        Transform::from_translation(KITE_POSITION),
    ));
}

fn reset_hazards(
    mut commands: Commands,
    hazards: Query<Entity, With<Hazard>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in hazards {
        commands.entity(entity).despawn();
    }
    spawn_hazards(&mut commands, &mut meshes, &mut materials);
}

fn snare(
    mut commands: Commands,
    hazards: Query<(Entity, &Transform, &Hazard), Without<Stuck>>,
    mut crows: Query<
        (
            Entity,
            &mut Transform,
            &mut CrowState,
            &mut Velocity,
            &mut DesiredVelocity,
        ),
        (With<Crow>, Without<InjuredTimer>, Without<Hazard>),
    >,
) {
    for (hazard_entity, hazard_transform, hazard) in hazards {
        for (crow_entity, mut crow_transform, mut state, mut velocity, mut desired) in &mut crows {
            if !state.is_attackable() {
                continue;
            }
            if hazard_transform
                .translation
                .distance(crow_transform.translation)
                > hazard.capture_radius
            {
                continue;
            }
            commands
                .entity(hazard_entity)
                .insert((Stuck(crow_entity), InjureStuckTimer::default()));
            *state = CrowState::CapturedBy(hazard_entity);
            crow_transform.translation = hazard_transform.translation;
            velocity.0 = Vec3::ZERO;
            desired.0 = Vec3::ZERO;
            commands.trigger(PlaySfx {
                sound: Sfx::Impact,
                position: hazard_transform.translation,
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
    hazards: Query<(Entity, &Transform, &mut Mobbable, &Stuck), With<Hazard>>,
    mut crows: Query<(&Transform, &mut CrowState, &Species), With<Crow>>,
    carrying: Query<(), With<Carrying>>,
    time: Res<Time>,
) {
    for (hazard_entity, hazard_transform, mut mobbable, stuck) in hazards {
        let mob_strength: f32 = crows
            .iter()
            .filter(|(_, state, _)| state.is_attackable())
            .map(|(transform, _, species)| {
                let distance = transform.translation.distance(hazard_transform.translation);
                mob_contribution(distance, MOB_OVERPOWER_RADIUS, *species)
            })
            .sum();
        if mob_strength >= mobbable.minimum {
            mobbable.time += time.delta_secs();
        } else {
            mobbable.time = (mobbable.time - time.delta_secs()).max(0.0);
        }
        if mobbable.time < MOB_REQUIRED_SECONDS {
            continue;
        }
        let next_state = if carrying.contains(stuck.0) {
            CrowState::ReturningToRoost
        } else {
            CrowState::FollowLeader
        };
        commands.entity(stuck.0).insert(next_state);
        commands.entity(hazard_entity).despawn();
        for (_, mut state, _) in crows
            .iter_mut()
            .filter(|(_, state, _)| **state == CrowState::Mobbing(hazard_entity))
        {
            *state = CrowState::FollowLeader;
        }
    }
}

fn injure_stuck_crow(
    mut commands: Commands,
    hazards: Query<(Entity, &mut InjureStuckTimer, &Stuck), With<Hazard>>,
    crows: Query<(&Transform, Option<&Carrying>), With<Crow>>,
    time: Res<Time>,
) {
    for (hazard_entity, mut injure_timer, stuck) in hazards {
        injure_timer.0.tick(time.delta());
        if !injure_timer.0.just_finished() {
            continue;
        }
        commands
            .entity(hazard_entity)
            .remove::<(Stuck, InjureStuckTimer)>();
        commands
            .entity(stuck.0)
            .insert((InjuredTimer::default(), CrowState::ReturningToRoost))
            .try_remove::<Carrying>();
        let Ok((crow_transform, carrying)) = crows.get(stuck.0) else {
            continue;
        };
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

/// Strength a crow at `distance` contributes toward mobbing a target, scaling
/// linearly from full strength at `MOB_FULL_STRENGTH_PLATEAU` down to zero at
/// `radius` (and clamped to zero beyond that).
fn mob_contribution(distance: f32, radius: f32, species: Species) -> f32 {
    let falloff = ((radius - distance) / (radius - MOB_FULL_STRENGTH_PLATEAU)).clamp(0.0, 1.0);
    species.strength() * falloff
}

impl Default for InjureStuckTimer {
    fn default() -> Self {
        Self(Timer::new(
            Duration::from_secs(INJURE_STUCK_SECONDS),
            TimerMode::Once,
        ))
    }
}
