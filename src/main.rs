#![expect(clippy::type_complexity)]
use bevy::prelude::*;

use crate::{
    camera::CameraPlugin, cat::CatPlugin, crow::CrowPlugin, flock::FlockPlugin, hud::HudPlugin,
    input::InputPlugin, world::WorldPlugin,
};

mod camera;
mod cat;
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
            CatPlugin,
        ))
        .run();
}
