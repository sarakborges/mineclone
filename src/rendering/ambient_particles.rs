use std::collections::HashMap;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        ambient_particle::{AmbientParticleDefinition, AmbientParticleRange},
        ambient_particle_registry::{AmbientParticleRegistry, AmbientParticleRule, AmbientParticleSource},
        fluid::{FluidId, FluidRegistry},
    },
    player::camera::GameplayCamera,
    rendering::wind::Wind,
    voxel::world::VoxelWorld,
    world::{biome::CurrentBiome, current_context::CurrentDimensionContext},
};

const MAX_ACTIVE_PARTICLES: usize = 512;
const MAX_PARTICLE_DISTANCE: f32 = 64.0;
const FLUID_SURFACE_ATTEMPTS: usize = 6;
const AMBIENT_POSITION_ATTEMPTS: usize = 4;

pub(crate) struct AmbientParticlesPlugin;

impl Plugin for AmbientParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientParticleRuntime>()
            .init_resource::<AmbientParticleAssets>()
            .add_systems(
                Update,
                (spawn_ambient_particles, update_ambient_particles)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(GameState::Gameplay), clear_ambient_particles);
    }
}

#[derive(Component)]
struct AmbientParticle {
    velocity: Vec3,
    acceleration: Vec3,
    lifetime: f32,
    age: f32,
    base_scale: f32,
    wander_strength: f32,
    phase: Vec3,
}

#[derive(Resource)]
struct AmbientParticleRuntime {
    emission_remainders: HashMap<String, f32>,
    random_state: u64,
}

impl Default for AmbientParticleRuntime {
    fn default() -> Self {
        Self {
            emission_remainders: HashMap::new(),
            random_state: 0xa57e_21c4_d913_7b6f,
        }
    }
}

impl AmbientParticleRuntime {
    fn take_emissions(&mut self, id: &str, rate: f32, delta_seconds: f32) -> usize {
        let remainder = self.emission_remainders.entry(id.to_owned()).or_default();
        *remainder += rate * delta_seconds;
        let count = remainder.floor() as usize;
        *remainder -= count as f32;
        count
    }

    fn unit(&mut self) -> f32 {
        self.random_state = self
            .random_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.random_state >> 40) as u32) as f32 / 16_777_215.0
    }

    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    fn range(&mut self, range: AmbientParticleRange) -> f32 {
        range.min + (range.max - range.min) * self.unit()
    }

    fn jittered_velocity(&mut self, particle: &AmbientParticleDefinition) -> Vec3 {
        Vec3::from_array(particle.velocity)
            + Vec3::new(
                self.signed() * particle.velocity_jitter[0],
                self.signed() * particle.velocity_jitter[1],
                self.signed() * particle.velocity_jitter[2],
            )
    }
}

#[derive(Resource, Default)]
struct AmbientParticleAssets {
    mesh: Option<Handle<Mesh>>,
    materials: HashMap<String, Handle<StandardMaterial>>,
}

#[derive(SystemParam)]
struct AmbientParticleEnvironment<'w> {
    wind: Res<'w, Wind>,
    world: Res<'w, VoxelWorld>,
}

