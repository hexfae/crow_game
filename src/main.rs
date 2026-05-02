use bevy::prelude::*;

use crate::{camera::CameraPlugin, crow::CrowPlugin, world::WorldPlugin};

mod camera;
mod crow;
mod world;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CameraPlugin, CrowPlugin, WorldPlugin))
        .run();
}
