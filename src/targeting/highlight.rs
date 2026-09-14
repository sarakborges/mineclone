use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use super::{
    BlockTargetingScene,
    block::BlockTargetingSet,
    placement::placement_voxel,
};
use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry,
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
        secondary_property::SecondaryPropertyRegistry,
    },
    player::camera::GameplayCamera,
    tools::BrushMode,
};

const HIGHLIGHT_SCALE: f32 = 1.01;
const BRUSH_GHOST_SCALE: f32 = 1.012;
const BRUSH_GHOST_ALPHA: f32 = 0.30;
const BRUSH_CLEAR_GHOST_ALPHA: f32 = 0.12;

type HighlightTarget<'w, 's> = Single<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (
        With<TargetHighlight>,
        Without<BrushGhost>,
        Without<GameplayCamera>,
    ),
>;

type BrushGhostTarget<'w, 's> = Single<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static MeshMaterial3d<StandardMaterial>,
    ),
    (
        With<BrushGhost>,
        Without<TargetHighlight>,
        Without<GameplayCamera>,
    ),
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

#[derive(SystemParam)]
struct TargetHighlightInput<'w, 's> {
    scene: BlockTargetingScene<'w, 's>,
    brush_mode: Res<'w, BrushMode>,
}

#[derive(SystemParam)]
struct TargetHighlightContent<'w> {
    blocks: Res<'w, BlockRegistry>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
}

#[derive(SystemParam)]
struct TargetHighlightView<'w, 's> {
    materials: ResMut<'w, Assets<StandardMaterial>>,
    highlight: HighlightTarget<'w, 's>,
    brush_ghost: BrushGhostTarget<'w, 's>,
}

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

fn update_highlight(
    input: TargetHighlightInput,
    content: TargetHighlightContent,
    mut view: TargetHighlightView,
) {
    let Some(hit) = input.scene.hit() else {
        *view.highlight.1 = Visibility::Hidden;
        *view.brush_ghost.1 = Visibility::Hidden;
        return;
    };

    let selected_item = input.scene.selected_item();
    if selected_item == Some(BRUSH_TOOL_ID) {
        *view.highlight.1 = Visibility::Hidden;

        let Some(block) = content.blocks.get(hit.block_id) else {
            *view.brush_ghost.1 = Visibility::Hidden;
            return;
        };
        if !block
            .secondary_properties
            .iter()
            .any(|property| property == DYED_PROPERTY_ID)
        {
            *view.brush_ghost.1 = Visibility::Hidden;
            return;
        }

        let color = input.brush_mode.dye_id().map_or(
            Color::srgba(0.92, 0.92, 1.0, BRUSH_CLEAR_GHOST_ALPHA),
            |dye_id| {
                content
                    .secondary_properties
                    .get(DYED_PROPERTY_ID, dye_id)
                    .map_or(
                        Color::srgba(1.0, 1.0, 1.0, BRUSH_GHOST_ALPHA),
                        |definition| {
                            let [red, green, blue] = definition.color.to_srgb();
                            Color::srgba(red, green, blue, BRUSH_GHOST_ALPHA)
                        },
                    )
            },
        );
        if let Some(mut material) = view.materials.get_mut(&view.brush_ghost.2.0) {
            material.base_color = color;
        }

        view.brush_ghost.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
        *view.brush_ghost.1 = Visibility::Visible;
        return;
    }

    *view.brush_ghost.1 = Visibility::Hidden;

    let selected_block = selected_item.filter(|item_id| content.blocks.get(item_id).is_some());
    let placement_preview_visible = selected_block.is_some()
        && placement_voxel(hit, input.scene.world(), input.scene.player_translation()).is_some();

    if placement_preview_visible {
        *view.highlight.1 = Visibility::Hidden;
        return;
    }

    view.highlight.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
    *view.highlight.1 = Visibility::Visible;
}
