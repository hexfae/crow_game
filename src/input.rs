use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext};

pub struct InputPlugin;

#[derive(Component)]
pub struct Player;

#[derive(InputAction)]
#[action_output(Vec3)]
pub struct MoveLeader;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Direct;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Recall;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Restart;

#[derive(InputAction)]
#[action_output(f32)]
pub struct Zoom;

#[derive(InputAction)]
#[action_output(Vec3)]
pub struct PanCamera;

#[derive(Resource, Default)]
pub struct CommandCursor {
    pub world_position: Option<Vec3>,
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<Player>()
            .init_resource::<CommandCursor>()
            .add_systems(Update, update_cursor);
    }
}

fn update_cursor(
    mut cursor: ResMut<CommandCursor>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut egui_contexts: Query<&mut EguiContext, With<PrimaryEguiContext>>,
    ui_interactions: Query<&Interaction>,
) {
    let egui_wants_pointer = egui_contexts
        .single_mut()
        .is_ok_and(|mut ctx| ctx.get_mut().wants_pointer_input());
    let ui_wants_pointer = ui_interactions
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Hovered | Interaction::Pressed));
    if egui_wants_pointer || ui_wants_pointer {
        cursor.world_position = None;
        return;
    }
    cursor.world_position = pointer_world_position(&windows, &cameras);
}

fn pointer_world_position(
    windows: &Query<&Window>,
    cameras: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec3> {
    let window = windows.single().ok()?;
    let screen_position = window.cursor_position()?;
    let (camera, camera_transform) = cameras.single().ok()?;
    let ray = camera
        .viewport_to_world(camera_transform, screen_position)
        .ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    Some(ray.get_point(distance))
}
