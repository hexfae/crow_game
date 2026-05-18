use std::time::Duration;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use rand::RngExt;

use crate::{
    audio::{PlaySfx, Sfx},
    input::{Direct, MoveLeader, PanCamera, Player, Recall, Restart, Zoom},
    world::{Carryable, MissionPhase, Roost, Score},
};

const LEADER_SPEED: f32 = 4.0;
const LEADER_SPAWN_HEIGHT: f32 = 3.0;
const LEADER_LIGHT_INTENSITY: f32 = 200_000.0;
const LEADER_LIGHT_OFFSET: Vec3 = Vec3::new(0.0, 4.0, 0.0);

const CARRION_COUNT: usize = 8;
const RAVEN_COUNT: usize = 2;

const SPAWN_AREA_HALF_EXTENT: f32 = 5.0;
const SPAWN_MIN_HEIGHT: f32 = 5.0;
const SPAWN_MAX_HEIGHT: f32 = 6.0;
const INITIAL_SPEED: f32 = 2.0;

/// Per-second decay rate for blending current velocity toward desired velocity.
const VELOCITY_SMOOTHING_RATE: f32 = 5.0;
/// Squared-speed below which we don't reorient the crow toward its velocity.
const FACING_EPSILON_SQUARED: f32 = 0.001;
/// Squared-speed equivalent for the leader's manual control.
const LEADER_FACING_EPSILON_SQUARED: f32 = 0.01;

/// Pickup, deposit, and recovery all happen within this radius of their target.
const PICKUP_RADIUS: f32 = 0.5;
const ROOST_RADIUS: f32 = 1.0;

const INJURY_RECOVERY_SECONDS: u64 = 5;

#[derive(Component)]
pub struct Crow;

#[derive(Component)]
pub struct LeaderCrow;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    Carrion,
    Raven,
}

impl Species {
    pub fn scale(self) -> f32 {
        match self {
            Species::Carrion => 0.2,
            Species::Raven => 0.3,
        }
    }

    pub fn speed_factor(self) -> f32 {
        match self {
            Species::Carrion => 1.0,
            Species::Raven => 0.6,
        }
    }

    pub fn strength(self) -> f32 {
        match self {
            Species::Carrion => 1.0,
            Species::Raven => 3.0,
        }
    }
}

#[derive(Component, Default, PartialEq)]
pub enum CrowState {
    #[default]
    FollowLeader,
    SeekTarget(Vec3),
    ReturningToRoost,
    RecoveringFromInjury,
    CapturedBy(Entity),
    Mobbing(Entity),
    GrabCarryable(Entity),
}

#[derive(Default, Component)]
pub struct Velocity(pub Vec3);

#[derive(Default, Component)]
pub struct DesiredVelocity(pub Vec3);

#[derive(Default, Component)]
pub struct FlockNeighbors(pub Vec<Entity>);

#[derive(Component)]
pub struct Carrying(pub Entity);

#[derive(Component)]
pub struct InjuredTimer(Timer);

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub enum CrowSystems {
    Steer,
    Integrate,
}

pub struct CrowPlugin;

