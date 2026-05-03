use bevy::{platform::collections::HashMap, prelude::*};
use bevy_enhanced_input::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::ResourceInspectorPlugin};

use crate::{
    crow::{Crow, CrowSystems, DesiredVelocity, FlockNeighbors, LeaderCrow, Velocity},
    input::{CommandCursor, Direct, Recall},
};

pub struct FlockPlugin;

#[derive(Component)]
struct HomeVisualizer;

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
    pub home: Vec3,
    pub home_weight: f32,
}

#[derive(Resource, Default)]
pub struct DirectedTarget(pub Option<Vec3>);

impl Plugin for FlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .init_resource::<BoidParams>()
            .init_resource::<DirectedTarget>()
            .register_type::<BoidParams>()
            .add_plugins(EguiPlugin::default())
            .add_plugins(ResourceInspectorPlugin::<BoidParams>::default())
            .add_systems(Startup, setup)
            .add_systems(
                FixedUpdate,
                (
                    update_home,
                    visualize_home,
                    rebuild_grid,
                    query_neighbors,
                    boids_steer,
                )
                    .chain()
                    .in_set(CrowSystems::Steer),
            )
            .add_observer(on_direct)
            .add_observer(on_recall);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        HomeVisualizer,
        Transform::from_translation(Vec3::ZERO),
        Mesh3d(meshes.add(Cuboid::from_length(0.1))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    ));
}

fn update_home(
    mut boid_params: ResMut<BoidParams>,
    directed_target: Res<DirectedTarget>,
    leader: Single<&Transform, With<LeaderCrow>>,
) {
    const ALTITUDE_OFFSET: Vec3 = Vec3::new(0.0, 3.0, 0.0);
    let base = directed_target.0.unwrap_or(leader.translation);
    boid_params.home = base + ALTITUDE_OFFSET;
}

fn on_direct(
    _: On<Start<Direct>>,
    cursor: Res<CommandCursor>,
    mut directed_target: ResMut<DirectedTarget>,
) {
    if let Some(world_position) = cursor.world_position {
        directed_target.0 = Some(world_position);
    }
}

fn on_recall(_: On<Start<Recall>>, mut directed_target: ResMut<DirectedTarget>) {
    directed_target.0 = None;
}

fn visualize_home(
    boid_params: Res<BoidParams>,
    mut home_visualizer: Single<&mut Transform, With<HomeVisualizer>>,
) {
    home_visualizer.translation = boid_params.home;
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
    others: Query<(&Transform, &Velocity), With<Crow>>,
    mut crows: Query<
        (&Transform, &Velocity, &FlockNeighbors, &mut DesiredVelocity),
        (With<Crow>, Without<LeaderCrow>),
    >,
) {
    for (transform, velocity, neighbors, mut desired_velocity) in &mut crows {
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
        let alignment_force = neighbor_velocity_sum / neighbor_count - velocity.0;
        let cohesion_force = neighbor_position_sum / neighbor_count - transform.translation;
        let home_force = (boid_params.home - transform.translation).normalize_or_zero();

        desired_velocity.0 = (separation_force * boid_params.separation_weight
            + alignment_force * boid_params.alignment_weight
            + cohesion_force * boid_params.cohesion_weight
            + home_force * boid_params.home_weight)
            .clamp_length_max(boid_params.max_speed);
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
            neighbor_radius: 3.0,
            separation_weight: 1.0,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            max_speed: 10.0,
            home: Vec3::new(0.0, 5.0, 0.0),
            home_weight: 5.0,
        }
    }
}
