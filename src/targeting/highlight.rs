use bevy::{light::NotShadowCaster, prelude::*};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};
use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry, builtin_ids::BRUSH_TOOL_ID,
        secondary_property::SecondaryPropertyRegistry,
    },
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    tools::{BrushMode, DYED_PROPERTY_ID},
    voxel::world::VoxelWorld,
};

const HIGHLIGHT_SCALE: f32 = 1.01;
const BRUSH_GHOST_SCALE: f32 = 1.012;
const BRUSH_GHOST_ALPHA: f32 = 0.30;
const BRUSH_CLEAR_GHOST_ALPHA: f32 = 0.12;

type HighlightTarget<'w, 's> = Single<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (With<TargetHighlight>, Without<BrushGhost>, Without<GameplayCamera>),
>;

type BrushGhostTarget<'w, 's> = Single<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static MeshMaterial3d<StandardMaterial>,
    ),
    (With<BrushGhost>, Without<TargetHighlight>, Without<GameplayCamera>),
>;

pub struct TargetHighlightPlugin;

impl Plugin for TargetHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_highlight)
            .add_systems(
                Update,
                update_highlight
                    .in_set(BlockTargetingSet::Visuals)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHighlight;

#[derive(Component)]
struct BrushGhost;

fn spawn_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(HIGHLIGHT_SCALE)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.18),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        TargetHighlight,
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(BRUSH_GHOST_SCALE)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, BRUSH_GHOST_ALPHA),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        BrushGhost,
        DespawnOnExit(GameState::Gameplay),
    ));
}

#[allow(clippy::too_many_arguments)]
fn update_highlight(
    targeted: Res<TargetedBlock>,
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    secondary_properties: Res<SecondaryPropertyRegistry>,
    brush_mode: Res<BrushMode>,
    world: Res<VoxelWorld>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut highlight: HighlightTarget,
    mut brush_ghost: BrushGhostTarget,
) {
    let Some(hit) = targeted.0 else {
        *highlight.1 = Visibility::Hidden;
        *brush_ghost.1 = Visibility::Hidden;
        return;
    };

    let selected_item = hotbar.item_at(hotbar.selected_slot());
    if selected_item == Some(BRUSH_TOOL_ID) {
        *highlight.1 = Visibility::Hidden;

        let Some(block) = blocks.get(hit.block_id) else {
            *brush_ghost.1 = Visibility::Hidden;
            return;
        };
        if !block
            .secondary_properties
            .iter()
            .any(|property| property == DYED_PROPERTY_ID)
        {
            *brush_ghost.1 = Visibility::Hidden;
            return;
        }

        let color = brush_mode.dye_id().map_or(
            Color::srgba(0.92, 0.92, 1.0, BRUSH_CLEAR_GHOST_ALPHA),
            |dye_id| {
                secondary_properties.get(DYED_PROPERTY_ID, dye_id).map_or(
                    Color::srgba(1.0, 1.0, 1.0, BRUSH_GHOST_ALPHA),
                    |definition| {
                        Color::srgba(
                            definition.color.r,
                            definition.color.g,
                            definition.color.b,
                            BRUSH_GHOST_ALPHA,
                        )
                    },
                )
            },
        );
        if let Some(material) = materials.get_mut(&brush_ghost.2.0) {
            material.base_color = color;
        }

        brush_ghost.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
        *brush_ghost.1 = Visibility::Visible;
        return;
    }

    *brush_ghost.1 = Visibility::Hidden;

    let selected_block = selected_item.filter(|item_id| blocks.get(item_id).is_some());
    let placement_preview_visible = selected_block.is_some()
        && placement_voxel(hit, &world, player.translation).is_some();

    if placement_preview_visible {
        *highlight.1 = Visibility::Hidden;
        return;
    }

    highlight.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
    *highlight.1 = Visibility::Visible;
}
