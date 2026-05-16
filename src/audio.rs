use bevy::prelude::*;
use bevy_seedling::prelude::*;

use crate::crow::{Crow, CrowState};

#[derive(Resource)]
struct SoundBank {
    distress_caw: Handle<AudioSample>,
}

#[derive(Component)]
struct DistressAudio;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SeedlingPlugin::default())
            .add_systems(Startup, load_bank)
            .add_systems(Update, (attach_listener, manage_distress_audio));
    }
}

fn load_bank(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(SoundBank {
        distress_caw: server.load("sounds/distress_caw.ogg"),
    });
}

fn attach_listener(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for entity in &cameras {
        commands.entity(entity).insert(SpatialListener3D);
    }
}

fn manage_distress_audio(
    mut commands: Commands,
    crows: Query<(Entity, &CrowState, Option<&Children>), (With<Crow>, Changed<CrowState>)>,
    distress_audio: Query<(), With<DistressAudio>>,
    bank: Res<SoundBank>,
) {
    for (crow, state, children) in &crows {
        let existing = children
            .and_then(|c| c.iter().find(|&child| distress_audio.contains(child)));
        match (state, existing) {
            (CrowState::CapturedBy(_), None) => {
                commands.entity(crow).with_child((
                    DistressAudio,
                    SamplePlayer::new(bank.distress_caw.clone()).looping(),
                    Transform::default(),
                    sample_effects![SpatialBasicNode::default()],
                ));
            }
            (state, Some(audio)) if !matches!(state, CrowState::CapturedBy(_)) => {
                commands.entity(audio).despawn();
            }
            _ => {}
        }
    }
}
