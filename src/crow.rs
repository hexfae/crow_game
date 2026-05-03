use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use rand::RngExt;

use crate::input::{Direct, MoveLeader, Player, Recall};

pub struct CrowPlugin;

#[derive(Component)]
pub struct Crow;

#[derive(Component)]
pub struct LeaderCrow;

#[derive(Default, Component)]
pub struct Velocity(pub Vec3);

#[derive(Default, Component)]
pub struct DesiredVelocity(pub Vec3);

#[derive(Default, Component)]
pub struct FlockNeighbors(pub Vec<Entity>);

#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub enum CrowSystems {
    Steer,
    Integrate,
}

impl Plugin for CrowPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (CrowSystems::Steer, CrowSystems::Integrate).chain(),
        )
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, integrate.in_set(CrowSystems::Integrate))
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
    mut crows: Query<(&DesiredVelocity, &mut Velocity, &mut Transform), With<Crow>>,
) {
    let dt = time.delta_secs();
    let smoothing = (5.0 * dt).min(1.0);
    for (desired, mut velocity, mut transform) in &mut crows {
        velocity.0 = velocity.0.lerp(desired.0, smoothing);
        transform.translation += velocity.0 * dt;
        if velocity.0.length_squared() > 0.001 {
            let direction = velocity.0.normalize();
            transform.look_to(direction, Vec3::Y);
        }
    }
}

fn move_leader(
    fire: On<Fire<MoveLeader>>,
    mut leaders: Query<(&mut Transform, &mut Velocity), With<LeaderCrow>>,
    time: Res<Time>,
) {
    const LEADER_SPEED: f32 = 4.0;
    let Ok((mut transform, mut velocity)) = leaders.get_mut(fire.context) else {
        return;
    };
    velocity.0 = fire.value * LEADER_SPEED;
    transform.translation += velocity.0 * time.delta_secs();
    if velocity.0.length_squared() > 0.001 {
        let direction = velocity.0.normalize();
        transform.look_to(direction, Vec3::Y);
    }
}

fn stop_leader(
    complete: On<Complete<MoveLeader>>,
    mut leaders: Query<&mut Velocity, With<LeaderCrow>>,
) {
    let Ok(mut velocity) = leaders.get_mut(complete.context) else {
        return;
    };
    velocity.0 = Vec3::ZERO;
}
