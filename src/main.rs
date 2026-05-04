#![expect(clippy::type_complexity)]
use bevy::prelude::*;

use crate::{
    camera::CameraPlugin, crow::CrowPlugin, flock::FlockPlugin, hud::HudPlugin, input::InputPlugin,
    world::WorldPlugin,
};

mod camera;
mod crow;
mod flock;
mod hud;
mod input;
mod world;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CameraPlugin,
            CrowPlugin,
            FlockPlugin,
            HudPlugin,
            InputPlugin,
            WorldPlugin,
        ))
        .run();
}
