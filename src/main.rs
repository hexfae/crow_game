use bevy::prelude::*;

use crate::{camera::CameraPlugin, world::WorldPlugin};

mod camera;
mod world;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CameraPlugin, WorldPlugin))
        .run();
}
