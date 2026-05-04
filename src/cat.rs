use std::f32::consts::PI;

use bevy::prelude::*;
use rand::RngExt;

const SPEED: f32 = 5.0;

#[derive(Component)]
struct Cat;

#[derive(Component)]
struct WalkTo(Vec3);

#[derive(Component)]
struct Boredom(Timer);

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (walk, experience_boredom));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Cat,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/cat/cat.glb"))),
        Boredom(Timer::from_seconds(1., TimerMode::Repeating)),
        Transform::from_scale(Vec3::splat(0.6))
            .with_translation(Vec3::new(8., -0.5, -3.))
            .with_rotation(Quat::from_rotation_y(PI * 0.75)),
    ));
}

fn walk(mut commands: Commands, cats: Query<(Entity, &mut Transform, &WalkTo)>, time: Res<Time>) {
    for mut cat in cats {
        let offset = cat.2.0 - cat.1.translation;
        let distance = offset.length();
        if distance < 0.1 {
            commands.entity(cat.0).remove::<WalkTo>();
            continue;
        }
        let direction = offset / distance;
        let step = (SPEED * time.delta_secs()).min(distance);
        cat.1.translation += direction * step;
        cat.1.look_to(direction, Vec3::Y);
    }
}

fn experience_boredom(
    mut commands: Commands,
    cats: Query<(Entity, &mut Boredom), Without<WalkTo>>,
    time: Res<Time>,
) {
    let mut rng = rand::rng();
    for mut cat in cats {
        cat.1.0.tick(time.delta());
        if cat.1.0.just_finished() && rng.random_bool(0.5) {
            let position = Vec3::new(
                rng.random_range(-1.5..5.5),
                -0.5,
                rng.random_range(-1.5..5.5),
            );
            commands.entity(cat.0).insert(WalkTo(position));
        }
    }
}
