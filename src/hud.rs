use bevy::prelude::*;

use crate::world::{MissionPhase, Score, WorldTimer};

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct PhaseText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (update_score, update_time, update_phase));
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(24.),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Text::new("recovered: "))
                .with_child((ScoreText, TextSpan::new("0")));
            parent
                .spawn(Text::new("time: "))
                .with_child((TimeText, TextSpan::new("0m 0s")));
            parent
                .spawn(Text::new("phase: "))
                .with_child((
                    PhaseText,
                    TextSpan::new(MissionPhase::default().to_string()),
                ));
        });
}

fn update_score(mut text: Single<&mut TextSpan, With<ScoreText>>, score: Res<Score>) {
    if score.is_changed() {
        text.0 = score.0.to_string();
    }
}

fn update_time(
    mut text: Single<&mut TextSpan, With<TimeText>>,
    timer: Res<WorldTimer>,
    phase: Res<State<MissionPhase>>,
) {
    let remaining = timer.remaining_in(*phase.get()).as_secs_f32();
    let minutes = (remaining / 60.) as u32;
    let seconds = remaining % 60.;
    text.0 = format!("{minutes}m {seconds:04.1}s");
}

fn update_phase(mut text: Single<&mut TextSpan, With<PhaseText>>, phase: Res<State<MissionPhase>>) {
    if phase.is_changed() {
        text.0 = phase.get().to_string();
    }
}
