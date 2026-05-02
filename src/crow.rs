use bevy::prelude::*;

pub struct CrowPlugin;

#[derive(Component)]
struct Crow;

impl Plugin for CrowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Crow,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/crow/crow.glb"))),
    ));
}
