use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::ResourceInspectorPlugin};
use rand::seq::IteratorRandom;

use crate::{
    audio::{PlaySfx, Sfx},
    crow::{
        Carrying, Crow, CrowState, CrowSystems, DesiredVelocity, FlockNeighbors, LeaderCrow,
        Species, Velocity,
    },
    input::{CommandCursor, Direct, Recall},
    world::{Carryable, Mobbable, Roost, Weight},
};

const ALTITUDE_OFFSET: Vec3 = Vec3::new(0.0, 3.0, 0.0);

pub struct FlockPlugin;

#[derive(Resource)]
pub struct SpatialGrid {
    cells: HashMap<IVec3, Vec<Entity>>,
    cell_size: f32,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct BoidParams {
    neighbor_radius: f32,
    separation_weight: f32,
    alignment_weight: f32,
    cohesion_weight: f32,
    max_speed: f32,
    goal_weight: f32,
    vertical_blend: f32,
}

impl Plugin for FlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .init_resource::<BoidParams>()
            .register_type::<BoidParams>()
            .add_plugins(EguiPlugin::default())
            .add_plugins(ResourceInspectorPlugin::<BoidParams>::default())
            .add_systems(
                FixedUpdate,
                (rebuild_grid, query_neighbors, reset_stale_grab, boids_steer)
                    .chain()
                    .in_set(CrowSystems::Steer),
            )
            .add_observer(on_direct)
            .add_observer(on_recall);
    }
}

fn on_direct(
    _: On<Start<Direct>>,
    mut commands: Commands,
    cursor: Res<CommandCursor>,
    mut free_crows: Query<(Entity, &mut CrowState, &Species), Without<Carrying>>,
    mut carrying_crows: Query<(Entity, &mut CrowState, &Species), With<Carrying>>,
    carryables: Query<(&Transform, Entity, &Weight), With<Carryable>>,
    mobbables: Query<(&Transform, Entity, &Mobbable)>,
    roost: Single<&Transform, With<Roost>>,
) {
    let Some(world_position) = cursor.world_position else {
        return;
    };
    let mut rng = rand::rng();
    let mut acknowledged = false;
    if let Some((_, entity, weight)) = carryables
        .iter()
        .find(|(transform, _, _)| transform.translation.distance(world_position) < 1.)
    {
        let min_strength = free_crows
            .iter()
            .filter(|(_, state, species)| {
                state.accepts_commands() && species.strength() >= weight.0
            })
            .map(|(_, _, s)| s.strength())
            .reduce(f32::min);
        if let Some(min) = min_strength
            && let Some(chosen) = free_crows
                .iter()
                .filter(|(_, state, species)| {
                    state.accepts_commands() && species.strength() == min
                })
                .map(|(e, _, _)| e)
                .choose(&mut rng)
            && let Ok((_, mut state, _)) = free_crows.get_mut(chosen)
        {
            *state = CrowState::GrabCarryable(entity);
            acknowledged = true;
        }
    } else if let Some((_, entity, mobbable)) = mobbables
        .iter()
        .find(|(transform, _, _)| transform.translation.distance(world_position) < 1.)
    {
        let mut candidates: Vec<(Entity, f32)> = free_crows
            .iter()
            .filter(|(_, state, _)| state.accepts_commands())
            .map(|(e, _, s)| (e, s.strength()))
            .collect();
        candidates.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.index().cmp(&b.0.index()))
        });
        let mut selected: Vec<(Entity, f32)> = Vec::new();
        let mut accumulated = 0.0;
        for (e, s) in candidates {
            if accumulated >= mobbable.minimum {
                break;
            }
            selected.push((e, s));
            accumulated += s;
        }
        let mut cut = 0;
        while cut < selected.len() && accumulated - selected[cut].1 >= mobbable.minimum {
            accumulated -= selected[cut].1;
            cut += 1;
        }
        for (crow_entity, _) in &selected[cut..] {
            if let Ok((_, mut state, _)) = free_crows.get_mut(*crow_entity) {
                *state = CrowState::Mobbing(entity);
                acknowledged = true;
            }
        }
    } else if roost.translation.distance(world_position) < 1. {
        for (_, mut state, _) in &mut carrying_crows {
            if state.accepts_commands() {
                *state = CrowState::ReturningToRoost;
                acknowledged = true;
            }
        }
    } else {
        for (_, mut state, _) in free_crows.iter_mut().chain(carrying_crows.iter_mut()) {
            if state.accepts_commands() && !matches!(*state, CrowState::GrabCarryable(_)) {
                *state = CrowState::SeekTarget(world_position);
                acknowledged = true;
            }
        }
    }
    if acknowledged {
        commands.trigger(PlaySfx {
            sound: Sfx::Command,
            position: world_position,
        });
    }
}

fn on_recall(
    _: On<Start<Recall>>,
    mut commands: Commands,
    mut crows: Query<&mut CrowState>,
    leader: Single<&Transform, With<LeaderCrow>>,
) {
    let mut acknowledged = false;
    for mut state in &mut crows {
        if state.accepts_commands() && !matches!(*state, CrowState::FollowLeader) {
            *state = CrowState::FollowLeader;
            acknowledged = true;
        }
    }
    if acknowledged {
        commands.trigger(PlaySfx {
            sound: Sfx::Recall,
            position: leader.translation,
        });
    }
}

