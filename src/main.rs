#![expect(clippy::type_complexity)]
use bevy::prelude::*;

use crate::{
    audio::AudioPlugin, camera::CameraPlugin, cat::CatPlugin, crow::CrowPlugin,
    flock::FlockPlugin, hawk::HawkPlugin, hud::HudPlugin, input::InputPlugin,
    particles::ParticlesPlugin, roster::RosterPlugin, world::WorldPlugin,
};

mod audio;
mod camera;
mod cat;
mod crow;
mod flock;
mod hawk;
mod hud;
mod input;
mod particles;
mod roster;
mod world;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            AudioPlugin,
            CameraPlugin,
            CrowPlugin,
            FlockPlugin,
            HudPlugin,
            InputPlugin,
            ParticlesPlugin,
            RosterPlugin,
            WorldPlugin,
            CatPlugin,
            HawkPlugin,
        ))
        .run();
}
