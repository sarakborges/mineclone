use bevy::prelude::*;

use crate::app::{game_state::GameState, pause_state::PauseState};

/// Shared gameplay health for every damageable entity, including the player.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct EntityHealth {
    current: f32,
    max: f32,
    hurt_timer: f32,
}

impl EntityHealth {
    pub(crate) fn new(max: f32) -> Self {
        assert!(max.is_finite() && max > 0.0, "entity health must be positive and finite");
        Self { current: max, max, hurt_timer: 0.0 }
    }

    pub(crate) fn current(self) -> f32 { self.current }
    pub(crate) fn max(self) -> f32 { self.max }
    pub(crate) fn is_dead(self) -> bool { self.current <= 0.0 }

    pub(crate) fn damage(&mut self, amount: f32) -> bool {
        if self.is_dead() { return true; }
        self.current = (self.current - amount.max(0.0)).max(0.0);
        if amount > 0.0 { self.hurt_timer = 0.15; }
        self.is_dead()
    }

    pub(crate) fn tick_hurt(&mut self, delta: f32) {
        self.hurt_timer = (self.hurt_timer - delta).max(0.0);
    }

    pub(crate) fn is_hurt(self) -> bool { self.hurt_timer > 0.0 }
}


#[derive(Component, Clone)]
pub(crate) struct DamageFlashMaterial {
    pub(crate) original: Handle<StandardMaterial>,
}


pub(crate) struct EntityPlugin;

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            tick_entity_health
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(PauseState::Running)),
        )
        .add_systems(
            PostUpdate,
            sync_entity_damage_flash.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn tick_entity_health(time: Res<Time>, mut health: Query<&mut EntityHealth>) {
    for mut health in &mut health {
        health.tick_hurt(time.delta_secs());
    }
}

fn sync_entity_damage_flash(
    health: Query<(&EntityHealth, &Children)>,
    descendants: Query<&Children>,
    mesh_materials: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&DamageFlashMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for (entity_health, root_children) in &health {
        let mut entities = Vec::new();
        for &child in root_children {
            entities.push(child);
            entities.extend(descendants.iter_descendants(child));
        }
        for mesh_entity in entities {
            let Ok((mesh_entity, current, flash)) = mesh_materials.get(mesh_entity) else {
                continue;
            };
            if entity_health.is_hurt() {
                if flash.is_none() {
                    let flash_material = materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    });
                    commands.entity(mesh_entity).insert((
                        DamageFlashMaterial { original: current.0.clone() },
                        MeshMaterial3d(flash_material),
                    ));
                }
            } else if let Some(flash) = flash {
                commands.entity(mesh_entity)
                    .insert(MeshMaterial3d(flash.original.clone()))
                    .remove::<DamageFlashMaterial>();
            }
        }
    }
}