fn rebuild_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    crows: Query<(Entity, &Transform), With<Crow>>,
) {
    spatial_grid.cells.clear();
    for (entity, transform) in &crows {
        let cell = (transform.translation / spatial_grid.cell_size)
            .floor()
            .as_ivec3();
        spatial_grid.cells.entry(cell).or_default().push(entity);
    }
}

fn query_neighbors(
    spatial_grid: Res<SpatialGrid>,
    boid_params: Res<BoidParams>,
    positions: Query<&Transform, With<Crow>>,
    mut crows: Query<(Entity, &Transform, &mut FlockNeighbors), (With<Crow>, Without<LeaderCrow>)>,
) {
    let radius_squared = boid_params.neighbor_radius.powi(2);
    for (own_entity, transform, mut neighbors) in &mut crows {
        neighbors.0.clear();
        let cell = (transform.translation / spatial_grid.cell_size)
            .floor()
            .as_ivec3();
        for delta_x in -1..=1 {
            for delta_y in -1..=1 {
                for delta_z in -1..=1 {
                    let offset = IVec3::new(delta_x, delta_y, delta_z);
                    let Some(bucket) = spatial_grid.cells.get(&(cell + offset)) else {
                        continue;
                    };
                    for &other_entity in bucket {
                        if other_entity == own_entity {
                            continue;
                        }
                        let Ok(other_transform) = positions.get(other_entity) else {
                            continue;
                        };
                        if transform
                            .translation
                            .distance_squared(other_transform.translation)
                            < radius_squared
                        {
                            neighbors.0.push(other_entity);
                        }
                    }
                }
            }
        }
    }
}

fn boids_steer(
    boid_params: Res<BoidParams>,
    leader: Single<&Transform, With<LeaderCrow>>,
    others: Query<(&Transform, &Velocity), With<Crow>>,
    carryables: Query<&Transform, With<Carryable>>,
    mobbables: Query<&Transform, With<Mobbable>>,
    roost: Single<&Transform, With<Roost>>,
    mut crows: Query<
        (
            &Transform,
            &Velocity,
            &FlockNeighbors,
            &CrowState,
            &Species,
            &mut DesiredVelocity,
        ),
        (With<Crow>, Without<LeaderCrow>),
    >,
) {
    for (transform, velocity, neighbors, state, species, mut desired_velocity) in &mut crows {
        let goal = match state {
            CrowState::FollowLeader => leader.translation + ALTITUDE_OFFSET,
            CrowState::SeekTarget(target) => *target + ALTITUDE_OFFSET,
            CrowState::ReturningToRoost | CrowState::RecoveringFromInjury => roost.translation,
            CrowState::CapturedBy(_) => continue,
            CrowState::Mobbing(entity) => {
                if let Ok(mobbable) = mobbables.get(*entity) {
                    mobbable.translation
                } else {
                    continue;
                }
            }
            CrowState::GrabCarryable(entity) => {
                if let Ok(carryable) = carryables.get(*entity) {
                    carryable.translation
                } else {
                    continue;
                }
            }
        };
        let (mut separation_force, mut neighbor_velocity_sum, mut neighbor_position_sum) =
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
        for &neighbor in &neighbors.0 {
            let Ok((other_transform, other_velocity)) = others.get(neighbor) else {
                continue;
            };
            let offset_from_neighbor = transform.translation - other_transform.translation;
            separation_force +=
                offset_from_neighbor / offset_from_neighbor.length_squared().max(0.01);
            neighbor_velocity_sum += other_velocity.0;
            neighbor_position_sum += other_transform.translation;
        }

        let neighbor_count = neighbors.0.len().max(1) as f32;
        let (alignment_force, cohesion_force) = match state {
            CrowState::ReturningToRoost
            | CrowState::RecoveringFromInjury
            | CrowState::GrabCarryable(_)
            | CrowState::Mobbing(_) => (Vec3::ZERO, Vec3::ZERO),
            _ => (
                neighbor_velocity_sum / neighbor_count - velocity.0,
                neighbor_position_sum / neighbor_count - transform.translation,
            ),
        };
        let goal_force = (goal - transform.translation).normalize_or_zero();

        let flock_force = (separation_force * boid_params.separation_weight
            + alignment_force * boid_params.alignment_weight
            + cohesion_force * boid_params.cohesion_weight)
            * Vec3::new(1.0, boid_params.vertical_blend, 1.0);

        desired_velocity.0 = (flock_force + goal_force * boid_params.goal_weight)
            .clamp_length_max(boid_params.max_speed * species.speed_factor());
    }
}

fn reset_stale_grab(mut crows: Query<&mut CrowState>, carryables: Query<(), With<Carryable>>) {
    for mut state in &mut crows {
        if let CrowState::GrabCarryable(entity) = *state
            && carryables.get(entity).is_err()
        {
            *state = CrowState::FollowLeader;
        }
    }
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self {
            cells: HashMap::new(),
            cell_size: 4.0,
        }
    }
}

impl Default for BoidParams {
    fn default() -> Self {
        Self {
            neighbor_radius: 8.0,
            separation_weight: 1.0,
            alignment_weight: -1.0,
            cohesion_weight: 2.0,
            max_speed: 10.0,
            goal_weight: 5.0,
            vertical_blend: 0.1,
        }
    }
}
