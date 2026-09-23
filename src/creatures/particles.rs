use std::collections::HashMap;

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::creature::{CreatureParticleEffect, CreatureRegistry},
};

use super::{CreatureAnimationState, CreatureInstance};

const MAX_PARTICLE_FRAME_DELTA_SECONDS: f32 = 0.05;
const INITIAL_RANDOM_STATE: u32 = 0xA5A5_1F3D;

#[derive(Component, Default)]
pub(super) struct CreatureParticleEmitter {
    initialized: bool,
    state: String,
    revision: u64,
    timer: f32,
}

#[derive(Component)]
pub(super) struct CreatureParticle {
    velocity: Vec3,
    gravity: f32,
    age: f32,
    lifetime: f32,
    start_scale: f32,
    end_scale: f32,
}

#[derive(Resource, Default)]
pub(super) struct CreatureParticleAssets {
    mesh: Option<Handle<Mesh>>,
    materials: HashMap<[u32; 4], Handle<StandardMaterial>>,
}

#[derive(SystemParam)]
struct CreatureParticleSpawner<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    assets: ResMut<'w, CreatureParticleAssets>,
    random_state: Local<'s, u32>,
}

impl CreatureParticleSpawner<'_, '_> {
    fn initialize_random_state(&mut self) {
        if *self.random_state == 0 {
            *self.random_state = INITIAL_RANDOM_STATE;
        }
    }

    fn mesh(&mut self) -> Handle<Mesh> {
        if let Some(mesh) = &self.assets.mesh {
            return mesh.clone();
        }

        let mesh = self.meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        self.assets.mesh = Some(mesh.clone());
        mesh
    }

    fn material(&mut self, color: [f32; 4]) -> Handle<StandardMaterial> {
        let color_key = color.map(f32::to_bits);
        if let Some(existing) = self.assets.materials.get(&color_key) {
            return existing.clone();
        }

        let alpha = color[3];
        let handle = self.materials.add(StandardMaterial {
            base_color: Color::srgba(color[0], color[1], color[2], alpha),
            metallic: 0.0,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            unlit: true,
            alpha_mode: if alpha < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        });
        self.assets.materials.insert(color_key, handle.clone());
        handle
    }

    fn spawn_effect(&mut self, origin: Vec3, effect: &CreatureParticleEffect) {
        let mesh = self.mesh();
        let material = self.material(effect.color);

        for _ in 0..effect.count {
            let angle = random_01(&mut self.random_state) * std::f32::consts::TAU;
            let radius = effect.spawn_radius * random_01(&mut self.random_state).sqrt();
            let horizontal = Vec2::new(angle.cos(), angle.sin());
            let size = effect.size * (0.78 + random_01(&mut self.random_state) * 0.44);
            let lifetime = effect.lifetime * (0.88 + random_01(&mut self.random_state) * 0.24);
            let vertical_jitter =
                random_signed(&mut self.random_state) * effect.vertical_jitter;
            let position = origin
                + Vec3::new(
                    horizontal.x * radius,
                    effect.y_offset
                        + random_signed(&mut self.random_state)
                            * effect.spawn_radius
                            * 0.15,
                    horizontal.y * radius,
                );
            let velocity = Vec3::new(
                horizontal.x * effect.horizontal_speed,
                effect.vertical_speed + vertical_jitter,
                horizontal.y * effect.horizontal_speed,
            );

            self.commands.spawn((
                Name::new("CreatureParticle"),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position).with_scale(Vec3::splat(size)),
                Visibility::default(),
                CreatureParticle {
                    velocity,
                    gravity: effect.gravity,
                    age: 0.0,
                    lifetime,
                    start_scale: size,
                    end_scale: effect.end_scale,
                },
                DespawnOnExit(GameState::Gameplay),
            ));
        }
    }
}

pub(super) fn emit_creature_particles(
    time: Res<Time>,
    definitions: Res<CreatureRegistry>,
    mut spawner: CreatureParticleSpawner,
    mut creatures: Query<(
        &CreatureInstance,
        &Transform,
        &CreatureAnimationState,
        &mut CreatureParticleEmitter,
    )>,
) {
    spawner.initialize_random_state();

    let dt = time
        .delta_secs()
        .min(MAX_PARTICLE_FRAME_DELTA_SECONDS);
    for (instance, transform, animation, mut emitter) in &mut creatures {
        let Some(definition) = definitions.get(&instance.definition_id) else {
            continue;
        };

        let changed = !emitter.initialized
            || emitter.state != animation.name
            || emitter.revision != animation.revision;

        if changed {
            emitter.initialized = true;
            emitter.state.clone_from(&animation.name);
            emitter.revision = animation.revision;
            emitter.timer = 0.0;
        }

        let Some(effect) = definition.particle_effects.get(&animation.name) else {
            continue;
        };

        if changed {
            spawner.spawn_effect(transform.translation, effect);
            emitter.timer = effect.interval;
            continue;
        }

        if effect.interval <= 0.0 {
            continue;
        }

        emitter.timer -= dt;
        if emitter.timer <= 0.0 {
            spawner.spawn_effect(transform.translation, effect);
            emitter.timer += effect.interval;
            if emitter.timer <= 0.0 {
                emitter.timer = effect.interval;
            }
        }
    }
}

pub(super) fn update_creature_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut CreatureParticle, &mut Transform)>,
) {
    let dt = time
        .delta_secs()
        .min(MAX_PARTICLE_FRAME_DELTA_SECONDS);
    for (entity, mut particle, mut transform) in &mut particles {
        particle.age += dt;
        if particle.age >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity.y += particle.gravity * dt;
        transform.translation += particle.velocity * dt;

        let progress = (particle.age / particle.lifetime).clamp(0.0, 1.0);
        let factor = 1.0 + (particle.end_scale - 1.0) * progress;
        transform.scale = Vec3::splat((particle.start_scale * factor).max(0.001));
    }
}

fn random_01(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}

fn random_signed(state: &mut u32) -> f32 {
    random_01(state) * 2.0 - 1.0
}
