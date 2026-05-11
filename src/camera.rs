use bevy::prelude::*;

use crate::crow::LeaderCrow;

const OFFSET: Vec3 = Vec3::new(10., 10., 15.);
const DECAY_RATE: f32 = 5.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, watch_leader);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(OFFSET).looking_at(Vec3::Y, Vec3::Y),
    ));
}

fn watch_leader(
    mut camera: Single<&mut Transform, With<Camera3d>>,
    leader: Single<&Transform, (With<LeaderCrow>, Without<Camera3d>)>,
    time: Res<Time>,
) {
    let offset = leader.translation + OFFSET;
    camera
        .translation
        .smooth_nudge(&offset, DECAY_RATE, time.delta_secs());
}
