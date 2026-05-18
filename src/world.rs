use std::{f32::consts::PI, fmt::Display, time::Duration};

use bevy::{camera::primitives::Aabb, color::Mix, prelude::*, time::Stopwatch};
use rand::RngExt;

use crate::{
    crow::{Crow, InjuredTimer},
    input::Restart,
};
use bevy_enhanced_input::prelude::*;

const DAY_LENGTH: Duration = Duration::from_mins(10);
const DUSK_LENGTH: Duration = Duration::from_mins(1);
const NIGHT_LENGTH: Duration = Duration::from_mins(1);
const DAY_TO_DUSK_WINDOW: Duration = Duration::from_secs(60);
const DUSK_TO_NIGHT_WINDOW: Duration = Duration::from_secs(30);
const NIGHT_TO_RESULTS_WINDOW: Duration = Duration::from_secs(10);
const DAY_LIGHT: LinearRgba = LinearRgba::WHITE;
const DUSK_LIGHT: LinearRgba = LinearRgba::new(1.0, 0.25, 0.05, 1.0);
const NIGHT_LIGHT: LinearRgba = LinearRgba::new(0.02, 0.04, 0.15, 1.0);
const DAY_SKY: LinearRgba = LinearRgba::new(0.3, 0.5, 0.8, 1.0);
const DUSK_SKY: LinearRgba = LinearRgba::new(0.7, 0.25, 0.1, 1.0);
const NIGHT_SKY: LinearRgba = LinearRgba::new(0.005, 0.01, 0.05, 1.0);

const GROUND_HALF_EXTENT: f32 = 7.0;
const PLAYFIELD_HALF_EXTENT: f32 = 5.0;
const ROOST_POSITION: Vec3 = Vec3::new(-4.5, 0.25, -4.5);
const ROOST_DIMENSIONS: Vec3 = Vec3::new(2.0, 0.5, 2.0);
const COVER_POSITION: Vec3 = Vec3::new(-5.0, 8.0, -5.0);
const COVER_DIMENSIONS: Vec3 = Vec3::new(5.0, 0.5, 5.0);

const LIGHT_CARRYABLE_COUNT: usize = 6;
const LIGHT_CARRYABLE_DIMENSIONS: Vec3 = Vec3::new(2.0, 0.2, 0.5);
const LIGHT_CARRYABLE_WEIGHT: f32 = 1.0;

const HEAVY_CARRYABLE_COUNT: usize = 2;
const HEAVY_CARRYABLE_DIMENSIONS: Vec3 = Vec3::new(0.7, 0.2, 0.7);
const HEAVY_CARRYABLE_WEIGHT: f32 = 3.0;

#[derive(Component)]
pub struct Carryable;

#[derive(Component)]
pub struct Weight(pub f32);

#[derive(Component)]
pub struct Mobbable {
    /// Total crow strength required in radius to intimidate the attacker.
    pub minimum: f32,
    /// How long (in seconds) the attacker has been mobbed.
    pub time: f32,
}

#[derive(Component)]
pub struct Roost;

#[derive(Component)]
pub struct Cover;

#[derive(Component)]
struct Sun;

#[derive(Component)]
pub struct UnderCover;

#[derive(Default, Resource)]
pub struct Score(pub u32);

#[derive(Default, Resource)]
pub struct Injured(pub u32);

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MissionPhase {
    #[default]
    Day,
    Dusk,
    Night,
    Results,
}