fn spawn_ambient_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<&Transform, With<GameplayCamera>>,
    dimension: CurrentDimensionContext,
    current_biome: Res<CurrentBiome>,
    registry: Res<AmbientParticleRegistry>,
    fluids: Res<FluidRegistry>,
    environment: AmbientParticleEnvironment,
    mut runtime: ResMut<AmbientParticleRuntime>,
    mut particle_assets: ResMut<AmbientParticleAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    active_particles: Query<Entity, With<AmbientParticle>>,
) {
    let mut active_count = active_particles.iter().count();
    if active_count >= MAX_ACTIVE_PARTICLES {
        return;
    }

    let delta_seconds = time.delta_secs().min(0.1);
    let camera_position = camera.translation;

    for rule in registry.iter() {
        if active_count >= MAX_ACTIVE_PARTICLES {
            break;
        }

        let Some(source) = active_source(rule, &dimension, &current_biome, &fluids) else {
            continue;
        };
        let requested = runtime.take_emissions(
            &rule.id,
            rule.particle.spawn_rate * source.weight,
            delta_seconds,
        );
        let count = requested.min(MAX_ACTIVE_PARTICLES - active_count);
        if count == 0 {
            continue;
        }

        let (mesh, material) = resolve_particle_assets(
            rule,
            &mut particle_assets,
            &mut meshes,
            &mut materials,
        );
        for _ in 0..count {
            let position = match source.fluid_id {
                Some(fluid_id) => find_fluid_surface_position(
                    camera_position,
                    fluid_id,
                    &rule.particle,
                    &environment.world,
                    &mut runtime,
                ),
                None => find_ambient_position(
                    camera_position,
                    &rule.particle,
                    &environment.world,
                    &mut runtime,
                ),
            };
            let Some(position) = position else {
                continue;
            };

            let size = runtime.range(rule.particle.size);
            let lifetime = runtime.range(rule.particle.lifetime);
            let velocity = runtime.jittered_velocity(&rule.particle)
                + environment.wind.velocity(rule.particle.wind_influence);
            let phase = Vec3::new(
                runtime.unit() * std::f32::consts::TAU,
                runtime.unit() * std::f32::consts::TAU,
                runtime.unit() * std::f32::consts::TAU,
            );
            commands.spawn((
                AmbientParticle {
                    velocity,
                    acceleration: Vec3::from_array(rule.particle.acceleration),
                    lifetime,
                    age: 0.0,
                    base_scale: size,
                    wander_strength: rule.particle.wander_strength,
                    phase,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position).with_scale(Vec3::splat(size)),
            ));
            active_count += 1;
        }
    }
}

struct ActiveParticleSource {
    weight: f32,
    fluid_id: Option<FluidId>,
}

fn active_source(
    rule: &AmbientParticleRule,
    dimension: &CurrentDimensionContext<'_>,
    current_biome: &CurrentBiome,
    fluids: &FluidRegistry,
) -> Option<ActiveParticleSource> {
    match &rule.source {
        AmbientParticleSource::Dimension { id } => {
            (dimension.id().as_str() == id).then_some(ActiveParticleSource {
                weight: 1.0,
                fluid_id: None,
            })
        }
        AmbientParticleSource::Biome { id } => current_biome
            .influences
            .iter()
            .find(|influence| influence.id == *id && influence.weight > 0.0)
            .map(|influence| ActiveParticleSource {
                weight: influence.weight,
                fluid_id: None,
            }),
        AmbientParticleSource::FluidSurface { id } => {
            fluids.id_of(id).map(|fluid_id| ActiveParticleSource {
                weight: 1.0,
                fluid_id: Some(fluid_id),
            })
        }
    }
}

fn resolve_particle_assets(
    rule: &AmbientParticleRule,
    assets: &mut AmbientParticleAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let mesh = assets
        .mesh
        .get_or_insert_with(|| meshes.add(Cuboid::new(1.0, 1.0, 1.0)))
        .clone();
    let material = assets
        .materials
        .entry(rule.id.clone())
        .or_insert_with(|| {
            let [red, green, blue] = rule.particle.color.to_srgb();
            materials.add(StandardMaterial {
                base_color: Color::srgba(red, green, blue, rule.particle.opacity),
                alpha_mode: if rule.particle.opacity < 1.0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                perceptual_roughness: 1.0,
                unlit: true,
                ..default()
            })
        })
        .clone();
    (mesh, material)
}

