use bevy::prelude::*;

use crate::{camera::CameraPlugin, crow::CrowPlugin, flock::FlockPlugin, world::WorldPlugin};

mod camera;
mod crow;
mod flock;
mod world;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CameraPlugin,
            CrowPlugin,
            FlockPlugin,
            WorldPlugin,
        ))
        .run();
}
