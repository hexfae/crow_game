use bevy::prelude::*;
use bevy_seedling::prelude::*;

use crate::crow::{Crow, CrowState};

#[derive(Resource)]
struct SoundBank {
    distress_caw: Handle<AudioSample>,
    pickup: Handle<AudioSample>,
    deposit: Handle<AudioSample>,
    impact: Handle<AudioSample>,
    command_caw: Handle<AudioSample>,
    recall_caw: Handle<AudioSample>,
}

#[derive(Component)]
struct DistressAudio;

#[derive(Component)]
struct DistressDelay(Timer);

const DISTRESS_DELAY_SECONDS: f32 = 1.0;

#[derive(Clone, Copy)]
pub enum Sfx {
    Pickup,
    Deposit,
    Impact,
    Command,
    Recall,
}

#[derive(Event)]
pub struct PlaySfx {
    pub sound: Sfx,
    pub position: Vec3,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SeedlingPlugin::default())
            .add_systems(Startup, load_bank)
            .add_systems(
                Update,
                (attach_listener, manage_distress_state, tick_distress_delay),
            )
            .add_observer(on_play_sfx);
    }
}

fn load_bank(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(SoundBank {
        distress_caw: server.load("sounds/distress_caw.ogg"),
        pickup: server.load("sounds/pickup.ogg"),
        deposit: server.load("sounds/deposit.ogg"),
        impact: server.load("sounds/impact.ogg"),
        command_caw: server.load("sounds/command_caw.ogg"),
        recall_caw: server.load("sounds/recall_caw.ogg"),
    });
}

fn attach_listener(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for entity in &cameras {
        commands.entity(entity).insert(SpatialListener3D);
    }
}

fn on_play_sfx(fire: On<PlaySfx>, mut commands: Commands, bank: Res<SoundBank>) {
    let handle = match fire.sound {
        Sfx::Pickup => bank.pickup.clone(),
        Sfx::Deposit => bank.deposit.clone(),
        Sfx::Impact => bank.impact.clone(),
        Sfx::Command => bank.command_caw.clone(),
        Sfx::Recall => bank.recall_caw.clone(),
    };
    commands.spawn((
        SamplePlayer::new(handle),
        Transform::from_translation(fire.position),
        sample_effects![SpatialBasicNode::default()],
    ));
}

fn manage_distress_state(
    mut commands: Commands,
    crows: Query<(Entity, &CrowState, Option<&Children>), (With<Crow>, Changed<CrowState>)>,
    distress_audio: Query<(), With<DistressAudio>>,
    delays: Query<(), With<DistressDelay>>,
) {
    for (crow, state, children) in &crows {
        let audio_child = children
            .and_then(|c| c.iter().find(|&child| distress_audio.contains(child)));
        if matches!(state, CrowState::CapturedBy(_)) {
            if audio_child.is_none() && !delays.contains(crow) {
                commands.entity(crow).insert(DistressDelay(Timer::from_seconds(
                    DISTRESS_DELAY_SECONDS,
                    TimerMode::Once,
                )));
            }
        } else {
            if let Some(audio) = audio_child {
                commands.entity(audio).despawn();
            }
            commands.entity(crow).try_remove::<DistressDelay>();
        }
    }
}

fn tick_distress_delay(
    mut commands: Commands,
    mut delayed: Query<(Entity, &mut DistressDelay)>,
    time: Res<Time>,
    bank: Res<SoundBank>,
) {
    for (crow, mut delay) in &mut delayed {
        delay.0.tick(time.delta());
        if delay.0.just_finished() {
            commands.entity(crow).with_child((
                DistressAudio,
                SamplePlayer::new(bank.distress_caw.clone()).looping(),
                Transform::default(),
                sample_effects![SpatialBasicNode::default()],
            ));
            commands.entity(crow).remove::<DistressDelay>();
        }
    }
}