fn find_ambient_position(
    center: Vec3,
    particle: &AmbientParticleDefinition,
    world: &VoxelWorld,
    runtime: &mut AmbientParticleRuntime,
) -> Option<Vec3> {
    for _ in 0..AMBIENT_POSITION_ATTEMPTS {
        let angle = runtime.unit() * std::f32::consts::TAU;
        let radius = runtime.unit().sqrt() * particle.spawn_radius;
        let position = center
            + Vec3::new(
                angle.cos() * radius,
                runtime.signed() * particle.vertical_range,
                angle.sin() * radius,
            );
        let voxel = position.floor().as_ivec3();
        if world.is_loaded_at(voxel)
            && world.cell_at(voxel).is_none()
            && world.fluid_at(voxel).is_none()
        {
            return Some(position);
        }
    }
    None
}

fn find_fluid_surface_position(
    center: Vec3,
    fluid_id: FluidId,
    particle: &AmbientParticleDefinition,
    world: &VoxelWorld,
    runtime: &mut AmbientParticleRuntime,
) -> Option<Vec3> {
    let vertical = particle.vertical_range.ceil() as i32;
    let center_voxel = center.floor().as_ivec3();

    for _ in 0..FLUID_SURFACE_ATTEMPTS {
        let angle = runtime.unit() * std::f32::consts::TAU;
        let radius = runtime.unit().sqrt() * particle.spawn_radius;
        let x = center_voxel.x + (angle.cos() * radius).round() as i32;
        let z = center_voxel.z + (angle.sin() * radius).round() as i32;
        let minimum_y = (center_voxel.y - vertical).max(0);
        let maximum_y = center_voxel.y + vertical;

        for y in (minimum_y..=maximum_y).rev() {
            let voxel = IVec3::new(x, y, z);
            let Some(fluid) = world.fluid_at(voxel) else {
                continue;
            };
            if fluid.fluid_id != fluid_id {
                continue;
            }
            let above = voxel + IVec3::Y;
            if world.fluid_at(above).is_some() || world.cell_at(above).is_some() {
                continue;
            }
            return Some(Vec3::new(
                x as f32 + runtime.unit(),
                y as f32 + fluid.height() + 0.02,
                z as f32 + runtime.unit(),
            ));
        }
    }
    None
}

fn update_ambient_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera: Single<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    mut particles: Query<(Entity, &mut AmbientParticle, &mut Transform)>,
) {
    let delta_seconds = time.delta_secs().min(0.1);
    let camera_position = camera.translation;

    for (entity, mut particle, mut transform) in &mut particles {
        particle.age += delta_seconds;
        if particle.age >= particle.lifetime
            || transform.translation.distance_squared(camera_position)
                > MAX_PARTICLE_DISTANCE * MAX_PARTICLE_DISTANCE
        {
            commands.entity(entity).despawn();
            continue;
        }

        let wander = Vec3::new(
            (particle.phase.x + particle.age * 1.7).sin(),
            (particle.phase.y + particle.age * 1.3).sin(),
            (particle.phase.z + particle.age * 1.9).sin(),
        ) * particle.wander_strength;
        let acceleration = particle.acceleration;
        particle.velocity += (acceleration + wander) * delta_seconds;
        transform.translation += particle.velocity * delta_seconds;

        let voxel = transform.translation.floor().as_ivec3();
        if !world.is_loaded_at(voxel) || world.cell_at(voxel).is_some() {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = particle.age / particle.lifetime;
        let fade = if progress < 0.12 {
            progress / 0.12
        } else if progress > 0.82 {
            (1.0 - progress) / 0.18
        } else {
            1.0
        }
        .clamp(0.0, 1.0);
        transform.scale = Vec3::splat(particle.base_scale * fade.max(0.05));
    }
}

fn clear_ambient_particles(
    mut commands: Commands,
    particles: Query<Entity, With<AmbientParticle>>,
    mut runtime: ResMut<AmbientParticleRuntime>,
) {
    for entity in &particles {
        commands.entity(entity).despawn();
    }
    runtime.emission_remainders.clear();
}
