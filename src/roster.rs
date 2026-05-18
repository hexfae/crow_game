use bevy::prelude::*;

use crate::{
    camera::CameraFocus,
    crow::{Crow, CrowState, LeaderCrow, Species},
};

const REBUILD_INTERVAL: f32 = 0.25;
const PORTRAIT_SIZE: f32 = 56.0;
const PORTRAIT_TOTAL_HEIGHT: f32 = 78.0;

pub struct RosterPlugin;

#[derive(Component)]
struct Roster;

#[derive(Component)]
struct PortraitFor(Entity);

#[derive(Resource)]
struct PortraitTextures {
    carrion: Handle<Image>,
    raven: Handle<Image>,
}

#[derive(Resource)]
struct RebuildTimer(Timer);

impl Plugin for RosterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RebuildTimer(Timer::from_seconds(
            REBUILD_INTERVAL,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, (load_textures, spawn_roster))
        .add_systems(Update, (rebuild, handle_clicks));
    }
}

fn load_textures(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(PortraitTextures {
        carrion: server.load("textures/carrion.png"),
        raven: server.load("textures/raven.png"),
    });
}

fn spawn_roster(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::FlexEnd,
            padding: UiRect {
                right: Val::Px(16.),
                bottom: Val::Px(16.),
                ..default()
            },
            ..default()
        })
        .with_children(|outer| {
            outer.spawn((
                Roster,
                Node {
                    max_height: Val::Percent(90.),
                    max_width: Val::Percent(95.),
                    flex_direction: FlexDirection::Column,
                    flex_wrap: FlexWrap::WrapReverse,
                    column_gap: Val::Px(6.),
                    row_gap: Val::Px(6.),
                    ..default()
                },
            ));
        });
}

fn rebuild(
    mut commands: Commands,
    mut timer: ResMut<RebuildTimer>,
    time: Res<Time>,
    roster: Single<Entity, With<Roster>>,
    old_portraits: Query<Entity, With<PortraitFor>>,
    crows: Query<(Entity, Option<&CrowState>, &Species, Has<LeaderCrow>), With<Crow>>,
    textures: Res<PortraitTextures>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut entries: Vec<(Entity, Species, bool, bool, u8, &'static str)> = crows
        .iter()
        .map(|(entity, state, species, is_leader)| {
            let (priority, label) = classify(state, is_leader);
            let is_captured = matches!(state, Some(CrowState::CapturedBy(_)));
            (entity, *species, is_leader, is_captured, priority, label)
        })
        .collect();
    entries.sort_by_key(|(entity, species, _, _, priority, _)| {
        (*priority, -species_rank(*species), entity.index())
    });

    for portrait in &old_portraits {
        commands.entity(portrait).despawn();
    }

    commands.entity(*roster).with_children(|parent| {
        for (entity, species, is_leader, is_captured, _, label) in entries {
            let icon = match species {
                Species::Carrion => textures.carrion.clone(),
                Species::Raven => textures.raven.clone(),
            };
            let border = if is_captured {
                Color::srgb(0.9, 0.15, 0.15)
            } else if is_leader {
                Color::srgb(1.0, 0.6, 0.2)
            } else {
                Color::srgba(0.0, 0.0, 0.0, 0.6)
            };
            parent
                .spawn((
                    PortraitFor(entity),
                    Button,
                    Node {
                        width: Val::Px(PORTRAIT_SIZE),
                        height: Val::Px(PORTRAIT_TOTAL_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(3.0)),
                        row_gap: Val::Px(4.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(border),
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.7)),
                ))
                .with_children(|portrait| {
                    portrait.spawn((
                        ImageNode::new(icon),
                        Node {
                            width: Val::Px(PORTRAIT_SIZE - 10.),
                            height: Val::Px(PORTRAIT_SIZE - 10.),
                            ..default()
                        },
                    ));
                    portrait.spawn((
                        Text::new(label),
                        TextFont {
                            font_size: 11.0,
                            ..default()
                        },
                    ));
                });
        }
    });
}

fn handle_clicks(
    portraits: Query<(&Interaction, &PortraitFor), Changed<Interaction>>,
    focus: Single<&mut CameraFocus>,
) {
    let Some((_, portrait)) = portraits
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    *focus.into_inner() = CameraFocus::Following(portrait.0);
}

fn classify(state: Option<&CrowState>, is_leader: bool) -> (u8, &'static str) {
    if is_leader {
        return (0, "leader");
    }
    let Some(state) = state else {
        return (7, "idle");
    };
    match state {
        CrowState::CapturedBy(_) => (1, "caught"),
        CrowState::Mobbing(_) => (2, "mob"),
        CrowState::GrabCarryable(_) => (3, "grab"),
        CrowState::ReturningToRoost => (4, "carry"),
        CrowState::SeekTarget(_) => (5, "seek"),
        CrowState::RecoveringFromInjury => (6, "hurt"),
        CrowState::FollowLeader => (7, "idle"),
    }
}

fn species_rank(species: Species) -> i32 {
    match species {
        Species::Raven => 1,
        Species::Carrion => 0,
    }
}
