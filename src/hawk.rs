use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;

use crate::{
    crow::{Carrying, Crow, CrowState, FlockNeighbors, InjuredTimer},
    world::{Carryable, MissionPhase, UnderCover},
};

const SOAR_RADIUS: f32 = 10.0;
const SOAR_ALTITUDE: f32 = 10.0;
const SOAR_RATE: f32 = 1.0;

const DIVE_ABORT_COOLDOWN_SECS: f32 = 5.0;
const DIVE_MISS_COOLDOWN_SECS: f32 = 8.0;
const DIVE_COOLDOWN_SECS: f32 = 12.0;
const DIVE_SPEED: f32 = 20.0;
const DIVE_ARRIVAL_THRESHOLD: f32 = 0.5;
/// Distance at which the hawk locks onto the target position even if the crow moves.
const DIVE_COMMIT_DISTANCE: f32 = 3.0;
const DIVE_HIT_RADIUS: f32 = 0.5;

const CLIMB_SPEED: f32 = 10.0;
const CLIMB_ARRIVAL_THRESHOLD: f32 = 0.5;

/// A crow with at most this many flock neighbors is considered "isolated"
/// and becomes a candidate target.
const ISOLATION_NEIGHBOR_LIMIT: usize = 3;

#[derive(Component)]
struct Hawk;

#[derive(Component)]
enum HawkState {
    Soaring {
        phase: f32,
        cooldown: Timer,
    },
    Diving {
        target: Entity,
        locked_position: Option<Vec3>,
    },
    Climbing {
        target: Vec3,
        cooldown: Timer,
    },
}

pub struct HawkPlugin;

impl Plugin for HawkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MissionPhase::Dusk), spawn_hawk)
            .add_systems(OnExit(MissionPhase::Results), despawn_hawks)
            .add_systems(Update, (soar, pick_target, dive, climb).chain());
    }
}

fn spawn_hawk(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Hawk,
        HawkState::soaring(),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/hawk/hawk.glb"))),
        Transform::from_scale(Vec3::splat(0.5)),
    ));
}

fn despawn_hawks(mut commands: Commands, hawks: Query<Entity, With<Hawk>>) {
    for entity in hawks {
        commands.entity(entity).despawn();
    }
}

fn soar(hawks: Query<(&mut Transform, &mut HawkState), With<Hawk>>, time: Res<Time>) {
    for (mut transform, mut state) in hawks {
        let HawkState::Soaring { phase, .. } = &mut *state else {
            continue;
        };
        *phase += time.delta_secs() * SOAR_RATE;
        transform.translation = Vec3::new(
            phase.sin() * SOAR_RADIUS,
            SOAR_ALTITUDE,
            phase.cos() * SOAR_RADIUS,
        );
        // Face along the tangent of the circular soar path.
        transform.look_at(Vec3::Y * SOAR_ALTITUDE, Vec3::Y);
        transform.rotate_y(-PI / 2.0);
    }
}

fn pick_target(
    mut commands: Commands,
    hawks: Query<(Entity, &Transform, &mut HawkState), With<Hawk>>,
    candidates: Query<
        (Entity, &Transform, &FlockNeighbors, &CrowState),
        (With<Crow>, Without<Hawk>, Without<UnderCover>),
    >,
    time: Res<Time>,
) {
    for (hawk_entity, hawk_transform, mut state) in hawks {
        let HawkState::Soaring { cooldown, .. } = &mut *state else {
            continue;
        };
        cooldown.tick(time.delta());
        if !cooldown.is_finished() {
            continue;
        }

        let hawk_position = hawk_transform.translation;
        let target = candidates
            .iter()
            .filter(|(_, _, neighbors, crow_state)| {
                neighbors.0.len() <= ISOLATION_NEIGHBOR_LIMIT
                    && matches!(
                        crow_state,
                        CrowState::FollowLeader
                            | CrowState::SeekTarget(_)
                            | CrowState::ReturningToRoost
                    )
            })
            .min_by(|(_, a_transform, a_neighbors, _), (_, b_transform, b_neighbors, _)| {
                a_neighbors
                    .0
                    .len()
                    .cmp(&b_neighbors.0.len())
                    .then_with(|| {
                        a_transform
                            .translation
                            .distance_squared(hawk_position)
                            .total_cmp(&b_transform.translation.distance_squared(hawk_position))
                    })
            });

        let Some((target_entity, _, _, _)) = target else {
            cooldown.reset();
            continue;
        };

        commands
            .entity(hawk_entity)
            .insert(HawkState::diving(target_entity));
    }
}

