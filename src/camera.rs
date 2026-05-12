use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;

use crate::{crow::LeaderCrow, world::MissionPhase};

const ARM_OFFSET: Vec3 = Vec3::new(10., 10., 15.);
const POSITION_DECAY: f32 = 5.0;
const ROTATION_DECAY: f32 = 5.0;
const FOV_DECAY: f32 = 5.0;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup)
            .add_systems(Update, drive_rig)
            .add_systems(OnExit(MissionPhase::Results), request_snap);
    }
}

#[derive(Component)]
pub struct CameraRig {
    pub follow: Follow,
    pub arm_offset: Vec3,
    pub desired_rotation: Quat,
    pub desired_fov: f32,
    pub position_decay: f32,
    pub rotation_decay: f32,
    pub fov_decay: f32,
    pub snap: bool,
}

pub enum Follow {
    Entity(Entity),
    Fixed(Vec3),
}

fn setup(mut commands: Commands, leader: Single<Entity, With<LeaderCrow>>) {
    let initial = Transform::from_translation(ARM_OFFSET).looking_at(Vec3::Y, Vec3::Y);

    commands.spawn((
        Camera3d::default(),
        initial,
        CameraRig {
            follow: Follow::Entity(*leader),
            arm_offset: ARM_OFFSET,
            desired_rotation: initial.rotation,
            desired_fov: FRAC_PI_4,
            position_decay: POSITION_DECAY,
            rotation_decay: ROTATION_DECAY,
            fov_decay: FOV_DECAY,
            snap: true,
        },
    ));
}

fn drive_rig(
    camera: Single<(&mut Transform, &mut CameraRig, &mut Projection), With<Camera3d>>,
    transforms: Query<&Transform, Without<Camera3d>>,
    time: Res<Time>,
) {
    let (mut transform, mut rig, mut projection) = camera.into_inner();

    let origin = match rig.follow {
        Follow::Entity(entity) => {
            let Ok(target) = transforms.get(entity) else {
                return;
            };
            target.translation
        }
        Follow::Fixed(position) => position,
    };
    let desired_position = origin + rig.arm_offset;
    let dt = time.delta_secs();

    if rig.snap {
        transform.translation = desired_position;
        transform.rotation = rig.desired_rotation;
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = rig.desired_fov;
        }
        rig.snap = false;
    } else {
        transform
            .translation
            .smooth_nudge(&desired_position, rig.position_decay, dt);
        transform
            .rotation
            .smooth_nudge(&rig.desired_rotation, rig.rotation_decay, dt);
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective
                .fov
                .smooth_nudge(&rig.desired_fov, rig.fov_decay, dt);
        }
    }
}

fn request_snap(mut rig: Single<&mut CameraRig>) {
    rig.snap = true;
}
