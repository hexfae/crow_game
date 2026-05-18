use bevy::prelude::*;

use crate::{
    camera::CameraFocus,
    crow::{Crow, CrowState, LeaderCrow, Species},
};

const REBUILD_INTERVAL: f32 = 0.25;
const PORTRAIT_SIZE: f32 = 56.0;
const PORTRAIT_TOTAL_HEIGHT: f32 = 78.0;
const PORTRAIT_PADDING: f32 = 5.0;
const PORTRAIT_ICON_SIZE: f32 = PORTRAIT_SIZE - 2.0 * PORTRAIT_PADDING;
const PORTRAIT_BORDER: f32 = 2.0;
const PORTRAIT_INNER_PADDING: f32 = 3.0;
const PORTRAIT_ROW_GAP: f32 = 4.0;
const PORTRAIT_GAP: f32 = 6.0;
const PORTRAIT_LABEL_FONT: f32 = 11.0;
const ROSTER_EDGE_INSET: f32 = 16.0;
const ROSTER_MAX_HEIGHT_PERCENT: f32 = 90.0;
const ROSTER_MAX_WIDTH_PERCENT: f32 = 95.0;

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

/// Priority for sorting portraits — lower comes first, so the most urgent
/// statuses sit at the top of the roster.
#[repr(u8)]
#[derive(Clone, Copy)]
enum Priority {
    Leader = 0,
    Captured = 1,
    Mobbing = 2,
    Grabbing = 3,
    Carrying = 4,
    Seeking = 5,
    Injured = 6,
    Idle = 7,
}

struct PortraitData {
    crow: Entity,
    species: Species,
    is_leader: bool,
    is_captured: bool,
    priority: Priority,
    label: &'static str,
}

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
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::FlexEnd,
            padding: UiRect {
                right: Val::Px(ROSTER_EDGE_INSET),
                bottom: Val::Px(ROSTER_EDGE_INSET),
                ..default()
            },
            ..default()
        })
        .with_children(|outer| {
            outer.spawn((
                Roster,
                Node {
                    max_height: Val::Percent(ROSTER_MAX_HEIGHT_PERCENT),
                    max_width: Val::Percent(ROSTER_MAX_WIDTH_PERCENT),
                    flex_direction: FlexDirection::Column,
                    flex_wrap: FlexWrap::WrapReverse,
                    column_gap: Val::Px(PORTRAIT_GAP),
                    row_gap: Val::Px(PORTRAIT_GAP),
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

    let mut entries: Vec<PortraitData> = crows
        .iter()
        .map(|(entity, state, species, is_leader)| {
            let (priority, label) = classify(state, is_leader);
            PortraitData {
                crow: entity,
                species: *species,
                is_leader,
                is_captured: matches!(state, Some(CrowState::CapturedBy(_))),
                priority,
                label,
            }
        })
        .collect();
    entries.sort_by_key(|entry| {
        (
            entry.priority as u8,
            -species_rank(entry.species),
            entry.crow.index(),
        )
    });

    for portrait in &old_portraits {
        commands.entity(portrait).despawn();
    }

    commands.entity(*roster).with_children(|parent| {
        for entry in entries {
            spawn_portrait(parent, &entry, &textures);
        }
    });
}

fn spawn_portrait(parent: &mut ChildSpawnerCommands, entry: &PortraitData, textures: &PortraitTextures) {
    let icon = match entry.species {
        Species::Carrion => textures.carrion.clone(),
        Species::Raven => textures.raven.clone(),
    };
    let border = if entry.is_captured {
        Color::srgb(0.9, 0.15, 0.15)
    } else if entry.is_leader {
        Color::srgb(1.0, 0.6, 0.2)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.6)
    };
    parent
        .spawn((
            PortraitFor(entry.crow),
            Button,
            Node {
                width: Val::Px(PORTRAIT_SIZE),
                height: Val::Px(PORTRAIT_TOTAL_HEIGHT),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(PORTRAIT_INNER_PADDING)),
                row_gap: Val::Px(PORTRAIT_ROW_GAP),
                border: UiRect::all(Val::Px(PORTRAIT_BORDER)),
                ..default()
            },
            BorderColor::all(border),
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.7)),
        ))
        .with_children(|portrait| {
            portrait.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(PORTRAIT_ICON_SIZE),
                    height: Val::Px(PORTRAIT_ICON_SIZE),
                    ..default()
                },
            ));
            portrait.spawn((
                Text::new(entry.label),
                TextFont {
                    font_size: PORTRAIT_LABEL_FONT,
                    ..default()
                },
            ));
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

fn classify(state: Option<&CrowState>, is_leader: bool) -> (Priority, &'static str) {
    if is_leader {
        return (Priority::Leader, "leader");
    }
    let Some(state) = state else {
        return (Priority::Idle, "idle");
    };
    match state {
        CrowState::CapturedBy(_) => (Priority::Captured, "caught"),
        CrowState::Mobbing(_) => (Priority::Mobbing, "mob"),
        CrowState::GrabCarryable(_) => (Priority::Grabbing, "grab"),
        CrowState::ReturningToRoost => (Priority::Carrying, "carry"),
        CrowState::SeekTarget(_) => (Priority::Seeking, "seek"),
        CrowState::RecoveringFromInjury => (Priority::Injured, "hurt"),
        CrowState::FollowLeader => (Priority::Idle, "idle"),
    }
}

fn species_rank(species: Species) -> i32 {
    match species {
        Species::Raven => 1,
        Species::Carrion => 0,
    }
}