fn dive(
    mut commands: Commands,
    mut hawks: Query<(Entity, &mut Transform, &mut HawkState), With<Hawk>>,
    crows: Query<(Entity, &Transform, &CrowState), (With<Crow>, Without<Hawk>)>,
    carrying: Query<&Carrying>,
    under_cover: Query<(), With<UnderCover>>,
    time: Res<Time>,
) {
    for (hawk_entity, mut hawk_transform, mut hawk_state) in &mut hawks {
        let HawkState::Diving {
            target,
            locked_position,
        } = &mut *hawk_state
        else {
            continue;
        };

        let target_position = if let Some(locked) = *locked_position {
            locked
        } else {
            let Ok((_, target_transform, target_state)) = crows.get(*target) else {
                commands
                    .entity(hawk_entity)
                    .insert(HawkState::climb_after(&hawk_transform, DIVE_ABORT_COOLDOWN_SECS));
                continue;
            };
            if under_cover.contains(*target) || !target_state.is_attackable() {
                commands
                    .entity(hawk_entity)
                    .insert(HawkState::climb_after(&hawk_transform, DIVE_ABORT_COOLDOWN_SECS));
                continue;
            }
            target_transform.translation
        };

        let offset = target_position - hawk_transform.translation;
        let distance = offset.length();

        if distance < DIVE_ARRIVAL_THRESHOLD {
            let hit = crows
                .iter()
                .filter(|(_, _, crow_state)| crow_state.is_attackable())
                .find(|(_, crow_transform, _)| {
                    crow_transform
                        .translation
                        .distance(hawk_transform.translation)
                        < DIVE_HIT_RADIUS
                });
            let Some((crow_entity, _, _)) = hit else {
                commands
                    .entity(hawk_entity)
                    .insert(HawkState::climb_after(&hawk_transform, DIVE_MISS_COOLDOWN_SECS));
                continue;
            };
            commands
                .entity(hawk_entity)
                .insert(HawkState::climb_after(&hawk_transform, DIVE_COOLDOWN_SECS));
            commands
                .entity(crow_entity)
                .insert((InjuredTimer::default(), CrowState::ReturningToRoost))
                .try_remove::<Carrying>();
            if let Ok(carrying) = carrying.get(crow_entity) {
                commands
                    .entity(carrying.0)
                    .remove_parent_in_place()
                    .insert(Carryable);
            }
            continue;
        }

        if locked_position.is_none() && distance < DIVE_COMMIT_DISTANCE {
            *locked_position = Some(target_position);
        }

        let direction = offset / distance;
        hawk_transform.translation += direction * DIVE_SPEED * time.delta_secs();
        hawk_transform.look_to(direction, Vec3::Y);
    }
}

fn climb(
    mut commands: Commands,
    hawks: Query<(Entity, &mut Transform, &mut HawkState), With<Hawk>>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut state) in hawks {
        let HawkState::Climbing { target, cooldown } = &mut *state else {
            continue;
        };
        cooldown.tick(time.delta());
        let offset = *target - transform.translation;
        let distance = offset.length();
        if distance < CLIMB_ARRIVAL_THRESHOLD {
            commands.entity(entity).insert(HawkState::Soaring {
                phase: target.x.atan2(target.z),
                cooldown: Timer::new(cooldown.remaining(), TimerMode::Once),
            });
            continue;
        }
        let direction = offset / distance;
        transform.translation += direction * CLIMB_SPEED * time.delta_secs();
        if direction.xz().length_squared() > 0.001 {
            transform.look_to(direction, Vec3::Y);
        }
    }
}

impl HawkState {
    fn soaring() -> Self {
        Self::Soaring {
            phase: 0.0,
            cooldown: Timer::new(Duration::from_secs_f32(DIVE_COOLDOWN_SECS), TimerMode::Once),
        }
    }

    /// Builds a `Climbing` state that returns to the soar ring along the
    /// shortest horizontal path from `transform`.
    fn climb_after(transform: &Transform, cooldown_secs: f32) -> Self {
        let horizontal = -transform
            .translation
            .xz()
            .try_normalize()
            .unwrap_or(Vec2::X);
        let target = Vec3::new(
            horizontal.x * SOAR_RADIUS,
            SOAR_ALTITUDE,
            horizontal.y * SOAR_RADIUS,
        );
        Self::Climbing {
            target,
            cooldown: Timer::new(Duration::from_secs_f32(cooldown_secs), TimerMode::Once),
        }
    }

    fn diving(target: Entity) -> Self {
        Self::Diving {
            target,
            locked_position: None,
        }
    }
}
