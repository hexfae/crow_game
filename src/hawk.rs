use std::f32::consts::PI;

use bevy::prelude::*;

#[derive(Component)]
struct Hawk;

pub struct HawkPlugin;

impl Plugin for HawkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(Update, soar);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Hawk,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/hawk/hawk.glb"))),
        Transform::from_scale(Vec3::splat(0.5)),
    ));
}

fn soar(hawks: Query<&mut Transform, With<Hawk>>, time: Res<Time>) {
    let elapsed = time.elapsed_secs();
    for mut hawk in hawks {
        hawk.translation = Vec3::new(elapsed.sin() * 10., 10., elapsed.cos() * 10.);
        hawk.look_at(Vec3::Y * 10., Vec3::Y);
        hawk.rotate_y(-PI / 2.);
    }
}
