use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_hanabi::Gradient;
use bevy_hanabi::prelude::*;

const FEATHER_CAPACITY: u32 = 64;
const FEATHER_BURST_COUNT: f32 = 32.0;
const FEATHER_SPAWN_RADIUS: f32 = 0.05;
const FEATHER_MIN_SPEED: f32 = 0.5;
const FEATHER_MAX_SPEED: f32 = 1.5;
const FEATHER_LIFETIME: f32 = 1.0;
const FEATHER_SPIN_RANGE: f32 = 8.0;
const FEATHER_GRAVITY: Vec3 = Vec3::new(0.0, -1.5, 0.0);
const FEATHER_SIZE: f32 = 0.45;
const FEATHER_FADE_START: f32 = 0.7;
const EFFECT_CLEANUP_SECONDS: f32 = 1.5;

#[derive(Resource)]
struct EffectBank {
    feather_burst: Handle<EffectAsset>,
    feather_texture: Handle<Image>,
}

#[derive(Component)]
struct Lifetime(Timer);

#[derive(Clone, Copy)]
pub enum Particle {
    FeatherBurst,
}

#[derive(Event)]
pub struct SpawnParticles {
    pub kind: Particle,
    pub position: Vec3,
}

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HanabiPlugin)
            .add_systems(Startup, load_bank)
            .add_systems(Update, tick_lifetimes)
            .add_observer(on_spawn_particles);
    }
}

fn load_bank(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
    server: Res<AssetServer>,
) {
    commands.insert_resource(EffectBank {
        feather_burst: effects.add(feather_burst()),
        feather_texture: server.load("textures/feather.png"),
    });
}

fn feather_burst() -> EffectAsset {
    let writer = ExprWriter::new();
    let init_position = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(FEATHER_SPAWN_RADIUS).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_velocity = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer
            .lit(FEATHER_MIN_SPEED)
            .uniform(writer.lit(FEATHER_MAX_SPEED))
            .expr(),
    };
    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_life =
        SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(FEATHER_LIFETIME).expr());
    let init_rotation = SetAttributeModifier::new(
        Attribute::F32_0,
        (writer.rand(ScalarType::Float) * writer.lit(TAU)).expr(),
    );
    let init_spin = SetAttributeModifier::new(
        Attribute::F32_1,
        ((writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(FEATHER_SPIN_RANGE)).expr(),
    );
    let rotation = (writer.attr(Attribute::F32_0)
        + writer.attr(Attribute::F32_1) * writer.attr(Attribute::AGE))
    .expr();
    let gravity = AccelModifier::new(writer.lit(FEATHER_GRAVITY).expr());

    let mut color = Gradient::<Vec4>::new();
    color.add_key(0.0, Vec4::ONE);
    color.add_key(FEATHER_FADE_START, Vec4::ONE);
    color.add_key(1.0, Vec4::new(1.0, 1.0, 1.0, 0.0));

    let mut module = writer.finish();
    module.add_texture_slot("feather");
    let feather_slot = module.lit(0u32);

    EffectAsset::new(
        FEATHER_CAPACITY,
        SpawnerSettings::once(FEATHER_BURST_COUNT.into()),
        module,
    )
    .with_name("feather_burst")
    .init(init_position)
    .init(init_velocity)
    .init(init_age)
    .init(init_life)
    .init(init_rotation)
    .init(init_spin)
    .update(gravity)
    .render(ColorOverLifetimeModifier {
        gradient: color,
        blend: ColorBlendMode::Overwrite,
        mask: ColorBlendMask::RGBA,
    })
    .render(SetSizeModifier {
        size: Vec3::splat(FEATHER_SIZE).into(),
    })
    .render(ParticleTextureModifier {
        texture_slot: feather_slot,
        sample_mapping: ImageSampleMapping::Modulate,
    })
    .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane).with_rotation(rotation))
}

fn on_spawn_particles(fire: On<SpawnParticles>, mut commands: Commands, bank: Res<EffectBank>) {
    let handle = match fire.kind {
        Particle::FeatherBurst => bank.feather_burst.clone(),
    };
    commands.spawn((
        ParticleEffect::new(handle),
        EffectMaterial {
            images: vec![bank.feather_texture.clone()],
        },
        Transform::from_translation(fire.position),
        Lifetime(Timer::from_seconds(EFFECT_CLEANUP_SECONDS, TimerMode::Once)),
    ));
}

fn tick_lifetimes(
    mut commands: Commands,
    mut effects: Query<(Entity, &mut Lifetime)>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in &mut effects {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