impl Plugin for CrowPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (CrowSystems::Steer, CrowSystems::Integrate).chain(),
        )
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (recover_from_injury, integrate, pickup, deposit_in_roost)
                .chain()
                .in_set(CrowSystems::Integrate),
        )
        .add_systems(OnExit(MissionPhase::Results), reset_crows)
        .add_observer(move_leader)
        .add_observer(stop_leader);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let crow_scene =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/crow/crow.glb"));
    let mut rng = rand::rng();

    for (species, count) in [
        (Species::Carrion, CARRION_COUNT),
        (Species::Raven, RAVEN_COUNT),
    ] {
        for _ in 0..count {
            let position = Vec3::new(
                rng.random_range(-SPAWN_AREA_HALF_EXTENT..SPAWN_AREA_HALF_EXTENT),
                rng.random_range(SPAWN_MIN_HEIGHT..SPAWN_MAX_HEIGHT),
                rng.random_range(-SPAWN_AREA_HALF_EXTENT..SPAWN_AREA_HALF_EXTENT),
            );
            let direction = Vec3::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(-0.2..0.2),
                rng.random_range(-1.0..1.0),
            )
            .normalize_or_zero();

            commands.spawn((
                Crow,
                species,
                CrowState::default(),
                Velocity(direction * INITIAL_SPEED),
                DesiredVelocity::default(),
                FlockNeighbors::default(),
                Transform::from_translation(position).with_scale(Vec3::splat(species.scale())),
                SceneRoot(crow_scene.clone()),
            ));
        }
    }

    commands
        .spawn((
            Crow,
            LeaderCrow,
            Species::Carrion,
            Player,
            Velocity::default(),
            Transform::from_scale(Vec3::splat(Species::Carrion.scale()))
                .with_translation(Vec3::Y * LEADER_SPAWN_HEIGHT),
            SceneRoot(crow_scene),
            actions!(Player[
                (
                    Action::<MoveLeader>::new(),
                    DeadZone::default(),
                    Scale::splat(3.0),
                    bindings![
                        KeyCode::KeyD,
                        (KeyCode::KeyA, Negate::all()),
                        (KeyCode::Space, SwizzleAxis::YXZ),
                        (KeyCode::ControlLeft, SwizzleAxis::YXZ, Negate::all()),
                        (KeyCode::KeyS, SwizzleAxis::ZYX),
                        (KeyCode::KeyW, SwizzleAxis::ZYX, Negate::all()),
                    ],
                ),
                (Action::<Direct>::new(), bindings![MouseButton::Left]),
                (Action::<Recall>::new(), bindings![MouseButton::Right]),
                (Action::<Restart>::new(), bindings![KeyCode::Enter]),
                (
                    Action::<Zoom>::new(),
                    bindings![(Binding::mouse_wheel(), SwizzleAxis::YXZ)],
                ),
                (
                    Action::<PanCamera>::new(),
                    DeadZone::default(),
                    bindings![
                        KeyCode::ArrowRight,
                        (KeyCode::ArrowLeft, Negate::all()),
                        (KeyCode::ArrowDown, SwizzleAxis::ZYX),
                        (KeyCode::ArrowUp, SwizzleAxis::ZYX, Negate::all()),
                    ],
                ),
            ]),
        ))
        .with_child((
            PointLight {
                color: Color::srgb(1.0, 0.0, 0.0),
                intensity: LEADER_LIGHT_INTENSITY,
                ..default()
            },
            Transform::from_translation(LEADER_LIGHT_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
        ));
}

fn integrate(
    time: Res<Time<Fixed>>,
    mut crows: Query<(&DesiredVelocity, &mut Velocity, &mut Transform, &CrowState), With<Crow>>,
) {
    let dt = time.delta_secs();
    let blend = (VELOCITY_SMOOTHING_RATE * dt).min(1.0);
    for (desired, mut velocity, mut transform, state) in &mut crows {
        if matches!(state, CrowState::CapturedBy(_)) {
            continue;
        }
        velocity.0 = velocity.0.lerp(desired.0, blend);
        transform.translation += velocity.0 * dt;
        if velocity.0.length_squared() > FACING_EPSILON_SQUARED {
            transform.look_to(velocity.0.normalize(), Vec3::Y);
        }
    }
}

fn pickup(
    mut commands: Commands,
    crows: Query<(&mut CrowState, &Transform, Entity)>,
    carryables: Query<&Transform, With<Carryable>>,
) {
    for (mut state, crow_transform, crow_entity) in crows {
        if let CrowState::GrabCarryable(carryable_entity) = *state
            && let Ok(carryable_transform) = carryables.get(carryable_entity)
            && carryable_transform
                .translation
                .distance(crow_transform.translation)
                < PICKUP_RADIUS
        {
            commands
                .entity(crow_entity)
                .insert(Carrying(carryable_entity));
            commands
                .entity(carryable_entity)
                .remove::<Carryable>()
                .set_parent_in_place(crow_entity);
            *state = CrowState::ReturningToRoost;
            commands.trigger(PlaySfx {
                sound: Sfx::Pickup,
                position: crow_transform.translation,
            });
        }
    }
}

