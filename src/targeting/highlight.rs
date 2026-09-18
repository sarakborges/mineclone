use bevy::{
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    light::NotShadowCaster,
    prelude::*,
};

use super::{
    BlockTargetingScene, BlockTargetingVisualSnapshot,
    block::BlockTargetingSet,
    placement::placement_voxel,
};
use crate::{
    app::game_state::GameState,
    content::{
        block::BlockRegistry,
        builtin_ids::{BRUSH_TOOL_ID, CHISEL_TOOL_ID, DYED_PROPERTY_ID},
        secondary_property::SecondaryPropertyRegistry,
    },
    player::camera::GameplayCamera,
    tools::BrushMode,
    voxel::{
        microblock::{
            MICROBLOCK_EDGE, ChiselResolution, MicroblockMask, local_cell, parent_voxel,
        },
        raycast::raycast_micro_voxels,
    },
};

const HIGHLIGHT_SCALE: f32 = 1.025;
const TARGET_HIGHLIGHT_RENDER_LAYER: usize = 2;
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
        Without<ChiselPlacementGhost>,
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
        Without<ChiselPlacementGhost>,
        Without<GameplayCamera>,
    ),
>;

type ChiselPlacementTarget<'w, 's> = Single<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (
        With<ChiselPlacementGhost>,
        Without<BrushGhost>,
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

#[derive(Component)]
struct ChiselPlacementGhost;

#[derive(SystemParam)]
struct TargetHighlightInput<'w, 's> {
    scene: BlockTargetingScene<'w, 's>,
    brush_mode: Res<'w, BrushMode>,
    chisel_resolution: Res<'w, ChiselResolution>,
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
    chisel_placement: ChiselPlacementTarget<'w, 's>,
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
            // Keep the translucent selection shell in front of near-coplanar
            // texture/parallax layers without making it win over distant geometry.
            depth_bias: 4.0,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        RenderLayers::layer(TARGET_HIGHLIGHT_RENDER_LAYER),
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

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(HIGHLIGHT_SCALE)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.30, 0.95, 0.65, 0.24),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        RenderLayers::layer(TARGET_HIGHLIGHT_RENDER_LAYER),
        ChiselPlacementGhost,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_highlight(
    input: TargetHighlightInput,
    content: TargetHighlightContent,
    mut view: TargetHighlightView,
    mut last_scene: Local<Option<BlockTargetingVisualSnapshot>>,
) {
    let scene_snapshot = input.scene.visual_snapshot();
    let scene_changed = last_scene.as_ref() != Some(&scene_snapshot);
    let chisel_selected = input.scene.selected_item() == Some(CHISEL_TOOL_ID);
    if !scene_changed
        && !chisel_selected
        && !input.brush_mode.is_changed()
        && !content.blocks.is_changed()
        && !content.secondary_properties.is_changed()
    {
        return;
    }
    *last_scene = Some(scene_snapshot);

    let Some(hit) = input.scene.hit() else {
        hide_if_visible(&mut view.highlight.1);
        hide_if_visible(&mut view.brush_ghost.1);
        hide_if_visible(&mut view.chisel_placement.1);
        return;
    };

    if chisel_selected {
        hide_if_visible(&mut view.brush_ghost.1);
        let precise = raycast_micro_voxels(
            input.scene.world(),
            input.scene.player_translation(),
            input.scene.player_forward(),
            8.0,
        );
        let Some(precise) = precise.filter(|precise| {
            precise.voxel == hit.voxel
                && content.blocks.get(precise.block_id).is_some_and(|block| block.can_fragment())
        }) else {
            hide_if_visible(&mut view.highlight.1);
            hide_if_visible(&mut view.chisel_placement.1);
            return;
        };
        let width = input.chisel_resolution.cell_width() as i32;
        let (translation, edge) = snapped_preview(precise.fine, width);
        if view.highlight.0.translation != translation {
            view.highlight.0.translation = translation;
        }
        if view.highlight.0.scale != Vec3::splat(edge) {
            view.highlight.0.scale = Vec3::splat(edge);
        }
        show_if_hidden(&mut view.highlight.1);

        let placement_cell = precise.fine + precise.normal;
        let placement_voxel = parent_voxel(placement_cell);
        // The preview must match the actual Chisel edit: only a previously
        // carved cell of the targeted macroblock can be restored, never air or
        // a fresh neighboring block. Also suppress no-op green previews.
        let can_place = precise.normal != IVec3::ZERO
            && placement_voxel == precise.voxel
            && input.scene.world().cell_at(placement_voxel).is_some_and(|cell| {
                content.blocks.get(cell.block_id).is_some_and(|block| block.can_fragment())
                    && MicroblockMask::can_restore(cell)
                    && {
                        let mut mask = MicroblockMask::from_cell(cell);
                        mask.edit(local_cell(placement_cell), *input.chisel_resolution, true)
                    }
            });
        if can_place {
            let (translation, edge) = snapped_preview(placement_cell, width);
            if view.chisel_placement.0.translation != translation {
                view.chisel_placement.0.translation = translation;
            }
            if view.chisel_placement.0.scale != Vec3::splat(edge) {
                view.chisel_placement.0.scale = Vec3::splat(edge);
            }
            show_if_hidden(&mut view.chisel_placement.1);
        } else {
            hide_if_visible(&mut view.chisel_placement.1);
        }
        return;
    }

    hide_if_visible(&mut view.chisel_placement.1);
    if view.highlight.0.scale != Vec3::ONE {
        view.highlight.0.scale = Vec3::ONE;
    }
    let selected_item = input.scene.selected_item();
    if selected_item == Some(BRUSH_TOOL_ID) {
        hide_if_visible(&mut view.highlight.1);

        let Some(block) = content.blocks.get(hit.block_id) else {
            hide_if_visible(&mut view.brush_ghost.1);
            return;
        };
        if !block
            .secondary_properties
            .iter()
            .any(|property| property == DYED_PROPERTY_ID)
        {
            hide_if_visible(&mut view.brush_ghost.1);
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
        let material_handle = &view.brush_ghost.2.0;
        let material_color_changed = view
            .materials
            .get(material_handle)
            .is_some_and(|material| material.base_color != color);
        if material_color_changed
            && let Some(mut material) = view.materials.get_mut(material_handle)
        {
            material.base_color = color;
        }

        let translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
        if view.brush_ghost.0.translation != translation {
            view.brush_ghost.0.translation = translation;
        }
        show_if_hidden(&mut view.brush_ghost.1);
        return;
    }

    hide_if_visible(&mut view.brush_ghost.1);

    let selected_block = selected_item.filter(|item_id| content.blocks.get(item_id).is_some());
    let placement_preview_visible = selected_block.is_some()
        && placement_voxel(hit, input.scene.world(), input.scene.player_translation()).is_some();

    if placement_preview_visible {
        hide_if_visible(&mut view.highlight.1);
        return;
    }

    let translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
    if view.highlight.0.translation != translation {
        view.highlight.0.translation = translation;
    }
    show_if_hidden(&mut view.highlight.1);
}

fn snapped_preview(fine: IVec3, width: i32) -> (Vec3, f32) {
    let origin = IVec3::new(
        fine.x.div_euclid(width) * width,
        fine.y.div_euclid(width) * width,
        fine.z.div_euclid(width) * width,
    );
    let edge = width as f32 / MICROBLOCK_EDGE as f32;
    (
        origin.as_vec3() / MICROBLOCK_EDGE as f32 + Vec3::splat(edge * 0.5),
        edge,
    )
}

fn hide_if_visible(visibility: &mut Visibility) {
    if *visibility != Visibility::Hidden {
        *visibility = Visibility::Hidden;
    }
}

fn show_if_hidden(visibility: &mut Visibility) {
    if *visibility != Visibility::Visible {
        *visibility = Visibility::Visible;
    }
}