#[derive(Default, Resource)]
pub struct WorldTimer(pub Stopwatch);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<Injured>()
            .init_state::<MissionPhase>()
            .init_resource::<WorldTimer>()
            .insert_resource(ClearColor(Color::from(DAY_SKY)))
            .add_systems(Startup, setup)
            .add_systems(Update, (pass_time, tint_sun, count_injured))
            .add_systems(FixedUpdate, update_cover)
            .add_systems(OnExit(MissionPhase::Results), reset_world)
            .add_observer(on_restart);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(
            Vec3::Y,
            Vec2::splat(GROUND_HALF_EXTENT),
        ))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));
    commands.spawn((
        Sun,
        DirectionalLight {
            shadows_enabled: true,
            color: Color::from(DAY_LIGHT),
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Roost,
        Mesh3d(meshes.add(Cuboid::from_size(ROOST_DIMENSIONS))),
        MeshMaterial3d(materials.add(Color::srgb_u8(150, 75, 0))),
        Transform::from_translation(ROOST_POSITION),
    ));
    commands.spawn((
        Cover,
        Mesh3d(meshes.add(Cuboid::from_size(COVER_DIMENSIONS))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_translation(COVER_POSITION),
    ));

    spawn_carryables(&mut commands, &mut meshes, &mut materials);
}

fn spawn_carryables(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mut rng = rand::rng();
    let light_material = materials.add(Color::srgb_u8(255, 255, 0));
    let heavy_material = materials.add(Color::srgb_u8(120, 120, 120));
    let light_mesh = meshes.add(Cuboid::from_size(LIGHT_CARRYABLE_DIMENSIONS));
    let heavy_mesh = meshes.add(Cuboid::from_size(HEAVY_CARRYABLE_DIMENSIONS));

    for _ in 0..LIGHT_CARRYABLE_COUNT {
        commands.spawn((
            Mesh3d(light_mesh.clone()),
            MeshMaterial3d(light_material.clone()),
            Carryable,
            Weight(LIGHT_CARRYABLE_WEIGHT),
            random_carryable_transform(&mut rng),
        ));
    }
    for _ in 0..HEAVY_CARRYABLE_COUNT {
        commands.spawn((
            Mesh3d(heavy_mesh.clone()),
            MeshMaterial3d(heavy_material.clone()),
            Carryable,
            Weight(HEAVY_CARRYABLE_WEIGHT),
            random_carryable_transform(&mut rng),
        ));
    }
}

fn random_carryable_transform(rng: &mut impl rand::Rng) -> Transform {
    let position = Vec3::new(
        rng.random_range(-PLAYFIELD_HALF_EXTENT..PLAYFIELD_HALF_EXTENT),
        0.0,
        rng.random_range(-PLAYFIELD_HALF_EXTENT..PLAYFIELD_HALF_EXTENT),
    );
    let yaw = rng.random_range(0.0..PI * 2.0);
    Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw))
}

fn update_cover(
    mut commands: Commands,
    crows: Query<(Entity, &Transform), With<Crow>>,
    covers: Query<(&GlobalTransform, &Aabb), With<Cover>>,
) {
    for (crow_entity, crow_transform) in &crows {
        let crow_xz = crow_transform.translation.xz();
        let is_under_cover = covers.iter().any(|(cover_transform, aabb)| {
            let center_xz = cover_transform.translation().xz() + aabb.center.xz();
            let half_extents_xz = aabb.half_extents.xz();
            let delta = (crow_xz - center_xz).abs();
            delta.cmple(half_extents_xz).all()
        });
        if is_under_cover {
            commands.entity(crow_entity).insert(UnderCover);
        } else {
            commands.entity(crow_entity).try_remove::<UnderCover>();
        }
    }
}

fn tint_sun(
    mut sun: Single<&mut DirectionalLight, With<Sun>>,
    mut clear_color: ResMut<ClearColor>,
    world_timer: Res<WorldTimer>,
) {
    let elapsed = world_timer.0.elapsed();
    let (from_light, from_sky, to_light, to_sky, boundary, window) = if elapsed < DAY_LENGTH {
        (
            DAY_LIGHT,
            DAY_SKY,
            DUSK_LIGHT,
            DUSK_SKY,
            DAY_LENGTH,
            DAY_TO_DUSK_WINDOW,
        )
    } else if elapsed < DAY_LENGTH + DUSK_LENGTH {
        (
            DUSK_LIGHT,
            DUSK_SKY,
            NIGHT_LIGHT,
            NIGHT_SKY,
            DAY_LENGTH + DUSK_LENGTH,
            DUSK_TO_NIGHT_WINDOW,
        )
    } else {
        (
            NIGHT_LIGHT,
            NIGHT_SKY,
            NIGHT_LIGHT,
            NIGHT_SKY,
            DAY_LENGTH + DUSK_LENGTH,
            DUSK_TO_NIGHT_WINDOW,
        )
    };
    let transition_start = boundary.saturating_sub(window);
    let progress = if elapsed <= transition_start {
        0.0
    } else {
        ((elapsed - transition_start).as_secs_f32() / window.as_secs_f32()).clamp(0.0, 1.0)
    };
    sun.color = Color::from(from_light.mix(&to_light, progress));
    clear_color.0 = Color::from(from_sky.mix(&to_sky, progress));
}