fn deposit_in_roost(
    mut commands: Commands,
    crows: Query<(&Transform, &Carrying, &mut CrowState, Entity)>,
    roost: Single<&Transform, With<Roost>>,
    mut score: ResMut<Score>,
) {
    for (transform, carrying, mut state, entity) in crows {
        if matches!(*state, CrowState::CapturedBy(_)) {
            continue;
        }
        if transform.translation.distance(roost.translation) < ROOST_RADIUS {
            commands.entity(entity).remove::<Carrying>();
            commands.entity(carrying.0).despawn();
            *state = CrowState::FollowLeader;
            score.0 += 1;
            commands.trigger(PlaySfx {
                sound: Sfx::Deposit,
                position: roost.translation,
            });
        }
    }
}

fn recover_from_injury(
    mut commands: Commands,
    crows: Query<(Entity, &Transform, &CrowState, &mut InjuredTimer), With<Crow>>,
    roost: Single<&Transform, With<Roost>>,
    time: Res<Time>,
) {
    for (entity, transform, state, mut injured_timer) in crows {
        if transform.translation.distance(roost.translation) >= ROOST_RADIUS {
            continue;
        }
        if !matches!(state, CrowState::RecoveringFromInjury) {
            commands
                .entity(entity)
                .insert(CrowState::RecoveringFromInjury);
        }
        injured_timer.0.tick(time.delta());
        if injured_timer.0.just_finished() {
            commands
                .entity(entity)
                .remove::<InjuredTimer>()
                .insert(CrowState::FollowLeader);
        }
    }
}

fn reset_crows(
    mut commands: Commands,
    mut crows: Query<
        (Entity, &mut CrowState, &mut Velocity, &mut DesiredVelocity),
        (With<Crow>, Without<LeaderCrow>),
    >,
    carrying: Query<&Carrying>,
) {
    for (entity, mut state, mut velocity, mut desired) in &mut crows {
        if let Ok(carrying) = carrying.get(entity) {
            commands.entity(carrying.0).despawn();
        }
        commands.entity(entity).remove::<(InjuredTimer, Carrying)>();
        *state = CrowState::FollowLeader;
        velocity.0 = Vec3::ZERO;
        desired.0 = Vec3::ZERO;
    }
}

fn move_leader(
    fire: On<Fire<MoveLeader>>,
    leader: Single<(&mut Transform, &mut Velocity), With<LeaderCrow>>,
    time: Res<Time>,
) {
    let (mut transform, mut velocity) = leader.into_inner();
    velocity.0 = fire.value * LEADER_SPEED;
    transform.translation += velocity.0 * time.delta_secs();
    if velocity.0.length_squared() > LEADER_FACING_EPSILON_SQUARED {
        transform.look_to(velocity.0.normalize(), Vec3::Y);
    }
}

fn stop_leader(_: On<Complete<MoveLeader>>, mut leader: Single<&mut Velocity, With<LeaderCrow>>) {
    leader.0 = Vec3::ZERO;
}

impl CrowState {
    pub fn accepts_commands(&self) -> bool {
        matches!(
            self,
            Self::FollowLeader | Self::SeekTarget(_) | Self::GrabCarryable(_) | Self::Mobbing(_)
        )
    }

    pub fn is_attackable(&self) -> bool {
        !matches!(self, Self::CapturedBy(_) | Self::RecoveringFromInjury)
    }
}

impl Default for InjuredTimer {
    fn default() -> Self {
        Self(Timer::new(
            Duration::from_secs(INJURY_RECOVERY_SECONDS),
            TimerMode::Once,
        ))
    }
}
