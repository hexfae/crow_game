use std::{f32::consts::PI, time::Duration};

use bevy::prelude::*;

use crate::crow::{Crow, CrowState, FlockNeighbors};

const SOAR_RADIUS: f32 = 10.;
const SOAR_ALTITUDE: f32 = 10.;
const SOAR_RATE: f32 = 1.0;
const DIVE_COOLDOWN_SECS: f32 = 10.;
const DIVE_SPEED: f32 = 20.;
const DIVE_ARRIVAL_THRESHOLD: f32 = 0.5;
const ISOLATION_NEIGHBOR_LIMIT: usize = 3;

#[derive(Component)]
struct Hawk;

#[derive(Component)]
enum HawkState {
    Soaring { phase: f32, cooldown: Timer },
    Diving { locked_position: Vec3 },
}

pub struct HawkPlugin;

impl Plugin for HawkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (soar, dive));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Hawk,
        HawkState::soaring(),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/hawk/hawk.glb"))),
        Transform::from_scale(Vec3::splat(0.5)),
    ));
}

fn soar(
    mut commands: Commands,
    hawks: Query<(Entity, &mut Transform, &mut HawkState), With<Hawk>>,
    candidates: Query<
        (Entity, &Transform, &FlockNeighbors, &CrowState),
        (With<Crow>, Without<Hawk>),
    >,
    time: Res<Time>,
) {
    for (entity, mut transform, mut state) in hawks {
        let HawkState::Soaring { phase, cooldown } = &mut *state else {
            continue;
        };
        *phase += time.delta_secs() * SOAR_RATE;
        transform.translation = Vec3::new(
            phase.sin() * SOAR_RADIUS,
            SOAR_ALTITUDE,
            phase.cos() * SOAR_RADIUS,
        );
        transform.look_at(Vec3::Y * SOAR_ALTITUDE, Vec3::Y);
        transform.rotate_y(-PI / 2.);

        cooldown.tick(time.delta());
        if !cooldown.is_finished() {
            continue;
        }

        let hawk_position = transform.translation;
        let Some((_, target_transform, _, _)) = candidates
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
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(hawk_position)
                    .total_cmp(&b.1.translation.distance_squared(hawk_position))
            })
        else {
            cooldown.reset();
            continue;
        };

        commands.entity(entity).insert(HawkState::Diving {
            locked_position: target_transform.translation,
        });
    }
}

fn dive(
    mut commands: Commands,
    hawks: Query<(Entity, &mut Transform, &HawkState), With<Hawk>>,
    time: Res<Time>,
) {
    for (entity, mut transform, state) in hawks {
        let HawkState::Diving {
            locked_position, ..
        } = state
        else {
            continue;
        };
        let offset = *locked_position - transform.translation;
        let distance = offset.length();
        if distance < DIVE_ARRIVAL_THRESHOLD {
            commands.entity(entity).insert(HawkState::Soaring {
                phase: transform.translation.x.atan2(transform.translation.z),
                cooldown: Timer::new(Duration::from_secs_f32(DIVE_COOLDOWN_SECS), TimerMode::Once),
            });
            continue;
        }
        let direction = offset / distance;
        transform.translation += direction * DIVE_SPEED * time.delta_secs();
        transform.look_to(direction, Vec3::Y);
    }
}

impl HawkState {
    fn soaring() -> Self {
        Self::Soaring {
            phase: 0.,
            cooldown: Timer::new(Duration::from_secs_f32(DIVE_COOLDOWN_SECS), TimerMode::Once),
        }
    }
}
