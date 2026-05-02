use bevy::prelude::*;
use rand::RngExt;

pub struct CrowPlugin;

#[derive(Component)]
pub struct Crow;

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
        .add_systems(FixedUpdate, integrate.in_set(CrowSystems::Integrate));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = rand::rng();
    for _ in 0..50 {
        let position = Vec3::new(
            rng.random_range(-5.0..5.),
            rng.random_range(5.0..10.),
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
            transform.look_to(velocity.0.normalize(), Vec3::Y);
        }
    }
}
