use bevy::prelude::*;

use crate::world::{Injured, MissionPhase, Score, WorldTimer};

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct TimeText;

#[derive(Component)]
struct PhaseText;

#[derive(Component)]
struct NightFade;

#[derive(Component)]
struct ResultsScreen;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup, spawn_fade))
            .add_systems(Update, (update_score, update_time, update_phase, fade))
            .add_systems(OnEnter(MissionPhase::Results), spawn_results)
            .add_systems(OnExit(MissionPhase::Results), despawn_results);
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
            parent.spawn(Text::new("phase: ")).with_child((
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

fn spawn_fade(mut commands: Commands) {
    commands.spawn((
        NightFade,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.)),
        ZIndex(10),
    ));
}

fn fade(mut fade: Single<&mut BackgroundColor, With<NightFade>>, timer: Res<WorldTimer>) {
    fade.0 = Color::srgba(0., 0., 0., timer.night_fade_alpha());
}

fn despawn_results(mut commands: Commands, results: Query<Entity, With<ResultsScreen>>) {
    for entity in results {
        commands.entity(entity).despawn();
    }
}

fn spawn_results(
    mut commands: Commands,
    score: Res<Score>,
    injured: Res<Injured>,
    timer: Res<WorldTimer>,
) {
    let elapsed = timer.0.elapsed().as_secs();
    let minutes = elapsed / 60;
    let seconds = elapsed % 60;
    commands
        .spawn((
            ResultsScreen,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(24.),
                ..default()
            },
            ZIndex(20),
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("day complete"));
            parent.spawn(Text::new(format!("recovered: {}", score.0)));
            parent.spawn(Text::new(format!("injured: {}", injured.0)));
            parent.spawn(Text::new(format!("time: {minutes}m {seconds}s")));
        });
}
