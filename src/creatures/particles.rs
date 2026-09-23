use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::creature::{CreatureParticleEffect, CreatureRegistry},
};

use super::{CreatureAnimationState, CreatureInstance};

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

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_creature_particles(
    time: Res<Time>,
    definitions: Res<CreatureRegistry>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particle_assets: ResMut<CreatureParticleAssets>,
    mut random_state: Local<u32>,
    mut creatures: Query<(
        &CreatureInstance,
        &Transform,
        &CreatureAnimationState,
        &mut CreatureParticleEmitter,
    )>,
) {
    if *random_state == 0 {
        *random_state = 0xA5A5_1F3D;
    }

    let dt = time.delta_secs().min(0.05);
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
            spawn_effect(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut particle_assets,
                &mut random_state,
                transform.translation,
                effect,
            );
            emitter.timer = effect.interval;
            continue;
        }

        if effect.interval <= 0.0 {
            continue;
        }

        emitter.timer -= dt;
        if emitter.timer <= 0.0 {
            spawn_effect(
                &mut commands,
                &mut meshes,
                &mut materials,
                &mut particle_assets,
                &mut random_state,
                transform.translation,
                effect,
            );
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
    let dt = time.delta_secs().min(0.05);
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

#[allow(clippy::too_many_arguments)]
fn spawn_effect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    particle_assets: &mut CreatureParticleAssets,
    random_state: &mut u32,
    origin: Vec3,
    effect: &CreatureParticleEffect,
) {
    let mesh = if let Some(mesh) = &particle_assets.mesh {
        mesh.clone()
    } else {
        let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        particle_assets.mesh = Some(mesh.clone());
        mesh
    };

    let color_key = effect.color.map(f32::to_bits);
    let material = if let Some(existing) = particle_assets.materials.get(&color_key) {
        existing.clone()
    } else {
        let alpha = effect.color[3];
        let handle = materials.add(StandardMaterial {
            base_color: Color::srgba(
                effect.color[0],
                effect.color[1],
                effect.color[2],
                alpha,
            ),
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
        particle_assets
            .materials
            .insert(color_key, handle.clone());
        handle
    };

    for _ in 0..effect.count {
        let angle = random_01(random_state) * std::f32::consts::TAU;
        let radius = effect.spawn_radius * random_01(random_state).sqrt();
        let horizontal = Vec2::new(angle.cos(), angle.sin());
        let size = effect.size * (0.78 + random_01(random_state) * 0.44);
        let lifetime = effect.lifetime * (0.88 + random_01(random_state) * 0.24);
        let vertical_jitter = random_signed(random_state) * effect.vertical_jitter;
        let position = origin
            + Vec3::new(
                horizontal.x * radius,
                effect.y_offset + random_signed(random_state) * effect.spawn_radius * 0.15,
                horizontal.y * radius,
            );
        let velocity = Vec3::new(
            horizontal.x * effect.horizontal_speed,
            effect.vertical_speed + vertical_jitter,
            horizontal.y * effect.horizontal_speed,
        );

        commands.spawn((
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

fn random_01(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32
}

fn random_signed(state: &mut u32) -> f32 {
    random_01(state) * 2.0 - 1.0
}
