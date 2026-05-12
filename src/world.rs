use std::{f32::consts::PI, fmt::Display, time::Duration};

use bevy::{camera::primitives::Aabb, color::Mix, prelude::*, time::Stopwatch};
use rand::RngExt;

use crate::crow::{Crow, InjuredTimer};

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

#[derive(Component)]
pub struct Carryable;

#[derive(Component)]
pub struct Mobbable {
    /// The minimum amount of crows to intimidate the attacker.
    pub minimum: usize,
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
            .add_systems(FixedUpdate, update_cover);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(7., 7.)))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));
    commands.spawn((
        Sun,
        DirectionalLight {
            shadows_enabled: true,
            color: Color::from(DAY_LIGHT),
            ..default()
        },
        Transform::from_xyz(1., 1., -1.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Roost,
        Mesh3d(meshes.add(Cuboid::new(2., 0.5, 2.))),
        MeshMaterial3d(materials.add(Color::srgb_u8(150, 75, 0))),
        Transform::from_xyz(-4.5, 0.25, -4.5),
    ));
    commands.spawn((
        Cover,
        Mesh3d(meshes.add(Cuboid::new(5., 0.5, 5.))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(-5., 8.0, -5.),
    ));

    let mut rng = rand::rng();
    for _ in 0..8 {
        let position = Vec3::new(rng.random_range(-5.0..5.), 0., rng.random_range(-5.0..5.));
        let rotation = rng.random_range(0.0..PI * 2.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2., 0.2, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 255, 0))),
            Carryable,
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(rotation)),
        ));
    }
}

fn update_cover(
    mut commands: Commands,
    crows: Query<(Entity, &Transform), With<Crow>>,
    covers: Query<(&GlobalTransform, &Aabb), With<Cover>>,
) {
    for (crow_entity, crow_transform) in &crows {
        let under = covers.iter().any(|(cover_xf, aabb)| {
            let center_xz = cover_xf.translation().xz() + Vec2::new(aabb.center.x, aabb.center.z);
            let he_xz = Vec2::new(aabb.half_extents.x, aabb.half_extents.z);
            let d = (crow_transform.translation.xz() - center_xz).abs();
            d.cmple(he_xz).all()
        });
        if under {
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
    let t = if elapsed <= transition_start {
        0.0
    } else {
        ((elapsed - transition_start).as_secs_f32() / window.as_secs_f32()).clamp(0.0, 1.0)
    };
    sun.color = Color::from(from_light.mix(&to_light, t));
    clear_color.0 = Color::from(from_sky.mix(&to_sky, t));
}

fn count_injured(injuries: Query<(), Added<InjuredTimer>>, mut injured: ResMut<Injured>) {
    injured.0 += injuries.iter().count() as u32;
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
    pub fn minimum(minimum: usize) -> Self {
        Self { minimum, time: 0. }
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
        write!(
            f,
            "{}",
            match self {
                MissionPhase::Day => "Day",
                MissionPhase::Dusk => "Dusk",
                MissionPhase::Night => "Night",
                MissionPhase::Results => "Results",
            }
        )
    }
}
