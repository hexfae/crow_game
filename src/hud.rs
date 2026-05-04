use bevy::prelude::*;

use crate::world::Score;

#[derive(Component)]
struct ScoreText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, update_score);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Text::new("recovered: "),
            Node {
                justify_self: JustifySelf::Center,
                ..default()
            },
        ))
        .with_child((ScoreText, TextSpan::new("0")));
}

fn update_score(mut text: Single<&mut TextSpan, With<ScoreText>>, score: Res<Score>) {
    if score.is_changed() {
        text.0 = score.0.to_string();
    }
}
