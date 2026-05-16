use bevy::prelude::*;
use bevy_hanabi::Gradient;
use bevy_hanabi::prelude::*;

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
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.05).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(0.5).uniform(writer.lit(1.5)).expr(),
    };
    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_life = SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(1.0).expr());
    let init_rotation = SetAttributeModifier::new(
        Attribute::F32_0,
        (writer.rand(ScalarType::Float) * writer.lit(std::f32::consts::TAU)).expr(),
    );
    let init_spin = SetAttributeModifier::new(
        Attribute::F32_1,
        ((writer.rand(ScalarType::Float) - writer.lit(0.5)) * writer.lit(8.0)).expr(),
    );
    let rotation =
        (writer.attr(Attribute::F32_0) + writer.attr(Attribute::F32_1) * writer.attr(Attribute::AGE))
            .expr();
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -1.5, 0.0)).expr());

    let mut color = Gradient::<Vec4>::new();
    color.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0));
    color.add_key(0.7, Vec4::new(1.0, 1.0, 1.0, 1.0));
    color.add_key(1.0, Vec4::new(1.0, 1.0, 1.0, 0.0));

    let mut module = writer.finish();
    module.add_texture_slot("feather");
    let slot_zero = module.lit(0u32);

    EffectAsset::new(64, SpawnerSettings::once(32.0.into()), module)
        .with_name("feather_burst")
        .init(init_pos)
        .init(init_vel)
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
            size: Vec3::splat(0.45).into(),
        })
        .render(ParticleTextureModifier {
            texture_slot: slot_zero,
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
        Lifetime(Timer::from_seconds(1.5, TimerMode::Once)),
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