fn count_injured(new_injuries: Query<(), Added<InjuredTimer>>, mut injured: ResMut<Injured>) {
    injured.0 += new_injuries.iter().count() as u32;
}

fn on_restart(
    _: On<Start<Restart>>,
    state: Res<State<MissionPhase>>,
    mut next_state: ResMut<NextState<MissionPhase>>,
) {
    if *state.get() == MissionPhase::Results {
        next_state.set(MissionPhase::Day);
    }
}

fn reset_world(
    mut commands: Commands,
    mut world_timer: ResMut<WorldTimer>,
    mut score: ResMut<Score>,
    mut injured: ResMut<Injured>,
    carryables: Query<Entity, With<Carryable>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    world_timer.0.reset();
    score.0 = 0;
    injured.0 = 0;
    for entity in carryables {
        commands.entity(entity).despawn();
    }
    spawn_carryables(&mut commands, &mut meshes, &mut materials);
}

fn pass_time(
    state: Res<State<MissionPhase>>,
    mut next_state: ResMut<NextState<MissionPhase>>,
    mut world_timer: ResMut<WorldTimer>,
    time: Res<Time>,
) {
    if *state.get() == MissionPhase::Results {
        return;
    }
    world_timer.0.tick(time.delta());
    let elapsed = world_timer.0.elapsed();
    let new_phase = if elapsed < DAY_LENGTH {
        MissionPhase::Day
    } else if elapsed < DAY_LENGTH + DUSK_LENGTH {
        MissionPhase::Dusk
    } else if elapsed < DAY_LENGTH + DUSK_LENGTH + NIGHT_LENGTH {
        MissionPhase::Night
    } else {
        MissionPhase::Results
    };
    if *state.get() != new_phase {
        next_state.set(new_phase);
    }
}

impl Mobbable {
    pub fn minimum(minimum: f32) -> Self {
        Self { minimum, time: 0.0 }
    }
}

impl WorldTimer {
    pub fn night_fade_alpha(&self) -> f32 {
        let elapsed = self.0.elapsed();
        let results_at = DAY_LENGTH + DUSK_LENGTH + NIGHT_LENGTH;
        let fade_start = results_at.saturating_sub(NIGHT_TO_RESULTS_WINDOW);
        if elapsed <= fade_start {
            return 0.0;
        }
        ((elapsed - fade_start).as_secs_f32() / NIGHT_TO_RESULTS_WINDOW.as_secs_f32())
            .clamp(0.0, 1.0)
    }

    pub fn remaining_in(&self, phase: MissionPhase) -> Duration {
        let phase_end = match phase {
            MissionPhase::Day => DAY_LENGTH,
            MissionPhase::Dusk => DAY_LENGTH + DUSK_LENGTH,
            MissionPhase::Night => DAY_LENGTH + DUSK_LENGTH + NIGHT_LENGTH,
            MissionPhase::Results => return Duration::ZERO,
        };
        phase_end.saturating_sub(self.0.elapsed())
    }
}

impl Display for MissionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MissionPhase::Day => "Day",
            MissionPhase::Dusk => "Dusk",
            MissionPhase::Night => "Night",
            MissionPhase::Results => "Results",
        })
    }
}
