use std::f32::consts::{FRAC_PI_3, FRAC_PI_4};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::{crow::LeaderCrow, input::Zoom, world::MissionPhase};

const POSITION_DECAY: f32 = 5.0;
const ROTATION_DECAY: f32 = 5.0;
const FOV_DECAY: f32 = 5.0;
const ZOOM_SENSITIVITY: f32 = 0.1;

#[derive(Clone, Copy)]
struct ZoomView {
    arm: Vec3,
    aim: Vec3,
    fov: f32,
}

const STREET_VIEW: ZoomView = ZoomView {
    arm: Vec3::new(2.0, 1.5, 3.0),
    aim: Vec3::new(0.0, 1.0, 0.0),
    fov: FRAC_PI_3,
};

const CHASE_VIEW: ZoomView = ZoomView {
    arm: Vec3::new(10.0, 10.0, 15.0),
    aim: Vec3::ZERO,
    fov: FRAC_PI_4,
};

const BIRDSEYE_VIEW: ZoomView = ZoomView {
    arm: Vec3::new(2.0, 25.0, 3.5),
    aim: Vec3::ZERO,
    fov: FRAC_PI_4,
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup)
            .add_systems(Update, (derive_from_zoom, drive_rig).chain())
            .add_systems(OnExit(MissionPhase::Results), request_snap)
            .add_observer(on_zoom);
    }
}

#[derive(Component)]
pub struct CameraRig {
    pub follow: Follow,
    pub zoom: f32,
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
    let zoom = 0.5;
    let (arm, rotation, fov) = arm_from_zoom(zoom);

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(arm).with_rotation(rotation),
        CameraRig {
            follow: Follow::Entity(*leader),
            zoom,
            arm_offset: arm,
            desired_rotation: rotation,
            desired_fov: fov,
            position_decay: POSITION_DECAY,
            rotation_decay: ROTATION_DECAY,
            fov_decay: FOV_DECAY,
            snap: true,
        },
    ));
}

fn arm_from_zoom(zoom: f32) -> (Vec3, Quat, f32) {
    let (from, to, segment_t) = if zoom < 0.5 {
        (STREET_VIEW, CHASE_VIEW, zoom * 2.0)
    } else {
        (CHASE_VIEW, BIRDSEYE_VIEW, (zoom - 0.5) * 2.0)
    };
    let t = EaseFunction::SmoothStep.sample_unchecked(segment_t.clamp(0., 1.));
    let arm = from.arm.lerp(to.arm, t);
    let aim = from.aim.lerp(to.aim, t);
    let fov = from.fov.lerp(to.fov, t);
    let rotation = Transform::from_translation(arm)
        .looking_at(aim, Vec3::Y)
        .rotation;
    (arm, rotation, fov)
}

fn derive_from_zoom(mut rig: Single<&mut CameraRig>) {
    let (arm, rotation, fov) = arm_from_zoom(rig.zoom);
    rig.arm_offset = arm;
    rig.desired_rotation = rotation;
    rig.desired_fov = fov;
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

fn on_zoom(fire: On<Fire<Zoom>>, mut rig: Single<&mut CameraRig>) {
    rig.zoom = (rig.zoom - fire.value * ZOOM_SENSITIVITY).clamp(0., 1.);
}
