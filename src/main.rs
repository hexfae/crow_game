#![expect(clippy::type_complexity)]

use bevy::prelude::*;

use crate::{
    audio::AudioPlugin, camera::CameraPlugin, cat::CatPlugin, crow::CrowPlugin,
    flock::FlockPlugin, hawk::HawkPlugin, hazard::HazardPlugin, hud::HudPlugin,
    input::InputPlugin, particles::ParticlesPlugin, roster::RosterPlugin, world::WorldPlugin,
};

mod audio;
mod camera;
mod cat;
mod crow;
mod flock;
mod hawk;
mod hazard;
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
            CatPlugin,
            CrowPlugin,
            FlockPlugin,
            HawkPlugin,
            HazardPlugin,
            HudPlugin,
            InputPlugin,
            ParticlesPlugin,
            RosterPlugin,
            WorldPlugin,
        ))
        .run();
}
