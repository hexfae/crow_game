use std::time::Duration;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use rand::RngExt;

use crate::{
    input::{Direct, MoveLeader, Player, Recall},
    world::{Carryable, Roost, Score},
};

const LEADER_SPEED: f32 = 4.0;

#[derive(Component)]
pub struct Crow;

#[derive(Component)]
pub struct LeaderCrow;

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
        .add_observer(move_leader)
        .add_observer(stop_leader);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = rand::rng();
    for _ in 0..50 {
        let position = Vec3::new(
            rng.random_range(-5.0..5.),
            rng.random_range(5.0..6.),
            rng.random_range(-5.0..5.),
        );
        let direction = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(-0.2..0.2),
            rng.random_range(-1.0..1.0),
        )
        .normalize_or_zero();

        commands.spawn((
            Crow,
            CrowState::default(),
            Velocity(direction * 2.0),
            DesiredVelocity::default(),
            FlockNeighbors::default(),
            Transform::from_translation(position).with_scale(Vec3::splat(0.2)),
            SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/crow/crow.glb")),
            ),
        ));
    }
    commands
        .spawn((
            Crow,
            LeaderCrow,
            Player,
            Velocity::default(),
            Transform::from_scale(Vec3::splat(0.2)).with_translation(Vec3::Y * 3.),
            SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/crow/crow.glb")),
            ),
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
            ]),
        ))
        .with_child((
            PointLight {
                color: Color::srgb(1.0, 0.0, 0.0),
                intensity: 200_000.0,
                ..default()
            },
            Transform::from_translation(Vec3::Y * 4.).looking_at(Vec3::ZERO, Vec3::Y),
        ));
}

fn integrate(
    time: Res<Time<Fixed>>,
    mut crows: Query<(&DesiredVelocity, &mut Velocity, &mut Transform, &CrowState), With<Crow>>,
) {
    let dt = time.delta_secs();
    let smoothing = (5.0 * dt).min(1.0);
    for (desired, mut velocity, mut transform, state) in &mut crows {
        if let CrowState::CapturedBy(_) = state {
            continue;
        }
        velocity.0 = velocity.0.lerp(desired.0, smoothing);
        transform.translation += velocity.0 * dt;
        if velocity.0.length_squared() > 0.001 {
            let direction = velocity.0.normalize();
            transform.look_to(direction, Vec3::Y);
        }
    }
}

fn pickup(
    mut commands: Commands,
    crows: Query<(&mut CrowState, &Transform, Entity)>,
    carryables: Query<&Transform, With<Carryable>>,
) {
    for (mut state, transform, crow_entity) in crows {
        if let CrowState::GrabCarryable(carryable_entity) = *state
            && let Ok(carryable) = carryables.get(carryable_entity)
            && carryable.translation.distance(transform.translation) < 0.5
        {
            commands
                .entity(crow_entity)
                .insert(Carrying(carryable_entity));
            commands
                .entity(carryable_entity)
                .remove::<Carryable>()
                .set_parent_in_place(crow_entity);
            *state = CrowState::ReturningToRoost;
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
        if let CrowState::CapturedBy(_) = *state {
            continue;
        }
        if transform.translation.distance(roost.translation) < 1.0 {
            commands.entity(entity).remove::<Carrying>();
            commands.entity(carrying.0).despawn();
            *state = CrowState::FollowLeader;
            score.0 += 1;
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
        if transform.translation.distance(roost.translation) < 1.0 {
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
}

fn move_leader(
    fire: On<Fire<MoveLeader>>,
    leader: Single<(&mut Transform, &mut Velocity), With<LeaderCrow>>,
    time: Res<Time>,
) {
    let (mut transform, mut velocity) = leader.into_inner();
    velocity.0 = fire.value * LEADER_SPEED;
    transform.translation += velocity.0 * time.delta_secs();
    if velocity.0.length_squared() > 0.01 {
        let direction = velocity.0.normalize();
        transform.look_to(direction, Vec3::Y);
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
}

impl Default for InjuredTimer {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs(5), TimerMode::Once))
    }
}
